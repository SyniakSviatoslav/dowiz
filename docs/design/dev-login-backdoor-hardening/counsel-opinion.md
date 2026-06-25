# Counsel Opinion — Dev-Login Backdoor Hardening

**Slug:** `dev-login-backdoor-hardening`
**Role:** Counsel (Philosopher · Physician). Advisory. Human decides.
**Re:** `proposal.md` + `docs/adr/0003-dev-login-fail-closed.md`
**Date:** 2026-06-22
**Verdict in one line:** The *fix* is sound, proportionate, and honest. The *incident* is under-examined. The proposal hardens the door but says almost nothing about whether someone already walked through it. My friction is on the incident, not the engineering.

---

## 0. What I am NOT contesting

The defense-in-depth design (B explicit flag + C dev-kid + D boot fail-fast) is good engineering and I have verified its load-bearing claims against live source — the hardcoded literals (`server.ts:874`, `local.ts:44-45`), the `!!configuredSecret` gate (`dev-guard.ts:19-21`), the three call sites. The NODE_ENV blind spot is named honestly rather than hidden. Failure-first is real. The "smallest that holds" framing is true. I will not re-litigate robustness — that is the Breaker's and Architect's ground. My lenses look *around* and *ahead* of the patch.

---

## 1. Reasoning by lens (only what's load-bearing)

### Care / harm — who gets hurt, and the harm already in the past tense
The proposal frames harm in the **future conditional**: "if a leaked token is presented, it is rejected." But the severity line says this was **live CRITICAL on prod**, and the escalation chain (anonymous → `role:owner` → `/onboarding/start` → org+location+membership creation) was **executable for some real window of time**. The honest question is not "will it be exploited" but "**was it, and did a real person's tenant get touched.**"

This matters for care because the people who could be hurt are not abstract: a real owner whose storefront sits next to a forged one, a customer whose order data lives under a tenant created by an attacker, the operator who has to answer "were we breached" to a partner. The fix is correct and care-respecting in steady state. The **gap is that the proposal treats the breach as a config bug to close, not as an event that may have produced victims.** Closing a door you found open is necessary; it is not the same as checking who is already inside.

### Honesty / consent — the UI/comment tells a comforting story that source contradicts
Two shipped comments assert a falsehood as reassurance:
- `server.ts:871-873`: *"In production the secret is unset and the seeded test account has no usable password_hash, so this always rejects."*
- `local.ts:42`: *"In production the secret is unset, so these hardcoded creds never work."*

Both were **false at the time they shipped** — the secret *was* set on prod (leaked from staging), and the bypass never reads `password_hash` at all, so the "no usable hash" reassurance was structurally irrelevant. This is not malice; it is a comment that encoded an *assumption* ("prod won't have the secret") as a *guarantee* ("this always rejects"). It is the textual twin of the bug. Worth naming under honesty because comforting-but-false invariants are how this class of hole stays invisible: the next reader trusts the comment and doesn't check the gate. The remediation should **delete these comments**, not soften them — a corrected comment that still claims safety would be the same trap one revision later. (Server-authoritative posture is otherwise intact; this is a documentation-honesty point, not a trust-boundary one.)

### Fairness / stakeholders — the cost of the fix is fairly distributed; the cost of the *incident* has no owner
The fix's costs land on the right shoulders: staging/CI operators set one flag (cheap, theirs to bear), prod gets safety (the party at risk benefits). Fair. But the **incident's cost** — forensic review, possible disclosure, the labor of proving non-exploitation — is currently **unassigned**. R-6 hands "immediate mitigation" to the operator and stops. Nobody owns "establish whether the live backdoor was used." Unowned work in an incident is work that doesn't happen; the fairness failure is that the *next* person to discover an undisclosed prior breach inherits a far larger bill.

