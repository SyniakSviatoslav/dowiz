//! topics — native port of tools/telemetry/topics.sh.
//!
//! Dedicated Telegram topic aggregators. Reuses the same primitives the
//! shell version used (git log, plan.jsonl, roadmap doc, eval-layer result)
//! and posts to topic message threads via the Telegram Bot API (ureq).
//! No python subprocess, no langchain/typer/rich.
//!
//! Topic ids (mirrors topics.sh):
//!   267 Hermes (default; DOD plan/step/retro + monitor)
//!   291 Planning (unified plans/tasks/roadmaps, last 7d)
//!   292 Git      (commits/pushes, both repos)
//!   294 Benchmarks (entropy/eval/bench results)
//!
//! Env: TELEGRAM_BOT_TOKEN, TELEGRAM_CHAT_ID, TELEGRAM_TOPIC_ID (per-call override).

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const DOWIZ: &str = "/root/dowiz";
const BEBOP: &str = "/root/bebop-repo";
const BEBOP_REF: &str = "openbebop/main";
const ROADMAP: &str =
    "/root/dowiz/docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md";

fn log_dir() -> PathBuf {
    let d = env::var("LOG_DIR").unwrap_or_else(|_| "/root/ops/topics".to_string());
    fs::create_dir_all(&d).ok();
    PathBuf::from(d)
}

// ── Telegram client ──────────────────────────────────────────────────────────────
struct Tg {
    token: String,
    chat: String,
    topic: String,
}

impl Tg {
    fn from_env(topic_override: &str) -> Option<Tg> {
        let token = env::var("TELEGRAM_BOT_TOKEN").ok()?;
        let chat = env::var("TELEGRAM_CHAT_ID").ok()?;
        let topic = if !topic_override.is_empty() {
            topic_override.to_string()
        } else {
            env::var("TELEGRAM_TOPIC_ID").unwrap_or_else(|_| "267".to_string())
        };
        Some(Tg { token, chat, topic })
    }

    /// Send `text` to the configured topic. Returns true on HTTP 2xx.
    fn send(&self, text: &str) -> bool {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.token
        );
        // Telegram truncates; keep under 4096.
        let body = text.chars().take(4000).collect::<String>();
        let payload = ureq::json!({
            "chat_id": self.chat,
            "message_thread_id": self.topic.parse::<i64>().unwrap_or(267),
            "text": body,
            "disable_web_page_preview": true,
        });
        match ureq::post(&url).send_json(payload) {
            Ok(_) => true,
            Err(e) => {
                eprintln!("topics: tg_send failed: {}", e);
                false
            }
        }
    }
}

