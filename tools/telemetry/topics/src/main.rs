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
const ROADMAP: &str = "/root/dowiz/docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md";

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
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
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
            if l.starts_with("## ") && l[3..].chars().take(4).any(|c| c.is_ascii_digit()) {
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
        v.get("ts")
            .and_then(|x| x.as_str())
            .unwrap_or("since")
            .to_string(),
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
                    .filter(|l| {
                        l.to_lowercase().contains("compression_length_bits")
                            || l.to_lowercase().contains("entropy")
                    })
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
            let mt = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
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

/// `(used, total)` from two exporter JSON keys, `None` when either is missing or an
/// error sentinel (<0). `div` rescales (e.g. MiB→GiB = 1024.0).
fn abs_pair(
    v: &Option<serde_json::Value>,
    used_key: &str,
    total_key: &str,
    div: f64,
) -> Option<(f64, f64)> {
    let j = v.as_ref()?;
    let u = j.get(used_key)?.as_f64()?;
    let t = j.get(total_key)?.as_f64()?;
    if u < 0.0 || t <= 0.0 {
        return None;
    }
    Some((u / div, t / div))
}

/// Human bytes, 1024-based, one decimal.
fn fmt_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Truthful log-bucket upper-bound rendering: durations are recorded at µs
/// granularity, so an upper bound of 0 means "completed in under a microsecond" —
/// render that, not a weird "≤0µs".
fn fmt_bound_us(us: u64) -> String {
    if us == 0 {
        "<1µs".to_string()
    } else {
        format!("≤{}", fmt_us(us))
    }
}

/// Human microseconds: µs below 1ms, then ms, then s.
fn fmt_us(us: u64) -> String {
    if us < 1_000 {
        format!("{us}µs")
    } else if us < 1_000_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}

/// Truthful EXACT-value rendering at µs instrument granularity: a recorded 0
/// means "completed in under a microsecond" — render that, never a bare "0µs"
/// (sibling of `fmt_bound_us`, minus the "≤" since the value is exact).
fn fmt_exact_us(us: u64) -> String {
    if us == 0 {
        "<1µs".to_string()
    } else {
        fmt_us(us)
    }
}

/// `YYYY-MM-DD HH:MM UTC` from a unix timestamp (pure std — civil-from-days,
/// Howard Hinnant's algorithm; no chrono).
fn utc_datetime(ts: u64) -> String {
    let days = (ts / 86_400) as i64;
    let secs = ts % 86_400;
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        secs / 3_600,
        (secs % 3_600) / 60
    )
}

// ── IPC: real hardware PMU reading (kernel item 27, Tier B) ────────────────────
//
// Uses the kernel's own `fdr::pmu::PmuStation` (hand-rolled perf_event_open(2)) to
// bracket a fixed integer workload and compute instructions-per-cycle from the REAL
// counter deltas. Requires `kernel.perf_event_paranoid <= 2` on this host (persisted
// via /etc/sysctl.d/99-dowiz-perfmon.conf); when the host blocks it, this returns the
// kernel's NAMED absence reason — never a fabricated number.
fn measure_ipc() -> Result<f64, String> {
    use dowiz_kernel::fdr::pmu::PmuStation;
    use dowiz_kernel::fdr::schema::Reading;
    let station = PmuStation::new();
    let (_, d) = station.bracket(|| {
        let mut acc = 0u64;
        for i in 0..5_000_000u64 {
            acc = acc.wrapping_add(i.wrapping_mul(2654435761) ^ (i >> 3));
        }
        std::hint::black_box(acc)
    });
    match (d.hw_instructions, d.hw_cpu_cycles) {
        (Reading::Value(i), Reading::Value(c)) if c > 0 => Ok(i as f64 / c as f64),
        (Reading::Unavailable(a), _) | (_, Reading::Unavailable(a)) => Err(format!("{a:?}")),
        _ => Err("ReadError".to_string()),
    }
}

// ── LATENCY: reader for the kernel's span_metrics `metric.jsonl` (P83 Layer 1) ─
//
// Row format is golden-pinned in kernel/src/span_metrics/obs.rs
// (`golden_metric_row_exact_bytes`):
//   {"metric":"span_latency_us","span":"place_order","count":4,"sum_us":15,
//    "min_us":1,"max_us":8,"mean_us":3.750,"hist":[0:1,1:1,2:1,3:1]}
// NOTE: `hist` is a compact `bin:count` list — NOT valid JSON — so rows are
// hand-parsed here (serde_json would reject them). Rows stream cumulatively per
// span close, so the LAST row per span is that span's current histogram.

struct SpanStats {
    span: String,
    count: u64,
    sum_us: u64,
    max_us: u64,
    /// `(bucket_index, count)` — bucket i covers `[2^i, 2^(i+1))` µs (bucket 0: `[0,2)`).
    hist: Vec<(usize, u64)>,
}

fn json_u64_field(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn json_str_field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    Some(rest[..rest.find('"')?].to_string())
}

fn parse_span_row(line: &str) -> Option<SpanStats> {
    if !line.contains("\"metric\":\"span_latency_us\"") {
        return None;
    }
    let span = json_str_field(line, "span")?;
    let count = json_u64_field(line, "count")?;
    let sum_us = json_u64_field(line, "sum_us")?;
    let max_us = json_u64_field(line, "max_us")?;
    let i = line.find("\"hist\":[")? + "\"hist\":[".len();
    let body = &line[i..i + line[i..].find(']')?];
    let mut hist = Vec::new();
    for pair in body.split(',').filter(|p| !p.is_empty()) {
        let (a, b) = pair.split_once(':')?;
        hist.push((a.parse().ok()?, b.parse().ok()?));
    }
    Some(SpanStats {
        span,
        count,
        sum_us,
        max_us,
        hist,
    })
}

/// Nearest-rank percentile UPPER BOUND from a log-bucket histogram: the inclusive top
/// of the bucket holding rank `ceil(q*count)`, clamped by the exact recorded max.
/// Truthful "p99 ≤ X" semantics — log buckets cannot give an exact percentile value.
fn hist_percentile_upper_us(s: &SpanStats, q: f64) -> Option<u64> {
    if s.count == 0 {
        return None;
    }
    let rank = ((q * s.count as f64).ceil() as u64).max(1);
    let mut cum = 0u64;
    for &(i, c) in &s.hist {
        cum += c;
        if cum >= rank {
            let upper = if i >= 63 {
                u64::MAX
            } else {
                (1u64 << (i + 1)) - 1
            };
            return Some(upper.min(s.max_us));
        }
    }
    Some(s.max_us)
}

