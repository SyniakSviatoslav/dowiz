# Resolution — Soft Access Gate (Triad RESOLVE, R1 → R2)

> Phase: RESOLVE · System Architect (Triad) · Date: 2026-06-20
> **R2 (this revision): STOP-ETHICS human decisions folded in** — see the dedicated
> section below. Source of decisions: `ethical-decisions.md` (owner ruling, 2026-06-20).
> Inputs: `proposal.md` (now REVISED) · `breaker-findings.md` (B1–B12) ·
> `counsel-opinion.md` (STOP-1, STOP-2) · `ADR-soft-access-gate.md` (now ACCEPTED)
> Every disposition below is grounded in source (not summary): pg-boss queue DDL
> (`1790000000009:20`, `1790000000011`), Fastify ctor (`server.ts:138-142`, no
> `trustProxy`), rate-limit reg (`server.ts:462`), `provider.ts:5,37` (`locationId`
> required), `1790000000007:16` (`location_id NOT NULL`), `audit.ts:38`,
> `db/src/index.ts:32` (NOBYPASSRLS guardrail), `auth.ts:138/213` (open self-serve),
> `websocket.ts:118` (XFF precedent), no `/privacy` route in `apps/web`.

## Breaker findings — disposition

| ID | Sev | Disposition | How (1-line) |
|----|-----|-------------|--------------|
| **B1** | CRIT | **FIX** | New forward-only migration `1790000000042` pre-creates the queue + partition table **under the migration role** (mirrors `1790000000011`); runtime `createQueue` becomes a no-op. Boot-best-effort path was structurally blocked by `REVOKE CREATE ON SCHEMA pgboss`. (§4 Migration B, §8) |
| **B2** | CRIT | **FIX** (route) + **DEFER-FLAG** (telemetry/otp) | Managed `keyGenerator` reads real client IP from `Fly-Client-IP`/`X-Forwarded-For[0]`; same value feeds `ip_hash`. Global `trustProxy` NOT flipped. telemetry/otp `request.ip`=proxy is the same latent bug → separate defer-flag, owner platform/security. (§2, §7, §9) |
| **B3** | HIGH | **FIX** (sweep into scope) | Reconciliation cron `access-request.reconcile` re-enqueues `notified_at IS NULL` rows. Outbox rejected (over-eng); bare accept-risk rejected (admin UI also deferred → no v1 recovery). (§5, §6, §9) |
| **B4** | HIGH | **ACCEPT-RISK** (owner: security) | `USING(true)` documented as linter/anti-BYPASSRLS guard, **not** isolation; real boundary = GRANT layer (app-wide). Dedicated write-only role doesn't shrink the read surface (worker/erasure/ops-list SELECT) → over-eng. Compensated by claim-check, ip_hash, 1 PII column, day-one erasure. (§3 B4 block, §9) |
| **B5** | HIGH | **FIX** | Enqueue is fire-and-forget **after the reply** → INSERT-only latency on every path; honeypot does equivalent cheap work; malformed email → route self-parse → uniform `200` (never global 400). (§5, §7) |
| **B6** | MED | **FIX** | Worker calls a thin **direct `EmailAdapter.sendOps()`** (system channel, no `locationId`, no tenant audit) — does **not** route through `NotificationDispatcher`, which requires `locationId` and writes `location_id NOT NULL` audit. (§7) |
| **B7** | MED | **FIX** | Per-job exhaustion does NOT page; the sweep emits **one** aggregated ops alert on persistent `notified_at IS NULL` backlog; present-but-invalid key → one boot alert. (§2, §8) |
| **B8** | MED | **FIX** | Worker **claims-before-send** (CAS `WHERE notified_at IS NULL RETURNING email`), rolls back + throws on send failure, and **acks-without-throwing on a missing row** (erasure tolerance). (§5) |
| **B9** | MED | **DEFER-FLAG** (confirmed) + cross-ref STOP-1 | Proposal nowhere claims access control; owner-onboarding-invite-gating is the defer-flag that would make "gate" true. Copy reframed to interest (ties to STOP-1). (§9 B9, §10) |
| **B10** | LOW | **ACCEPT-RISK** (owner: ops) | Mooted by day-one DELETE grant + erasure runbook — rows erasable regardless of table presence after rollback. (§4 down note, §9) |
| **B11** | LOW | **ACCEPT-RISK / mitigate** (owner: security) | Honeypot demoted to **secondary**; rate-limit (real IP) is primary; field renamed off `website` to dodge autofill; `autocomplete="off"`. Not load-bearing. (§9, §10) |
| **B12** | LOW | **ACCEPT-RISK / document** (owner: frontend) | localStorage guard = **UX-only, not a control**; never blocks a resubmit. (§9, §10) |

