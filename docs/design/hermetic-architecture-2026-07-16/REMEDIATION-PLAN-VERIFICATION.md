# REMEDIATION-PLAN VERIFICATION — decorrelated re-check of HERMETIC-REMEDIATION-PLAN.md §6

> Independent read-only audit (2026-07-16). Verifier did **not** write the plan or its blueprints.
> Goal: refute, against live ground truth, the seven things the plan's own §6 doubt audit flags as
> "asserted, not re-verified live." Each of the 5 requested checks below carries command output /
> file:line evidence, then a verdict. No code or plan file was edited by this document.

---

## Check 1 — §6.1 `kernel/src/wasm.rs` H2 ∥ P4 collision — **CONFIRMED (disjoint; plan's downgrade is correct/conservative)**

**H2's planned wasm.rs edit sites** (BLUEPRINT-H2 §1/§2.4/§2.2, re-grepped against live `kernel/src/wasm.rs`, 1139 lines):
- `wasm.rs:29` — `use std::collections::HashMap;` → `BTreeMap` (Site 4).
- `wasm.rs:105-109` — `struct LedgerOut { … funnel: HashMap<…> }` → `BTreeMap` (Site 4). *(grep: struct at :105, funnel field at :109.)*
- `wasm.rs:239` — `let mut funnel: HashMap<…> = HashMap::new();` → `BTreeMap` (Site 4).
- `wasm.rs:749-751` — `DriftClass::Damped => 0.0 / Resonant => 1.0 / Unstable => 2.0` inside `spectral_flat_logic` (fn at :735) → `.wire_code()` (Site 2). *(Audit cited :748-751; live is :749-751, off-by-one, negligible.)*
- `wasm.rs:~1122` — the `spectral_flat_logic` pin test.

**P04's planned wasm.rs additions** (BLUEPRINT-P04 §5): NEW `route_js`, `mst_js`, and six `geo_*_js` wrapper pairs, explicitly "follow the established `geo_*_js` pattern (`wasm.rs:470-607`)." The live `geo_*_js` block runs `:470` (`geo_haversine_logic`) through `:582+` (`geo_point_in_polygon_logic`) — a contiguous `_logic`+`#[wasm_bindgen] *_js` region. P04 appends new pairs of that same shape, plus (implicitly) new `use crate::router/dsu` imports near the top and their `#[wasm_bindgen]` registrations.

**Diff-level verdict:** the two edit sets touch **genuinely disjoint code bodies**. H2 edits the `LedgerOut`/`funnel` struct region (`:29`, `:105-109`, `:239`, all *above* line 470) and the `spectral_flat_logic` match arms (`:749-751`, *below* the geo block). P04 inserts new function pairs *inside/after* the geo `:470-607` block. These live in separate hunks with untouched context between them; git 3-way merge handles them without conflict. The **only** shared surface is the top-of-file `use` block near `:29` — H2 mutates the existing `HashMap` import line while P04 would add *new* `use` lines; different lines, mechanically trivial to reconcile, not a semantic collision. The plan's own handling ("Different regions, low merge risk, but it is one hot file; … sequence the two `wasm.rs` edits within one lane rather than claiming clean parallelism", §3 caveat (a)) is **accurate and, if anything, conservative**. No correction needed.

---

## Check 2 — §6.2 `hydra.rs` quiescence for H1 — **CONFIRMED (settled; clean tree, all G9 landed)**

