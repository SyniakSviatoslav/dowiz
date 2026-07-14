# dowiz / bebop2 — EXECUTION PLAN (bottom-up, non-AI-first, truth-vs-plan)

Generated 2026-07-14 from the 9 design plans + master roadmap. Ground truth
re-verified against the live trees first; stale plan claims corrected inline.
Operator directive: bottom-up (ground → core → surface → platform); non-AI-first
is a HARD filter (LLM/attention/GPU-fitting/quantum-walk deferred); newer
clarifications outrank older on collision; autonomous autopilot; ask only on
red-line (money/auth/RLS/migrations) or true contradiction.

## Ground-truth corrections (truth > plan)
- **dowiz eigensolver duplication is ALREADY dead**: `kernel/src/markov.rs`
  reuses `crate::spectral` (Faddeev-LeVerrier + Durand-Kerner), killing the
  dual-authority hazard. Confirmed by reading the tree.
- **bebop2 STILL has a live dual-authority hazard**: `bebop2/proto-cap/tests/
  mesh_consensus.rs` carries its OWN Faddeev-LeVerrier + Durand-Kerner, while
  `bebop2/core/src/{algebra,fft,chebyshev,lyapunov,active}.rs` also compute
  spectral quantities with NO parity gate between them. This is the same hazard
  markov.rs already closed on the dowiz side. → TOP PRIORITY (A3).
- **eqc already exists + proven** (`tools/eqc`, commit c7c1e0f5): float +
  fixed-point Q-format codegen, SymPy-oracle self-check. NOT to be rebuilt.
- **living-knowledge engine already resurrected** (commit 7848c6d1) but NOT
  wired into the steady retrieval path. → A2 (wire, don't rebuild).
- **3 broken backup scripts already repointed** (commit d92cd6bf); ONE remnant
  remains: `scripts/automation/tier3-batch.sh` still imports deleted `apps/api`.
- **flat-Vec matmul still present** in `kernel/src/spectral.rs` (3 sites) and
  `absorbing.rs` (1 site). Small but violates the DOD invariant.

## Priority order (bottom-up; non-AI-first hard filter)

### Tier 0 — CONSOLIDATE AUTHORITY (foundation of reproducibility)
- **A3** bebop2: extract ONE `core::linalg::eig` (Faddeev-LeVerrier + Durand-Kerner
  already implemented in markov.rs / mesh_consensus.rs) into `bebop2/core`;
  route mesh_consensus + algebra + fft + chebyshev + lyapunov + active through it;
  add a parity-gate test asserting identical spectra vs a second method
  (power-iteration / reference). Kills the last silent-drift hazard.
- **A4** dowiz kernel: `Vec<Vec<f64>>` → contiguous `Vec<f64>` (row-major) in
  spectral.rs/absorbing.rs; expose a CSR-ready layout. DOD + SIMD prep.
- **A1** dowiz: repoint `scripts/automation/tier3-batch.sh` `apps/api` import →
  `attic/apps-api` (matches d92cd6bf). One-line wiring fix.

### Tier 1 — WIRE WHAT'S BUILT (cheap, proven)
- **A2** dowiz: wire resurrected living-knowledge engine into the steady
  retrieval path (M0: biggest win, no new math). Deterministic, no AI.
- **S0.5** dowiz: criterion benchmark of ONE consolidated eigensolve vs the
  naive `Vec<Vec>` copies; record the ns delta. Proves Tier-0 work.

### Tier 2 — NEW ORGANS (non-AI)
- **B4** dowiz: Rust backup organ — content-addressed blocks + recoverable index
  (FastCDC from blueprint), 2-phase ATTIC hardening. Closes off-Hetzner 3-2-1-1-0.
- **B1** bebop2: full Kalman filter — generalize `core::kalman::ema_next`
  (already a 1D Kalman) to courier state (predict=SE(3) transform, update=prob
  correction); feed markov/geo. Non-AI, has a real consumer.
- **B5** dowiz: L0 trigram exact-search index (deterministic grep upgrade).

### Tier 3 — DEFERRED (AI / not-fundamental; blueprints exist, NOT touched yet)
micrograd-autodiff · eqc-IR online learner · wgpu GPU-UI (AccessKit) · capture→
redraw · quantum-walk T2 Szegedy · trained-attention · HMM/Viterbi. Each stays
in its blueprint; autopilot does not start these until Tier 0–2 are green and the
operator flips the gate. NON-AI-FIRST means the core stays deterministic pure-fn.

## Wave assignment (parallel where independent)
- Wave A (Tier 0 ground): A3 (bebop2 eig consolidate) + A4 (dowiz flat→contiguous)
  + A1 (backup repoint). Independent repos → parallel subagents, red-line-safe
  (no money/auth/RLS/migration touched).
- Wave B (Tier 1): A2 (wire LK) + S0.5 (bench). dowiz only.
- Wave C (Tier 2): B4 (backup organ) + B1 (Kalman) + B5 (trigram). Independent.

## Verification gate (every wave)
- bebop2: `cargo test --workspace` → 0 failed (currently 751).
- dowiz: `cargo test -p dowiz-kernel` → 0 failed (currently 152); node web 20/20.
- No merge to main without explicit go-ahead. Commits code-only, --no-verify
  authorized. Push plans first.

## Non-negotiable
- Fail-closed on malformed input (kernel never panics on bad matrix).
- Deterministic: fixed-point / Jacobi, no libm-order-dependent approximation on
  gated paths. Money path untouched (integer tax already proven via eqc).
- Reuse-first: do NOT re-implement an eigensolver; consolidate the existing one.
