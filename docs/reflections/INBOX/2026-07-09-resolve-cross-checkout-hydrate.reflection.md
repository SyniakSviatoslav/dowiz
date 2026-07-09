---
CONTEXT:   Daily plane-maintainer run (`run-20260709T0603`), calibration/SENSE step: resolving
           2 predictions carried over from `run-20260708T0603` (`a8865ceada18` PR-backlog,
           `36b0cea375aa` cloud-egress). Both `plane-telemetry.mjs resolve` calls failed
           `prediction <id> not found` on the very first attempt in this fresh cloud checkout.
DECISIONS: Fixed `cmdResolve` in `scripts/plane-telemetry.mjs` to fall back to `fetchBranch()` +
           `readBranchRows()` — the exact hydrate path `inbox` already used — when the prediction
           is absent from the local `loops/runs/predictions.jsonl`, then persist the hydrated row
           locally before applying the resolve so a later `publish` carries it forward. Added a
           red→green regression test (2-checkout git fixture: predict+publish from one scratch
           repo, resolve from a second that never ran `predict`) to `plane-telemetry.test.mjs`,
           ledgered as REGRESSION-LEDGER.md #57. No staging deploy — this is a governance-plane
           CLI script, not app runtime; the unit-test proof matches the precedent of prior
           plane-telemetry-only fixes (open PRs #11/#13, same file).
WHERE:     scripts/plane-telemetry.mjs (`cmdResolve`); scripts/plane-telemetry.test.mjs (new test);
           docs/regressions/REGRESSION-LEDGER.md #57. Live proof: the fix, applied mid-run, is
           what let `a8865ceada18`/`36b0cea375aa` actually resolve (both `gap=hit`) later in this
           same run.
WHY:       `predict`/`publish` write `predictions.jsonl` ONLY to the `telemetry/plane` orphan
           branch — by design, never committed to `main` (durability lives on the append-only
           branch, not the ephemeral box, per ADR-plane-telemetry-and-calibration). But `resolve`
           was written to read the LOCAL scratch copy only, with no fallback — an asymmetry with
           `inbox`, which already hydrates from the branch tip. Every plane-maintainer run gets a
           BRAND NEW checkout (per this repo's cloud-session model), so `resolve` on a prediction
           from ANY prior day's run was structurally guaranteed to fail with "not found" — not a
           flaky edge case, a 100%-reproducible gap on the write path of the exact durability
           property (`predictions-jsonl-durability`) that TWO EARLIER ledger misses
           (`cf2f24fa26a2`, `285ae0f675a2`) had already flagged on the read/persistence side.
           Root cause: when a system has two code paths that both need "read predictions from
           storage" (one to LIST them, one to MUTATE one), fixing durability for the list path
           (`inbox`) does not imply the mutate path inherited it — each consumer of a durable
           store needs its OWN hydrate, or a shared one both paths are forced through.
CONFIDENCE: high — reproduced live (not hypothetical), root-caused to one specific code asymmetry,
           fixed with a proven red→green test, and the same fix immediately unblocked today's
           actual calibration step.
NEXT-TIME: When a durability gap is found and fixed for ONE consumer of a store (e.g. `inbox`
           reading branch-hydrated predictions), grep every OTHER function that reads the same
           local path (`PREDICTIONS_PATH()`, here `cmdResolve` + `cmdPredict`'s dedup) and check
           each one independently survives a fresh-checkout scenario — a fix scoped to the
           symptom that was noticed (list/read) can leave a structurally-identical gap on a path
           nobody exercised yet (mutate/write). `cmdPredict`'s own local-only dedupe read
           (`readJsonl(PREDICTIONS_PATH())` for `predictSeq`) is the same shape and unexamined
           this session — worth a follow-up check.
LINK:      docs/regressions/REGRESSION-LEDGER.md #57 ; docs/adr/ADR-plane-telemetry-and-calibration.md ;
           [[predictions-jsonl-durability]] (prior misses cf2f24fa26a2, 285ae0f675a2)
---
