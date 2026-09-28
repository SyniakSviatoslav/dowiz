// STAGED STATE for the recorder (lane W-LESSON, 2026-09-28). Some lessons teach a screen that
// only exists while the venue is in a state a read-only recording cannot make: a courier's
// offer, a run in hand, cash at the door; a table with a round on it; a checkout with pickup
// and delivery zones switched on. W-VIDEO filmed them against whatever the QA hub happened to
// hold (a courier left on shift with one order, no open table): C2..C4 and every round lesson
// came out with their controls absent.
//
// A stage makes that state INSIDE THE BROWSER. tools/learn/capture.mjs answers the stage's
// reads from here (Playwright route interception), and a staged lesson's own writes (open the
// shift, take the order, delivered, pay) are answered `{ ok: true }` in the browser too -- so
// the app moves on exactly as it does on a real venue, and NOTHING reaches the venue. A staged
// lesson therefore never writes, even on the QA hub.
//
// A stage: { start, after, answers, writes, patch, local, geo, dialogs, speech }
//   start     the state the recording opens in
//   after     { n: state } -- from the moment step n's control is tapped or typed into, the
//             reads answer from `state` (the app's own reload after the tap then shows it)
//   answers   { state: { '<path>': body | (ctx) => body } } -- a whole answer, by exact pathname
//   patch     { '<path>': object } -- merged into the venue's REAL answer (deep, objects only)
//   local     localStorage entries seeded before the app starts
//   geo       { latitude, longitude } -- the browser grants geolocation there
//   writes    { '<path>': body | 'offline' } -- what one of the lesson's own writes answers, where
//             the app reads the answer (the courier's voice proposal); 'offline' = the network never
//             carried it (the request fails, as in a tunnel); 'session' = the recorder's own sign-in
//             answer (ctx.session); any other write answers { ok: true }
//   outbox    [ { route, tag } ] -- taps already waiting in the phone's outbox (lib/outbox.js's
//             IndexedDB) when the app starts: outboxShim puts them there; { db, rows } names
//             another app's outbox database (the room's is dowiz.room.outbox)
//   signedOut true: the app opens on its sign-in screen (the role's session is not seeded); a
//             write staged as 'session' answers the recorder's REAL sign-in, so the app goes on
//             signed in exactly as after a real one -- and no credential is typed on camera
//   pick      { n: value } -- step n taps the option carrying data-v="value" rather than the one
//             already chosen (the recorder's default: tapping "English" keeps the film English)
//   choose    { n: [selector, value] } -- before step n, the native <select> at selector takes value
//             (its change event fires): a field it shows (a campaign's "days", its tag) is then on screen
//   dialogs   true: a native confirm() is accepted (the courier's "delivered?")
//   speech    a phrase: the browser's recogniser is replaced by one that "hears" it (speechShim),
//             so a voice lesson shows what the app does with a phrase -- headless Chromium has no mic
// Everything here is pure data and pure functions; capture.test.mjs proves each rule.

export const QA_SLUG = 'qa-durres';
const T0 = Date.UTC(2026, 8, 28, 11, 0);   // a fixed lunchtime: fixtures carry no wall clock

/// One courier order as `GET /api/courier/tasks` spells it (read off the QA hub 2026-09-28).
export function courierOrder(o = {}) {
  return { id: 'b7e41c20-5d1a-4c55-9d0e-3f6a2c9e7a11', items: 2, payment: 'cash', status: 'READY', total: 1800,
    address: { line: 'Rruga Tregtare 5, Durrës', note: 'Floor 2, ring Hoxha', lat_udeg: 41318500, lon_udeg: 19449800,
      parts: { street: 'Rruga Tregtare', house: '5', private: false } },
    contact: { name: 'Ana', phone: '+355 69 000 0000' }, ...o };
}
const SECOND = courierOrder({ id: 'c3a9f5e2-8b7d-4e11-a2c4-6d0b9e1f3a22', items: 1, total: 900, payment: 'card',
  address: { line: 'Bulevardi Epidamn 12, Durrës', note: '', lat_udeg: 41316000, lon_udeg: 19452000, parts: { street: 'Bulevardi Epidamn', house: '12', private: false } } });
