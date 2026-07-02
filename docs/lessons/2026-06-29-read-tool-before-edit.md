---
TRIGGER: File has not been read yet
CAUSE: >
  Inspecting a file via Bash (`cat`/`sed`/`grep`) before calling Edit does not satisfy the
  Edit/Write tool's read-precondition — only a Read tool call primes it. Reaching for Edit
  right after a Bash-only peek fails with "File has not been read yet" and costs a wasted
  cycle (re-read, then retry the edit).
ACTION: >
  When the target of your NEXT tool call is Edit or Write → cause: Bash cat/sed/grep does not
  prime the Edit precondition → do: open the file with the Read tool first (even for a quick
  peek), not Bash. Reserve Bash sed/grep/cat for scan-only, never-edit inspection (e.g. counting
  matches, checking existence) where you will not follow up with an Edit on that file.
LINK: docs/reflections/ARCHIVE/self-improve-2026-06-29.md (Issue 2) ; .claude/CLAUDE.md "Read before edit"
SCOPE: Any file the agent is about to Edit or Write. Restates/operationalizes the existing
  CLAUDE.md "Read before edit" rule as an error-signature-keyed nudge — not a new rule.
STATUS: active
---

# Read (tool), not Bash cat, any file you will Edit

Source: reflection `self-improve-2026-06-29.md` (Issue 2), 3 wasted Edit cycles in one session.

The habit of using Bash `sed -n`/`cat` for a cheap one-shot peek at a file, then reaching for
Edit, fails: Edit's read-precondition is tracked against the Read tool specifically, not against
"has this content been seen in the transcript." The fix is procedural, not code: if the file is
about to be edited, use Read — even for a one-line peek. Bash inspection stays fine for
scan-only paths (grep counts, existence checks) that will not be followed by an Edit call in the
same turn.
