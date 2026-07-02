# Resolution — Plane Telemetry Egress + Principles Ratchet + Local Closed Loop (Breaker R1+R2 · Counsel R1+R2)

- **Date:** 2026-07-02
- **Author:** System Architect (DeliveryOS)
- **Inputs:** `breaker-findings.md` (R1: 1 CRIT + 4 HIGH + 5 MED + 2 LOW; R2: closure-audit + 3 HIGH + 5 MED/LOW) · `counsel-opinion.md` (both rounds OPINION: proceed; ETHICAL-STOP: none)
- **Outputs updated:** `proposal.md`, `docs/adr/ADR-plane-telemetry-and-calibration.md`; `.gitignore`/`.gitattributes` R1 edits **reverted** (R2-M5).
- **Folded in:** operator addendum **PART 3 — Closed Loop (local ingestion)**.
- **Result:** R1 + R2 every CRITICAL/HIGH → **fix**; MED → fix or accept-risk-with-mitigation; LOW → fix; Counsel R1+R2 → addressed. **No CRITICAL or HIGH remains open.** ADR **🟢 APPROVED — pending build-proof** (STOP-DESIGN-B test table = DoD).

Disposition classes: **FIX** (design changed) · **ACCEPT-RISK** (justification + owner) · **DEFER-FLAG** (documented MISSING).

> Sections below are ordered: R1 Breaker table · R1 Counsel · PART 3 · **ROUND 2 (R2 Breaker + R2 Counsel)** · ship-discipline · status.

## Breaker findings

| # | Sev | Finding (one line) | Disposition | What changed |
|---|---|---|---|---|
| C1 | CRITICAL | "Authoritative local JSONL" evaporates — gitignored + ephemeral cloud box → 0 durable events. | **FIX** | Durability = the **git repo**. `.gitignore` un-ignores `plane-events-*.jsonl` + `predictions.jsonl` (not a protect-path); REPORT step commits+pushes them with the daily digest. Telegram = secondary; git = source of truth. proposal §4/§6/§7, ADR Decision 2. |
| H1 | HIGH | Denylist misses real in-use secrets (`DEV_AUTH_SECRET=stg-e2e-secret`, `sbp_`, `sb_secret_`, Plisio/R2). | **FIX** (+ ACCEPT-RISK residual R1) | Added `sbp_`/`sb_secret_`/`FlyV1`/`fm[12]_`/`fo1_` + vendor-hex patterns; added the load-bearing **`KEY=VALUE` secret-name rule** (redacts the value regardless of shape); **canary guardrail test** (red→green) per class. Truly-shapeless-unprefixed residual accepted (R1, owner = pattern-list maintainer). proposal §8/§9. |
| H2 | HIGH | Phone regex mangles `run_id`/dates → Telegram↔JSONL bridge severed for 100% of runs. | **FIX** | Redaction is now **field-scoped**: only free-text (`detail`/`note`/prediction strings) scanned; structural fields (`run_id`,`ts`,`event_id`,`seq`,`tags`,`refs`) **never** scanned. Summary composed from verbatim structural fields. Phone pattern hardened (≥9 digits, ISO-date-guarded) and only ever runs on free text. proposal §6/§8. |
| H3 | HIGH | Silent-skip: chat_id unset + ephemeral → months of zero telemetry, exit 0, "success". | **FIX** | New plane-guard **telemetry-liveness SOFT check** (newest committed event age < N days) + a **channel-status line** in every digest (`telegram=sent\|skipped:reason · git=committed`). Silence surfaces in the daily gate. proposal §9, R8. |
| H4 | HIGH | Advisory→authority drift prevented only by prose; the gate to wire it into is already open. | **FIX** | New plane-guard **advisory-forever HARD check**: greps the gate surface, FAILS if `predictions.jsonl`/`plane-events` referenced by any gate outside the allowlisted liveness check. Barrier is now structural, not a reviewer's memory. proposal §9, ADR Decision 5, R6. |
| M1 | MED | Self-reported calibration is forgeable (backdate/hedge/cherry-pick). | **ACCEPT-RISK (partial fix)** | Self-report inherent to single-agent introspection. Added `predict_seq` + **resolve-refuses-out-of-order** ordering friction; framed as **calibration-not-hit-rate** (Goodhart guard); un-gated removes the gaming incentive. Residual = determined self-deceiver, accepted (R-M1, owner = operator). proposal §5/§10, ADR Decision 5. |
| M2 | MED | Run-level idempotency false — whole-run re-fire mints a new random `run_id` → no dedup. | **FIX** | `run_id` now **derived from the firing timestamp** (not random) → a double-fire reproduces identical `(run_id,kind,seq)` → identical `event_id` → deduped. `event_id` is `ts`-free. proposal §5/§6. |
| M3 | MED | Two parallel cloud sessions → fragmented/unmergeable telemetry OR TOCTOU dedup race. | **FIX** | Folded into C1: each session commits; `.gitattributes merge=union` unions append-only lines; **dedup moved to READ time** (`jq unique_by`) — the check-then-append tail-grep (the TOCTOU race) is removed. proposal §6, `.gitattributes`. |
| M4 | MED | `sendDocument` attaches the monthly file (over-egress) + stale redaction on old records. | **FIX** | Attach only the **current-run slice**, written to a temp file and **re-scanned with the current pattern list at attach time**; monthly file never attached. proposal §7. |
| M5 | MED | Redactor fail-OPEN on write contradicts "redact before every write". | **FIX** | Fail-**closed on both paths**: on redactor throw, write a `{kind:"redactor_error"}` stub and drop the payload — never an un-redacted record. proposal §7. |
| L1 | LOW | `schema_version` never validated; `seq` referenced but absent from the record. | **FIX** | `seq` added as a first-class monotonic field (authoritative order + dedup identity); read tooling filters `schema_version==1` and warns on unknown. proposal §5/§9. |
| L2 | LOW | Box-local clocks + monthly suffix → skewed ordering, split runs, partial digest. | **FIX** | UTC-only timestamps documented; **ordering by `seq` not `ts`**; `resolve` last-write-wins by `predict_seq` not `ts_actual`; `digest` globs current+previous month files. proposal §5/§9. |

