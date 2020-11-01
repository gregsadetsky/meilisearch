use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Seek, SeekFrom};
use std::sync::mpsc::sync_channel;
use std::time::Instant;

use anyhow::Context;
use bstr::ByteSlice as _;
use grenad::{Writer, Sorter, Merger, Reader, FileFuse, CompressionType};
use heed::types::ByteSlice;
use log::{debug, info, error};
use rayon::prelude::*;
use crate::index::Index;
use self::store::Store;
use self::merge_function::{
    main_merge, word_docids_merge, words_pairs_proximities_docids_merge,
    docid_word_positions_merge, documents_merge,
};
pub use self::transform::{Transform, TransformOutput};

use crate::MergeFn;
use super::UpdateBuilder;

mod merge_function;
mod store;
mod transform;

#[derive(Debug, Copy, Clone)]
enum WriteMethod {
    Append,
    GetMergePut,
}

fn create_writer(typ: CompressionType, level: Option<u32>, file: File) -> io::Result<Writer<File>> {
    let mut builder = Writer::builder();
    builder.compression_type(typ);
    if let Some(level) = level {
        builder.compression_level(level);
    }
    builder.build(file)
}

fn create_sorter(
    merge: MergeFn,
    chunk_compression_type: CompressionType,
    chunk_compression_level: Option<u32>,
    chunk_fusing_shrink_size: Option<u64>,
    max_nb_chunks: Option<usize>,
    max_memory: Option<usize>,
) -> Sorter<MergeFn>
{
    let mut builder = Sorter::builder(merge);
    if let Some(shrink_size) = chunk_fusing_shrink_size {
        builder.file_fusing_shrink_size(shrink_size);
    }
    builder.chunk_compression_type(chunk_compression_type);
    if let Some(level) = chunk_compression_level {
        builder.chunk_compression_level(level);
    }
    if let Some(nb_chunks) = max_nb_chunks {
        builder.max_nb_chunks(nb_chunks);
    }
    if let Some(memory) = max_memory {
        builder.max_memory(memory);
    }
    builder.build()
}

fn writer_into_reader(writer: Writer<File>, shrink_size: Option<u64>) -> anyhow::Result<Reader<FileFuse>> {
    let mut file = writer.into_inner()?;
    file.seek(SeekFrom::Start(0))?;
    let file = if let Some(shrink_size) = shrink_size {
        FileFuse::builder().shrink_size(shrink_size).build(file)
    } else {
        FileFuse::new(file)
    };
    Reader::new(file).map_err(Into::into)
}

fn merge_readers(sources: Vec<Reader<FileFuse>>, merge: MergeFn) -> Merger<FileFuse, MergeFn> {
    let mut builder = Merger::builder(merge);
    builder.extend(sources);
    builder.build()
}

fn merge_into_lmdb_database(
    wtxn: &mut heed::RwTxn,
    database: heed::PolyDatabase,
    sources: Vec<Reader<FileFuse>>,
    merge: MergeFn,
    method: WriteMethod,
) -> anyhow::Result<()> {
    debug!("Merging {} MTBL stores...", sources.len());
    let before = Instant::now();

    let merger = merge_readers(sources, merge);
    let mut in_iter = merger.into_merge_iter()?;

    match method {
        WriteMethod::Append => {
            let mut out_iter = database.iter_mut::<_, ByteSlice, ByteSlice>(wtxn)?;
            while let Some((k, v)) = in_iter.next()? {
                out_iter.append(k, v).with_context(|| {
                    format!("writing {:?} into LMDB", k.as_bstr())
                })?;
            }
        },
        WriteMethod::GetMergePut => {
            while let Some((k, v)) = in_iter.next()? {
                let mut iter = database.prefix_iter_mut::<_, ByteSlice, ByteSlice>(wtxn, k)?;
                match iter.next().transpose()? {
                    Some((key, old_val)) if key == k => {
                        let vals = vec![Cow::Borrowed(old_val), Cow::Borrowed(v)];
                        let val = merge(k, &vals).expect("merge failed");
                        iter.put_current(k, &val)?;
                    },
                    _ => {
                        drop(iter);
                        database.put::<_, ByteSlice, ByteSlice>(wtxn, k, v)?;
                    },
                }
            }
        },
    }

    debug!("MTBL stores merged in {:.02?}!", before.elapsed());
    Ok(())
}

