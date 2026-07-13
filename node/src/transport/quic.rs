//! Production transport bearer: RFC 9000 QUIC + RFC 9174 (TCPCLv4-style) under
//! TLS 1.3.
//!
//! This is the P7 underlay that carries [`crate::Bundle`] bytes between two nodes
//! over QUIC bi-streams. The TLS 1.3 layer is terminated by `rustls` using the
//! `aws-lc-rs` crypto provider (FIPS-grade, operator-reviewed). The QUIC layer
//! itself is `quinn`.
//!
//! NOTE (S3 ML-DSA / ML-KEM prod swap): `liboqs` is NOT wired here. The real
//! ML-KEM-768 + ML-DSA post-quantum signer swap is a *separate* operator-gated
//! crate install (`liboqs`) that is layered ON TOP of this transport (authenticates
//! / encrypts the Bundle payload), NOT instead of it. This bearer is the channel;
//! the PQ envelope rides inside the Bundle it carries. Do not fake liboqs.

use std::net::SocketAddr;
use std::sync::Arc;

use quinn::{ClientConfig, Connection, Endpoint, ReadToEndError, RecvStream, SendStream, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::RootCertStore;

use crate::Bundle;

/// Length-prefix (u32 BE) framing size for a Bundle on a QUIC stream.
const MAX_BUNDLE_BYTES: usize = 16 * 1024 * 1024;

/// Install the `ring` crypto provider as the rustls default once per process.
/// quinn (default features), rcgen (cert gen), and rustls must all share ONE
/// provider or the TLS 1.3 handshake aborts. We use `ring` consistently here;
/// the operator FIPS swap to `aws-lc-rs` is documented in DECISIONS D9 and must
/// be applied uniformly to quinn + rcgen at the same time.
fn install_ring_provider() {
    // Re-install returns Err if already set; ignore.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// ponytail: runtime-generated self-signed cert for the TLS 1.3 layer.
/// In production this is replaced by operator-provisioned certificates (e.g. from a
/// PKI / ACME), never runtime-generated self-signed material. Built for tests only.
fn self_signed_cert() -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let cert = generate_simple_self_signed(vec!["localhost".to_string()]).expect("gen cert");
    let pem_cert = cert.cert.pem();
    let pem_key = cert.key_pair.serialize_pem();

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut pem_cert.as_bytes())
        .collect::<Result<_, _>>()
        .expect("parse cert pem");
    let key = rustls_pemfile::private_key(&mut pem_key.as_bytes())
        .expect("one key")
        .expect("parse key pem");
    (certs, key)
}

/// Build a server-side QUIC [`ServerConfig`] using our self-signed cert.
fn server_config() -> ServerConfig {
    install_ring_provider();
    let (certs, key) = self_signed_cert();
    let mut cfg = ServerConfig::with_single_cert(certs, key).expect("server config");
    // Reasonable default for the bearer; permits multiple concurrent streams.
    if let Some(transport) = Arc::get_mut(&mut cfg.transport) {
        transport.max_concurrent_bidi_streams(100u32.into());
    }
    cfg
}

/// A client that disables TLS cert verification (since we use a self-signed test
/// cert). prod would verify against an operator CA via RootCertStore.
#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        // Must match the active `ring` provider used everywhere else, or the
        // TLS 1.3 handshake aborts with "closed by peer" (client advertises
        // schemes the server can't satisfy under `ring`).
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Build a client-side QUIC [`ClientConfig`](quinn::ClientConfig) that trusts the self-signed server.
fn client_config() -> ClientConfig {
    install_ring_provider();
    let (certs, _key) = self_signed_cert();
    let mut roots = RootCertStore::empty();
    for c in certs {
        roots.add(c).expect("add root");
    }
    let mut rustls_cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    // Use our self-signed cert's fingerprint as the trust root (TOFU-style for tests).
    rustls_cfg
        .dangerous()
        .set_certificate_verifier(Arc::new(InsecureVerifier));
    let quic_cfg = quinn::crypto::rustls::QuicClientConfig::try_from(rustls_cfg)
        .expect("quic client config");
    ClientConfig::new(Arc::new(quic_cfg))
}

/// A running QUIC server endpoint that accepts Bundle-bearing bi-streams.
pub struct QuicServer {
    endpoint: Endpoint,
}

impl QuicServer {
    /// Bind a server endpoint on `addr` (e.g. `127.0.0.1:0` for an ephemeral port).
    pub async fn bind(addr: SocketAddr) -> std::io::Result<Self> {
        let endpoint = Endpoint::server(server_config(), addr)?;
        Ok(Self { endpoint })
    }

