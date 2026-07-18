# SYNTHESIS SCORECARD — 5-Persona Hostile Ecosystem Audit (2026-07-18)

> Synthesizes: `AUDIT-2026-07-18-{FEYNMAN,HERZOG,TORVALDS,ARCHITECT,PERFORMANCE}-*.md`.
> Read-only audit pass, per operator directive — no fixes applied here. One exception pending
> operator confirmation: `.env` mode 666 (§4).

## 1. Category scorecard (Stability / Productivity / Growth Potential / Failure Risk)

Grades A–F, same scale the ARCHITECT report already established for cross-consistency.

| Component | Stability | Productivity (real throughput) | Growth Potential | Failure Risk |
|---|---|---|---|---|
| **CORE** | C — 540+ tests green, but Feynman found the canonical field operator is 4 different equations across the codebase, one numerically unstable (spectral radius 1.588, `\|u\|→10³⁹` in 200 steps) inside a check labeled "fail-closed" | B — kernel micro-benches hold up when re-run (place_order 73.2ns, token_bucket 52.0ns); genuinely fast where tested | **A-** — real strengths every reviewer independently confirmed (deterministic Law, real PQ crypto, honest RED→GREEN culture) | C — the instability lives inside a *safety* check; a DeadProbe found in P40/P41's own no-AI verification means at least one "proof" isn't proving anything |
| **PROTOCOL** | C+ — real crypto (a genuine SSR-2020 forgery independently caught and fixed), but 2 live regressions (no_std RED, insecure-TLS default-on) confirmed AGAIN this pass, still open | C — event-log fsync ceiling measured at ~1,650 durable events/s, never benchmarked in any design doc that assumes it | B — ~70% built-and-proven delivery-domain remains real, unchanged by this audit | D — 100% stranded from dowiz's own kernel; a lever this large sitting unconnected is itself a risk (drift, bit-rot, the exact pattern already found 3x this session) |
| **DELIVERY** | **F** — the one loadable page (`web/index.html`) crashes instantly in any real browser (imports Node's `fs`, calls `process.exit`); root `/` serves a placeholder Figma mockup of a fictional pizzeria | F — nothing to measure, 0% deployable | D — real design substrate exists (Sea&Sheet, narrative-cinematic layer) but zero of it reaches a screen | **F** — the interface actively misrepresents its own state (README claims rendering that doesn't happen) |
| **AGENT** | D — `AgentLoop` has zero callers; MCP server has no binary; local-browser MCP design is 100% unbuilt | F — 0 of 20 designed metric IDs emit anywhere; no observable throughput exists to grade | C — Ollama client is real and works; fine-tuning correctly deferred with real criteria (a genuine strength, not just an absence) | C — mostly inert rather than actively dangerous, but inert-and-unmonitored is its own risk class |
| **ECOSYSTEM/OPS** | **F** — Telegram monitoring root-caused as fully broken (5 Python scripts deleted 2026-07-17, 0 of 5 Rust replacements ever compiled or wired, exporter dead 37+ hours); `pgrust.service` targets a binary that doesn't exist | F — nothing running to measure | D — real infra exists on paper (Hetzner object storage, disk-cleanup done, backup topology designed) | **F (CRITICAL)** — `.env` mode 666 with a single, un-backed-up copy of the courier PII encryption key + JWT signing key + CF token; separately, `dowiz.fly.dev` is LIVE PRODUCTION (HTTP 200) serving source code deleted from the repo 5 days ago — unpatchable by construction |

**Overall system grade: D+.** Real, load-bearing engineering strength exists in CORE and PROTOCOL
(independently confirmed by all 5 reviewers, not just asserted). Everything from DELIVERY outward
— the layer a human or a dollar actually touches — is either non-functional, unmeasured, or
actively lying about its own state. The gap between the two halves is the whole finding.

## 2. Cross-cutting pattern (found independently by 4 of 5 reviewers, different angles)

