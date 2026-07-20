# dowiz — Strategic Regret-Minimization Synthesis (2026-07-20)

> **Status: RESEARCH SYNTHESIS, not a blueprint.** No code, no DoD, no build items. Produced from
> ten independent, file:line-verified Opus research passes (listed in §0) feeding one Fable
> synthesis pass, per operator instruction ("opus for research, fable for synthesis"). Answers the
> operator's question verbatim: *"ask yourself what would you do right now to not regret about the
> architecture decisions in 1-2 years, also what was missed, lost from all the prompts, researches,
> plans, what is missing for the actual dowiz launch... find the unwired gaps, blind spots — search
> & think omnidirectionally & with crossections."*

## 0. Source reports (all file:line-cited against the live tree, 2026-07-20)

| # | Scope |
|---|---|
| A | Unwired-capability sweep (kernel/engine/tools) |
| B | Space-grade roadmap real-vs-blueprint ratio + integration-debt risk |
| C | DeliveryOS product launch-readiness |
| D | Cross-session memory archaeology (dropped threads) |
| E | Fresh critique of this session's own `LIVING-MEMORY-WAVE-PROPAGATION-FINISHING-LAYER-SYNTHESIS-2026-07-20.md` |
| F | Monitoring / continuous runtime verification |
| G | Rollback / versioning / packaging / backup / cross-device / cross-environment |
| H | Configurability / maintenance / delivery / support / marketing |
| I | Scalability / extensibility / quality standards / documentation |
| J | Dynamic CPU/GPU/memory resource allocation |

---

## 1. The pattern: verification stops at the moment of authorship

Read together, all ten reports describe the same broken hop, each at a different layer. dowiz has
built an exceptional *write-time* truth pipeline — Kani proofs, byte-exact KATs, statistical bench
A/B gates, type-system firewalls, re-executing CI — and almost nothing that carries a truth forward
after the moment it is authored. The instance per report:

- **A**: code written and tested, never wired into a running path — Hydra (50KB, zero callers),
  MemoryStore, PPR, messenger, blocklist, spine, the eqc proof-half, absorbing-ETA.
- **B**: code written and *claimed green in commit messages*, never pushed, never CI'd, never
  merged — ~18 items, 21 local branches, ~15,000 lines.
- **C**: kernel truths accumulate (35 of the last 100 commits) while the product that would
  actually *run* them sits at ~0% shippable.
- **D**: decisions and claims written into memory, never closed or re-verified — the money-leg
  parked half-open, recall@5 silently regressing from a 29-query to a 12-query oracle on port.
- **E**: rulings written into a design doc, never reconciled backward — a phantom §8 ruling cited
  twice that was never made, a safety-critical safeguard list quoted three inconsistent ways,
  whole sections still assuming a design a later ruling abandoned.
- **F**: safety proven once in CI, never re-proven in production; the single live gate in the
  codebase guards a system (Hydra) that never runs.
- **G**: rollback exists for binaries but nothing preserves data truths — no wire-format
  versioning, no mesh backup, staging docs pointing at a retired host.
- **H**: configuration frozen at compile time; CD deliberately absent, so nothing crosses into
  "running" without manual ceremony.
- **I**: docs written for the authoring session — CONTRIBUTING.md still instructs `pnpm install` a
  month after the JS stack was deleted.
- **J**: sensors measure real load, actuators exist, and no loop connects them.

The unifying law: **effort is inversely proportional to distance from authorship.** The house
culture "verified, not claimed" is genuinely honored — but only at birth. Every claim silently
degrades from "this is true" to "this was true once, in isolation, when written." Note also that
this is the *second* occurrence of the pattern: a 2026-07-19 wave already wired seven built-but-
unwired surfaces (A), and this audit found nine more plus 18 stranded branches. That is not a list
of incidents; it is a generator. The definition-of-done in this repo is effectively "written +
green in isolation," and until it becomes "wired + landed + observed running," every fix wave will
be followed by another. The 1–2 year regret, absent a change, is precise: a repository containing
three diverging systems — the proven kernel, a shadow codebase of unlanded capability roughly as
large, and a knowledge layer matching neither — with the gaps growing at the rate of authorship.