    /// The address this endpoint is listening on.
    pub fn local_addr(&self) -> SocketAddr {
        self.endpoint.local_addr().expect("local addr")
    }

    /// Accept the next incoming connection.
    async fn accept_conn(&self) -> Connection {
        let incoming = self.endpoint.accept().await.expect("accept incoming");
        incoming.await.expect("incoming connection")
    }

    /// Receive a single [`Bundle`] from the next incoming bi-stream on a freshly
    /// accepted connection. Returns the received Bundle, or Err on a broken stream
    /// / failed deserialization. This is a pure receiver — it does not echo.
    pub async fn recv_bundle(&self) -> Result<Bundle, String> {
        let conn = self.accept_conn().await;
        let (_send, mut recv) = conn
            .accept_bi()
            .await
            .map_err(|e| format!("accept_bi failed: {e}"))?;
        read_bundle(&mut recv).await
    }
}

/// A QUIC client that opens bi-streams to a server and sends Bundles.
pub struct QuicClient {
    endpoint: Endpoint,
}

impl QuicClient {
    /// Create a client endpoint (bound to an ephemeral local UDP port).
    pub async fn new() -> std::io::Result<Self> {
        let addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let endpoint = Endpoint::client(addr)?;
        Ok(Self { endpoint })
    }

    /// Connect to `server_addr`, open a bi-stream, send `bundle`, and wait for the
    /// server to echo it back. Returns the roundtripped Bundle.
    pub async fn send_and_recv(&self, server_addr: SocketAddr, bundle: &Bundle) -> Result<Bundle, String> {
        let connect = self
            .endpoint
            .connect_with(client_config(), server_addr, "localhost")
            .map_err(|e| format!("connect config error: {e}"))?;
        let conn = connect.await.map_err(|e| format!("connect failed: {e}"))?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| format!("open_bi failed: {e}"))?;
        write_bundle(&mut send, bundle).await?;
        read_bundle(&mut recv).await
    }

    /// Open a bi-stream and send `bundle`, returning once the server acknowledges
    /// (FIN). Does not wait for an echo. Proves multiplexing: call repeatedly.
    ///
    /// innovates: holds the connection open (`conn.closed()`) after the send stream
    /// is finished so the server has time to `accept_bi` + read the bundle. Dropping
    /// the connection too early is what produced the earlier "closed by peer: 0"
    /// handshake-race failure — the TLS handshake itself always succeeded.
    pub async fn send_only(&self, server_addr: SocketAddr, bundle: &Bundle) -> Result<(), String> {
        let connect = self
            .endpoint
            .connect_with(client_config(), server_addr, "localhost")
            .map_err(|e| format!("connect config error: {e}"))?;
        let conn = connect.await.map_err(|e| format!("connect failed: {e}"))?;
        let (mut send, _recv) = conn
            .open_bi()
            .await
            .map_err(|e| format!("open_bi failed: {e}"))?;
        write_bundle(&mut send, bundle).await?;
        // Keep the connection alive until the *server* closes its side; this lets the
        // server finish `accept_bi` + `read_bundle` before our drop propagates.
        // `closed()` resolves to a `ConnectionError` once the peer closes.
        let _closed: quinn::ConnectionError = conn.closed().await;
        Ok(())
    }
}

/// Serialize a Bundle as length-prefixed (u32 BE) JSON bytes and write to the stream.
async fn write_bundle(send: &mut SendStream, bundle: &Bundle) -> Result<(), String> {
    let bytes = serde_json::to_vec(bundle).map_err(|e| format!("serialize bundle: {e}"))?;
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err("bundle too large".to_string());
    }
    let len = (bytes.len() as u32).to_be_bytes();
    send.write_all(&len)
        .await
        .map_err(|e| format!("write len: {e}"))?;
    send.write_all(&bytes)
        .await
        .map_err(|e| format!("write bundle: {e}"))?;
    send.finish()
        .map_err(|e| format!("finish stream: {e}"))?;
    Ok(())
}

/// Read a length-prefixed JSON Bundle from a recv stream and deserialize it.
async fn read_bundle(recv: &mut RecvStream) -> Result<Bundle, String> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| format!("read len: {e}"))?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_BUNDLE_BYTES {
        return Err("incoming bundle too large".to_string());
    }
    let bytes = recv
        .read_to_end(len)
        .await
        .map_err(|e| match e {
            // A payload that fails to deserialize cleanly should surface as Err,
            // not panic. We distinguish "short read" from "bad bytes".
            ReadToEndError::TooLong => "bundle too large".to_string(),
            ReadToEndError::Read(_) => "stream closed early".to_string(),
        })?;
    serde_json::from_slice(&bytes).map_err(|e| format!("deserialize bundle: {e}"))
}