## Counsel advice

| Item | Disposition | What changed |
|---|---|---|
| (a) Connection mapping is the loosest bond — strengthen or downgrade honestly. | **ADDRESSED (strengthen + bound)** | Reframed as **give-first costly signal** (agent pays the predictability cost + publishes proof BEFORE asking for trust → that asymmetry opens access), and **explicitly labelled a partial mapping** (predictability sub-property, not full "opens access"). proposal §4.7, ADR Decision 6. |
| (b) Add "ledger is forever a mirror, never a stick" verbatim as a standing constraint. | **ADDRESSED** | Added **verbatim** as a blockquoted Standing Constraint in the ADR, enforced by the H4 advisory-forever check. |
| (c) Frame calibration-not-hit-rate explicitly in model-calibration.md spec. | **ADDRESSED** | model-calibration.md required-content spec added to proposal §4: calibration=reliability, no score to maximize, over/under-confidence both flags, Brier-note, reflection-is-the-growth-row-is-input. |
| §3 Guard the template — governance-plane-only fence. | **ADDRESSED** | Governance-plane-only rule added to §8 + charter edit; product/courier-plane reuse = separate 🔴 Triadic-Council decision. |
| §3 Keep reflection primary, row secondary. | **ADDRESSED** | Stated in the model-calibration.md spec (proposal §4). |
| §3 Timebox as bounded governance investment. | **ACKNOWLEDGED** | Noted; not on the launch-trigger critical path (Counsel §Long-horizon). Non-goal NG2 fences scope-creep to an observability platform. |
| §5 Open question: is power more legible to the operator than to the people it watches? | **ACKNOWLEDGED (philosophical, carried)** | The downward-transparency asymmetry is real and out of this design's scope (governance-plane); the governance-plane-only fence + SCOUT PII ban are the local guards. Named here as a standing awareness item, not a design defect. |
| ETHICAL-STOP watch-lines (weaken Layer-1 / SCOUT personal data / R6 gating-PR). | **ACKNOWLEDGED** | All three are now structurally or policy-guarded (allowlist-by-construction, SCOUT ban, advisory-forever check). Watch-lines carried for the implementer. |

## PART 3 — Closed loop (operator scope addendum, folded in)

Requirement: maintainer output must be **visible, storable, adjustable** on the local box — a closed loop where the local project can be triggered/changed by the maintainer's findings. Designed as the **ingestion** half of the seam (C1's git-as-durable-store already provides transport: cloud commits, local pulls). Same single CLI, stdlib only.

