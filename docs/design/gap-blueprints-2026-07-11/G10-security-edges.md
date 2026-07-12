# G10 — Known Security Edges (none closed, some forgotten since 2026-06-27)

> Research + execution blueprint. **Read-only research; no code changed, no DB touched, no git
> mutation.** Ground truth taken from the CURRENT tree (`feat/paleo-dinosaur-digs`), migrations,
> deny.toml, Cargo.lock, and the living-memory corpus. Every line/commit citation was re-verified
> in this pass. Author date 2026-07-11.

Source gap: `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` §3 (last bullet), §7.9,
§9 rec 10. The 06-27 "deeper security (lower pri)" trio (items 1-3) was flagged in
`memory/full-project-analysis-2026-06-27.md:131-132` and then absent from every later
memory/ledger entry for two weeks — the *forgetting*, not the vulns, is the primary defect this
blueprint closes (see §4 step 0 + §6).

---

## 1. Gap & evidence

Seven carried security edges, none closed:

| # | Item | Where (verified this pass) | Scope | First flagged |
|---|------|----------------------------|-------|---------------|
| G10-1 | Courier hash same-input "collision" | `apps/api/src/routes/courier/auth.ts:243-244,249` | prod (Node) | 2026-06-27 |
| G10-2 | CORS wildcard on `POST /api/orders` | `apps/api/src/server.ts:149-156` | prod (Node) | 2026-06-27 |
| G10-3 | pii-cipher has no key rotation | `apps/api/src/lib/pii-cipher.ts:1-19` | prod (Node) | 2026-06-27 |
| G10-4 | RUSTSEC-2023-0071 rsa Marvin | `rebuild/Cargo.lock:2452`; `deny.toml:43-50` | rebuild (Rust), NOT prod | 2026-07-06 |
| G10-5 | Yanked num-bigint 0.4.7 | `rebuild/Cargo.lock:1811`; `deny.toml:43-50` | rebuild (Rust), NOT prod | 2026-07-06 |
| G10-6 | B3 NOBYPASSRLS flip still open | `packages/db/src/index.ts:32-39`; memory pg-privilege | prod (Node) | 2026-06-29 |
| G10-7 | OR-1: is the live operational pool BYPASSRLS? | inference from G10-6 evidence | prod (Node) | 2026-07-02 |

**Load-bearing scoping fact:** G10-4/G10-5 live only in `rebuild/` (Rust), which is **0% cut over
in prod** (audit §8: "Prod: 0 routes on Rust"). Prod push/JWT run on the Node stack, unaffected by
these Rust advisories. They are **pre-cutover gate blockers**, not live-prod exposure. G10-1/2/3/6/7
are in the live Node product.

---

## 2. Research findings — per-item ground truth & severity

### G10-1 — Courier hash same-input "collision" — LOW (correctness / DoS, not takeover)

Verbatim (`courier/auth.ts:241-250`):
```
const emailHash = crypto.createHash('sha256').update(emailOrPhone).digest('hex');
const phoneHash = crypto.createHash('sha256').update(emailOrPhone).digest('hex');   // identical value; UNUSED
... SELECT id, password_hash, status FROM couriers WHERE email_hash = $1 OR phone_hash = $1  // $1 = emailHash only
```
It is **not** a cryptographic hash collision. Two facts combine:
- `emailHash` and `phoneHash` are computed identically from `emailOrPhone`; only `emailHash` is
  bound (`[emailHash]`). `phoneHash` is dead code — a smell that reveals intent-drift (author meant
  two params for a unified email-or-phone login field, but the single field makes them equal).
- The lookup is `email_hash = $1 OR phone_hash = $1`, returns `rows[0]` with **no `ORDER BY`/`LIMIT`**.
  Schema (`migrations/1780421029538_couriers.ts:8-10`): `email_hash text NOT NULL UNIQUE`, but
  `phone_hash text` is **nullable and NOT unique**. Register schema (`auth.ts:39`):
  `phone: z.string().optional()` with **no format validation** → phone accepts an arbitrary string,
  including another courier's email string → email/phone namespaces overlap.

