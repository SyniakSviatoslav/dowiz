import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  BootRatchet,
  DegradeFlagStore,
  resetBootFlags,
  DEGRADE_FLAG_KEYS,
  type FlagBackend,
} from '../src/lib/reliability/ratchet.js';

// In-memory flag backend simulating a PERSISTED store — the exact vector that
// let trap #15 re-arm on every boot (a prior run left flags=true; the buggy
// boot READ them instead of resetting).
class InMemoryFlagBackend implements FlagBackend {
  private flags: Record<string, boolean> = {};
  constructor(seed?: Record<string, boolean>) {
    if (seed) this.flags = { ...seed };
  }
  read() {
    return { ...this.flags };
  }
  write(f: Record<string, boolean>) {
    this.flags = { ...f };
  }
}

// ---- restart-regression (trap #15) ------------------------------------------
// A fresh boot MUST start with every non-money surface clean — the persisted
// flag trap is disarmed. RED on the buggy boot (flags persisted, read-not-reset).
test('restart-regression #15: degrade flags RESET on restart', () => {
  const backend = new InMemoryFlagBackend();
  // Simulate the prior run leaving every surface degraded (the armed trap):
  for (const k of DEGRADE_FLAG_KEYS) backend.write({ ...backend.read(), [k]: true });
  const store = new DegradeFlagStore(backend);

  const before = store.readAll();
  assert.ok(
    DEGRADE_FLAG_KEYS.every((k) => before[k] === true),
    'precondition: flags were persisted as degraded (trap armed)',
  );

  // THE FIX: a fresh boot resets all flags before any health cycle.
  resetBootFlags(store);

  const after = store.readAll();
  assert.ok(
    DEGRADE_FLAG_KEYS.every((k) => after[k] === false),
    'all surfaces clean after restart — trap disarmed',
  );
});

// ---- boot-grace -------------------------------------------------------------
// No persistent degrade decision may be taken before the first successful
// health cycle (or a bounded grace deadline). RED on buggy boot (degrade fires
// immediately during boot).
test('boot-grace: degrade SUPPRESSED during grace window, fires after first OK', async () => {
  const alerts: any[] = [];
  const r = new BootRatchet({
    alertSink: { publish: async (c, m) => { alerts.push({ c, m }); } },
  });

  assert.equal(r.inBootGrace, true, 'starts in boot grace');

  const suppressed = await r.degrade('S1', 'rust unhealthy during boot');
  assert.equal(suppressed, false, 'degrade suppressed during boot grace');
  assert.equal(alerts.length, 0, 'no alert while suppressed (no flag writes)');

  // First successful upstream probe exits the grace window.
  r.recordOk();
  assert.equal(r.inBootGrace, false, 'grace exits after first OK');

  const fired = await r.degrade('S1', 'rust unhealthy after boot');
  assert.equal(fired, true, 'degrade fires once grace has passed');
  assert.equal(alerts.length, 1, 'real alert emitted on genuine degrade');
  assert.equal(alerts[0].c, 'ops.cutover_degrade');
});

test('boot-grace: degrade fires if grace deadline elapses with no OK (no infinite immunity)', async () => {
  const r = new BootRatchet({});
  // Simulate a boot far in the past, beyond the grace deadline.
  (r as any).bootAt = Date.now() - 200_000;
  assert.equal(r.inBootGrace, false, 'grace expired');

  const fired = await r.degrade('S2', 'rust never came up');
  assert.equal(fired, true, 'degrade fires after grace expiry (no silent immunity)');
});

// ---- alert-on-degrade (the real alert, not just a log line) -----------------
test('alert-on-degrade: a genuine degrade emits one bus event with surface+reason', async () => {
  const events: any[] = [];
  const r = new BootRatchet({
    alertSink: { publish: async (channel, msg) => { events.push({ channel, msg }); } },
  });
  r.recordOk(); // out of grace
  await r.degrade('S6', 'order.dispatch relay dead');

  assert.equal(events.length, 1);
  assert.equal(events[0].channel, 'ops.cutover_degrade');
  assert.equal(events[0].msg.surface, 'S6');
  assert.equal(events[0].msg.reason, 'order.dispatch relay dead');
});
