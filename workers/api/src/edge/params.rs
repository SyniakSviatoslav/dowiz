//! THE ROUTER HANDS OVER PATH PARAMETERS STILL PERCENT-ENCODED (`worker` 0.8's matcher does not
//! decode), so `/api/owner/supplies/нори/uses` -- which a browser sends as `%D0%BD...` -- reached
//! its handler as the escaped text and answered 404 "unknown supply" (dubin-sushi, 2026-10-17:
//! 45 of its 63 supplies had ids like that). [`Ctx::from_worker`](super::Ctx::from_worker)
//! decodes every parameter in [`NAMES`] once, so no handler sees an escape.

/// Every `:name` a route in `lib.rs` declares (a test keeps this complete).
pub const NAMES: &[&str] = &["code", "id", "key", "kind", "n", "name", "peer", "sitting", "slug", "token", "zone"];

/// `raw` with its `%XX` escapes decoded, when they decode to UTF-8; otherwise `raw` unchanged.
pub fn decoded(raw: &str) -> String {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
        match (b[i], b.get(i + 1).and_then(|&c| hex(c)), b.get(i + 2).and_then(|&c| hex(c))) {
            (b'%', Some(hi), Some(lo)) => {
                out.push(hi << 4 | lo);
                i += 3;
            }
            (c, _, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

#[cfg(test)]
mod tests;
