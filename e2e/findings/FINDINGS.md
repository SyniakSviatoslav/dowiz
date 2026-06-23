# FINDINGS — Synthetic-User RSI Loop (live ledger)

> Living counter. Each row is a finding from an autonomous persona session. Status is
> `open → routed → fixing → Closed (lock id)` or `Flagged (decision)`. Runtime artifacts
> (per-finding JSON, traces, videos, transcripts) live in `e2e/findings/` and are gitignored.

## Finding schema (`e2e/findings/F-XXXX.json`)
```json
{ "id":"F-0001","round":1,"role":"owner","persona":"owner-friday-7pm-rush",
  "surface":"/admin/orders","viewport":"1280","locale":"al",
  "goal":"confirm and assign 5 incoming orders during rush",
  "step":"tapped Confirm on order #3",
  "observed":"button showed no loading state; double-tapped; unsure if it worked",
  "expected_as_user":"immediate in-flight feedback; no ambiguity",
  "category":"UX_FRICTION","severity":"major",
  "signature":"orders:confirm:no-loading-feedback",
  "repro":{"seed":4821,"trace":"traces/F-0001.zip","video":"videos/F-0001.webm","steps_ref":"transcripts/F-0001.md"},
  "route":"polish:B1+B3","status":"open" }
```
Categories → route: `BUG`/`A11Y_FUNC` → new RED row in `e2e/MATRIX.md` (inner convergence loop locks it with a regression); `UX_FRICTION`/`DESIGN_INCONSISTENCY` → `/audit-gate` work-item (verdict A/B/C/D); `CONTRACT_GAP` → `MISSING`/`BLOCKED-contract` flag (human, server read-only); `OUT_OF_SCOPE_WISH` → `DRIFT` backlog (human, do not build); `DUPLICATE`/`NOT_A_BUG` → drop with reason. Dedup key = `(surface, step, category, signature)`. Severity = `critical | major | minor | nit`.

## Saturation tracker
| Round | Personas × VP × locale run | NEW valid in-scope (≥minor) | Rounds-without-new | Notes |
|------:|----------------------------|----------------------------:|-------------------:|-------|
| —     | (loop not yet entered)     | —                           | 0                  | Phase A scaffolding only |

Exit (discovery) = a FULL round (all personas × {390,768,1280} × {al,en}) yields **0** new valid in-scope findings ≥ minor, held **K** consecutive rounds (K=2; go-live K=3). The set is fixed-or-expanding between rounds, never narrowed.

## Open findings
_None yet — the driver (A2) has not run._

## Flagged (human decisions: CONTRACT_GAP / DRIFT)
_None yet._
