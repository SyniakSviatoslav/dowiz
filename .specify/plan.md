# Plan — hydraulic-loop-v2 + living-knowledge remainder

## Ground-truth finding (deep research, 2026-07-14)
A full scan of `docs/design/hydraulic-loop-v2/BLUEPRINTS.md` (23 BPs / 5 waves) against the live
bebop + bebop2 trees shows **BPs 01–21, 15, 17, 18–20 are already built AND committed** — the
bebop baseline is `cargo test --workspace` green (777 tests, 0 failed), working tree clean. The
"missing core features" fear was unfounded: deep research *prevented* re-doing finished work.

## Genuine remaining gaps (verified against live files)
| id | file | gap | verify |
|----|------|-----|--------|
| G1 | bebop/stabilizer.rs | `dt<=0 → return 0.0` = fail-open (adaptation allowed on malformed dt). Should freeze/refuse. | RED: dt=-1 allows motion; GREEN: frozen. |
| G2 | bebop/coherence.rs | `acc -= u[b.min(n-1)]` silently remaps out-of-range edge → must skip edge + error (D2). | RED: OOB edge corrupts sum; GREEN: skipped. |
| G3 | bebop/active_inference.rs | `advise()` validates only `b[0]` length, not every `b[a]` (D9 fail-open panic). | RED: ragged b[a] panics; GREEN: None. |
| G4 | living-knowledge 545f37df | recall@5=1.0 engine built but on recover/stash-1 — NOT on feat branch. | operator decision (risky JS + ONNX). |

## Out-of-scope (verified already done / red-line)
- BP-22 resonator.ts, BP-23 #2/#3/#4 reference files absent from this tree → not in current
  branch; flagged, not fabricated.
- P10 force-push (irreversible) → operator go-ahead + ref backup required; NOT auto-run.

## Architecture
All gaps are pure-Rust, std-only, collision-free (different files in bebop-repo). Implement as
RED→GREEN: write failing test → fix → green via `cargo test -p bebop`. No shared mutable state →
max-lane parallel safe (but I execute sequentially here to re-verify each with literal output).

## RED→GREEN per gap
- G1: test `dt=-1.0` → `monitor_adaptation` returns false (freeze); currently returns true (allowed).
- G2: test OOB edge index → propagate must skip + (count error or saturate), not silently clobber.
- G3: test ragged `b` (b[1] wrong len) → `advise` returns None, no panic.

## Risks
- G1/G2 change behavior of existing functions → MUST keep all existing coherence/stabilizer tests green.
- G4 is a cross-branch merge of a JS+ONNX spike → needs operator sign-off, not silent autopilot.