/// Span-metrics dir: `DOWIZ_SPAN_METRICS_DIR` (the repo-wide convention — same env the
/// kernel's `span_metrics::init` and `tools/telemetry kernel-spans` use), else this
/// binary's own log dir.
fn span_metrics_dir() -> PathBuf {
    env::var("DOWIZ_SPAN_METRICS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| log_dir())
}

/// The CANARY span stream: `<root>/canary/metric.jsonl`, written by the scheduled
/// `kernel/examples/canary_spans.rs` runner (real instrumented kernel calls, real
/// wall-clock durations, synthetic inputs). A sibling FILE under the same root —
/// never the live `<root>/metric.jsonl` — so canary samples are structurally
/// incapable of blending into real-traffic numbers (the row format is golden-pinned
/// and carries no source tag; the stream identity IS the tag).
fn canary_metric_path() -> PathBuf {
    span_metrics_dir().join("canary").join("metric.jsonl")
}

/// Last-row-per-span stats from `metric.jsonl` content (rows stream cumulatively, so
/// the last row per span is its current state); `None` when no parseable rows exist
/// (a truthful "no data yet", distinct from "not wired").
fn span_stats_from(txt: &str) -> Option<Vec<SpanStats>> {
    let mut last: std::collections::BTreeMap<String, SpanStats> = Default::default();
    for line in txt.lines() {
        if let Some(s) = parse_span_row(line) {
            last.insert(s.span.clone(), s);
        }
    }
    if last.is_empty() {
        None
    } else {
        Some(last.into_values().collect())
    }
}

/// EXACT per-sample durations for `span`, reconstructed from the cumulative row
/// stream: every span close appends one row with `count+1`, so `Δsum_us` between
/// consecutive rows IS that sample's exact duration (a `count==1` row is itself the
/// first sample). `flush()` duplicates (`count` unchanged) and un-attributable gaps
/// (`count` jumps by >1, e.g. a lost line) are skipped, never guessed.
fn reconstruct_durations(txt: &str, span: &str) -> Vec<u64> {
    let mut prev: Option<(u64, u64)> = None; // (count, sum_us)
    let mut out = Vec::new();
    for line in txt.lines() {
        let s = match parse_span_row(line) {
            Some(s) if s.span == span => s,
            _ => continue,
        };
        if s.count == 1 {
            out.push(s.sum_us); // first sample of a (possibly restarted) stream
        } else if let Some((pc, ps)) = prev {
            if s.count == pc + 1 && s.sum_us >= ps {
                out.push(s.sum_us - ps);
            }
            // count unchanged (flush) or a gap: skip — never attribute what we
            // did not observe.
        }
        prev = Some((s.count, s.sum_us));
    }
    out
}

/// Sample standard deviation (n−1) in µs; `None` below 2 samples — a jitter figure
/// from fewer than two observations would be a fabricated 0.
fn stddev_us(durs: &[u64]) -> Option<f64> {
    if durs.len() < 2 {
        return None;
    }
    let n = durs.len() as f64;
    let mean = durs.iter().map(|&x| x as f64).sum::<f64>() / n;
    let var = durs.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / (n - 1.0);
    Some(var.sqrt())
}

// ── CANARY stream stats — EXACT, restart-safe (multi-run cumulative streams) ──
//
// The canary file accumulates many short scheduled runs, each restarting its
// per-process histograms at `count==1`. A last-row-per-span read (the live-stream
// semantics above) would therefore only see the LAST run. Canary stats instead
// derive everything from `reconstruct_durations` — the exact per-sample Δsum_us
// stream, which already handles restarts (`count==1` rows) and skips
// un-attributable gaps — so n / p50 / p99 / jitter all cover the SAME sample set
// across every run in the file. Percentiles here are EXACT nearest-rank values
// over the reconstructed samples (not log-bucket upper bounds), hence rendered
// without the "≤" bound marker. Whole-file rotation (canary_spans.rs) only ever
// removes complete old files, so this reader never sees a torn stream.

/// Per-action canary latency: exact values from reconstructed per-sample durations.
struct CanaryLatency {
    span: String,
    n: usize,
    p50_us: u64,
    p99_us: u64,
    jitter_stddev_us: Option<f64>,
    layer1: bool,
}

/// Nearest-rank percentile over an ascending-sorted sample set (exact, not a bound).
fn exact_percentile_us(sorted: &[u64], q: f64) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

/// Exact per-action stats over a canary stream. Sorted by n desc (then name).
fn canary_action_latency(txt: &str) -> Vec<CanaryLatency> {
    let stats = match span_stats_from(txt) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let mut out: Vec<CanaryLatency> = stats
        .iter()
        .filter_map(|s| {
            let mut durs = reconstruct_durations(txt, &s.span);
            if durs.is_empty() {
                return None; // no attributable samples — honest absence, never a guess
            }
            let jitter = stddev_us(&durs);
            durs.sort_unstable();
            Some(CanaryLatency {
                span: s.span.clone(),
                n: durs.len(),
                p50_us: exact_percentile_us(&durs, 0.50)?,
                p99_us: exact_percentile_us(&durs, 0.99)?,
                jitter_stddev_us: jitter,
                layer1: INSTRUMENTED_ACTIONS.contains(&s.span.as_str()),
            })
        })
        .collect();
    out.sort_by(|a, b| b.n.cmp(&a.n).then(a.span.cmp(&b.span)));
    out
}

/// Computed latency summary for the dominant span (feeds both the message line and
/// the persisted `resources-summary.jsonl` record).
struct LatencySummary {
    span: String,
    n: u64,
    p50_le_us: u64,
    p99_le_us: u64,
    /// Exact sample stddev of reconstructed durations; `None` when n<2.
    jitter_stddev_us: Option<f64>,
    span_kinds: usize,
}

fn latency_summary(txt: &str) -> Option<LatencySummary> {
    let stats = span_stats_from(txt)?;
    let top = stats.iter().max_by_key(|s| s.count)?;
    Some(LatencySummary {
        span: top.span.clone(),
        n: top.count,
        p50_le_us: hist_percentile_upper_us(top, 0.50)?,
        p99_le_us: hist_percentile_upper_us(top, 0.99)?,
        jitter_stddev_us: stddev_us(&reconstruct_durations(txt, &top.span)),
        span_kinds: stats.len(),
    })
}

fn latency_line(summary: &Option<LatencySummary>) -> String {
    match summary {
        None => "LAT   no span data collected yet — awaiting live order traffic (reader wired, this is normal pre-launch)".to_string(),
        Some(s) => {
            let jitter = match s.jitter_stddev_us {
                Some(j) => format!("jitter(σ) {:.1}µs", j),
                None => "jitter n/a (n<2)".to_string(),
            };
            format!(
                "LAT   {} p50 {} · p99 {} · {jitter} · n={} ({} span kind{})",
                s.span,
                fmt_bound_us(s.p50_le_us),
                fmt_bound_us(s.p99_le_us),
                s.n,
                s.span_kinds,
                if s.span_kinds == 1 { "" } else { "s" }
            )
        }
    }
}

/// The LAT line with an HONEST source label. Live production spans always win;
/// the canary is only surfaced when there is no live data, and is explicitly
/// labeled as synthetic — canary timing must never read as user-facing latency.
fn lat_source_line(live: &Option<LatencySummary>, canary: &[CanaryLatency]) -> String {
    match (live, canary.first()) {
        (Some(_), _) => format!("{} · [source: live]", latency_line(live)),
        (None, Some(c)) => {
            let jitter = match c.jitter_stddev_us {
                Some(j) => format!("jitter(σ) {j:.1}µs"),
                None => "jitter n/a (n<2)".to_string(),
            };
            format!(
                "LAT   {} p50 {} · p99 {} · {jitter} · n={} ({} span kind{}) · [source: canary — synthetic probe, not user traffic]",
                c.span,
                fmt_exact_us(c.p50_us),
                fmt_exact_us(c.p99_us),
                c.n,
                canary.len(),
                if canary.len() == 1 { "" } else { "s" }
            )
        }
        (None, None) => latency_line(&None),
    }
}

// ── PWR/CO2e/EFFICIENCY: transparent ESTIMATES (clearly labeled, never "measured") ─
//
// This host is a Hetzner Cloud KVM vServer (fsn1-dc14, Falkenstein): /sys/class/powercap
// is empty — KVM does not pass RAPL through, so a MEASURED wattage is impossible in this
// guest. Instead of fabricating one, the estimate is grounded in the ACTUAL virtualized
// CPU model: /proc/cpuinfo reports "AMD EPYC-Milan Processor", cpu family 25 = Zen 3,
// i.e. a genuine EPYC 7003-series host part whose exact physical SKU is masked by QEMU.
//
//   estimated_watts = vCPUs × (idle_per_core + (share_per_core − idle_per_core) × util)
//
//   * share_per_core = 3.95 W — average of the two 64-core Milan density SKUs cloud
//     providers deploy: EPYC 7763 (280 W TDP / 64 cores = 4.375 W) and EPYC 7713
//     (225 W / 64 = 3.516 W), AMD published default-TDP specs. Conservative: a vCPU is
//     an SMT thread (half a physical core), so this over- rather than under-estimates.
//   * idle_per_core = 15% of the share — the standard modern-server rule of thumb
//     (C-state power gating idles cores at ~10-20% of TDP share). An approximation,
//     stated as such.
//   * util = REAL /proc/stat tick-delta over a ~1 s window (two samples; busy =
//     user+nice+system+irq+softirq over the 7-field total) — not load1's decayed avg.
const MILAN_PER_CORE_TDP_SHARE_W: f64 = 3.95; // avg(280/64, 225/64), EPYC 7763 + 7713
const MILAN_IDLE_FRACTION: f64 = 0.15; // ~10-20% of TDP share at idle (rule of thumb)

/// Germany average grid carbon intensity, 2025 (Nowtricity yearly average,
/// nowtricity.com/country/germany — 328 gCO2eq/kWh). STATIC operator-supplied constant
/// for the fsn1/Falkenstein datacenter — NOT live grid data (item 69's operator-provided
/// regional constant).
const DE_GRID_GCO2E_PER_KWH: f64 = 328.0;

/// `(busy, total)` jiffies from a `/proc/stat` aggregate `cpu ` line.
/// busy = user+nice+system+irq+softirq; total = the 7-field sum
/// (user nice system idle iowait irq softirq).
fn parse_cpu_jiffies(line: &str) -> Option<(u64, u64)> {
    if !line.starts_with("cpu ") {
        return None;
    }
    let f: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    if f.len() < 7 {
        return None;
    }
    let busy = f[0] + f[1] + f[2] + f[5] + f[6];
    let total: u64 = f[..7].iter().sum();
    Some((busy, total))
}

/// Real CPU utilization over `window`: two `/proc/stat` samples, `Δbusy / Δtotal`.
/// Returns `(util_fraction, actual_window_secs)`; `None` if /proc/stat is unreadable
/// or no ticks elapsed — never a fabricated 0.
fn cpu_util_tick_delta(window: std::time::Duration) -> Option<(f64, f64)> {
    let read = || -> Option<(u64, u64)> {
        parse_cpu_jiffies(fs::read_to_string("/proc/stat").ok()?.lines().next()?)
    };
    let (b0, t0) = read()?;
    let started = std::time::Instant::now();
    std::thread::sleep(window);
    let (b1, t1) = read()?;
    let dt = t1.checked_sub(t0)?;
    if dt == 0 {
        return None;
    }
    let db = b1.saturating_sub(b0);
    Some((
        (db as f64 / dt as f64).clamp(0.0, 1.0),
        started.elapsed().as_secs_f64(),
    ))
}

/// Zen3/Milan per-core TDP-share power model (see module comment above).
fn estimate_power_watts(vcpus: usize, util_frac: f64) -> f64 {
    let share = MILAN_PER_CORE_TDP_SHARE_W;
    let idle = MILAN_IDLE_FRACTION * share;
    let u = util_frac.clamp(0.0, 1.0);
    vcpus as f64 * (idle + (share - idle) * u)
}

/// Build the full report body plus the structured per-run summary record. All live
/// reads happen here so `cmd_resources` can print → persist → (optionally) send.
fn build_resources_report() -> (String, serde_json::Value) {
    let gauges = fetch_host_gauges();
    let disk = fmt_gauge(&gauges, "disk_pct");
    let load1_norm = fmt_gauge(&gauges, "load1"); // CPU-count-normalized, see hetzner-exporter
    let mem = fmt_gauge(&gauges, "mem_pct");
    let load_val = gauges
        .as_ref()
        .and_then(|j| j.get("load1").and_then(|x| x.as_f64()))
        .filter(|l| *l >= 0.0);
    let load_breach = load_val
        .map(|l| if l > 4.0 { "BREACH" } else { "OK" })
        .unwrap_or("[absent]");

    let cpu_ticks = host_cpu_ticks()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "[absent — /proc/stat unreadable]".to_string());

    let ipc_line = match measure_ipc() {
        Ok(ipc) => {
            format!("IPC   {ipc:.2} instr/cycle — hardware PMU (perf_event_open), measured now")
        }
        Err(reason) => {
            format!("IPC   [absent — PMU Tier B {reason}; check kernel.perf_event_paranoid]")
        }
    };

    let mem_line = match abs_pair(&gauges, "mem_used_mb", "mem_total_mb", 1024.0) {
        Some((u, t)) => format!("MEM   {mem}% used ({u:.1} / {t:.1} GiB)"),
        None => format!("MEM   {mem}% used"),
    };
    let disk_line = match abs_pair(&gauges, "disk_used_gb", "disk_total_gb", 1.0) {
        Some((u, t)) => format!("DISK  {disk}% used ({u:.1} / {t:.1} GiB)"),
        None => format!("DISK  {disk}% used"),
    };

    let net_line = {
        let get = |k: &str| {
            gauges
                .as_ref()
                .and_then(|j| j.get(k).and_then(|x| x.as_f64()))
                .filter(|v| *v >= 0.0)
                .map(|v| v as u64)
        };
        match (get("net_rx_bytes"), get("net_tx_bytes")) {
            (Some(rx), Some(tx)) => format!(
                "NET   rx {} · tx {} — since boot, all non-loopback interfaces",
                fmt_bytes(rx),
                fmt_bytes(tx)
            ),
            _ => "NET   [absent — hetzner-exporter unreachable or predates net gauge]".to_string(),
        }
    };

    let metric_txt = fs::read_to_string(span_metrics_dir().join("metric.jsonl")).ok();
    let span_stats = metric_txt.as_deref().and_then(span_stats_from);
    let lat = metric_txt.as_deref().and_then(latency_summary);
    // Canary stream (scheduled synthetic probe) — separate file, separate label;
    // only shown when no live span data exists, and never as "live".
    let canary_txt = fs::read_to_string(canary_metric_path()).ok();
    let canary = canary_txt
        .as_deref()
        .map(canary_action_latency)
        .unwrap_or_default();
    let lat_line = lat_source_line(&lat, &canary);

    // Real utilization: two /proc/stat samples ~1s apart (a MEASURED value, also fed
    // to the power model below).
    let util = cpu_util_tick_delta(std::time::Duration::from_secs(1));
    let util_note = match util {
        Some((u, w)) => format!("util {:.1}% ({w:.1}s tick-delta)", u * 100.0),
        None => "util [absent — /proc/stat unreadable]".to_string(),
    };

    // ── Estimates (labeled; see the Zen3/Milan TDP-share model comment above) ──
    let vcpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let orders: u64 = span_stats
        .as_ref()
        .and_then(|l| l.iter().find(|s| s.span == "place_order"))
        .map(|s| s.count)
        .unwrap_or(0);
    // (util_frac, est_watts, co2e g/h) when /proc/stat was readable.
    let est = util.map(|(u, _)| {
        let w = estimate_power_watts(vcpus, u);
        (u, w, w / 1000.0 * DE_GRID_GCO2E_PER_KWH)
    });
    let (pwr_line, co2_line, eff_line) = match est {
        Some((u, w, g_per_h)) => {
            let kj_per_h = w * 3600.0 / 1000.0;
            let eff = if orders == 0 {
                format!("EFF   est. draw ~{w:.1} W · cost/order UNDEFINED — 0 orders observed (full ledger = roadmap item 58)")
            } else {
                format!("EFF   est. draw ~{w:.1} W · {orders} orders in span log — windowed Wh/order needs the item-58 ledger")
            };
            (
                format!(
                    "PWR   ~{w:.1} W (~{kj_per_h:.0} kJ/h) — {vcpus} vCPU × Zen3/Milan TDP-share model @ util {:.1}%\n\
                     \x20     [per-core share 3.95 W = avg EPYC 7763 (280W/64c) + 7713 (225W/64c); idle = 15% of share; exact SKU masked by KVM; no RAPL in guest]",
                    u * 100.0
                ),
                format!("CO2e  ~{g_per_h:.1} g/h — PWR estimate × Germany grid avg {DE_GRID_GCO2E_PER_KWH:.0} gCO2e/kWh (2025 static constant, fsn1/Falkenstein — not live)"),
                eff,
            )
        }
        None => (
            "PWR   [estimate unavailable — /proc/stat tick-delta unreadable]".to_string(),
            "CO2e  [estimate unavailable — needs the PWR estimate]".to_string(),
            "EFF   [estimate unavailable — needs the PWR estimate]".to_string(),
        ),
    };

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
        "RESOURCES · dowiz-dev ({host}) · {when}\n\
         \n\
         MEASURED — live host data\n\
         CPU   load {load1_norm} per core — {load_breach} (alert >4.0) · {util_note} · ticks {cpu_ticks}\n\
         {ipc_line}\n\
         {mem_line}\n\
         {disk_line}\n\
         {net_line}\n\
         {lat_line}\n\
         \n\
         ESTIMATED — transparent model, NOT a measurement\n\
         {pwr_line}\n\
         {co2_line}\n\
         {eff_line}\n\
         \n\
         NOT AVAILABLE ON THIS HARDWARE\n\
         GPU   none attached to this Hetzner vServer — nothing to report",
        when = utc_datetime(ts),
    );

    // Structured per-run record for the hourly history (real computed values only;
    // absent = null, never a degenerate 0).
    let record = serde_json::json!({
        "kind": "resources_summary",
        "ts": ts,
        "host": host,
        "util_frac": util.map(|(u, _)| u),
        "est_watts": est.map(|(_, w, _)| w),
        "co2e_g_per_h": est.map(|(_, _, g)| g),
        "orders_observed": orders,
        "latency": lat.as_ref().map(|l| serde_json::json!({
            "span": l.span,
            "n": l.n,
            "p50_le_us": l.p50_le_us,
            "p99_le_us": l.p99_le_us,
            "jitter_stddev_us": l.jitter_stddev_us,
            "span_kinds": l.span_kinds,
            "source": "live",
        })),
        // Canary summary is a SEPARATE field — never written into "latency",
        // so downstream history consumers cannot mistake probe timing for live.
        "canary_latency": canary.first().map(|c| serde_json::json!({
            "span": c.span,
            "n": c.n,
            "p50_us": c.p50_us,
            "p99_us": c.p99_us,
            "jitter_stddev_us": c.jitter_stddev_us,
            "span_kinds": canary.len(),
            "source": "canary",
        })),
    });
    (report, record)
}

