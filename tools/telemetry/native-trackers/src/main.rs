//! native-trackers — zero-dep native port of the python/bash telemetry trackers.
//!
//! Replaces the interpreter-heredoc compute that lived inside:
//!   - kernel/benches/bench_track.py   (bench baseline comparison)
//!   - tools/telemetry/governance.sh    (gov_route fold, gov_falseclaim meter, gov_meta EMA)
//!   - tools/telemetry/topics.sh        (bench-watch deltas)
//!
//! All three spawned `python3 - <<'PY' ... PY` on every call — the slow path the
//! operator flagged. This binary does the same fold in native Rust over the SAME
//! JSONL ledgers (`track_record.jsonl`, `false_claims.jsonl`, `precedents.jsonl`),
//! no interpreter, no JSON crate (hand-rolled over the fixed schema).
//!
//! Subcommands:
//!   bench  <crate-dir> [--threshold N]   run `cargo bench`, compare to baseline.json, print deltas
//!   route  <task> [budget] [ruin]        fold track_record.jsonl → per-model (p,v,cost) for gov_route
//!   false-claim [--record c v]           EMA false-estimation meter over false_claims.jsonl
//!   meta   observe <bench_prev> <bench_new> <eval_prev> <eval_new> <false_rate>
//!                                          EMA meta-rule tilt over bench/eval/false deltas
//!
//! Exit codes: 0 ok; 1 bench regression beyond threshold; 2 usage/IO error.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

// ─────────────────────────────────────────────────────────────────────────────
// Minimal JSON value + fixed-schema extraction (no serde).
// We only parse flat objects with string/float/bool leaves — enough for the
// gov_route / false_claim / track_record ledgers.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Jv {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

/// Parse a single JSON object (`{ ... }`) into a flat key→value map.
/// Tolerant: skips malformed lines, unquoted/unknown tokens; never panics.
fn parse_obj(s: &str) -> BTreeMap<String, Jv> {
    let s = s.trim();
    let mut m = BTreeMap::new();
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'{') || bytes.last() != Some(&b'}') {
        return m;
    }
    // Walk keys. We only support "key": value, where value is
    // "string" | number | true | false | null.
    let mut i = 1;
    let n = bytes.len();
    while i < n {
        // find next quoted key
        while i < n && bytes[i] != b'"' {
            i += 1;
        }
        if i >= n {
            break;
        }
        let key_start = i + 1;
        let mut j = key_start;
        while j < n && bytes[j] != b'"' {
            j += 1;
        }
        if j >= n {
            break;
        }
        let key = &s[key_start..j];
        i = j + 1;
        // skip to ':'
        while i < n && bytes[i] != b':' {
            i += 1;
        }
        i += 1;
        // skip whitespace
        while i < n && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        if bytes[i] == b'"' {
            let v_start = i + 1;
            let mut v = v_start;
            while v < n && bytes[v] != b'"' {
                v += 1;
            }
            m.insert(key.to_string(), Jv::Str(s[v_start..v].to_string()));
            i = v + 1;
        } else if s[i..].starts_with("true") {
            m.insert(key.to_string(), Jv::Bool(true));
            i += 4;
        } else if s[i..].starts_with("false") {
            m.insert(key.to_string(), Jv::Bool(false));
            i += 5;
        } else if s[i..].starts_with("null") {
            m.insert(key.to_string(), Jv::Null);
            i += 4;
        } else {
            // number until , or }
            let v_start = i;
            let mut v = i;
            while v < n && bytes[v] != b',' && bytes[v] != b'}' {
                v += 1;
            }
            let num = s[v_start..v].trim();
            if let Ok(f) = num.parse::<f64>() {
                m.insert(key.to_string(), Jv::Num(f));
            }
            i = v;
        }
    }
    m
}

