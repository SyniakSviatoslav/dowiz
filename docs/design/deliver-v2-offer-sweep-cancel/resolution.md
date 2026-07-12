# Resolution — offer-sweep grace-cancel (breaker round 1)

Inputs: `breaker-findings.md` (F1-F8), `counsel-opinion.md` (ETHICAL-STOP: none on this change; one
pre-registered on enablement). Design-time; no production code here.

## Headline
The breaker's two HIGHs (F1 missed `ORDER_CANCELLED` fan-out; F2 gate-laundering + guardrail-blind
owner-callable export) **broke Option B's core justification**. Reconsidered honestly: **Option B is
withdrawn**; the chosen fix is **Option A + coupling-fix** (widen the machine for three SYSTEM-only
CANCELLED edges; close owner exposure at the route layer; route Pass 4 through `updateOrderStatus`; publish
`ORDER_CANCELLED` post-commit). This is the option that keeps `order-machine.ts` the **single transition
authority** — the coordinator's explicit tie-breaker.

## Chosen option — A-with-coupling-fix vs B-hardened
| Axis | B-hardened | A + coupling-fix (chosen) |
|---|---|---|
| Single transition authority | NO — a second `ALLOWED_FROM` shadow table bypasses `assertTransition` | YES — machine + `assertTransition` remain the one gate |
| F2 (laundering / owner-callable / guardrail-blind) | Unmitigable — export is owner-importable + call-invisible to the scan | Dissolved — no new export, no new raw UPDATE; owner exposure closed by explicit route authz |
| Guardrail | Green but *false-comforting* (machine-illegal edge in a "blessed" file) | Green because the funnel is real (no allowlist edit) |
| F5 dashboard-delta drift | Present (hand-rolled delta) | Dissolved (`fetchOrderDelta` canonical shape) |
| F6 machine-vs-history divergence | Present (shadow authority) | Dissolved (edges legal) |
| Cost | Second cancel mutator + shadow array | Machine widening + route guard + exhaustive-transition pin |

## Resolution table
| # | Sev | Disposition | Fix / rationale |
|---|-----|-------------|-----------------|
| **F1** | HIGH | **FIX** | Worker publishes `BUS_CHANNELS.ORDER_CANCELLED` **post-commit** (sanctioned `signals.ts:237-248` pattern) → `lifecycle-handlers.ts:27` resolves dwell alerts (`app_resolve_order_alerts`) + `boss.cancel`s pending `notify.dispatch.*` escalation jobs. Proof: F1 test (§proof 6). |
| **F2** | HIGH | **FIX (dissolved by option flip)** | Option A has no new export and no new raw UPDATE — the guardrail passes because `updateOrderStatus` genuinely is the mutator (no laundering). Owner exposure closed by route-layer `assertOwnerTargetAllowed` (403 for owner `CANCELLED` from CONFIRMED/PREPARING/READY). Proof: guardrail green + `RAW_CANCEL_ALLOW` unchanged + coupling-fix test (§proof 2,4). |
| **F3** | HIGH | **FIX (dissolved) + assert** | Cash-safety is now `updateOrderStatus`'s existing property (writes no ledger/trace); no boundary claim to over-assert. The R2-3 fold is **extended** to terminalize any active assignment on any `→CANCELLED` (idempotent) so a widened edge can't strand a binding; terminalizing writes no `'hold'`. Worker also hard-re-checks "no active assignment" under lock (F7). Proof: `courier_cash_ledger` count = 0 (§proof 5). |
| **F4** | MED | **SPLIT: FIX + ACCEPT** | Consequential fan-out (ORDER_CANCELLED, customer push) is POST-commit (F1). The live WS delta stays pre-commit *inside* `updateOrderStatus` — the pre-existing whole-codebase mutator property (every caller). ACCEPT: negligible self-reconciling window; dangerous effects are post-commit. Registered as a cross-cutting known-risk on `updateOrderStatus`, not this path. |
| **F5** | MED | **FIX (dissolved)** | `updateOrderStatus` calls `fetchOrderDelta` → canonical dashboard shape for free; the worker no longer hand-rolls a delta. No shape-drift surface remains. |
| **F6** | MED | **FIX (dissolved) + PIN** | Edges are now legal machine edges → history matches the machine; no shadow authority. PIN: exhaustive `assertTransition` test asserting the exact legal edge set (incl. the three additions) so the widening is conscious + future drift fails red. (Replaces the withdrawn "shadow transition table" pin — counsel condition satisfied in stronger form.) |
| **F7** | LOW | **FIX** | Worker re-checks `NOT EXISTS(active assignment)` under the per-row `FOR UPDATE` lock immediately before the mutator call; a fresh binding → ROLLBACK + continue. Never cancels an order a courier just took. Proof: F7 anti-race test (§proof 7). |
| **F8** | LOW | **ACCEPT + FLAG** | Lock order `orders`→`courier_assignments` is `updateOrderStatus`'s **pre-existing** fold order (already live for the IN_DELIVERY edge via owner PATCH/signals/courier cancel). Serialized by the sweep advisory lock + low volume. Implementer confirms the accept/dispatch bind path locks `orders` first; else LOW follow-up. Not new to this change. |

