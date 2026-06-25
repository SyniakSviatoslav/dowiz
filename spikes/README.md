# `spikes/` — the RECON speed (the sandbox side of the boundary)

> Governed by [docs/operating-model/agent-operating-model.md](../docs/operating-model/agent-operating-model.md).
> This directory is the **recon speed**: open, cheap, throwaway, non-conformant code that touches
> reality to produce **evidence & decisions** — never merge-able product code.

## 🔴 The boundary (sacred)
- Spike code lives **only here**. It NEVER reaches `apps/`/`packages/` as merge-able code.
- `apps/`/`packages/` MUST NOT `import` from `spikes/`.
- Crossing the boundary happens **only** via a separate `build` prompt under full discipline —
  the spike's *evidence* informs the build; its *code* is thrown away.
- Relaxed governance here is NOT relaxed **product red lines**: zero PII in AI/queues/logs, integer
  money, server-only price/status, RLS FORCE, zero cookies, tenant isolation, human-authority on
  `deliver`. These hold in the sandbox too.

## Spike protocol (§2)
1. State the **hypothesis** + the **cheapest test that falsifies it** → record in Mem0.
2. **Time-box** it (write the clock down). Buffer ∝ the number of systems the step stitches, not the
   size of the step — complexity lives at the seams.
3. Build the **ugly fast** that touches reality (NOT the polished whole — put the marshmallow first).
4. Collect **evidence**: real-world feedback/telemetry/log (PostHog, a pilot owner, the first real
   paid order from someone else's venue) — not only Playwright.
5. **Decide:** confirmed → write a separate `build` prompt; falsified → throwaway + Mem0 note;
   unclear → an even cheaper spike. The code stays in `spikes/`.

## Classification
Spike work carries the `spike` label (see §1). `post-edit-gates` frees `spike` changes from
system-fit / design-system / full matrix, but enforces the red-line grep and the boundary.
