# Reflection — self-inspection (2026-06-29)

**WHAT:** Ran the tooling-integration-eval council + 4 build phases (corpus/G1, oh-my-mermaid/G3,
DeepEval+G5, Skyvern/G4) + deploy/verify; then self-inspected.
**OUTCOME:** shipped; one harness defect fixed; behavioral lessons captured.

## Issue 1 — red-line hook false-positived on the regression ledger (4× in one session)

**WHERE:** `.claude/hooks/post-edit-gates.sh::red_lines()`.
**WHY (causal root):** the gate greps the **entire edited file** for code-behavior red-line patterns
(`document.cookie`, `parseFloat…price`, `customer_phone`, …). The regression ledger's *purpose* is to
**document** those very bug classes in prose, so every guardrail row I added re-tripped it. The gate
keyed on **content shape** without considering **file role** — markdown is never executed or shipped,
so a code-behavior red-line cannot exist there. A gate that flags the document describing a pattern is
mis-scoped (CLAUDE.md: "narrow it until it only catches the regression").
**FIX:** exempt `docs/*` + `*.md`/`*.mdx`/`*.markdown` from the content scan; unchanged for all code.
Proven red→green: ledger edit → exit 0; `document.cookie`/`parseFloat…price`/`customer_phone` in a
`.ts` → still RED-LINE exit 2.

## Issue 2 — wasted 3 Edit cycles on "File has not been read yet"

**WHY (causal root):** I inspected files with `Bash sed/cat` (cheap, habitual) and then reached for
`Edit`, which requires the file to have been read via the **Read tool** specifically. Bash reads don't
satisfy that precondition. The habit optimized for one-shot inspection but cost a re-read each time I
then needed to edit.
**HOW TO APPLY:** when I intend to edit a file, open it with **Read** (not Bash), even for a peek —
the Read also primes the Edit. Reserve Bash `sed`/`grep` for scan-only/never-edit inspection.

## Issue 3 — first commit timed out (pre-commit Docker build > 2 min)

**WHY:** the pre-commit hook runs a full Fly Docker build; the default 2-min Bash timeout killed it
mid-build. Not a defect — expected. **HOW TO APPLY:** commits on this repo need a long timeout
(≥7 min) or a background run from the first attempt.

## PROPAGATE TO
- [guardrail] the hook narrowing is itself the deterministic fix (red→green proven); it strengthens
  signal-to-noise without weakening any code path. Reversible via git.
- [lesson] "Read (not Bash) any file you will Edit" — candidate pre-edit lesson.
- [lesson] "regression ledger / docs edits never need a staging deploy; they have zero runtime surface"
  — already applied this session (no staging deploy for CI-gate/doc-only phases).

_Advisory: the librarian/worker enacts; do not auto-edit sibling surfaces._