// ── git helpers ────────────────────────────────────────────────────────────────
fn git_log_since(repo: &str, r#ef: &str, since_days: u32) -> Vec<(String, String, String)> {
    // returns Vec<(hash, date, subject)>
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(repo).arg("log");
    if !r#ef.is_empty() {
        cmd.arg(r#ef);
    }
    cmd.arg(format!("--since={} days ago", since_days))
        .arg("--pretty=format:%h|%ad|%s")
        .arg("--date=short");
    let out = cmd.output().ok();
    let text = match out {
        Some(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };
    text.lines()
        .filter(|l| l.contains('|'))
        .map(|l| {
            let mut it = l.splitn(3, '|');
            (
                it.next().unwrap_or("").to_string(),
                it.next().unwrap_or("").to_string(),
                it.next().unwrap_or("").to_string(),
            )
        })
        .collect()
}

// ── git-watch: post NEW commits from both repos to topic 292 ──────────────────
fn cmd_git_watch(iv: u64) -> i32 {
    let tg = match Tg::from_env("292") {
        Some(t) => t,
        None => {
            eprintln!("topics git-watch: TELEGRAM_BOT_TOKEN/CHAT_ID not set; idling.");
            return 2;
        }
    };
    let state = log_dir().join(".git_watch_state");
    let mut seen: Vec<String> = fs::read_to_string(&state)
        .unwrap_or_default()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    eprintln!(
        "topics git-watch: posting new commits (dowiz + openbebop) -> topic 292 every {}s",
        iv
    );
    loop {
        // best-effort fetch of live openbebop ref
        let _ = Command::new("git")
            .arg("-C")
            .arg(BEBOP)
            .arg("fetch")
            .arg("openbebop")
            .arg("--quiet")
            .output();

        let mut fresh: Vec<(String, String, String)> = Vec::new();
        for (h, d, s) in git_log_since(DOWIZ, "", 7) {
            if !seen.contains(&h) {
                seen.push(h.clone());
                fresh.push((h, d, s));
            }
        }
        for (h, d, s) in git_log_since(BEBOP, BEBOP_REF, 7) {
            if !seen.contains(&h) {
                seen.push(h.clone());
                fresh.push((h, d, s));
            }
        }
        if !fresh.is_empty() {
            let mut msg = String::from("GIT — new commits\n");
            for (h, d, s) in &fresh {
                msg.push_str(&format!("  {} [{}] {}\n", h, d, s));
            }
            tg.send(&msg);
            let joined = seen
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let _ = fs::write(&state, joined);
        }
        std::thread::sleep(std::time::Duration::from_secs(iv));
    }
}

// ── plans: aggregate DOD + git + roadmap -> topic 291 ────────────────────────
fn cmd_plans() -> i32 {
    let tg = match Tg::from_env("291") {
        Some(t) => t,
        None => {
            eprintln!("topics plans: TELEGRAM_BOT_TOKEN/CHAT_ID not set.");
            return 2;
        }
    };

    // best-effort fetch
    let _ = Command::new("git")
        .arg("-C")
        .arg(BEBOP)
        .arg("fetch")
        .arg("openbebop")
        .arg("--quiet")
        .output();

    let mut events: Vec<(String, String, String)> = Vec::new(); // (date, repo, text)

    // (a) DOD plans
    let pj = log_dir().join("plan.jsonl");
    let mut n_plans = 0usize;
    let mut eta_min = 0i64;
    let mut eta_tok = 0i64;
    if let Ok(txt) = fs::read_to_string(&pj) {
        for l in txt.lines() {
            if l.trim().is_empty() {
                continue;
            }
            if let Some(d) = serde_json_or_fallback(l) {
                n_plans += 1;
                eta_min += d.0;
                eta_tok += d.1;
                events.push((
                    d.2.clone(),
                    "DOD".into(),
                    format!("{} — {} (eta {}min tok {})", d.3, d.4, d.0, d.1),
                ));
            }
        }
    }
    // (b) git log both repos
    for (h, d, s) in git_log_since(DOWIZ, "", 7) {
        events.push((d, "dowiz".into(), format!("{} {}", h, s)));
    }
    for (h, d, s) in git_log_since(BEBOP, BEBOP_REF, 7) {
        events.push((d, "openbebop".into(), format!("{} {}", h, s)));
    }
    // (c) roadmap doc ## N. headers
    if let Ok(txt) = fs::read_to_string(ROADMAP) {
        for l in txt.lines() {
            if l.starts_with("## ")
                && l[3..].chars().take(4).any(|c| c.is_ascii_digit())
            {
                events.push(("since".into(), "ROADMAP".into(), l[3..].trim().to_string()));
            }
        }
    }

    events.sort_by(|a, b| a.0.cmp(&b.0));
    let mut msg = format!(
        "UNIFIED PLANS & TASKS — last 7d\n{} items | {} DOD plans | ETA sum {}min | ~{} tok\n{}\n",
        events.len(),
        n_plans,
        eta_min,
        eta_tok,
        "-".repeat(32)
    );
    let mut cur_day: Option<String> = None;
    for (ts, repo, text) in &events {
        let day = ts.chars().take(10).collect::<String>();
        if cur_day.as_deref() != Some(day.as_str()) {
            cur_day = Some(day.clone());
            msg.push_str(&format!("\n{}\n", day));
        }
        let icon = match repo.as_str() {
            "DOD" => "[plan]",
            "dowiz" => "[dowiz]",
            "openbebop" => "[bebop]",
            "ROADMAP" => "[road]",
            _ => "•",
        };
        msg.push_str(&format!("  {} [{}] {}\n", icon, repo, text));
    }
    tg.send(&msg);
    0
}

/// Minimal JSON parse for plan.jsonl fields we need, without a serde dep for the
/// whole binary (ureq already pulls serde, so we reuse its json).
fn serde_json_or_fallback(line: &str) -> Option<(i64, i64, String, String, String)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let get = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let i = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_i64())
            .or_else(|| v.get(k).and_then(|x| x.as_f64()).map(|f| f as i64))
            .unwrap_or(0)
    };
    Some((
        i("eta_min"),
        i("eta_tokens"),
        v.get("ts").and_then(|x| x.as_str()).unwrap_or("since").to_string(),
        get("id"),
        get("title"),
    ))
}

