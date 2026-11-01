// SPDX-License-Identifier: MIT OR Apache-2.0
//! W-MG-ATK3: this crate's `open` + `verify_doc` natively, over every image in $ATK3_DIR, one line
//! per file in the verifiers' shape, so tools/gates/atk3_run.py can hold it against wasm32 and
//! Python. Under `cargo test` (debug) overflow checks are ON, so a sum that wraps silently in a
//! release build is a named panic here. No $ATK3_DIR: nothing to read, the test passes.

use bebop_image::View;
use bebop_kv::verify::KvError;
use bebop_kv::zc::KvIn;
use bebop_wasm_reader::{abi::status_of, open, Kind};

fn line(bytes: &[u8]) -> String {
    let mut d = match open(bytes) {
        Ok(d) => d,
        Err(r) => return format!("kv status={} n=0 root=0", status_of(&r)),
    };
    let fam = match d.kind {
        Kind::Log => "log",
        Kind::Doc => "doc",
        Kind::Kv { .. } => "kv",
    };
    let mut s = format!("{fam} status=0 n={} root={}", d.len(), d.fold);
    match d.verify_doc() {
        Some(Ok(c)) => s += &format!(" || verify records={} chained={} legacy={} redacted={} broken={} forks={} seals={} checked={} heads={} keys={} genesis={} segments={} cut={} cut_redacted={} declared={} keys_disagree={} unsealed={} merges={} merges_bad={}", c.records, c.chained, c.legacy, c.redacted, c.broken, c.forks, c.seals, c.checked, c.heads, c.keys, c.genesis, c.segments, c.cut, c.cut_redacted, c.declared, c.keys_disagree, c.unsealed, c.merges, c.merges_bad),
        Some(Err(r)) => s += &format!(" || verify status={}", status_of(&r)),
        None => {}
    }
    for a in &d.anchors {
        let over: String = a.over.iter().map(|b| format!("{b:02x}")).collect();
        s += &format!(" || anchor kind={} over={over} time_ms={} proof_len={} (present, NOT verified by this reader)", a.kind, a.time_ms, a.proof.len());
    }
    s
}

/// The zero-copy KV reader over the caller's bytes (`KvIn` on `View`): no padding, its own walks.
fn zc_line(bytes: &[u8]) -> String {
    match KvIn::open(View::new(bytes)) {
        Ok(_) => "kv status=0".to_string(),
        Err(KvError::NotKv) => "kv status=2 n=0 root=0".to_string(),
        Err(KvError::BadCrc(_)) => "kv status=10 n=0 root=0".to_string(),
        Err(KvError::UnknownVersion { .. }) => "kv status=12 n=0 root=0".to_string(),
        Err(KvError::Tree(_)) => "kv status=2 n=0 root=0".to_string(),
    }
}

#[test]
fn every_image_gets_a_line_and_no_panic() {
    let Ok(dir) = std::env::var("ATK3_DIR") else { return };
    let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "img")).collect();
    names.sort();
    let mut panics = 0;
    let mut out = String::new();
    for p in names {
        let bytes = std::fs::read(&p).unwrap();
        let r = std::panic::catch_unwind(|| line(&bytes));
        let got = r.unwrap_or_else(|_| {
            panics += 1;
            "PANIC".to_string()
        });
        out += &format!("{} {}\n", p.file_stem().unwrap().to_string_lossy(), got);
    }
    std::fs::write(format!("{dir}/native.lines"), out).unwrap();
    let mut z = String::new();
    for p in std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "img")) {
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if name.starts_with("res_") {
            continue; // the two amplifiers are read once above; a third copy proves nothing
        }
        let bytes = std::fs::read(&p).unwrap();
        let r = std::panic::catch_unwind(|| zc_line(&bytes));
        z += &format!("{name} {}\n", r.unwrap_or_else(|_| "PANIC".to_string()));
    }
    std::fs::write(format!("{dir}/zc.lines"), z).unwrap();
    assert_eq!(panics, 0, "a reader never panics on a file");
}
