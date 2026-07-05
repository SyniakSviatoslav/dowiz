# S10-PLATFORM-ADMIN / PROVISIONING Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S10 council. Assets, trust boundaries, and
> the failure modes the Rust port must not silently introduce — on the surface where a defect is a
> *privilege escalation* (owner→platform-admin), a *cross-tenant breach* (one tenant's data leaking to
> another), or a *destructive-op misfire* (restore to the wrong target, a tenant provisioned into
> another's data, a backup key in a log). Read alongside `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the four S10 privilege planes (proposal §1/§2) + fold-in of the settled
  controls (B4 platform-admin authz, P6 provisioning/claim), the secrets-exposure-incident (backup key),
  and the cutover-concurrency + Phase-D-irreversibility classes unique to the LAST strangler flip.
- **Scope note:** the B3 (NOBYPASSRLS) flip and its platform-read path (B4 R1) are **B3-council fixes**;
  recorded here because they change what S10 must hold, but the *fix* lives there (Q4a). The backup CRON
  worker (S8), the owner GDPR export/erase DEFINER fns (S9), and the `cutover_flags` mechanism (the
  harness's own ADR) are OUT of S10 — S10 owns only the admin *triggers*, the acquisition/claim ops, and
  the tenant-lifecycle gates. The DR-drill orchestration + row-180 two-writer are **permanent-Node
  carve-outs inside S10** (proposal §5/§9), not ports.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| A1 | The **platform-admin authority** (`platform_admins` allowlist) | non-tenant, NO-RLS table (GRANT-protected) | The highest privilege in the system; a forged/escalated admin reads and acts across EVERY tenant and triggers DR ops |
| A2 | The **cross-tenant admin reads** (all-`locations` fallback/health, r2-check) | `locations`/`owner_notification_targets` (tenant, RLS) | Reading every tenant's config incl. public phones; the reads that work ONLY via BYPASSRLS today (empty post-B3 without a platform-read path) |
| A3 | The **backup encryption key** (`BACKUP_ENCRYPTION_KEY`/`BACKUP_KEYRING`) | `process.env` ONLY (never Zod schema, never git) | One key from the ciphertext to the entire database; the secrets-exposure-incident is the standing reason it must never be logged/committed |
| A4 | The **DR-drill capability** (`runRestoreVerify` + `DATABASE_URL_ADMIN`) | admin trigger + Node subprocess (`pg_restore`, sandbox superuser-adjacent cred) | A weaponizable, minutes-long, superuser-cred operation; misfire = resource exhaustion or a restore to the wrong target |
| A5 | The **provisioning capability** (shadow-spine + tokens) | `provision_grants`/`provision_shadow` policy + `app.provision_token` GUC | Writing net-new tenants; a defect provisions a tenant into another's data or admits a second concurrent spine |
| A6 | The **ownership-transfer authority** (`claim_transfer` DEFINER + claim tokens) | `claim_invites` + mig-071 DEFINER fn | Transfers a whole tenant to an authenticated owner; a token error grants a tenant to the wrong party (the R3-1 theft vector) |
| A7 | The **ops secret** (`PROVISION_OPS_SECRET`) | `process.env`, `x-provision-ops-secret` header | The sole boundary on the externally-reachable cross-tenant provisioning write front door |
| A8 | The **platform-admin audit trail** (`platform_admin_audit_log`) | non-tenant, NO-RLS table (hashed ip/ua) | The only record of who did what across tenants; write-ahead so a destructive drill is visible before it runs |
| A9 | The **ingested acquisition PII** (`place_raw`, `menu_draft`) | `acquisition_sources` (tenant) | Scraped restaurant data held without consent; must be erasable (born-with-erasure) and never persisted un-stripped (allergen write-strip) |
| A10 | The **Phase-D decommission decision** (the un-cut vine) | the cutover-harness ADR (REV-C10) | The one long-horizon irreversibility: without a dated trigger + owner, the "temporary" Node front-door becomes the permanent incumbent |

## 2. Trust boundaries

- **TB-1 owner → platform-admin (privilege boundary).** An owner JWT authenticates a *tenant* principal;
  platform-admin is a *higher* plane. The boundary is the **`platform_admins` allowlist re-read keyed on
  `OwnerClaims.user_id`** — NOT a role claim (a role claim would put the boundary inside a forgeable 24h
  token). The Rust `Claims` enum stays 3-role; there is no fourth variant to forge.
- **TB-2 unauthenticated → `/api/admin/*` (the plane gate).** The B4 root-hook gates every request whose
  *matched route pattern* is under `/api/admin`. In axum this becomes a nested gated `Router` — the
  boundary is structural (registration outside the router is the only escape), re-proven by a
  sibling-closure test. The raw path is never trusted (case/`%2e`/`%2f`/lookalike).
- **TB-3 external → `/internal/acquisition/*` (the ops-secret gate).** Reachable externally today; the
  ONLY boundary is the timing-safe `PROVISION_OPS_SECRET` compare + fail-closed-404. Deliberately
  decoupled from the platform-admin allowlist AND the dev-login family. Network isolation is deferred
  (Q2a) — so the secret is load-bearing on a public interface.
- **TB-4 claim token → tenant ownership (`/api/claim/*`).** A single-use opaque token transfers a whole
  tenant. The authority is the token + (for web) a bound recipient hash; org/location are derived inside
  `claim_transfer`, never from the request (no IDOR). A token-only invite is refused on the web path
  (`CONTACT_REQUIRED`) — else a leaked token binds ownership to ANY account (R3-1).
- **TB-5 provisioning token → shadow-spine write (`app.provision_token` GUC).** The `provision_shadow`
  policy admits the spine write only when the txn-local GUC matches an unconsumed, unexpired grant. The
  boundary is the policy + the FOR-UPDATE/state-pinned/consume-LAST ordering — a context-free write is not
  admitted (0 rows), and a racing second runner ROLLBACKs.
- **TB-6 admin request → cross-tenant DB read (RLS traversal).** The admin reads cross ALL tenants — a
  boundary that is BYPASSRLS today and must become an explicit platform-read DEFINER/role post-B3 (TB
  currently held only by the pool posture, the un-fixed B3 dependency).
- **TB-7 drill → backup ciphertext + sandbox (the DR boundary).** The drill decrypts with A3 and restores
  to a SANDBOX (`DATABASE_URL_ADMIN`), never prod. The boundary between "drill" and "prod" is the sandbox
  target — the LC7 fix-2 bug (smoke once ran on PROD) is the canonical breach of this boundary.
- **TB-8 stack → stack (cutover) + program → Phase-D (decommission).** During the S10 overlap both stacks
  read the same allowlist (B3-independent, trivially consistent). The *novel* boundary is temporal: after
  S10 flips, the front-door shim must be CUT (Phase D) — the boundary between "temporary vine" and
  "permanent incumbent" is a dated trigger + owner that does not exist yet (REV-C10).

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S10-T1** | **Owner → platform-admin escalation** — an owner acts as an admin across all tenants | Modeling admin as a 4th JWT role (B4 Alt-A), adding a `PlatformAdminClaims` variant, a mint site, or the Rust gate reading `role` instead of the allowlist | RED LINE: 3-role `Claims` enum unchanged (`unknown_role_is_rejected` forbids a 4th); allowlist re-read keyed on `OwnerClaims.user_id`; `platform_admins` SELECT-only to the operational role (self-serve escalation impossible); revoke → next-request-deny. E2E: owner→403 ×6, platform-admin→200 ×6 |
| **S10-T2** | **`/internal/*` exposed / secret leak / brute-force** — mass cross-tenant provisioning via a leaked ops secret | Non-timing-safe compare; the secret in the Zod red-line schema (a prod-offender); the secret in a log; a widened reachability in the port | Carry `crypto.timingSafeEqual` + fail-closed-404 + env-not-schema + rate-limits; a grep-gate that no log prints the secret; the port does not widen `/internal/*` reachability; DEFER network isolation as an owned, threshold-triggered residual (Q2a) |
| **S10-T3** | **Backup key leak** — the AES key reaches a log/commit → one breach from the ciphertext | `BACKUP_ENCRYPTION_KEY` in the Zod schema, in a drill error line, in a Sentry tag, or committed (the secrets-incident replayed) | Carry env-only reads + `redactPII` on every drill line + truncated Sentry tags; a grep-gate proves no log prints the key / R2 secret / `DATABASE_URL_ADMIN`; never in git |
| **S10-T4** | **Cross-tenant admin abuse / empty recovery** — an admin read leaks/branches tenants, OR returns 0 rows at incident time post-B3 | A per-tenant branch inside an admin handler (BOLA); the all-`locations` read surviving only via BYPASSRLS with no platform-read path built (Q4a) | Uniform platform-admin-only (no per-tenant branch); build the platform-read DEFINER fn/role as a cutover prerequisite; prove `fallback/health` returns fleet rows under NOBYPASSRLS before S10 flips; audit every action with actor_id |
| **S10-T5** | **Restore-to-wrong-target** — a drill/restore hits PROD or resolves the wrong key → garbage/destruction | The drill's smoke pool targeting PROD (LC7 fix-2 bug class); `resolveBackupKey` returning a wrong key; a port that INVENTS a restore-to-prod endpoint | The drill targets `createSandboxDatabase` only (assert the smoke pool's connection string is the sandbox, dropped after); `resolveBackupKey` FAILS LOUD on unknown keyId; checksum the DECRYPTED plaintext vs manifest before restore; **NO restore-to-prod endpoint exists** (a coverage assertion the S10 namespace exposes none) |
| **S10-T6** | **Provisioning a tenant into another's data / theft of ownership** — a spine lands in a claimed tenant, a second concurrent spine, or a leaked token binds ownership to the wrong account | Porting the shadow-spine context-free (no `app.provision_token` GUC), out-of-order (advance before consume), dropping the state-pin, or dropping the `CONTACT_REQUIRED` token-only refusal | Carry the `provision_shadow` GUC + FOR-UPDATE → state-pinned advance → consume-LAST ordering verbatim (a racing runner → ROLLBACK); `erase_shadow_tenant` guards `owner_id IS NULL` (never erases a CLAIMED tenant); `claim_transfer` derives org in-fn (no IDOR); web refuses token-only invites → `CONTACT_REQUIRED` |
| **S10-T7** | **DR-drill weaponization** — repeated heavy drills exhaust resources / hold a superuser-cred connection | Dropping the rate-limit, the single-flight lock, or the kill-switch; re-introducing the release-while-held lock leak (permanent self-DoS) | Carry 3/5min rate-limit + ONE advisory lock held on a dedicated client (unlock-THEN-release on the same session) → 409 + `ADMIN_DRILLS_ENABLED` kill-switch (scopes ONLY the 2 heavy drills; recovery reads never darkened) + write-ahead audit |
| **S10-T8** | **Admin plane-gate bypass** — a sibling/future admin route reaches a handler without the gate | An axum admin route registered OUTSIDE the gated `Router`; a raw-path prefix middleware re-opening the case/`%2e`/`%2f`/`/api/administrators` bypasses | Nest ALL admin routes under ONE gated `Router` (`route_layer`); key on the matched pattern not the raw path; a clippy/test tripwire; **re-prove sibling-closure in Rust** (an ungated `/api/admin` sibling → 403; lookalike NOT gated) |
| **S10-T9** | **Audit trail loss / PII in audit** — a destructive drill leaves no trail, or raw ip/ua is stored | Writing the audit inside the drill tx (rolled back on failure → no trail); storing raw ip/ua/user-agent | Carry the write-ahead `started` row in its OWN committed tx BEFORE the drill (`auditStart`); hashed ip/ua only (`sha256`); best-effort read audit that never fails the read |
| **S10-T10** | **Premature / unauthorized S10 flip → Phase-D never fires** — S10 flipped without DoD, OR flipped and then the Node vine is never cut | Flipping `cutover_flags` without `readiness_ok`/sign-off; deferring Phase-D indefinitely (the un-cut vine) | `cutover_flags` FORCE RLS + platform-admin policy + `readiness_ok` gate + operator sign-off (harness T11); every flip audited (`updated_by`); **a dated Phase-D trigger + named owner recorded in the ADR** (Q5b) so "temporary" cannot silently become permanent |

## 4. What the B3 RLS flip changes for S10

- **Today (BYPASSRLS):** the platform-admin gate is already B3-independent (non-tenant no-RLS
  `platform_admins`, a plain point-read). The **provisioning writes already traverse RLS correctly**
  (`provision_shadow` policy + `app.provision_token` GUC; `claim_transfer`/`erase_shadow_tenant` DEFINER
  carve-outs) — this half of S10 is B3-safe by construction. **The danger is the cross-tenant admin
  READS** (`fallback/health`, `r2-check`): they "work" today only because the pool is BYPASSRLS — the
  isolation question is masked exactly as the S3 anonymizer-N1 / S5 order-create masking, but on the
  highest-privilege plane.
- **Post-flip (NOBYPASSRLS):** the all-`locations` reads return **0 rows** unless run via an explicit
  platform-read DEFINER fn/role (B4 R1, **unbuilt**). This is a **HARD cutover blocker** (Q4a), the S10
  twin of B4 R11 (`requireLocationAccess`). The gate and the provisioning writes are unaffected; only the
  cross-tenant reads need the new path.
- **S10's rule:** the admin gate is correct **independent of which pool role is live** (non-tenant no-RLS
  allowlist); the provisioning writes are correct under both postures (they write through a policy, not
  around it); the cross-tenant reads must be re-pathed through a platform-read mechanism BEFORE S10 flips
  onto NOBYPASSRLS — the B3 flip and the Node→Rust flip stay two orthogonal, independently-reversible
  events, with the platform-read path the one bridge that must land first.

