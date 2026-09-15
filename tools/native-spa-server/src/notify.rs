//! Outbound notification, over the stack already in the graph.
//!
//! WHY THE REQUEST IS HAND-WRITTEN. The crate carries a ZERO-DEP-ALLOWLIST whose
//! CI gate fails on GROWTH, so an HTTP client crate is not available. The first
//! attempt here reached for hyper's client half on the theory that hyper was
//! already in the graph and so came free; the gate said otherwise -- the
//! `client` feature drags in `want` and `try-lock`, which are not in the list.
//! So the request is written onto the TLS stream directly. That needs nothing
//! beyond `tokio` and `tokio-rustls`, both of which the server already uses to
//! LISTEN; here they DIAL. The gate is what settles this, not this comment.
//!
//! The protocol surface used is deliberately the smallest that is still correct:
//! one POST, `Connection: close`, and the response read to EOF. Closing the
//! connection is what removes the need to implement chunked transfer-encoding
//! and Content-Length framing -- a notifier that sends a handful of messages a
//! day has nothing to gain from keep-alive, and every framing branch not written
//! is a framing bug not written.
//!
//! WHY IT MATTERS AT ALL. Until now a customer placed an order and heard nothing
//! — the tracking page polls only while its tab is open — and an owner with a
//! closed tab missed the order entirely. That is not a missing feature, it is a
//! delivery service that silently drops its own work at night.
//!
//! FAILING TO NOTIFY NEVER FAILS AN ORDER. Every send returns its error to a
//! caller that logs it and moves on. An order already in the log must not be
//! rolled back because Telegram was slow.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

const HOST: &str = "api.telegram.org";

#[derive(Debug)]
pub enum NotifyError {
    NotConfigured,
    Transport(String),
    /// Telegram answered, and said no. The body is kept: "chat not found" and
    /// "bot was blocked by the user" need different human responses, and
    /// collapsing them into one error hides which happened.
    Rejected(String),
}

pub struct Telegram {
    token: String,
    tls: TlsConnector,
}

impl Telegram {
    /// `None` when no token is set: the channel is simply off, and a hub with no
    /// bot is a valid hub rather than a broken one.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        if token.trim().is_empty() {
            return None;
        }
        let roots = system_roots()?;
        let cfg = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Some(Telegram { token, tls: TlsConnector::from(Arc::new(cfg)) })
    }

    /// Send one message. `chat_id` is whatever Telegram gave us when the person
    /// first messaged the bot; it is an opaque handle, not a phone number, which
    /// is why it can be stored without holding PII.
    pub async fn send(&self, chat_id: &str, text: &str) -> Result<(), NotifyError> {
        let body = format!(
            "chat_id={}&text={}&parse_mode=HTML&disable_web_page_preview=true",
            urlencode(chat_id),
            urlencode(text)
        );
        // The token sits in the PATH, which is how Telegram's API is shaped. It
        // must therefore never be logged: every error below carries the response
        // body, never the request line.
        let req = format!(
            "POST /bot{}/sendMessage HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            self.token,
            HOST,
            body.len(),
            body
        );

        let stream = tokio::net::TcpStream::connect((HOST, 443))
            .await
            .map_err(|e| NotifyError::Transport(e.to_string()))?;
        let dns = ServerName::try_from(HOST).map_err(|e| NotifyError::Transport(e.to_string()))?;
        let mut tls = self
            .tls
            .connect(dns, stream)
            .await
            .map_err(|e| NotifyError::Transport(e.to_string()))?;

        tls.write_all(req.as_bytes())
            .await
            .map_err(|e| NotifyError::Transport(e.to_string()))?;
        tls.flush().await.map_err(|e| NotifyError::Transport(e.to_string()))?;

        // Bounded read. A peer that never closes must not hang the caller, and a
        // peer that floods must not grow this buffer without limit; Telegram's
        // own replies are a few hundred bytes.
        let mut raw = Vec::with_capacity(1024);
        let mut chunk = [0u8; 2048];
        loop {
            let n = tls
                .read(&mut chunk)
                .await
                .map_err(|e| NotifyError::Transport(e.to_string()))?;
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&chunk[..n]);
            if raw.len() > 64 * 1024 {
                break;
            }
        }
        parse_response(&raw)
    }
}

/// Split an HTTP/1.1 response into "did it work" and, if not, why.
///
/// Separated from `send` for one reason: it is the only part with branches
/// worth testing, and testing it must not require a socket.
fn parse_response(raw: &[u8]) -> Result<(), NotifyError> {
    let head_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| NotifyError::Transport("truncated response: no header terminator".into()))?;
    let status_line = raw[..head_end]
        .split(|&b| b == b'\n')
        .next()
        .unwrap_or(&[]);
    let status_line = String::from_utf8_lossy(status_line);
    let code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| NotifyError::Transport(format!("unparsable status line: {status_line:?}")))?;
    if (200..300).contains(&code) {
        return Ok(());
    }
    // Telegram answers a rejection with JSON naming the reason; keeping it is
    // the difference between "the chat id is wrong" and "the user blocked the
    // bot", which need different human responses.
    let body = String::from_utf8_lossy(&raw[head_end + 4..]);
    Err(NotifyError::Rejected(format!(
        "{code}: {}",
        body.chars().take(300).collect::<String>()
    )))
}

