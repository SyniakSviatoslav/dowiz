// Voice intent types. The engine emits IntentProposal objects — PURE, READ-ONLY DATA.
// The engine holds ZERO write capability (no handler reference, no store dispatch). The
// ConfirmationGate is the only component that can apply a proposal, and a STATEFUL proposal
// applies only after an explicit human confirm. This file defines the data; capability-table.ts
// classifies it; confirmation-gate.ts enforces it. (Council voice-control / ADR-0015 §6.)

/**
 * The closed set of voice intents in ACTIVE scope (storefront menu + checkout READ-ONLY).
 * Anything not here — allergen/dietary filters, checkout field writes, place-order, payment,
 * order finalization, admin/courier actions — has NO IntentKind by design and is REJECTed
 * fail-closed by classify(). Adding a member forces a capability-table.ts entry (the build
 * breaks otherwise) — that exhaustiveness is the ratchet.
 */
export type IntentKind =
  | 'ADD_TO_CART' //        STATEFUL  → confirm-gated
  | 'SET_SORT' //           READ_ONLY
  | 'SET_MACRO_LENS' //     READ_ONLY
  | 'SELECT_CATEGORY' //    READ_ONLY (dietary-named categories rejected, see dietary-denylist.ts)
  | 'SET_SEARCH' //         READ_ONLY
  | 'TOGGLE_COMPARE' //     READ_ONLY
  | 'READ_ORDER' //         READ_ONLY (checkout read-back of the user's own cart)
  | 'NAVIGATE_CHECKOUT'; // READ_ONLY (navigation only — no field writes, no place-order)

/** Capability classification. REJECT is the fail-closed default for any unknown/excluded intent. */
export type Capability = 'READ_ONLY' | 'STATEFUL' | 'REJECT';

/**
 * What the engine emits. Immutable data only — no functions, no handler references (R2-F: the
 * engine surface carries zero write capability). `kind` is a raw string at the boundary so the
 * gate can fail-closed REJECT a value the matcher should never have produced.
 */
export interface IntentProposal {
  readonly kind: string;
  readonly args: Readonly<Record<string, unknown>>;
  readonly transcript: string;
  /** 0..1 matcher confidence. */
  readonly confidence: number;
}

/** Disposition of a proposal submitted to the gate. */
export type GateStatus = 'applied' | 'pending-confirm' | 'rejected';

export interface GateResult {
  readonly kind: string;
  readonly capability: Capability;
  readonly status: GateStatus;
  readonly reason?: string;
}
