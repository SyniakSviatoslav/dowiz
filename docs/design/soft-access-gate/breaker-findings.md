# Breaker Findings — Soft Access Gate

> Round: R1 · Breaker (Triad) · Date: 2026-06-20
> Target: `docs/design/soft-access-gate/proposal.md` + `docs/adr/ADR-soft-access-gate.md`
> Mode: adversarial. Zero fixes. Each finding grounded in repo source, not the proposal text.

Verdict: the design is mostly sound on the claim-check / RLS-write mechanics, but it
ships **two CRITICAL defects** (boot-time queue creation is structurally blocked; the
rate-limit + ip_hash anti-abuse layer is inert behind the Fly proxy) and several
HIGH/MED gaps where stated invariants are not actually upheld by the cited code.

---

## CRITICAL

### B1 [CRITICAL] B-OPS / B-FAIL · New pg-boss queue cannot be created at boot — silent total notification loss

The design's chosen path for queue provisioning (§8 "Queue registration",
"**Decision: rely on the runtime idempotent `createQueue` at boot**") is structurally
blocked by an already-shipped migration.

- Boot loop: `for (const qName of ALL_QUEUES) await queue.boss.createQueue(qName).catch(...)`
  runs over the **operational pool** (`apps/api/src/server.ts:283-292`).
- `ALL_QUEUES = Object.values(QUEUE_NAMES)` (`apps/api/src/lib/registry.ts:56`) — so the
  new queue does appear in the loop once added to `QUEUE_NAMES`. Good.
- BUT pg-boss v10 backs **each queue with its own partition table** created via DDL
  (`packages/db/migrations/1790000000011_pgboss-bootstrap-schema.ts:49`), and migration
  `1790000000009_pgboss-revoke-runtime-ddl.ts:20` does `REVOKE CREATE ON SCHEMA pgboss
  FROM PUBLIC`. The operational role connecting pg-boss is NOBYPASSRLS with no CREATE.
- Therefore `createQueue('access-request.notify')` for a **genuinely new** queue throws
  (insufficient privilege to create the partition), and the boot loop **swallows it**
  (`.catch((err) => console.warn(...))`, `server.ts:290`).

**Break scenario:** Deploy ships. Boot logs a single warn line nobody reads. Every
submit INSERTs the row fine, then `queue.boss.send('access-request.notify', ...)`
either throws (queue absent) or the job lands in a non-existent queue and is never
worked → **100% of operator notifications silently lost**, indefinitely, with no
exhausted-retry ops alert (the alert in §6 only fires on a job that *ran* and
exhausted retries — a job that was never enqueued produces nothing). The proposal's
own §8 flags this as "an integration step, not a silent assumption," but then
**chooses the blocked path** as the primary. The `APP_QUEUES`-edit fallback is
explicitly dismissed ("editing a past migration won't re-run").

Violated invariant: "Worker not started → jobs accumulate durably… No data loss"
(§6) is false — the *queue itself* never exists, so there is nothing to accumulate
into. Observability claim "<1 min to see truth" (§8) fails: the failure mode is a
swallowed boot warn + zero ops alert.

---

### B2 [CRITICAL] B-SEC / B-SCALE · Rate-limit + ip_hash are inert behind the Fly proxy — `trustProxy` is not set

