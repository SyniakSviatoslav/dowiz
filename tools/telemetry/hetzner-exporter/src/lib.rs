//! hetzner-exporter — host-resource gauges (pure-std).
//!
//! Replaces the deleted `tools/telemetry/hetzner_exporter.py`. Computes the
//! load-bearing host gauges ON REQUEST (no background loop, near-zero CPU) and
//! exposes them as a canonical f64 array (kernel `field_metrics` wire shape).
//!
//! Gauges (operator-selected frictions):
//!   disk_pct — root filesystem usage %       (alert > 90)
//!   load1    — 1-min load / #cpu (normalized) (alert > 1.0 sustained = saturated)
//!   mem_pct  — RAM used %                     (alert > 90)

/// Ordered numeric schema — the canonical native f64 layout.
pub const GAUGE_SCHEMA: [&str; 3] = ["disk_pct", "load1", "mem_pct"];

/// Compute host resource state as the canonical native f64 array.
/// Non-numeric metadata (`ts`) rides at the edge, not in the native array.
pub fn gauges_native() -> [f64; 3] {
    // disk_pct from statvfs on "/".
    let disk_pct = match statvfs_root() {
        Some((bavail, blocks)) if blocks > 0 => {
            let used = blocks - bavail;
            round1((used as f64 / blocks as f64) * 100.0)
        }
        _ => -1.0,
    };

    // load1 normalized by CPU count.
    let load_norm = match loadavg() {
        Some(l1) => {
            let ncpu = num_cpus();
            round2(l1 / ncpu as f64)
        }
        None => -1.0,
    };

    // mem_pct from /proc/meminfo.
    let mem_pct = match meminfo() {
        Some((total, avail)) if total > 0 => round1((1.0 - avail as f64 / total as f64) * 100.0),
        _ => -1.0,
    };

    [disk_pct, load_norm, mem_pct]
}

/// EDGE helper: native array + ts -> JSON-friendly dict (Gatus/Telemetry text).
pub fn gauges_dict() -> std::collections::HashMap<String, f64> {
    let v = gauges_native();
    let mut m = std::collections::HashMap::new();
    for (k, val) in GAUGE_SCHEMA.iter().zip(v.iter()) {
        m.insert((*k).to_string(), *val);
    }
    m
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[cfg(target_os = "linux")]
fn statvfs_root() -> Option<(u64, u64)> {
    // Read statvfs via libc is overkill; parse /proc/mounts-free approach using
    // the `stat` syscall is not std, so we read the root filesystem free/blocks
    // by shelling to `df`? No — pure-std. Use std::fs::metadata for total bytes
    // on "/" is not available cross-platform for free blocks. Instead read
    // /proc/self/mountinfo? Simpler: use `std::os::unix` statvfs via libc is not
    // std. We approximate disk usage from /proc/mounts is complex. Use a direct
    // syscall-free method: read `/sys/...`? Not std.
    //
    // Pragmatic pure-std path: spawn `df` is forbidden (no subprocess in a gauge
    // crate? it's fine, but we prefer std). We use the `nix`? no. Acceptable:
    // read available/used from `df -B1 /` via std::process is allowed (it's std).
    use std::process::Command;
    let out = Command::new("df").arg("-B1").arg("/").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    // Second line: Filesystem 1B-blocks Used Available Use% Mounted
    let line = s.lines().nth(1)?;
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 4 {
        return None;
    }
    let blocks: u64 = cols[1].parse().ok()?;
    let bavail: u64 = cols[3].parse().ok()?;
    Some((bavail, blocks))
}

#[cfg(not(target_os = "linux"))]
fn statvfs_root() -> Option<(u64, u64)> {
    None
}

#[cfg(target_os = "linux")]
fn loadavg() -> Option<f64> {
    let s = std::fs::read_to_string("/proc/loadavg").ok()?;
    s.split_whitespace().next()?.parse::<f64>().ok()
}

#[cfg(not(target_os = "linux"))]
fn loadavg() -> Option<f64> {
    None
}

#[cfg(target_os = "linux")]
fn meminfo() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = 0u64;
    let mut avail = 0u64;
    for line in s.lines() {
        let mut it = line.split(':');
        let key = it.next()?.trim();
        let val = it.next()?.split_whitespace().next()?.parse::<u64>().ok()?;
        if key == "MemTotal" {
            total = val;
        } else if key == "MemAvailable" {
            avail = val;
        }
    }
    if total == 0 {
        None
    } else {
        Some((total, avail))
    }
}

#[cfg(not(target_os = "linux"))]
fn meminfo() -> Option<(u64, u64)> {
    None
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_fixed_three_gauges() {
        assert_eq!(GAUGE_SCHEMA.len(), 3);
        assert_eq!(GAUGE_SCHEMA[0], "disk_pct");
        assert_eq!(GAUGE_SCHEMA[1], "load1");
        assert_eq!(GAUGE_SCHEMA[2], "mem_pct");
    }

    #[test]
    fn gauges_native_returns_three_values() {
        let g = gauges_native();
        assert_eq!(g.len(), 3);
        // On a real Linux host every gauge should be in [0,100] (unless error -1).
        for (i, v) in g.iter().enumerate() {
            if *v >= 0.0 {
                assert!(
                    (0.0..=100.0).contains(v),
                    "gauge {} = {} out of [0,100]",
                    i,
                    v
                );
            }
        }
    }

    #[test]
    fn gauges_dict_is_schema_keyed() {
        let d = gauges_dict();
        assert_eq!(d.len(), 3);
        assert!(d.contains_key("disk_pct"));
        assert!(d.contains_key("load1"));
        assert!(d.contains_key("mem_pct"));
    }
}
