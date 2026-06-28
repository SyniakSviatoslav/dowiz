# Triadic Council Verdict — P6-2 (shadow-spine write via one-time provisioning token)

**Date:** 2026-06-28 · **Seats:** system-architect · system-breaker · counsel · all grounded against live source.
**Stage:** Council-light on a 🔴 RLS red-line, design-only (no code, no migration placed).
**Proposal under review:** Places → spine via a single-use provisioning token + narrow additive RLS policy
(operator decision 1b, `p6-operator-decisions.md`). Scope: write `organizations` (owner_id NULL) +
`locations` (status='closed', published_at NULL) + `menu_versions` v1 — **no products, no LLM** — then advance
`acquisition_sources → spine_provisioned`. Plus GO-exception (a): noindex + sitemap exclusion.

## CONSOLIDATED VERDICT: **APPROVE-WITH-CONDITIONS**
The mechanism is sound: an **additive permissive** `FOR INSERT … WITH CHECK` policy gated on a hashed
one-time-token GUC, OR-ed with the existing tenant policies, written in **one transaction with consume-LAST**.
Confirmed against live source: permissive-OR does **not** weaken tenant isolation; pgcrypto `digest()` is
present (`1780310044710:8`); the "inert today / enforced only under a future NOBYPASSRLS lock" framing is
**honest, not overclaimed** (counsel scored agent-health GREEN on exactly this self-disclosure). Ship only
after the blocking conditions below. **Two genuine holes** must be designed out first (H1, H2/C1).

---

## 🔴 BLOCKERS (must be designed out before P6-2 code)

### B1 — Dedup anchor is on a table the provisioning write never touches (breaker H1, CRITICAL-of-set)
The claimed backstop "`place_id UNIQUE` on `acquisition_sources` + whole spine in one tx" is **inert**: the
provisioning tx writes `organizations`/`locations`/`menu_versions` — none carry `place_id` or
`UNIQUE(acquisition_source_id)` — and (as specified) doesn't even touch `acquisition_sources`. Mint returns a
token "once" but nothing enforces **one grant per `acquisition_source_id`**; mint twice → two valid grants →
two complete shadow spines for one place_id (consume-LAST only serializes *one shared* token, refuted as the
guard for the *multi-grant* path). **Fix:** the dedup anchor must be a guarded conditional transition **inside
the same tx** — `UPDATE acquisition_sources SET state='spine_provisioned', <spine fks> WHERE id=$src AND
state='places_fetched'` with `rowCount=1` required (else ROLLBACK) — and/or `UNIQUE(acquisition_source_id)`
on the grant and on the spine link. The state-advance must NOT be a separate post-COMMIT tx (crash-replay
re-provisions).

### B2 — noindex does NOT stop the OG identity unfurl (breaker H2 + counsel C1 — same hole, two lenses)
GO-exception (a) governs **search indexing**, not **social unfurl** or **direct-URL human render**. The
bot/SSR path `renderMenuPage` (`ssr-renderer.ts:242-332`, routed by `isBot()` in `ssr.ts:17-26`) and the
human `serveSpaShell` (`spa-shell.ts:64-90`) both emit the **real restaurant name + logo OG tags for any
slug with zero `status` check** and no `noindex`. So pasting a shadow `/s/:slug` into Slack/WhatsApp/etc.
unfurls the unconsented restaurant's identity regardless of noindex — the GDPR-Art-14 / passing-off / "annoyed
owner C&D" exposure. **Counsel's ETHICAL-STOP** fires here: the binding "honest labeling **everywhere it
renders**" guard attaches at the **first human-renderable surface**, which P6-2 creates. **Fix (C1, lifts the
stop):** the closed + `owner_id IS NULL` storefront must **not render the real-name page** (return the
closed/not-found stub, no OG name/logo) to humans **and** to bot/unfurlers until P6-3's labeled
"preview mockup — not a live store" render exists. Apply the `status`-gate + noindex on the **SSR/bot path**
(`renderMenuPage` + `serveSpaShell` SELECT at `spa-shell.ts:113-115`), not only the SPA shell. Slug-obscurity
is not the control.

### B3 — `published_at`-stays-NULL invariant must ship WITH P6-2 (breaker M1 + counsel C3)
P6-2's non-orderability rests **only** on `published_at IS NULL → 409 NOT_PUBLISHED` (`orders.ts:134-137`) —
the *accidental* gate. Decision-3's real `status`-reject does **not exist yet**. Acceptable sequencing **iff**
P6-2 carries a written guardrail: **no shadow row ever gets `published_at` set** until BOTH decision-3's
`status`-reject is proven red→green AND the labeled render (B2) exists. Without it, a later "just publish it
for the demo" silently reopens the council's CRITICAL anonymous-real-order hole.

### B4 — Provisioning rides the dev-login backdoor flag (breaker H3)
Mint + provision routes ride `/api/dev` → require `ALLOW_DEV_LOGIN==='true'` + `DEV_AUTH_SECRET`
(`dev-guard.ts:30-62`). Prod sets neither → **provisioning is unreachable in prod**, yet that's the target.
Flipping `ALLOW_DEV_LOGIN=true` to run it **re-arms every other `/api/dev` minter under the same single
secret** — including `mock-auth` (the CRITICAL owner-JWT backdoor, ADR-0003) + `seed-data`. **Fix:** P6-2's
ops surface needs its **own** internal-auth gate (separate secret / separate guard), decoupled from the
dev-login family, before it can run in prod safely. (On M0/staging the dev-guard is acceptable.)

