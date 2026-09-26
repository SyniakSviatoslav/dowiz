# Telegram bots in groups with configurable routing, and Russian as the 4th language — research and plan

Date 2026-09-26. Read-only research; nothing below was built. Every file:line was read in the working tree at this date
(HEAD `git show` used where the tree is mid-split). Items marked **unverified** were not proved by a command or a
primary source. Sibling documents: `2026-09-26-menu-ingredients-stock.md` (stock events, lots, analytics — its event
names are used here as notification sources) and `2026-09-26-kitchen-role.md` (KDS; not present at the time of writing).

---

## Part A — Telegram groups with configurable routing

### A1. Current state (verified)

**The bot is the venue's.** `crates/dowiz-hub/src/settings/known.rs:68-92` defines `notify.telegram.token` (redacted
everywhere by shape, `settings.rs:42`), `notify.telegram.chat` ("your own user id, or a group's id. Send the bot any
message first, then use the test button") and `notify.telegram.chat.bar`. `social.telegram.channel` (`:47`) is a
public channel for AI-drafted posts. `notify::bot_token` (`workers/api/src/notify.rs:55-61`) takes the venue token,
else the platform secret `TELEGRAM_BOT_TOKEN` (an env secret; `storefront.rs:579-581` also exposes
`TELEGRAM_BOT_USERNAME` to the storefront as `telegramBot`). Activation counts `telegram_chats` = token AND chat set
(`services/venue/activation.rs:101-107`).

**How a chat id is obtained today:** by hand. The owner pastes a numeric id into the console
(`public/admin/more.js:347-375`, "notifications" pane), and `POST /api/owner/notify/test` (`notify.rs:214-257`) sends
one line and returns Telegram's own description on failure ("chat not found" = the owner never wrote to the bot). There
is **no inbound Telegram handling at all**: `grep -rn setWebhook|callback_query|getUpdates|my_chat_member
workers/api/src crates` = 0 hits. `docs/notifications/telegram-onboarding.md` describes a `/start <connect_token>`
deep-link flow with a `telegram_connect_tokens` table — that is the deleted Postgres/TS system, never ported (the
No-SQL migration of 2026-09-22 deleted D1; see memory). `docs/adr/ADR-TELEGRAM-NOTIFICATIONS-ACTIONS.md` (2026-06-22,
PROPOSED) is likewise from the old stack (pg-boss, RLS); its transactional/operational/quality taxonomy and its
quiet-hours ethics note are still worth reusing (see A4), the rest is dead.

**What is sent, and where** (all `kind = "telegram"`, all plain text, `parse_mode` never set — `notify.rs:65-90`
deliberately avoids Markdown escaping; text cut at 4096 chars):

| event | rendered by | queued/sent | to |
|---|---|---|---|
| order placed | `notify::order_text` (`notify.rs:94-158`: venue, #id8, name-initial, phone only for delivery, kind/table, address, note, lines, totals, payment) | queued in the DO turn that writes the order: `hubdo.rs:709` → `enqueue_bell` `:951-1023` | `notify.telegram.chat`; bar lines to `.chat.bar` when set (`bell_route.rs:150-195`, ids `{order}/telegram[/bar]`) |
| amendment adds lines | `bell_route::ticket_text` (no prices) | same path, id `{order}/amend/{seq}/telegram` | same |
| exception count crosses threshold (voids, comps, refunds, pay-outs, till over/short, wallet legs) | `exceptions/alert.rs:52-75` (`due`), words in sq/en/uk `:86-108`; `exceptions/legs.rs:133` | queued from the DO turn | `notify.telegram.chat` |
| WhatsApp/Instagram inbound message relay | `channels.rs:431-437` | **sent inline** from the Meta webhook request, not via the outbox (loses the message on a Telegram hiccup — the exact defect `outbox.rs` was written to close) | `notify.telegram.chat` |
| AI post approved | `services/engagement/verdict.rs:50-70` | sent inline (owner is waiting for the verdict) | `social.telegram.channel` |
| notifications test / integrations check | `notify.rs:214`, `integrations.rs:89` (`getMe`) | inline | chat |

**Not sent today:** status changes (confirmed/ready/handed/delivered/cancelled), bookings, stock events, waste,
stocktake variance, low stock, digests, abandoned-outbox notices (those go to the venue error log via `loud!`,
`errlog.rs:118-124`, visible in the console only).

**The outbox** (`workers/api/src/outbox.rs`, `outbox/rails.rs`): image `outbox`, kind `o`, `Entry{id, kind, text, to,
queued_at_ms, tries, next_at_ms}` (`outbox.rs:68-100`); rendered at enqueue time on purpose (`:78-81`); 6 tries with
10 s → 10 min backoff (`:38-58`); abandoned entries are `loud!`-ed (`rails.rs:170-176`). The minute cron
(`wrangler.toml:145` `"* * * * *"`, `lib.rs:566-587`) walks the venue registry and drains every venue — one DO read per
venue per minute, bound written down at ~40 venues (`rails.rs:143-150`). `drain` (`rails.rs:33-140`) sends each due
entry with one `fetch` and applies verdicts in one image write; a missing token leaves entries waiting (`:57-63`).
Depth/age are on the owner health pane (`services/operations/mod.rs:112-114`). **Telegram errors are flattened to a
string** (`notify.rs:80-88`): 403 (kicked), 400 (migrated, chat not found) and 429 (`retry_after`) all look the same and
all consume one of the six tries.

**Privacy today:** the outbox store is registered (`privacy/registry/venue.rs:55-59`: Name, Phone, Address,
OrderContent, subjects Customer, purpose Kitchen, basis Contract) and Telegram is a venue processor
(`privacy/registry/outside.rs:27-32`: "none offered by Telegram; the ticket is cut to what the kitchen needs";
host `api.telegram.org` `:88`). `ticket_contact` (`notify.rs:171-186`) minimises to "First L." and phone only for
delivery. The registry does **not** yet know: group titles, the Telegram user id of whoever links a group or presses a
button, or that a group has N members rather than one owner.

**Meta webhook, as the pattern to copy** (`channels.rs:380-419`): venue named by the HOST, unsigned request refused
before any DO read, signature = the only authority, a venue without a secret acknowledges and drops (Meta retries 4xx
for days). Route `/api/webhooks/meta` (`lib.rs:415-416`).

**Languages of the messages:** the order ticket is English with emoji ("delivery", "pickup", "table", "tip");
exception alerts follow the venue's `default_locale` in sq/en/uk with a fallback to English
(`hubdo/exceptions.rs:67`, `alert.rs:86-92`). No per-chat language exists.

### A2. Telegram Bot API facts that shape the design (sources)

- **Rate limits** (verbatim, https://core.telegram.org/bots/faq): "In a single chat, avoid sending more than one message
  per second" (429 eventually); in groups "bots are not able to send more than 20 messages per minute"; broadcasts
  "about 30 messages per second". 429 carries `parameters.retry_after` seconds
  (https://core.telegram.org/bots/api#responseparameters).
- **Privacy mode** (https://core.telegram.org/bots/features#privacy-mode): a bot with privacy mode ON receives only
  "Commands explicitly meant for them (e.g., /command@this_bot)", "General commands (e.g. /start) if the bot was the
  last bot to send a message to the group", replies to its own messages, and "all service messages"; bot administrators
  get all messages; turning privacy mode off requires re-adding the bot. Consequence: keep privacy mode ON — a
  `/link@bot CODE` command and inline buttons work, and no kitchen chatter ever reaches the hub (a privacy plus that the
  notice can state).
- **Getting the chat id safely:** `Update.my_chat_member` ("The bot's chat member status was updated in a chat",
  https://core.telegram.org/bots/api#update) arrives when the bot is added/removed — it names the chat and who added
  it, but anyone can add the venue's bot to any group, so `my_chat_member` alone must not link. The `startgroup` deep
  link `https://t.me/<bot>?startgroup=CODE` lets the user pick a group and sends `/start@bot CODE` there; parameter
  ≤ 64 chars of `A-Za-z0-9_-` (features#deep-linking). Design: link = a one-time code minted in the console, redeemed by
  a command in the group; `my_chat_member` only maintains state (left/kicked, promoted).
- **Supergroup migration:** a group upgraded to a supergroup gets a new negative id; sends to the old id fail with 400
  "group chat was upgraded to a supergroup chat" and `parameters.migrate_to_chat_id`; the service message carries
  `migrate_to_chat_id`/`migrate_from_chat_id` (api#message; https://github.com/TelegramBots/telegram.bot/issues/171,
  https://github.com/atipugin/telegram-bot-ruby/issues/239). Store the new id from either signal and retry at once.
- **Forum topics:** supergroups with topics enabled (`Chat.is_forum`) accept `message_thread_id` on `sendMessage`;
  the General topic is thread 1 (**unverified**: not quoted from the reference in this session). One "Restaurant"
  supergroup can then hold Kitchen / Bar / Owners / Couriers topics, each a routing target. The WooCommerce Telegram
  plugin encodes a target as `-1001234567890:42` (chat:thread) —
  https://github.com/catcodestudio/woocommerce-telegram-order-notifications — a compact convention to reuse in
  `Entry.to`.
- **403** "Forbidden: bot was kicked from the supergroup chat" / "bot was blocked by the user" when removed
  (https://github.com/eternnoir/pyTelegramBotAPI/discussions/2205); 400 "chat not found" for a wrong id. These are
  terminal for that chat, not retryable.
- **Inline buttons:** `InlineKeyboardMarkup` under the message; pressing sends `callback_query{id, from, message,
  chat_instance, data}`; `data` ≤ 64 bytes; must be answered with `answerCallbackQuery` (api#callbackquery). The
  reference recommends editing the keyboard (`editMessageReplyMarkup`) after a toggle rather than sending a new message
  (features#inline-keyboards).
- **Webhook security:** `setWebhook(secret_token)` → header `X-Telegram-Bot-Api-Secret-Token` on every delivery,
  1-256 chars; `allowed_updates` to receive only `message`, `my_chat_member`, `callback_query` (api#setwebhook).
- **Formatting:** `parse_mode` HTML or MarkdownV2; text ≤ 4096 chars. HTML with `<b>` and escaped `&<>` is the safer
  choice for dish names (MarkdownV2 needs 18 characters escaped). Today's plain text can stay; bold headers are optional.

**What POS systems do:** Poster's JARVIS bot (https://joinposter.com/en/applications/jarvis-bot): "add JARVIS to your
team's group" and "set up reports or notifications to be sent to this group"; notification types are new orders and
bookings, sales statistics, stock balances with configurable "stock limit", "flexible schedule settings for reports";
the sell is "share the necessary information without providing access to the Poster control panel". Poster's Courier
Bot (https://joinposter.com/en/applications/delivery-bot) sends orders to couriers via Telegram. Onlizer does the same
as a connector (https://onlizer.com/poster_pos/telegram_bot). Syrve/iiko has no first-party bot; the market builds
report bots on its OLAP API per location, daily sales and cancellations to a channel
(https://freelancehunt.com/en/project/integratsiya-zvitiv-syrve-iiko-telegram-botom/1544701.html). The WooCommerce
plugin above is the closest to the matrix the operator wants: per event a chat/topic, per-status lists, low-stock on the
platform's own threshold, custom templates, one retry, a delivery log with Telegram's HTTP status, and a privacy line
"messages may include order and customer details, depending on your template".

### A3. Design

**Principles.** (1) The venue's bot, privacy mode on, no platform bot needed. (2) A group is linked by a code minted in
the console, never by being added. (3) One pure router decides fan-out; the object turn writes the entries; the cron
sends — the outbox stays the one queue. (4) A group carries a language and a personal-data level; the default is
"no customer data". (5) Every Telegram failure is classified, and a dead chat is muted loudly, not retried six times.

**A3.1 Data model** — one settings key holding a JSON array (`Settings::set/get`, the settings image is already read in
`enqueue_bell` `hubdo.rs:959-963`, so routing costs no extra read). No new image.

```
notify.tg.groups = [ {
  id:        "g_kitchen"            // stable slug, chosen at link time
  chat:      -1001234567890         // i64; rewritten on migrate_to_chat_id
  thread:    42 | null              // forum topic; null = whole chat
  title:     "Dubin Kitchen"        // from the update's chat.title; display only
  kind:      "group"|"supergroup"|"private"|"channel"
  lang:      "sq"|"en"|"uk"|"ru"    // message language for this group (Part B)
  pii:       "none"|"fulfil"|"full" // none = items/table/kind only; fulfil = + address, phone, note (couriers); full = today's ticket
  subs:      { "order.placed":"now", "stock.low":"digest", "digest.daily":"now", ... }   // absent = off
  quiet:     { from: 1380, to: 420 } | null    // venue-local minutes; holds non-urgent events until `to`
  digest:    { daily_at: 540, weekly: { dow: 1, at: 540 } } | null
  state:     "active"|"left"|"muted"          // left = 403/kicked; muted = owner paused it
  linked:    { at_ms, by: "<staff principal>", tg_user: 12345678 }   // tg_user = personal data (registry)
  health:    { last_ok_ms, fails, last_error }
} ]
notify.tg.link   = { code, created_ms, by }   // one pending code, 10 min TTL, single use
notify.tg.webhook_secret = "<32 random chars>"  // set by setWebhook; key ends in `secret` → redacted
```

Legacy `notify.telegram.chat` / `.chat.bar` are **derived** into two implicit groups (kitchen: `order.placed`,
`order.amended`, `exceptions.*`, `inbox.message`, pii `full`; bar: station bar) by the router when `notify.tg.groups`
is absent — no migration write, nothing changes for the two live venues until an owner opens the new pane.

**A3.2 Event catalogue** (event key → source turn → default group → urgency class per the old ADR's taxonomy):

| key | source (file) | payload rendered | class |
|---|---|---|---|
| `order.placed`, `order.amended` | `hubdo.rs:709`, `enqueue_bell` | ticket per station | transactional (breaks quiet hours) |
| `order.status.{confirmed,preparing,ready,handed,delivered,cancelled}` | the status command in `hubdo`/`command/` (NEW hook, same turn) | "#id8 → READY" | transactional |
| `order.late` | dwell rule (none exists in the Worker — NEW; the old ADR's `order.dwell_escalation`) | pending > N min | transactional |
| `order.refund`, `exceptions.threshold`, `legs.*` | `exceptions/alert.rs`, `legs.rs` (re-routed) | as today | operational |
| `booking.new` | `booking.rs` (NEW hook) | date, covers, name-initial | operational |
| `stock.received` (partія/lot arrives), `stock.wasted`, `stock.stocktake.variance` | the stock append turn (`hubdo.rs:614-660`; events `Received/Wasted/Stocktake` per the sibling doc §1.3) | item, qty, lot, expiry | operational |
| `stock.low` | fold at append: on_hand crosses below supply `lowAt` (sibling §1.1, `supplies.rs:155`) — emit once per crossing (like `alert.rs` "once per crossing") | list of items under threshold | operational |
| `stock.expiry` | needs lots + `expiry` (sibling §3.3/§3.5) — nightly cron in venue tz | lots expiring in ≤ N days | digest-only |
| `stock.86` | a placement refused for stock (sibling §3.3 `refused`) | dish, wanted/available | transactional (kitchen) |
| `inbox.message` | `channels.rs:431` (moved from inline to the outbox) | channel, peer, text | operational |
| `system.outbox_abandoned`, `system.integration` | `rails.rs:170-176`, `errlog::record` | what failed | operational (owners) |
| `digest.daily`, `digest.weekly` | cron, venue-local time | orders, revenue, top dishes, waste value, low-stock list, variance (sibling §4 analytics catalogue) | scheduled |

**A3.3 Flow.** Hub event (DO turn) → `notify_route::fan_out(event, &groups, now_ms) -> Vec<Entry>` (pure, new
module beside `bell_route.rs`) → written into the outbox in the same turn (as `enqueue_bell` does today) → minute cron
`drain` sends. `Entry.to` = `"<chat>"` or `"<chat>:<thread>"`; `Entry.id` = `{event}/{key}/{group}` so a retried
command overwrites its own entry. Rendering at enqueue in the group's `lang` and `pii` level (one text per group, not per
event — 3 groups = 3 entries). Quiet hours: `fan_out` sets `next_at_ms` to the window's end for non-transactional
events (the existing `due()` honours it; nothing new in the drain). Digest: at enqueue nothing is rendered; the router
writes/refreshes one tiny marker record in the outbox image (`kind "d"`, `{group, next_ms}`), so the minute sweep — which
already reads every venue's outbox image — sees "a digest is due" with no extra read; on due, the drain asks the DO to
fold the day (a DO turn, `now_ms` bounded to that venue's day in `dowiz_hub::tz`) and enqueues one entry per digest
group. **CPU risk:** a day's fold in the Worker's 10 ms on the Free plan (memory: 1.4 % of requests already hit the
kill) — the fold must run in the DO (its own limit, **unverified** on Free) and reuse the snapshot the sibling doc
proposes (§3.7), not re-walk the log.

**A3.4 Rails changes.** (a) `notify::telegram` returns a structured error `{status, description, migrate_to,
retry_after}`; (b) the drain maps: 429 → `next_at_ms = now + retry_after·1000`, no try consumed; 400 migrate →
rewrite the group's `chat`, retry immediately; 403 / 400 "chat not found" → group `state = left`, abandon every entry
for that chat now, `loud!` + one notice to every `owners` group; (c) per-chat pacing: at most 15 sends per chat per
drain (20/min limit), the rest wait a minute; (d) cap sends per cron invocation (Free plan subrequest limit per
invocation, **unverified** — 50 on Free is the commonly cited figure; three groups × a busy lunch could exceed it, so
the cap must exist); (e) `message_thread_id` and optional `reply_markup` on the request.

**A3.5 Linking and security.** Console → "Link a group" mints `code` (10 chars base32, 10 min, single use) and shows:
"Add @bot to the group, then send `/link@bot CODE`" plus the `startgroup=CODE` deep link. The bot's webhook is set by
the hub (`POST /api/owner/telegram/connect` → `setWebhook{url: https://<venue-host>/api/webhooks/telegram,
secret_token, allowed_updates}`); inbound `POST /api/webhooks/telegram` mirrors `channels::webhook`: venue by HOST,
header `X-Telegram-Bot-Api-Secret-Token` compared before any DO read, 200 always after that (Telegram retries non-2xx).
Handled updates: `message` with `/link` or `/start` command → code check → group appended with `linked.tg_user`; `message`
with `migrate_to_chat_id` → rewrite; `my_chat_member` `left|kicked` → `state = left`, `administrator|member` → note;
`callback_query` → A3.6. **Who may link:** whoever holds a code minted by an owner/manager (console role is the
authority); the linker's Telegram user id and name are recorded for the audit and are NEW personal data of staff
(registry `venue.rs` entry, subjects Staff, purpose Operations, retention = the group's life, erase = unlink).
**Caveat:** `setWebhook` replaces any webhook the venue's bot already has; a venue that uses the same bot elsewhere
loses it (operator decision Q1).

**A3.6 Inline actions.** Phase 5 only. `callback_data = "1:<action>:<order8|item>:<nonce6>"` (≤ 64 bytes); actions per
group role: kitchen → `ready`, `86` (mark dish unavailable), `batch.ok` (confirm a received lot); owners → all plus
`cancel`; couriers → none in phase 1. Authorisation = the chat is a linked, active group AND the action is in that
group's allow-list AND (optional, Q3) `from.id` is on the venue roster's Telegram-id map. The action runs through the
existing command path with a principal `telegram:<group>:<from.id>` recorded as the signer (the stock ledger already
requires a signer for `Wasted/Stocktake`, sibling §1.3). Answer with `answerCallbackQuery` and edit the keyboard to
"✓ done by …". A `cancel` needs a confirm step (nonce echo) as the old ADR's BR-15 had.

**A3.7 Privacy.** Default `pii = none` for every newly linked group: the kitchen ticket carries items, quantities,
kind (delivery/pickup/table N), note is dropped unless `fulfil`. `fulfil` (couriers) adds address, phone, note.
`full` is today's ticket and is what the two live venues keep. Digest and stock messages carry no customer data by
construction. Registry additions: group title + chat id (venue data, not personal), linker/presser Telegram id and
first name (Staff), and the `outside.rs:27` Telegram processor description must say "a group whose members the venue
chooses". The notice (P8) sentence on Telegram gains "or a staff group". `tools/gates/personal-data.sh` baseline will
move; that is the intended proof.

**A3.8 Failure handling summary.** Enqueue failure = the order still lands (`hubdo.rs:710` logs). Send failure =
retry with backoff; 429 honoured; dead chat muted in one attempt and reported to owners; abandoned = errlog + owners
group; health pane lists per-group `last_ok_ms/fails/last_error`; the per-group "test" button sends one line to that
chat/thread and shows Telegram's words. Webhook failure = 200 + drop (never 5xx to Telegram); a missing secret means
the venue never connected: acknowledge and drop, not `loud!` (same reasoning as `channels.rs:404-410`).

### A4. Owner UI — text wireframe (More → Notifications → Groups)

```
Notifications                                                     [Test all]
Bot: @dubin_sushi_bot  ✓ connected (webhook set 12:03)            [Reconnect]

Groups                                                            [+ Link a group]
┌ Kitchen  ·  "Dubin Kitchen" (supergroup, topic 42)  ·  lang SQ  ·  data: none  ·  ✓ 09:41  [Test] [Mute] [Unlink]
├ Owners   ·  "Dubin Owners" (group)                  ·  lang UK  ·  data: full  ·  ✓ 09:40  [Test] [Mute] [Unlink]
└ Couriers ·  "Dubin Riders" (group)                  ·  lang SQ  ·  data: fulfil·  ✗ 403 left the group 08:12 — re-link

What goes where                                          Kitchen   Owners   Couriers
 New order                                               ● now     ● now    ○
 Order amended (lines added)                             ● now     ○        ○
 Order status (ready / handed / delivered)               ○         ○        ● now
 Late order (pending > 10 min)                           ● now     ● now    ○
 Cancelled / refund                                      ○         ● now    ○
 New booking                                             ○         ● now    ○
 Delivery received (lot)                                 ● now     ◐ digest ○
 Ingredient below lowAt                                  ● now     ◐ digest ○
 Lot expiry ≤ 2 days                                     ◐ digest  ◐ digest ○
 Waste written                                           ○         ◐ digest ○
 Stocktake variance                                      ○         ● now    ○
 Exceptions over threshold                               ○         ● now    ○
 Customer message (WhatsApp / Instagram)                 ○         ● now    ○
 Integration failures                                    ○         ● now    ○
 Daily digest  09:00                                     ○         ● now    ○
 Weekly digest Mon 09:00                                 ○         ● now    ○
 (● now  ◐ in the digest  ○ off — tap to cycle)

Per group (sheet):  language [SQ EN UK RU]   customer data [none | delivery details | everything]
                    quiet hours [23:00 – 07:00] (orders always ring)   digest [daily 09:00] [weekly Mon]

Link a group (sheet):  1. Add @dubin_sushi_bot to the group.  2. Send there:  /link@dubin_sushi_bot K7Q2-M4XZ
                       or open  t.me/dubin_sushi_bot?startgroup=K7Q2M4XZ    Code expires in 09:41.
```
Storage of the matrix = `subs` per group; the console renders columns from `notify.tg.groups` and rows from a shared
event list exported by the Worker (`GET /api/owner/notify/events`, so the console never hardcodes the catalogue).

### A5. Roadmap rows (Part A)

| # | step | files | proving test | gates | risk | operator decision |
|---|---|---|---|---|---|---|
| A0 | Inbound webhook + link code + `my_chat_member` + migration rewrite; `connect` route calls `setWebhook` | new `workers/api/src/telegram.rs` (+`/tests.rs`, +`/inbound.rs` pure parser), `lib.rs` routes, `settings/known.rs` keys, `admin/more.js` link sheet, `admin/i18n.js` words ×3(4) | pure tests: secret header mismatch → 401 before any read; `/link CODE` once → group appended, twice → refused; migrate service message → chat rewritten. Live: link a real group on dubin-sushi, `getWebhookInfo` shows the URL | personal-data (new Staff fields), file-size, paths, evals i18n | `setWebhook` clobbers the venue bot's existing webhook | Q1 |
| A1 | Groups model + pure router `notify_route::fan_out`; `Entry.to = chat[:thread]`; drain sends `message_thread_id`; structured Telegram errors (429/403/migrate); legacy keys derived | `notify_route.rs` (new, pure), `notify.rs`, `outbox/rails.rs`, `hubdo.rs::enqueue_bell` | pure: two groups, one order → two entries with different `pii` renderings; 403 → state left + entries abandoned; 429 → `next_at_ms` = retry_after; legacy venue with no `notify.tg.groups` → byte-identical entry to today (`bell_route` tests already pin the text) | idem-done, record, one-image | subrequest cap per cron invocation on Free (**unverified**) | — |
| A2 | Event sources: status changes, bookings, stock received/wasted/variance/low (hook in the stock append turn, coordinate with the stock roadmap), inbox relay moved to the outbox, exceptions re-routed | `hubdo.rs`, `command/*status*`, `booking.rs`, `channels.rs:431`, `exceptions/alert.rs` | pure per event: fold → entries; `stock.low` fires once per crossing (mirror `alert.rs` level test); inbox relay survives a Telegram 5xx (entry waits) | event-kinds, no-scoring | `stock.low` needs the fold at append (sibling §1.6 CPU note) | which events by default |
| A3 | Owner matrix UI, per-group language/pii/test/mute/unlink, `GET /owner/notify/events` | `admin/notify-groups.js` (new), `more.js`, `i18n.js`, `services/operations/mod.rs` (health per group) | node tests for the matrix cycle and for "no group → legacy pane unchanged"; design gate on the sheet | ui-adoption, tap-size, evals i18n | 300-line ratchet on `more.js` (already 600+) | default pii for kitchen |
| A4 | Digests + quiet hours; due marker in the outbox image; fold in the DO at venue-local time | `outbox.rs` (kind `d`), `rails.rs::sweep`, `hubdo.rs` digest turn, `notify_route.rs` (quiet), digest renderer in sq/en/uk/ru | pure: quiet window wraps midnight; transactional events ignore it; marker due → exactly one entry per digest group per day (idempotent id `digest/{day}/{group}`); measured CPU of the fold on a 100-order day | venue-clock, clock, idem-done | 10 ms CPU; digest content depends on the stock analytics (sibling §4) | quiet-hours class of "late order" (the old ADR's STOP-ETHICS-1: does a late pending order wake the owner) |
| A5 | Inline actions (`ready`, `86`, `batch.ok`, `cancel` with confirm) with group-role allow-list; `answerCallbackQuery`; keyboard edit | `telegram.rs` (callback branch), `command/*`, `hubdo.rs`, registry | pure: callback from an unlinked chat → refused; action not in the allow-list → refused; nonce replay → refused; signer recorded on the stock event | principal-binds, record, personal-data | authority is the group, not the person, unless a roster map exists | Q3 |
| A6 | Pacing (15/chat/drain), 403 auto-mute with an owners notice, health pane, `parse_mode: HTML` opt-in with escaping | `rails.rs`, `notify.rs`, `operations/mod.rs`, `admin/health` | pure: 40 due entries for one chat → 15 sent, 25 kept; escaping test with `<`, `&`, `"` in a dish name | — | HTML escaping is where "a notification silently stops arriving" (`notify.rs:92-93`) — keep plain text default | — |

Cost on the Free plan: enqueue = 0 extra requests; sends = one subrequest each inside the existing minute cron; digest =
one DO turn per group per day; the webhook = one request per command/button press (rare). Bytes: a group record ≈ 400 B
in the settings image; the marker ≈ 60 B. Nothing here adds a per-venue-per-minute read beyond the existing sweep.

---

## Part B — Russian (`ru`) as the 4th language everywhere

### B1. Inventory of every place the language set lives (verified)

Counts: "keys" = string keys per language (from `grep -o "key: '"` ÷ 3; ±5 %); "words" = words in the file, all
languages and code (÷3 ≈ per language, upper bound).

| area | file:line | what | size |
|---|---|---|---|
| **Language-set literals, JS** | `admin/i18n.js:362`, `courier/i18n.js:10`, `room/i18n.js:143`, `store/i18n.js:259`, `wiki/wiki-core.js:8`, `lib/learn.js:30`, `platform/landing.js:252-253,504` (T-keyed), `admin/campaigns.js:31` (`TPL_LANGS` incl. `en_US` for WhatsApp templates) | `LANGS = ['sq','en','uk']` | 8 copies |
| **Language-set literals, Rust** | `workers/api/src/learn.rs:28`, `privacy/notice/words.rs:82`, `privacy/dpa.rs:112`, `mcp/tools.rs:46` (JSON schema enum), `waitlist.rs:27` (`["uk","en","sq"]`), `crates/dowiz-hub/src/consent/wordings.rs:23`, `exceptions/alert.rs:86-108` (match arms), `services/customers/forget.rs:217`, `services/campaigns/send.rs:57`, `services/engagement/voice/say.rs:10-20` (`[&str; 3]` LINES), `voice/words.rs:47`, `voice/grammar.rs:4`, `crates/dowiz-hub/src/voice.rs:25,85-87` (grammar verbs uk/sq/en), `crates/dowiz-hub/src/post.rs:376-380` (language name for the AI prompt), `hubdo/exceptions.rs:67` (default "en"), `storefront.rs:687`, `posts.rs:84`, `notice.rs:276` (default "sq") | match arms / const arrays | 20 sites |
| **Tools & gates literals** | `tools/learn/check.mjs:15`, `assemble-plan.mjs:16`, `publish.mjs:37`, `build-lessons.mjs:26,42-49`, `all-plan.mjs:24`, `tools/evals/collect/i18n.mjs:9` (rule: 0 missing keys for sq/en/uk), `tools/gates/learn.sh:19` (item 4 via `build-lessons --check`) | | 7 |
| **Tests asserting three** | `learn/lessons.test.mjs:158`, `lib/mcp-words.test.mjs:7`, `admin/zones.test.mjs:17,75-79,116`, `admin/voice.test.mjs:12,54-56`, `room/voice.test.mjs:11,94-96`, `courier/learn.test.mjs:54-99`, `courier/screens.test.mjs:17`, `tools/learn/capture.test.mjs:12`, `assemble.test.mjs:39`, `forget/tests.rs:252`, `dpa/tests.rs:38`, `voice/say/tests.rs` (7 hits), `voice/grammar/tests.rs:105` | `deepEqual(LANGS, [...])` etc. | 13 files |
| **UI dictionaries (owner console)** | `admin/i18n.js` (680 keys, 6 993 w), `customers.js` (69), `campaigns.js` (57), `bookings.js` (43), `exceptions.js` (41), `floorplan.js` (37), `zones.js` (25), `printer.js` (18), `refund.js` (16), `tableqr.js` (10) | `T = { sq:{}, en:{}, uk:{} }` merged via `install` | ≈ 1 000 keys, ≈ 5 800 w/lang |
| **UI dictionaries (other apps)** | `store/i18n.js` (261 keys), `store/booking-words.js` (27), `room/i18n.js` (175), `courier/i18n.js` (143), `lib/mcp-words.js` (40), `wiki/wiki-core.js` (21), `ui-gallery/gallery.js` (94, dev only) | | ≈ 670 keys, ≈ 3 500 w/lang |
| **Locale/intl/voice maps** | `store/i18n.js:262` `intlLocale`, `store/nav.js:135`, `courier/i18n.js:13` `VOICE {sq-AL,en-US,uk-UA}`, `lib/voice.js:29,70,82` `tagFor`, `ui-gallery/gallery.js:57`, `platform/app.js:110`, `kit/screens/{track-order:46,e-receipt:68}` (`uk-UA` hardcoded) | BCP-47 tags | 8 |
| **Script detection** | `store/track.js:111` `langOf = CYRILLIC ? 'uk' : ALBANIAN ? 'sq' : 'en'` | **breaks with ru** (Cyrillic is ambiguous) | 1 |
| **Language switchers** | `admin/app.js:63,166` (`[data-l]` buttons), `courier/app.js:500,700` (`nextLang` cycles LANGS — fine), `room/app.js:192` (`nextLang`), `app.js:176` (store `setLang`), `platform/index.html:40-42` (three hardcoded buttons), `wiki/wiki.js:134` (`pickLang` over stored keys) | | 6 |
| `<html lang>` defaults | `admin,courier,store,room,wiki` = sq; `platform/index.html, hub.html, kit` = uk; `ui-gallery` = en | | 9 |
| **Storefront content (dish/category names)** | image `i18n`, key `<locale>/<entity>/<id>/<field>` (`hubstore.rs:836-852`); write: per-product `translations` map (`owner.rs:753-764`) and bulk `POST /api/owner/i18n` (`owner.rs:1098-1180`, ≤ 500 rows); read: `storefront.rs:302-345`, `?locale=`, fallback = venue's own string. **No machine translation anywhere** (`grep -rin translat` = storage code only). The catalogue comment sizes it as "165 dishes × 3 languages × 3 fields" (`hubstore.rs:851`) — 4 languages is still far under the 10 MiB ceiling | per venue: dubin-sushi 165 dishes (memory), sushi-durres 165 | ≈ 330-500 strings/venue |
| **Learn lessons** | `docs/learn/lessons/<role>/*.yaml`: 61 lessons (owner 36, waiter 12, courier 7, guest 6; `lessons.json:1` counts) — not 56; every `title/goal/caption` requires sq/en/uk (`build-lessons.mjs:42-49` refuses a missing one; README:114) | 1 300 uk strings, 6 710 uk words (8 131 en) | 1 300 strings |
| **Learn media** | `learn/media/manifest.json`: cuts are PER UI LANGUAGE (`cuts.sq/en/uk`, each with `subs_sq/en/uk.vtt`); 15 cuts = 5 recorded lessons × 3; `tools/learn/assemble-plan.mjs:39-45` writes one VTT per LANGS; `check.mjs:71-74` requires every `subs_<l>.vtt`; served by `learn.rs` from R2 `dowiz-learn` | ru = a 4th recording per lesson + a 4th VTT in every existing cut | 5 now, 61 eventually |
| **Wiki** | `wiki/wiki.js:1` "three caption tracks", `wiki-core.js:69`; captions come from `lessons.json` | | follows learn |
| **Landing / platform** | `platform/landing.js` T (124 keys, 5 046 w total; ≈ 1 700 w/lang), `index.html:40-42` buttons, `hub.html` (lang uk), film subtitles rendered per language (memory: uk/en/sq at 1080p); marketing copy literally says "Three languages" (`landing.js:105,129,152`) | | 124 keys + film |
| **Waitlist email** | `waitlist.rs:27,108` LANGS, `mail_raw` subject per lang `:62-73` | the only email in the Worker | ~5 strings |
| **Legal — P8 privacy notice** | `privacy/notice/{sq,en,uk}.rs` (1 340 / 1 270 / 1 136 words), `words.rs:74-82` | Rust consts, structured `Words` | ≈ 1 100 words |
| **Legal — P9 DPA** | `docs/privacy/DPA-v1-2026-09-24.{sq,en,uk}.md` (1 061 / 1 008 / 880 words), `include_str!` in `dpa.rs:28-40`, `check()` pins the version | | ≈ 900 words |
| **Consent wordings** | `consent/wordings.rs:23-30` — "Three languages are THREE wordings: a shared id could not say which was read" — a 4th wording is a 4th consent id | 1 sentence | 1 |
| **Erasure promise / STOP line** | `forget.rs:209-217`, `campaigns/send.rs:57` | 2 sentences | 2 |
| **Telegram/alert templates** | `alert.rs:86-108` (4 + 11 words), `notify::order_text` (English only), `legs.rs` | | ~20 words |
| **Voice / MCP prompts** | `services/engagement/voice/say.rs:20` (≈ 12 refusal lines × 3), `voice/words.rs:47` (spoken numbers 1-12), `voice/grammar.rs` + `crates/dowiz-hub/src/voice.rs` (deterministic grammar: verbs per language, `verb_word(v, uk, sq)` `:87`), `post.rs:376` language name, `mcp/tools.rs:46` enum, `lib/mcp-words.js` | grammar work, not just strings | ~150 words + grammar |
| **Stemmer** | `crates/dowiz-core/src/stem.rs:350` `Language::Ru` already exists | search stemming for ru is free | 0 |
| **Kit (design prototype)** | `public/kit/*` `lang="uk"`, `menu('uk')` (`kit/data.js:36`), `uk-UA` dates | not a product surface; leave | 0 |

Hardcoded-"3" assumptions (must change): `[&str; 3]` at `learn.rs:28`, `notice/words.rs:82`, `wordings.rs:23`,
`say.rs:20`; comments "three languages" in 14 files (list above, plus `nav.js:31,37`, `booking.js:254,386`,
`booking-words.js:1`); `zones.test.mjs:75` "merges the three languages"; `wiki.js:1` "three caption tracks";
`hubstore.rs:851` sizing comment; landing marketing copy; the learn README (`:28-40,114`).

**Total per language to produce:** ≈ 1 670 UI keys (≈ 9 000 words) + 124 landing keys (≈ 1 700 words) + 1 300 lesson
strings (≈ 7 000 words) + P8 1 100 + P9 900 + ≈ 200 words of runtime Rust strings + grammar verbs + per-venue dish names
(≈ 330-500 strings each) ≈ **20 000 words of ru text**, of which ≈ 2 000 (legal) need human review.

### B2. How to do it

1. **One source for the set first (B0).** `public/lib/langs.js` exporting `LANGS`, `INTL` (`{sq:'sq-AL', en:'en-GB',
   uk:'uk-UA', ru:'ru-RU'}`), `VOICE`, and `dowiz_hub::lang::{LANGS, is_lang}` in Rust; every literal above imports it;
   `build-lessons`/`check`/`publish`/`evals` read one `tools/learn/langs.mjs` (or the lib file). New gate
   `tools/gates/langs.sh`: grep for any `['sq'|"sq"` literal list outside the source files → baseline 0. Then adding
   `ru` is one edit per language (plus the texts).
2. **Machine-draft, then check.** Draft ru from **uk** (closest language; the operator reads both) with en as the
   second reference, key by key, into the same dictionary shape; a script `tools/i18n/draft.mjs` that emits a `ru:{}`
   block with every key of the union and marks each value `⟨draft⟩` until reviewed — the evals i18n rule (0 missing)
   passes structurally while a second rule ("no ⟨draft⟩ markers in a published surface") ratchets the review. The
   Worker must not translate at runtime (10 ms, cost); the venue's `ai.endpoint` (`integrations.rs:65-70`) may be
   offered in the console for **dish names** as a "draft ru/en/uk" button writing through `POST /api/owner/i18n`
   (Q5) — every draft is shown to the owner before it is saved.
3. **Human review:** P8 notice, P9 DPA, the consent wording (a new consent id `ru`), the erasure promise and the STOP
   line — legal texts; the operator authorised shipping P8/P9 without a lawyer (memory 2026-09-24), so the same rule
   can apply to ru with the operator as reviewer, noted in `docs/privacy/LEGAL-DECISIONS-2026-09-24.md`.
4. **Fallback rules (Q4):** UI keys — none needed once the gate holds 0 missing; until then a missing ru key falls to
   **en** (today a missing key renders as the key itself — `say.rs:5-6` — keep that in tests, fall back only in the
   dictionaries' `t()`). Dish content — `ru → en → venue default` (the existing rule "venue's own string" stays the
   last step, `storefront.rs:318-330`); the alternative `ru → uk` is a product choice. Legal — `ru` served only when
   the reviewed text exists; else `en` with a visible line "this notice is in English until the Russian text is
   reviewed" (never Albanian to a Russian reader; the notice's default `_ => SQ` at `words.rs:76` becomes explicit).
5. **Detection:** today only the landing reads `navigator.language` (`landing.js:252`); the storefront defaults to
   `sq` (`store/i18n.js:248`) and the wiki inherits stored keys. Add: on first visit with no stored `dw_lang`, pick
   the first of `navigator.languages` whose 2-letter tag is in `LANGS`; server-side `Accept-Language` for `/privacy`
   and `/dpa` when `?lang` is absent (`notice.rs:276` `lang_of`), and for the MCP `menu` tool default. Fix
   `track.js:111`: Cyrillic → distinguish ru (`ёъыэ`) from uk (`іїєґ`), else the UI language.
6. **Gates that enforce 4-language completeness:** evals i18n (`LANGS` + ru, rule 0 missing, plus the draft-marker
   rule); `build-lessons --check` with ru required (phase: `LANGS_REQUIRED` vs `LANGS_PUBLISHED` so the 61 YAMLs can
   be translated in batches without a red gate; the published set widens when a role is complete); `check.mjs` VTT per
   language; `learn.sh` item 4; `dpa/tests.rs` and `forget/tests.rs` loops over `LANGS`; `langs.sh` (literal lists);
   the design gate on the 4-button switchers (tap-size for a 4th button on the phone login).

### B3. Roadmap rows (Part B)

| # | step | files | proving test | gates | risk | decision |
|---|---|---|---|---|---|---|
| B0 | Single source of the set; replace 8 JS + 20 Rust + 7 tool literals; new `langs.sh` gate; comments/tests say "every language" | `lib/langs.js` (new), `crates/dowiz-hub/src/lang.rs` (new), all sites in B1 rows 1-4 | every existing test green with no behaviour change; `langs.sh` baseline 0 | langs (new), file-size | a missed literal shows up only when ru is added — the gate is the proof | — |
| B1 | ru in every UI dictionary (≈ 1 670 keys), intl/voice maps, `track.js` script fix, 4th switcher button on admin login / platform | the 17 dictionary files, `lib/voice.js`, `courier/i18n.js:13`, `admin/app.js`, `platform/index.html` | evals i18n: 0 missing for ru; draft-marker rule; `voice.test.mjs` "every word the mic uses" ×4; screenshot of the 4-button login at 360 px | evals i18n, tap-size, ui-adoption | speech recognition quality for ru (`lib/voice.js` uses the browser's engine; **unverified** on the venue phones) | fallback en vs uk |
| B2 | Storefront content ru for both venues; `default_locale` and translation locale validated against `LANGS` (**unverified** whether `owner.rs:1034-1058` validates the locale at all); optional AI draft button | `owner.rs`, `storefront.rs`, `admin/menu*.js`, per-venue data via `POST /api/owner/i18n` | live: `?locale=ru` on dubin-sushi returns ru names, a missing one falls back per the chosen chain; 500-row batch test | one-venue, personal-data (none) | catalogue tooling belongs to the stock roadmap (sibling doc) | Q5 |
| B3 | Legal ru: `notice/ru.rs`, `DPA-v1-…ru.md`, consent wording `ru`, erasure promise, STOP line | `privacy/notice/`, `docs/privacy/`, `dpa.rs`, `wordings.rs`, `forget.rs`, `send.rs` | `dpa/tests.rs` ×4; notice tests ×4; a consent accepted under `ru` is reported as `ru` | consent, personal-data | legal wording quality; reviewer named | Q6 |
| B4 | Learn ru: 61 YAMLs (1 300 strings), build/publish/check with `LANGS_REQUIRED/PUBLISHED`, `subs_ru.vtt` in every cut, a ru cut per recorded lesson | `docs/learn/lessons/**`, `tools/learn/*.mjs`, `lessons.json`, `media/manifest.json`, `learn.rs:28`, `wiki` copy | `build-lessons --check` green; `learn.sh` green; `check.mjs` passes on a 4-track cut; wiki shows 4 tracks | learn, learn.prove | 61 recordings × 1 more cut on the box (overnight renders were authorised 2026-09-26) | batch order: courier (7) → guest (6) → waiter (12) → owner (36) |
| B5 | Landing/platform ru: `landing.js` T.ru, 4th button, `hub.html`, waitlist mail, film subtitles; marketing copy "four languages" | `platform/*`, `waitlist.rs` | landing test per language; a waitlist row with `lang=ru` mails in ru | evals boot bytes (landing grows ≈ 20 KB) | film re-render for ru | — |
| B6 | Runtime Rust texts ru: alerts, refusal lines, voice grammar verbs, AI prompt language, MCP enum, Telegram templates from Part A in ru | `alert.rs`, `say.rs`, `voice/words.rs`, `dowiz-hub voice.rs`, `post.rs`, `mcp/tools.rs`, `notify_route` renderer | `say/tests.rs` no key falls through ×4; grammar tests for ru verbs (mirror `voice.rs:405-409`) | vocabulary, no-scoring | the grammar is rules, not strings — real work | — |

### Open questions for the operator

- **Q1** May the hub call `setWebhook` on the venue's bot (it replaces any webhook that bot already has), or must the
  owner paste a webhook secret and set it themselves?
- **Q2** Default customer-data level for a newly linked group: `none` (proposed) — the kitchen sees items and table/kind
  only; couriers get `fulfil`. Should the two live venues' existing chats keep `full`?
- **Q3** Inline buttons: is "anyone in the linked kitchen group" enough authority to mark an order ready or a dish 86,
  or must each presser be a roster member with a stored Telegram id (new staff personal data)?
- **Q4** Quiet hours: which events may wait (proposed: stock, waste, bookings, digests; never orders/status/late).
  And the old ADR's open ethics row: does a *late pending order* wake the owner at night?
- **Q5** ru fallback for dish names: `ru → en → venue` (proposed) or `ru → uk → venue`? And may the console offer an
  AI-drafted translation via the venue's own `ai.endpoint`?
- **Q6** Who reviews the Russian legal texts (P8, P9, consent wording)? Same rule as 2026-09-24 (operator, no lawyer)?
- **Q7** Learn media: record a ru cut for all 61 lessons, or publish ru captions on the uk cut first (the UI in the
  video would be Ukrainian with Russian subtitles) and record later?
- **Q8** Forum topics: require a supergroup with topics for "one group, many topics", or keep it optional (a plain
  group per team works without topics)?

### Unverified items (to prove before building)

- Free-plan subrequest cap per cron invocation and the DO's CPU budget for a digest fold.
- The General topic's `message_thread_id` value and whether `sendMessage` to a non-forum supergroup tolerates a thread id.
- Whether `owner.rs:1034-1058` rejects an unknown locale in `POST /api/owner/i18n` (it names the field and entity;
  the locale check was not read).
- Browser speech recognition quality for `ru-RU` on the venue phones.