const SHIFT = { id: 'qa-courier', cash: 0, deliveries: 3 };
const tasks = (onShift, mine = [], available = []) => ({ onShift, mine, available, shift: onShift ? SHIFT : null, courierId: 'qa-courier' });

/// The courier's states. An offer's clock is counted from when the offer was first served.
export const COURIER = {
  offShift: { '/api/courier/tasks': tasks(false) },
  waiting:  { '/api/courier/tasks': tasks(true) },
  pickList: { '/api/courier/tasks': tasks(true, [], [courierOrder(), SECOND]) },
  offer:    { '/api/courier/tasks': ({ since }) => tasks(true, [courierOrder({ offerEndsMs: since + 120000 })]) },
  active:   { '/api/courier/tasks': tasks(true, [courierOrder()]) },
  onTheWay: { '/api/courier/tasks': tasks(true, [courierOrder({ status: 'IN_DELIVERY' })]) },
  // the next run on the road, after the cash one was delivered (C4's door refusal)
  nextRun:  { '/api/courier/tasks': tasks(true, [courierOrder({ ...SECOND, status: 'IN_DELIVERY' })]) },
};
const EARNINGS = { cashInHand: 1800, expectedCash: 0, today: { deliveries: 3, cash: 1800, tips: 200 }, week: { deliveries: 17, cash: 9400, tips: 900 }, month: { deliveries: 61, cash: 35200, tips: 3100 } };
const HISTORY = { history: [
  { at: T0 - 3600e3, street: 'Rruga Tregtare 5', status: 'DELIVERED', total: 1800, cashCollected: 1800 },
  { at: T0 - 7200e3, street: 'Bulevardi Epidamn 12', status: 'DELIVERED', total: 900, cashCollected: 0 },
  { at: T0 - 9000e3, street: 'Rruga Aleksandër Goga 3', status: 'DELIVERED', total: 2400, cashCollected: 2400 } ] };
for (const s of Object.values(COURIER)) { s['/api/courier/earnings'] = EARNINGS; s['/api/courier/history'] = HISTORY; }

/// One round as `GET /api/staff/room` spells it (room/*.test.mjs fixtures, order fields).
export function round(o = {}) {
  return { id: 'e1f2a3b4-0000-4000-8000-000000000001', seq: 1, status: 'CONFIRMED', placed_by: 'staff', payment_status: 'unpaid',
    subtotal: 2580, discount: 0, tip: 0, total: 2580, payments: [], adjustments: [], location_id: QA_SLUG,
    fulfilment: { kind: 'dine_in', table: '4' },
    items: [{ product_id: 'qa-salmon-roll', name: 'QA Salmon roll', quantity: 2, unit_price: 900, modifier_ids: [] },
      { product_id: 'qa-tea', name: 'QA Green tea', quantity: 3, unit_price: 260, modifier_ids: [] }], ...o };
}
const GUEST_ROUND = round({ id: 'e1f2a3b4-0000-4000-8000-000000000002', seq: 2, status: 'PENDING', placed_by: 'guest', subtotal: 900, total: 900,
  items: [{ product_id: 'qa-salmon-roll', name: 'QA Salmon roll', quantity: 1, unit_price: 900, modifier_ids: [] }] });
// Table 4 has two rounds, so tapping it opens the sitting's list of rounds (room/app.js: ONE round
// opens straight into the round sheet, and the sitting screen W2 teaches would never be seen).
const SECOND_ROUND = round({ id: 'e1f2a3b4-0000-4000-8000-000000000004', seq: 2, subtotal: 780, total: 780,
  items: [{ product_id: 'qa-tea', name: 'QA Green tea', quantity: 3, unit_price: 260, modifier_ids: [] }] });
export const ROOM = {
  table: { '/api/staff/room': { sittings: [
    { sitting_id: 'qa-sit-4', table: '4', rounds: [round(), SECOND_ROUND] },
    { sitting_id: 'qa-sit-6', table: '6', rounds: [round({ id: 'e1f2a3b4-0000-4000-8000-000000000003', fulfilment: { kind: 'dine_in', table: '6' }, subtotal: 2000, total: 2000,
      items: [{ product_id: 'qa-tuna-roll', name: 'QA Tuna roll', quantity: 2, unit_price: 1000, modifier_ids: [] }] })] }] } },
  // a guest's own round from the table's QR, waiting for the waiter (W6): the only round there,
  // so the first round row the recorder taps is the one with the confirm/reject bar
  guest: { '/api/staff/room': { sittings: [{ sitting_id: 'qa-sit-4', table: '4', rounds: [GUEST_ROUND] }] } },
};