**Collision vector:** an invited courier registers `phone` = a victim courier's login-email string.
Their `phone_hash = sha256(victimEmail)` now equals the victim's `email_hash`. On the victim's next
login, `WHERE email_hash=$1 OR phone_hash=$1` matches **both rows**; `rows[0]` is nondeterministic.
Password verify still runs against `rows[0].password_hash`, so:
- attacker→victim takeover: **impossible** (attacker's own password only unlocks the attacker row).
- victim login: may resolve to the attacker row → victim's correct password fails → **targeted
  login DoS / nondeterministic account resolution**.

**Exploitability caps:** courier registration is **invite-gated** (`auth.ts:55-58`, `courier_invites
… FOR UPDATE`) — attacker must be an owner-issued (semi-trusted) courier; must know the victim's
exact email string; yields only DoS/ambiguity within one tenant's courier set.
CVSS-style: `AV:N/AC:H/PR:L/UI:N/S:U/C:N/I:N/A:L` ≈ **3.1 LOW**. Defense-in-depth / correctness bug.

### G10-2 — CORS wildcard on `POST /api/orders` — LOW (largely inert; policy smell)

Verbatim (`server.ts:142-156`): the `@fastify/cors` plugin default-denies cross-origin
(`origin: false`, `credentials: false`); a separate `onRequest` hook then sets
`Access-Control-Allow-Origin: *` when `url.startsWith('/api/orders') && method === 'POST'` (also for
`/public/locations/` and `/s/`).

**Why the wildcard is mostly defanged (the honest nuance):**
- The ACAO:* header is gated on `method === 'POST'`, so the CORS **preflight** (`OPTIONS`, method
  ≠ POST) never receives it → the plugin answers the preflight with `origin:false`. A cross-origin
  browser JSON POST is **non-simple** (application/json always preflights) → **blocked before the
  real request fires**. A `text/plain` "simple" POST dodges preflight, but Fastify won't JSON-parse
  it → Zod 400.
- The endpoint is **already public + unauthenticated** (order create needs no auth — 06-27 memory);
  server-side/curl POSTs ignore CORS entirely, so the wildcard adds ~nothing to spam surface.
  Rate-limit (100/min/IP, LEDGER #20 `IP_THROTTLE`) is the real spam control.
- App is **bearer-only / cookie-less** (LEDGER #55) → no ambient credentials for CSRF to abuse.

Net: enables cross-origin *reading* of a *simple* request's response only — and a JSON order-create
isn't simple. Effective attack surface ≈ informational. CVSS-style ≈ **2-3 LOW**. The `*` is
over-broad policy hygiene worth tightening (cheap), not a live exploit.

### G10-3 — pii-cipher has no key rotation — LOW latent / MEDIUM-if-key-compromised

`pii-cipher.ts`: AES-256-GCM, single key from `COURIER_PII_ENCRYPTION_KEY` (base64→32B), cached on
`globalThis`. Ciphertext layout = `[IV(12)][ciphertext][authTag(16)]` — **no key-id/version byte**.
A legacy-base64 detector exists (`:33-36`) but there is **no key versioning**. Consequences:
- Rotating the env var makes **every existing blob fail GCM auth** (undecryptable) → data loss;
  there is no way to know which key encrypted a given blob → no lazy/keyed re-encryption possible.
- **Blast radius is small** (good news): `encryptPII()` call sites are only
  `couriers.{email_encrypted, phone_encrypted, full_name_encrypted}` (register at `auth.ts:79-85`
  + one synthetic in `dev/mock-auth.ts:483`). One table, ~3 columns.

Rotation requires either (a) a **versioned format** — 1-byte key-id prefix + a keyring (id→key map):
decrypt selects by id, encrypt uses "current" — plus a background re-encrypt/backfill; or (b)
**envelope encryption** (per-record DEK wrapped by a rotatable KEK) for rotation-without-rewrite.
No attacker exploit today (operational/compliance capability gap). If the key is *ever* suspected
compromised, the inability to rotate without downtime + a re-encrypt migration becomes a real
incident-response gap. Severity: **not a CVSS vuln** — operational risk, LOW→MEDIUM conditional.

### G10-4 — RUSTSEC-2023-0071 (rsa Marvin) — MEDIUM in the abstract / negligible in-context; rebuild-only

`rsa 0.9.10` (`Cargo.lock:2452`). Advisory = Marvin key-recovery **timing side-channel** on PKCS#1
v1.5; **no fixed upstream version**. `deny.toml:43-49` documents the exact chain:
`web-push → jwt-simple → superboring → rsa` (prod path in the Rust rebuild) — and rsa is *also* a
dev-dependency (test RSA-2048 keygen, `crates/api/Cargo.toml:147-150`).
- **Not in prod**: prod push runs on npm `web-push` (Node); this Rust chain ships only when a Rust
  surface cuts over (0% today). So it's a **cargo-deny Gate-3 RED / pre-cutover blocker**.
- **Effective exploitability even in the Rust stack ≈ negligible**: Marvin needs an *adaptive
  RSA-decryption timing oracle* over many queries. VAPID web-push signs with **ECDSA P-256**, not
  RSA decryption; the rsa code path is present-but-not-exercised by VAPID signing. No server-side
  RSA decryption oracle is exposed.
CVSS-style: advisory ≈ **5.9 MEDIUM**; effective-in-this-deployment ≈ **LOW/negligible**.
Remediation (EXPANSION-PLAN Layer 0.6): swap the web-push/JWT crypto path so the graph stops pulling
rsa — build the VAPID JWT via `p256`/`ecdsa` + `jsonwebtoken` ES256, or adopt a maintained push
crate without the `jwt-simple→superboring→rsa` chain.
**Already deterministically tracked** (deny.toml `ignore=[]` — the gate goes RED, not rubber-stamped)
— this is NOT the "silently forgotten" class. Caveat below (Gate-3 no-op risk).

### G10-5 — Yanked num-bigint 0.4.7 — LOW (hygiene, not a vuln); rebuild-only

`num-bigint 0.4.7` (`Cargo.lock:1811`), **yanked** (maintainers pulled that version — not a CVE),
reached via `jsonwebtoken → simple_asn1 → num-bigint` (`deny.toml:46-48`). `yanked = "deny"` →
cargo-deny RED. Rebuild-only; prod Node unaffected. Fix = `cargo update -p num-bigint` to a
non-yanked patch, or bump `simple_asn1`/`jsonwebtoken`. **Same PR/bucket as G10-4** (JWT dep-chain
refresh). Severity **LOW / informational**. Already Gate-3 tracked.

### G10-6 — B3 NOBYPASSRLS flip still open — latent-CRITICAL (defense-in-depth off); XL effort; RED-LINE

**Prod runs as `dowiz_app` with `rolbypassrls = t`** (memory `pg-privilege-hardening-2026-06-29`
§2026-06-30 audit: "the REAL operational role is `dowiz_app` (bypassrls=t)"). The boot-guard
(`packages/db/src/index.ts:32-39`) only rejects `current_user === 'postgres'` — it does **not**
check `rolbypassrls`. So `dowiz_app` passes the guard while bypassing RLS ⇒ **FORCE-RLS is a live
no-op, ~103 policies dormant, tenant isolation = app-layer WHERE clauses only** (memory
`security-sweep-findings-2026-07-02`: "app-layer WHERE is the only live tenant boundary").

The flip is DEFERRED behind an **8-item program** (`b3-auth-hardening-council-2026-07-03`
resolution §4): enumerate+wrap ~123 raw `.db.query` paths; wire the fail-closed role-grant chain
(`GRANT dowiz_app TO dowiz_app_rls` — absent today ⇒ naive flip = total login lockout incl.
operator); money-key fix (RC4 keys on `app.current_tenant`, which `withTenant` never sets);
DEFINER-fn owner-pin; live silent-denial detection; orphan audit at flip; drift-reconciliation (mig
077 unguarded throws); dated WS-URL migration.

Two prior staging flips were **cleanly reverted** after catching real gaps (pg-privilege memory
§2026-06-30 Phase 3): the **first-membership/org WRITE bootstrap** has no admitting policy
(chicken-and-egg — `app_member_location_ids()` is empty for a brand-new membership). Phases 1+2 (23
additive policies + 21 SECURITY DEFINER maintenance fns, migs 077/078/079) landed **DARK/inert**.

What breaks if flipped naively **today**: public checkout TX rolls back (anon INSERTs, no policy);
ALL auth dies (`users`/`auth_refresh_tokens` zero-policy FORCE); every order transition 409s; owner
401 globally; onboarding/mock-auth 500 on membership bootstrap; ~15 GUC-less workers doing
cross-tenant sweeps (need DEFINER wrappers — see the `courier-offer-sweep.ts:129-130` R-FLAG). And
LEDGER #50 (C1 anon fail-open policies) would flip from latent → a **live table-wide cross-tenant
PII siphon**.

Severity framing (honest): **not a live exploit by itself** while every app-WHERE predicate is
correct — but the flip being OFF means the **entire second layer of defense is disabled**. Any
single missing WHERE predicate becomes an un-backstopped cross-tenant breach, and the 07-02 sweep
already found **5 live** such misses (#1/#4/#5/#6/#7). So: a single app-WHERE miss under this posture
= cross-tenant PII (**High**), and misses are demonstrated. This is the highest-value + highest-effort
item and is **RED-LINE (auth/RLS/migrations) → operator-gated + council**.

### G10-7 — OR-1: is the live operational pool BYPASSRLS? — effectively ANSWERED (YES) by inference

OR-1 asked whether the live write pool bypasses RLS (severity of LEDGER #50 flips on it). Evidence,
**without connecting to any DB**:
- Staging `dowiz_app` = `bypassrls=t` (pg-privilege memory, verified via a throwaway NOBYPASSRLS
  probe role in that session).
- Prod deploys from `origin/main` on the same role/topology; the GUC-coverage audit concluded
  flipping "breaks essentially the whole app" ⇒ prod **depends on** bypass.
- The boot-guard checks `'postgres'` only, not `rolbypassrls` ⇒ a healthy prod tells us nothing
  about the bit; app health is consistent with bypass-on.

So OR-1 ≈ **YES, prod is BYPASSRLS**, by strong inference. The residual unknown is a *live confirmation
of the exact bit*, deliberately not performed here (no-DB rule). It matters because if prod were
*unexpectedly* NOBYPASSRLS, LEDGER #50/C1 would already be a live siphon — evidence says it is not.
The deterministic way to close OR-1 permanently (without a manual probe) is the **two-mode boot-guard
that records the bit** (§4 step 5) — turning a one-off operator question into a continuously-asserted,
self-documenting invariant.

---

## 3. Options & tradeoffs — fix vs accept-risk, per item

| # | Fix option | Accept-risk option | Recommendation |
|---|-----------|--------------------|----------------|
| G10-1 | Split to `WHERE email_hash=$1 OR phone_hash=$2` with a validated/normalized phone, add deterministic tie-break (reject >1 match as ambiguous, or `ORDER BY created_at LIMIT 1` + explicit uniqueness), drop dead `phoneHash` | Accept: invite-gated, DoS-only, tenant-local | **FIX** — trivial, removes a latent auth-path ambiguity; guardrail cheap |
| G10-2 | Replace `*` with an env-driven storefront-origin allowlist (or reflect same-site); keep menu GET embeddable via explicit allowlist | Accept: preflight gap + no-cookie + public endpoint make it near-inert | **TIGHTEN** (cheap) OR **ACCEPT** with a posture doc — operator's call; either is defensible |
| G10-3 | Versioned ciphertext format (1-byte key-id) + keyring + re-encrypt backfill; or envelope encryption | Accept: no live exploit, small blast radius, re-encrypt is feasible ad hoc | **FIX the FORMAT now** (versioned, forward-only), **defer the backfill** — cheap insurance, avoids a future big-bang |
| G10-4 | Swap web-push/JWT crypto path off rsa (p256/ES256 or maintained push crate) | Accept-with-rationale: rebuild-only + VAPID uses EC + no decryption oracle | **ACCEPT (documented) until cutover; FIX in Layer 0.6** before any Rust push surface ships |
| G10-5 | `cargo update -p num-bigint` / bump simple_asn1 | (n/a — trivial) | **FIX** — bundle with G10-4's JWT dep PR |
| G10-6 | The 8-item staged program → staging flip → prod flip | Accept: keep app-WHERE as sole boundary (status quo) | **FIX, STAGED, OPERATOR-GATED** — but not now; do the cheap wins + OR-1 boot-guard first |
| G10-7 | Two-mode boot-guard that records `rolbypassrls` | (n/a) | **FIX** — enables G10-6 and self-documents OR-1 forever |

---

## 4. Recommended execution blueprint (sequenced by severity × effort)

Ordering rationale: land the cheap, low-risk, non-red-line wins first (they also close the
*forgetting*); then the OR-1 boot-guard (unblocks and de-risks B3); gate the XL B3 program last
behind explicit operator + council. Every step follows the repo's **Verified-by-Math** rule (ship
the RED case alongside the green) and the REGRESSION-LEDGER ratchet (guardrail red→green + a row
*before* "done").

Legend: **[GATE]** markers — `🟢 auto` (deterministic, non-red-line) · `🔴 operator` (red-line:
auth/RLS/migrations/money) · `🟠 council` (red-line design review first).

---

**STEP 0 — Stop the forgetting (the "never again" mechanism). [GATE 🟢 auto]**
The reason this trio rotted 2 weeks is that nothing deterministic tracked it. Fix the tracking
BEFORE the vulns.
- Action: add one REGRESSION-LEDGER **PENDING** row per open item below (matching the #50/#26
  PENDING format: symptom / root cause / guardrail-type / where / date), each carrying an explicit
  `review-by:` date. Add `scripts/guardrail-security-debt-review.mjs` (wired into `verify:all` +
  a `plane-guard.mjs` pending-check, mirroring #50's pending-check and #49's "advisory-forever"
  hard-friction): parse the ledger's PENDING rows; **exit 1** when `today > review-by` and no
  disposition (fixed | re-accepted-with-new-date) is recorded.
- VbM proof: seed a fixture PENDING row with `review-by` in the past → guard exits 1 (RED); set it
  to the future or add a disposition → exit 0 (GREEN); whole repo green today.
- Effort: **S** (½ day).

---

**STEP 1 — G10-5 + G10-4 dep-chain refresh (rebuild JWT/push crypto). [GATE 🟢 auto for 0-5; 🟠 council for the push swap]**
- Action (5, trivial): `cargo update -p num-bigint` to a non-yanked patch (or bump `simple_asn1`);
  re-run `cargo deny check advisories`.
- Action (4, medium): replace the `web-push → jwt-simple → superboring → rsa` chain — build the
  VAPID JWT with `p256`/`ecdsa` + `jsonwebtoken` ES256 (already a direct dep), or adopt a maintained
  push crate without that transitive rsa. Keep the dev-only test RSA keygen out of the release graph
  (it already is a `[dev-dependencies]` entry).
- **[GATE 🟠]** the push-crate swap touches the S8 notification port design → route through the
  rebuild-jobs-s8 council note before landing.
- VbM proof: `cargo deny check advisories` goes GREEN (RED today on exactly these two — the falsifiable
  case is the current tree); a VAPID-signature **golden known-answer test** (sign a fixed payload with
  a fixed EC key → assert byte-identical to a pre-computed signature) proves the new path is
  equivalent; push E2E on the Rust surface (staging) delivers a real notification.
- Effort: **S** (5) + **M** (4).

**STEP 1b — Harden Gate 3 so it can't silently no-op. [GATE 🟢 auto]**
`sovereign-gate.sh:56` "degrades to SKIP-with-warning when cargo-deny isn't installed", and it is
**not yet a required CI check** (0b-6 open, `proposed-sovereign-core-ci/` unplaced). That is the same
false-green class as `verify:secrets`/gitleaks (audit §1 risk 6). Action: make the cargo-deny step a
**required** CI job that **installs cargo-deny and FAILS if absent** (no skip-warn in CI).
VbM proof: a CI run with cargo-deny removed must go RED (not green-skip); with it present + clean,
GREEN. Effort: **S**.

---

**STEP 2 — G10-1 courier hash. [GATE 🔴 operator — auth path]**
- Action: normalize/validate `phone` (E.164 or reject) at register; bind two distinct params
  (`email_hash=$1 OR phone_hash=$2`) where `$2 = sha256(normalizedPhone)` (or `NULL`-guard the
  phone arm); make the match deterministic — **reject when the lookup returns >1 row** (an
  ambiguous identifier is a hard `401`/`409`, never a silent `rows[0]`); delete dead `phoneHash`.
  Consider a `phone_hash` partial-unique index (nullable-unique) so the namespace-overlap
  registration is rejected at the DB.
- VbM proof (falsifiable): a test seeds courier A (email X) + courier B (phone-string = X); asserts
  login with X **either** resolves deterministically to A **or** returns "ambiguous", and **never**
  silently authenticates against B's row. RED against current code (nondeterministic `rows[0]`);
  GREEN after. Add as a LEDGER row (guardrail-type `unit`/`integration`).
- Effort: **S-M**. Red-line because it edits a live auth handler → operator-gated, staging E2E.

---

**STEP 3 — G10-2 CORS. [GATE 🟢 auto if tighten to allowlist; operator decision fix-vs-accept]**
- Fix path: replace `Access-Control-Allow-Origin: *` on `/api/orders` POST with an env-driven
  storefront-origin allowlist (reflect only known tenant/storefront origins); keep the menu-GET
  embed case behind an explicit allowlist entry.
- VbM proof: a **cross-origin request that must FAIL post-fix and passes/echoes-`*` pre-fix** — an
  integration/E2E asserting that a request with `Origin: https://evil.example` to `POST /api/orders`
  receives **no** `Access-Control-Allow-Origin: *` (RED today: header is `*`; GREEN after: absent or
  echoing only an allowlisted origin), and a control request from an allowlisted storefront origin
  still gets its ACAO. Add a static guardrail (`scripts/guardrail-no-wildcard-cors.mjs`) that greps
  `server.ts` for `Access-Control-Allow-Origin', '*'` and fails, so a re-add is caught.
- Accept path: if the operator judges the preflight-gap + no-cookie + public-endpoint analysis
  sufficient, write `docs/security/CORS-order-create-posture.md` (matching the
  `RLS-anon-path-isolation-posture.md` accept-risk pattern) + a PENDING ledger row with review-by.
- Effort: **S**.

---

**STEP 4 — G10-3 pii-cipher versioned format. [GATE 🔴 operator — PII crypto]**
- Action (now): change `encryptPII` to prepend a **1-byte key version** (`0x01`) →
  `[ver][IV][ct][tag]`; `decryptPII` reads the version and selects from a keyring
  (`COURIER_PII_ENCRYPTION_KEY` = v1; future `..._V2` = v2). Keep the legacy-base64 branch for
  pre-versioned blobs (treat as v0). This makes rotation a *config* change + a **lazy re-encrypt on
  next write**, not a big-bang. Defer the full backfill migration until a rotation is actually needed
  (blast radius is 1 table / 3 columns — trivially re-encryptable then).
- VbM proof (falsifiable): round-trip `decrypt(encrypt(x)) === x` across v0/v1/v2 fixtures; a RED
  case proves that a v2-encrypted blob **fails** to decrypt under a v1-only keyring and **succeeds**
  once v2 is in the ring (i.e., the version byte is load-bearing, not cosmetic). Existing
  `pii-cipher.test.ts` extended; add a LEDGER row.
- Effort: **M**. Red-line (PII at rest) → operator-gated; must prove backward-compat with existing
  prod ciphertext before deploy.

---

**STEP 5 — G10-7 OR-1 closure via two-mode boot-guard. [GATE 🔴 operator — boot-guard]**
This is the pivot that de-risks B3 and answers OR-1 permanently.
- Action: extend `createOperationalPool`'s `on('connect')` guard (`index.ts:32-39`) to also
  `SELECT rolbypassrls FROM pg_roles WHERE rolname = current_user` and:
  - record the bit as a structured boot log line + a `/health` sub-check (self-documenting OR-1);
  - **mode `warn`** (bypass expected) while `RLS_ENFORCED !== 'true'` — no crash on today's prod;
  - **mode `FATAL`** once `RLS_ENFORCED='true'` and the bit is still `t` (post-flip regression guard).
  This is exactly the P0-4 "two-mode boot-guard" the B3 council pre-approved as SAFE-NOW
  (`b3-auth-hardening-council-2026-07-03`, "no-op on today's prod so safe").
- VbM proof (falsifiable): boot under a NOBYPASSRLS probe role with `RLS_ENFORCED=true` → clean;
  boot under a bypass role with `RLS_ENFORCED=true` → **FATAL** (RED); with `RLS_ENFORCED` unset →
  warn + boot (GREEN). Add a LEDGER row (guardrail-type `boot-guard`).
- Effort: **S-M**. Red-line (boot-guard) → operator-gated; the FATAL arm ships dark behind
  `RLS_ENFORCED` so it cannot brick prod.

---

**STEP 6 — G10-6 B3 NOBYPASSRLS flip. [GATE 🔴 operator + 🟠 council — auth/RLS/migrations, XL]**
Do **not** start until Steps 0-5 land. This is the 8-item program from
`b3-auth-hardening/resolution.md §4`; it is a multi-session staged effort, not a single change.
Blueprint of the flip's proof sequence (the "staging probe sequence proving policies bind without
breaking the app"):
1. **Enumerate + wrap** the ~123 raw `.db.query` paths onto the `withTenant`/DEFINER seam;
   wire the fail-closed grant chain (`GRANT dowiz_app TO dowiz_app_rls`); money-key fix (RC4);
   DEFINER owner-pin; worker DEFINER wrappers (e.g. `app_sweep_expired_offers`). All **DARK/inert**
   (already partly done: migs 077/078/079).
2. **Narrow C1** (LEDGER #50) — the anon fail-open SELECT/UPDATE policies — via a SECURITY DEFINER
   scoped by order-id/token-hash with pinned search_path (reuse #3's primitive), BEFORE the flip, or
   customer order-tracking 404s for everyone (the KNOWN TRAP).
3. **Staging probe** (VbM RED→GREEN, no prod): under a throwaway `CREATE ROLE rp NOBYPASSRLS; GRANT
   dowiz_app TO rp; SET LOCAL ROLE rp` role with **no** `app.user_id` GUC, assert `SELECT` on
   `orders`/`order_items`/`customers` returns **0 rows** (RED today = all rows → proves policies are
   dormant; GREEN after narrowing → proves they bind). This is exactly the skip-registered probe
   LEDGER #50 mandates in `verify-rls.ts`.
4. **Flip on staging** (`ALTER ROLE dowiz_app NOBYPASSRLS`) → run the **full lifecycle E2E**
   (checkout create, order transitions, owner dashboard, courier flow, onboarding/mock-auth
   bootstrap, GDPR erasure) + `verify:rls`. A single 500/409/401 = a missing policy → **revert**
   (`ALTER ROLE … BYPASSRLS`, documented debug-revert, never a give-up "fix") and patch. (Two prior
   attempts caught the membership-bootstrap gap this way.)
5. **Flip the boot-guard to FATAL** (`RLS_ENFORCED=true`, Step 5) so the posture cannot silently
   regress.
6. **Prod flip** = a separate explicit operator step after staging soaks.
- Effort: **XL** (multi-session). Every sub-step red-line; council on the flip design.

---

## 5. Risks & rollback

- **Steps 0, 1(0-5), 1b, 3(tighten)** — non-red-line, deterministic; rollback = revert the commit.
  Zero prod-behavior risk (rebuild-only or static gates).
- **Step 1 push swap** — risk: a subtly wrong VAPID signature silently breaks push. Mitigation: the
  golden-KAT test is the gate; rebuild is dark, so a regression never reaches prod push (Node path
  untouched).
- **Step 2 (courier auth)** — risk: over-strict phone validation rejects legit couriers; the
  "reject >1 match" arm could lock out a pre-existing collided pair. Mitigation: staging E2E on real
  courier login + a one-time scan for existing `phone_hash = any email_hash` before enabling the
  hard-reject; rollback = revert (auth handler is isolated).
- **Step 4 (pii format)** — risk: a format bug makes existing prod ciphertext undecryptable
  (courier PII read failure). Mitigation: the v0/v1 backward-compat test is the gate; deploy is
  additive (new writes v1, old blobs still decrypt); rollback = revert (old code still reads v1? No
  — so **do not delete the v0/legacy branch**; the revert-safe design keeps decrypt able to read all
  versions). Ship decrypt-side first, encrypt-side second if extra caution wanted.
- **Step 5 (boot-guard)** — risk: the FATAL arm bricks prod if shipped un-gated. Mitigation: it is
  behind `RLS_ENFORCED` (unset in prod today) → warn-only; rollback = unset the env / revert.
- **Step 6 (B3 flip)** — highest risk: a naive flip = total login lockout incl. operator, checkout
  rollback, global 401/409. Mitigation is the entire staged program: dark policies, the NOBYPASSRLS
  probe role, full-lifecycle E2E gate, **instant `ALTER ROLE … BYPASSRLS` revert** (proven 2.x on
  staging twice), prod flip as a separate gated step. Never re-grant BYPASSRLS as a "fix" — only as
  a documented debug-revert.

---

## 6. Operator decision points

1. **G10-2 fix vs accept** — tighten CORS to an allowlist (Step 3 fix) or record the
   accept-risk posture doc + PENDING ledger row? (Both defensible; the analysis says near-inert.)
2. **G10-4 accept-until-cutover** — confirm the documented accept-risk for RUSTSEC-2023-0071 while
   rebuild is dark (rebuild-only, EC-VAPID, no oracle), with the Layer-0.6 swap as the closing
   condition before any Rust push surface ships. Sign the deny.toml rationale as operator-accepted.
3. **G10-6 sequencing** — authorize the B3 program to *start* (Step 6) only after Steps 0-5, and
   only via the council + staged flip. This is the single largest security uplift (turns 103 dormant
   policies live) and the single largest risk. Decide whether business-validation (audit rec 9)
   outranks it for the next session's bandwidth.
4. **G10-3 backfill timing** — approve shipping the versioned *format* now (Step 4) and deferring
   the re-encrypt *backfill* until a rotation is actually triggered.
5. **Step 0 guard teeth** — decide `guardrail-security-debt-review.mjs` mode: hard `exit 1` on an
   overdue review-by (blocks CI) vs plane-guard friction-warn. (Recommend hard-fail in `verify:all`,
   mirroring the ratchet's monotonic rule — that is what makes silent-forgetting *physically*
   impossible, the whole point of G10.)

---

*Blueprint only — no code, DB, or git state was modified in producing it. The single file created is
this document.*
