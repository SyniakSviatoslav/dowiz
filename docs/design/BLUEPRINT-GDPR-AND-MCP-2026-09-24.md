# Privacy (GDPR + Albania Law 124/2024) and MCP for every role: what exists, what is missing, what to build

**Date:** 2026-09-24 · **HEAD read:** `5abeb0ac` · **Lane:** research + planning (Opus), read-only on code.
**Roadmap rows:** `ROADMAP-2026-09-22.md` § "Wave P". Every claim about the tree cites `file:line` at that
HEAD; every claim about law or a protocol cites a URL (list at the end). Nothing here was run against
production. **This is an engineering plan, not legal advice — §6 lists what a lawyer must decide.**

---

## 0. The findings that change the plan (read these if nothing else)

1. **Albania's deadline is 30 days, not "one month", and erasure has its own 30-day clock.** Law 124/2024
   Art. 12(4): answer "no later than 30 (thirty) days", extendable to 60; Art. 15(2): erasure "as soon as
   possible no later than 30 (thirty) days" from receipt. GDPR Art. 12(3) is one month + two. The forget
   route's own promise — *"copies older than tonight in the venue's own backup expire within one month"*
   (`workers/api/src/services/customers/forget.rs:255`) — is **false today**: the nightly copies are kept
   **35 days** (`cloud.rs:264`, `KEEP_WEEKLY_MS = 35 d`; `keys_to_drop` keeps one copy per week for five
   weeks, `cloud.rs:274-298`). One constant change (P3) makes the sentence true under both laws.
2. **A restore can bring a forgotten person back.** `POST /api/owner/restore` (`lib.rs:434`,
   `services/operations/mod.rs:274`) imports a bundle verbatim; nothing reapplies earlier erasures, and a
   bundle from before the erasure carries the phone in the clear. Same for Cloudflare's 30-day
   point-in-time recovery of the Durable Object. Fix: a pseudonymous **erasure register** outside the
   venue's images, replayed after every restore/import (P3).
3. **C3 ("forget the customer") landed and is sound for four stores — and does not touch seven others.**
   `c8e82907` redacts `people`, `consent`, the hot `log` and every archive, in one object turn, with a
   `Forgotten` declaration (`hubdo/forget.rs:60-135`). It does **not** reach `inbox` (WhatsApp/Instagram
   threads), `threads` (guest↔staff chat), `bookings` (`contact_name`/`contact_phone`, `booking.rs:89-90`),
   `outbox` (pending kitchen tickets carry name+phone+address, `notify.rs:106-126`), `idem` (cached full
   responses), `audit` (7-day error log), `campaign` send marks, nor messages already delivered to the
   venue's Telegram chat. §2 has the full map.
4. **The "tombstones = declarations" law is claimed but not gated.** `forget.rs:22` says a half-done
   erasure is "law 9 RED"; `e2e/gates/conservation.mjs:15-39` lists thirteen laws and law 9 is the tax
   block. `/api/owner/health` prints `redacted` and `declared` side by side
   (`services/operations/mod.rs:168-169`) and nothing compares them. P2 adds it as a law.
5. **The owner assistant sends customer names and phones to a third-party model.** `assist.rs` posts
   "facts" to the venue-configured `ai.endpoint`; the owner and courier facts include each order's full
   `contact` (`services/engagement/assist.rs:200, 265`). That endpoint is an undisclosed processor and
   probably an international transfer. Minimise first (P11), disclose second.
6. **The public privacy page is lorem ipsum.** `workers/api/public/kit/screens/privacy.js:9-19` — two
   headings ("Cancelation Policy", "Terms & Condition") over placeholder Latin. No venue has a notice
   (Art. 13 GDPR / Art. 13 Law 124).
7. **MCP exists, owner-only, keyed by a one-year all-powerful API key.** `POST /api/mcp`
   (`workers/api/src/mcp.rs:287`) speaks protocol `2025-06-18` (`mcp.rs:23`), 18 tools, JSON-only (no
   SSE), and trades a `dowiz_…` key for a five-minute owner JWT, then dispatches in-process through
   `crate::route` (`mcp.rs:169-230`) — **the right shape** (one authorisation path: the routes). But the
   key is a year-long owner credential with no scopes (`services/identity/keys.rs:25`), accepted on
   **every** route, not only `/api/mcp` (`auth.rs:495`), so any key can call `restore` or `forget`. There
   is no OAuth, so Claude.ai/Desktop connectors, `codex mcp login` and `opencode mcp auth` cannot connect
   as a person, and staff, couriers, guests and the platform operator cannot connect at all.
8. **The MCP spec moved twice since `mcp.rs` was written.** `2025-11-25` added Client ID Metadata
   Documents (CIMD) and `2026-07-28` made the protocol **stateless**: no `initialize` handshake, no
   `Mcp-Session-Id`, `server/discover`, per-request `_meta` version, `resultType` on every result,
   `Mcp-Method`/`Mcp-Name` headers, Multi Round-Trip Requests (`input_required`) replacing server-initiated
   elicitation, and DCR deprecated in favour of CIMD. Clients in the field still speak `2025-06-18` and
   `2025-11-25`, so the server must answer both (P15).

---

## 1. The law, verified

### 1.1 Albania — Law No. 124/2024 "On Personal Data Protection"

Read from the Commissioner's official English text (idp.al PDF) on 2026-09-24:

| Topic | Law 124/2024 | GDPR counterpart | Note for dowiz |
|---|---|---|---|
| Alignment | Footnote 1: "fully aligned with Regulation (EU) 2016/679 … and Directive (EU) 2016/680" | — | Design to GDPR, then apply the Albanian deltas below |
| Entry into force | Art. 101(1): 15 days after publication (published 17 Jan 2025 → in force 1 Feb 2025) | — | |
| **Delayed articles** | Art. 101(2): Art. 29(3) (telling data subjects of a breach), 31 (DPIA), 32 (prior consultation), 35, 36, 64, 65, 67(2,3,5) enter into force **two years after publication (≈ 17 Jan 2027)** | 34, 35, 36 | Build the DPIA and breach-to-subject paths now; they bind within four months |
| Legal bases | Art. 7 | Art. 6 | Orders: contract; fiscal receipts: legal obligation; marketing: consent |
| Consent | Art. 8 (demonstrable, distinguishable, withdrawable) | Art. 7 | `consent_log` already stores wording + channel (`services/customers/consent_log.rs`) |
| Minors | Art. 7(6): consent-based online processing lawful only from **16**; under 16 needs a parent | Art. 8 (16, states may lower) | Marketing consent needs an age statement; orders are contract, not consent |
| Transparency | Art. 12, 13 (information) | Art. 12-14 | §4 P8 |
| **Deadline** | Art. 12(4): **30 days**, +up to 60 with reasons given within the 30 | Art. 12(3): one month, +two | Use 30 days everywhere |
| Access | Art. 14 | Art. 15 | P5 |
| Rectification / erasure | Art. 15 (erasure **≤ 30 days**, grounds a–dh) | Art. 16, 17 | P2, P3 |
| Right to be forgotten (published data) | Art. 16 | Art. 17(2) | Nothing dowiz publishes carries a customer; reviews/feedback text must be checked (P1) |
| Restriction | Art. 17 | Art. 18 | P5 |
| Portability | Art. 18 | Art. 20 | P5 |
| Objection | Art. 19 | Art. 21 | Consent withdrawal exists; objection to legitimate-interest analytics does not |
| Automated decisions | Art. 20 | Art. 22 | `no-scoring` gate already forbids customer scores |
| By design / default | Art. 23 | Art. 25 | P1's gate is the "by design" evidence |
| Representative | Art. 25: a controller/processor outside Albania processing under Art. 4 must appoint a representative in Albania | Art. 27 | Depends on where dowiz is established — §6 |
| Processors | Art. 26 | Art. 28 | P9 |
| Records of processing | Art. 27 (notification to the Commissioner **abolished**) | Art. 30 | P10 |
| Security | Art. 28 | Art. 32 | |
| **Breach** | Art. 29(1): Commissioner "as soon as possible, but no later than **72 hours**"; (2) processor tells controller "immediately"; (3) tell subjects (from 2027) | Art. 33, 34 | P12 |
| DPIA | Art. 31 (from 2027) | Art. 35 | Courier live location = systematic monitoring of workers: screen it (P13) |
| DPO | Art. 33: public bodies, large-scale monitoring, large-scale sensitive data | Art. 37 | Probably not required for a venue; lawyer (§6) |
| Transfers | Art. 39-42; Commissioner **Decision no. 1 of 30 Apr 2025** lists adequate destinations: all EU/EEA states, Convention 108 parties with working DPAs, countries with an EU adequacy decision | Chapter V | Cloudflare, Meta, Stripe, Telegram are outside Albania — §2.4 |
| Direct marketing | Art. 46 | Art. 21(2-3) + ePrivacy | Campaigns (C6) already consent-gated by the `consent` gate |
| Fines | Art. 94: up to ALL 1 bn / 2 % turnover; up to **ALL 2 bn / 4 %** for principles, consent, **data-subject rights (Art. 12-20)** and transfers | Art. 83 | Rights failures are in the top tier |

### 1.2 GDPR

The venues are in Albania (not EU). GDPR reaches this platform in two ways that a lawyer must confirm
(§6): (a) **Art. 3(1)** if dowiz — the platform operator — is established in the EU, for its own
processing including as a processor; (b) **Art. 3(2)(a)** only where a venue *targets* people *in the
Union* (an EU tourist ordering while physically in Durrës is not "in the Union"). The practical answer is
the same either way: Law 124/2024 copies GDPR, so one design satisfies both, and the stricter number wins
(30 days from Albania; everything else from GDPR).

---

## 2. The data map — every personal-data field, where it lives, why, on what basis, for how long

**Roles.** The **venue** is the controller for its customers, its staff and its couriers. **dowiz** is the
venue's processor for all of that, and a controller for its own data (owner accounts, platform operator,
the waitlist). Every row below is per venue unless marked PLATFORM.

### 2.1 Venue stores (Durable Object `HubImages`, one per venue, SQLite-backed, `wrangler.toml:119-130`)

