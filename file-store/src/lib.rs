use std::fs::File as StdFile;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use tempfile::NamedTempFile;
use uuid::Uuid;

const UPDATE_FILES_PATH: &str = "updates/updates_files";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not parse file name as utf-8")]
    CouldNotParseFileNameAsUtf8,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    PersistError(#[from] tempfile::PersistError),
    #[error(transparent)]
    UuidError(#[from] uuid::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Deref for File {
    type Target = NamedTempFile;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for File {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

#[derive(Clone, Debug)]
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn new(path: impl AsRef<Path>) -> Result<FileStore> {
        let path = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&path)?;
        Ok(FileStore { path })
    }
}

impl FileStore {
    /// Creates a new temporary update file.
    /// A call to `persist` is needed to persist the file in the database.
    pub fn new_update(&self) -> Result<(Uuid, File)> {
        let file = NamedTempFile::new_in(&self.path)?;
        let uuid = Uuid::new_v4();
        let path = self.path.join(uuid.to_string());
        let update_file = File { file, path };

        Ok((uuid, update_file))
    }

    /// Creates a new temporary update file with the given Uuid.
    /// A call to `persist` is needed to persist the file in the database.
    pub fn new_update_with_uuid(&self, uuid: u128) -> Result<(Uuid, File)> {
        let file = NamedTempFile::new_in(&self.path)?;
        let uuid = Uuid::from_u128(uuid);
        let path = self.path.join(uuid.to_string());
        let update_file = File { file, path };

        Ok((uuid, update_file))
    }

    /// Returns the file corresponding to the requested uuid.
    pub fn get_update(&self, uuid: Uuid) -> Result<StdFile> {
        let path = self.get_update_path(uuid);
        let file = StdFile::open(path)?;
        Ok(file)
    }

    /// Returns the path that correspond to this uuid, the path could not exists.
    pub fn get_update_path(&self, uuid: Uuid) -> PathBuf {
        self.path.join(uuid.to_string())
    }

    /// Copies the content of the update file pointed to by `uuid` to the `dst` directory.
    pub fn snapshot(&self, uuid: Uuid, dst: impl AsRef<Path>) -> Result<()> {
        let src = self.path.join(uuid.to_string());
        let mut dst = dst.as_ref().join(UPDATE_FILES_PATH);
        std::fs::create_dir_all(&dst)?;
        dst.push(uuid.to_string());
        std::fs::copy(src, dst)?;
        Ok(())
    }

    pub fn update_total_size(&self) -> Result<u64> {
        let mut total = 0;
        for uuid in self.all_uuids()? {
            total += self.get_size(uuid?)?;
        }
        Ok(total)
    }

    pub fn get_size(&self, uuid: Uuid) -> Result<u64> {
        Ok(self.get_update(uuid)?.metadata()?.len())
    }

    pub fn delete(&self, uuid: Uuid) -> Result<()> {
        let path = self.path.join(uuid.to_string());
        std::fs::remove_file(path)?;
        Ok(())
    }

    /// List the Uuids of the files in the FileStore
    pub fn all_uuids(&self) -> Result<impl Iterator<Item = Result<Uuid>>> {
        Ok(self.path.read_dir()?.map(|entry| {
            Ok(Uuid::from_str(
                entry?.file_name().to_str().ok_or(Error::CouldNotParseFileNameAsUtf8)?,
            )?)
        }))
    }
}

pub struct File {
    path: PathBuf,
    file: NamedTempFile,
}

impl File {
    pub fn persist(self) -> Result<()> {
        self.file.persist(&self.path)?;
        Ok(())
    }
}