/// The storefront with every checkout choice switched on (the QA venue has pickup, zones and
/// cards off): merged into the venue's real menu answer, so the dishes stay the venue's own.
const MENU = `/api/public/locations/${QA_SLUG}/menu`;
const ALL_ON = { location: { pickup: true, hasDeliveryZones: true, payments: { cash: true, card: true } } };
const SAVED = JSON.stringify([{ line: 'Rruga Tregtare 5, Durrës', lat: 41.3185, lng: 19.4498 }]);

/// The courier's agent panel (courier/mcp.js): one key already made, and the new one the mint returns.
const KEYS = { '/api/courier/mcp/keys': { keys: [{ id: 'qa-key-1', label: 'Laptop', expiresMs: T0 + 30 * 86400e3 }] } };
const AGENT = { waiting: { ...COURIER.waiting, ...KEYS } };

/// A table booking (store/booking.js, booking-mine.js): the request is answered with the booking and
/// its guest token, and the booking then reads back as requested, cancellable by the guest.
const BOOKINGS = `/api/public/locations/${QA_SLUG}/reservations`, BOOKING_ID = 'qa-booking-000001';
const BOOKED = { booked: { [`${BOOKINGS}/${BOOKING_ID}`]: { id: BOOKING_ID, status: 'REQUESTED', slotMin: Math.floor(T0 / 60000) + 26 * 60,
  party: 2, tableN: 4, contactName: 'Ana', next: ['CANCELLED_BY_GUEST'] } } };

/// The floor (room/floor.js; the server answers each table's state): one table in every state, the
/// dirty one tappable, and a dirty table that is on no plan (W4).
const ft = (n, x, y, state, o = {}) => ({ n, x, y, w: 60, h: 60, shape: n % 2 ? 'rect' : 'circle', seats: 4, state, ...o });
const FLOOR = { '/api/staff/floor': { planW: 390, planH: 300, zones: [{ id: 'qa-hall', name: 'QA Hall', tables: [
  ft(1, 70, 70, 'free'), ft(2, 195, 70, 'booked'), ft(3, 320, 70, 'ordering'),
  ft(4, 70, 200, 'waiting', { sitting_id: 'qa-sit-4' }), ft(5, 195, 200, 'paying', { sitting_id: 'qa-sit-5' }), ft(6, 320, 200, 'dirty', { sitting_id: 'qa-sit-6' })] }],
  unplaced: [{ table: '12', state: 'dirty', sitting_id: 'qa-sit-12' }] } };
/// The till's last answer as this phone keeps it (room/till.js, dw_room_till): a closed drawer, its Z (W10).
const Z = JSON.stringify({ loc: QA_SLUG, ans: { kind: 'till.closed', open: false, opened_at: T0 - 8 * 3600e3, closed_at: T0,
  float: { ALL: 500000, EUR: 5000 }, counted: { ALL: 1230000, EUR: 12000 }, expected: { ALL: 1234000, EUR: 12000 },
  over_short: { ALL: -4000, EUR: 0 }, pay_in: {}, pay_out: {}, cash_paid: {} } });
/// The waiter's agent panel (room/mcp.js): one key already made, and the new one the mint returns (W11).
const STAFF_KEYS = { '/api/staff/mcp/keys': { keys: [{ id: 'qa-key-2', label: 'Laptop', expiresMs: T0 + 30 * 86400e3 }] } };

