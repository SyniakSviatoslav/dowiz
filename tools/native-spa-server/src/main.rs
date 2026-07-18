//! `native-spa-server` binary — DK-04 entrypoint.
//!
//! Replaces the nginx container. Serves a static SPA directory over HTTP/1.1
//! by default, or HTTP/2 when a TLS certificate + key are supplied via
//! `--tls-cert` / `--tls-key`. Configuration is exclusively via CLI flags and
//! environment variables; there are NO secret reads and NO `.env` loading.
//!
//! NO-COURIER-SCORING: this is a genuine from-scratch static server, not a
//! scoring proxy. Behaviour is locked by RED tests in `tests/`.

use std::path::PathBuf;

use clap::Parser;
use native_spa_server::{api::ApiState, build_router, resolve_root, DEFAULT_PORT, DEFAULT_ROOT};

#[derive(Parser, Debug)]
#[command(
    name = "native-spa-server",
    about = "DK-04 native-Rust static SPA server (zero-OCI, replaces nginx).",
    version
)]
struct Cli {
    /// Static web root to serve (mirrors legacy nginx `root`).
    #[arg(long, env = "SPA_ROOT", default_value = DEFAULT_ROOT)]
    root: PathBuf,

    /// TCP port to bind (mirrors legacy nginx `listen 8080`).
    #[arg(long, env = "SPA_PORT", default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Bind address (default all interfaces).
    #[arg(long, env = "SPA_BIND", default_value = "0.0.0.0")]
    bind: String,

    /// Optional TLS certificate (PEM). When set, HTTP/2 is served.
    #[arg(long, env = "SPA_TLS_CERT")]
    tls_cert: Option<PathBuf>,

    /// Optional TLS private key (PEM). Required when `--tls-cert` is set.
    #[arg(long, env = "SPA_TLS_KEY")]
    tls_key: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let root = resolve_root(Some(cli.root.clone()));
    let api = ApiState::build_default();
    let router = build_router(&root, api);
    let addr = format!("{}:{}", cli.bind, cli.port);

    match (cli.tls_cert, cli.tls_key) {
        (Some(cert_path), Some(key_path)) => {
            serve_tls(router, &addr, &cert_path, &key_path).await?;
        }
        (None, None) => {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            eprintln!("[native-spa-server] HTTP/1.1 listening on http://{addr} (root={root})",
                addr = addr, root = root.display());
            axum::serve(listener, router).await?;
        }
        (Some(_), None) | (None, Some(_)) => {
            eprintln!("[native-spa-server] ERROR: --tls-cert and --tls-key must be set together");
            std::process::exit(2);
        }
    }
    Ok(())
}

/// HTTP/2 (h2) server: terminate TLS with rustls and serve over ALPN h2.
async fn serve_tls(
    router: axum::Router,
    addr: &str,
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    use tokio_rustls::rustls::{pki_types::PrivateKeyDer, ServerConfig};

    let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(
        std::fs::File::open(cert_path)?,
    ))
    .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut std::io::BufReader::new(
        std::fs::File::open(key_path)?,
    ))?
    .ok_or("no private key found in TLS key file")?;
    let key_der: PrivateKeyDer<'static> = key.into();

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key_der)?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!(
        "[native-spa-server] HTTP/2 (TLS) listening on https://{addr} (root)",
        addr = addr
    );

    loop {
        let (stream, _peer) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let router = router.clone();
        tokio::spawn(async move {
            let tls_stream = match tls_acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[native-spa-server] TLS accept error: {e}");
                    return;
                }
            };
            // Drive the TLS stream with the HTTP/1.1+HTTP/2 auto protocol
            // negotiator (ALPN h2/http1.1 set above). `TowerToHyperService`
            // adapts our axum `Router` (a `tower::Service`) to hyper's
            // connection API.
            let hyper_service =
                hyper_util::service::TowerToHyperService::new(router);
            if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                hyper_util::rt::TokioExecutor::new(),
            )
            .serve_connection_with_upgrades(
                hyper_util::rt::TokioIo::new(tls_stream),
                hyper_service,
            )
            .await
            {
                eprintln!("[native-spa-server] TLS conn error: {e}");
            }
        });
    }
}
