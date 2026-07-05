# S8-JOBS/NOTIFICATIONS Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S8-jobs/notifications Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **NO ETHICAL-STOP**

S8 is the surface that touches the two most abusable things the system holds — the **VAPID private
key** and **customer consent** — and it does so *while nobody is looking*: a background runtime that
fires a side effect on a clock, long after the request that scheduled it has gone. If any surface could
quietly push a person who said "stop," leak a phone into a durable sink, or double-pay a courier by a
retry, it is this one. I looked adversarially at every place S8 could cross a grounded line and touch a
real person — the customer who opted out and is pushed anyway, the phone that survives in a DLQ row, the
owner spammed twice by a retry, the owner whose order state is flipped by a forged webhook, the courier
double-paid by a cron that fired on two stacks. **None crosses a grounded red-line in the direction the
line protects, because on every one the packet's *own* recommended disposition closes the line — or, for
the fail-open webhook, *fixes* it.** So the friction here is Opinion, not a stop:

- The **consent line** — an opted-out person must not be pushed — is not merely carried, it is
  **structurally closed by re-checking consent at dispatch time, per attempt**. Verified: the customer
  path re-reads `customer_devices WHERE opted_in=true` fresh inside every handler run
  (`workers/index.ts:124-130`), so an opt-out that lands *after* enqueue *and* a retry that re-runs the
  job both re-read the durable flag — an opted-out user is not re-pushed. This is the correct interaction
  of at-least-once × consent, and it is the sharpest thing the packet gets right. Affirm it (§1).
- The **VAPID private key** never leaves config (`workers/index.ts:90-93`), and the **claim-check** is
  real at the payload level — all three job types carry ids only, no phone/name/address
  (`workers/index.ts:14-36`, verified). The customer push is genuinely no-PII (`:134,152-161`). The line
  `нуль-PII-у-ШІ`/claim-check is honored by the design, not walked into (§2).
- The **fail-open Telegram webhook** — the one live *defect* on this surface — the packet does not carry,
  it **FIXES** (Q4a, fail-closed constant-time). A STOP over a gap the packet already closes would be
  verdict-not-friction. I affirm the fix and push it *harder* than the packet (§4): it should not wait
  for the port.

**Why no stop, and why S8 tracks S5/S7, not S4.** The S4 STOP was a *verified intersection*: the doorway
photo escaped the erasure graph (`анонімізувати-не-видаляти`) and the packet's *own* recommended
disposition left it open. Nothing in S8 rises to that. Each S8 harm-vector (opted-out push, PII sink,
double-notify, forged webhook, settlement double-pay) is either a **current correct property the port
carries visibly**, a **current defect the port fixes**, or a **money landmine already owned by an
operator gate** — and on every one the direction the line protects is held by the packet's disposition.
This is the S5/S7 posture: conditions, not stop.

**The three findings that change the packet's own framing (verified, load-bearing).** My verification
sharpened three of the packet's claims into something more specific — and in two cases *worse* — than the
packet states:

1. The **notification dedup the packet calls "idempotent by a dedup key" does not durably exist.** The
   only telegram dedup is an **in-memory `Set`, reset on restart** (`workers/index.ts:59,70-71`), and the
   `notification_outbox_audit` "`ON CONFLICT DO NOTHING`" it cites as the durable floor **has no unique
   constraint to conflict on** — the table's only key is a random-uuid PK (`mig 007:12-31`), so the
   `ON CONFLICT` is a **no-op decoration that never fires**. So on the *exact* at-least-once failure the
   whole surface is built around — crash-after-send, before pg-boss marks complete, re-run on a fresh
   process — the owner gets a **duplicate Telegram message**, and the packet's own §1 rule ("idempotency
   in Postgres, never in-memory") is violated by the code it describes as already-correct. This is a
   **BUILD, not a CARRY**, and the packet's language hides it (§3).
2. The **DLQ/log PII-freeness is incidental, not structural.** The claim-check is real at the *payload*,
   but the DLQ-adjacent `error_message` column is written with **raw `err.message`**
   (`workers/index.ts:471,525`) into a table *doc-commented* "PII-free" (`mig 007:8`). It is clean today
   only because transport errors happen to carry no contact — one future error that interpolates a
   re-fetched field turns a durable sink into a PII store. The packet's guardrail (redacted `last_error`
   + no-PII-pattern assertion) is exactly what converts *incidental* to *structural* — affirm it, and
   name the distinction so the port builds the guarantee rather than inheriting the luck (§2).