/// The owner's till link (admin/ebills.js, fiscal.js): a linked till with one code still to match,
/// one matched, the till's floor; the fiscal sender switched off, one bill waiting, then armed (O11a/b).
const OWNER_Q = '/api/owner';
const EBILLS = { [`${OWNER_Q}/ebills`]: {
  config: { enabled: true, user: 'qa-till', pos_id: 1, secret_set: true },
  state: { last_ok_ms: T0 - 120e3, placed: 14, paid: 12, noted: 2, watermark: 3481 },
  products: [{ id: 'qa-salmon-roll', name: 'QA Salmon roll', price: 900 }, { id: 'qa-tuna-roll', name: 'QA Tuna roll', price: 1000 },
    { id: 'qa-water', name: 'QA Water', price: 150 }],
  unmatched: [{ code: '1042', name: 'Salmon roll 8pc', price: 900, suggest: [{ product_id: 'qa-salmon-roll', product: 'QA Salmon roll', price_agrees: true }] }],
  mapped: [{ code: '1001', name: 'Water 0.5', product: 'QA Water', product_id: 'qa-water' }],
  floor: { at_ms: T0 - 60e3, tables: [{ table: '1', occupied: true, unpaid: 1800 }, { table: '2', occupied: false }, { table: '3', occupied: true, unpaid: 900 }] } } };
const fiscal = armed => ({ [`${OWNER_Q}/fiscal`]: { armed, why: armed ? [] : ['not armed'], last_ok_ms: T0 - 3600e3, backlog: 1, overdue: 0, sent: 41,
  waiting: [{ order_id: 'QA-1042', stage: 'queued', deadline: T0 + 3600e3 }], floor: ['Hall', 'Bar'], items: [{ code: 'FEE1', name: 'Delivery fee' }],
  arming: { armed, sale_unit: 'Hall', fee_item: 'FEE1', cancel_armed: false }, marker: 'QA', confirm: 'ARM QA', cancel_note: '', cancels: [] } });
const TILL_LINK = { off: { ...EBILLS, ...fiscal(false) }, armed: { ...EBILLS, ...fiscal(true) } };

/// The owner's customers (admin/more.js openCustomers, customers.js): three people, one with a card and a
/// second number linked in; the reveal log with two reveals (O13, O13b). Names and numbers are made up.
const CUSTOMERS = { [`${OWNER_Q}/customers`]: { customers: [
    { key: 'c-ana', name: 'Ana', phone: '+355 69 000 0001', orders: 12, spent: 21600, lastAt: T0 - 86400e3,
      note: 'Window seat', tags: ['regular'], allergens: ['sesame'], usualTable: '4', lang: 'en', birthdayMd: '03-14', linked: ['+355 69 000 0009'] },
    { key: 'c-ben', name: 'Ben', phone: '+355 69 000 0002', orders: 5, spent: 7400, lastAt: T0 - 3 * 86400e3 },
    { key: 'c-dea', name: 'Dea', phone: '+355 69 000 0003', orders: 2, spent: 2800, lastAt: T0 - 9 * 86400e3 }] },
  [`${OWNER_Q}/customers/reveals`]: { reveals: [
    { reason: 'Called about a forgotten bag', by: 'owner', atMs: T0 - 2 * 3600e3 }, { reason: 'Delivery address check', by: 'owner', atMs: T0 - 26 * 3600e3 }] } };
/// The exceptions report (admin/exceptions.js): two comps and a void today, and one wallet leg missing (O12).
const EXCEPTIONS = { [`${OWNER_Q}/exceptions`]: { threshold: 3, lateMinutes: 45, alertChat: true, names: { 'u-1': 'Arta' },
    groups: [{ kind: 'comp', reason: 'Waited too long', count: 2, amount: { ALL: 1800 } }, { kind: 'void', reason: 'Wrong table', count: 1, amount: { ALL: 900 } }],
    rows: [{ kind: 'comp', at: T0 - 5400e3, order_id: 'QA-1041', reason: 'Waited too long', by: 'u-1', amount: 900, currency: 'ALL' },
      { kind: 'comp', at: T0 - 3600e3, order_id: 'QA-1044', reason: 'Waited too long', by: 'u-1', amount: 900, currency: 'ALL' },
      { kind: 'void', at: T0 - 1800e3, order_id: 'QA-1046', reason: 'Wrong table', by: 'u-1', amount: 900, currency: 'ALL' }] },
  [`${OWNER_Q}/wallet/legs`]: { holds: false, spends: [], handBack: [], audit: { missing: [{ order_id: 'QA-1039', wallet: 'stamps', amount: 500, currency: 'ALL' }], mismatched: [], orphans: [] } } };

