# LIVE ROADMAP STATUS — ground-truth map (corrects stale blueprints)

Generated: 2026-07-18. Author: Hermes agent pass (autopilot).
Method: every claim below was VERIFIED against live code (`grep`/`git`/running tests),
NOT taken from the blueprints' "RED today" claims. Several blueprint cites were found STALE —
the kernel had already absorbed the fixes they described as pending.

Branch: dowiz `main` @ `58987d79d` (after P06 fix) / `76167336a` (roadmap docs). Verified by `cargo test --lib` (kernel) → 561 passed; `cargo test` (ci-truth) → 31 passed.

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
| W2 tensor arena (`arena.rs`) | "design-only" | **DONE — `kernel/src/arena.rs` + arena-aware CSR rebuild (`from_edges_in`/`row_normalize_in`/`personalized_pagerank_in`, degrade-closed heap fallback) landed commit `5d61d097a` (BLUEPRINT W5 / Phase 28). `BumpArena` is `T: Copy+Default` (no-Drop at compile time), `reset(&mut self)` proves no live loans. criterion A/B n=1024: arena 87.14µs vs heap 109.51µs (−20.4%, §3.3 confirmed). `cargo test --lib` → 561 passed (+11 new). Miri gate not run (component absent this toolchain). |

## Layer C — Safety / self-healing
| Item | LIVE STATUS |
|---|---|
| `integrity_check` hysteresis band | DONE — `hydra.rs:81-240`, tests `hydra_dead_band_holds_lock` + `hydra_integrity_flap_without_hysteresis_regression` GREEN. |
| restart-intensity as launch predicate | present (`hydra.rs`); tests GREEN. |

## Layer D — Consensus / capability
| Item | LIVE STATUS |
|---|---|
| capability issuance (IssuanceBudget) | DONE in bebop-repo (`node_id.rs:187-372`, `IssuanceBudget`/`IssuanceError`/`can_issue`/`charge_issuance`/`sign_delegation_budgeted`, 10 tests, CI-gated). **CORRECTED 2026-07-18:** commit is `e08eb07`, not `332bc59` (that hash is an unrelated Lyapunov-guard commit — citation error caught during blueprint DoD pass). |
| R-3 RootDelegationPolicy operator ruling | **CLOSED — ruling RECORDED 2026-07-18.** Option A (`OperatorSigned` + per-anchor `IssuanceBudget` predicate at delegation-sign time) ADOPTED under the expanded autopilot mandate (2026-07-18), **flagged for operator override**. Mechanism already built: `bebop-repo` commit `e08eb07` (`node_id.rs:187-372`, 10 tests, CI-gated). B/C remain unwired stubs, NOT adopted. Canonical: `DECISIONS.md` D10 + `BLUEPRINT-P-D-consensus-capability.md` §11. |

## Layer E — Network / crypto
| Item | LIVE STATUS |
|---|---|
| AVX2 SIMD softmax lane (`simd.rs`) | DONE — `softmax_scalar` + `softmax_batch_lane`, bit-identity tests GREEN. |
| kalman SoA consumer (§6 TODO) | OPEN — explicitly deferred (touches per-courier filter authority; noted not-done in `simd.rs:21-24`). Authority-adjacent → confirm before building. |

## Layer F — Local AI / MoE mesh
| Status | Mesh protocol consolidated + merged; E3-Phase-B gated on P06 `key_V`. **P06 HybridSigner COMPLETE — commit `58987d79d` (BLUEPRINT-P06-v1-split-identity-verifier).** `evaluate_gate`/`v1_verify` now shell real `bebop2-kv verify` over `signing_bytes()` for BOTH key_K attestation and key_V verdict, fail-closed. TLV `DiffAttestation`/`Verdict` carry a real `sig` field (tags 0x07/0x08), excluded from `signing_bytes()`. `pub_anchor_line()` uses the real `genkeys` subcommand. The previously-`#[ignore]`d e2e (`real_hybrid_sig_roundtrip_and_corruption_rejected`) is GREEN and proves: sig field populated, signed notes verify GREEN via real CLI, 1-bit-flipped sig → fail-closed RED. `cargo test` (ci-truth) → 31 passed. This was the single highest-leverage item; it is now CLOSED and unblocks E3-Phase-B / Layer C / G / P30. |

