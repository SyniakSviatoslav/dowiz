//! THE LIVE SEAM (W-MG-M6b; feature `anchor-live`, default OFF): `Calendar` over HTTPS and the
//! block-explorer fetch, by shelling out to `curl`. The only code under `doc_seal` that reaches a
//! service, and the verifier never calls it.
//!
//! DECART (2026-10-31): `ureq 2` with `tls` (rustls + ring + webpki, ~70 crates, the heaviest
//! compile in this repo's graph; `agent-facade`'s `web-fetch` feature shows the shape) against
//! `curl`, already on this box and on every CI runner, ZERO crates. This is an OPERATOR tool run
//! by hand or from a cron on a box with curl -- not library code the Worker links (the Worker has
//! `fetch`) -- and the default build must stay what it was, so the lighter side wins. `ureq`
//! remains the answer the day a pure-Rust client is required; the seam does not change.
//! VERIFY the default graph is untouched: `cargo tree -e no-dev > a; cargo tree -e no-dev
//! --features anchor-live > b; diff a b` is empty (the feature pulls no crate).
//!
//! PROTOCOL (opentimestamps-client `calendar.py`): `POST <calendar>/digest`, body = the 32 raw
//! bytes, `Accept: application/vnd.opentimestamps.v1` -> 200 with a serialized timestamp whose
//! initial message is the digest; `GET <uri>/timestamp/<hex commitment>` -> 200 with a serialized
//! timestamp, 404 while the calendar has not committed (the body says so in words). Explorer:
//! `GET <api>/block-height/<h>` -> the block hash as text; `GET <api>/block/<hash>` -> JSON.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::doc_seal::ots::Calendar;

pub const ACCEPT: &str = "Accept: application/vnd.opentimestamps.v1";
pub const USER_AGENT: &str = "User-Agent: dowiz-anchor/1 (W-MG-M6b)";
/// The block explorer `fetch-headers` reads (mempool.space serves the same paths).
pub const EXPLORER: &str = "https://blockstream.info/api";

/// `Calendar` over `curl`; `timeout_s` bounds every call (law 1: bound every wait).
pub struct CurlCalendar {
    pub timeout_s: u32,
}

impl Default for CurlCalendar {
    fn default() -> Self {
        CurlCalendar { timeout_s: 30 }
    }
}

fn tmp(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dowiz-anchor-{}-{tag}", std::process::id()))
}

/// One `curl`: the status code and the body. A non-zero curl exit (DNS, TLS, timeout) is an
/// `Err` with curl's own words; an HTTP error is returned as its code for the caller to name.
fn http(timeout_s: u32, args: &[&str], out: &Path) -> Result<(u16, Vec<u8>), String> {
    let t = timeout_s.to_string();
    let o = Command::new("curl")
        .args(["-sS", "-m", &t, "-o", &out.display().to_string(), "-w", "%{http_code}", "-H", ACCEPT, "-H", USER_AGENT])
        .args(args)
        .output()
        .map_err(|e| format!("curl could not run: {e}"))?;
    let body = std::fs::read(out).unwrap_or_default();
    let _ = std::fs::remove_file(out);
    if !o.status.success() {
        return Err(format!("curl exit {:?}: {}", o.status.code(), String::from_utf8_lossy(&o.stderr).trim()));
    }
    let code: u16 = String::from_utf8_lossy(&o.stdout).trim().parse().map_err(|_| "curl printed no HTTP status".to_string())?;
    Ok((code, body))
}

fn words(body: &[u8]) -> String {
    String::from_utf8_lossy(&body[..body.len().min(200)]).trim().to_string()
}

impl Calendar for CurlCalendar {
    fn submit(&self, calendar: &str, digest: &[u8; 32]) -> Result<Vec<u8>, String> {
        let inp = tmp("digest");
        std::fs::write(&inp, digest).map_err(|e| format!("{}: {e}", inp.display()))?;
        let url = format!("{}/digest", calendar.trim_end_matches('/'));
        let data = format!("@{}", inp.display());
        let r = http(self.timeout_s, &["-X", "POST", "--data-binary", &data, &url], &tmp("reply"));
        let _ = std::fs::remove_file(&inp);
        let (code, body) = r?;
        if code != 200 {
            return Err(format!("{url}: HTTP {code}: {}", words(&body)));
        }
        if body.is_empty() {
            return Err(format!("{url}: HTTP 200 with an empty body"));
        }
        Ok(body)
    }

    fn upgrade(&self, uri: &str, msg: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let url = format!("{}/timestamp/{}", uri.trim_end_matches('/'), crate::crypto::hex(msg));
        let (code, body) = http(self.timeout_s, &[&url], &tmp("upgrade"))?;
        match code {
            200 if !body.is_empty() => Ok(Some(body)),
            404 => Ok(None),
            c => Err(format!("{url}: HTTP {c}: {}", words(&body))),
        }
    }
}

/// The explorer's block JSON for `height`: two GETs (height -> hash -> block).
pub fn fetch_block_json(explorer: &str, height: u64, timeout_s: u32) -> Result<String, String> {
    let api = explorer.trim_end_matches('/');
    let url = format!("{api}/block-height/{height}");
    let (code, body) = http(timeout_s, &[&url], &tmp("height"))?;
    if code != 200 {
        return Err(format!("{url}: HTTP {code}: {}", words(&body)));
    }
    let hash = String::from_utf8_lossy(&body).trim().to_string();
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{url}: not a block hash: {hash:?}"));
    }
    let url = format!("{api}/block/{hash}");
    let (code, body) = http(timeout_s, &[&url], &tmp("block"))?;
    if code != 200 {
        return Err(format!("{url}: HTTP {code}: {}", words(&body)));
    }
    String::from_utf8(body).map_err(|_| format!("{url}: the block JSON is not UTF-8"))
}