/// The owner's promo codes (admin/more.js openPromos): one running, one waiting for its first day (O14a).
const PROMOS = { list: { [`${OWNER_Q}/promotions`]: { promotions: [
  { code: 'WELCOME10', kind: 'percent', value: 10, minOrder: 1500, used: 23, maxUses: 100, active: true },
  { code: 'AUTUMN', kind: 'fixed', value: 300, used: 0, active: true, fromMs: T0 + 3 * 86400e3, untilMs: T0 + 33 * 86400e3 }] } } };
/// The owner's campaigns (admin/campaigns.js): one sent last week, and the one the lesson saves, unsent,
/// with the preview's count before anything is queued (O14b).
const CAMPAIGN_ID = 'qa-campaign-autumn';
const CAMPAIGN = { id: CAMPAIGN_ID, name: 'Autumn', text: 'A warm soup for the cold days.', segment: { kind: 'not_seen_since', days: 30 },
  template: { name: 'autumn_soup', lang: 'en', params: ['Miso soup'] }, atMs: T0 };
const CAMPAIGNS = { list: {
  [`${OWNER_Q}/campaigns`]: { tags: ['regular', 'vip'], campaigns: [
    { campaign: { id: 'qa-campaign-summer', name: 'Summer', text: 'Cold noodles are back.', segment: { kind: 'everyone_consented' }, atMs: T0 - 7 * 86400e3 }, sent: 41 },
    { campaign: CAMPAIGN, sent: 0 }] },
  [`${OWNER_Q}/campaigns/${CAMPAIGN_ID}`]: { campaign: CAMPAIGN, report: { sent: 0, waiting: 0, delivered: 0, withdrawn: 0, abandoned: 0 } } } };
/// The owner's post drafts (admin/more.js openPosts): one draft waiting for a yes (O14c).
const POSTS = { list: { [`${OWNER_Q}/posts`]: { enabled: true, channel: '@qa_durres', posts: [
  { id: 'qa-post-1', text: 'Today: miso soup for 450 lek.', about: 'Miso soup', state: 'draft' },
  { id: 'qa-post-0', text: 'Fresh salmon rolls all week.', about: 'QA Salmon roll', state: 'published' }] } } };

/// The owner's inbox (admin/more.js openInbox, openThread): two customers who wrote, one unread (O16d).
/// A peer is a bare number: a '+' would be encoded into the path the thread reads.
const INBOX = { list: {
  [`${OWNER_Q}/inbox`]: { channels: { whatsapp: true, instagram: true }, threads: [
    { peer: '355690000001', channel: 'whatsapp', name: 'Ana', last: 'Are you open tonight?', fromThem: true, unread: 1, atMs: T0 - 600e3 },
    { peer: 'qa.ben', channel: 'instagram', name: 'Ben', last: 'Thank you!', fromThem: false, unread: 0, atMs: T0 - 86400e3 }] },
  [`${OWNER_Q}/inbox/355690000001`]: { messages: [
    { fromThem: true, text: 'Hello! Are you open tonight?', atMs: T0 - 660e3 }, { fromThem: true, text: 'Until what time?', atMs: T0 - 600e3 }] } } };
/// The venue's agreement with dowiz (admin/dpa.js): an older version accepted, the current one not yet (O16e).
const DPA_V = '2026-09', DPA_OLD = { version: '2026-06', atMs: T0 - 90 * 86400e3 };
const DPA = { old: { [`${OWNER_Q}/dpa`]: { version: DPA_V, accepted: DPA_OLD, current: false } } };
/// The venue's own Telegram bot and one kitchen group (admin/telegram.js), and the printer's queue (O16f).
const TELEGRAM = { [`${OWNER_Q}/telegram`]: { tokenSet: true, bot: { username: 'qa_durres_bot', hook_ms: T0 - 60e3 }, legacy: false, pending: null,
  groups: [{ id: 'kitchen', chat: '-1001', title: 'Kitchen', kind: 'supergroup', lang: 'en', pii: 'fulfil', state: 'active', digest_at: 540,
    subs: { 'order.placed': 'now' }, health: { ok_ms: T0 - 300e3 }, waiting: 0 }],
  events: [{ key: 'order.placed', area: 'orders', live: true, scheduled: false, urgent: true },
    { key: 'digest.daily', area: 'analytics', live: true, scheduled: true, urgent: false }],
  langs: ['sq', 'en', 'uk', 'ru'], stations: ['kitchen', 'bar'] },
  [`${OWNER_Q}/print/jobs`]: { jobs: [{ orderId: 'f5418efd-0000-4000-8000-000000000001', state: 'printed' },
    { orderId: '699d16ac-0000-4000-8000-000000000002', state: 'failed', tries: 3, code: 'paper' }] } };