| Requirement | Disposition | What changed |
|---|---|---|
| VISIBLE | **ADDED** | `scripts/plane-telemetry.mjs inbox`: `git fetch origin` → discover artifacts new since a cursor (committed `plane-events` rows, `plane-status-*.md`, `docs/reflections/INBOX/*.md`, `gh` PRs/issues labeled `plane-guard`) → errors-first actionable summary (hard fails → escalations → PRs awaiting review → unresolved predictions → reflections). proposal Part 3, ADR Decision 8. |
| STORABLE | **ADDED (reuses C1)** | Record = committed JSONL/digests (C1). Only per-box cursor `loops/runs/inbox-cursor.json` is local state (gitignored — not the record). proposal Part 3. |
| ADJUSTABLE / closed loop | **ADDED** | `inbox --json` stable shape `{schema_version, items:[{kind,source,…}], counts, advisory:true}` so loop-orchestrator/reflections/librarian can file a maintainer finding as a local work item. proposal Part 3. |
| Authority boundary | **ADDED (structural + ADR clause)** | Ingestion is **read-only** (fetch + cursor-write only); NEVER auto-executes fixes / auto-merges PRs / auto-mutates gates; `advisory:true` stamped on every payload; human/local session decides. Stated in the ADR **next to the mirror-not-stick clause** as a second standing constraint. R9. |
| Recurrence (loop-shaped) | **ADDED (noted, not built)** | Idempotent + cursor-based → any scheduler (post-`git pull` hook / daily cron / later `loops/` card) drives it. YAGNI: the card is a thin wrapper later. proposal Part 3. |
| BoE (inbox scan cost) | **ADDED** | One `git fetch` + one optional `gh` call + a few-MB `--since` line-scan, < 2 s wall; offline < 100 ms. proposal §2. |
| Failure matrix (cursor corruption / offline / no `gh`) | **ADDED** | Corrupt/missing/future cursor → full rescan + fresh cursor, never crash (R10); no network → git-only view (`online:false`); `gh` absent → pane `skipped`. proposal §7. |

Files changed for Part 3: `proposal.md` (§2 BoE, new Part 3 section, §7 failure matrix, §10 risks R9/R10), `ADR` (title, Decision 8, second standing-constraint clause, operability line, risks line).

## ROUND 2 — RE-ATTACK dispositions

R2 verdict was NOT-converged: closed (H2/M4/M5/L1/L2), honest-partial (H1/M1), **relocated** (C1/M2/M3/H4), + 3 new HIGH + 5 new MED/LOW. All now dispositioned; the central move is the **`telemetry/plane` branch + git-plumbing** pivot, which resolves the relocations and lets the R1 `.gitignore`/`.gitattributes` edits be **reverted**.

