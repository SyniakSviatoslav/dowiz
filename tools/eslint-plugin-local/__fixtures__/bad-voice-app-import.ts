// Test fixture for no-voice-app-import (must BE flagged).
// ANTI-PATTERN: the voice engine reaching outside itself into the consuming app, a fetch/API client,
// or a Cart mutator. Any of these would let a mutating capability cross INTO the engine boundary,
// defeating "the engine holds zero write capability" (ADR-0015 §6 / proposal §6 guardrail #1 G1b).

// ANTI-PATTERN: importing the consuming app directly.
import { setSortBy } from '../../apps/web/src/pages/client/MenuPage.js';

// ANTI-PATTERN: importing a Cart* mutator — a write-capable closure could ride in on this.
import { useSharedCart } from '../../apps/web/src/lib/CartProvider.js';

// ANTI-PATTERN: a raw fetch/API-client package — a network side-channel outside VoiceHandlers.
import axios from 'axios';

export function badWire(): void {
  setSortBy('price-asc');
  useSharedCart();
  void axios;
}
