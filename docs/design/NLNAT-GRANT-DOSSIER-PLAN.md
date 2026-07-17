# NLnet NGI Zero — grant dossier PLAN (planning only, not a submission)

> BLUEPRINT-P19 §3. **This is a planning document. No grant application is submitted by
> this phase.** Submission is a separate gated action owned by the operator at / after
> the public flip (Phase 18). A reviewer cannot evaluate a private repo, so the dossier
> is finalized after the flip.

## Eligibility — confirmed

- **FOSS license mandate — SATISFIED.** Repo is AGPLv3 since `ac1caba40`
  (`LICENSE`, full AGPLv3). No further license work for eligibility.
- **First-time applicant ceiling ≤ €50,000** — budget must fit under this.
- **R&D focus — genuine.** Post-quantum mesh transport, self-healing routing,
  partition-tolerant settlement are real research-and-development.
- **European dimension — fits E60.** EU/Ukraine delivery-sovereignty; local-first + no
  surveillance (M8) aligns with NGI's privacy/resilience mandate.
- **Format** — concise English via `nlnet.nl/propose`.

## Narrative shape (sections to write at submission)

1. **What** — zero-dependency, post-quantum, decentralized delivery protocol; delivery
   as the demonstrator (mirrors POSITIONING.md one-sentence pitch).
2. **Why it matters to NGI/Europe** — resilience (no SPOF, works partitioned), privacy
   (local-only telemetry, no courier scoring/surveillance — M8·E58), post-quantum
   readiness ahead of harvest-now-decrypt-later, EU/UA sovereignty (E60).
3. **Technical merit / credibility** — cite the concrete built substrate: ML-DSA-65 +
   ML-KEM-768 with in-repo FIPS 204/203 KATs, self-certifying node identity (ADR-0007),
   local PQ-at-rest persistence (the **spectral/sqlless** content-addressed `BlockStore`
   + `FileEventStore`, with **pgrust** as the uniform SQL-fallback/backup target — never
   SQLite; ADR-0008 is being updated SQLite→pgrust). Cite ADR-0020
   (`docs/adr/0020-oss-license-tm-dco.md`) for the license claim. If Phase 14 has
   landed, add the two-hub per-hub-wiki delta-exchange demo as replication evidence.
4. **Work plan / deliverables** — map work-packages to roadmap phases genuinely R&D and
   not yet done at flip time (candidates: P3 PQ trust-root hardening, P9 confidential
   self-healing wire, P14 dispute/escrow + per-hub graph-wiki). Each WP = a falsifiable
   done-test lifted from the roadmap.
5. **Budget ≤ €50k** — person-months against the WPs above, itemized; explicitly under
   the first-time ceiling. No hosting/GPU capex contradicting the self-host /
   scale-to-zero posture.
6. **Team / European dimension** — sole maintainer, EU/UA base, AGPLv3 commons intent
   (MANIFESTO C6·C9).

## Timeline — target the NEXT open call

The 2026-06Z window is closed (deadline 2026-06-01 passed). NGI Zero Commons calls
recur. **Target the first open call whose deadline falls AFTER the public flip** — a
reviewer cannot evaluate a private repo.

> **REASONED NON-APPLY NOTE (placeholder — fill at flip time):** if the flip slips past
> a window, the correct output is a dated, written "not this call, here's why, here's the
> target" note, not a rushed submission or silence. Acceptance (BLUEPRINT-P19 §7.1):
> either a submission receipt exists OR a dated, reasoned non-apply decision exists.
> Silence fails.

## Status

Plan only. No submission, no portal reference, no application number yet. The actual
send is gated on the operator's public-flip go (O17) + a live, API-visible public repo.