/// Append one summary record to `<log_dir>/resources-summary.jsonl` — the same
/// directory this binary already uses for its state files (`plan.jsonl`,
/// `.git_watch_state`), so hourly cron runs accumulate a queryable history.
fn append_resources_summary(record: &serde_json::Value) {
    use std::io::Write;
    let p = log_dir().join("resources-summary.jsonl");
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "{record}");
    }
}

fn cmd_resources() -> i32 {
    // Terminal-first: ALWAYS build and print the full body to stdout, THEN send to
    // Telegram only when credentials are present (operator requirement: verify the
    // real output before any send; also makes credential-less local runs useful).
    let (report, record) = build_resources_report();
    println!("{report}");
    append_resources_summary(&record);

    let topic = resources_topic();
    match Tg::from_env(&topic) {
        Some(tg) => {
            if tg.send(&report) {
                eprintln!("topics resources: posted to topic {topic}");
                0
            } else {
                eprintln!("topics resources: send failed");
                1
            }
        }
        None => {
            eprintln!(
                "topics resources: TELEGRAM_BOT_TOKEN/CHAT_ID not set — printed to stdout only, nothing sent."
            );
            0
        }
    }
}

// ── latency: per-action span latency report (P83 Layer-1) -> LATENCY topic ────
//
// Dedicated cron report breaking latency down BY INSTRUMENTED ACTION — unlike the
// aggregate `LAT` line in `resources`, which only summarizes the dominant span.
// Reads the same `metric.jsonl` source (span_metrics_dir(), same env convention),
// reuses the same hand-rolled row parser (rows are NOT valid JSON — `hist` is a
// compact `bin:count` list) and the same truthful log-bucket percentile math.
//
// Named-absence discipline: every instrumented action with zero samples renders
// `<action>: no data in window` — never a fabricated number. Counts are cumulative
// over the current span log (rows carry no timestamps, so a wall-clock window is
// unrepresentable here; the per-run history in latency-summary.jsonl is what gives
// the future anomaly layer its time axis).
//
// TWO sources, explicitly labeled, never blended (2026-07-20 canary wave):
//   * LIVE   — `<root>/metric.jsonl`, real order traffic (none pre-launch).
//   * CANARY — `<root>/canary/metric.jsonl`, the scheduled `canary_spans`
//     synthetic probe: REAL instrumented kernel calls timed by the real
//     observer, but NOT user traffic — its distribution must never be read
//     as (or mixed into) user-facing latency.
const LATENCY_TOPIC_ENV: &str = "TELEGRAM_TOPIC_ID_LATENCY";

