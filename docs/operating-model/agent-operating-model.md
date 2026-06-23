# dowiz / DeliveryOS — Agent Operating Model: two speeds, one boundary

> **Operational-layer charter** (NOT a product phase; sibling of Session-Handoff, not build/audit
> prompts). It is the **rules under which all other prompts run** — it does not execute, it governs.
> **Authority split:** product scope / contracts / red lines → `v4.5` (this charter does NOT change
> them); *how the agent system works* (speeds, modes, classification, recon/retro/fresh-eyes
> protocols) → this document. Full source charter pasted by the operator 2026-06-23.

## The central tension (named)
Four of five innovation principles pull opposite ways. **1** (fast cheap iteration), **3** (bypass
dogma), **5** (newcomer breaks conformity) → openness & change. **2** (system-fit) + existing
governance → discipline & conformity. You cannot max both — they destroy each other (infinite
iteration kills solo discipline; total conformity kills breakthrough).

## The resolution — architectural, not willpower: **two speeds, one boundary**

| | **RECON speed** | **EXECUTION speed** |
|---|---|---|
| Principles | 1 · 3 · 5 | 2 (+ existing governance) |
| Lives in | `spikes/` (+ challenge-log, fresh-eyes notes) | `apps/` + `packages/` |
| Character | open, cheap, throwaway, non-conformant, touches reality | disciplined, conformant, proven |
| Governance | relaxed (EXCEPT product red lines) | full (inventory-truth, tests, design system) |
| Output | **evidence & decisions**, not merge-able code | merge-able code that passed the gates |

**Principle 4 (psychological safety) is the glue** that lets you move between speeds without fear:
a mistake in recon = expected iteration; an honest red in execution = success, not failure.

## 🔴 IRON PRINCIPLES OF THE OPERATIONAL LAYER
- 🔴 **Two speeds, one boundary — sacred.** Recon output (`spikes/`, fresh-eyes, challenge) NEVER
  reaches `apps/` as merge-able code. Crossing the boundary is only via a separate `build` prompt
  under full discipline. Spike code in `apps/` = violation.
- 🔴 **Product red lines outrank every mode.** Even in the relaxed sandbox the `v4.5` invariants
  hold: human-authority on `deliver` (courier always completes); correlation/cash = friction not
  verdict; price/status server-only; zero PII in AI/queues/logs; integer money; RLS FORCE; zero
  cookies; tenant isolation. Relaxing the mode ≠ relaxing the red lines.
- 🔴 **The decision registry bounds iteration & challenge.** Spin fast *inside* an open question;
  do not reopen a closed one (chain `v4.5`). Challenge is allowed but = a *logged challenge with
  evidence*; only NEW real-world evidence reopens — never second-guessing. (Named risk #1: a
  deferred item creeping back into CORE.)
- 🔴 **Honest red = success.** A well-proven `FAIL`/`MISSING`/`BLOCKED` is a SUCCESSFUL run, equal
  to `PASS`. An agent that hides a problem to go green violates the system; one that surfaces the
  truth with evidence fulfills it.
- 🔴 **System-fit over individual cleverness (in execution).** A clever solution that works but
  bypasses the shared layer / tokens / established patterns fails the gate *despite* working.

## §1 · EXTENDED CLASSIFICATION (the biggest lever in one move)
`require-classification` emits one of four labels; `post-edit-gates` routes the governance mode.

| Label | Speed | Lives in | Governance mode | Output | Principle |
|---|---|---|---|---|---|
| **`spike`** | recon | `spikes/` | relaxed; throwaway ok; *except* red lines | evidence + decision (not code in `apps/`) | 1, 3 |
| **`build`** | execution | `apps/`,`packages/` | full: inventory-truth, tests, design system, system-fit gate | merge-able code, green tests | 2 |
| **`audit`** | execution | whole repo (read-first) | full: provable, zero contract regression | PASS/FAIL + GO/NO-GO | 2 |
| **`challenge`** | recon | `docs/decisions/` | relaxed; questions authority WITH evidence | logged challenge; reopen only on new real evidence | 4 |

**Routing in `post-edit-gates`:**
- `spike` / `challenge` → **skip** system-fit, design-system, full matrix; **keep** red lines
  (grep PII/cookie/secrets/float-money) and the boundary (`spike` changes only in `spikes/`).
- `build` / `audit` → **full discipline**: lint:gates, tokens, shared layer, zero changed server
  contracts, tests.
- **Boundary (hard):** fail any `spike` change outside `spikes/` and any import from `spikes/` into
  `apps/`/`packages/`. Crossing only via a new `build` prompt.

## §2–§7 · PROTOCOLS (run on demand — see docs/operating-model/protocols.md)
- **§2 Spike:** hypothesis + cheapest falsifier → Mem0; time-box; build the *ugly fast* that touches
  reality (not the polished whole — Marshmallow puts the marshmallow first); buffer ∝ #systems the
  step stitches; real-world feedback (PostHog / first real paid order), not only Playwright.
- **§3 Skunkworks:** relaxed sandbox in `spikes/` (ignore build/audit iron principles EXCEPT red
  lines); adoption-trial external tools ONE at a time on a working base before core adoption.
- **§4 Player roles & system-fit:** Drafter (cheap rotation model) · Adversarial reviewer (a
  DIFFERENT model that breaks the draft) · Integrator-orchestrator (Claude Code) · Naive/fresh-eyes
  (context-free agent). Pre-agreed stop/pivot condition subordinates ego to coherence.
- **§5 Psychological safety:** for you — NO-GO = the net worked; blameless retro; right to stop at a
  green checkpoint; sustainable pace as invariant. For agents — positive framing "a well-proven
  FAIL/MISSING is a SUCCESSFUL run" flips the incentive from hide-to-please to surface-the-truth.
- **§6 Fresh eyes:** a context-free agent (no Mem0/Repowise/handoff) on one problem with a minimal
  prompt; diff against the context-loaded view; surgical (on specific snags / comfort questions),
  not a dilution of the whole pipeline.
- **§7 Challenge channel:** a logged challenge with evidence in `docs/decisions/`; reopen only on new
  real-world evidence; a challenge that brings evidence is welcomed as value, not insubordination.

## DEFINITION OF DONE (model "installed" when)
🔴 `require-classification` emits `spike|build|audit|challenge` and routes the mode · 🔴
`post-edit-gates` keeps system-fit on `build`/`audit` and FREES `spike`/`challenge` (except red
lines + `spikes/` boundary) · 🔴 boundary hard: zero spike code in `apps/`, zero import from
`spikes/` into core · 🔴 prompt templates carry the positive "honest FAIL/MISSING = success" ·
🔴 blameless-retro and fresh-eyes are standing protocols; hypothesis registry + retro lessons live
in Mem0 · 🔴 player roles documented · 🔴 the pre-agreed stop/pivot condition is recorded (closes
the open accountability item, `v4.5` §9).

## OUT OF SCOPE
Does not change product contracts/schema/red lines (`v4.5` is authority — this charter only
*surrounds* it with modes); does not rewrite internal build/audit/loop-prompt logic; does not add
governance-for-governance — the goal is the opposite: let recon breathe while execution stays
disciplined; does not relax red lines in any mode.
