import type { IntentProposal } from './types.js';
import type { PcmAudio, Transcriber } from './transcriber.js';
import { matchIntent } from './matcher.js';
import type { Locale, MenuContext } from './matcher.js';

/**
 * The real voice source: decoded audio utterances → ASR → the SAME deterministic matcher the
 * MockProvider uses → IntentProposals. It is the production analogue of MockProvider (identical
 * output shape: `intents()` yields `AsyncIterableIterator<IntentProposal>`), differing only in
 * where the transcript comes from — a live transcriber instead of a scripted string list.
 *
 * SAFETY BY CONSTRUCTION (ADR-0015 §6, R2-F / M3):
 *  - The engine is a SOURCE. It yields pure `readonly` IntentProposal DATA and holds ZERO write
 *    capability — no VoiceHandlers, no store dispatch, no menu setter. The ConfirmationGate (in
 *    apps/web) is the sole SINK that pulls proposals and may apply them; a STATEFUL proposal still
 *    needs a human confirm there. Nothing this class can do mutates the cart or the menu.
 *  - Its only injected collaborator is a `Transcriber` (audio→text). That is not a function-typed
 *    parameter and carries no mutator — so there is no surface to smuggle a write-capable closure in.
 *  - It never imports apps/web. Semantic args only; the web adapter maps them to real setters.
 *
 * Transcription quality is NOT proven by this class's unit test (a FakeTranscriber makes it
 * deterministic); it is proven by the separate real-audio eval harness over a fixed WAV corpus.
 */
export class WhisperProvider {
  readonly #transcriber: Transcriber;
  readonly #locale: Locale;
  readonly #menu: MenuContext;

  constructor(transcriber: Transcriber, locale: Locale, menu: MenuContext) {
    this.#transcriber = transcriber;
    this.#locale = locale;
    this.#menu = menu;
  }

  /**
   * Transcribe each utterance and yield the IntentProposal the matcher resolves. Utterances arrive
   * as decoded PCM segments — VAD-segmented mic audio in the browser, fixed WAV decodes in the eval
   * harness. An utterance that transcribes to silence, or whose transcript resolves to no confident
   * intent, yields nothing (fail-quiet: an un-heard command does nothing, never a wrong action).
   * A transcriber error on one utterance is swallowed so one bad segment cannot end the stream.
   */
  async *intents(utterances: AsyncIterable<PcmAudio>): AsyncIterableIterator<IntentProposal> {
    for await (const pcm of utterances) {
      let transcript: string;
      try {
        transcript = await this.#transcriber.transcribe(pcm);
      } catch {
        continue; // one failed segment must not kill the mic session; skip and keep listening
      }
      if (!transcript) continue;
      const proposal = matchIntent(transcript, this.#locale, this.#menu);
      if (proposal) yield proposal;
    }
  }

  /** Transcribe + match a SINGLE utterance (the eval harness path; also handy for a warmup probe). */
  async once(pcm: PcmAudio): Promise<{ transcript: string; proposal: IntentProposal | null }> {
    const transcript = await this.#transcriber.transcribe(pcm);
    const proposal = transcript ? matchIntent(transcript, this.#locale, this.#menu) : null;
    return { transcript, proposal };
  }
}