/// The venue's API keys (admin/more.js openKeys): one key for the accounting program (O16h).
const API_KEYS = { [`${OWNER_Q}/apikeys`]: { keys: [{ id: 'qa-apikey-accounting', label: 'Accounting', lastUsedMs: T0 - 3600e3, expiresMs: T0 + 300 * 86400e3 }] } };
/// The agents' keys the venue's people minted in their own apps (admin/mcp.js): one waiter's (O16j).
const MCP_KEYS = { [`${OWNER_Q}/mcp/keys`]: { keys: [{ id: 'qa-mcp-arta', label: 'Laptop', role: 'waiter', holder: 'u-1', name: 'Arta', expiresMs: T0 + 30 * 86400e3 }] } };

/// The waiter's sign-in (W1): the room behind it, one payment still in the room's outbox.
const PAY = `/api/staff/orders/${round().id}/pay`;
const SIGN_IN = { answers: ROOM, start: 'table', signedOut: true,
  writes: { '/api/staff/login': 'session', '/api/staff/claim': 'session', [PAY]: 'offline' },
  outbox: { db: 'dowiz.room.outbox', rows: [{ route: PAY, tag: 'pay:' + round().id }] } };

export const STAGES = {
  O14a: { start: 'list', after: {}, answers: PROMOS },
  // O14b: the segment is chosen before the field it shows (a native select: no tap opens it on film)
  O14b: { start: 'list', after: {}, answers: CAMPAIGNS, choose: { 7: ['[data-tour="campaigns.segment"]', 'not_seen_since'], 8: ['[data-tour="campaigns.segment"]', 'tag'] },
    writes: { [`${OWNER_Q}/campaigns`]: { campaign: CAMPAIGN },
      [`${OWNER_Q}/campaigns/${CAMPAIGN_ID}/preview`]: { preview: { count: 18, already: 0, cost_minor: 54, currency: 'USD' } } } },
  O14c: { start: 'list', after: {}, answers: POSTS },
  O16d: { start: 'list', after: {}, answers: INBOX },
  // O16e: the agreement reads as not yet accepted in its current version; Accept answers it accepted
  O16e: { start: 'old', after: {}, answers: DPA,
    writes: { [`${OWNER_Q}/dpa/accept`]: { version: DPA_V, accepted: { version: DPA_V, atMs: T0 }, current: true } } },
  O16f: { start: 'on', after: {}, answers: { on: TELEGRAM } },
  O16h: { start: 'on', after: {}, answers: { on: API_KEYS }, writes: { [`${OWNER_Q}/apikeys`]: { key: 'example-key-shown-once', id: 'qa-apikey-new' } } },
  O16j: { start: 'on', after: {}, answers: { on: MCP_KEYS } },
  // O19: the mic "hears" a rejection; the hub's answer is the read-back with its reason field (admin/voice.js propose)
  O19: { start: 'on', after: {}, answers: { on: {} }, speech: 'reject 4821',
    writes: { '/api/voice': { understood: true, needsConfirmation: true, verb: 'reject', orderId: 'qa-order-4821', readback: 'Reject order 4821', token: 'staged-voice' } } },
  // O16a: the console opens on its sign-in screen; the sign-in is answered with the recorder's own
  O16a: { start: 'in', after: {}, answers: { in: {} }, signedOut: true, writes: { '/api/auth/login': 'session' } },
  O12: { start: 'report', after: {}, answers: { report: EXCEPTIONS },
    writes: { [`${OWNER_Q}/wallet/legs/repair`]: { wouldWrite: ['QA-1039'], wouldHandBack: [], refused: [], handBackRefused: [] } } },
  O13: { start: 'list', after: {}, answers: { list: CUSTOMERS } },
  O13b: { start: 'list', after: {}, answers: { list: CUSTOMERS } },
  O11a: { start: 'off', after: {}, answers: TILL_LINK },
  // O11b: the phrase typed, Arm tapped: the sheet reloads armed and shows Disarm
  O11b: { start: 'off', after: { 10: 'armed' }, answers: TILL_LINK },
  C2: { start: 'offShift', after: { 1: 'pickList', 3: 'offer' }, answers: COURIER, geo: true },
  C3: { start: 'active', after: {}, answers: COURIER, geo: true },
  C4: { start: 'onTheWay', after: { 3: 'nextRun' }, answers: COURIER, geo: true, dialogs: true },
  // C5: a "picked up" the phone is still holding, and the network still down for it
  C5: { start: 'active', after: {}, answers: COURIER, geo: true,
    outbox: [{ route: `/api/courier/orders/${courierOrder().id}/pickup`, tag: `pickup:${courierOrder().id}` }],
    writes: { [`/api/courier/orders/${courierOrder().id}/pickup`]: 'offline' } },
  // C1: the help button is there only while the courier stands still (courier/app.js initGuide)
  C1: { start: 'waiting', after: {}, answers: COURIER, geo: true },
  // C6: the hub's answer to a heard command is a proposal the courier confirms (services/engagement/voice.rs)
  // (the words C6's caption tells the courier to say), and the assistant's answer to the ask box
  C6: { start: 'waiting', after: {}, answers: COURIER, geo: true, speech: 'picked up',
    writes: { '/api/voice': { understood: true, needsConfirmation: true, verb: 'pickup', readback: 'Order B7E4 picked up', token: 'staged-voice' },
      '/api/courier/assist': { answer: 'Today: 3 deliveries, 1800 lek in cash, 200 lek in tips.' } } },
  C7: { start: 'waiting', after: {}, answers: AGENT, geo: true,
    writes: { '/api/courier/mcp/keys': { key: 'example-key-shown-once' } } },
  W1: SIGN_IN,
  W4: { start: 'table', after: {}, answers: { table: { ...ROOM.table, ...FLOOR } } },
  W10: { start: 'table', after: {}, answers: ROOM, local: { dw_room_till: Z } },
  W11: { start: 'table', after: {}, answers: { table: { ...ROOM.table, ...STAFF_KEYS } },
    writes: { '/api/staff/mcp/keys': { key: 'example-key-shown-once' } } },
  W2: { start: 'table', after: {}, answers: ROOM },
  W5: { start: 'table', after: {}, answers: ROOM },
  W6: { start: 'guest', after: {}, answers: ROOM },
  // W7: a wallet payment in euros, so the rate, the fill button and the wallet code are on screen
  W7: { start: 'table', after: {}, answers: ROOM, pick: { 3: 'wallet', 4: 'EUR' } },
  W8: { start: 'table', after: {}, answers: ROOM },
  // W12: the mic "hears" a payment; the hub's answer is the read-back the waiter confirms (room/voice.js propose)
  W12: { start: 'table', after: {}, answers: ROOM, speech: 'table 4 paid cash',
    writes: { '/api/voice': { understood: true, needsConfirmation: true, verb: 'pay', table: '4', amount: 2580,
      readback: 'Table 4 paid cash', token: 'staged-voice' } } },
  // G1: "two salmon rolls" is matched against the venue's own menu, in the page (store/voice-order.js)
  G1: { speech: 'two QA Salmon roll' },
  G5: { start: 'booked', after: {}, answers: BOOKED, writes: { [BOOKINGS]: { id: BOOKING_ID, access_token: 'staged-guest-token', status: 'REQUESTED' } } },
  G3: { patch: { [MENU]: ALL_ON }, local: { dw_addrs: SAVED } },
  G3b: { patch: { [MENU]: ALL_ON } },
};