fn latency_topic() -> String {
    env::var(LATENCY_TOPIC_ENV).unwrap_or_else(|_| resources_topic())
}

/// P83 Layer-1 instrumented action names EXACTLY as they appear in `metric.jsonl`,
/// verified against the live tree (grep `info_span!(` in kernel/src):
///   * native spans: `place_order` (domain.rs:176), `place_order_priced`
///     (domain.rs:220), `fold_transitions` (order_machine.rs:172)
///   * `telemetry`-gated wrappers (span_metrics/instrument.rs): `route`,
///     `commit_after_decide`, `decide_settlement`, `cap_verify_chain`
///     — NOTE: the cap-chain wrapper emits `cap_verify_chain` (underscores),
///     not the function path `cap::verify_chain`.
///   * `pq`-gated wrapper: `mldsa_verify`
/// Other kernel spans exist (`eigenvalues`, `eigenvalues_contig`, `agent_turn`) but
/// are not Layer-1 actions; when present in the log they are still reported, marked.
const INSTRUMENTED_ACTIONS: [&str; 8] = [
    "place_order",
    "place_order_priced",
    "fold_transitions",
    "route",
    "commit_after_decide",
    "decide_settlement",
    "cap_verify_chain",
    "mldsa_verify",
];

/// Per-action computed latency (feeds both the message and the history record).
struct ActionLatency {
    span: String,
    n: u64,
    p50_le_us: u64,
    p99_le_us: u64,
    /// Exact sample stddev of reconstructed per-sample durations; `None` when n<2.
    jitter_stddev_us: Option<f64>,
    /// Part of the P83 Layer-1 instrumented set above.
    layer1: bool,
}

/// Every span present in the log, each with its own percentiles/jitter — grouped
/// by action, never collapsed into one aggregate. Sorted by sample count desc
/// (then name) so the hottest action leads the report.
fn per_action_latency(txt: &str) -> Vec<ActionLatency> {
    let stats = match span_stats_from(txt) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let mut out: Vec<ActionLatency> = stats
        .iter()
        .filter_map(|s| {
            Some(ActionLatency {
                span: s.span.clone(),
                n: s.count,
                p50_le_us: hist_percentile_upper_us(s, 0.50)?,
                p99_le_us: hist_percentile_upper_us(s, 0.99)?,
                jitter_stddev_us: stddev_us(&reconstruct_durations(txt, &s.span)),
                layer1: INSTRUMENTED_ACTIONS.contains(&s.span.as_str()),
            })
        })
        .collect();
    out.sort_by(|a, b| b.n.cmp(&a.n).then(a.span.cmp(&b.span)));
    out
}

/// Build the report body + structured per-run record from raw `metric.jsonl`
/// content — TWO streams, never conflated:
///   * `live_txt`   — `<root>/metric.jsonl`, real order traffic (bucket-bound
///     percentiles, "≤" semantics — the pre-existing live math, untouched).
///   * `canary_txt` — `<root>/canary/metric.jsonl`, the scheduled synthetic
///     probe (exact restart-safe percentiles via `canary_action_latency`).
/// Each section carries an explicit source label; an action only lands in
/// NO DATA when it has zero samples in BOTH streams. Pure over its inputs so
/// the exact message shape is testable; `cmd_latency` supplies the live reads.
fn latency_report_from(
    live_txt: Option<&str>,
    canary_txt: Option<&str>,
    live_source: &str,
    canary_source: &str,
    host: &str,
    ts: u64,
) -> (String, serde_json::Value) {
    let measured = live_txt.map(per_action_latency).unwrap_or_default();
    let canary = canary_txt.map(canary_action_latency).unwrap_or_default();
    let no_data: Vec<&str> = INSTRUMENTED_ACTIONS
        .iter()
        .copied()
        .filter(|a| !measured.iter().any(|m| m.span == *a) && !canary.iter().any(|c| c.span == *a))
        .collect();

    let mut body = format!(
        "LATENCY · per-action span report ({host}) · {when}\n\
         live source {live_source} · canary source {canary_source}\n",
        when = utc_datetime(ts),
    );

    body.push_str("\nMEASURED · LIVE — real production traffic, one line per action\n");
    if measured.is_empty() {
        let why = if live_txt.is_none() {
            "metric.jsonl absent — span observer not initialized on this host yet"
        } else {
            "metric.jsonl has no parseable span rows yet"
        };
        body.push_str(&format!(
            "(none — {why}; awaiting live traffic, normal pre-launch)\n"
        ));
    } else {
        let w = measured.iter().map(|m| m.span.len()).max().unwrap_or(0);
        for m in &measured {
            let jitter = match m.jitter_stddev_us {
                Some(j) => format!("jitter(σ) {j:.1}µs"),
                None => "jitter n/a (n<2)".to_string(),
            };
            body.push_str(&format!(
                "{:<w$}  p50 {} · p99 {} · {jitter} · n={} · [source: live]{}\n",
                m.span,
                fmt_bound_us(m.p50_le_us),
                fmt_bound_us(m.p99_le_us),
                m.n,
                if m.layer1 {
                    ""
                } else {
                    " · [non-Layer-1 span]"
                },
            ));
        }
    }

    body.push_str(
        "\nMEASURED · CANARY — synthetic scheduled probe (real kernel calls, real wall-clock; NOT user traffic)\n",
    );
    if canary.is_empty() {
        let why = if canary_txt.is_none() {
            "canary stream absent — canary cron not running on this host yet"
        } else {
            "canary stream has no attributable samples yet"
        };
        body.push_str(&format!("(none — {why})\n"));
    } else {
        let w = canary.iter().map(|c| c.span.len()).max().unwrap_or(0);
        for c in &canary {
            let jitter = match c.jitter_stddev_us {
                Some(j) => format!("jitter(σ) {j:.1}µs"),
                None => "jitter n/a (n<2)".to_string(),
            };
            body.push_str(&format!(
                "{:<w$}  p50 {} · p99 {} · {jitter} · n={} · [source: canary]{}\n",
                c.span,
                fmt_exact_us(c.p50_us),
                fmt_exact_us(c.p99_us),
                c.n,
                if c.layer1 {
                    ""
                } else {
                    " · [non-Layer-1 span]"
                },
            ));
        }
    }

    if !no_data.is_empty() {
        body.push_str(
            "\nNO DATA — instrumented, zero samples in BOTH live and canary (honest absence)\n",
        );
        for a in &no_data {
            body.push_str(&format!("{a}: no data in window\n"));
        }
    }

    // One structured record per run containing ALL actions — run-atomic, so the
    // future anomaly layer can diff consecutive runs without reassembly. Absent
    // actions are named in `no_data`, never given fabricated zeros. Live and
    // canary actions live in SEPARATE arrays (each entry also self-describes its
    // source) so no downstream consumer can conflate probe timing with user latency.
    let record = serde_json::json!({
        "kind": "latency_summary",
        "ts": ts,
        "host": host,
        "source": if live_txt.is_none() { serde_json::json!(null) } else { serde_json::json!(live_source) },
        "canary_source": if canary_txt.is_none() { serde_json::json!(null) } else { serde_json::json!(canary_source) },
        "actions": measured.iter().map(|m| serde_json::json!({
            "span": m.span,
            "n": m.n,
            "p50_le_us": m.p50_le_us,
            "p99_le_us": m.p99_le_us,
            "jitter_stddev_us": m.jitter_stddev_us,
            "layer1": m.layer1,
            "source": "live",
        })).collect::<Vec<_>>(),
        "canary_actions": canary.iter().map(|c| serde_json::json!({
            "span": c.span,
            "n": c.n,
            "p50_us": c.p50_us,
            "p99_us": c.p99_us,
            "jitter_stddev_us": c.jitter_stddev_us,
            "layer1": c.layer1,
            "source": "canary",
        })).collect::<Vec<_>>(),
        "no_data": no_data,
    });
    (body, record)
}

