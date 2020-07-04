use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use heed::EnvOpenOptions;
use serde::Deserialize;
use structopt::StructOpt;
use warp::{Filter, http::Response};

use mega_mini_indexer::{BEU32, Index};

#[cfg(target_os = "linux")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Debug, StructOpt)]
#[structopt(name = "mmi", about = "The server side of the mmi project.")]
struct Opt {
    /// The database path where the LMDB database is located.
    /// It is created if it doesn't already exist.
    #[structopt(long = "db", parse(from_os_str))]
    database: PathBuf,

    /// The maximum size the database can take on disk. It is recommended to specify
    /// the whole disk space (value must be a multiple of a page size).
    #[structopt(long = "db-size", default_value = "107374182400")] // 100 GB
    database_size: usize,

    /// The ip and port on which the database will listen for HTTP requests.
    #[structopt(short = "l", long, default_value = "127.0.0.1:9700")]
    http_listen_addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    std::fs::create_dir_all(&opt.database)?;
    let env = EnvOpenOptions::new()
        .map_size(opt.database_size)
        .max_dbs(10)
        .open(&opt.database)?;

    let index = Index::new(&env)?;

    // We run and wait on the HTTP server

    // Expose an HTML page to debug the search in a browser
    let dash_html_route = warp::filters::method::get()
        .and(warp::filters::path::end())
        .map(|| warp::reply::html(include_str!("../../public/index.html")));

    let dash_bulma_route = warp::filters::method::get()
        .and(warp::path!("bulma.min.css"))
        .map(|| Response::builder()
            .header("content-type", "text/css; charset=utf-8")
            .body(include_str!("../../public/bulma.min.css"))
        );

    let dash_jquery_route = warp::filters::method::get()
        .and(warp::path!("jquery-3.4.1.min.js"))
        .map(|| Response::builder()
            .header("content-type", "application/javascript; charset=utf-8")
            .body(include_str!("../../public/jquery-3.4.1.min.js"))
        );

    let dash_papaparse_route = warp::filters::method::get()
        .and(warp::path!("papaparse.min.js"))
        .map(|| Response::builder()
            .header("content-type", "application/javascript; charset=utf-8")
            .body(include_str!("../../public/papaparse.min.js"))
        );

    #[derive(Deserialize)]
    struct QueryBody {
        query: String,
    }

    let env_cloned = env.clone();
    let query_route = warp::filters::method::post()
        .and(warp::path!("query"))
        .and(warp::body::json())
        .map(move |query: QueryBody| {
            let before_search = Instant::now();
            let rtxn = env_cloned.read_txn().unwrap();

            let documents_ids = index.search(&rtxn, &query.query).unwrap();

            let mut body = Vec::new();
            if let Some(headers) = index.headers(&rtxn).unwrap() {
                // We write the headers
                body.extend_from_slice(headers);

                for id in documents_ids {
                    let content = index.documents.get(&rtxn, &BEU32::new(id)).unwrap();
                    let content = content.expect(&format!("could not find document {}", id));
                    body.extend_from_slice(&content);
                }
            }

            Response::builder()
                .header("Content-Type", "text/csv")
                .header("Time-Ms", before_search.elapsed().as_millis().to_string())
                .body(String::from_utf8(body).unwrap())
        });

    let routes = dash_html_route
        .or(dash_bulma_route)
        .or(dash_jquery_route)
        .or(dash_papaparse_route)
        .or(query_route);

    let addr = SocketAddr::from_str(&opt.http_listen_addr).unwrap();
    eprintln!("listening on http://{}", addr);
    warp::serve(routes).run(addr).await;

    Ok(())
}