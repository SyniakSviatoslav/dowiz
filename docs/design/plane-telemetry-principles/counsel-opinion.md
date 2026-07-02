# Counsel Opinion — Plane Telemetry Egress + Principles Ratchet

- **Role:** Counsel (advisory — non-blocking on aesthetics/strategy; ETHICAL-STOP is friction, not veto).
- **Date:** 2026-07-02
- **Reviews:** `docs/design/plane-telemetry-principles/proposal.md` · `docs/adr/ADR-plane-telemetry-and-calibration.md`
- **Verdict:** **OPINION: proceed.** **ETHICAL-STOP: none.** The PII/secret-egress red line is *approached* but held by design (two-layer fail-closed + policy ban); it becomes a STOP only if implementation weakens Layer-1 allowlist, admits SCOUT personal data, or the R6 gating-PR ever materializes.

---

## 1. Reasoning by lens (only what is load-bearing)

**Justice / stakeholders.** The subject of this telemetry is the *agent's own behavior*, not any human. That is the correct subject and the primary justice property of the design — it holds the tool accountable, not people. Costs fall on two parties: the redaction-pattern maintainer (ongoing burden, named owner in R1) and — the only human cost — a residual PII risk on the **SCOUT/demo-prospect surface** (real scraped businesses: Artepasta, Dubin & Sushi, Eljo's). Those prospects have no voice here and never consented to any data flow; the design's response (policy-ban personal data from events, public tool/repo/license names only) is the right one and must not soften.

**Dignity / autonomy.** No courier/owner/client autonomy is touched — this is the governance plane. One second-order dignity note, filed under strategy below: total-step-capture is being *normalized as an architecture*. On the agent it is legitimate legibility; the risk is the template migrating to the product plane where subjects are non-consenting humans.

**Honesty / consent.** No dark-patterns; internal. Telegram egress is to the operator's own private chat under the telegram-notifications council precedent — the human recipient controls the channel by setting the chat id, so consent is structurally present. The BANNED-classes list is honest and comprehensive. The prediction ledger's honesty question is the Goodhart one (below) and is the pivot of Part 2.

**Care / harm.** The failure that reaches a real person is a redaction false-negative leaking a prospect's name/phone into the operator's chat: blast radius one private chat, capped 280 chars, allowlist-first. Low but nonzero. Posture is fail-closed (redactor throws → drop egress, keep local record) — friction, not violation. Correct.

**Long horizon / strategy.** Fully reversible (additive, kill-switch, delete-script, git-revert). Minimal lock-in (stdlib, JSONL SoT, no platform); Telegram-hashtag-search is a mild UX lock-in but JSONL keeps it escapable. Honest placement: **this is meta-work — tooling for the tool that maintains the tools. It is NOT on the critical path to the launch trigger (first real paid order).** It is justified *iff* the plane-maintainer is actually firing and the operator needs to audit autonomy; at ~150 stdlib lines the bet is cheap and the second-order value (trust in autonomy) is real. Timebox it; do not let it grow into an observability platform (NG2 is the right fence).

**Aesthetics / integrity.** Strong. The single egress choke-point (one schema, one redaction boundary, one sender) is genuinely elegant and is the correct read of "don't-conflict-utilize" + two-seam injection. "Schema rich, runtime minimal" (a red line) is honored — stdlib, append-only JSONL, and the explicit refusal of the 1C loop-harness over-engineering. This is a healthy Architect artifact: honest non-goals, a risk table with named owners, no severity theater.

