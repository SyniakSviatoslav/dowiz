# The canvas first frame's shared libraries: their long comments

The canvas board (`workers/api/public/room/canvas/`) loads two SHARED libraries before its first
frame: `workers/api/public/lib/outbox.js` (the offline write queue, also used by the room, the
courier and the console) and `workers/api/public/room/logic.js` (the room app's pure rules). Every
byte they ship counts against the first frame's 61440 B gzip budget (`tools/gates/canvas-wire.sh`),
and their comments were more than half of it: outbox.js 6415 -> 2293 B gzip, logic.js 5875 -> 2778 B
gzip without them (W-CV2P, 2026-10-07). So the explanations live HERE, the same way
`src/lib.rs` `THE HOST FILES` carries the canvas host files' own.

How this file was made: every comment that occupied whole lines was moved, found by the TypeScript
parser (so no string, template or regex was touched), and each block is filed under the code line
it stood before. Comments at the end of a code line stayed in the file. The code is unchanged: both
files print identically to their old versions with comments removed (TypeScript printer,
`removeComments`). WHEN YOU CHANGE ONE OF THESE FILES, change its section here in the same commit.

## outbox.js (workers/api/public/lib/outbox.js)

#### before `const DB_NAME = 'dowiz.outbox';`

    The tap that survives the basement.

    WHAT IT IS FOR. Reads already survive a dead signal on one surface: the
    console draws `replica.js` before it asks anything. WRITES did not survive
    anywhere. `public/kit/sw.js` says "`/api/` IS NEVER TOUCHED" and
    `public/sw.js` says "NETWORK FIRST, ALWAYS", both correctly -- a cached
    price is worse than no price -- and the courier app had an offline panel
    with a Retry button and nothing that queued a tap. A courier in a lift, a
    basement or a tram taps "picked up", `fetch` rejects, a toast says so, and
    the tap is gone. The kitchen is still waiting for a courier who left.

    WHY THIS NEEDS NO CRDT, which is worth stating because the obvious reflex is
    to reach for one. A courier's actions on ONE order are a SEQUENCE, not a set
    of concurrent edits: accept, then picked up, then delivered. The server's FSM
    refuses an illegal edge as an error rather than a silent no-op, and an
    `Idempotency-Key` makes the replay of the same tap the same answer. A queue
    that drains IN ORDER, replays idempotently, and believes the refusal is a
    deterministic merge already -- built out of what exists rather than out of a
    new vocabulary that has to be kept honest.

    THE KEY IS MINTED AT TAP TIME, NOT AT DRAIN TIME, and that is the whole
    point. A key generated as the request goes out is a new key on every attempt,
    which is exactly the duplicate the header exists to prevent; a key minted
    when the courier's thumb came off the glass is the identity of THAT TAP, and
    it stays the same however many times the phone finds and loses a signal.

    WHAT IT NEVER DOES. It never caches a read, and it holds nothing but the
    requests this browser has not managed to send. It never retries a refusal:
    a queued "delivered" replayed after the owner cancelled comes back 409 from
    the FSM, and a client that retried that would hammer production until the
    phone died. 409 is an ANSWER -- it is dropped and reported, and the courier
    is told the order changed while they were away.

    STORAGE CAN FAIL AND THAT IS NORMAL -- a private window, blocked site data,
    a full quota. Unlike `replica.js`, which degrades to "no replica" and carries
    on, a queue that cannot store REFUSES: `queue()` answers `{ ok: false }` so
    the caller can tell the courier the tap was not saved. Silently accepting a
    tap into nothing is the defect this file exists to close, not a fallback.

#### before `export const MAX_QUEUE = 64;`

    HOW MANY TAPS MAY WAIT. This is a HYPOTHESIS about a shift, not a
    measurement, and it is written down as one: this surface shows a courier
    ONE job at a time and a job takes at most three taps (accept, picked up,
    delivered), so 64 is about twenty jobs' worth of offline work -- longer
    than any tunnel, shorter than a queue that takes minutes to drain over a
    re-found 3G connection. What IS verified is the property that matters: the
    queue is finite and says so at the boundary (see the test).

#### before `const RETRY_STATUS = new Set([408, 425, 429, 500, 502, 503, 504]);`

    WHICH END GIVES WAY WHEN IT IS FULL, and why not the orphan's end. The
    orphan (`offline/OfflineQueue.ts`, typed against a deleted app and imported
    by nothing) evicted the OLDEST entry to make room. Here that would delete a
    "picked up" the courier was told was saved, leaving the "delivered" behind
    it to be refused by the FSM as an illegal edge -- two taps lost, one of them
    silently. This queue refuses the NEW tap instead, at the moment of the tap,
    while the courier is still looking at the screen and can be told.

    Statuses that mean "ask again later". 409 is deliberately NOT here: see
    `retryAfterOf` for the one 409 that is, and why it is distinguishable.

#### before `const MAX_TRIES = 6;`

    How many times a transient answer is honoured before the entry is dropped
    and reported. A server that has answered 5xx six times over the backoff
    below has been failing for four minutes; a courier needs to know.

#### before `const BACKOFF_MS = [2_000, 5_000, 15_000, 30_000, 60_000, 120_000];`

    Backoff between drains, indexed by the head entry's try count. Never faster
    than two seconds: a phone that keeps flapping between one bar and none must
    not spend its battery on the retry.

#### before `export function newKey(){`

    The identity of one tap. `randomUUID` is present on every browser this
    product supports over HTTPS; the fallback exists because it is absent on
    plain HTTP, which is how a developer's phone reaches a laptop on the LAN,
    and a key that throws there would take the whole tap with it.

#### before `function retryAfterOf(res){`

    The one 409 that is not a refusal. `idempotency.rs` rule 4: a retry that
    arrives while the FIRST copy of this same key is still running is answered
    409 WITH a `Retry-After`, precisely so it is not mistaken for rule 3's
    "same key, different body". The FSM's illegal-transition 409 carries no
    such header. Without this test, a client doing the right thing -- one tap,
    one key, a retry after a timeout -- would drop its own tap as "the order
    changed" the one time the header did its job.

#### before `try { req = indexedDB.open(name, DB_VERSION); }`

    `indexedDB` itself THROWS on access in a blocked-cookies third-party
    context rather than being undefined, so the guard has to be a try, not
    an `in` check.

#### before `if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE, { keyPath: 'seq', autoIncremen ...`

    `seq` IS THE ORDER. An auto-incrementing primary key means a cursor in
    its natural direction is insertion order, which is the property the
    whole design rests on -- a "delivered" must never overtake the
    "picked up" in front of it, or the FSM refuses work that was done.

#### before `let out;`

    THE TRANSACTION'S `complete` IS THE ANSWER, not the request's `success`.
    A `put` whose success fired and whose transaction then aborted (quota) has
    not stored anything, and resolving on success would have reported that tap
    as queued. `done` carries the value out; `complete` is what resolves.

#### before `export function createOutbox({ name = DB_NAME, authorize = () => ({}), onChange = () => {}, onSent = ...`

    An outbox over one origin's `/api`.

    `authorize()` is asked for headers AT DRAIN TIME, not at tap time: a courier
    whose token was refreshed while the phone was in a pocket must not replay a
    tap under the token that has since expired. It may return `null` to mean
    "there is no session right now", which pauses the drain rather than burning
    the entry's tries against a 401.
    `name`: ONE QUEUE PER APP. The courier and the room run on the same origin;
    sharing one database would let one app's drain replay the other's taps under
    its own token, and one app's sign-out would clear the other's queue.

#### before `let rerun = false;`

    A tap queued WHILE a drain is in flight must not wait for the next
    `online` event that may never come. The re-entrant call cannot run the
    loop (one sender, or two tabs' worth of duplicates), so it leaves this
    mark and the finishing drain runs again.

#### before `async function pending(){`

    Everything still waiting, oldest first, for a surface to draw.

#### before `async function queue(route, { method = 'POST', body = null, tag = null, key = newKey() } = {}){`

    Put a tap in the queue. The key is minted at TAP time and travels with the
    entry for the rest of its life.

    A CALLER MAY SUPPLY THE KEY, and the courier surface does. It mints one
    before its FIRST, direct attempt and hands the same one here if that
    attempt died on the network -- because a request whose response was lost
    may well have landed, and queueing it under a fresh key is the duplicate
    order the header exists to prevent, arriving by the back door.

    Answers `{ ok, key, seq }` or `{ ok: false, reason }` -- `'full'` when the
    bound is reached, `'nostore'` when this browser will not keep a queue at
    all. Both are for the caller to SAY, not to swallow.

#### before `schedule(0);`

    Try immediately. A tap made while the signal is merely SLOW should not
    wait for an `online` event that will never fire, because the browser
    never thought it was offline.

#### before `async function drain(){`

    Send what is waiting, in order, until something says wait.

    IT STOPS AT THE FIRST ENTRY IT CANNOT RESOLVE rather than skipping it.
    Draining past a stuck "picked up" would deliver "delivered" to an FSM that
    has never seen the pickup, and the correct refusal that follows would
    destroy a delivery that really happened.

#### before `if (!auth) { schedule(BACKOFF_MS[0]); return; }`

    No session: pause, do not spend a try. The courier will sign in
    again and the tap is still theirs until then.

#### before `entry.last_error = String(e?.message || e);`

    The network, not the server. Nothing has been decided about this
    tap, so it keeps its place and its key.

#### before `schedule(after);`

    Rule 4: the first copy of this exact key is still running. This is
    not a refusal and must not consume a try.

#### before `let detail = '';`

    THE SERVER ANSWERED AND SAID NO. 409 from the FSM (the order moved
    on without this courier), 403 (it is not theirs any more), 404 (it
    is gone), 400/422 (this tap can never be accepted). Repeating any of
    them produces the same answer for ever, so the entry is dropped and
    the surface is told which one it was. 401 is here too, deliberately:
    a tap replayed after a DIFFERENT courier signs in on the same phone
    would put one person's delivery under another's name.

#### before `function start(){`

    Begin listening and try once. Called on load: a queue that only drained on
    the `online` EVENT would sit for ever on a phone that was already online
    when the app opened -- which is every reopen after the tunnel.

#### before `async function forget(){`

    Throw the queue away. Used when the session ends, for the same reason
    `replica.js` forgets its copy: the next person at this screen is not
    entitled to the last one's unsent work, and replaying it under their
    token would file it under their name.

## logic.js (workers/api/public/room/logic.js)

#### before `import * as Money from '../lib/money.js';`

    The room app's PURE rules: no DOM, no fetch, no storage. Everything here is
    imported by `room/*.test.mjs` and called for real, so the imports are
    RELATIVE ('../lib/...'), which resolves to the same `/lib/...` URL in the
    browser and to the same file under `node --test`.

    MONEY IS INTEGER MINOR UNITS and is DRAWN ONLY BY `lib/money.js`. Parsing a
    typed amount is here, and is string arithmetic on the currency's decimals
    (`DECIMALS`, generated from the kernel) -- never a float, never a `/ 100`.

#### before `export const money = (amount, code, locale) => (code ? Money.format(amount, code, locale) : '—');`

    The escaper is /lib/ui's (`core.js`): no room module imported it from here, and re-exporting
    it made every page that loads these RULES download the design system's core (3.2 KB gzip on
    the canvas board's wire budget, W-CV1B).

    Draw an amount. The one door to `lib/money.js` for this app. A currency
    not yet known draws a dash, NOT a guess: `money.js` would fall back to two
    decimals, and 1500 lek drawn as "15.00" is the fourth-money-copy defect.

#### before `export const parseCaps = s => new Set(String(s || '').split(',').map(x => x.trim()).filter(Boolean)) ...`

    ── who may do what ─────────────────────────────────────────────────────────

    The token's `caps`: the canonical comma-joined spelling (`caps.rs` Display).

#### before `export function stageOf(status) {`

    `command::room_rules::stage`, the same three answers.

#### before `const NO_PAYMENT = new Set([...REFUSED, 'REFUNDING']);`

    `command::pay::refuses_payment`: the kernel's REFUSED set (money not kept)
    plus REFUNDING (money on its way back). Built from `/lib/vocab.js` so the
    `vocab` gate's rule holds: no hand copy of the status set.

#### before `export const GUEST = 'guest';`

    WHAT THIS PERSON IS SHOWN for one round. The server refuses the same things
    (`command::amend::apply`, `pay::decide`); hiding them is so a waiter is not
    offered a button whose only answer is "no".
    The signer word a guest's round carries (`placer::GUEST`, `pay::GUEST`).

#### before `export const guestWaiting = r => r?.placed_by === GUEST && r?.status === 'PENDING';`

    A guest's round still waiting for the room (D9). Not on the bill and not
    payable until a waiter confirms it: the server refuses the payment too
    (`dowiz_hub::room::pay::decide`) and leaves it out of `sitting::bill`.

#### before `remove: editable && orders && (stage === 'before' || voids),`

    After the pass a line leaves only with `void` (it was cooked).

#### before `comp: editable && orders && voids,`

    A comp is the void-holder's word at any stage.

#### before `export const canTill = caps => caps.has('open_till');`

    The till is the Counter-Manager's (`Cap::OpenTill`).

#### before `export const REASONS = ['mistake', 'guest_changed', 'unavailable', 'dropped', 'other'];`

    `room_rules::VoidReason`, in order. `other` carries text.

#### before `export function reasonWord(kind, text) {`

    The word the server parses, or null when it would refuse it.

#### before `export const METHODS = ['cash', 'card', 'cheque', 'transfer', 'gift_card', 'wallet', 'other'];`

    `command::pay::validate_method`, exact.

#### before `export const TILL_CURRENCIES = ['ALL', 'EUR'];`

    The two piles a Durrës drawer holds.

#### before `const decimalsOf = code => DECIMALS[code] ?? 2;`

    ── amounts typed by a person ───────────────────────────────────────────────

#### before `export function parseMinor(text, code) {`

    "20", "20.5", "20,50", "1 950" -> minor units of `code`, or null. More
    fractional digits than the currency has is refused, not rounded: 97.505
    lek is a typing mistake, not an amount.

#### before `export function minorToInput(minor, code) {`

    Minor units back to what an input field holds ("20.00", "1950"). An INPUT
    value, not a display: display is `money()`.

#### before `export function quoteOf(orderCur, paidCur) {`

    ── a payment in another currency (command/pay/fx.rs) ───────────────────────

    THE HUMAN UNIT is the board by the till: "1 EUR = 97.50 ALL". The side
    that is not lek is the one quoted per unit; between two non-lek
    currencies, the one handed over is.

#### before `export function parseRate(text) {`

    "97.50" -> exact fraction {n, d} of BigInts, or null.

#### before `export function ratePpm(text, orderCur, paidCur) {`

    `rate_ppm`: ORDER minor units per ONE PAYMENT minor unit, x 1 000 000,
    rounded half-up, from the human quote. Exact integer arithmetic. Null when
    the two currencies are the same (the server refuses a rate then) or the
    text is not a rate.

#### before `const ppm = paidCur === base`

    Paid IS the base: one paid major = R order majors.

#### before `: halfUp(r.d * dO * M, r.n * dP);`

    Order is the base: one paid major = 1/R order majors.

#### before `export function convertPpm(amount, ppm) {`

    `fx::convert`, for the PREVIEW beside the field. The figure that counts is
    the server's `amount_in_order_currency`, shown once it answers.

#### before `export function maxAmountFor(owedMinor, ppm) {`

    The most of `paidCur` that does not pay past `owedMinor` of the order's
    currency at `ppm` (the server refuses Σ > total).

#### before `export const settles = p => (Number(p?.amount_in_order_currency ?? p?.amount ?? 0) || 0) + (Number(p ...`

    ── what is still owed ──────────────────────────────────────────────────────

    `command::pay::settles`: what one recorded payment took off the bill --
    its bill share plus the TIP it carried, which raised the total by itself.

#### before `export function keepTheChange(handed, owedMinor) {`

    ── a tip at payment (OPERATIONAL-BLIND-SPOTS §2.3) ─────────────────────────

    "KEEP THE CHANGE": the guest hands `handed` for `owedMinor` (both in the
    bill's currency). The payment is the owed share, the rest is the tip --
    the server raises the round's tip and total by it in the same `Paid`.
    Null when the hand-over does not cover what is owed.

#### before `export function tipMinor(text, code) {`

    The tip field, parsed: '' is no tip (0), anything else must read as
    minor units of the bill's currency. Null = refuse before sending.

#### before `export const walletOk = (method, wallet) => method !== 'wallet' || String(wallet || '').trim().lengt ...`

    A wallet payment names its wallet (`PayIn.wallet`), and only it does.

#### before `export const walletField = v => {`

    D13 (G6): what the wallet field sends. The customer's own code -- their
    order token, three dot-joined parts -- goes as `wallet_token`, and staff
    spend only the wallet it names; anything else is a wallet id, which only
    the owner may name (`room/pay/whose.rs`).

#### before `export const walletTipOk = (method, tip) => method !== 'wallet' || !(tip > 0);`

    A wallet pays the bill's share only (`command::pay::decide`): its leg
    debits `amount`, so a tip on it would settle money nobody paid.

#### before `export function owed(round) {`

    What a round still owes, in its order's currency. The payments list when
    the round carries one (a pay answer's `order` does); the card's `paid`
    otherwise.

#### before `export const sittingDue = s => (s?.rounds || []).filter(r => !REFUSED.has(r.status) && !guestWaiting ...`

    `took_money`'s complement, for the bill: a refused round is not owed, and
    neither is a guest's round nobody has confirmed (D9, `sitting::billed`).

#### before `export function slugOfHost(hostname, search) {`

    ── where, and how old ──────────────────────────────────────────────────────

    The venue the address names -- the storefront's rule (`store/state.js`).

#### before `export function ageOf(ms) {`

    An age in the largest whole unit: {n, unit: 's'|'m'|'h'}.

#### before `export function canTransfer(caps, round) {`

    ── moving lines and tables (BLUEPRINT-POS-THE-ROOM §2.9) ───────────────────

    `command::transfer::apply`: a round gives or takes lines only BEFORE the
    kitchen, unpaid, and by someone who takes orders. There is no capability
    that makes it legal after the pass (the server says why in its header).

#### before `export function transferTargets(caps, sittings, fromId) {`

    Every OTHER round in the room that may receive lines: any sitting, so a
    guest who joins another table takes their dishes with them.

#### before `export function transferBody(loc, from, to, lines) {`

    The body `POST /staff/orders/:id/transfer` takes, or `{error}` naming the
    i18n key of why it would be refused. Lines are indices into the source's
    items AS THIS SCREEN SHOWED THEM; both versions travel so a stale screen
    is refused rather than moving the wrong dish.

#### before `if (picked.length === n) return { error: 'notAllLines' };`

    A round with no lines left is a cancellation, not a move (room_rules).

#### before `export function canMoveSitting(caps, sitting) {`

    May this sitting be moved to another table? Every round still in the room
    goes; a PAID one not yet served blocks it (`transfer::sitting`).

#### before `export function refusalKey(status, message) {`

    The server's refusal, as the i18n key that says it in the waiter's
    language, or null to show its own words. Matched on the server's fixed
    phrases (`transfer.rs`, `room_rules.rs`, `transfer/sitting.rs`).

#### before `export const FLOOR_STATES = ['free', 'booked', 'ordering', 'waiting', 'paying', 'dirty'];`

    ── floor states ────────────────────────────────────────────────────────────

    The six states `GET /api/staff/floor` answers (`command::floor::FloorState`),
    in the legend's order. Anything else the server sends is drawn as free.

#### before `export const canClear = (caps, state) => caps.has('take_orders') && state === 'dirty';`

    May this signer tap a table to clear it? The cap is TakeOrders, the one the
    route checks; only a dirty table is cleared. The server decides again.
