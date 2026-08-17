---
name: fable-method
description: think/act/prove/grow protocol for any non-trivial task.
license: MIT
---

# The Fable Method — think · act · prove · grow

A distilled, literal work protocol (source: `docs/research/AI-TOOLS-COMPENDIUM-2026-08.md`
§III.5, repo `Sahir619/fable-method`, MIT). The core insight: most agent
instruction files say **what to value** ("be careful, check your work"). This
says **what to do**, step by step — so a weaker model can follow it literally.

## think — before touching anything
1. **Classify** the request in one line: what *kind* of task is it (bug fix /
   feature / migration / investigation / refactor)? Write it down first.
2. **Name "done"**: state the exact command / test / output that proves the task
   is finished. If you cannot name a verification, you are not ready to start.
3. **Gather evidence in parallel from primary sources**: read the real files,
   run the real commands, fetch the real URLs. Never work from memory or
   assumption when the source is reachable.

## act — the smallest correct change
1. **Commit to one approach.** Do not hedge across alternatives; pick one and go.
2. **Change the smallest correct thing**: the minimal diff that satisfies the
   goal. No drive-by refactors, no scope creep.
3. **Verify by observation**: run it, read the actual output. Do not assume it
   worked — capture the evidence.

## prove — report honestly
1. **Result first**: state what happened, with concrete numbers / evidence,
   before any narrative or explanation.
2. **Caveats once, briefly**: what you did not verify, what could be wrong.
   Lead with the result, follow with the caveat — never bury the limitation.

## grow — distill the domain
When a non-trivial task is solved in a new domain, distill the approach into a
reusable adapter / skill — the same way this method was distilled from a
stronger model. Every hard win should make the next run cheaper.

## Verification rule (the blind judge)
Verify by **diffing + executing**, never by reading reports. Every claim carries
a named, runnable verification. A result without evidence is a hypothesis, not a
result.

## Provenance
Validated by the original author across 15 eval rounds and 260+ agent runs;
blind LLM judges verified by diffing and executing code, not reading reports
(each case in `eval/cases/`, log in `eval/RESULTS.md`).
