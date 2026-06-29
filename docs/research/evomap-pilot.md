# EvoMap pilot — out-of-tree harness tool (egress-gated)

**STATUS: SCAFFOLDED — PENDING OPERATOR RUN.** Out-of-band dev/harness experiment. Not wired, not in CI,
not a dependency. **Owner:** Operator. **Expiry:** freeze if not run in 30 days.

## What it is
[EvoMap](https://evomap.ai/) — "AI self-evolution infrastructure": a Genome Evolution Protocol (GEP)
**external network + marketplace** where agents share/inherit reusable capability assets ("Genes/Capsules").
Evaluated as a possible source of pre-validated agent capabilities.

## Boundary & controls (G5 + Ethics Charter)
- **🔴 Egress boundary (load-bearing).** EvoMap is an EXTERNAL network. **No dowiz code, data, PII, tenant
  context, secrets, or learned-from-our-data capabilities** may be published to the GEP network. Inbound
  only (browse/evaluate public Genes); any outbound capability-sharing is OUT OF SCOPE for this pilot.
- **Commons clause (Ethics Charter).** "AI is a collective human tool… never captured for the exclusive
  benefit of a narrow group." Do not feed dowiz's learned capabilities into a third-party marketplace —
  that would both leak and capture. Inbound evaluation only.
- **Subprocessor/classification.** If a real EvoMap endpoint is ever called, it must first be classified
  in `compliance/env-classification.md` (external-subprocessor) + added to `compliance/subprocessors.md`
  (the G5 gate fails closed otherwise). No `EVOMAP_*` env enters `packages/config` in this pilot.
- **No dowiz credential** in any sidecar env — `node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>`.
- `evomap`/`@evomap/` are FORBIDDEN-DEP (never a product dependency).

## What it measures
Whether any GEP Gene/Capsule meaningfully beats our existing skills — inbound, read-only, on synthetic
tasks. Unproven nascent infra; adopt nothing without a SEPARATE ADR + a verified provenance/license trail.

_Results: (operator fills)._
