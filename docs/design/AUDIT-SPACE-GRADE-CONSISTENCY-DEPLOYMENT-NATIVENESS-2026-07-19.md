# Consistency-Enforcement Audit — Space-Grade Quality Standard, Deployment-Context Honesty, Max-Nativeness

**Date:** 2026-07-19 · **Role:** consistency audit (one of three parallel audits; this one covers the
deployment/quality/nativeness dimension only) · **Standing rule enforced:**
`/root/.claude/projects/-root-dowiz/memory/space-grade-quality-not-deployment-scoped-2026-07-19.md` —
(a) never reject/downscope hardening on "not literally a spacecraft / just cloud hardware" reasoning;
(b) the actual deployment target for this kernel arc is **local & offline-first consumer-grade
hardware, typically WITHOUT ECC** — never assume cloud/ECC server RAM; (c) max nativeness — reuse
existing in-kernel primitives (Keccak, CRC32, …) over new deps or new hand-rolled duplicates.

**Documents audited (full reads):** `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md`
(SYNTH), `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (ROADMAP, §A–§J, items 1–54 — nothing
beyond 54 exists), `DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md` (DAI),
`CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md` (CRASH),
`KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` (KLEENE). **Spot-checked** (deployment/
hardware-sensitive item docs): `BLUEPRINT-ITEM-27-pmu-classifier-input` (BP-27),
`BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite` (BP-04/29), `BLUEPRINT-ITEM-31-enactment-per-crate-gate`
(BP-31E), `AUDIT-ITEM-31-dependency-findings` (AI-31), `AUDIT-ITEM-26-batching-measurements`
(AI-26), `BLUEPRINT-ITEM-30-state-machine-audit`, `AUDIT-ITEM-30-state-machine-final`. Keyword
sweeps (cloud/ECC/Hetzner/datacenter/commodity/server/aspirational/overkill/…) ran across all
BLUEPRINT-/AUDIT-ITEM docs. RAW-PROMPT-\* captures are verbatim source records and are deliberately
NOT correction targets.

This audit produces findings only — no target document is rewritten here.

---

## Section 1 — Deployment-context instances (stated, assumed, or implied)

The known-fixed instance (Sentinel → item 54, operator-reversed) is confirmed correctly recorded in
KLEENE §2.3 and ROADMAP §J (lines 675–693, 783–812) — that is the model treatment. The instances
below are everything else found.

| # | file:section (line) | Instance | Class | Correction |
|---|---|---|---|---|
| 1.1 | SYNTH §6 (line 143) | "**On ECC-RAM Hetzner hosts the residual value is modest** — this is a defense-in-depth option for the operator to weigh, not a recommendation to blanket the kernel in it." | **WRONG** — the exact rejected pattern, uncaught. Both prongs: ECC-cloud premise is factually wrong (target = local, non-ECC consumer hardware), AND the wrong premise is used to demote the SIHFT proposal from recommendation to "option to weigh." This is the same sentence-shape the operator reversed for Sentinel, but in §6 it was never retro-corrected. | Replace with the corrected premise, mirroring ROADMAP §J's item-54 language: "On the actual target — local, offline-first, consumer-grade hardware that typically LACKS ECC — the in-memory SEU/bit-flip fault class is real, and software voting's residual value is material, not modest." Keep the (valid, physics-grounded) shared-silicon caveat at line 141 unchanged; the *narrow scoping to pure computations* survives on that genuine engineering ground — only the value-assessment sentence and the "not a recommendation" demotion are premise-poisoned. |
| 1.2 | SYNTH §9 item 12 (line 176) + ROADMAP §0 (line 22) + §E (line 320) | Item 12 framed "optional, operator's defense-in-depth call"; ruling rows carry no premise correction. | **WRONG (inherited)** — the "optional" grading flows from 1.1's stale valuation. The operator ruled PURSUE anyway, so no work was lost, but the recorded justification and the pilot's sizing ("one pure computation") were fixed under the wrong premise. | Annotate the item-12 ruling rows with the corrected deployment premise (the same annotation §J gave item 54); when the design pass starts, size the triple-vote registry under the non-ECC premise — per the standing rule's boundary logic, scope SIHFT to the computation surfaces items 40/54 (checksum-detection) do NOT cover (vote-and-mask vs detect-and-refuse are different responses). |
| 1.3 | SYNTH §11 T2 (line 208) | "This kernel targets **x86 Hetzner hosts**; no such tie has been measured." | **WRONG premise, conclusion stands** — deployment claim is wrong twice (not Hetzner; not x86-only — wasm32 is a documented target, BP-27 already plans aarch64 analogues). The T2 gate-closed verdict survives untouched on its independent ground (no quantum hardware anywhere, no measured classical tie). | Replace the sentence with: "This kernel targets local, offline-first consumer hardware (x86_64 + wasm32 today); no quantum hardware target exists and no classical tie has been measured." Verdict unchanged. |
| 1.4 | BP-27 §5 (lines 236–243; also line 113 "current production host") | "**Plain statement: on the actual deploy target**, every Tier-B hardware counter will read `{"unavailable":"permission_denied"}` today." The probed host is an AMD EPYC-Milan KVM guest (cloud VM). | **WRONG** — conflates the current dev/self-management box (a cloud KVM guest, `hypervisor` flag, empty powercap) with the arc's deploy target. On local consumer hardware the availability picture *inverts*: RAPL (`/sys/class/powercap/intel-rapl`) typically EXISTS on consumer Intel/AMD Linux boxes, and `perf_event_paranoid` is normally 2 (mainline default) not 4, so Tier-B is often reachable. The named-absence design itself is deployment-agnostic and survives unchanged — only the factual claims are wrong. | Rephrase to: "on the *current dev/self-management host* (a KVM guest) Tier-B reads permission_denied; on the actual deploy target — local consumer hardware — RAPL and PMU access are typically present, and the named-absence path covers hosts where they are not." Same fix for line 113's "production host." The §5 table's honest "probed live on this host" caption is correct — keep. |
| 1.5 | CRASH §2.1 (line 184) + §2.3(iii) (lines 244–246) + ROADMAP §I item 46 (lines 592–595) | "A **single deployed dowiz kernel binary** is NOT unpredictable to itself"; full fixed-point rewrite parked behind trigger "a **multi-ISA deployment requirement**." | **AMBIGUOUS** — the ruling itself is legitimate (see 2.3 below), but the framing leans single-host. The local-first premise implies a *fleet of heterogeneous consumer machines* whose peers replay each other's DecisionUnits (`import_unit`'s replay-before-persist), so cross-host/multi-ISA divergence is a nearer concern than the text implies. Scope (ii) (every cross-host comparison surface integer-domain or golden-covered) already protects the load-bearing surfaces, which is why this is ambiguous, not wrong. | Add one sentence to item 46 / CRASH §2.3: "the local-first mesh target means heterogeneous peer hardware; the multi-ISA reopening trigger must be evaluated against fleet heterogeneity (incl. aarch64 consumer devices), not against a single-host assumption — scope (ii)'s cross-host surfaces are the first line either way." |
| 1.6 | DAI Q2 / ROADMAP item 33 | separate-core rejected: `core_pinning.rs` DECART — "on a **single-socket host** there is no locality to exploit." | **CORRECT** — consumer local hardware is essentially always single-socket; the corrected premise *strengthens* this rejection. |
| 1.7 | ROADMAP §C item 27 correction (lines 291–301) | Root/`CAP_PERFMON` bypass observed on the dev host; unprivileged path proven via errno tests; host-knob flagged NEEDS-OPERATOR-DECISION. | **CORRECT** — explicitly dev-host-scoped, never generalized to the deploy target. |
| 1.8 | BP-04/29 + ROADMAP items 4+29 | RAPL "joules_uj reporting `unavailable:no_rapl_interface` (named absence) on **this RAPL-less host**"; field first-class, present-and-explicitly-empty. | **CORRECT** — deployment-agnostic schema; honest "this host" phrasing throughout. |
| 1.9 | SYNTH §5 tier (c) (line 133) | Reserved-RAM/ramoops "requires host cooperation… host-config-dependent, not pure userland"; tier (b) is the target. | **CORRECT** — honest. Note (opportunity, not defect): under the local premise the operator *controls* the hardware, so tier (c) is more attainable than the cloud framing implied; worth one line when tier (c) is next scoped. |
| 1.10 | SYNTH §20(a), §21 | Optical-compression pilot runs "through the local GGUF path (no network dependency)"; RAPL "host-dependent, not assumed universal." | **CORRECT** — consistent with offline-first. |
| 1.11 | KLEENE §2.3 + ROADMAP §J header & item 54 | The recorded operator reversal + corrected premise, applied in full. | **CORRECT** — the model treatment the other findings should copy. |
| 1.12 | BP-31E line 56 (`tools/telemetry/hetzner-exporter`) | Crate name in a dependency table. | **CORRECT** — ops-telemetry tooling for the operator's current box; not a kernel-arc deployment claim. |

**Section 1 count: 3 primary WRONG (1.1, 1.3, 1.4) + 1 inherited WRONG (1.2) + 1 AMBIGUOUS (1.5) + 7
CORRECT catalogued.** No other cloud/ECC/datacenter/server-RAM assumption found in the five main
docs or the swept item docs.

---

## Section 2 — Scope-downs / deferrals / rejections, re-examined for the rejected reasoning pattern

Every scope-down found across the four arcs was re-checked for "not really needed at our scale /
not literally a spacecraft / cloud-ECC covers it" reasoning.

| # | file:section | Scope-down | Verdict | Reasoning check |
|---|---|---|---|---|
| 2.1 | SYNTH §6 + §9.12 | SIHFT demoted to "optional… not a recommendation" | **VIOLATION** (the only uncaught instance) | Same root as finding 1.1 — deployment-context valuation ("ECC-RAM Hetzner hosts ⇒ residual value modest") used to demote a hardening proposal. Correction covered in 1.1/1.2. The *structural* narrowing (pure computations only, vote-mismatch = breaker trip not SEU-immunity claim) stays — that part is genuine engineering merit. |
| 2.2 | BP-31E §4.2/§4.4 | JSON parser Phase-A scoping; `native-spa-server`/`llm-adapters` cutover deferred | LEGITIMATE | Adversarial-boundary security honesty (serde_json's decade of fuzz load) + measured tree-win analysis (`cargo tree -i`: removal shrinks nothing there). Named reopening triggers. |
| 2.3 | CRASH §2.3 / ROADMAP item 46 | Kernel-wide f64→fixed rewrite REJECTED as disproportionate | LEGITIMATE (with the 1.5 framing amendment) | Evidence-based: one ledger row ever, transcendental-specific; rewrite risks regressions in proven oracle-covered code; containment (golden coverage + inventory) built instead; named reopening triggers. No deployment-context demotion. |
| 2.4 | ROADMAP item 26 / AI-26 | Group-commit operator-gated not default; M2 KEEP-AS-IS; M3 don't-batch | LEGITIMATE | Measured (strace, p50/p99); crash-contract change named as the gating reason; scope law held (no code landed). |
| 2.5 | CRASH §5 | Coq/Lean/proof-carrying-code/verified-compilers OUT OF SCOPE | LEGITIMATE | Cost/benefit vs runtime-verification already operational; explicitly notes static proof cannot address the bit-flip class (consistent with the corrected premise); named reopening trigger (DO-178C-class demand). |
| 2.6 | KLEENE §2.5 / item 53 | lint-gate LOW priority, sequenced last | LEGITIMATE | The contribution surface does not exist and is operator-gated (ADR-0020); named escalation trigger promotes it to pre-flip blocker. |
| 2.7 | KLEENE §2.2 / item 52 | Miri scoped to 19-block/4-module real unsafe surface, not miri-everything | LEGITIMATE | Corrected inventory; filtering unsafe-free modules "matches zero unsafe — theater"; SIMD/FFI limits stated honestly. |
| 2.8 | ROADMAP item 49 / CRASH §1.1 | Hybrid/LSM snapshot parked | LEGITIMATE | FDR replay bounded by construction (2×1 MiB); EventLog store not even wired yet (item 2 defect) — measure-first, named trigger. |
| 2.9 | CRASH §1.2 / item 48 | Register/stack core dumps not pursued; restart authority external | LEGITIMATE | KISS/`Kernel_Init`-over-`Kernel_Recover`; platform-shape is factual (std userspace on Linux), not a cloud assumption; systemd watchdog works identically on local hosts. |
| 2.10 | ROADMAP item 43 / DAI §3 | Constant-time gate cheap-but-optional for the toy pilot | LEGITIMATE | Input-plane classification (public/synthetic by construction); mandatory dudect branch has a named reopening trigger (any secret-adjacent consumer). |
| 2.11 | ROADMAP item 34 / DAI §3 | Real-product pilot surfaces DEFERRED behind the toy pilot | LEGITIMATE | Risk sequencing (zero product data/PII until the pipeline is proven), operator-ruled. |
| 2.12 | DAI Q1/Q3/§3 | TVM/Burn rejected; LLM-class permanently out; GPU out; training out | LEGITIMATE | Zero-dep law + auditability + *measured* P-F physics (memory-bandwidth-bound) + P6 determinism — none is deployment-context reasoning. |
| 2.13 | CRASH §4 | WCET tooling out of scope; no_std/#[panic_handler] not applicable | LEGITIMATE | Factual platform shape (preemptive scheduler + page cache); bounded-loop clause adopted instead. |
| 2.14 | SYNTH §11 T2/T3 | Quantum algorithms gated closed; quantum vocabulary rejected | LEGITIMATE verdict, WRONG premise sentence (finding 1.3) | Gate-closed stands: no quantum hardware exists on ANY plausible substrate. |
| 2.15 | SYNTH §20(c) / item 28 | Optical compression restricted to archival/display plane | LEGITIMATE | Lossy-vs-determinism-plane boundary; "97% fidelity on a signature is 100% failure." |
| 2.16 | CRASH §3 / item 45 | No runtime kill-switch service / dual-binary pipeline / AI-health monitor | LEGITIMATE | Over-design guard; the feature gate + CI job + Guardian `None` path each independently testable. |
| 2.17 | SYNTH §3 | meshNetwork not relied on; Hermit not adopted; MIRAI/seL4/Lean not recommended | LEGITIMATE | Evidence-based (dormancy, unverified claims, security thesis, archived project, no leverage). |
| 2.18 | AUDIT/BP-ITEM-30 | `hub_supervisor`: "graph machinery arguably overkill… parity pin, not collapse" | LEGITIMATE | Structural-fit reasoning (linear typed pipeline vs graph kit), not scale/deployment reasoning. |
| 2.19 | ROADMAP items 40/47/54 | Breaker composition deferred until item 9 exists | LEGITIMATE | Pure sequencing; each names the composition so nothing is re-derived; design does not gate. |

**Section 2 count: 19 scope-downs re-examined → 1 violation (2.1, same root as 1.1 — counted once
in the totals) + 18 legitimate proportionality calls cleared.** The known-fixed Sentinel reversal
is the 20th and is already correct.

---

## Section 3 — New dependencies / hand-rolled duplicates of existing kernel primitives

| # | file:section | Surface | Verdict | Notes / correction |
|---|---|---|---|---|
| 3.1 | AI-31 (line 67) / ROADMAP §A item 31 (lines 110–113) | **Dual in-kernel Keccak-f[1600]** (`event_log.rs:67` vs `pq/keccak.rs:156`) | **ALREADY-TICKETED — confirmed** | Dedup ticket owed to item 25, filed not fixed. Consistent with the standing rule; nothing further needed beyond executing the ticket. |
| 3.2 | ROADMAP §C item 31 (lines 246–250) + BP-31E §4.1 | **`kernel::json` (parser + serializer) built SEPARATE from `fdr::json` (serialize-only)** — the kernel now carries two JSON-write/string-escaping surfaces | **UNFLAGGED-DUPLICATION CANDIDATE** (minor) | SYNTH §10/P2 named this exact shape the failure mode ("a second escaping primitive, the exact P2 failure"). BP-31E acknowledges it only parenthetically ("whether `fdr::json`'s writer later moves under `kernel::json` is cosmetic; do not churn") and adds a round-trip test — but unlike the dual-Keccak case, **no dedup-or-parity ticket was recorded**. Correction: file the ticket in the 3.1 format — either consolidate `fdr::json`'s writer under `kernel::json::write`, or record a permanent escaper parity pin + a one-escaper-implementation rule. Verify escaper sharing in the exec-branch code when it merges. |
| 3.3 | ROADMAP §J item 51 / KLEENE §2.4 | ShadowDivergence carries "short **digests** of D and the proposed action" — digest primitive unnamed | **SPEC GAP** (low) | Max-nativeness requires naming the reused primitive *in the spec*, not at build time. Correction: one line in item 51 — "digest = the in-kernel CRC32 (hardware-fault plane, matches items 40/54) or truncated in-kernel SHA3; never a new algorithm." Prevents a third ad-hoc hash appearing under deadline. |
| 3.4 | BP-31E §4.1 | `serde_json` added as **dev-dependency** differential oracle | JUSTIFIED | Dev-deps sit outside the `-e no-dev` proof surface; allowlist stays empty; correct use of the oracle discipline. |
| 3.5 | BP-27 §3 | Hand-rolled raw `perf_event_open(2)` syscall (~150–200 LOC, `asm!`) | JUSTIFIED | No std/glibc binding exists; zero-dep forces the hand-roll; `_rdpmc` alternative correctly rejected (SIGSEGV vs named-absence doctrine); every errno → named `Absence`. |
| 3.6 | KLEENE §2.2 / item 52 | Miri nightly component (separate analysis toolchain) | JUSTIFIED | CI-time analysis, never builds shipped artifacts; own recorded pin; item-14's letter preserved. |
| 3.7 | ROADMAP item 37 / DAI §2 | New scalar integer-domain matmul reference oracle (vs existing `mat.rs` f64 matmul) | JUSTIFIED | Different numeric domain; test-only retained-forever oracle — the §4 checklist's own explicitly-permitted duplicate class; DAI names the mat.rs-shape reuse. |
| 3.8 | ROADMAP item 41 / DAI Q4 | `#[repr(align(64))]` (first in-repo use), SHA3 init self-check, ML-DSA codesign | JUSTIFIED / exemplary | New surface honestly flagged as new; hash + signing both REUSE existing in-kernel primitives (`event_log` Keccak, `pq/codesign.rs`); objcopy parked with named trigger. |
| 3.9 | ROADMAP item 6 | `ct_gate.rs` — new `ct_eq` constant-time primitive + Welch-t harness | JUSTIFIED | No prior CT-eq existed; the pq `kem.rs`/`hybrid.rs` variable-time tag compares (KNOWN-RED P91.2) are its intended first customer — one primitive, converging not forking. |
| 3.10 | ROADMAP item 36 | eqc indexed-summation IR — "one extension, two consumers, never two IRs" | JUSTIFIED / exemplary anti-duplication. |
| 3.11 | AI-31 §(c) | `sha2` KEPT in `native-spa-server` (vs kernel-Keccak swap) | JUSTIFIED | Independent re-verification corrected the blueprint: the swap would *pull* `aes-gcm`+`curve25519-dalek` (net dep increase) since `pq` is feature-gated; boundary-format + WebCrypto interop reasoning; hand-rolling SHA-256 barred by the "no new crypto" discipline. Genuine merit, not deployment reasoning. |
| 3.12 | ROADMAP item 5 | Kernel-owned restricted pattern matcher replacing `regex` | JUSTIFIED | Removal (not addition); differential-proven before cutover; permanent naive reference oracle. |
| 3.13 | ROADMAP item 54 / KLEENE §2.3 | Sentinel reuses the FDR CRC32 | JUSTIFIED / exemplary — the max-nativeness law applied verbatim ("no second CRC, no new algorithm, no external crate"). |