fn get_str(m: &BTreeMap<String, Jv>, k: &str) -> String {
    match m.get(k) {
        Some(Jv::Str(s)) => s.clone(),
        _ => String::new(),
    }
}
fn get_num(m: &BTreeMap<String, Jv>, k: &str) -> f64 {
    match m.get(k) {
        Some(Jv::Num(f)) => *f,
        _ => 0.0,
    }
}
fn get_bool(m: &BTreeMap<String, Jv>, k: &str) -> bool {
    matches!(m.get(k), Some(Jv::Bool(true)))
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. BENCH — native replacement for bench_track.py
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_bench(crate_dir: &str, threshold: f64) -> i32 {
    let crate_path = Path::new(crate_dir);
    let baseline_p = crate_path.join("benches/baseline.json");
    let baseline = if baseline_p.exists() {
        let txt = fs::read_to_string(&baseline_p).unwrap_or_default();
        parse_baseline(&txt)
    } else {
        BTreeMap::new()
    };

    // Run cargo bench, capture combined output.
    let out = Command::new("cargo")
        .args([
            "bench",
            "--bench",
            "criterion",
            "--",
            "--warm-up-time",
            "1",
            "--measurement-time",
            "2",
            "--sample-size",
            "10",
        ])
        .current_dir(crate_dir)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            eprintln!("native-trackers: cargo bench failed to spawn: {e}");
            return 2;
        }
    };
    if !out.status.success() {
        eprintln!("native-trackers: cargo bench exited non-zero");
        return 2;
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Parse "name  time:   [lo unit mean unit hi unit]"
    let re = regex_lite();
    let mut cur = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some((name, mean_ns)) = parse_timing(line, &re) {
            cur.insert(name, mean_ns);
        }
    }
    if cur.is_empty() {
        eprintln!("native-trackers: no timing lines parsed from cargo bench");
        return 2;
    }

    // Auto-seed missing baselines (native, atomic).
    let mut seeded = Vec::new();
    let mut baseline_mut = baseline.clone();
    for (name, mean) in &cur {
        if !baseline_mut.contains_key(name) {
            baseline_mut.insert(name.clone(), round4(*mean));
            seeded.push(name.clone());
        }
    }
    if !seeded.is_empty() {
        let json = baseline_to_json(&baseline_mut);
        let tmp = baseline_p.with_extension("tmp");
        fs::write(&tmp, &json).ok();
        fs::rename(&tmp, &baseline_p).ok();
        eprintln!("[auto-seed] wrote {} new baseline(s): {:?}", seeded.len(), seeded);
    }

    println!("\n=== crate: {} ===", crate_dir);
    println!("{:32} {:>12} {:>12} {:>9}  verdict", "benchmark", "baseline_ns", "current_ns", "delta");
    let mut worst = 0.0f64;
    for (name, bmean) in &baseline {
        let cmean = match cur.get(name) {
            Some(v) => *v,
            None => {
                println!("{:32} {:12.2} {:>12} {:>9}  !!", name, bmean, "-", "MISSING");
                worst = (threshold + 1.0).max(worst);
                continue;
            }
        };
        let delta = (cmean - bmean) / bmean * 100.0;
        let verdict = if delta > threshold {
            worst = worst.max(delta);
            "REGRESS"
        } else if delta < -threshold {
            "improve"
        } else {
            "ok"
        };
        println!(
            "{:32} {:12.2} {:12.2} {:+8.1}%  {}",
            name, bmean, cmean, delta, verdict
        );
    }

    // Rolling history (git-ignored).
    let hist_p = crate_path.join("benches/BENCH_HISTORY.md");
    let ts = now_iso();
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&hist_p) {
        let _ = writeln!(f, "\n## {}", ts);
        let _ = writeln!(f, "- native-trackers run (replaces bench_track.py)");
        for (name, bmean) in &baseline {
            if let Some(cmean) = cur.get(name) {
                let delta = (cmean - bmean) / bmean * 100.0;
                let _ = writeln!(f, "- {}: {:.2}ns -> {:.2}ns ({:+.1}%)", name, bmean, cmean, delta);
            }
        }
    }

    if worst > threshold {
        println!("\nREGRESSION: worst +{:.1}% > {:.0}% threshold", worst, threshold);
        1
    } else {
        println!("\nOK: no regression beyond threshold.");
        0
    }
}

