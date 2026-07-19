# META-GAP AUDIT — 2026-07-18/19 research wave (2026-07-19)

> **Planning document — writes ZERO product code, touches no branches, pushes nothing.**
> Operator-commissioned meta-review of everything produced in the 2026-07-18→19 research session:
> the `MASTER-STATUS-LEDGER-2026-07-19.md`, the 3 prior syntheses (S1/S2/S3), the mesh refactor
> plan, and all **21** blueprint files P75–P96 (P84 is operator-gated with no file). The brief:
> find every omission, deferred-"to-better-times" item, blind spot, or gap **not already captured
> as a tracked item anywhere**, and specifically assess coverage of cross-platform, cross-device,
> testing, logging/telemetry, metrics, agent/model verification, code-review/quality, and
> interface/UI checks. Method: six parallel deep-reads of the full blueprint set + the P38
> WebGPU-fallback canon + the P54/P55/P56 verification trio; every load-bearing claim below was
> ground-truthed against the files on disk this pass. Findings cite blueprint section numbers.

**Honest headline.** The wave is high quality. Most blueprints carry *real* falsifiable DoDs with
named RED tests, honest "cut-if-no-measured-win" exits, and correctly-labelled scope exclusions —
the master ledger's §6 self-assessment ("short on landed wiring, not on sound design") holds up
under this second adversarial read. The gaps below are genuine but **concentrated**: they cluster
into four systemic issues (an orphaned-from-tracking pair, a missing cross-device DoD line on the
GPU blueprints, a bench-gate that structurally does not run the crates two blueprints depend on,
and a missing crypto conformance-vector prerequisite) plus a tail of medium/low per-blueprint
items. Several dimensions the brief asked about (agent/model verification, crypto cross-device)
came back **correctly out of scope** — recorded here as non-gaps so they are not re-litigated.

---

## 1. Findings table

Severity: **HIGH** = a stated DoD is unsatisfiable as written, a standing project gate is
violated, or a produced item is untracked. **MEDIUM** = a real hidden prerequisite / undocumented
conflict a worker will hit. **LOW** = a caveat worth recording, defensible within the item's scope.
**NON-GAP** = the brief asked; the answer is "correctly covered / correctly excluded."

