# BLUEPRINT H4 — Self-Governance Ritual: a cheap interim firing mechanism

> **Anchor:** Gender **V-1 / V-2** × Rhythm **F5 / F6**; root cause **RC-2 — Verification organs
> without independent teeth** (ranked-table rows #16, #27, adjacent to #2, #7, #15).
> **Depends-on:** conceptually related to — but **NOT blocked by** — `BLUEPRINT-P06`
> (key_V independent re-executor), `BLUEPRINT-P01 §2.7` (claim-latency ledger appender), and
> `BLUEPRINT-P08 §4` (claim-latency anomaly consumer that feeds `FalseClaimMeter`). This is the
> **cheap interim layer**, not a substitute for any of those three.
> **Status: PROPOSAL ONLY — requires the operator to apply.** `.claude/settings.json`,
> `.claude/hooks/**`, and `AGENTS.md` hook-adjacent config are **protected paths** in this repo per
> the governance-gate-topology rule; an agent cannot self-edit them. The only sanctioned unlock is the
> operator's own `! <cmd>`. This document specifies what to apply; it applies nothing.

---

## §0 — The problem: a MANDATORY ritual with no structural trigger

The repo carries two governance rituals declared binding on every agent: the **2-question doubt check**
(`AGENTS.md:121-158`) and the **Detailed Planning Protocol** (`AGENTS.md:160-221`). Both are labelled
mandatory — the doubt check verbatim says *"MANDATORY, not optional — at three points, not just at the
end."* Neither has any mechanism that makes it run.

The gap is exact and structural: **a mandatory ritual whose only trigger is the agent remembering to run
it.** On any forgetful turn it silently degrades to "whenever the agent happens to think of it," and its
absence is indistinguishable from its presence — nothing records that it was skipped.

Why this matters is not rhetorical; it is the repo's own standard turned on itself. Ananke
(`AGENTS.md:243-250`) demands that *"anything that matters for long-term health should not depend on the
maintainer remembering to do it… structurally inevitable."* The doubt ritual exists **specifically to
catch mistakes before they compound across a session's stages** — yet its own firing is remembered, not
inevitable. **The organ built to enforce the standard fails the standard it enforces.** Rhythm Finding 5
names this as "a pendulum you have to push by hand every swing"; Gender V-1/V-2 name the deeper cousin —
the "done" gate and `FalseClaimMeter` read author-supplied evidence. H4 does not fix the deep cousin (P06
does). H4 fixes the cheapest, nearest layer: give the ritual a **structural trigger and a durable record**
so that "was it run?" is answerable at all.

This is a **chosen** gap (governance hooks were suspended by operator directive 2026-07-15 for full
self-management), so H4 is written as an *offer the operator can accept*, not a defect report. It is
designed to be re-enablable without reversing that directive — it adds friction only at plan-authoring
time, touches no product/CI/red-line path, and blocks nothing by default (warn-only mode is the default).

---

## §1 — Current-state evidence (re-verified live, 2026-07-16)

**Hooks configuration — confirmed empty.** `.claude/settings.json` in full is a `permissions` block with
**no `hooks` key**:

```json
{ "permissions": { "allow": ["Bash(*)","Edit(*)","Write(*)","Read(*)","Glob(*)","Grep(*)"], "deny": [] } }
```

All twelve scripts in `.claude/hooks/` (`protect-paths.sh`, `serious-gate.sh`, `verify-safety-floor.sh`,
`red-line-doubt-gate.sh`, `guard-bash.sh`, `require-classification.sh`, `route-request.sh`,
`post-edit-gates.sh`, `pre-edit-lessons.sh`, `loop-detector.sh`, `attractor-stop.sh`) are identical
no-op pass-throughs:

```bash
#!/usr/bin/env bash
# HOOK DISABLED 2026-07-15 — operator directive: remove ALL governance gates / red-line
# friction for full self-management of the repo. No-op pass-through (exit 0).
exit 0
```

The pre-disable shape (git `31810b389:.claude/hooks/protect-paths.sh`) is the template §2 mirrors: reads
the tool-event JSON from stdin, extracts `.tool_input.file_path` via a jq→python3→python→node fallback
chain, and gates on the path. **The audit claim is re-verified true: zero firing mechanism exists.**

**AGENTS.md ritual text — verbatim, load-bearing excerpts.** The doubt check (`:121-124`):

> *"## Session/plan closing ritual — the 2-question doubt check (operator, 2026-07-16) — **MANDATORY,
> not optional — at three points, not just at the end**… 1. During planning… 2. During research…
> 3. During blueprint organization."*

The two questions (`:141-148`): *"1. What are you least confident about right now? List 6-7 concrete
things you did not properly investigate… 2. What's the biggest thing I'm missing about the situation?"*

The Planning Protocol's own hooks note (`:212-219`) already concedes the gap and names the constraint:

> *"the operator asked for rules **and** hooks. Steps 1-8 above are the rule… A literal git-hook/CI
> enforcement… is a legitimate follow-up, but `.claude/` config is a protected path this session does not
> self-edit — per the standing governance gate-topology rule, that unlock is the operator's own
> `! <cmd>`, not an agent action. **Flagged here as the concrete next step if literal enforcement is
> wanted.**"*

H4 **is** that flagged concrete next step, specified.

---

## §2 — Target-state design (concrete, cheap, honest about limits)

**What a hook CAN and CANNOT do — stated up front.** Claude Code hooks fire on **tool events**
(`PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`), never on cognition. A hook **cannot** verify the
agent genuinely reasoned through the two questions — no structural signal proves a thought occurred.
A hook **can** reliably observe **structural residue** of the ritual: does a self-critique artifact exist
*alongside* a newly written plan? Was a decorrelated verifier subagent dispatched? These are the only
enforceable signals, and H4 is built entirely on them. The design substitutes *"did the agent leave the
structural trace the ritual requires?"* for the unanswerable *"did the agent think?"*

### (a) A `PostToolUse` (and `Stop`) plan-artifact hook — `require-selfcritique.sh`

Trigger: `PostToolUse` matching `Write|Edit` whose `.tool_input.file_path` matches
`docs/design/**/*{BLUEPRINT,ROADMAP,PLAN}*.md` (extract path exactly as `protect-paths.sh` does). On a
match, the hook checks for the structural residue of the ritual and **warns** (default) or **blocks**
(strict mode, operator-set) if it is absent. Residue = **any one of**:

1. a co-located sibling file `*SELF-CRITIQUE*.md` / `*2Q-AUDIT*.md` in the same directory (the pattern
   `SELF-CRITIQUE-2Q-DOUBT-AUDIT.md` already used in `sovereign-roadmap-2026-07-16/`), **or**
2. a `## Self-critique` / `## 2-question` / `least confident` heading grep-present **inside** the written
   doc itself, **or**
3. a `Stop`-hook-observable record (see (b)) that a decorrelated verifier subagent was dispatched for
   this doc.

Honest limit, stated in the hook's own header comment: this proves a *self-critique section exists*, not
that it is *honest or non-trivial*. It defeats silent omission (the actual Finding-5 failure mode — the
ritual simply not run), not a bad-faith stub. That is a deliberate, worthwhile floor: the observed
failure is forgetting, not forging. A `Stop` hook variant runs the same check across all plan docs a
session wrote, as the session-close backstop the ritual's point-3 ("blueprint organization") calls for.

### (b) A minimal claim-latency-adjacent local log — `docs/ledger/ritual-run.jsonl`

Distinct from — and a **stopgap until** — the full P01 ledger (`docs/ledger/claim-latency.jsonl`) and P08
consumer. One JSONL line **appended by the hook** (not by the agent) per completed plan doc:

```json
{"ts":"2026-07-16T15:40:00Z","doc":"docs/design/.../BLUEPRINT-H4.md","self_critique_present":true,"decorrelated_agent_dispatched":false,"mode":"warn"}
```

**Why the hook appends it, not the agent** — this is the load-bearing honesty point. A log the agent
writes about its own ritual is self-certified: it reproduces Gender V-2 exactly (the audited party
asserting `verified`). The value comes **only** from the hook computing `self_critique_present` by
grepping the file itself — an independent read of structural fact, not the agent's claim. The
`decorrelated_agent_dispatched` field is populated from a `SubagentStop` observation, not agent
self-report. This gives a durable, low-cost record of *whether the ritual left its trace* — a first,
honest teeth on the ritual — before P01's real per-commit ledger lands. When P01 ships,
`ritual-run.jsonl` is retired into `claim-latency.jsonl`; H4 explicitly does not want two ledgers.

---

## §3 — Migration steps (what the operator would need to approve/apply)

An agent cannot perform any of these — every step touches a protected path.

1. **Operator unlock.** Via `! <cmd>` (the sanctioned self-mod unlock), or by hand, add the two hook
   scripts under `.claude/hooks/` (`require-selfcritique.sh`, plus its `Stop` variant), mirroring the
   `protect-paths.sh` stdin-JSON/path-extraction template preserved at git `31810b389`.
2. **Register in `settings.json`.** Add a `hooks` block: `PostToolUse` matcher `Write|Edit` →
   `require-selfcritique.sh`; `Stop` → the session-close variant. Ship in `mode:"warn"` (exit 0 always,
   log + advisory message) so nothing is blocked until the operator opts into strict mode.
3. **Create the ledger sink.** `git add` an empty `docs/ledger/ritual-run.jsonl` (+ `.gitkeep` on the
   dir) so the append target exists on a fresh checkout — otherwise the schedule/record is host state, the
   very Rhythm-Finding-2 anti-pattern.
4. **Document the mode switch.** Note in `AGENTS.md:212-219` (operator edit) that literal enforcement now
   exists in `warn` mode and how to flip to `block`. This closes the "flagged as next step" loop in-place.
5. **Retire-on-P01.** When `BLUEPRINT-P01 §2.7` lands `claim-latency.jsonl`, fold `ritual-run.jsonl`'s
   `self_critique_present` signal into it as one more per-commit column and delete the interim sink.

---

## §4 — Acceptance criteria (numbered, falsifiable)

1. `.claude/settings.json` contains a non-empty `hooks` block registering `require-selfcritique.sh` on
   `PostToolUse:Write|Edit` and a `Stop` handler. (`jq '.hooks' settings.json` ≠ `null`.)
2. Writing a new `docs/design/**/*BLUEPRINT*.md` with **no** self-critique section and **no** sibling
   audit file emits an advisory in `warn` mode / a non-zero exit in `block` mode. (Reproduce with a
   throwaway doc; observe the message.)
3. The same write appends exactly one line to `docs/ledger/ritual-run.jsonl` with
   `self_critique_present:false`. (`tail -1` shows it.)
4. Writing a plan doc **with** a `## Self-critique` heading appends a line with `self_critique_present:true`
   and emits no advisory. (Positive-path reproduction.)
5. `self_critique_present` is computed by the hook grepping the file, never read from a tool_input field
   the agent supplied. (Code inspection: the value's only source is a `grep` of `$FILE`.)
6. Fresh `git clone` carries the hook scripts, the `settings.json` block, and an existing (possibly empty)
   `docs/ledger/ritual-run.jsonl` — the mechanism is reproducible from canon, not host state.
7. Default mode blocks nothing (`exit 0` on every path in `warn`); strict mode is an explicit operator flag.

---

## §5 — What this does NOT fix, and how it composes

**H4 is a cheap partial mitigation, not the deep fix.** It is explicit about its ceiling:

- **It does not give the ritual independent teeth (Gender V-1).** It proves a self-critique *artifact*
  exists; it cannot verify the reasoning is honest or that an independent party did the checking. The real
  fix is **P06's key_V independent re-executor** (`key_K ≠ key_V`, fresh-worktree re-execution) — the only
  mechanism that makes the passive verifier independent of the active author. H4 buys a forgetting-floor
  while P06 is unbuilt; it is not a substitute and must not be cited as closing V-1.
- **It does not build `FalseClaimMeter`'s real feed (Gender V-2 / RC-2 #7, #27).** `ritual-run.jsonl` is a
  binary "did the trace exist" record, not the per-commit authored→CI-green latency the meter needs. That
  is **P01 §2.7** (appender) + **P08 §4** (anomaly consumer). H4's `self_critique_present` column is a
  *forward-compatible stub* that P01 absorbs, not the ledger itself.
- **It does not cover V-3** (independent peer breach probe) at all — out of scope.

**Composition when the deep layers land:** H4 is the interim rung on the same ladder. `ritual-run.jsonl`
retires into P01's `claim-latency.jsonl` (§3.5). H4's `decorrelated_agent_dispatched` signal becomes
redundant once P06's key_V is the mandated verifier — at that point the residue H4 greps for is upgraded
from "a self-critique section exists" to "a key_V RED|GREEN verdict is signed," and the hook's grep target
changes accordingly. Until then, H4 is the honest, buildable-today answer to Rhythm Finding 5's exact
complaint: **the ritual meant to be structurally inevitable finally gets a structure that fires it** — a
weak structure, named as weak, but no longer merely remembered.
