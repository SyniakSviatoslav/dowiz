# Verification report · security-redblue · v0.1 · 2026-07-02

Вердикт: **CERTIFIED**
Верифікатор: loop-architect (модель: opus-4.8) · cross-opinion (M11): FLAGGED для незалежної моделі (OpenRouter-міст) — менш корельований погляд на the anti-cheat surface (evidence-not-assumed + no-autonomous-offense honesty).

Loop card: `loops/security-redblue.yaml` · Charter: `docs/security/security-loop.md`
Anti-cheat harness: `tools/security-redblue/dry-run.mjs` (hermetic, no attack traffic — 20/20)
Grounded in the SHIPPED surfaces: blue scouts `scripts/asset-surface-scan.mjs` + `scripts/scout-feeds.mjs` (real, help/parse-verified, `--test-fixture` seam); invariant checks `pnpm run verify:{rls,secrets,privacy,env}` (real package.json scripts → verify-rls.ts / verify-secrets.ts / pii-leak-detector.test.ts / verify-env.ts); security-E2E `admin-platform-authz.spec.ts` · `courier-room-authz-isolation.spec.ts` · `flow-security-contracts.spec.ts` · `flow-security-regression-2026-06.spec.ts`; telemetry `scripts/plane-telemetry.mjs` (kind=scout|probe verified, emit smoke passed). Governing docs: `docs/security/redteam-{toolset-analysis,runbook}.md`, `docs/governance/model-calibration.md`, `docs/operating-model/living-loop-system-v3.md` §5.

