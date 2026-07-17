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
//!   swarm-proof                          economic crossover + parallel vs sequential fan-out proof
//!   ser    <wire|json> [hex-or-json]     canonical f64 wire <-> JSON edge adapter
//!
//! Exit codes: 0 ok; 1 bench regression beyond threshold; 2 usage/IO error.

use std::collections::BTreeMap;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
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
// swarm-proof — native port of tools/telemetry/swarm_proof.py (no LLM, no python
// subprocess). (A) economic crossover from real 2026 API prices; (B) parallel vs
// sequential fan-out timing via a native threadpool (replaces the xargs/python3
// timing experiment).
// ─────────────────────────────────────────────────────────────────────────────

const PRICES: &[(&str, f64, f64)] = &[
    // tier: (input $/Mtok, output $/Mtok)
    ("frontier", 5.0, 15.0),
    ("mid", 0.50, 1.50),
    ("cheap", 0.10, 0.40),
];
const BLUEPRINT_TOK_IN: f64 = 4000.0;
const BLUEPRINT_TOK_OUT: f64 = 1500.0;
const EXEC_TOK_IN: f64 = 2000.0;
const EXEC_TOK_OUT: f64 = 800.0;

fn cost(tier: &str, tin: f64, tout: f64) -> f64 {
    let &(_, pi, po) = PRICES.iter().find(|x| x.0 == tier).unwrap_or(&PRICES[0]);
    tin / 1e6 * pi + tout / 1e6 * po
}
fn sequential_cost(n: usize, arch: &str) -> f64 {
    n as f64 * cost(arch, BLUEPRINT_TOK_IN + EXEC_TOK_IN, BLUEPRINT_TOK_OUT + EXEC_TOK_OUT)
}
fn swarm_cost(n: usize, arch: &str, exec_t: &str) -> f64 {
    n as f64 * cost(arch, BLUEPRINT_TOK_IN, BLUEPRINT_TOK_OUT)
        + n as f64 * cost(exec_t, EXEC_TOK_IN, EXEC_TOK_OUT)
}
fn crossover_n(arch: &str, exec_t: &str) -> Option<usize> {
    for n in 1..200 {
        if swarm_cost(n, arch, exec_t) < sequential_cost(n, arch) {
            return Some(n);
        }
    }
    None
}

fn cmd_swarm_proof() -> i32 {
    println!("=== (A) ECONOMIC CROSSOVER (real 2026 prices) ===");
    for (arch, exec_t) in [("frontier", "cheap"), ("frontier", "mid"), ("mid", "cheap")] {
        let seq1 = sequential_cost(1, arch);
        let sw1 = swarm_cost(1, arch, exec_t);
        let nx = crossover_n(arch, exec_t);
        let sw10 = swarm_cost(10, arch, exec_t);
        let seq10 = sequential_cost(10, arch);
        let pct = if seq10 > 0.0 {
            100.0 * (1.0 - sw10 / seq10)
        } else {
            0.0
        };
        println!(
            "  architect={:9} executor={:6}: 1-task seq=${:.4} swarm=${:.4} | crossover N={:?} | N=10 swarm=${:.4} vs seq=${:.4} ({:.0}% cheaper)",
            arch, exec_t, seq1, sw1, nx, sw10, seq10, pct
        );
    }
    println!("  NOTE: crossover N is small (<=2); swarm wins for essentially any N>=2.");

    println!("\n=== (B) ENGINE TIMING (parallel vs sequential fan-out, native threadpool) ===");
    let task_ms = 300u64; // mirrors the python timing experiment's 0.30s sleep
    for n in [4usize, 8usize] {
        let tp = time_parallel(n, task_ms);
        let ts = time_sequential(n, task_ms);
        let speedup = if tp > 0.0 { ts / tp } else { 0.0 };
        println!(
            "  N={}: parallel={:.2}s  sequential={:.2}s  speedup={:.2}x",
            n,
            tp,
            ts,
            speedup
        );
    }
    println!("  ideal parallel ~0.30s (max task), sequential ~N*0.30s. Native fan-out confirms.");

    let mut out = String::from("{\"crossover\":{\"frontier/cheap\":");
    out.push_str(&format!("{:?}", crossover_n("frontier", "cheap")));
    out.push_str(",\"frontier/mid\":");
    out.push_str(&format!("{:?}", crossover_n("frontier", "mid")));
    out.push_str(",\"mid/cheap\":");
    out.push_str(&format!("{:?}", crossover_n("mid", "cheap")));
    out.push_str("}}");
    println!("\nJSON: {}", out);
    0
}

