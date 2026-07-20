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

// ── resources: hourly host/kernel resource+efficiency pulse -> RESOURCES topic ──
//
// TELEGRAM-OPS-OBSERVABILITY-CHANNEL-DESIGN-2026-07-20.md §3/§5, prioritized
// build-order item 1 ("maximum operator value, zero roadmap dependency").
// Named-absence everywhere (kernel::fdr::Reading<T> philosophy, §3's "never a
// number that wasn't measured"): every unmeasured field renders
// `[absent — <reason>]`, never a fabricated 0. Real fields wired here: disk/
// mem/load from hetzner-exporter's /health, host cpu_ticks from /proc/stat.
// Everything the design doc marks "needs a planned roadmap item first"
// (efficiency/joules/CO2e/threaded traces/build-provenance/ttft) or
// "structural gap" (GPU, mesh peer telemetry, network) stays absent by
// design — this subcommand does not get ahead of the items that gate them.
//
// No RESOURCES topic exists yet (design doc §1: "new topics get IDs at
// creation time") — set TELEGRAM_TOPIC_ID_RESOURCES once the operator creates
// it; until then this falls back to the HERMES topic (267) so a dry run has
// somewhere real to land rather than silently failing to find a topic.
const RESOURCES_TOPIC_ENV: &str = "TELEGRAM_TOPIC_ID_RESOURCES";
const RESOURCES_TOPIC_FALLBACK: &str = "267";

fn resources_topic() -> String {
    env::var(RESOURCES_TOPIC_ENV).unwrap_or_else(|_| RESOURCES_TOPIC_FALLBACK.to_string())
}

/// Host CPU aggregate tick count from `/proc/stat`'s leading `cpu ` line (sum
/// of user+nice+system+idle+iowait+irq+softirq+steal — the same fields the
/// kernel scheduler accounts). `None` if unreadable/unparseable (not Linux, or
/// the line shape changed) — never a fabricated 0.
fn host_cpu_ticks() -> Option<u64> {
    let s = fs::read_to_string("/proc/stat").ok()?;
    let line = s.lines().next()?;
    if !line.starts_with("cpu ") {
        return None;
    }
    let sum: u64 = line
        .split_whitespace()
        .skip(1)
        .filter_map(|f| f.parse::<u64>().ok())
        .sum();
    Some(sum)
}

/// Best-effort fetch of hetzner-exporter's live `/health` JSON
/// (disk_pct/load1/mem_pct/ts — see `tools/telemetry/hetzner-exporter/src/lib.rs`).
/// `None` if the exporter isn't reachable (not running, wrong host) — the
/// caller renders each gauge as a named absence rather than guessing.
fn fetch_host_gauges() -> Option<serde_json::Value> {
    let resp = ureq::get("http://127.0.0.1:9091/health")
        .timeout(std::time::Duration::from_secs(2))
        .call()
        .ok()?;
    resp.into_json::<serde_json::Value>().ok()
}

fn fmt_gauge(v: &Option<serde_json::Value>, key: &str) -> String {
    match v.as_ref().and_then(|j| j.get(key).and_then(|x| x.as_f64())) {
        Some(x) if x >= 0.0 => format!("{x:.1}"),
        Some(_) => "[absent — exporter reported an error sentinel]".to_string(),
        None => "[absent — hetzner-exporter unreachable at 127.0.0.1:9091]".to_string(),
    }
}

