---
description: llm-wiki style lint over bebop-lang's ROADMAP / TASKS / SESSION-HANDOFF / exp.journal — contradictions, stale claims, orphan tasks. Proposes edits, changes nothing.
---
Lint the bebop-lang planning docs. Read bebop-lang/ROADMAP.md ("Critical path" + "Open decisions"), bebop-lang/TASKS.md (generated from HISTORY.md headers — never propose editing the table itself), bebop-lang/docs/SESSION-HANDOFF.md, and the last 40 lines of bebop-lang/docs/exp.journal, plus `git log --oneline -15`.
Report, one line each, grouped: (1) contradictions between the four (a task DONE in one and open in another, a number that differs between two docs, a HEAD/fixpoint hash that is stale); (2) stale claims (a "next" that already happened per the journal or git log; a measurement the journal has since refuted); (3) orphan tasks (in TASKS/HISTORY but in no roadmap step, or in a roadmap step but not in HISTORY); (4) journal lines without a VERDICT or with VERDICT:pending older than a day.
For each: the file:line to change and the one-line replacement. Change nothing; end with the count per group.
