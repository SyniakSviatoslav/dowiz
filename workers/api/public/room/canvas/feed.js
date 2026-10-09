// THE BOARD'S FEED, pure (feed.test.mjs): the hub's reads as the lines src/board/feed.rs reads;
// money formatted here (logic.js -> lib/money.js), never in Rust. ASCII QUOTES ONLY (rule 11).
import { parseCaps, sittingDue, money, guestWaiting } from '../logic.js';

const STATION = { sushi: 1, kitchen: 2, bar: 3 };
export const ROLE = { waiter: 0, 'counter-manager': 1, kitchen: 2, owner: 3 };
export const BUMPS = ['confirm', 'preparing', 'ready', 'collected'];

export const clean = v => String(v ?? '').replace(/[\t\n\r]+/g, ' ');

export function toFeed({ orders = [], sittings = [], currency = null, locale = 'sq', now = Date.now(), when = () => '' } = {}) {
  const out = [];
  for (const o of orders || []) {
    if (!o || !o.id) continue;
    const f = o.fulfilment || {};
    const sched = Number(o.scheduled_for_ms) || 0;
    const startMs = sched > 0 ? sched : Number(o.created_at_ms) || 0;
    const kind = f.table ? 't' : f.kind === 'delivery' ? 'd' : 'p';
    out.push(['T', o.id, o.status, Math.trunc(startMs), o.kitchen && o.kitchen.seen ? 1 : 0, kind, f.table, f.note, sched > now ? when(sched) : ''].map(clean).join('\t'));
    for (const l of o.items || []) {
      out.push(['L', Math.max(0, Number(l.quantity) | 0), STATION[l.station] || 2, l.name || l.product_id, l.note].map(clean).join('\t'));
    }
  }
  for (const s of sittings || []) {
    const rounds = s.rounds || [];
    out.push(['R', s.sitting_id, s.table, rounds.length, money(sittingDue(s), currency, locale),
      rounds.some(guestWaiting) ? 1 : 0, rounds.map(r => r.status).join(',')].map(clean).join('\t'));
  }
  return out.join('\n') + '\n';
}

const WORDS = ('Live Offline Polling LoginLine Email Password ClaimCode SignIn Claim HaveCode HaveAccount ColNew ColPreparing ' +
  'ColReady Tables All StSushi StKitchen StBar KTable KPickup KDelivery KMin BumpConfirm BumpPreparing BumpReady BumpCollected KSeen ' +
  'KUnseen NoTickets NoTables NoAccess Rounds Due Refresh SignOut Saved Error Loading More Waiter CounterManager Kitchen Owner ' +
  'GuestWaiting Room ForTime StopReject StopCancel KReason KReasonHint KReasonNeeded Close ThemeSystem ThemeDark ThemeLight Back Send ' +
  'NoLines Subtotal Discount Total Owed AddItem AddN Search NoMatch SoldOut Remove Comp Comped MoveTable Move WhyRemove WhyComp ' +
  'RMistake RGuestChanged RUnavailable RDropped ROther OtherText NeedReason ChangedReload Take TakeN Amount Method Currency Rate ' +
  'MCash MCard MCheque MTransfer MGiftCard MWallet RateNeeded OffTheBill FillOwed Taken PaidInFull BadAmount BadRate BadTip Tip ' +
  'WalletCode NeedWallet WalletNoTip MoveLines MoveLinesTo PickLines PickRound NoTargets NotAllLines MoveSitting MoveSittingHint ' +
  'Moved KitchenHasIt RoundPaid AlreadyThere NotHere GuestRound GuestConfirm GuestConfirmed GuestRejected QueuedSaved QueueFull ' +
  'QueueNoStore MenuFailed NoSlug').split(' ');
export { WORDS };
export const W = name => { const i = WORDS.indexOf(name); return i < 0 ? '' : i; };

export function ages(settings) {
  const v = settings?.values?.['notify.order.late_min'];
  const late = /^\d{1,4}$/.test(String(v ?? '').trim()) ? Number(v) : 0;
  return late >= 2 ? [Math.floor(late / 2), late] : null;
}

export function opens(s) {
  const caps = parseCaps(s?.staff?.caps);
  return [caps.has('advance'), caps.has('take_orders')];
}
