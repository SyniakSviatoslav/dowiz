# RLS / Security Gap Analysis vs Existing Plans
## Kernel-Unify + Rewrite-Plan reconciliation (2026-07-15)

### Source of truth
The pasted findings (R1–R13, D1–D7, M1–M6, P1–P6) map 1:1 onto the
repo's own red-team audit `/root/dowiz/docs/red-team/2026-07-13/D2-rls-data-governance.md`,
which is the authoritative, already-approved plan for this surface. The kernel
blueprints (HK-00..06 in `REWRITE-PLAN.md`) deliberately do NOT cover RLS/SQL —
that is a **separate red-line domain** (auth/money/RLS/migrations), gated by
`/council` + `backend-contract-convergence`, never by the kernel autopilot.

### DECISIVE FINDING — the findings describe a DORMANT tier
Commit `e1505e1d` ("chore(declutter C2): quarantine retired Supabase/Fly/RLS stack")
moved the **entire Postgres/Supabase data tier into `attic/`**: `apps-api`,
`apps-worker`, `packages-db` (all 140 migrations), `fly.toml`. Those packages are
git-tracked + reactivatable but **NOT installed, NOT built, NOT deployed** on this
branch. There is **no live server tier** (`grep` for `server/`, `axum`, `tokio_postgres`,
`sqlx`, `rusqlite`, `CREATE TABLE` outside `attic/` → 0 hits), **no live migrations**,
therefore **no live RLS-enforced datastore**.

=> The R1–R13 / M1–M6 / P1–P6 findings are **reactivation gates**, not live
breaches. They MUST be fixed before `attic/` is un-quarantined; they are NOT
exploitable on the current branch because the tables do not run.

### Reconciliation table (pasted finding → plan status)
| Finding | Severity (as pasted) | Plan status in D2 | Live exposure this branch | Action class |
|---|---|---|---|---|
| R1 couriers no RLS + password_hash/PII | HIGH | CONFIRMED (D2 §R1) | 0 (attic only) | reactivation gate |
| R2 telegram_login_tokens no RLS owner tokens | HIGH | CONFIRMED (D2 §R2) | 0 (attic only) | reactivation gate |
| R3 fail-open anonymous policies orders/customers | HIGH | CONFIRMED (D2 §R3) | 0 (attic only) | reactivation gate |
| R4 six tenant tables no RLS | MED-HIGH | CONFIRMED (D2 §R4) | 0 | reactivation gate |
| R5 customer_devices RLS disabled | MED | CONFIRMED (D2 §R5) | 0 | reactivation gate |
| R6 USING(true) policies backup/access | MED/LOW | CONFIRMED (D2 §R6) | 0 | reactivation gate |
| R7 BYPASSRLS hidden in misnamed migration | MED | CONFIRMED (D2 §R7) | 0 | reactivation gate |
| R8 dead RLS CI gate + no boot-guard | HIGH(gate)/LOW(live) | CONFIRMED (D2 §R8) | 0 | restore before reactivation |
| R9 tenant isolation = app-code discipline, not RLS | HIGH(systemic) | CONFIRMED (D2 §R9) | 0 | architectural decision |
| R10–R12 message-bus raw SQL / pool / role | LOW–MED | CONFIRMED (D2 §R10–R12) | 0 (attic only) | reactivation gate |
| R13 no service-role key in client | CLEAN | CONFIRMED (D2 §R13) | n/a — positive | none |
| M1–M3 client-side pricing authority | HIGH | CONFIRMED (D2 §M1–M3) | 0 (no `server/`, WASM-only) | server-reprice before $ flows |
| M4–M6 kernel int-overflow / currency / dead checks | LOW–INFO | CONFIRMED (D2 §M4–M6) | 0 (unit_price=0 stub) | harden kernel |
| P1–P6 GDPR erasure cascade gaps | HIGH | CONFIRMED (D2 §P1–P6) | 0 (anonymizer in attic) | fix anonymizer before reactivation |

### What is MISSING / NOT DONE (vs the pasted list)
1. **Nothing in the pasted list is un-planned** — every item is already captured,
   confirmed, and severity-rated in D2. The pasted list is a faithful extract of D2.
2. **The missing ingredient is the REACTIVATION GATE itself** — D2's fixes are
   written as prose, not as enforced gates. Before `attic/` is restored:
   - restore `verify:rls` as a wired CI script + add a runtime `FORCE ROW LEVEL
     SECURITY` boot-guard (D2 R8).
   - convert every `USING(true)` / no-RLS / fail-open `IS NULL` policy to a
     tenant-scoped predicate (R1–R6, R3).
   - isolate + assert `rolbypassrls = false` on the connected role at boot (R7, R12).
