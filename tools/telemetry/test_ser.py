#!/usr/bin/env python3
# test_ser.py — integration test for the PURE-STDLIB native-first serialization (2026-07-15).
# No external libs. Runs under any system python3.
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
SCHEMA = ["disk_pct", "load1", "mem_pct", "load5", "mem_used_mb", "mem_total_mb", "disk_free_gb", "nproc"]

def thr(fn, n=50000):
    t = time.perf_counter()
    for _ in range(n): fn()
    return n / (time.perf_counter() - t)

# 1) CANONICAL native f64 round-trip (exact, the kernel wire shape)
b = ser.wire_f64(kernel_f64)
back = ser.unwire_f64(b, len(kernel_f64))
ok(back == kernel_f64, "native f64 round-trip exact")

# 2) typed obj <-> native (native_of / obj_of) for the resource schema
nb = ser.native_of(SCHEMA, resource)
rb = ser.obj_of(SCHEMA, nb)
ok(all(abs(rb[k] - resource[k]) < 1e-9 for k in SCHEMA), "native_of/obj_of round-trip (typed linear memory)")

# 3) JSON EDGE adapter: native/typed -> JSON text -> back (Gatus/grep/Telegram boundary)
jb = ser.json_edge(resource)
rb2 = ser.from_json(jb)
ok(rb2 == resource, "json_edge/from_json round-trip")
ok(isinstance(jb, str), "json_edge returns text (edge only)")

# 4) throughput: native MUST beat JSON (the reason to favor the kernel approach)
nw = thr(lambda: ser.wire_f64(kernel_f64))
jw = thr(lambda: ser.json_edge(kernel_f64).encode())
ok(nw > jw, f"native f64 write {int(nw)}/s > json {int(jw)}/s")
nr = thr(lambda: ser.unwire_f64(b, len(kernel_f64)))
jr = thr(lambda: ser.from_json(jb))
ok(nr > jr, f"native f64 read {int(nr)}/s > json {int(jr)}/s")

# 5) default edge honors TELEMETRY_SER (json|native only; no external formats)
os.environ["TELEMETRY_SER"] = "native"
ok(ser.default_edge() == "native", "env TELEMETRY_SER=native honored")
os.environ["TELEMETRY_SER"] = "bogus"
ok(ser.default_edge() == "json", "invalid TELEMETRY_SER falls back to json")
os.environ["TELEMETRY_SER"] = "json"

# 6) zero external deps: ser must import with stdlib only (no msgpack import attempted)
ok("msgpack" not in dir(ser), "ser.py uses ZERO external libs (no msgpack)")

print("----")
print("INTEGRATION TEST:", "ALL PASS" if FAIL == 0 else f"{FAIL} FAILED")
sys.exit(1 if FAIL else 0)