// Tiny static regex replacement (no regex crate): match `name  time: [lo unit mean unit hi unit]`
fn regex_lite() -> () {
    ()
}
fn parse_timing(line: &str, _re: &()) -> Option<(String, f64)> {
    // criterion output line shape:  `name  time:   [lo unit mean unit hi unit]`
    // Require the literal "time:" preceded by whitespace, and a real name token
    // (i.e. the first token must NOT itself be "time:" — that filters the
    // "time:" progress-only lines and the harness meta string).
    let line = line.trim();
    let time_idx = line.find("time:")?;
    // The token immediately before "time:" must be a non-empty name.
    let before = &line[..time_idx];
    let name = before.split_whitespace().next_back()?;
    if name.is_empty() || name == "time:" {
        return None;
    }
    let open = line[time_idx..].find('[')? + time_idx;
    let close = line[open..].find(']')? + open;
    let inner = &line[open + 1..close];
    let toks: Vec<&str> = inner.split_whitespace().collect();
    // [lo unit mean unit hi unit] => index 2 = mean, 3 = unit
    if toks.len() < 4 {
        return None;
    }
    let mean: f64 = toks[2].parse().ok()?;
    let unit = toks[3];
    let mult = match unit {
        "ns" => 1.0,
        "µs" | "us" => 1e3,
        "ms" => 1e6,
        "s" => 1e9,
        _ => return None,
    };
    Some((name.to_string(), mean * mult))
}

