#!/usr/bin/env python3
# test_ser.py — integration test for the serialization adapter (2026-07-15).
# Run with the venv that has msgpack: /tmp/fmtbench-venv/bin/python test_ser.py
import sys, time, os
sys.path.insert(0, os.path.dirname(__file__))
import ser

FAIL = 0
def ok(c, m):
    print(("PASS" if c else "FAIL") + ": " + m)
    global FAIL
    if not c: FAIL += 1

resource = {"disk_pct": 81.5, "load1": 0.04, "mem_pct": 8.5, "ts": 1784134680,
            "load5": 0.10, "mem_used_mb": 1200, "mem_total_mb": 16000, "disk_free_gb": 14, "nproc": 4}
plan_row = {"id": "verify-12345", "title": "Spec-driven DOD reporting",
            "topics": "plan,step,alert,track,retro", "steps": 7, "eta_min": 12,
            "state": "planned"}
kernel_f64 = [0.0, 3.003585, 1.501792, 0.375448, 8.0, 0.004173, 0.5, 0.008346, 2.0, 1.0, 1.0, 1.0]

# 1) JSON round-trip (default) — equality
for name, obj in (("resource", resource), ("plan_row", plan_row)):
    b = ser.dump(obj, "json")
    back = ser.load(b, "json")
    ok(back == obj, f"json round-trip {name}")

# 2) msgpack round-trip — equality (only if msgpack available)
if ser.msgpack is not None:
    for name, obj in (("resource", resource), ("plan_row", plan_row)):
        b = ser.dump(obj, "msgpack")
        back = ser.load(b, "msgpack")
        ok(back == obj, f"msgpack round-trip {name} ({len(b)}B vs json {len(ser.dump(obj))}B)")
        ok(len(b) <= len(ser.dump(obj, "json")), f"msgpack <= json bytes for {name}")
else:
    print("SKIP: msgpack round-trip (not installed in this interpreter)")

# 3) native f64 kernel boundary — round-trip exact, and FASTER than json
b = ser.wire_f64(kernel_f64)
back = ser.unwire_f64(b, len(kernel_f64))
ok(back == kernel_f64, "native f64 round-trip exact")
# throughput sanity: native must beat json on write
def thr(fn, n=50000):
    t = time.perf_counter()
    for _ in range(n): fn()
    return n / (time.perf_counter() - t)
nw = thr(lambda: ser.wire_f64(kernel_f64))
jw = thr(lambda: ser.dump(kernel_f64, "json"))
ok(nw > jw, f"native f64 write {int(nw)}/s > json {int(jw)}/s")

# 4) TELEMETRY_SER env selects default
os.environ["TELEMETRY_SER"] = "msgpack"
if ser.msgpack is not None:
    ok(ser.default_fmt() == "msgpack", "env TELEMETRY_SER=msgpack honored")
else:
    ok(True, "env select skipped (no msgpack)")
os.environ["TELEMETRY_SER"] = "json"

print("----")
print("INTEGRATION TEST:", "ALL PASS" if FAIL == 0 else f"{FAIL} FAILED")
sys.exit(1 if FAIL else 0)
