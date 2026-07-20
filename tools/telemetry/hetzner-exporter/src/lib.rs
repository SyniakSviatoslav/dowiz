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

/// Absolute RAM usage `(used_bytes, total_bytes)` — the SAME `/proc/meminfo` read that
/// backs `mem_pct` (used = MemTotal − MemAvailable), exposed absolutely so consumers can
/// render "2.8 / 30.6 GiB" next to the percentage. `None` on read failure.
pub fn mem_bytes() -> Option<(u64, u64)> {
    let (total_kb, avail_kb) = meminfo()?;
    Some((total_kb.saturating_sub(avail_kb) * 1024, total_kb * 1024))
}

/// Absolute root-filesystem usage `(used_bytes, total_bytes)` — the SAME statvfs-style
/// read that backs `disk_pct`. `None` on read failure.
pub fn disk_bytes() -> Option<(u64, u64)> {
    let (bavail, blocks) = statvfs_root()?;
    Some((blocks.saturating_sub(bavail), blocks))
}

/// Cumulative network I/O since boot: `(rx_bytes, tx_bytes)` summed across every
/// non-loopback interface in `/proc/net/dev`. Counters, not rates — the consumer decides
/// whether to show totals or diff two snapshots. `None` on read/parse failure (never a
/// fabricated 0), same degrade-closed convention as the other gauges.
#[cfg(target_os = "linux")]
pub fn net_io_bytes() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/net/dev").ok()?;
    parse_net_dev(&s)
}

#[cfg(not(target_os = "linux"))]
pub fn net_io_bytes() -> Option<(u64, u64)> {
    None
}

/// Pure parser for `/proc/net/dev` content. Per-interface lines are `name: <16 counters>`
/// where field 1 is rx_bytes and field 9 is tx_bytes; the two header lines carry no `:`
/// (the `Inter-|`/`face |` banner) and are skipped naturally. `lo` is excluded — loopback
/// traffic is not host network I/O.
fn parse_net_dev(s: &str) -> Option<(u64, u64)> {
    let mut rx = 0u64;
    let mut tx = 0u64;
    let mut seen = false;
    for line in s.lines() {
        let (name, rest) = match line.split_once(':') {
            Some(x) => x,
            None => continue,
        };
        if name.trim() == "lo" {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 16 {
            return None; // malformed row: refuse to report a partial sum
        }
        rx = rx.checked_add(fields[0].parse().ok()?)?;
        tx = tx.checked_add(fields[8].parse().ok()?)?;
        seen = true;
    }
    if seen {
        Some((rx, tx))
    } else {
        None
    }
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

    #[test]
    fn parse_net_dev_sums_non_loopback_and_skips_lo() {
        let fixture = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 918392190 1670912    0    0    0     0          0         0 918392190 1670912    0    0    0     0       0          0
  eth0: 1000       10    0    0    0     0          0         0 2000       20    0    0    0     0       0          0
   wt0:  500        5    0    0    0     0          0         0  700        7    0    0    0     0       0          0
";
        // lo excluded; eth0 + wt0 summed: rx = 1000+500, tx = 2000+700.
        assert_eq!(parse_net_dev(fixture), Some((1500, 2700)));
    }

    #[test]
    fn parse_net_dev_refuses_malformed_rows() {
        assert_eq!(parse_net_dev("eth0: 12 34\n"), None); // short row: no partial sum
        assert_eq!(parse_net_dev("header only, no colon rows\n"), None); // nothing seen
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn net_io_bytes_reads_real_counters_on_linux() {
        // A live Linux host with any non-loopback interface must produce a real reading.
        let (rx, tx) = net_io_bytes().expect("/proc/net/dev readable on this host");
        // Cumulative counters on a network-connected box are > 0 (a 0 here would mean
        // the box has literally never sent a packet — worth failing loudly over).
        assert!(rx > 0, "rx_bytes must be a positive accumulator, got {rx}");
        assert!(tx > 0, "tx_bytes must be a positive accumulator, got {tx}");
    }
}