3. The **settlement cron's "watermark bounds even a lock-miss" safety is *false until 085 lands*, and 085
   is a draft.** Verified: `2026-07-10 00:00:00+00` at three sites in
   `docs/design/audit-fix-money/migration-drafts/1790000000085_settlements-catchup.ts` (lines 66/133/148)
   — a **draft**, not in `packages/db/migrations/`, **not applied**. Today is `2026-07-04` — six days
   out. So the packet's reassurance (proposal §6 / threat S8-T3: "even a lock miss is bounded, not
   doubled") is only true *post-085*; **before 085 lands, the settlement cron's single-flight advisory
   lock is the *only* double-pay guard, not defense-in-depth** — the same 085-coupling the S7 seat found
   for `/regenerate`, now on the cron S8 schedules (§5).

---

## Verification note (I read the live source behind every load-bearing claim)

- **Consent is re-checked at dispatch, per attempt — the enqueue→dispatch and retry windows are both
  closed.** `workers/index.ts:124-130` — `handleCustomerStatus` re-fetches `customer_devices … WHERE
  opted_in=true` *inside the handler*, so every run (including a retry) reads the durable flag fresh;
  `:209-237` — `handleDispatch` re-reads `ont.prefs` + status at dispatch and suppresses on the category
  gate; `:366-392` — `handleTelegramSend` re-reads targets + prefs per invocation. The opt-out write is
  durable (`customer/push.ts:73-76`, `UPDATE customer_devices SET opted_in=false`). Confirmed: an
  opted-out person is filtered out at read time on every attempt.
- **The "FOR UPDATE at dispatch" the packet promises is on the *writer*, not the dispatch *read*.**
  `notificationPrefsService.ts:44` — `setCategoryPref` locks the owner target `FOR UPDATE` and INSERTs
  the consent audit in the **same txn** (`:60-65`) — the atomic consent *change*. The dispatch-time
  *reads* (`workers/index.ts:124,209`) are plain filtered SELECTs, per-attempt. This split is *correct*
  (the writer is where atomicity belongs; the hot read need not lock) — but the packet's phrasing
  ("prefs re-checked at dispatch under FOR UPDATE") conflates them; the port must keep the writer's
  `FOR UPDATE`+audit and must **not** over-lock the hot dispatch read.
- **The claim-check holds at the payload; the customer push is no-PII.** `workers/index.ts:14-22`
  (`NotifyDispatchJob` = `{targetId, eventType, orderId, locationId, attempt, held}`), `:24-28`
  (`CustomerStatusJob` = `{orderId, locationId, event}`), `:30-36` (`TelegramSendJob` =
  `{event, entity_id, location_id, dedupKey, attempt}`) — no PII field representable. Customer push body:
  `:134` "Build minimal payload (no PII)", `:144-161` — `Order #ABCD Delivered` + `formatMoney(total)` +
  `data:{orderId, locationId, url}`. Confirmed.
- **The customer double-push self-heals at the device; the owner double-notify does not.**
  `workers/index.ts:155` — the customer push carries `tag: order-${orderId}`, so a duplicate push
  *replaces* rather than stacks on the OS — an accidental but real idempotency floor for the customer.
  The Telegram send has **no such floor** (messages stack), and its only dedup is in-memory (below) — so
  a retry double-notify lands on the **owner**. Confirmed.
- **The telegram dedup is in-memory-only; the durable floor the packet cites is a no-op.**
  `workers/index.ts:59` "reset on restart", `:70-71` `private dedupCache = new Set<string>()`, checked
  `:350`, set `:484`. The `notification_outbox_audit` INSERTs use `ON CONFLICT DO NOTHING`
  (`:440-442,468-471,491-495`) but the table has **no unique constraint** — `mig 007:12-31` creates only
  a `gen_random_uuid()` PK and three **non-unique** indexes. So `ON CONFLICT` matches nothing and never
  suppresses; the audit is an append-only log, not a dedup. Confirmed: the durable dedup the packet
  describes does not exist.
- **The DLQ-adjacent `error_message` is a raw free-text sink, doc-labelled PII-free.**
  `workers/index.ts:471` (`result.reason`), `:521-526` (archive with `err.message`) → a `text` column
  (`mig 007:24`) the migration comments "PII-free" (`:8`). Clean today by incident (transport errors),
  not by structure. Confirmed.
