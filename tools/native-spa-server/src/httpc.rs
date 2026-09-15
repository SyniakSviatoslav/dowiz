//! The one outbound HTTP client, over the stack already in the graph.
//!
//! WHY IT EXISTS. Two things now call out of this process: the Telegram
//! notifier and the AI assistant. The notifier grew its own request writer
//! first; a second copy for the assistant would be a second place for a framing
//! bug to live, and they would diverge the day one of them learned about
//! redirects or timeouts. So there is one.
//!
//! WHY IT IS HAND-WRITTEN. The zero-dep allowlist may only SHRINK, and every
//! HTTP client crate is outside it — including hyper's own client half, whose
//! `client` feature pulls `want` and `try-lock`. The gate refused that, which is
//! what the gate is for. `tokio` and `tokio-rustls` are already here to LISTEN;
//! this makes them DIAL.
//!
//! PLAIN HTTP IS SUPPORTED, and that is not a weakness to apologise for: the
//! whole point of the local-AI path is a model on `127.0.0.1:11434` that never
//! leaves the machine. Requiring TLS to talk to a loopback socket would add
//! certificate ceremony to a connection that cannot be intercepted, while
//! blocking the one arrangement where the venue's data provably does not leave
//! their own hardware. TLS is required for everything else — see `is_local`.

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

#[derive(Debug)]
pub enum HttpError {
    /// The URL could not be understood.
    BadUrl(String),
    /// Plain HTTP to a host that is not loopback — refused, not downgraded.
    InsecureRemote(String),
    /// No usable trust anchors on this machine.
    NoTrustAnchors,
    Transport(String),
    /// The peer answered, and said no. Carries the status and the body, because
    /// "invalid api key" and "model not found" need different human responses
    /// and collapsing them hides which happened.
    Status(u16, String),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::BadUrl(u) => write!(f, "bad url: {u}"),
            HttpError::InsecureRemote(h) => {
                write!(f, "refusing plain http to {h}; use https for anything but loopback")
            }
            HttpError::NoTrustAnchors => write!(f, "no CA bundle on this machine"),
            HttpError::Transport(e) => write!(f, "transport: {e}"),
            HttpError::Status(c, b) => write!(f, "http {c}: {b}"),
        }
    }
}

pub struct Url {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub fn parse_url(raw: &str) -> Result<Url, HttpError> {
    let bad = || HttpError::BadUrl(raw.to_string());
    let (scheme, rest) = raw.split_once("://").ok_or_else(bad)?;
    let tls = match scheme {
        "https" => true,
        "http" => false,
        _ => return Err(bad()),
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    // Credentials in a URL would end up in a log line the moment anything went
    // wrong. Refuse them rather than carry them.
    if authority.contains('@') {
        return Err(bad());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            (h, p.parse().map_err(|_| bad())?)
        }
        _ => (authority, if tls { 443u16 } else { 80u16 }),
    };
    if host.is_empty() {
        return Err(bad());
    }
    Ok(Url { tls, host: host.to_string(), port, path: path.to_string() })
}

/// Is this host on this machine?
///
/// Only these get to skip TLS. An unencrypted request to anything else would put
/// an API token on the wire in clear, and "the operator asked for it" is not a
/// defence when the token is the venue's.
///
/// THE PREFIX TEST THAT WAS HERE FIRST WAS A HOLE, and the test below is what
/// found it: `starts_with("127.")` also matches `127.0.0.1.evil.com`, a name
/// anyone can register, pointing anywhere. So the 127/8 check PARSES the host as
/// four decimal octets and matches only if the whole string is one — a name with
/// anything after the address is a name, not an address.
pub fn is_local(host: &str) -> bool {
    let h = host.trim_start_matches('[').trim_end_matches(']');
    if h == "localhost" || h == "::1" {
        return true;
    }
    is_loopback_v4(h)
}

/// Exactly four decimal octets, first one 127, nothing else in the string.
fn is_loopback_v4(h: &str) -> bool {
    let mut parts = h.split('.');
    let mut octets = [0u16; 4];
    for slot in &mut octets {
        let Some(p) = parts.next() else { return false };
        if p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        match p.parse::<u16>() {
            Ok(v) if v <= 255 => *slot = v,
            _ => return false,
        }
    }
    // A fifth component means this is a hostname that merely begins with an
    // address, which is the whole bug.
    parts.next().is_none() && octets[0] == 127
}

fn tls_connector() -> Result<TlsConnector, HttpError> {
    let roots = crate::notify::system_roots().ok_or(HttpError::NoTrustAnchors)?;
    let cfg = ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(cfg)))
}

