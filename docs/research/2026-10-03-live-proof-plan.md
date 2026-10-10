# Live proof plan: 62 links on the real site (W-LIVE, phase A)

Lane W-LIVE, 2026-10-03. Tree: copy of main 34aa2bc0. Deployed Worker: **e7d11a55** (MEASURED
`GET https://qa-durres.dowiz.org/api/version` -> `{"built_at":"2026-10-02T17:29:42Z","commit":"e7d11a55…"}`),
two commits behind main. fd777995 (order-for-later kept, courier invite link, customer/courier chat)
and 34aa2bc0 (publish card, landing rewrite) are **not deployed**.

Phase A was design only: no node, playwright, cargo or python. Evidence here comes from read-only
`curl` GETs, Cloudflare GraphQL reads and file reads. MEASURED = a command was run, with its output
quoted. CITED = file:line. Rows 1-58 are the matrix in
`docs/research/2026-10-02-system-integration-check.md` §1. Rows 59-62 were added by main's amendment.

Operator, 2026-10-03: "ось що реально потрібно замість мокапів" and "не просто мокати дані, а перевіряти
усе на живому сайті з реальними підключеннями, детальними контрактами та схемами і версіями й описом".

## 0. Summary (uk)

- **Класи (62 ряди):** LIVE-NOW **28** · NEEDS-KEY **15** · NEEDS-CODE **10** · NOT-LIVE-BY-DESIGN **9**.
  NEEDS-CODE — четвертий клас, якого картка не мала. Ланка відсутня в коді (DISC у звіті 10-02), тож ключ
  її не оживить. Раннер друкує для неї `NOT-LIVE` з причиною «від'єднано в коді, рядок roadmap X». Такий
  ряд ніколи не рахується як прохід, і алерту він не викликає.
- **Головна знахідка (MEASURED):** кожен запуск із GitHub зустрічає **Cloudflare Bot Fight Mode**.
  Відповідь — `managed_challenge` для ASN 8075 (Microsoft, тобто раннери GitHub). Через це
  `health-cron` червоний на кожному запуску (8 із 8, 2026-10-01T18:30Z..10-03T02:11Z). Ніхто не отримав
  алерту: немає `TELEGRAM_BOT_TOKEN` або `OPS_TELEGRAM_CHAT_ID`. `heartbeat-monitor` при цьому
  **зелений** (6 із 6), бо сторінку-челендж (403 < 500) він рахував як «живий».
  Поки BFM увімкнений, **жоден** живий доказ за розкладом із GitHub не дійде до сайту. Це стосується
  health-cron, key-flows, evals-nightly і майбутнього live-proof.
- **Ключі Stripe, `TELEGRAM_BOT_USERNAME`, `OTEL_EXPORTER_OTLP_ENDPOINT`, `BACKUP_SEAL_PK` — секрети
  всього Worker.** Якщо поставити TEST-ключ Stripe, тестові картки ввімкнуться на **всіх** закладах. Тому
  Stripe потребує ізоляції QA: ключі на рівні закладу або окремий QA-Worker. Це NEEDS-CODE + NEEDS-KEY.

## 1. Cross-cutting findings (these come before the rows)

### F-A. Bot Fight Mode challenges every GitHub-hosted probe (MEASURED)

```
POST https://api.cloudflare.com/client/v4/graphql   (token /root/.cf_token; zone 2929a252ceafe79f0bc52e9b9f0b92ee)
firewallEventsAdaptive(filter:{datetime_gt:"2026-10-01T00:00:00Z", userAgent:"dowiz-health-cron"})
-> {"action":"managed_challenge","clientAsn":"8075","clientCountryName":"US","clientRequestHTTPHost":"dubin-sushi.dowiz.org",
    "clientRequestPath":"/admin/","datetime":"2026-10-03T02:11:28Z","ruleId":"bot_fight_mode","source":"botFight", ...}
   (same for dowiz.org /healthz and /, sushi-durres /manifest.webmanifest, /api/public/locations/sushi-durres/eta, dubin-sushi menu …)
filter clientRequestHTTPHost:"webhook.dowiz.org"
-> managed_challenge, botFight, ASN 8075, userAgent curl/8.5.0 at 2026-10-03T01:59:53Z, 10-02T23:13:59Z, 19:39:31Z, 14:50:16Z, 08:25:35Z, 02:08:58Z …
```

