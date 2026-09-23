// THE GATE'S OWN PROOF. Runs `conservation.mjs` against a stubbed platform and
// checks that each law goes red when it should and green when it should not.
//
// WHY THIS EXISTS AND NOT A LIVE CORRUPTION. This repo keeps a catalogue of
// instruments that measured nothing, and the rule that came out of it is that
// a gate is triggered before it is trusted. Laws 6 and 7 are about a corrupted
// record and an edited ledger — the two things nobody may go and do to a live
// venue to see whether the alarm works. So the platform is stubbed instead:
// `fetch` answers with the shapes `/api/owner/health` and
// `/api/owner/backup/cloud` really return, and the gate is run unmodified.
//
// THE GREEN CASES ARE HALF THE PROOF. A check that refuses everything is not a
// check, and "absent" must read as NOT MEASURED rather than as zero, or the
// first deployment that has not run its nightly yet turns the gate red and the
// gate gets switched off.
//
//   node e2e/gates/conservation.prove.mjs
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const GATE = path.join(HERE, 'conservation.mjs');
const NIGHT = 26 * 3600e3;
const hex = (c) => c.repeat(64);

const ok = { images: {}, orders: 6, events: 6, quarantined: [] };

// ── law 10's fixtures ──
const T = 1790000000000;
const period = (over) => ({
  till_id: 'main', opened_at: T, opened_by: 'p1', closed_at: T + 100, closed_by: 'p1', counted_at: T + 90,
  ...over,
});
const tillHealth = (over = {}) => ({
  open: false,
  outside: [],
  periods: [period({
    float: { ALL: 10000, EUR: 5000 },
    cash_paid: { ALL: 1500, EUR: 2000 },
    pay_in: { ALL: 4500 },
    pay_out: { EUR: 1000 },
    counted: { ALL: 15900, EUR: 6000 },
    over_short: { ALL: -100, EUR: 0 },
    ...over,
  })],
});
// Orders carry no tax block and a total their lines make, so laws 3 and 9
// stay quiet and only law 10 is under test.
const cashOrder = (id, at, amount, extra = {}) => ({
  id, total: 5000, items: [{ price: 5000, quantity: 1 }],
  payments: [{ method: 'cash', amount, at, ...extra }],
});
const tillOrders = () => [
  cashOrder('r1', T + 10, 1500),
  cashOrder('r2', T + 20, 2000, { currency: 'EUR', rate_ppm: 975000, amount_in_order_currency: 1950 }),
  { id: 'r5', total: 5000, items: [{ price: 5000, quantity: 1 }], payments: [{ method: 'card', amount: 900, at: T + 40 }] },
];

const witnessed = (over) => ({
  configured: true,
  witness: { atMs: Date.now() - NIGHT, records: 6, archived: 0, total: 6, tip: 'aa', found: [], ...over },
});

