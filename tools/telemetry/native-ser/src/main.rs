//! native-ser CLI — command surface matching the deleted ser.py, so nothing
//! that shelled out to `python3 ser.py ...` breaks.
//!
//!   native-ser wire 3.14 -2.5 0          # -> hex LE f64 bytes (canonical)
//!   native-ser unwrap <hex>              # -> space-separated f64s
//!   native-ser obj disk_pct=81 load1=0.12 mem_pct=9   # -> schema-ordered native f64s
//!   native-ser unobj disk_pct load1 mem_pct <hex>     # -> schema-keyed values
//!   native-ser json '{"a":1,"b":2}'     # -> validate + re-emit
//!   native-ser edge                     # -> current TELEMETRY_SER selector
//!   native-ser selftest                 # -> parity check vs the Python contract
//!
//! `obj`/`unobj` exercise the schema-ordered native form (the kernel C-ABI
//! `field_metrics` layout): key order is fixed, so native consumers never
//! depend on JSON key presence.

use native_ser::{default_edge, from_json, json_edge, native_of, obj_of, wire_f64, unwire_f64, Json};
use std::collections::HashMap;

const DEFAULT_SCHEMA: &[&str] = &["disk_pct", "load1", "mem_pct"];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("native-ser: wire|unwrap|obj|unobj|json|edge|selftest");
        std::process::exit(2);
    }
    match args[1].as_str() {
        "wire" => {
            let vals: Vec<f64> = args[2..].iter().filter_map(|s| s.parse().ok()).collect();
            let b = wire_f64(&vals);
            print!("{}", hex(&b));
        }
        "unwrap" => {
            let hex = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let b = unhex(hex);
            let v = unwire_f64(&b, None);
            let s: Vec<String> = v.iter().map(|x| format!("{:.6}", x)).collect();
            println!("{}", s.join(" "));
        }
        "obj" => {
            // key=val pairs -> schema-ordered native f64 array on stdout.
            let mut map: HashMap<String, f64> = HashMap::new();
            for kv in &args[2..] {
                if let Some((k, v)) = kv.split_once('=') {
                    if let Ok(f) = v.parse::<f64>() {
                        map.insert(k.to_string(), f);
                    }
                }
            }
            let b = native_of(DEFAULT_SCHEMA, &map);
            let arr = unwire_f64(&b, None);
            let s: Vec<String> = arr.iter().map(|x| format!("{:.6}", x)).collect();
            println!("{}", s.join(" "));
        }
        "unobj" => {
            // schema keys... then `<hex>` -> schema-keyed `k=v` lines.
            if args.len() < 4 {
                eprintln!("native-ser: unobj <schema...> <hex>");
                std::process::exit(2);
            }
            let hex = args.last().unwrap();
            let schema: Vec<&str> = args[2..args.len() - 1].iter().map(|s| s.as_str()).collect();
            let b = unhex(hex);
            let back = obj_of(&schema, &b);
            for k in &schema {
                println!("{}={}", k, back.get(*k).copied().unwrap_or(-1.0));
            }
        }
        "json" => {
            let s = args.get(2).map(|x| x.as_str()).unwrap_or("");
            match from_json(s) {
                Ok(j) => println!("{}", json_edge(&j)),
                Err(e) => {
                    eprintln!("native-ser: json parse error: {e}");
                    std::process::exit(1);
                }
            }
        }
        "edge" => println!("{}", default_edge()),
        "selftest" => selftest(),
        other => {
            eprintln!("native-ser: unknown subcommand `{other}`");
            std::process::exit(2);
        }
    }
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{:02x}", x));
    }
    s
}

fn unhex(s: &str) -> Vec<u8> {
    let s = s.trim();
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() || i < bytes.len() {
        if i + 1 >= bytes.len() {
            break;
        }
        let hi = (bytes[i] as char).to_digit(16);
        let lo = (bytes[i + 1] as char).to_digit(16);
        if let (Some(h), Some(l)) = (hi, lo) {
            out.push((h * 16 + l) as u8);
        }
        i += 2;
    }
    out
}

fn selftest() {
    // (1) wire round-trip bit-identical.
    let v = [3.14f64, -2.5, 0.0, 1e9];
    let b = wire_f64(&v);
    let back = unwire_f64(&b, None);
    assert_eq!(v.to_vec(), back, "wire round-trip bit-identical");

    // (2) native_of/obj_of schema-ordered.
    let schema = ["disk_pct", "load1", "mem_pct"];
    let mut obj = HashMap::new();
    obj.insert("disk_pct".to_string(), 81.0);
    obj.insert("load1".to_string(), 0.12);
    obj.insert("mem_pct".to_string(), 9.0);
    let b = native_of(&schema, &obj);
    let back = obj_of(&schema, &b);
    assert_eq!(back.get("disk_pct").copied().unwrap_or(-1.0), 81.0);
    assert_eq!(back.get("load1").copied().unwrap_or(-1.0), 0.12);

    // (3) json edge round-trip.
    let j = Json::Obj(vec![
        ("disk_pct".into(), Json::Num(81.0)),
        ("load1".into(), Json::Num(0.12)),
    ]);
    let s = json_edge(&j);
    let parsed = from_json(&s).expect("json parse");
    assert_eq!(parsed, j);

    // (4) default_edge validates.
    std::env::set_var("TELEMETRY_SER", "native");
    assert_eq!(default_edge(), "native");
    std::env::remove_var("TELEMETRY_SER");

    println!("SELFTEST PASS: wire+obj+json+edge contracts hold");
}
