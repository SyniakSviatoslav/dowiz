# Off-main feat/* Integration Wave Plan (2026-07-17) — FINAL

Operator directive: integrate the 12 off-main `feat/*` branches. Ground-truth-first
harvest (live `git` metadata, not pasted status). Branches are NOT deleted; ghosts are
marked resolved-in-favor-of-main. Per "commit-history loss non-critical" (operator
2026-07-17) deletion is permitted but NOT done without explicit say-so.

## Population A — recent (2026-07-17), small, CLEAN (0 content conflicts, 0 packages/ resurrections)
| branch | unique vs main | risk | verdict |
|---|---|---|---|
| feat/hermetic-remediation | 1 doc | none | MERGED Wave 1a |
| feat/p08-typed-metrics-prior-run | 2 kernel (typed_metrics.rs, lib.rs) | none | MERGED Wave 1b (+3 tests) |
| feat/spectral-energy-flow-evolution | 7 kernel + 6 doc | lib.rs module decl conflict (resolved: kept both simd+stats) | MERGED Wave 1c |
| feat/agentic-mesh-protocol-2026-07-17 | 8 kernel + 22 doc | event_log stable-id conflict (resolved: kept branch append_raw fix) + 5 NEW non-compiling ports/agent/* files (dropped) | MERGED Wave 1d (partial: value extracted, stale subtree dropped) |

## Population B — ancient forks (base 129f73a427ad, ~864 behind main)
VERIFIED: 7 of 8 are **stale full-kernel snapshots** from 2026-07-13 with **0 NEW kernel
files** vs main (only old versions of existing modules + 22-731 decommissioned
packages/ files + 179-291 conflict markers). Their intent is already on main via P-series
(rw-03 estimate_order_total shipped P07; kalman/mat/pq primitives evolved on main; rw-02
channel.js already decommissioned). VERDICT: RESOLVED-in-favor-of-main, NOT merged.
Left intact on remote for reference.
- feat/kalman-organ
- feat/decentralized-pq-protocol
- feat/remove-legacy-thin-layer
- feat/agent-capability-boost
- feat/markov-attractor-signal
- feat/rw-02-delete-channel-js
- feat/rw-03-kernel-money-authority

EXCEPTION — feat/pq-crypto-tier1: the ONLY ancient branch with **15 genuinely NEW kernel
files** (a self-contained `kernel/src/pq/` subsystem: ML-DSA-65 / ML-KEM-768 / X25519 /
AES-GCM, zero external PQ crates, NIST ACVP KAT vectors vendored). It is REAL KAT-gated PQ
(AGENTS.md D8/D9), not hand-rolled. Extracted (NOT the stale snapshot) as an OPT-IN `pq`
feature so the canonical order/money core stays serde-free (M4 discipline).
- branch `feat/pq-extract` (from main) + `kernel/src/pq/` + KAT vectors + `pq` feature
  (serde/serde_json/aes-gcm/curve25519-dalek gated behind it) + `paste` dev-dep.
- VERIFIED: `cargo test --features pq pq::` → **107 KAT tests byte-exact vs NIST ACVP**,
  0 failed. Default build + 452 tests unaffected (no serde in default graph).
- MERGED + pushed to origin/main `0a85184b0`.

## Waves executed
- Wave 1 (MERGED + pushed `cabc01f6a`): hermetic, p08, spectral, agentic-mesh.
  `cargo build --lib` + `cargo test` green. Final kernel 452 passed. No packages/ in tree.
- Wave 2 (7 stale ghosts): RESOLVED-in-favor-of-main, branches left intact, no merge.
- Wave 3 (pq-crypto-tier1): EXTRACTED as opt-in `pq` feature (`0a85184b0`), 107 KAT tests
  byte-exact vs NIST ACVP. Shipped to main.

## Final ground truth (2026-07-17, main = 0a85184b0)
- 4 recent branches merged (value captured, stale parts dropped).
- 7 ancient stale forks resolved-in-favor-of-main (not merged, intent already on main).
- 1 ancient branch (pq-crypto-tier1) extracted as verified KAT-gated PQ core on main.
- All 12 off-main branches brought to resolution. Kernel: 452 default tests + 107 pq KAT
  tests green. No decommissioned workspace resurrected.

## Status — COMPLETE
- [x] Harvest + classify
- [x] Wave 1 (4 recent) merged + pushed
- [x] Wave 2 (7 stale ghosts) resolved-in-favor-of-main
- [x] Wave 3 (pq-crypto-tier1) extracted as opt-in feature, 107 KAT tests pass, pushed
- [x] Final ground-truth report
