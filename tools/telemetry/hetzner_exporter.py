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
# Adapter seam: exporter payload format selectable per-layer via TELEMETRY_SER
# (json | msgpack). Default json so Gatus JSON-body conditions keep working.
# Resolve module dir from the real script path (works if copied/moved, not just cwd).
import sys as _sys
_sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
from ser import dump as _ser_dump

HOST, PORT = "127.0.0.1", 9091


def gauges():
    # disk%
    st = os.statvfs("/")
    disk_pct = round((1 - st.f_bavail / st.f_blocks) * 100, 1)
    # load1 normalized by cpu count
    load1, *_ = os.getloadavg()
    try:
        ncpu = os.cpu_count() or 1
    except Exception:
        ncpu = 1
    load_norm = round(load1 / ncpu, 2)
    # mem%
    mi = {}
    with open("/proc/meminfo") as f:
        for line in f:
            if ":" in line:
                k, v = line.split(":", 1)
                mi[k.strip()] = int(v.split()[0])
    mem_total = mi.get("MemTotal", 1)
    mem_avail = mi.get("MemAvailable", mi.get("MemFree", 0))
    mem_pct = round((1 - mem_avail / mem_total) * 100, 1)
    return {"disk_pct": disk_pct, "load1": load_norm, "mem_pct": mem_pct, "ts": int(time.time())}


class H(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path not in ("/health", "/"):
            self.send_response(404); self.end_headers(); return
        body = _ser_dump(gauges())
        self.send_response(200)
        ctype = "application/msgpack" if os.environ.get("TELEMETRY_SER") == "msgpack" else "application/json"
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):  # quiet (override BaseHTTPRequestHandler)
        pass


if __name__ == "__main__":
    ThreadingHTTPServer((HOST, PORT), H).serve_forever()
