# Kernel-Speed Latency/Friction Upgrade — Verified Decisions (2026-07-15)

## Mandate (operator, autopilot)
System speed = slowest part; always accelerate the slowest. Architectural spine
for ALL new work: native Rust kernel compute (no bash/python in critical path),
rustls+ring pure-Rust TLS (no OpenSSL/curl shell-out), BIDIRECTIONAL async comms
(spool/drainer pattern), VECTORLESS indexed graphs (adjacency + spectral graph
theory + graph ENERGY = Σ|λ|, NOT vector embeddings) + spectral/energy methods
wherever applicable.

## Verified patterns (do not re-litigate; reuse)

### 1. Async comms = spool/drainer (bidirectional, crash-safe)
- Pure kernel half: `kernel/src/spool.rs` — `Spool` state machine
  (append/claim_next/ack/reclaim/compact_drop + backpressure `is_full`).
  Verified-by-Math: 6 GREEN tests. No I/O (pure-std firewall).
- I/O adapter (outside kernel): JSONL file + background drainer. Producer appends
  ONE line (microseconds, never blocks) → returns; drainer claims FIFO, does the
  slow network work, ACKs, removes line. Crash mid-drain = un-ACKed record stays
  & is reclaimed → zero loss.
- Critical-path latency: was ~7.4s/3msgs (sync tg_send) → 361–847ms (kernel
  compute + 1-line spool append + bg drain).

### 2. Vectorless graph energy
- `kernel/src/spectral.rs`: `graph_energy(adj)=Σ|λ|` + `graph_spectrum` →
  {ρ, |λ₂|, gap, λ₂(Laplacian/Fiedler), energy, drift}. Reuses existing
  zero-dep eigensolver. 5 GREEN tests (K3 E=4, empty E=0, disconnected Fiedler=0,
  P3 √2-spectrum, Csr parity).
- `kernel/src/csr.rs`: `Csr::to_adjacency()` + `Csr::energy()` over CSR graph.
- Mirrored into bebop2 core: `bebop2/core/src/energy.rs` (`graph_energy`,
  `spectral_radius`) — 5 GREEN tests. NO new deps (zero-dep rule holds both repos).

### 3. rustls/ring for TLS (no OpenSSL)
- Crate `ureq` with `features=["tls","json"]` → pure-Rust rustls + webpki-roots
  (= ring). Verified absent in dep tree: `openssl`, `native-tls`. Present:
  `rustls`, `ring`.
- Drainer crate: `tools/telemetry/rust-spool` (telegram) + `tools/async-spool`
  (generic: telegram + http dest, deadletter on parse error).

### 4. Reporting = kernel compute, never bash math
- `kernel/src/reporting.rs` (HK-11): `pace_next` (max(last+gap, now), never
  past), card math (ETA/tokens/retro). 8 GREEN tests.
- CLI `hermes-agent-kernel-rewrite/cli` exposes report_plan/step/retro/track →
  `{"text":...}`. Telemetry `tools/telemetry/telemetry` calls the kernel binary
  via `TELEMETRY_KERNEL_BIN` + `_kernel_json`, then `tg_deliver` (spool).
- Parser bug fixed: `title` first, flags any order → kernel computes ETA/tokens
  (was 0min/0tok because `title="$*"` swallowed flags).

## State machine on the two kernels (structural note — NOT yet unified)
- CANONICAL kernel: `/root/dowiz/kernel` (wasm lib, 303 lib tests pass). Has
  spool, spectral/graph-energy, reporting-less. Telemetry invokes a PREBUILT
  `hermes-kernel` binary from `/root/hermes-agent-kernel-rewrite` (separate repo).
- The reporting ops + CLI live in `/root/hermes-agent-kernel-rewrite/hermes-kernel`
  (smaller, has cli/). DIVERGENCE: two kernels. Unifying is a structural
  red-line-adjacent decision — left for operator, documented, not done unilaterally.

## Test baseline (2026-07-15)
- dowiz/kernel: 303 lib pass; 1 pre-existing FAIL (`living_knowledge::
  adapter_routes_query_to_bridge_and_ranks_correctly`) — shells out to `node`
  which crashes on this box (node internal-modules error); reproduced on a clean
  `git stash` → environmental, NOT a regression. IGNORE for upgrade gating.
- bebop2/core: energy tests 5/5 green; core builds (6 pre-existing warnings).

## Commits this session
- f199c9e3 feat(telemetry): kernel-speed reporting — pure kernel compute + async spool drain
- 6e6f9b7d feat(kernel): vectorless graph-energy core — E=Σ|λ| + graph_spectrum
- d39621e0 feat(kernel): spool — pure crash-safe async work-queue state machine
