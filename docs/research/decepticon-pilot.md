# Decepticon defensive red-team pilot — authorized, dowiz-infra-only

**STATUS: SCAFFOLDED — PENDING OPERATOR AUTHORIZATION + RUN.** Out-of-band. Not wired into the app, not
in CI, not a dependency. **Owner:** Operator (must be the authorizing party for dowiz infra).

## What it is
[PurpleAILAB/Decepticon](https://github.com/PurpleAILAB/Decepticon) (Apache-2.0) — an autonomous
**offensive red-team** agent (recon → exploit → privesc → lateral → C2; 16 specialist agents; runs inside
its own Kali sandbox). Piloted ONLY as **authorized defensive red-teaming of dowiz's OWN infrastructure**
(find our holes before an attacker does).

## 🔴 Ethics & authorization (non-negotiable — Ethics Charter + security policy)
- **Authorized, own-infra ONLY.** Decepticon may target ONLY dowiz-owned systems (staging-first), with
  written authorization on file. **Never** another party's systems — that would be an attack, which the
  charter forbids. Purpose is DEFENSE (peace/safety of our users), not offense.
- **Mechanical scope gate (load-bearing).** Before any engagement:
  `node scripts/decepticon-pilot/authorized-scope-attest.mjs <targets-file>` — fail-closed: any
  non-dowiz host → exit 1 (red→green proven). The RoE target list passes ONLY if every host is dowiz-owned.
- **Sandbox isolation.** Decepticon's Kali `sandbox-net` stays isolated from the dowiz management plane;
  run against **staging**, never prod data, with a snapshot/restore plan. Generate its RoE/ConOps/OPPLAN
  package first (the tool does this) and keep it on file.
- **No dowiz secret in the sidecar env** — `node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>`.
- `decepticon` is FORBIDDEN-DEP (out-of-tree only, never a product dependency).

## What it produces
A prioritized findings report against dowiz staging (real attack-chain, not a scanner) → each confirmed
finding becomes a fix + a red regression test. Promotion of anything beyond a one-shot authorized
engagement = a SEPARATE decision.

## Run
```
node scripts/decepticon-pilot/authorized-scope-attest.mjs targets.txt   # must exit 0 (dowiz-owned only)
node scripts/skyvern-pilot/no-credential-attest.mjs sidecar.env         # must exit 0
# then run Decepticon out-of-tree against the authorized staging targets; triage findings → fixes+tests.
```
_Findings: (operator fills on an authorized run)._
