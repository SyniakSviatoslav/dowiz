# Spec — Kernel-unify autopilot finalization (KU03)

## WHY
Operator directive 2026-07-15: finish literally everything from the big rewrite prompt.
Concretely the live, un-done gaps (verified by deleg_84061e1e + repo reads):
- **M1–M6 money integrity**: `place_order` trusts caller `unit_price` (M1/M2), no trusted
  catalog; currency field is a stub `_currency` ignored (M5). M4 overflow already hardened
  in money.rs. This is LIVE kernel code, not the quarantined attic.
- **VertexBridge** (`engine/src/bridge.rs`): registered but `upload_once()` only increments a
  counter — never touches a GPU. The real "unwired organ".
- **TS legacy layer**: 11,167 `.ts` files (excl. node_modules/attic) — the legacy oracle/UI.
  Grep shows NO TS duplicates the kernel money authority (already Rust-side), so a blind
  delete would break the product. Safe purge = remove TS that re-implements kernel/engine
  compute (scanned: none for money; will scan other patterns); keep the UI shell.
- **Kernel-unify**: done (harmonic ported + parity-gated; eigensolvers parity-locked).

## WHAT (acceptance)
1. Money authority centralized in kernel: `place_order` derives unit prices from a trusted
   catalog (fallback to caller only when catalog absent → explicitly untrusted); currency is a
   real typed field with cross-currency guard. M1/M2/M5 closed. M4 already green.
2. VertexBridge GPU path real behind `feature = "gpu"`: a `wgpu`-backed buffer + genuine
   `queue.write_buffer`; default (headless) build stays offline-clean and carries a
   headless-safe mock so the GREEN gate (exactly 1 real/1 mock upload, 0 json) holds.
3. TS purge: delete TS files that duplicate kernel/engine compute; produce a precise manifest;
   document the remaining legacy UI as out-of-kernel-autopilot-scope.
4. Every change RED→GREEN with real `cargo test`; spec/plan/tasks + DoD plan/step/retro.

## NON-GOALS
- Full Python/TS→Rust rewrite of the UI (multi-week, breaks product). Documented, not done.
- RLS/SQL attic fixes (operator red-line; reactivation gates, already reported).
- Pushing branches (operator gate, unanswered).

## RED-PROOF acceptance
- money: test that place_order WITH a trusted catalog ignores a tampered caller unit_price.
- vertexbridge: test that gpu feature performs exactly 1 write_buffer; headless does 0.
- ts-purge: manifest of deleted files; kernel + engine test suites stay green after.