```
$ git log --oneline -10 -- kernel/src/hydra.rs
d0e71cec9 feat(kernel): make BreachAlert public + wire ser/de (40-byte fixed layout)
b5b583e49 feat(hydra): G9 hub convergence + replay-safe breach witness
fab17275a feat(hydra): G9 receiver-verifiable breach alert (forge-proof, no-trust)
5403a3eff feat(hydra): G9 self-witness — breach persists into WORM log (anti-silent-heal)
1701eabd1 feat(hydra): breach-alarm broadcast (G9) — unbounded, no-per-event-consent
3e5f805e3 feat(hydra): G9 defensive anti-tamper — Live/Locked + consent-gated replicate
82e52c02e feat(hydra): Воля АНУ — self-evolving closed-loop organism (G3-G8)

$ git status --short
?? docs/design/hermetic-architecture-2026-07-16/BLUEPRINT-H1-event-log-fail-open-fix.md
?? docs/design/hermetic-architecture-2026-07-16/BLUEPRINT-H2-mirror-pin-sweep.md
?? docs/design/hermetic-architecture-2026-07-16/BLUEPRINT-H3-breach-detection-independent-probe.md
?? docs/design/hermetic-architecture-2026-07-16/BLUEPRINT-H4-self-governance-ritual-enforcement.md
?? docs/design/hermetic-architecture-2026-07-16/HERMETIC-REMEDIATION-PLAN.md

$ git status --short -- kernel/src/hydra.rs
(empty)
```

**Verdict:** the working tree is clean except for the five untracked Hermetic docs; `hydra.rs` has **zero** uncommitted modifications. Every G9 breach-witness commit (`1701eabd1`→`d0e71cec9`) is landed in history, not in flight. H1's "step 0" quiescence gate would pass right now. The memory-recorded G3–G8 gaps the plan worries about are *latent future* work, not a *currently-open edit* on the file — nothing is in flight. Note: H1's cited HEAD `2a0558e0d` is stale vs. the actual doc-only HEAD, but this does not affect quiescence (no `hydra.rs` bytes are dirty).

---

## Check 3 — §6.3 three quick-win facts — **CONFIRMED (all three hold; one label nuance on `kernel/=5`)**

**(a) `kernel/=5` — EXISTS.**
```
$ ls -la 'kernel/=5'
-rw-r--r-- 1 root root 0 Jul 13 15:29 kernel/=5     # 0 bytes, still present
$ git check-ignore -v 'kernel/=5'
.gitignore:77:kernel/=5	kernel/=5              # <-- explicitly git-IGNORED, not merely untracked
$ git ls-files --error-unmatch 'kernel/=5'
error: pathspec 'kernel/=5' did not match any file(s)  # not tracked
```
Nuance: the audit (§3 row 29) and plan (§4 #29) call it "untracked." It is in fact **gitignored** at `.gitignore:77` — a stronger state (that is why `git status` never surfaces it as `??`). The fix (`rm 'kernel/=5'`) remains valid, but is now *incomplete*: it should also delete the dead `.gitignore:77` entry. Advisory refinement, not a blocking error.

**(b) Quarantined UI tree — EXISTS, unimported, ignored.**
```
$ ls apps/web/node_modules/@deliveryos/.ignored_ui/        # dist/ node_modules/ src/ package.json …
$ find …/.ignored_ui -name AnimatedNumber.tsx
apps/web/node_modules/@deliveryos/.ignored_ui/src/components/molecules/AnimatedNumber.tsx
$ git check-ignore …/.ignored_ui/  → matched (ignored)
$ grep -rn 'ignored_ui|AnimatedNumber' apps/web --include=*.ts(x) | grep -v node_modules
NONE (no tracked source references)
```
Tree still present, `AnimatedNumber.tsx` still there, gitignored, and **no import** of `.ignored_ui`/`AnimatedNumber` in tracked `apps/web` source. Matches plan #28 exactly (delete + optional CI grep).

**(c) Dead one-shot cron `bebop-library-star-list` — CONFIRMED `[active]`, never fired.** `hermes` CLI is available (`/usr/local/bin/hermes`).
```
$ hermes cron list
  3f0dee1a57ff [active]
    Name: bebop-library-star-list
    Schedule: once at 2026-07-12 15:30      # 4 days in the PAST (today 2026-07-16)
    Next run: 2026-07-12T15:30:00+00:00     # past
  ⚠ Gateway is not running — jobs won't fire automatically.
```
`~/.hermes/cron/jobs.json` corroborates: `"enabled": true, "state": "scheduled", repeat {times:1, completed:0}`, run_at 2026-07-12 (past) — i.e. active, never completed (`last_run` effectively None). Matches plan #17 / audit row 17 verbatim.

---

## Check 4 — Completeness re-check (independent) — **CONFIRMED (29 rows, each once, correctly bucketed)**

Read `HERMETIC-ARCHITECTURE-PRINCIPLES.md` §3 (rows 1-29) and cross-checked every row number against the plan §1 table (plan lines 33-61). Result: rows **1,2,3,…,29 each appear exactly once**, in order, no gaps, no duplicates. Bucket tally independently recomputed:
- Blueprint-mapped (17): 1-7, 10-13, 15, 16, 18, 23, 24, 27 → counted = 17. ✓ Each row's "Fix location" cell names a real NEW (H1/H2/H3/H4) or EXISTING (P01/P02/P06/P07/P08/P12) artifact.
- Quick-wins (6): 14, 17, 19, 20, 28, 29 → 6. ✓ Each row cell says QUICK-WIN.
- Backlog (6): 8, 9, 21, 22, 25, 26 → 6. ✓ Each row cell says BACKLOG.
- 17 + 6 + 6 = **29/29, zero silent drops.** The plan's own completeness claim (§1 lines 63-65) is accurate.

Spot-check of miscategorization risk: the H2 sub-section mapping is internally consistent with H2's actual structure (#10→H2§2.1, #12→§2.4, #18→§2.3, #23→§2.2, #24→§2.5 — all match H2's real headings). No row is mapped to a non-existent or wrong section.

