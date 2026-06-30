import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { MockProvider } from '../mock-provider.js';
import type { MenuContext } from '../matcher.js';
import { ConfirmationGate } from '../confirmation-gate.js';
import type { VoiceHandlers } from '../confirmation-gate.js';

// Integration (council voice-control / ADR-0015 §6 + §10): scripted transcripts → SAME matcher →
// real ConfirmationGate. This is the deterministic intent→action path the Playwright spec mirrors.
// RED if a dietary-named category reaches the menu by voice, or a STATEFUL add applies without confirm.

const MENU: MenuContext = {
  products: [
    { id: 'p-sufllaqe', name: 'Sufllaqe' },
    { id: 'p-greek', name: 'Greek Salad' },
  ],
  categories: [
    { id: 'c-pizza', name: 'Pizza' },
    { id: 'c-glutenfree', name: 'Pa gluten' },
  ],
};

function makeSpies(): { calls: Record<string, number>; handlers: VoiceHandlers } {
  const calls: Record<string, number> = {};
  const bump = (k: string) => () => {
    calls[k] = (calls[k] ?? 0) + 1;
  };
  const handlers: VoiceHandlers = {
    addToCart: bump('addToCart'),
    setSort: bump('setSort'),
    setMacroLens: bump('setMacroLens'),
    selectCategory: bump('selectCategory'),
    setSearch: bump('setSearch'),
    toggleCompare: bump('toggleCompare'),
    readOrder: bump('readOrder'),
    navigateCheckout: bump('navigateCheckout'),
  };
  return { calls, handlers };
}

describe('voice MockProvider → ConfirmationGate integration', () => {
  it('READ_ONLY intents auto-apply; a dietary category is REJECTed end-to-end', async () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    const provider = new MockProvider(
      ['rendit sipas çmimit', 'trego kategorinë pizza', 'trego kategorinë pa gluten', 'shko te arka'],
      'sq',
      MENU,
    );

    const statuses: string[] = [];
    for await (const proposal of provider.intents()) {
      statuses.push(gate.submit(proposal).status);
    }

    assert.equal(calls.setSort, 1, 'sort applied');
    assert.equal(calls.navigateCheckout, 1, 'checkout navigation applied');
    assert.equal(calls.selectCategory, 1, 'ONLY the safe category applied (Pizza), never "Pa gluten"');
    assert.ok(statuses.includes('rejected'), 'the dietary category produced a rejection');
  });

  it('a voiced ADD is held pending and applies ONLY after an explicit confirm', async () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    const provider = new MockProvider(['shto dy sufllaqe'], 'sq', MENU);

    for await (const proposal of provider.intents()) {
      const r = gate.submit(proposal);
      assert.equal(r.status, 'pending-confirm');
    }
    assert.equal(calls.addToCart, undefined, 'NOT applied on submit');
    assert.notEqual(gate.pending, null);

    gate.confirm(); // the human taps the confirm chip
    assert.equal(calls.addToCart, 1, 'applied once, after confirm');
  });
});