export const stageFor = id => STAGES[id] || null;

/// The select a stage sets before step `n` ([selector, value]), or null.
export const chooseFor = (stage, n) => (stage && stage.choose && stage.choose[n]) || null;

/// The state after step `n` is acted on (unchanged when the stage names none for it).
export const stateAfter = (stage, n, state) => (stage && stage.after && stage.after[n]) || state;

const READS = new Set(['GET', 'HEAD']);
/// capture-lib.mjs's AUTH: the sign-in and refresh routes (repeated, not imported: this module stays leaf).
const AUTH = /^\/api\/(auth\/(login|refresh)|staff\/login|courier\/(auth\/)?login)$/;
/// The answer the browser gets for one /api/ request in `state`, or null (the request goes on
/// to the venue, through the write guard). `ctx.since` is when `state` was first served.
///   { status, body }            a staged read, or a staged lesson's own write ({ ok: true })
///   { status: 0, offline: true } the network "never carried" this write: the caller aborts it
///   { status: 0, patch }        fetch the venue's answer and merge `patch` into it
export function stagedAnswer(stage, state, method, path, ctx = { since: T0 }) {
  if (!stage) return null;
  if (READS.has(method)) {
    const a = stage.answers && stage.answers[state] && stage.answers[state][path];
    if (a !== undefined) return { status: 200, body: typeof a === 'function' ? a(ctx) : a };
    if (stage.patch && stage.patch[path]) return { status: 0, patch: stage.patch[path] };
    return null;
  }
  if (method === 'OPTIONS') return null;
  // Only a lesson whose screens are staged fakes its writes; a patched one (the storefront)
  // still sends nothing: capture's guard aborts its writes as before.
  const w = stage.writes && stage.writes[path];
  // A sign-in or a token refresh the stage does not name goes to the venue as always (the guard
  // lets it through): a faked refresh would sign the recording out.
  if (!w && AUTH.test(path)) return null;
  if (w === 'offline') return { status: 0, offline: true };
  if (w === 'session') return { status: 200, body: ctx.session || { ok: true, staged: true } };
  if (w) return { status: 200, body: w };
  return stage.answers ? { status: 200, body: { ok: true, staged: true } } : null;
}

