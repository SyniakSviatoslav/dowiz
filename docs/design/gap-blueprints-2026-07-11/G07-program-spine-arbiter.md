# G07 — Program Spine Arbiter: ranking the four competing futures

> Gap blueprint, 2026-07-11. Research + design only — no code, no ranking is in force until the
> operator signs the draft in §4. Sources: full audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md`
> (§1, §2.3, §7.1, §7.7, §7.8, §8, §9 rec 2), the living-memory corpus
> (`/root/.claude/projects/-root-dowiz/memory/`), `docs/design/rebuild-plan/REBUILD-MAP.md`,
> `docs/design/dowiz-brand/EXPANSION-PLAN.md`, `/root/bebop-repo` (README, `bebop2/ARCHITECTURE.md`,
> `docs/design/delivery-protocol/`), and `DeliveryOS-Business-Value-Sort.md`. Every load-bearing
> claim below is traceable to one of these; estimates are labeled as estimates.

---

## 1. Gap & evidence

**The finding (audit §7.1, its highest-order finding):** four program-level narratives each carry
an "authoritative" marker and compete for one operator's bandwidth. Nothing ranks them. Each was
declared "the program" while it was active. None has been closed. The serial pivot between them is
the mechanism by which everything else stalls — the cutover half froze the day the Sovereign Core
arc opened; the MVP exit gate froze the day bebop-repo opened; bebop itself has already pivoted
internally three times in 3.5 days (coding agent → physics planner → delivery protocol → PQ crypto).

**The four authoritative markers, verbatim:**

| Future | Declared | Marker (exact text) | Where |
|---|---|---|---|
| (A) Rebuild S1–S10 cutover | 2026-07-04 | "This is now **the program spine**" (operator directive) | memory `rebuild-decision-rust-astro-2026-07-04.md` |
| (B) Sovereign Core MVP + Expansion | 2026-07-05→07 | "**Authoritative** living plan" / MANIFESTO = "the **final state of truth** for the product" | `docs/design/dowiz-brand/EXPANSION-PLAN.md` line 2; memory `sovereign-core-mvp-handoff-2026-07-06.md` |
| (C) Open-source flip (ADR-020) | 2026-07-03 | "**FINAL** program goal" | memory `open-source-goal-adr020-2026-07-03.md` |
| (D) bebop / bebop2 | 2026-07-08→now | "Operator directive (2026-07-10) … it is **the API contract the agents implement against**" | `/root/bebop-repo/bebop2/ARCHITECTURE.md` |

**Corroborating evidence gathered for this blueprint (all verified read-only 2026-07-11):**

- **Attention is measurably monotonic and total.** Commits/day: dowiz peaked at 120 (07-05,
  rebuild) and 58 (07-07, sovereign core), then collapsed to 2 (07-08), 0 (07-09), 3 (07-10),
  0 (07-11). bebop-repo over the same window: 22 → 39 → 44 → 5 (+ a 1,331-line uncommitted diff).
  Since 07-08 the live-revenue repo has received 5 commits; the 3-day-old repo received ~110.
- **The "FINAL goal" has no charter.** ADR-020 is referenced in at least 8 committed dowiz docs
  (`ADR-safe-reversal-spine.md`, `ADR-audit-fix-rls-reliability.md`, REBUILD-MAP, rebuild-plan
  inventories, ops runbooks) — but **no ADR-020 file exists** in the working tree, on `origin/main`,
  or anywhere in ref history (`find` + `git log --all --diff-filter=A` + `git grep` on origin/main
  all empty). Its only canonical text is a living-memory file. A program-level goal whose charter
  is a memory note is exactly the "authority without a spine" failure this gap names.
- **Nothing is ever explicitly parked.** The corpus is excellent at recording state (152 files)
  and contains **zero PARKED markers**. Arcs end in "gated-but-unclosed" (audit §2.3). The
  cross-branch todo map (07-10) — the closest thing to an arbiter — was outdated within 24 hours
  (it predates the bebop2 crypto pivot and the 9 new detritus files).
- **The cost is not hypothetical.** Concrete rot attributable to unranked pivoting: staging
  cutover state unknown for 6 days (h_t frame pinned to an abandoned branch lineage); the 085
  migration's "2026-07-10 HARD gate" date passed with no recorded disposition; two whole PQ crypto
  implementations exist only in an uncommitted working tree on one machine; GDPR fixes staged but
  not in prod for 6+ days; cross-repo detritus recurred within 24h of being "fixed".
- **The sharpest lens (audit §7.8 + Business-Value-Sort Tier 4):** since the Business-Value-Sort
  warned that the process apparatus risks becoming "витончена заміна" (elegant procrastination) for
  the one validating event — **a real restaurant placing a real paid order** — a complete Rust
  rewrite was built dark, a kernel was event-sourced, and a crypto library was hand-rolled. The
  audit found **no evidence of a single real (non-demo) production order or claimed venue**. The 12
  outreach demos remain shadow tenants; WS3/WS4 outreach was never built.

**Audit recommendation 2 (this blueprint's mandate):** write a one-page, operator-signed arbiter
doc ranking the four, giving each a "next session does X" and an explicit PARKED marker for the
rest.

---

## 2. Research findings — the four scorecards

Burn figures are estimates from commit-density, memory-file dates, and session-transcript counts
(120 dowiz + 27 bebop transcripts); token counts are not directly recoverable and are stated as
session-equivalents. "% complete" figures are the audit's own (§8).

### Scorecard A — REBUILD-MAP S1–S10 cutover ("the program spine", 07-04)

| Axis | Assessment |
|---|---|
| **% complete** | ~50% by the program's own definition: build-half ~100% (10/10 surfaces dark, council-approved, 1,041 Rust `#[test]` fns), cutover-half ~15% (mechanism proven on staging, 9/10 flagged there 07-05, **prod 0%**), FE ~10% (3/27 Astro islands), decommission 0%. **Frozen since 2026-07-05.** |
| **"Done" means** | Phase D: Node removed, full 179-spec parity run, 48h staging soak, reliability-gate GO, all regression-ledger rows re-proven on the new stack, per-surface prod flips with S5-money/S9-GDPR explicit operator confirm, REV-C10 decommission owner+date filled. |
| **Distance-to-value** | **Earns no new revenue by construction** — it is a parity rebuild (CARRY-VERBATIM default; the oracle asserts identical behavior). What "done" earns: maintainability, perf/RSS, compile-time money correctness, OSS contributor gravity (per the 07-04 decision memo), and retirement of the two-stack tax. Pure cost-reduction + optionality; value realized only if the product has a future worth maintaining. |
| **Burn so far** | ~211 commits in 48h (07-04/05), 6 inventory lanes + 5 research lanes + 8 councils (S2–S10 packets with breaker/counsel per surface), a 2,344-row traceability.csv. Est. **10–15 sessions**; operator gates consumed: ≥10 explicit sign-offs (stack override, S2/S3/S4 §4 signatures, ETHICAL-STOP lift, council waivers). |
| **Decay if parked** | **HIGH and already accruing.** The parity oracle and keep-set rot silently as the Node stack evolves (staging has since been redeployed from sovereign-core lineages — current flag state unknown, audit risk #4); migrations 085–089 remain unplaced drafts; the 085 watermark date passed unrecorded; the bifurcated-history gap (997/941 commits) widens the eventual merge. A half-flipped staging with unknown state "is the worst of both" (audit rec 7). |
| **Reversibility of parking** | MEDIUM. The flip mechanism itself is reversible by design (1 UPDATE, 2,384ms rollback proven). But re-entry cost grows with every Node-side change (parity re-verification, oracle re-run, keep-set regeneration). Parking is safe **only if the parked state is written down** (staging re-probe + dispositions) — otherwise it silently converts to abandonment. |
| **Interdependencies** | The Sovereign Core kernel physically lives inside this tree (`rebuild/crates/domain`) — B continues even if A parks. The cutover front-door lives in the Node app and is inert (safe). A's prod flips share the same unsolved bifurcated-history merge design as B's prod merge. The Rust story materially helps C (self-host + contributor gravity, per the 07-04 memo). |

### Scorecard B — Sovereign Core MVP + EXPANSION-PLAN ("AUTHORITATIVE", 07-07)

| Axis | Assessment |
|---|---|
| **% complete** | ~58% of the MVP exit gate: 0b-1..0b-5 done (0b-5 inject-deploy-revert RED proof on staging v265→v266 — the strongest deployed-reality proof in the project), 1.1/1.2/1.5 + 2.2/2.3 done and staging-deployed. Open: 1.3 (sync port), 1.4 (signed envelope), 2.1 (distribution artifacts), 2.4 (aggregator stub), 0b-6 (CI gate), the full staging-validation checklist (never recorded green), and the prod merge. EXPANSION-PLAN Layers 0–2 are almost entirely open. |
| **"Done" means** | MVP exit gate: own-channel owner data-hub live in prod — direct 0%-commission checkout routed through `kernel::decide`, authoritative event log, customer ownership, channel attribution. (EXPANSION-PLAN "done" is much larger — brand, Better Auth, doors, telemetry — and should be scoped separately.) |
| **Distance-to-value** | **Closest of the four to product value, with one honest caveat:** prod already has a working direct checkout (Node). The MVP's *new* value is architectural (event-sourced kernel, attribution, customer ownership) plus the doors it unlocks (QR → TMA → WhatsApp — the checkout channels a real venue would actually use). Revenue still requires a venue; the MVP makes the offer crisper, it does not create demand. Also: 2.2/2.3 staging work directly serves validation measurement (attribution dashboard = funnel telemetry). |
| **Burn so far** | 07-05→07-07: ~91 dowiz commits, 16 memory files on 07-07 alone, multiple cloud autobuild sessions (scheduled routine + two concurrent cloud sessions on 07-06). Est. **12–18 sessions** including the harness/token-economy work entangled with this arc; operator gates: F4 fork pick, 0b-3 signature decision, council purge, harness unlock, "push further" routing decisions. |
| **Decay if parked** | LOW–MEDIUM. Everything is committed and pushed on `feat/sovereign-core-phase-zero`/paleo lineage with an excellent resume cursor (PROGRESS.md + handoff memory). Two live decay points: `hub_checkout` flag's "verify in next session" was never recorded done, and staging keeps being redeployed (the 2.2/2.3 validation evidence goes stale). Prod merge shares the bifurcation problem. |
| **Reversibility of parking** | GOOD — the best-documented arc in the project; re-entry is one read of PROGRESS.md. |
| **Interdependencies** | Lives inside A's tree but does not require A's cutover. **B's own plan contains C's gates** (EXPANSION-PLAN Layer 0 = secrets scrub, license flip, scanners — verbatim ADR-020 items). B's prod merge is the natural pilot for the rewrite-aware merge design that also unblocks the GDPR trio (audit rec 1). The doors (QR/TMA) are the features venue validation would exercise. |

### Scorecard C — Open-source ADR-020 flip ("FINAL goal", 07-03)

| Axis | Assessment |
|---|---|
| **% complete** | ~20% of prep, 0/3 hard gates. Done: creds rotated, compliance repo public-ready (old memory), DCO/AGPL *concepts* settled. Not done: LICENSE is still Apache-2.0 while Cargo.toml says AGPL (half-relicensed inconsistency, found by the 07-07 research fleet), TRADEMARK.md absent, secrets **history** scrub not executed (26 dirty remote branches), gitleaks not installed (CI false-green, known since 07-08), EUTM not filed ("DeliveryOS is a weak descriptive mark" — brand decision open), pricing-v2 landing not built. And the ADR itself was never committed (§1). |
| **"Done" means** | Repo public under AGPLv3 + TRADEMARK.md + DCO, pricing-v2 hosted-cloud as the only sold path, history verifiably clean, security posture publishable. |
| **Distance-to-value** | Distribution + optionality, no direct revenue. Contributor gravity and self-host credibility are real but **amplify an audience that does not yet exist** — open-sourcing an unvalidated product earns stars, not customers. The pricing-v2 frame is the monetization story and it presupposes demand. |
| **Burn so far** | **Lowest of the four** — est. 2–4 sessions (ADR drafting, the OSS-hardening research inside the 07-07 fleet, memory upkeep). |
| **Decay if parked** | LOW. Gates don't rot. Two slow leaks: every new commit marginally grows the eventual scrub surface, and the half-relicensed state is a latent legal inconsistency that should be reconciled regardless of the flip. The gitleaks false-green is a live security gap independent of C (fix it now, it costs one line). |
| **Reversibility of parking** | EXCELLENT — parking is the safe state. **The flip itself is the irreversible thing** (one-way door: history + secrets out forever). C is the one future where "parked by default, opened deliberately" is unambiguously correct. |
| **Interdependencies** | Hard-gated on the secrets remote scrub — which should be designed as one operation with the bifurcated-history merge (both are history-rewrite events on the same remote; doing them separately means two force-push windows). Strengthened by A (Rust self-host story). Note the revealed preference: **bebop-repo shipped full OSS scaffolding (AGPL, GOVERNANCE, DCO, wiki) on day one** — the OSS instinct is genuine; dowiz's gate friction redirected it to the ungated repo. |

### Scorecard D — bebop / bebop2 (agent → protocol → crypto, 07-08→now)

| Axis | Assessment |
|---|---|
| **% complete** | Category-dependent, and that is the finding. As a dowiz rewrite: **0%** (zero product surface, grep-verified — the audit's central correction). As its own project: bebop host CLI substantial (~16K LOC, 275 tests, real review gate); bebop2 crypto core ~85% (2 primitives — Argon2id, ML-DSA-65 — **uncommitted**); bebop2 kernel/cli/reloop 0%; old↔new equivalence harness 0%; delivery protocol: disciplined design docs + tested library primitives (matcher/PoD/reputation/ledger), **no network layer, no nodes, no deployment, no users** — `delivery/` is empty. |
| **"Done" means** | **Undefined — no exit gate exists anywhere in the repo.** Four ambitions in 3.5 days, each internally reasoned, none with a DoD. The most honest available "done"s: (agent) a tagged binary release with ≥1 external user; (crypto) bebop2 replaces `crates/bebop` behind the planned equivalence oracle + external audit before guarding value; (protocol) one live two-node match→PoD→settle loop with a real courier. None is scoped. |
| **Distance-to-value** | **Longest of the four.** The agent CLI competes in a crowded space with zero distribution effort so far. The crypto is explicitly not shippable for money/identity without external audit (its own ARCHITECTURE warns against optimizing PQ into insecurity; the audit calls KAT-green necessary-not-sufficient). The protocol is pre-cold-start, and — decisive — **its own research says the cold-start play is "be the backend for direct orders … restaurant keeps margin" (DECOUPLED-MATCHER §4), i.e., D's path to value runs directly through dowiz having real venues.** What D earns today is learning and long-horizon optionality; the web3-logistics postmortem it commissioned lists exactly how this class of project dies ("value at node one or die at cold-start"). |
| **Burn so far** | ~110 commits + 27 sessions in 3.5 days (a faster burn rate than any dowiz arc), consuming ~100% of operator attention since 07-08. **0 memory files** despite 27 sessions. |
| **Decay if parked** | **HIGHEST acute decay — but only until a capture session runs.** Today: two complete PQ implementations (+1,331 lines) exist only in an uncommitted working tree, 5 commits unpushed, on one machine, with no memory corpus and stale ARCHITECTURE/CHANGELOG docs. Parking *without capture* loses the most of any option. After one commit+push+memory-bootstrap session, decay drops to LOW (pure library code, no live deployment to rot). |
| **Reversibility of parking** | POOR today / GOOD after capture. |
| **Interdependencies** | The delivery-protocol thread is **the only place the two repos' futures genuinely intersect** (audit §8 end): a trustless dispatch layer dowiz could one day plug into. That bridge exists only as untracked research files sitting in the wrong repo. The three-model-review rule here vs. the proxy-purge rule in dowiz is an unreconciled philosophical fork (audit §7.10) that invites rule-shopping while both repos are "active". |

### Operator revealed preferences (from the corpus — characterization, not psychoanalysis)

1. **Overrides analysis toward building.** The 5-lane research verdict was *no-rewrite*; overridden
   within the session to "complete rebuild is the program" (07-04). Councils were waived (07-05),
   then purged (07-07, "ground truth over proxy"). The pattern: when process output says *slow
   down*, the operator keeps the deterministic gates and deletes the advisory layer.
2. **Honors hard gates even while sprinting.** Red-line prod merges (GDPR trio) sat operator-gated
   for 6+ days rather than being bypassed; "never bypass human gates" has held. The gate
   discipline works; **what's missing is an SLA on the queue, not compliance** (audit §7.3).
3. **Keeps returning to the same attractors:** sovereignty/decentralization ("no central server"),
   post-quantum identity, first-principles math/physics, token/cost economy. Every pivot since
   07-04 moves *down* the stack toward these — and away from market contact. No session since the
   corpus began has been spent on outreach (WS3/WS4 remain unbuilt).
4. **Parks by omission, never by decision.** Zero explicit PARKED markers in 152 memory files.
   Each future was left "gated-but-unclosed" — which is why all four still carry live authority.
5. **The OSS instinct is real** (bebop scaffolded AGPL+DCO+GOVERNANCE on day one) but flows to
   wherever it is ungated.
6. **Process churn is itself a cost the operator will pay** (§7.7: council → optional → purged;
   model routing v2→v3.4 in 4 days). A new governance artifact must therefore be *one page,
   deterministic, and cheap to keep current* — or it will be the next thing purged.

Design consequence of 1–6: the arbiter doc must be operator-signed (2), one page (6), enforce
parking as an explicit act (4), be deterministic-gate-backed rather than advisory (1), give the
attractor threads a legitimate parked home rather than pretending they'll be dropped (3), and
route the OSS energy through its gates rather than around them (5).

---

## 3. Options & tradeoffs — the rankings that could be signed

The audit (rec 9) and the Business-Value-Sort (Tier 4 → Tier 1+5) converge on the same claim:
**the binding constraint is unvalidated demand, not engineering.** Zero real production orders
exist after ~4 weeks of world-class engineering. Any honest ranking has to either accept that
claim or explicitly rebut it. Four coherent options:

### Option 1 — VALIDATE-FIRST (recommended; §4)
P0 = a standing business-validation track on the **live Node prod stack** (ship the GDPR trio,
fix the 3-kind 422 checkout bug, WS3 outreach, venue #1 in concierge mode). Among the four
futures: B active but throttled to what serves validation; A formally mothballed with a written
parked-state; D parked after one capture session; C parked behind its own gates (with the
gitleaks one-liner folded into P0 hygiene).
- **For:** directly attacks the only invalidated assumption; both strategy docs demand it; the
  GDPR-trio ship doubles as the pilot for the bifurcated-merge design every other future needs;
  cheapest information per session; every future's eventual value is conditional on its answer.
- **Against:** validation is operator-heavy work (outreach, human contact) that agents can only
  partially do — the real risk is that it stalls for the same reason it has stalled for a month,
  while the parked engineering futures rot. Mitigation: PARKED ≠ frozen-without-state — each gets
  a one-session closure/capture first, and the review date forces a re-decision in 2 weeks.

### Option 2 — FINISH-THE-SPINE (resume A to completion)
Engineering-coherence argument: the two-stack state is the single biggest ongoing tax (every
change made twice or diverging); a half-flipped staging is "the worst of both"; finish the cutover
now while context is warm, then validate on the clean stack.
- **For:** kills the parity-rot clock permanently; the mechanism is proven; ~58-route tail is
  enumerated, not open-ended.
- **Against:** the tail is uniformly red-line (money/PII/FORCE-RLS) — the slowest, most
  operator-gated work in the repo; the FE half (24 islands + budget decision) is months, not days;
  and it produces **zero new information about whether anyone will pay**. This is precisely the
  Tier-4 "elegant procrastination" shape: excellent, and not where the next unit of effort earns.
  Context is also no longer warm (6 days frozen, staging state unknown) — the "while it's fresh"
  argument expired on 07-08.

### Option 3 — DOUBLE-DOWN BEBOP (the protocol is the real prize)
Accept dowiz as a done-enough MVP in maintenance mode; chase the decentralized-protocol thesis
(trillion-class TAM per its own investability doc) with full attention.
- **For:** largest theoretical upside; the operator's attractors all point here; the work quality
  is genuinely high (384/384 tests, real adversarial review).
- **Against:** three independent rebuttals, two of them from D's own documents. (1) The protocol's
  cold-start plan requires restaurants taking direct orders — i.e., a working dowiz with venues;
  D cannot reach node-one value without P0 succeeding first. (2) The web3-logistics postmortem D
  commissioned: token/protocol-first without mandated pain at node one = ShipChain/FOAM. (3)
  "Maintenance mode" is false while prod carries known GDPR erasure gaps and a live checkout 422 —
  the floor for parking dowiz is shipping those, which is most of P0 anyway. Also: hand-rolled
  crypto guarding real value without external audit is a risk the repo's own docs refuse.

### Option 4 — OSS-FIRST (flip public for distribution)
Execute Layer 0 + the scrub + the flip; let contributors and self-hosters become the growth loop.
- **For:** C is cheap, the prep list is concrete, and public accountability could itself force
  product focus.
- **Against:** blocked on hard gates regardless (EUTM filing, scrub, brand decision — weeks of
  operator-side latency); it is the only *irreversible* move on the table, taken from the weakest
  possible position (unvalidated product, dirty remote, half-relicensed tree); OSS distribution
  amplifies demand that hasn't been demonstrated. Correct as the FINAL wave — which is exactly
  what its own memory file says ("this is the LAST wave").

**Position taken:** Option 1. The Business-Value-Sort's one-line conclusion — "обмеження це
невалідований попит" (the constraint is unvalidated demand) — has not been rebutted by anything
built since it was written; it has been confirmed by it. Options 2–4 all spend the scarcest
resource (operator bandwidth) on work whose value is *conditional on* the question Option 1
answers. The only serious counter-argument (validation may stall on operator-side friction) is
handled structurally: the arbiter doc's review date converts "stalled" from a silent state into a
forced re-decision.

---

## 4. Recommended ranking + ready-to-sign draft arbiter doc

**Recommended ranking of the four futures** (beneath the standing P0 validation track):
**B > A > D > C** — B is nearest done and nearest product value; A parks with a written state
frame and clear re-entry; D parks after a capture session (highest acute decay today) with its
protocol thread explicitly tied to P0's outcome; C stays parked behind its own irreversibility
gates, as its own charter always said.

The draft below is the deliverable of audit rec 2. It is written to be copied verbatim to
`docs/operating-model/PROGRAM-SPINE.md`, edited if the operator disagrees, and signed. It is
deliberately one page.

```markdown
# PROGRAM SPINE — the single arbiter of program-level priority
> Operator-signed. This is the ONLY document that may call a program "authoritative".
> Any other doc claiming authority is subordinate to this ranking and must link here.
> Machine-readable block at bottom; enforced by plane-guard P12 (see G07 blueprint §5).

## Standing track — P0: BUSINESS VALIDATION (dominates everything below)
The one invalidated assumption is demand. Until venue #1 places real paid orders, engineering
futures are throttled to what serves this track or is legally/ethically required.
NEXT SESSIONS DO: (1) ship the GDPR/webhook trio to prod (5ded9f19 · 58caf4f4 · d6b3473e) via a
rewrite-aware tree-merge, staging-rehearsed — this is also the merge-design pilot every future
needs; (2) fix the 3-kind 422 checkout bug (drafts exist); (3) install gitleaks (1 line, kills
the CI false-green); (4) WS3 outreach: QR + operator footer + claim flow on the 12 demos; venue
#1 concierge onboarding. SUCCESS = first real non-demo paid order. STALL = review date arrives
with no outreach session run → mandatory re-sign with an explicit alternative.

## Ranked futures
1. ACTIVE (throttled) — (B) Sovereign Core MVP.
   Next session does: run /reliability-gate L0–L11 vs staging for 2.2/2.3, record it green, then
   scope the prod merge on top of the P0 merge design. Throttle: sessions on B that do not serve
   P0 (attribution/checkout/doors) or the exit gate are out of scope. EXPANSION-PLAN executes
   ONLY its Layer-0 hygiene items that P0 needs; Layers 1–2 wait for venue #1.
2. PARKED — (A) Rebuild S1–S10 cutover.
   Closure session (one, timeboxed): re-probe staging cutover flags vs docs/ops/rebuild-cutover-
   h_t.json, record the 085-watermark disposition, write the dated parked-state frame.
   RE-ENTRY: ≥1 paying venue AND operator re-signs this doc. REVIEW: at each review date, decide
   keep-parked / resume / formally retire. The build-half is an asset either way; the kernel
   (crates/domain) stays live under B.
3. PARKED — (D) bebop / bebop2.
   Capture session (one, immediately — highest acute decay): commit+push Argon2id + ML-DSA-65,
   bootstrap /root/bebop-repo memory (MEMORY.md + bebop2-pivot arc note), move the 11 bebop
   research files out of /root/dowiz, fix the stale ARCHITECTURE/CHANGELOG claims.
   RE-ENTRY: P0 success milestone reached, or operator re-signs. The delivery-protocol thread
   re-enters ONLY once dowiz has real order flow (its own cold-start plan requires it).
4. PARKED (gated) — (C) Open-source ADR-020 flip.
   Prep allowed opportunistically (license-field reconciliation, TRADEMARK.md draft, commit the
   actual ADR-020 file — the FINAL goal deserves a committed charter). The FLIP stays behind its
   3 hard gates (history scrub · EUTM · explicit go) + re-sign. Design the remote scrub together
   with the bifurcated-history merge as ONE force-push operation.

## Standing rules (bind all agents and the operator's future selves)
R1. WIP limit: at most ONE engineering future ACTIVE at a time, plus the P0 standing track.
R2. A new future may only be opened by adding it here AND re-signing this doc. Commits into an
    unregistered or PARKED program area go RED (plane-guard P12) absent a human-only override.
R3. Every pivot away from an ACTIVE future requires writing its PARKED entry (state frame +
    re-entry criteria) BEFORE the new work starts. No silent abandonment.
R4. The word "authoritative" in any other doc is a claim ON this doc, not a grant BY it.
R5. Review date below is a hard gate: past-due = plane-guard RED until re-signed.

## Machine block (parsed by plane-guard P12 — keep valid)
    {"active":["B"],"parked":{"A":{"paths":["rebuild/web/**","apps/api/src/lib/cutover/**"],
    "review":"2026-07-25"},"D":{"paths":["<bebop-repo>"],"review":"2026-07-25"},
    "C":{"paths":[],"review":"2026-07-25"}},"review":"2026-07-25"}

SIGNED: ______________________ (operator)   DATE: 2026-07-__
```

Notes on the draft: the review date (2026-07-25, two weeks) matches the corpus's demonstrated
half-life of any governance artifact; the P0 stall clause is the honest answer to Option 1's main
weakness; A's paths glob deliberately excludes `rebuild/crates/domain/**` (the kernel stays live
under B); D's closure work targets exactly the audit's §6.1/§6.5 at-risk items.

---

## 5. Adoption blueprint

**Where it lives (dual home, one canonical):**
- **Canonical:** `docs/operating-model/PROGRAM-SPINE.md` — alongside the other governing rules
  (`verified-by-math.md`, `task-exit-rule.md`, `model-agnostic-playbook.md`). Committed, so every
  branch and every agent sees it; protected the same way those are.
- **Memory corpus:** a new entry `program-spine-arbiter-2026-07-11.md` + an index line at the
  **very top of MEMORY.md** (above the todo map — it outranks it), pointing at the canonical file
  and stating the current ranking in one line. HERMES mirror picks it up via the standing
  `sync-memory-to-hermes.mjs` rule.
- **bebop-repo:** one pointer line in its AGENTS.md ("program priority is governed by
  dowiz/docs/operating-model/PROGRAM-SPINE.md; this repo is PARKED unless that doc says
  otherwise") — added during D's capture session.

**Enforcement — the WIP-limit mechanism (design only, no code in this blueprint):**

A new plane-guard pattern **P12 "program-spine"**, following the exact shape of P1–P11
(`scripts/plane-guard.mjs` `rec()` registry, hard/soft severity, runs in `verify:all --ci` and
pre-commit). Four deterministic checks, each with a RED case per VbM:

1. **Parked-path guard (hard):** parse the machine block; if any staged/committed path matches a
   PARKED program's globs, go RED — unless a human-only override line exists in
   `.claude/state/spine-override` (`program|expiry-epoch`, same mechanics as `fable-override`,
   already guard-bash-protected as a class: agents cannot write their own bypass). RED proof:
   stage a file under `rebuild/web/`, run the guard, observe RED; add override, observe GREEN.
2. **WIP limit (hard):** `active` array length > 1 → RED. RED proof: add a second entry.
3. **Stale-arbiter gate (hard):** `review` date in the past → RED with the message "re-sign
   PROGRAM-SPINE.md". This converts governance rot from silent to loud — the exact failure mode
   of every predecessor artifact (todo map outdated in 24h, META-CONTROLLER.md stale). RED proof:
   set review to yesterday.
4. **Signature presence (soft):** `SIGNED:` line non-blank and machine block parseable; soft
   because the doc's first commit predates the signature.

Additionally fold audit rec 6 into the same check: a deny-glob for bebop-topic files
(`*bebop*`, the crypto-research filename set) landing in `/root/dowiz` — the prose rule has
failed twice; this makes it a gate. Protect `PROGRAM-SPINE.md` itself via the existing
protect-paths hook list so edits are deliberate.

**Why this fits the repo's philosophy:** it is a deterministic gate, not an advisory proxy
(§0·GP-compatible — it would have survived the 07-07 purge); it is falsifiable with a shipped RED
case (VbM); it reuses the human-only override pattern the operator already trusts; and it costs
one file read per run.

**Keeping it current (procedure):**
- Any status change (park/unpark/open/retire) = edit the doc + machine block, bump the date,
  re-sign, update the MEMORY.md one-liner. One edit, three lines.
- The review date is the cadence: every 2 weeks, or immediately on P0 success (first real paid
  order — which should trigger a full re-rank, since validation flips several scorecards).
- Session-start rule for leads: read the MEMORY.md spine line before choosing work; a session
  whose plan touches a PARKED program without an override is misrouted by definition.
- The P12 guard is the backstop, not the process: the doc stays current because a stale doc
  turns CI red, not because anyone remembers.

**Adoption order (all operator-gated where marked):**
1. Operator edits/signs the §4 draft → commit `docs/operating-model/PROGRAM-SPINE.md` (operator).
2. Write the memory entry + MEMORY.md index line; re-run the HERMES sync.
3. Build P12 + `.claude/state/spine-override` wiring + RED proofs + REGRESSION-LEDGER row
   (normal gated dev work; red-line adjacent since it touches hook territory).
4. Run D's capture session and A's closure session (the two "next session does X" items that
   stop active bleeding), in that order — D's uncommitted crypto is the larger exposure.
5. Add the bebop-repo pointer line during the capture session.

---

## 6. Operator decision points

| # | Decision | Options | This blueprint's recommendation |
|---|---|---|---|
| 1 | Accept P0 (business validation) as dominating? | yes / no — if no, pick Option 2/3/4 from §3 explicitly and sign that instead | **Yes** — both strategy docs and the audit converge; no evidence rebuts it |
| 2 | Which future is the ONE active engineering track? | B / A / D / none | **B**, throttled to validation-serving + exit-gate work |
| 3 | A's fate at first review | keep parked / resume / formally retire (Phase-D never happens; Node stack is the product) | Keep parked; decide resume-vs-retire only after venue data exists — retiring now discards a proven mechanism for no information gain |
| 4 | Approve D's capture session (1 session, commits+pushes crypto, bootstraps memory) | yes / no | **Yes, immediately** — the uncommitted PQ work is the single largest unprotected WIP in either repo |
| 5 | Commit an actual ADR-020 file during C's parked prep? | yes / no | Yes — a FINAL goal without a committed charter is how this gap happened |
| 6 | Review cadence | 2 weeks (drafted) / weekly / event-driven only | 2 weeks + event trigger on first real paid order |
| 7 | Build the P12 plane-guard? | yes / prose-only doc | **Yes** — every prose-only predecessor (todo map, standing detritus rule) failed within days; this repo's only durable rules are the gated ones |
| 8 | The stall clause (P0 makes no outreach progress by review date) | mandatory re-sign with alternative / drop the clause | Keep it — it is the honesty mechanism that distinguishes this doc from the four it replaces |

---

*Blueprint authored 2026-07-11 by a read-only research session. The only file created is this
document. Nothing in /root/dowiz or /root/bebop-repo was modified; no ranking is in force until
the §4 draft is signed.*