// Each case: what the two endpoints answer, and the sentence the gate must say.
const CASES = {
  healthy: { health: ok, backup: witnessed(), red: false },

  'quarantined record': {
    health: { ...ok, events: 5, quarantined: [{ image: 'log', id: hex('a'), at: 0, reason: 'kind' }] },
    backup: witnessed(),
    red: 'unreadable (kind)',
  },
  'quarantine in another image': {
    health: { ...ok, quarantined: [{ image: 'audit', id: hex('c'), at: 3, reason: 'subject-framing' }] },
    backup: witnessed(),
    red: 'audit record',
  },
  'the log does not add up': {
    health: { ...ok, events: 4, quarantined: [{ image: 'log', id: hex('a'), at: 0, reason: 'kind' }] },
    backup: witnessed(),
    red: 'claims 6 records, serves 4, withholds 1',
  },

  contradicted: {
    health: ok,
    backup: witnessed({ found: ['the history shrank: 20 records on 1789808707000, 16 now'] }),
    red: 'the history shrank',
  },
  'the nightly has stopped': {
    health: ok,
    backup: witnessed({ atMs: Date.now() - 5 * 24 * 3600e3 }),
    red: 'the nightly has stopped',
  },
  // ABSENT IS NOT ZERO, in both directions: a venue whose nightly has not run
  // since the witness shipped, and a deployment older than the `events` field.
  'no census yet': { health: ok, backup: { configured: true }, red: false },
  'a worker without the new fields': {
    health: { images: {}, orders: 6 },
    backup: { configured: true },
    red: false,
  },

  // ── LAW 3 ──
  //
  // IT HAD NO PROVE CASE AT ALL: the stub answered `[]` for the order list, so
  // the money law was never exercised in either direction. That is how it kept
  // a missing tip term — `total` is `subtotal + fee + tip` and the law
  // expected `lines + fee - discount`, so the first tipped order would have
  // been reported as a breach. Measured before the fix: 0 of 52 live orders
  // carried a tip, which is the only reason it had never fired.
  'a tipped order adds up': {
    health: ok,
    backup: witnessed(),
    orders: [{ id: 'ord_t', total: 1800, tip: 300, delivery_fee: 200,
               items: [{ price: 650, quantity: 2 }] }],
    red: false,
  },
  'a total that does not add up': {
    health: ok,
    backup: witnessed(),
    orders: [{ id: 'ord_w', total: 9999, tip: 0, delivery_fee: 200,
               items: [{ price: 650, quantity: 2 }] }],
    red: 'total 9999',
  },
  // A DISCOUNT COMES OFF, A TIP GOES ON, and getting the two signs the same
  // way round is the arithmetic this law exists to check.
  'a discounted and tipped order adds up': {
    health: ok,
    backup: witnessed(),
    orders: [{ id: 'ord_d', total: 1500, tip: 300, delivery_fee: 200, discount: 300,
               items: [{ price: 650, quantity: 2 }] }],
    red: false,
  },

  // ── LAW 8 ──
  //
  // The two halves it can find, and the two states it must stay quiet about.
  'a projection that differs from the log': {
    health: { ...ok, rebuild: { intact: false, orders: 6, stale: ['ord_7'], stranded: [], unheld: [], modelled: true } },
    backup: witnessed(),
    red: 'differs from a fresh fold',
  },
  'stock held for an order that has ended': {
    health: { ...ok, rebuild: { intact: false, orders: 6, stale: [], stranded: ['ord_9'], unheld: [], modelled: true } },
    backup: witnessed(),
    red: 'an order the log says has ended',
  },
  'the rebuild could not run': {
    health: { ...ok, rebuild: { intact: false, error: 'hub refused a rebuild: 503' } },
    backup: witnessed(),
    red: 'the rebuild could not run',
  },
  // A VENUE THAT MODELS NO RECIPES IS THE NORMAL STATE OF THIS PRODUCT -- both
  // live venues are in it -- and `unheld` must never be a breach, or the gate
  // is red for every venue that has not switched stock control on.
  'a venue with no recipes owes no reservations': {
    health: { ...ok, rebuild: { intact: true, orders: 6, stale: [], stranded: [], unheld: ['ord_1', 'ord_2'], modelled: false } },
    backup: witnessed(),
    red: false,
  },
  // ABSENT IS NOT INTACT, but it is not a breach either: a deployment older
  // than this field has not been measured. Same rule as `events` above.
  'a worker without the rebuild field': { health: ok, backup: witnessed(), red: false },

  // ── LAW 9: TAX CONSERVATION ──
  //
  // The tax block must satisfy four equations: (a) group taxes sum to total,
  // (b) allocated discounts sum to the order discount, (c) bases and totals
  // add up correctly for the venue's inclusive flag, (d) the stored tax can
  // be recomputed from base and rate.
  //
  // A healthy tax block: one group at 20% inclusive, base 750, tax 112, fee
  // at same rate (base 300, tax 50), tip 100, no discount allocated.
  // Expected total (inclusive): 750 + 300 + 100 = 1150.
  'a valid tax block (inclusive, one group, with fee and tip)': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_ok',
      total: 1150,
      tip: 100,
      delivery_fee: 300,
      discount: 0,
      items: [{ price: 750, quantity: 1 }],
      tax: {
        inclusive: true,
        groups: [{ rate_ppm: 200000, base: 750, tax: 125, lines: 1 }],
        fee: { rate_ppm: 200000, base: 300, tax: 50 },
        total: 175,
        discount_allocated: [],
      },
    }],
    red: false,
  },

  // THE KERNEL'S ROUNDING, on the one kind of input where half-up has two
  // readings: an odd denominator (1e6 + 200001). eqc_gen's
  // apply_tax_inclusive_int gives tax 80001 on 480001; Math.round(b/2) gave
  // 80000 and would have called this correct receipt a breach.
  'a valid tax block (inclusive, odd rate: the kernel rounds b/2 down)': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_odd', total: 480001, tip: 0, delivery_fee: 0, discount: 0,
      items: [{ price: 480001, quantity: 1 }],
      tax: {
        inclusive: true,
        groups: [{ rate_ppm: 200001, base: 480001, tax: 80001, lines: 1 }],
        fee: null, total: 80001, discount_allocated: [],
      },
    }],
    red: false,
  },

  // (a) GROUP TAXES DO NOT SUM TO TOTAL
  'tax block: group taxes do not sum to total': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_a',
      total: 1150,
      tip: 100,
      delivery_fee: 300,
      discount: 0,
      items: [{ price: 750, quantity: 1 }],
      tax: {
        inclusive: true,
        groups: [{ rate_ppm: 200000, base: 750, tax: 125, lines: 1 }],
        fee: { rate_ppm: 200000, base: 300, tax: 50 },
        total: 200, // WRONG: should be 125 + 50 = 175
        discount_allocated: [],
      },
    }],
    red: 'sum to 175, but tax.total is 200',
  },

  // (b) ALLOCATED DISCOUNTS DO NOT SUM TO ORDER DISCOUNT
  'tax block: allocated discounts do not sum to order discount': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_b',
      total: 1075,
      tip: 100,
      delivery_fee: 300,
      discount: 75,
      items: [{ price: 750, quantity: 1 }],
      tax: {
        inclusive: true,
        groups: [{ rate_ppm: 200000, base: 675, tax: 112, lines: 1 }],
        fee: { rate_ppm: 200000, base: 300, tax: 50 },
        total: 162,
        discount_allocated: [50], // WRONG: should be [75]
      },
    }],
    red: 'allocated discounts sum to 50, but order discount is 75',
  },

  // (c) BASES AND TOTALS DO NOT ADD UP (INCLUSIVE)
  'tax block: bases do not sum to total (inclusive venue)': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_c',
      total: 9999, // WRONG: should be 750 + 300 + 100 = 1150
      tip: 100,
      delivery_fee: 300,
      discount: 0,
      items: [{ price: 750, quantity: 1 }],
      tax: {
        inclusive: true,
        groups: [{ rate_ppm: 200000, base: 750, tax: 125, lines: 1 }],
        fee: { rate_ppm: 200000, base: 300, tax: 50 },
        total: 175,
        discount_allocated: [],
      },
    }],
    red: 'bases and tax expected total 1150, but order total is 9999',
  },

  // (d) STORED TAX DOES NOT MATCH RECOMPUTED TAX
  'tax block: stored tax does not match recomputed tax': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_d',
      total: 1150,
      tip: 100,
      delivery_fee: 300,
      discount: 0,
      items: [{ price: 750, quantity: 1 }],
      tax: {
        inclusive: true,
        groups: [{ rate_ppm: 200000, base: 750, tax: 999, lines: 1 }], // WRONG tax
        fee: { rate_ppm: 200000, base: 300, tax: 50 },
        total: 1049, // Also wrong total as a result
        discount_allocated: [],
      },
    }],
    red: 'recomputed tax 125, but stored tax is 999',
  },

  // GREEN: inclusive venue with two rate groups and discount
  'a valid tax block (multiple groups with discount)': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_tax_multi',
      total: 1100, // 450 (base group 20%) + 225 (base group 10%) + 300 (fee) + 125 (tip) = 1100
      tip: 125,
      delivery_fee: 300,
      discount: 75,
      items: [
        { price: 500, quantity: 1 }, // 20% group
        { price: 250, quantity: 1 }, // 10% group
      ],
      tax: {
        inclusive: true,
        groups: [
          { rate_ppm: 100000, base: 225, tax: 20, lines: 1 }, // 250 - 25 discount = 225, 10% tax = 20
          { rate_ppm: 200000, base: 450, tax: 75, lines: 1 }, // 500 - 50 discount = 450, 20% tax = 75
        ],
        fee: { rate_ppm: 200000, base: 300, tax: 50 },
        total: 145, // 20 + 75 + 50
        discount_allocated: [50, 25], // allocated across two groups
      },
    }],
    red: false,
  },

  // GREEN: no tax block (order placed before tax was configured)
  'an order without a tax block': {
    health: ok,
    backup: witnessed(),
    orders: [{
      id: 'ord_no_tax',
      total: 1150,
      tip: 100,
      delivery_fee: 300,
      discount: 0,
      items: [{ price: 750, quantity: 1 }],
      // no tax block
    }],
    red: false,
  },

  // ── LAW 10: THE TILL, PER CURRENCY ──
  //
  // One period opened at T with a lek and a euro float; two cash payments in
  // it (1500 lek, €20.00 = 2000 cents at a rate -- the drawer counts the euro,
  // not the lek it was worth), a 4500-lek courier hand-in, a €10 pay-out.
  //   ALL: 10000 + 1500 + 4500        = 16000, counted 15900 -> -100
  //   EUR:  5000 + 2000        - 1000 =  6000, counted  6000 ->    0
  'a balanced till in two currencies': {
    health: { ...ok, till: tillHealth() },
    backup: witnessed(),
    orders: tillOrders(),
    red: false,
  },
  'a balanced till in one currency': {
    health: { ...ok, till: { open: false, outside: [], periods: [period({
      float: { ALL: 10000 }, cash_paid: { ALL: 1500 }, pay_in: {}, pay_out: {},
      counted: { ALL: 11500 }, over_short: { ALL: 0 } })] } },
    backup: witnessed(),
    orders: [cashOrder('r1', T + 10, 1500)],
    red: false,
  },
  // OFF BY 100 IN ONE CURRENCY: the euro pile's recorded over/short says 0
  // and the parts say -100. The lek pile balances; the gate names the euro.
  'a till off by 100 in one currency': {
    health: { ...ok, till: tillHealth({ counted: { ALL: 15900, EUR: 5900 } }) },
    backup: witnessed(),
    orders: tillOrders(),
    red: 'EUR: counted 5900 - (float 5000 + cash 2000 + in 0 - out 1000 = 6000) is -100, recorded over/short 0',
  },
  // A CASH PAYMENT THE TILL NEVER FOLDED: the orders hold a third cash
  // payment inside the period that the till's cash_paid does not include.
  'a cash payment missing from the till': {
    health: { ...ok, till: tillHealth() },
    backup: witnessed(),
    orders: [...tillOrders(), cashOrder('r3', T + 30, 700)],
    red: 'the orders hold 2200 cash, the till folded 1500',
  },
  'cash taken with no till open': {
    health: { ...ok, till: tillHealth() },
    backup: witnessed(),
    orders: [...tillOrders(), cashOrder('r4', T - 5000, 700)],
    red: 'r4: 700 ALL cash at',
  },
  // AN OPEN DRAWER IS STILL TAKING MONEY: its numbers are not balanced yet,
  // however they look.
  'an open till is not balanced yet': {
    health: { ...ok, till: { open: true, outside: [], periods: [period({
      closed_at: null, counted: { ALL: 1 }, over_short: null })] } },
    backup: witnessed(),
    orders: tillOrders(),
    red: false,
  },
  'a till that could not fold': {
    health: { ...ok, till: { error: '500: the till log does not fold: till record 3: unknown kind till.bribe' } },
    backup: witnessed(),
    orders: tillOrders(),
    red: 'the till could not be folded',
  },
  // ABSENT IS NOT ZERO: a Worker without the till block has not measured it,
  // and must not call every cash payment "outside".
  'a worker without the till field': {
    health: ok,
    backup: witnessed(),
    orders: tillOrders(),
    red: false,
  },
};