/// One request, one response, connection closed.
///
/// `Connection: close` with the body read to EOF is what removes chunked
/// transfer-encoding and Content-Length framing from the problem entirely. For
/// a handful of calls a minute there is nothing to gain from keep-alive, and
/// every framing branch not written is a framing bug not written.
///
/// `timeout_ms` is mandatory rather than optional. A local model can take tens
/// of seconds to answer and a dead socket can take forever; a caller that
/// forgets to bound the wait hangs a request handler, so there is no way to
/// forget.
pub async fn request(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    timeout_ms: u64,
    max_response: usize,
) -> Result<(u16, Vec<u8>), HttpError> {
    let u = parse_url(url)?;
    if !u.tls && !is_local(&u.host) {
        return Err(HttpError::InsecureRemote(u.host));
    }

    let mut head = format!("{method} {} HTTP/1.1\r\nHost: {}", u.path, u.host);
    if (u.tls && u.port != 443) || (!u.tls && u.port != 80) {
        head.push_str(&format!(":{}", u.port));
    }
    head.push_str("\r\nConnection: close\r\n");
    for (k, v) in headers {
        // A header value containing CRLF would let a caller inject headers, or
        // a whole second request, into this one.
        if v.contains('\r') || v.contains('\n') || k.contains(':') {
            return Err(HttpError::BadUrl(format!("illegal header {k}")));
        }
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));

    let work = async {
        let stream = tokio::net::TcpStream::connect((u.host.as_str(), u.port))
            .await
            .map_err(|e| HttpError::Transport(e.to_string()))?;
        let raw = if u.tls {
            let dns = ServerName::try_from(u.host.clone())
                .map_err(|e| HttpError::Transport(e.to_string()))?;
            let mut s = tls_connector()?
                .connect(dns, stream)
                .await
                .map_err(|e| HttpError::Transport(e.to_string()))?;
            write_and_read(&mut s, head.as_bytes(), body, max_response).await?
        } else {
            let mut s = stream;
            write_and_read(&mut s, head.as_bytes(), body, max_response).await?
        };
        Ok::<_, HttpError>(raw)
    };

    let raw = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), work)
        .await
        .map_err(|_| HttpError::Transport(format!("timed out after {timeout_ms}ms")))??;

    parse_response(&raw)
}

async fn write_and_read<S>(
    s: &mut S,
    head: &[u8],
    body: &[u8],
    max_response: usize,
) -> Result<Vec<u8>, HttpError>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    s.write_all(head).await.map_err(|e| HttpError::Transport(e.to_string()))?;
    if !body.is_empty() {
        s.write_all(body).await.map_err(|e| HttpError::Transport(e.to_string()))?;
    }
    s.flush().await.map_err(|e| HttpError::Transport(e.to_string()))?;

    let mut out = Vec::with_capacity(4096);
    let mut buf = [0u8; 8192];
    loop {
        // STOP AT CONTENT-LENGTH rather than waiting for the close. Reading to
        // EOF works, but it makes every response depend on how politely the
        // peer shuts down -- and one of them will not be polite.
        if let Some(end) = complete_at(&out) {
            out.truncate(end);
            return Ok(out);
        }
        match s.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                out.extend_from_slice(&buf[..n]);
                // A peer that floods must not grow this without limit.
                if out.len() > max_response {
                    break;
                }
            }
            // AN ABRUPT CLOSE IS NOT A LOST RESPONSE. rustls reports a peer that
            // closes without `close_notify` as an error, and python's
            // http.server -- and plenty of real servers -- do exactly that. The
            // first version of this discarded a complete, valid, already-read
            // HTTP response because of how the connection ended, which showed up
            // as "could not reach the model" for a model that had answered.
            // Truncation attacks are what close_notify guards against; that
            // matters when the length is unknown, so this only forgives the EOF
            // once a parseable response has arrived.
            Err(e) => {
                if parse_response(&out).is_ok() {
                    return Ok(out);
                }
                return Err(HttpError::Transport(e.to_string()));
            }
        }
    }
    Ok(out)
}

/// The byte length of a complete response, if `raw` holds one.
///
/// Only the `Content-Length` case: a chunked response has no length to compute
/// without decoding it, and those still fall through to the close. Nothing here
/// speaks to a peer that sends neither.
fn complete_at(raw: &[u8]) -> Option<usize> {
    let head_end = raw.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let head = String::from_utf8_lossy(&raw[..head_end]).to_ascii_lowercase();
    if head.contains("transfer-encoding:") {
        return None;
    }
    let at = head.find("\r\ncontent-length:")? + "\r\ncontent-length:".len();
    let len: usize = head[at..]
        .lines()
        .next()?
        .trim()
        .parse()
        .ok()?;
    let want = head_end.checked_add(len)?;
    (raw.len() >= want).then_some(want)
}

/// Split a response into its status and body.
pub fn parse_response(raw: &[u8]) -> Result<(u16, Vec<u8>), HttpError> {
    let head_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| HttpError::Transport("truncated response: no header terminator".into()))?;
    let status_line = raw[..head_end].split(|&b| b == b'\n').next().unwrap_or(&[]);
    let status_line = String::from_utf8_lossy(status_line);
    let code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| HttpError::Transport(format!("unparsable status line: {status_line:?}")))?;
    Ok((code, raw[head_end + 4..].to_vec()))
}