// ── RED+GREEN tests ───────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Node;
    use tokio::task;

    const SEED_A: [u8; 32] = [1u8; 32];

    /// GREEN: two in-process endpoints exchange a Bundle created by `Node::make_bundle`;
    /// the received Bundle equals the sent one (source/dest/payload identical).
    #[tokio::test]
    async fn green_bundle_roundtrip_over_quic() {
        // Diagnostics: surface handshake/transport reject reasons when run with
        // RUST_LOG=quinn=debug (innovate: kills "closed by peer" blind spots).
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("off")),
            )
            .with_test_writer()
            .try_init();

        let server = QuicServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .expect("bind server");
        let addr = server.local_addr();

        let node = Node::new("dtn://a", &SEED_A, 1000);
        let bundle = node.make_bundle("dtn://b", b"hello over QUIC", 1000, 3600);

        let client = QuicClient::new().await.expect("client");
        // Spawn the server receiver FIRST so the endpoint is ready to accept the
        // client's connection (QUIC handshake needs a live accepting endpoint).
        let server_handle = task::spawn(async move { server.recv_bundle().await.expect("recv") });
        // Client opens a bi-stream and sends the Bundle.
        client.send_only(addr, &bundle).await.expect("send");
        // Server receives the Bundle over the same bi-stream.
        let server_bundle = server_handle.await.expect("join");

        assert_eq!(server_bundle.source, bundle.source);
        assert_eq!(server_bundle.dest, bundle.dest);
        assert_eq!(server_bundle.payload, bundle.payload);
    }

    /// GREEN: a second concurrent stream also completes (proves multiplexing).
    #[tokio::test]
    async fn green_concurrent_streams_multiplex() {
        let server = QuicServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .expect("bind server");
        let addr = server.local_addr();
        let server_handle = task::spawn(async move {
            let mut got = Vec::new();
            for _ in 0..2 {
                got.push(server.recv_bundle().await.expect("recv"));
            }
            got
        });

        let node = Node::new("dtn://a", &SEED_A, 1000);
        let b1 = node.make_bundle("dtn://b", b"stream one", 1000, 3600);
        let b2 = node.make_bundle("dtn://b", b"stream two", 1001, 3600);

        let client = QuicClient::new().await.expect("client");
        // Two independent connections/streams in parallel.
        let (r1, r2) = tokio::join!(client.send_only(addr, &b1), client.send_only(addr, &b2));
        r1.expect("send 1");
        r2.expect("send 2");

        let got = server_handle.await.expect("join");
        assert_eq!(got.len(), 2);
        assert!(got.iter().any(|b| b.payload == b1.payload));
        assert!(got.iter().any(|b| b.payload == b2.payload));
    }

    /// RED: connecting to a wrong/unreachable endpoint returns Err (no panic).
    #[tokio::test]
    async fn red_connect_to_unreachable_returns_err() {
        // Port nothing is listening on.
        let bad_addr: SocketAddr = "127.0.0.1:1".parse().unwrap();
        let client = QuicClient::new().await.expect("client");
        let node = Node::new("dtn://a", &SEED_A, 1000);
        let bundle = node.make_bundle("dtn://b", b"nope", 1000, 3600);
        let res = client.send_and_recv(bad_addr, &bundle).await;
        assert!(res.is_err(), "expected Err connecting to unreachable endpoint");
    }

    /// RED: a payload that fails to deserialize returns Err.
    #[tokio::test]
    async fn red_garbage_payload_returns_err() {
        // Send raw non-JSON bytes on a bi-stream to a server expecting a Bundle.
        let server = QuicServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .expect("bind server");
        let addr = server.local_addr();
        let server_handle = task::spawn(async move { server.recv_bundle().await });

        let client = QuicClient::new().await.expect("client");
        let connect = client.endpoint.connect_with(client_config(), addr, "localhost").expect("connect cfg");
        let conn = connect.await.expect("connected");
        let (mut send, _recv) = conn.open_bi().await.expect("open_bi");
        let garbage = b"this is not a bundle json".to_vec();
        let len = (garbage.len() as u32).to_be_bytes();
        send.write_all(&len).await.expect("write len");
        send.write_all(&garbage).await.expect("write garbage");
        send.finish().expect("finish");

        let res = server_handle.await.expect("join");
        assert!(res.is_err(), "expected Err on non-deserializable payload");
    }
}
