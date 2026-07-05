# Reflection — a commit bundling unrelated concerns dropped the one step none of its concerns "owned"

- **Date:** 2026-07-05
- **Trigger:** meta-loop backlog item 6 (autonomous-continuation task) — retrospective read of
  commit `b536ca07c0e716b967f7ec169f615e07770e6158` ("nutrition/BOM product-card + font/consistency
  polish; sandbox-swarm-gate + skill-evolution loops"), whose missing ledger row was already
  retroactively fixed by ledger row #68.
- **Class:** self-improvement loop / discipline-triggered-step-dies family (same law as ledger #48
  and the `2026-07-03-swarm-mergeback-rot` reflection).

## WHAT happened

One commit shipped three unrelated units of work in a single pass: a storefront feature
(nutrition/BOM product card + Cyrillic font fallback, with its own 7+7 unit tests), two harness
*documentation* systems (Sandbox-Swarm-Gate, skill-self-evolution), and a dev-plane scaffold
script. The storefront feature had real red→green tests but **no ledger row** — a direct violation
of the ratchet rule ("every future fix adds a guardrail + a ledger row before it is 'done'") by the
same commit that was simultaneously *writing* two harness docs about process discipline. Ledger
row #68 already fixed this retroactively; this reflection is about *why* it happened, which #68's
own "why" column only partially covers.

## WHERE

Commit `b536ca07` diff spans `apps/web/src/lib/dishNutrition.ts` + `.test.ts`,
`packages/ui/src/theme/fonts.ts` + `.test.ts` (the actual fix), alongside
`docs/design/harness/SANDBOX-SWARM-GATE.md`, `docs/design/harness/SKILL-EVOLUTION.md`,
`loops/registry.md`, `scripts/sandbox-swarm-gate.mjs` (unrelated harness additions) — six-plus
concerns in one commit.

## WHY (causal root, not just location)

Ledger row #68's own "why" column already names the mechanical gap: no gate exists that ties
"new `*.test.ts` files under `apps/web|packages/ui`" to "the ledger must also be touched." But
there's a layer above that worth naming for the self-improvement loop specifically: **the
ledger-row step is discipline-triggered** (an agent has to remember to do it, same as ledger #48's
"what is hook-enforced survives, what is discipline-triggered dies" law, and the same root
`2026-07-03-swarm-mergeback-rot` already used for a different discipline-triggered step, the
sandbox merge-back). A single-concern commit gets a single mental "am I done" checklist pass at
the end, and ledger-row is on it. A **six-concern commit** gets that same one checklist pass spread
across six concerns' worth of attention — the probability that the pass happens to land on the
*storefront feature's* checklist specifically (as opposed to "did I write the harness docs
correctly," "did I wire loops/registry.md," "does the scaffold script default to dry-run") drops
with each additional unrelated concern folded in. The commit message itself is evidence: it
narrates the storefront feature, then the harness systems, then notes "Telemetry report delivered
in-session; the durable subsystem is a follow-up" — i.e., the author was already tracking a
different follow-up debt (telemetry) at the moment the ledger-row debt should have been caught, and
didn't cross-check against the ratchet rule for the concern that had shipped earlier in the same
diff.

This is the third recurrence of the exact same law (#48, swarm-mergeback-rot, this) — which is
itself worth noting: a "discipline-triggered step dies without a hook" failure keeps recurring in
*new* discipline-triggered steps (ledger rows, sandbox merge-back, and originally the
reflection→lesson chain itself) faster than each individual recurrence gets hook-enforced. The
`guardrail-ledger-integrity.mjs` check added for #48 verifies row *numbering*, not row
*completeness* against touched files — so it did not, and structurally cannot, catch this specific
instance either.

## CONFIDENCE

High on the observation (diff stat and commit message read directly); medium on "commit
bundling breadth" as the dominant cause vs. a contributing one — plausible confound: end-of-session
fatigue after landing 6 concerns could equally explain a missed step regardless of bundling.

## NEXT-TIME

Prefer one concern per commit where the concerns are genuinely unrelated (storefront feature vs.
harness process docs vs. dev tooling) — not for style, but because the end-of-commit "am I done"
checklist has to be run once per concern to reliably catch concern-specific requirements like a
ledger row. If concerns must be bundled (e.g., time pressure), explicitly re-run the ratchet
checklist once per distinct concern before considering the commit done, rather than once for the
whole diff.

## PROPAGATE (candidate — advisory; librarian/ratchet decides)

Not promoted to a new lesson on its own — it is the third instance of the already-lessoned
"discipline-triggered step dies" law (#48 → guardrail-sandbox-staleness.mjs + meta-controller.mjs
already exist as its deterministic response). A genuinely new deterministic artifact here would be
a gate that diffs touched-`src`-with-new-tests against ledger-row presence per commit — flagged as
a candidate for `docs/governance/HARNESS-IMPROVEMENTS.md` if a future recurrence makes a fourth
instance of this specific sub-pattern (ledger-row-orphaned-by-bundling) appear.

## LINK

`docs/regressions/REGRESSION-LEDGER.md` #48, #68 · commit `b536ca07` ·
`docs/reflections/INBOX/2026-07-03-swarm-mergeback-rot.reflection.md` (same law, different step)