| # | Sev | Finding | Disposition | What changed |
|---|---|---|---|---|
| R2-C1 | HIGH | git-durability collides with heavy pre-commit / `--no-verify` / no-commit-to-main / unmerged-branch invisibility. | **FIX (pivot)** | Dedicated orphan `telemetry/plane` branch via **plumbing** (no husky, no main commit) + **in-emitter fail-closed secret-scan on the committed blob** + readers read the branch. Non-ff → re-parent/retry, never force-push. proposal §4/§6/§7, ADR Decision 2. |
| R2-H2 | HIGH | read-time dedup silently drops distinct events (per-box `seq` collision under shared `run_id`). | **FIX** | Per-process `crypto.randomUUID()` `nonce`; `event_id` globally unique; dedup drops only exact re-sends; parallel same-minute boxes both survive. Sort=(ts,run_id,seq)+nonce. proposal §5/§6. |
| R2-H3 | HIGH | `inbox` ingests remote text into LLM consumers → prompt-injection; `advisory:true` is a label. | **FIX + test** | Sanitize all remote text (strip ANSI/control, JSON-escape, cap, tag `content_trust:"untrusted-remote"`); `spawnSync` arg arrays; injection-fixture guardrail (red→green); ADR/model-calibration mandate LLM consumers treat content as DATA. proposal Part 3 + §9. |
| R2-M1 | MED | no provenance on committed telemetry → git-history laundering / poisonable ledger. | **Accept-risk + FIX(surface)** | `inbox`/`digest` attach git sha/author/branch; non-allowlisted committer → `provenance:"unexpected"` (advisory, no auto-reject). Residual: no commit-signature gate (R-M1b). proposal §6/Part 3. |
| R2-M2 | MED | "advisory-forever" grep is evadable; "structural" overclaimed. | **FIX(honest reword) + hardening** | Reworded to **FRICTIONED + review-forcing, not "structurally impossible"** (ADR standing constraint + Decision 5); check hardened to walk an **enumerated** surface + indirection heuristics (closes bypass #2 casual case). Residual obfuscator accepted, named. |
| R2-M3 | MED | reused `execSync(\`gh … ${JSON.stringify}\`)` → shell injection, now reachable via remote PR titles. | **FIX** | All `git`/`gh` via `spawnSync` arg arrays, `shell:false`. Called out as a build-fix to the reused path. proposal §8. |
| R2-M4 | MED | canary must exercise space-bearing + lowercase secret shapes. | **FIX** | `KEY=VALUE` rule case-insensitive + whole-value; canary fixture MUST include `FLY_API_TOKEN=FlyV1 fm2_…`, `token=`, `password:`. proposal §8/§9. |
| R2-M5 | MED | tracked files inside disposable-scratch `loops/runs/` → routine wipe + permanent git weight. | **FIX** | `.gitignore`/`.gitattributes` un-ignore **reverted**; record lives only on `telemetry/plane` under a clean `telemetry/` path; `loops/runs/` stays scratch. |
| R2-L1 | LOW | concurrent `inbox` runs regress the cursor. | **Accept (harmless)** | Advisory lockfile + idempotent; worst case re-surfaces advisory items (noise). proposal Part 3. |
| R2-L2 | LOW | `gh` absent hides awaiting-review escalations (ingestion silent-skip). | **FIX** | Explicit `PR/issue pane UNAVAILABLE (gh missing/unauthed)` line, never silently empty. proposal §7/Part 3. |

### Counsel Round 2

| Item | Disposition | What changed |
|---|---|---|
| (a) Give Part 3 a structural authority guard equal to H4. | **ADDRESSED** | New plane-guard **Part-3 ingestion-authority HARD check** (HARD-fails if gate/`scripts` code pipes `inbox` output into exec/auto-apply). proposal §9, ADR Decision 8 + standing constraint. |
| (b) Extend "mirror never a stick" to surfaced reflections, verbatim. | **ADDRESSED** | New verbatim standing constraint: "reflections surfaced in the inbox are read to understand what the agent LEARNED, never to grade the agent" (observer-effect / convergence-theater guard). ADR. |
| (c) Inbox ordering = uncertainty FIRST, encode as stable contract. | **ADDRESSED** | Order fixed: unresolved-predictions/misses → reflections → hard-fails → escalations → PRs → ok-last; encoded in spec + `--json` array order. proposal Part 3, ADR Decision 8. |
| Watch total surface / timebox. | **ACKNOWLEDGED** | NG2 fence reaffirmed; noted as bounded governance investment off the launch-trigger path (ADR consequences). |
| §5 sharpened open question (courier↔platform legibility asymmetry). | **ACKNOWLEDGED (carried)** | Named standing-awareness item, out of scope for this governance-plane design. |
| Honesty reword of "structural" (ties to R2-M2). | **ADDRESSED** | Both standing constraints + Decision 5 now say **FRICTIONED, not impossible**. |

## Ship-discipline note
Net change is **docs-only** for durability: the R1 `.gitignore`/`.gitattributes` edits were **reverted** (`.gitattributes` byte-identical to origin; `.gitignore` carries only an inert comment noting durability lives on `telemetry/plane`). No runtime/UI surface, no product code path → the deploy+Playwright loop does not apply. The **implementation** build (`plane-telemetry.mjs` incl. `inbox`, 3 plane-guard checks, canary + injection-fixture guardrails, charter + `model-calibration.md`) is a separate change carrying its own red→green proof — see the ADR **STOP-DESIGN-B** threat-model test table (the DoD for the 🟢 approval).

## Status
**ADR 🟢 APPROVED — pending build-proof. No CRITICAL or HIGH open** after two rounds. Standing accepted risks for the Breaker to re-probe on the *built* artifact: R1/R2-M4 (canary vs novel/space/lowercase shapes), R-M1 (out-of-order resolve, hedge-to-0.5), R-M1b (forge a row → must surface `provenance:"unexpected"`), R2-M2/R6/R9 (attempt to wire the ledger or `inbox` output into a gate via indirection → friction checks must trip), R2-H3 (injection fixture → sanitized DATA), R2-C1 (planted-secret blob → push aborted).