3. **Money authority (M1–M3):** the live path is WASM-only (`Storefront.svelte`
   runs the kernel client-side; `unit_price: 0` stub, so no live theft today). The
   fix (server re-price from trusted catalog) is a `server/` crate that does NOT
   exist on this branch — it is a roadmap item, not a regression.

### Verdict for the kernel-unify autopilot
- The unification work (r1–r5) is **orthogonal** to RLS/SQL: kernels are
  pure-Rust/std, no DB. No overlap, no conflict.
- The RLS/security findings are **out of scope for this autopilot** and correctly
  excluded from HK-00..06. They are tracked, not missing.
- **No code edits made to `attic/` or RLS** — these are operator red-line
  (auth/money/RLS/migrations). Per AGENTS.md they require `/council` + per-change
  confirmation. This document is the gap analysis + reconciliation the operator
  asked for; the fixes are queued as reactivation gates, not applied.

### CORRECTIONS — 2-pass subagent review (supersedes "nothing un-planned", 2026-07-15)
The first pass overstated completeness. A deeper cross-read (deleg_84061e1e)
found real, LIVE, un-planned gaps the attic framing hid:

1. **M1–M6 MONEY INTEGRITY — entirely MISSING (not "attic-only").** These sit on
   LIVE kernel code, not the quarantined tier:
   - M1 client-side authoritative pricing (WASM Storefront) — no server re-price.
   - M2 `kernel/src/domain.rs::place_order` trusts caller `unit_price`, no catalog.
   - M4 `kernel/src/money.rs` unchecked integer math (release wraps).
   - M5 no currency field / mixed-currency guard. M6 dead defensive checks.
   Only safe TODAY because web glue stubs `unit_price:0` (M3). **No ADR/plan
   covers any money finding.** Top genuine gap.
2. **R10/R11 message-bus — LIVE, MISSING.** `packages/platform/src/message-bus.ts`
   has raw-SQL channel-identifier interpolation (latent SQLi) + unpinned pool
   connection. This is live TS, not attic. No plan.
3. **D1-F2 role gate** on courier routes is plan-only-not-done (cheap route-guard,
   deferred). D1-F1 live prod owner cred is OPERATOR (decommission runnable only
   off repo host).
4. **R7** (BYPASSRLS migration hygiene) + **DSAR/export** endpoint — no plan at all.

=> Revised verdict: the pasted list is PARTIALLY planned. RLS/PII/most D-items are
class-covered by ADR-0007/0008/0009 (design-level, code undone). But **money (M1–M6)
+ live message-bus (R10/R11) + D1-F2 are genuinely un-planned and on LIVE code** —
the kernel autopilot's "0 live exposure" claim applies to RLS/SQL only, NOT to the
money engine. Money authority is the actionable next frontier.

### UNWIRED ORGANS — corrected (VertexBridge found, was missed first pass)
- `resonator` (bebop2-core, host-gated) — WIRED via wasm `resonate`; only stranded
  at app level (deleted JS loader). Not dead.
- `living_knowledge` (dowiz kernel) — adapter registered + fail-closed, but the
  JS engine it bridges to (`scripts/lk-bridge.mjs`) is ABSENT; real spike on an
  off-tree branch. Stranded, not dead.
- **`VertexBridge` (dowiz/engine/src/bridge.rs) — REGISTERED but UNWIRED.** This is
  the actual "unwired organ" the directive worried about. `upload_once()` only
  increments a counter; `wgpu` is absent from `engine/Cargo.toml`. The zero-copy
  contract is RED→GREEN tested but models a GPU it never touches. Activation: add
  `wgpu` behind a `gpu` feature + real `queue.write_buffer`. NOTE: this is in the
  `engine` crate, OUTSIDE the kernel/wasm-lib autopilot scope — but it is a real
  gap worth queueing.

### Recommended next (operator decision, not executed)
A) Leave `attic/` quarantined (current state — safe, 0 live exposure for RLS/SQL).
B) When un-quarantining: run D2's fix list as a gated migration wave with
   `verify:rls` + boot-guard, then re-run the red-team probe before first real
   order (G11 GREEN criterion).
C) **Money frontier (new):** draft a server/node-authoritative re-pricer design
   (closes M1/M2); add `checked_*` arithmetic + currency field (M4/M5) to the
   kernel — these are kernel-scope, NOT red-line, and should be planned next.
D) **VertexBridge:** queue GPU activation behind a `gpu` feature (engine crate).
