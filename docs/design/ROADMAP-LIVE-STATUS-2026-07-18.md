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
| A1 eqc-rs extensions / A2 ema organ / A3 money organ / A6 CORDIC | codegen-leg items | NOT VERIFIED this pass — require the eqc-rs compiler leg; deferred (out of scope for a grep-level check). Open. |

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
| capability issuance (IssuanceBudget) | DONE in bebop-repo (`node_id.rs`, max_per_epoch:3, charge_issuance); merged + pushed `332bc59`. |
| R-3 RootDelegationPolicy operator ruling | OPEN — operator decision (audit recommends A/B/C); not code-blocked. |

## Layer E — Network / crypto
| Item | LIVE STATUS |
|---|---|
| AVX2 SIMD softmax lane (`simd.rs`) | DONE — `softmax_scalar` + `softmax_batch_lane`, bit-identity tests GREEN. |
| kalman SoA consumer (§6 TODO) | OPEN — explicitly deferred (touches per-courier filter authority; noted not-done in `simd.rs:21-24`). Authority-adjacent → confirm before building. |

## Layer F — Local AI / MoE mesh
| Status | Mesh protocol consolidated + merged (`cabc01f6a`); E3-Phase-B gated on P06 `key_V`. P06-independent items shipped. |

## Layer G — Product/UI (greenfield `web/`)
| Item | LIVE STATUS |
|---|---|
| `web/` beachhead | `web/` exists (index.html, package.json, serve.mjs, src/app.mjs) but `app.mjs` is EMPTY (0 lines) and NO `FieldSim` wiring present. Genuinely OPEN — browser surface, hard to verify headlessly. Money-flip explicitly gated out. |

## Layer H — Ops / telemetry
| Item | LIVE STATUS |
|---|---|
| `ci.yml:23` bug | verify against live `.github/workflows` — NOT checked this pass. |
| benchmark CI gate / ledger migration | `docs/regressions/REGRESSION-LEDGER.md` exists (P-B G11). |

## What is GENUINELY OPEN (autopilot candidates)
1. **Layer A codegen-leg** (A1/A2/A3/A6) — needs eqc-rs compiler; large, deferred.
2. **Layer B/W2 tensor arena** (`arena.rs`) — large structural, thousands of LOC.
3. **Layer E kalman SoA** — small-ish but authority-adjacent (per-courier filter).
4. **Layer G `web/app.mjs` FieldSim + 21 kernel exports wiring** — browser surface.
5. **Layer D R-3 operator ruling** — decision, not code.

## Conclusion for autopilot
The small/medium in-kernel correctness fixes (Layers A–E minus the codegen-leg/arena/kalman)
are COMPLETE and GREEN. Remaining work is either large structural (arena, eigenvector
masterwork), browser UI (Layer G), or operator-decision/red-line (R-3, RLS/P06). Picking the
next unit requires a scope call on which open item to tackle — they are not interchangeable.

NOTE: blueprints `BLUEPRINT-P-A..P-H` carry STALE "RED today" cites for A4/A5/A7/B-exactly-once/
C-integrity — all already landed. Do not re-derive from those cites; trust live `git`/tests.
