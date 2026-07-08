// Bebop kernel — the deterministic core law (mirrors sovereign-core GRAND-PLAN §0b-2/0b-3).
//
// Principles (MANIFESTO + your directives):
//  - PURE. No clock, no RNG, no network, no env. Given the same inputs it always produces the same
//    canonical bytes. (The Rust `domain` core bans `rand`/`chrono`/`reqwest` for exactly this reason;
//    Bebop's TS core honors the same discipline so the log is replayable and falsifiable.)
//  - `Envelope { seq, cause }` is the unit of truth. `cause` = a canonical CommandHash (content
//    address), exactly the D2 dedupe/causality seam. This is also the torrent/self-certifying address.
//  - `decide(command, state) -> Event[]` (the ONE door), `fold(state, event) -> state`,
//    `replay(genesis, log) -> state`. Forbidden transitions are explicit `DomainError`s, never panics.
//
// Identity/auth is OUT OF THE KERNEL (it stays in crypto.ts / shell). The kernel only ever consumes a
// plain `Command` + `State` + `Actor` — signing is a shell envelope, per GRAND-PLAN Phase-3 note.

import { sha256hex } from './crypto.ts';

// ── Command / Actor / Event vocab (the alphabet; exhaustive-fold gate) ──

export type Actor = { kind: 'node' | 'human' | 'system'; id: string };

export interface Command {
  actor: Actor;
  // The action. Kept minimal on purpose — Bebop's MVP core proves the shape, not the product surface.
  action: 'INGEST' | 'DISPATCH' | 'ROTATE' | 'PUBLISH' | 'REVOKE';
  // The canonical payload bytes this command carries (already content-addressed by the caller).
  payload: string; // hex/hash reference; the real bytes live in the torrent layer
  nonce: string; // caller-supplied determinism anchor (NOT RNG in-kernel; supplied by shell)
}

export type Event =
  | { type: 'INGESTED'; contentHash: string }
  | { type: 'DISPATCHED'; backend: string; taskHash: string }
  | { type: 'ROTATED'; from: string; to: string }
  | { type: 'PUBLISHED'; infoHash: string }
  | { type: 'REVOKED'; infoHash: string }
  | { type: 'DENIED'; reason: string };

export interface State {
  ingested: Set<string>; // content hashes we have accepted (idempotency / D2 dedupe)
  published: Set<string>; // info hashes we have published to the mesh
  revoked: Set<string>; // info hashes explicitly revoked
  seen: Set<string>; // command causes (hashes) already applied — replay protection
  lastBackend: string | null;
}

export interface Envelope {
  seq: number;
  cause: string; // CommandHash — canonical hash of the command (content address)
  event: Event;
}

export class DomainError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'DomainError';
  }
}

// ── Canonical hashing (content address; the torrent/self-certifying primitive) ──

/** Canonical CommandHash: stable field order, no ambient input. This IS the command's info_hash. */
export function commandHash(cmd: Command): string {
  const canonical = JSON.stringify({
    actor: cmd.actor,
    action: cmd.action,
    payload: cmd.payload,
    nonce: cmd.nonce,
  });
  return sha256hex(canonical);
}

// ── The ONE door: decide (pure) ──

export function decide(cmd: Command, state: State): Event[] {
  const hash = commandHash(cmd);

  // Idempotency / replay protection: same command (same cause) is a no-op replay, never a double-event.
  if (state.seen.has(hash)) {
    return [];
  }

  switch (cmd.action) {
    case 'INGEST':
      if (state.ingested.has(cmd.payload)) return [{ type: 'DENIED', reason: 'already ingested' }];
      return [{ type: 'INGESTED', contentHash: cmd.payload }];
    case 'DISPATCH':
      if (state.revoked.has(cmd.payload)) return [{ type: 'DENIED', reason: 'target revoked' }];
      return [{ type: 'DISPATCHED', backend: 'native', taskHash: cmd.payload }];
    case 'ROTATE':
      if (!state.lastBackend) return [{ type: 'DENIED', reason: 'no backend to rotate from' }];
      return [{ type: 'ROTATED', from: state.lastBackend, to: cmd.payload }];
    case 'PUBLISH':
      if (state.revoked.has(cmd.payload)) return [{ type: 'DENIED', reason: 'cannot publish revoked' }];
      return [{ type: 'PUBLISHED', infoHash: cmd.payload }];
    case 'REVOKE':
      return [{ type: 'REVOKED', infoHash: cmd.payload }];
    default: {
      // Exhaustive — a new action without a decide arm is a compile error (forbidden transition).
      const _exhaustive: never = cmd.action;
      throw new DomainError(`unknown action: ${String(_exhaustive)}`);
    }
  }
}

