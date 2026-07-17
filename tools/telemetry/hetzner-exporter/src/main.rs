//! hetzner-exporter — pure-std HTTP edge.
//!
//! Serves the gauges as JSON on 127.0.0.1:9091/health for Gatus polling:
//!   curl 127.0.0.1:9091/health
//!   -> {"disk_pct":81.0,"load1":0.12,"mem_pct":9.0,"ts":...}
//!
//! Opt-in native edge: TELEMETRY_SER=native serves raw LE f64 bytes (the
//! kernel wire shape) for native consumers, length = 3*8 = 24 bytes.
//!
//! Usage:
//!   hetzner-exporter                 # foreground, serve forever
//!   hetzner-exporter --once          # compute + print one JSON snapshot, exit
//!   hetzner-exporter --selftest      # verify gauges compute + JSON is valid

use hetzner_exporter::{gauges_dict, gauges_native, GAUGE_SCHEMA};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

const HOST: &str = "127.0.0.1";
const PORT: u16 = 9091;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let force_native = args.iter().any(|a| a == "--native");
    if args.iter().any(|a| a == "--once") {
        println!("{}", snapshot_json());
        return;
    }
    if args.iter().any(|a| a == "--selftest") {
        selftest();
        return;
    }
    if args.iter().any(|a| a == "--probe") {
        probe_once(force_native);
        return;
    }
    serve_forever(force_native);
}

/// Edge selection: explicit `--native` flag wins; else the `TELEMETRY_SER`
/// env var; default JSON. (Keeping the env path for drop-in parity with the
/// deleted ser.py contract.)
fn use_native_edge(force_native: bool) -> bool {
    force_native || std::env::var("TELEMETRY_SER").unwrap_or_default() == "native"
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn snapshot_json() -> String {
    let d = gauges_dict();
    let ts = now_ts();
    // Field order must be stable for Gatus JSON-body conditions.
    let mut s = String::from("{");
    let mut first = true;
    for k in GAUGE_SCHEMA.iter() {
        if !first {
            s.push(',');
        }
        s.push_str(&format!("\"{}\":{}", k, d.get(*k).copied().unwrap_or(-1.0)));
        first = false;
    }
    s.push_str(&format!(",\"ts\":{}}}", ts));
    s
}

fn serve_forever(force_native: bool) {
    let addr = format!("{}:{}", HOST, PORT);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("hetzner-exporter: bind {addr} failed: {e}");
            std::process::exit(1);
        }
    };
    eprintln!("hetzner-exporter: serving {addr}/health (ctrl-c to stop)");
    for stream in listener.incoming() {
        match stream {
            Ok(mut s) => {
                // Read + discard the request, then respond. Keep it lean.
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let native = use_native_edge(force_native);
                let (body, ctype) = if native {
                    let v = gauges_native();
                    let mut b = Vec::with_capacity(24);
                    for x in v.iter() {
                        b.extend_from_slice(&x.to_le_bytes());
                    }
                    (b, "application/octet-stream")
                } else {
                    (snapshot_json().into_bytes(), "application/json")
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    ctype,
                    body.len()
                );
                let _ = s.write_all(resp.as_bytes());
                let _ = s.write_all(&body);
            }
            Err(_) => continue,
        }
    }
}

fn probe_once(force_native: bool) {
    // Deterministic, harness-safe check of what the server WOULD answer:
    // build the same body the HTTP handler builds, print it, assert shape.
    // (A real socket round-trip is exercised by the unit tests + a manual
    // `hetzner-exporter &; curl` when operating the box.)
    let native = use_native_edge(force_native);
    if native {
        let v = gauges_native();
        let mut b = Vec::with_capacity(24);
        for x in v.iter() {
            b.extend_from_slice(&x.to_le_bytes());
        }
        assert_eq!(b.len(), 24, "native body = 24 bytes (3 LE f64)");
        let vals: Vec<f64> = (0..3)
            .map(|i| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&b[i * 8..i * 8 + 8]);
                f64::from_le_bytes(buf)
            })
            .collect();
        println!("PROBE NATIVE: bytes={} f64s={:?}", b.len(), vals);
    } else {
        let s = snapshot_json();
        assert!(s.starts_with('{'), "json body");
        println!("PROBE JSON: {}", s);
    }
}

fn selftest() {
    // gauges compute + JSON is well-formed.
    let json = snapshot_json();
    assert!(json.starts_with('{') && json.ends_with('}'), "json shape");
    assert!(json.contains("disk_pct"), "has disk_pct");
    assert!(json.contains("mem_pct"), "has mem_pct");
    // native wire length is exactly 24 bytes (3 f64).
    let v = gauges_native();
    let mut b = Vec::with_capacity(24);
    for x in v.iter() {
        b.extend_from_slice(&x.to_le_bytes());
    }
    assert_eq!(b.len(), 24, "native wire is 3*8 bytes");
    println!("SELFTEST PASS: gauges compute, json+native edges valid (24-byte wire)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_selection_defaults_to_json() {
        std::env::remove_var("TELEMETRY_SER");
        assert!(!use_native_edge(false));
        assert!(use_native_edge(true));
        std::env::set_var("TELEMETRY_SER", "native");
        assert!(use_native_edge(false));
        std::env::remove_var("TELEMETRY_SER");
    }

    #[test]
    fn native_wire_is_three_f64() {
        let v = gauges_native();
        let mut b = Vec::with_capacity(24);
        for x in v.iter() {
            b.extend_from_slice(&x.to_le_bytes());
        }
        assert_eq!(b.len(), 24);
        let back: Vec<f64> = (0..3)
            .map(|i| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&b[i * 8..i * 8 + 8]);
                f64::from_le_bytes(buf)
            })
            .collect();
        assert_eq!(back, v.to_vec());
    }
}