## 5. Residual risks (summary for the human)

- **The un-cut vine / Phase-D (S10-T10 / Q5b)** — the one long-horizon irreversibility no per-surface gate
  catches: after S10 flips, the "temporary" Node front-door becomes the permanent incumbent unless a
  **dated cut-trigger + named owner** is recorded NOW (REV-C10). From the inside, "not yet" and "never"
  are indistinguishable without a pre-committed trigger. **The most likely counsel flag.** Owner:
  **OPERATOR (the un-cut-vine owner)** — the closing act of the rebuild; the human fills the name + date.
- **The cross-tenant read B3 blocker (S10-T4 / Q4a)** — the platform-read path is unbuilt; without it the
  admin recovery reads empty at incident time. A cutover blocker, not a build blocker. Owner: architect +
  B3 owner (build R1) + S10 lead (gate the flip on it).
- **`/internal/*` external reachability (S10-T2 / Q2a)** — a cross-tenant write front door on one shared
  secret, network isolation deferred (B4 Alt-C, R10). Acceptable ONLY as an explicit, owned,
  threshold-triggered defer, never by silence. Owner: operator (threshold) + S10 lead (no-widen invariant).
- **The ownership-transfer "needs work" residual (S10-T6 / Q2b)** — P6 deferred transfer hardening; the
  `CONTACT_REQUIRED` theft guard is the live control, but the residual must be an explicit accepted-risk,
  not re-shipped by silence through the rewrite. Owner: operator + S10 lead + a future claim-hardening
  council.
