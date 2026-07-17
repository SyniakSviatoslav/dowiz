#!/usr/bin/env python3
"""Integration test for the NATIVE serialization crate (native-ser).

Native upgrade (2026-07-17): the deleted `ser.py` (pure-stdlib Python) is replaced
by `tools/telemetry/native-ser` (pure-Rust, zero external crates). This test drives
the compiled `native-ser` binary to prove the same contract the old `test_ser.py`
asserted: canonical f64 wire round-trips, schema-ordered native form round-trips,
JSON edge is valid, and the crate is truly dependency-free.

Run: python3 tools/telemetry/test_ser.py
(Requires: cargo build --bin native-ser  in tools/telemetry/native-ser)
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
NATIVE_SER_BIN = os.path.join(HERE, "native-ser", "target", "debug", "native-ser")

SCHEMA = ["disk_pct", "load1", "mem_pct"]
SAMPLE = {"disk_pct": 81.0, "load1": 0.12, "mem_pct": 9.0}


def run(args, stdin=None):
    p = subprocess.run([NATIVE_SER_BIN, *args], input=stdin, text=True,
                       capture_output=True)
    return p


fails = 0


def ok(cond, msg):
    global fails
    print(("  PASS: " if cond else "  FAIL: ") + msg)
    if not cond:
        fails += 1


def main():
    if not os.path.exists(NATIVE_SER_BIN):
        print(f"WARN: native-ser bin not built at {NATIVE_SER_BIN}; "
              f"run `cargo build --bin native-ser` in tools/telemetry/native-ser")
        sys.exit(2)

    # (1) canonical f64 wire round-trip (bit-identical kernel C-ABI form).
    wire = run(["wire", *[str(x) for x in SAMPLE.values()]])
    hexstr = wire.stdout.strip()
    ok(len(hexstr) == len(SAMPLE) * 16, f"wire emits {len(SAMPLE)*16} hex chars (8/value)")
    back = run(["unwrap", hexstr])
    vals = [float(x) for x in back.stdout.split()]
    ok(vals == list(SAMPLE.values()), f"wire round-trips exact: {vals} == {list(SAMPLE.values())}")

    # (2) schema-ordered native form (obj -> native array, unobj -> schema keys).
    obj = run(["obj"] + [f"{k}={v}" for k, v in SAMPLE.items()])
    native_vals = [float(x) for x in obj.stdout.split()]
    ok(native_vals == list(SAMPLE.values()),
       f"obj emits schema-ordered native array: {native_vals}")
    # round-trip back via unobj.
    back_obj = run(["unobj", *SCHEMA, hexstr])
    kv = {}
    for line in back_obj.stdout.strip().splitlines():
        k, _, v = line.partition("=")
        kv[k] = float(v)
    ok(kv == SAMPLE, f"unobj recovers schema keys: {kv} == {SAMPLE}")

    # (3) json edge is valid JSON (no external libs involved).
    j = run(["json", '{"disk_pct":81.0,"load1":0.12,"mem_pct":9.0}'])
    parsed = json.loads(j.stdout)
    ok(parsed.get("disk_pct") == 81.0, "json edge re-emits valid JSON with disk_pct=81.0")
    ok(isinstance(parsed, dict), "json edge output is a JSON object")

    # (4) zero external deps is a build property; the crate [dependencies] block is empty.
    cargo_toml = os.path.join(HERE, "native-ser", "Cargo.toml")
    with open(cargo_toml) as f:
        txt = f.read()
    # Only inspect the dependency-declaration block, not prose that mentions "serde".
    deps_block = txt.split("[dependencies]")[-1].split("[")[0] if "[dependencies]" in txt else ""
    ok("[dependencies]" in txt and "serde" not in deps_block and "=" not in deps_block.strip(),
       "native-ser [dependencies] is EMPTY (zero external deps, no serde)")

    print("\n" + ("ALL GREEN" if fails == 0 else f"{fails} FAILURE(S)"))
    sys.exit(1 if fails else 0)


if __name__ == "__main__":
    main()