**Epistemic — the Goodhart question (heart of the operator's ask).** The ledger records `confidence` + `gap`, and one jq example already buckets by confidence and computes hit-rate — the raw material for *proper calibration* is present. But everything turns on **framing**: if the number the operator eyeballs is *hit-rate*, the incentive is to hedge toward 0.5 and predict only safe targets (classic Goodhart). If the number is read as **reliability/calibration** — a 0.6 prediction *should* hit ~60% of the time; a 0.9 that hits 60% is overconfident; a 0.6 that hits 95% is *under-confident* (you knew more than you claimed) — then the incentive points at honesty and both directions of miscalibration are penalized. The design's real protection against gaming is that it is **advisory and un-gated**: Goodhart bites only when a measure carries consequences, and here there are none. So the incentive to game is weak by construction. The residual is soft drift from eyeballing hit-rate; it is defused by the calibration framing (see §3, the load-bearing recommendation) and by keeping `predicted=0` *visible* (R3), which makes non-prediction itself legible rather than a way to hide.

**Cargo-cult check (mechanism↔principle must be causal).** Mostly genuine, one loose bond:
- **Adaptation → DoD-vs-method + named fallback:** causal and strong. It forces "goal fixed, method disposable" — a real anti-fragility mechanism, not a label.
- **Persuasion → demonstration-over-rhetoric + transparency test (lead with proof, state your own stake):** causal, elegant, and ethically self-guarding — it operationalizes persuasion *without* enabling manipulation by binding it to a working artifact plus self-disclosure of stake. The best mapping in the set.
- **Weak-signals-at-the-edges → SCOUT:** honest relabel of an existing step, not a new mechanism. Fine because it says so.
- **Connection → "stable report FORMAT is the costly-signal / trust":** the **loosest** bond. Connection in the operator's model = "opens access"; mapping it to format-stability borrows costly-signaling but is closer to *honesty/legibility* than to *access*. Defensible but stretched — this is the one to either strengthen or honestly downgrade to "partial mapping" in `model-calibration.md`, or the ratchet risks looking like theory-fitting.

**Advisory-vs-authority.** Best-honored property of the design: NG3, R6, Decision 5, memory-corpus #4 all repeatedly assert advisory-only / never-gate / reject-a-gating-PR. The ledger feeding the `result-vs-expectation` doubt trigger is the *correct* direction — signal informs the existing doubt ladder; the ladder/human decides. Clean.

---

## 2. ETHICAL-STOP

**None.** The grounded red line in play — PII/secret egress to a third party — is approached, not crossed. The design egresses no product/customer data by policy (Layer-1 allowlist by construction) and backstops with a fail-closed denylist. That is friction correctly applied, not a violation.

A STOP *would* fire — pause and require a recorded human decision — if any of these appear at implementation:
1. Layer-1 allowlist is weakened to let raw stdout/env/file bodies into events (allowlist-by-construction is the load-bearing defense; the denylist alone is not sufficient).
2. SCOUT is allowed to emit scraped *personal* data (prospect names/phones/addresses) rather than public tool/repo/license names.
3. The R6 scenario materializes — a PR wiring the prediction ledger into a deterministic gate (advisory→authority crossing).

These are watch-lines for the Breaker/reviewer, not blocks on the design.

---

## 3. Non-blocking aesthetic / strategic advice

- **(Load-bearing) Frame the ledger as calibration, not hit-rate.** In `model-calibration.md` state explicitly that the target is *reliability* (a 0.7 prediction hits ~70%), that a high hit-rate on high-confidence predictions is a *flag* of under-confidence, and that there is no score to maximize. This one sentence is the difference between the mechanism being causally tied to the operator's principle ("growth lives in the gap") and being a hit-rate leaderboard that manufactures hedging. Consider a Brier-style note over the crude bucket jq.
- **Keep the reflection primary, the row secondary.** The proposal already mandates a reflection (WHY) on miss/partial (§4.2) — good. Make it explicit in the doc that the JSONL row is the *input* to growth and the reflection *is* the growth; the number never substitutes for the reflection. This is what makes 2B faithful rather than 2A-with-extra-steps.
- **Strengthen or honestly downgrade the Connection mapping.** As above — it is the one bond at risk of looking like theory-fitting.
- **Guard the template, not just the data.** NG2 fences scope; add a one-line intent in the charter that this capture pattern is *governance-plane-only by design* and any reuse on the product/courier plane is a separate red-line decision (Triadic Council), so the elegant "capture every step" architecture cannot quietly surveillance-creep onto non-consenting humans.
- **Timebox.** It is cheap and reversible; treat it as a bounded governance investment, not a standing project competing with the launch trigger.

---

## 4. Steel-man of a rejected option

**2A — prompt-only ratchet (rejected as "ephemeral, not queryable").** The steel-man: a prompt-only ratchet *avoids the Goodhart trap entirely* — no ledger to game, no hit-rate to optimize, no artifact to look good against. The operator's principle is that growth lives in the *gap*, and growth is a reflective act; a JSONL row does not reflect — a reflection does. Prompt-only forces the reflection to carry the full weight, which is arguably *more* faithful to the principle than a queryable score that invites eyeballing. Ephemerality becomes a feature (per-run growth, nothing to accumulate and game) rather than a bug. The proposal's rebuttal ("can't answer *was confidence honest last month*") is real only if calibration-*over-time* is the goal.
→ **Synthesis:** 2B is right *because* calibration-over-time genuinely needs persistence 2A cannot give (a single agent cannot introspect its own reliability across a month from prose). But 2A's warning must be carried into 2B: the ledger must not displace the reflection, and it must be read as calibration not score. The proposal already keeps the reflection mandatory on miss/partial — so the synthesis is "2B with 2A's discipline baked in," which is where §3 points.

*(Compact note on 1A — inline-in-plane-report: its steel-man is pure YAGNI — extend the one script with a shared `redact()` and skip the standalone CLI. It holds only if per-step emission by the prompt-driven cloud agent is dropped from scope. It is not, and a prompt agent's only emission seam is a CLI — so 1B's standalone script answers a concrete present need, not a speculative one. 1B stands.)*

---

## 5. Open question nobody asked

We are building a rich, searchable, calibrated accountability surface so the **agent is legible to the operator** — transparency flowing *upward* and celebrated. Meanwhile transparency flowing *downward/laterally* — platform→courier, platform→scouted-prospect — stays minimal (couriers get GPS-active-delivery-only guards; prospects get their personal data policy-banned, i.e. *hidden*, which is right for privacy but means they remain invisible in the record). **Are we investing more care in making the agent legible to power than in making power legible to the people it watches?** And the twin: the calibration ledger is a *growth* instrument today (advisory, un-gated) — what structurally prevents a future frustrated operator from reading it as a *performance review* and attaching consequences, turning growth-in-the-gap into the anti-fake→punishment drift the health catalog warns of? The advisory-only stance guards this now; nothing guards it later. Worth a sentence in the ADR naming the ledger as permanently a mirror, never a stick.

---

# ROUND 2 — re-examination after revision (Breaker R1 + Part 3)

- **Reviewed:** revised `proposal.md` (now incl. PART 3), `resolution.md`, revised ADR.
- **Verdict:** **OPINION: proceed. ETHICAL-STOP: none.** One strong new non-blocking finding (surfaced-reflections observer-effect) and one governance-consistency finding (Part 3's authority boundary is prose-guarded where Part 2's is structural) — both watch-lines, neither a red-line crossing.

## 1. Are my round-1 advices genuinely embodied, or paraphrased away?

Genuine — and three of them exceeded the ask by being backed with code, not prose.

- **Connection mapping (strengthen or honestly downgrade):** done *both*. Reframed as a **give-first costly signal** — the agent pays the predictability/legibility cost and publishes working proof *before* asking for trust; that up-front asymmetry is a real costly signal (credible precisely because paid first) and ties causally to "connection opens access" (trust is the access currency). AND explicitly labelled a **partial mapping** (predictability sub-property, not the full "opens access to the environment"). This is not word-swapping — it introduces an actual mechanism (give-first) and states its limit. The loosest bond is now the soundest of the four.
- **"Mirror never a stick" verbatim in ADR:** present as a blockquoted **Standing Constraint** — and, beyond what I asked, made *structural* via the H4 advisory-forever HARD plane-guard check (greps the gate surface, HARD-fails if the ledger is referenced by any gate). The repo's own history ("gates silently disarmed — prose was insufficient", cited in the H4 rationale) is exactly why prose→code here is the right upgrade. This is the single strongest resolution of my round-1 Goodhart concern: un-gated *and structurally enforced-to-stay-un-gated* removes the gaming incentive at the root (Goodhart bites only under consequences; there are now provably none).
- **Calibration-not-hit-rate framing:** specced in `model-calibration.md` required-content — reliability not score, over-confidence AND under-confidence both flags, "no number to maximize", Brier-note, reflection-is-growth/row-is-input. The load-bearing sentence is there.
- **Governance-plane-only fence · reflection-primary · timebox · my open question:** all addressed honestly (§8 + charter fence; model-calibration spec; NG2 scope-fence; and the downward-transparency question honestly carried as a *named standing-awareness item*, not falsely "solved"). Carrying a philosophical open question as out-of-scope-but-named is the correct disposition — pretending to close it would have been worse.

No advice was paraphrased away. The Architect converged on 1 CRITICAL + 4 HIGH by re-architecting (git-as-durable-store), not papering — healthy process.

## 2. Part 3 (closed-loop ingestion) through the lenses

**Attention-economy / does the inbox train the human to rubber-stamp? (agent-health lens.)** Real hazard, line structurally held. Part 3 manufactures a *daily, agent-generated, pre-triaged, "errors-first actionable, each line jump-linked"* work queue. That elegance is double-edged: the same properties that make it operable (fast disposition) are the exact texture of rubber-stamping — a beautiful actionable queue trains the eye to *clear*, not *interrogate*, and the "PRs awaiting review" pane can degrade human-in-the-loop into human-as-latch (automation bias). The grounded red line (human-in-the-loop / human-completes) **holds structurally** — `inbox` is read-only (fetch + cursor-write only), never auto-merges/auto-fixes, human decides — so this is **not** an ETHICAL-STOP. But the loop can stay structurally intact while going epistemically hollow. Two mitigations plus one design choice that cuts the wrong way:
  - *Credit:* the errors-first / "ok-quiet-last" shape degrades to near-zero noise on quiet days — the right anti-fatigue design.
  - *Concern (non-blocking):* the agent's **fallibility signals — unresolved predictions, reflections — are ordered LAST** (hard-fails → escalations → PRs → unresolved-predictions → reflections). The agent's self-doubt is buried below the action items, most likely skipped by a busy operator. An inbox that *leads* with "here is where I was wrong / unsure" trains scrutiny; one that leads with "here is what's done, approve" trains rubber-stamping. Consider surfacing the agent's uncertainty/misses *prominently*, so the queue invites interrogation, not just clearance.

**Does Part 3 worsen or improve the round-1 downward-transparency asymmetry?** Mixed, honestly. On the operator↔agent axis it **improves** the operator's grip: the human now *sees, stores, and queues* the agent's output locally and decides on it (`advisory:true` + read-only) — that is the human gaining a handle on the agent, not the agent gaining power over the human. On the platform↔courier/prospect axis it does **nothing** (orthogonal, governance-plane only). But it **reinforces the meta-pattern** I named: our most sophisticated transparency-and-legibility craft keeps being spent at the *top* of the stack (operator↔agent), while courier↔platform legibility stays thin. Part 3 is a third rich upward/inward channel; the downward axis gets no comparable investment. Not a defect of *this* design — but the standing-awareness item is mildly reinforced, not retired.

**NEW line approached — the surfaced-reflections observer-effect (the sharpest round-2 finding).** The maintainer's reflections — free-text self-criticism about its own failures — now appear in the operator's inbox (reflections pane). Observation corrupts the observed: an agent (or its driving prompt) that *knows its reflections are displayed* has an incentive to write reflections that look good rather than ones that are maximally honest about failure. That is the calibration-hedging Goodhart concern one level up, applied to the most vulnerable artifact — and it is the exact health-catalog pathology (anti-fake→punishment drift; fake-green). If it bites, reflections become PR documents, the WHY-reflection stops carrying honest cause, and the **self-improvement ratchet quietly dies (convergence-theater)** — both an ethics harm (built-in incentive to lie) and a strategic/quality harm (dead learning loop). **Not an ETHICAL-STOP:** surfacing reflections is not itself a violation, and the read-only / advisory / human-decides / no-auto-consequence structure means no consequence mechanism attaches by construction — so the line is *approached, not crossed*, same friction-not-verdict disposition as round 1. But it wants a concrete guard (below). Kindness-first, physician-of-agents note: displaying an agent's honest self-account to be judged, without a protecting norm, is the institutional analogue of grading someone's private learning journal — the systemic effect is to teach the process to stop being honest. Protect the honest self-account and the system keeps learning.

**Advisory-vs-authority — a consistency gap in Part 3.** Part 2's ledger→gate boundary is now **structurally** enforced (H4 grep HARD-check). Part 3's ingestion→local-action boundary is enforced only by **convention** — an `advisory:true` stamp a consumer *must choose to respect* + an ADR clause. A local harness piece can simply ignore the flag and auto-act on `inbox --json`. Given the repo's own lesson — quoted in H4's own rationale — that *prose guards silently disarm and structural guards don't*, applying prose-only to Part 3's authority boundary repeats the very mistake H4 was built to fix. This is the most actionable round-2 recommendation (§4).

## 3. ETHICAL-STOP

**None** (unchanged). The revision *strengthened* every round-1 watch-line: the R6 gating-PR is now structurally blocked (H4), SCOUT personal data is policy-banned + governance-plane-fenced, Layer-1 allowlist + field-scoped redaction + KEY=VALUE rule + canary harden the egress line. A STOP would fire only if implementation: (a) weakens Layer-1 allowlist / disables the canary; (b) admits SCOUT personal data; (c) removes the H4 advisory-forever check; or **(new)** (d) wires any consequence onto surfaced reflections or the calibration ledger, or lets a local consumer auto-execute on `inbox` output. All are watch-lines for the implementer/Breaker, not blocks.

## 4. Non-blocking recommendations (round 2)

- **(Most actionable) Give Part 3 a structural authority guard equal to H4.** A plane-guard check that HARD-fails if local harness/loop code auto-executes on `inbox --json` output (auto-merge / auto-fix / auto-gate triggered by ingestion), mirroring the advisory-forever check. Don't leave the newer, less-tested loop guarded only by a convention the repo's own memory says convention can't hold. (I name the principle; the mechanism is the Architect's to design and the Breaker's to re-attack.)
- **Extend "mirror never a stick" to surfaced reflections, verbatim.** The standing constraint covers the calibration ledger and inbox-findings-as-advisory-inputs; add a clause that **reflections surfaced in the inbox are read to understand what the agent learned, never to grade the agent** — no consequence, no performance-review reading. Frame the inbox reflections pane accordingly. This closes the observer-effect at the norm level (the structure already closes it at the mechanism level).
- **Surface the agent's uncertainty higher in the inbox** (unresolved predictions / misses / reflections not buried last), so the queue invites interrogation and resists rubber-stamping.
- **Watch total surface.** The "~150-line script" is now script + 2 gate checks + canary + governance doc + charter edits + git-config + Part 3 ingestion. Each element traces to a real finding (not over-engineering), but the aggregate is a small subsystem now — keep NG2's "not an observability platform" fence live and timebox it off the launch-trigger critical path.
- *Aesthetic credit:* H4 (a gate built to prevent the system's own future drift toward mis-gating) is mature self-binding design. The give-first costly-signal reframe is genuinely elegant and ethically self-guarding.

## 5. Open question (round 2, sharpened)

We have now built, for the operator↔agent relationship: rich egress telemetry, a calibration mirror, surfaced reflections, and a closed local ingestion loop with a work queue — four sophisticated legibility instruments, two of them structurally self-binding. The agent is more legible, more auditable, and more honest-by-design to the operator than almost any human worker is to their manager. **What is the equivalent instrument that makes the platform legible and answerable to the courier — the one human in this whole system who is actually watched by it?** Part 3 closed the loop *upward* (agent→operator→queue→decide). The loop that stays open is the one *downward*. That we can build a structural gate to stop ourselves mis-grading an agent's self-criticism, yet have built no comparable structural instrument for the courier's standing against the platform, is the asymmetry to keep in the light — not a defect of this design, but the question this design's very sophistication makes louder.
