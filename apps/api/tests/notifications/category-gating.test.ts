import test from 'node:test';
import assert from 'node:assert/strict';
import {
  getEventCategory,
  isSuppressedByCategory,
} from '../../src/notifications/event-registry.js';

// TG_CATEGORY_GATING gating logic (docs/design/telegram-notifications-actions/).
// Category = reversibility of consequence, not loudness. The ETHICAL invariant is the
// load-bearing assertion here: every irreversible-in-quiet-window event MUST stay
// transactional, i.e. never suppressible by a toggle.
test('notification category gating', async (t) => {
  await t.test('irreversible events are transactional and never suppressed', () => {
    const transactional = [
      'order.created',
      'order.timeout_cancelled',
      'cash.reconcile_discrepancy',
      'delivery.flag_raised',
      'order.substitution_needs_human',
      'order.pending_aging', // signed transactional at STOP-ETHICS-1
      'ops.worker_liveness',
      'ops.backup_failed',
      // registered in EVENT_REGISTRY but absent from OPERATIONAL/QUALITY sets —
      // pin its transactional classification so a reclassification can't pass silently.
      'ops.degradation_changed',
    ];
    for (const ev of transactional) {
      assert.equal(getEventCategory(ev), 'transactional', `${ev} must be transactional`);
      // Even with BOTH toggles explicitly off, a transactional event is never suppressed.
      assert.equal(
        isSuppressedByCategory(ev, { operational: false, quality: false }),
        false,
        `${ev} must always send`,
      );
    }
  });

  await t.test('operational defaults ON, honours explicit toggle', () => {
    // Whole OPERATIONAL_EVENTS set is operational — not just shift.started.
    for (const ev of ['shift.started', 'shift.closed', 'shift.close_reminder']) {
      assert.equal(getEventCategory(ev), 'operational', `${ev} must be operational`);
    }
    // default ON: absent prefs => not suppressed
    assert.equal(isSuppressedByCategory('shift.started', {}), false);
    assert.equal(isSuppressedByCategory('shift.started', null), false);
    // explicit ON => not suppressed; explicit OFF => suppressed
    assert.equal(isSuppressedByCategory('shift.started', { operational: true }), false);
    assert.equal(isSuppressedByCategory('shift.started', { operational: false }), true);
  });

  await t.test('quality defaults OFF, honours explicit toggle', () => {
    assert.equal(getEventCategory('rating.low_received'), 'quality');
    // default OFF: absent prefs => suppressed
    assert.equal(isSuppressedByCategory('rating.low_received', {}), true);
    assert.equal(isSuppressedByCategory('rating.low_received', null), true);
    // explicit ON => not suppressed (low rating always sent when quality ON)
    assert.equal(isSuppressedByCategory('rating.low_received', { quality: true }), false);
    assert.equal(isSuppressedByCategory('rating.low_received', { quality: false }), true);
  });

  await t.test('unknown events fail safe to transactional', () => {
    assert.equal(getEventCategory('some.unmapped.event'), 'transactional');
    assert.equal(isSuppressedByCategory('some.unmapped.event', { operational: false, quality: false }), false);
  });

  await t.test('category toggles are independent', () => {
    // operational ON while quality OFF: shift sends, low-rating does not
    const prefs = { operational: true, quality: false };
    assert.equal(isSuppressedByCategory('shift.started', prefs), false);
    assert.equal(isSuppressedByCategory('rating.low_received', prefs), true);
  });
});
