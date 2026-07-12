# security-redblue — the blue-team + red-team security loop (charter)

> A **two-arm** self-security loop over dowiz's **own application layer**. One arm (BLUE) is
> autonomous, safe, and in-harness. The other (RED) is **human-gated, disposable-Kali-only** — the
> loop **orchestrates and ingests** it but **never fires attack traffic**. Findings are **advisory
> forever**: every confirmed finding becomes an ordinary red→green guardrail + ledger row, and every
> auth/RLS/money/PII finding goes to the **Triadic Council before any fix**.

- **Loop card:** [`loops/security-redblue.yaml`](../../loops/security-redblue.yaml)
- **Verification report (M1–M11):** [`loops/reports/security-redblue-0.1.md`](../../loops/reports/security-redblue-0.1.md)
- **Anti-cheat harness:** [`tools/security-redblue/dry-run.mjs`](../../tools/security-redblue/dry-run.mjs) (20/20)
- **Memory:** [`loops/memory/security-redblue.md`](../../loops/memory/security-redblue.md)
- **Grounded in:** [`redteam-toolset-analysis-2026-07-02.md`](./redteam-toolset-analysis-2026-07-02.md) ·
  [`redteam-runbook.md`](./redteam-runbook.md) · CLAUDE.md Self-improvement loop §1/§4/§6 ·
  [`model-calibration.md`](../governance/model-calibration.md) · living-loop-system-v3 §5

---

## 1. Why two arms

dowiz owns **code and data, not infrastructure** (Fly edge/VMs, Supabase host/VPC, Cloudflare R2 are
third parties — off-limits by AUP + law). So the only legitimate self-red-team target is the
**application layer over HTTPS at dowiz's own hostnames, on staging**: authz/RLS enforcement,
injection defenses, JWT invariants, WS-token exposure, secret/PII egress, auth rate-limiting.

That surface has two natures, so the loop has two arms:

| | **BLUE arm** | **RED arm** |
|---|---|---|
| **Nature** | static + read-only + advisory replay | active offensive probing |
| **Runs** | **autonomously, in-harness** | **human, on a disposable Kali VM** |
| **The loop's role** | executes it | **orchestrates** (plan) + **ingests** (findings) — never fires |
| **Target** | own code/env + own staging hostnames | own staging hostnames only |
| **Output** | evidence artifacts + advisory signals | findings (each → guardrail/council) |

The hard line between them is the loop's central safety property: **`no-autonomous-offense`** — the
harness never generates attack traffic and never installs offensive tooling. The anti-cheat harness
enforces this structurally (Part C: no offensive export exists).

## 2. BLUE arm — the autonomous safe sweep

Everything here is read-only or advisory; it mutates only local scratch and the ledger rows a human
lands. Each step must produce a **captured evidence artifact** — a step with no artifact is
`fake-green:no-evidence` (a RED violation), never "clean".

1. **Static security review** — `npx eslint .` (tools/eslint-plugin-local: test-integrity + local
   security-class rules), plus targeted grep/read of the authz/RLS/secret/PII surfaces. The lint
   surface is also where red→green guardrails get authored.
2. **Asset-surface scout** — `node scripts/asset-surface-scan.mjs` (read-only crt.sh CT diff of
   `%.dowiz.*`). A **NEW subdomain** since baseline is the signal (a forgotten staging/preview host).
   Emits a `plane-telemetry` `scout` event.
3. **Upstream advisory watch** — `node scripts/scout-feeds.mjs` (release/advisory feeds of shipped
   deps incl. `argon2`, `jose`, `pg` — the red-line-adjacent ones). Advisory.
4. **Dependency audit** — `pnpm audit` (advisory; feeds the scout triage).
5. **Invariant backstops** — `pnpm run verify:rls` (FORCE / NOBYPASSRLS / WITH-CHECK DB backstop),
   `verify:secrets`, `verify:privacy` (PII-leak detector), `verify:env`. No DB env → honest
   `skipped-no-env` (blocks completion → INCOMPLETE), **never a faked pass**.
6. **Security-E2E replay** — `pnpm exec playwright test` over `admin-platform-authz`,
   `courier-room-authz-isolation`, `flow-security-contracts`, `flow-security-regression-2026-06`
   against staging — proves the shipped authz/RLS/JWT guardrails still hold.

## 3. RED arm — human-gated, orchestrated + ingested (never fired)

The loop **emits the engagement plan** from [`redteam-runbook.md`](./redteam-runbook.md) §3 and
**ingests results** through the finding gate. It does **not** run any of the tools below.

- **Workstation:** disposable Kali (`kalilinux/kali-rolling`, `--rm`) or a snapshot-revert VM —
  never 40 installs on the build box.
