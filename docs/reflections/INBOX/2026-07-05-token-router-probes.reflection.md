# Reflection — token-economy probes → TOKEN ROUTER rule (2026-07-05)

**WHAT:** Ran 7 controlled probe dispatches (2 live A/B pairs + dispatch-floor pair + 1 corrupted
retry) + fresh local benches across every token layer; codified the results as AGENTS.md
"RULE: TOKEN ROUTER" + refreshed map-reduce floor figures; CLAUDE.md bullet proposed (protected
zone, operator-gated). Report: docs/research/token-economy-comparison-2026-07-05.md.

**WHY (causal, not just where):** The stack's headline numbers over-generalized because each came
from ONE task shape. The causal driver of graph-first savings is **the number of speculative file
reads the graph replaces** — a narrow trace replaces few (−19.5%), a broad sweep replaces dozens
(−52.7%). Quoting the sweep-end number (−54.9%) for all agentic work was the error mode; the
router table fixes it by keying the route to task shape, and quality floors key on red-line class
because the audit probe showed the native lane's extra 72K bought real extra findings (the
owner-pickup WS/DB divergence) — cost-cutting without a critic lane WOULD have lost signal.

**Surprises worth ratcheting:**
1. ~1/7 lanes corrupts at dispatch (garbage return, ≈1 floor wasted) → retry budget now in the
   map-reduce spec. If recurrence climbs, promote to a loop-harness auto-retry guardrail.
2. repowise skeleton measured 9.5% of full read on orders.ts — far better than its documented ~37%.
3. The protect-paths hook correctly blocked the CLAUDE.md edit (governance gate held under an
   autonomous agent applying an operator directive — the ratchet worked as designed).

**Escalation (open, council-class, NOT touched):** probe 2N found
`routes/owner/dashboard.ts:379-429` broadcasts `status:'PICKED_UP'` on the dashboard WS channel
but never persists `orders.status` (row stays IN_DELIVERY) — UI/DB divergence, state-machine/
contract class. Also `SCHEDULED` has no writer. Needs a decision, not a silent fix.