### Long horizon / engineering hygiene — the deeper pattern, not this instance
This is the **third** time the codebase has shipped a "dev convenience that is fail-closed only by an environment assumption" (the dev-guard `!!secret`, the inline handler, the latent `local.ts` duplicate — three call sites of one flawed idea). The proposal fixes the three sites. The 2nd-order question: **why does this pattern keep being born?** Because dev-login is *valuable enough to keep* (14 E2E specs) but *dangerous enough to kill* (this incident) — and that tension regenerates the hazard every time someone adds a convenience under a "non-prod only" comment. B+C+D taxes each *instance*. What it does not do is make the *class* harder to reintroduce. A lint/CI assertion that fails on any new hardcoded-credential literal or any new `devLoginAllowed`-style gate keyed on mere presence would tax the *pattern*. That is the lock-in-avoiding, year-from-now-no-regret move. (Non-blocking — see §3.)

### Aesthetics / conceptual integrity — the fix is elegant; the duplication is the wart
"One change in the gate function covers all three call sites" is genuinely elegant — capability bound to one seam. The dev-kid reuse of the existing `verifyAuthToken` kid-mismatch is the most beautiful part: defense that already existed, now *aimed*. The remaining aesthetic dissonance is the **two near-identical login handlers** (`server.ts` inline + `local.ts` plugin) with overlapping-but-divergent behavior (different token TTL — 1d vs 15m, different refresh-token handling, different cred sets). Two handlers doing "login" with subtly different security properties is exactly the kind of incoherence that breeds the *next* bug. The proposal's "align or delete the duplicate" (item 5) is the right instinct; I'd push it from optional-sounding to **resolve to exactly one path** — concept integrity here is also security integrity.

### Epistemics — the load-bearing unverified assumption
Named honestly by the proposal itself (R-1, R-2, assumptions 1-3): **prod's actual NODE_ENV is unknowable from the repo.** The design correctly refuses to lean on it. But there is a *second* unverified assumption the proposal does **not** flag: it assumes the leaked owner token (and any tenant it spawned) **either doesn't exist or will be erased by key rotation.** That assumption is doing silent work — it's why the doc can treat "already-minted token" as a clean cryptographic problem (C rejects it) rather than a forensic one (what did it *do* while valid). Key rotation kills the *token*; it does not un-create the *org/location/membership rows* the token may have written. Those persist after rotation. That is the gap between "the credential can no longer be used" and "the system is clean."

---

## 2. ETHICAL-STOPs — grounded red lines only

I raise **two**. Both are *friction*, not verdicts: they pause the council and require a **recorded human decision**, they do not block the fix from shipping and do not override a conscious operator. The hardening should proceed regardless; these are about the *incident around* the hardening.

### ETHICAL-STOP-1 — Forensic determination before "closed" is declared
**Grounded line:** *care-not-purpose / harm-to-a-real-person* + *server-authoritative truth*. A confirmed live owner-minting backdoor on production is a security event affecting potentially real tenants, not only a config defect.
**Why it's a stop:** the proposal closes the hole and rotates the key but **never asks whether the creds were already used**, and treats the leaked token as a purely cryptographic object. "Fixed" is being defined as "can't happen again," when for a live-on-prod backdoor the human-decision-requiring question is "**did it already happen, and to whom.**"
**What the human must decide (recorded):** run a forensic pass *before* declaring remediation complete, or consciously accept not doing so and record why. Concrete, grounded anchors that exist in this codebase (I verified them — these are not hypothetical):
  1. **`organizations` / `memberships` / `locations` rows on prod** created by an account that should never have created any (the seeded `test@dowiz.com` user, or orgs with no plausible human provenance). The self-escalation chain *writes these rows* — they are the durable fingerprint and they **survive JWT key rotation**. This is the single most important check.
  2. **`auth_refresh_tokens` rows** — note: the *inline* `server.ts` handler writes none and logs nothing (thinnest possible trace), but the `local.ts` bypass path reuses the refresh-token insert, so a refresh-token row tied to the test user is a positive signal of *that* path being hit.
  3. **There is no auth/login audit log** (I checked — settlement/backup/courier have audit tables; auth does not). This is itself a finding: it means absence of evidence here is **not** evidence of absence. The human must decide knowing the logs are thin.