---

## 2. What to do now to not regret it in 1–2 years — ranked top 8

**1. Land the stranded work.** Push all 21 branches to origin *today* (pure safety, zero merge
risk, and this repo has already lost untracked work once to exactly this exposure — B, memory
record), then serially merge the ~18 items through full CI, item 9 (breaker) first since it is the
roadmap's own declared unblock for items 11/12/21/27/32. Resolves B directly. **Cost: cheap to
push, moderate to merge. Leverage: highest in the entire audit** — the largest asset recovery per
unit effort, and it is actively decaying: every day of merge-base drift makes 15,000 lines harder
to land.

**2. Decide the durability spine as one design, not three retrofits.** Persistence, wire-format
versioning, and (reserved) mesh replication are one architectural decision: the persisted
order-event stream *is* the replication unit *is* the versioned artifact. Wire MemoryStore/PgStore
as the order path's durable sink (A finding 1, C's `Mutex<HashMap>`), and before the first real
byte is persisted, stamp a version envelope on the stream — G shows versioning exists only at the
crypto-suite layer, and a decentralized mesh that cannot force synchronized upgrades has no
node-skew story at all. Deliberately de-prioritized against G's own sharpest urgency: *building*
cross-mesh backup can wait (zero users, zero data), but the *format decision* cannot — retrofitting
versioning after real orders exist is the canonical 2-year regret, near-free now, near-impossible
later. **Cost: cheap (envelope) + moderate (wiring). Leverage: this is the ossification point.**

**3. Rebalance the commit budget to the product lane.** Declare the space-grade arc "sufficient
for launch," freeze new hardening blueprints (items 70–78 are self-declared aspirational — B), and
redirect the 35:6 kernel-vs-product ratio (C) into the W1/W2 wave. Fold the operator's marketing
correction in here: the first product surface ships with **native telemetry from day one** —
attribution and analytics built on the existing FDR/span-metrics stack (F, H), zero external SDKs
— rather than being retrofitted after launch, which is when every product caves and bolts on a
third-party tracker. **Cost: cheap (a decision). Leverage: trajectory-level** — the only item on
this list that changes *where the repo is* in two years rather than how safe it is.

**4. Build runtime re-verification before wiring any live safety organ.** Make FDR readable at
runtime (an Alarm record something actually reacts to — F), make the drift-gate self-test that it
still fires, and close J's loop: wire the existing load sensor into the existing admission gate,
add the one memory-budget env var feeding the existing arena constructor (J says this reuses only
tested primitives). Then — and only then — decide Hydra: wire it into a boot path or consciously
archive it (A finding 6). The sequencing matters more than the speed; see blind spot (i). **Cost:
moderate. Leverage:** this is the mechanism that ends the "proven once, trusted forever" failure
mode F names, and every future safety claim — including the living-memory doc's "physically
bounded" principle — has no enforcement substrate without it.

**5. Clear the operator decision queue explicitly.** The four launch decisions (C), the parked
money-leg settlement scope (D), Wasmtime fuel (D), the gated 53× event-log batching (I). For each:
decide, or defer with a date. Half-open red-line decisions are precisely how scope drifts past a
red line without anyone ever deciding — and D shows they have already sat 10–18 days. Everything
in section 3 queues behind these. **Cost: cheap in hours — but it is operator time, the binding
resource (see blind spot vi).**

**6. Reconcile the living-memory doc before any code flows from it.** Remove the phantom §8 ruling
citations, canonicalize the safeguard list to one enumeration, update every section still assuming
the pre-pivot external-L1 design (the safety argument currently rests on a process boundary the
design no longer has), verify the concurrent-action redesign against the doc's own P4 and L4
criteria, and either split the neurograph's ten responsibilities or explicitly rule the accretion
acceptable (E). **Cost: cheap — it's editing. Leverage: prevents the expensive class of building on
sand;** E's defects are exactly the kind that become load-bearing code.