// ── bench-watch: poll eval-layer result + entropy ledger -> topic 294 ──────────
fn cmd_bench_watch(iv: u64) -> i32 {
    let tg = match Tg::from_env("294") {
        Some(t) => t,
        None => {
            eprintln!("topics bench-watch: TELEGRAM_BOT_TOKEN/CHAT_ID not set.");
            return 2;
        }
    };
    let state = log_dir().join(".bench_state");
    let mut prev_mt: i64 = fs::read_to_string(&state)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    eprintln!("topics bench-watch: -> topic 294 every {}s", iv);
    loop {
        // entropy ledger (openbebop/rust-core) — best-effort
        if PathBuf::from(BEBOP).join("rust-core").exists() {
            let out = Command::new("cargo")
                .arg("test")
                .arg("-p")
                .arg("rust-core")
                .arg("entropy_ledger")
                .arg("--")
                .arg("--nocapture")
                .current_dir(BEBOP)
                .output()
                .ok();
            if let Some(o) = out {
                let s = String::from_utf8_lossy(&o.stdout);
                let filtered: Vec<&str> = s
                    .lines()
                    .filter(|l| l.to_lowercase().contains("compression_length_bits") || l.to_lowercase().contains("entropy"))
                    .rev()
                    .take(3)
                    .collect();
                if !filtered.is_empty() {
                    let joined = filtered.join(" ");
                    tg.send(&format!("openbebop entropy_ledger: {}", joined));
                }
            }
        }
        // eval-layer result
        let res = PathBuf::from(DOWIZ).join("eval-layer/deepeval-result.json");
        if let Ok(meta) = fs::metadata(&res) {
            let mt = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
            if mt > prev_mt {
                prev_mt = mt;
                let _ = fs::write(&state, mt.to_string());
                if let Ok(txt) = fs::read_to_string(&res) {
                    let summ = txt.chars().take(180).collect::<String>();
                    tg.send(&format!("dowiz eval-layer: {}", summ));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(iv));
    }
}

fn usage() -> i32 {
    eprintln!("usage: topics <subcommand> [interval_s]");
    eprintln!("  git-watch [60]     post new commits (dowiz+openbebop) -> topic 292");
    eprintln!("  plans             aggregate DOD+git+roadmap -> topic 291");
    eprintln!("  bench-watch [120]  poll entropy/eval -> topic 294");
    2
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
        std::process::exit(2);
    }
    let iv: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
    let rc = match args[1].as_str() {
        "git-watch" => cmd_git_watch(iv),
        "plans" => cmd_plans(),
        "bench-watch" => cmd_bench_watch(iv),
        _ => usage(),
    };
    std::process::exit(rc);
}
