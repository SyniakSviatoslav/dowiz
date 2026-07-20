# BLUEPRINT — Item 27 (response half): acting on a PMU-informed classification, via the bounded-control path

- **Date:** 2026-07-19 · **Tier:** 4 (roadmap §E) · **Status:** BLUEPRINT (planning artifact, no code)
  — **after item 21** (roadmap §E item 27: "after item 21"; item-27 classifier-input blueprint scope
  guard: "The response half … is Tier 4, strictly behind item 9 (breaker) and item 21 (autonomic
  gain-scheduling)").
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §E item 27
  (line 398), §C item 27 classifier-input half (lines 337–364, DONE `03887462a`);
  `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §18(c) (line 332, "any autonomic response
  … goes through §16's same bounded-control-law mechanism"), §9 addendum item 27 (line 342);
  **`BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md` (the landed classifier-input companion,
  read)**; live source `kernel/src/fdr/pmu.rs`, `kernel/src/markov.rs`, `kernel/src/spectral.rs`.
- **Relationship to items 9 & 21:** the response half does not build a new response mechanism — it
  routes PMU-informed classifications through item 21's bounded-control-law path and item 9's breaker.
  It is purely the *wiring* that lets a hardware-counter-informed verdict *act*, within the exact same
  determinism/stability constraints.

---

## 1. Scope / goal (one paragraph)

Wire the **response** to a PMU-informed classification — the second half of item 27, deliberately held
back from the classifier-input half (which landed diagnostic-grade, changing no behavior). PMU
counters (cache-miss/IPC/branch-mispredict) already ride alongside every `Verdict`/`DriftClass`
emission as an FDR companion (classifier-input half, DONE `03887462a`, `kernel/src/fdr/pmu.rs`). This
half lets a classification that was *informed by* those counters **act** — but under an absolute
constraint (synthesis §18(c)): "any autonomic response that reacts to a hardware-counter-informed
classification goes through §16's same bounded-control-law mechanism — pre-proven stability bounds,
FDR-logged adjustments, deterministic replay — **not** a separate CPU- or GPU-specific fast path."
So the response half builds **no new response subsystem**; it makes PMU counters a legitimate *input
feature* to item 21's control-law table, and routes the extreme end through item 9's breaker. The one
new thing it must prove: a PMU-informed verdict's response is byte-identical on replay of the same
counter sequence (P6 determinism), even though the counters themselves are host-variable — which
requires the counters to enter the *classification*, never the *control law's arithmetic*, directly.

---

## 2. Verified current state — grounded

- **The classifier-input half is DONE and diagnostic-grade.** `kernel/src/fdr/pmu.rs` (`03887462a`):
  `PmuStamp` (all `Reading<u64>`), Tier-A (rdtsc + minflt/majflt/ctxt-switches, zero-permission) +
  Tier-B (instructions/cycles/cache-misses/branch-misses via hand-rolled `perf_event_open` raw
  syscall, every failure a named `Absence`). Wired via `PmuStation::bracket`; the `markov_attractor`
  bin logs ONE `markov_verdict` FDR record carrying `verdict_str()` + the PMU delta on the SAME record
  (roadmap §C lines 337–349). **`analyze_detailed`/`classify_drift` stay pure (P6 preserved)** — the
  classifier-input half added PMU as a *recorded companion*, not a classifier input yet. **No CI gate
  is keyed to any PMU value** (roadmap §C: "diagnostic-grade; NO CI gate keyed to any PMU value").
- **The classifiers are pure and the response target (item 21) does not exist yet.** `markov::Verdict`
  (`markov.rs:42`), `spectral::DriftClass` (`spectral.rs:674`) — the classification the response acts
  on. Item 21's bounded-control-law path (`autonomic.rs`, the `BoundedRate` + `LAW_TABLE`) is where
  the response *goes* — it must be built first.
- **The breaker (extreme-end route) does not exist yet** — item 9. A PMU-informed classification at
  the extreme end (e.g. sustained cache-miss storm + `StrangeAttractor`) routes through the breaker,
  not unilaterally.
- **The determinism constraint is the crux.** The classifier-input half kept `analyze_detailed`/
  `classify_drift` pure specifically so PMU noise never enters the deterministic decision. The
  response half must preserve this: PMU counters may inform *which class* the system is in (a
  classification-layer decision, where they already ride), but the control-law *arithmetic* that
  computes the bounded adjustment must be a function of the **class**, not the raw counter — otherwise
  a replay of the same counter sequence would not reproduce the same adjustment on a different host
  (the P6 violation §18(c) forbids).

---

## 3. Implementation plan — exact wiring (no new subsystem)

1. **`kernel/src/markov.rs` / `spectral.rs` — PMU as a classification input (guarded).** The
   classifier-input half kept PMU a *companion*; the response half optionally lets PMU-derived signals
   *contribute to the classification* (synthesis §18(c): "a new signal class into the one classifier
   the kernel already owns — never a separate, parallel monitoring system"). **Constraint:** if PMU
   contributes to the class, it enters via a *quantized/classified* form (e.g. "cache-miss rate above
   a fitted threshold → contributes to the `Resonant`/`Unstable` decision"), never as a raw float in
   the verdict arithmetic — so the verdict stays a deterministic function of *classified* inputs. If
   this cannot be done without breaking P6-purity, PMU stays a companion-only signal and the response
   acts on the non-PMU verdict (fail-safe: the response half degrades to item-21-as-is).
2. **`kernel/src/autonomic.rs` (item 21) — the response path, reused.** A PMU-informed `(DriftClass,
   Verdict)` flows through item 21's `schedule()` → `BoundedRate` adjustment → FDR event, **the exact
   same path** a non-PMU classification uses. No PMU-specific control law, no CPU/GPU fast path.
3. **`kernel/src/breaker/` (item 9) — the extreme-end route.** A PMU-informed extreme classification
   routes through the breaker's `tick`/trip path (like item 21's extreme responses), not a unilateral
   PMU-triggered action. Demonstrated: a PMU-informed `Unstable` + `StrangeAttractor` → breaker path.
4. **No new CI gate keyed to PMU values** (synthesis §9 item 27: "the write-up labels PMU signals
   diagnostic-grade … and no CI job is keyed to them"). The response half preserves this — the PMU
   signal is diagnostic-grade even when it informs a response; the *response's determinism* is gated
   (P6), the *PMU value* is not.

Zero new dependency (`fdr/pmu.rs` is zero-dep; the response reuses item 21's path).

---

## 4. Tests / proofs — 5-point hardening applicability

The response is control-law wiring (item 21's path); the 5-point standard:

- **Item 1 (oracle):** **YES** — the PMU-informed classification → adjustment mapping is exhaustively
  enumerable over `{DriftClass × Verdict × PMU-quantized-band}` (item 21's 9-combo space × a small
  fixed set of PMU bands). A `#[test]` sweeps it, asserting the adjustment is item-21's table value.
