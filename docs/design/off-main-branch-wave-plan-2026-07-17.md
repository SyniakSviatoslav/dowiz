# Off-main feat/* Integration Wave Plan (2026-07-17)

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

## Population B — ancient stale forks (base 129f73a427ad, ~864 behind main, ~620-675 ahead)
RE-VERIFIED 2026-07-17: these are NOT additive feature branches — they are **stale full-kernel
snapshots** from 2026-07-13. `kalman-organ`'s "6810 unique insertions" = the entire old
`kernel/src` tree (absorbing.rs/cart.rs/geo.rs/kalman.rs/mat.rs/...) that `main` already
evolved past via the P-series. Verified facts:
- Each reintroduces 22-731 decommissioned `packages/`/`apps/` files + 179-291 conflict markers.
- `rw-03-kernel-money-authority`: `estimate_order_total` already on main (`kernel/src/money.rs:394`, P07).
- `rw-02-delete-channel.js`: `channel.js` count on main = 0 (already decommissioned).
- `kalman-organ` etc.: `kalman.rs`/`mat.rs`/pq primitives ALREADY exist on main (evolved versions).
VERDICT: NOT merged. Merging = replay a 3-week-old kernel over the hardened substrate →
179-291 conflicts + 2000+ dead JS files. Genuinely-new algorithm code (if any) is buried in
stale context and needs surgical EXTRACTION + re-base, not a merge.
→ WAVE 3 (DEFERRED, per-branch re-base + extraction required). Branches left intact on remote
  for targeted value recovery.

## Waves executed
- Wave 1 (MERGED + pushed to origin/main `cabc01f6a`): hermetic, p08, spectral, agentic-mesh.
  Per merged branch: `cargo build --lib` + `cargo test` green. Final kernel: **452 passed, 0 failed**.
  No `packages/`/`apps/` file in any post-merge `main` tree (grep-confirmed 0).
- Wave 2 (GHOSTS): left branches intact; each marked resolved-in-favor-of-main above.
- Wave 3 (DEFERRED): 8 ancient forks — require dedicated per-branch re-base + extraction. Not done this session.

## Status
- [x] Harvest + classify
- [x] Wave 1a hermetic merge
- [x] Wave 1b p08 merge + test (431)
- [x] Wave 1c spectral merge + test
- [x] Wave 1d agentic-mesh merge (partial) + test (452)
- [x] Push origin/main to cabc01f6a (FF, clean)
- [x] Wave 2 ghost rationale recorded
- [ ] Wave 3 per-branch re-base/extraction (DEFERRED, needs operator go + dedicated effort)
