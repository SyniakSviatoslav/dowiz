---
name: max-lanes-orchestration
description: Partition a multi-part goal into collision-free parallel lanes, fan out one concurrent subagent per lane, keep the shared integration point for the lead, and review-gate each lane's diff before it lands. Use whenever a task decomposes into 2+ independent units of work — a multi-file build, a broad audit, a per-item transform, batch scaffolding, or a max-lanes remediation across several surfaces — and you want to run them in parallel without the lanes clobbering each other. Triggers on "run these in parallel", "fan out", "do all of these at once", "spin up an agent for each", "max lanes", "parallel remediation", or when about to emit multiple Agent tool-uses in one turn. Its core job is the partition — it produces a lane ownership manifest (lane to file globs, provably disjoint) so two lanes never touch the same file, routes doers to sonnet and hard reasoning to fable, reserves any one hot file for the lead, and names the merge gate. Prevents the lane-collision class that has bitten this repo more than once.
---

# Max-Lanes Orchestration

> The conductor's skill: turn one big goal into many parallel lanes that run at full capacity **without stepping on each other**, then integrate on green. The speed comes from parallelism; the safety comes from the partition being **collision-free by construction**, not by luck.

## Why this exists (read this first)

Fanning out concurrent subagents is the house style — `CLAUDE.md` Agent Discipline says to dispatch 2+ independent units as concurrent Agent tool-uses in one turn. The failure mode is **not** the fan-out; it is a **sloppy partition**. When two lanes are handed overlapping file ownership, they collide mid-flight and the damage is invisible until a write lands on top of another lane's read.

This has happened here more than once — it is a **recurring class**, which is exactly why it deserves a codified procedure:

- **Money lane vs FE lane (2026-07-03):** both lanes were assigned `orders.ts`, `money.ts`, `CheckoutPage.tsx`, `i18n-catalog.ts`. The money lane's prompt *claimed* exclusive ownership; nothing *verified* disjointness. The collision surfaced only when a PostToolUse hook flagged "file changed after your previous read." (`docs/reflections/INBOX/2026-07-03-money-lane-impl.reflection.md` §1)
- **Design-system prune (2026-07-02):** two sessions shared one checkout; staged-but-uncommitted deletions had no owner and got swept into a parallel lane's commit — HEAD was unbuildable for ~40 min. (`docs/reflections/INBOX/design-system-prune-collision-2026-07-02.md`)

Both trace to one root: **lane ownership was asserted in prose, never checked as a machine-verifiable disjoint manifest.** This skill makes the partition the first-class artifact and the collision-free check the entry gate to fan-out.

## When to use / when not

**Use it** when the goal splits into **2+ lanes that are collision-free** — different files/dirs/surfaces, no shared mutable state, no ordering dependency: multi-file builds, broad audits/searches, per-item transforms, batch scaffolding, multi-surface remediation.