- **No restore-to-prod endpoint (S10-T5 / Q3a)** — the real DR restore is a manual runbook; the port must
  NOT invent an authenticated restore-over-prod trigger (a new weaponizable, irreversible capability). The
  gap is accepted, not filled. Owner: operator (runbook posture).
- **The axum plane-gate mechanism (S10-T8 / Q1a)** — the one part of S10 that is NOT a verbatim carry
  (Fastify's runtime hook → Rust's structural router nesting); the sibling-closure property must be
  re-proven in Rust. **The most likely breaker escalation** — the council should have the breaker attack
  the escape (a sibling, a normalized-path trick). Owner: S10 lead + breaker.

**Most of S10's failure modes are NOT introduced by the rewrite** — the platform-admin gate, the DR-drill
hardening, the provisioning RLS ordering, the claim theft guard, and the backup-key posture are all
**current controls (B4/P6/secrets-incident) the port must carry VISIBLY** (matrix row + test). The
rewrite's genuinely *new* risks are three: **(1)** the axum plane-gate mechanism (S10-T8 — no verbatim
carry), **(2)** the un-fixed cross-tenant-read B3 blocker surfacing at the flip (S10-T4), and **(3)** the
Phase-D irreversibility that only the LAST surface's completion exposes (S10-T10). **Breaker-escalation
candidate: the plane-gate escape (S10-T8).** **Counsel-flag candidate: the un-cut vine (S10-T10 / Q5b)** —
deferring Phase-D indefinitely re-plants the exact lock-in the rebuild exists to escape; acceptable only
as a dated, owned trigger.

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1a
axum-plane-gate / Q2a `/internal` network boundary / Q2b ownership-transfer residual / Q3
restore-runbook + backup-key / Q4a platform-read path (B3 blocker) / Q5b Phase-D decommission owner).
**packet-status: 🟡 DRAFT.**