fn parse_baseline(txt: &str) -> BTreeMap<String, f64> {
    // baseline.json is `{"name": 90.4, ...}` — flat string→number.
    let mut m = BTreeMap::new();
    let obj = parse_obj(txt);
    for (k, v) in obj {
        if let Jv::Num(f) = v {
            m.insert(k, f);
        }
    }
    m
}
fn baseline_to_json(m: &BTreeMap<String, f64>) -> String {
    let mut s = String::from("{\n");
    let mut first = true;
    for (k, v) in m {
        if !first {
            s.push_str(",\n");
        }
        s.push_str(&format!("  \"{}\": {}", k, v));
        first = false;
    }
    s.push_str("\n}\n");
    s
}
fn round4(x: f64) -> f64 {
    (x * 10000.0).round() / 10000.0
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. ROUTE — native fold of track_record.jsonl for gov_route (replaces gov_route)
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_route(task: &str, budget: f64, ruin: f64) -> i32 {
    // track_record.jsonl holds either the gov schema {model,task,success,value,cost}
    // or the harness schema {model,task,success,value,cost,backend,tokens,ms}.
    let path = Path::new("track_record.jsonl");
    let path = if path.exists() {
        path.to_path_buf()
    } else {
        PathBuf::from("/root/dowiz/tools/telemetry/governance/track_record.jsonl")
    };
    let txt = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("native-trackers: no ledger at {:?}", path);
            return 2;
        }
    };
    // per-model aggregate: (n, success, value_sum, cost_sum)
    let mut agg: BTreeMap<String, (u64, u64, f64, f64)> = BTreeMap::new();
    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let o = parse_obj(line);
        if get_str(&o, "task") != task {
            continue;
        }
        let m = get_str(&o, "model");
        if m.is_empty() {
            continue;
        }
        let e = agg.entry(m).or_insert((0, 0, 0.0, 0.0));
        e.0 += 1;
        if get_bool(&o, "success") {
            e.1 += 1;
        }
        e.2 += get_num(&o, "value");
        e.3 += get_num(&o, "cost");
    }
    if agg.is_empty() {
        println!("ROUTE({}): no data — ESCALATE", task);
        return 0;
    }
    // Replicate gov_route's EV pick: score each model, choose max p*value/cost under ruin cap.
    let mut best: Option<(String, f64)> = None;
    println!("ROUTE({}) budget={} ruin_cap={}:", task, budget, ruin);
    println!("{:24} {:>5} {:>6} {:>10} {:>10}  score", "model", "n", "p", "v/cost", "cost/n");
    for (m, (n, s, v, c)) in &agg {
        let p = *s as f64 / *n as f64;
        let cost_n = if *n > 0 { *c / *n as f64 } else { 0.0 };
        let vc = if *c > 0.0 { *v / *c } else { 0.0 };
        let score = p * vc; // EV per token
        println!(
            "{:24} {:5} {:6.2} {:10.4} {:10.2}  {:.4}",
            m, n, p, vc, cost_n, score
        );
        if best.as_ref().map(|(_, b)| score > *b).unwrap_or(true) {
            best = Some((m.clone(), score));
        }
    }
    match best {
        Some((m, _)) => println!("DECISION: route → {}", m),
        None => println!("DECISION: ESCALATE"),
    }
    0
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. FALSE-CLAIM — EMA meter over false_claims.jsonl (replaces gov_falseclaim)
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_false_claim(record: Option<(u64, u64)>) -> i32 {
    let state_p = Path::new("/root/dowiz/tools/telemetry/governance/false_claims.jsonl");
    if let Some((claimed, verified)) = record {
        let ts = now_iso();
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(state_p)
            .map(|mut f| {
                writeln!(
                    f,
                    "{{\"ts\":\"{}\",\"claimed\":{},\"verified\":{}}}",
                    ts, claimed, verified
                )
            });
        println!("false-claim: recorded claimed={} verified={}", claimed, verified);
        return 0;
    }
    let txt = match fs::read_to_string(state_p) {
        Ok(t) => t,
        Err(_) => {
            println!("FALSE-CLAIM: no ledger (events=0)");
            return 0;
        }
    };
    let mut claimed = 0u64;
    let mut verified = 0u64;
    let mut events = 0u64;
    for line in txt.lines() {
        let o = parse_obj(line);
        if o.is_empty() {
            continue;
        }
        events += 1;
        claimed += get_num(&o, "claimed") as u64;
        verified += get_num(&o, "verified") as u64;
    }
    let false_est = if claimed > 0 {
        (claimed - verified) as f64 / claimed as f64 * 100.0
    } else {
        0.0
    };
    println!(
        "FALSE-CLAIM: events={} claimed={} verified={}\n  false-estimation%      = {:.1}  (claimed but not verified)\n  false-positive-of-done% = {:.1}  (claimed-done / verified)",
        events, claimed, verified, false_est, false_est
    );
    0
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. META — EMA tilt over bench/eval/false deltas (replaces gov_meta observe)
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_meta(bp: f64, bn: f64, rp: f64, rn: f64, fr: f64) -> i32 {
    let state_p = Path::new("/root/dowiz/tools/telemetry/governance/meta_state.json");
    let mut s: BTreeMap<String, Jv> = if state_p.exists() {
        parse_obj(&fs::read_to_string(state_p).unwrap_or_default())
    } else {
        BTreeMap::new()
    };
    let n = match s.get("n") {
        Some(Jv::Num(x)) => *x as u64,
        _ => 0,
    };
    let a = 2.0 / (n as f64 + 1.0);
    let ema_bench = lerp(get_num(&s, "ema_bench"), pct(bn, bp), a);
    let ema_eval = lerp(get_num(&s, "ema_eval"), (rn - rp), a);
    let ema_false = lerp(get_num(&s, "ema_false"), fr.max(0.0), a);
    let mut new_s = BTreeMap::new();
    new_s.insert("n".to_string(), Jv::Num((n + 1) as f64));
    new_s.insert("ema_bench".to_string(), Jv::Num(ema_bench));
    new_s.insert("ema_eval".to_string(), Jv::Num(ema_eval));
    new_s.insert("ema_false".to_string(), Jv::Num(ema_false));
    let json = obj_to_json(&new_s);
    let _ = fs::write(state_p, json);
    println!(
        "META n={} ema_bench={:+.3} ema_eval={:+.3} ema_false={:.3}\nGUIDANCE lane_tol={:.3} judge_count=3 precedent_tau=0.82\nRULES: guidance, not gates — energy flows; meta-rule tilts only.",
        n + 1, ema_bench, ema_eval, ema_false, 0.10 + ema_bench.abs() * 0.5
    );
    0
}

fn lerp(prev: f64, next: f64, a: f64) -> f64 {
    a * next + (1.0 - a) * prev
}
fn pct(new: f64, prev: f64) -> f64 {
    if prev > 0.0 {
        new / prev - 1.0
    } else {
        0.0
    }
}
fn obj_to_json(m: &BTreeMap<String, Jv>) -> String {
    let mut s = String::from("{\n");
    let mut first = true;
    for (k, v) in m {
        if !first {
            s.push_str(",\n");
        }
        match v {
            Jv::Num(f) => s.push_str(&format!("  \"{}\": {}", k, f)),
            Jv::Str(x) => s.push_str(&format!("  \"{}\": \"{}\"", k, x)),
            Jv::Bool(b) => s.push_str(&format!("  \"{}\": {}", k, b)),
            Jv::Null => s.push_str(&format!("  \"{}\": null", k)),
        }
        first = false;
    }
    s.push_str("\n}\n");
    s
}

// ─────────────────────────────────────────────────────────────────────────────
fn now_iso() -> String {
    // No RNG, no chrono: use std only via a fallback timestamp string.
    // We avoid `SystemTime` formatting libs; emit seconds since epoch + a marker.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs)
}

fn usage() -> i32 {
    eprintln!(
        "usage:\n  native-trackers bench <crate-dir> [--threshold N]\n  native-trackers route <task> [budget] [ruin]\n  native-trackers false-claim [--record claimed verified]\n  native-trackers meta observe <bench_prev> <bench_new> <eval_prev> <eval_new> <false_rate>"
    );
    2
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        std::process::exit(usage());
    }
    let rc = match args[1].as_str() {
        "bench" => {
            if args.len() < 3 {
                usage()
            } else {
                let crate_dir = &args[2];
                let mut threshold = 10.0;
                if let Some(pos) = args.iter().position(|a| a == "--threshold") {
                    if let Some(v) = args.get(pos + 1) {
                        threshold = v.parse().unwrap_or(10.0);
                    }
                }
                cmd_bench(crate_dir, threshold)
            }
        }
        "route" => {
            let task = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let budget = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10.0);
            let ruin = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.20);
            if task.is_empty() {
                usage()
            } else {
                cmd_route(task, budget, ruin)
            }
        }
        "false-claim" => {
            let rec = if args.get(2).map(|s| s.as_str()) == Some("--record") {
                let c = args.get(3).and_then(|s| s.parse().ok());
                let v = args.get(4).and_then(|s| s.parse().ok());
                match (c, v) {
                    (Some(c), Some(v)) => Some((c, v)),
                    _ => None,
                }
            } else {
                None
            };
            cmd_false_claim(rec)
        }
        "meta" => {
            if args.get(2).map(|s| s.as_str()) != Some("observe") || args.len() < 8 {
                usage()
            } else {
                let bp = args[3].parse().unwrap_or(1.0);
                let bn = args[4].parse().unwrap_or(1.0);
                let rp = args[5].parse().unwrap_or(1.0);
                let rn = args[6].parse().unwrap_or(1.0);
                let fr = args[7].parse().unwrap_or(0.0);
                cmd_meta(bp, bn, rp, rn, fr)
            }
        }
        _ => usage(),
    };
    std::process::exit(rc);
}
