# Tasks — finish hydraulic-loop remainder (RED→GREEN)

Ordered, testable increments. Each: RED (failing test) → implement → GREEN (literal cargo test).

## Wave A — bebop Rust gaps (collision-free, max-lane)
- [ ] G1 [LANE:1] stabilizer fail-closed on dt<=0
  RED: `monitor_adaptation(v_cur,v_prev,-1.0)` returns true (allowed) — malformed dt permits motion.
  GREEN: returns false (freeze). Grep guard: `lyapunov_derivative` dt<=0 must NOT yield "allowed".
  Files: crates/bebop/src/stabilizer.rs. Keep existing stabilizer tests green.
- [ ] G2 [LANE:2] coherence index-clamp D2 — skip OOB edge + record error
  RED: edge index > n-1 silently remapped via `b.min(n-1)` corrupts neighbor sum.
  GREEN: OOB edge skipped (no clobber), error counter increments / returns Result.
  Files: crates/bebop/src/coherence.rs. Keep mass-conservation tests green.
- [ ] G3 [LANE:3] active_inference advise validates every b[a] (D9 fail-closed)
  RED: ragged `b` (b[1].len() != n*n) → `advise` panics on `b[a][...]`.
  GREEN: `advise` returns None without panic.
  Files: crates/bebop/src/active_inference.rs. Keep existing active_inference tests green.

## Wave B — operator-gated (NOT auto-executed)
- [ ] G4 [LANE:9] living-knowledge 545f37df merge to feat branch
  BLOCKED on operator decision (risky JS + ONNX embedder spike; recall@5=1.0 proven but tree
  divergent). Present plan, await go-ahead + provide merge strategy. Do NOT force-push.
- [ ] P10 OSS readiness force-push (irreversible)
  BLOCKED: requires ref backup + verify no real secrets. Operator go-ahead only.

## Convergence
- [ ] C1 full `cargo test --workspace` (bebop) + `cargo test` (dowiz kernel) → literal `0 failed`.
- [ ] C2 update ROADMAP-GROUND-TRUTH with verified-done vs planned; push plan to remote first.
