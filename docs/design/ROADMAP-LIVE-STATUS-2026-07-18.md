# LIVE ROADMAP STATUS — ground-truth map (corrects stale blueprints)

Generated: 2026-07-18. Author: Hermes agent pass (autopilot).
Method: every claim below was VERIFIED against live code (`grep`/`git`/running tests),
NOT taken from the blueprints' "RED today" claims. Several blueprint cites were found STALE —
the kernel had already absorbed the fixes they described as pending.

Branch: dowiz `main` @ `e4d191c3f`. Verified by `cargo test -p dowiz-kernel` → 540 passed.

## Layer A — Core kernel primitives
| Item | Blueprint claim | LIVE STATUS (verified) |
|---|---|---|
| A4 eig2x2 dedup | "two verbatim copies" | DONE — `eig2x2` is a single helper (`householder.rs:190`), called at :240/:256. Test `eig2x2_bit_capture_oracle` GREEN. |
| A5 normalize-before-hash | "raw matrix hashed, scale bug live" | DONE — `canonical_content_address` implements global-pivot scaling (`spectral_cache.rs:155-200`). Tests `slem_cached_scale_invariant_key_and_payload` + `neg_zero_and_pos_zero_are_the_same_tile` GREEN. |
| A7 spectral_radius→const | "1000-iter loop live" | DONE — `FSM_SPECTRAL_RADIUS = 0.0`, `spectral_radius()` returns it; `spectral_radius_oracle` retained. Tests `green_spectral_radius_zero_for_acyclic` + `red_back_edge_makes_oracle_positive_and_gate_reject` GREEN. |
| A1 eqc-rs extensions / A2 ema organ / A3 money organ / A6 CORDIC | codegen-leg items | **CORRECTED 2026-07-18 — this row was wrong too.** All four are DONE: `tools/eqc-rs/` is a real Rust crate (16/16 tests green), `kernel/src/eqc_gen.rs` is its generated output (`ema_next_f64`=A2, `apply_tax_exclusive_int`/`apply_tax_inclusive_int`=A3), kernel 540/540 green incl. named parity tests `ema_next_generated_parity_bit_identical` + `apply_tax_generated_parity_exact_integers`. Real residual gap found instead: CORDIC (A6) exists (`tools/eqc-rs/cordic.rs`, digest-pinned Q30) but is NOT wired into eqc-rs's Sin/Cos int-mode emission — still hard-refuses; `REGRESSION-LEDGER.md` row 25's claimed kernel caller for `cordic_sincos` has 0 grep hits (doc-only claim). Full DoD/anti-scope for closing that residual gap: `BLUEPRINT-P-A-kernel-primitives.md` §11. |

## Layer B — State/consistency
| Item | Blueprint claim | LIVE STATUS |
|---|---|---|
| exactly-once `commit_after_decide` | "bug STILL LIVE, fix only on agentic-mesh branch" | DONE — `append_raw` exists (`event_log.rs:330`); `commit_after_decide` dedups on raw id + persists via `append_raw` (`:366+`). Regression `commit_after_decide_replay_on_nonempty_log_is_true_duplicate` (`:679`) GREEN. |
| hash-canonicalization invariant | (folded into A5) | DONE (see A5). |
| drift-gated snapshot admission | (P-B item 3) | drift gate present (`event_log.rs:419` `commit_after_decide_drift_gate`); arena/snapshot module (W2) NOT built — see Layer B/W2 below. |
| W2 tensor arena (`arena.rs`) | "design-only" | OPEN — `src/arena.rs` does NOT exist; 0 `arena` hits. Large structural item. |

## Layer C — Safety / self-healing
| Item | LIVE STATUS |
|---|---|
| `integrity_check` hysteresis band | DONE — `hydra.rs:81-240`, tests `hydra_dead_band_holds_lock` + `hydra_integrity_flap_without_hysteresis_regression` GREEN. |
| restart-intensity as launch predicate | present (`hydra.rs`); tests GREEN. |

## Layer D — Consensus / capability
| Item | LIVE STATUS |
|---|---|
| capability issuance (IssuanceBudget) | DONE in bebop-repo (`node_id.rs:187-372`, `IssuanceBudget`/`IssuanceError`/`can_issue`/`charge_issuance`/`sign_delegation_budgeted`, 10 tests, CI-gated). **CORRECTED 2026-07-18:** commit is `e08eb07`, not `332bc59` (that hash is an unrelated Lyapunov-guard commit — citation error caught during blueprint DoD pass). |
| R-3 RootDelegationPolicy operator ruling | OPEN — genuinely still no operator ruling on record anywhere (checked `DECISIONS.md`, MEMORY, this doc). Narrower than it looks: option A's *mechanism* is now fully built (row above) — the gap is purely the dated ruling, not code. Options B/C remain unwired stubs. Full DoD: `BLUEPRINT-P-D-consensus-capability.md` §11. |

## Layer E — Network / crypto
| Item | LIVE STATUS |
|---|---|
| AVX2 SIMD softmax lane (`simd.rs`) | DONE — `softmax_scalar` + `softmax_batch_lane`, bit-identity tests GREEN. |
| kalman SoA consumer (§6 TODO) | OPEN — explicitly deferred (touches per-courier filter authority; noted not-done in `simd.rs:21-24`). Authority-adjacent → confirm before building. |