---

## Check 5 (verifier's choice) — H1 changes the `ingest_peer_breach` signature H3 depends on — **CONFIRMED**

Plan §3 Wave 2: *"Because H1 changes the very signature H3 consumes (`ingest_peer_breach` becomes `Result<(), StoreError>`, H1 §2.4), H3 must build after H1 lands … a data-shape dependency."*

- **Live current signature:** `kernel/src/hydra.rs:329` → `pub fn ingest_peer_breach(&mut self, alert: &BreachAlert) {` — returns unit `()`, discards `append_raw`'s return. Matches H1's premise exactly.
- **H1 §2.4 (line 136):** changes it to `ingest_peer_breach(&mut self, alert) -> Result<(), StoreError>` ("propagate the `append_raw?`"). So H1 genuinely alters the return type (unit → `Result`).
- **H3 consumes it:** H3 §2 (Attests-Locked path) and §3 step 5 ("Wire the Locked verdict to the **existing** `ingest_peer_breach` (no new sink)") both route through it. H3 also uses `append_raw` (step 4), which H1 §2.3 likewise re-types to `Result<AppendOutcome, StoreError>`.

Both blueprints' text corroborate the dependency, and the *live* pre-H1 signature confirms the change is real (not a phantom). The plan's "data-shape dependency, not just merge-conflict avoidance" characterization is accurate — H3's call sites must handle the new `Result`, and H1 + H3 additionally share `hydra.rs`. **CONFIRMED.**

---

## Overall verdict — does HERMETIC-REMEDIATION-PLAN.md need a correction?

**No blocking correction required.** Every load-bearing claim the plan's §6 doubt audit flagged as "asserted, not re-verified" holds up when checked against live ground truth: the `wasm.rs` regions are diff-level disjoint (§6.1), `hydra.rs` is quiescent (§6.2), and all three quick-win facts (§6.3) are true. Completeness (29/29) and the H1→H3 signature dependency both confirm. The plan's honesty in *downgrading* the H2∥P4 claim to a lane-sequencing instruction (rather than a parallelism guarantee) is vindicated — the diff-level check finds even less collision risk than the plan conservatively assumed.

**One optional refinement (advisory, not an error):** plan §4 quick-win #29 currently reads

> `**#29 — stray artifact.** `rm 'kernel/=5'`. One command.`

`kernel/=5` is not merely "untracked" — it is pinned in `.gitignore:77`. Suggest amending #29 to also remove that dead ignore line, e.g.: *"`rm 'kernel/=5'` and delete the now-dead `.gitignore:77` `kernel/=5` entry."* The same "untracked" wording appears in the audit's §3 row 29 evidence; both are technically "gitignored." This does not change any wave, dependency, or fix outcome.

*Read-only verification. No source code, blueprint, or the remediation plan was edited by this audit.*