**Section 3 count: 13 surfaces examined → 0 unjustified new external dependencies anywhere in items
33–54; 1 previously-flagged duplication confirmed ticketed (3.1); 1 new unflagged duplication
candidate (3.2 — ticket owed); 1 spec gap (3.3 — one-line fix); 10 justified/exemplary.**

---

## Summary

| Check | Real violations | False-positives checked & cleared |
|---|---|---|
| 1. Deployment-context honesty | **4** — SYNTH §6:143 (ECC-Hetzner SIHFT valuation, the headline), SYNTH §11:208 (x86-Hetzner target claim), BP-27 §5:236+113 (dev-host conflated with deploy target), ROADMAP item-12 rows (inherited stale valuation); plus **1 ambiguous** (CRASH/item-46 single-binary framing vs local-first fleet) | 7 correct instances catalogued |
| 2. Scope-down reasoning | **1** — SIHFT §6 demotion (same root as check-1 headline; counted once overall) | 18 legitimate proportionality calls cleared (JSON Phase-A, batching gating, float-rewrite rejection, Coq/Lean OOS, Miri scoping, lint-gate priority, Hybrid park, CT-gate branch, pilot deferral, TVM/LLM/GPU rejections, WCET, quantum T2/T3, optical plane, item-45 not-built list, framework rejections, hub_supervisor pin, breaker sequencing) |
| 3. Max-nativeness / zero-dep | **2 minor** — kernel::json second write/escape surface (ticket owed, dual-Keccak format), item-51 digest primitive unnamed | 10 justified/exemplary + dual-Keccak confirmed already-ticketed |

**Net: 5 distinct real findings needing correction (4 deployment-context + wait — the SIHFT
valuation and its inherited rows are one root with two sites; strictly: 3 primary wrong statements,
1 inherited annotation gap, 1 ambiguous framing, 2 minor nativeness gaps). Zero uncaught instances
of the rejected "not really space" scope-down pattern beyond the SIHFT §6 sentence. Zero
unjustified new dependencies in the newer arcs (33–54) — the max-nativeness law is being followed
with two small spec-level exceptions.** All corrections above are cite-able for a follow-up
editing pass; none was applied to the target documents by this audit.
