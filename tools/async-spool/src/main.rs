//! async-spool — generic JSONL spool drainer adapter (pure-Rust rustls/ring).
//!
//! ## Why this exists
//! This is the generalized successor to `tools/telemetry/rust-spool`. That
//! binary was Telegram-only: the producer appended
//! `{ "chat_id", "topic_id", "text" }` lines and the drainer POSTed them to the
//! Bot API. Here we generalize the *same proven* read→pace→send→drop-on-ok→
//! deadletter-on-error contract to an arbitrary destination field:
//!
//! Every spool line is a JSON object with a `"dest"` discriminator plus a
//! payload:
//!   * `dest: "telegram"` → POST to api.telegram.org/bot<TOKEN>/sendMessage
//!     (body shape reuses the telemetry-spool contract: chat_id, text,
//!      message_thread_id).
//!   * `dest: "http"`      → POST to the `url` field with `body` (JSON).
//!
//! ## Delivery contract (inherits the telemetry guarantees)
//! * The producer's work path only WRITES one JSON line (microseconds) — it
//!   never blocks on the network.
//! * A line is removed from the spool ONLY after the send returns success, so a
//!   crash mid-drain loses nothing (the line stays and is retried next pass).
//! * A line that cannot be parsed / has an unknown `dest` / is missing required
//!   fields is never silently lost: it is appended to a `<spool>.deadletter`
//!   file and removed from the live queue, so it neither loops forever nor
//!   vanishes.
//! * Pacing gap (default 3.5s) is honoured between sends, read from
//!   `ASYNC_SPOOL_GAP_S` (float).
//! * TLS is pure-Rust rustls + webpki-roots (ring) via ureq. NO OpenSSL, NO
//!   native-tls, NO curl.

use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Default pacing gap in seconds between two sends. MUST match hermes-kernel
/// `reporting::TG_MIN_GAP_S`
/// (`hermes-agent-kernel-rewrite/hermes-kernel/kernel/src/reporting.rs:26`).
/// Single record of this cross-repo decision:
/// `docs/design/hermetic-architecture-2026-07-16/PACING-CONTRACT.md`.
/// Env-overridable via `ASYNC_SPOOL_GAP_S`; the default is pinned by
/// `default_gap_self_pin` (+ optional present-sibling test).
const DEFAULT_GAP_S: f64 = 3.5;
/// Poll interval when the spool is empty (seconds).
const IDLE_POLL_S: u64 = 2;
/// Send/connect timeout per attempt.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);
/// Max attempts per send before giving up (the line stays queued & retries
/// next pass — never deadlettered for a transient delivery failure).
const MAX_ATTEMPTS: u32 = 4;

/// A line that the dispatcher knows how to deliver.
#[derive(Debug, Clone)]
enum Sendable {
    Telegram {
        chat_id: String,
        text: String,
        topic_id: i64,
    },
    Http {
        url: String,
        body: serde_json::Value,
    },
}

/// Classification of one raw spool line.
enum LineClass {
    /// Parsed and dispatchable.
    Sendable(Sendable),
    /// Unparseable / unknown dest / missing required field. Carries the raw
    /// line so it can be preserved in the deadletter file.
    Dead(String),
}

#[derive(Debug, Deserialize)]
struct RawLine {
    dest: Option<String>,
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

/// Default Telegram topic (Hermes). Mirrors telemetry-spool default.
const DEFAULT_TOPIC_ID: i64 = 267;

/// Read the gap from env (float seconds); fall back to the default.
fn gap_seconds() -> f64 {
    match std::env::var("ASYNC_SPOOL_GAP_S") {
        Ok(v) => v.trim().parse::<f64>().unwrap_or(DEFAULT_GAP_S),
        Err(_) => DEFAULT_GAP_S,
    }
    .max(0.0)
}

fn spool_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push("async-spool");
    std::fs::create_dir_all(&p).ok();
    p
}

fn queue_path() -> PathBuf {
    let mut p = spool_dir();
    p.push("queue.jsonl");
    p
}