/// Percent-encode a value for a form body or a query string.
pub fn urlencode(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_parse() {
        let u = parse_url("https://api.telegram.org/bot123/sendMessage").unwrap();
        assert!(u.tls);
        assert_eq!(u.host, "api.telegram.org");
        assert_eq!(u.port, 443);
        assert_eq!(u.path, "/bot123/sendMessage");

        let u = parse_url("http://127.0.0.1:11434/v1/chat/completions").unwrap();
        assert!(!u.tls);
        assert_eq!(u.host, "127.0.0.1");
        assert_eq!(u.port, 11434);
        assert_eq!(u.path, "/v1/chat/completions");

        // No path means the root, not an empty request line.
        assert_eq!(parse_url("https://example.com").unwrap().path, "/");
        assert_eq!(parse_url("http://example.com").unwrap().port, 80);
    }

    #[test]
    fn bad_urls_are_refused() {
        for u in ["", "example.com", "ftp://example.com", "https://", "https://:443/x"] {
            assert!(parse_url(u).is_err(), "accepted {u:?}");
        }
        // Credentials in a URL end up in the first log line that mentions it.
        assert!(parse_url("https://user:pw@example.com/x").is_err());
    }

    #[test]
    fn loopback_is_recognised_and_nothing_else_is() {
        for h in ["localhost", "127.0.0.1", "127.1.2.3", "::1", "[::1]"] {
            assert!(is_local(h), "{h} should be local");
        }
        // Every one of these is a name or address an attacker can control or
        // point elsewhere. `127.0.0.1.evil.com` is the one that a `starts_with`
        // check lets through, and it is registrable.
        for h in [
            "example.com",
            "10.0.0.1",
            "0.0.0.0",
            "notlocalhost",
            "127.0.0.1.evil.com",
            "127.0.0.1.",
            "127.0.0.1x",
            "1270.0.0.1",
            "127.0.0.256",
            "127.0.0",
            "localhost.evil.com",
            "",
        ] {
            assert!(!is_local(h), "{h} must NOT be treated as local");
        }
    }

    /// The rule that keeps an API token off the wire in clear.
    #[tokio::test]
    async fn plain_http_to_a_remote_host_is_refused_before_connecting() {
        let e = request("POST", "http://example.com/v1/x", &[], b"{}", 1000, 4096)
            .await
            .expect_err("must refuse");
        assert!(matches!(e, HttpError::InsecureRemote(_)), "{e}");
    }

    /// A header value with a newline in it could inject a second request.
    #[tokio::test]
    async fn header_injection_is_refused() {
        let e = request(
            "POST",
            "http://127.0.0.1:1/x",
            &[("authorization", "Bearer x\r\nX-Evil: 1")],
            b"",
            1000,
            4096,
        )
        .await
        .expect_err("must refuse");
        assert!(matches!(e, HttpError::BadUrl(_)), "{e}");
    }

    /// The framing that stops a response depending on a polite shutdown.
    #[test]
    fn a_content_length_response_is_complete_before_the_close() {
        let full = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        assert_eq!(complete_at(full), Some(full.len()));
        // Not yet: two of the five body bytes have arrived.
        assert_eq!(complete_at(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhe"), None);
        // Headers not finished.
        assert_eq!(complete_at(b"HTTP/1.1 200 OK\r\nContent-Len"), None);
        // Anything trailing the body is not part of it.
        let extra = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhelloXXXX";
        assert_eq!(complete_at(extra), Some(extra.len() - 4));
        // Case must not matter -- header names are case-insensitive and real
        // servers disagree about how to spell this one.
        assert!(complete_at(b"HTTP/1.1 200 OK\r\ncontent-length: 5\r\n\r\nhello").is_some());
        // A chunked response has no length to compute; it falls through.
        assert_eq!(
            complete_at(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello"),
            None
        );
        assert_eq!(complete_at(b"HTTP/1.1 204 No Content\r\n\r\n"), None);
    }

    #[test]
    fn responses_split_into_status_and_body() {
        let (c, b) = parse_response(b"HTTP/1.1 200 OK\r\nX: 1\r\n\r\n{\"ok\":true}").unwrap();
        assert_eq!(c, 200);
        assert_eq!(b, b"{\"ok\":true}");
        let (c, _) = parse_response(b"HTTP/1.1 404 Not Found\r\n\r\n").unwrap();
        assert_eq!(c, 404);
        assert!(parse_response(b"HTTP/1.1 200 OK\r\nX: 1").is_err(), "truncated");
        assert!(parse_response(b"").is_err());
    }

    /// A caller cannot forget the timeout, so a dead peer cannot hang a handler.
    #[tokio::test]
    async fn a_dead_peer_times_out_rather_than_hanging() {
        // Port 1 on loopback refuses or blackholes; either way this must return.
        let started = std::time::Instant::now();
        let r = request("POST", "http://127.0.0.1:1/x", &[], b"", 300, 4096).await;
        assert!(r.is_err());
        assert!(started.elapsed().as_secs() < 5, "took {:?}", started.elapsed());
    }

    #[test]
    fn form_encoding_survives_non_ascii() {
        assert_eq!(urlencode("a b"), "a%20b");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencode("Durrës"), "Durr%C3%ABs");
    }
}