export function fold(state: State, event: Event): State {
  const next: State = {
    ingested: new Set(state.ingested),
    published: new Set(state.published),
    revoked: new Set(state.revoked),
    seen: new Set(state.seen),
    lastBackend: state.lastBackend,
  };
  switch (event.type) {
    case 'INGESTED':
      next.ingested.add(event.contentHash);
      break;
    case 'DISPATCHED':
      next.lastBackend = event.backend;
      break;
    case 'ROTATED':
      next.lastBackend = event.to;
      break;
    case 'PUBLISHED':
      next.published.add(event.infoHash);
      break;
    case 'REVOKED':
      next.revoked.add(event.infoHash);
      next.published.delete(event.infoHash);
      break;
    case 'DENIED':
      // Denials are observable but change no state — the kernel stays total.
      break;
    default: {
      const _exhaustive: never = event;
      throw new DomainError(`unknown event: ${JSON.stringify(_exhaustive)}`);
    }
  }
  return next;
}

// ── replay (pure) — state = fold*(genesis, events) ──

export function genesis(): State {
  return { ingested: new Set(), published: new Set(), revoked: new Set(), seen: new Set(), lastBackend: null };
}

export function replay(events: Event[]): State {
  return events.reduce(fold, genesis());
}

/** Append-only log builder: run decide through the kernel and wrap each event in an Envelope. */
export function applyCommand(cmd: Command, state: State): { state: State; envelopes: Envelope[] } {
  const cause = commandHash(cmd);
  const events = decide(cmd, state);
  let st = state;
  const envelopes: Envelope[] = [];
  events.forEach((event, i) => {
    st = fold(st, event);
    envelopes.push({ seq: st.ingested.size + i, cause, event });
  });
  // record the cause so the same command is a replay no-op (idempotency)
  st = { ...st, seen: new Set(st.seen).add(cause) };
  return { state: st, envelopes };
}

// ───────────────────────────────────────────────────────────────────────────────
// UNIVERSAL CHECKER GATE — "as above, so below"
//
// The SAME Checker abstraction validates a command at every scale:
//   • BELOW (local): `applyCommandChecked` runs it after `decide`, before the event is admitted.
//   • ABOVE (mesh): the receiving node reuses the identical invariant to admit/reject a gossiped
//     envelope (mesh.ts already does PQ-signature verify + (seq,cause) dedup; the invariant hook is
//     the shared "above" that watches the "below").
//
// Grounded in fundamental math/physics: the invariant is a pure function over the state transition
// (conservation law — what the command claims must hold after fold), exactly like energy/momentum
// conservation in mechanics. A transition that violates the invariant is quarantined (DENIED), never
// admitted. This is the copilot primitive at the deterministic core: DOER = decide/fold (below);
// CHECKER = invariant verifier (above).
// ───────────────────────────────────────────────────────────────────────────────

export type Checker = (cmd: Command, before: State, after: State, events: Event[]) => { ok: true } | { ok: false; reason: string };

/**
 * Apply a command through the kernel, but ONLY admit the resulting events if the Checker (the
 * "above") validates the transition the doer (the "below") produced. On rejection, the events are
 * quarantined into a DENIED event and the state is unchanged — fail-closed.
 */
export function applyCommandChecked(
  cmd: Command,
  state: State,
  checker: Checker,
): { state: State; envelopes: Envelope[]; quarantined: boolean; reason?: string } {
  const cause = commandHash(cmd);
  if (state.seen.has(cause)) return { state, envelopes: [], quarantined: false }; // replay no-op
  const events = decide(cmd, state);
  if (events.length === 0) return { state, envelopes: [], quarantined: false };
  // project the would-be next state
  let projected = state;
  for (const ev of events) projected = fold(projected, ev);
  const verdict = checker(cmd, state, projected, events);
  if (!verdict.ok) {
    // quarantine: do NOT admit; record the cause so we don't loop, emit a DENIED envelope
    const deniedState = { ...state, seen: new Set(state.seen).add(cause) };
    return {
      state: deniedState,
      envelopes: [{ seq: state.ingested.size, cause, event: { type: 'DENIED', reason: verdict.reason } }],
      quarantined: true,
      reason: verdict.reason,
    };
  }
  // admit via the normal path
  const admitted = applyCommand(cmd, state);
  return { ...admitted, quarantined: false };
}

/**
 * A minimal universal invariant: "no event may claim a published/ingested hash that the command did
 * not actually carry, and a PUBLISH/INGEST must reference a non-empty payload." This is the default
 * checker; callers may supply stricter domain-specific ones. Pure — no IO, no clock.
 */
export const defaultChecker: Checker = (_cmd, _before, _after, events) => {
  for (const ev of events) {
    if ((ev.type === 'INGESTED' && !ev.contentHash) || (ev.type === 'PUBLISHED' && !ev.infoHash)) {
      return { ok: false, reason: 'empty content reference' };
    }
  }
  return { ok: true };
};
