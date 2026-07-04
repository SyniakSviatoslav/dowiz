import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { WhisperProvider } from '../whisper-provider.js';
import type { Transcriber, PcmAudio } from '../transcriber.js';
import type { MenuContext } from '../matcher.js';
import { ConfirmationGate } from '../confirmation-gate.js';
import type { VoiceHandlers } from '../confirmation-gate.js';

// Engine-loop wiring proof (ADR-0015 §6 / H1 CI-deterministic half). A FakeTranscriber makes the
// audio→text step deterministic so this proves ONLY: transcribe→match→yield, fail-quiet on
// non-commands/silence/errors, no-write source, and the SAME gate outcome as the scripted mock.
// It proves NOTHING about real transcription quality — that is the separate real-audio harness.

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

/** Returns scripted transcripts in order — one per transcribe() call. A null entry throws (segment error). */
class FakeTranscriber implements Transcriber {
  #queue: (string | null)[];
  calls = 0;
  constructor(transcripts: (string | null)[]) {
    this.#queue = [...transcripts];
  }
  async transcribe(_audio: PcmAudio): Promise<string> {
    this.calls += 1;
    const next = this.#queue.shift();
    if (next === null) throw new Error('simulated ASR failure on this segment');
    return next ?? '';
  }
}

/** One dummy PCM segment per scripted utterance (content irrelevant to the fake). */
async function* segments(n: number): AsyncIterableIterator<PcmAudio> {
  for (let i = 0; i < n; i++) yield new Float32Array(160); // 10 ms @ 16 kHz — shape only
}

function makeSpies(): { calls: Record<string, number>; handlers: VoiceHandlers } {
  const calls: Record<string, number> = {};
  const bump = (k: string) => () => {
    calls[k] = (calls[k] ?? 0) + 1;
  };
  const handlers: VoiceHandlers = {
    // addToCart reports apply-outcome (council R-a); the spy counts AND reports a real mutation.
    addToCart: () => {
      calls['addToCart'] = (calls['addToCart'] ?? 0) + 1;
      return true;
    },
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

describe('WhisperProvider engine loop', () => {
  it('transcribes each utterance and yields the matched proposal (transcribe→match→yield)', async () => {
    const fake = new FakeTranscriber(['rendit sipas çmimit', 'shko te arka']);
    const engine = new WhisperProvider(fake, 'sq', MENU);
    const kinds: string[] = [];
    for await (const p of engine.intents(segments(2))) kinds.push(p.kind);
    assert.deepEqual(kinds, ['SET_SORT', 'NAVIGATE_CHECKOUT']);
    assert.equal(fake.calls, 2, 'transcriber called once per utterance');
  });

  it('is fail-quiet: silence and non-command speech yield NOTHING', async () => {
    const fake = new FakeTranscriber(['', 'the weather is nice today', 'rendit sipas çmimit']);
    const engine = new WhisperProvider(fake, 'sq', MENU);
    const kinds: string[] = [];
    for await (const p of engine.intents(segments(3))) kinds.push(p.kind);
    assert.deepEqual(kinds, ['SET_SORT'], 'only the real command produced a proposal');
  });

  it('a transcriber error on one segment does not end the stream', async () => {
    const fake = new FakeTranscriber(['rendit sipas çmimit', null, 'shko te arka']);
    const engine = new WhisperProvider(fake, 'sq', MENU);
    const kinds: string[] = [];
    for await (const p of engine.intents(segments(3))) kinds.push(p.kind);
    assert.deepEqual(kinds, ['SET_SORT', 'NAVIGATE_CHECKOUT'], 'stream survived the failed segment');
  });

  it('yields pure readonly data — no handler/mutator crosses the engine boundary', async () => {
    const fake = new FakeTranscriber(['rendit sipas çmimit']);
    const engine = new WhisperProvider(fake, 'sq', MENU);
    for await (const p of engine.intents(segments(1))) {
      for (const v of Object.values(p)) assert.notEqual(typeof v, 'function', 'no function on a proposal');
    }
  });

  it('drives the SAME gate outcomes as the scripted mock (STATEFUL held, READ_ONLY applied)', async () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    const fake = new FakeTranscriber(['rendit sipas çmimit', 'shto dy sufllaqe']);
    const engine = new WhisperProvider(fake, 'sq', MENU);

    const statuses: string[] = [];
    for await (const p of engine.intents(segments(2))) statuses.push(gate.submit(p).status);

    assert.equal(calls.setSort, 1, 'READ_ONLY sort auto-applied');
    assert.equal(calls.addToCart, undefined, 'STATEFUL add NOT applied on submit');
    assert.deepEqual(statuses, ['applied', 'pending-confirm']);
    gate.confirm();
    assert.equal(calls.addToCart, 1, 'add applied only after the human confirm');
  });

  it('once() returns transcript + proposal for a single utterance (harness path)', async () => {
    const fake = new FakeTranscriber(['shto një Greek Salad']);
    const engine = new WhisperProvider(fake, 'sq', MENU);
    const { transcript, proposal } = await engine.once(new Float32Array(160));
    assert.equal(transcript, 'shto një Greek Salad');
    assert.equal(proposal?.kind, 'ADD_TO_CART');
    assert.equal(proposal?.args.productId, 'p-greek');
  });
});