| # | Blueprint / area | Gap found | Severity | Recommendation |
|---|---|---|---|---|
| **G1** | **P95, P96 vs MASTER-STATUS-LEDGER** | Two complete, DoD-bearing blueprints (`P95` living-memory index persistence; `P96` ETA-from-live-speed) exist on disk but appear **nowhere** in the ledger — not in the §1 status table, not in the §4 work list. Verified: `grep` for "P95"/"P96" in the ledger = **0 hits**; the only in-dir references are the two files themselves. Cause is benign (both written 01:17/01:19, ~13–14 min *after* the ledger was frozen at 01:05) — they are genuine late additions from the same wave, **not scratch or superseded**. Effect is real: the handoff artifact that claims to be "the single consolidation of the entire session" is incomplete. | **HIGH** (tracking integrity) | Back-register both in the ledger §1 + §4: **P95** = FULLY-BLUEPRINTED / HOLD (gated on precondition P95-C1, a real repeated-write/query caller); **P96** = FULLY-BLUEPRINTED / GO (non-red-line product fix). Both are standalone — no dependency in either direction on P75–P94. |
| **G2** | **P86, P87, P88 (physics/GPU) vs P38 standing gate** | P38 §12.3 establishes a **standing gate**: *"every surface blueprint carries the FE-16 'renders correctly on the WebGL2 and CPU floors' DoD line."* **None of P86/P87/P88 carries that DoD line.** Worse, **P88's atomicity-by-default policy is written entirely for the WGSL/WebGPU compute model** (`atomicAdd`, `workgroupBarrier`, `var<storage,read_write>`, two-level workgroup reductions) — **none of which exist in WebGL2** (no compute shaders, no atomics) — and never states whether the policy is vacuous, WebGPU-only, or needs a separate multi-pass reduction spec on the fallback rung P38 mandates must run on the courier's mid-tier phone. P87's GPU leg uses per-cell atomic-scatter + texel-skip with no WebGL2 analog; P86 introduces `R32Uint` without confirming it is renderable on the WebGL2/mobile floor. | **HIGH** (P88), **MEDIUM** (P86/P87) | Add the FE-16 WebGL2/CPU-floor DoD line to all three. P88 must add an explicit scope clause: either (i) the compute legs are WebGPU-only and the WebGL2 floor never runs them, or (ii) the WebGL2 rung is fragment-only with a distinct multi-pass reduction spec — commit to one. *Partially mitigated:* all three are build-gated on the P38 §4.2 operator GPU-compute decision (OD-11) and none silently takes it, so nothing ships un-gated; but the DoD line and P88's policy-scope clause are still absent. |
| **G3** | **P81 + P82 (bench harnesses) — gate-ownership hole** | P75's bench-id/baseline **schema** is real and complete, but P75's *running* CI gate is **dowiz-kernel-only** (`cd kernel && cargo bench --bench criterion`; verified against P75 §5 and dowiz `ci.yml`). P75 §2.2 disclaims wiring engine/bebop into a gate; P81/P82 §2 disclaim gate semantics as "P75's." Net: **no blueprint owns creating the same-runner A/B bench-regression CI job for (a) the dowiz `engine` crate or (b) the bebop repo.** Yet P81 D3 and P82 D4 both assert "an injected slowdown trips **the P75 gate** RED" — a DoD that cannot pass as written because no gate runs their crates. P82 is worst: **cross-repo** — bebop's own `ci.yml` has zero bench jobs; `native-trackers`/`bench_track.py`/the schema live only in dowiz — so P82's per-frame "continuous-gate" value proposition is structurally broken. | **HIGH** | Add an explicit owner for the per-crate/per-repo A/B bench job: either broaden P75's M1 scope beyond kernel, or give P81 (engine job) and P82 (a *bebop-repo* CI bench job) each their own gate-wiring deliverable. Until then P81/P82 baselines are recorded-but-ungated and their D3/D4 DoDs are unmeetable. |
| **G4** | **P91 (KEM ring correction) — missing conformance-vector prerequisite** | P91's entire D3 conformance spine is `kem_acvp_encaps_decaps_byte_exact`, but **`kernel/src/pq/kat/acvp/kem-encap-decap.json` does not exist** (verified: the ACVP dir holds only the 3 ML-DSA files `key-gen/sig-gen/sig-ver.json`), and there is **no KEM-shaped ACVP loader harness** in-repo (the 107 existing KATs are ML-DSA-only). P91 names the vector *category* ("real NIST ACVP ML-KEM-768 vectors") but **pins no concrete artifact** — no NIST URL, version tag, or generation script — and its port source is an **unpinned working-tree path** (`/root/bebop-crypt/...`), unlike P85 which pins commit `986646a`. The FIPS-203 compliance proof for a crypto correctness fix therefore rests on a vector set that neither exists nor has tracked provenance. | **HIGH** | Track "acquire + pin ML-KEM-768 ACVP vectors (with provenance) and write the KEM ACVP loader harness" as an explicit P91 prerequisite. Anchor the schoolbook port source to a commit, matching P85's discipline. |
| **G5** | **P85, P91, P92 (all crypto) — constant-time verification absent** | No crypto blueprint mandates a `dudect`/timing/secret-dependent-branch check; project memory confirms the codebase has "0 dudect/fuzz/proptest" on this surface. P85's reviewer mandate covers *functional* NTT bit-identity only, not timing on secret-dependent KEM arithmetic. P91 §8.2 checks decryption-**failure statistics** but never constant-time for ML-KEM decaps (FO re-encryption + implicit rejection + mod-q reductions are the classic timing-leak sites). P92 assumes constant-time AEAD tag comparison without asserting it as a review item. | **MEDIUM–HIGH** (P91), **LOW–MEDIUM** (P85/P92) | Add a "no secret-dependent branch/timing; constant-time tag/compare" line to the §3/§8 adversarial-review surfaces. Most material for P91 (decaps) — do not close P91 on functional KATs alone. |
| **G6** | **P92, P93, P94 — no shared discriminant/registry authority** | P92 allocates `FrameKind 0x04–0x07`; P93 allocates `SigningDomainVersion {0x01,0x02}` + TLV `FIELD 0x04–0x07` + `DOMAIN_*` strings; P94 uses `Resource/Action` discriminants for bit indices. Each self-defines its code points; **no single cross-blueprint registry** guarantees non-collision. Project memory records a prior **real "0x12 scope-discriminant collision between B1/B2"** — the identical bug class is structurally un-prevented here. (These currently live in *different* registries, so there is no live collision — this is preventive.) | **MEDIUM** | Add one pinned discriminant-registry doc as a prerequisite for the M1→P93→P92→P94 lane; every new frame-kind / signing-domain / TLV code point registers there. |
| **G7** | **P92, P93, P94 + M1 — shared hot-file single-writer schedule not centralized** | `signed_frame.rs` is edited by P92-M1 **and** P93; `hybrid_gate.rs` by P93 (relocate `seen`, change `check` signature) **+** P92 (caller) **+** P94 (verify_chain subset rewrite); `roster.rs` by P93 **+** P94. Pairwise sequencing is caught piecemeal (P93 §4.7 is the best), but **no blueprint states the full single-writer schedule** across all four units on these three files. Parallel companion: kernel-side, **P77/P79/P80 all append to `kernel/benches/criterion.rs`** despite "disjoint parallel lanes" claims (true for source files, false for the shared bench registry). | **MEDIUM** | One blueprint (or the ledger §3) should own the explicit write-order for `signed_frame.rs`/`hybrid_gate.rs`/`roster.rs` and note the shared `criterion.rs` append-point so parallel workers serialize on it. |
| **G8** | **P78 vs P82 — bench-harness ordering inversion** | P78's B3/B4 benches target bebop2 `proto-wire` + `delivery-domain`; verified: **neither crate has a criterion `[[bench]]` harness** (only `bebop2/core` does, via `verify_lane.rs`). P78 disclaims standing up the harness ("that's P82's scope") — but the ledger sequences **P78 → P82**, so P78's D-BENCH deliverable depends on infrastructure assigned to a *later* blueprint. | **MEDIUM** | Either move the proto-crate harness setup ahead of P78, or give P78 a minimal self-contained harness. Resolve the P78/P82 order explicitly. |
| **G9** | **P75 — core mechanism rests on unverified external-tool assumptions** | The whole gate rests on criterion 0.5's `--save-baseline`/`--baseline` A/B flags and the exact on-disk `target/criterion/<id>/change/estimates.json` layout. P75 §0.6 honestly admits these are "established knowledge, WebSearch budget exhausted — verify exact flag spelling before wiring." Honestly flagged, but it is the single point the entire gate depends on. | **MEDIUM** | Verify criterion 0.5 CLI + JSON-output format against the pinned tool version *before* building the gate; make the tool version a recorded input. |
| **G10** | **P76 — cross-repo CI checkout unspecified** | The A2 deliverable is a CI leg `cargo test -p bebop-delivery-domain --features kernel-rlib`, and that feature pulls `dowiz-kernel = { path = "../../../dowiz/kernel" }`. For it to *compile* in bebop's CI, the runner must check out the **dowiz repo at a sibling path** — a cross-repo multi-checkout topology P76 never specifies (it only notes the path link "can break" and that a name-list guard catches it). | **MEDIUM** | Specify the CI checkout topology (how bebop CI obtains the sibling dowiz kernel) as an explicit environmental prerequisite of the `kernel-rlib` leg. |
| **G11** | **P79 — alloc-count harness never committed + bit-identical-f64 FP risk** | Two RED tests (`red_causal_20k_allocs_bounded`, `red_spectral_no_kclone_reorder`) lean on an allocation-count harness; verified: `kernel/Cargo.toml:156` states `tests/arena_counting_allocs.rs`'s source **"was never committed"** and the `[[test]]` slot is commented out — P79 presents it as an available pattern, not a to-build prerequisite. Separately, `red_spectral_evecs_bit_identical` asserts **bit-identical f64** between pointer-chased `Vec<Vec<f64>>` rows and a contiguous `k·n` buffer; a contiguous layout can trigger different auto-vectorization/FMA-contraction/reassociation, so exact equality can false-RED on some targets even though the algorithm is unchanged. | **MEDIUM** | Have P79 author (or explicitly depend on) the alloc-count harness. Relax the spectral DoD from strict f64 bit-identity to the `UᵀU=I` 1e-9 KAT (already present) OR pin a `-ffp-contract`/target to make bit-identity portable. |
| **G12** | **P80, P82 — iai-callgrind lane assumed unconditional but is host-conditional in P75** | P80 §7 and P82 route sub-100ns crypto/hash benches to "P75's iai instruction-count lane" and say "do not gate on wall-clock." But P75 §4.5 (M5) makes the iai lane **explicitly deferrable** (if valgrind is unavailable on the runner, those benches become `gateable:false`, measured-not-gated). Consumers do not carry that conditional, so their ns-scale benches may silently end up ungated. | **LOW–MEDIUM** | Propagate P75's M5-conditional into P80/P82: state the fallback (measured-not-gated) for ns-scale benches if iai is unavailable. |
| **G13** | **P92 — D-BENCH NO-GO threshold left symbolic** | The measure-first D-BENCH gate is defined, but `FASTPATH_BENEFIT_THRESHOLD` is a symbolic "break-even N," **never a concrete number** (§10.3). The NO-GO decision is therefore not machine-decidable until the bench is run and someone picks a number — the gate can be argued either way post-hoc. | **LOW** | Pre-commit the threshold rule (e.g. "fast-path GO only if measured presence-frame volume ≥ N and per-frame saving ≥ X% at that N") before running D-BENCH. |
| **G14** | **P83 — metric emit-only; no consumer for `kernel_span` rows** | P83 §6 claims regressions "surface automatically, not at review time," but that holds only for Layer 2's `perf`/alert path. For Layer 1's per-function span trend, §7 promises only that a rising p99 is "queryable by fn" — **no automated consumer/alerter is wired** for `kind=kernel_span` rows (unlike `bench.jsonl`, which P75 revives a real rolling-window consumer for). The named consumer (self-improvement loop / `markov.rs`) is intended, not wired. | **LOW** | Either wire a minimal `kernel_span` p99-drift alerter into the existing `tools/ops-alert` pattern, or downgrade §6's "automatic" claim to "queryable" for Layer 1. |
| **G15** | **P96 — no live telemetry for the accuracy win; verification stops at port boundary** | P96's entire value prop is "more accurate ETA," yet there is **no live telemetry** proposed to confirm the accuracy improvement in the field, and **no per-order logging of which path fired** (adaptive vs static fallback). Verification stops at the kernel/port boundary — it *asserts* (does not E2E-verify) that the improved value reaches and renders unchanged in the customer/courier view. | **LOW** | Defensible for a fail-safe non-red-line change, but note the boundary. If it ships, add a lightweight adaptive-vs-fallback path counter + (optional) an offline field-replay accuracy check. |
| **G16** | **P95 — proves correctness-equivalence, only asserts the speedup** | P95's proptest suite is rigorous on *correctness equivalence* (byte-identity / rank-equivalence), but the O(corpus)→O(doc) speedup that is the entire reason to build it is **asserted, not benchmarked** — there is no perf-regression DoD. | **LOW** | Acceptable at HOLD status; if it moves to build, add a benchmark DoD confirming the incremental-update speedup. |
| **G17** | **P81, P82 — stale "P75 to be written" cross-reference** | P81 §9 and P82 §9 describe P75 as "to be written"; P75 is complete (same date). Cosmetic, but a worker may go looking for a nonexistent draft or re-derive the schema. | **LOW** | One-line reconciliation: point P81/P82 §9 at the finished P75 file. |

