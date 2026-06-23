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
| 0 (smoke) | 1 (client-first-timer, scripted) | 0 | — | Checkpoint-A plumbing smoke — NOT a discovery round (ScriptedReasoner ≠ discovery engine) |
| 1 | client-first-timer, client-price-skeptic (LlmReasoner, free) | 0 valid | 0 | 3 findings — all NOT_A_BUG (LLM hallucinated selectors); driver hardened |
| 2 | client-first-timer (LlmReasoner, free) | 0 valid | 0 | exposed observe-before-hydrate timing bug (fixed); no valid finding |
| 3 | client-first-timer (LlmReasoner, free, grounded+settle) | 0 valid | 1 | clean session — searched + add-to-cart via real data-testids; 0 false findings |

> NOTE: rounds run a cost-bounded SUBSET on free models — these are NOT full saturation rounds.
> A saturation round = all personas × {390,768,1280} × {al,en}; deep discovery wants more rounds
> and/or a stronger model (free slugs are heavily 429'd and weak → short sessions). The set is
> expand-only; subset rounds never count toward the K-round saturation exit.

### Triage log
- **R1 F-21311 / F-41412 / F-74853** (`/s/demo:click:{menu-button|California (6)|item-card-0}:unactionable`):
  **NOT_A_BUG — LLM hallucinated selectors** (none exist; real testid is `menu-item`). Root cause:
  observation was too thin, so the model guessed. **Fixed** by grounding observe() with real
  selectors + instructing the reasoner to copy only from the actions list. Dropped with reason.
- **R0 smoke ×2** (`…:add it to the cart:unactionable`): NOT_A_BUG — ScriptedReasoner guessed
  wrong selector; real add-to-cart at `ProductCard.tsx:182` (`tooltip.add_to_cart`). Dropped.
- **R3**: no findings — the grounded persona completed its happy path on real controls (signal
  that the storefront client happy-path has no friction at this depth).

Exit (discovery) = a FULL round (all personas × {390,768,1280} × {al,en}) yields **0** new valid in-scope findings ≥ minor, held **K** consecutive rounds (K=2; go-live K=3). The set is fixed-or-expanding between rounds, never narrowed.

## Open findings
_None yet — the driver (A2) has not run._

## Flagged (human decisions: CONTRACT_GAP / DRIFT)
_None yet._