**Do NOT fan out** (keep it solo/sequential — this skill will tell you to) when:
- The work centers on **one hot file** (e.g. `server.ts`, `orders.ts`, `i18n.ts`, a barrel `index.ts`) — concurrent edits to one file is the collision you are trying to avoid. Reserve it for the lead.
- There is a **strict dependency chain** (B needs A's output) — run sequentially.
- The change is trivial — just do it.

## The decision tree

```
Goal arrives
   │
   ▼
Does it split into 2+ units? ──no──▶ do it solo (this skill does not apply)
   │ yes
   ▼
Are the units collision-free?  (different files/dirs/surfaces, no shared mutable
state, no ordering dependency)
   │                              │
   │ yes                          │ no / partly
   ▼                              ▼
Build the ownership manifest   Separate the tangled part OUT as a
(lane → disjoint globs)        LEAD-OWNED integration step; fan out only
   │                           the parts that ARE collision-free.
   ▼
Manifest globs provably disjoint? ──no──▶ re-partition until disjoint, or pull the
   │ yes                                   overlap into the lead's integration step
   ▼
SPAWN one subagent per lane (concurrent Agent tool-uses in ONE turn)
   ▼
Each lane iterates + produces its own proof (red→green test / Playwright / request.*)
   ▼
GATE each lane's diff (quality · safety · ethics) before it lands
   ▼
LEAD integrates: shared wiring / registration / hot-file edits it reserved
   ▼
RATCHET: qualified change → reflection (+ ledger guardrail if a bug was fixed)
```

## The procedure

### 1. SCOPE — partition into collision-free lanes
Decompose the goal by **surface**, not by task-count. For each candidate lane write down the **globs it owns** (the files/dirs it will edit). Then apply the disjointness test:

- **No two lanes share a glob.** If lane A owns `apps/api/src/routes/owner/**` and lane B owns `apps/api/src/routes/owner/gdpr.ts`, they overlap — merge them or narrow B.
- **Any file two lanes both need → it is NOT a lane. It is a lead-owned integration step** done *after* the fan-out (registration, barrel exports, a shared type, `i18n-catalog.ts`, `server.ts` route wiring).
- **Shared mutable global state has no owner.** The git index is shared: never let a lane `git add`/`commit` from a shared checkout. Prefer `Agent(isolation:"worktree")` so each lane gets its own working tree and index (this is what would have prevented the design-system-prune collision).

### 2. Write the ownership manifest (the entry gate to fan-out)
Produce an explicit lane → globs table **before** dispatching. This is the artifact that makes the partition checkable instead of asserted. Minimal shape:

```
lane        owns (disjoint globs)                              model    integration-owned-by-lead
money       apps/api/src/routes/orders.ts, ui/lib/money.ts     sonnet   i18n-catalog.ts (shared)
authz       apps/api/src/routes/owner/**                       sonnet   —
fe-lc9      apps/web/src/pages/admin/**                         sonnet   —
```

If you cannot fill this table with **disjoint** rows, you are not ready to fan out — re-partition. Put the shared cells in the "integration-owned-by-lead" column, never in two lanes.

### 3. SPAWN — fan out, one subagent per lane, in ONE turn
Emit the concurrent `Agent` tool-uses in a **single message** (`CLAUDE.md`: "concurrent subagents in a single message"). Route by role:

| Role | Model | Why |
|------|-------|-----|
| Lane doer (implement the lane) | **sonnet** | fast, cheap, high-throughput — the bulk of edits |
| Hard reasoning inside a lane (a tricky design fork) | **fable** | strongest reasoning for the tricky forks |
| Gate review of a lane diff | **opus** | reviewer role — catches what doers miss |
| Mechanical diff reviewers | **haiku** | `invariant-guardian` / `security-sentinel` are terse and machine-parseable |

Give each lane a **self-contained prompt**: its owned globs, its goal, the proof it must produce, and an explicit "do NOT touch files outside your globs — if you need one, stop and report it to the lead." Prefer `isolation:"worktree"` for write-lanes so their index is private.

### 4. GATE — review each lane's diff before it lands
Every lane's diff passes a quality + safety + **ethics** review before merge (this is the Sandbox-Swarm-Gate rubric — `docs/design/harness/SANDBOX-SWARM-GATE.md` §4). A box is credited by **proof (artifact / test / command output)**, never by intent:

- **Quality:** `pnpm typecheck` + `pnpm build` green on the lane; Mandatory Proof satisfied (UI → Playwright `toBeVisible/toContainText`; API → `request.*`); bugfix → red→green guardrail; no false-green (the 10 banned classes).
- **Safety:** `invariant-guardian` PASS + `security-sentinel` PASS; red-line diff (auth/money/RLS/PII/secrets/migrations/bulk) → the standing human approval those require is present; `protect-paths` re-applied to the diff.
- **Ethics:** the `CLAUDE.md` Ethics Charter is a hard, non-removable gate criterion — a violation is an immediate REJECT + human escalation, no green overrides it.

Any red → REJECT with the criterion named → back to the same lane. Merge only on all-green.

### 5. INTEGRATE — the lead does the shared step (never a lane)
After the lanes land, the lead does the integration it reserved in step 1: registration, barrel exports, the shared hot-file edit, the cross-lane wiring. This is serial and single-owner **on purpose** — it is the part that was not collision-free.

### 6. RATCHET — feed the self-improvement loop
A qualified change (touched ≥3 files OR ≥3 iterations OR closed a stage OR touched a red-line — `CLAUDE.md` thresholds) writes a reflection to `docs/reflections/INBOX/`; a fixed bug adds a red→green `REGRESSION-LEDGER.md` row. If a lane collision *did* happen, that is a candidate guardrail (a machine-checkable ownership manifest a PreToolUse hook consults), not just a lesson — hand it to the `librarian`.

## Definition of done

- [ ] A written **lane ownership manifest** exists and its globs are **provably disjoint** (no file in two lanes). — the manifest table
- [ ] Every shared/hot file was handled as a **lead-owned integration step**, not assigned to a lane. — the manifest's integration column + the lead's post-fan-out commit
- [ ] Each lane was dispatched with a self-contained prompt and produced **its own proof**. — per-lane proof artifact
- [ ] Each lane's diff **passed the gate** (quality · safety · ethics) or was explicitly abandoned. — per-lane gate verdict
- [ ] **Zero cross-lane file collisions** observed (no "file changed after your previous read" on a file another lane owned). — clean run / git status attributable to single owners
- [ ] Qualified change → reflection (+ ledger guardrail if a bug was fixed). — `docs/reflections/INBOX/` entry

## Anti-patterns (each is a real incident here)

- **Overlapping ownership asserted in prose.** Two lanes "both own" `orders.ts` because each prompt says so — nothing checks it. (money-lane collision) → the manifest disjointness test is the fix.
- **Committing from a shared index.** A lane commits while another lane's deletions sit staged. (design-system-prune) → use `isolation:"worktree"`; never verify-then-commit across concurrent lanes.
- **Fanning out a hot file.** Splitting edits to `server.ts`/`i18n.ts` across lanes. → reserve hot files for the lead's integration step.
- **Gate-by-vibe.** Merging a lane because it "looks fine." → the gate is credited by proof only (`CLAUDE.md` Task-Exit Rule).

## Relationship to the harness

This skill is the **reusable SCOPE→SPAWN partitioning procedure** — the execution skill that loops invoke for their fan-out step. It is not itself a loop. The full sandbox+gate+merge+ratchet machinery lives in the **Sandbox-Swarm-Gate** loop (`loops/sandbox-swarm-gate.yaml`, `docs/design/harness/SANDBOX-SWARM-GATE.md`); this skill is what SSG's SCOPE and SPAWN steps *do*. The gate rubric referenced in step 4 is SSG §4 — do not re-derive it here; reuse it.