const RUNNER = `
globalThis.fetch = async (url) => {
  const body = url.includes('/api/auth/login')
    ? { access_token: 't', user: { locationId: 'stub-venue' } }
    : url.includes('/api/owner/health') ? HEALTH
    : url.includes('/api/owner/backup/cloud') ? BACKUP
    : url.includes('/api/owner/orders') ? ORDERS
    : {};
  return { status: 200, text: async () => JSON.stringify(body) };
};
await import(GATE);
`;

let failed = 0;
for (const [name, c] of Object.entries(CASES)) {
  const script = RUNNER
    .replace('HEALTH', JSON.stringify(c.health))
    .replace('ORDERS', JSON.stringify(c.orders || []))
    .replace('BACKUP', JSON.stringify(c.backup))
    .replace('GATE', JSON.stringify(GATE));
  const r = spawnSync(process.execPath, ['--input-type=module', '-e', script], {
    encoding: 'utf8',
    env: { ...process.env, HOSTS: 'stub' },
  });
  const out = (r.stdout || '') + (r.stderr || '');
  const red = r.status === 1;
  const want = c.red !== false;
  let verdict = red === want ? 'ok' : `WRONG COLOUR (exit ${r.status})`;
  if (verdict === 'ok' && want && !out.includes(c.red)) {
    verdict = `red, but did not say "${c.red}"`;
  }
  if (verdict !== 'ok') {
    failed += 1;
    console.log(`  ${name}: ${verdict}\n${out.split('\n').map((l) => `    | ${l}`).join('\n')}`);
  } else {
    console.log(`  ${name}: ${want ? 'red' : 'green'}, as it should be`);
  }
}

if (failed) {
  console.log(`\nTHE GATE DOES NOT FIRE AS DESCRIBED — ${failed} case(s).`);
  process.exit(1);
}
console.log('\nEVERY LAW FIRES, AND ONLY WHEN IT SHOULD.');
