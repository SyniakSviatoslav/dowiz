# Constitution — SDD GLOBAL HOOK (2026-07-14, binding)

Governing principles for all work in dowiz + bebop/bebop2. Source of truth = live repo +
these rules; plans are desired-state, never ground truth.

1. **Verify with real execution, always.** Every "done" = literal `cargo test` / `node` /
   `playwright` output pasted. Never fake-green. No `.only`, no inflated timeout, no
   `expect(true)`, no commented assert.
2. **Ground truth outranks plans.** Re-verify code claims with grep/git/tests before trusting a
   "done" status. A blueprint's "CURRENT STATE" can be stale — read the file.
3. **SDD pipeline is mandatory** before non-trivial code: constitution→spec→plan→tasks→analyze
   →implement→converge. Each task RED→GREEN on a real test, marked `[LANE:n]` when parallel.
4. **RED→GREEN or not done.** No blueprint is "done" without a deterministic, falsifiable gate
   proven red-then-green. Do not weaken existing gates.
5. **Fewest correct files; fail-closed.** Minimal is good, correct-and-minimal is the bar.
   Malformed input ⇒ refuse, not proceed. Integer-only accounting for tokens/entropy.
6. **Max-lane autopilot, but verified.** Parallel subagents only for collision-free files (own
   worktree each); MAIN re-verifies EVERY lane with literal `cargo test`. doer ≠ reviewer.
7. **Red-line gate.** money/auth/RLS/migrations/crypto/history-rewrite = separate human-approval,
   never in the general flow. Authorized (2026-07-14) for this autopilot run, BUT P10 force-push
   is irreversible → backup ref + verify no real secrets first.
8. **Local-offline-first.** No network deps in the kernel/runtime; std-only crates; no external
   services in the deterministic core.