**Disposition if accepted:** if the operator records "checked rows 1-2, found nothing anomalous, accept residual uncertainty given thin logs" — that is a legitimate human close. Refusing to *look* is the thing the stop forbids.

### ETHICAL-STOP-2 — Disclosure obligation is a human decision, not an engineering omission
**Grounded line:** *honesty / consent* + *anonymize-don't-delete (data-stewardship duty)*. There is a `/compliance` SoT, a RoPA, a privacy gate — this org has explicitly taken on a data-steward posture toward real users.
**Why it's a stop:** if real customer/owner PII sat under a tenant reachable by the forged owner token while it was live, there may be a **breach-disclosure duty** (to affected users, to a partner, to a regulator depending on jurisdiction/data) that is **independent of whether the hole is now closed.** The proposal is silent on disclosure. Silence is itself a decision — and disclosure-by-omission is the dark-pattern version of incident response: the system stays comfortable, the affected person never finds out.
**What the human must decide (recorded):** based on STOP-1's findings — *was real-user data within blast radius during the live window?* If yes → a conscious disclosure decision (notify / don't-notify-and-why) recorded in `/compliance`. If STOP-1 establishes no real tenant/data was ever touched (e.g. prod had no real users yet — the launch trigger is "first real paid order," and per project memory prod may still be pre-launch/dark) → disclosure obligation likely **does not attach**, and *that* reasoning gets recorded. The stop is satisfied by a recorded human judgment either way; it is violated only by never asking.
**Note tying the two together:** the launch-trigger context ("first real paid order") is decisive here. If no real paid order ever occurred, the *severity of consequence* drops sharply even though the *severity of the defect* is CRITICAL — there may be no victim. The human decision is much cheaper to make correctly than to skip. This is exactly why I friction now rather than later.

---

## 3. Non-blocking advice (aesthetic / strategic — ignore freely)

1. **Strongly consider removing dev *password*-login entirely; keep only `x-dev-auth-secret` mock-auth.** This is the biggest simplification available and I'd weight it heavily. The `/dev/mock-auth` header path already mints test identities and is what most of the 14 specs use; the *password* bypass (`/api/auth/local/login` with literal creds) is a second, redundant door into the same room. One auth-bypass mechanism is comprehensible and auditable; two with divergent token TTLs and cred sets is the incoherence that bred this incident. Collapsing to header-only mock-auth would make B+C smaller (one path to gate), make item-5's "align or delete duplicate" trivial (delete both inline + plugin password bypass), and remove the hardcoded-credential class outright rather than relocating it to env. The cost is rewriting whichever specs hit password-login to use `/dev/mock-auth` — bounded, one-time, and the kind of thing R-5's spec audit will quantify anyway. *Steel-manned against in §4.*

2. **Make the comments tell the truth or not exist.** Delete the "always rejects in prod" comments (§1 honesty). A reassuring-but-false invariant in a security path is worse than no comment.

3. **Tax the pattern, not just the instances (long horizon).** A cheap CI assertion — no hardcoded email/password literal in `apps/api/src/**`, and any `devLoginAllowed`-shaped gate must be greppable + flag-keyed — would stop the *fourth* incarnation from being born. This is the year-from-now-no-regret move; the per-instance fix is not.

4. **Add a positive-confirmation startup log line** (the proposal already suggests `dev-login: DISABLED` in prod — endorse it). Positive assertion in logs > inferred-from-absence, given there is no auth audit log to fall back on.

5. **Aesthetic: resolve to exactly one login handler.** Two `/auth/local/login` implementations with different security properties is the wart. Pick one.

---

## 4. Steel-man of a rejected option

