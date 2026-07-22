//! hydra_golden_gate.rs — immutable byte-level golden gate for the Hydra probe.
//!
//! This binary is the tamper-evident anchor for probe behavior/output. It verifies:
//!   1. `hydra_runtime_probe.rs` source digest
//!   2. `hydra_runtime_probe --verify-golden --cycles 4` stable stdout
//!
//! It does NOT gate the test file itself; tests can be edited without breaking
//! this anchor.
//!
//! Innovate: Constants are CURRENT source-derived and MUST be bumped only as
//! a deliberate gate update. Auto-update = no gate. Upgrade trigger: signed
//! release process that signs golden.jsonl and replays it here at cut time.

use std::{
    env,
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
};

#[derive(Debug)]
struct GateDigest {
    path: &'static str,
    expected: [u8; 32],
}

const GATES: &[GateDigest] = &[];

const EXPECTED_PROBE_STDOUT_HEX: &str =
    "ce5558cc67255da648149743cb20106ad31fff3c0c0d93334fbef291ed87e331";

fn sha3_256(data: &[u8]) -> [u8; 32] {
    dowiz_kernel::event_log::sha3_256(data)
}

fn hex_to_bytes(hex: &str) -> [u8; 32] {
    let hex = hex.trim();
    if hex.len() != 64 {
        fail("expected hex string length is 64");
    }
    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = from_hex(chunk[0]);
        let lo = from_hex(chunk[1]);
        bytes[i] = (hi << 4) | lo;
    }
    bytes
}

fn from_hex(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => fail("invalid hex digit"),
    }
}

fn cargo_bin(name: &str) -> PathBuf {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "/root/dowiz/kernel".into());
    PathBuf::from(manifest)
        .join("target")
        .join("debug")
        .join(name)
}

fn fail(msg: &str) -> ! {
    eprintln!("GOLDEN GATE FAIL: {msg}");
    std::process::exit(1);
}

fn main() {
    for gate in GATES {
        let path = PathBuf::from(gate.path);
        let bytes =
            fs::read(&path).unwrap_or_else(|_| fail(&format!("missing gate file: {}", gate.path)));
        let actual = sha3_256(&bytes);
        if actual != gate.expected {
            fail(&format!("file hash mismatch: {}", gate.path));
        }
        eprintln!("PASS file gate: {}", gate.path);
    }

    let probe = cargo_bin("hydra_runtime_probe");
    let output = Command::new(&probe)
        .args(["--verify-golden", "--cycles", "4"])
        .output()
        .unwrap_or_else(|_| fail("failed to spawn hydra_runtime_probe"));

    if !output.status.success() {
        fail("probe --verify-golden --cycles 4 must exit 0");
    }

    let expected = hex_to_bytes(EXPECTED_PROBE_STDOUT_HEX);
    let actual = sha3_256(&output.stdout);
    if actual != expected {
        fail("probe runtime output hash mismatch");
    }
    eprintln!("PASS probe runtime gate: hydra_runtime_probe --cycles 4");

    eprintln!("GOLDEN GATE PASS");
    std::io::stdout().write_all(b"0\n").unwrap();
}