fn deadletter_path() -> PathBuf {
    let mut p = spool_dir();
    p.push("queue.deadletter");
    p
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Parse one raw line into a [`LineClass`].
fn classify(raw: &str) -> LineClass {
    let parsed: RawLine = match serde_json::from_str(raw) {
        Ok(p) => p,
        Err(_) => return LineClass::Dead(raw.to_string()),
    };
    let dest = match parsed.dest {
        Some(d) => d,
        None => return LineClass::Dead(raw.to_string()),
    };
    match dest.as_str() {
        "telegram" => {
            let chat_id = parsed
                .rest
                .get("chat_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let text = parsed
                .rest
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let topic_id = parsed
                .rest
                .get("message_thread_id")
                .and_then(|v| v.as_i64())
                .unwrap_or(DEFAULT_TOPIC_ID);
            match (chat_id, text) {
                (Some(chat_id), Some(text)) => {
                    LineClass::Sendable(Sendable::Telegram { chat_id, text, topic_id })
                }
                _ => LineClass::Dead(raw.to_string()),
            }
        }
        "http" => {
            let url = parsed
                .rest
                .get("url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let body = parsed.rest.get("body").cloned().unwrap_or(
                serde_json::Value::Object(parsed.rest.clone()),
            );
            match url {
                Some(url) => LineClass::Sendable(Sendable::Http { url, body }),
                None => LineClass::Dead(raw.to_string()),
            }
        }
        _ => LineClass::Dead(raw.to_string()),
    }
}

/// Read the queue. Returns each non-empty line with its index and classification.
fn read_spool(path: &PathBuf) -> Vec<(usize, LineClass)> {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    data.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, classify(l.trim())))
        .collect()
}

/// Append raw lines to the deadletter file (preserves everything, never loses).
fn deadletter(raws: &[String]) {
    if raws.is_empty() {
        return;
    }
    let path = deadletter_path();
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        for r in raws {
            let _ = writeln!(f, "{}", r);
        }
        let _ = f.flush();
    }
}

/// Rewrite the queue file dropping the given line indices (the sent line + any
/// deadletter lines). Robust: if the file vanished, no-op.
fn drop_lines(path: &PathBuf, drop_idx: &[usize]) {
    let drop: std::collections::BTreeSet<usize> = drop_idx.iter().copied().collect();
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let kept: Vec<&str> = data
        .lines()
        .enumerate()
        .filter(|(i, l)| !l.trim().is_empty() && !drop.contains(i))
        .map(|(_, l)| l)
        .collect();
    let mut tmp = path.clone();
    tmp.set_extension("jsonl.tmp");
    if let Ok(mut f) = std::fs::File::create(&tmp) {
        if !kept.is_empty() {
            let _ = writeln!(f, "{}", kept.join("\n"));
        }
        let _ = f.flush();
        let _ = std::fs::rename(&tmp, path);
    }
}

#[derive(Debug, Deserialize)]
struct TgResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
}

/// Retry backoff delay for send `attempt` (1-based): exponential base-2 s with
/// **full jitter** (`2s · 2^(attempt-1)`, capped at 32 s, `× rand[0.5,1.0)`).
///
/// ADR (H2 §2.5 / finding #24): jitter breaks spool synchronization — N drainers
/// hitting a downed endpoint no longer retry in lock-step (thundering herd). The
/// helper is DUPLICATED in `telemetry-spool` (the two spools share no lib crate;
/// a `spool-common` path dep is heavier than warranted for one fn) — the
/// duplication is a conscious, accepted trade. RNG is a dep-free splitmix64 seed
/// off the wall clock (non-crypto is sufficient for jitter). The terminal
/// fixed-2s transient pause in `run` and the idle poll are deliberately left
/// un-jittered (single-drainer deployment).
fn backoff_delay(attempt: u32) -> Duration {
    const BASE_MS: u64 = 2_000;
    const CAP_MS: u64 = 32_000;
    let shift = attempt.saturating_sub(1).min(20);
    let exp_ms = BASE_MS.saturating_mul(1u64 << shift).min(CAP_MS);
    Duration::from_millis((exp_ms as f64 * jitter_unit()) as u64)
}