**Steel-man of Option A (NODE_ENV-only gate) — the one the proposal rejects most firmly.** The proposal's rejection is correct *for this codebase*, but the strongest case *for* A deserves a fair hearing, because A's underlying instinct is sound: **the most reliable gate is the one with zero operator action required.** B (explicit flag) introduces a new env var that *two* environments must remember to set — and "operator forgets to set the flag" is a real failure mode (staging E2E silently breaks) that A doesn't have. A's deeper virtue is *fewer knobs*: a single, conventional, industry-standard `NODE_ENV !== 'production'` check is something every engineer already understands and can't misconfigure by *omission* (the default behavior is the safe one without anyone setting anything). The proposal's own R-1/R-2 show that B+D *also* end up coupled to NODE_ENV anyway (D fires on it), so A-haters can't claim full decoupling. A pure-NODE_ENV shop with a *trustworthy, in-repo* NODE_ENV (e.g. set in Dockerfile per-stage) would be right to prefer A's simplicity over B's extra-var ceremony.

**Why it still loses here:** the specific, *verified* fact that kills A is §2's finding — prod's NODE_ENV is set out-of-band via an invisible Fly secret, *not* in the repo, and `.env.example` even says `development`. A's "can't misconfigure by omission" virtue evaporates when the value is itself invisible and unverifiable. A is the right answer in a shop that controls NODE_ENV in-repo; this shop doesn't, and the proposal proved it. Steel-man honored; rejection stands on a grounded fact, not taste.

**Briefer steel-man of "full removal of dev-login" (the §3.1 advice, argued *against* itself):** keeping password-login has one real virtue — it tests the *actual production login codepath* (Zod schema, role/location resolution, refresh-token issuance) that `/dev/mock-auth` *bypasses*. Mock-auth mints a token directly; password-login exercises `local.ts`'s real resolution logic. Deleting it means the real login path loses an E2E exercise. That's a genuine cost — not fatal (a real-account argon2 login spec could cover it), but honest. So §3.1 is advice, not a stop.

---

## 5. The open question nobody asked

**The proposal asks "how do we make sure prod fails closed." Nobody asked: "if prod had no real users, was this even a CRITICAL — and if it *did*, why is establishing that not step one?"**

The entire remediation is shaped by the word CRITICAL, and CRITICAL is correct for the *defect class*. But severity-of-defect and severity-of-consequence are different axes, and the proposal never separates them. The launch trigger is "first real paid order." Per the project's own context, prod may still be dark/pre-launch. **If no real human ever held an account or placed an order on prod during the live window, this is a CRITICAL defect with zero victims** — a near-miss, not a breach — and the correct response is "close it, log the near-miss, move on," *not* a full disclosure-and-forensics fire drill. **If even one real owner or customer existed, it's a breach** and STOP-1/STOP-2 become mandatory.

The single fact that collapses most of this opinion's uncertainty — **how many real (non-test, non-seed) users and paid orders existed on prod during the window the backdoor was live** — is cheaply knowable (it's a `SELECT count(*)`), is not asked anywhere in the proposal or ADR, and *should be the first thing established*, because it determines whether STOP-1/STOP-2 are a five-minute confirmation or a genuine incident. Asking it first makes the right response cheap. Not asking it leaves the team performing either too much ceremony (if dark) or too little care (if not) — and not knowing which.

---

*Counsel out. The fix should ship. The incident should be looked at before "done" is said. Both stops are satisfiable by a recorded human judgment — including the judgment "we were dark, no victims, near-miss." The one thing not on the table is declaring it closed without looking.*

---

## RE-EXAMINE (2026-06-22) — post-resolution

I re-read `resolution.md` against this opinion and against live schema. Short version: the resolution **heard** both stops and the open question and framed all three correctly as NEEDS-HUMAN-DECISION with named owners — the *governance* framing is adequate. But the **queries are not yet sufficient to collapse the uncertainty**: two of them are wrong against the actual schema, and a third silent failure mode (RLS) would make a real breach *look* clean. That is a concrete, fixable gap, and it matters precisely because these queries are the instrument the human's "we were dark, no victims" close will rest on. A query that silently returns zero is worse than no query — it manufactures false reassurance, the exact §1-honesty failure I flagged.