## 4-умовний тест (M2) — це петля, не промпт
1. Recurring trigger — the operator repeatedly hardens their own app-layer security posture (recurring cross-tenant/IDOR/JWT/injection/secret classes across the codebase's history). ✓
2. Fixed multi-stage body per run (SENSE blue scouts+static+dep-audit+E2E-replay & red-plan → DIAGNOSE triage by class/severity/red-line → ACT guardrail|council|carry → VERIFY evaluateRun evidence+disposition gate → REPEAT/exit). ✓
3. Machine verification at every disposition (evidence-not-assumed · no-close-without-guardrail · council-before-redline · advisory-forever · scout-surfaces-signal — all in `evaluateRun`/`classifyFinding`). ✓
4. Hard exit (exactly one verdict per run ∈ {ADVISORY-COMPLETE, INCOMPLETE, RED:violation}; gate always 'advisory'; §5 report + per-run artifact). ✓

## Рубрика M1–M11

| # | критерій | PASS/FAIL | доказ |
|---|---|---|---|
| M1 | структурна повнота (4 блоки + DNA) | PASS | card has all DNA fields filled, no placeholders. 4 blocks: **Тригер** (`/security-redblue` + dry-run + blue scripts) · **Виконавчі навички** (11 battle-tested: 2 scouts + 4 verify:* + pnpm audit + eslint + 4-spec E2E replay + telemetry + red-arm orchestration/intake) · **Ціль+Верифікація** (terminal verdict + 5 machine gates in evaluateRun + M11 flag) · **Вихід+Пам'ять** (single verdict per run + loops/runs artifact + `memory_file` with runbook). |
| M2 | 4-умовний тест | PASS | see above — loop-shaped (fixed SENSE→VERIFY body, machine gates, hard exit), not a one-shot prompt. |
| M3 | верифікація РЕАЛЬНА (не вайб) | PASS | `evaluateRun`/`classifyFinding` grade on PARSED facts: evidence artifact present (not null), disposition ∈ enum, class ∈ red-line set, guardrail!=null && ledger_row===true. NOT "no findings surfaced = secure". Dry-run Scenario A2/A4/A5 prove a run that only *looks* clean goes RED. |
| M4 | жорсткий вихід (ALL-must-hold) | PASS | exit_conditions: ADVISORY-COMPLETE REQUIRES every blue step evidenced ∧ red plan emitted ∧ every confirmed finding terminally disposed. Verdict is exactly one of {ADVISORY-COMPLETE, INCOMPLETE, RED:violation}; gate always 'advisory'. No "until it looks secure". |
| M5 | iron principles увімкнені (no-fake-green) | PASS | 9 ENFORCED principles: no-fake-green, evidence-not-assumed, advisory-forever, no-autonomous-offense, council-before-redline, close-only-with-guardrail, own-assets-staging-only, no-person-profiling, never-weaken-a-gate — each asserted by the dry-run, not declarative (A2/A4/A5/A8/A9 + Part C). |
| M6 | skill-driven (нуль фантомних навичок) | PASS | every skill verified real: asset-surface-scan (help printed), scout-feeds (exports parsed), verify:{rls,secrets,privacy,env} (grepped from package.json → real .ts targets), pnpm audit (built-in), 4 security-E2E specs (exist in e2e/tests/), plane-telemetry emit --kind scout (smoke returned event_id). Red-arm tools are ORCHESTRATED not invented — they run out-of-harness on Kali (docs/security/redteam-runbook.md). Zero phantom skills. |
| M7 | гейти у високоризикових точках | PASS | 5 gates at false-turn points: (a) SCOPE (third-party target STOPS the step); (b) AUTONOMY (red arm human-gated — harness never fires); (c) RED-LINE (council before any auth/RLS/money/PII fix); (d) EVIDENCE (no artifact ⇒ INCOMPLETE/RED, not 'secure'); (e) ETHICS (no person-profiling). Each maps to a dry-run scenario or a structural assertion. |
| M8 | out-of-scope + escalation | PASS | out_of_scope: third-party infra, offensive traffic/tooling, auto-fix, person-profiling, prod, product code, gate-behavior, the DB-owner invariant track. escalation: red-line→council; third-party→OUT-OF-SCOPE stop; no-env→honest skipped-no-env(INCOMPLETE); RED:violation→STOP+raise (never self-clear); missing skill→degrade+record; recurrent class→propose graduation (human-gated). |
| M9 | anti-cheat dry-run (зламане → RED/escalate) | PASS | `node tools/security-redblue/dry-run.mjs` → **20/20 PASS**, exit 0; broken/cheating fixtures all go RED/INCOMPLETE (block below). Mutation-intent check confirmed: the three cheat fixtures (executed-no-evidence, closed-without-guardrail, redline-autofix) each return RED:violation, never ADVISORY-COMPLETE; clean run gate === 'advisory' (never PASS). |
| M10 | пам'ять підключена | PASS | `memory_file` `loops/memory/security-redblue.md` (blue recipe + red engagement + disposition rules + lessons + run-history table); harness `always_emit_report:true` writes `loops/runs/security-redblue-<ts>.json` + telemetry emits kind=scout|probe; predict/resolve calibration wired per model-calibration.md. |
| M11 | separate-agent крос-рев'ю | PASS (flagged) | marked for an independent model (OpenRouter-bridge) to re-attack the anti-cheat honesty — especially evidence-not-assumed (could a "clean" run slip through without artifacts?) and no-autonomous-offense (is the offensive capability truly absent from the code shape?). Logged as the M11 hook. |

## Anti-cheat dry-run (M9) — 20/20 PASS

Command: `node tools/security-redblue/dry-run.mjs` (re-runnable, hermetic, no network/attack traffic).

**PART A — verdict-engine anti-cheat (the finding gate + advisory boundary), 12 assertions:**
- A1 unrun blue step ⇒ **INCOMPLETE** (not "clean").
- A2 executed-without-evidence ⇒ **RED:violation** (fake-green:no-evidence) + flags the step issue.
- A3 confirmed finding without a guardrail ⇒ **INCOMPLETE** (cannot close).
- A4 closed-without-guardrail (disposition says landed, no guardrail/ledger) ⇒ **RED:violation** (VIOLATION:closed-without-guardrail).
- A5 red-line (rls) auto-fixed ⇒ **RED:violation** (VIOLATION:council-bypass).
- A6 red-line (jwt) council-queued ⇒ **ADVISORY-COMPLETE** (routed, not fixed — terminally disposed).
- A7 non-red-line finding closed w/ guardrail+ledger ⇒ **ADVISORY-COMPLETE**.
- A8 clean run w/ evidence ⇒ **ADVISORY-COMPLETE**, gate === 'advisory'.
- A9 gate === 'advisory' on EVERY verdict branch (never a PASS gate).
- A10 unconfirmed finding does not block completion (advisory triage).

**PART B — real blue-arm smoke (crt.sh asset-surface scout via `--test-fixture`, no network), 3 assertions:**
- B1 the scout SURFACES a planted forgotten-preview subdomain as NEW (total_new≥1) + flags the exact host.
- B2 a baseline-matching surface ⇒ 0 NEW (real diff, not always-red / not always-green).

**PART C — structural (red arm is intake-only), 2 assertions:**
- C1 NO offensive/attack export exists in the harness (attack/exploit/fireSqlmap/runAutorize/sendPayload/bruteForce all undefined) — `no-autonomous-offense` is a property of the code shape, not a toggle.
- C2 the harness exports the verdict engine (evaluateRun/classifyFinding) — intake + grade only.

A status-only "no findings = secure" loop would falsely certify A1/A2/A3/A4/A5. The gate blocks all five.

## Test seam (why hermetic core, not full-live)
Full LIVE certification needs a human + a disposable Kali VM (the RED arm), a reachable staging DB
(verify:rls/privacy), and a live Playwright browser vs staging — environment-blocked for an autonomous
cert. The CERTIFIABLE core — the finding-gate verdict logic (evidence-not-assumed · no-close-without-
guardrail · council-before-redline · advisory-forever) + the blue-arm scout smoke + the structural
no-offense property — is certified hermetically (20/20). Live steps to run end-to-end are in
`loops/memory/security-redblue.md` + `docs/security/security-loop.md` §2–3.

## Residual / follow-ups
- Not yet run against a live staging engagement (BUILT+CERTIFIED, not yet RUN). First live run = a
  run-history row in the memory file + the first REGRESSION-LEDGER rows for any confirmed finding.
- M11 independent cross-review still to be executed by the OpenRouter-bridge model; flagged, not yet returned.
- The DB-owner proactive-invariant track (SECURITY-DEFINER search_path, NOBYPASSRLS, RLS WITH CHECK)
  stays owned separately (runbook §6); this loop replays verify:rls as a backstop, it does not own those migrations.
- Suricata/ELK (network-IDS/SIEM) are PARKED-with-trigger: re-open only if dowiz leaves managed Fly for infra it controls (runbook §6).
