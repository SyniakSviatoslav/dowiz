//! Entry point: tokio runtime, SQLite open, axum bind.
//!
//! The SQLite file lives at `./data/dowiz.sqlite` (relative to CWD) by default.
//! The `data/` directory is created if missing. Pass `DOWIZ_DB` to override the
//! path. `web/dist` (relative to CWD) is served as the SPA.

use std::path::PathBuf;

use dowiz_server::routes::{build_router, AppState};
use dowiz_server::store::Store;

#[tokio::main]
async fn main() {
    let db_path: PathBuf = std::env::var("DOWIZ_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/dowiz.sqlite"));

    let store = match Store::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "FATAL: could not open SQLite at {}: {}",
                db_path.display(),
                e
            );
            std::process::exit(1);
        }
    };
    let store = std::sync::Arc::new(store);

    let dist_dir: PathBuf = std::env::var("DOWIZ_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("web/dist"));

    let state = AppState { store };
    let app = build_router(state, dist_dir);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("dowiz-server listening on http://{}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("FATAL: could not bind {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("FATAL: server error: {}", e);
        std::process::exit(1);
    }
}