- **Item 5 (formal / falsifiability — the headline P6 proof):** **a replayed counter sequence
  reproduces the identical verdict sequence** (synthesis §9 item 27) — feed a recorded PMU-counter
  trace + state sequence, assert the *response* (adjustment + FDR events) is byte-identical to a second
  replay. **And** the negative: a raw PMU float leaking into the control-law arithmetic would break
  this — a test that deliberately routes a raw counter into the adjustment fails the replay-equality,
  proving the P6-purity guard works (the item-27-response analog of the planted-fault self-test).
- **Item 3 (debug-differential):** `debug_assert!` the response adjustment equals item-21's
  class-derived adjustment (cross-check that PMU did not alter the arithmetic, only the class).
- **Item 2 (dudect):** **N/A** — no secret-dependent timing (PMU counters and drift class are not
  secrets). Record `N/A(no-secret-compare)`.
- **Item 4 (asm):** **N/A** — no branch-free constant-time path.

---

## 5. Acceptance criteria (falsifiable) — synthesis §9 item 27

1. **Grep shows one classification mechanism** — no parallel PMU monitor; PMU enters the *existing*
   `Verdict`/`DriftClass` pipeline (already true from the classifier-input half; preserved).
2. **A replayed counter sequence reproduces the identical verdict/response sequence** (P6 determinism)
   — the headline proof; byte-identical adjustment + FDR events across replays.
3. **Any autonomic response to a PMU-informed verdict demonstrably routes through item 21's
   bounded-control-law path** (and the extreme end through item 9's breaker) — no PMU-specific fast
   path (grep + a routed-response test).
4. **The write-up labels PMU signals diagnostic-grade** with §4's weakest-precedented caveat quoted,
   and **no CI job is keyed to a PMU value** (preserved from the classifier-input half).
5. **A token-only cost report demonstrably fails the §4 checklist's review** (synthesis §9 item 27 /
   §21) — the energy/hardware-first telemetry rule; any response report includes cycles/joules per
   operation alongside token counts (the `hw` field is already first-class from items 4+29).
6. Zero new dependency; `cargo tree` unchanged.

---

## 6. Dependency gates

- **After item 21** (roadmap §E item 27: "after item 21") — the response *goes through* item 21's
  bounded-control-law path, which must exist. Hard gate.
- **Behind item 9** (the extreme-end route is the breaker's trip path) — item-27 classifier-input
  blueprint scope guard: "strictly behind item 9 (breaker) and item 21."
- **Builds on the classifier-input half (DONE, `03887462a`, `fdr/pmu.rs`)** — the PMU companion data
  already flows; this half lets it *act*.
- **Blocks:** nothing. Leaf item.

---

## 7. Open questions (operator ruling)

1. **Whether PMU counters should contribute to the *classification* at all, or stay companion-only.**
   The classifier-input half deliberately kept `analyze_detailed`/`classify_drift` pure. Letting PMU
   *inform the class* (§3.1) risks the P6-purity the half protected — if a P6-safe quantized entry
   cannot be designed, the response half degrades to acting on the non-PMU verdict. Whether to attempt
   the PMU-into-classification path (richer, riskier) or keep PMU companion-only and act on the
   existing verdict (safer, simpler — the ponytail default) is an **executor judgment guided by the P6
   test**, not an operator gate — but flagged because it is the design fork that determines the item's
   value. Recommendation: attempt quantized entry; fall back to companion-only if the replay-equality
   proof (§4) cannot hold.
2. **The weakest-precedented caveat is binding.** Synthesis §18(c)/§4: PMU verification is "more
   studied as an attack vector than deployed as routine CI." The response half must not overclaim — no
   CI gate on PMU values, diagnostic-grade labeling preserved. This is a stated constraint, not an
   open question, recorded so the response half is not mistaken for a license to gate on PMU.