fn cmd_resources() -> i32 {
    let topic = resources_topic();
    let tg = match Tg::from_env(&topic) {
        Some(t) => t,
        None => {
            eprintln!("topics resources: TELEGRAM_BOT_TOKEN/CHAT_ID not set.");
            return 2;
        }
    };

    let gauges = fetch_host_gauges();
    let disk = fmt_gauge(&gauges, "disk_pct");
    let load1_norm = fmt_gauge(&gauges, "load1"); // already CPU-count-normalized, see hetzner-exporter
    let mem = fmt_gauge(&gauges, "mem_pct");
    let load_breach = gauges
        .as_ref()
        .and_then(|j| j.get("load1").and_then(|x| x.as_f64()))
        .map(|l| if l > 4.0 { "BREACH" } else { "OK" })
        .unwrap_or("[absent]");

    let cpu_ticks = host_cpu_ticks()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "[absent — /proc/stat unreadable]".to_string());

    let host = Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "host".to_string());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let report = format!(
        "RESOURCES dowiz-dev ({host}) ts={ts}\n\
         CPU    load1(norm) {load1_norm} | breach>4.0 {load_breach} | cpu_ticks {cpu_ticks}\n\
         \x20      IPC: [absent — PMU Tier B PermissionDenied (perf_event_open), item 27]\n\
         MEM    host mem_pct {mem}\n\
         DISK   {disk}\n\
         NET    [absent — no network metric in hetzner-exporter; unplanned gap]\n\
         GPU    [absent — zero GPU telemetry exists; declared-empty seam]\n\
         PWR    joules: [absent — NoRaplInterface expected on Hetzner, unverified]\n\
         CO2e   [not configured — NoRegionalConstant; item 69 + operator grid constant]\n\
         \n\
         EFFICIENCY [absent — item 58 Work-Normalized Cost Ledger NOT BUILT]\n\
         LATENCY [absent — span_latency_us histogram reader not wired into `topics` yet]"
    );

    if tg.send(&report) {
        eprintln!("topics resources: posted to topic {topic}");
        0
    } else {
        eprintln!("topics resources: send failed");
        1
    }
}

fn usage() -> i32 {
    eprintln!("usage: topics <subcommand> [interval_s]");
    eprintln!("  git-watch [60]     post new commits (dowiz+openbebop) -> topic 292");
    eprintln!("  plans             aggregate DOD+git+roadmap -> topic 291");
    eprintln!("  bench-watch [120]  poll entropy/eval -> topic 294");
    eprintln!("  resources          one-shot host resource pulse -> RESOURCES topic (§TELEGRAM-OPS-OBSERVABILITY-CHANNEL-DESIGN-2026-07-20.md)");
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
        "resources" => cmd_resources(),
        _ => usage(),
    };
    std::process::exit(rc);
}

#[cfg(test)]
mod resources_tests {
    use super::*;

    #[test]
    fn host_cpu_ticks_reads_a_positive_sum_on_linux() {
        // /proc/stat is Linux-only; this crate ships for the ops box (Hetzner,
        // Linux) so a None here would itself be a real, actionable finding —
        // not something to special-case away in the test.
        let t = host_cpu_ticks().expect("/proc/stat readable on this host");
        assert!(t > 0, "cpu ticks must be a positive accumulator, got {t}");
    }

    #[test]
    fn fmt_gauge_renders_a_present_positive_value() {
        let v = Some(serde_json::json!({"disk_pct": 71.3}));
        assert_eq!(fmt_gauge(&v, "disk_pct"), "71.3");
    }

    #[test]
    fn fmt_gauge_names_absence_when_key_missing() {
        let v = Some(serde_json::json!({"other_key": 1.0}));
        let out = fmt_gauge(&v, "disk_pct");
        assert!(
            out.starts_with("[absent"),
            "missing key must render a named absence, got {out:?}"
        );
    }

    #[test]
    fn fmt_gauge_names_absence_when_exporter_unreachable() {
        let out = fmt_gauge(&None, "disk_pct");
        assert!(
            out.contains("unreachable"),
            "None (no exporter response) must name the unreachable reason, got {out:?}"
        );
    }

    #[test]
    fn fmt_gauge_never_silently_renders_an_error_sentinel_as_a_real_number() {
        // hetzner-exporter's own convention: -1.0 signals a computation error
        // (see hetzner-exporter/src/lib.rs). Must never be printed as "-1.0".
        let v = Some(serde_json::json!({"disk_pct": -1.0}));
        let out = fmt_gauge(&v, "disk_pct");
        assert!(
            out.starts_with("[absent"),
            "a -1.0 error sentinel must render as a named absence, not a number, got {out:?}"
        );
    }

    #[test]
    fn resources_topic_falls_back_then_honors_override() {
        // One test, not two: both cases mutate the same process-global env
        // var, and cargo test runs tests in parallel by default within one
        // process — two separate tests touching it would be a real race.
        env::remove_var(RESOURCES_TOPIC_ENV);
        assert_eq!(resources_topic(), RESOURCES_TOPIC_FALLBACK);
        env::set_var(RESOURCES_TOPIC_ENV, "999");
        assert_eq!(resources_topic(), "999");
        env::remove_var(RESOURCES_TOPIC_ENV);
    }
}
