# Sandbox-Swarm-Gate (SSG) — harness architecture

> **Status:** design + scaffold (this doc + `loops/registry.md` entry `sandbox-swarm-gate` + `scripts/sandbox-swarm-gate.mjs`).
> **Owner action to enable:** one scoped hook relaxation — see [§9 Operator action](#9-operator-action-the-one-thing).
> **Connects to:** `CLAUDE.md` (Ethics Charter · Agent Discipline · Self-improvement loop · red-line globs),
> the loop system (`loops/registry.md`, `tools/loop-harness/`, `.claude/commands/loop-orchestrator.md`,
> `.claude/agents/loop-architect.md`), the agent roster (`.claude/agents/`), and the ratchet stores
> (`docs/regressions/REGRESSION-LEDGER.md`, `docs/reflections/`, `docs/lessons/`).

## 1. What this is (and what problem it names)

Every repeatable multi-agent process in this harness has independently reinvented the same shape:
carve work into isolated lanes, run agents in parallel at full capacity, then review before anything
lands. This session ran exactly that pattern by hand — worktree-isolated `Agent` lanes fanned out on
the audit remediation (money / authz / FE / RLS), a strong-model review gate over each lane's diff,
merge-on-green. **Sandbox-Swarm-Gate formalizes that working pattern into one reusable loop** so it
stops being re-improvised per task.

The three named parts:

| Part | Is | Is not |
|------|----|--------|
| **Sandbox** | one throwaway git worktree per lane (`.claude/worktrees/<lane>/`), no prod/secret/main reach. Iteration inside is **unrestricted** — edit any file, run any build/test, no per-edit protect-path friction. | a place with real-world reach; a place where the Ethics Charter is suspended; a path that auto-merges. |
| **Swarm** | concurrent subagents via the **already-integrated** `Agent` tool (`isolation:"worktree"`) + the `Workflow` tool for deterministic fan-out. Doers on sonnet, reasoning on fable, review on opus. | an excuse to fan out work that mutates one hot file; a bypass of the one-integration-point rule. |
| **Gate** | the **review/audit boundary** where the real quality + safety + **ethics** gate lives: a strong-reasoning-model review of each sandbox diff **before** merge. Merge-on-green ratchet; reject → back to the sandbox. | a rubber stamp; a place that can be relaxed; a place that can auto-merge a red-line without the standing human approval it already requires. |

The core move is **relocation, not deletion**: the per-edit guards (`protect-paths.sh`,
`serious-gate.sh`, `red-line-doubt-gate.sh`) that today create friction on *every* edit are moved
off the fast inner loop and **re-applied, in full, at the gate on the merge diff**. Speed inside the
throwaway box; the whole guard wall at the door out of it.

## 2. The non-negotiable boundary (read this before anything else)

> **"Unrestricted sandbox" = unrestricted CODE ITERATION inside a throwaway worktree that has no
> real-world reach. It is NOT a relaxation of the rules that matter.**

Concretely, the sandbox being "unrestricted" means, and *only* means:

- Inside `.claude/worktrees/<lane>/` an agent may edit any **product** file (including
  `packages/db/migrations/`, contracts, `package.json`) and run any build/test **without per-edit
  protect-path blocking** — because that worktree is a disposable copy with no path to prod, secrets,
  `main`, or the real branch.

It explicitly does **NOT** mean any of the following, all of which remain in force:

1. **The Ethics Charter is never suspended.** The `CLAUDE.md` Ethics Charter (no AI for
   military/warfare; war is never the only solution; peace for everyone; AI as a collective human
   tool) is a **hard, non-removable gate criterion**. A sandbox diff that violates it is rejected at
   the gate and escalated to a human — no test-green overrides it. The Charter is enforced **at the
   gate**, not on the inner edit; it is not "relaxed," it is *relocated* like every other guard.
2. **No auto-merge of red-lines.** Changes touching auth / money / RLS / PII / secrets /
   `packages/db/migrations/` / bulk-edit still require the gate to pass **and** the standing human
   approval those already carry (`serious-gate` Council clearance, `red-line-doubt-gate` human
   window, `CLAUDE.md` Ship Discipline "prod only on explicit approval"). The gate does not mint that
   approval; it *checks for* it.
3. **The guards are relocated, not deleted.** `protect-paths` / `serious-gate` /
   `red-line-doubt-gate` / `invariant-guardian` / `security-sentinel` all run **at the gate** over the
   merge diff. Relaxing them *inside* the sandbox is only sound **because** they run at the door out.
   Relaxing the sandbox without the gate wired = deleting the guard. The two are one indivisible pair
   (see §9).
4. **The harness itself stays protected even inside a sandbox.** `.claude/` and `.github/` remain
   hard-blocked *even in-sandbox*, so the sandbox can never be used to edit the very gate, hooks, or
   ethics that will review it. The sandbox is for **product** iteration only.

This is precisely the operator's design: *the review/audit level holds the actual quality/safety/
ethics gate.* The fast lane is fast; the door is where the wall is.

## 3. The loop

```
SCOPE ─▶ SPAWN ─▶ ITERATE ─▶ GATE ─▶ (MERGE | REJECT) ─▶ RATCHET
  │        │         │          │            │              │
  │        │         │          │            │              └─ ledger row + reflection (if qualified)
  │        │         │          │            └─ MERGE on green · REJECT → back to ITERATE (same sandbox)
  │        │         │          └─ strong-model review of the lane diff: quality + safety + ETHICS
  │        │         └─ unrestricted edit/build/test inside the worktree; no per-edit friction
  │        └─ one worktree per lane; doers=sonnet, fan-out via Agent(isolation:worktree)/Workflow
  └─ partition into collision-free lanes (different files/dirs/surfaces; no shared mutable state)
```

- **SCOPE.** The lead (or `loop-orchestrator`) partitions the goal into **collision-free lanes** —
  different files/dirs/surfaces, no shared mutable state, no ordering dependency (`CLAUDE.md` Agent
  Discipline: *"partition into collision-free lanes; keep any shared integration point for the lead
  to do after the fan-out"*). Any single hot file / strict dependency chain stays a **lead-owned
  integration step**, not a lane.
- **SPAWN.** One sandbox (worktree) per lane. Fan out concurrent subagents in a single message
  (multiple `Agent` tool-uses) or a `Workflow` for deterministic fan-out. Model routing per §5.
- **ITERATE.** Inside each sandbox, the lane agent iterates **unrestricted** — this is where the
  speed comes from. It must still *produce* the proof its diff will need at the gate (red→green test,
  Playwright artifact for UI, `request.*` assertion for API) — the gate checks for proof, so the
  sandbox authors it.
- **GATE.** Each lane's diff is reviewed by a **strong-reasoning model** (opus/fable) that runs the
  §4 rubric — quality, safety/security, **ethics** — and consults the read-only reviewer agents
  (`invariant-guardian`, `security-sentinel`) plus the mechanical checks. Verdict: **MERGE** (all
  green) or **REJECT** (any red).
- **MERGE / REJECT.** MERGE integrates the lane into the real branch (via the lead's normal
  commit → staging → validate Ship Discipline; red-lines additionally need their standing human
  approval). REJECT sends the diff **back to the same sandbox** with the failing criterion named —
  the worktree is not thrown away until the lane is merged or abandoned.
- **RATCHET.** A qualified change (touched ≥3 files OR ≥3 iterations OR closed a stage OR touched a
  red-line — `CLAUDE.md` thresholds) writes a reflection to `docs/reflections/INBOX/` and, if it
  fixed a bug, a `REGRESSION-LEDGER.md` row with a red→green guardrail. Monotonic: improvements lock
  in, never roll back.

## 4. The gate rubric (quality · safety · ethics)

The gate is one strong-model review pass over the lane diff. **Every box must be green to MERGE.**
Any red → REJECT with the criterion named → back to the sandbox. A box is credited by **proof
(artifact / test name / command output)**, never by intent ("looks fine" = FAIL — `CLAUDE.md`
Task-Exit Rule).

### A. Quality (mechanical + no-false-green)
- [ ] `pnpm typecheck` green on the lane. (output pasted)
- [ ] `pnpm build` green on the lane. (output pasted)
- [ ] Relevant tests green; **Mandatory Proof Rule** satisfied — UI change → Playwright
      `toBeVisible()/toContainText()` against staging; API change → ≥1 `request.*` assertion.
- [ ] If the change fixes a bug: a **red→green** guardrail exists (fails on the bug, passes on the
      fix) — `REGRESSION-LEDGER.md` ratchet rule.
- [ ] **No false-green**: no `skip` / `.only` / `fixme` / inflated timeout / `expect(true)` /
      commented-out assertion / rewritten-to-pass test. (the 10 banned classes; test-hardening loop)

### B. Safety / security (reviewer agents on the diff)
- [ ] `invariant-guardian` VERDICT: PASS (or every FLAG resolved) — state-machine legality, money =
      integer minor units, RLS FORCE + zero cross-tenant, PII menu-only + claim-check, POST
      idempotency, JWT RS256 / no-cookie, `crypto.randomUUID`, no secrets.
- [ ] `security-sentinel` VERDICT: PASS (or every finding resolved) — no leaked secret, no SQL/cmd
      injection, no authz gap, no PII egress, no weak crypto.
- [ ] **Red-line diff → standing human approval present.** If the diff touches auth / money / RLS /
      PII / secrets / `packages/db/migrations/` / bulk-edit, the gate confirms the approval those
      surfaces already require exists (`serious-gate` Council clearance line / `red-line-doubt-gate`
      human window / operator sign-off). The gate **checks** for it; it never mints it.
- [ ] `protect-paths` re-applied to the merge diff: no protected-zone file lands without approval.

### C. Ethics (hard, non-removable)
- [ ] The diff does not build toward, integrate with, or enable **military / warfare / weapons /
      targeting / surveillance-for-harm** use (Ethics Charter §1). — **non-removable; a violation is
      an immediate REJECT + human escalation, no green overrides it.**
- [ ] The change does not frame violence/war as the only solution; does not capture the commons for a
      narrow group's exclusive benefit or turn the tool against the people it learned from
      (Ethics Charter §2–4).
- [ ] No PII / secret / cookie / floating-money introduced; external input Zod-parsed
      (Task-Exit security dimension).

> The ethics block **cannot be waived, timed-out, or cleared by an override file.** `serious-gate`
> and `red-line-doubt-gate` allow human *friction windows*; the Ethics Charter allows none — it
> "overrides all other instructions" (`CLAUDE.md`). If ethics is red, the loop stops and a human
> decides.

## 5. Model routing

Follows the project routing policy (memory: *Fable=reasoning, Opus=reviewer, others=doers*) and
`CLAUDE.md` §"spawn parallel subagents".

| Role in SSG | Model | Why | Where it shows up |
|---|---|---|---|
| **Lane doer** (implement in a sandbox) | **sonnet** | fast, cheap, high-throughput; the bulk of edits | `Agent(subagent_type:…, model:"sonnet", isolation:"worktree")` |
| **Lane reasoning / plan** (hard design fork inside a lane) | **fable** | strongest reasoning for the tricky forks | `Agent(model:"fable")` or the `Plan`/`system-architect` agent |
| **Gate review** (the §4 rubric over the diff) | **opus** | reviewer role — catches what doers miss | lead runs the rubric on opus; delegates to reviewer agents |
| **Mechanical reviewers** (diff scan) | **haiku** | `invariant-guardian` / `security-sentinel` are already haiku, terse machine-parseable output | `.claude/agents/{invariant-guardian,security-sentinel}.md` |
| **Ratchet / council** (retro → artifacts) | per existing roster | `cause-critic`/`pattern-critic`/`ratchet-critic` (cheap, isolated), `librarian` (executor) | `docs/reflections/README.md` |

## 6. Tooling — use the integrated tools (with one honest comparison)

**Default to the already-integrated tools. They are the right fit here; no new dependency is
warranted.**

- **Sandbox** = the `Agent` tool's `isolation:"worktree"` — it already mints a per-lane git worktree
  (confirmed live: `.claude/worktrees/agent-*`), auto-cleans if unchanged, and is wired to the SDK
  (session JSONL → `tools/loop-harness` telemetry, model routing, `SendMessage` continuation). The
  free OSS primitive underneath is `git worktree` itself — which the scaffold script uses directly.
- **Swarm fan-out** = concurrent `Agent` tool-uses in one message (already the `CLAUDE.md` house
  style) for opportunistic parallelism; the **`Workflow`** tool for deterministic, repeatable fan-out
  (both in the `allow` list of `.claude/settings.json`). `Monitor` for waiting on background lanes.
- **Gate** = the read-only reviewer agents (`invariant-guardian`, `security-sentinel`) + the
  mechanical `pnpm` gates + a strong-model rubric pass. All in-tree.

**Would a free OSS tool beat these?** The one real alternative class is *containerized* agent
sandboxes (e.g. Dagger's `container-use` MCP, or a devcontainer-per-agent). They buy **stronger
isolation** — separate filesystem + network namespace — which matters when running *untrusted
third-party* code. That is **not** this use case: SSG runs trusted first-party agents on our own
repo, where a git worktree already gives branch/FS-copy isolation with **zero** added dependency and
**keeps** the SDK wiring (telemetry, model routing, agent roster) that a container layer would sever.
**Recommendation: stay on the integrated `Agent(isolation:"worktree")` + `Workflow`.** Revisit
containers only if the threat model ever changes to "execute untrusted code," which SSG is not for.

## 7. How it plugs into the existing loop system

SSG is a **loop like any other** — it lives under the same runtime, not beside it:

- **Registry.** Registered in `loops/registry.md` as `sandbox-swarm-gate` (see that file) with the
  4-condition classification, DoD, and verification, exactly matching the existing card format.
- **Orchestrator.** `loop-orchestrator` (`.claude/commands/loop-orchestrator.md`) is the runtime
  dispatcher: on a request it runs the 4-condition test, matches `sandbox-swarm-gate` by
  intent/`problem_signature` ("a repeatable multi-agent process to run at full capacity with a review
  gate"), REUSE/ADAPT-PARAMS (lanes / models / scope), dispatches, supervises the STOP-gate, harvests
  memory. It does **not** build or mutate the loop's structure — that is `loop-architect`.
- **Architect / certification.** Structural changes to SSG (a gate criterion, a block, an exit
  condition) go through `loop-architect` (`.claude/agents/loop-architect.md`) + M1–M11 re-cert. Until
  a `/build-verify-loop verify sandbox-swarm-gate` run stamps CERTIFIED, the card stays **DRAFT** and
  the orchestrator will not dispatch it (`loops/registry.md` health rule).
- **Harness telemetry.** On finish (success/stall/abort) the loop emits the §5 LOOP REPORT via the
  `finalize` seam (`tools/loop-harness/src/cli.ts finalize`), persisting to `loops/runs/`. The card's
  `harness:` node carries `progress_metric: lanes_not_yet_merged` (↓ = better; the breaker watches
  it).
- **Ratchet.** Merged lanes feed the standing self-improvement loop: qualified changes →
  `docs/reflections/INBOX/` → Council retro (`cause`/`pattern`/`ratchet` critics) → `librarian`
  enacts a `REGRESSION-LEDGER.md` guardrail / `docs/lessons/` lesson. SSG doesn't replace the ratchet;
  it feeds it, once per merged lane.

## 8. Sequence diagram

```mermaid
sequenceDiagram
    autonumber
    actor Human
    participant Orch as loop-orchestrator
    participant Lead as Lead agent
    participant WT as Sandbox worktrees<br/>(.claude/worktrees/&lt;lane&gt;)
    participant Doers as Swarm doers<br/>(Agent isolation:worktree · sonnet)
    participant Gate as Gate<br/>(opus rubric + reviewer agents)
    participant Real as Real branch / staging
    participant Ratchet as Ledger + reflections

    Human->>Orch: goal (a repeatable multi-agent process)
    Orch->>Orch: 4-condition test · match sandbox-swarm-gate (CERTIFIED?)
    Orch->>Lead: dispatch loop (lanes, models, scope)
    Lead->>Lead: SCOPE — partition into collision-free lanes
    par one sandbox per lane
        Lead->>WT: SPAWN worktree(lane-A) / worktree(lane-B) …
        Lead->>Doers: fan out Agent(isolation:worktree, model:sonnet) per lane
        Note over WT,Doers: ITERATE — UNRESTRICTED edit/build/test<br/>(no per-edit protect-path friction;<br/>.claude & .github still blocked)
        Doers-->>WT: lane diff + proof (red→green test / Playwright / request.*)
    end
    Doers->>Gate: submit each lane diff for review
    Gate->>Gate: A Quality (typecheck/build/tests green · no false-green)
    Gate->>Gate: B Safety (invariant-guardian + security-sentinel · red-line approval present)
    Gate->>Gate: C ETHICS (Charter — hard, non-removable)
    alt all green
        Gate-->>Lead: MERGE
        Lead->>Real: integrate (commit → staging → validate; red-line → human approval)
        Real->>Ratchet: qualified change → reflection + ledger guardrail
    else any red
        Gate-->>Doers: REJECT (criterion named) → back to the SAME sandbox
        Note over Doers,WT: re-ITERATE until green or abandoned
    end
    Ratchet-->>Orch: LOOP REPORT (finalize → loops/runs/)
```

## 9. Operator action — THE one thing

**To make the sandbox actually unrestricted, exactly one change is needed — and it must be applied as
an indivisible pair with the gate being live.** Today the three PreToolUse gate hooks resolve their
root via `git rev-parse --show-toplevel`, which *inside* a worktree is the worktree dir — so
`packages/db/migrations/`, `packages/db/`, contracts, and `package.json` are **still hard-blocked
inside a sandbox** (and from the main tree, any worktree path is blocked because it contains
`.claude/`). The sandbox is not actually unrestricted until the hooks learn to recognize a sandbox
path and relax **there only**.

### The pairing rule (non-negotiable)
> Apply the relaxation below **only after** the gate (§4) is wired to re-run the FULL
> `protect-paths` + `invariant-guardian` + `security-sentinel` + **ethics** review on the lane's
> merge diff before it lands. **Relaxation without the gate = deletion of the guard.** They ship
> together or not at all.

### Proposed diff — `.claude/hooks/protect-paths.sh` (operator applies; `.claude/hooks` is a protected zone)

Insert immediately **after** the `REL` is computed (current line 52) and **before** the `PROTECTED`
grep (current line 54):

```diff
   case "$FILE" in
     "$ROOT"/*) REL="${FILE#"$ROOT"/}" ;;
     /*) exit 0 ;;
     *) REL="$FILE" ;;
   esac

+# ── Sandbox-Swarm-Gate relaxation (IN-SANDBOX ONLY) ────────────────────────────
+# A throwaway worktree under .claude/worktrees/<lane>/ is a sandbox with NO real-world
+# reach. Product-code iteration there is unrestricted BY DESIGN — the quality/safety/
+# ethics GATE re-runs the FULL protect-path + invariant + security + ethics review on
+# the MERGE diff before anything lands. Guards are RELOCATED to the gate, not deleted.
+# Keyed on the absolute $FILE so it holds whether the editor's cwd is the worktree or
+# the main tree. Governance/harness (.claude, .github) stay HARD-BLOCKED even in-sandbox
+# so a sandbox can never edit the very gate/ethics that will review it.
+if printf '%s' "$FILE" | grep -qE '/\.claude/worktrees/[^/]+/'; then
+  WT_REL="${FILE##*/.claude/worktrees/*/}"          # path relative to the worktree root
+  if printf '%s' "$WT_REL" | grep -qE '(^|/)(\.github|\.claude)/'; then
+    echo "BLOCKED (in-sandbox): '$WT_REL' — harness/governance stays protected even in a sandbox worktree." >&2
+    exit 2
+  fi
+  exit 0   # all other product paths: unrestricted in-sandbox; the GATE enforces at merge
+fi
+# ───────────────────────────────────────────────────────────────────────────────

   PROTECTED='(^|/)(migrations|\.github|\.claude)/|…'
```

### Mirror the same early-relax in the other two PreToolUse gates
Both currently block inside a worktree the same way:

- **`.claude/hooks/serious-gate.sh`** — after `fp` is parsed (current line 42), add the same
  `printf '%s' "$fp" | grep -qE '/\.claude/worktrees/[^/]+/' && exit 0` early-relax (its existing
  `case "$rel" in docs/*|loops/*|.claude/*) exit 0` only relaxes when *cwd is the main tree*; it does
  not relax when the agent's cwd IS the worktree, so migrations/schema/etc. still block in-sandbox).
- **`.claude/hooks/red-line-doubt-gate.sh`** — after `fp` is parsed (current line 49), add the same
  early-relax. In-sandbox the *advisory* doubt-pass is noise (there is no real-world reach to doubt
  about); the **real** red-line doubt-pass and human window are enforced **at the gate** on the merge
  diff, where they belong.

> No `.claude/settings.json` change is required — the hook wiring (matchers) stays as-is; only the
> three hook scripts gain the sandbox-scoped early-relax. Since `.claude/hooks/` is a protected zone,
> **the operator applies these three edits** (this doc proposes them; the agent does not touch the
> zone).

### Why this is safe
1. Keyed strictly on the literal path segment `/.claude/worktrees/<lane>/` — nothing outside a
   sandbox worktree is affected; the main tree keeps every guard.
2. `.claude/` and `.github/` stay blocked *even in-sandbox* — the sandbox can't rewrite the gate,
   hooks, CI, or ethics that will judge it.
3. The full guard set re-runs at the gate on the merge diff (§4). A red-line change still cannot
   reach the real branch / main / prod without the gate passing **and** its standing human approval.
4. Reversible: deleting the added block restores the pre-SSG behavior exactly.

## 10. Runbook — how to run it

`scripts/sandbox-swarm-gate.mjs` is a **thin, safe** orchestration aid (Node ESM, no new deps).
**Dry-run by default**; mutating ops require `--apply`. **It never merges and never disables the
ethics gate** — there is no merge subcommand; merge stays a lead-driven, gate-passed, human-approved
act.

```bash
# 0. See everything the tool does
node scripts/sandbox-swarm-gate.mjs --help

# 1. SCOPE + SPAWN — create one sandbox worktree per collision-free lane
node scripts/sandbox-swarm-gate.mjs new money   --apply   # → .claude/worktrees/ssg-money  (branch ssg/money)
node scripts/sandbox-swarm-gate.mjs new authz   --apply
node scripts/sandbox-swarm-gate.mjs new fe       --apply
node scripts/sandbox-swarm-gate.mjs list                  # inventory of SSG sandboxes

# 2. SWARM — fan out doers into the sandboxes (done via the Agent tool, not this script):
#    Agent(subagent_type:"Backend Architect", model:"sonnet", isolation:"worktree", prompt:"… lane: money …")
#    Agent(subagent_type:"claude",            model:"sonnet", isolation:"worktree", prompt:"… lane: fe    …")
#    — concurrent tool-uses in ONE message; or a Workflow for deterministic fan-out.

# 3. GATE — print the rubric, and the (dry-run) merge plan for a lane
node scripts/sandbox-swarm-gate.mjs checklist              # the §4 quality/safety/ETHICS rubric
node scripts/sandbox-swarm-gate.mjs plan money             # diff stat + red-line/protected classification
#    Then run the actual gate: opus rubric pass + invariant-guardian + security-sentinel over the diff,
#    + `pnpm typecheck && pnpm build` + the lane's proof (Playwright / request.* / red→green test).

# 4. MERGE (green) — lead-driven, NOT this script: normal Ship Discipline
#    commit (feature branch) → deploy staging → validate → red-line? human approval → merge.
#    REJECT (red) — send the named failing criterion back to the SAME sandbox; re-iterate.

# 5. Cleanup a spent sandbox (after merge/abandon)
node scripts/sandbox-swarm-gate.mjs rm money --apply

# 6. Telemetry — on finish emit the §5 LOOP REPORT via the harness finalize seam
#    (see the harness: node in loops/sandbox-swarm-gate.yaml — to be authored by loop-architect at cert time)
```

**Definition of done for one SSG run:** every dispatched lane is either MERGED (all §4 boxes green +
red-line approvals present) or explicitly ABANDONED; no sandbox left un-reviewed; the LOOP REPORT
emitted; any qualified change carries its reflection + ledger guardrail.