- **The Telegram webhook fails OPEN on a missing header, gated only by the leakable URL-path secret.**
  `telegram-webhook.ts:75` — route registered at `/webhook/telegram/${telegramBotSecret}` (URL secret is
  referer/log-leakable); `:88-95` — with `telegramBotSecret` set, a *mismatched* header → 401; `:96-99` —
  a *missing* header → log warning, **process anyway** ("backward compat"). Live fail-open confirmed. A
  forged webhook can drive `order.confirm`/`order.reject` (LIVE, `:314-350,444-478`), `store.*`/`pref.*`
  (dark, flag-gated `:223-224`), and the `/start login_` owner-login binding (`:565-593`). The last is a
  session-fixation-adjacent surface (Breaker's chain to trace).
- **085 is a draft, six days out.** `docs/design/audit-fix-money/migration-drafts/1790000000085_settlements-catchup.ts`
  — literal `2026-07-10 00:00:00+00` at `:66,133,148`; the header self-warns "If prod apply slips past
  2026-07-10, the operator MUST bump the literal (all THREE occurrences)" (`:22`). Not in
  `packages/db/migrations/`. Today `2026-07-04`. Confirmed — the same landmine the S5/S7 seats already
  gave forcing functions.

---

## By charge

### 1. Consent — the re-check-at-dispatch closes the spam window, retry included; name the writer/reader split

**Affirm the strongest correctly-built property on the surface, and do not let the port mis-implement
it.** The tasking's question — "is a consent re-check at dispatch under FOR UPDATE enough, or is there a
window where the opted-out are still spammed?" — has a precise, verified, *reassuring* answer, and it is
better than the packet states. There is **no durable spam window**, and the reason is exactly the
at-least-once discipline the surface is built on: because consent is re-read *inside the handler*, per
run, three windows that *could* spam an opted-out person are all closed:

- **The enqueue→dispatch window is closed.** Prefs/opt-out are checked at **dispatch**, not at enqueue
  (`workers/index.ts:124,231`) — a person who opts out after the job is queued but before it fires is
  filtered out at the read. Verified.
- **The at-least-once retry window is closed — this is the load-bearing one.** A job that re-runs
  (crash-before-complete, or an explicit retry) re-reads consent *fresh* each time, because the read is
  in the handler body, not cached from the first attempt. So the exact failure mode that makes this
  surface scary — a job re-firing — does **not** re-push someone who opted out between attempts. The
  packet's at-least-once × consent contract is *honored by construction* here, and I affirm it loudly.
- **The only residual is the irreducible read→send microwindow** — a person who opts out in the
  milliseconds between the dispatch read (`opted_in=true`) and the `webpush.sendNotification` call
  (`:177`) gets one in-flight push already on the wire. This is sub-second, unavoidable in any
  distributed sender, and not a "spam window" in any meaningful sense. No action.

**The one precision the port must carry, and the mis-port to forbid.** The packet says "prefs re-checked
at dispatch under FOR UPDATE." Verified, the `FOR UPDATE` is on the **owner category-pref *writer***
(`notificationPrefsService.ts:44`, with a same-txn consent audit `:60-65`) — *not* on the dispatch-time
*read*, which is a plain filtered SELECT for both owner and customer. This split is *right*: the writer
is where the consent change must be atomic + audited (GDPR-legible); the hot dispatch read must **not**
lock (over-locking the notify path is a self-inflicted contention bug). The port must (a) keep the
writer's `FOR UPDATE` + same-txn `notification_prefs_audit` verbatim — this is the proof-of-consent trail
— and (b) implement the dispatch read as a filter, not a lock. **The mis-port to forbid:** a future
"simplify the writer" that drops the `FOR UPDATE`/audit because "the dispatch read already checks
consent" — the read checks the *value*; the writer's lock+audit is what makes a concurrent
web↔telegram toggle atomic and the consent change *provable*. Name both halves so neither is lost (§C-1).

### 2. PII claim-check — real at the payload; incidental at the DLQ/log; the guardrail is the fix

**Affirm the claim-check and the no-PII customer push; name the incidental-vs-structural gap the guardrail
closes.** The packet's Q5 is correct in direction and the code backs it: no job payload can carry a
phone/name/address (`workers/index.ts:14-36`), the worker re-fetches under the tenant/customer GUC seat
(`:122`, `set_config('app.user_id',…)`), the render masks the phone (`pii-mask.ts:11-16`) and takes only
a first name (`:584`), and the customer push body is money + a short id (`:144-161`). This is a textbook
claim-check and I affirm it. But the tasking's second question — "is it *honest* that the DLQ and logs
stay PII-free?" — has a sharper answer than "yes":

- **The payload is PII-free structurally** (the type forbids it) — this is honest and load-bearing;
  affirm.
- **The DLQ/audit `error_message` is PII-free only *incidentally*.** It is written with raw `err.message`
  / `result.reason` (`workers/index.ts:471,521-526`) into a `text` column the migration *labels*
  "PII-free" (`mig 007:8,24`). Today the errors are transport-level ("Telegram API error 403", "Order not
  found: <id>") and carry no contact — but that is *luck*, not *structure*. A future handler that throws
  with an interpolated re-fetched field would silently make a durable row a PII store, under a comment
  that says it can't happen. The packet's Q5(a) — "DLQ persists the claim + a **redacted** `last_error`;
  a guardrail asserts no DLQ row contains a phone/name/address pattern" — is *exactly* the conversion
  from incidental to structural. **Affirm the guardrail as load-bearing, and name plainly that today's
  clean DLQ is incidental** so the port builds the assertion rather than trusting the comment (§C-2).
- **The render carries the unmasked `delivery_address` to the owner — and that is correct, not a leak.**
  `fetchOrderDetails` pulls `o.delivery_address` unmasked (`:581`) into the Telegram body. This is *egress
  to the legitimate recipient* (the owner is the data controller who must know where to deliver), not a
  durable sink — the same owner-plaintext-PII the S7 seat ratified. And it is *privacy-by-default*: the
  per-location `telegram_alert_detail` (`:598-600`) defaults to `'area'` (coarse), so an owner must opt
  *up* to see the full address/phone. That default is a dignity-positive choice; affirm it and port the
  default with the constant.

### 3. At-least-once → double-notify — the money is DB-guarded; the notification dedup is a BUILD the packet's language hides

**This is the one place I raise the packet's own honesty against itself, and it is the sharpest charge in
this document.** The tasking asks: "is it honestly named that idempotency lives on each handler, not the
queue?" For **money**, yes and verifiably: settlement is watermark-idempotent (one atomic DEFINER call),
auto-cancel is `WHERE status='PENDING'` status-CAS, refund_due is the N5 partial-unique — the double-fire
is genuinely bounded by Postgres, and the packet is right to trust the DB guard over the queue. Affirm
the money side; it is the Breaker's robustness domain and I do not re-litigate it.

**But for *notifications* the packet describes a durable dedup that does not exist, and the gap lands on a
real person.** Verified above: the only telegram dedup is an **in-memory `Set`, reset on restart**
(`workers/index.ts:59,70-71`), and the `notification_outbox_audit` "`ON CONFLICT DO NOTHING`" the packet
names as the archive/dedup floor **has no unique constraint** (`mig 007:12-31`) — it never suppresses
anything. So:

- On the **exact at-least-once failure the surface is built to survive** — a worker sends the Telegram
  message, then dies before pg-boss marks the job complete — the job re-runs on a fresh process whose
  `dedupCache` is empty, and the **owner gets a duplicate order ping**. Not a rare crash: every deploy,
  every worker restart, every OOM re-arms the empty cache.
- The customer path has **no dedup at all** in `handleCustomerStatus` — but the push `tag: order-<id>`
  (`:155`) collapses duplicates *at the device*, so the customer double-push is benign. The annoyance
  falls entirely on the **owner**, exactly as the tasking frames it.

Weighed across lenses: **honesty** (`design каже правду`) — the packet's §3.4 ("dedup via a dedup key +
archive to `notification_outbox_audit`") reads as if the durable dedup is *present*; it describes the
*target*, and a reader would carry an in-memory `Set` believing it a durable floor. **Wholeness** — the
packet's own §1 rule ("idempotency in Postgres, not Redis/memory, made structural") is *violated* by the
very handler it holds up. **Care** — the harm is small (owner annoyance, not money) but it is the surface
telling the owner about their livelihood, and a duplicate "new order!" that turns out to be the same
order erodes exactly the signal the owner most needs to trust.

**Disposition (§C-3).** Not a stop — the harm is bounded and the packet's *recommended* disposition
(Q11a: the `access-request.notify` **claim-before-send CAS** as the gold standard, and §3.4's dedup-key
intent) points the right way. But the packet must **name the current state honestly**: the notification
dedup is *in-memory best-effort today, not durable*, the `ON CONFLICT` floor is a no-op, and the port is
a **BUILD** (a Postgres claim-before-send: a unique dedup key / a CAS on a durable row, checked *before*
the external send) — **not a CARRY of the `Set`**. Do not let the rewrite re-ship "the owner can be
double-pinged on any restart" as silent parity behind language that implies it is already solved. This is
the S8 twin of the S5 Potemkin finding: a property described as built that the runtime does not back.

### 4. Telegram fail-open (Q4 🔴) — affirm the FIX; but a live security hole should not wait for the port

**Affirm Q4(a) as the correct, minimal fix — and push it *earlier* than the packet frames it.** The
finding is real and verified (`telegram-webhook.ts:96-99`): a webhook with the secret configured but the
header *missing* is processed anyway, and a forged webhook can flip an owner's order state
(`order.confirm`/`order.reject`, LIVE — not flag-gated). The fix is one constant-time compare that makes
a missing OR mismatched header → 401, and it **aligns the code with the E2E the suite already asserts**
(missing→401) — the cleanest possible fix, where the guardrail exists and the *code* was the deviation.
Two sharpenings, one strategic and load-bearing:

- **The fix makes the leakable thing non-load-bearing — say so.** The route is gated by the **URL-path
  secret** (`:75`), which is referer/log/proxy-leakable (the packet's Q-TG-URL-SECRET). Today, an
  attacker who has *seen the URL* (a log line, a proxy, a browser referer) can forge a webhook with **no
  header**. Post-fix, the **header** is the real gate and constant-time — so a leaked URL alone no longer
  suffices. The fix is not just "close the fail-open"; it correctly *demotes* the leakable URL to a router
  token. Affirm precisely.
- **Do not defer a live hole to the 8th strangler surface.** The packet frames this as **FIX-IN-PORT** —
  meaning the fail-open stays open on the Node stack, serving real owners, *until S8 ports* (weeks,
  possibly, and S8 is explicitly the last of `S5→S6→S7→S8`). Six days before the launch trigger, a live
  webhook that can flip an owner's confirm/reject state is not a parity footnote — it is a hole in
  *today's* store. The honest, care-driven move (and it rhymes with the S5/S7 "don't couple a fix to a
  big migration"): **land the fail-closed header on Node *now*, as its own tested commit** (it aligns
  with the existing test, so it is red→green today), and let the port **carry the already-fixed
  behavior**. This keeps the port a pure port *and* protects real users immediately — the opposite of
  deferring safety to a rewrite (§C-4). The `/start login_` binding is an additional forged-webhook blast
  surface (session-fixation-adjacent); flag it to the Breaker to trace, but the one constant-time fix
  closes the common vector regardless.

### 5. 085 watermark + settlement double-fire (Q6/Q7c) — inherit the S5 forcing function; and name the pre-085 gap S8 uniquely owns

**Insist on it — this is the closest thing on the surface to a live, silent, real-people money harm, and
S8 is the surface that *fires the cron*.** The watermark landmine is verified (draft, `2026-07-10` × 3,
six days out) and already twice-flagged: the S5 seat lifted it into an operator-owned timing gate with a
pre-apply `literal >= apply_date` assertion (S5 `counsel-opinion.md:281-284`); the S7 seat made S7's DoD
*point at* that forcing function rather than re-footnote it (S7 `counsel-opinion.md:311-318`). S8 is now
the **third packet** to carry it — and that diffusion is itself the risk: *a landmine owned by three
packets is a landmine no one owns.* Two conditions, one inherited and one S8-specific:

- **Inherit the S5 forcing function; do not re-prose it.** S8's DoD line (the settlement cron is
  single-flight + at-least-once-idempotent) must **point at** the S5 pre-apply assertion — the same move
  S7 made — not restate the watermark as a paragraph in §6. S8 does not author or apply 085; it *fires
  the cron that trips it*, so its obligation is to make the schedule *unable* to trip it silently (§C-5).
- **The S8-specific sharpening: before 085 lands, single-flight is the *only* guard, not
  defense-in-depth.** The packet reassures (proposal §6, threat S8-T3): "the settlement effect is
  watermark-idempotent so even a lock miss is bounded, not doubled." Verified — that is **only true
  post-085**. 085 is a draft; until it lands, the settlement DEFINER is *not* watermark-guarded, so a
  cron double-fire — the exact hazard Q7c names (two stacks, Node+Rust, both running the fleet) — has
  **no DB backstop**. This is the same 085-coupling the S7 seat found for `/regenerate` ("safety
  conditional on 085, and 085 hasn't landed"), now on the **cron**. So the "exactly-one-stack-owns-the-
  fleet" posture (Q7c) is not belt-and-suspenders during the overlap window — *it is the belt*, with the
  suspenders (the watermark) not yet fastened. The DoD's fleet-atomic-flip probe and the double-fire→one-
  effect idempotency probe must be **proven green before the settlement cron runs on Rust at all**, and
  the packet should state plainly that pre-085 the single-flight lock carries the whole load (§C-5).
  Direction, as always: erring EARLY double-pays; the over-paid courier does not complain, so nothing
  surfaces it — `готівка → алерт-тертя` on the payout side.

### 6. Charter, scope, and the real people

- **Charter: clean.** Jobs and notifications. No military/warfare, no surveillance-for-harm, no
  commons-capture. The two most abusable assets — the VAPID key (fleet-scale push-spoof if leaked) and
  customer consent — are the ones the packet guards hardest, and correctly: key in config never logged
  (`workers/index.ts:90-93`), consent re-checked per attempt (§1). The one surveillance-adjacent thing
  the surface *could* do — push a person against their will — is structurally prevented. `нуль-PII-у-ШІ`
  is not in play (no AI path on this surface; the claim-check keeps PII off durable sinks regardless).
  The Charter's consent/dignity spirit is *realised* in the opt-out-respected-per-attempt design, not
  merely un-violated. It serves the launch trigger (the owner is reliably told about the first real paid
  order — *if* the dedup §3 and the fail-open §4 are closed).
- **Scope: disciplined, three fixes in the honest direction.** The money-math (`app_generate_settlements`),
  the order machine, GDPR erasure logic, the backup pipeline, and the Plisio webhook are correctly
  *excluded* (S8 owns scheduling + single-flight + at-least-once plumbing, not the semantics); the new
  `jobs` table is additive-forward-only, no business-table change. The three FIX-IN-PORTs (Q4 fail-closed,
  Q8 backoff+DLQ baseline, Q10 lock-id registry) are the *honest* direction — re-shipping the fail-open
  webhook or the "24/30 bare defaults / DLQ-no-consumer" gap as "parity" would be the neglect-laundering
  the S5 seat named. One scope watch-item: the **Q8 hardened-baseline behavior change** (backoff+DLQ
  everywhere + paging on dead jobs) is strictly-more-reliable but *is* a behavior change — the packet
  itself predicts this as the likely counsel flag, and it is right: it must be an **explicit
  operator-signed improvement**, not carried by silence. I affirm the change and affirm that it must be
  *signed*, not slipped (§C-6).
- **The three real people, by who-bears-the-cost.** The **customer** is protected *from* unwanted push
  (opt-out per attempt) and from double-push (device tag-collapse) — well-built. The **owner** bears the
  two residuals: the double-*notify* on retry (§3, in-memory dedup) and the forged-webhook order-flip
  (§4, fail-open) — the two sharpest §§. The **courier** bears the money landmine (§5, 085 double-pay) —
  silent, because the over-paid courier does not complain. Every seat in this council speaks for the
  *job firing correctly*; §§1–5 are where I make the council also speak for the *person the job reaches,
  or fails to reach, or reaches twice.*

---

## Non-blocking aesthetic / strategic notes

- **Consent-re-checked-per-attempt is the design-language high point — say it out loud.** The surface's
  hardest ethical requirement (never push someone who said stop) is met not by a special-case guard but
  by a *structural* property: the consent read lives *inside the handler*, so at-least-once retry — the
  thing that makes every other handler dangerous — makes *this* one *safer*, because a retry re-reads the
  latest consent. The failure mode that threatens the surface is turned into the mechanism that protects
  the person. That is aesthetics doing its ethical job — a whole design where the risky property and the
  protective property are the same property. Affirm it, and port the *per-attempt-read* shape verbatim;
  the moment someone "optimizes" it into an enqueue-time check, the elegance and the ethics both break.
- **The `Set` dedup is aesthetics-as-leading-indicator running in reverse — the S8 twin of the S5
  Potemkin.** An in-memory cache *looks* like idempotency (it has a dedup key, a cache, an eviction
  policy) but is backed by *nothing durable* — and it sits under a `notification_outbox_audit`
  "`ON CONFLICT DO NOTHING`" that *looks* like a durable floor but conflicts on *nothing*. Two layers of
  idempotency-*theater* over a null guarantee. A beautiful-looking dedup is more dangerous than an
  obviously-absent one, because its shape earns the trust that it works. The port's job is to make the
  shape and the guarantee match: a real Postgres claim-before-send, where the dedup you can *see* is the
  dedup that *holds* (§C-3). This is "schema-rich, runtime-minimal" in its honest form — one durable
  claim row, checked before egress, replacing two layers of decoration.
- **"Schema-rich, runtime-minimal" is at its best in the 21-crons-shed-the-queue cut — affirm the
  restraint.** The packet's move to run 21 pure-cron workers as `tokio` loop + `pg_try_advisory_lock`
  with **no job row** (proposal §3.6), and keep the durable `jobs` table only for the ~9 that genuinely
  need cross-process handoff, is the *good* twin of the doctrine (the S5 seat named its evil twin, the
  Potemkin UI). It is the materially smaller surface the from-scratch rebuild *earns* over "port pg-boss
  1:1" — build the durable table for what needs durability; do not round-trip 21 cron sweeps through a
  queue they never needed. Affirm the cut; keep it a cut (do not let "for symmetry" pull the 21 back into
  the table).
- **Name the fleet-overlap cut-date now — the same un-cut-vine the S6 seat flagged.** The Q7 posture is
  "exactly one stack owns the fleet, flip atomically" — good. But an overlap you can run indefinitely is
  an overlap you never end, and from the inside "not yet time to shed Node" and "never intend to" are
  indistinguishable without a pre-committed trigger. Name the S8-fleet overlap end-trigger + owner ("both
  stacks stable N days ⇒ retire the Node fleet + its session-mode listener connection, dated") so the
  single-stack-ownership discipline has a defined *end*, not just a defined *shape* (§C-7). This rhymes
  with the S6 overlap cut-date and the handoff's open `терпіння↔прив'язаність` item.

---

## Steel-man of a rejected option (obligatory)

**Q4 option (b) — "CARRY the fail-open, parity-pure" — the disciplined position my §4 pushes against.**

Its strongest case, made fairly — and made *stronger* than the packet's own steel-man: the whole rebuild
rests on a **parity oracle** (the 174-spec E2E net as the sole arbiter of "did the port preserve
behavior"). Every FIX-IN-PORT *blurs* that oracle: when the port both changes behavior (fail-open →
fail-closed) *and* re-implements the surface in a new language, a downstream difference becomes
un-attributable — was it the intended fix, or a port bug the fix masked? The cleanest discipline is
therefore *not* "carry the fail-open" (which the packet rightly rejects as re-shipping a security gap)
but a sharper third option the packet does not name: **fix it, but *not in the port* — land the
fail-closed compare on the Node stack first, as its own tested commit, prove it green against the existing
E2E, and let the port carry the already-fixed, already-tested behavior.** Under that framing, "carry
parity" is *correct at the moment of the port* — because parity is then measured against a Node stack that
*already* fails closed. That is a genuinely strong position: it protects the oracle *and* it does not
re-ship a hole. I do not dismiss it.

**Why I land against pure-carry — and adopt the steel-man's insight wholesale.** The steel-man is right
about the *vehicle* (the fix should not be entangled with the language port, exactly so the oracle stays
clean) and I *adopt* it — that is precisely §C-4: **fix on Node now, carry-fixed in the port.** Where
pure-carry-until-the-port loses is *timing and who pays*: framing the fix as FIX-IN-PORT means the live
hole stays open on the stack serving real owners until the *8th and last* strangler surface ports — and
six days from the launch trigger, that is a real store whose confirm/reject state a leaked URL can flip.
The steel-man's own logic (keep the fix out of the port) points to the *earlier* fix, not the deferred
one: if the fix should not live in the port, then it should live *now*, on Node, where it protects today's
users and where the existing test already makes it red→green. So the steel-man wins on *discipline* (don't
entangle the fix with the port — adopted) and loses only on the packet's implicit *schedule* (defer to the
port) — and the honest reading of the steel-man's own principle is "fix it sooner, standalone," not "fix
it later, in the rewrite." Carry-fixed in the port; fix-live on Node first.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical/epistemic load.

1. **[consent, §1 — affirm the best-built property] Carry the per-attempt consent read *and* the writer's
   FOR UPDATE+audit; forbid both mis-ports.** AFFIRM the dispatch-time, per-attempt consent re-check
   (`workers/index.ts:124,209,366`) as the correct at-least-once × consent contract — an opted-out person
   is not re-pushed on a retry. Port the read as a **filter, not a lock** (do not over-lock the hot notify
   path). Port `setCategoryPref`'s **`FOR UPDATE` + same-txn `notification_prefs_audit` verbatim**
   (`notificationPrefsService.ts:44,60-65`) as the GDPR proof-of-consent trail. Record two forbidden
   mis-ports: (a) moving the consent check to *enqueue* time (re-opens the window), and (b) "simplifying"
   away the writer's lock/audit because "the read checks consent" (the read checks the value; the writer
   makes the change atomic and provable).
2. **[PII, §2 — incidental → structural] Build the DLQ/log guardrail; do not trust the comment.** AFFIRM
   the payload claim-check (`workers/index.ts:14-36`) and the no-PII customer push (`:144-161`). Name on
   the record that the DLQ-adjacent `error_message` is PII-free *incidentally* today (raw `err.message`
   into a `text` column doc-labelled "PII-free", `mig 007:8,24`) — and build the packet's Q5(a) guardrail:
   the DLQ persists a **redacted** `last_error`, and an assertion fails the build if any DLQ/audit row
   matches a phone/name/address pattern. Carry the `telegram_alert_detail` default `'area'` (coarse) as a
   privacy-by-default choice (`:598-600`).
3. **[honesty, §3 — the sharpest] Name the notification dedup as a BUILD, not a CARRY.** Record the
   verified truth: the telegram dedup is an **in-memory `Set`, reset on restart** (`workers/index.ts:70`),
   and the `notification_outbox_audit` "`ON CONFLICT DO NOTHING`" is a **no-op** (no unique constraint,
   `mig 007:12-31`) — so a crash-after-send re-run **double-pings the owner**. The port must **build a
   durable Postgres claim-before-send** (a unique dedup key / the `access-request.notify` CAS the packet
   itself names as gold standard, Q11a), checked *before* the external send — not port the `Set`. Do not
   let §3.4's "dedup via a dedup key + archive" language re-ship an in-memory best-effort as if durable.
   (Money idempotency is separately DB-guarded and correct — affirm and leave to the Breaker.)
4. **[security/timing, §4] Fix the fail-open on Node *now*, standalone; carry-fixed in the port.** AFFIRM
   Q4(a) (fail-closed, constant-time, gate on the header not the URL). But do **not** defer it to the
   port: land the fail-closed compare on the Node stack as its own red→green commit (it aligns with the
   existing E2E `missing→401`), so a live webhook that can flip an owner's order state is closed for
   *today's* users, six days from launch — and the port carries already-fixed behavior (keeping the parity
   oracle clean). Flag the `/start login_` forged-webhook binding (`telegram-webhook.ts:565-593`) to the
   Breaker as an additional blast surface to trace.
5. **[money/timing, §5 — the live landmine] Inherit the S5 085 forcing function; name the pre-085 gap S8
   owns.** S8's DoD must **point at** the S5 pre-apply `literal >= apply_date` assertion (S5
   `counsel-opinion.md:281-284`), not re-prose the watermark — S8 is the surface that *fires the cron*.
   State plainly that until 085 lands (draft, `2026-07-10` × 3, six days out), the settlement cron's
   single-flight advisory lock is the **only** double-pay guard — the "watermark bounds even a lock miss"
   reassurance (threat S8-T3) is false pre-085 — so the fleet-atomic-flip probe and the double-fire→one-
   effect probe must be **green before the settlement cron runs on Rust at all**. A landmine owned by
   three packets (S5/S7/S8) needs one *forcing function*, not three footnotes.
6. **[strategic, §6] Sign the Q8 hardened-baseline behavior change explicitly.** Backoff+DLQ as the
   baseline for every queue + paging on dead-job count is strictly-more-reliable — affirm it — but it *is*
   a behavior change from the 24/30 bare-defaults state (a job that used to vanish into `failed` now
   retries and pages). It must be an **operator-signed documented improvement**, not carried by silence.
   (The packet predicts this flag; it is right.)
7. **[strategic, § aesthetics] Give the S8 fleet-overlap a cut-date.** Name the overlap end-trigger +
   owner now ("both stacks stable N days ⇒ retire the Node fleet + its session-mode listener connection,
   dated") so "exactly one stack owns the fleet" has a defined *end*, not only a defined *shape* — the
   same un-cut-vine the S6 seat flagged.

---

## The question nobody asked (§7)

Every seat in this council — and the tasking that framed it — measures S8 from one direction: **do not
send the notification that shouldn't be sent.** The consent re-check, the opt-out per attempt, the
PII-claim-check, the dedup to avoid annoyance, the quiet-hours hold — every control *reduces* or *gates*
notification. The whole ethical apparatus is built to protect the person from a message they did not want.
That frame is correct and, on this surface, beautifully built (§1).

**Nobody speaks for the person who *opted in* — who is *relying* on the notification — and cannot tell a
silence that means "nothing happened" from a silence that means "it failed and no one told you."** The
owner who enabled order pings and is waiting for the lunch rush; the customer who opted into "on the way"
and is standing by the door. On this surface, a notification they were promised can vanish **silently** in
at least four ways, and on none of them is the *waiting person* told: the Telegram webhook returns 200
even on internal failure (`telegram-webhook.ts:123,128`, carried by design); a send that hits a tripped
circuit breaker is skipped to an audit row (`workers/index.ts:398-404`); a target that exhausts retries is
auto-*disabled* (`:331`) — the owner's channel is turned off and they find out by missing an order; a
customer subscription that returns a stale 410 is auto-*pruned* (`:183-191`) — the customer is silently
un-subscribed. The packet's Q8 fix pages ops on *queue* dead-jobs — but these are *notification-level*
silent failures, and the person they fail is not ops; it is the owner or the customer who opted in and is
counting on the ping. The consent model is a masterpiece of "don't reach those who said no"; there is **no
symmetric guarantee** for "*do* reliably reach those who said yes — and when you can't, tell *them*, not
just the audit table."

The unasked question is not technical and it does not block the port: *the surface works hard to guarantee
you are not notified against your consent — what guarantees the person who consented, and is relying on the
notification, is either reached or honestly told the channel is down?* The honest answer is the same shape
as the S6 seat's (couple the truth-signal to the thing that failed, and tell the affected person): when a
notification an owner *opted into* silently fails — circuit tripped, target auto-disabled, subscription
auto-pruned — the *owner* should be able to see their channel is degraded (a status affordance, a "your
Telegram alerts are paused" surface), not discover it as a missed order. That sits alongside the packet's
own DLQ-ops-alert fix (Q8): extend the observability from *queue dead-jobs* to *notification-channel
degradation*, and route it to the **person waiting**, so the owner who stops getting order pings — and the
customer who stops getting delivery updates — is not the one who discovers, in silence, that the channel
they trusted went dark.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change, blocks S8, or
overrides a conscious operator decision.*