**"Stated as fixed, never verified live."** The exact same failure shape recurs at every layer:
- FEYNMAN: the canonical operator is asserted once, implemented four different ways, never
  cross-checked against its own claimed derivation.
- TORVALDS: telemetry was "replaced" by a commit message, the replacement was never compiled.
- PERFORMANCE: a benchmark-regression fix (REGRESSION-LEDGER row 23) silently re-opened; the gate
  that should have caught the re-opening is itself fail-open by construction (3 independent ways).
- ARCHITECT: `dowiz.fly.dev` still answers HTTP 200 for source that's been gone 5 days — nobody
  checked whether "deleted" meant "also stopped serving."
- HERZOG: a README asserts rendering that has never once produced a pixel.

This is the SAME meta-failure P56 was built today to catch automatically (its `StaleGround`
detector's own worked fixture is this exact pattern). The irony, stated plainly per the audit's
own rule: **the meta-verification system designed to catch "claimed-done-but-isn't" is itself
currently 0% wired** (Performance's finding) — the fox is still designing the henhouse's locks.

## 3. GO/NO-GO (Architect's verdict, restated as the load-bearing conclusion)

**NO-GO for real orders. Months away, not weeks.** No process on this machine accepts an order
on any channel. The single fastest real path to a live "first order" is not forward — it's
reverting the purge (`79ef316f6`) and patching the OLD stack that's still, right now, silently
serving real traffic on `dowiz.fly.dev`. That is the honest, load-bearing fact this entire audit
converges on from five independent angles.

## 4. Immediate action required (severity CRITICAL, not deferred to a future wave)

Per the operator's "change nothing" directive, none of these were touched. Listed by urgency,
none require a design decision — all are mechanical/operational:

1. **`.env` mode 666 → 600.** One command, zero design risk, closes an actively-exploitable
   write-access hole on the courier PII encryption key. **Awaiting explicit operator go-ahead**
   (asked directly, not yet confirmed as of this synthesis).
2. **`dowiz.fly.dev` zombie.** Decide explicitly: kill it, or restore its source so it's patchable
   again. Silently leaving it running unpatchable, serving real customers, is the worst of the
   three options and is the current default.
3. **Telegram/monitoring blindness.** Either finish compiling+wiring the 5 Rust replacements, or
   revert to the deleted-but-working Python stack until they're ready. The current state (2
   duplicate daemons interpolating a dead exporter's silence into fake-looking metrics) is worse
   than having no monitoring at all — it *looks* alive.
4. **Bench-regression gate.** Fix the 3 independent fail-open paths (partial baseline coverage,
   exit-0-always tracker, the explicitly-rejected cross-host comparison still running in CI) —
   this is what stops finding #2/#3's class of problem from recurring silently a fourth time.

## 5. Full finding index

| Report | Findings | Critical | High | Medium | Low |
|---|---|---|---|---|---|
| FEYNMAN (meta-patterns/math) | 21 | 1 | 2 | 12 | 6 |
| HERZOG (interface/design) | 11 | 1 | 3 | 5 | 2 |
| TORVALDS (code/ops) | 32 | 2 | 9 | 16 | 5 |
| ARCHITECT (whole-system, hostile) | 16 + addendum | — | — | — | — |
| PERFORMANCE (metrics/benchmarks) | ~10 | — | 1 (gate fail-open) | — | — |
| **Total** | **90+** | **4+** | **15+** | **33+** | **13+** |

Full detail, evidence, and fix-guidance per finding: the 5 source reports in `docs/research/`.

---

## 6. Fix-work priority triage (analyst-append, 2026-07-18)

> Read-only audit + this triage: NO code/fix applied. This section SORTS the fix-work by
> priority so the operator can dispatch waves in the right order. Priority axis = exploit/correctness
> exposure first, then observability that hides future breakage, then deployability/money-path, then
> cleanup. "Blocker" = needs operator red-line decision before a fix may touch it.

### Tier 0 — BLOCKED on operator go-ahead (red-line: secrets/auth/prod), fix is mechanical
1. **`.env` mode 666 → 600** — write hole on courier PII key + JWT key + CF token. One command. **BLOCKER: awaiting operator confirm** (§4.1).
2. **`dowiz.fly.dev` zombie** — LIVE PROD serving source deleted 5 days ago, unpatchable by construction. Decide: kill or restore source. **BLOCKER: operator decision** (§4.2).
3. **Telegram/monitoring blindness** — finish 5 Rust replacements OR revert to deleted Python stack. **BLOCKER: operator decision** (§4.3).

### Tier 1 — CRITICAL correctness (silent data corruption / safety check that lies) — NOT blocked, dispatch now
4. **FEYNMAN canonical field operator: 4 inconsistent equations, one numerically unstable** (spectral radius 1.588, `‖u‖→10³⁹` in 200 steps) *inside a "fail-closed" check*. Highest-leverage kernel fix: collapse to ONE derived operator, cross-check vs its own derivation. Stable, verifiable.
5. **PROTOCOL no_std RED regression** — confirmed open again this pass. Breaks the PQ core build on no_std targets.
6. **PROTOCOL insecure-TLS default-on** — confirmed open again. Ships an insecure default; flip to secure-by-default.

### Tier 2 — CRITICAL observability (the meta-failure: "claimed fixed, never verified") — NOT blocked
7. **Bench-regression gate fail-open (3 paths)** — partial baseline coverage, exit-0-always tracker, rejected cross-host CI comparison. This is the gate that should catch Tier-1 regressions recurring silently. Fix first enables catching #5/#6's class a 4th time. (§4.4)
8. **P56 `StaleGround` detector / meta-verification: 0% wired** — the system built to catch "claimed-done-but-isn't" is itself unwired. Until #7+#8 land, every "fixed" claim is unverifiable by automation.
9. **P40/P41 DeadProbe** — a "proof" in the no-AI verification that isn't proving anything. Replace with a live probe.

### Tier 3 — Deployability / money-path (DELIVERY grade F) — BLOCKED-ish (large rewrite, needs design decision)
10. **`web/index.html` crashes on load** — imports Node `fs`, calls `process.exit`. Root `/` serves Figma mockup of fictional pizzeria. Blocks ALL real customer/revenue touch. **Needs operator scope decision (rewrite old stack vs new kernel UI).**
11. **DELIVERY 0% deployable / interface misrepresents state** — README claims rendering that never produced a pixel. Tie to #10.
12. **AGENT `AgentLoop` zero callers / MCP no binary** — already addressed in-kernel (P40 AgentLoop executor + tools, this session, GREEN, unpushed). Remaining: wire a host binary + emit the 20 designed metric IDs (currently 0 emit).

### Tier 4 — Quality / drift (lower urgency, real but not safety-bearing)
13. **PROTOCOL 100% stranded from dowiz kernel** — reconnect the ~70% proven delivery-domain. Drift/bit-rot risk, not active danger.
14. **Feynman medium-severity math (12) / Torvalds medium (16)** — batch-fix; each has fix-guidance in its source report.
15. **ECOSYSTEM `pgrust.service` targets nonexistent binary** — mechanical; tie to #3 revert-or-finish.

### One-line priority order to dispatch
`#1→#2→#3` (operator unblock) → `#4` (kernel correctness, biggest single win) → `#5,#6` (protocol regressions) → `#7,#8,#9` (observability so fixes STAY fixed) → `#10,#11,#12` (money-path, needs design call) → `#13,#14,#15` (drift/cleanup).

*Note: items #5,#6,#12's kernel half, #9's detection were worked this session (P40 AgentLoop GREEN, R-3/P34 PQ primitives GREEN, P47 red-line-clean scaffold). Those are capability builds, not the audit's regression fixes — the audit regressions above remain OPEN pending the operator's Tier-0 go-ahead and a Tier-1/2 fix wave.*

