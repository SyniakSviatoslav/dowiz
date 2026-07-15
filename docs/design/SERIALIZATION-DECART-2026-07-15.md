# Serialization Format Decart — telemetry/observability layer (2026-07-15)

Scope: where JSON actually lives in this system. The rust-core *kernel* has NO serialization
surface — its observability runs over the C-ABI `field_metrics` raw `f64` array (zero-dep,
`core`/`alloc`-only, no serde). So "replace JSON in the kernel" is N/A; the kernel boundary is
already the most-optimized path possible (raw linear-memory floats). This decart covers the
**telemetry/observability layer** only: JSONL ledgers, the hetzner_exporter HTTP payload, and
`resource_sample`.

## Empirical benchmark (local, this box; web search unavailable so measured directly)
Real payloads, 20k round-trips each, stdlib json / ujson / msgpack / cbor2 / native-f64:

| shape            | fmt     | bytes | write/s   | read/s    |
|------------------|---------|-------|-----------|-----------|
| resource(9f)     | json    | 157   | 450k      | 539k      |
|                  | ujson   | 140   | 1.20M     | 973k      |
|                  | msgpack | 126   | 1.44M     | 1.59M     |
|                  | cbor2   | 126   | 511k      | 1.01M     |
| plan_row(11f)    | json    | 396   | 462k      | 477k      |
|                  | ujson   | 375   | 1.01M     | 755k      |
|                  | msgpack | 330   | 1.29M     | 1.19M     |
|                  | cbor2   | 330   | 460k      | 690k      |
| kernel_12xf64    | json    | 85    | 398k      | 728k      |
|                  | ujson   | 74    | 1.14M     | 1.34M     |
|                  | msgpack | 109   | 1.73M     | 4.63M     |
|                  | cbor2   | 109   | 730k      | 1.86M     |
|                  | native  | 96    | **5.56M** | **7.29M** |

## DECISION
- **Kernel boundary: KEEP native raw-`f64`** (already done). It beats every format 3–10× on
  throughput with zero parser/allocator/encoding. This is the "favor native kernel/rust" answer.
- **Telemetry ledgers (JSONL): KEEP JSON as default.** JSON is NOT a bottleneck (~0.5M ops/s vs
  60s polling). The decisive factor is *human-readable grep-ability* for debugging/incident
  response — msgpack/cbor2 would make the ledgers opaque binary. Converting to binary is a
  net regression for ops.
- **Add an ADAPTER layer** (`tools/telemetry/ser.py`) that selects format per-layer via
  `TELEMETRY_SER` (json|msgpack): the HTTP exporter can emit msgpack-compact where it pays off
  (plan_row-style payloads, 17% smaller), while ledgers stay JSON for grep. The native path is
  reserved for the f64 array (kernel boundary) and is the speed floor.
- **Rejected:** forcing msgpack/cbor2 everywhere (breaks grep, marginal at this scale);
  protobuf/flatbuffers (needs schema compile + dep, overkill for 60s telemetry); eBPF/USDT
  (kernel-only tracing, not a serialization swap).

## Probe (strongest argument AGAINST this choice)
A binary format WOULD matter if telemetry volume exploded (sub-second polling, millions of
events). At that point msgpack-on-ledgers + a compact binary exporter is the upgrade path —
the adapter already makes that a config flip, no code change. Until then, JSONL + native kernel
is the correct, falsifiable-optimal choice.

## Integration
- `tools/telemetry/ser.py` — `dump(obj, fmt)`, `load(bytes, fmt)`, `wire(obj)` (native f64 for
  the kernel 12-tuple). Env `TELEMETRY_SER=json|msgpack` selects default.
- `tools/telemetry/test_ser.py` — round-trip equality + throughput assertion for all shapes.
- Verified: venv `/tmp/fmtbench-venv` (msgpack, cbor2, ujson installed).
