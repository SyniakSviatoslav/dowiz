// The agent in the hub, the PURE half (lane W-KITCHEN, 2026-09-26): what the
// hub's answer to a chat line means, what the conversation keeps, and which
// starter lines a person is offered. No DOM, no clock, no network
// (`assistant-logic.test.mjs`); `admin/assistant.js` is the DOM half.
//
// THE HUB DECIDES. A line typed here goes to `POST /api/voice`, the same door
// the mic uses: the hub reads the words against the speaker's CAPABILITIES and
// answers now (a count, a screen to show), asks the assistant (a question), or
// PROPOSES a write the person confirms with one tap. Nothing is acted on from
// the words alone.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).
import { needsReason } from './voice-plan.js';

/// How many lines the panel keeps; the oldest go first.
export const MAX_KEPT = 40;

/// The screens a "show me" may open, as console tabs.
export const SCREENS = ['kitchen', 'menu', 'stock', 'orders'];

/// Append, keeping the last `MAX_KEPT`.
export const push = (log, msg) => [...(log || []), msg].slice(-MAX_KEPT);

/// The hub's answer to a line, as one message. `kind`:
///   say      a refusal or a plain line (`text`, maybe `heard`)
///   status   open / waiting counts
///   show     navigate to `screen`
///   ask      a question for the assistant
///   propose  a write to confirm: `text` (the read-back), `token`, `verb`, `reason`
export function reading(r){
  if (!r || !r.understood) return { from: 'hub', kind: 'say', text: (r && r.say) || null, heard: (r && r.heard) || null };
  if (r.needsConfirmation && r.token) {
    return { from: 'hub', kind: 'propose', text: r.readback || '', token: r.token, verb: r.verb || '', reason: needsReason(r.verb), state: 'open' };
  }
  if (r.action === 'status') return { from: 'hub', kind: 'status', open: Number(r.open) | 0, waiting: Number(r.waiting) | 0 };
  if (r.action === 'show' && SCREENS.includes(r.screen)) return { from: 'hub', kind: 'show', screen: r.screen };
  if (r.action === 'ask' && r.question) return { from: 'hub', kind: 'ask', question: String(r.question) };
  return { from: 'hub', kind: 'say', text: null, heard: null };
}

/// A proposal, once answered: `done`, `cancelled` or `failed`. Other messages
/// are returned untouched.
export function settle(log, i, state){
  return (log || []).map((m, j) => (j === i && m.kind === 'propose' ? { ...m, state } : m));
}

/// The starter lines a person is offered, by the words behind them. Each is
/// an i18n key whose TEXT is what is sent, in the reader's language, so the
/// hub's grammar reads it as if it had been typed. Only lines this principal
/// may act on are offered.
export function starters(p){
  const has = w => !p.staff || p.caps.has(w);
  return [
    ['asStatus', has('advance') || has('take_orders')],
    ['asShowKitchen', has('advance')],
    ['asShowStock', has('stock')],
    ['asLow', has('stock') || has('catalog')],
  ].filter(([, ok]) => ok).map(([k]) => k);
}

/// A line worth sending: trimmed, not empty, not a novel.
export const MAX_LINE = 400;
export function clean(text){
  const s = String(text ?? '').replace(/\s+/g, ' ').trim();
  return s && s.length <= MAX_LINE ? s : null;
}

/// The body a typed line is sent as: the mic's shape, certain and final.
export const lineBody = (text, lang) => ({ transcript: text, confidence: 1, is_final: true, lang });