- **Ordered engagement (staging, modest volume):** crt.sh subdomain enum → Burp + **Autorize**
  (cross-tenant/IDOR) → **JWT Editor** (RS256 invariants: alg-confusion / `alg:none` / claim-tamper /
  revoked-expired) → **SQLmap** (injection immunity + RLS non-bypass, authenticated) → **John the
  Ripper** (one-shot argon2 param certification on sample staging hashes — never prod, PII red-line).
- **Ethics fence:** **no person-profiling** — Maigret + theHarvester person-harvest + SpiderFoot
  social modules stay OFF; scoped to `*.dowiz.*` owned assets only.

Each finding is handed back to the loop via `intakeFinding`/`evaluateRun`
(`tools/security-redblue/dry-run.mjs`), which grades its disposition.

## 4. Authority boundary — findings are advisory forever

A red-team (or blue) finding is **not a special class**; it feeds the exact same self-improvement
loop as every other fix (runbook §5). The loop **informs**; deterministic guardrails / tests / the
human **decide** (memory-corpus pattern #4).

- **Every confirmed finding → red→green guardrail + `docs/regressions/REGRESSION-LEDGER.md` row.**
  A finding is "closed" **only** with a landed guardrail (`disposition==='guardrail-landed'`,
  `guardrail!=null`, `ledger_row===true`). Claiming closed without it is `closed-without-guardrail`
  (a RED violation).
- **Red-line findings (auth / RLS / money / PII / JWT) → Triadic Council BEFORE any fix.** Never
  auto-fixed — `auto-fixed` on a red-line is `council-bypass` (a RED violation). Terminal
  disposition for a red-line is `council-queued`.
- **The loop is never a gate.** `evaluateRun().gate` is always `"advisory"`. A finding only ever
  influences a gate by *graduating into a deterministic guardrail* — and by then it is a test, not
  the loop. This mirrors the calibration ledger's advisory-forever constraint
  ([`model-calibration.md`](../governance/model-calibration.md) §3).

## 5. Telemetry + calibration

Per living-loop-system-v3 §5 and [`model-calibration.md`](../governance/model-calibration.md):

- **Emit per step:** `node scripts/plane-telemetry.mjs emit --kind scout|probe --outcome … --emitter
  security-redblue …` (advisory tags only — telemetry is **never** a gate).
- **Predict-before / resolve-after:** each run declares **DoD separately from METHOD with a named
  FALLBACK**, then `predict` before probing and `resolve` after — read the gap as calibration, not a
  score. (DoD = evidenced posture + disposed findings; primary method = in-harness blue + Kali red;
  fallback = manual runbook steps + scripted in-harness assertions — DoD unchanged, method swapped.)
- **Finalize:** the harness always prints the full §5 LOOP REPORT and writes a lossless per-run
  record to `loops/runs/` + a trend line to `loops/runs/metrics.jsonl`.

## 6. Exit conditions (all must hold)

A run finishes when **(a)** every declared blue step is executed with a captured artifact (or an
honest `skipped-no-env`), **(b)** the red plan is emitted and every supplied finding is ingested, and
**(c)** every confirmed finding has a terminal disposition. It then lands in exactly one verdict:

- **ADVISORY-COMPLETE** — all steps evidenced + all confirmed findings terminally disposed.
- **INCOMPLETE** — an unrun step or an undisposed confirmed finding → carried forward.
- **RED:violation** — `fake-green:no-evidence` / `closed-without-guardrail` / `council-bypass` →
  STOP + escalate to the operator; the loop must never self-clear a violation.

No "until it looks secure". `gate` is always `advisory`.

## 7. Certification

Full live cert needs a human + disposable Kali + staging DB (environment-blocked for an autonomous
run). The **certifiable core** — the finding-gate verdict logic + the blue-arm scout smoke — is
certified hermetically by `tools/security-redblue/dry-run.mjs` (**20/20**), which feeds
known-cheating fixtures and asserts each goes RED/INCOMPLETE, drives the real `asset-surface-scan`
through its fixture seam to prove a planted NEW subdomain is surfaced, and asserts structurally that
no offensive function exists in the harness. See
[`loops/reports/security-redblue-0.1.md`](../../loops/reports/security-redblue-0.1.md) for the full
M1–M11 scorecard.

---

*Cross-references: [redteam-toolset-analysis-2026-07-02.md](./redteam-toolset-analysis-2026-07-02.md) ·
[redteam-runbook.md](./redteam-runbook.md) · [model-calibration.md](../governance/model-calibration.md) ·
[REGRESSION-LEDGER.md](../regressions/REGRESSION-LEDGER.md) ·
[living-loop-system-v3.md](../operating-model/living-loop-system-v3.md)*