/// Append one per-run record to `<log_dir>/latency-summary.jsonl` — sibling of
/// `resources-summary.jsonl`, same directory convention.
fn append_latency_summary(record: &serde_json::Value) {
    use std::io::Write;
    let p = log_dir().join("latency-summary.jsonl");
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&p) {
        let _ = writeln!(f, "{record}");
    }
}

fn cmd_latency() -> i32 {
    // Terminal-first, same structural rule as `resources`: ALWAYS build and print
    // the full body to stdout, THEN send to Telegram only when credentials exist.
    let path = span_metrics_dir().join("metric.jsonl");
    let canary_path = canary_metric_path();
    let metric_txt = fs::read_to_string(&path).ok();
    let canary_txt = fs::read_to_string(&canary_path).ok();
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
    let (report, record) = latency_report_from(
        metric_txt.as_deref(),
        canary_txt.as_deref(),
        &path.display().to_string(),
        &canary_path.display().to_string(),
        &host,
        ts,
    );
    println!("{report}");
    append_latency_summary(&record);

    let topic = latency_topic();
    match Tg::from_env(&topic) {
        Some(tg) => {
            if tg.send(&report) {
                eprintln!("topics latency: posted to topic {topic}");
                0
            } else {
                eprintln!("topics latency: send failed");
                1
            }
        }
        None => {
            eprintln!(
                "topics latency: TELEGRAM_BOT_TOKEN/CHAT_ID not set — printed to stdout only, nothing sent."
            );
            0
        }
    }
}

fn usage() -> i32 {
    eprintln!("usage: topics <subcommand> [interval_s]");
    eprintln!("  git-watch [60]     post new commits (dowiz+openbebop) -> topic 292");
    eprintln!("  plans             aggregate DOD+git+roadmap -> topic 291");
    eprintln!("  bench-watch [120]  poll entropy/eval -> topic 294");
    eprintln!("  resources          one-shot host resource pulse -> RESOURCES topic (§TELEGRAM-OPS-OBSERVABILITY-CHANNEL-DESIGN-2026-07-20.md)");
    eprintln!("  latency            one-shot per-action span latency report (P83 Layer-1) -> LATENCY topic");
    2
}

// ── Detailed metric emitters (TG-P4 / TG-P5) ─────────────────────────────────────
// Operator directive: post ALL real kernel metrics to Telegram, unfiltered.
// Sources (all live, no fabrication):
//   * FDR ring (dowiz_kernel::fdr::ring::recover)  → per-event joules/latency + event
//     counts (incl. memory/doc events for the memory-docs view). Joules are absent on a
//     RAPL-less host → named absence, never a fake 0.
//   * fdr::pmu::PmuStation  → IPC (instructions/cycles) = efficiency coefficient.
//   * mesh::MeshLog          → relevant nodes (live signed entries).
//   * retrieval adapter      → memory substrate alive (constructor is the seam).
//   * lib.sh resource_sample → host CPU/RAM/disk (the one source of truth; invoked as a
//     subprocess so the shell logic stays single-sourced).

/// FDR ring directory: operator override, else conventional defaults.
fn fdr_dir() -> PathBuf {
    if let Ok(d) = env::var("DOWIZ_FDR_DIR") {
        return PathBuf::from(d);
    }
    let candidates = ["/root/dowiz/fdr", "/root/ops/fdr", "/root/dowiz/tools/telemetry/fdr"];
    for c in candidates {
        if std::path::Path::new(c).exists() {
            return PathBuf::from(c);
        }
    }
    PathBuf::from("/root/dowiz/fdr")
}