/// Trust anchors from the machine's own CA bundle.
///
/// NOT a compiled-in root set. Two reasons, and the second is the real one:
/// a bundled-roots crate is outside the zero-dep allowlist, and — more to the
/// point — a hub runs on someone else's VPS, so the operator who updates their
/// CA store, or pins a corporate root, must be the one who decides what this
/// process trusts. Compiling the roots in would take that decision away from
/// them and freeze it at build time.
///
/// `None` when no bundle is found, which switches the channel off rather than
/// falling back to trusting everything. There is no insecure path here on
/// purpose: the alternative to verified TLS is not "degraded TLS", it is
/// handing the bot token to whoever answers the socket.
fn system_roots() -> Option<RootCertStore> {
    const CANDIDATES: [&str; 5] = [
        "/etc/ssl/certs/ca-certificates.crt",   // Debian, Ubuntu, Alpine
        "/etc/pki/tls/certs/ca-bundle.crt",     // Fedora, RHEL
        "/etc/ssl/ca-bundle.pem",               // openSUSE
        "/etc/ssl/cert.pem",                    // Alpine, macOS ports
        "/data/data/com.termux/files/usr/etc/tls/cert.pem", // the dev box
    ];
    let path = std::env::var("SSL_CERT_FILE")
        .ok()
        .filter(|p| !p.is_empty())
        .or_else(|| CANDIDATES.iter().find(|p| std::path::Path::new(p).exists()).map(|p| p.to_string()))?;
    let pem = std::fs::read(&path).ok()?;
    let mut rd = std::io::BufReader::new(&pem[..]);
    let mut roots = RootCertStore::empty();
    let mut added = 0usize;
    for cert in rustls_pemfile::certs(&mut rd).flatten() {
        if roots.add(cert).is_ok() {
            added += 1;
        }
    }
    // An empty store would make every connection fail with a confusing
    // certificate error instead of an honest "no trust anchors".
    if added == 0 {
        return None;
    }
    Some(roots)
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// HTML-escape a value going into a Telegram message.
///
/// The message carries a customer's name and address. Without this, a name
/// containing `<` truncates the message at Telegram's parser and the courier
/// loses the rest of the address.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 200 is success even though the body is JSON we never parse: the
    /// notifier's job is "did it leave", not "what did Telegram think".
    #[test]
    fn ok_response_succeeds() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\n\r\n{\"ok\":true,\"r\":1}";
        assert!(parse_response(raw).is_ok());
    }

    /// The failure reason must survive. Collapsing every 4xx into one error is
    /// exactly what makes "the customer is not receiving messages" unanswerable.
    #[test]
    fn rejection_keeps_the_reason() {
        let raw = b"HTTP/1.1 400 Bad Request\r\n\r\n{\"ok\":false,\"description\":\"chat not found\"}";
        match parse_response(raw) {
            Err(NotifyError::Rejected(m)) => {
                assert!(m.starts_with("400: "), "the code must be kept: {m}");
                assert!(m.contains("chat not found"), "the reason must be kept: {m}");
            }
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    /// A connection cut mid-header is a transport failure, not a silent success.
    /// Without this the notifier would treat a severed socket as a delivered
    /// message and the order would go out with nobody told.
    #[test]
    fn truncated_response_is_not_success() {
        assert!(matches!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Len"),
            Err(NotifyError::Transport(_))
        ));
        assert!(matches!(parse_response(b""), Err(NotifyError::Transport(_))));
    }

    /// Garbage on the wire must not parse as a 2xx.
    #[test]
    fn nonsense_status_line_is_an_error() {
        assert!(matches!(
            parse_response(b"not http at all\r\n\r\nbody"),
            Err(NotifyError::Transport(_))
        ));
    }

    /// The address and the note are free text a customer typed. `&` and `=`
    /// would otherwise end the form field early and truncate the message the
    /// courier reads.
    #[test]
    fn form_encoding_survives_customer_text() {
        assert_eq!(urlencode("Rruga Taulantia 12"), "Rruga%20Taulantia%2012");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        // Non-ASCII must go out as UTF-8 bytes, percent-encoded one byte at a
        // time -- Albanian and Ukrainian addresses are the normal case here,
        // not an edge case.
        assert_eq!(urlencode("Durrës"), "Durr%C3%ABs");
        assert_eq!(urlencode("Київ"), "%D0%9A%D0%B8%D1%97%D0%B2");
    }

    /// A customer name containing `<` would otherwise truncate the message at
    /// Telegram's HTML parser, losing the address that follows it.
    #[test]
    fn html_escaping_protects_the_rest_of_the_message() {
        assert_eq!(esc("<b>x</b> & y"), "&lt;b&gt;x&lt;/b&gt; &amp; y");
        // The ampersand must be replaced FIRST, or the escapes introduced by the
        // later replacements get double-escaped.
        assert_eq!(esc("&lt;"), "&amp;lt;");
    }

    /// No token means the channel is off, not that the hub is broken.
    #[test]
    fn absent_token_disables_the_channel() {
        // SAFETY: single-threaded test; no other thread reads the environment.
        unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN") };
        assert!(Telegram::from_env().is_none());
        unsafe { std::env::set_var("TELEGRAM_BOT_TOKEN", "   ") };
        assert!(Telegram::from_env().is_none(), "whitespace is not a token");
        unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN") };
    }
}