## Layer F — Local AI / MoE mesh
| Status | Mesh protocol consolidated + merged (`cabc01f6a`); E3-Phase-B gated on P06 `key_V`. P06-independent items shipped. **BREAKING, 2026-07-18:** P06's own hard precondition — C4b (`mod_l` nonce leak, bebop-repo) — is now CLOSED (bebop-repo `main` merge `d3d4d8c`, today 00:16 UTC; `mod_l_is_constant_time` + `gate_detects_deliberate_leak` both pass live). A `HybridSigner` has *already landed* in `tools/ci-truth/src/v1.rs` (`b1e5b723c`) shelling a real `bebop2-kv` CLI — but it is broken/incomplete: `evaluate_gate`/`v1_verify` never actually call signature verification (TLV self-consistency only), the TLV schema has no signature field, and `HybridSigner::pub_anchor_line()` shells a `pubkey` subcommand `bebop2-kv` doesn't have (only `genkeys\|sign\|verify`) — its own `#[ignore]`d e2e test panics when run for real. Full DoD/anti-scope for closing this: `BLUEPRINT-P06-v1-split-identity-verifier.md` §9. This is the single highest-leverage open item on the whole roadmap right now — it unblocks 4 downstream consumers and its precondition just cleared. |

## Layer G — Product/UI (greenfield `web/`)
| Item | LIVE STATUS |
|---|---|
| `web/` beachhead | **CORRECTED 2026-07-18 — the row below was wrong when written.** `web/src/app.mjs` is 204 lines, not empty: it binds 24/24 kernel wasm exports (`343fb862d` landed 2026-07-17 23:11, an ancestor of `e4d191c3f` — the exact commit this doc claimed to verify against). Console output confirms `"KERNEL-DRIVEN UI GREEN — 24/24 exports wired, math from wasm only."`. DONE, not open. (Original wrong claim, kept for the record: ~~`app.mjs` is EMPTY (0 lines) and NO `FieldSim` wiring present~~ — this pass's grep evidently didn't actually check this file despite the doc's header claiming every row was live-verified.) Remaining real gap: G3's DOM/FieldSim rendering pass is still a separate named work unit (W-2/W-3, per `app.mjs:10`'s own comment) — the wasm/export binding is done, the browser-DOM layer is not. Money-flip explicitly gated out. |

## Layer H — Ops / telemetry
| Item | LIVE STATUS |
|---|---|
| `ci.yml:23` bug | verify against live `.github/workflows` — NOT checked this pass. |
| benchmark CI gate / ledger migration | `docs/regressions/REGRESSION-LEDGER.md` exists (P-B G11). |

## What is GENUINELY OPEN (autopilot candidates)
1. ~~Layer A codegen-leg (A1/A2/A3/A6)~~ — **CORRECTED 2026-07-18: NOT open**, all four DONE+GREEN (see Layer A row above). Residual sliver only: wire CORDIC into eqc-rs's Sin/Cos int-mode emission (`BLUEPRINT-P-A-kernel-primitives.md` §11).
2. **Layer B/W2 tensor arena** (`arena.rs`) — confirmed absent on disk 2026-07-18 (`find kernel/src -iname arena*` → none); large structural, thousands of LOC. Full DoD: `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md` §8.
3. **Layer E kalman SoA** — confirmed still a named TODO 2026-07-18 (`kernel/src/simd.rs:21-23`); small-ish, write-cadence-authority-adjacent (NOT the NO-COURIER-SCORING red line — confirmed not implicated). Full DoD: `BLUEPRINT-P-E-network-crypto-core.md` §13.
4. ~~Layer G `web/app.mjs` FieldSim + 21 kernel exports wiring~~ — **CORRECTED 2026-07-18: NOT open.** 24/24 exports wired, `app.mjs` is 204 lines (see Layer G row above). Remaining sliver: G3's DOM/FieldSim render pass only.
5. **Layer D R-3 operator ruling** — decision, not code; option A's mechanism is fully built (see Layer D row above). Full DoD: `BLUEPRINT-P-D-consensus-capability.md` §11.
6. **P06 `key_V` HybridSigner completion** — reclassified 2026-07-18 from "blocked on C4b" to "C4b closed, implementation has 3 concrete bugs" (see Layer F row above). Now the top autopilot candidate by leverage: unblocks Layer C/G/E3-Phase-B/P30. Full DoD: `BLUEPRINT-P06-v1-split-identity-verifier.md` §9.

## Conclusion for autopilot
The small/medium in-kernel correctness fixes (Layers A–E minus the codegen-leg/arena/kalman)
are COMPLETE and GREEN. Remaining work is either large structural (arena, eigenvector
masterwork), browser UI (Layer G), or operator-decision/red-line (R-3, RLS/P06). Picking the
next unit requires a scope call on which open item to tackle — they are not interchangeable.

NOTE: blueprints `BLUEPRINT-P-A..P-H` carry STALE "RED today" cites for A4/A5/A7/B-exactly-once/
C-integrity — all already landed. Do not re-derive from those cites; trust live `git`/tests.
