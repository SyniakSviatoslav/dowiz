# DeliveryOS / dowiz — Loop Selection Router (v1)

**Date:** 2026-06-27 · The **mandatory intake gate** above the whole loop system. It runs **first on every command** in the Claude Code (dev-plane) terminal and decides: run an existing loop, build a new one, or just execute directly. Sits above the harness (v3-FINAL), the loop builder, and the autoupgrade loop. Enforced as a pre-hook.

---

## 0. Core principle (the load-bearing guard)

**Mandatory *selection* ≠ mandatory *loop*.** Selection runs on every command; the most common *outcome* is **DIRECT — no loop.** Forcing every command into a loop would make the terminal unusable. The router defaults to DIRECT for anything that isn't an iterative task toward a measurable goal.

**Cheap hot path:** deterministic-first (tag/scope match + one-shot detection); LLM-classify only on genuine ambiguity. It **routes only**; it never overrides Class A/B, oracle-admissibility, or the security carve-out.

---

## 1. The decision tree (three outcomes + bounce)

```
command ──► ROUTER
1. ONE-SHOT? (query · single action · explanation) ──► DIRECT  (common case)
2. else LOOP-WORTHY:
   2a. MATCH a registered loop? ──► RUN it (best by tag score; tie → flag/clarify)
   2b. NO MATCH: ORACLE-ADMISSIBLE? ── yes ──► BUILD (loop builder)
                                     └─ no  ──► BOUNCE ("define measurable criteria")
──► announce one line ──► proceed ──► record routing telemetry
```

---

## 2–7. Registry · hot-path discipline · enforcement · inherited boundaries · telemetry · anti-gold-plating

The router selects from `runs/registry.json` (the loop manifest the builder writes to on auto-register). Deterministic-first; generic tags (green/loop/flow/wiring) never match a loop alone. Enforcement = a `UserPromptSubmit` hook + a CLAUDE.md directive. The router never grants new powers — Class A/B, admissibility, and the security carve-out of the target loop hold unchanged. Every decision is announced (one line) and recorded (`runs/routing.jsonl`). DIRECT is the default; the router is a thin fast decision, not a Loop.

---

## 8. Order of work

1. Registry · 2. Deterministic router + one-shot heuristic · 3. hook + CLAUDE.md · 4. BUILD path · 5. LLM-classify fallback · 6. routing telemetry + recall.

---

## Implementation status (appended by build)

- **2026-06-27 — §1 deterministic router + §2 registry + §6 telemetry built** in
  `tools/loop-harness/src/router.ts` (+ `registry.ts`).
  - `route(command, registry)` → `{ DIRECT | RUN | BUILD | BOUNCE, loopId?, goal?, reason, confidence,
    announce }`. DIRECT is the DEFAULT (anything not loop-worthy). RUN on a SPECIFIC-tag match
    (generic tags green/loop/flow/wiring don't match alone; ties flagged ambiguous). BUILD when
    loop-worthy + no match + oracle-admissible (reuses `assessAdmissibility`). BOUNCE when no
    admissible metric. Routes only — never overrides Class A/B / admissibility / carve-out (§5).
  - Every decision ANNOUNCES one line; `toRoutingRecord` → append-only `runs/routing.jsonl` (§6).
  - CLI entrypoint (hook-usable, reads the prompt from stdin/argv, advisory exit-0).
  - 7 tests (DIRECT default · RUN match · retired-not-matched · BUILD · BOUNCE · always-announce ·
    scoreMatches ranking). Live over the real registry: git status→DIRECT · "i18n parity loop"→RUN
    i18n · "BE polishing loop"→BUILD · "make it prettier in a loop"→BOUNCE.
  - **Enforcement (§4) prepared, NOT installed:** `tools/loop-harness/router-hook.sh` is a ready
    `UserPromptSubmit` hook; the operator wires it into `.claude/settings.json` + a CLAUDE.md line
    (`.claude/` is protect-paths-gated → prepare-artifact, don't circumvent).
  - **Deferred (§8 step 5):** the LLM-classify fallback for genuine ambiguity (deterministic-first is
    proven; add the small classification pass only for the ambiguous tail) + routing recall (§6
    learning command→loop mappings). The router is a thin decision, not a Loop (no breaker/report).