/// Dep-free jitter factor in `[0.5, 1.0)` from a wall-clock-seeded splitmix64.
/// Non-crypto: jitter only needs to be uncorrelated across processes.
fn jitter_unit() -> f64 {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let unit = (z >> 11) as f64 / (1u64 << 53) as f64; // [0.0, 1.0)
    0.5 + 0.5 * unit // [0.5, 1.0)
}

/// Deliver one sendable. Returns true on success (line should be dropped).
fn deliver(bot_token: &str, line: &Sendable) -> bool {
    match line {
        Sendable::Telegram { chat_id, text, topic_id } => {
            let url = format!(
                "https://api.telegram.org/bot{}/sendMessage",
                bot_token
            );
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "disable_web_page_preview": true,
                "message_thread_id": topic_id,
            });
            for attempt in 1..=MAX_ATTEMPTS {
                let resp = ureq::post(&url)
                    .timeout(SEND_TIMEOUT)
                    .send_json(body.clone());
                let parsed: Result<TgResponse, ureq::Error> =
                    resp.and_then(|r| Ok(r.into_json::<TgResponse>()?));
                match parsed {
                    Ok(TgResponse { ok: true, .. }) => return true,
                    Ok(r) => {
                        eprintln!("async-spool: telegram rejected: {:?}", r.description);
                        if attempt >= MAX_ATTEMPTS {
                            return false;
                        }
                        std::thread::sleep(backoff_delay(attempt));
                    }
                    Err(_) => {
                        if attempt >= MAX_ATTEMPTS {
                            return false;
                        }
                        std::thread::sleep(backoff_delay(attempt));
                    }
                }
            }
            false
        }
        Sendable::Http { url, body } => {
            for attempt in 1..=MAX_ATTEMPTS {
                let resp = ureq::post(url).timeout(SEND_TIMEOUT).send_json(body.clone());
                match resp {
                    Ok(_) => return true,
                    Err(e) => {
                        eprintln!("async-spool: http send failed ({}): {:?}", url, e);
                        if attempt >= MAX_ATTEMPTS {
                            return false;
                        }
                        std::thread::sleep(backoff_delay(attempt));
                    }
                }
            }
            false
        }
    }
}

fn run(bot_token: String) {
    let path = queue_path();
    let gap = gap_seconds();
    eprintln!(
        "async-spool: draining {} (pace {}s, idle poll {}s)",
        path.display(),
        gap,
        IDLE_POLL_S
    );
    let mut last_send: Option<f64> = None;
    loop {
        let entries = read_spool(&path);
        if entries.is_empty() {
            std::thread::sleep(Duration::from_secs(IDLE_POLL_S));
            continue;
        }

        // Partition: deadletter lines (bad) keep their original index + raw
        // text; sendable lines keep their index + parsed payload.
        let mut dead: Vec<(usize, String)> = Vec::new();
        let mut sendable: Vec<(usize, Sendable)> = Vec::new();
        for (idx, cls) in entries {
            match cls {
                LineClass::Dead(raw) => dead.push((idx, raw)),
                LineClass::Sendable(s) => sendable.push((idx, s)),
            }
        }

        // Preserve unparseable/undispatchable lines. They leave the live queue
        // (so they don't loop forever) but are NEVER lost — appended verbatim
        // to the deadletter file with their original index prefix.
        let mut to_drop: Vec<usize> = Vec::new();
        if !dead.is_empty() {
            let raws: Vec<String> = dead.iter().map(|(i, r)| format!("{}:{}", i, r)).collect();
            deadletter(&raws);
            to_drop.extend(dead.into_iter().map(|(i, _)| i));
        }

        // Honour the pacing contract before sending the first sendable entry.
        if let Some((idx, line)) = sendable.into_iter().next() {
            let now = now_secs();
            if let Some(t) = last_send {
                let earliest = t + gap;
                if earliest > now {
                    std::thread::sleep(Duration::from_secs_f64((earliest - now).max(0.0)));
                }
            }
            if deliver(&bot_token, &line) {
                to_drop.push(idx);
                last_send = Some(now_secs());
            } else {
                // transient delivery failure: keep the line, back off, retry.
                std::thread::sleep(Duration::from_secs(2));
            }
        }

        // Drop everything that should leave the live queue this pass
        // (deadletter lines first, then any successfully sent line).
        if !to_drop.is_empty() {
            drop_lines(&path, &to_drop);
        }
    }
}