---

## 2. Coverage assessment against the eight dimensions the brief named

Honest per-dimension verdict across the whole P75–P96 set (plus the closed-no-action items).

| Dimension | Verdict | Detail |
|---|---|---|
| **Cross-platform** (deploy-target vs dev, ARM/x86, WASM vs native) | **Adequately covered where it applies.** | P75's central design *is* the cross-host problem (same-runner A/B cancels the host constant) — the best treatment in the set. P90 flags "all numbers single-machine" and requires an N∈{1,2,4,8} confirm before shipping a swap. P89 consciously avoids per-platform-libm FFT. Crypto (NTT/KEM/P94) is pure integer arithmetic → deterministic WASM-vs-native, a genuine non-gap. **One real gap: G11** (P79 bit-identical-f64 is target-sensitive). |
| **Cross-device** (mobile courier vs desktop, WebGPU vs WebGL2, browsers) | **The set's weakest dimension — G2.** | P38 establishes a canonical WebGPU→WebGL2→CPU ladder + a standing "WebGL2-floor DoD line" gate; **P86/P87/P88 omit that line** and P88's atomics policy is silent on the fallback rung. Also note (inherited from P38): feature detection is **coarse** (`navigator.gpu` present/absent) — there is **no fine-grained WebGPU feature/limit matrix** (`shader-f16`, atomics, storage-texture limits) anywhere, so P87's f16 presentation tier and P88's atomics have no per-device availability analysis. P89/P90 are CPU-only → correctly N/A. |
| **Testing coverage** | **Strong across the set.** | Nearly every blueprint carries named RED tests + falsifiable DoDs, several with explicit anti-cheat siblings (P81/P82 injected-slowdown proofs; P82 forged-frame `check`; P94 exhaustive 408-pair equivalence + golden-byte KAT; P89 DoD *is* the verdict table). Gaps are in *prerequisite infra* (G3/G4/G8/G11) and *constant-time* (G5), not in test design intent. |
| **Logging / telemetry** | **Covered by P83; correctly reuses, does not reinvent.** | P83 integrates with the existing `tools/telemetry` monitor loop, `log_event`, JSONL ledgers, and the `tg_deliver` alert seam rather than inventing a parallel system, and it handles the deploy-target-vs-dev question directly (system-wide `perf -a` to distinguish kernel vs `rustc` load). Residuals: **G14** (Layer-1 span rows emit-only), **G15** (P96 no field telemetry). |
| **Metrics** | **Covered; separation of concerns is clean.** | Offline/gated metrics (criterion baselines → `baseline.json`) are cleanly separated from live metrics (`metric.jsonl`). The one structural issue is **G3** (the gate that would consume engine/bebop baselines does not run those crates). |
| **Verification of local/connected agents or models** | **NON-GAP — correctly out of scope for this wave.** | Confirmed by grepping all 20 P75–P96 files: none invokes or gates on a model/agent whose correctness isn't verified. P75 *explicitly* separates deterministic benches from LLM probes ("LLM benches stay pass/fail probes, NOT gated"). The one model-in-loop surface (P40 agent loop + P21 local-LLM serving, incl. HK-05/HK-09 routing) is already owned by the **P54/P55/P56 verification trio** (dated 2026-07-18) — P54 verifies agent *behavior* with deterministic Rust oracles (no LLM judge), P21 verifies model *serving*. Today's set adds **no new unverified model surface**. This dimension is well-covered; do not manufacture a gap here. |
| **Code-review / quality checks** | **Covered as real, tracked DoD rows.** | The "independent adversarial review" gate is a genuine `D-REVIEW` row in every crypto/mesh blueprint (attestation filed, blueprint RED on FAIL) — not hand-waved. P85 restores an actually-existing `three-model-review.sh`. The quality gap is narrow and specific: **G5** (review mandates omit constant-time). |
| **Interface / UI checks** | **Mostly N/A for this set; the one real interface concern is G2.** | No P75–P96 item is a UI-surface blueprint. The rendering-adjacent ones (P86/P87/P88) fold their "interface check" into the WebGL2-floor gate (→ G2). P96 explicitly makes no UI change but *asserts* rather than E2E-verifies the render is unaffected (→ G15, LOW). |

