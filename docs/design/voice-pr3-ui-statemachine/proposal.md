# Design Proposal — Voice PR-3: MicFab + UI State Machine (transcription review)

> Register: System Architect. **Scope: transcription review, not new architecture.** The design is
> already binding (ADR-0015 + ui-spec.md). This documents the spec→code transcription of the pure
> FSM reducer and its injected contract. Most sections are "unchanged from ADR-0015 / ui-spec §X".
>
> **No new ADR.** ADR-0015 (`docs/adr/0015-voice-control.md`) already exists, is council-reviewed,
> and is binding. A per-PR ADR for a 1:1 transcription of an approved state diagram would be ADR
> sprawl for a non-decision. This proposal is the design record; the ADR is the authority.

## Spec provenance (honesty note)

The binding, in-tree spec sections are `docs/design/voice-control/ui-spec.md` **§2** (the state-machine
diagram + per-state visual + error matrix), **§3** (confirmation surface), plus `proposal.md §6`
(idempotency/write-sink) and `docs/adr/0015-voice-control.md §6`. The two doc names in the task brief
(`PHASE1-IMPLEMENTATION-PLAN.md §3`, `VOICE-UI-REFERENCE.md`) are **not present** in this tree; ui-spec
§2/§3 is the authoritative state-machine spec and is what the reducer transcribes.
**Status of the code:** `packages/ui/src/voice/types.ts` (injected contract) is written; the pure
`state-machine.ts` reducer is the deliverable-in-progress this proposal reviews.

## 1. What's being built

A pure, framework-free finite-state machine — `voiceReducer(state, event) => state` — that transcribes
ui-spec §2 1:1, plus the injected `VoiceGate`/`VoiceEngine` contract (`types.ts`) the mount site wires to
the real engine (PR-4) and the already-built `ConfirmationGate` (PR-2). The reducer is presentation/state
logic only; it holds **zero write capability**. Binding spec: ui-spec §2 (states/errors), §3 (confirm
chip), proposal §6 (write-sink/idempotency), ADR-0015 §6 (confirm-then-execute is the sole write sink).

## 2. Back-of-envelope

- **States:** 9 phases — `idle · disclosure · permission-request · listening · transcribing ·
  confirming · disambiguating · applied · error` (ui-spec §2 diagram, 1:1).
- **Events:** ~17 — `TAP · DISCLOSURE_ACCEPT · DISCLOSURE_DECLINE · PERMISSION_GRANTED ·
  PERMISSION_DENIED · PARTIAL_TRANSCRIPT · TRANSCRIBING · PROPOSAL · NO_MATCH · AMBIGUOUS ·
  RESOLVE_CANDIDATE · CONFIRM · CANCEL · ENGINE_ERROR · APPLIED_TIMEOUT · CONFIRM_TIMEOUT · RESET`.
- **Network/DB/FS calls in this file:** 0. **New runtime dependencies:** 0. **Migrations:** 0.
- **Bundle cost:** near-zero — a `switch (state.phase)` over 9 cases returning plain objects. No React,
  no timers, no DOM in the reducer (timers/DOM live in the hook, PR-3b). `types.ts` is type-only →
  erased at build (0 bytes emitted).

## 3. Options (concept + tradeoff)

| Option | Concept | Verdict |
|---|---|---|
| **(a) Pure reducer `(state,event)=>state`** | Elm/Redux-style pure FSM; effects pushed to the hook | **CHOSEN** |
| (b) Imperative class state machine | Encapsulated mutable object with methods | Rejected |
| (c) Library (xstate) | Statechart DSL + interpreter | Rejected |

**Why (a):** matches the repo's "no new deps" rule and the existing `packages/voice/confirmation-gate.ts`
"pure data in, gate mediates writes" style; trivially unit-testable under the repo's `node:test`-only
runner (no jsdom/React needed to assert a transition). **(b)** couples state to effects and needs a DOM
harness to test. **(c)** adds a runtime dep + a statechart DSL for a 9-node graph — over-engineering
against "schema rich, runtime minimal"; the reducer *is* the schema, no interpreter runtime required.