fn main() {
    let bot_token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            // Try /root/dowiz/.env without echoing the token (best-effort).
            let env_path = PathBuf::from("/root/dowiz/.env");
            if let Ok(contents) = std::fs::read_to_string(&env_path) {
                for line in contents.lines() {
                    if let Some(v) = line.strip_prefix("TELEGRAM_BOT_TOKEN=") {
                        let v = v
                            .trim()
                            .trim_matches(|c| c == '"' || c == '\'' || c == '\r');
                        if !v.is_empty() {
                            return run(v.to_string());
                        }
                    }
                }
            }
            eprintln!("async-spool: TELEGRAM_BOT_TOKEN not set; exiting");
            std::process::exit(1);
        }
    };
    run(bot_token);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Cross-repo pacing pin (finding #18). The authority lives in a SIBLING repo
    // (hermes-kernel `reporting::TG_MIN_GAP_S`), which a dowiz test cannot
    // compile-time reference. This self-assertion makes any local edit to the
    // default break a test, forcing a conscious re-verify against
    // PACING-CONTRACT.md rather than a silent drift. See
    // docs/design/hermetic-architecture-2026-07-16/PACING-CONTRACT.md.
    #[test]
    fn default_gap_self_pin() {
        assert_eq!(DEFAULT_GAP_S, 3.5, "pacing gap desynced from PACING-CONTRACT");
    }

    // Optional stronger tier: when the hermes-kernel sibling repo happens to be
    // checked out alongside dowiz, assert our value matches the live authority.
    // SKIPS cleanly (passes) when the sibling is absent so a lone dowiz checkout
    // never false-fails.
    #[test]
    fn default_gap_matches_present_sibling() {
        let sibling =
            "/root/hermes-agent-kernel-rewrite/hermes-kernel/kernel/src/reporting.rs";
        let src = match std::fs::read_to_string(sibling) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("skip: hermes-kernel sibling absent ({sibling})");
                return;
            }
        };
        match parse_const_f64(&src, "TG_MIN_GAP_S") {
            Some(v) => assert_eq!(
                v, DEFAULT_GAP_S,
                "async-spool DEFAULT_GAP_S ({DEFAULT_GAP_S}) != hermes-kernel ({v})"
            ),
            None => eprintln!("skip: TG_MIN_GAP_S not found in {sibling} (format changed?)"),
        }
    }

    // Retry backoff carries jitter and stays within the exponential envelope:
    // delay(attempt) ∈ [exp/2, exp) where exp = min(2s·2^(attempt-1), 32s).
    #[test]
    fn backoff_delay_is_jittered_within_envelope() {
        for attempt in 1u32..=8 {
            let shift = attempt.saturating_sub(1).min(20);
            let exp_ms = 2_000u64.saturating_mul(1u64 << shift).min(32_000);
            for _ in 0..64 {
                let ms = backoff_delay(attempt).as_millis() as u64;
                assert!(
                    ms >= exp_ms / 2 && ms < exp_ms,
                    "attempt {attempt}: delay {ms}ms outside [{}, {})",
                    exp_ms / 2,
                    exp_ms
                );
            }
        }
    }

    // Parse `... <name> ... = <f64> ;` from Rust source (test-only helper for the
    // present-sibling pin). Returns None if no such line exists.
    fn parse_const_f64(src: &str, name: &str) -> Option<f64> {
        for line in src.lines() {
            if line.contains(name) && line.contains('=') {
                let rhs = line.split('=').nth(1)?;
                let val = rhs.trim().trim_end_matches(';').trim();
                if let Ok(v) = val.parse::<f64>() {
                    return Some(v);
                }
            }
        }
        None
    }
}
