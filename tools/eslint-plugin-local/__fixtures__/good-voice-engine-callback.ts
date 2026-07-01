// Test fixture for no-voice-engine-callback (valid — must NOT be flagged).
// CORRECT: the engine is a SOURCE. It yields readonly IntentProposal DATA via an AsyncIterable and
// takes only data / object-port parameters — never a callback. The ConfirmationGate (the sink, in
// apps/web) is the one that legitimately holds the handler object. (ADR-0015 §6 / R2-F.)

interface IntentProposal { readonly kind: string; readonly confidence: number; }

// An object PORT (interface with a method) is fine to depend on — it is not a function-typed param,
// and it carries no menu/cart mutator.
export interface Transcriber {
  transcribe(audio: Float32Array): Promise<string>;
}

// A handlers OBJECT is a legitimate SINK param even though its PROPERTIES are functions — its type is
// a TSTypeReference (VoiceHandlers), not a function type, so it is not the banned callback param.
export interface VoiceHandlers {
  readonly addToCart: (args: Readonly<Record<string, unknown>>) => void;
}

export class GoodEngine {
  // object port + plain data — no closure crosses in.
  constructor(private transcriber: Transcriber, private locale: string) {}

  // yields DATA; the caller pulls (sink pulls source), no callback handed to the engine.
  async *intents(utterances: AsyncIterable<Float32Array>): AsyncIterableIterator<IntentProposal> {
    for await (const u of utterances) {
      const t = await this.transcriber.transcribe(u);
      if (t) yield { kind: 'SET_SORT', confidence: 0.8 };
    }
  }
}

// The gate (sink) may take the handlers object — allowed: object param, not a function param.
export class GoodGate {
  constructor(private handlers: VoiceHandlers) {}
  apply(p: IntentProposal): void { this.handlers.addToCart({ kind: p.kind }); }
}