### Are the stops + open question adequately captured?
- **STOP-1 (forensics before "closed")** — captured. Resolution §Part-B-STOP-1 + R-7 carry the framing intact: durable rows survive rotation, run before declaring closed *or* record accepted residual uncertainty, owner = Operator, no auth audit table so absence ≠ proof. Framing adequate. **Queries: partially adequate — see corrections below.**
- **STOP-2 (disclosure)** — captured. §Part-B-STOP-2 + R-8: correctly made *dependent* on STOP-1, owner = data-steward, recorded in `/compliance`. Framing fully adequate; it is a judgment, not a query, so nothing to correct.
- **Open question (prod dark / real users+orders in window)** — captured as R-9, correctly named as the fact that splits "CRITICAL-with-victims vs near-miss," operator defines `<window-start>`. Framing adequate. **Query has a broken column — see below.**

### Do the queries actually cover row-provenance that survives key rotation? — three corrections needed
I verified column names against `packages/db/migrations/1780310071220_core-identity.ts`, `1780310074262_orders.ts`, `1780314625706_auth-refresh-tokens.ts`. The provenance coverage is *conceptually* right (orgs/locations/memberships, refresh tokens, real users/orders, absence of auth audit) but three execution defects mean the queries as written do not yet collapse the uncertainty:

1. **RLS will silently zero the forensic counts (most dangerous).** `organizations`, `locations`, `memberships`, `customers`, `orders` are all `ENABLE` + **`FORCE ROW LEVEL SECURITY`** with `tenant_isolation` policies keyed on `app.user_id` / `app_member_location_ids()`. A forensic query run as the normal app role with `app.user_id` unset (or set to a non-owning user) returns **zero rows by policy**, not by fact. The human would read "0 anomalous orgs" and record "no victims" when the rows are merely *hidden*. **The queries must be run as a BYPASSRLS / superuser role (or with the policies explicitly bypassed), and the resolution must say so.** Without that instruction the instrument lies in the safe direction — unacceptable for a close that feeds a disclosure decision.

2. **The open-question order query references a non-existent column.** `SELECT count(DISTINCT customer_phone) FROM orders` — `orders` has **no `customer_phone`**; it has `customer_id uuid REFERENCES customers(id)`, and the phone lives on `customers.phone`. As written the query errors out (or, worse, if someone "fixes" it by guessing, returns nothing). Correct shape: join `orders o JOIN customers c ON c.id = o.customer_id` and count `DISTINCT c.phone`, or count `DISTINCT o.customer_id`.

