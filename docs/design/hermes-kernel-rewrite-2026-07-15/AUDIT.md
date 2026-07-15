# Hermes Audit — 2026-07-15

> Four independent research passes (empirical usage-history analysis, source-architecture review,
> search/execution code review, plus direct investigation of the memory/verification layer),
> synthesized here. Every claim below traces to a file:line citation or a quoted log line — nothing
> in this document is a guess. Written the way you'd want an older sibling to actually tell you:
> not softened, not performatively harsh either — just what's true, and what to do about it.

## The honest headline

Hermes is not badly built. Across four independent reviews, the parts that are *hard* to get
right — subprocess safety, cross-platform quirks, non-destructive data compaction, tiered
tool-output handling — turned out to be genuinely mature. The process-group kill discipline, the
incremental-UTF-8-safe output draining, the Windows compat layer, the spill-to-disk tool-result
tiering: these are better than typical for an open-source agent tool, and said so plainly by
reviewers whose whole job was to find fault.

The failures don't cluster where you'd expect ("it's slow," "it's fragile"). They cluster in one
specific *shape*, and it repeats across all four investigations independently:

**Everything that requires a systemic, self-enforcing invariant is implemented as a soft nudge
instead of a hard rule — and everything that's pure, deterministic bookkeeping logic gets
duplicated slightly-wrong across multiple code paths instead of written once and proven correct.**

That's not a vibe. Here's the evidence for both halves.

## Half one: soft nudges where hard invariants belong

- **`agent/verification_stop.py`'s own docstring says it outright**: *"This module is intentionally
  policy-only. It never runs checks itself; it turns the passive verification ledger into a bounded
  follow-up."* Hermes already collects real verification evidence — `terminal_tool.py` and
  `file_tools.py` both write to a SQLite ledger (`verification_evidence.db`) recording what actually
  ran and its exit code. The data exists. It's just never made *authoritative* — a turn can still
  claim "done" without the ledger agreeing.
- **The empirical usage log confirms this is not theoretical.** ~30 correction instances across 8
  days of real use, clustered around exactly this failure mode. The operator's own words, unprompted,
  in the log: *"a lot of time was spent because of the falsified statements. There is an urgent need
  to fix this issue pattern now and never repeat it in the future."* A four-correction cluster in 15
  minutes on 07-09 (lines 401-410) is a false-green CI claim, corrected four times before it actually
  held.
- **Model routing is availability-first, never quality-aware** — confirmed by an explicit comment in
  the code itself (`models.py:1283-1287`) explaining the fallback chain was deliberately built to
  never auto-escalate to a stronger model, for cost-safety reasons. Reasonable goal, wrong dial: the
  same mechanism that stops it from accidentally spending on a flagship model also means a hard
  reasoning task that hits a rate limit silently lands on a *weaker* free model with zero signal that
  quality just degraded. Six of seven models in the configured fallback chain are free-tier.
- **`shell_hooks.py`'s policy gates fail open** — a hook that errors or times out returns `None`,
  which means "allow," not "block." If a hook exists specifically as a safety gate, a wedged hook
  silently disables it.
- **`flush_min_turns: 6` is present in this operator's actual config and does nothing** — zero code
  in the repository reads it. It documents a protection ("flush memory before it's compressed away")
  that doesn't exist for the built-in memory store; the hook that could serve this purpose only fires
  for third-party plugin memory providers, none of which are configured here.

## Half two: pure logic, duplicated, each copy slightly wrong

- **Pagination/truncation math is implemented at least four separate times** in the search path
  (content mode, files-only mode, count mode, and the grep fallback), and the primary one is
  provably broken: `total_count` is computed from a payload that was already clipped to
  `limit + offset` lines before counting, so it can mathematically never exceed `offset + limit`.
  The `truncated` flag that's supposed to tell the model "there's more, page further" can never fire
  in the common case — the model is told "50 total matches" when there were 200, and believes it saw
  everything.
- **The same duplication pattern shows up in retry/dispatch logic**: the concurrent tool-dispatch
  path has a proper 420s batch deadline; the *sequential* dispatch path — same job, different code
  path — has no deadline at all, and the heartbeat mechanism meant to detect hangs actively defeats
  itself here (it keeps signaling "alive" while a tool call with no internal timeout blocks forever).
- **Checkpoint resume conflates duplicate prompts** by keying completion on stripped message text
  rather than `(text, index)` — a dataset with two identical prompts (common when sampling different
  toolsets against the same task) marks both "done" the instant one finishes. Silent lost work, not a
  crash — the worst kind of bug because nothing tells you it happened.
- **A retry path can double-execute non-idempotent commands**: any non-timeout exception from a
  remote backend gets blindly retried up to 3 times, including *after* the command may have already
  started mutating state. `rm`, `git push --force`, a DB write — retried blind.

## What's actually good, stated plainly (an honest audit says this part too)

The subprocess/execution safety floor is genuinely well built: every terminal command runs through a
monotonic-deadline, process-*group* kill (not just the leading process), with interrupt-safety and
group-exit verification. Tool-output handling is a real three-tier defense — per-tool char caps,
spill-to-disk for anything over 100K chars (replaced in-context by a preview and a re-readable file
path — this is the standout design choice in the whole codebase), and a per-turn aggregate budget
that spills the largest results first if several medium ones would overflow together. `max_turns`
hitting its ceiling degrades gracefully (one final summarization call, session persisted, explicit
"send `continue`" hint) rather than truncating mid-thought. Compaction is non-destructive — archived
messages stay FTS5-searchable, nothing is actually deleted. The Windows subprocess-compat layer
justifies every flag combination in its own comments and is defended by tests that check *exact*
call-site kwargs. Cold start was directly measured on this install at 0.24-0.81s — genuinely fast.

None of that needs to be rewritten. Rewriting well-engineered code because a rewrite is underway
would be exactly the kind of unforced, novelty-seeking change a careful engineer avoids.

## The one-sentence diagnosis

Hermes trusts itself more than its own evidence — it collects the data to know when it's wrong
(verification ledger, error taxonomy, rate-limit signals) and then treats that data as advisory
instead of authoritative, while the parts that should be simple, provably-correct arithmetic
(pagination bounds, timeout budgets, dedup keys) get re-derived ad hoc at each call site instead of
being centralized and proven once. Both problems have the same fix, which is the subject of
REWRITE-PLAN.md: move the *decision* logic — not the I/O, not the subprocess handling, not the
things already done well — into one deterministic core that can't be talked past, with the existing
(good) I/O layer kept exactly as-is around it.