## ETHICAL-STOP — disposition

| STOP | Status | What changed / what remains human |
|------|--------|-----------------------------------|
| **STOP-1** (microcopy sells scarcity the product lacks) | **REVISED (design-cleared)** | All user-facing copy reframed to "register interest / keep me posted / we'll be in touch." Banned-string list added (waitlist, request access, early access, position #, approved, under review, application). Full sq/en key table in §10. **Remains human:** final copy sign-off (per CLAUDE.md, copy is a human surface); OR — if a *true* waitlist is wanted — ship owner-onboarding-invite-gating first/together. |
| **STOP-2** (PII collected day one, erasure deferred to Stage-30) | **REVISED (design-cleared) + human items isolated** | Day-one erasure path shipped: operational role `DELETE` grant + `scripts/erase-access-request.ts <email>` + operator runbook (NOT Stage-30). `user_agent` dropped. `ip_hash` kept with named purpose (abuse forensics / rate-limit, real client IP). Stage-30 demoted to *automation over an existing manual path*. **Remains human:** (1) lawful basis wording (recommend: legitimate interest); (2) retention window `[N]` months (recommend: 12, then auto-erase); (3) a `/privacy` page/section must exist before the CTA ships (none exists today). |

## STOP-ETHICS human decisions → design changes (R2)

The owner ruled on both ETHICAL-STOPs (`ethical-decisions.md`). The two STOPs are now
**resolved by human decision** (no longer "design-cleared, wording pending"). Each ruling
and the concrete design change it forced:

| Human decision | Ruling | Design change applied |
|---|---|---|
| **STOP-1 — gate truthfulness** | **Invite-gating FIRST.** Make the gate *true* rather than soften copy. | `owner-onboarding-invite-gating` promoted from defer-flag → **hard blocking prerequisite**. Sequencing fixed: invite-gating Council → shipped → this feature. Its surface (`auth.ts:112-118/138` Google, `:184-188/213` Telegram, `onboarding.ts:30`) recorded as **input for a separate Council**, not designed here. Copy rule: "register interest / we'll be in touch" allowed at any state; "waitlist/approved" only **after** invite-gating ships. (proposal §1, §8 launch gate, §9 prerequisite, §10; ADR §Decision-9, §Prerequisite) |
| **STOP-2a — lawful basis** | **Explicit consent** (withdrawable), not legitimate interest. | New columns `consent_at timestamptz NOT NULL` + `privacy_version text NOT NULL` (proposal §4). Server validates `consent === true` literal → **400** on missing/false (justified anti-enumeration-safe, §5/§7). Frontend: required consent checkbox, **submit disabled until ticked** (§10). All "legitimate interest" copy → "consent". (proposal §1/§4/§5/§7/§10; ADR §Decision-8/9) |
| **STOP-2b — retention** | **12 months + auto-erase**, in scope. | New pg-boss cron `access-request.retention-sweep` (`DELETE … WHERE created_at < now() - '12 months'`), mirroring `anonymizer.retention` (`workers/anonymizer-retention.ts:21-28`), advisory-lock guarded, config `ACCESS_REQUEST_RETENTION`/`_CRON`. Day-one manual erasure stays as the floor. Stage-30 anonymizer-folding demoted to redundant. (proposal §5/§6/§8; ADR §Decision-8) |
| **STOP-2c — /privacy page** | **Build minimal `/privacy`** (sq/en). | New SPA route `<Route path="/privacy" …>` in `apps/web/src/main.tsx:39-50` + `PrivacyPage.tsx` (static, design tokens, i18n). Renders the notice version === `PRIVACY_NOTICE_VERSION` written on submit. Content: basis(consent)/data/purpose/retention(12mo)/rights/contact. (proposal §8 routing + §10; ADR §Decision-8) |
| **STOP-2d — user_agent** | **Dropped** (no named use). | Confirmed dropped in R1; re-affirmed in DDL (§4). `ip_hash` retained, hashes the **real client IP** (B2 fix, `Fly-Client-IP`/XFF — not `request.ip`=proxy). |

### Anti-enumeration × consent-400 — explicit reconciliation (does not weaken B5)
B5 (uniform-200) protects **email-existence** secrecy. The consent-400 is **orthogonal**:
`consent` is a client-controlled boolean independent of the email, so a 400 on
`consent !== true` reveals nothing about whether the email is new/duplicate/fake. The 400
runs **before** any DB/honeypot work (so it is *cheaper* than a real submit, no DoS
amplification) and **behind** the same per-IP 5/min bucket. The frontend disables submit
until consent is ticked, so a real user never reaches the 400 — only a UI-bypassing script
does, and it learns nothing. **Decision: keep the explicit 400 for consent** (real
lawful-basis enforcement; collapsing it into uniform-200 would silently accept
no-consent rows) while uniform-200 stays for email/honeypot/malformed.

### New risks created by R2 (handed to the Breaker for re-attack)
N1 consent-400 oracle/DoS · N2 privacy_version & retention-window drift · N3 retention
DELETE racing notify/reconcile (B8.1 tolerance reused) · N4 stale-consent on `ON CONFLICT
DO NOTHING` duplicate · N5 `/privacy` SPA route × SSR shell. Full dispositions in
proposal §9 "NEW risks." Each carries a Breaker-verification ask.

## Counsel steel-man (inbox-only, no table)
**Recommendation: keep the table** — decisive reason: it **survives a Resend outage**
(inbox-only loses the lead if the one best-effort email fails) + dedup + `status='new'`
queryability (admin UI deferred). The steel-man's real force is the bar it sets:
*persisting PII is defensible only with an erasure answer* — which is exactly why STOP-2's
day-one DELETE path is now mandatory and in scope. Inbox-only recorded as a legitimate
absolute-minimalist fallback, not chosen. (§11)

## Scope changes vs PROPOSED draft (cumulative through R2)
- **Into scope (R1):** reconciliation/sweep cron (`access-request.reconcile`); migration B
  (`1790000000042`) for queue durability; day-one erasure path + script; aggregated alert.
- **Into scope (R2 / STOP-ETHICS):** explicit-consent capture (`consent_at` +
  `privacy_version` columns, server `consent:true` validation, consent checkbox);
  **12-month retention auto-erase cron** (`access-request.retention-sweep`); minimal
  **`/privacy` page** (sq/en SPA route).
- **New blocking prerequisite (R2, separate change):** `owner-onboarding-invite-gating` —
  promoted from defer-flag → **must ship before launch**, own Council.
- **Out of schema:** `user_agent` column dropped.
- **Stayed out (defer-flags):** admin review UI, Turnstile/hCaptcha, double-opt-in,
  re-consent flow (newer `privacy_version`), telemetry/otp `request.ip` fix (B2 corollary),
  `locale` enum reconciliation. Stage-30 anonymizer-folding demoted to redundant (the
  dedicated retention sweep handles erasure).

## STOP-ETHICS items — RESOLVED by human decision (was: pending)
All five prior human-decision items are now **closed** by the owner ruling
(`ethical-decisions.md`):
1. **[STOP-1] Gate truthfulness** → **RESOLVED: invite-gating ships first** (blocking
   prerequisite). "Waitlist/approved" copy forbidden until then.
2. **[STOP-2] Lawful basis** → **RESOLVED: explicit consent** (withdrawable). Columns +
   checkbox + server validation in scope.
3. **[STOP-2] Retention window** → **RESOLVED: 12 months**, auto-erase cron in scope.
4. **[STOP-2] `/privacy` page** → **RESOLVED: built in scope** (sq/en SPA route).
5. **[B9 / product-truth] "gate" vs capture** → **RESOLVED via #1**: invite-gating makes
   the gate real before launch; no interim mislabeling permitted.

### Residual human sign-off (non-blocking, normal CLAUDE.md copy rule)
- **Final copy polish** (sq + en) for the §10 table — copy is a human surface per
  CLAUDE.md. The *content/basis/retention/banned-strings* are decided; only wording
  refinement remains, and it must not reintroduce any banned scarcity string pre-gating.

---

## R2 resolution (RESOLVE-round R2, 2026-06-20)

> Source: `breaker-findings.md` §"R2 Re-attack" (R2-1…R2-10) + `counsel-opinion.md`
> §"R2 Re-examine" (advisories #1, #2). Every disposition below is grounded in re-read
> source, not summary. New code-grounded facts this round (verified myself):
> - **16 existing `boss.schedule(...)` calls** run at runtime under the operational
>   (NOBYPASSRLS) pool: `anonymizer-retention.ts:27`, `server.ts:452` (FREE_TIER_WATCH),
>   `courier-cron.ts:21,26`, `backup/index.ts:40-46`, `backup/backup-verify-scheduled.ts:31,40`,
>   `reconciliation.ts:41`, `settlement-cron.ts:26`, `dwell-monitor.ts:26`, `signal-raiser.ts:24`,
>   `liveness-checker.ts:38`, `order-timeout-sweep.ts:33`, `rates-refresh.ts:25`. These are
>   live on prod (anonymizer/backups/settlement run today) → the runtime role **provably**
>   writes `pgboss.schedule`. (R2-1)
> - `0009:45-48` + `0011:139-142` both `ALTER DEFAULT PRIVILEGES … GRANT … ON TABLES TO
>   PUBLIC` in schema `pgboss`, and `0011:128-132`/`0009:31-36` grant-loop all *existing*
>   pgboss tables — so `pgboss.schedule` is DML-grantable whether it predates or postdates
>   the grant-loop. (R2-1)
> - `Fly-Client-IP` appears in **zero** source files (grep); only `websocket.ts:118` reads
>   raw `x-forwarded-for` (for a log line, not a security key). (R2-2)
> - `SPA_ROUTES` (`server.ts:870`) = `['/admin','/courier','/dashboard','/s/','/login','/branding-preview']`
>   — `/privacy` absent; not-found handler 404s any non-`text/html` GET. (R2-5)
> - `CheckoutPage.tsx:913` already ships a "contact … for removal" privacy pattern → an
>   erasure-contact line is an established repo convention. (Counsel R2 #2)

### R2 dispositions — table

| R2 ID | Sev | Disposition | How (1-line) |
|-------|-----|-------------|--------------|
| **R2-1** | HIGH | **FIX (mirror proven cron) + tighten proof** | The retention/reconcile crons inherit the **already-proven** runtime `boss.schedule` path (16 live schedulers, incl. `anonymizer.retention`, write `pgboss.schedule` on prod today); `pgboss.schedule` DML is covered by `0009`/`0011` grant-loops **and** `ALTER DEFAULT PRIVILEGES`. No grant-hole → **no new migration needed**. The break ("schedule table postdates grants") is disproven: pg-boss v10's contractor installs `job`+`schedule` in one run and the default-privileges line covers any later table. **Fix delta:** the worker copies the `anonymizer` shape **exactly** (no `.catch` swallow on `schedule`, so a real failure surfaces to boot, not silenced) + a new **boot-assert/proof obligation** that the two cron schedules exist in `pgboss.schedule` after boot. 12-month auto-erase is no longer undemonstrated. |
| **R2-2** | HIGH | **FIX (Fly-Client-IP only; no spoofable XFF trust)** | `clientIp()` reads **exclusively** `Fly-Client-IP` (the Fly edge sets & overwrites it on every request → not client-injectable, unlike XFF). **Removed** the `X-Forwarded-For[0]` trust-fallthrough entirely — it was the spoof hole. Non-prod fallback = `request.ip` (deterministic, never the client-controlled XFF), gated `NODE_ENV !== 'production'`. In prod, **missing `Fly-Client-IP` → fail-closed to a single shared bucket** (degrade, never trust a spoofable header) + a one-time boot warn. Boot-assert that `Fly-Client-IP` is observed on the first real request. telemetry/otp stay **defer-flag** (unchanged owner: platform/security). |
| **R2-3** | HIGH | **FIX (fold consent into uniform-200 honeypot contour; drop the 400)** | The 400 is **eliminated**, killing both the DoS-amplifier ("cheapest path") and the status/latency route-fingerprint at once. No-consent is treated as a **silent honeypot-class reject inside the uniform-200 contour**: same 200 `{ok:true}`, same equivalent cheap no-op work, **no INSERT, no `consent_at`**. Lawful basis is **preserved** — a row is only ever written on `consent === true`, and *no row = no processing*, so silently dropping a no-consent body collects nothing (the lawful-basis red line is "no row without consent", which holds). Frontend keeps the disabled-button UX. This is strictly better than the 400 on every axis the Breaker named. (Supersedes N1 accept.) |
| **R2-4** | MED | **ACCEPT-RISK (recovery = now-proven reconcile cron)** | Fire-and-forget enqueue lost on a single-machine deploy/OOM is recovered by `access-request.reconcile` — whose scheduling is **proven** by the R2-1 fix (no longer "may not schedule"). Bounded loss = ≤(15min cadence + grace) of un-notified rows, self-healed, never data loss (row is committed). Owner: ops. |
| **R2-5** | HIGH | **FIX (add `/privacy` to `SPA_ROUTES`, Accept-agnostic 200)** | Add `'/privacy'` to `SPA_ROUTES` (`server.ts:870`) so the not-found handler serves `index.html` for **any** GET to `/privacy` regardless of `Accept` (regulator/curl/crawler/unfurler get 200 + the SPA shell, not a JSON 404). The current handler already serves the shell when `request.url` matches a `SPA_ROUTES` prefix **independent of `Accept`** — so the one-line list edit is the complete fix. Proof: `GET /privacy` with `Accept: */*` → 200, not 404. |
| **R2-6** | MED | **FIX (content-hash backstop, CI-asserted)** | `PRIVACY_NOTICE_VERSION` is derived from / asserted against a **content hash of the rendered notice prose** via a CI test: the test hashes the notice copy strings and fails if the hash changed without `PRIVACY_NOTICE_VERSION` being bumped. This makes "version label binds to text" mechanical, not hand-discipline — closes the "edit prose, forget to bump" gap the Breaker found. Owner: frontend + security. |
| **R2-7** | MED | **ACCEPT-RISK (retention from first contact, justified) + doc** | Keep `ON CONFLICT DO NOTHING` (consent-pinning is correct per N4/Counsel) and retention-from-`created_at`. A re-submitter at month 11 being erased at month 12 is **acceptable**: the lawful interest dates from first contact; 12mo from first contact is within the defensible band (Counsel: 12 is "long-but-not-unreasonable"). Refreshing `created_at`/`consent_at` on duplicate would forge a consent record (the dishonest move N4 rejects). **Documented** so the semantics are deliberate, not accidental. The `/privacy` copy is tightened to "for up to 12 months **from when you first contact us**" so it cannot be mis-read as last-contact. Owner: security. (Re-consent flow stays defer-flag.) |
| **R2-8** | MED | **FIX (structural Zod gate for non-email fields)** | A dedicated strict Zod parse runs the **consent/honeypot/locale** fields *before* the uniform-200 email contour: `z.object({ consent: z.literal(true), website: z.string().max(0).optional(), locale: z.string().optional() })`. `consent` is `z.literal(true)` → `"true"`/`1`/missing all **fail structurally** (not a slip-prone `if (body.consent)`). On consent-fail the route takes the **R2-3 silent-honeypot path** (uniform 200, no INSERT). Email existence stays un-enumerated (email is parsed leniently *after*, never gated). `consent_at` is therefore only ever set on a structurally-validated literal `true`. |
| **R2-9** | LOW | **FIX (bound reconcile re-feed) + ACCEPT residual** | Reconcile predicate gains a **bounded-attempt guard**: a new `notify_attempts` smallint (incremented by the notify worker on exhaustion) caps re-enqueue at e.g. 10 cumulative attempts; rows past the cap are skipped by reconcile and surface in the **one** aggregated alert as `status='new'`-stuck, not re-fed forever. Kills the permanent-failure treadmill (bad address / invalid key). Residual (one stale alert until operator acts) accepted. Owner: ops. |
| **R2-10** | MED | **FIX (CI-asserted launch gate, not a comment)** | Two mechanical gates: (1) a **build-time feature flag `ACCESS_GATE_PUBLIC_ENABLED`** (default `false`) — the CTA route/page is not rendered in prod while false; (2) a **CI banned-strings test** that greps the access-request i18n keys for `waitlist|request access|early access|position #|approved|under review|application` and **fails the build** if any appear while `ACCESS_GATE_INVITE_GATING_SHIPPED` is not set. Converts the STOP-1 sequencing promise (was a §8 sentence) into an honesty-**proof** (Counsel R2 #1). |

### Counsel R2 advisories — disposition

| Advisory | Disposition |
|----------|-------------|
| **#1 launch-gate as CI assertion** | **= R2-10 FIX.** Banned-strings grep + flag, not a checklist line. |
| **#2 `/privacy` operable erasure contact** | **FIX.** `/privacy` content list (§8) gains a **concrete, reachable erasure-request channel** (a real `privacy@`/operator email or contact route — not a placeholder, not the internal `WAITLIST_NOTIFY_EMAIL`), mirroring the existing `CheckoutPage.tsx:913` "contact … for removal" convention. "Withdraw anytime" is now operable, not aspirational. Owner: ops/security to provision the address before launch. |
| #3 re-consent promotable on material notice change | Already recorded as defer-flag; re-affirmed (non-blocking). |
| #4 not-pre-checked Playwright assertion | Folded into proof obligations (assert `not.toBeChecked()` on initial render). |
| #5 12mo→6mo consider | Human already ruled 12mo; within defensible band; no change. |

### Net R2 status
- **HIGH remaining unresolved: none.** R2-1 (proven-pattern fix), R2-2 (Fly-Client-IP-only fix),
  R2-3 (drop the 400), R2-5 (`SPA_ROUTES` fix) are all **mechanically fixed**, not hand-waved.
- **accept-risk this round:** R2-4 (deploy-loss recovered by now-proven reconcile cron; owner ops),
  R2-7 (retention-from-first-contact; owner security), R2-9 residual (one stale alert; owner ops).
- B4 (shared-pool PII read) accept-risk carries forward unchanged (owner security).

### Scope changes vs R2 banner (cumulative through R2-RESOLVE)
- **Behaviour change:** consent enforcement moves from **400** → **silent uniform-200
  honeypot-class drop** (R2-3) backed by a **structural Zod literal-`true` gate** (R2-8).
  The "consent-400" red line in proposal §1/§5/§6/§7 and ADR §Decision-6 is **superseded**.
  *(R3-3b correction: the follow-on claim "no non-200 for any well-formed-or-malformed body"
  was an over-promise — a malformed-**JSON** body still gets a framework 400 at the
  content-type parser. See §"R3 resolution" R3-3b: the honest invariant is "indistinguishable
  along the email-existence axis", which holds; the framework-400 is accept-risk, not an
  email-oracle.)* The only **route-emitted** non-200 is a transport failure (DB 503/500).
- **Into scope (R2-RESOLVE):** `Fly-Client-IP`-only `clientIp()` (R2-2); `/privacy` in
  `SPA_ROUTES` (R2-5); privacy-notice **content-hash CI test** (R2-6); **structural consent
  Zod gate** (R2-8); reconcile **bounded-attempt guard** + `notify_attempts` column (R2-9);
  `ACCESS_GATE_PUBLIC_ENABLED` flag + **banned-strings CI test** (R2-10); **boot-assert** the
  two cron schedules exist + `Fly-Client-IP` observed (R2-1/R2-2 proof); `/privacy` **erasure
  contact** address (Counsel #2).
- **No new migration for R2-1** — the cron schedule path is already grant-clean and proven.
  `notify_attempts smallint NOT NULL DEFAULT 0` is added to the **existing** `1790000000041`
  DDL (not yet shipped) — no separate migration.

---

## R3 resolution (RESOLVE-round R3, 2026-06-20)

> Source: `breaker-findings.md` §"R3 Focused verification" (R3-1…R3-5). Mode: each R3-*
> finding → fix / accept-risk(+owner) / defer. Every disposition re-grounded in the working
> tree this round (verified myself, not from proposal text):
> - `server.ts:129-136` `unhandledRejection`/`uncaughtException` guards explicitly "kept alive"
>   (suppress Node's crash-and-exit). `:401` `await anonymizerRetentionWorker.start()` — a bare
>   top-level `await` in `main()` with **no surrounding try/catch** (only try blocks are the
>   queue-table check `:295-306` and `fastify.listen` `:903-910`). `:905` `await fastify.listen`
>   runs **after** every worker `.start()`. `:913` `main();` invoked **bare** (no `.catch`).
> - `anonymizer-retention.ts:27` `boss.schedule(...)` is **NOT** `.catch`-wrapped (confirmed);
>   `rates-refresh.ts:25` **IS** `.catch`-wrapped (the safe shape).
> - `server.ts:516-523` global error handler: `error.validation` → **400**; else
>   `statusCode || 500` → **500**. Malformed JSON is rejected by Fastify's content-type parser
>   → this handler → **400**, before any route handler.
> - `server.ts:870` `SPA_ROUTES` still lacks `/privacy` in HEAD (R2-5 is design, not yet merged).
> - `server.ts:439` `LivenessChecker` is a **pg-boss-scheduled cron in the worker-drain
>   context** (`liveness-checker.ts:32-40`) — it never sees the HTTP request path's headers, so
>   it is NOT a seam for re-arming the `Fly-Client-IP` warn (corrected R3-2 disposition).

### R3 dispositions — table

| R3 ID | Sev | Disposition | How (1-line) |
|-------|-----|-------------|--------------|
| **R3-1** | HIGH | **FIX (best-effort `.catch` + post-listen fail-fast boot-assert)** | The new cron's `boss.schedule` is **`.catch`-wrapped** (copy `rates-refresh.ts:25`, NOT un-`.catch`'d `anonymizer-retention.ts:27`) so a schedule throw **cannot abort `main()` before `fastify.listen()`** — HTTP always comes up (no zombie). Failure is made **visible the right way**: a post-`listen()` boot-assert `SELECT`s `pgboss.schedule` for both cron rows and `process.exit(1)` on a miss → Fly restarts, **deploy shows red** (fail-fast > silent live-but-HTTP-dead process the `:129` kept-alive guard would mask). The R2-1 "copy anonymizer, no `.catch`" instruction is **superseded** — it backfired given the real boot topology. 12mo-erase is never on a zombie path. Supersedes proposal §5/ADR Decision-3. **Defer-flag (owner: platform):** legacy `anonymizer-retention.ts` (+ other un-`.catch`'d schedules before `listen()`) carry the same latent zombie pattern; separate ticket to `.catch`-wrap them and/or add `main().catch(()=>process.exit(1))` at `server.ts:913`. NOT widened here. |
| **R3-2** | LOW | **ACCEPT-RISK + cheap re-arm (owner: platform)** | Fail-closed shared-bucket degrade fails **safe** (over-throttle, never under-protect, never re-opens spoof). Replace the one-time boot-latch with a throttled in-process re-warn: module-level `lastFlyMissingWarnAt` + 60s interval in `clientIp()` → a mid-life `Fly-Client-IP` loss re-surfaces ≤60s of the next request, ≤1 warn/min (no log-storm). Pure in-process; `LivenessChecker` rejected as the seam (worker-drain context, no HTTP headers). Residual = ≤60s blind window on a config-change-only degrade. |
| **R3-3a** | HIGH | **FIX (counsel-R3: client success gated on consent-in-body + 2xx)** | The frontend shows `success` **only when both** (a) it locally verified the body it actually sent carried `consent === true` (strict post-serialize check — "I sent real consent") **and** (b) 2xx. A no-consent-through-bug/serialization-drift send (`true`→`"true"`, dropped field, autofilled honeypot) renders **`error`/retry, never a false success**, even on a 200. Closes the B11-class silent legit-user lead-loss **on the human side** without the server leaking consent state (uniform-200 / anti-enumeration intact). **Negative UI proof added:** submit with consent NOT in the sent body → assert success copy is NOT shown. (proposal §10, Proof obligations) |
| **R3-3b** | HIGH→LOW | **ACCEPT-RISK + reformulate invariant (owner: security)** | The "zero non-200 for any well-formed-or-malformed body" claim was an **over-promise**: malformed-JSON → Fastify content-type parser → **400** before the route, which the route cannot suppress. But that 400 is **framework-level, app-wide, and identical for every JSON route** → it leaks **nothing about email existence** (not an enumeration oracle; route is publicly-known). **Reformulate the invariant honestly: "indistinguishable along the email-existence axis"** (which holds) and **accept-risk** the framework-400 (suppressing = app-wide parser surgery, over-eng against a non-leak). Removes the over-obligation from §1 + ADR Decision-6. Anti-enumeration red line **intact**, downgraded HIGH→accepted-LOW. |
| **R3-4** | MED | **FIX (flag gates BACKEND `fastify.register`, not just render) — critical for STOP-1** | `ACCESS_GATE_PUBLIC_ENABLED` (default off) gates the **`fastify.register(accessRequestRoutes, …)` call itself** (`server.ts`). While off, `POST /api/access-requests` is **unmounted → 404** via `setNotFoundHandler` (`server.ts:871`) — the capture endpoint is **not publicly POST-able before invite-gating ships** (no seeding `access_requests`, no operator-email trigger). Migrations still ship (additive, no route to write them). **This** — not the render flag — satisfies STOP-1's "door must not be open before the gate is real." Launch = single reviewable flag flip after invite-gating lands. **CI proof:** flag off → POST returns 404; flag on → 200. (proposal §8, ADR Decision-9, Proof obligations) |
| **R3-5** | LOW | **VERIFIED-CLOSED (no action)** | Retention DELETE × notify/reconcile is **in-flight-safe**, re-confirmed: the notify worker's CAS `UPDATE … WHERE id=$1 AND notified_at IS NULL RETURNING …` returns 0 rows on a deleted row and **acks without throwing** (B8.1). A retention sweep deleting a row mid-notify cannot crash the worker; the only residual is one wasted ack-noop notify cycle and the *already-accepted* R2-7 lead-loss policy (re-submitter erased 12mo from first contact). No new defect. Closed. |

### Net R3 status — honest CRITICAL/HIGH assessment

- **CRITICAL remaining: none.**
- **HIGH remaining unresolved: none.** R3-1 (best-effort `.catch` + fail-fast boot-assert),
  R3-3a (client success gated on consent-in-body + negative proof), R3-4 (backend route-
  registration gate) are all **mechanically fixed** against verified topology, not hand-waved.
  R3-3b is **reformulated + accepted** (the HIGH was an over-promise, not a real email-oracle —
  the email-existence non-leak holds; downgraded to accepted-LOW).
- **The two NEW R3-HIGH the Breaker opened are closed by mechanism, not by re-assertion:**
  R3-1's cure (un-`.catch`'d schedule) is **replaced** with the safe shape + a real runtime
  fail-fast guard; R3-3's silent-200 inversion is left intact server-side (it correctly serves
  anti-enumeration + lawful basis) and the legit-user-loss is closed **at the client** where it
  belongs, with the malformed-JSON sub-claim honestly retracted to accept-risk.

### Accepted-risk register (cumulative, all rounds, with owners) — current

| Risk | Round | Owner | Note |
|------|-------|-------|------|
| **B4** — `USING(true)` RLS = no row isolation; shared operational pool reads all emails | R1 | Security | real boundary = GRANT-layer + parameterized-query norm; dedicated role doesn't shrink read surface. |
| Resend daily-cap exceeded on launch spike | R1 | Ops | best-effort; bulk `status='new'`; one aggregated alert (B7). |
| Rare duplicate operator email (two-worker race) | R1 | Ops | reduced by B8 claim-before-send; low-value mail. |
| No CAPTCHA → bots fill table within rate-limit | R1 | Security | honeypot + 5/min real-IP + unique(email) cap; PII = 1 col; day-one erasure. |
| **B10** — rollback leaves PII table | R1 | Ops | mooted by day-one DELETE grant + runbook. |
| **B11** — honeypot autofill false-negative | R1 | Security | honeypot secondary; rate-limit primary; **now also closed on human side by R3-3a** (client gates success on consent-in-body). |
| **B12** — localStorage guard | R1 | Frontend | UX-only, never a control. |
| **R2-4** — fire-and-forget enqueue lost on deploy/OOM | R2 | Ops | recovered by reconcile cron (scheduling now also boot-fail-fast-proven, R3-1); bounded, never data loss. |
| **R2-7** — retention from first contact (re-submitter erased 12mo from first) | R2 | Security | lawful interest dates from first contact; refreshing would forge consent; copy tightened "from first contact". |
| **R2-9** — permanent send-failure treadmill | R2 | Ops | bounded by `notify_attempts < $cap`; residual = one stale alert. |
| **R3-2** — `Fly-Client-IP` mid-life-loss warn latch | R3 | Platform | fail-safe degrade; throttled in-process re-warn ≤60s; ≤1 warn/min. |
| **R3-3b** — framework malformed-JSON 400 at parser | R3 | Security | NOT an email-existence oracle (app-wide framework 400); invariant reformulated; over-promise retracted. |

### Scope changes vs R2 (cumulative through R3-RESOLVE)
- **Behaviour change (R3-1):** the new cron worker's `boss.schedule` is **`.catch`-wrapped
  best-effort** (NOT the un-`.catch`'d anonymizer shape) + a **post-`listen()` fail-fast
  boot-assert** (`process.exit(1)` on missing schedule rows). The R2-1 "no `.catch` so it
  surfaces to boot" instruction is **superseded** (it surfaced as a no-HTTP zombie, masked by
  the kept-alive guard).
- **Behaviour change (R3-4):** `ACCESS_GATE_PUBLIC_ENABLED` gates **backend route
  registration** (route 404s while off), not just frontend render — the R2-10 flag was
  render-only and left the endpoint live (R3-4). STOP-1 now enforced at the reachable surface.
- **Behaviour change (R3-3a):** frontend `success` requires **locally-confirmed
  `consent===true`-in-sent-body + 2xx**, not a bare 200 → no false success on a no-consent send.
- **Invariant reformulation (R3-3b):** anti-enumeration red line restated as "indistinguishable
  along the email-existence axis" (§1, ADR Decision-6); the "zero non-200 for any body"
  over-promise removed; framework malformed-JSON 400 accepted as a non-oracle.
- **Defer-flag added (R3-1, owner: platform):** `.catch`-wrap / fail-fast the **legacy**
  un-`.catch`'d `boss.schedule` crons (anonymizer et al.) before `listen()` — same latent
  zombie pattern; separate ticket, NOT widened into this scope. The NEW worker is correct here.
- **No new migration** for any R3 item — all are runtime/route/frontend changes.