---

## 🟠 FIX-CONDITIONS (carry into the build; not standalone blockers)

- **C-arch-1 — pre-gen UUIDs, no `RETURNING` on `organizations`.** `organizations` has no SELECT policy
  (`core-identity.ts`), so `INSERT … RETURNING id` fails under enforced RLS. Mirror `onboarding.ts:74,80`
  (`crypto.randomUUID()` in app code). Policy stays `FOR INSERT` (no read surface).
- **C-arch-2 — `menu_versions` needs its own `provision_shadow` policy.** It has FORCE RLS
  `tenant_isolation` (`1780338982018:11-15`) and the spine writes it. No `owner_id`/`status` discriminator →
  token-only WITH CHECK (SQL in the architect verdict).
- **C-arch-3 — schema-qualify `digest()` + prove inert-for-tenant.** The additive `WITH CHECK` injects
  `digest()`/`current_setting` into the org/location **hot path** (owner onboarding). search_path fragility is
  known (`verify:rls` fails; 13 DEFINER fns missing search_path). Schema-qualify the digest call and prove the
  policy errors *nothing* for `owner_id IS NOT NULL` inserts under a real FORCE-RLS role (breaker M2:
  break-by-error, not just widening).
- **C-arch-4 — single BEGIN/COMMIT mandatory + `FOR UPDATE` on the grant.** `set_config(...,true)` is
  txn-local; this codebase runs autocommit by default (`storefrontService.ts:11-14`). One tx must enclose
  `set_config → SELECT … FOR UPDATE (0 rows→ROLLBACK) → inserts → state-advance → consume-LAST`. Consume in a
  separate tx breaks atomicity and reopens B1 (breaker M4).
- **C-sec-1 — GUC plaintext leak is bounded, state it.** `set_config('app.provision_token', <plaintext>)`
  puts the token on a statement that PG/Supabase logs may capture; hashing-in-policy protects only at-rest.
  Blast radius bounded (ops-only role, shadow rows only, single-use ~5min TTL). Either mint without a logged
  plaintext statement, or record the accepted bounded risk explicitly (breaker M3).
- **C-eth-1 (C2) — day-one hard-delete.** Erasure path (org+location+menu_versions+provision_grants by
  `acquisition_source_id`) must be **born with** the artifact at P6-2, per the access_requests day-one-DELETE
  precedent. Operator-triggerable hard-delete (the owner UI button can wait for P6-3).
- **C-eth-2 (C5, recommend) — retention TTL on unclaimed shadow tenants.** Self-hard-delete after N days —
  closes the storage-limitation (Art 5(e)) gap for a never-claimed, never-told restaurant's row.
- **C-ops-1 — reaper named (Q6/L1).** `CREATE INDEX … ON provision_grants(expires_at) WHERE consumed_at IS
  NULL` + a concrete periodic sweep (existing pg-boss/reconcile cron) `DELETE … WHERE expires_at < now() -
  interval '1 day'`, plus a stalled-source metric (no silent MANUAL_REVIEW sink).

---

## PROOF GATE (the only artifact that proves the policy)
Red→green RLS test under an **explicit NOBYPASSRLS / FORCE-RLS test role** (NOT the live operational role,
which bypasses RLS today — `db/index.ts:34-38` blocks only the literal `postgres`). Must prove: (a) token
write admitted; (b) same-role write **without** token rejected; (c) tenant-context write unaffected;
(d) **second use of the same token rejected** (consumed); (e) expired token rejected. Ledger row, red→green.

## Corrections to the proposal (factual, against live source)
1. **Sitemap leak is already false for P6-2:** `getActiveLocations` filters `has_products`
   (`seo.ts:90,123`); a P6-2 shadow has zero products → already excluded. `status<>'closed'` exclusion is
   **forward-necessary for P6-3** (once products land), not a P6-2 fix. `owner_id` is on `organizations`, not
   queried here → don't add it to seo.ts.
2. **pgcrypto present** (`1780310044710:8`) — `digest()` available; Q resolved.
3. **Consume-LAST is forced, not preferred** — consuming first makes the subsequent spine INSERTs fail their
   own policy.
4. Migration head after P6-1 (068) = **069**.

## Refuted attacks (do NOT break it)
Q2 accidental widening (tenant insert sets `owner_id=userId` + never sets the GUC); Q3 NULL/empty-GUC
collision (`digest(NULL)`→NULL; `digest('')` can't collide with a 32-byte-random hash); Q3 recursion/perf
(subquery hits only `provision_grants`, 3 evals/spine); same-token double-spine (consume-LAST + row-lock
serializes). The *multi-grant* path (B1) is the live hole, not these.

## Steel-man on record (non-blocking)
Counsel notes the operator-rejected **`search_path`-pinned SECURITY DEFINER fn** would be operationally
simpler (no grants table, no GUC log-leak, no consume race, no reaper) and its incremental safety loss vs the
token is only realized *after* a role-lock that isn't scheduled. Operator chose the token (RLS-through,
single-use, auditable) — recorded so the added moving parts are tracked as real cost, not free.

---

## Decision gate
P6-2 proceeds to code once: **B1–B4 designed out**, the fix-conditions folded into the build, the day-one
hard-delete (C2) + published_at-NULL guardrail (B3) committed alongside, and the NOBYPASSRLS red→green test
is the proof artifact. ETHICAL-STOP (B2/C1) lifts the moment the no-human-real-name-render condition is
adopted. **Awaiting operator GO on this hardened plan before any P6-2 code or migration 069.**
