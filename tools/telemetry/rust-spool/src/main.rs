//! telemetry-spool — background Telegram drainer.
//!
//! ## Why this exists (operator mandate: system speed = slowest part)
//! The old `tg_send` was called SYNCHRONOUSLY on the agent's work critical path
//! behind a 3.5s/msg throttle. 3 messages = 7.4s, serializing the agent's own
//! work behind the network. That made the *network* the slowest part of the
//! whole system.
//!
//! The fix: the work path only appends one line of JSON to a spool file
//! (`~/.cache/telemetry-spool/queue.jsonl`) — microseconds, never blocks. A
//! single long-lived instance of THIS binary drains the spool, sending each
//! message to Telegram via pure-Rust HTTP (ureq) and honouring the kernel's
//! pacing contract (3.5s gap). The agent runs at kernel speed; the network
//! catches up in the background.
//!
//! ## Wire protocol (one JSON object per spool line)
//! `{ "chat_id": "-100...", "topic_id": 257, "text": "..." }`
//! `topic_id` may be omitted (defaults to the Hermes topic 267).
//! A line is removed from the spool only after Telegram returns ok:true, so a
//! crash mid-drain loses nothing (the line stays and is retried next pass).

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Default Telegram topic (Hermes). Mirrors lib.sh default.
const DEFAULT_TOPIC_ID: i64 = 267;
/// Pacing gap in seconds. MUST match hermes-kernel `reporting::TG_MIN_GAP_S`.
const TG_MIN_GAP_S: f64 = 3.5;
/// Poll interval when the spool is empty (seconds).
const IDLE_POLL_S: u64 = 2;

#[derive(Debug, Clone, Deserialize)]
struct SpoolEntry {
    chat_id: String,
    #[serde(default = "default_topic")]
    topic_id: i64,
    text: String,
}

fn default_topic() -> i64 {
    DEFAULT_TOPIC_ID
}

#[derive(Debug, Serialize, Deserialize)]
struct TgResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
}

fn spool_path() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push("telemetry-spool");
    std::fs::create_dir_all(&p).ok();
    p.push("queue.jsonl");
    p
}

/// Now as fractional unix seconds (drainer-side clock; the pure pace logic
/// lives in the kernel, but the drainer supplies the wall clock here).
fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Read all pending spool entries (best-effort; returns what it can parse).
fn read_spool(path: &PathBuf) -> Vec<(usize, SpoolEntry)> {
    let mut out = Vec::new();
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return out,
    };
    for (i, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(e) = serde_json::from_str::<SpoolEntry>(line) {
            out.push((i, e));
        }
    }
    out
}

/// Remove a specific line index from the spool (the one we just sent ok).
/// Rewrites the file minus that line. Robust: if the file vanished, no-op.
fn drop_line(path: &PathBuf, idx: usize) {
    let data = match std::fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return,
    };
    let kept: Vec<&str> = data
        .lines()
        .enumerate()
        .filter(|(i, l)| !l.trim().is_empty() && *i != idx)
        .map(|(_, l)| l)
        .collect();
    let mut tmp = path.clone();
    tmp.set_extension("jsonl.tmp");
    if let Ok(mut f) = std::fs::File::create(&tmp) {
        // Join with newlines and add a trailing newline ONLY when non-empty, so
        // an emptied spool becomes a 0-byte file (not a lone "\n" artifact).
        if !kept.is_empty() {
            let _ = writeln!(f, "{}", kept.join("\n"));
        }
        let _ = f.flush();
        let _ = std::fs::rename(&tmp, path);
    }
}

/// Send one message via the Bot API (pure-Rust ureq). Returns true on ok:true.
fn send_one(bot_token: &str, e: &SpoolEntry) -> bool {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let body = serde_json::json!({
        "chat_id": e.chat_id,
        "text": e.text,
        "disable_web_page_preview": true,
        "message_thread_id": e.topic_id,
    });
    for attempt in 1..=4 {
        let resp = ureq::post(&url)
            .timeout(Duration::from_secs(10))
            .send_json(body.clone());
        let parsed: Result<TgResponse, ureq::Error> =
            resp.and_then(|r| Ok(r.into_json::<TgResponse>()?));
        match parsed {
            Ok(TgResponse { ok: true, .. }) => return true,
            Ok(r) => {
                // telegram returned ok:false
                eprintln!("spool: telegram rejected: {:?}", r.description);
                if attempt >= 4 {
                    return false;
                }
                std::thread::sleep(Duration::from_secs(2 * attempt as u64));
            }
            Err(_) => {
                if attempt >= 4 {
                    return false;
                }
                std::thread::sleep(Duration::from_secs(2 * attempt as u64));
            }
        }
    }
    false
}

fn main() {
    let bot_token = match std::env::var("TELEGRAM_BOT_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            // try to read from dowiz/.env without echoing the token
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
            eprintln!("spool: TELEGRAM_BOT_TOKEN not set; exiting");
            std::process::exit(1);
        }
    };
    run(bot_token);
}

fn run(bot_token: String) {
    let path = spool_path();
    eprintln!(
        "spool: draining {} (pace {}s, idle poll {}s)",
        path.display(),
        TG_MIN_GAP_S,
        IDLE_POLL_S
    );
    let mut last_send: Option<f64> = None;
    loop {
        let entries = read_spool(&path);
        if entries.is_empty() {
            std::thread::sleep(Duration::from_secs(IDLE_POLL_S));
            continue;
        }
        // honour the kernel pacing contract: don't send before last+gap
        let now = now_secs();
        if let Some(t) = last_send {
            let earliest = t + TG_MIN_GAP_S;
            if earliest > now {
                let wait = (earliest - now).max(0.0);
                std::thread::sleep(Duration::from_secs_f64(wait));
            }
        }
        // drain in order; stop the batch if pacing would be violated (next loop
        // iteration re-checks). We send one, then the loop recomputes the gap.
        let (idx, entry) = &entries[0];
        if send_one(&bot_token, entry) {
            drop_line(&path, *idx);
            last_send = Some(now_secs());
        } else {
            // transient failure: back off, retry next loop (don't drop the line)
            std::thread::sleep(Duration::from_secs(2));
        }
    }
}