---

## 3. Explicit non-gaps (recorded so they are not re-litigated as "missed")

These were checked and are **correct scope decisions**, not omissions:

- **Crypto blueprints (P85/P91/P92/P93/P94) have no cross-device story** — correct; they are
  platform-agnostic by nature. P94 in particular *proves* word-size/endianness independence
  (fixed `[u32;18]`, never serialized) — a deliberate non-gap.
- **Agent/model output verification absent from P75–P96** — correct; the surface that needs it is
  owned by the pre-existing P54/P55/P56 trio (see §2). No new unverified surface was introduced.
- **P89, P90 cross-platform** — clean (P89 CPU-only and not GPU-gated; P90 flags single-machine
  and gates the swap on a multi-hardware confirm).
- **The five CLOSED-NO-ACTION-NO-TARGET scans** (BitNet, QKD, fraud-scoring, bit-slicing,
  energy-currency) — each closed with a named reopening trigger; not re-scanned here.
- **P77, P83, P89, P94** are the cleanest individual blueprints — their deferrals are correctly
  labelled and their DoDs are self-sufficient.

---

## 4. Summary

Of 21 blueprints reviewed against eight coverage dimensions, the audit found **four HIGH findings**
and a tail of medium/low items — a good outcome for a wave this large, and consistent with the
day's dominant result that the core architecture survived adversarial scrutiny. The four that
warrant action before their workers start:

1. **G1 — P95/P96 are orphaned from the ledger** (0 references; written 13–14 min after it froze).
   Real roadmap items, invisible to the handoff artifact. Back-register both.
2. **G2 — the GPU blueprints (P86/P87/P88) drop P38's mandatory WebGL2-floor DoD line**, and P88's
   atomics-by-default policy never confronts that WebGL2 has no compute atomics at all. The one
   genuine cross-device blind spot in the set (mitigated only by the P38 §4.2 build gate).
3. **G3 — P81's engine gate and P82's bebop gate do not exist**: P75's *schema* is complete but its
   *running* CI job is kernel-only, so two blueprints' "the P75 gate goes RED" DoDs are
   unsatisfiable as written; P82's is cross-repo and breaks its continuous-gate value prop.
4. **G4 — P91's FIPS-203 conformance spine has no vectors**: the ML-KEM ACVP JSON and a KEM loader
   harness do not exist in-repo, and the vector provenance + port source are unpinned.

The remaining findings are prerequisite/coordination hygiene: a missing constant-time review line
across the crypto set (G5, most material for P91), no shared discriminant registry despite a prior
`0x12` collision (G6), an un-centralized single-writer schedule on three shared mesh files (G7), a
P78/P82 harness ordering inversion (G8), and per-blueprint items G9–G17. **Two dimensions the brief
specifically asked about — agent/model-output verification and crypto cross-device — are genuine
non-gaps** (§3): today's set introduces no unverified model surface (the P54/P55/P56 trio owns that
surface), and crypto is platform-agnostic by construction. No severity was manufactured; where a
blueprint was well-scoped it is recorded as such.

---

*Cross-references: `MASTER-STATUS-LEDGER-2026-07-19.md` (the ledger missing P95/P96) ·
`BLUEPRINT-P38-webgpu-render-engine.md` §3.6/§4.1/§4.2/§12.3 (the WebGL2-floor standing gate) ·
`BLUEPRINT-P54/P55/P56` (the agent/model-verification trio) · `BLUEPRINT-P75`…`P96` ·
`BLUEPRINT-P63-shell-platform-spike.md` §2 (the closest thing to a device matrix) · memory:
`crypto-safe-first-pass-2026-07-14.md` (the 0x12 collision + "0 dudect" precedents),
`performance-priority-over-minimal-change-2026-07-17.md`.*
