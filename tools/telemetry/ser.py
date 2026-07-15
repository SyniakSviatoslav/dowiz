#!/usr/bin/env python3
# ser.py — serialization adapter between telemetry layers (2026-07-15).
#
# The point is NOT to force one format everywhere. It is an ADAPTER: each layer picks the
# format that fits, and this module is the single seam.
#   - ledgers (JSONL): JSON  -> human-readable, grep-able, debuggable (default)
#   - HTTP exporter:    msgpack when TELEMETRY_SER=msgpack (17% smaller on string-heavy
#                       payloads), else JSON (Gatus reads JSON body conditions)
#   - kernel boundary:  native raw f64 array (the C-ABI field_metrics wire shape) — no
#                       parser, no allocator; the speed floor (5.5M w/s, 7.3M r/s)
#
# Usage:
#   from ser import dump, load, wire_f64, unwire_f64, default_fmt
#   dump(obj)                       -> bytes  (default fmt from TELEMETRY_SER)
#   load(b, fmt="json")             -> obj
#   wire_f64([f64,...])             -> bytes  (native little-endian doubles)
#   unwire_f64(b)                   -> list[float]
import os, json, struct

try:
    import msgpack
except ImportError:
    msgpack = None

DEFAULT = os.environ.get("TELEMETRY_SER", "json").lower()
if DEFAULT not in ("json", "msgpack"):
    DEFAULT = "json"


def dump(obj, fmt=DEFAULT):
    if fmt == "msgpack":
        if msgpack is None:
            raise RuntimeError("msgpack not installed; pip install msgpack")
        return msgpack.packb(obj, use_bin_type=True)
    return json.dumps(obj).encode("utf-8")


def load(b, fmt=DEFAULT):
    if fmt == "msgpack":
        if msgpack is None:
            raise RuntimeError("msgpack not installed; pip install msgpack")
        return msgpack.unpackb(b)
    return json.loads(b.decode("utf-8"))


def wire_f64(values):
    """Kernel boundary: raw little-endian f64 array (no length prefix, caller knows N)."""
    return struct.pack("<%dd" % len(values), *values)


def unwire_f64(b, n=None):
    if n is None:
        n = len(b) // 8
    return list(struct.unpack("<%dd" % n, b[: n * 8]))


def default_fmt():
    """Live-read the env so callers can flip format without re-importing."""
    f = os.environ.get("TELEMETRY_SER", "json").lower()
    return f if f in ("json", "msgpack") else "json"