fn write_into_lmdb_database(
    wtxn: &mut heed::RwTxn,
    database: heed::PolyDatabase,
    mut reader: Reader<FileFuse>,
    merge: MergeFn,
    method: WriteMethod,
) -> anyhow::Result<()> {
    debug!("Writing MTBL stores...");
    let before = Instant::now();

    match method {
        WriteMethod::Append => {
            let mut out_iter = database.iter_mut::<_, ByteSlice, ByteSlice>(wtxn)?;
            while let Some((k, v)) = reader.next()? {
                out_iter.append(k, v).with_context(|| {
                    format!("writing {:?} into LMDB", k.as_bstr())
                })?;
            }
        },
        WriteMethod::GetMergePut => {
            while let Some((k, v)) = reader.next()? {
                let mut iter = database.prefix_iter_mut::<_, ByteSlice, ByteSlice>(wtxn, k)?;
                match iter.next().transpose()? {
                    Some((key, old_val)) if key == k => {
                        let vals = vec![Cow::Borrowed(old_val), Cow::Borrowed(v)];
                        let val = merge(k, &vals).expect("merge failed");
                        iter.put_current(k, &val)?;
                    },
                    _ => {
                        drop(iter);
                        database.put::<_, ByteSlice, ByteSlice>(wtxn, k, v)?;
                    },
                }
            }
        }
    }

    debug!("MTBL stores merged in {:.02?}!", before.elapsed());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IndexDocumentsMethod {
    /// Replace the previous document with the new one,
    /// removing all the already known attributes.
    ReplaceDocuments,

    /// Merge the previous version of the document with the new version,
    /// replacing old attributes values with the new ones and add the new attributes.
    UpdateDocuments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UpdateFormat {
    /// The given update is a real **comma seperated** CSV with headers on the first line.
    Csv,
    /// The given update is a JSON array with documents inside.
    Json,
    /// The given update is a JSON stream with a document on each line.
    JsonStream,
}

pub struct IndexDocuments<'t, 'u, 'i> {
    wtxn: &'t mut heed::RwTxn<'i, 'u>,
    index: &'i Index,
    pub(crate) log_every_n: Option<usize>,
    pub(crate) max_nb_chunks: Option<usize>,
    pub(crate) max_memory: Option<usize>,
    pub(crate) linked_hash_map_size: Option<usize>,
    pub(crate) chunk_compression_type: CompressionType,
    pub(crate) chunk_compression_level: Option<u32>,
    pub(crate) chunk_fusing_shrink_size: Option<u64>,
    pub(crate) indexing_jobs: Option<usize>,
    update_method: IndexDocumentsMethod,
    update_format: UpdateFormat,
    autogenerate_docids: bool,
}

impl<'t, 'u, 'i> IndexDocuments<'t, 'u, 'i> {
    pub fn new(wtxn: &'t mut heed::RwTxn<'i, 'u>, index: &'i Index) -> IndexDocuments<'t, 'u, 'i> {
        IndexDocuments {
            wtxn,
            index,
            log_every_n: None,
            max_nb_chunks: None,
            max_memory: None,
            linked_hash_map_size: None,
            chunk_compression_type: CompressionType::None,
            chunk_compression_level: None,
            chunk_fusing_shrink_size: None,
            indexing_jobs: None,
            update_method: IndexDocumentsMethod::ReplaceDocuments,
            update_format: UpdateFormat::Json,
            autogenerate_docids: true,
        }
    }

    pub fn index_documents_method(&mut self, method: IndexDocumentsMethod) {
        self.update_method = method;
    }

    pub fn update_format(&mut self, format: UpdateFormat) {
        self.update_format = format;
    }

    pub fn enable_autogenerate_docids(&mut self) {
        self.autogenerate_docids = true;
    }

    pub fn disable_autogenerate_docids(&mut self) {
        self.autogenerate_docids = false;
    }

    pub fn execute<R, F>(self, reader: R, progress_callback: F) -> anyhow::Result<()>
    where
        R: io::Read,
        F: Fn(usize, usize) + Sync,
    {
        let before_indexing = Instant::now();

        let transform = Transform {
            rtxn: &self.wtxn,
            index: self.index,
            chunk_compression_type: self.chunk_compression_type,
            chunk_compression_level: self.chunk_compression_level,
            chunk_fusing_shrink_size: self.chunk_fusing_shrink_size,
            max_nb_chunks: self.max_nb_chunks,
            max_memory: self.max_memory,
            index_documents_method: self.update_method,
            autogenerate_docids: self.autogenerate_docids,
        };

        let output = match self.update_format {
            UpdateFormat::Csv => transform.from_csv(reader)?,
            UpdateFormat::Json => transform.from_json(reader)?,
            UpdateFormat::JsonStream => transform.from_json_stream(reader)?,
        };

        let TransformOutput {
            primary_key,
            fields_ids_map,
            users_ids_documents_ids,
            new_documents_ids,
            replaced_documents_ids,
            documents_count,
            documents_file,
        } = output;

        // We delete the documents that this document addition replaces. This way we are
        // able to simply insert all the documents even if they already exist in the database.
        if !replaced_documents_ids.is_empty() {
            let update_builder = UpdateBuilder {
                log_every_n: self.log_every_n,
                max_nb_chunks: self.max_nb_chunks,
                max_memory: self.max_memory,
                linked_hash_map_size: self.linked_hash_map_size,
                chunk_compression_type: self.chunk_compression_type,
                chunk_compression_level: self.chunk_compression_level,
                chunk_fusing_shrink_size: self.chunk_fusing_shrink_size,
                indexing_jobs: self.indexing_jobs,
            };
            let mut deletion_builder = update_builder.delete_documents(self.wtxn, self.index)?;
            deletion_builder.delete_documents(&replaced_documents_ids);
            let _deleted_documents_count = deletion_builder.execute()?;
        }

        let mmap = if documents_count == 0 {
            None
        } else {
            let mmap = unsafe {
                memmap::Mmap::map(&documents_file).context("mmaping the transform documents file")?
            };
            Some(mmap)
        };

        let bytes = mmap.as_ref().map(AsRef::as_ref).unwrap_or_default();
        let documents = grenad::Reader::new(bytes).unwrap();

        // The enum which indicates the type of the readers
        // merges that are potentially done on different threads.
        enum DatabaseType {
            Main,
            WordDocids,
            WordsPairsProximitiesDocids,
        }

        let linked_hash_map_size = self.linked_hash_map_size;
        let max_nb_chunks = self.max_nb_chunks;
        let max_memory = self.max_memory;
        let chunk_compression_type = self.chunk_compression_type;
        let chunk_compression_level = self.chunk_compression_level;
        let log_every_n = self.log_every_n;
        let chunk_fusing_shrink_size = self.chunk_fusing_shrink_size;

        let jobs = self.indexing_jobs.unwrap_or(0);
        let pool = rayon::ThreadPoolBuilder::new().num_threads(jobs).build()?;

        let (receiver, docid_word_positions_readers, documents_readers) = pool.install(|| {
            let num_threads = rayon::current_num_threads();
            let max_memory_by_job = max_memory.map(|mm| mm / num_threads);

            let readers = rayon::iter::repeatn(documents, num_threads)
                .enumerate()
                .map(|(i, documents)| {
                    let store = Store::new(
                        linked_hash_map_size,
                        max_nb_chunks,
                        max_memory_by_job,
                        chunk_compression_type,
                        chunk_compression_level,
                        chunk_fusing_shrink_size,
                    )?;
                    store.index(
                        documents,
                        documents_count,
                        i,
                        num_threads,
                        log_every_n,
                        &progress_callback,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut main_readers = Vec::with_capacity(readers.len());
            let mut word_docids_readers = Vec::with_capacity(readers.len());
            let mut docid_word_positions_readers = Vec::with_capacity(readers.len());
            let mut words_pairs_proximities_docids_readers = Vec::with_capacity(readers.len());
            let mut documents_readers = Vec::with_capacity(readers.len());
            readers.into_iter().for_each(|readers| {
                main_readers.push(readers.main);
                word_docids_readers.push(readers.word_docids);
                docid_word_positions_readers.push(readers.docid_word_positions);
                words_pairs_proximities_docids_readers.push(readers.words_pairs_proximities_docids);
                documents_readers.push(readers.documents);
            });

            // This is the function that merge the readers
            // by using the given merge function.
            let merge_readers = move |readers, merge| {
                let mut writer = tempfile::tempfile().and_then(|f| {
                    create_writer(chunk_compression_type, chunk_compression_level, f)
                })?;
                let merger = merge_readers(readers, merge);
                merger.write_into(&mut writer)?;
                writer_into_reader(writer, chunk_fusing_shrink_size)
            };

            // The enum and the channel which is used to transfert
            // the readers merges potentially done on another thread.
            let (sender, receiver) = sync_channel(3);

            debug!("Merging the main, word docids and words pairs proximity docids in parallel...");
            rayon::spawn(move || {
                vec![
                    (DatabaseType::Main, main_readers, main_merge as MergeFn),
                    (DatabaseType::WordDocids, word_docids_readers, word_docids_merge),
                    (
                        DatabaseType::WordsPairsProximitiesDocids,
                        words_pairs_proximities_docids_readers,
                        words_pairs_proximities_docids_merge,
                    ),
                ]
                .into_par_iter()
                .for_each(|(dbtype, readers, merge)| {
                    let result = merge_readers(readers, merge);
                    if let Err(e) = sender.send((dbtype, result)) {
                        error!("sender error: {}", e);
                    }
                });
            });

            Ok((receiver, docid_word_positions_readers, documents_readers)) as anyhow::Result<_>
        })?;

        let mut documents_ids = self.index.documents_ids(self.wtxn)?;
        let contains_documents = !documents_ids.is_empty();
        let write_method = if contains_documents {
            WriteMethod::GetMergePut
        } else {
            WriteMethod::Append
        };

        debug!("Writing using the write method: {:?}", write_method);

        // We write the fields ids map into the main database
        self.index.put_fields_ids_map(self.wtxn, &fields_ids_map)?;

        // We write the primary key field id into the main database
        self.index.put_primary_key(self.wtxn, primary_key)?;

        // We write the users_ids_documents_ids into the main database.
        self.index.put_users_ids_documents_ids(self.wtxn, &users_ids_documents_ids)?;

        // We merge the new documents ids with the existing ones.
        documents_ids.union_with(&new_documents_ids);
        documents_ids.union_with(&replaced_documents_ids);
        self.index.put_documents_ids(self.wtxn, &documents_ids)?;

        debug!("Writing the docid word positions into LMDB on disk...");
        merge_into_lmdb_database(
            self.wtxn,
            *self.index.docid_word_positions.as_polymorph(),
            docid_word_positions_readers,
            docid_word_positions_merge,
            write_method
        )?;

        debug!("Writing the documents into LMDB on disk...");
        merge_into_lmdb_database(
            self.wtxn,
            *self.index.documents.as_polymorph(),
            documents_readers,
            documents_merge,
            write_method
        )?;

        for (db_type, result) in receiver {
            let content = result?;
            match db_type {
                DatabaseType::Main => {
                    debug!("Writing the main elements into LMDB on disk...");
                    write_into_lmdb_database(
                        self.wtxn,
                        self.index.main,
                        content,
                        main_merge,
                        WriteMethod::GetMergePut,
                    )?;
                },
                DatabaseType::WordDocids => {
                    debug!("Writing the words docids into LMDB on disk...");
                    let db = *self.index.word_docids.as_polymorph();
                    write_into_lmdb_database(
                        self.wtxn,
                        db,
                        content,
                        word_docids_merge,
                        write_method,
                    )?;
                },
                DatabaseType::WordsPairsProximitiesDocids => {
                    debug!("Writing the words pairs proximities docids into LMDB on disk...");
                    let db = *self.index.word_pair_proximity_docids.as_polymorph();
                    write_into_lmdb_database(
                        self.wtxn,
                        db,
                        content,
                        words_pairs_proximities_docids_merge,
                        write_method,
                    )?;
                },
            }
        }

        info!("Update processed in {:.02?}", before_indexing.elapsed());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use heed::EnvOpenOptions;

    #[test]
    fn simple_document_replacement() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with ids from 1 to 3.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,name\n1,kevin\n2,kevina\n3,benoit\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 3 documents now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);
        drop(rtxn);

        // Second we send 1 document with id 1, to erase the previous ones.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,name\n1,updated kevin\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is **always** 3 documents.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);
        drop(rtxn);

        // Third we send 3 documents again to replace the existing ones.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,name\n1,updated second kevin\n2,updated kevina\n3,updated benoit\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is **always** 3 documents.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);
        drop(rtxn);
    }

    #[test]
    fn simple_document_merge() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with duplicate ids and
        // change the index method to merge documents.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,name\n1,kevin\n1,kevina\n1,benoit\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.index_documents_method(IndexDocumentsMethod::UpdateDocuments);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is only 1 document now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 1);

        // Check that we get only one document from the database.
        let docs = index.documents(&rtxn, Some(0)).unwrap();
        assert_eq!(docs.len(), 1);
        let (id, doc) = docs[0];
        assert_eq!(id, 0);

        // Check that this document is equal to the last one sent.
        let mut doc_iter = doc.iter();
        assert_eq!(doc_iter.next(), Some((0, &br#""1""#[..])));
        assert_eq!(doc_iter.next(), Some((1, &br#""benoit""#[..])));
        assert_eq!(doc_iter.next(), None);
        drop(rtxn);

        // Second we send 1 document with id 1, to force it to be merged with the previous one.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,age\n1,25\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.index_documents_method(IndexDocumentsMethod::UpdateDocuments);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is **always** 1 document.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 1);

        // Check that we get only one document from the database.
        let docs = index.documents(&rtxn, Some(0)).unwrap();
        assert_eq!(docs.len(), 1);
        let (id, doc) = docs[0];
        assert_eq!(id, 0);

        // Check that this document is equal to the last one sent.
        let mut doc_iter = doc.iter();
        assert_eq!(doc_iter.next(), Some((0, &br#""1""#[..])));
        assert_eq!(doc_iter.next(), Some((1, &br#""benoit""#[..])));
        assert_eq!(doc_iter.next(), Some((2, &br#""25""#[..])));
        assert_eq!(doc_iter.next(), None);
        drop(rtxn);
    }

    #[test]
    fn not_auto_generated_csv_documents_ids() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with ids from 1 to 3.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"name\nkevin\nkevina\nbenoit\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.disable_autogenerate_docids();
        builder.update_format(UpdateFormat::Csv);
        assert!(builder.execute(content, |_, _| ()).is_err());
        wtxn.commit().unwrap();

        // Check that there is no document.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 0);
        drop(rtxn);
    }

    #[test]
    fn not_auto_generated_json_documents_ids() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents and 2 without ids.
        let mut wtxn = index.write_txn().unwrap();
        let content = &br#"[
            { "name": "kevina", "id": 21 },
            { "name": "kevin" },
            { "name": "benoit" }
        ]"#[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.disable_autogenerate_docids();
        builder.update_format(UpdateFormat::Json);
        assert!(builder.execute(content, |_, _| ()).is_err());
        wtxn.commit().unwrap();

        // Check that there is no document.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 0);
        drop(rtxn);
    }

    #[test]
    fn simple_auto_generated_documents_ids() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with ids from 1 to 3.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"name\nkevin\nkevina\nbenoit\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 3 documents now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);

        let docs = index.documents(&rtxn, vec![0, 1, 2]).unwrap();
        let (_id, obkv) = docs.iter().find(|(_id, kv)| kv.get(0) == Some(br#""kevin""#)).unwrap();
        let kevin_uuid: String = serde_json::from_slice(&obkv.get(1).unwrap()).unwrap();
        drop(rtxn);

        // Second we send 1 document with the generated uuid, to erase the previous ones.
        let mut wtxn = index.write_txn().unwrap();
        let content = format!("id,name\n{},updated kevin", kevin_uuid);
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content.as_bytes(), |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is **always** 3 documents.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);

        let docs = index.documents(&rtxn, vec![0, 1, 2]).unwrap();
        let (kevin_id, _) = docs.iter().find(|(_, d)| {
            d.get(0).unwrap() == br#""updated kevin""#
        }).unwrap();
        let (id, doc) = docs[*kevin_id as usize];
        assert_eq!(id, *kevin_id);

        // Check that this document is equal to the last
        // one sent and that an UUID has been generated.
        assert_eq!(doc.get(0), Some(&br#""updated kevin""#[..]));
        // This is an UUID, it must be 36 bytes long plus the 2 surrounding string quotes (").
        assert!(doc.get(1).unwrap().len() == 36 + 2);
        drop(rtxn);
    }

    #[test]
    fn reordered_auto_generated_documents_ids() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with ids from 1 to 3.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,name\n1,kevin\n2,kevina\n3,benoit\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 3 documents now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);
        drop(rtxn);

        // Second we send 1 document without specifying the id.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"name\nnew kevin"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 4 documents now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 4);
        drop(rtxn);
    }

    #[test]
    fn empty_csv_update() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 0 documents and only headers.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"id,name\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is no documents.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 0);
        drop(rtxn);
    }

    #[test]
    fn json_documents() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with an id for only one of them.
        let mut wtxn = index.write_txn().unwrap();
        let content = &br#"[
            { "name": "kevin" },
            { "name": "kevina", "id": 21 },
            { "name": "benoit" }
        ]"#[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Json);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 3 documents now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);
        drop(rtxn);
    }

    #[test]
    fn empty_json_update() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 0 documents.
        let mut wtxn = index.write_txn().unwrap();
        let content = &b"[]"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Json);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is no documents.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 0);
        drop(rtxn);
    }

    #[test]
    fn json_stream_documents() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 3 documents with an id for only one of them.
        let mut wtxn = index.write_txn().unwrap();
        let content = &br#"
        { "name": "kevin" }
        { "name": "kevina", "id": 21 }
        { "name": "benoit" }
        "#[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::JsonStream);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 3 documents now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 3);
        drop(rtxn);
    }

    #[test]
    fn invalid_documents_ids() {
        let path = tempfile::tempdir().unwrap();
        let mut options = EnvOpenOptions::new();
        options.map_size(10 * 1024 * 1024); // 10 MB
        let index = Index::new(options, &path).unwrap();

        // First we send 1 document with an invalid id.
        let mut wtxn = index.write_txn().unwrap();
        // There is a space in the document id.
        let content = &b"id,name\nbrume bleue,kevin\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        assert!(builder.execute(content, |_, _| ()).is_err());
        wtxn.commit().unwrap();

        // First we send 1 document with a valid id.
        let mut wtxn = index.write_txn().unwrap();
        // There is a space in the document id.
        let content = &b"id,name\n32,kevin\n"[..];
        let mut builder = IndexDocuments::new(&mut wtxn, &index);
        builder.update_format(UpdateFormat::Csv);
        builder.execute(content, |_, _| ()).unwrap();
        wtxn.commit().unwrap();

        // Check that there is 1 document now.
        let rtxn = index.read_txn().unwrap();
        let count = index.number_of_documents(&rtxn).unwrap();
        assert_eq!(count, 1);
        drop(rtxn);
    }
}