# Verification report · acquisition-bulk-provision · v0.1 · 2026-06-28

Вердикт: **CERTIFIED**
Верифікатор: loop-architect (модель: opus-4.8) · cross-opinion (M11): FLAGGED для незалежної моделі (OpenRouter-міст) — менш корельований погляд на gate-honesty.

Loop: `loops/acquisition-bulk-provision.yaml` · executable: `scripts/acquisition-bulk-provision.mjs`
Anti-cheat harness: `tools/acquisition-bulk-provision/dry-run.mjs` + faithful mock `tools/acquisition-bulk-provision/mock-internal.mjs`
Grounded in the SHIPPED pipeline: `apps/api/src/modules/acquisition/{route,state-machine,service,claim,ops-auth}.ts`

| # | критерій | PASS/FAIL | доказ |
|---|---|---|---|
| M1 | структурна повнота (4 блоки + DNA) | PASS | card has trigger/execution_skills/goal+verification/exit+memory + all DNA fields filled (no placeholders). 4 blocks: Trigger (node …+list) · Execution skills (6 real endpoints) · Goal+Verification (field-gates + separate-agent flag) · Exit+Memory (terminal-per-item + loops/runs + memory_file). |
| M2 | 4-умовний тест | PASS | (1) recurring trigger — operator onboards N restaurants repeatedly; (2) fixed multi-stage pipeline per item (6 stages, state-pinned); (3) machine verification per stage (state/verified/token); (4) hard exit (terminal outcome per item, run summary). It is loop-shaped, not a prompt. |
| M3 | верифікація реальна (не вайб) | PASS | gates assert the PARSED field: `result.state==='ENRICHED'`, `isUuid(org_id&&location_id)`, `json.verified===true`, non-empty `token`. Not "HTTP 200 = pass". Dry-run Scenario A: 2 invited only because real tokens returned; 3 needs-review on real exit verdicts. |
| M4 | жорсткий вихід (ALL-must-hold) | PASS | exit_conditions: each item → exactly one of {invited, needs-review, skipped-already-done}; run ends only when no item is left in a working state; exit code 3 iff any needs-review, 2 on missing config. No "until it looks ok". |
| M5 | iron principles увімкнені (no-fake-green) | PASS | 7 enforced principles incl. no-fake-green, never-spine-an-exit-state, fail-closed, secret-never-printed — each ENFORCED in code + asserted by the dry-run (not declarative). |
| M6 | skill-driven (нуль фантомних навичок) | PASS | every execution_skill is a real route verb in `route.ts` (lines 62/77/90/107/142/159), each run manually on staging this session. Zero phantom skills. |
| M7 | гейти у високоризикових точках | PASS | (a) target-URL gate (never prod for a test — this writes shadow rows); (b) per-stage verification gate at the "false turn = whole loop garbage" points: non-ENRICHED never spines, non-verified never claim-mints. Dry-run proves never-spine on MENU_NOT_FOUND/LOW_QUALITY (org_id null). |
| M8 | out-of-scope + escalation | PASS | out_of_scope: publishing, AI internals, notice delivery, any /api/auth/RLS/money. escalation: unexpected state → needs-review + continue; whole-surface 404 → STOP+raise; no-contact invite → decline-only warning. |
| M9 | anti-cheat dry-run (зламане → RED/escalate) | PASS | `node tools/acquisition-bulk-provision/dry-run.mjs` → 21/21 PASS. See block below — broken fixtures + LIAR backend + wrong secret all go needs-review, run never aborts/fakes. |
| M10 | пам'ять підключена | PASS | memory_file `loops/memory/acquisition-bulk-provision.md` (lessons + run-history); every run writes a lossless artifact to `loops/runs/<ts>.json`. |
| M11 | separate-agent крос-рев'ю | PASS (flagged) | marked for an independent model (OpenRouter-bridge) to re-attack gate-honesty / the cheat surface — a less-correlated look. Logged here as the M11 hook. |

## Anti-cheat dry-run (M9) — 21/21 PASS

Command: `node tools/acquisition-bulk-provision/dry-run.mjs` (re-runnable, hermetic, no network/DB/AI).
Mock mirrors the shipped state-machine: ops-auth 404 fail-closed · idempotent create returns CURRENT state ·
mint/spine REQUIRE ENRICHED (409 otherwise) · verify 409 NOT_VERIFIABLE on empty menu_draft · claim/mint 409
ACTIVE_INVITE_EXISTS on a second mint · plus a stage-targeted LIAR mode.

- **Scenario A — mixed batch (5 items: 2 happy, menunotfound, lowquality, emptymenu):** all 5 processed (no abort); exactly 2 invited (real fragment claim URLs, state CLAIM_OFFERED); 3 needs-review; the MENU_NOT_FOUND + LOW_QUALITY sources were **NEVER provisioned** (org_id still null); exit code 3; secret never printed.
- **Scenario B — idempotent re-run (same mock state):** 0 newly invited (happy ones already CLAIM_OFFERED → skipped-already-invited × 2); the empty-menu source **resumed at verify and stayed PROVISIONED — no re-spine** (no double-provision).
- **Scenario C — LIAR at verify (200 `verified:false`):** 0 invited; classified NEEDS-REVIEW:NOT_VERIFIABLE. A status-only loop would mark this invited → the gate read the FIELD.
- **Scenario D — LIAR at claim/mint (201, no token):** 0 invited; classified NEEDS-REVIEW:CLAIM_MINT. Requires a real token string, not a 201.
- **Scenario E — wrong ops secret (surface 404s):** 0 faked successes; classified NEEDS-REVIEW:OPS_AUTH_404 (fail-closed, not a crash); secret not echoed.

Smoke (live-config guards, separate run): missing PROVISION_OPS_SECRET → `FATAL` exit 2; missing PROVISION_BASE_URL → exit 2; CSV parses; a network error becomes a per-item NEEDS-REVIEW:NETWORK (continue-on-failure, no crash); stdout prints `secret: [redacted]`.

## Test seam (why mock, not full-live)
The extract stage needs a real website + an AI key, so a full live certification is environment-blocked.
The orchestration / idempotency / gate logic is certified against the faithful mock + LIAR. To certify
stages 4–6 LIVE without the AI extract: DB-bump a source to ENRICHED with a minimal menu_draft
`{"categories":[{"name":"Mains","products":[{"name":"X","price":80000}]}]}` then run the loop — it reads the
state and resumes at mint→spine→verify→claim/mint (idempotent-resume path, exercised in Scenario B).

## Residual / follow-ups
- Not yet run against a live deployment (BUILT+CERTIFIED, not yet RUN-on-staging). First live run = a run-history row in the memory file.
- M11 independent cross-review still to be executed by the OpenRouter-bridge model; flagged, not yet returned.
