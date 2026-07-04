# Reflection: plane-maintainer trigger probe on new dowiz-maintainer env

## CONTEXT
Continuation of the cloud-claude setup session ([[claude-cloud-github-authz-2026-07-02]],
[[plane-maintainer-agent-2026-07-02]]). Prior state: routine `plane-maintainer`
(trig_01DgtaGih6VQVRNsKfgKMVBh) worked for checkout but the trigger was believed to still point
at the secrets-less old env `env_01He3unMkxVs2F2yBgBjYwRH`; a new env `dowiz-maintainer` with
staging secrets existed but was thought un-wired.

## DECISIONS
- Inspected the trigger via `RemoteTrigger get` BEFORE acting — discovered it was ALREADY
  repointed to `env_015taQQGBFyF2XLuRRUwUgfd` (updated 13:39 UTC, after the 12:34 firing),
  so no update call was needed.
- Fired `RemoteTrigger run` as the probe (the definitive end-to-end access test from the debug
  map) rather than trusting the picker or the env list. Result: HTTP 200, session
  `cse_01Psnn1SgcmYwwqpZoyE1xtv`.
- Accepted that the probe executes the FULL daily routine — by design the routine self-escalates
  safely if secrets are absent, so the run doubles as the secrets test.
- Updated the plane-maintainer memory in place (no duplicate memory) with env id + probe result.

## WHERE
- claude.ai `/v1/code/triggers` (remote state, no repo files changed by this session)
- `/root/.claude/projects/-root-dowiz/memory/plane-maintainer-agent-2026-07-02.md` (updated)

## WHY-causal
The prior session's memory recorded a *stale hypothesis* ("trigger may still point at the OLD
env") as if it were an open action item. The causal root of the near-miss: remote mutable state
(trigger→env binding) was cached in memory prose instead of being re-read at session start.
Because I ran `get` before `update`, the stale note cost only one API call; had I updated
blindly, I could have clobbered the operator's 13:39 repoint (idempotent here, but the pattern —
act-on-cached-remote-state — is the class that breaks). This is the memory-corpus pattern
"verify-artifact-not-proxy" applied to remote API state: the memory is a proxy, `get` is the
artifact.

## CONFIDENCE
High on the wiring conclusion (200 + session_id is the documented definitive signal).
Medium on secrets-reach-checkout — that is only provable inside session
cse_01Psnn1SgcmYwwqpZoyE1xtv's transcript; explicitly flagged as the open verification.

## NEXT-TIME
- Any memory note about remote mutable state (triggers, envs, fly secrets, DNS) must be treated
  as a hypothesis to re-read, never a fact to act on — always `get` before `update`/`run`.
- When writing memories about in-flight operator actions, phrase them as "as of <time>, X;
  re-check" rather than "X — do Y", so the next session probes instead of executes.
- A probe that runs a full autonomous routine is acceptable only when the routine's fail-safe
  (self-escalation on missing secrets) is already proven; otherwise use a minimal probe prompt.

## LINK
- [[plane-maintainer-agent-2026-07-02]]
- [[claude-cloud-github-authz-2026-07-02]]
- [[memory-corpus-meta-patterns-2026-07-02]] (verify-artifact-not-proxy)

---

**Curation note (librarian, 2026-07-04 drain pass):** CHALLENGE — is this a genuine recurring
root, or a benign one-off? The WHY (act on cached remote-state-in-memory instead of re-`get`ing
it) is real and correctly reasoned, but two things weigh against promotion: (1) the outcome was
idempotent (one wasted `get` call, zero side effect — the operator's 13:39 repoint was never
clobbered); (2) the underlying pattern is already a NAMED, established meta-pattern
(`verify-artifact-not-proxy`, `memory-corpus-meta-patterns-2026-07-02`) with its own deterministic
instance already gated in `scripts/plane-guard.mjs` (`P1/P2 verify-artifact-not-proxy` — no
commit/deploy piped to tail/head/grep). This reflection is a second, milder manifestation
(memory-of-remote-API-state, not a masked exit code), not a new bug class. NOT distilled into
`docs/lessons/`: the trigger surface is a remote `claude.ai` trigger/env API call, not a repo file
`pre-edit-lessons` could ever match on Edit/Write/MultiEdit — a lesson here would be permanently
dead weight (never inject), violating "store must not grow." PRUNED — folded into the
already-established `verify-artifact-not-proxy` pattern rather than a new artifact. Archiving.
