// Domain contract gate — the three kernel domains, over HTTP, against a real hub.
//
// Each check asserts something the KERNEL decides, so a regression in
// `dowiz_kernel::reservation`, `::pass`, `::thread` or `::ledger_account` shows
// up here as well as in `cargo test`. The unit tests prove the law; this proves
// the law is what production actually applies.
//
// It has already caught, against production and nowhere else:
//   * a guest booking writing "" into a column that references `users(id)`;
//   * `i64::into()` binding a JavaScript BigInt, which D1 rejects by throwing —
//     a bare 500 with no body and nothing in the response saying why;
//   * a resent message answering "already used by a different message" because
//     the idempotency check ran AFTER the kernel's, and a resend carries a new
//     timestamp.
//
// It writes to the hub it is pointed at and deletes what it wrote. Point it at a
// venue you are willing to have rows created in.

const HOST = process.env.HOST || 'https://dubin-sushi.dowiz.org';
const SLUG = process.env.SLUG || 'dubin-sushi';
const API = `${HOST}/api/public/locations/${SLUG}`;

const results = [];
function check(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${ok || !detail ? '' : `\n      ${detail}`}`);
}

async function call(path, init) {
  const r = await fetch(API + path, init);
  const text = await r.text();
  let body = null;
  try { body = text ? JSON.parse(text) : null; } catch { /* plain text error */ }
  return { status: r.status, body, text };
}

const post = (path, payload) => call(path, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify(payload),
});

const nowMin = () => Math.floor(Date.now() / 60_000);
const tag = `e2e-${Date.now()}`;

export async function run() {
  // ── Reservations: the FSM refuses what it must ────────────────────────────
  const past = await post('/reservations', {
    party: 2, slotMin: 1000, requestId: `${tag}-past`,
  });
  check('a slot in the past is refused by the kernel',
    past.status === 422 && /before now/.test(past.text), `${past.status} ${past.text}`);

  const huge = await post('/reservations', {
    party: 0, slotMin: nowMin() + 120, requestId: `${tag}-party`,
  });
  check('a party size outside the venue policy is refused',
    huge.status === 422 && /party size/.test(huge.text), `${huge.status} ${huge.text}`);

  const slot = nowMin() + 20;
  const made = await post('/reservations', {
    party: 2, slotMin: slot, occasion: tag, requestId: `${tag}-ok`,
  });
  check('a valid request becomes a reservation',
    made.status === 200 && made.body?.id, `${made.status} ${made.text}`);
  const id = made.body?.id;

  const again = await post('/reservations', {
    party: 2, slotMin: slot, occasion: tag, requestId: `${tag}-ok`,
  });
  check('the same requestId replays instead of booking a second table',
    again.body?.replayed === true && again.body?.id === id, again.text);

  const early = await call(`/reservations/${id}/pass`);
  check('a pass is refused before the venue confirms',
    early.status === 409 && /confirmed/.test(early.text), `${early.status} ${early.text}`);

  const illegal = await post(`/reservations/${id}/action`, { to: 'SEATED' });
  check('a forbidden transition is an error, not a no-op',
    illegal.status === 409 && /illegal/.test(illegal.text), `${illegal.status} ${illegal.text}`);

  const confirm = await post(`/reservations/${id}/action`, { to: 'CONFIRMED', actor: 'VENUE' });
  check('the venue may confirm', confirm.body?.status === 'CONFIRMED', confirm.text);

  const detail = await call(`/reservations/${id}`);
  check('the status is replayed from events, and the cache agrees',
    detail.body?.status === 'CONFIRMED'
    && detail.body?.statusCacheDrifted === false
    && detail.body?.history?.length === 2, detail.text?.slice(0, 200));

  // ── Passes ────────────────────────────────────────────────────────────────
  const issued = await call(`/reservations/${id}/pass`);
  const code = issued.body?.code;
  check('a confirmed booking gets a pass that fits in a QR',
    typeof code === 'string' && code.length === 80, `len=${code?.length}`);

  const good = await post('/pass/verify', { code });
  check('the venue scanner accepts its own pass inside the window',
    good.body?.ok === true, JSON.stringify(good.body));

  const tampered = await post('/pass/verify', { code: code.slice(0, 79) + (code[79] === 'Z' ? 'Y' : 'Z') });
  check('a tampered code is refused by the tag, not by the window',
    tampered.body?.ok === false && /tag/.test(tampered.body?.why || ''),
    JSON.stringify(tampered.body));

  const nonsense = await post('/pass/verify', { code: 'not-a-pass' });
  check('nonsense is refused with a reason rather than a crash',
    nonsense.body?.ok === false && !!nonsense.body?.why, JSON.stringify(nonsense.body));

  // ── The wallet journal ────────────────────────────────────────────────────
  const user = `${tag}-user`;
  const fresh = await call(`/wallet?user=${user}`);
  check('an untouched wallet has no balance, which is not zero',
    fresh.body?.balanceMinor === null && fresh.body?.conserved === true,
    JSON.stringify(fresh.body));

  const noRail = await post('/wallet/topup', {
    user, amountMinor: 1000, currency: 'ALL', providerRef: '', requestId: `${tag}-w0`,
  });
  check('a top-up with no payment behind it is refused',
    noRail.status === 422 && /providerRef/.test(noRail.text), `${noRail.status} ${noRail.text}`);

  const topped = await post('/wallet/topup', {
    user, amountMinor: 12_345, currency: 'ALL',
    providerRef: `${tag}-rail`, requestId: `${tag}-w1`,
  });
  check('a top-up with a rail reference posts', topped.body?.id, topped.text);

  const after = await call(`/wallet?user=${user}`);
  check('the balance is the replay, and the journal still nets to zero',
    after.body?.balanceMinor === 12_345 && after.body?.conserved === true,
    JSON.stringify(after.body));

  const replayTop = await post('/wallet/topup', {
    user, amountMinor: 12_345, currency: 'ALL',
    providerRef: `${tag}-rail`, requestId: `${tag}-w1`,
  });
  const afterReplay = await call(`/wallet?user=${user}`);
  check('a replayed top-up does not move money twice',
    replayTop.body?.replayed === true && afterReplay.body?.balanceMinor === 12_345,
    `${replayTop.text} / ${afterReplay.text}`);

  // ── The delivery estimate ─────────────────────────────────────────────────
  // Each of these asserts that ONE input moves ONE part of the answer. An
  // estimate that does not change with distance, cooking time or the queue is
  // the published-string problem wearing a JSON hat.
  const near = await post('/eta', { items: [{ cookingMin: 8, quantity: 1 }], distanceM: 1000 });
  const far = await post('/eta', { items: [{ cookingMin: 8, quantity: 1 }], distanceM: 8000 });
  check('distance moves the travel leg and nothing else',
    far.body?.parts?.travelMin > near.body?.parts?.travelMin
    && far.body?.parts?.prepMin === near.body?.parts?.prepMin,
    `${JSON.stringify(near.body?.parts)} vs ${JSON.stringify(far.body?.parts)}`);

  const slow = await post('/eta', { items: [{ cookingMin: 45, quantity: 1 }], distanceM: 1000 });
  check("the dish's own cooking time moves the prep leg",
    slow.body?.parts?.prepMin - near.body?.parts?.prepMin === 37,
    `${slow.body?.parts?.prepMin} vs ${near.body?.parts?.prepMin}`);

  const many = await post('/eta', { items: [{ cookingMin: 8, quantity: 4 }], distanceM: 1000 });
  check('four portions cost more than one but not four times more',
    many.body?.parts?.prepMin > near.body?.parts?.prepMin
    && many.body?.parts?.prepMin < near.body?.parts?.prepMin * 4,
    `${many.body?.parts?.prepMin} vs ${near.body?.parts?.prepMin}`);

  const collected = await post('/eta', { items: [{ cookingMin: 8, quantity: 1 }], pickup: true });
  check('pickup drops the journey and both handovers',
    collected.body?.parts?.travelMin === 0 && collected.body?.parts?.overheadMin === 0
    && collected.body?.lowMin < near.body?.lowMin,
    JSON.stringify(collected.body?.parts));

  const emptyBasket = await post('/eta', { items: [] });
  check('an empty basket has no estimate', emptyBasket.status === 400, emptyBasket.text);

  const twice = await post('/eta', { items: [{ cookingMin: 8, quantity: 1 }], distanceM: 1000 });
  check('the same inputs give the same answer — the estimate reads no clock',
    JSON.stringify(twice.body) === JSON.stringify(near.body),
    `${JSON.stringify(near.body)} vs ${JSON.stringify(twice.body)}`);

  // ── Threads ───────────────────────────────────────────────────────────────
  const noThread = await post('/threads/does-not-exist/messages', {
    from: 'CUSTOMER', body: 'x', clientId: `${tag}-m0`,
  });
  check('a message to a thread that does not exist is refused',
    noThread.status === 404, `${noThread.status} ${noThread.text}`);

  const badParty = await post('/threads/does-not-exist/messages', {
    from: 'ADMIN', body: 'x', clientId: `${tag}-m1`,
  });
  check('an unknown party is refused before anything is looked up',
    badParty.status === 400 && /unknown party/.test(badParty.text), badParty.text);

  const failures = results.filter(r => !r.ok).length;
  console.log(failures
    ? `\n${failures} of ${results.length} FAILED`
    : `\nall ${results.length} domain contracts hold`);
  console.log(`\nrows written under the tag ${tag} — remove them with:`);
  console.log(`  wrangler d1 execute dowiz --remote --command "DELETE FROM reservation_events ` +
              `WHERE reservation_id IN (SELECT id FROM reservations WHERE occasion='${tag}'); ` +
              `DELETE FROM reservations WHERE occasion='${tag}'; ` +
              `DELETE FROM ledger_postings WHERE tx_id IN (SELECT id FROM ledger_tx WHERE memo LIKE '${tag}%'); ` +
              `DELETE FROM ledger_tx WHERE memo LIKE '${tag}%';"`);
  return failures;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exit(await run());
}