## Counsel conditions
| Condition | Disposition |
|---|---|
| Pin the shadow transition table with a test | **SATISFIED (stronger).** No shadow array under Option A; pinned instead on the canonical machine via an exhaustive `assertTransition` test (F6 PIN). |
| ADR addendum ships WITH the code (merge-gate) | **ACCEPTED.** Addendum written now (`ADR-deliver-v2-cash-as-proof.md` §Addendum); merge is gated on it landing in the same PR. |
| Improve customer cancel copy (courier-scarcity, not fault; refund coming if prepaid) | **ACCEPTED (OPEN for build).** Proposal §10 OPEN item: honest cause attribution + prepaid-refund note, i18n al/en. |
| Register STOP-REFUND-BEFORE-GRACE | **ACCEPTED.** Recorded in the ADR addendum: no co-enable of `DISPATCH_OWNER_GRACE_ENABLED` + prepaid until a paid grace-cancel writes `refund_due` (or is proven impossible). Owner: payments + grace-cancel councils jointly. |
| Counsel §5 "what is owed to the let-down customer" | Routed to the grace-cancel STOP-ETHICS enablement council (out of scope for this deploy-unblock). |

## ETHICAL-STOP handling
- **On this change: NONE** (dark on both sides, gate satisfied honestly, gap pre-existing + disclosed).
  No revise-or-human-flag needed to proceed with the dark code.
- **Pre-registered STOP-REFUND-BEFORE-GRACE**: recorded in the ADR, owned by the enablement councils —
  flagged for human decision at enablement, not now.

## HIGH count in design
- Entering: F1, F2, F3 = 3 HIGH.
- After resolution: F1 FIX, F2 dissolved by option flip, F3 dissolved + asserted. **0 HIGH remain.**
- Self-adversarial re-check on Option A (did the flip introduce a new HIGH?):
  - Widened edge stranding an active assignment → closed by the fold extension (F3 disposition).
  - Owner over-exposure → closed by the route coupling-fix (F2).
  - Machine/history divergence → gone (edges legal).
  - Pre-commit phantom broadcast of *consequential* effects → moved post-commit (F1/F4).
  No new HIGH found. **Design is re-attackable at 0 HIGH.**

## Deltas written this round
- `proposal.md` — decision flipped to Option A + coupling-fix; §3/§4/§6/§7/§8/§10/appendix/proof-plan revised.
- `docs/adr/ADR-deliver-v2-cash-as-proof.md` — Addendum (three edges, route guard, ORDER_CANCELLED fold,
  fold extension, STOP-REFUND-BEFORE-GRACE).
- `resolution.md` — this file.