/// The taps an `outbox` stage finds waiting (addInitScript: serialised, nothing from this scope).
/// The database, store and entry shape are lib/outbox.js's own (DB_NAME, DB_VERSION, STORE, queue()).
export function outboxShim(arg) {
  const rows = Array.isArray(arg) ? arg : arg.rows, db = (!Array.isArray(arg) && arg.db) || 'dowiz.outbox';
  try {
    const req = indexedDB.open(db, 1);
    req.onupgradeneeded = () => { if (!req.result.objectStoreNames.contains('queue')) req.result.createObjectStore('queue', { keyPath: 'seq', autoIncrement: true }); };
    req.onsuccess = () => {
      const tx = req.result.transaction('queue', 'readwrite'), st = tx.objectStore('queue');
      st.clear();
      rows.forEach((r, i) => st.add({ route: r.route, method: 'POST', body: null, key: 'staged-' + i, queued_at: Date.now() - 60000, tries: 0, tag: r.tag, last_status: 0, last_error: '' }));
      tx.oncomplete = () => req.result.close();
    };
  } catch { /* no IndexedDB: the lesson shows no tag, and the probe says ABSENT */ }
}

/// The recogniser a `speech` stage puts in the page (Playwright addInitScript, so it is
/// serialised: nothing from this module's scope). start() "hears" `phrase` a moment later, as
/// one final result, then ends -- the shape lib/voice.js reads off the Web Speech API.
export function speechShim(phrase) {
  class Heard {
    constructor() { this.lang = 'en-GB'; this.onresult = null; this.onerror = null; this.onend = null; }
    start() {
      setTimeout(() => {
        const res = [{ transcript: phrase, confidence: 0.92 }]; res.isFinal = true;
        this.onresult && this.onresult({ results: [res], resultIndex: 0 });
        this.onend && this.onend();
      }, 700);
    }
    stop() { this.onend && this.onend(); }
    abort() {}
  }
  window.SpeechRecognition = Heard; window.webkitSpeechRecognition = Heard;
}

/// Deep merge of plain objects (arrays and scalars are replaced): the patch over the real answer.
export function merge(base, patch) {
  if (!patch || typeof patch !== 'object' || Array.isArray(patch)) return patch;
  const out = base && typeof base === 'object' && !Array.isArray(base) ? { ...base } : {};
  for (const [k, v] of Object.entries(patch)) out[k] = merge(out[k], v);
  return out;
}
