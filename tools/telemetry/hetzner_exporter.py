#!/usr/bin/env python3
# hetzner_exporter.py — zero-dep host-resource exporter for Gatus (OPTION C, 2026-07-15).
#
# Serves a tiny JSON snapshot of the Hetzner box's most important resource gauges on
# 127.0.0.1:9091/health. Gatus (running in host-net mode) polls it every 60s and asserts
# [STATUS]==200 + [BODY].disk_pct<90 + [BODY].mem_pct<90, alerting to Telegram topic 257 on
# breach. Near-zero CPU: it only computes on request, no background loop.
#
#   python3 hetzner_exporter.py            # foreground; pair with a supervisor/background
#   curl 127.0.0.1:9091/health             # -> {"disk_pct":81.0,"load1":0.12,"mem_pct":9.0,"ts":...}
#
# Gauges (only the load-bearing ones, per operator):
#   disk_pct  — root filesystem usage %        (the real friction point; alert >90)
#   load1     — 1-min load normalized by #cpu  (alert >1.0 sustained = saturated)
#   mem_pct   — RAM used %                      (alert >90)
import os, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
# Native-first: gauges are computed as a canonical f64 array (kernel C-ABI form), and the
# HTTP EDGE uses an adapter (json | native | msgpack) per TELEMETRY_SER. Default json so
# Gatus JSON-body conditions keep working. Resolve module dir from real script path.
import sys as _sys
_sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
from ser import json_edge, wire_f64, default_edge
ser_wire = wire_f64

HOST, PORT = "127.0.0.1", 9091

# Ordered numeric schema — the canonical native f64 layout (matches field_metrics thinking).
GAUGE_SCHEMA = ["disk_pct", "load1", "mem_pct"]


def gauges_native():
    """CANONICAL: compute host resource state as a native f64 array (kernel form).
    Non-numeric metadata (ts) rides at the edge, not in the native array."""
    st = os.statvfs("/")
    disk_pct = round((1 - st.f_bavail / st.f_blocks) * 100, 1)
    load1, *_ = os.getloadavg()
    ncpu = os.cpu_count() or 1
    load_norm = round(load1 / ncpu, 2)
    mi = {}
    with open("/proc/meminfo") as f:
        for line in f:
            if ":" in line:
                k, v = line.split(":", 1)
                mi[k.strip()] = int(v.split()[0])
    mem_total = mi.get("MemTotal", 1)
    mem_avail = mi.get("MemAvailable", mi.get("MemFree", 0))
    mem_pct = round((1 - mem_avail / mem_total) * 100, 1)
    return [disk_pct, load_norm, mem_pct]


def gauges_dict():
    """EDGE helper: native array + ts label -> JSON-friendly dict (Gatus/Telemetry text)."""
    vals = gauges_native()
    d = dict(zip(GAUGE_SCHEMA, vals))
    d["ts"] = int(time.time())
    return d


class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path not in ("/health", "/"):
            self.send_response(404); self.end_headers(); return
        # CANONICAL native array -> EDGE adapter (json for Gatus/Telemetry; native raw opt-in)
        native = gauges_native()
        edge = default_edge()
        if edge == "native":
            body = ser_wire(native)                              # raw f64 bytes (native consumer)
            ctype = "application/octet-stream"
        else:
            body = json_edge(gauges_dict()).encode("utf-8")     # Gatus/Telemetry JSON edge
            ctype = "application/json"
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):  # quiet (override BaseHTTPRequestHandler)
        pass


if __name__ == "__main__":
    ThreadingHTTPServer((HOST, PORT), H).serve_forever()
