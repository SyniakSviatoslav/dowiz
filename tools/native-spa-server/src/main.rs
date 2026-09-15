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
use native_spa_server::{
    api::ApiState, build_router, resolve_root, serve_with_timeout,
    webhook::WebhookState, DEFAULT_HEADER_READ_TIMEOUT,
    DEFAULT_PORT, DEFAULT_ROOT,
};

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
    /// Where this hub keeps its store. Absent => this binary is a static server
    /// and nothing else, which is what it was before the hub moved onto it.
    #[arg(long, env = "HUB_DIR")]
    hub_dir: Option<PathBuf>,

    /// Seed the hub's catalogue from a bundle and exit. This is how a hub gets
    /// its menu: `--hub-dir <dir> --seed-catalog bundle.json`.
    #[arg(long)]
    seed_catalog: Option<PathBuf>,

    /// Provision a hub on a VPS and expose it through a tunnel, then exit:
    /// `--provision dubin-sushi --hostname dubin.example.com`.
    ///
    /// Needs `HETZNER_API_TOKEN`, `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`
    /// and, to point a name at it, `CLOUDFLARE_ZONE_ID`. Refuses by name
    /// otherwise -- see `p67`.
    #[arg(long)]
    provision: Option<String>,

    /// The hostname the venue will be reached on. Without it the tunnel is
    /// created and configured but nothing is pointed at it.
    #[arg(long)]
    hostname: Option<String>,

    /// Where the hub binary is fetched from on first boot.
    #[arg(long, env = "HUB_BINARY_URL")]
    hub_binary_url: Option<String>,

    /// Report what exists without changing anything.
    #[arg(long)]
    provision_status: bool,

    /// Add a person to the hub's roster and exit:
    /// `--hub-dir <dir> --add-person owner:ana@dubin.al:Ana`.
    ///
    /// The password comes from `PERSON_PASSWORD`, or is generated and printed
    /// once. Deliberately NOT an HTTP route -- see where it is handled.
    #[arg(long)]
    add_person: Option<String>,

    #[arg(long, env = "SPA_TLS_KEY")]
    tls_key: Option<PathBuf>,
}

/// `role:identifier:name`, e.g. `owner:ana@dubin.al:Ana` or
/// `courier:+355691234567:Eni`.
///
/// The identifier is what the person types to log in, so it is lowercased here
/// once rather than at every comparison later.
fn parse_person_spec(spec: &str) -> std::io::Result<(String, dowiz_hub::token::Role, String)> {
    let mut parts = spec.splitn(3, ':');
    let role = parts.next().unwrap_or_default();
    let id = parts.next().unwrap_or_default().trim().to_ascii_lowercase();
    let name = parts.next().unwrap_or("").trim().to_string();
    let role = dowiz_hub::token::Role::from_str(role)
        .filter(|r| matches!(r, dowiz_hub::token::Role::Owner | dowiz_hub::token::Role::Courier))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "role must be `owner` or `courier`",
            )
        })?;
    if id.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--add-person needs role:identifier[:name]",
        ));
    }
    let name = if name.is_empty() { id.clone() } else { name };
    Ok((id, role, name))
}

/// What already exists, changing nothing.
///
/// A read-only command first, because the first thing an operator wants from a
/// provisioning tool is to be told what is already there -- and because a tool
/// whose only mode is "create" gets run twice.
async fn provision_status() -> Result<(), Box<dyn std::error::Error>> {
    use native_spa_server::p67::{CloudflareTunnel, Hetzner};
    let mut any = false;
    match Hetzner::from_env() {
        Ok(h) => match h.list_hubs().await {
            Ok(list) if list.is_empty() => eprintln!("[p67] hetzner: no dowiz-labelled servers"),
            Ok(list) => {
                any = true;
                for (id, name, status) in list {
                    eprintln!("[p67] hetzner: {id}  {name}  {status}");
                }
            }
            Err(e) => eprintln!("[p67] hetzner: {e}"),
        },
        Err(e) => eprintln!("[p67] hetzner: {e}"),
    }
    match CloudflareTunnel::from_env() {
        Ok(cf) => match cf.count_tunnels().await {
            // The 1,000-tunnel cap is an account limit, so the gauge is
            // reported against it rather than as a bare number.
            Ok(n) => {
                any = true;
                eprintln!("[p67] cloudflare: {n} live tunnels (cap 1000)");
            }
            Err(e) => eprintln!("[p67] cloudflare: {e}"),
        },
        Err(e) => eprintln!("[p67] cloudflare: {e}"),
    }
    if !any {
        eprintln!("[p67] nothing reachable; set the credentials named above");
    }
    Ok(())
}