3. **"Real *paid* order" is under-specified — `created_at` alone over-counts.** The launch trigger is the first real **paid** order, but the query filters only on `created_at >= window`. `orders` carries `status order_status` and `payment_outcome payment_method/payment_outcome` (default `'pending'`). A pending/cancelled order in the window is not a victim-bearing paid order. To actually answer "was there a real paid order," the filter should also constrain `payment_outcome` (a settled/paid value) and/or `status`, otherwise the count conflates abandoned carts with real transactions and inflates apparent severity. (This cuts *toward* caution, unlike #1/#2 which cut toward false-clean — but it still means the query as written does not answer the question it claims to.)

Provenance items that **are** adequately covered (no correction): `organizations.owner_id` join (column exists, nullable — good, "no plausible human owner" is exactly the null/seed case to look for); `memberships` by `user_id`; `auth_refresh_tokens.user_id` + `created_at`; the verified-absent auth/login audit table (correctly treated as "absence ≠ proof"). One quiet provenance note: `memberships ON DELETE CASCADE` from `users` — see the new concern on deleting `empty@` below.

**Verdict on the queries:** the *set* of checks is right and does cover rotation-surviving provenance; the *instrument* has one safe-direction lie (RLS) and two correctness bugs (missing column, paid-vs-created). Until #1–#3 are corrected in `resolution.md`, a clean result is not trustworthy. This is a HIGH-value, ~10-minute fix and should be done before the operator runs them, because a falsely-clean forensic pass is how STOP-1 gets satisfied in form while violated in substance.

### Did the revised design introduce any NEW ethical/strategic concern or long-term smell?
Reviewed the four revisions (dev-kid segregation, flag in both guards, deleting `empty@`, exempt-when-gate-open rate limit). Mostly clean; the design got *more* honest (the "false comfort comment → delete not soften" became explicit, the kid-rejection over-claim was retracted). Concerns are minor and non-blocking:

- **Deleting `empty@dowiz.com` — harmless on prod, but confirm the blast radius isn't a forensic erasure.** `memberships` and `auth_refresh_tokens` both `ON DELETE CASCADE` from `users`. If the remediation's "delete the empty@ cred" ever becomes "delete the empty@ *user row* on prod," the cascade would silently destroy exactly the membership/refresh-token rows STOP-1 wants to inspect. The resolution means delete the *code literal*, not a prod row — but make that explicit, because "delete empty@" is ambiguous and a literal-minded implementer running it against prod would shred evidence. **Sequence: forensics (STOP-1) before any user-row deletion, always.** (New, low, easily closed by one sentence.)

- **Dev-kid segregation is a genuine net-positive, no new smell.** It removes the singleton-kid coupling and aims an existing check. The one strategic watch-item already named (R-3: shared dev kid staging↔CI) is correctly accepted — both non-prod, boundary is prod-rejection. Fine.

- **Exempt-when-gate-open rate limit — clean, and actually the more honest design.** It hardens the *real* argon2 path (email+IP) where the real threat lives and stops pretending to rate-limit a dead-in-prod path. No new concern. Minor watch: "fail-closed when the limiter store is down" (proposal §7) is the right call for the real-login path; just ensure that doesn't accidentally extend to the dev path in a way that flakes CI — but that is Breaker/Architect ground, not mine.

### Did deleting one bypass path while keeping `x-dev-auth-secret` mock-auth leave the codebase cleaner, or just relocate the risk?
**Net cleaner, but a partial cleanup, not the full one I advised in §3.1.** Honest accounting:
- *Cleaner:* the hardcoded-credential **class** shrinks from two cred pairs in two files to one literal moved to env, and the divergent-TTL duplicate handler dies — the §1 aesthetic wart (two login handlers, different security properties) is reduced. That is real concept-integrity gain.
- *Relocated, not eliminated:* the password-bypass *mechanism* survives in the inline handler (now flag-gated), and the `x-dev-auth-secret` mock-auth family survives entirely. So there are still **two** auth-bypass mechanisms with different shapes (password-literal vs header-secret), both now under one flag. The §3.1 advice — collapse to header-only mock-auth, delete password-bypass outright — was *not* taken; the resolution kept password-login for the real-codepath E2E coverage I steel-manned in §4. That is a legitimate engineering choice, not a dodge. But it means the *class* ("dev convenience fail-closed by assumption") is taxed per-instance, not eliminated — exactly my §1 long-horizon point, now confirmed as a conscious deferral rather than an oversight. The §3.3 "tax the pattern in CI" advice remains the un-taken move that would actually close the class; still non-blocking.

So: relocation reduced, not pure elimination. Acceptable. The residual is one extra bypass mechanism whose only remaining justification is real-login E2E coverage — if that coverage is ever moved to a real-account argon2 spec, the inline password bypass should then be deleted (a future-cleanup marker, not a blocker).

### ETHICAL-STOPs still open
- **STOP-1 — OPEN, NEEDS-HUMAN-DECISION. Governance framing adequate; queries NOT yet adequate** (RLS-bypass instruction missing; one missing column; paid-vs-created under-specified — see three corrections above). Fix the queries, then the human can run them and record a trustworthy result.
- **STOP-2 — OPEN, NEEDS-HUMAN-DECISION. Fully adequate** as framed (judgment dependent on STOP-1, recorded in `/compliance`). No correction.
- **Open question (R-9) — OPEN, NEEDS-HUMAN-DECISION. Adequate framing; query needs the customer_phone→customers.phone join and a paid/status filter** (corrections #2/#3).

### One-line readiness verdict
**Ethically ready to ship the *hardening* now (it is sound, honest, and the stops are non-blocking) — but the forensic/disclosure decisions are not yet *recordable with confidence* until the three query defects (RLS-bypass, missing column, paid-filter) are corrected, because as written a real breach could read as clean. Fix the queries (~10 min), run them as a BYPASSRLS role, record the human judgment, and only then say "closed."**

---

## RE-EXAMINE round 2 (2026-06-22) — confirmation pass on RESOLVE round 2

I re-verified the four corrected forensic queries against live migrations (not against the architect's claims — against the source). All four corrections are now **confirmed sound**. The query instrument is trustworthy.

### (1) BYPASSRLS/superuser role — CORRECT and present
The resolution adds a **CRITICAL EXECUTION PRE-REQ** block (Part-B STOP-1) and repeats it on the open-question query: all forensic counts must run as a `BYPASSRLS`/superuser role, with a `SELECT current_user, current_setting('is_superuser')` verification before trusting any count, and an explicit "do NOT `SET ROLE` to the app role." This directly closes the safe-direction lie I flagged. Verified grounding: `organizations`/`locations`/`memberships` are `FORCE ROW LEVEL SECURITY` with `tenant_isolation` policies keyed on `app_member_location_ids()` (`core-identity.ts:84-102`), and `customers`/`orders` likewise (`orders.ts:74-85`). The app role with `app.user_id` unset returns zero by policy — the instruction is necessary and correctly stated.
One precision note (non-blocking, the conservative instruction already covers it): `users` itself has **no RLS** (`core-identity.ts:7-17`) and `auth_refresh_tokens` is explicitly `DISABLE ROW LEVEL SECURITY` (`auth-refresh-tokens.ts:19`), so STOP-1 query #2 (refresh tokens) and the bare `users` scan would read true even as the app role. The RLS lie bites only the *JOINed* tenant tables (orgs/locations/memberships/customers/orders). The blanket "run everything as BYPASSRLS" is the right call anyway — it is correct, simpler to follow, and removes the foot-gun of a human deciding per-query which role to assume. No change needed.

### (2) Enum + column names — CORRECT, matches the migrations exactly
- `order_status` is **UPPERCASE** with values `PENDING|CONFIRMED|PREPARING|READY|IN_DELIVERY|DELIVERED|REJECTED|CANCELLED|SCHEDULED|PICKED_UP` — verified verbatim (`extensions-and-enums.ts:14`). The architect's claim holds.
- `payment_outcome` values are `pending|paid_full|paid_partial|refused_payment|refused_goods|customer_cancelled_on_door` — verified verbatim (`extensions-and-enums.ts:16`). There is **no `'paid'` value**; the corrected query's `payment_outcome IN ('paid_full','paid_partial')` is exactly right.
- `orders` has **no `customer_phone`**; it has `customer_id uuid REFERENCES customers(id)` (`orders.ts:24`), and `phone` lives on `customers` (`orders.ts:11`). The rewritten `JOIN customers c ON c.id = o.customer_id` + `count(DISTINCT c.phone)` is correct, and the offered `count(DISTINCT o.customer_id)` phone-free alternative is equivalent and also correct.
- The status filter `status NOT IN ('PENDING','CANCELLED','REJECTED','SCHEDULED')` is sound for "settled/real" — it excludes unconfirmed, abandoned, and not-yet-due scheduled orders while retaining the in-flight-but-real states (CONFIRMED/PREPARING/READY/IN_DELIVERY/PICKED_UP/DELIVERED). Reasonable; the payment_outcome filter is the load-bearing one and it is correct.

### (3) CASCADE ordering note — CORRECT and present
STOP-1 item #4 is present and states the right thing: `memberships` and `auth_refresh_tokens` are both `ON DELETE CASCADE` from `users` — verified (`core-identity.ts:57`, `auth-refresh-tokens.ts:7`). "Delete `empty@dowiz.com`" means **delete the code literal ONLY, never the prod `users` row**, and the mandatory sequence is **forensics first, any user-row cleanup (if ever) second**. This closes the silent-evidence-erasure path I raised. Correct.

### CI/deploy redesign (R2-1: staging gate + prod unauth smoke) — ethical/strategic review
Reviewed for "weaker prod validation as an availability/integrity risk to real future orders." My assessment: **net-positive, no new ETHICAL-STOP.** Reasoning across lenses:
- *Care/integrity of future real orders:* the concern would be real if prod validation were simply *deleted*. It is not. The authenticated lifecycle + telegram suites move to a **staging gate that runs BEFORE prod deploy** (`needs: [validate, staging-e2e]`) against the *same image*, and prod keeps an unauthenticated smoke (health + storefront-read + the 401 negative-auth assertions). The order lifecycle is still proven on identical bits pre-promotion — coverage relocated, not lost. The residual gap (a prod-only config/env divergence the staging image wouldn't catch) is exactly what the R2-2 deploy-time NODE_ENV assert backstops. So integrity of future orders is *better* served than by the prior arrangement, which could only validate prod by minting owner tokens on prod — the very capability being removed.
- *Honesty/consistency:* the redesign refuses the two dishonest shortcuts explicitly (rejected (c) ephemeral real prod tenant — would pollute the STOP-1 forensic tables every deploy, a genuinely elegant catch; rejected (d) a narrow prod validation token — still a prod credential-minting bypass). Refusing to keep *any* token-minting path on prod is the consistent move: you cannot both remove the backdoor and keep a deploy-time backdoor. Good.
- *Long horizon:* moving authenticated E2E to staging is the conventional, durable shape (prod smoke = read-only; write-path proof = pre-prod env). It removes the structural pressure that *created this incident* — "we need a way to mint identities on prod to validate" was the seed of the leaked secret. This redesign closes that pressure at the source. Year-from-now: no regret.
- *One watch-item (non-blocking, already owned):* the order-of-operations correctly sequences "rewire CI (step 1) → ship flag-gated code (step 2) → operator unsets prod secret (step 3)" so prod is never simultaneously backdoored AND unvalidated. The honest failure note (if the operator unsets the secret before step 1, the four prod mock-auth steps go 404 → validation red but deploy still succeeds) is stated plainly. That is a loud, correct fail direction, not a silent one. Owner is named (Implementer + Operator). Adequate.

No surveillance-creep, no dignity issue, no dark-pattern, no a11y regression in this redesign — it is pure pipeline topology. The only strategic cost is a longer deploy pipeline (staging gate adds a stage before prod), which is the right trade for removing prod token-minting.

### Remaining query defects
**None.** All three query corrections (RLS-bypass, customer_id join, paid-status filter) and the CASCADE ordering note are present, correct, and grounded in the actual migrations. The instrument no longer lies in either direction.

### ETHICAL-STOP status — round 2
- **STOP-1 — now ADEQUATELY FRAMED *and* INSTRUMENTED.** Still OPEN as NEEDS-HUMAN-DECISION (it is a human close by design — the operator must run the queries as BYPASSRLS, define `<window-start>`, and record the result), but the prior blocker on my side (queries could read false-clean) is **cleared**. The stop is now satisfiable in substance, not just in form.
- **STOP-2 — OPEN, NEEDS-HUMAN-DECISION, fully adequate.** Unchanged; depends on STOP-1, recorded in `/compliance`. No correction.
- **Open question (R-9) — query CORRECTED and trustworthy.** Still NEEDS-HUMAN-DECISION (operator runs + interprets), but the instrument is sound.

### One-line final readiness verdict
**Confirmed: the four forensic-query defects are correctly fixed and grounded in the live migrations, the CASCADE ordering note is present and right, and the CI/deploy redesign introduces no new ethical or strategic concern (it removes prod token-minting — a net integrity gain for future real orders). Counsel is satisfied: the hardening is ethically ready to ship and the forensic/disclosure decisions are now recordable with confidence. The only thing left is the human act — run the queries as BYPASSRLS, record the judgment, then say "closed."**
