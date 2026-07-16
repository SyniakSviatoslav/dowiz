# BLUEPRINT W22 — Governance/doc finalize + DOD retro

## WHY
After W17-21 verify, close the loop: document the governance state truthfully and produce a
DOD retro with literal 0-failed proof for every wave.

## WHAT (acceptance)
- `docs/design/` note: governance hooks are DELIBERATELY suspended per operator directive
  2026-07-15 (CLAUDE.md: "Mandatory Proof Rule / Ship Discipline / Self-improvement loop —
  SUSPENDED"). NOT a regression; do NOT restore without explicit operator word.
- DOD retro: append to SWARM-MANIFEST a "VERIFIED" section citing the literal `cargo test`
  count per wave (kernel/engine/web), each RED→GREEN gate name, and the commit SHA.
- Update `.specify/tasks.md` KU03-T* to DONE/STALE where covered by W17-20.

## RED→GREEN
- RED: no DOD retro; governance state ambiguous (looks like a bug).
- GREEN: doc states suspended-by-directive; retro has 0-failed per wave; tasks.md updated.

## FILES (Owns — docs only, disjoint)
- Modify: `docs/design/SWARM-MANIFEST-2026-07-16.md` (VERIFIED section), `.specify/tasks.md`
- Create: `docs/design/GOVERNANCE-SUSPENDED-2026-07-15.md` (truth note)

## RISKS
- Do NOT re-enable hooks (that would contradict the operator directive). Docs only.
