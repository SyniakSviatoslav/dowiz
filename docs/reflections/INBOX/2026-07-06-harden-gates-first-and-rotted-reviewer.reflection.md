# Reflection — harden the gates before the red-line work, and treat reviewer-context-health as part of verification

**Date:** 2026-07-06 · **Slug:** harden-gates-first-and-rotted-reviewer
**Qualified because:** ≥3 code/harness files + red-line-adjacent (money-core sequencing, harness gate edits).

## CONTEXT
After shipping the OG previews and authoring the Sovereign Core MVP grand plan, I executed Phase 0
autonomously: token/harness optimization + the SAFE core-hardening, then reached the crown-jewel money
extraction (`pricing.rs`) and stopped before it. Operator granted all permissions + asked me to
auto-finish and auto-continue in fresh sessions.

## DECISIONS
1. **Harden the deterministic gates BEFORE the red-line structural work** — banned f64/f32 in the pure core
   and wired cargo-deny into the sovereign gate *first*, so the eventual `pricing.rs` extraction runs UNDER
   that gate protection (a float leak or bad crate fails the gate, not staging/prod).
2. **DEFER the money extraction to a fresh, focused context** with a DECORRELATED (independent, hand-derived,
   non-mirror) money oracle — rather than verify 884 lines of money byte-parity from a very deep session.
3. **Unlock the harness surgically** (operator-approved, human-run `!` bootstrap): opened `.claude/*` while
   keeping migrations/.env/db/contracts/.github/lockfile + the human-only `.claude/state` override files
   protected; then SEALED the guard-bash inline-interpreter-write hole I had flagged (+ armament test).

## WHERE
`rebuild/crates/domain/clippy.toml`, `rebuild/deny.toml`, `rebuild/scripts/sovereign-gate.sh`,
`.claude/hooks/{protect-paths,guard-bash,route-request}.sh`, `docs/regressions/LEDGER-INDEX.md`,
`docs/design/sovereign-core-mvp/*`.

## WHY (causal)
1. **Deterministic gates must PRECEDE the risky work they protect.** Adding the f64 ban + cargo-deny before
   touching money means the extraction is born inside a stricter gate — the gate catches the defect class
   at compile/check time instead of the defect reaching a live money surface. Hardening-last would let the
   f64/replay hazard the extraction introduces slip through the very window it exists to close.
2. **A rotted reviewer context is itself a red-line failure mode — same root as #56.** Regression #56 (the
   inclusive-tax double-charge) shipped "certified green" because a MIRROR oracle verified the code against
   itself. Verifying money byte-parity from a 400K-token session is the same shape: the *verifier* is the
   compromised proxy, not the code. The antidote is DECORRELATION (an independent oracle in a fresh context),
   which is a property of the *reviewer*, not of trying harder. So "reviewer-context health" belongs in the
   verification plan, not just the test suite.
3. **guard-bash is a command-TEXT gate — structurally blind to external scripts.** It catches inline shell
   mutators/redirects and now inline interpreter writes (`python3 -c "...write_text('.env')"`), but a
   `python3 file.py` that writes a protected path is uninspectable — same limit that always applied to
   `bash file.sh`. Sealing the inline hole is real; the external-script gap is inherent to the layer, not a
   bug to "fix" there. Narrowing (require `-c`/`-e`/heredoc) removed a prose false-positive without losing
   the deny (armament-tested deny=2 / prose=0 / read=0).

## CONFIDENCE
HIGH on (1) and (2) — (2) is directly evidenced by #56 + the whole live-surface failure root. MEDIUM on the
guard-bash seal's completeness (inline sealed + tested; external-script writes inherently open at that layer).

## NEXT-TIME
- Sequence the deterministic gates that police a defect class BEFORE the change that could introduce it.
- Never establish red-line (esp. money) correctness from a deep/rotted context — decorrelate to a fresh,
  independent verifier with a hand-derived (non-mirror) oracle. Put "is the reviewer's context healthy?"
  on the verification checklist.
- When sealing a command-text gate, state the inherent blind spot (external scripts) rather than implying
  completeness; armament-test deny + non-over-block.

## LINK
[[sovereign-core-mvp-handoff-2026-07-06]] · docs/design/sovereign-core-mvp/LEAD-REVIEW.md ·
REGRESSION-LEDGER #56 (mirror-oracle) · reflection 2026-07-06-og-preview-cutover-stack-mismatch (same
proxy-not-real-verification family) · docs/lessons/2026-07-02-gate-state-file-expiry.md (armament test).