The entire abuse-control story (§2 "Per-IP: 5 requests/minute", §7 "per-IP
rate-limit (5/min)", `ip_hash` for forensics) depends on `request.ip` being the
client IP. It is not.

- `Fastify({ logger, maxHeaderSize, bodyLimit })` is constructed with **no
  `trustProxy` option** (`apps/api/src/server.ts:138-142`). Default Fastify
  `trustProxy=false` → `request.ip` is the **socket peer**, i.e. the Fly edge proxy's
  internal address, identical for every external client.
- `@fastify/rate-limit` is registered with **no `keyGenerator`**
  (`server.ts:462-465`), so it keys on `request.ip` = the proxy IP.
- The existing public routes already exhibit this latent bug: `telemetry.ts:48` and
  `:103` hash `request.ip`, and `otp.ts:36,:114` fall back to `req.ip`. None set
  `trustProxy`.

**Break scenario A (DoS on legit users):** Because all external traffic shares one
proxy socket IP, the per-IP 5/min limit is effectively a **single global bucket of
5/min for the whole planet**. A launch spike of 20-60/min (the design's own target,
§2) **throttles legitimate users to 429** after the first 5 globally. The proposal's
back-of-envelope ("5/min tolerates a fat-finger… without ever throttling a real
user") is wrong by construction.

**Break scenario B (rate-limit useless against bots):** If `trustProxy` were later
flipped to read `X-Forwarded-For` without pinning to the trusted Fly hop count, a bot
spoofs `X-Forwarded-For` per request → unbounded per-IP buckets → 5/min cap bypassed
entirely → table fills to the `unique(email)` ceiling. Either way the 5/min number
does not deliver what §2 claims.

**B-DATA corollary:** `ip_hash = sha256(proxyIP)` is the **same constant** for all
users → the column has zero forensic value (cannot distinguish or rate-limit by real
client). The "PII-minimal hashed IP" is real, but it is also **useless data**.

Violated invariant: §2 rate-limit math, §7 "per-IP rate-limit (5/min)", and the §9
accepted-risk "5/min/IP… cap damage" all assume per-client IP that the runtime does
not provide.

---

## HIGH

### B3 [HIGH] B-FAIL / B-CONSIST · Enqueue-after-commit gap is a real notification-loss path the design *accepts* but mislabels as covered

§5 "Submit vs notify ordering" and §6 "Queue unavailable at enqueue" claim a crash
between commit and enqueue is "recoverable via `status='new' AND notified_at IS NULL`
sweep." But the **re-notify sweep is a defer-flag** (§9, "re-notify sweep… tracked,
not built"). So in v1 the *only* recovery is "the operator reads the dashboard list"
— and the **admin review UI is also a defer-flag** (§9). 

**Break scenario:** API process is killed (Fly redeploy, OOM) in the window after
`COMMIT` and before/within `boss.send`. The row exists, `notified_at IS NULL`, no job
enqueued, no retry (pg-boss only retries jobs that exist). With both the sweep and the
admin UI deferred, the operator has **no surfaced signal at all** — they would have to
manually run raw SQL they were never told to run. Combined with B1 this is not an edge
case; under B1 it is the *steady state*. The INSERT and `boss.send` are **not in one
transaction** (the repo's enqueue is a plain `boss.send`, `queue-provider.ts:42`, not a
transactional outbox), so atomicity is impossible as designed.

Violated invariant: "the operator dashboard (`status='new'`) is the fallback discovery
path" (§6) — that dashboard does not exist in v1.

### B4 [HIGH] B-SEC · `USING(true)` ops policy on a PII table gives zero row-level defense, and the operational role is shared by the entire app

§3/§7 justify the `FOR ALL USING(true)` policy by analogy to `ops_worker_heartbeat`
(`1780691408625_ops-heartbeat-policy.ts` — verified identical shape). The analogy is
**unsound for a PII table**:

- `ops_worker_heartbeat` holds `(worker_id, last_seen_at)` — non-PII infra state
  (`…ops-heartbeat.ts`). `access_requests` holds raw email.
- **Every** Fastify route shares the single operational `pool` created at
  `server.ts:247` and passed as `db: pool` to ~40 route registrations
  (`server.ts:484,556-581…`). The `USING(true)` policy means the operational role can
  `SELECT *` from `access_requests` with no row predicate.
- Consequence: RLS provides **no containment**. Any present-or-future logic/SQLi flaw
  in *any* route on that shared pool reads the entire email list. The GRANT layer
  (`anon`/`authenticated` revoked) is the *only* boundary, and it is shared
  app-wide — there is no separation between the anonymous capture route and
  authenticated owner routes.

This does **not** break the stated "anon SELECT = 0 rows" invariant (that holds — anon
has no schema USAGE, `1780421100065:51`). It breaks the *implied* "PII is isolated"
posture: a `USING(true)` policy on PII is security theater; the proposal's own caveat
("the GRANT layer is the actual access boundary") concedes RLS does nothing here, yet
ranks it as equivalent to the non-PII heartbeat precedent.

Violated invariant: the §1 red line "DB is source-of-truth… PII surface" minimization
— RLS is claimed as a PII control (§7 "RLS proof") but contributes no isolation for
this table beyond the linter.

### B5 [HIGH] B-CONSIST / B-SEC · Anti-enumeration "identical 200" invariant is not upheld by the cited error paths

§1/§3/§7 red line: "identical `200 {ok:true}` for new / duplicate / honeypot."
Verified against the actual global handlers:

- Zod validation failure returns **HTTP 400** with `{ code:400, error: issues }`
  (`server.ts:516-523`) — not 200. A malformed-but-existing vs malformed-but-new email
  can't be distinguished, so the *new-vs-duplicate* oracle is safe, but the **uniform
  200 claim is false** for the malformed class.
- §6 itself contradicts §3: DB INSERT timeout returns "a generic `503`/`500`."
- **Timing oracle (the real enumeration risk):** §5/§7 assert "the conflict path is a
  single DB round-trip like the insert path." But the **new** path additionally does
  `queue.boss.send(...)` (`queue-provider.ts:42`) after the INSERT; the **duplicate**
  path (`ON CONFLICT DO NOTHING RETURNING id` → 0 rows) does **not** enqueue (§5). So
  new-email latency = INSERT + enqueue round-trip; duplicate-email latency = INSERT
  only. That is a measurable timing delta that **is** a new-vs-existing oracle —
  precisely the thing anti-enumeration must kill. The proposal asserts no timing branch
  exists; the design's own enqueue-on-new-only logic creates one.

Violated invariant: "Anti-enumeration: identical… response… No timing branch that
leaks existence" (§7) — the enqueue-only-on-new branch is a timing branch that leaks
existence.

---

## MEDIUM

### B6 [MED] B-FAIL · `EmailAdapter` on the dispatcher seam either fails the `location_id NOT NULL` audit contract or bypasses audit entirely

§7 routes the operator email through the existing `NotificationDispatcher` with a
synthetic target `{ channel:'email', address: WAITLIST_NOTIFY_EMAIL }`. But the seam's
contracts are tenant-shaped:

- `NotificationTarget` requires `locationId` and `NotificationData` requires
  `locationId` (`provider.ts:1-6,:44`). An access request has **no location** (§7's own
  "exists before any org/location/membership").
- `writeAudit` inserts into `notification_outbox_audit` where
  `location_id uuid NOT NULL` (`1790000000007_notification-outbox-audit.ts:16`). Any
  audit write for this email would violate NOT NULL unless faked with a sentinel UUID.
- The `status` CHECK is `IN ('queued','sending','delivered','failed','archived')`
  (`…007:18-21`, extended by `…010`) — fine, but the location coupling is the break.

**Break scenario:** Either the worker writes audit and **throws on NOT NULL** (turning
a best-effort email into a job failure / retry storm), or it skips audit — in which
case the §8 observability counters (`notify.failed`, audit trail) silently don't exist
for this channel. Reusing the "proven dispatcher seam" is not free: the seam assumes a
tenant.

### B7 [MED] B-SEC / B-OPS · Resend secret absence is graceful, but a present-but-invalid key + free-tier cap have no surfaced signal

§7 adds `RESEND_API_KEY` as `.optional()` (mirrors `***REDACTED***`,
`packages/config/src/index.ts:19`) — verified Resend/WAITLIST are **not yet** in the
env schema (grep: zero hits). Absence → email leg no-ops. Fine.

But:
- §2 concedes a launch spike **exceeds Resend free 100/day**. On the 101st new email,
  Resend returns a 4xx daily-cap rejection → job retries 5× over ~15min
  (`retryLimit:5`) → exhausts → ops alert. With a 500-2000/day spike (§2), that is
  **hundreds of exhausted-retry ops alerts/day** to Telegram-ops = pager-storm /
  alert-fatigue, the opposite of the §2 goal "zero operator pager noise."
- A **present-but-invalid** `RESEND_API_KEY` (typo'd Fly secret) is **not** caught at
  boot (registration is gated only on *presence*, §7) → every notify silently 401s and
  exhausts retries. No boot-time validation of the key.

Violated invariant: §2 "zero operator pager noise" under the exact spike the design
says it is built for.

### B8 [MED] B-CONSIST · At-least-once worker guard can still double-send, and the GDPR-erasure race crashes the worker

§5 "at-least-once → idempotent notify": the `WHERE id=$1 AND notified_at IS NULL`
guard is run **after** the Resend send, and §5 admits "at-worst a rare duplicate only
if two deliveries send before either updates." That is accepted. But two unhandled
races:

1. **Erasure race:** §8 puts `access_requests.email` in scope for the future
   anonymizer (Stage-30 defer-flag), and the operational role has no DELETE today
   (§4) — so v1 is safe. *However*, the moment Stage-30 ships erasure, a row deleted
   between enqueue and worker pickup makes the worker's `WHERE id=$1` point-read return
   0 rows. The proposal asserts (§5) "There is no scenario where the worker reads a
   missing row" — true only because erasure is deferred; it is a latent crash the
   moment the deferred work lands, and nothing in the design records the worker's
   must-handle-missing-row obligation.
2. **Two-worker grab:** pg-boss can hand the same job to two workers on visibility-
   timeout expiry. Both read `notified_at IS NULL`, both send (two operator emails),
   then both UPDATE. §5 calls this "acceptable." For an operator it is noise, not data
   loss — agreed MED, not higher. Flagged only because §5 frames the guard as the
   idempotency key while the send precedes the guard.

### B9 [MED] B-ANTIPATTERN · Soft gate gates nothing — capture is shipped against an open self-serve signup

§9 / ADR precondition: owner onboarding is **open self-serve** — Google login
(`auth.ts:112-118,:138`) and Telegram login (`auth.ts:184-188,:213`) both mint an
`owner` token on first login with no allowlist; onboarding only checks
`requireRole(['owner'])` (`onboarding.ts:30`). The proposal honestly flags this as a
defer-flag and does **not** claim it gates access. Recorded here as MED (not a defect
in *this* design, but a product-truth the Council must not lose): the feature's name
("access gate") implies a control it does not provide. Anyone who fills the waitlist
can, in parallel, just log in with Google and self-provision. The capture has value;
the "gate" framing is aspirational until owner-onboarding-invite-gating ships.

---

## LOW

### B10 [LOW] B-DATA · Forward-only `down()` no-op leaves a PII table on rollback

§4/§8: rollback = "leave the (empty, harmless) table in place." If rollback happens
*after* the table has rows, the PII table persists with no application reading it and
no erasure path (DELETE not granted, anonymizer deferred). Harmless operationally,
but it is an un-erasable PII puddle outside any retention pipeline until Stage-30.

### B11 [LOW] B-SEC · Honeypot via off-screen positioning can be auto-filled by password managers

§10 honeypot is an off-screen `website` field with `autoComplete="off"`,
`aria-hidden="true"`. Real password managers (1Password, Chrome autofill) ignore
`autoComplete="off"` and may populate any text field, especially one named `website`.
A legit user with aggressive autofill trips the honeypot → silent `200` with **no
INSERT** (§6) → the user believes they joined the waitlist but did not. Silent
false-negative on a real user. LOW because rare and non-destructive, but it is a
real legit-user-loss path, and the "silent 200, no INSERT" design makes it
**undetectable** to both the user and ops.

### B12 [LOW] B-CONSIST · localStorage "already submitted" guard blocks legitimate second interest

§10's client-side localStorage gate is trivially bypassed by bots (not a real
control) and simultaneously blocks a legitimate user who wants to register a *second*
email (e.g. work + personal). Cosmetic, but it inverts the intended effect: useless
against the adversary, mildly hostile to the honest user.

---

## Regression note
No prior round. All findings are R1. Re-attack should re-verify B1 (queue DDL grant)
and B2 (`trustProxy`) first — they are the two that turn the whole feature inert.

---

# R2 Re-attack

> Round: R2 (re-attack + regression) · Breaker (Triad) · Date: 2026-06-20
> Target: REVISED `proposal.md` (post-R1-fix + STOP-ETHICS) + `resolution.md` +
> `ethical-decisions.md` + `ADR-soft-access-gate.md`.
> Mode: adversarial, source-grounded. Zero fixes. New IDs `R2-*`. R1 regression verdicts inline.
> Verdict up front: **2 new HIGH** (R2-1 boss.schedule DDL gap; R2-3 consent-400 DoS-amplifier
> real, contradicting resolution.md), **1 new HIGH/CRIT-borderline** (R2-5 `/privacy` 404 to
> non-browser clients — GDPR-notice unreachable, confirmed in code). No new strict-CRITICAL,
> but R2-1 *re-opens B1* for two of the three queues, so B1 is **partially REGRESSED**.

## A. Regression of R1 fixes

### R2-1 [HIGH] B-OPS / B-FAIL · B1-fix only half-closes: `boss.schedule()` for the two NEW cron queues still needs runtime writes the migration never proves — and the reconcile/retention crons are scheduled at runtime, not by Migration B · **B1 PARTIALLY REGRESSED**

Migration B (`1790000000042`, proposal §4) pre-creates **three** queues' partition tables
under the migration role — good for `access-request.notify` (matches the proven
`1790000000011` pattern, verified `1790000000011:113-115` loops `createQueue` over
`APP_QUEUES` under the superuser `MIGRATION_DB_URL`). **But** the two cron queues
(`access-request.reconcile`, `access-request.retention-sweep`) are not driven by
`boss.send` — they are driven by `boss.schedule(...)` **at runtime**, in-process in the
`web` server (every existing cron does exactly this: `server.ts:452`
`queue.boss.schedule(FREE_TIER_WATCH, ...)`; `anonymizer-retention.ts:27`
`boss.schedule(...)`; `dwell-monitor.ts:26`, `courier-cron.ts:21,26`, etc.).

The proposal's retention worker (§5) calls `boss.createQueue(QUEUE)` **then**
`boss.schedule(QUEUE, cron, ...)` at boot — copying `anonymizer-retention.ts:26-27`
verbatim. `createQueue` is now a no-op (queue pre-created — fine). **But `boss.schedule`
writes a row into pg-boss's `pgboss.schedule` table.** The proposal never verifies the
runtime operational (NOBYPASSRLS, no-CREATE) role can `INSERT` into `pgboss.schedule`. The
grant re-apply in Migration B (§4 `DO $g$ … GRANT … ON pgboss.%I TO PUBLIC`) only loops
`information_schema.tables WHERE table_schema='pgboss'` — so it grants DML on whatever
pgboss tables **exist at migration time**. That is the same belt-and-suspenders re-grant
`1790000000011:122-136` and `0009:26-39` already do, so `pgboss.schedule` is almost
certainly grantable — **but the proposal asserts durability for `boss.send` (notify) and
never separately reasons about the `boss.schedule` write-path for the two crons.** The
load-bearing claim "the §6 'jobs accumulate durably' invariant is now true" (§4) is proven
**only for notify**, not for the cron-scheduling writes.

- **Break scenario:** if `pgboss.schedule` was created by a *later* pg-boss internal
  migration than the grant-loop last ran over (pg-boss v10 added the `schedule` table in a
  specific contractor step), the runtime `boss.schedule()` for `access-request.retention-sweep`
  throws at boot → caught by the existing swallow (`rates-refresh.ts:25` shows crons already
  `.catch(...)` their schedule; anonymizer does **not** catch → would surface, but the
  retention worker copies the *anonymizer* shape which throws into the boot try/catch). Net:
  the **12-month auto-erase cron silently never schedules** → GDPR retention auto-erase (a
  STOP-2 hard invariant, §1) **does not run**, and nothing pages (B7 explicitly says
  retention errors don't page — §6 "Retention sweep cron errors → no user-facing surface").
- **Number:** 0 retention sweeps × 365 days = the table grows unbounded past 12 months with
  **no erasure**, while `/privacy` promises "up to 12 months" (§10 copy). The lawful-basis
  promise is then false in production with zero alert.

Violated invariant: §1 "Retention = 12 months, then auto-erase" + §6 "PII persists one
extra day until the next successful tick — bounded, not lost" (it is *not* bounded if the
cron never schedules). **B1 regression:** the B1-fix proof obligation (§8: "a test asserting
the partition table exists + send→drain sets notified_at") covers `notify` only — it does
**not** cover that the two cron queues actually *schedule* under the runtime role.

### R2-2 [HIGH] B-SEC · B2-fix `X-Forwarded-For[0]` fallback is client-spoofable, and `Fly-Client-IP` is asserted but used NOWHERE in the repo — the "real client IP" claim is unproven and the fallback is exploitable · **B2 OPEN (residual)**

Grounded:
- `Fly-Client-IP` appears in **zero** files (`grep -rln 'fly-client-ip' apps/ packages/` → empty).
  The proposal's entire B2-fix rests on Fly injecting + overwriting this header (§2
  `clientIp()`), but there is **no in-repo precedent, no test, and no proof** Fly sets it on
  this app. The one cited precedent, `websocket.ts:118`, reads
  `req.headers['x-forwarded-for'] || req.socket.remoteAddress` — the **raw full XFF string**,
  untrimmed, **for logging only** (`const clientIp = ...`, never a rate-limit/security key).
  It is not evidence that `Fly-Client-IP` exists, nor that XFF[0] is trustworthy.
- **Spoof break:** the §2 helper falls through to `xff.split(',')[0].trim()` whenever
  `Fly-Client-IP` is absent or falsy. If Fly does **not** inject `Fly-Client-IP` (unproven),
  the route trusts the **client-supplied** `X-Forwarded-For` first hop. An attacker sends
  `X-Forwarded-For: <random-per-request>` → unbounded distinct rate-limit keys → the 5/min
  cap is **bypassed entirely** (R1 B2 break B, *unchanged*), and `ip_hash =
  sha256(attackerControlledString)` becomes attacker-controlled garbage — forensically
  useless and a potential **hash-flood** (attacker chooses the stored value). The repo's
  Fastify still has **no `trustProxy`** (server.ts:138-142, re-verified — only `maxHeaderSize`
  + `bodyLimit`), so `request.ip` remains the proxy and the helper *must* fall through to the
  spoofable XFF in any non-Fly-Client-IP path.
- The proposal's claim "in prod `Fly-Client-IP` always wins" is an **assumption stated as a
  fact**; the design provides no boot-time assertion that the header is present, so a Fly
  config change / proxy bypass silently degrades to the spoofable path with no signal.

Violated invariant: §2 "per-IP 5/min … caps a single real IP to ≤7,200/day" and §7 "ip_hash
hashes the real client IP." Both depend on an unverified header. **B2 not fully closed** —
the fix is correct *if and only if* `Fly-Client-IP` is injected, which the repo does not
demonstrate.

### R2-3 [HIGH] B-SCALE / B-SEC · The consent-400 IS a DoS amplifier and a route-fingerprint oracle — directly contradicting resolution.md's "cheaper than a real submit, not a DoS amplifier" · **N1 = REAL, not accepted**

resolution.md (§"Anti-enumeration × consent-400") and proposal §9-N1 both assert the 400
"does **no** DB work (rejected pre-INSERT) so it is *cheaper* than a real submit, not a DoS
amplifier" and "behind the same per-IP 5/min rate-limit, so flood is capped." Grounded
attack on that reasoning:

1. **The 5/min cap is the SAME inert cap as B2 (R2-2).** Because the rate-limit key is the
   spoofable/global value, a `consent:false` flood is **not** capped per-attacker. The whole
   "behind the rate-limit" mitigation inherits B2's defect: spoof XFF → unbounded 400s.
2. **"Cheaper than a real submit" is exactly what makes it the amplifier.** A 400 that
   short-circuits before INSERT is the **cheapest** endpoint on the box, so an attacker
   prefers it: `consent:false` bodies cost the server a Zod/hand-parse + reply, no DB, no
   honeypot no-op. That is a higher requests/CPU ratio than any real path → it is the
   **optimal flood target**, not a benign one. The resolution's logic is inverted: "cheaper"
   = better for the attacker's amplification, not safer.
3. **Timing oracle moved, not killed.** B5 spent effort making new/duplicate/honeypot
   *latency-identical* (enqueue off critical path). The consent-400 path returns on a
   **different, faster branch** (pre-INSERT) than every 200 path (which does at least an
   INSERT attempt or an equivalent honeypot no-op, §6). So `consent:false` vs `consent:true`
   is now a **clean latency + status-code oracle that distinguishes the validation branch** —
   the B5 uniform-latency invariant holds *within* the 200 class but the 400 branch is a
   measurably distinct, faster response. It leaks nothing about *email existence* (true), but
   it re-introduces a "the route branches observably" surface that B5's whole design fought
   to remove, and it is the branch an attacker probes to confirm they've found the
   access-request endpoint (route fingerprint).

Violated invariant: §2 "zero operator pager noise"/"trivial write workload — the risk is
abuse volume" — the cheapest-path 400 maximizes abuse volume per CPU. resolution.md's N1
disposition ("not a DoS amplifier") is **wrong by construction**; N1 should be HIGH-open, not
accept/mitigate.

### R2-4 [MEDIUM] B-FAIL · B5-fix fire-and-forget enqueue runs in the `web` process and is lost on every deploy/restart of a single-machine app — the sweep "self-heals" but on a 15-min + grace delay, and a `void enqueue().catch()` that itself throws synchronously can still surface · **B5 mostly holds, residual loss path**

`fly.toml` confirms a single `web` machine (`auto_stop_machines = false`, `[[vm]] web 512mb`)
and `kill_timeout = "30s"`. The B5-fix replies first, then `void enqueue(...).catch(...)`
(§5/§7). Grounded residuals:
- The detached enqueue is an **unawaited promise in the `web` process**. On any deploy
  (SIGTERM, 30s kill window) or OOM (512mb, sharp image-processing on the same process —
  spa-proxy.ts:184 loads `sharp` in-process), the continuation is dropped. Every row whose
  reply was sent but whose enqueue hadn't flushed is **un-notified until the reconcile cron**
  (15-min cadence + grace, §5). For a launch spike (§2: 60/min bursts) coinciding with a
  deploy, that is up to ~minutes × tens of un-notified rows relying entirely on the cron —
  which itself may not be scheduled (R2-1). So B5's "lost enqueue is caught by the sweep"
  chains onto the R2-1 risk: **if the reconcile cron didn't schedule, the lost enqueue is
  lost for real.**
- B5 timing-oracle is genuinely closed for the **200 class** (good — regression PASS for the
  email-existence oracle). It is **not** closed against the consent-400 branch (see R2-3).

Verdict: B5 substantially holds; flagged MED because the recovery path's reliability is
**coupled to R2-1** (the cron) and to single-machine restart timing.

### R2-9 [LOW] B-CONSIST · B8-fix CAS rollback-on-send-failure creates an infinite-retry loop when send *permanently* fails (bad-but-present `RESEND_API_KEY`, or a domain Resend keeps 4xx-ing) · **B8 holds, but interacts with B7**

§5 B8-fix: on send failure the worker `UPDATE … SET notified_at = NULL WHERE id=$1` and
**throws** so pg-boss retries. Correct for transient failures. But a **permanent** send
failure (invalid-but-present key → 401 every time; or Resend rejects the ops address) means:
claim → send 401 → rollback notified_at=NULL → throw → retry (≤5, ~15min) → exhaust. Per B7,
exhaustion "does not page" and the row stays `notified_at IS NULL`. The reconcile cron then
**re-enqueues that same permanently-failing row every 15 min forever** (§5 reconcile predicate
is exactly `notified_at IS NULL AND created_at < now()-grace`). So a single bad address
produces an **unbounded re-enqueue + 5-retry-burst every 15 min, indefinitely**, and B7's
"one aggregated alert" fires once then the backlog persists (the row never clears). Not data
loss; it is a **steady low-grade retry treadmill** with one stale alert. The proposal's claim
that exhaustion is terminal (§2 "~15 min total before exhaustion") is false once reconcile
re-feeds it. LOW (bounded CPU, one row), but the "self-heals" framing is wrong for permanent
failures.

## B. New R2 risks (consent + 12mo retention + /privacy)

### R2-5 [HIGH] B-SEC / B-OPS · `/privacy` returns a hard JSON 404 to any non-`text/html` client (regulator tooling, curl, crawler, link-checker) — the GDPR notice is unreachable on exactly the access pattern a regulator uses · **N5 = CONFIRMED BROKEN in code**

The proposal (§8, N5) says "verify `spa-proxy.ts` serves `/privacy`." The actual SPA fallback
is **not** in `spa-proxy.ts` (that file is API routes only) — it is `server.ts:869-880`:

```
const SPA_ROUTES = ['/admin', '/courier', '/dashboard', '/s/', '/login', '/branding-preview'];
fastify.setNotFoundHandler((request, reply) => {
  if (request.method === 'GET' &&
      (request.headers.accept?.includes('text/html') ||
       SPA_ROUTES.some(prefix => request.url === prefix || request.url.startsWith(prefix + '/'))))
    return reply.sendFile('index.html');
  reply.status(404).send({ error: 'Not found', path: request.url });
});
```

`/privacy` is **not** in `SPA_ROUTES`. So serving the shell depends **entirely** on the
request carrying `Accept: text/html`. Break:
- A browser user clicking the consent-label link → `Accept: text/html` → shell served → OK.
- A **regulator / DPO tool / `curl` / headless link-checker / many crawlers / a Slack/email
  link-unfurler** sends `Accept: */*` or `application/json` or no/`*/*` accept → **hard
  `404 {error:'Not found'}`**. The GDPR notice the consent record points to is **unreachable**
  by precisely the non-browser audit clients that verify it exists. `ethical-decisions.md`
  states "A link-to-404 is itself a GDPR failure" — this is a link-to-404 for non-browser
  clients, in the shipped code, unless `/privacy` is added to `SPA_ROUTES`.
- The proposal's N5 mitigation ("`main.tsx` has a `*` catch-all") is **client-side only** —
  it never runs because the server 404s before any HTML/JS is delivered to a non-`text/html`
  client.

Violated invariant: STOP-2d / §8 "the link must resolve (no link-to-404, which would be a
GDPR failure)." Confirmed broken against `server.ts:870`. HIGH because it is a
legal-compliance surface the design explicitly made load-bearing, demonstrably failing for
audit-class clients.

### R2-6 [MEDIUM] B-DATA / B-CONSIST · `privacy_version` is a build-time constant the design admits can drift from the rendered page, with NO enforcement that a copy edit bumps it — the consent record can attest to text the user never saw · **N2 = under-mitigated**

§8: "Minimal version scheme: a date string (e.g. `'2026-06-20'`) — no version table needed."
§5: `privacy_version = PRIVACY_NOTICE_VERSION` (a constant). The mitigation (N2) is "both
single-sourced; the page renders the same constant; proof obligation: a test asserting
rendered version === constant." Grounded gap:
- "Single-sourced" only guarantees the **page's displayed version label** equals the
  **stored** version. It does **NOT** guarantee the version was **bumped when the prose
  changed**. The version is a hand-maintained constant in `PrivacyPage.tsx` + a config; an
  engineer edits the privacy copy (retention sentence, basis wording) and **forgets to bump
  the date string** — the proposed test still passes (rendered label === stored constant ===
  unchanged old date), and every new consent row attests `privacy_version='2026-06-20'` while
  the actual text is the v2 prose. The audit trail now **lies**: rows claim consent to a
  version whose text no longer matches.
- There is no content hash, no table snapshotting the notice text per version, and (N4)
  `ON CONFLICT DO NOTHING` means re-consenters keep the *first* version forever. So a
  material notice change is invisible in the consent evidence for both new (un-bumped) and
  returning (stale) users.

Violated invariant: §1 "the row records … `privacy_version` (which notice)" — it records a
*label*, not a binding to the *text*, and nothing forces the label to move when the text
does. The N2 proof obligation tests the wrong thing (label===label, not label===text).

### R2-7 [MEDIUM] B-CONSIST · `ON CONFLICT (email) DO NOTHING` + retention DELETE = a re-submitting user's consent silently resets the clock OR is permanently stale — and the "12-month" promise is measured from FIRST submit, not last contact · **N3/N4 interaction, new**

Combining the accepted N3 (retention DELETE on `created_at`) and N4 (`DO NOTHING` keeps first
consent):
- `created_at` is set once on the first INSERT and **never updated** (DO NOTHING). The
  retention sweep deletes `WHERE created_at < now() - '12 months'` (§5). So a user who
  re-submits interest at month 11 (re-affirming consent client-side, ticking the box again)
  has their row **deleted at month 12 from the *original* submit** — i.e. one month after
  they actively re-engaged. The operator loses an **active, freshly-consented** lead because
  retention keys on first-seen, and `DO NOTHING` refuses to refresh `created_at`/`consent_at`.
- Conversely the stored `consent_at`/`privacy_version` is frozen at first submit (N4), so the
  evidence is for the *oldest* consent even though the user just re-consented under possibly
  newer copy (R2-6). Either reading is wrong: the row both **over-retains** relative to last
  consent (keeps old version) and **under-retains** relative to last contact (deletes 12mo
  from first, not last).

Violated invariant: §10 copy "we store your email … for up to 12 months" — ambiguous against
a re-consenting user; the implementation measures from first submit, the user reasonably reads
it as from last contact. resolution.md calls N4 "intended semantics"; the **retention-clock
interaction** with `created_at` is *not* analyzed anywhere and is a genuine new gap.

### R2-8 [MEDIUM] B-CONSIST / B-SEC · Consent Zod `.strict()` with `consent: true` literal — the route's own lenient self-parse (B5) BYPASSES the strict schema, so the literal-`true` guard depends entirely on hand-written code the design only describes, not the schema · **new**

§5: the typed surface is `{ email, website?, locale?, consent: true }` "Zod `.strict()`",
**but the route still hand-parses** to preserve the B5 uniform-200 for email/honeypot.
Grounded tension:
- The repo's global Zod handler returns 400 on schema failure (`server.ts` content-type
  parser + per-route `schema:{body}` like `otp.ts:35`). The proposal **explicitly does NOT
  use schema-gating** for this route (§7 point 3: "this route does its **own** body parse and
  never relies on the schema validator to gate"). So the `.strict()` / `consent: true` literal
  is **decorative** — it documents intent but is not the enforcement path. Enforcement is
  whatever the hand-parser checks.
- Therefore "the server REALLY rejects no-consent before INSERT" is true **only if** the
  hand-parser checks `body.consent === true` with strict equality. The design describes this
  (§5) but, because it deliberately avoids the schema gate, there is **no schema-level
  backstop**: a coding slip (`if (body.consent)` truthy-check instead of `=== true`) would
  silently accept `consent: 1`, `consent: "true"`, `consent: "false"` (non-empty string is
  truthy) → a row written without a real boolean-true consent, while `consent_at NOT NULL`
  (set unconditionally in the handler, §4) still satisfies the DB constraint → **lawful-basis
  invariant violated with the DB happily attesting consent.** The `NOT NULL` "belt-and-
  suspenders" (§4) does **not** catch this: it checks presence of a timestamp the handler
  always sets, not the truth of the consent boolean.

Violated invariant: §1 "No row is written without a server-validated `consent === true`." The
guarantee is asserted to live in a hand-parser that the design routes *around* the only
schema enforcement the framework provides; nothing structural enforces strict-`true`.

### R2-10 [LOW] B-ANTIPATTERN · Launch-gate (§8 STOP-1) is "a release-sequencing gate, not a code flag" — i.e. a comment, with no enforcing mechanism · **invite-gating prerequisite is non-blocking in code**

§8 "Launch gate": "This is a **release-sequencing gate, not a code flag**." So the hard
blocking prerequisite (STOP-1, invite-gating must ship first) is enforced **only by human
discipline / a checklist**, with nothing in the artifact that mechanically prevents the CTA
shipping before invite-gating. Given that owner onboarding is *verified* open self-serve
(`auth.ts` mints owner on first Google/Telegram login — re-confirmed via resolution.md
citations; the door is genuinely open), a "soft access gate" that launches without the
prerequisite is a dark-pattern (Counsel STOP-1) — and the only thing stopping that is a
sentence in §8. No `FEATURE_*` flag, no CI gate, no assertion. LOW (process, not code defect),
but the strongest invariant in the whole design (STOP-1 "blocking prerequisite") rests on the
weakest enforcement (a comment).

## Regression summary (R1 → R2)

| R1 ID | R2 status | Note |
|-------|-----------|------|
| B1 (CRIT) | **PARTIALLY REGRESSED** → R2-1 | notify queue fixed; the two **cron** queues' runtime `boss.schedule` write-path is unproven → retention auto-erase may silently never schedule. |
| B2 (CRIT) | **OPEN (residual)** → R2-2 | fix is correct *iff* `Fly-Client-IP` is injected — used **nowhere** in repo, no proof; XFF[0] fallback stays spoofable; `trustProxy` still unset (server.ts:138-142 re-verified). |
| B3 (HIGH) | holds, **coupled to R2-1** | reconcile cron is the recovery path; if it doesn't schedule (R2-1), B3's recovery evaporates. |
| B4 (HIGH) | holds (accept-risk unchanged) | shared operational pool reads all emails — still true; not re-litigated. |
| B5 (HIGH) | **holds for 200-class**; residual R2-4 + oracle re-opened by R2-3 (consent-400 branch). |
| B6 (MED) | holds (direct EmailAdapter avoids `location_id NOT NULL`). |
| B7 (MED) | holds, but R2-9 shows reconcile re-feeds permanently-failing rows → one stale alert + treadmill. |
| B8 (MED) | holds; R2-9 infinite-retry-on-permanent-failure residual (LOW). |
| B9 (MED) | holds; R2-10 shows the STOP-1 prerequisite has no code enforcement. |
| B10–B12 (LOW) | unchanged. |
| N1 | **REJECTED accept** → R2-3 HIGH (DoS amplifier real, contradicts resolution.md). |
| N2 | under-mitigated → R2-6 MED (version label ≠ text binding). |
| N3 | new interaction → R2-7 MED (retention clock × first-submit `created_at`). |
| N4 | confirmed intended but → R2-7 (clock interaction unanalyzed). |
| N5 | **CONFIRMED BROKEN** → R2-5 HIGH (`/privacy` 404 to non-`text/html` clients, server.ts:870). |

## New CRITICAL/HIGH for the conductor
- **R2-1 [HIGH]** retention/reconcile cron `boss.schedule` durability unproven → 12-month
  auto-erase may silently never run, no page. (B1 partial regression.)
- **R2-2 [HIGH]** B2-fix depends on `Fly-Client-IP` that appears **nowhere** in repo; XFF[0]
  fallback spoofable; rate-limit + ip_hash still defeatable. (B2 residual-open.)
- **R2-3 [HIGH]** consent-400 IS a cheapest-path DoS amplifier + route-fingerprint oracle —
  resolution.md's "not a DoS amplifier" is inverted; N1 should be HIGH-open.
- **R2-5 [HIGH]** `/privacy` hard-404s to every non-`text/html` client (regulator/curl/crawler)
  — confirmed at `server.ts:870`, `/privacy` absent from `SPA_ROUTES`. GDPR link-to-404.

No new strict-CRITICAL, but R2-1 + R2-2 each re-open a former CRITICAL as a residual HIGH:
the two anti-abuse / durability fixes are **not yet provably closed**.

---

# R3 Focused verification

> Round: R3 (focused R2-fix verification + regression) · Breaker (Triad) · Date: 2026-06-20
> Target: REVISED `proposal.md` (post-R2-RESOLVE) + `resolution.md` §"R2 resolution" +
> `ADR-soft-access-gate.md`. Mode: adversarial, code-grounded. Zero fixes. New IDs `R3-*`.
> Scope: verify the 4 R2-HIGH fixes are closed *by mechanism*, MED-fix regression, new-hole scan.
>
> Ground-truth read this round (verified myself against working tree, not proposal text):
> - `server.ts:289-292` boot `createQueue` loop (`.catch(console.warn)` swallow — unchanged).
> - `server.ts:401` `anonymizerRetentionWorker.start()`; `:428` rates; `:440` liveness —
>   the worker-start block is a sequence of **top-level `await`s inside `main()`** with **no
>   surrounding try/catch** (the only try/catch in `main` are `:295-304` queue-table check
>   and `:902-908` `fastify.listen`).
> - `server.ts:905` `await fastify.listen(...)` runs **AFTER** every worker `.start()`.
> - `server.ts:913` `main();` invoked **bare — no `.catch()`** → a reject routes to the
>   `process.on('unhandledRejection', … kept alive)` guard (`:129-135`).
> - `anonymizer-retention.ts:27` `boss.schedule(...)` is **NOT** `.catch`-wrapped (confirmed);
>   `rates-refresh.ts:25` and `reconciliation.ts:41` **ARE** `.catch`-wrapped.
> - `0009:45-48` + `0011:139-142` `ALTER DEFAULT PRIVILEGES … ON TABLES … TO PUBLIC`;
>   `0009:31-36`/`0011:128-136` grant-loop over existing pgboss BASE TABLEs.
> - `server.ts:870` `SPA_ROUTES = ['/admin','/courier','/dashboard','/s/','/login','/branding-preview']`
>   — `/privacy` still absent in HEAD; `setNotFoundHandler` (`:871-880`) is `Accept`-OR-prefix.
> - `server.ts:516-523` global error handler: `error.validation` → **400**; else `statusCode ||
>   500` → **500**. There is no route-level path that converts a thrown handler error to 200.
> - `otp.ts:36` keyGenerator precedent (`req.body?.phone || req.ip`); no `Fly-Client-IP` anywhere.

## Per-R2-HIGH verdict

### R2-1 (mirror proven cron) — **OPEN-RESIDUAL (downgraded to MED-effect, but a NEW boot-fragility surfaced → R3-1 HIGH)**

**Grant-path claim: CLOSED.** The architect's core assertion holds against source.
`anonymizer.retention` does `boss.schedule(ANONYMIZER_RETENTION, cron, …)` at
`anonymizer-retention.ts:27` and is started on prod at `server.ts:401` — so the runtime
NOBYPASSRLS operational role **provably** writes `pgboss.schedule` today (16 live schedulers
confirmed, `:452/:401/:428/:440/backup/courier/settlement/dwell/...`). `pgboss.schedule`
DML-grantability is covered whether the table pre- or post-dates the grant-loop: `ALTER
DEFAULT PRIVILEGES … ON TABLES` (`0009:45`, `0011:139`) covers tables created *after* it ran,
and the grant-loop (`0011:128`) covers tables existing *at* migrate. The R2-1 "schedule table
postdates grants" break is genuinely disproven. **No new migration needed — correct.**

**No queue is on a runtime-CREATE path that REVOKE blocks: CLOSED.** All three queues
(`notify`/`reconcile`/`retention-sweep`) get their partition tables from Migration B under the
migration role; runtime `createQueue` at `server.ts:290` is a no-op. Verified the boot loop
iterates `ALL_QUEUES = Object.values(QUEUE_NAMES)` (`registry.ts:56`), so adding the three
names makes them no-op-verified, not CREATE-attempted. CLOSED.

**Boot-assert claim: NOT SPECIFIED as a real mechanism — and the copy-the-anonymizer-shape
decision is actively HARMFUL given the real boot topology → see R3-1.** The proposal says the
retention worker "copies the anonymizer shape exactly — `schedule` is NOT wrapped in `.catch`,
so a genuine schedule failure surfaces to the boot path instead of being swallowed"
(proposal §5, R2-1 block). I verified the boot topology this depends on, and the claim is
**inverted by the actual control flow**: see R3-1 below. The "boot-assert that both schedule
rows exist post-boot" is described as a *proof obligation* (§"Proof obligations"), i.e. a test
to be written — it is **not** a runtime self-check that fails the deploy; it is a CI/test
aspiration with no specified failing assertion in the artifact. So "boot-assert" ≠ a live
guard. **OPEN-residual** (the durability is real; the *visibility-of-failure* is not).

### R2-2 (Fly-Client-IP only) — **CLOSED (design reads Fly-Client-IP exclusively; no XFF trust) — one residual DoS-shape flagged R3-2 LOW**

Verified the §2 `clientIp()` helper: it reads `req.headers['fly-client-ip']` only, then in
prod returns the constant `'no-fly-client-ip'` (fail-closed shared bucket), then non-prod
returns `req.ip`. **The R1/R2 spoofable `X-Forwarded-For[0]` fallthrough is genuinely gone** —
grep confirms no XFF read in the helper. **CLOSED** on the spoof vector (R2-2's load-bearing
break).

- **Local fallback active in prod? NO.** The `req.ip` branch is gated behind
  `if (process.env.NODE_ENV === 'production') return 'no-fly-client-ip'` *before* it — so prod
  can never reach `req.ip`. Correct; matches the design. CLOSED.
- **Fail-closed → DoS? Real but ACCEPTED-shape (R3-2 LOW).** If `Fly-Client-IP` ever
  disappears in prod (Fly config change / internal direct-hit), every request keys on the
  single constant `'no-fly-client-ip'` → one global 5/min bucket → **planet-wide 429** during
  the outage. This is the *same* self-throttle B2-A described, just relocated to a
  misconfig-only branch. The design names it explicitly ("fail-closed … throttles everyone
  (safe, noticed)") and pairs it with a boot-warn, so it is a *documented* degrade, not a
  silent hole. It is strictly safer than trusting a spoofable header. **Accept-shape; logged
  R3-2 LOW** only because the boot-warn is "one-time on first request" — a Fly change *after*
  the first good request would not re-warn (the degrade could go unnoticed mid-life). Not a
  blocker.

### R2-3 (drop 400, fold no-consent into silent 200) — **CLOSED for the stated threat, but the inversion created a real observability/lawful-loss class → R3-3 HIGH**

**Does it hide real errors (malformed JSON / 500) under 200? NO for 500, NO for malformed —
verified against the actual error handler.** The architect's claim "no non-200 for any
well-formed-or-malformed body; the only non-200 is a DB-503 transport failure" is **only true
if the handler does its own try/catch and self-replies 200 on every parse path**. Ground-truth:
the global handler (`server.ts:516`) returns **400** on `error.validation` and **500** on any
thrown error. So:
- A **malformed JSON body** (not valid JSON at all) is rejected by Fastify's content-type
  parser **before** the route handler runs → **400**, regardless of the route's internal
  lenient parse. The "uniform-200 even for malformed" invariant is **false for non-JSON
  bodies** — the route never sees them. This is a *route-fingerprint leak the design claims it
  closed* (a `Content-Type: application/json` + `{` truncated body → 400; a valid-JSON
  no-consent body → 200). The branch the design fought to remove (R2-3) is **re-introduced one
  layer up** at the JSON parser. Distinct status for malformed-vs-valid = the same fingerprint
  class R2-3 named. **This is R3-3.**
- A genuine **INSERT 5xx** (DB down mid-submit): if the handler `await`s the INSERT and lets
  it throw, the global handler emits **500** — which *contradicts* "only non-200 is DB-503"
  unless the route catches and self-maps to 503. The design says "DB 503" but the framework
  default is 500; nothing in the artifact specifies the catch→503 mapping. Minor, but it means
  the "uniform contour" is narrower than claimed.

**Silent loss of legit lead (B11/honeypot-autofill class): OPEN, now WIDER.** This is the real
cost of the inversion. Before R2-3, a no-consent body got a **400** the frontend could surface
("tick consent"). After R2-3, a no-consent body — *including a frontend bug where consideration/
consent serialization sends `consent: "true"` (string) or omits it* — returns **silent
200 `{ok:true}`**, no row, no notify, no signal to user OR ops. The user sees "thanks, we'll be
in touch"; the lead is **gone with zero trace**. The `z.literal(true)` gate makes this *more*
likely to bite a real user than the old hand-`if`, because any client/serialization drift
(`true`→`"true"`, boolean→`1`, a JSON lib quirk) now silently drops instead of erroring. The
honeypot-autofill path (B11) is folded into the **same** silent-200, so a password-manager that
fills the renamed honeypot field also silently drops a real user with no observability. **The
lawful-basis red line holds (no row without true-consent — CORRECT), but the *availability* red
line "frictionless capture" is violated for an un-measurable fraction of real users, and there
is now NO counter that distinguishes 'silently-dropped-no-consent' from 'never-submitted'.**
This is R3-3 (HIGH): the inversion trades a *loud* lawful-basis-safe 400 for a *silent*
lead-loss + a *new* 400-fingerprint at the JSON-parser layer. The design asserts "an equivalent
cheap no-op for timing parity" but specifies **no telemetry counter** for the dropped-body
count, so the blind spot is total.

### R2-5 (/privacy in SPA_ROUTES) — **CLOSED-CONDITIONAL (mechanism correct; NOT YET in HEAD)**

The fix is mechanically correct *as specified*: `setNotFoundHandler` (`server.ts:871-880`)
serves `index.html` when `request.method === 'GET' && (Accept includes text/html OR
SPA_ROUTES prefix-matches)`. Adding `'/privacy'` to the `SPA_ROUTES` array makes the
**prefix branch** fire **independent of `Accept`** — so `GET /privacy` with `Accept: */*` →
`index.html` 200. The branch is genuinely `Accept`-agnostic (it is an `OR`), and there is **no
earlier 404 interceptor** — `setNotFoundHandler` is the terminal fallback; all real routes are
registered before it, and none match `/privacy`. **Mechanism: CLOSED.**

- **Caveat (verification, not a new finding):** `/privacy` is **still absent** from
  `SPA_ROUTES` in HEAD (`server.ts:870`, re-confirmed this round). The fix is a one-line
  *design instruction*, not yet applied code. The proof obligation (`GET /privacy` `Accept: */*`
  → 200) is correct and testable. As a design verdict: CLOSED. As a shipped-state: unbuilt
  (expected — this is design phase). No finding; flagged so the conductor does not read
  "CLOSED" as "merged."

## MED-fix regression

- **R2-8 (`z.literal(true)` structurally BEFORE email contour): CLOSED structurally, but the
  structural-vs-hand-parse tension R2-8 raised is NOT fully dissolved.** The design now states
  a dedicated `ControlFields = z.object({ consent: z.literal(true), website: …, locale: … })`
  parse runs *before* the lenient email parse (proposal §5). `z.literal(true)` does reject
  `"true"`/`1`/missing structurally — verified that is real Zod behaviour. **However** this Zod
  object is parsed *by hand in the handler* (`ControlFields.safeParse(body)`), **not** via
  Fastify `schema:{body}` (the design explicitly avoids schema-gating to preserve uniform-200,
  §7). So it is "structural" in the sense of *being a Zod literal* but still *invoked by
  hand-written control flow* — if an engineer wires the email parse first, or forgets to branch
  on `ControlFields.safeParse().success`, the literal never runs. It is **less** slip-prone than
  a raw `if` (the slip moved from "truthy-check bug" to "forgot-to-call-safeParse bug"), so R2-8
  is **improved, not eliminated**. Regression: PASS (no worse than R2-8 disposition); residual
  noted, not escalated. The R3-3 silent-loss consequence (above) is the *downstream* cost of
  this being a silent-200 on fail.

- **R2-10 (CI launch-gate: `ACCESS_GATE_PUBLIC_ENABLED` default-off + banned-strings gated on
  invite-gating-shipped): PARTIAL — the flag is NOT specified as a runtime route-registration
  gate, so the "CTA route is live despite off" hole is OPEN → R3-4 MED.** The disposition says
  "the CTA route/page is not rendered in prod while false" (resolution R2-10) — but **who reads
  the flag, and where, is unspecified.** Two distinct enforcement points exist and the design
  conflates them:
  - **Frontend render gate** ("page is not rendered"): a build-time flag that omits the CTA from
    the SPA bundle. This is real *if* the bundler tree-shakes on it.
  - **Backend route registration**: `POST /api/access-requests` is registered in `server.ts`
    like every other route (the ADR §Decision-5 bakes the path into the route file). **Nothing
    in the artifact gates that `fastify.register(...)` on `ACCESS_GATE_PUBLIC_ENABLED`.** So if
    the flag only hides the *frontend CTA*, the **backend endpoint is live and publicly
    POST-able** the moment the migration ships — a scripted client can submit/seed
    `access_requests` (and trigger operator emails) **before invite-gating ships**, which is
    exactly the STOP-1 "door open before the gate is real" condition the flag was promoted to
    prevent. The banned-strings CI test only greps i18n copy — it does **not** assert the route
    is unregistered. **R3-4 (MED):** the launch-gate flag enforces *copy honesty* and maybe
    *frontend render*, but does **not** mechanically prevent the *backend capture endpoint* from
    being live pre-gating. The "CTA is dead while off" claim is unproven for the API surface.
    (Regression vs R2-10: the flag is a real mechanism for copy, NOT proven for route-liveness —
    the R2-10 "flag that nothing enforces in runtime" suspicion is *partially confirmed* for the
    route layer.)

- **R2-6 (content-hash CI test): SPECIFIED as a real mechanism, regression PASS.** The design
  states a CI test hashes the rendered notice prose strings and fails if the hash changes
  without `PRIVACY_NOTICE_VERSION` bumping (proposal §5, resolution R2-6). That is a genuine
  mechanical backstop for the R2-6/N2 "edit prose, forget to bump" gap — it tests
  label-binds-to-text, not the old label===label. CLOSED as a design mechanism (test must be
  written; assertion is well-defined: `hash(prose) === recorded_hash_for(VERSION)`).

## New findings (R3-*)

### R3-1 [HIGH] B-OPS / B-FAIL · The retention/reconcile cron's un-`.catch`'d `boss.schedule` runs BEFORE `fastify.listen()` in an un-guarded boot block → a schedule throw aborts the *entire* `main()`, and the `unhandledRejection`-"kept alive" guard turns it into a half-booted process: workers stopped, HTTP **never starts**, OR (worse) deploy looks healthy while crons silently never scheduled

The R2-1 "fix delta" — *copy the anonymizer shape, do NOT `.catch` the schedule, so a genuine
failure surfaces to boot* — is **inverted by the real boot topology**, which I verified:

- The worker `.start()` calls live as **top-level `await`s in `main()`** with **no surrounding
  try/catch** (`server.ts:371-440`; the only try blocks in `main` are the queue-table check
  `:295-304` and `fastify.listen` `:902-908`). `anonymizer-retention.ts:27`'s un-`.catch`'d
  `boss.schedule` therefore rejects **straight up through `main()`**.
- `fastify.listen()` is at `server.ts:905` — **after** every worker start (`:401`, `:428`,
  `:440`). So if the new retention worker's `boss.schedule` throws at `:401`-equivalent, the
  reject propagates out of `main()` **before line 905 ever runs**.
- `main()` is invoked **bare at `:913` (`main();`, no `.catch`)**. The reject becomes an
  `unhandledRejection` → caught by the `:129` guard that **logs and keeps the process alive**
  (explicitly: "suppresses Node's default crash-and-exit"). 

**Break scenario:** the new retention worker's `boss.schedule('access-request.retention-sweep',
…)` throws for *any* reason (a real pgboss internal-migration race, a transient pgboss schema
lock, a genuine grant gap on a fresh env, a pg-boss version skew). Because it is un-`.catch`'d
and the boot block is un-guarded:
1. `main()` rejects at the worker-start line.
2. `fastify.listen()` at `:905` is **never reached** → the web process **serves no HTTP** (no
   `/livez`, no `/readyz`, no API) — but `process.on('unhandledRejection', kept-alive)` keeps
   the process **running**, so Fly sees a live process that *fails its HTTP health check* → the
   machine flaps/restarts in a loop, OR if `/livez` is TCP-only it looks "up" while 100% of
   requests hang. The `.catch`-less choice doesn't "surface to boot" cleanly — it **poisons the
   whole boot sequence** because the schedule is *upstream* of `listen()`.

Contrast: `rates-refresh.ts:25` and `reconciliation.ts:41` **do** `.catch` their schedule —
precisely so a schedule failure does **not** abort the boot sequence. The design's decision to
copy the **anonymizer** (un-`.catch`'d) shape is the **minority pattern**, and it is only
"safe" for the anonymizer because the anonymizer has been running successfully on prod for
months (its schedule never throws). A **brand-new** queue's first schedule is the *least*
proven write in the system, and the design hangs it un-`.catch`'d *before* `listen()`. The
R2-1 claim "a genuine schedule failure surfaces to the boot path instead of being swallowed" is
**true but catastrophic**: it surfaces by **taking down HTTP**, not by a clean fail-fast exit
(there is no `main().catch(() => process.exit(1))`).

Violated invariant: proposal §"R2-1" "a real failure surfaces to boot, not silenced" — it
surfaces as a **half-booted no-HTTP process**, not a clean signal; and the §1/§OPS "visibility
of a failed deploy <1 min" is *worse* than the swallow it replaced, because the failure mode is
"process alive, HTTP dead, crons unscheduled" which the `unhandledRejection`-kept-alive guard
specifically masks from a crash-loop detector. Also re-opens R2-1's own concern: the
"boot-assert both schedule rows exist" is a *test*, not a runtime guard, so nothing live catches
"scheduled-zero-of-two."

### R3-2 [LOW] B-SCALE / B-OPS · Fail-closed shared-bucket boot-warn is one-time-on-first-request → a Fly-Client-IP loss that begins AFTER the first good request degrades the whole planet to one 5/min bucket with no fresh warning

Verified the §2 design: the boot-assert/warn fires "on the first real prod request" if
`Fly-Client-IP` is absent. If the first request *has* the header (normal), the warn never arms;
a *later* Fly-edge change that drops the header silently routes every client to the
`'no-fly-client-ip'` constant bucket → global 5/min 429 with **no re-warn** (the one-time latch
already passed). Bounded (config-change-only, and it fails *safe* — over-throttle, not
under-protect), so LOW; flagged because the design frames the degrade as "visible, not silent"
and the visibility is only guaranteed at boot/first-request, not continuously.

### R3-3 [HIGH] B-CONSIST / B-SEC / B-OPS · The R2-3 inversion (silent-200-on-everything) creates an unmeasurable real-user lead-loss class AND re-introduces a malformed-vs-valid 400 fingerprint at the JSON-parser layer the route can't reach

Two grounded sub-breaks (full reasoning in the R2-3 verdict above):

1. **Silent lead-loss, now wider than B11 and un-observable.** Folding no-consent +
   honeypot-trip + any client serialization drift (`true`→`"true"`, missing field, `1`) into
   one silent `200 {ok:true}` with **no INSERT and no counter** means a real consenting user
   hit by a frontend/serialization bug receives "we'll be in touch," leaves no row, triggers no
   notify, and produces **zero ops signal**. There is no metric in the design that separates
   "silently-dropped body" from "never-submitted," so the loss rate is **structurally
   un-knowable**. The old consent-400 was loud (frontend could surface it); the inversion
   trades a lawful-basis-safe loud signal for a silent availability hole. (Lawful basis itself
   is intact — *no row without true-consent* holds — but the *frictionless-capture* invariant,
   §1, is silently violated for an unmeasurable fraction.)

2. **Malformed-JSON 400 fingerprint the route never sees.** Verified `server.ts:516`: a body
   that is not valid JSON is rejected by Fastify's content-type parser → global handler → **400
   `{code:400}`**, *before* the route's lenient self-parse runs. So `POST /api/access-requests`
   with a truncated/garbage body returns **400**, while a valid-JSON no-consent body returns
   **200** — a clean status-code oracle distinguishing "this is the access-request route"
   (the exact route-fingerprint surface R2-3 claimed to eliminate by "no non-200 for any
   well-formed-or-malformed body"). The claim is false: the route cannot make the JSON parser
   return 200; "malformed body → 200" is unachievable at the route layer.

Violated invariant: proposal §1 "frictionless public expression-of-interest" + the R2-3 red
line "the route emits **no non-200 for any well-formed-or-malformed body**" — both broken:
non-200 (400) *is* emitted for malformed bodies at the parser layer, and well-formed-but-drifted
bodies are silently lost with no observability. HIGH because it is simultaneously (a) a real,
unmeasurable user-loss path and (b) a re-opened fingerprint the round claimed closed.

### R3-4 [MED] B-OPS / B-ANTIPATTERN · `ACCESS_GATE_PUBLIC_ENABLED` is specified to hide the CTA *render* but NOT to gate backend route registration → the capture endpoint is live & publicly POST-able before invite-gating ships, defeating the STOP-1 prerequisite the flag was promoted to enforce

Verified (full reasoning in MED-regression above): the design specifies the flag hides the
*frontend CTA render* ("the CTA route/page is not rendered in prod while false") and a CI
banned-strings grep over i18n copy — **neither gates `fastify.register` of the
`POST /api/access-requests` route.** The route path is baked into the route file (ADR
§Decision-5) and registered unconditionally like every sibling. So with the flag off:
frontend shows no CTA, but the backend endpoint **accepts submits** (seeds `access_requests`,
emits operator email, exercises the whole PII pipeline) for anyone who knows the path. The
STOP-1 invariant ("door must not be open before the gate is real") is enforced for the *visible*
surface but **not** the *reachable* one. R2-10's "is the flag a real runtime enforcer or just a
prapor" suspicion is **confirmed for the route layer**: it is render-only. MED (the data is
consent-gated and erasable; the breach is "endpoint live pre-gating," not a data leak), but it
falsifies the R2-10 claim that the flag mechanically closes the launch gate.

### R3-5 [LOW] B-DATA · Retention DELETE is in-flight-safe per B8.1 (CONFIRMED), but reconcile can re-enqueue a row in the same window the retention sweep deletes it → one wasted notify cycle, ack'd as no-op

Re-verifying the accept-risk R2-7 / N3 angle the conductor asked about ("is the 12mo DELETE
in-flight-safe with reconcile/notify"): **YES, the crash-safety is real** — the notify worker's
CAS `UPDATE … WHERE id=$1 AND notified_at IS NULL RETURNING …` returns 0 rows on a deleted row
and the worker **acks without throwing** (B8.1, proposal §5). A retention DELETE landing mid-
notify cannot crash the worker. The only residual: the reconcile cron can re-enqueue a
`notified_at IS NULL` row at the same instant the retention sweep deletes it (a month-12
re-submitter, R2-7) → the re-enqueued job picks up a missing row → ack-noop. Wasted cycle, no
crash, no double-anything. **LOW, accept** — confirms R2-7's in-flight safety claim; the only
gap is the *lead-loss* semantics R2-7 already accepted (re-submitter erased 12mo from first
contact), which is a policy accept, not a new defect.

## R3 regression summary (R2 → R3)

| R2 ID | R3 status | Note |
|-------|-----------|------|
| R2-1 (HIGH) | **grant-path CLOSED; boot-visibility OPEN → R3-1 HIGH** | runtime `boss.schedule` provably grant-clean; but un-`.catch`'d schedule before `listen()` in un-guarded `main()` poisons boot (R3-1). |
| R2-2 (HIGH) | **CLOSED** | Fly-Client-IP-only verified; no XFF read; prod never reaches `req.ip`. Residual R3-2 LOW (one-time warn). |
| R2-3 (HIGH) | **stated-threat CLOSED; inversion opened R3-3 HIGH** | 400 DoS-amplifier + status-oracle gone — but silent-200 = unmeasurable lead-loss + malformed-JSON 400 re-fingerprints at parser layer. |
| R2-5 (HIGH) | **CLOSED (mechanism); not yet in HEAD** | `SPA_ROUTES` + `/privacy` is Accept-agnostic OR-branch, no earlier 404 interceptor. One-line edit unbuilt (design phase). |
| R2-6 (MED) | CLOSED | content-hash CI test is a real binding mechanism. |
| R2-8 (MED) | PASS (improved, residual) | `z.literal(true)` real; still hand-invoked (not schema-gated) → slip moved, not erased; feeds R3-3 silent-loss. |
| R2-9 (LOW) | unchanged | bounded re-feed guard holds. |
| R2-10 (MED) | **PARTIAL → R3-4 MED** | flag gates copy + maybe render; does NOT gate backend route registration → endpoint live pre-gating. |
| R2-7 (accept) | confirmed in-flight-safe → R3-5 LOW | retention DELETE × notify/reconcile is crash-safe (B8.1); residual = accepted lead-loss only. |

## New CRITICAL/HIGH for the conductor
- **R3-1 [HIGH]** un-`.catch`'d `boss.schedule` (copied from anonymizer) runs before
  `fastify.listen()` in an un-guarded `main()` invoked bare → a schedule throw aborts boot
  before HTTP starts, and the `unhandledRejection`-kept-alive guard masks it as a live-but-dead
  process (no clean fail-fast, no `<1min` deploy-failure visibility). The R2-1 "surface, don't
  swallow" delta backfires given the real boot topology.
- **R3-3 [HIGH]** R2-3's silent-200-on-everything inversion: (a) unmeasurable real-user
  lead-loss (no counter separates silently-dropped from never-submitted) — worse than the loud
  consent-400 it replaced; (b) malformed-JSON still returns 400 at the Fastify parser layer the
  route can't reach → the "no non-200 for any body" / route-fingerprint claim is false.

No new strict-CRITICAL. Two new HIGH (R3-1 durability-via-boot-topology, R3-3 silent-200
inversion). R2-2 and R2-5 are genuinely closed; R2-1's grant-path is closed but its
failure-visibility is not; R2-3's stated threat is closed but the cure opened R3-3.

## VERDICT
**YES — CRITICAL/HIGH remains unresolved.** Two new HIGH (R3-1: un-`.catch`'d cron schedule
before `fastify.listen` in an un-guarded bare-`main()` poisons boot under the kept-alive guard;
R3-3: R2-3's silent-uniform-200 inversion = unmeasurable real-user lead-loss + a malformed-JSON
400 fingerprint the route cannot suppress) are open. R2-2 and R2-5 are mechanically closed;
R2-1's grant-path and R2-3's stated DoS/oracle threats are closed, but each cure introduced the
new HIGH above. Not clear.
