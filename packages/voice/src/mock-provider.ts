import type { IntentProposal } from './types.js';
import { matchIntent } from './matcher.js';
import type { Locale, MenuContext } from './matcher.js';

/**
 * A scripted voice source for deterministic tests and the Playwright path. It takes a fixed list of
 * transcripts (as the real engine would emit AFTER ASR) and runs them through the SAME matcher the
 * live engine uses, yielding IntentProposals — no mic, no model, no nondeterminism. This is what lets
 * Playwright assert intent→DOM without a live microphone (proposal §10). It proves the matcher + gate
 * wiring; it does NOT prove transcription quality (that is the separate, human-mic Phase-0 gate).
 */
export class MockProvider {
  readonly #transcripts: readonly string[];
  readonly #locale: Locale;
  readonly #menu: MenuContext;

  constructor(transcripts: readonly string[], locale: Locale, menu: MenuContext) {
    this.#transcripts = transcripts;
    this.#locale = locale;
    this.#menu = menu;
  }

  /** Yields one IntentProposal per transcript the matcher resolves; unmatched transcripts are skipped. */
  async *intents(): AsyncIterableIterator<IntentProposal> {
    for (const t of this.#transcripts) {
      const proposal = matchIntent(t, this.#locale, this.#menu);
      if (proposal) yield proposal;
    }
  }
}