## 4. Data / migrations

**N/A — explicitly.** No DB, no schema, no migration, no `packages/db` touch. This module is client-side
presentation state only. Transcript strings are ephemeral in-memory data, never persisted.

## 5. Consistency + idempotency

- **CANCEL/RESET are idempotent no-ops** from non-cancelable phases → return the **same state reference**
  (no spurious re-render, no double-teardown).
- **CONFIRM fires `gate.confirm()` only from the `confirming` phase** — guards against double-fire; the
  gate itself is `consumed-once` (`confirmation-gate.ts:76-84`: no pending ⇒ no-op reject).
- **Stray/late engine callback after a barge-in RESET** (e.g. an `onProposal` arriving post-abort) is a
  **state-guarded silent no-op** — an event with no valid transition from the current phase returns state
  unchanged. Confirm-then-execute stays fail-closed (ADR-0015 §6; proposal §6 idempotency).

## 6. Failure / degradation

- **`ENGINE_ERROR` can interrupt any phase → `error`** (fail-safe; never silently stuck). Each error kind
  maps to one `voice.err.<kind>` i18n key (`types.ts:51`; ui-spec §2 error matrix). Every error node
  leaves touch fully working (voice is additive — ui-spec §7).
- **`PERMISSION_DENIED` → `error`** with the FAB still present (no re-prompt loop; ui-spec §2 matrix).
- **Barge-in** (re-tap during permission/listening/transcribing/confirming/disambiguating) = the hook's
  job: `gate.cancel()` + `engine.abort()` + `RESET`. The reducer only processes the resulting `RESET`.
- **FLAG for the hook (PR-3b), not this file:** if the injected `gate`/`engine` *throws* (e.g. `start()`/
  `abort()` faults), that is a hook-layer concern — wrap `engine.start/abort` + `gate.confirm/cancel` in
  try/catch and map a throw to an `ENGINE_ERROR` dispatch. The reducer is pure and cannot catch effects.
  **Not implemented here — raised as a hook-implementation requirement.** No code written.

## 7. Security + tenant isolation

- **No PII/audio in this file.** Transcript strings pass through as **opaque display data** — never
  persisted, never logged by this module (ui-spec §3.4/§8). No `console.*` of transcript content.
- **No tenant/auth/money surface touched.** No fetch, no API client, no store, no `@deliveryos/db|config`.
- **Verification (expected zero real matches):**
  `rg -n "fetch\(|/api/|apiClient|localStorage|@deliveryos/(db|config)|useStore" packages/ui/src/voice`
  → the only hit is the word "import" inside a *comment* in `types.ts:4`; there are **no** ES import,
  `require`, or `fetch` statements. `types.ts` is 100% `export interface`/`export type` (erased at build).

## 8. Operability

- No new env vars, no new endpoints, no observability surface. 100% client-side presentation/state logic.
- Gated behind the existing `VITE_VOICE_CONTROL_ENABLED` dark flag (ui-spec §1 render predicate clause 1).
  **Not yet wired to a mount point in this PR** — dark code, launch is a separate explicit act (Ship
  Discipline §4). Rollback = the mount site never renders the FAB; the reducer is inert unless dispatched.

## 9. Open / accepted risks

- **(a) Disambiguation plumbing depends on a future engine (PR-4)** implementing `onAmbiguous` /
  `resolveCandidate?` — **accepted**, structurally optional (`types.ts:88` marks `resolveCandidate?`
  optional; an engine that never emits `onAmbiguous` falls back to cancel). Owner: PR-4 lead.
- **(b) The ~900ms "applied → idle" auto-return timer** is arbitrary UX polish, not safety-relevant (the
  write already happened via the gate on human confirm) — **accepted**. Owner: PR-3b hook / design polish.

**Accepted, not re-derived:** all safety invariants (confirm-then-execute, dietary touch-only,
money-has-no-voice-grammar, actor-anonymous/zero-egress) are unchanged from ADR-0015 + ui-spec and are
enforced by the injected `ConfirmationGate` (`confirmation-gate.ts`), not by this reducer.
