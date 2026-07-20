# BLUEPRINT — Item 44: Arc-Wide CI Integration + Retroactive Checklist Pass (final)

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — CI is where the
  ruling stops being aspiration: every safety artifact of items 33–43 is **re-executed** (never
  presence-checked), and the arc lands with the zero-dep allowlist still empty.
- **Sources read this session:** roadmap §H item 44 (lines 606–613) + the §H header standing law
  (lines 489–493: zero new external crates, §4 checklist via item 6's machinery, item-25 procedure);
  CHECKLIST.md (the standing law + the **§10/P7 "re-execute, never presence-check"** correction, with
  `min_tests` floors and the proven RED path); `docs/audits/hardening/HOT-PATHS.tsv` (`@ZONE` +
  data-row grammar, `mode=lib/dudect/kani`, `min_tests`, `checklist`, `gap` columns); item 6's
  `hardening-gate` job (`ci.yml`, per item-07 blueprint §0 at `ci.yml:488`); the bench-gate
  (`baseline.json`, `bench_track.py`); item 29's FDR schema (per-inference **cycles + joules** where
  RAPL exists); synthesis §21 rule (a **token-count-only** cost report **fails review**).
- **Dependency gate:** **after items 40 + 42** (final item of the arc).

---

## 1. Scope / goal + non-goals

**Goal.** Join the inference hot paths to item 6's designated-hot-path list; make the §4 CI job
**re-execute** the item-37 oracle corpus, the item-40 planted-fault checksum test, and (if gated) the
item-43 dudect self-test; add item-39 benches to the bench-gate baseline; carry per-inference
**cycles + joules** in the FDR per item 29's schema; and prove the whole arc lands with the zero-dep
allowlist still empty.

**Non-goals.** NOT a parallel checklist — item 6's machinery is reused unchanged (§H standing law).
NOT a new CI framework — new `HOT-PATHS.tsv` rows + the existing `hardening-gate`/`kani-gate`/bench
jobs. NOT a token-count cost report — this is an inference engine measured in cycles/joules, not an
LLM measured in tokens (§21; "tokens" is the wrong unit and the arc's non-goal).

## 2. Grounding

- **The gate machinery already exists and is P7-correct.** `hardening-gate.sh` re-executes named-
  filter tests and asserts `N passed >= min_tests` (a filter matching zero tests is RED — the
  anti-forgery core, CHECKLIST.md §10/P7). New algorithmic code that touches a `@ZONE` with **no
  manifest row fails CI** — so the inference paths *must* register.
- **The bench-gate exists** (`baseline.json` + `bench_track.py`, criterion A/B) — item 39's bench
  joins it (already specified in item 39).
- **The FDR schema carries cycles/joules** (item 29, RAPL-aware; RAPL-less hosts show a *named*
  absence, not silent omission — the item-4/29 discipline).

## 3. Implementation plan

1. **Register the inference hot paths in `HOT-PATHS.tsv`:** an `@ZONE` line for the inference
   module(s) (e.g. `kernel/src/inference/`), plus data rows for the item-39 SIMD kernel, the item-40
   checksum oracle, and the item-42 scheduler — each with a single-token `filter`, a `min_tests`
   floor, `mode=lib` (or `dudect` for a gated item-43 row), the `checklist` column (which of 1–5 are
   present), and a `gap` note.
2. **`hardening-gate` re-executes** (live process exit + parsed counts, `--locked --offline`): the
   item-37 oracle corpus, the item-40 planted-fault checksum test, and (if item 43 is gated) the
   dudect self-test. **Never** presence-checks.
3. **Benches join the baseline** — item 39's bench in `baseline.json`, guarded by the bench-gate.
4. **FDR cost telemetry per item 29:** per-inference records carry **cycles + joules** (RAPL where
   present; a named `unavailable:no_rapl_interface` absence otherwise). A **token-count-only cost
   report fails review** (§21).
5. **RED-path demonstration** (P7 one level up): a deliberately **artifact-less** test diff touching
   an inference hot path (new algorithmic code, no manifest row / no oracle) **fails CI** — recorded
   in the PR (the item-6 §2.4 precedent).
6. **Zero-dep verified last:** `cargo tree -e no-dev` resolves to `dowiz-kernel` root alone — the arc
   lands with the allowlist **still empty**.

## 4. Required proofs (5-point checklist mapping)

Item 44 is the **retroactive whole-arc checklist pass** — it wires all five items into CI:
- **1 (oracle):** the item-37 corpus is re-executed in the job.
- **2 (dudect):** the item-43 self-test is re-executed **iff** the pilot is gated (public-plane toy
  pilot ⇒ the row records the cheap-branch ruling + reopening trigger, not a running dudect gate).
- **3 (differential):** item-39's `debug_assert_eq!`-vs-oracle runs in the debug test pass.
- **4 (asm):** item-42's dispatch assembly spot-check is filed under item 14's toolchain-keyed audit,
  re-triggered on compiler bumps.
- **5 (kani/native-structural):** the structural properties (cyclomatic-1, const offsets) are
  re-executed as native tests (no Kani toolchain needed for these — item-7 rescope logic).
- **P7:** every verdict is a live exit code + parsed count; the artifact-less-diff RED path is proven.

## 5. Falsifiable acceptance criteria

1. A deliberately **artifact-less test diff touching an inference hot path FAILS CI** (RED-path
   proven, recorded in the PR) — the anti-forgery core extended to the arc.
2. The **full inference suite is green**, with the oracle corpus, the planted-fault checksum test,
   and (if gated) the dudect self-test all **RE-EXECUTED** (live counts ≥ `min_tests`), never
   presence-checked.
3. Item-39 benches are in `baseline.json` and guarded by the bench-gate.
4. FDR per-inference records carry **cycles + joules** (or a *named* RAPL absence) per item 29; a
   token-count-only cost report is **rejected** (§21). **RED→GREEN:** a cost report that emits only a
   token count fails review.
5. `cargo tree -e no-dev` resolves to `dowiz-kernel` **root alone** — **ZERO external crates**; the
   arc lands with the allowlist still empty.

## 6. Dependency gate + operator-decision-needed

- **Gate:** after items 40 + 42 (final).
- **Operator-decision-needed:** **none new.** One **composition FLAGGED** (a different agent's range,
  §I item 45): the inference subsystem should land **behind a non-default `inference` cargo feature**
  (the `pq`/`slot-arena`/`ct-gate` surface-control pattern). If item 45's `ai-optional-gate` is in
  play, item 44's CI job MUST run **both** (a) **default-features** (AI absent) with the FULL kernel
  suite green — proving the kernel stays non-AI and AI-optional — and (b) **`--features inference`**
  for the arc's own suite. This composition is **named, not built here** (item 45 is §I scope); item
  44 wires the two-mode job when item 45's feature exists.