| Store (image) | Declared at | Personal fields | Subject | Purpose | Basis | Retention today | Erasure today |
|---|---|---|---|---|---|---|---|
| `log` (+ `log.archives`) | `hubstore.rs:31` | `contact.phone`, `contact.name`, `fulfilment.note`, `address.{line,note,parts,lat_udeg,lon_udeg}` in every `Placed`/delta `order_json` (`forget.rs:100-112`) | customer | fulfil the order; accounting | contract; legal obligation (money) | **forever** (rotated to archives, never pruned) | **YES** — redacted in place, declared (`forget.rs:149-184`) |
| `people` | `hubstore.rs:832` | the customer record (`services/customers/record.rs`), aliases | customer | CRM card | legitimate interest / consent | forever | **YES** (`forget_people`, `forget.rs:81-84`) |
| `consent` | `consent_log.rs:16` | key (HMAC pseudonym), channel, wording, time | customer | proof of consent | legal obligation (Art. 8(1)) | forever | **YES** — redacted, withdrawal kept as proof (`hubdo/forget.rs:81-93`) |
| `bookings` | `booking.rs:133` | `contact_name`, `contact_phone`, `phone_key` index (`booking.rs:89-103, 156-160`) | guest | table reservation | contract | forever | **NO** |
| `inbox` | `channels.rs:320` | WhatsApp `wa_id` (the phone number), Instagram id, message text | customer | customer care | legitimate interest | bounded by count ("the prune bounds the rest", `channels.rs:561`) | **NO** (the blueprint's step 3 was not built) |
| `threads` | `social.rs:79` | guest↔staff messages at a table/order | guest | service | contract | ? (not measured) | **NO** |
| `outbox` | `outbox.rs:33` | rendered Telegram/WhatsApp tickets: name, phone, address (`notify.rs:106-126`) | customer | tell the kitchen | contract | removed on delivery (`outbox/rails.rs:120`); abandoned after 6 tries | **NO** (a pending entry survives the erasure) |
| `idem` | `idempotency/mod.rs:39` | the **full response** of a replayable call (rule 1, `idempotency/mod.rs:13-15`) — a placed order echoes its contact | customer | exactly-once | contract | swept nightly | **NO** |
| `campaign` | `services/campaigns/campaign.rs:35` | sent marks per customer key | customer | "did we already tell them" | consent | pruned when filed (`send.rs:156`) | partial (key only) |
| `ledger` | `wallet.rs:102` | `wallet:<key>` postings, provider refs | customer | stored value | contract + accounting | forever | deliberately **kept** (Art. 17(3)(b)/(e); CRM blueprint §3.3 step 4) |
| `ops` | `hubstore.rs:829` | courier id, shift, **last position** | courier | dispatch, live ETA | contract (+ DPIA, §4 P13) | latest fix only, meaning expires in minutes (`hubstore.rs:1157-1180`) | n/a for customers; courier path **NO** |
| `audit` | `hubstore.rs:857` | errors naming a venue, courier audit, `Revealed` (who looked at which pseudonym) | staff, customer (pseudonym) | security | legitimate interest | 7 days (`errlog.rs:19`) | pseudonymous; error text unchecked |
| `till`, `fiscal`, `ebills`, `floor` | `command/till.rs:42`, `fiscal/wire.rs:30`, `ebills/state.rs:22-25` | staff ids on drawer events; fiscal docs (no buyer fields found: `grep -i buyer fiscal/` → 0) | staff | tax, cash control | legal obligation | forever | kept (legal) — staff ids only |
| `settings` | `hubstore.rs:40` | venue contact, `notify.*` tokens, `ai.endpoint` | venue | config | contract | forever | n/a |

### 2.2 Platform stores (the `__platform` object: `registry` + `identity`, `platform_store.rs:36-43`)

| Record kind | Declared at | Fields | Subject | Erasure today |
|---|---|---|---|---|
| `user` | `identity_store.rs:34` | email, display name, password hash | owner, staff | **NO route** |
| `member`, `roster`, `invite` | `:35, 42-43` | who works where, invited name, phone hash (`services/courier/hiring.rs:47`) | staff, courier | uninvite only |
| `courier` | `:41` | name, phone (hash index), password hash | courier | **NO route** |
| `admin` | `:36` | platform operator | operator | n/a |
| `refresh`, `csession`, `apikey` | `:38-40` | session ids, device hints, key hashes | all | expire |
| `wl` (waitlist) | `waitlist.rs:33` | email, venue name, language, source (`waitlist.rs:145`) | prospect | **NO route** |
| `witness` | `witness/mod.rs:34` | `(records, tip)` per image — no personal data | — | n/a |

### 2.3 Copies outside the object

| Copy | Where | Personal data | Reached by erasure? |
|---|---|---|---|
| Nightly bundle | venue's own S3-compatible bucket, `<prefix>/<venue>/<stamp>.json[.gz][.sealed]`; images `log, catalog, settings, posts, stock` + each archive once (`hubstore.rs:1501-1502`) | the log's contacts | **No** — ages out after **35 days** (`cloud.rs:264`) |
| Cloudflare DO point-in-time recovery | Cloudflare | every image | No — 30-day floor nothing here can shorten (`hubstore.rs:1490-1497`) |
| Owner's manual download | `GET /api/owner/backup` (`lib.rs:433`) → owner's device | everything | No — the venue's own copy; the notice must say so |
| Telegram kitchen chat | Telegram's servers | name, phone, address per ticket (`notify.rs:126`) | No |
| WhatsApp / Instagram | Meta | the conversation | No (Meta's side) |
| Venue AI endpoint | whatever `ai.endpoint` names | order `contact` in "facts" (`assist.rs:200, 265`) | No |
| Browser — storefront/kit | the diner's device | `dw_name`, `dw_phone`, `dw_addr`, `dw_addrs`, `dw_addr_parts`, `dw_orders`, `dw_bookings`, `dw_c_jwt`, `dw_c_last` (`public/kit/me.js:80-86`, `grep -rhoE "KEY = 'dw_"`) | No — the device's; a "forget me on this device" button is cheap (P5) |
| Browser — staff/courier | `dw_rt`, `dw_room_session`, `dw_room_till`, `dw_loc` | session tokens | logout clears |
| Service-worker caches | `public/sw.js`, `kit/sw.js`, `courier/sw.js`, `room/sw.js` | must be checked: an API response cached by a SW is a copy | unmeasured — P1 greps `caches.put` of `/api/` |

### 2.4 Processors and recipients

| Party | What it receives | Role | Contract | Transfer |
|---|---|---|---|---|
| Cloudflare (Workers, DO, KV `MEDIA`, Email Routing `send_email`) | everything | dowiz's sub-processor | Cloudflare Customer DPA (incl. SCCs) | US company, global edge; DO location is decided at object creation (jurisdiction option) — **unmeasured for the live venues** |
| Meta (WhatsApp Business Platform, Instagram, `graph.facebook.com`) | messages, numbers | processor for the venue for WhatsApp Business (WhatsApp Business data processing terms) | venue accepts Meta's terms | US/IE |
| Telegram (`api.telegram.org`) | kitchen tickets with name/phone/address | **no DPA exists**; it is the venue's own chat tool | none | UAE/global — treat as the venue's recipient; minimise (P11) |
| S3-compatible provider | the nightly bundle (sealable, `backupSeal`) | the venue's own processor, chosen by the venue | venue's | wherever the bucket is |
| Stripe (`api.stripe.com`) | payment data | processor / independent controller for payments | Stripe DPA | US/IE |
| ebills.al, e-fiskalizimi (`efiskalizimi-app.tatime.gov.al`) | invoices, NUIS, staff operator codes | ebills: processor; tax authority: recipient under legal obligation | ebills contract | Albania |
| Venue AI endpoint | order contact in facts | processor, unknown to the customer | **none** | unknown |
| `open.er-api.com` | exchange rates only | — | — | no personal data |

---

## 3. Gaps against the articles (GDPR numbering; Law 124 in brackets)

| Article | Gap | Row |
|---|---|---|
| 5(1)(e) storage limitation [6] | order contacts, bookings, inbox, waitlist kept forever; no schedule | P6 |
| 5(1)(c) minimisation [6] | assist sends contacts to a model; Telegram gets phone for pickup orders too | P11 |
| 5(2) accountability [22] | no data map in the tree; nothing stops a new store of personal data appearing without an erasure path | P1 |
| 6 [7] | bases not written down per purpose | P10 |
| 7 [8] | consent exists; no age statement for marketing consent (Art. 7(6) Law 124: 16) | P8 |
| 12 [12] | no request intake, no clock, 30 days not tracked | P4 |
| 13-14 [13] | notice is lorem | P8 |
| 15 [14] access | owner sees a masked list + reveal; no export of everything about one person | P5 |
| 16 [15] rectification | `PUT …/record` edits the card; log contacts are immutable except by redaction | P5 (document: correct via note + redaction) |
| 17 [15] erasure | 7 venue stores + platform records not reached; backups 35 d; restore resurrects | P2, P3, P7 |
| 18 [17] restriction | no "restricted" state | P5 |
| 20 [18] portability | no machine-readable export | P5 |
| 21 [19] objection | marketing: withdrawal exists; analytics/CRM legitimate interest: none | P5 |
| 25 [23] | covered by P1's gate as evidence | P1 |
| 28 [26] | no processor list, no dowiz↔venue DPA | P9 |
| 30 [27] | no ROPA | P10 |
| 32 [28] | seal exists but off until `BACKUP_SEAL_PK` (Wave F16); API keys unscoped, one year | F16, P14-P16 |
| 33-34 [29] | no breach runbook, no "became aware" timestamp | P12 |
| 35 [31] | no DPIA screening for courier tracking | P13 |
| 44+ [39-42] | transfers unmapped; DO location unmeasured | P9 |
| staff/courier rights | no export/erasure for courier, staff, owner, waitlist | P7 |

---

## 4. The design (GDPR part)

### 4.1 One registry, and a gate that makes "every store" a checked claim (P1)

A table in code, not a document: `workers/api/src/privacy/registry.rs` (new) — one row per store that can
hold personal data: `{ store, kind, fields, subject, purpose, basis, retention, erase: Eraser, export:
Exporter }`, where `Eraser` is an enum naming the function that reaches it (`Redact(log)`,
`Remove(table,kind)`, `Rebuild(logimage, predicate)`, `Retain(reason)`, `DeviceOnly`, `External(notice)`).
Everything else in this section is driven from this table: the export walks it, the forget command walks
it, the ROPA and the privacy notice are rendered from it.

**The gate** `tools/gates/personal-data.sh` (new) fails when:
- a `pub const IMAGE_… : &str = "…"` or `pub const K_… : &str = "…"` exists in `workers/api/src` whose value
  is not a row in the registry (including rows that say `NotPersonal` with a reason);
- a new `https://` host appears in `workers/api/src` that is not in the registry's recipients list;
- a new `localStorage/sessionStorage.setItem` key appears under `workers/api/public` not in the registry.

Built the way the other gates are: comments stripped first (`ROADMAP-2026-09-22.md` §5), a baseline that
may only fall, and a `.prove.sh` that adds a fake `IMAGE_X` and sees RED, removes it and sees GREEN.

### 4.2 Erasure that reaches every store (P2)

Extend `hubdo/forget.rs` — still ONE object turn, still ordered so a failure is loud — with the missing
stores, each through the registry's `Eraser`:

| Store | Mechanism | Why this one |
|---|---|---|
| `bookings` | redact `contact_name`/`contact_phone` in place, drop the `rsv.phone/<key>/…` index rows | bookings are event-folded like orders; keep the table's history, lose the person |
| `inbox` | rebuild the log image without the subject `whatsapp/<wa_id>` / `instagram/<id>` for every id linked to the key (aliases, C4) | the inbox is not witnessed, so a rebuild is allowed (CRM §3.3 step 3) |
| `threads` | same predicate rebuild over threads of the person's orders | |
| `outbox` | pending entries whose order id is in scope: rewrite the rendered text without the person, or drop if the order has ended | an ended order's ticket has no purpose |
| `idem` | remove cached responses for the person's order ids | a cached response is a copy |
| `campaign` | remove marks under the key | |
| `audit` | scrub error text matching the person's phone spellings | 7-day store, but a stray phone in an error string is still a copy |
| `ledger` | **keep**; answer states it and why (accounting); refund-first decision is the venue's (§6) | |

The **live-order refusal stays** (`forget.rs:228-230`): an order on its way still needs its address.

### 4.3 Backups, restores and the erasure register (P3)

- `KEEP_WEEKLY_MS` 35 d → **21 d** (`cloud.rs:264`). With a nightly copy and 7 days of dailies, the oldest
  copy holding an erased person is ≤ 22 days after the erasure, inside Albania's 30. (28 d also fits, but
  leaves no slack for a request that arrives on day 29 of the Art. 12(4) window; 21 d costs one weekly copy.)
- **Erasure register** (new record kind `forgot` in the platform `registry`): `{ venue, key, orders[],
  archives[], at }` — pseudonymous (the key is an HMAC; order ids are random). After `restore`/`import`
  (`services/operations/mod.rs:274-286`) and after any Cloudflare PITR (runbook step), the Worker replays
  every register entry for that venue through the same forget command. Idempotent by construction: the
  command already declares only `tombstones − declared` (`forget.rs:158-162`).
- The response notice changes to the true sentence: "*Copies in the venue's nightly backup expire within
  22 days and nothing reads them; a restored backup is re-forgotten automatically.*"
- The CRM blueprint's rule 4 (`people`/`consent` backed up by overwrite, never by date) is kept.

**Why not crypto-shredding now.** The CRM blueprint (§2.2, row C) already scored it: it needs a cipher in
the hub, a key image that must never be in a dated backup, and every `contact` reader to decrypt; and it
does nothing for history already in the clear. The re-entry condition stands (a copy the venue cannot
rewrite within the window). Redaction + short backup window + register is enough and is ~200 lines.

### 4.4 Requests, the clock, identity (P4)

- `POST /api/public/locations/:slug/privacy-request` (new): `{ kind: access|erase|rectify|restrict|object|
  port, phone, order_id?, message }`. Files a `dsr` record in the venue's `people` image with
  `received_at` and `due_at = +30 d`; returns a reference. No action happens on this route — it is the
  intake, not the act (a token that can erase a person is a token whose theft erases a person, CRM §3.3).
- **Identity check** (Law 124 Art. 12(3)): either the requester writes from the same number on the
  venue's WhatsApp (the inbox thread proves control of `wa_id`), or quotes a past order id + the phone it
  was placed with. The owner confirms in the console; `due_at` shows on the dashboard; an overdue request
  is a red line on `/api/owner/health`.
- The owner then acts with the existing routes (forget) or P5's (export/restrict).

### 4.5 Access, portability, restriction, objection (P5)

- `GET /api/owner/customers/:key/export` (new): walks the registry and returns **JSON** (machine-readable,
  Art. 20 [18]) and an **HTML page in sq/en/uk** (the human copy for Art. 15 [14]): orders (lines, money,
  times, addresses), bookings, consent history with the wording shown, messages, wallet statement, who
  revealed the number and when (`Revealed`), processors that received it. Audited as a `Revealed`-class
  event (an export is a reveal).
- `restricted` state on the customer card: excluded from campaigns (the `consent` gate already routes all
  sends through consent), hidden from `customers` list reveal, kept for fulfilment. Objection to CRM
  profiling = the same flag.
- **Device**: a "forget me on this device" button in the storefront/kit profile clearing every `dw_*` key
  the registry lists as `DeviceOnly`.

### 4.6 Retention (P6)

One nightly pass per venue (the existing 03:17 cron, `cloud.rs:29`), driven by the registry's `retention`:

| Data | Proposed | Mechanism |
|---|---|---|
| Order contact/address (the order itself stays for accounting) | redact **24 months** after the order ended (lawyer to confirm; §6) | `Hub::redact` with an age predicate + a `Forgotten{reason:"retention"}` declaration per batch |
| Bookings | 12 months after the slot | redaction as above |
| Inbox / threads | 12 months after last message | rebuild |
| Courier positions | as today (latest fix only) | — |
| Audit | 7 days (as today) | — |
| Waitlist (PLATFORM) | 24 months or on request | remove |
| Revoked sessions / keys | on expiry | exists |

The declaration keeps law P2's count honest: retention tombstones are declared exactly like erasure ones.

### 4.7 Staff, couriers, owners, prospects (P7)

`POST /api/owner/couriers/:id/forget`, `…/staff/:id/forget` (owner), `POST /api/platform/users/:id/forget`
and `/api/platform/waitlist/forget` (operator): remove the `user`/`courier`/`invite`/`roster` records and
sessions; **keep** till and fiscal events (legal obligation) with the person's id, which then points at
nothing; courier earnings stay as accounting. Export mirrors P5 for these subjects.

### 4.8 The notice, the processor list, the ROPA, the breach runbook, the DPIA (P8-P13)

- **P8 Notice** at `https://<slug>.dowiz.org/privacy`, rendered from the registry + venue settings
  (controller name, address, contact, NUIS, which processors are ON — Telegram, WhatsApp, S3, Stripe, AI),
  in sq/en/uk; linked from checkout, booking and consent prompts; replaces `kit/screens/privacy.js`.
  Includes the backup window (22 days), the Commissioner's complaint route (Law 124 Art. 86), the 30-day
  answer, the 16-year age line for marketing consent. **Text reviewed by a lawyer before it ships.**
- **P9 Processors**: `docs/privacy/PROCESSORS.md` (new) + a platform page; the dowiz↔venue DPA (Art. 28
  [26]) as a click-accept at hub creation with its version stored on the `loc` record.
- **P10 ROPA**: `GET /api/owner/privacy/ropa` and `GET /api/platform/privacy/ropa`, rendered from the
  registry (Art. 30 [27]).
- **P11 Minimisation**: strip `contact` from assist facts (courier keeps address line only for its own
  run); Telegram ticket prints the phone only for delivery orders; the AI endpoint is listed as a processor
  in the notice when set.
- **P12 Breach**: `docs/runbooks/BREACH.md` (new): who decides, the clock starts at a recorded
  `incident.aware_at` (a platform record, so the 72 h is a number), the Commissioner's form, the
  processor→controller "immediately" duty (Art. 29(2)) as a template to every affected venue, subject
  notice (binding from ≈ Jan 2027), and the concrete detection sources already in the tree
  (`/api/platform/errors`, witness contradictions, `conservation.mjs` breaches, `worker_errors`).
- **P13 DPIA screening** for courier live location and for the CRM (a document with the EDPB criteria
  answered); a full DPIA if two or more criteria hit.

---

## 5. MCP: dowiz for Claude, Codex and OpenCode, every role

### 5.1 What exists (and keeps)

`workers/api/src/mcp.rs` is the right kernel: a tool is **one call to an existing route, dispatched
in-process** (`crate::route`, `mcp.rs:169-230`), because a Worker fetching its own hostname answers 522
(`mcp.rs:165-168`). So "what an agent may do" is decided in one place — the routes. Everything below keeps
that property and adds: identity for every role (OAuth), a catalogue that covers every route, protocol
versions current clients speak, and the safety layer (scopes, confirmations, audit, rate limits).

### 5.2 The protocol surface (P15)

- **Endpoint**: `https://dowiz.org/mcp` — one universal URL for signed-in roles (owner, staff, courier,
  platform operator). Venue comes from the **grant**, never from a parameter. `/api/mcp` stays as the
  legacy owner-API-key door until the console no longer shows it, then answers 410.
- **Guest endpoint**: `https://<slug>.dowiz.org/mcp` — authless, venue from `Host` (the rule courier
  login already follows, memory `courier-login-venue-from-host`), public tools only (§5.4).
- **Transport**: Streamable HTTP, POST → one JSON response (no SSE needed: nothing streams); 401 with
  `WWW-Authenticate: Bearer resource_metadata="https://dowiz.org/.well-known/oauth-protected-resource/mcp",
  scope="…"` — Claude does not honour the header on a 200, and a tool error is not a 401 (Claude connector
  docs).
- **Two protocol generations at once**:
  - `2025-06-18` / `2025-11-25`: `initialize` → `protocolVersion` negotiated; `notifications/initialized`;
    `ping`; elicitation for confirmations where the client declares it.
  - `2026-07-28`: `server/discover`; version and client capabilities read from `_meta`
    (`io.modelcontextprotocol/protocolVersion`); `resultType: "complete" | "input_required"` on every
    result; `Mcp-Method`/`Mcp-Name` header checked against the body (`HeaderMismatch` −32020);
    `ttlMs`/`cacheScope: "private"` on list results; deterministic `tools/list` order.
  - Stateless in both: the grant carries everything; no server session.
- **Tool annotations** on every tool: `readOnlyHint`, `destructiveHint`, `idempotentHint`,
  `openWorldHint` (inbox reply, campaign send, courier assign are open-world: a person is told).

### 5.3 OAuth 2.1 inside the Worker (P14)

The Worker is both the resource server and the authorisation server (one issuer: `https://dowiz.org`).
The Worker is Rust, so Cloudflare's `workers-oauth-provider` (TypeScript) is the reference, not the
dependency.

| Endpoint | Standard | Notes |
|---|---|---|
| `/.well-known/oauth-protected-resource/mcp` | RFC 9728 | `resource: "https://dowiz.org/mcp"` exactly as users type it; `authorization_servers: ["https://dowiz.org"]`; `scopes_supported` = the minimal read set |
| `/.well-known/oauth-authorization-server` | RFC 8414 | `code_challenge_methods_supported: ["S256"]`, `client_id_metadata_document_supported: true`, `token_endpoint_auth_methods_supported: ["none", …]` (Claude chooses CIMD only when both are present), `authorization_response_iss_parameter_supported: true` (RFC 9207), `registration_endpoint` (DCR, deprecated but Claude/Codex/OpenCode still fall back to it) |
| `/oauth/authorize` | OAuth 2.1 + PKCE | reuses the existing login forms (owner/staff email+password, courier phone+password); then a **consent screen** naming the client (from its CIMD), the **redirect host** (loopback warning when only loopback), the role, the venue (a picker when the user has several), and the scopes as sentences |
| `/oauth/token` | form-urlencoded (Claude requires it) | PKCE verify, `resource` must equal the canonical URI (RFC 8707), refresh-token **rotation** for public clients, `invalid_grant` on a dead refresh token |
| `/oauth/register` | RFC 7591 | compatibility only; rate-limited; clients expire unused after 30 days |
| CIMD fetch | draft-ietf-oauth-client-id-metadata-document | HTTPS only, no private IPs, size and time bounded, cached; accept `http://localhost/callback` and `http://127.0.0.1/callback` **port-agnostic** (Claude Code's CIMD declares both) |

**Tokens are grants, not JWTs handed out**: the access token is 32 random bytes; the platform `identity`
image stores `sha256(token) → grant { user, role, venue, caps (staff), scopes, client_id, aud, exp,
refresh_family }` (new kind `mcpgrant`). Access tokens live 1 h, refresh 30 days rotating. At each call
the Worker resolves the grant, builds the **same `Principal`** the HTTP door builds (`auth.rs:360-373`) —
`Owner`, `Staff{caps}` (narrowed by the live roster via `staff_caps`, `auth.rs:378`), `Courier`, or the
platform `admin` check (`platform.rs:85-110`) — mints the inner five-minute token of that kind, and
dispatches in-process exactly as `mcp.rs` does today. **No new privilege path**: a tool that the role's
route refuses is refused with the route's own 403/404. Token passthrough is impossible by construction
(the inner token never leaves the process; the MCP token is never sent anywhere).

**Revocation**: the console lists grants (client, role, scopes, last used) and revokes one; removing a
staff member from the roster already narrows their caps at the door; the courier's session rule applies
to courier grants.

**API keys** (`dowiz_…`) get scopes and a venue binding in the same change, and stop being accepted on
`restore`, `forget`, `apikeys`, `ebills/config`, `fiscal/ebills` (the admin scope) unless the key carries
it.

### 5.4 The tool catalogue, by role (P16)

Generated from ONE table (`workers/api/src/mcp/catalogue.rs`, new) whose rows are `(tool, route, verb,
roles, scope, danger, schema, annotations)`; a gate (`tools/gates/mcp-coverage.sh`, new) fails when a
route in `lib.rs:250-457` has neither a row nor an explicit `NotAnAgentTool(reason)` (webhooks, print
polling, media bytes, `/api/live` sockets). Role = who may SEE the tool in `tools/list`; the route still
decides.

**Guest (authless, per-venue host)** — scope none:
`menu` (`/api/public/locations/:slug/menu`), `reach` (`/api/public/reach`), `rates`, `eta_quote`
(`…/eta`), `promo_check`, `consent_wordings`, `tables_available` (`…/tables`), `place_order`
(`POST …/orders`, **Idempotency-Key required**; returns a server-minted **order handle** = the customer
token), `order_status(handle)` (`/api/order/:id`), `order_feedback(handle)`, `sitting_bill(handle)`,
`stamps(handle)`, `book_table` (`POST …/reservations`), `booking(handle)`, `booking_action(handle)`,
`booking_pass(handle)`, `wallet_balance/statement/top_up(handle)`, `thread_read/send(handle)`,
`privacy_request` (P4). *A handle is a tool argument, which is how the 2026-07-28 spec says cross-call
state travels.*

**Waiter** (caps `take_orders`, `take_payment`): `room` (`/api/staff/room`), `floor`, `amend_round`,
`guest_round_confirm`, `pay` (money), `transfer`, `move_sitting`, `sitting_cleared`, `aggregator_enter`,
`tips`.
**Kitchen** (`advance`): `room`, `kitchen_ack`, `order_action` (confirm/preparing/ready).
**Counter-manager** (`void`, `open_till`): `refund` / `returned` (**dangerous**), `till_open/count/close/
pay_in/pay_out` (**close is dangerous**: a Z report).
**Courier**: `tasks`, `shift`, `accept`, `pickup`, `deliver`, `refused`, `position`, `earnings`, `history`,
`courier_assist`.
**Owner** — everything the console does, grouped by scope:
- `read`: `dashboard`, `orders`, `analytics`, `exceptions`, `health`, `history`, `activation`,
  `integrations`, `graph`, `stock`, `waste`, `products`, `categories`, `promotions`, `campaigns`,
  `campaign_report`, `couriers`, `courier_detail`, `staff`, `reservations_day`, `floorplan`, `tables_qr`,
  `print_jobs`, `wallet_legs`, `ebills_status`, `fiscal_status`, `receipt`, `features`, `settings`,
  `branding`, `backup_status`, `apikeys`.
- `orders`: `order_action`, `assign_courier`, `reservation_action`.
- `menu`: `set_dish`, `create_product`, `delete_product` (**dangerous**), `set_category`,
  `delete_category` (**dangerous**), `translations`, `menu_import`, `supplies_import`, `recipes_import`,
  `set_supply`, `retire_supply`, `stock_move`, `product_image`, `clear_product_image`.
- `people`: `customers`, `reveal_customer` (audited), `customer_record`, `consent_act`, `link`/`unlink`,
  `rekey`, `export_customer` (P5), `forget_customer` (**dangerous**), `reveals`, `inbox`, `inbox_thread`,
  `inbox_reply`.
- `marketing`: `create_promotion`, `delete_promotion`, `campaign_define`, `campaign_preview`,
  `campaign_send` (**dangerous**: messages people), `posts`, `post_draft/approve/reject`.
- `admin`: `venue_state`, `zones`, `floorplan_set`, `place`, `logo`, `branding_set`, `feature_set`,
  `setting_set`, `notify_test`, `integrations_check`, `staff_invite/set`, `courier_invite/uninvite/active`,
  `ebills_config`, `ebills_map`, `fiscal_ebills_arm` (**dangerous**: typed phrase
  `SEND FISCAL INVOICES TO EBILLS`, `fiscal/ebills_arm.rs:30`), `rotate_log`, `backup_to_cloud`,
  `restore` (**dangerous**), `apikey_create/revoke`, `wallet_legs_repair` (**dangerous**),
  `owner_assist`.
**Platform operator** (`platform` scope, `admin` record required): `hubs`, `create_hub`, `errors`,
`waitlist`, plus P7's `forget_user`/`forget_waitlist` (**dangerous**). The operator does **not** get a
venue's owner tools unless the operator is that venue's owner — the HTTP routes grant nothing more, and
neither may MCP.

### 5.5 Resources and prompts (P17)

Resources (read-only, `cacheScope: "private"`): `dowiz://venue/menu`, `dowiz://venue/orders/live`,
`dowiz://venue/order/{id}`, `dowiz://venue/floor`, `dowiz://venue/stock`, `dowiz://venue/health`,
`dowiz://venue/privacy/ropa`, `dowiz://courier/tasks`. Prompts: `morning_check` (health + exceptions +
low stock), `close_the_day` (till count → Z report → backup), `eighty_six` (take a dish off everywhere),
`privacy_request` (walk a DSR from intake to export/forget with the 30-day clock), `courier_next_run`.

### 5.6 Safety (P18)

- **Scopes**: `read`, `orders`, `menu`, `room`, `till`, `money`, `people`, `marketing`, `admin`,
  `platform`. `scopes_supported` advertises `read` only; everything else arrives by step-up (403
  `insufficient_scope` with the full scope set for the operation in one challenge, per spec).
- **Dangerous actions are two-phase**, independent of client support: `prepare_<tool>` returns a summary
  ("Refund 1,500 ALL on order ab12… to card ending 42") and a **server-minted confirmation handle** bound
  to `(grant, tool, sha256(args))`, valid 5 min, single use; `<tool>(handle, confirm_phrase)` executes.
  Where the client speaks 2026-07-28 MRTR or 2025-11-25 elicitation, the server additionally asks
  in-band; the handle is the floor that works in all three CLIs. `destructiveHint: true` makes Claude,
  Codex and OpenCode ask the human before the call.
- **Idempotency**: every write tool takes `idempotency_key` (the schema marks it required for the three
  queueable routes the `idempotent` gate counts) and forwards it as `Idempotency-Key`; absent, the server
  derives it from `(grant, JSON-RPC id)` so a client retry of the same request cannot double-execute.
- **Audit**: every tool call that writes appends `{ via:"mcp", grant, client_id, role, user, tool,
  args_sha256, outcome, at }` to a new `agent` kind in the venue's `audit` image with **90-day** retention
  (the error log's 7 days is too short for "who did this"). Routes already record the actor as the user;
  the audit adds *through which agent*.
- **Rate limits** per grant (GCRA, `BLUEPRINT-ITEM-08-gcra-swap-2026-07-19.md`): 120 reads/min, 20
  writes/min, 3 dangerous/10 min; per IP for the guest endpoint (`place_order` 5/min).
- **Output hygiene**: phone numbers come back masked (`dowiz_hub::redact`) except through
  `reveal_customer`/`export_customer`, which are audited; tool results never include tokens.

### 5.7 Per-client setup

**Claude Code** (CLI, native OAuth, CIMD):
```bash
claude mcp add --transport http --scope user dowiz https://dowiz.org/mcp
# in a session: /mcp → dowiz → Authenticate (browser: log in, pick role + venue, approve scopes)
# guest (no auth, one venue):
claude mcp add --transport http sushi-durres https://sushi-durres.dowiz.org/mcp
```
**Claude.ai / Claude Desktop / mobile**: Settings → Connectors → *Add custom connector* → URL
`https://dowiz.org/mcp` → Connect. The Worker must accept redirect URI
`https://claude.ai/api/mcp/auth_callback` (and `https://claude.com/api/mcp/auth_callback`), answer
discovery and token calls in < 10 s, and be reachable from `160.79.104.0/21`.

**OpenAI Codex** (`~/.codex/config.toml`):
```toml
[mcp_servers.dowiz]
url = "https://dowiz.org/mcp"          # streamable HTTP; then: codex mcp login dowiz
tool_timeout_sec = 60

[mcp_servers.dowiz-guest]
url = "https://sushi-durres.dowiz.org/mcp"
```
**OpenCode** (`opencode.json`):
```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "dowiz": { "type": "remote", "url": "https://dowiz.org/mcp" },
    "dowiz-guest": { "type": "remote", "url": "https://sushi-durres.dowiz.org/mcp", "oauth": false }
  }
}
```
then `opencode mcp auth dowiz` (tokens land in `~/.local/share/opencode/mcp-auth.json`).

**Headless (CI and the e2e below)**: the operator mints a **QA grant** in the console (a normal grant,
same AS, bound to `qa-durres`, one role, 7-day expiry) and each client sends it as a header:
Claude Code `--mcp-config e2e/mcp/claude.json` with `"headers": {"Authorization": "Bearer ${DOWIZ_QA_TOKEN}"}`;
Codex `bearer_token_env_var = "DOWIZ_QA_TOKEN"`; OpenCode `"headers": {"Authorization": "Bearer
{env:DOWIZ_QA_TOKEN}"}, "oauth": false`.

### 5.8 End-to-end, one scripted session per client (P19)

`e2e/mcp/{claude,codex,opencode}.sh` (new), against `qa-durres.dowiz.org` only (refuses any other host,
the F20 rule), each running non-interactively (`claude -p … --allowedTools mcp__dowiz_*`,
`codex exec …`, `opencode run …`) through the same story, with **every step read back over HTTP** (the F6
rule — a model saying "done" is not a check):

1. guest grant: `menu` → `place_order` (TEST order, idempotency key) → read back `/api/order/:id` = PENDING;
   the same key again → same order (`existing: true`).
2. kitchen grant: `order_action confirm/preparing/ready` → read back status; a waiter grant calling
   `order_action` → the route's 403 (no new privilege path).
3. courier grant: `accept` → `pickup` → `deliver` → read back DELIVERED.
4. owner grant: `refund` without a handle → refused; `prepare_refund` → handle → `refund(handle)` → read
   back the refund leg; the audit image has one `via:"mcp"` record with the grant and client id.
5. owner grant with `read` only → `set_dish` answers 403 `insufficient_scope` listing `menu`.
6. teardown in `finally`: every TEST order closed; `conservation.mjs` on `qa-durres` holds.

Plus `e2e/mcp/oauth.sh`: curl-only discovery conformance — 401 carries `resource_metadata`; PRM
`resource` equals the URL; AS metadata carries S256, CIMD flag, `none`, `iss` flag; a token minted for
another `resource` is refused; a refresh token used twice kills the family.

---

## 6. What needs a lawyer (do not ship P8/P9 text without these answers)

1. **Where dowiz is established**, hence whether GDPR applies directly (Art. 3(1)) and whether Law 124
   Art. 25 requires an Albanian representative.
2. **The controller/processor split** and the dowiz↔venue DPA text (Art. 28 / Law 124 Art. 26), including
   sub-processor approval for Cloudflare, Meta, Stripe, and the venue-chosen S3 and AI endpoint.
3. **Retention periods** where Albanian accounting and tax law override erasure: how long order money,
   fiscal documents and till records must be kept, and whether the customer's name/phone on an order is
   part of that record (this document assumes it is not — the money stays, the person goes).
4. **Transfers**: Cloudflare/Meta/Stripe (US) under Commissioner Decision no. 1 (EU adequacy decisions →
   the EU-US Data Privacy Framework?) or SCCs needing Commissioner approval (Law 124 Art. 41); Telegram,
   which offers no DPA — is a kitchen chat on Telegram lawful at all, or only with minimised content?
5. **Courier location**: basis (employment vs contractor), DPIA need (Art. 31 from Jan 2027), worker
   information duties.
6. **Wallet balance on erasure**: refund first, or keep the pseudonymous account claimable.
7. **Marketing**: Art. 46 + Albania's electronic-communications rules for WhatsApp campaigns; the 16-year
   line for consent.
8. **DPO**: whether any venue or dowiz crosses Art. 33's "large scale".
9. The **privacy notice** and the **breach runbook** wording in three languages.

## 7. Recommended against

- **Self-service erasure by the diner** — the only diner credential is one order's token; P4's intake +
  owner action is the answer (CRM §4).
- **Crypto-shredding now** — §4.3; re-entry when a copy exists that the venue cannot rewrite within 30 days.
- **Rebuilding the order log without the subject** — breaks the witness by construction (CRM §2.2 row A).
- **A separate MCP privilege model** — any tool that does something its route would not is a second
  authorisation path; the catalogue gate and the in-process dispatch are there to make that impossible.
- **Handing agents the owner's API key as the MCP credential** — it is a year-long all-powerful secret
  pasted into config files; OAuth grants are per person, per role, per venue, scoped and revocable.

## 8. What was not verified

- The live venues' Durable Object locations/jurisdiction (not queried: no production logins).
- Whether any service worker caches an `/api/` response (P1 greps it).
- `threads` retention; `idem` sweep age; the exact 2026-07-28 support level of each client today
  (Claude Code, Codex, OpenCode versions change weekly — P19 is the measurement).
- Whether Claude Code's `--mcp-config` expands `${VAR}` in headers in `-p` mode (P19 proves it or uses a
  generated file).

## Sources

- Law No. 124/2024, official English text: https://idp.al/wp-content/uploads/2025/04/Law-no.124-2024-DP.pdf
  (Art. 7(6), 12(4), 15(2), 25, 26, 27, 29, 31, 33, 39-42, 94, 101 read directly)
- CMS expert guide, Albania: https://cms.law/en/int/expert-guides/cms-expert-guide-to-data-protection-and-cyber-security-laws/albania
- IAPP, "Albania's personal data protection law": https://iapp.org/news/a/albania-s-personal-data-protection-law-a-legal-framework-harmonized-with-the-gdpr
- Karanovic & Partners, Commissioner Decision no. 1 (30 Apr 2025) on adequate countries: https://www.karanovicpartners.com/news/albania-aligns-with-gdpr-new-decision-on-adequate-countries-for-data-transfers/
- Clym, dates of adoption and entry into force: https://www.clym.io/regulations/law-no-1242024-on-personal-data-protection-albania
- GDPR, Regulation (EU) 2016/679: https://eur-lex.europa.eu/eli/reg/2016/679/oj
- ICO on erasure and backups (as cited in the CRM blueprint §2.2): https://marketinglaw.osborneclarke.com/data-and-privacy/new-ico-guidance-on-deleting-personal-data/
- Cloudflare DO storage and 30-day PITR: https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/
- Cloudflare DO data location (jurisdictions): https://developers.cloudflare.com/durable-objects/reference/data-location/
- Cloudflare Customer DPA: https://www.cloudflare.com/cloudflare-customer-dpa/
- WhatsApp Business data processing terms: https://www.whatsapp.com/legal/business-data-processing-terms
- Stripe DPA: https://stripe.com/legal/dpa
- MCP authorization (2026-07-28): https://modelcontextprotocol.io/specification/latest/basic/authorization
- MCP changelog 2026-07-28: https://modelcontextprotocol.io/specification/2026-07-28/changelog
- Claude connectors — authentication (CIMD/DCR, callback URLs, form-urlencoded token, 10 s limit, egress range): https://claude.com/docs/connectors/building/authentication
- Claude custom connectors: https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp
- Codex MCP config (`url`, `bearer_token_env_var`, `http_headers`, `codex mcp login`, OAuth callback settings): https://learn.chatgpt.com/docs/extend/mcp?surface=cli
- OpenCode MCP servers (`type: "remote"`, `oauth`, `opencode mcp auth`): https://opencode.ai/docs/mcp-servers/
- Cloudflare `workers-oauth-provider` (reference implementation, TypeScript): https://github.com/cloudflare/workers-oauth-provider
