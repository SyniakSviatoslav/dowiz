//! Outbound notification, over the stack already in the graph.
//!
//! THE TRANSPORT LIVES IN `httpc`, not here. It started here, written onto the
//! TLS stream directly because the zero-dep allowlist admits no HTTP client
//! crate -- not even hyper's own client half, whose `client` feature pulls
//! `want` and `try-lock`; the gate refused that, which is what the gate is for.
//! When the AI assistant needed to call out as well, the request writer MOVED
//! rather than being copied: two copies are two places for a framing bug to
//! live, and they diverge the day one of them learns about timeouts.
//!
//! What stays here is what is specific to Telegram: the form encoding, the
//! HTML escaping, and knowing that an error body is worth keeping.
//!
//! WHY IT MATTERS AT ALL. Until now a customer placed an order and heard nothing
//! — the tracking page polls only while its tab is open — and an owner with a
//! closed tab missed the order entirely. That is not a missing feature, it is a
//! delivery service that silently drops its own work at night.
//!
//! FAILING TO NOTIFY NEVER FAILS AN ORDER. Every send returns its error to a
//! caller that logs it and moves on. An order already in the log must not be
//! rolled back because Telegram was slow.

use tokio_rustls::rustls::RootCertStore;

use crate::httpc::{self, urlencode};

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
}

impl Telegram {
    /// `None` when no token is set: the channel is simply off, and a hub with no
    /// bot is a valid hub rather than a broken one.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok()?;
        if token.trim().is_empty() {
            return None;
        }
        // Fail CLOSED at construction if this machine has no CA bundle: a
        // notifier that cannot verify a certificate must be absent, not
        // present-and-trusting.
        system_roots()?;
        Some(Telegram { token })
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
        // must therefore never be logged: the error below carries the response
        // body, never the request line.
        let url = format!("https://{HOST}/bot{}/sendMessage", self.token);
        let (code, raw) = httpc::request(
            "POST",
            &url,
            &[("content-type", "application/x-www-form-urlencoded")],
            body.as_bytes(),
            15_000,
            64 * 1024,
        )
        .await
        .map_err(|e| NotifyError::Transport(e.to_string()))?;

        if (200..300).contains(&code) {
            return Ok(());
        }
        // Telegram answers a rejection with JSON naming the reason; keeping it
        // is the difference between "the chat id is wrong" and "the user blocked
        // the bot", which need different human responses.
        Err(NotifyError::Rejected(format!(
            "{code}: {}",
            String::from_utf8_lossy(&raw).chars().take(300).collect::<String>()
        )))
    }
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
pub(crate) fn system_roots() -> Option<RootCertStore> {
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