GitHub run 37088913592 ("Production health", created 2026-10-03T02:11:20Z): step "probe production" =
failure; annotation `production health FAILED; no TELEGRAM_BOT_TOKEN secret or OPS_TELEGRAM_CHAT_ID
variable` (MEASURED via the public Actions API; the repo is public). From the box, `health.sh` passes
20/20 (main's measurement). The cause is the network origin, not the code. The analytics token
(`/root/.cf_analytics_token`) cannot read this: `does not have permission 'com.cloudflare.api.account.zone.analytics.read'`.
Only `/root/.cf_token` can.

What this lane did (files it owns under the amendment):
- `.github/workflows/heartbeat-monitor.yml` now probes the four `/healthz`: platform, sushi-durres,
  dubin-sushi and qa-durres. "Up" means 200, body `ok`, and no `cf-mitigated` header. A challenge is
  `CHALLENGED`, and a run with any target not up is red even after the alert went out. The dead tunnel
  target is kept as a comment.
- `tools/live-checks/health.sh`: every FAIL row now carries `CHALLENGED by Cloudflare (cf-mitigated: …)`
  when present, plus the first 80 bytes of the body. The ETA POST now saves its headers.

What only the operator can do (the Free plan's BFM cannot be skipped by a WAF custom rule, per CF docs).
Pick one:
1. Turn Bot Fight Mode off for dowiz.org (Security -> Bots). The scheduled probes then work from GitHub
   as written.
2. Keep BFM, and let the scheduled probes run from somewhere that is not a datacenter. The box is not
   always on. A Cloudflare Worker cron in the same account might work, but nobody has tested whether BFM
   challenges a Worker subrequest.

### F-B. The deployed Worker lags main by two commits

Row 2 (order for later) is fixed in main by fd777995. Live runs e7d11a55, so the row-2 probe is
expected to go **FAILED** until main deploys. That FAILED is a real outcome, not a probe bug. Every probe
records `/api/version.commit` in its evidence, so a FAILED can be matched to the build it hit.

### F-C. Secret files on the box (presence and name only; no value was printed)

| file | names inside | used by rows |
|---|---|---|
| `/root/.dowiz_owner` | BOOTSTRAP_SECRET, OWNER_EMAIL, OWNER_PASSWORD, COURIER_PHONE, COURIER_PASSWORD, QA_COURIER_PHONE, QA_COURIER_PASSWORD, KITCHEN_EMAIL, KITCHEN_PASSWORD, QA_HUB_COURIER_PHONE, QA_HUB_COURIER_PASSWORD | most LIVE-NOW rows |
| `/root/.cf_token` | CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID | F-A, 44 |
| `/root/.cf_deploy_token` | CLOUDFLARE_ACCOUNT_ID, CLOUDFLARE_API_TOKEN | 31 (R2 read of LEARN), 26 cleanup |
| `/root/.cf_analytics_token` | CLOUDFLARE_ACCOUNT_ID, CLOUDFLARE_API_TOKEN | 44 (account-level workers analytics) |
| `/root/.cf_mail_token` | CLOUDFLARE_API_TOKEN | (32, not needed by a probe) |
| `/root/.dowiz_backup_seal.pk` (2445 B), `.sk` (72 B), 2026-09-23 | not inspected | 27, 29 |
| `/root/.dowiz_offsite_s3` (324 B), 2026-09-24 | **not inspectable**: the auto-mode classifier refused reading its structure (Credential Exploration). Main or the operator must say whether it holds R2 S3 credentials for a bucket | 26, 28 |

No Stripe, Telegram, WhatsApp/Meta, OTEL, ebills.al or Wolt credential exists on the box.

### F-D. Contract findings from reading the code

- `notify/tg.rs:140` `send() -> Result<(), Fail>` throws away Telegram's `message_id`. Without it a
  probe cannot read the message back at Telegram. A bot never receives its own messages as updates.
  Fix (NEEDS-CODE, small): keep the id in the rail outcome and expose the last one in
  `/api/owner/health.rails`. The probe then calls `forwardMessage`/`deleteMessage` with it.
- `stripe.rs` sends no `Stripe-Version` header (`grep -rn "Stripe-Version" workers/api/src` -> 0), so
  the API version is the Stripe account's default and is unpinned. The contract for row 10 pins it
  only once keys exist.
- `services/ordering/rates.rs:75-115`: when er-api fails, the identity answer `stale:true` is
  `cache_put` for an hour like a good one. An upstream answer that is itself days old passes as
  `stale:false`, because `asOf` is passed through and never compared to the clock. The row-59 probe
  checks the age itself.
- `.github/workflows/heartbeat-monitor.yml` (fixed here): "up" was `code < 500`, which reads a 403
  challenge as healthy.

### F-E. Rules every probe obeys (phase B)

1. **Host guard.** The runner refuses at startup unless the host's first label starts with `qa-`. It
   reuses `e2e/flows/lib.mjs`, whose guard already does this and exits 2. External services are named
   one by one in each contract, never by wildcard.
2. **Status.** `LIVE-PROVEN` only when the effect was read back at the consumer AND the live response
   validated against the contract's schema. A probe that cannot reach its service is `FAILED`, or
   `NEEDS-KEY` when the key is absent. A skip is never a pass. The runner prints `proven=N of M run`,
   and `M` must equal the LIVE-NOW count, or the run fails (memory gates-that-count-skips-as-passes).
3. **Clean-up.** Every TEST order or booking carries the prefix `LIVE-<run>` in the contact name and is
   ended (cancel/reject/refund) by the same probe. Restore data outlives the run in a pending file, the
   same pattern as `e2e/flows/lib.mjs` `PENDING`.
4. **Reuse, do not duplicate:** `e2e/flows/lib.mjs` (`api`, `owner`, `menu`, `ownerOrders`,
   `statusOf`, `_retry-fetch`, the IPv4-first DNS fix), `e2e/flows/f1..f4` for the browser checkout,
   `e2e/walk/q2-kitchen.mjs` (kitchen), `q5-stock.mjs` (stock), `courier.mjs`, and
   `tools/evals/collect/cf.mjs` (Workers analytics). `e2e/qa-staging/_harness.mjs` points at
   the legacy host, the stack removed on 2026-07-15. Only its `makePage` and `recorder` shapes are
   reusable, so the runner does not import it.
5. **Output:** one JSON line per row, `{row, status, evidence, at, build, contract}`, where `build` is
   the live `/api/version.commit` and `contract` is the contract's version.

## 2. Class counts

| class | n | rows |
|---|---|---|
| LIVE-NOW | 28 | 1 2 3 4 14 15 16 17 18 19 20 21 22 23 30 31 33 37 38 39 43 44 47 48 58 59 60 61 |
| NEEDS-KEY | 15 | 5 6 7 8 10 13 24 26 27 28 29 32 40 42 62 |
| NEEDS-CODE | 10 | 9 11 25 34 35 36 41 45 46 49 |
| NOT-LIVE-BY-DESIGN | 9 | 12 50 51 52 53 54 55 56 57 |

## 3. The rows

Format: **probe** (steps on qa-durres) · **read-back** (where the effect is read, at the consumer) ·
**external** (the real third-party endpoint, or "none: the hub is the consumer") · **secret** (name,
and whether it exists today) · **class**. Every contract is in `tools/live-proof/contracts/<row>.json`.

**1 Storefront ordering.** Probe: a real Chromium at 390x844 opens `https://qa-durres.dowiz.org/`,
adds QA Water, checks out with pickup and cash, contact `LIVE-<run>`. This reuses `e2e/flows/f1-customer.mjs`.
Read-back: `GET /api/order/:id` with the customer's `access_token`, then `GET /api/owner/orders`. Both
must show the order PENDING with the same total. Clean-up: owner `reject`. External: none.
Secret: OWNER_EMAIL/PASSWORD (exist). **LIVE-NOW.**

**2 Order for later.** Probe: `POST /api/public/locations/qa-durres/orders` with the storefront's own
body (as saved by `e2e/walk/q2-kitchen.mjs`) plus `scheduled_for_ms = now + 3 h`, rounded to the
venue's slot. Read-back: the owner's order shows `scheduled_for_ms` equal to what was sent, and so does
the kitchen board `GET /api/staff/kitchen`. Then reject. External: none. Secret: owner (exists).
**LIVE-NOW. Expected FAILED on e7d11a55** (dropped at ingress). Expected LIVE-PROVEN after fd777995 is
deployed.

**3 Kitchen ack.** Probe: place a TEST order, then invite a TEST kitchen staff member
(`POST /api/owner/staff/invite`), claim the invite, and log in (`/api/staff/login`). Then
`POST /api/staff/orders/:id/kitchen-ack`. Read-back: the owner's order has `kitchen.seen.at`, and
`/api/owner/health.kitchen.unseen` does not list it. Clean-up: reject the order, suspend the staff member.
External: none. Secret: owner (exists). **LIVE-NOW.**

**4 Print rail.** Probe: mint a venue API key for the "printer" (`POST /api/owner/apikeys`), place an
order, then `POST /api/print/poll` with the key, `GET /api/print/job/:token`, `DELETE /api/print/job/:token`.
Read-back: the ticket bytes contain the order's dish names, and `/api/owner/print/jobs` shows the job
acked. The probe is the printer, because the printer is the real consumer protocol. A physical printer
is not part of this proof, and the row says so. Clean-up: revoke the key, reject the order. External:
none (hardware not covered). Secret: owner (exists). **LIVE-NOW.**

**5 Outbox -> Telegram bell/digest.** Probe: `POST /api/owner/telegram/connect` with a TEST bot token,
link the TEST group, place an order, and let the DO alarm drain the outbox. Read-back at Telegram: the
`message_id` from health.rails (needs F-D) is passed to `forwardMessage` into the probe's private chat.
The text must contain the order id. Then `deleteMessage` on both. External: `https://api.telegram.org/bot<T>/`
(Bot API, unversioned URL). Secret: **a TEST bot token + the chat id of a group the operator is in**.
Neither exists. **NEEDS-KEY** (+ the small message_id change, F-D).

**6 Customer Telegram link.** Probe: GET the public menu. `telegramBot` must equal `getMe.username` of
the platform's customer bot, and `https://t.me/<username>` must answer 200. External: Telegram Bot API
`getMe`. Secret: Worker secret `TELEGRAM_BOT_USERNAME` (MEASURED unset: `"telegramBot":null` on
qa-durres) and its bot's token for `getMe`. These are platform-wide, so they also turn the link on for
the real venues, which is what the product wants. **NEEDS-KEY** (operator: create the customer bot and
`wrangler secret put TELEGRAM_BOT_USERNAME`).

**7 Outbox -> WhatsApp / campaign.** Probe: set `notify.whatsapp.{token,phone_id,to}` on qa-durres to
the Meta TEST number, then place an order. Read-back: Graph accepts with a `wamid`, and the delivery
status arrives at `/api/webhooks/meta` (needs row 9). Until 9 exists, the honest ceiling is "Graph
accepted", which the evidence states. External: `https://graph.facebook.com/v21.0/<phone_id>/messages`
(CITED `channels.rs:31`). Secret: **Meta app + WhatsApp Cloud API test number + a permanent token +
one verified recipient number the operator owns**. None exist. **NEEDS-KEY.**

**8 Meta webhook (WA/IG inbound).** Probe: a real inbound message needs a real WhatsApp sender. The
schedulable option is two Cloud API numbers: A sends to B, and B's webhook is qa-durres. Read-back:
`GET /api/owner/inbox` shows the message from A with its text. Signature `x-hub-signature-256` with
`notify.meta.secret` = the app secret. A self-signed POST would prove only Worker -> DO and is not
counted. External: Meta Graph v21.0 + webhooks. Secret: as row 7, plus the **app secret** and a
**second WhatsApp number**. **NEEDS-KEY.**

**9 WhatsApp `statuses`.** The consumer does not exist (`channels.rs:250-263` reads `messages` only).
The runner reports NOT-LIVE "disconnected in code (D21/N1-C)". **NEEDS-CODE** (+ row 7's keys).

**10 Stripe payment.** Probe, once isolated: place an order with `payment:"card"`, confirm the
PaymentIntent server-side with Stripe TEST `pm_card_visa`, and wait for Stripe's real webhook to reach
`/api/webhooks/stripe`. Read-back: the order shows paid, and `GET https://api.stripe.com/v1/payment_intents/<id>`
shows `succeeded` with `metadata.order_id` = the order. External: `https://api.stripe.com/v1/` with
**no Stripe-Version pinned today** (F-D). Secret: `STRIPE_SECRET_KEY`, `STRIPE_PUBLISHABLE_KEY`,
`STRIPE_WEBHOOK_SECRET`. All are unset (MEASURED `"stripePublishableKey":null`) and **platform-wide**
(CITED `stripe.rs:49,132`, `storefront.rs:851`). TEST keys on the production Worker would put every real
venue into test mode. **NEEDS-KEY + NEEDS-CODE (per-venue keys or a QA Worker).**

**11 Card refund.** No code calls `/v1/refunds`. **NEEDS-CODE** (W-REFUND) after row 10.

**12 Fiscal send.** NOT-LIVE by decision (operator 2026-09-24: import only; `SEND_ENABLED=false`).
The probe still runs, read-only. `POST /api/owner/fiscal/ebills` arm must answer 409, and
`/api/owner/health.fiscal` must show `configured:false` with 0 queued. Anything else is FAILED, because
a queue would age silently. **NOT-LIVE-BY-DESIGN.**

**13 eBills import (till -> dowiz).** Probe: configure qa-durres's ebills link to a TEST ebills.al
account (`POST /api/owner/ebills/config`) and wait for one alarm poll. Read-back: `/api/owner/ebills`
shows `lastPollMs` advanced with no error, and the imported sale appears in `/api/owner/orders` with
channel ebills. External: `https://www.ebills.al` (CITED `ebills/client.rs:26`). Secret: **an ebills.al
test account with at least one sale**. Pointing qa-durres at Dubin's real account would copy a real
venue's fiscal data into QA, and this lane will not do it. **NEEDS-KEY.**

**14 Courier invite.** Probe: `POST /api/owner/couriers/invite`, then claim the code on the qa host
with `POST /api/courier/auth/claim` and a TEST phone, then log in. Read-back: `GET /api/owner/couriers`
lists the new courier, and the courier's `/api/courier/tasks` answers 200 for qa-durres. Clean-up:
`POST /api/owner/couriers/:id/uninvite`. External: none. This proves invite -> claim. Delivery of the
code by a rail is still DISC (F5). After fd777995 the code travels as a shareable link, and the contract
moves to 1.1.0. Secret: owner (exists). **LIVE-NOW.**

**15 Courier login venue.** Probe: QA_HUB_COURIER logs in on the qa-durres host, and the token's venue
must be qa-durres. The same login with a body `location_id` of another venue must be refused as a
contradiction. Other venues' hosts are never touched. External: none. Secret: QA_HUB_COURIER_* (exist).
**LIVE-NOW.**

**16 Courier claim / app.** Probe: a TEST pickup order reaches READY, then the courier turns the shift on,
takes it from the pool, and picks it up. It is not delivered: no exit from DELIVERED, see
`e2e/walk/courier.mjs`. It is ended by `refused` + `returned`. Read-back: the owner's order shows the
courier id and status at every step. A real Chromium opens `/courier/` and counts `/api/live` socket
opens (memory dowiz-live-bugs-only-a-browser-found). External: none. Secret: QA_HUB_COURIER_* (exist).
**LIVE-NOW.**

**17 Live ETA / positions.** Probe: during 16, `POST /api/courier/position` with a point in Durrës.
Read-back: the customer's `GET /api/order/:id` (their token) carries that position, rounded as the
store map rounds it. External: none. **LIVE-NOW.**

**18 Booking / floor / room / pass.** Probe: `GET /tables` with `slotMin` finds a free slot, then the
guest `POST /reservations` with a TEST name and phone. Read-back: `GET /api/owner/reservations` shows it.
The share link's `GET /reservations/:id` and `/pass` verify, then cancel through the guest's action.
External: none. **LIVE-NOW.**

**19 Wallet / threads.** Probe: with the customer token from row 1, `GET /wallet` and
`/wallet/statement`, then `POST /threads/:id/messages`. Read-back: `GET /api/owner/threads` shows the
message. External: none. **LIVE-NOW.**

**20 Stock ledger.** Probe: reuse `e2e/walk/q5-stock.mjs`. QA Salmon roll = 80 g salmon + 120 g rice
(qa-setup). Read `/api/owner/stock`, place one roll, advance it to READY, and read again. Read-back: the
salmon falls by exactly 80 g and the rice by exactly 120 g. Then refund, which puts the stock back.
External: none. **LIVE-NOW.**

**21 Recipes import.** Probe: `POST /api/owner/recipes/import` dry run, then apply on a QA dish.
Read-back: `GET /api/owner/products/:id/takes` lists the lines. Restore the old lines from the pending
file. External: none. **LIVE-NOW.**

**22 Analytics.** Probe: read `/api/owner/analytics`, place an order and advance it to READY, then read
again. Read-back: the count and revenue move by exactly that order. Read `/api/owner/analytics/kitchen`
the same way. External: none. **LIVE-NOW.**

**23 Privacy: forget.** Probe: an order with a fresh TEST phone, then `POST /api/owner/customers/:key/forget`.
Read-back: `/api/owner/customers` no longer reveals the phone, and `/api/owner/health.redacted`
increases by the order's records. External: none. **LIVE-NOW.**

**24 Retention prune.** Probe: Workers analytics (`tools/evals/collect/cf.mjs`, the account analytics
token) shows that yesterday's 03:17Z scheduled invocation succeeded. Proving that records were deleted
needs the oldest errlog record's age, which only `/api/platform/errors` returns, and that route is
platform-admin only. Secret: **a platform-admin login for the probe** (no route writes `platform_admins`;
operator decision). **NEEDS-KEY.**

**25 Crypto-shredding.** Feature `shred` is not compiled into the Worker. **NEEDS-CODE** (DG10w/DW8).

**26 Nightly S3 copy.** Probe: set `cloud.s3.{endpoint,region,bucket,key,secret,prefix}` on qa-durres.
The endpoint is the R2 S3 API (`https://<account>.r2.cloudflarestorage.com`), the bucket is a QA bucket,
and the prefix is `live-proof/`. Then `POST /api/owner/backup/cloud`. Read-back at the sink: a SigV4 GET
of the returned `key` from R2 with the same S3 credentials. The bytes must gunzip to a bundle whose
venue is qa-durres and whose order count equals `/api/owner/health.orders`. Clean-up: delete the object
and clear the settings. External: R2's S3 API (AWS SigV4, `s3` service, region `auto`). Secret: **R2 S3
credentials scoped to one QA bucket**. Main must say whether `/root/.dowiz_offsite_s3` is that (F-C).
Creating a bucket or token is not this lane's to do. **NEEDS-KEY** (it becomes LIVE-NOW the moment main
confirms the file).

**27 Backup seal.** Probe: `/api/owner/health.backupSeal` must say sealed. Then the row-26 push must
produce a `*.sealed` object, not plain gzip. External: as 26. Secret: Worker secret `BACKUP_SEAL_PK`,
platform-wide. The key pair exists on the box (`/root/.dowiz_backup_seal.pk`, 2026-09-23), but the 10-02
report says the secret is not installed (OA-3/F16). **NEEDS-KEY** (main/operator: `wrangler secret put
BACKUP_SEAL_PK` from that file, then deploy).

**28 Witness / chain_check.** Probe: after row 26's push, `GET /api/owner/backup/cloud` gives
`witness.found:true` and a tip equal to the venue's log tip. The `*.witness.json` object at the
returned `witnessKey` is read from R2, and its `tip`/`records` must match. External: as 26.
**NEEDS-KEY** (same key as 26).

**29 seal-open.** Probe, on the box only: fetch row 27's sealed object and open it with
`tools/seal-open` and `/root/.dowiz_backup_seal.sk`. `gunzip -t` must accept the result. The secret half
must never go to GitHub, so this probe is box-only and the workflow skips it as NEEDS-KEY. **NEEDS-KEY**
(after 26+27).

**30 MCP server.** Probe: mint an owner API key (`POST /api/owner/apikeys`), then speak MCP to
`POST /api/mcp` as a client: `initialize`, `tools/list`, and a read tool (orders). Read-back: the tool's
answer equals `/api/owner/orders`, and the role list matches `GET /api/mcp` (MEASURED 200, protocol
`2025-06-18`). Revoke the key, and a call after revoke must be 401. External: none, because the probe is
the MCP client. **LIVE-NOW.**

**31 Learn / lessons.** Probe: with an owner token, `GET /api/learn/manifest` (MEASURED 401 without one),
then `GET /api/learn/media/<first key>`. Read-back: the bytes are more than 0, the content type is
video/* or image/*, and the length equals the R2 object's size in `LEARN`, listed with the deploy token
(read only). External: R2 (Cloudflare API, read). **LIVE-NOW.**

**32 Landing / waitlist / mail.** Probe: `POST https://dowiz.org/api/waitlist` with a TEST address.
Read-back: the mail arrives in a mailbox the probe can read, and the entry is visible in
`/api/platform/waitlist`. This writes outside qa-durres, to the platform's `__platform` object, and it
mails the operator. Secret: **operator permission for one TEST entry per run + a readable test mailbox
(e.g. a Cloudflare Email Routing address forwarding to an IMAP box) + a platform-admin login**.
**NEEDS-KEY.**

**33 Landing PQ copy.** Probe, read only: `GET https://dowiz.org/platform/landing-words.js` must be
byte-identical to the deployed commit's file, and `tools/gates/pq-words.sh`'s forbidden phrases must
occur 0 times in it. MEASURED today: 200, 39773 B, `every packet` 0. External: none. **LIVE-NOW.**

**34 ML-DSA live signer** / **35 Event chain signature** / **36 kernel pq shim.** There is no live path.
The probe asserts nothing live and reports NOT-LIVE "disconnected in code (SEL4-4 / CLEAN-PQ)".
**NEEDS-CODE.**

**37 Auth tokens.** Probe: owner login, then `/api/owner/orders` is 200. The same token with one flipped
payload byte gives 401, and no token gives 401. External: none. **LIVE-NOW.**

**38 Customer phone mask.** Probe: place an order with a TEST phone, then read `/api/owner/customers`.
Compute the mask with the code's fallback key `dowiz-unconfigured` (CITED `customers/handlers.rs:46-52`),
using the same function as the code. Equal means `AUTH_SIGNING_KEY` is unset and the row is FAILED
(predictable mask). Different means LIVE-PROVEN. External: none. **LIVE-NOW** (the expected outcome is
unknown until it runs).

**39 Bootstrap.** Probe, box only: `POST /api/bootstrap` on qa-durres with `BOOTSTRAP_SECRET`.
Idempotent: the response matches the existing seed and the catalogue's generation is unchanged. A wrong
secret is refused. In CI only the refusal half runs, because `BOOTSTRAP_SECRET` can seed any venue and
does not go to GitHub. **LIVE-NOW.**

**40 OpenTelemetry export.** Probe: the Worker exports to `OTEL_EXPORTER_OTLP_ENDPOINT` + `/v1/traces`
(CITED `otel.rs:180`). Read-back: the collector's query API returns the probe's trace (the probe sends a
`traceparent` with its own id). Secret: **an OTLP endpoint + its query token** (e.g. a free Grafana Cloud
stack). The setting is platform-wide. **NEEDS-KEY.**

**41 W-TELEM / DW6.** Not merged. **NEEDS-CODE.**

**42 AI assistant.** Probe: set `ai.enabled`, `ai.endpoint`, `ai.token` and `ai.model` on qa-durres,
then `POST /api/owner/integrations/check {which:"ai"}` and `/api/owner/assist` with a question.
Read-back: an answer that names a dish from the QA menu (redacted facts only). External: any
OpenAI-compatible endpoint (`GET <base>/models`, `POST <base>/chat/completions`). Secret: **an
OpenAI-compatible endpoint + token**, e.g. Cloudflare Workers AI's OpenAI-compatible endpoint with a
Workers-AI-scoped token. None exists today. **NEEDS-KEY.**

**43 Voice parser.** Probe: `POST /api/voice` with a TEST transcript ("two QA salmon rolls"). Read-back:
the parsed intent names the QA Salmon roll with quantity 2, the same as `admin/voice.js` renders.
External: none (the parser is local). **LIVE-NOW.**

**44 DAG timers vs cron.** Probe: (a) Workers analytics with the account analytics token, reusing
`tools/evals/collect/cf.mjs`: exactly 1 scheduled invocation per day, the last one within 26 h and
successful. (b) Enqueue an outbox entry (an order on a venue with a print/notify rail) and make no
further request. Read-back: `/api/owner/health.outbox.waiting` falls to 0 within 2 min, driven by the
DO alarm alone. External: Cloudflare GraphQL (`workersInvocationsAdaptive`). Secret: analytics token
(exists). **LIVE-NOW.**

**45 DW7 block** / **46 DW1 replica** / **49 DG8 Datalog.** The consumer is absent. **NEEDS-CODE.**

**47 Console replica.** Probe: `GET /api/owner/orders` gives `generation` g, then place an order, then
`GET /api/owner/orders?since=g`. Read-back: `changes` contains the order and `generation` > g, folded
the same way `lib/replica.js` folds it. External: none. **LIVE-NOW.**

**48 Offline shell.** Probe: Chromium opens `/` and waits for SW `activated`, then `setOffline(true)`
and reload. The shell renders and `/api/*` is not served from the cache. Also a GET of `/sw.js`: its
`SHELL_CACHE` equals the deployed commit's file. External: none. **LIVE-NOW.**

**50-54 (13 core modules, agent-loop, legacy dirs, apps/courier, project.rs).** Nothing builds or calls
them. The runner reports NOT-LIVE with the reason. A check that they stay unreferenced belongs in
`unreached.py`, not here. **NOT-LIVE-BY-DESIGN.**

**55 heartbeat-monitor.** Retargeted in this lane (F-A). The old target `webhook.dowiz.org` -> 530 is
kept as a comment, so the row stays NOT-LIVE as a design (tunnel retired). The new heartbeat itself is
covered by row 58. **NOT-LIVE-BY-DESIGN.**

**56 visual.yml / 57 academia\*.** Old stack, and academia is dispatch-only since 0206f5dc.
**NOT-LIVE-BY-DESIGN.**

**58 CI workflows.** Probe: GitHub's public Actions API, no token, rate limit 60/h:
`GET /repos/SyniakSviatoslav/dowiz/actions/workflows/<file>/runs?per_page=5` for health-cron,
heartbeat-monitor, key-flows, evals-nightly and live-proof. Read-back: the latest conclusion and its age
must fit the schedule. For a failure, the check-run annotations are quoted
(`/check-runs/<job>/annotations`). MEASURED today: health-cron failure 8/8, evals-nightly failure
(10-01, 10-02), heartbeat "success" (a false green, F-A). External: `https://api.github.com`
(REST, `X-GitHub-Api-Version: 2022-11-28`). Secret: none. **LIVE-NOW. Expected FAILED** until F-A is
resolved.

**59 Currency rates.** Probe: `GET https://qa-durres.dowiz.org/api/public/rates?base=ALL` and
`GET https://open.er-api.com/v6/latest/ALL`. Read-back: `stale:false`, and each `ppm[c]` equals
`round(rate*1e6)` from er-api's answer of the same `time_last_update_utc`. `asOf` must be younger than
48 h. MEASURED: Worker `{"asOf":"Sat, 03 Oct 2026 00:02:32 +0000","base":"ALL","ppm":{"ALL":1000000,"EUR":10861,"USD":12295},"stale":false}`.
er-api `"EUR":0.010861,"USD":0.012295` -> equal. **When er-api is down:** the Worker answers identity
ppm + `stale:true` and caches that for 1 h (CITED `rates.rs:113-115`). The storefront then shows lek
only, which is FAILED (degraded), not a pass. **When er-api is stale:** the Worker still says
`stale:false` (F-D). The probe's age check catches it. External: `https://open.er-api.com/v6/latest/{base}`
(ExchangeRate-API open access, v6, daily update, attribution required by its terms). Secret: none.
**LIVE-NOW.**

**60 Map tiles.** Probe: GET the style the storefront uses (`https://tiles.openfreemap.org/styles/liberty`,
CITED `store/address.js:31`; also `positron`, `bright`, `dark`), then its TileJSON
(`https://tiles.openfreemap.org/planet`), then the Durrës tile z14/9076/6122 from the TileJSON's
`tiles[0]` template. Read-back: style `version:8`, TileJSON `tilejson:"3.0.0"`, tile 200
`application/vnd.mapbox-vector-tile` of more than 1 KB. The live CSP (`img-src`/`connect-src`) must
name `https://tiles.openfreemap.org`. A Chromium storefront visit counts tile requests, never pixels
(memory webgl-renders-nothing-on-the-box). MEASURED: style 200 43079 B; TileJSON 200, tiles
`…/planet/20260927_080001_pt/{z}/{x}/{y}.pbf`; tile 200 55980 B; CSP includes it. External:
OpenFreeMap (MapLibre style spec v8, TileJSON 3.0.0, OpenMapTiles schema). Secret: none. **LIVE-NOW.**

**61 Address geocoder.** Probe: one reverse lookup per run (`/reverse?format=jsonv2&lat=41.3153&lon=19.445&zoom=18&accept-language=sq`,
the storefront's exact query, CITED `store/address.js:96`). It sends a real User-Agent naming dowiz and
the repo, with at most 1 request/s and 3 per run (Nominatim usage policy). Read-back: 200, `licence`
says OpenStreetMap, and there is a `display_name` and `address`. The live CSP `connect-src` must name
`https://nominatim.openstreetmap.org`. MEASURED: 200, `place_id` 57009419, licence "Data © OpenStreetMap
contributors". External: Nominatim API (jsonv2 output). Secret: none. **LIVE-NOW.**

**62 Order aggregator (Wolt-type).** Today an aggregator order is entered by hand
(`POST /api/staff/orders/aggregator`, `command/aggregator.rs`; "there is no partner access"). The
probe can run that half on qa-durres: enter `wolt-LIVE<run>` twice. The second entry must return the
first order, and a different basket must give 409. But the row's real link is the partner's. Wolt
Order API, as documented on developer.wolt.com/docs/webhook and /docs/api/order (fetched 2026-10-03):
- webhook `POST` to our endpoint `{"id","type":"order.notification","order":{"id","venue_id","status","resource_url"},"created_at"}`,
  header `WOLT-SIGNATURE` = HEX HMAC-SHA256 of the body with the client secret, 3 retries at 5 s;
- `GET /orders/{id}` and `GET /v2/orders/{id}`, `PUT /orders/{id}/accept|reject|ready|delivered|confirm-preorder`,
  `Authorization: Bearer <JWT from the OAuth 2.0 flow>`;
- base `https://pos-integration-service.wolt.com`, test `https://pos-integration-service.development.dev.woltapi.com`.
Glovo's partner API was not fetched (UNVERIFIED). Secret: **a Wolt partner (POS integration) account
with test-venue credentials**. **NEEDS-KEY.**

## 4. Schedule (phase C design)

`.github/workflows/live-proof.yml`: `cron: '7 */6 * * *'` + `workflow_dispatch`. One job, the
Playwright container like `key-flows.yml` (its own PID namespace), running `node tools/live-proof/run.mjs`.
The QA host is refused unless it starts with `qa-`. Alerting reuses health-cron's step verbatim: secret
`TELEGRAM_BOT_TOKEN`, variable `OPS_TELEGRAM_CHAT_ID`. It fires on any `FAILED` line; NEEDS-KEY and
NOT-LIVE never page. **It will be challenged by BFM (F-A) until the operator chooses option 1 or 2.**
The workflow detects the challenge with `cf-mitigated` and says so in one line, instead of 28 FAILED.

Secrets it needs (named, not set):
- `LIVE_HOST` (https://qa-durres.dowiz.org), `LIVE_OWNER_EMAIL`, `LIVE_OWNER_PASSWORD`,
  `LIVE_COURIER_PHONE`, `LIVE_COURIER_PASSWORD` (the QA_HUB_COURIER pair);
- `LIVE_CF_ANALYTICS_TOKEN` + `LIVE_CF_ACCOUNT_ID` (row 44);
- later, as the NEEDS-KEY rows unlock: `LIVE_TG_TOKEN`, `LIVE_TG_CHAT`, `LIVE_S3_*`, `LIVE_META_*`,
  `LIVE_STRIPE_TEST_SECRET`.
Never in GitHub: `BOOTSTRAP_SECRET` and the seal `.sk`, so rows 29 and 39's write half are box-only.

## 5. Operator asks (uk, one line each)

1. Вимкнути Bot Fight Mode для dowiz.org (Security → Bots) або погодитися, що перевірки за розкладом ідуть не з GitHub. Інакше всі перевірки з GitHub бачать лише сторінку-челендж.
2. Додати в GitHub секрет `TELEGRAM_BOT_TOKEN` (токен бота для алертів) і змінну `OPS_TELEGRAM_CHAT_ID` (id чату, куди слати алерти). Без них про червоні запуски ніхто не дізнається.
3. Створити тестового Telegram-бота і групу, де є оператор; дати токен бота й id групи (ряд 5).
4. Створити бота для клієнтів і встановити секрет Worker `TELEGRAM_BOT_USERNAME` (ряд 6).
5. Meta: застосунок, тестовий номер WhatsApp Cloud API, постійний токен, app secret, один свій номер-отримувач і, для вхідних, другий номер WhatsApp (ряди 7, 8).
6. Stripe: TEST-ключі (secret, publishable, webhook secret) і рішення, як ізолювати QA: ключі на заклад чи окремий QA-Worker (ряди 10, 11).
7. Тестовий акаунт ebills.al з хоча б одним продажем (ряд 13).
8. R2: окремий QA-бакет і S3-ключі лише для нього, або підтвердити, що `/root/.dowiz_offsite_s3` — саме це (ряди 26, 28).
9. Встановити `BACKUP_SEAL_PK` з наявного `/root/.dowiz_backup_seal.pk` і задеплоїти (ряди 27, 29).
10. Акаунт платформ-адміна для перевірок (ряди 24, 32) і дозвіл на один TEST-запис у waitlist за запуск плюс скриньку, яку перевірка може читати (ряд 32).
11. OTLP-ендпоінт і токен для читання трас, наприклад Grafana Cloud free (ряд 40).
12. OpenAI-сумісний ендпоінт і токен для асистента, наприклад Cloudflare Workers AI (ряд 42).
13. Партнерський акаунт Wolt (POS integration) з тестовим закладом (ряд 62).

## 6. Phase B entry conditions

Everything phase B needs for the 28 LIVE-NOW rows exists on the box. That covers the credentials
(`/root/.dowiz_owner`, the analytics token), the QA hub, a `playwright` that `e2e/flows` already uses,
and network to every endpoint named above (MEASURED in phase A). Phase B waits only for "SLOT YOURS".
