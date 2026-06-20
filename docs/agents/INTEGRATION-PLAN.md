# Agentic System — Integration Plan

> How to take the [documentation corpus](./README.md) and stand it up as a *live* system in Claude Code, safely, in dependency order, with a proof-of-life gate at each step and a rollback for each. **Nothing here is active until these phases run.** This plan is the "build" the docs were written before.

## Guiding constraints (read first)

1. **`.claude/**` is a protected zone.** The `protect-paths.sh` PreToolUse hook **blocks** `Edit`/`Write` to `.claude/agents`, `.claude/commands`, `.claude/hooks`. Every live install of an agent/command/hook therefore needs an **explicit human approval** of that write (or a reviewed `cp` from a staging dir, which is not gated because the hook matches Edit/Write, not Bash). **Recommended:** stage under `docs/agents/_staging/` (or generate inline), then install with `cp` in a single reviewed step, file-by-file, reading each before it goes live.
2. **The deny-gate hook is the single highest-risk change** (it can `deny` the assistant's own edits). It is sequenced **last**, behind its own proof-of-life, and is independently revertible.
3. **Dependency order is real:** registry + Loop-Architect must exist **before** Orchestrator (it delegates to them); the Council comes **after** the loop core (its `design-convergence` loop lives in the registry); the hook is **last** (automation over a working manual flow).
4. **Cost discipline:** the triad/loops cost tokens. Don't wire the auto-gate until the manual flow (`/council`, `/converge-loop`) has proven itself by hand.
5. **This is governance tooling, not product code** — it does not touch the launch surface. It can land on its own branch independent of `feat/v1-hardening`.

## Prerequisites (Phase 0)

- Governance hooks already live (`post-edit-gates`, `require-classification` on Stop) — ✅ present.
- Repo healthy, Claude Code authenticated — ✅.
- Confirm `scripts/openrouter-implement.ts` exists/works on Linux for the M11 cross-opinion bridge — **verify; if absent, M11 degrades to separate-context Claude subagents (documented limitation, not a blocker).**
- Decide the branch: recommend `feat/agentic-system` off `main` (keep separate from the launch branch).

---

## Phase 1 — Loop skeleton (no behaviour change)

**Goal:** the registry + templates exist so later agents have somewhere to write.

**Actions** (all under `loops/`, not protected — plain Write/cp):
- `loops/registry.md` ← from [loops/REGISTRY.md](./loops/REGISTRY.md) (seed the two CERTIFIED cards + DRAFT rows).
- `loops/_templates/loop-card.yaml`, `verification-report.md`, `memory.md` (from the Loop-Core pack §8–10).
- `loops/error-fix-convergence.yaml`, `loops/design-convergence.yaml` (from the embedded cards in the loop docs).
- Empty `loops/reports/`, `loops/memory/` (with `.gitkeep`).

**Proof-of-life:** `loops/registry.md` lists the two CERTIFIED loops; templates have no placeholders.
**Rollback:** delete `loops/` (nothing references it yet).

---

## Phase 2 — Loop-Architect (quality keystone) · ⚠️ protected write

**Goal:** the only agent that certifies loops.

**Action:** install `.claude/agents/loop-architect.md` (from [roles/loop-architect.md](./roles/loop-architect.md) → convert the spec to the agent front-matter form in the Loop-Core pack §2). **Requires approval of the protected write.**

**Proof-of-life:** `/agents` lists `loop-architect`.
**Rollback:** remove the file; restart.

---

## Phase 3 — Core loop commands · ⚠️ protected writes

**Action:** install `.claude/commands/loop-orchestrator.md`, `build-verify-loop.md`, `converge-loop.md` (Loop-Core pack §1,3,4).

**Proof-of-life (the FIRST real test):** run **`/converge-loop`** on a narrow scope (one screen/role) → confirm REUSE + the error-fix discipline work end-to-end (live Playwright, no fake-green). This validates the loop machinery before any automation.
**Rollback:** remove the three command files.

---

## Phase 4 — The Triad (design plane) · ⚠️ protected writes

**Action:** install `.claude/agents/system-architect.md`, `system-breaker.md`, `counsel.md` (from the three role docs → agent front-matter per Triad-Council pack §2–4) + `.claude/commands/council.md` (pack §1). Put Breaker and Counsel on a different model/context than Architect where possible (M11 spirit).

**Proof-of-life (second real test):** run **`/council`** on the next genuine serious change (e.g. distance-pricing, a new external integration seam) → the triad converges, and `docs/design/<slug>/` gets `proposal.md` + `breaker-findings.md` + `counsel-opinion.md` + `resolution.md` (+ ADR). Then a first **Counsel health-pass** over the existing loops/ADRs → `docs/governance/agent-health-<date>.md`.
**Rollback:** remove the four files.

---

## Phase 5 — Restart & verify

`claude` restart (or `/agents` for instant). Confirm `/agents` shows 4 agents (architect, breaker, counsel, loop-architect) and `/help` shows the commands. Manual flow is now fully usable **by hand** — operate it this way for a while before automating.

---

## Phase 6 — Loop families (DRAFT → CERTIFIED) · incremental

For each DRAFT loop you actually need (don't bulk-enable): copy its card from the loop doc into `loops/<id>.yaml`, add the registry row, install its thin command, then **certify**: `/build-verify-loop verify <id>` (M1–M11 + anti-cheat dry-run on a **real broken fixture** in this repo). Only a CERTIFIED loop is dispatchable. Order by need; start with `backend-contract-convergence` and `build-stage` (the two most load-bearing), defer the rest until a real trigger.

**Proof-of-life per loop:** verification report shows all M1–M11 PASS + the dry-run going RED on the broken fixture.

---

## Phase 7 — OpenRouter cross-opinion (M11) · optional but recommended

Confirm the bridge (`scripts/openrouter-implement.ts`) runs on Linux; it's already referenced as an optional step inside `/build-verify-loop` and `/council`. This is the only *true* uncorrelated voice (Claude subagents are all Claude). If unavailable, document the degradation and proceed.

---

## Phase 8 — Hook automation (LAST, highest caution) · ⚠️⚠️ protected + behaviour-changing

Only after the manual flow has proven itself (Phases 3–5 exercised by hand several times).

**Actions** (from the require-classification pack):
1. Stage and **read** both scripts; install `.claude/hooks/require-classification.sh` (extends the existing) + `.claude/hooks/route-request.sh`.
2. Merge the `hooks` block into `.claude/settings.json` (PreToolUse gate + UserPromptSubmit router); **do not** touch `post-edit-gates`.
3. Add the one clearing line to `/council` step 8.
4. `.gitignore` += `.claude/state/` and `.claude/logs/`.
5. `mkdir -p .claude/state .claude/logs`.

**Proof-of-life (run all five):**
- Gate **denies** an uncleared serious edit (e.g. a migration) with the actionable directive.
- `echo "reason" > .claude/state/serious-override` → the same edit passes (logged ALLOW override); remove to re-arm.
- `/council <change>` GO writes `serious-cleared` → edit passes.
- Router nudges on a "add a migration for X" prompt and a "every time check Y" prompt.
- A normal UI/text edit passes with **zero** friction (tune `SERIOUS` regex if it false-fires).

**Rollback (must be instant):** revert the `settings.json` hooks block (or `rm` the two scripts). Because it's fail-open and override-able, worst case is a stray `deny` that the human overrides; still, keep the revert one command away.

---

## Sequencing summary

```
0 prereqs ─► 1 loop skeleton ─► 2 loop-architect ─► 3 core commands ─►[PoL: /converge-loop]
   ─► 4 triad + /council ─►[PoL: /council + health-pass] ─► 5 restart/verify
   ─► 6 loop families (per-need, each certified) ─► 7 OpenRouter M11 (opt)
   ─► 8 HOOK AUTOMATION (last, behind its own 5-point proof, instant rollback)
```

## What I will NOT do without an explicit go-ahead
- Write anything into `.claude/**` (protected; each is a reviewed, approved step).
- Activate the deny-gate hook (Phase 8) — it changes how every future edit is permitted.
- Bulk-certify all DRAFT loops — certify per real need to avoid phantom-skill loops.

## Open questions for you
1. **Branch:** put this on `feat/agentic-system` (recommended) or fold into `feat/v1-hardening`?
2. **Scope of first install:** minimal (Phases 1–5: triad + loop core, manual) or full (through Phase 8 automation)?
3. **M11 bridge:** is `scripts/openrouter-implement.ts` present/working here, or should we document the Claude-only degradation?
4. **Timing vs launch:** this is independent of the launch; install before or after the first real paid order?
