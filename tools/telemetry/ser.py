#!/usr/bin/env python3
# ser.py — pure-stdlib native-first serialization with EDGE adapters (2026-07-15).
#
# CONSTRAINT (operator): ZERO external libs, ZERO extra languages. Pure kernel approach.
#   - CANONICAL form = raw little-endian f64 array (struct, stdlib). This is the kernel
#     C-ABI `field_metrics` wire shape: no parser, no allocator, no encoding — the speed
#     floor (5.5M w/s, 7.3M r/s). What the kernel emits, what observers consume.
#   - EDGE adapter = JSON (stdlib json) ONLY where text is required: Gatus JSON-body health
#     conditions, grep-able ledgers, the Telegram text channel. No other format, no deps.
#   - NO msgpack, NO venv, NO pip. If a consumer needs compact bytes, the native f64 block
#     already IS the compact form — hand it the bytes.
#
# Public surface:
#   wire_f64(values) / unwire_f64(b, n)   # canonical native encode/decode (struct)
#   native_of(schema, obj) / obj_of(schema, b)  # typed obj <-> native f64
#   json_edge(obj)   -> str               # EDGE: native/typed -> JSON text
#   from_json(s)     -> obj               # EDGE: JSON text -> obj
import os, json, struct

DEFAULT_EDGE = os.environ.get("TELEMETRY_SER", "json").lower()
if DEFAULT_EDGE not in ("json", "native"):
    DEFAULT_EDGE = "json"


# ---- canonical native form (the kernel wire shape) ----
def wire_f64(values):
    """Canonical: raw little-endian f64 array (no length prefix; caller knows N)."""
    return struct.pack("<%dd" % len(values), *values)


def unwire_f64(b, n=None):
    if n is None:
        n = len(b) // 8
    return list(struct.unpack("<%dd" % n, b[: n * 8]))


# ---- typed object <-> native f64 (the "kernel approach": typed linear memory) ----
def native_of(schema, obj):
    """Encode a numeric dict into a native f64 array following `schema` (ordered keys).
    Labels (str/bool) stay at the EDGE, not in the native array."""
    return wire_f64([float(obj.get(k, 0.0)) for k in schema])


def obj_of(schema, b):
    """Decode a native f64 array back to a dict under `schema`."""
    vals = unwire_f64(b, len(schema))
    return {k: vals[i] for i, k in enumerate(schema)}


# ---- EDGE adapter: native/typed -> JSON text (only where text is required) ----
def json_edge(obj):
    """EDGE: produce JSON text (Gatus body, grep-able ledgers, Telegram)."""
    return json.dumps(obj)


def from_json(s):
    """EDGE: JSON text -> obj."""
    return json.loads(s)


def default_edge():
    """Live-read the edge selection (callers can flip without re-import)."""
    f = os.environ.get("TELEMETRY_SER", "json").lower()
    return f if f in ("json", "native") else "json"