/// Create a machine, a tunnel, and the route between them.
///
/// ORDER MATTERS AND IS NOT ARBITRARY. The tunnel is created FIRST, because its
/// connector token has to be inside the cloud-init the server boots with. A
/// server created first would come up with nowhere to connect and would need a
/// second pass over SSH -- which is the manual step this exists to remove.
///
/// It is also the order that fails cheapest: a tunnel with no server costs
/// nothing and is deleted in one call, while a server with no tunnel is a
/// machine being billed for that nobody can reach.
async fn provision(
    name: &str,
    hostname: Option<&str>,
    binary_url: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use native_spa_server::p67::{cloud_init, CloudflareTunnel, Hetzner, ServerSpec};

    let fail = |e: String| -> Box<dyn std::error::Error> { Box::new(std::io::Error::other(e)) };
    let binary_url = binary_url.ok_or_else(|| {
        fail("--hub-binary-url (or HUB_BINARY_URL) must say where the hub binary is fetched from".into())
    })?;

    let hetzner = Hetzner::from_env().map_err(|e| fail(e.to_string()))?;
    let cf = CloudflareTunnel::from_env().map_err(|e| fail(e.to_string()))?;

    eprintln!("[p67] creating tunnel {name}");
    let (tunnel_id, connector) = cf.create_tunnel(name).await.map_err(|e| fail(e.to_string()))?;
    eprintln!("[p67] tunnel {tunnel_id}");

    // From here on a failure must CLEAN UP the tunnel, or the account collects
    // orphans that count against the 1,000 cap and that nobody remembers
    // creating.
    let result = async {
        if let Some(host) = hostname {
            cf.configure_ingress(&tunnel_id, host, "http://127.0.0.1:8080")
                .await
                .map_err(|e| e.to_string())?;
            let rec = cf.route_dns(host, &tunnel_id).await.map_err(|e| e.to_string())?;
            eprintln!("[p67] {host} -> {tunnel_id}.cfargotunnel.com (dns {rec})");
        }
        let spec = ServerSpec {
            name: format!("dowiz-{name}"),
            user_data: Some(cloud_init(binary_url, &connector, 8080)),
            ..Default::default()
        };
        let id = hetzner.create_server(&spec).await.map_err(|e| e.to_string())?;
        Ok::<i64, String>(id)
    }
    .await;

    match result {
        Ok(id) => {
            eprintln!("[p67] server {id} created; it boots, installs the hub and dials the tunnel");
            eprintln!("[p67] next: --hub-dir on that box, --seed-catalog, --add-person owner:…");
            Ok(())
        }
        Err(e) => {
            eprintln!("[p67] FAILED: {e}");
            eprintln!("[p67] removing the tunnel so it does not count against the cap");
            if let Err(e2) = cf.destroy_tunnel(&tunnel_id).await {
                // Said loudly rather than swallowed: an orphan the operator does
                // not know about is worse than one they do.
                eprintln!("[p67] could NOT remove tunnel {tunnel_id}: {e2} -- remove it by hand");
            }
            Err(fail(e))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let root = resolve_root(Some(cli.root.clone()));
    let api = ApiState::build_default();
    if cli.provision_status {
        return provision_status().await;
    }
    if let Some(name) = &cli.provision {
        return provision(name, cli.hostname.as_deref(), cli.hub_binary_url.as_deref()).await;
    }

    // Creating a person is a SEPARATE INVOCATION, not a route. A hub with no
    // people must not expose a "create the first owner" endpoint: that endpoint
    // is unauthenticated by definition, and whoever reaches the box first owns
    // the restaurant. Requiring shell access to the VPS is the authorisation.
    if let (Some(dir), Some(spec)) = (&cli.hub_dir, &cli.add_person) {
        let (id, role, name) = parse_person_spec(spec)?;
        let password = match std::env::var("PERSON_PASSWORD") {
            Ok(p) if !p.is_empty() => p,
            _ => {
                // Generated rather than prompted: a password typed on a command
                // line ends up in the shell history and in `ps`.
                let bytes = dowiz_hub::crypto::random_bytes(12)?;
                let generated = dowiz_hub::crypto::b64url_encode(&bytes);
                eprintln!("[hub] generated password for {id}: {generated}");
                eprintln!("[hub] it is shown ONCE and is not recoverable from the roster.");
                generated
            }
        };
        let st = native_spa_server::hub::HubState::open(dir)?;
        st.add_person(&id, role, &name, &password).await?;
        eprintln!("[hub] {} {} added as {}", role.as_str(), id, name);
        return Ok(());
    }
    if let (Some(dir), Some(bundle)) = (&cli.hub_dir, &cli.seed_catalog) {
        let n = native_spa_server::hub::seed_catalog(dir, bundle)?;
        eprintln!("[hub] seeded {} categories and {} products into {}", n.0, n.1, dir.display());
        return Ok(());
    }

    let hub_state = match &cli.hub_dir {
        Some(dir) => {
            let st = native_spa_server::hub::HubState::open(dir)?;
            eprintln!("[hub] store at {}", dir.display());
            Some(st)
        }
        None => None,
    };
    // The webhook is built AFTER the hub, because it now hands inbound messages
    // to it. Phase 1: one hub per process. In production the per-hub BotFather
    // token → secret_token mapping comes from hub config.
    let webhook_state = std::sync::Arc::new(WebhookState {
        telegram: std::sync::Arc::new(
            intake_adapters::telegram::TelegramAdapter::new(
                std::env::var("DOWIZ_TELEGRAM_SECRET").unwrap_or_default(),
            ),
        ),
        intake: std::sync::Arc::new(
            dowiz_kernel::ports::hub_intake::IntakeService::new(vec![]),
        ),
        hub: hub_state.clone(),
    });
    let router = build_router(&root, api, webhook_state, hub_state);
    let addr = format!("{}:{}", cli.bind, cli.port);

    match (cli.tls_cert, cli.tls_key) {
        (Some(cert_path), Some(key_path)) => {
            serve_tls(router, &addr, &cert_path, &key_path).await?;
        }
        (None, None) => {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            eprintln!("[native-spa-server] HTTP/1.1 listening on http://{addr} (root={root}), \
                header-read-timeout={t:?}",
                addr = addr, root = root.display(), t = DEFAULT_HEADER_READ_TIMEOUT);
            serve_with_timeout(listener, router, DEFAULT_HEADER_READ_TIMEOUT).await?;
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