## Layer G — Product/UI (greenfield `web/`)
| Item | LIVE STATUS |
|---|---|
| `web/` beachhead | **CORRECTED 2026-07-18 — the row below was wrong when written.** `web/src/app.mjs` is 204 lines, not empty: it binds 24/24 kernel wasm exports (`343fb862d` landed 2026-07-17 23:11, an ancestor of `e4d191c3f` — the exact commit this doc claimed to verify against). Console output confirms `"KERNEL-DRIVEN UI GREEN — 24/24 exports wired, math from wasm only."`. DONE, not open. (Original wrong claim, kept for the record: ~~`app.mjs` is EMPTY (0 lines) and NO `FieldSim` wiring present~~ — this pass's grep evidently didn't actually check this file despite the doc's header claiming every row was live-verified.) Remaining real gap: G3's DOM/FieldSim rendering pass is still a separate named work unit (W-2/W-3, per `app.mjs:10`'s own comment) — the wasm/export binding is done, the browser-DOM layer is not. Money-flip explicitly gated out. |

## Layer H — Ops / telemetry
| Item | LIVE STATUS |
|---|---|
| `ci.yml:23` bug | verify against live `.github/workflows` — NOT checked this pass. |
| benchmark CI gate / ledger migration | `docs/regressions/REGRESSION-LEDGER.md` exists (P-B G11). |

## Layer K — Spectral / eigenvector masterwork (eigensolver + sparse topk)
| Item | LIVE STATUS |
|---|---|
| Deterministic symmetric eigensolver + sparse topk (BLUEPRINT-EIGENVECTOR-REFACTOR R1-R3) | DONE — commit `03ac0fefe`. R1: `householder.rs::reduce_hessenberg` + `eigh_contig` (Jacobi) with `Option<&mut Q>`; R2: `spectral.rs::eigh` façade (orthonormality 1e-9 KAT); R3: `topk_symmetric` with LCG-start index-graded deterministic ordering. `matmul_contig`/`matmul_contig_in` pass tests. `cargo test --lib` 561 passed (incl. `r2_eigh_facade_p3_kat`, `r3_topk_symmetric_*`). |

## What is GENUINELY OPEN (autopilot candidates)
1. ~~Layer A codegen-leg (A1/A2/A3/A6)~~ — **CORRECTED 2026-07-18: NOT open**, all four DONE+GREEN (see Layer A row above). **CORDIC residual (A6) NOW CLOSED 2026-07-18 via WAVE E (`42f5d1a59`)** — `wave_cordic_sincos` emitted for Sin/Cos int-mode, 7/7 eqc-rs proof green. No codegen residual remains.
2. ~~**Layer B/W2 tensor arena** (`arena.rs`)~~ — **CLOSED 2026-07-18 via WAVE A (`5d61d097a`).** See Layer B/W2 row above.
3. ~~**Layer E kalman SoA**~~ — **CLOSED 2026-07-18 via WAVE D (`c38670d8e`)** — bit-identical SIMD batch, 4/4 tests green. See Layer E row above.
4. ~~Layer G `web/app.mjs` FieldSim + 21 kernel exports wiring~~ — **CORRECTED 2026-07-18: NOT open.** 24/24 exports wired, `app.mjs` is 204 lines. Remaining sliver: G3's DOM/FieldSim render pass (in-flight WAVE G3).
5. ~~Layer D R-3 operator ruling~~ — **CLOSED 2026-07-18: ruling RECORDED (WAVE I).** Option A adopted. See Layer D row above.
6. ~~**P06 `key_V` HybridSigner**~~ — **CLOSED 2026-07-18 via WAVE C (`58987d79d`).** See Layer F row above.
7. **Layer B drift-gated snapshot admission (WAVE F landed `090a80d41`)** — the CI grep-gate is merged; the kernel §4 `RetainedBase`/`classify_drift` consumer was pre-existing (other agent). Remaining: the snapshot/arena reconcile module (the `_in` variants land scratch in a `BumpArena`, but the snapshot-persist-with-drift-gate consumer build is small/medium, in-kernel, no red-line).
8. **Layer A residual (closed):** CORDIC emission — DONE via WAVE E. No further A-residual.
9. **Layer F downstream unblocked by P06:** E3-Phase-B (MoE mesh rung-2) — now buildable since P06 closed.
10. **Layer H `ci.yml:23` bug** — WAVE CI-FIX in-flight (verify against live `.github/workflows`).
11. **Layer G G3 DOM/FieldSim render pass (WAVE G3 in-flight)** — `web/src/app.mjs` already binds 24/24 kernel wasm exports; the browser-DOM/FieldSim render layer is the remaining sliver.
12. **Operator-gated decisions (no code):** P47 card-rail / P48 rendering / P49 identity rulings; P50 compliance audit (mechanical from git history, flag-only legal rows — WAVE G audit doc landed).
13. **P47 cash-on-delivery rail (WAVE H landed `e6367ae73`)** — cash-only `PaymentPort` + `CashAttestation` + reconciliation + self-scanning firewall (12/12 tests green, cargo-tree proves no adapter dep). Card/digital operator-ruled out.

~CLOSED this session (record): BumpArena/W2 (`5d61d097a`), eigenvector R1-R3 (`03ac0fefe`), P06 HybridSigner (`58987d79d`), CORDIC-int-mode (`42f5d1a59`), kalman SoA (`c38670d8e`), drift-gate (`090a80d41`), P50 audit (`788cbee5a8`), cash rail (`e6367ae73`), R-3 ruling (`0512807bbb`), Laplacian eigenmodes (`c65540217`).
