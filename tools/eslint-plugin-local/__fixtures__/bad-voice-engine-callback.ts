// Test fixture for no-voice-engine-callback (must BE flagged).
// ANTI-PATTERN: the voice engine's public surface accepts callback/handler parameters. A write-capable
// closure (e.g. addToCart) could ride in on one, defeating "the engine holds zero write capability"
// (ADR-0015 §6 / breaker R2-F). The engine must be a pure source; the ConfirmationGate is the sink.

export interface BadEngineSource {
  // ANTI-PATTERN: an on()/subscribe callback API is exactly the shape R2-F rejects.
  subscribe(onResult: (proposal: unknown) => void): void;
}

export class BadEngine {
  // ANTI-PATTERN: a function-typed constructor param — a closure crosses the engine boundary.
  constructor(private onIntent: (proposal: unknown) => void) {}

  // ANTI-PATTERN: an event-callback method param.
  on(event: string, cb: (proposal: unknown) => void): void {
    this.onIntent(event);
    cb(event);
  }
}

// ANTI-PATTERN: an exported function taking a handler closure.
export function wireEngine(handler: (proposal: unknown) => void): void {
  handler(null);
}