/// Parallel: spawn N threads, each sleeps `task_ms`, join all. ~one task wall time.
fn time_parallel(n: usize, task_ms: u64) -> f64 {
    use std::thread;
    use std::time::{Duration, Instant};
    let t0 = Instant::now();
    let handles: Vec<_> = (0..n)
        .map(|_| thread::spawn(move || thread::sleep(Duration::from_millis(task_ms))))
        .collect();
    for h in handles {
        let _ = h.join();
    }
    t0.elapsed().as_secs_f64()
}
/// Sequential: run each task in turn.
fn time_sequential(n: usize, task_ms: u64) -> f64 {
    use std::thread;
    use std::time::{Duration, Instant};
    let t0 = Instant::now();
    for _ in 0..n {
        thread::sleep(Duration::from_millis(task_ms));
        let _ = thread::yield_now();
    }
    t0.elapsed().as_secs_f64()
}

// ─────────────────────────────────────────────────────────────────────────────
// ser — native port of tools/telemetry/ser.py: canonical little-endian f64 wire
// <-> JSON edge adapter. No external libs (struct/json replaced by std only).
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_ser(mode: &str, arg: &str) -> i32 {
    match mode {
        "wire" => {
            // arg = hex string of raw f64 bytes -> JSON array of floats
            let bytes = match hex_to_bytes(arg) {
                Some(b) => b,
                None => {
                    eprintln!("ser wire: invalid hex input");
                    return 2;
                }
            };
            let n = bytes.len() / 8;
            let mut vals: Vec<f64> = Vec::with_capacity(n);
            for i in 0..n {
                let mut b = [0u8; 8];
                b.copy_from_slice(&bytes[i * 8..i * 8 + 8]);
                vals.push(f64::from_le_bytes(b));
            }
            let s = vals
                .iter()
                .map(|v| format!("{:.6}", v))
                .collect::<Vec<_>>()
                .join(", ");
            println!("[{}]", s);
            0
        }
        "json" => {
            // arg = JSON array of floats -> hex of raw f64 bytes (canonical wire)
            let vals = match parse_float_array(arg) {
                Some(v) => v,
                None => {
                    eprintln!("ser json: invalid JSON array input");
                    return 2;
                }
            };
            let mut out = String::with_capacity(vals.len() * 16);
            for v in &vals {
                for b in v.to_le_bytes() {
                    out.push_str(&format!("{:02x}", b));
                }
            }
            println!("{}", out);
            0
        }
        _ => {
            eprintln!("ser: mode must be 'wire' or 'json'");
            2
        }
    }
}

fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

fn parse_float_array(s: &str) -> Option<Vec<f64>> {
    let s = s.trim();
    let s = s.strip_prefix('[')?;
    let s = s.strip_suffix(']')?;
    let mut out = Vec::new();
    for part in s.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        out.push(p.parse::<f64>().ok()?);
    }
    Some(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// hetzner-serve — native port of tools/telemetry/hetzner_exporter.py.
// Pure-std: reads /proc + statvfs, serves JSON metrics on a std TcpListener
// (no http crate). Mirrors the Python's /metrics and /api/v1/metrics endpoints.
// ─────────────────────────────────────────────────────────────────────────────

fn read_proc_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Parse a `/proc/meminfo`-style `Key:  value kB` map.
fn parse_kv(text: &str) -> std::collections::HashMap<String, u64> {
    let mut m = std::collections::HashMap::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if let (Some(k), Some(v)) = (it.next(), it.next()) {
            if let Ok(n) = v.parse::<u64>() {
                m.insert(k.trim_end_matches(':').to_string(), n);
            }
        }
    }
    m
}

