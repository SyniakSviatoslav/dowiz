# ADR-0020 — Open-Source License, Trademark & DCO Policy

- Status: Accepted (license text landed `ac1caba40`, 2026-07-14); three implementation gates
  remain OPEN and are named explicitly in Consequences below — this ADR records the decision, not
  a claim that every enforcement mechanism is built.
- Date: 2026-07-16 (decision effective `ac1caba40`, 2026-07-14 — this ADR is written after its
  code, per the honest-dating convention in `docs/adr/README.md`)
- Red-line: LEGAL / SUPPLY-CHAIN. Forward-only. The public-flip and EUTM filing are one-way doors
  gated separately (see Consequences); this ADR itself changes no runtime behavior.
- Supersedes/relates: `MANIFESTO.md` C6; corrects the stale "Apache-2.0 vs AGPLv3 mismatch" /
  "force-push scrub BLOCKED" lines in `ARCHITECTURE.md` §8/S3 (both false at HEAD — see Context);
  relates to the P10-OSS-readiness track and the 2026-07-03 secrets-exposure incident.

## Context

This repo is ~97.8% single-authorship (one operator, one AI-assisted contributor lineage; the only
other tracked change is one non-creative Dependabot commit), which makes a project-wide relicense
legally uncomplicated — there is no second copyright holder whose consent a license change would
require. Apache-2.0 → AGPLv3 is a one-way-compatible relicense (AGPLv3 is strictly more restrictive;
no Apache-licensed contribution becomes unlicensable under it).

A prior secrets-exposure incident (2026-07-03) was rotated and the affected history scrubbed; the
H8 remediation runbook closed 2026-07-13, and a follow-up decision on 2026-07-16 (P10) explicitly
declined a full SHA-rewrite of `origin/main` as redundant — the scrub commit is already an ancestor
of the current tip, and `origin/main` already points at the scrubbed commit. This ADR does not
reopen that decision; it is recorded here only because `ARCHITECTURE.md`'s stale S3 line
incorrectly still describes the scrub as a blocked red-line, and that line is corrected by this ADR
existing (see Consequences).

**Two canon lines this ADR retires as false-at-HEAD** (both re-verified live 2026-07-16, not
assumed): `ARCHITECTURE.md`'s S3 line claiming an "Apache-2.0 vs AGPLv3 mismatch" and a "force-push
scrub BLOCKED (red-line)" are both false — `LICENSE` at repo root is the full AGPLv3 text (verified
2026-07-16), and the scrub was resolved as described above. Per this repo's own Mentalism principle
("an idea asserted as real without the code that would make it real is a defect," and the converse:
a canon line asserting a problem that no longer exists is equally a defect), these lines should be
corrected wherever ARCHITECTURE.md is next merged — this ADR is the decision record that correction
would cite.

## Decision

1. **License: AGPLv3** for the canonical repo. Landed at `ac1caba40` (2026-07-14) — `LICENSE` at
   repo root is the full 660-line AGPLv3 text, re-verified live at ADR-write time.
2. **Trademark: `TRADEMARK.md` brand leash, not a mesh control.** The protocol and runtime remain
   free to fork per M11 (a sovereign hub MAY fork the code, drop the `dowiz` brand, and keep running
   the protocol) — trademark protects the *name*, never a technical gate on the mesh. `TRADEMARK.md`
   exists in tree (re-verified 2026-07-16).
3. **DCO 1.1 required.** Every commit must carry a `Signed-off-by` trailer (`CONTRIBUTING.md`,
   `DCO` file both in tree, re-verified 2026-07-16). **OPEN gap, named honestly:**
   `CONTRIBUTING.md:17` currently states *"CI rejects commits without a valid Signed-off-by"* — this
   is **not yet true**: no `dco-check` job exists in `.github/workflows/ci.yml` (re-verified
   2026-07-16, zero grep hits). This is `BLUEPRINT-P01-ci-truth-floor.md`'s `dco-check` job (§2.3),
   not yet built. Until that job lands, `CONTRIBUTING.md:17` is a claim ahead of its check — flagged
   here rather than silently left standing, per this repo's own Mentalism/Hermetic-audit finding
   (Finding 1.4 in that blueprint, independently confirmed by `PRINCIPLE-1-MENTALISM.md`'s "idea
   asserted as real without code that manifests it" standard).
4. **NOTICE + MANIFESTO in tree.** Both exist (re-verified 2026-07-16); no action needed here.
5. **Per-tool MIT carve-outs — OPEN, operator decision, not resolved by this ADR.**
   `tools/async-spool/Cargo.toml` and `tools/native-spa-server/Cargo.toml` both still declare
   `license = "MIT"` (re-verified 2026-07-16) against an AGPLv3-only repo policy. Two paths, neither
   picked here: (a) **carve-out** — document the two MIT tool crates explicitly in `NOTICE` as an
   intentional permissive exception (they are standalone, non-workspace crates with no dependency
   edge into the AGPLv3 kernel), or (b) **flip** both to `AGPL-3.0-or-later` for uniformity. This ADR
   records the choice as outstanding; `BLUEPRINT-P01` §6 flags the same gap and defers to the
   operator. `kernel/Cargo.toml` itself **also has no `license` field yet** (re-verified 2026-07-16) —
   should read `license = "AGPL-3.0-or-later"` once P01 lands it; not fixed by this ADR since it's a
   Cargo-manifest edit, not a decision record.

## Consequences

- Copyleft (AGPLv3) keeps the commons open under the strongest common-use protection short of a
  network-clause-free copyleft; the trademark leash protects the `dowiz` brand identity without
  constraining anyone's right to fork the protocol itself (M11 is preserved, not weakened, by this
  ADR).
- DCO gives commit-level provenance — genuinely valuable in a near-single-author repo specifically
  *because* it becomes load-bearing the moment a second contributor appears; the CI enforcement gap
  (point 3 above) is the one piece of this decision not yet structurally guaranteed (Ananke: it is
  currently "documented," not yet "structurally inevitable" — `BLUEPRINT-P01`'s `dco-check` job
  closes that gap when it lands).
- **Two operator gates, deliberately NOT resolved by this ADR** (both one-way doors, both correctly
  left to the operator per this repo's standing rule that public-flip is never autonomous):
  - **EUTM brand + filing** — "dowiz" is judged the stronger, less-descriptive mark vs
    "DeliveryOS"; filing is an operator action with a real cost (EUIPO e-filing fee), to be decided
    before any filing occurs, not implied by this ADR.
  - **Public-flip go** — the repository visibility/announcement decision remains fully
    operator-gated, one-way, and is not authorized, triggered, or implied by anything in this ADR.
- **One remaining housekeeping item, not gated:** a full all-origin-refs `gitleaks` sweep (confirming
  no secret survives anywhere in `origin`'s reachable history, not just the scrubbed branch tip) is
  still recommended before any public-flip discussion, independent of this ADR's own scope.

## What this ADR closes

Retires the "ADR-020 does not exist" finding (Hermetic-architecture audit, Mentalism F1,
`PRINCIPLE-1-MENTALISM.md` / `HERMETIC-ARCHITECTURE-PRINCIPLES.md` ranked-findings row #3) — the
decision this file cites in 15+ other documents now resolves to a real, checkable path instead of a
`find` returning zero results. Per `BLUEPRINT-P02-canon-repair-operator-decisions.md` §3's draft
outline, which this ADR follows structurally; the canon-line corrections that outline's §1 (C-1, C-2)
proposes for `ARCHITECTURE.md` remain a separate, not-yet-merged edit (canon is "merge, never
append" — this ADR does not touch `ARCHITECTURE.md` itself).
