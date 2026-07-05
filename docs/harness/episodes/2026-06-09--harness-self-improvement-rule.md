# Episode: 2026-06-09--harness-self-improvement-rule

- **model**: deepseek-v4-flash-free / opencode
- **task**: Encode the harness self-improvement loop (RHO-adapted) as a permanent always-on rule
- **actions**:
  1. Surveyed existing harness infrastructure (rules, skills, workflows, memory, gates)
  2. Checked referenced docs existence (2 of 5 existed)
  3. Built failure-mode ledger from audit sweep + recent session
  4. Created `.agents/rules/harness-self-improvement.md` — the loop rule
  5. Created `docs/harness/model-rotation.md` — rotation registry
  6. Created `docs/harness/failure-mode-ledger.md` — priority queue with 14 entries
  7. Created `docs/harness/episodes/` — episode store with README
  8. Wrote this episode entry
- **diffs**: 5 new files, 0 product code changed
- **gate_results**: N/A (harness-only, no product gates run)
- **interventions**: none
- **diagnose**: systemic — harness lacked a dedicated self-improvement mechanism and failure-mode ledger
- **health**: 6 tool calls, 5 files written, 0 re-edits
- **verdict**: passed
