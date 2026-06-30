import type { GateResult, IntentProposal } from './types.js';
import { classify } from './capability-table.js';
import { isDietaryCategory } from './dietary-denylist.js';

/**
 * The handlers the gate may invoke. OWNED BY apps/web and injected at construction — the voice
 * ENGINE never receives this object, so the engine cannot mutate anything (R2-F / M3: source =
 * engine, sink = gate; no function-typed value crosses the engine boundary). Each handler maps to
 * exactly one READ_ONLY/STATEFUL intent; there is NO handler for money/checkout-write/dietary, so
 * even a mis-classified such intent has nothing to call.
 */
export interface VoiceHandlers {
  readonly addToCart: (args: Readonly<Record<string, unknown>>) => void;
  readonly setSort: (args: Readonly<Record<string, unknown>>) => void;
  readonly setMacroLens: (args: Readonly<Record<string, unknown>>) => void;
  readonly selectCategory: (args: Readonly<Record<string, unknown>>) => void;
  readonly setSearch: (args: Readonly<Record<string, unknown>>) => void;
  readonly toggleCompare: (args: Readonly<Record<string, unknown>>) => void;
  readonly readOrder: (args: Readonly<Record<string, unknown>>) => void;
  readonly navigateCheckout: (args: Readonly<Record<string, unknown>>) => void;
}

/**
 * The confirm-then-execute boundary. READ_ONLY proposals apply immediately; a STATEFUL proposal
 * (only ADD_TO_CART in active scope) is held PENDING and applies ONLY when confirm() is called —
 * which a UI wires exclusively to a human tap on the confirm chip. A REJECT proposal is dropped
 * and never touches a handler. There is no code path from an unconfirmed STATEFUL or a REJECT to a
 * mutation. (ADR-0015 §6.)
 */
export class ConfirmationGate {
  readonly #handlers: VoiceHandlers;
  #pending: IntentProposal | null = null;

  constructor(handlers: VoiceHandlers) {
    this.#handlers = handlers;
  }

  /** The pending STATEFUL proposal awaiting human confirm, if any (the UI renders the chip from this). */
  get pending(): IntentProposal | null {
    return this.#pending;
  }

  /** Engine output enters here as pure data. Returns the disposition without applying STATEFUL writes. */
  submit(proposal: IntentProposal): GateResult {
    let capability = classify(proposal.kind);

    // Dietary/allergen-named category selection is touch-only (breaker R2-B): downgrade to REJECT
    // BEFORE any auto-apply, so a mis-heard "show gluten-free" can never narrow the menu by voice.
    if (
      capability === 'READ_ONLY' &&
      proposal.kind === 'SELECT_CATEGORY' &&
      typeof proposal.args.categoryName === 'string' &&
      isDietaryCategory(proposal.args.categoryName)
    ) {
      return {
        kind: proposal.kind,
        capability: 'REJECT',
        status: 'rejected',
        reason: 'dietary-category-touch-only',
      };
    }

    if (capability === 'REJECT') {
      return { kind: proposal.kind, capability, status: 'rejected', reason: 'unknown-or-excluded-intent' };
    }
    if (capability === 'READ_ONLY') {
      this.#apply(proposal);
      return { kind: proposal.kind, capability, status: 'applied' };
    }
    // STATEFUL → hold for human confirm. Replaces any prior pending (last-proposal-wins, no replay).
    this.#pending = proposal;
    return { kind: proposal.kind, capability, status: 'pending-confirm' };
  }

  /** Apply the pending STATEFUL proposal. Called ONLY by a human confirm tap. No pending ⇒ no-op reject. */
  confirm(): GateResult {
    const p = this.#pending;
    if (!p) {
      return { kind: '', capability: 'REJECT', status: 'rejected', reason: 'no-pending-proposal' };
    }
    this.#pending = null;
    this.#apply(p);
    return { kind: p.kind, capability: 'STATEFUL', status: 'applied' };
  }

  /** Discard the pending proposal (human cancel / timeout / outside-tap — fail-safe to no action). */
  cancel(): void {
    this.#pending = null;
  }

  /** Dispatch a vetted proposal to its single handler. An unrecognized kind can never reach here
   * (classify REJECTs first); if it somehow did, the switch falls through to nothing. Fail-closed. */
  #apply(p: IntentProposal): void {
    switch (p.kind) {
      case 'ADD_TO_CART':
        this.#handlers.addToCart(p.args);
        break;
      case 'SET_SORT':
        this.#handlers.setSort(p.args);
        break;
      case 'SET_MACRO_LENS':
        this.#handlers.setMacroLens(p.args);
        break;
      case 'SELECT_CATEGORY':
        this.#handlers.selectCategory(p.args);
        break;
      case 'SET_SEARCH':
        this.#handlers.setSearch(p.args);
        break;
      case 'TOGGLE_COMPARE':
        this.#handlers.toggleCompare(p.args);
        break;
      case 'READ_ORDER':
        this.#handlers.readOrder(p.args);
        break;
      case 'NAVIGATE_CHECKOUT':
        this.#handlers.navigateCheckout(p.args);
        break;
    }
  }
}
