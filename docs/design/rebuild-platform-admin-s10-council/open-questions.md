# S10-PLATFORM-ADMIN / PROVISIONING Port — Council Packet · OPEN QUESTIONS

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S10 platform-admin/provisioning is ported. Each question has
> options + a lane-R3 recommendation — a *starting position for friction*, not a decision. S10 is the
> **highest-privilege plane** (platform-admin above owner) **and the LAST surface** (its flip triggers
> Phase-D decommission). Much of S10 is settled prior law (B4, P6) that this packet PORTS rather than
> re-opens; the live 🔴 questions are the *port mechanism* and the *un-fixed cross-dependencies*. Docs only.

Legend: **[AUTHZ]** privilege/gate · **[SEC]** secret/network/data · **[PROV]** provisioning/ownership ·
**[DR]** backup/restore · **[RLS]** cross-tenant tenancy · **[INFRA]** cutover/decommission ·
**[SCOPE]** surface placement. 🔴 = red-line, operator sign-off required.

---

### Q1 🔴 [AUTHZ] Platform-admin authority — REUSE B4; do NOT invent a 4th JWT role
Authority is a **server-side `platform_admins` allowlist fact**, re-read per request, keyed on
`OwnerClaims.user_id` (`lib/platform-admin.ts:20`). B4's **Alternative A (a 4th `platform_admin` JWT
role) was REJECTED** — it bakes authority into a 24h token, needs a mint site (self-serve escalation),
and ripples the red-line discriminated union.
- **(a) PORT B4 verbatim:** the Rust `Claims` enum stays 3-role (owner/courier/customer — already so, and
  `unknown_role_is_rejected` forbids a 4th); the admin gate = the allowlist point-read → 401/403/**503
  fail-closed**; `revoked_at` re-read = immediate insider-removal; `platform_admins` SELECT-only to the
  operational role (escalation structurally impossible); write-ahead hashed-ip/ua audit. *(recommend)*
- **(b) Add a `PlatformAdminClaims` variant** — REJECTED (this IS B4 Alt-A): a forgeable 24h-lifetime
  privilege at the highest tier, a new mint site, a discriminatedUnion ripple.

**R3 recommendation:** (a). This is settled law; the packet ports it. **🔴** only because the *tier* is the
highest — any drift (a role-based shortcut, a fail-open) is a cross-tenant breach. Owner: architect +
operator (ratify the port carries B4 intact).

### Q1a 🔴 [AUTHZ] The plane-gate mechanism — Fastify's root `onRequest` hook → axum has none
B4's structural authority is a **root-instance `onRequest` hook** keyed on the **matched route pattern**
(`request.routeOptions.url`, immune to case/`%2e`/`%2f`/trailing-slash and the `/api/administrators`
lookalike) — it flows into every route context and gates children/siblings/**future** routes with zero
detection (the boot-guard was deleted as unrealizable). **Axum has no post-routing hook keyed on the
matched template.**
- **(a) Nest ALL `/api/admin` routes under ONE `Router` carrying a `route_layer` (tower middleware = the
  gate); make registration OUTSIDE that router the only escape; add a clippy/test tripwire + a re-proven
  sibling-closure test (an ungated `/api/admin` sibling → 403).** *(recommend)*
- **(b) Per-handler gate extractor** — rejected: a future admin route that forgets the extractor silently
  escapes — the exact failure the root-hook structure was designed to prevent.
- **(c) A raw-path prefix middleware (`starts_with("/api/admin")` on the URI)** — rejected: keys on the
  raw path, re-opening the case/`%2e`/`%2f`/lookalike bypasses B4's matched-pattern predicate closed.

**R3 recommendation:** (a). The port cannot be a verbatim carry — Fastify's runtime hook becomes Rust's
structural router nesting + a compile/test tripwire. **🔴** because a gate-escape at this tier is a
cross-tenant breach; the sibling-closure property must be re-proven in Rust, not assumed. Owner: S10 lead
+ breaker (attack the escape: a sibling, a normalized-path trick).

### Q2a 🔴 [SEC] `/internal/acquisition/*` — reachable externally today; network isolation deferred
The acquisition-ops plane is mounted on the SAME server as the public surface; the **only** boundary is
the `PROVISION_OPS_SECRET` header (timing-safe, fail-closed-404, decoupled from dev-login — `ops-auth.ts`).
B4's Alternative C (network-isolated ops service, mTLS/segmentation) was **rejected as primary, kept as
future defense-in-depth (B4 R10)**.
- **(a) CARRY the secret-gated posture; DEFER network isolation as an owned residual with a named
  threshold** (e.g. first non-founder ops hire / N tenants ⇒ segment `/internal/*`); the port must not
  *widen* reachability, and keeps the rate-limits (30/min mint/spine, 10/min extract). *(recommend)*
- **(b) Add network isolation now** (bind `/internal/*` to a private interface / mTLS) — deferred, not
  rejected: correct long-term, over-engineered for 1–5 operators today; scheduled at the threshold.
- **(c) Fold `/internal/*` behind the platform-admin allowlist instead of a shared secret** — rejected:
  the decoupling is deliberate (enabling provisioning must not require an owner-JWT-minting admin session;
  a cron POSTs the retention sweep with the secret, not a human token).

**R3 recommendation:** (a). **🔴** because `/internal/acquisition/provision/*` is a cross-tenant *write*
front door on one shared secret; a leaked/brute-forced secret is a mass-provisioning capability. The
residual (external reachability) is acceptable *only* as an explicit, owned, threshold-triggered defer —
never by silence. Owner: operator (threshold) + S10 lead (no-widen invariant).

### Q2b 🔴 [PROV] Ownership transfer — P6 flagged "needs work"; carry the DEFINER + the theft guard
`acceptClaim` transfers a shadow via the token-gated DEFINER `claim_transfer` (token = sole authority, org
derived in-fn → no IDOR, no auto-publish); the **web path REFUSES a token-only invite → `CONTACT_REQUIRED`**
(`claim.ts:113`, the R3-1 theft vector: a leaked token-only invite would bind ownership to ANY account).
P6-provisioning memory: **"ownership transfer needs work."**
- **(a) CARRY the DEFINER carve-out + the `CONTACT_REQUIRED` refusal verbatim; surface the P6 "needs work"
  residual as an explicit accepted-risk row with an owner** — do NOT silently re-ship an under-specified
  transfer through the rewrite; do NOT open a transfer-hardening feature inside the port. *(recommend)*
- **(b) Harden ownership transfer now in the port** — rejected: couples a security port to a net-new
  hardening design (recipient-binding model, invite-lifecycle) at the worst moment; it is its own council.
- **(c) Carry the transfer but drop the token-only refusal** — REJECTED: re-opens the R3-1 theft vector.

**R3 recommendation:** (a). The theft guard IS the live control; the "needs work" is a *known, bounded*
residual (what P6 deferred), made explicit + owned, not re-litigated. **🔴** because ownership transfer is
the one place a token error grants a whole tenant to the wrong party. Owner: operator (accept the residual)
+ S10 lead (the seam) + a future claim-hardening council.

### Q2c [SCOPE] Row 60 `onboarding/start` — S2 or S10? Decide before either flips
`onboarding/start` calls `bootstrap_owner()` (SECURITY DEFINER, mints the first membership) — an
auth-shaped op. The map assigns it S10 (REBUILD-MAP lists provisioning under S10) but flags it borderline
S2, "needs explicit council confirmation before either S2 or S10 flips."
- **(a) Confirm S10 ownership** (it is the tenant-provisioning bootstrap, sibling to the acquisition spine)
  — the onboarding/activation lifecycle is a coherent S10 sub-plane. *(recommend)*
- **(b) Reassign to S2** (it mints a membership = an auth operation) — defensible; would pull the
  bootstrap into S2's JWT-parity gate.

**R3 recommendation:** (a), but this is a **council decision that must land before either S2 or S10 flips**
— otherwise the first-membership mint has two divergent implementations across stacks. Not 🔴 (no security
regression either way), but a hard sequencing gate. Owner: architect (S2 lead + S10 lead jointly).

### Q3 🔴 [DR] Backup/restore triggers — no restore-to-prod exists; keep the drill on Node; protect the key
Three admin routes: `GET /backups` (list), `POST /backups/verify` (restore **DRILL** to a **SANDBOX**),
`GET /backups/dr-report` (fleet drill). **There is NO restore-to-prod endpoint** — the real DR restore is
a manual runbook (DB creds + `pg_restore`). `runRestoreVerify` is a 400-line subprocess/stream/crypto
pipeline. Backup keys are env-only + fail-loud-on-unknown-keyId + redacted (secrets-incident).
- **Q3a [DR]:** **(a) DO NOT build a restore-to-prod endpoint in the port** — keep it a runbook; a
  confirmation-gated restore endpoint is its OWN council (double-confirm, target-pin, blast-radius),
  never an S10 side effect. Record the gap as accepted. *(recommend)* — (b) build a confirmation-gated
  restore endpoint now: REJECTED (a new weaponizable, irreversible capability the system deliberately
  lacks).
- **Q3b [DR]:** **(a) Keep `runRestoreVerify` orchestration ON NODE; the Rust admin route is a thin
  authenticated trigger** (gate + write-ahead audit + uuid-validate + rate-limit → invoke the Node drill).
  Permanent-Node carve-out (REV-7). *(recommend)* — (b) re-port the whole pipeline to Rust: REJECTED
  (over-engineering a 2-endpoint cold path; a large new Rust subprocess/crypto/superuser-cred surface).
- **Q3c 🔴 [SEC]:** **(a) CARRY the key posture verbatim** — `BACKUP_ENCRYPTION_KEY`/`BACKUP_KEYRING` from
  `process.env` (not the Zod schema); `resolveBackupKey` FAILS LOUD on an unknown keyId (the
  restore-to-wrong-target control); every drill line `redactPII`'d; a grep-gate proves no log prints the
  key / R2 secret / `DATABASE_URL_ADMIN`. *(recommend)*

**R3 recommendation:** Q3a (a), Q3b (a), Q3c (a). **🔴** on Q3c (the backup key is one leak from the
ciphertext — the secrets-exposure-incident is the standing reason) and on the *whole* of Q3 because DR is
irreversible: a wrong-target restore or a wrong key destroys data. Owner: operator (Q3a runbook posture) +
S10 lead (carve-out + key redaction).

### Q4a 🔴 [RLS] The cross-tenant admin reads — a HARD B3 blocker, unbuilt
`fallback/health` + `r2-check` SELECT over ALL `locations`/`owner_notification_targets` — they work
**today ONLY because the pool is BYPASSRLS**. Post-B3 (NOBYPASSRLS) they return **0 rows** unless run via
an explicit **platform-read mechanism** (a SECURITY-DEFINER fn or a platform-read role — B4 open item R1,
**still unbuilt**; `backup_metadata` has a system policy, `locations` does not).
- **(a) BUILD the platform-read DEFINER fn/role as an S10 cutover PREREQUISITE** (co-owned architect +
  B3 owner); prove `fallback/health` returns fleet rows under NOBYPASSRLS before S10 flips. *(recommend)*
- **(b) Flip S10 onto BYPASSRLS and defer B3 for the admin pool** — rejected: leaves the highest-privilege
  cross-tenant reads on the un-hardened pool posture indefinitely; contradicts the B3 direction.
- **(c) Scope the admin reads per-tenant** — REJECTED: that is the BOLA anti-pattern B4 removed; admin
  reads are cross-tenant by design.

**R3 recommendation:** (a). This is the S10 analogue of B4 R11 (which made `requireLocationAccess` a hard
B3 blocker): **S10 cannot flip onto NOBYPASSRLS without the platform-read path existing**, or an operator's
recovery read returns 0 rows at the worst possible moment (an incident). **🔴** — a cross-tenant read that
silently empties post-flip is both a broken recovery tool and a masked isolation question. Owner:
architect + B3 owner (build R1) + S10 lead (gate the flip on it).

### Q5a [INFRA] The cutover-flags bootstrap — S10 ports the authority the flip mechanism depends on
`cutover_flags` is platform-admin-gated (FORCE RLS + platform-admin policy). S10 ports the very
platform-admin authority the harness flip uses. During the S10 overlap both stacks must agree on
platform-admin authority.
- **(a) No special handling — both stacks read the same non-tenant no-RLS `platform_admins` table
  (B3-independent), so a flip flag written under one stack's gate is honored by the other.** *(recommend)*

**R3 recommendation:** (a). Trivially consistent (the allowlist is stack-agnostic); named so the council
sees the near-circularity (the surface that ports the admin gate is gated, for its own flip, by that gate)
and confirms it is benign. Not 🔴. Owner: S10 lead.

### Q5b 🔴 [INFRA] Phase-D decommission — the un-cut vine (REV-C10). Name a dated trigger + owner NOW
S10 is the LAST surface; its flip means all ten are Rust. Counsel's REV-C10: the front-door shim has **no
dated cut-trigger and no named owner** → the "temporary" vine becomes the permanent Node incumbent (the
exact lock-in the rebuild exists to escape; the handoff's `терпіння↔прив'язаність` open item).
- **(a) Record a dated Phase-D trigger + a named owner NOW in the cutover-harness ADR** — e.g. *"S10
  flipped + all-ten stable ≥ N days ⇒ front-door role migrates to Rust, Node shim deleted, by `<owner>`
  before `<date>`."* The owner is an OPERATOR decision (this agent cannot self-assign it). *(recommend)*
- **(b) Defer Phase-D indefinitely** ("decommission when it feels right") — REJECTED: from the inside,
  "not yet time to decommission Node" and "I never actually intend to" are indistinguishable without a
  pre-committed trigger (counsel REV-C10).

**R3 recommendation:** (a). **🔴** — this is the one long-horizon irreversibility no per-surface gate
catches: a permanent Node incumbent. The packet surfaces the un-cut-vine owner slot; the **human must fill
the name + date.** Owner: **OPERATOR (the un-cut-vine owner)** — the closing act of the whole rebuild.

---

## Decision-ordering note for the council

**Settled prior law that S10 PORTS (not re-opened):** the platform-admin authority + plane-gate + DR-drill
hardening + write-ahead audit (B4); the `provision_shadow` RLS policy + single-use tokens + `claim_transfer`
DEFINER + born-with-erasure (P6). The council's job on these is to confirm the port carries them intact,
not to re-litigate them.

**Port-blocking 🔴 (no S10 code builds before settled):** **Q1a** (the axum plane-gate mechanism — the one
thing that is NOT a verbatim carry) and **Q3c** (the backup-key redaction/env-only/fail-loud posture — a
standing red-line).

**Cutover-blocking, not build-blocking 🔴 (the Rust code can be built + dark-verified first, but the FLIP
cannot happen until these land):** **Q4a** (the platform-read path — a hard B3 blocker; without it the
cross-tenant reads empty post-flip) and **Q5b** (the Phase-D trigger + owner — recorded before S10 flips,
because S10's flip IS the Phase-D trigger).

**Owned residuals, not blockers:** **Q2a** (`/internal/*` network isolation deferred with a named
threshold), **Q2b** (the ownership-transfer "needs work" residual made explicit), **Q3a** (restore-to-prod
stays a runbook). **Sequencing gate:** **Q2c** (row 60 S2-vs-S10) must be decided before EITHER S2 or S10
flips.

**The single most likely breaker escalation:** the **axum plane-gate escape (Q1a)** — a sibling or a
normalized-path trick that reaches an admin handler without the gate, at the tier where that is a
cross-tenant breach. **The single most likely counsel flag:** the **un-cut vine (Q5b)** — deferring
Phase-D indefinitely re-plants the exact lock-in the rebuild exists to escape; it must be a dated,
owned trigger, not a silent "someday."