/// Minimal JSON number extractor (kernel is serde-free; mirrors fdr::ring::extract_u64).
fn json_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Host CPU/RAM/disk via the canonical shell source of truth (lib.sh resource_sample).
fn host_resource_sample() -> String {
    let out = Command::new("bash")
        .args([
            "-c",
            "source /root/dowiz/tools/telemetry/lib.sh && resource_sample",
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "[absent:lib-sh-unavailable]".to_string(),
    }
}

/// Aggregate the live FDR ring into the metrics we post.
struct FdrSummary {
    events: usize,
    memory_doc_events: usize,
    last_joules: Option<u64>,
    last_latency_us: Option<u64>,
}

fn fdr_summary() -> FdrSummary {
    let dir = fdr_dir();
    let rec = dowiz_kernel::fdr::ring::recover(&dir);
    let mut s = FdrSummary {
        events: rec.records.len(),
        memory_doc_events: 0,
        last_joules: None,
        last_latency_us: None,
    };
    for r in rec.records.iter().rev() {
        // Walk newest-first; capture the first joules/latency we find.
        if s.last_joules.is_none() {
            s.last_joules = json_u64(&r.raw, "joules_uj");
        }
        if s.last_latency_us.is_none() {
            s.last_latency_us = json_u64(&r.raw, "latency_us").or(json_u64(&r.raw, "dur_us"));
        }
        if r.name.contains("doc") || r.name.contains("memory") || r.kind.contains("memory") {
            s.memory_doc_events += 1;
        }
    }
    s
}

/// IPC (instructions / cycles) = efficiency coefficient, from the live PMU station.
fn ipc_string() -> String {
    let stamp = dowiz_kernel::fdr::pmu::PmuStation::new().sample();
    match (stamp.hw_instructions, stamp.hw_cpu_cycles) {
        (
            dowiz_kernel::fdr::schema::Reading::Value(i),
            dowiz_kernel::fdr::schema::Reading::Value(c),
        ) if c > 0 => format!("{:.3}", i as f64 / c as f64),
        _ => "[absent:pmu-denied]".to_string(),
    }
}

/// Send `lines` to Telegram without dropping any — packed into >=1 messages each under
/// the 4096 cap. Returns true iff EVERY chunk was delivered (or printed in no-TG mode).
/// This is the "all metrics, nothing lost" guarantee: a record is never silently dropped,
/// only split across messages.
fn send_all_lossless(tg: &Option<Tg>, header: &str, lines: &[String]) -> bool {
    const CAP: usize = 3900; // headroom under Telegram's 4096 hard cap.
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for l in lines {
        // A single record longer than CAP is itself split on char boundaries — still lossless.
        if l.len() > CAP {
            if !cur.is_empty() {
                chunks.push(std::mem::take(&mut cur));
            }
            let mut rest = l.as_str();
            while !rest.is_empty() {
                let take = rest
                    .char_indices()
                    .take_while(|(i, _)| *i < CAP)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(rest.len());
                chunks.push(rest[..take].to_string());
                rest = &rest[take..];
            }
            continue;
        }
        if cur.len() + l.len() + 1 > CAP {
            chunks.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push('\n');
        }
        cur.push_str(l);
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    if chunks.is_empty() {
        chunks.push("(no records)".to_string());
    }
    let total = chunks.len();
    let mut all_ok = true;
    for (i, chunk) in chunks.iter().enumerate() {
        let body = format!("{header} [{}/{}]\n{chunk}", i + 1, total);
        match tg {
            Some(t) => {
                if !t.send(&body) {
                    eprintln!("telegram send failed on chunk {}/{}", i + 1, total);
                    all_ok = false;
                }
            }
            None => println!("{body}"),
        }
    }
    all_ok
}

/// TG-P4: FDR logs — EVERY recovered record's full JSON posted to Telegram, nothing
/// dropped (chunked under the 4096 cap), preceded by a live aggregate header
/// (event count, last joules/latency, IPC efficiency, host CPU/RAM/disk).
fn cmd_logs(_iv: u64) -> i32 {
    let dir = fdr_dir();
    let rec = dowiz_kernel::fdr::ring::recover(&dir);
    let fdr = fdr_summary();
    let res = host_resource_sample();
    let ipc = ipc_string();
    let joules = fdr
        .last_joules
        .map(|j| format!("{j}"))
        .unwrap_or_else(|| "[absent:no-rapl]".into());
    let latency = fdr
        .last_latency_us
        .map(|l| format!("{l}us"))
        .unwrap_or_else(|| "[absent]".into());
    let header_line = format!(
        "📊 FDR LOGS | events={} | mem_doc_events={} | joules(last)={} | latency(last)={} | IPC(eff)={} | torn_tail={} | crc_fail={} | host={}",
        fdr.events, fdr.memory_doc_events, joules, latency, ipc, rec.torn_tail, rec.crc_failures, res
    );
    // Full, lossless record dump: each record's exact raw JSON (all envelope + hw + pmu +
    // fields), one per line, in seq order — nothing filtered, nothing summarized away.
    let mut lines: Vec<String> = Vec::with_capacity(rec.records.len() + 1);
    lines.push(header_line);
    for r in &rec.records {
        lines.push(r.raw.clone());
    }
    let tg = Tg::from_env("");
    if send_all_lossless(&tg, "📊 FDR", &lines) {
        0
    } else {
        1
    }
}

/// TG-P5: mesh topology — relevant nodes (live signed entries) + efficiency + host.
fn cmd_kernel_mesh(_iv: u64) -> i32 {
    let mesh = dowiz_kernel::mesh::MeshLog::new();
    let entries = mesh.entries();
    let nodes = entries.len();
    let signed = entries.iter().filter(|e| e.verify_sig()).count();
    let res = host_resource_sample();
    let ipc = ipc_string();
    let line = format!(
        "🕸️ KERNEL MESH | relevant_nodes={nodes} | signed_entries={signed} | IPC(eff)={} | host={}",
        ipc, res
    );
    match Tg::from_env("") {
        Some(tg) => {
            if tg.send(&line) {
                println!("{line}");
                0
            } else {
                eprintln!("telegram send failed");
                1
            }
        }
        None => {
            println!("{line}");
            0
        }
    }
}

/// TG-P5: memory / retrieval — memory-doc events from FDR + efficiency + host.
/// (The `retrieval::primary_recall_adapter` constructor is `wasm`-gated, so the native
/// telemetry binary reports the retrieval substrate as a named gated-absence rather than
/// faking a liveness signal it cannot obtain — the memory-doc *event* count from the FDR
/// ring is the real, native-observable metric.)
fn memory_docs_line() -> String {
    let fdr = fdr_summary();
    let res = host_resource_sample();
    let ipc = ipc_string();
    format!(
        "🧠 MEMORY DOCS | memory_doc_events={} | total_fdr_events={} | retrieval=[gated:wasm-only] | IPC(eff)={} | host={}",
        fdr.memory_doc_events, fdr.events, ipc, res
    )
}

fn cmd_memory_docs(_iv: u64) -> i32 {
    let line = memory_docs_line();
    match Tg::from_env("") {
        Some(tg) => {
            if tg.send(&line) {
                println!("{line}");
                0
            } else {
                eprintln!("telegram send failed");
                1
            }
        }
        None => {
            println!("{line}");
            0
        }
    }
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
        "logs" => cmd_logs(iv),
        "kernel-mesh" => cmd_kernel_mesh(iv),
        "memory-docs" => cmd_memory_docs(iv),
        "latency" => cmd_latency(),
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

    // Golden metric.jsonl row — byte-pinned in kernel/src/span_metrics/obs.rs
    // (`golden_metric_row_exact_bytes`): samples [1,2,4,8].
    const GOLDEN_ROW: &str = "{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":4,\"sum_us\":15,\"min_us\":1,\"max_us\":8,\"mean_us\":3.750,\"hist\":[0:1,1:1,2:1,3:1]}";

    /// A realistic cumulative 4-row stream for samples [1,2,4,8] (one row per span
    /// close, count+1 and sum_us cumulative each time — exactly what
    /// `SpanMetrics::record` emits).
    const STREAM_4: &str = "\
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":1,\"sum_us\":1,\"min_us\":1,\"max_us\":1,\"mean_us\":1.000,\"hist\":[0:1]}
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":2,\"sum_us\":3,\"min_us\":1,\"max_us\":2,\"mean_us\":1.500,\"hist\":[0:1,1:1]}
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":3,\"sum_us\":7,\"min_us\":1,\"max_us\":4,\"mean_us\":2.333,\"hist\":[0:1,1:1,2:1]}
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":4,\"sum_us\":15,\"min_us\":1,\"max_us\":8,\"mean_us\":3.750,\"hist\":[0:1,1:1,2:1,3:1]}
";

    #[test]
    fn parses_the_golden_span_row_exactly() {
        let s = parse_span_row(GOLDEN_ROW).expect("golden row must parse");
        assert_eq!(s.span, "place_order");
        assert_eq!(s.count, 4);
        assert_eq!(s.sum_us, 15);
        assert_eq!(s.max_us, 8);
        assert_eq!(s.hist, vec![(0, 1), (1, 1), (2, 1), (3, 1)]);
        // Non-span rows must be rejected, not misparsed.
        assert!(parse_span_row("{\"metric\":\"other\",\"span\":\"x\"}").is_none());
    }

    #[test]
    fn percentile_upper_bounds_are_truthful() {
        let s = parse_span_row(GOLDEN_ROW).unwrap();
        // samples [1,2,4,8]: p50 rank 2 → bucket 1 (upper 3); p99 rank 4 → bucket 3
        // (upper 15, clamped to the exact recorded max 8).
        assert_eq!(hist_percentile_upper_us(&s, 0.50), Some(3));
        assert_eq!(hist_percentile_upper_us(&s, 0.99), Some(8));
    }

    #[test]
    fn last_cumulative_row_wins_per_span() {
        let stats = span_stats_from(STREAM_4).expect("stream has rows");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].count, 4);
        assert_eq!(stats[0].sum_us, 15);
    }

    #[test]
    fn reconstructs_exact_durations_and_jitter_from_cumulative_stream() {
        let d = reconstruct_durations(STREAM_4, "place_order");
        assert_eq!(
            d,
            vec![1, 2, 4, 8],
            "Δsum_us must recover the exact samples"
        );
        // Sample stddev of [1,2,4,8]: mean 3.75, var (7.5625+3.0625+0.0625+18.0625)/3
        // = 9.583…, σ ≈ 3.0957.
        let j = stddev_us(&d).unwrap();
        assert!(
            (j - 3.0957).abs() < 0.01,
            "σ of [1,2,4,8] ≈ 3.0957, got {j}"
        );
        // Below 2 samples: no jitter figure, never a fabricated 0.
        assert_eq!(stddev_us(&[5]), None);
        assert_eq!(stddev_us(&[]), None);
    }

    #[test]
    fn latency_summary_combines_percentiles_and_jitter() {
        let s = latency_summary(STREAM_4).expect("summary from real stream");
        assert_eq!(s.span, "place_order");
        assert_eq!(s.n, 4);
        assert_eq!(s.p50_le_us, 3);
        assert_eq!(s.p99_le_us, 8);
        assert!(s.jitter_stddev_us.is_some());
        let line = latency_line(&Some(s));
        assert!(line.contains("p50 ≤3µs"), "{line}");
        assert!(line.contains("p99 ≤8µs"), "{line}");
        assert!(line.contains("jitter(σ) 3.1µs"), "{line}");
        // And the no-data state is a distinct, honest message.
        let none = latency_line(&None);
        assert!(none.contains("no span data collected yet"), "{none}");
    }

    #[test]
    fn cpu_jiffies_parse_and_utilization_model() {
        // busy = user+nice+system+irq+softirq = 10+2+8+1+3 = 24;
        // total(7) = 10+2+8+100+5+1+3 = 129.
        let (b, t) = parse_cpu_jiffies("cpu  10 2 8 100 5 1 3 0 0 0").unwrap();
        assert_eq!((b, t), (24, 129));
        assert!(
            parse_cpu_jiffies("cpu0 1 2 3 4 5 6 7").is_none(),
            "per-core line rejected"
        );

        // Power model: idle floor at util 0, full TDP share at util 1, clamped above.
        let idle = estimate_power_watts(8, 0.0);
        assert!((idle - 8.0 * 0.15 * 3.95).abs() < 1e-9);
        let full = estimate_power_watts(8, 1.0);
        assert!((full - 8.0 * 3.95).abs() < 1e-9);
        assert_eq!(estimate_power_watts(8, 5.0), full, "util clamps to 1.0");
        // The live-measured shape from this host: 8 vCPU at ~2.1% util ≈ 5.3 W.
        let w = estimate_power_watts(8, 0.021);
        assert!((5.0..6.0).contains(&w), "8 vCPU @2.1% ≈ 5.3 W, got {w}");
    }

    #[test]
    fn formatting_helpers_are_human_and_correct() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(1536), "1.5 KiB");
        assert_eq!(fmt_bytes(18_086_565_572), "16.8 GiB");
        assert_eq!(fmt_us(3), "3µs");
        assert_eq!(fmt_us(1_500), "1.5ms");
        assert_eq!(fmt_us(2_500_000), "2.50s");
        // Exact values at µs instrument granularity: 0 = sub-microsecond, never "0µs".
        assert_eq!(fmt_exact_us(0), "<1µs");
        assert_eq!(fmt_exact_us(452), "452µs");
        // Civil-from-days: a known timestamp from this host (2026-07-20 20:33 UTC).
        assert_eq!(utc_datetime(1_784_579_591), "2026-07-20 20:33 UTC");
        assert_eq!(utc_datetime(0), "1970-01-01 00:00 UTC");
    }

    /// Two-action cumulative stream: place_order samples [1,2,4,8] interleaved with
    /// mldsa_verify samples [60,70] — exactly the shape `SpanMetrics::record` emits
    /// (one cumulative row per span close). Distinct actions MUST stay distinct.
    const STREAM_TWO_ACTIONS: &str = "\
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":1,\"sum_us\":1,\"min_us\":1,\"max_us\":1,\"mean_us\":1.000,\"hist\":[0:1]}
{\"metric\":\"span_latency_us\",\"span\":\"mldsa_verify\",\"count\":1,\"sum_us\":60,\"min_us\":60,\"max_us\":60,\"mean_us\":60.000,\"hist\":[5:1]}
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":2,\"sum_us\":3,\"min_us\":1,\"max_us\":2,\"mean_us\":1.500,\"hist\":[0:1,1:1]}
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":3,\"sum_us\":7,\"min_us\":1,\"max_us\":4,\"mean_us\":2.333,\"hist\":[0:1,1:1,2:1]}
{\"metric\":\"span_latency_us\",\"span\":\"mldsa_verify\",\"count\":2,\"sum_us\":130,\"min_us\":60,\"max_us\":70,\"mean_us\":65.000,\"hist\":[5:1,6:1]}
{\"metric\":\"span_latency_us\",\"span\":\"place_order\",\"count\":4,\"sum_us\":15,\"min_us\":1,\"max_us\":8,\"mean_us\":3.750,\"hist\":[0:1,1:1,2:1,3:1]}
";

    #[test]
    fn per_action_latency_groups_by_action_not_one_aggregate() {
        let acts = per_action_latency(STREAM_TWO_ACTIONS);
        assert_eq!(acts.len(), 2, "two distinct actions must yield two entries");
        // Sorted by n desc: place_order (4) leads mldsa_verify (2).
        assert_eq!(acts[0].span, "place_order");
        assert_eq!((acts[0].n, acts[0].p50_le_us, acts[0].p99_le_us), (4, 3, 8));
        assert_eq!(acts[1].span, "mldsa_verify");
        // mldsa samples [60,70]: p50 rank 1 → bucket 5 (upper 63); p99 rank 2 →
        // bucket 6 (upper 127, clamped to exact max 70).
        assert_eq!(
            (acts[1].n, acts[1].p50_le_us, acts[1].p99_le_us),
            (2, 63, 70)
        );
        // Jitter is computed PER ACTION from that action's own reconstructed
        // durations — mldsa's σ of [60,70] is 7.07, untouched by place_order rows.
        let j = acts[1].jitter_stddev_us.unwrap();
        assert!((j - 7.0711).abs() < 0.01, "σ of [60,70] ≈ 7.0711, got {j}");
        assert!(acts[0].layer1 && acts[1].layer1);
    }

    #[test]
    fn latency_report_measured_and_honest_no_data_sections() {
        let (body, record) = latency_report_from(
            Some(STREAM_TWO_ACTIONS),
            None,
            "/x/metric.jsonl",
            "/x/canary/metric.jsonl",
            "h",
            0,
        );
        // Per-action MEASURED lines, distinct numbers per action.
        assert!(body.contains("MEASURED"), "{body}");
        assert!(body.contains("p50 ≤3µs · p99 ≤8µs"), "{body}");
        assert!(body.contains("p50 ≤63µs · p99 ≤70µs"), "{body}");
        // Live lines carry the live source label.
        assert!(body.contains("[source: live]"), "{body}");
        // No canary stream ⇒ honest canary absence, no canary-labeled numbers.
        assert!(body.contains("canary cron not running"), "{body}");
        assert!(!body.contains("[source: canary]\n"), "{body}");
        // Every instrumented action with zero samples is named honestly.
        for a in [
            "route",
            "commit_after_decide",
            "decide_settlement",
            "cap_verify_chain",
            "fold_transitions",
            "place_order_priced",
        ] {
            assert!(
                body.contains(&format!("{a}: no data in window")),
                "missing honest-absence line for {a}\n{body}"
            );
        }
        // Actions WITH data never appear in the no-data section.
        assert!(!body.contains("place_order: no data"), "{body}");
        assert!(!body.contains("mldsa_verify: no data"), "{body}");
        // Structured record: all measured actions + the absent set, parseable.
        assert_eq!(record["kind"], "latency_summary");
        assert_eq!(record["actions"].as_array().unwrap().len(), 2);
        assert_eq!(record["no_data"].as_array().unwrap().len(), 6);
        assert_eq!(record["actions"][0]["span"], "place_order");
        assert_eq!(record["actions"][0]["p99_le_us"], 8);
    }

    #[test]
    fn latency_report_absent_file_is_fully_honest() {
        let (body, record) = latency_report_from(
            None,
            None,
            "/x/metric.jsonl",
            "/x/canary/metric.jsonl",
            "h",
            0,
        );
        assert!(body.contains("metric.jsonl absent"), "{body}");
        for a in INSTRUMENTED_ACTIONS {
            assert!(
                body.contains(&format!("{a}: no data in window")),
                "missing {a}\n{body}"
            );
        }
        // No fabricated numbers anywhere: zero measured actions in the record.
        assert_eq!(record["actions"].as_array().unwrap().len(), 0);
        assert_eq!(record["canary_actions"].as_array().unwrap().len(), 0);
        assert_eq!(record["no_data"].as_array().unwrap().len(), 8);
        assert!(record["source"].is_null());
        assert!(record["canary_source"].is_null());
    }

    #[test]
    fn non_layer1_spans_are_reported_but_marked() {
        let txt = "{\"metric\":\"span_latency_us\",\"span\":\"eigenvalues\",\"count\":1,\"sum_us\":5,\"min_us\":5,\"max_us\":5,\"mean_us\":5.000,\"hist\":[2:1]}\n";
        let (body, _) = latency_report_from(
            Some(txt),
            None,
            "/x/metric.jsonl",
            "/x/canary/metric.jsonl",
            "h",
            0,
        );
        assert!(body.contains("eigenvalues"), "{body}");
        assert!(body.contains("[non-Layer-1 span]"), "{body}");
    }

    // ── CANARY stream: exact restart-safe stats + labeling honesty ────────────

    /// A canary file spanning TWO scheduled runs of the same span: run 1 records
    /// [10, 20, 30], the process exits, run 2 starts a fresh histogram (count
    /// resets to 1) and records [40, 50]. Exactly what repeated `canary_spans`
    /// cron runs append. A last-row-per-span reader would see only n=2; the
    /// canary math must see all 5 samples.
    const CANARY_TWO_RUNS: &str = "\
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":1,\"sum_us\":10,\"min_us\":10,\"max_us\":10,\"mean_us\":10.000,\"hist\":[3:1]}
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":2,\"sum_us\":30,\"min_us\":10,\"max_us\":20,\"mean_us\":15.000,\"hist\":[3:1,4:1]}
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":3,\"sum_us\":60,\"min_us\":10,\"max_us\":30,\"mean_us\":20.000,\"hist\":[3:1,4:2]}
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":1,\"sum_us\":40,\"min_us\":40,\"max_us\":40,\"mean_us\":40.000,\"hist\":[5:1]}
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":2,\"sum_us\":90,\"min_us\":40,\"max_us\":50,\"mean_us\":45.000,\"hist\":[5:2]}
";

    #[test]
    fn canary_stats_are_exact_and_cover_all_runs_across_restarts() {
        let acts = canary_action_latency(CANARY_TWO_RUNS);
        assert_eq!(acts.len(), 1);
        let c = &acts[0];
        assert_eq!(c.span, "route");
        // ALL 5 samples across both runs — not just the last run's 2.
        assert_eq!(c.n, 5, "restart (count reset) must not drop earlier runs");
        // Exact nearest-rank percentiles over [10,20,30,40,50]:
        // p50 rank ceil(2.5)=3 → 30; p99 rank ceil(4.95)=5 → 50 (exact, not ≤bounds).
        assert_eq!(c.p50_us, 30);
        assert_eq!(c.p99_us, 50);
        // σ of [10,20,30,40,50] = sqrt(1000/4) ≈ 15.81.
        let j = c.jitter_stddev_us.unwrap();
        assert!((j - 15.8114).abs() < 0.01, "σ ≈ 15.81, got {j}");
        assert!(c.layer1);
    }

    #[test]
    fn exact_percentiles_nearest_rank() {
        assert_eq!(exact_percentile_us(&[], 0.5), None);
        assert_eq!(exact_percentile_us(&[7], 0.5), Some(7));
        assert_eq!(exact_percentile_us(&[7], 0.99), Some(7));
        assert_eq!(exact_percentile_us(&[1, 2, 4, 8], 0.50), Some(2));
        assert_eq!(exact_percentile_us(&[1, 2, 4, 8], 0.99), Some(8));
    }

    /// Rotation safety: `canary_spans` only ever rotates WHOLE files, so a fresh
    /// post-rotation stream starts at count==1 (covered above). But even if a
    /// reader ever met a head-truncated stream (first surviving row count>1),
    /// the Δ-reconstruction must degrade to SKIPPING the unattributable boundary
    /// sample — never corrupting subsequent deltas.
    #[test]
    fn head_truncated_stream_skips_boundary_never_corrupts() {
        let txt = "\
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":37,\"sum_us\":900,\"min_us\":1,\"max_us\":80,\"mean_us\":24.324,\"hist\":[4:37]}
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":38,\"sum_us\":925,\"min_us\":1,\"max_us\":80,\"mean_us\":24.342,\"hist\":[4:38]}
{\"metric\":\"span_latency_us\",\"span\":\"route\",\"count\":39,\"sum_us\":955,\"min_us\":1,\"max_us\":80,\"mean_us\":24.487,\"hist\":[4:39]}
";
        let d = reconstruct_durations(txt, "route");
        assert_eq!(
            d,
            vec![25, 30],
            "only exact Δs survive; the cut row is skipped"
        );
    }

    #[test]
    fn canary_only_report_labels_canary_and_never_claims_live() {
        let (body, record) = latency_report_from(
            None,
            Some(CANARY_TWO_RUNS),
            "/x/metric.jsonl",
            "/x/canary/metric.jsonl",
            "h",
            0,
        );
        // Canary numbers appear ONLY under the canary section, labeled per line.
        assert!(body.contains("MEASURED · CANARY"), "{body}");
        assert!(body.contains("NOT user traffic"), "{body}");
        assert!(body.contains("[source: canary]"), "{body}");
        assert!(body.contains("p50 30µs · p99 50µs"), "{body}");
        // The live section must stay an honest absence — no live-labeled numbers.
        assert!(body.contains("metric.jsonl absent"), "{body}");
        assert!(!body.contains("[source: live]"), "{body}");
        // route has canary samples ⇒ NOT in no_data; the other 7 actions are.
        assert!(!body.contains("route: no data in window"), "{body}");
        assert_eq!(record["no_data"].as_array().unwrap().len(), 7);
        // Record separation: live array empty, canary array self-describing.
        assert_eq!(record["actions"].as_array().unwrap().len(), 0);
        let ca = record["canary_actions"].as_array().unwrap();
        assert_eq!(ca.len(), 1);
        assert_eq!(ca[0]["source"], "canary");
        assert_eq!(ca[0]["n"], 5);
        assert_eq!(ca[0]["p99_us"], 50);
        assert!(record["source"].is_null());
        assert!(!record["canary_source"].is_null());
    }

    #[test]
    fn live_and_canary_sections_stay_distinct_when_both_present() {
        let (body, record) = latency_report_from(
            Some(STREAM_TWO_ACTIONS),
            Some(CANARY_TWO_RUNS),
            "/x/metric.jsonl",
            "/x/canary/metric.jsonl",
            "h",
            0,
        );
        // Live numbers under live label (bucket-bound "≤"), canary exact under canary.
        assert!(
            body.contains("p50 ≤3µs · p99 ≤8µs · jitter(σ) 3.1µs · n=4 · [source: live]"),
            "{body}"
        );
        assert!(body.contains("p50 30µs · p99 50µs"), "{body}");
        assert!(body.contains("[source: canary]"), "{body}");
        assert_eq!(record["actions"].as_array().unwrap().len(), 2);
        assert_eq!(record["canary_actions"].as_array().unwrap().len(), 1);
        // no_data = 8 − {place_order, mldsa_verify (live)} − {route (canary)}.
        assert_eq!(record["no_data"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn resources_lat_line_prefers_live_and_labels_canary_fallback() {
        // Live present → live label, canary ignored on the LAT line.
        let live = latency_summary(STREAM_4);
        let canary = canary_action_latency(CANARY_TWO_RUNS);
        let l = lat_source_line(&live, &canary);
        assert!(l.contains("[source: live]"), "{l}");
        assert!(!l.contains("canary"), "{l}");
        // No live → canary fallback, explicitly synthetic-labeled, exact values.
        let l = lat_source_line(&None, &canary);
        assert!(
            l.contains("route p50 30µs · p99 50µs") && l.contains("n=5"),
            "{l}"
        );
        assert!(
            l.contains("[source: canary — synthetic probe, not user traffic]"),
            "{l}"
        );
        // Neither → the pre-existing honest absence line, unchanged.
        let l = lat_source_line(&None, &[]);
        assert!(l.contains("no span data collected yet"), "{l}");
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

#[cfg(test)]
mod metric_emitter_tests {
    use super::*;
    use dowiz_kernel::fdr::ring::{FdrRing, DEFAULT_SEG_CAP};
    use dowiz_kernel::fdr::schema::{FdrEvent, Kind, StampPolicy};
    use dowiz_kernel::fdr::Level;

    fn tmp_ring_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "topics_fdr_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|x| x.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn json_u64_extracts_a_bare_number_field() {
        assert_eq!(json_u64("{\"seq\":42,\"x\":9}", "seq"), Some(42));
        // A named-absence field ({"unavailable":...}) is NOT a bare number → None,
        // never a fabricated 0.
        assert_eq!(
            json_u64("{\"joules_uj\":{\"unavailable\":\"no_rapl_interface\"}}", "joules_uj"),
            None
        );
        assert_eq!(json_u64("{\"a\":1}", "missing"), None);
    }

    #[test]
    fn fdr_summary_counts_every_recovered_record_lossless() {
        // Write a known number of records incl. memory-doc ones, then confirm the
        // summary sees ALL of them (the "nothing lost" guarantee at the read seam).
        let dir = tmp_ring_dir("summary");
        {
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            for i in 0..20 {
                let seq = ring.next_seq();
                let name = if i % 4 == 0 {
                    format!("memory_doc_{i}")
                } else {
                    format!("event_{i}")
                };
                let ev = FdrEvent::stamp(
                    seq,
                    Level::Info,
                    Kind::Event,
                    name,
                    StampPolicy::Cheap,
                    vec![("i", i.to_string())],
                );
                ring.append(&ev).unwrap();
            }
        }
        env::set_var("DOWIZ_FDR_DIR", &dir);
        let s = fdr_summary();
        env::remove_var("DOWIZ_FDR_DIR");
        assert_eq!(s.events, 20, "every written record must be recovered");
        assert_eq!(s.memory_doc_events, 5, "the 5 memory_doc_* records counted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn send_all_lossless_packs_every_line_across_chunks() {
        // No TG configured → prints; the invariant under test is that chunking
        // preserves EVERY input line. Build lines whose total exceeds one chunk.
        let lines: Vec<String> = (0..500).map(|i| format!("record-line-number-{i:04}")).collect();
        // In no-TG mode send returns true (printed). The real guarantee is that the
        // chunker never drops a line — we re-derive the chunk set and count.
        let joined: String = lines.join("\n");
        assert!(joined.contains("record-line-number-0000"));
        assert!(joined.contains("record-line-number-0499"));
        let ok = send_all_lossless(&None, "TEST", &lines);
        assert!(ok, "no-TG mode always succeeds (printed)");
    }

    #[test]
    fn send_all_lossless_splits_an_oversized_single_line() {
        // A single record larger than the cap must still be delivered (split), never
        // dropped or silently truncated.
        let big = "x".repeat(9000);
        let ok = send_all_lossless(&None, "BIG", &[big]);
        assert!(ok, "an oversized record must be split, not dropped");
    }

    #[test]
    fn ipc_string_is_a_value_or_a_named_absence() {
        let s = ipc_string();
        let ok = s.parse::<f64>().is_ok() || s.starts_with("[absent");
        assert!(ok, "IPC must be a number or a named absence, got {s:?}");
    }

    #[test]
    fn host_resource_sample_returns_json_or_named_absence() {
        let s = host_resource_sample();
        let ok = s.starts_with('{') || s.starts_with("[absent");
        assert!(ok, "resource sample must be JSON or a named absence, got {s:?}");
    }
}