**7. Repair the front door.** Fix CONTRIBUTING.md (actively broken — I), write a live-node
operator runbook and one "how retrieval actually works" reference (I), delete the two
confirmed-dead staging memory files (G), fix the memory-index self-contradiction (D). **Cost:
cheap. Leverage: the bus-factor hedge** — the only item that compounds *other people's* (or future
sessions') effort, and everything above eventually needs a second pair of hands.

**8. Move business config out of compile time.** Payment provider, currency, and the
trusted-anchor roster as deployment configuration, plus a real enrollment path so a production
server does not boot rejecting every request (H; C's empty-roster finding is at heart a
config-tooling problem). C needs this for launch anyway; pre-launch it is a seam, post-launch it
is a migration. **Cost: cheap–moderate.**

**Explicit de-prioritizations against the report authors:** PPR wiring (A's #2) — internal
retrieval quality, neither launch nor safety; the cross-mesh backup *build* (G's sharpest finding)
— deferred behind format reservation; clippy/coverage (I) — fold into the landing wave's hygiene;
and any new blueprint authorship — the audit's clearest negative-value activity right now (B).

---

## 3. What's missing for launch — the anatomy of one real order

For one real order to exist, five things must happen; each is currently blocked (C is the spine
here; cross-references noted).

**(0) The four unmade operator decisions (C) gate everything below.** They cost hours; they have
cost weeks.

**(a) A customer reaches an ordering surface.** Storefront: 0% code since the JS drop (C). One
shortcut deserves serious evaluation: `messenger.rs` (A finding 3) already builds Telegram
deep-links, and Telegram-primary is standing product strategy — a Telegram-first ordering channel
may be *weeks* where a web storefront is *months*. This is the one place in the audit where an
unwired asset directly shortens the launch path rather than being deferred debt.

**(b) The order is admitted.** The production server boots with an empty capability roster and
rejects everything by design (C). Needs enrollment tooling + runtime config (H; item 8 above).

**(c) The order survives.** Persistence is a `Mutex<HashMap>`; a restart erases every order (C, A
finding 1). The durability spine (item 2). Non-negotiable — you cannot take money on volatile
memory.

**(d) The order is paid.** `NoOpPaymentAdapter`; the real adapter crate does not exist (C). The
seam is proven and firewalled (I), so this is bounded work — or decide cash-on-delivery for the
pilot, which is exactly the kind of question sitting in the decision queue.

**(e) The order is delivered.** The courier crate has no `[[bin]]`; nothing is installable on a
phone (C, G).

**Launch-day minimums that are NOT separable:** some staging tier (G — none exists at all;
shipping the W1/W2 wave with zero staging repeats the exact incident class the no-auto-deploy rule
memorializes — H); a runtime alarm that *reads* FDR (F — the Telegram dead-man heartbeat answers
"is it up," not "is it correct"); native telemetry in surface (a) per the operator's correction.

**Separable, format-reserved now:** full cross-mesh backup (G — a single-node pilot with an
off-node encrypted snapshot is an acceptable interim); PPR, spine, absorbing-ETA (A); Hydra
(behind item 4's sequencing).

C's honest distance — "months, not a wiring sprint" — stands. The ordering above is the
difference between months spent converging and months spent orbiting.

---

## 4. Blind spots — gaps between the gaps

**(i) The Hydra sequencing trap (A × F).** Report A implies "wire Hydra"; report F shows nothing
verifies any live gate keeps firing, and Hydra's is bypassable ("on intervention ALL safeties
lift") and never self-tests. Wiring Hydra *before* the F-loop exists converts dead code into false
confidence — the operator would believe a self-defense organ is on duty when nothing confirms it
fires. That is strictly worse than leaving it unwired. No single report states the ordering
constraint: **F's loop is a prerequisite for A's fix, not a sibling.**

**(ii) "Physically bounded" has no physics (E × J).** The living-memory doc's governing principle
— emergence must be transparent, safeguarded, kill-switched, *physically bounded* — requires
exactly the closed resource loop J proves does not exist. Worse, the neurograph, as a
ten-responsibility concentration of memory- and CPU-heavy work (vector index + Hebbian learning +
CRDT replication + self-spawning host in one process), is the first component the aggregate-load
blind spot would take down — and the self-spawning responsibility under unthrottled load is the
textbook runaway scenario. The principle is currently a sentence, not a mechanism.

**(iii) The "over-built" CI is right-sized after all (H × B).** H flags 20+ CI jobs as excessive
for the product's stage. But the landing wave of ~15,000 claimed-green-never-verified lines is
precisely what that gate battery is sized for — bench-regression A/B, v5c re-execution,
unconditional kernel+engine tests are the exact insurance the B risk needs. Combined with G (no
data-level rollback) and F (no runtime detection), the rule falls out: **serialize the 18 merges
through the full gate battery; do not bulk-merge; do not trim CI until the wave has landed.** A
rushed bulk landing is the compounding risk: a regression in one of 18 items would land with no
live detection and no data-level undo.

**(iv) The anti-drift tool has itself drifted (A × D × E × I).** `spine.rs` — the KnowledgeSpine
hash-chain ledger, whose entire purpose is tamper-evident knowledge custody — has zero callers (A
finding 7). Meanwhile the documented live diseases of this repo are all knowledge drift: a memory
index contradicting itself (D), a quality claim silently regressing on port (D), a design doc
citing a ruling never made (E), contributor docs describing a deleted stack (I), a Repowise index
a month stale. The failure the unwired tool exists to catch is the failure actually occurring.
When the spine does get wired, its highest-value first tenant may be the memory/design corpus,
not kernel events.

**(v) D0 is enforced per-crate and honored nowhere per-deployment (C × F × G × H).** The
invariant machinery — no-scoring CI job, `Ord` omission, deny-by-default red-lines — all operates
at crate granularity, brilliantly. At *deployment* granularity, today's reality contradicts D0
outright: "decentralized · local-first · reliability-over-latency" running as one disk plus one
vendor-controlled key (G), admitting zero orders (C), unwatched at runtime (F). No gate anywhere
asks "does the deployed system as a whole still satisfy D0?" The invariant-enforcement pattern
this repo mastered has never been lifted one level up.

**(vi) Operator attention is the binding constraint, and nothing economizes it.** All governance
is suspended in favor of operator directives; decisions sit parked 10–18 days (D); four launch
decisions queue (C); red-line-adjacent issues idle (D). The system's throughput ceiling is now one
person's decision bandwidth, and half the findings across all ten reports are queued behind it.
None of the reports names this. Item 5's explicit decision queue is the cheapest mitigation;
long-term, the fix is fewer standing half-open decisions, not more agent throughput.

---

## 5. If only three things

**First, push and land the stranded branches (item 1).** It is the cheapest large recovery in the
audit, the exposure is actively decaying and has already caused one real data loss, and the
roadmap's own declared pivot (item 9) is sitting in it. Nothing else pays this much for this
little.

**Second, decide the durability spine (item 2)** — persistence wired, format versioned,
replication reserved, as one design. It is the single decision that ossifies against you daily,
and every launch path in section 3 crosses it.

**Third, redirect the budget to the product (items 3 + 5 together):** decide the four decisions,
evaluate the Telegram-first channel, and ship the first surface with native telemetry from day
one. This is the only move that changes the two-year trajectory rather than the risk profile — the
kernel is already good enough to deserve a product.

Runtime re-verification (item 4) is deliberately fourth, not third: it matters most once something
runs in production, and after these three, something finally will. Just respect the one hard
ordering constraint this synthesis adds to the record: **the monitoring loop comes before Hydra
goes live** — a safety organ nobody watches is more dangerous than no safety organ at all.