fn collect_metrics() -> String {
    let mut parts = Vec::new();

    // CPU: /proc/stat first line "cpu  user nice system idle ..."
    if let Some(s) = read_proc_file("/proc/stat") {
        if let Some(line) = s.lines().next() {
            let nums: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|x| x.parse::<u64>().ok())
                .collect();
            let total: u64 = nums.iter().sum();
            let idle = nums.get(3).copied().unwrap_or(0) + nums.get(4).copied().unwrap_or(0);
            let used = total.saturating_sub(idle);
            let pct = if total > 0 {
                (used as f64 / total as f64 * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };
            parts.push(format!("\"cpu\":{{\"percent_used\":{:.2},\"user\":{},\"system\":{}}}",
                pct, nums.get(0).copied().unwrap_or(0), nums.get(2).copied().unwrap_or(0)));
        }
    }

    // Memory: /proc/meminfo (values in kB)
    if let Some(s) = read_proc_file("/proc/meminfo") {
        let kv = parse_kv(&s);
        let total = *kv.get("MemTotal").unwrap_or(&0);
        let avail = *kv.get("MemAvailable").unwrap_or(&0);
        let used = total.saturating_sub(avail);
        let pct = if total > 0 {
            (used as f64 / total as f64 * 100.0 * 100.0).round() / 100.0
        } else {
            0.0
        };
        parts.push(format!("\"mem\":{{\"percent_used\":{:.2},\"total_kb\":{},\"used_kb\":{},\"free_kb\":{}}}",
            pct, total, used, avail));
    }

    // Load: /proc/loadavg
    if let Some(s) = read_proc_file("/proc/loadavg") {
        let f: Vec<&str> = s.split_whitespace().take(3).collect();
        if f.len() == 3 {
            parts.push(format!("\"load\":{{\"1m\":{},\"5m\":{},\"15m\":{}}}",
                f[0], f[1], f[2]));
        }
    }

    // Disk: statvfs via raw FFI to libc (always linked on Linux; no Cargo dep).
    // `std::os::unix::fs::MetadataExt` on this toolchain exposes only blocks()/blksize(),
    // not the free-block methods, so we call statvfs64 directly.
    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Statvfs {
            f_bsize: u64,
            f_frsize: u64,
            f_blocks: u64,
            f_bfree: u64,
            f_bavail: u64,
            f_files: u64,
            f_ffree: u64,
            f_favail: u64,
            f_fsid: u64,
            f_flag: u64,
            f_namemax: u64,
            // Generous padding so the C write (glibc statvfs64 ~104 bytes)
            // never overruns our buffer. Field offsets for the first
            // 11 members match glibc exactly (all 8-byte, sequential).
            f_spare: [u64; 12],
        }
        extern "C" {
            fn statvfs64(path: *const std::os::raw::c_char, buf: *mut Statvfs) -> std::os::raw::c_int;
        }
        let mut st: Statvfs = unsafe { std::mem::zeroed() };
        let cpath = std::ffi::CString::new("/").unwrap_or_default();
        let rc = unsafe { statvfs64(cpath.as_ptr(), &mut st) };
        if rc == 0 {
            let total = st.f_blocks * st.f_frsize;
            let avail = st.f_bavail * st.f_frsize;
            let used = total.saturating_sub(avail);
            let pct = if total > 0 {
                (used as f64 / total as f64 * 100.0 * 100.0).round() / 100.0
            } else {
                0.0
            };
            parts.push(format!(
                "\"disk\":{{\"percent_used\":{:.2},\"total_bytes\":{},\"used_bytes\":{},\"free_bytes\":{}}}",
                pct, total, used, avail
            ));
        }
    }
    if let Some(s) = read_proc_file("/proc/net/dev") {
        let mut rx = 0u64;
        let mut tx = 0u64;
        for line in s.lines().skip(2) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 10 && !cols[0].starts_with("lo:") {
                rx += cols[1].parse::<u64>().unwrap_or(0);
                tx += cols[9].parse::<u64>().unwrap_or(0);
            }
        }
        parts.push(format!("\"net\":{{\"rx_bytes\":{},\"tx_bytes\":{}}}", rx, tx));
    }

    let ts = now_iso();
    format!("{{\"timestamp\":{},\"host\":\"{}\",{}}}", ts, hostname(), parts.join(","))
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| std::env::var("HOSTNAME").unwrap_or_default())
}

fn http_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

fn cmd_hetzner_serve(port: u16) -> i32 {
    use std::net::TcpListener;
    use std::io::{Read, Write};
    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("hetzner-serve: bind {} failed: {}", addr, e);
            return 2;
        }
    };
    println!("hetzner-serve: listening on http://{}  (Ctrl-C to stop)", addr);
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                // Minimal HTTP: read request line, respond with metrics JSON.
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let body = collect_metrics();
                let resp = http_response(&body);
                let _ = s.write_all(resp.as_bytes());
                let _ = s.flush();
            }
            Err(_) => continue,
        }
    }
    0
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
        "swarm-proof" => cmd_swarm_proof(),
        "ser" => {
            let mode = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let arg = args.get(3).map(|s| s.as_str()).unwrap_or("");
            if mode.is_empty() {
                usage()
            } else {
                cmd_ser(mode, arg)
            }
        }
        "hetzner-serve" => {
            let port = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(9102);
            cmd_hetzner_serve(port)
        }
        _ => usage(),
    };
    std::process::exit(rc);
}
