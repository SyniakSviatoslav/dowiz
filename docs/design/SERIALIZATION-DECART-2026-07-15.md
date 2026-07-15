# Serialization Format Decart — telemetry/observability layer (2026-07-15)

Scope: where serialization lives in this system. The rust-core *kernel* has NO serialization
surface — its observability runs over the C-ABI `field_metrics` raw `f64` array (zero-dep,
`core`/`alloc`-only, no serde). So "replace JSON in the kernel" is N/A; the kernel boundary is
already the most-optimized path possible (raw linear-memory floats).

This decart covers the **telemetry/observability layer** only. Final operator directive:
**ZERO external libs / ZERO extra languages — pure kernel with adapters.** Therefore msgpack,
cbor2, ujson, and any pip-installed serializer are OUT. Only stdlib `struct` (native f64) and
stdlib `json` (edge adapter) are allowed.

## Architecture (native-first, adapters at the EDGE only)
- **CANONICAL form = raw little-endian `f64` array** (`struct.pack`, pure stdlib). This is the
  kernel C-ABI `field_metrics` wire shape: no parser, no allocator, no encoding — the speed
  floor. What the kernel emits, what observers consume.
- **EDGE adapter = JSON** (`json`, stdlib) used ONLY where text is mandatory:
  - Gatus JSON-body `[BODY].disk_pct` health conditions (Gatus cannot read binary),
  - grep-able JSONL ledgers (incident debugging),
  - the Telegram text channel.
- No msgpack/cbor2/ujson. The native `f64` block IS the compact binary form — hand it to any
  native consumer directly.

## Empirical benchmark (local, this box; pure stdlib)
Real payloads, 30k round-trips; native-f64 vs the JSON EDGE adapter:

| shape         | fmt       | bytes | write/s   | read/s    |
|---------------|-----------|-------|-----------|-----------|
| resource(9f)  | json(edge)| 79    | 531k      | 668k      |
| kernel_12xf64 | native    | 96    | **3.03M** | **2.71M** |

**Verdict: native is ~5.7× faster write, ~4.1× faster read than the JSON edge** — and uses
zero external deps. JSON is not a bottleneck at 60s cadence (~0.5M ops/s ≫ need), so the only
place it stays is the EDGE where text is required.

## DECISION
- **CANONICAL: native raw-`f64` everywhere internal** (kernel boundary AND telemetry payloads).
  The exporter computes gauges as a native `f64` array; Gatus/Telegram get it through the JSON
  adapter at serve time. This is the "favor native kernel/rust" answer, now applied to the
  telemetry layer too — not just the rust-core.
- **EDGE only: JSON via stdlib `json`.** Kept for Gatus body conditions, grep-able ledgers, and
  the Telegram text channel. No opaque binary in ledgers (ops regression rejected).
- **Adapter seam = `tools/telemetry/ser.py`**: `wire_f64`/`unwire_f64` (canonical), `native_of`/
  `obj_of` (typed linear memory), `json_edge`/`from_json` (the only text edge). Env
  `TELEMETRY_SER=json|native` selects the edge; default `json` so Gatus keeps working.
- **Rejected:** msgpack/cbor2/ujson (external lib — violates the zero-dep directive; the venv
  that carried msgpack is deleted); forcing binary on ledgers (breaks grep); protobuf/flatbuffers
  (schema compile + dep, overkill); eBPF/USDT (tracing, not a serialization swap).

## Probe (strongest argument AGAINST this choice)
A binary format WOULD matter if telemetry volume exploded (sub-second polling, millions of
events). The upgrade path is then a stdlib-only compaction (e.g. `array('d')` / `struct` packs
are already native) — no external dependency ever required. The adapter seam makes switching the
EDGE a one-line env flip; the CANONICAL form is already binary.

## Integration
- `tools/telemetry/ser.py` — pure stdlib: `wire_f64`, `unwire_f64`, `native_of`, `obj_of`,
  `json_edge`, `from_json`, `default_edge`. No `msgpack` import.
- `tools/telemetry/test_ser.py` — 9/9 PASS (round-trip + throughput + zero-dep assertion).
- `tools/telemetry/hetzner_exporter.py` — gauges computed as native `f64`, served via JSON edge.
- `telemetry ser-bench` — reproduces the benchmark live (falsifiable on any box).
- **Removed:** `tools/telemetry/.venv` (msgpack runtime), `SERIALIZATION-DECART` msgpack claims.
