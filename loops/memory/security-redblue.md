# Memory — security-redblue loop

Runbook + lessons + run-history for the blue-team + red-team security loop.
Card: `loops/security-redblue.yaml` · Charter: `docs/security/security-loop.md` ·
Report: `loops/reports/security-redblue-0.1.md` · Anti-cheat: `tools/security-redblue/dry-run.mjs`.

## What this loop is (and is NOT)
- IS: a two-arm loop over dowiz's OWN app layer. BLUE = autonomous safe sweep (scouts + static +
  dep-audit + verify:* + security-E2E replay). RED = human-gated Kali-workstation engagement that
  the loop ONLY plans + ingests. Findings are advisory; each → red→green guardrail + ledger row;
  red-line → Triadic Council before any fix.
- IS NOT: an offensive tool. The harness NEVER fires attack traffic, installs offensive tooling,
  auto-fixes a finding, targets third-party infra (Fly/Supabase/R2), touches prod, or acts as a gate.

## Blue-arm run recipe (autonomous, safe)
```bash
# read-only scouts (advisory; NEW subdomain / new upstream advisory = signal)
node scripts/asset-surface-scan.mjs --json          # crt.sh CT diff; --update-baseline to accept surface
node scripts/scout-feeds.mjs --json                 # upstream release/advisory watch
pnpm audit                                          # dep vuln audit (advisory)
# invariant backstops (honest skipped-no-env if DB env absent — NEVER a faked pass)
pnpm run verify:rls && pnpm run verify:secrets && pnpm run verify:privacy && pnpm run verify:env
npx eslint .                                         # static security-class lint (guardrail-authoring surface)
# security-E2E replay vs staging (proves shipped authz/RLS/JWT guardrails hold)
VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test \
  e2e/tests/admin-platform-authz.spec.ts e2e/tests/courier-room-authz-isolation.spec.ts \
  e2e/tests/flow-security-contracts.spec.ts e2e/tests/flow-security-regression-2026-06.spec.ts --reporter=list
# telemetry (advisory tags only, never a gate)
node scripts/plane-telemetry.mjs emit --kind scout --outcome pass --emitter security-redblue --detail "…"
```

## Red-arm engagement (HUMAN + disposable Kali — the loop only plans + ingests)
Per `docs/security/redteam-runbook.md` §2–3, ordered, staging only, modest volume:
1. Disposable Kali: `docker run --rm -it kalilinux/kali-rolling /bin/bash` → `apt install kali-tools-web sqlmap john seclists`.
2. crt.sh subdomain enum → Burp + **Autorize** (cross-tenant/IDOR) → **JWT Editor** (alg-confusion /
   alg:none / claim-tamper / revoked-expired) → **SQLmap** (authenticated, `--risk=1 --level=2`,
   injection + RLS non-bypass) → **John the Ripper** (argon2 param cert on SAMPLE staging hashes; NEVER prod).
3. Ethics fence: person/social modules OFF (Maigret/theHarvester-person/SpiderFoot-social); own assets only.
Hand each finding back through the finding gate:
```js
import { evaluateRun, classifyFinding } from 'tools/security-redblue/dry-run.mjs';
// finding: { id, class: authz|rls|money|pii|jwt|injection|other, severity, confirmed,
//            disposition: open|guardrail-landed|council-queued|auto-fixed, guardrail, ledger_row }
```

## Finding disposition rules (the gate)
- Confirmed non-red-line → closed ONLY with `guardrail-landed` + `guardrail!=null` + `ledger_row===true`.
- Confirmed RED-LINE (auth/RLS/money/PII/JWT) → terminal disposition is `council-queued` (Triadic
  Council FIRST); `auto-fixed` on a red-line = VIOLATION (council-bypass).
- Blue step executed with `evidence===null` = VIOLATION (fake-green:no-evidence). Unrun step = INCOMPLETE.
- Unconfirmed findings are advisory triage — they never block completion.
- gate is ALWAYS `advisory` — the loop never emits a PASS.

## Lessons
- (build 2026-07-02) The cheatable surface of a SECURITY loop is "silence = secure". The gate must
  distinguish *scanned-and-clean* (evidence present, 0 findings) from *did-not-scan* (no artifact) —
  absence of evidence is never evidence of absence. Encoded as `evidence-not-assumed`.
- (build 2026-07-02) A security loop is most dangerous when it can auto-fix: a botched auth/RLS fix
  ships a hole. Red-line auto-fix is a hard VIOLATION; council-before-redline is enforced in the gate.
- (build 2026-07-02) Keep the offensive capability OUT of the harness entirely (structural, not a
  flag): dry-run Part C asserts NO offensive export exists. `no-autonomous-offense` is a property of
  the code shape, not a runtime toggle.

## Run history
| ts | verdict | blue steps evidenced | findings (confirmed/red-line) | disposed | notes |
|---|---|---|---|---|---|
| 2026-07-02 (cert) | anti-cheat 20/20 | dry-run smoke only | 0 live | n/a | BUILT+CERTIFIED; not yet run against live staging. First live run = a row here. |
