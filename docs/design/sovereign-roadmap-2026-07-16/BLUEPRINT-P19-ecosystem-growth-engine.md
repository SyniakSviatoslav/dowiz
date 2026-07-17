# BLUEPRINT — Phase 19: ECOSYSTEM GROWTH ENGINE

> Final phase of the 19-phase master roadmap (`R2-MERGED-PHASE-ROADMAP.md`). This document plans and
> drafts; it **submits nothing and publishes nothing**. Draft copy INSIDE this document is intended
> and expected; shipping it to a live site / grant portal is a separate, gated action owned by Phase 18
> (public flip) and the operator — not executed here. Canon: `ARCHITECTURE.md` (E6, E7, E56, E57, E60;
> V6 dual-track + metaphor discipline). Primary evidence: `R1-E-…-gap-analysis.md` §E6/E7/E56/E57/E60,
> §0.1, §1(V6), and its already-completed NLnet web-research (reused verbatim, NOT re-researched).
>
> **Anchors owned:** E6, E7, E56, E57, E60 (+ V6's G11 recording obligation).
> **Depends on:** **Phase 18** (HARD — the public repo IS the surface this phase markets and applies
> against; growth work before the flip has nothing real to point at). **Soft:** Phase 14 (a working
> per-hub wiki demo strengthens the grant dossier's technical credibility but does not block the writing).
> **Parallel-safe with:** Phase 17. **SCOPE RULE:** positioning/pricing/spec here describe the
> canonical brand + operator's own offering; a sovereign hub (M5/M9/M11) may fork the code, drop the
> brand, and price/monetize however it likes. Nothing here is a control over other hubs.

---

## 1. Current-state evidence

**E6 — positioning: DOCS-PARTIAL.** The only honest kernel of the pitch that exists is `MANIFESTO.md`
§0 (verified): *"A self-hosted, decentralized network of autonomous nodes … running a NON-AI,
post-quantum-secure delivery protocol where reliability > latency and decentralization is a hard
invariant, not a feature."* That is a constraints charter, not marketing. **No positioning or landing
document exists for the mesh-foundation-pivot era** (the 2026-07-16 reframe where the mesh/PQ protocol
is the foundation and delivery is a service on top — `ARCHITECTURE.md:4`, M1).

**Stale-pointer finding (small but real).** Project memory's *"Brand & design"* index line points at
`docs/design/dowiz-brand/BRAND-BIBLE.md` (*"Dowiz brand = Warm Cosmo-Noir … docs/design/dowiz-brand/
BRAND-BIBLE.md"*). **That file does not exist.** Verified: `ls docs/design/dowiz-brand/` →
*"No such file or directory"*; `find . -iname "*brand-bible*"` → zero hits. The Warm Cosmo-Noir brand
voice survives only as a memory topic note, never as an in-tree artifact — and even that note predates
the mesh pivot, so it positions *dowiz-the-delivery-brand*, not *the-protocol-as-headliner*. This
phase supersedes it: it writes the positioning in-tree and the memory pointer must be corrected to the
real path (acceptance §7.5). This is a live instance of the BRAIN-TOPOLOGY self-certification pattern
(a claimed artifact whose existence was never checked).

**E7 / E56 — B2B + grants: NOT BUILT (artifacts).** No grant dossier exists. No in-tree pricing doc
survives the pivot — the pre-pivot **ADR-020 pricing-v2** (hosted-cloud as the *only* sold path,
`Free ≤50 orders/$0 … Business ∞/$59`) lives **only in memory**, and its framing is now
architecturally wrong (see §4). **NLnet facts (reused from R1-E §E7 — already web-verified this
session, do NOT re-research):** NGI Zero Commons Fund — **first-time proposals ≤ €50,000** (subsequent
≤ €150k, lifetime cap €500k), 1–12-month projects, outputs **MUST be a recognised FOSS license**
(**AGPLv3 qualifies — already satisfied** by the `ac1caba40` license flip), R&D focus + a clear
**European dimension** (EU/UA delivery protocol fits E60), concise English application via
`nlnet.nl/propose`. **The most recent call (2026-06Z, €6.1M) closed — deadline 2026-06-01 12:00
Brussels has PASSED.** Calls recur; **the target is the NEXT open call, not an immediate submission.**

**E57 — usage-based PQ-api: NOT BUILT, deliberately spec-only.** No billing/usage surface exists in
the Rust stack (the legacy TS billing was deleted with the thin-layer). R1-E correctly flags that
building billing infrastructure now — before a single real paying inquiry exists — would violate the
project's own ponytail/lazy-dev discipline and MANIFESTO C8 (*"over-engineering is the #1 enemy …
YAGNI"*). **This phase respects that: it writes the spec and names the build-trigger; it schedules zero
implementation** (§5).

**E60 — EU/UA: positioning only.** No standalone artifact needed beyond the grant dossier's
European-dimension section (§3).

**Dependency reality.** All of the above is gated on **Phase 18** landing first: a grant reviewer, a
B2B prospect, and a landing visitor all need a *public repository* to look at. The soft Phase-14
dependency (per-hub wiki demo) only sweetens the dossier's technical-credibility section; it does not
block drafting, which can start immediately.

---

## 2. Positioning / landing-copy design (draft copy, per V6 dual-track)

**Core narrative (V6, `STRATEGIC-VECTORS:57`): the bebop PQ protocol is the HEADLINER; delivery-as-a-
service is the BY-PRODUCT / proof-of-concept riding on top.** Below is actual draft copy to live in the
positioning doc (and, at Phase 18/E5's discretion, seed a landing page). Metaphor discipline (V6 /
Fable Pattern 6) is honored: no bare "organism/emergent/swarm" — every such word sits next to a named
computed criterion or is replaced by "designed coordination."

> ### Hero
> **bebop — a post-quantum protocol for networks that must not fail.**
> Self-certifying nodes. Zero central server. Delivery is just the first thing we built on it.
>
> ### One sentence
> bebop is a decentralized, post-quantum-secure protocol where every node owns its own database and
> signs its own frames — ML-DSA-65 for signatures, ML-KEM-768 for key exchange, both FIPS 204/203 with
> bit-exact known-answer tests in the repo — and **dowiz** is a real delivery service that runs entirely
> on top of it, with no privileged coordinator anywhere in the loop.
>
> ### Three pillars
> **1 — The protocol is the product.** bebop is a wire format and a trust model, not an app. Every edge
> is autonomous: it holds its own ML-DSA identity, signs its own frames, and answers to no central CA.
> Any node can drop and the network routes around it by recomputed shortest path — designed
> coordination, not a leader election.
> **2 — Sovereignty is a hard invariant, not a tier.** There is no server we run that you depend on.
> The protocol carries zero external dependencies at the trust boundary. You self-host, or you don't
> run it. AGPLv3 guarantees the protocol can never be captured — fork it, drop our brand, keep the wire.
> **3 — Delivery is the proof.** dowiz is a working delivery network — orders, proof-of-delivery signed
> by the courier's own key, integer-exact settlement — that demonstrates the protocol under real load.
> It is deliberately a *by-product*: if delivery is boring and reliable on bebop, the protocol is doing
> its job.
>
> ### Why this exists
> Most "decentralized" systems re-centralize the moment money or matching is involved. bebop makes the
> economic control points (matcher, settlement) open and replicable by construction: any node can run
> the matcher; settlement is a threshold verifier (≥k-of-n), never a single oracle. Post-quantum is
> treated as a *protocol* — KEM, signatures, at-rest, code-signing, in-transit integrity, composed —
> not a checkbox primitive. Built in the EU/Ukraine, for infrastructure that has to keep working when
> the network doesn't.
>
> ### For builders
> AGPLv3. Every claim above is falsifiable against the repo: FIPS 204/203 KATs, deterministic
> event-sourced state, integer-only money, in-repo proofs. No AI in the protocol path — deterministic
> Rust/WASM only.

The positioning doc frames dowiz strictly as evidence-for-the-protocol, keeping the headliner/by-product
inversion that V6 mandates. **This copy is a draft inside this blueprint; publishing it is Phase 18/E5's
job, not this phase's to execute.**

---

## 3. NLnet NGI Zero dossier plan (eligibility · narrative · budget · timeline)

**Eligibility confirmation (all conditions from R1-E, checked):**
- **FOSS license mandate — SATISFIED.** Repo is AGPLv3 since `ac1caba40` (`/root/dowiz/LICENSE`, full
  AGPLv3, on `origin/main`). No further license work required for eligibility.
- **First-time applicant ceiling ≤ €50,000 — the budget must fit under this.**
- **R&D focus — genuine.** Post-quantum mesh transport, self-healing routing, partition-tolerant
  settlement are real research-and-development, not product polish.
- **European dimension — fits E60.** EU/Ukraine delivery-sovereignty angle; local-first + no
  surveillance (M8) aligns with NGI's privacy/resilience mandate.
- **Format** — concise English via `nlnet.nl/propose`.

**Narrative shape (the dossier sections to write):**
1. **What** — one-paragraph description mirroring §2's one-sentence pitch: a zero-dependency, post-
   quantum, decentralized delivery protocol; delivery as the demonstrator.
2. **Why it matters to NGI/Europe** — resilience (no SPOF, works partitioned), privacy (local-only
   telemetry, no courier scoring/surveillance — M8/E58), post-quantum readiness ahead of harvest-now-
   decrypt-later, EU/UA sovereignty (E60).
3. **Technical merit / credibility** — cite the concrete built substrate: ML-DSA-65 + ML-KEM-768 with
   in-repo FIPS 204/203 KATs, self-certifying node identity (ADR-0007), local PQ-at-rest persistence
   (ADR-0008 — see CORRECTED note), the eqc VERIFIED-BY-MATH proof discipline. **If Phase 14 has landed, add the two-hub
   per-hub-wiki delta-exchange demo** as evidence of working replication (this is the soft dependency's
   payoff).
   > ⚠ CORRECTED (operator, 2026-07-16): this dossier previously cited "local SQLite PQ-at-rest" as built substrate.
   > dowiz does NOT use SQLite as an architectural choice — the spectral/sqlless approach (content-addressed
   > `BlockStore` + JSONL `FileEventStore`) is the MAIN storage/retrieval path in dowiz's own kernel/engine, with
   > **pgrust as the uniform SQL-fallback/backup target, not SQLite** (ADR-0008 is being updated SQLite→pgrust). Cite
   > the PQ-at-rest substrate as the spectral/sqlless content-addressed store + pgrust, never SQLite.
4. **Work plan / deliverables** — map work-packages to roadmap phases that are genuinely R&D and not
   yet done at flip time (candidates: P3 PQ trust-root hardening, P9 confidential self-healing wire,
   P14 dispute/escrow + per-hub graph-wiki). Each WP = a falsifiable done-test lifted from the roadmap.
5. **Budget ≤ €50k** — person-months against the WPs above, itemized; explicitly under the first-time
   ceiling. No hosting/GPU capex that would contradict the self-host/scale-to-zero posture.
6. **Team / European dimension** — sole maintainer, EU/UA base, AGPLv3 commons intent (MANIFESTO C6/C9).

**Timeline against the NEXT call.** The 2026-06Z window is closed. NGI Zero Commons calls recur (per
R1-E's cited guide); **target the first open call whose deadline falls AFTER Phase 18's public flip** —
a reviewer cannot evaluate a private repo. Sequence: P18 flip → finalize dossier (repo URL, live
Discussions, README) → submit at that call. **Do not force a submission against a window that closes
before the repo is public.** The falsifiable outcome explicitly accepts a *reasoned decision not to
apply to a given call* (§7.1) — if the flip slips past a window, the correct output is a dated,
written "not this call, here's why, here's the target" note, not a rushed submission or silence.

---

## 4. Pricing / B2B refresh plan (mesh-pivot, not pre-pivot centralized framing)

**The pre-pivot pricing is architecturally wrong now.** ADR-020 pricing-v2 (memory-only) sold
**hosted-cloud as the *only* path** (`Free/Pro/Business` tiers on our servers, `Business ∞/$59`). That
model **directly contradicts** M1 (mesh = foundation), M5 (every hub sovereign), M6 (zero protocol
deps), and MANIFESTO C4/C13 (*no central server*). "We host, you rent, we're the dependency" is exactly
the re-centralization the pivot rejects. The refreshed doc must delete that framing.

**Refreshed pricing shape (mesh-consistent), written in-tree:**
- **The protocol is free, forever — AGPLv3 commons.** Self-hosting is always available and fully
  functional; it is never the crippled tier. This is the load-bearing inversion from the old model.
- **Monetize services ON TOP of the protocol, never the protocol itself:**
  - **Optional managed hosting** for operators who don't want to run their own node — sold as
    *convenience, not lock-in* (self-host remains a first-class equal; AGPLv3 guarantees exit). Modal
    scale-to-zero economics (H100 $0.001097/s) keep this honest — pay for compute used, no idle rent.
  - **B2B support / SLA / integration contracts** — the durable revenue: onboarding, custom bridges,
    priority fixes, deployment help. Sold against expertise, not gatekeeping.
  - **Usage-based PQ-api** (E57) — spec-only until the demand trigger fires (§5); referenced here, not
    priced-to-build.
- **Explicit non-goals** (stated in the doc so the pivot is unambiguous): no feature paywall on the
  protocol; no courier scoring/reputation as a sold product (E58 / NO-COURIER-SCORING); no data resale
  (M8 no-surveillance).

Deliverable: `docs/design/PRICING-B2B-MESH-2026.md` (or equivalent in-tree path), replacing the
memory-only pricing-v2, with a one-line note that it supersedes the pre-pivot ADR-020 pricing.

---

## 5. E57 usage-based PQ-api — SPEC-ONLY design + explicit build-trigger

**Status: SPECIFICATION ONLY. Zero implementation is scheduled in this phase.** This section defines
the shape so that *when* demand is real the build is a small, known step — not so that it is built now.

**Concept.** A hosted endpoint exposing the repo's zero-dep ML-DSA-65 / ML-KEM-768 primitives to
callers who want post-quantum crypto without self-hosting a node — the paid convenience layer over the
free protocol.

**Endpoint shape (design sketch, not to be implemented here):**
- `POST /v1/pq/sign` — body → ML-DSA-65 signature (caller-supplied key handle).
- `POST /v1/pq/verify` — {msg, sig, pubkey} → bool.
- `POST /v1/pq/kem/encaps` / `POST /v1/pq/kem/decaps` — ML-KEM-768 encapsulate / decapsulate.
- `GET  /v1/pq/usage` — the caller's metered counters (self-service, no surveillance).

**Pricing-model shape.** Usage-based: a free monthly quota, then pay-as-you-go per 1k operations, with
a per-account budget ceiling (reuses the TokenBucket/Budget posture from E19/F6). No seat licenses, no
minimums — metered convenience over a commons primitive.

**Metering approach.** Count operations locally as **typed metrics** consistent with M8 (per-process,
typed, local sink; never exfiltrated as surveillance); billing reads the local counter. No request
bodies retained; no PII; deterministic per-op accounting.

**BUILD-TRIGGER (named in writing, grep-able):**
> **BUILD-TRIGGER (E57):** the first qualified **paying B2B inquiry** — a real prospect who has asked
> to pay for hosted PQ operations. Until that fires, E57 stays spec-only. Building billing before a
> paying inquiry exists violates MANIFESTO C8 / ponytail (YAGNI). When it fires, E57 converts to an
> implementation phase (endpoints + metering + budget rails), not before.

This phase does **not** schedule that implementation work. The trigger is the falsifiable gate that
converts spec → build.

---

## 6. G11 recording mechanism (V6 stable-track criterion)

**G11 = "first real order"** is the stable-track closure criterion (`STRATEGIC-VECTORS:57`: *"stable =
delivery (G11 first real order)"*). It becomes *achievable* only once **Phase 13** (delivery on
protocol) and **Phase 16** (product UI rebuild) land — a real customer order must be placeable and must
verifiably fold across a mesh hub. This phase does not *cause* G11; it defines *how G11 gets recorded in
canon the moment it is true*, so the milestone is captured with evidence instead of self-certified.

**Recording template (to be entered into canon when true):**
- **Claim:** G11 — first real order.
- **Date:** the order's event-log timestamp (not the claim date).
- **Evidence:** order id + hub id + the signed event-log fold proving `Pending → … → Delivered` on a
  real (non-fixture) order — the Phase 13 done-test artifact, not a screenshot.
- **Named owner (verifier):** the **product/delivery-track owner** is accountable for verifying G11 is
  genuinely real before it is written (per R1-E: mechanics owned by the product cluster; *recording it
  as THE criterion* is this growth cluster's job). Concretely name the verifier in canon — the operator
  (`SyniakSviatoslav`) or a designated product-track lead — so the milestone has an accountable human,
  not an anonymous "GREEN."

**Guardrail:** G11 **cannot be pre-declared**. It is recorded only after Phases 13/16 land AND a real
order verifiably folds. Until then the canon carries the *template + named owner* (defined by this
phase), and the actual G-entry is appended when the evidence exists. This is the anti-self-certification
discipline applied to the project's own headline success metric.

---

## 7. Acceptance criteria (numbered checklist — all falsifiable)

1. **Grant decision is real, not silent.** EITHER a grant-application **submission receipt** exists
   (an artifact — confirmation email / portal reference number, recorded in-tree), OR a **dated,
   explicitly reasoned written decision NOT to apply** to the current/next call exists (target-call
   named, reason stated). Silence fails this test; both non-silent outcomes pass.
2. **Positioning doc merged in-tree** with the actual §2 draft copy present — the headliner/by-product
   inversion ("bebop PQ protocol = headliner, delivery = by-product") is written as concrete copy, not
   described as a to-do.
3. **Pricing/B2B doc refreshed and in-tree**, reflecting the **current mesh-pivot** architecture:
   `grep` finds the AGPLv3-commons + self-host-always-available framing and finds **no** "hosted-cloud
   as the only sold path" language; the doc explicitly supersedes pre-pivot pricing-v2.
4. **E57 exists as a spec document ONLY.** Its **build-trigger is named in writing and grep-able**
   (`grep -rn "BUILD-TRIGGER" docs/` hits the E57 line); AND there is **zero** implementation —
   `grep` finds no billing/usage/PQ-api endpoint code in the Rust stack.
5. **Stale brand-bible pointer resolved.** The finding is recorded (this doc §1), and the memory
   "Brand & design" pointer is corrected to the real in-tree positioning path (no live pointer to the
   non-existent `docs/design/dowiz-brand/BRAND-BIBLE.md`).
6. **G11 recording mechanism defined**, with the template + **named verifier owner** in place; the
   canon G11 entry is appended **once Phases 13/16 land a real order** (verifiable order evidence, not
   before).
7. **Metaphor discipline honored** (V6 / Fable Pattern 6): no bare "organism/emergent/swarm" in any
   published-intent copy without an adjacent named computed criterion, or reworded to "designed
   coordination."
8. **Nothing published, nothing submitted by this phase's execution.** Draft copy stays inside the
   in-tree docs; the actual site publish and grant submission are separate gated actions (Phase 18 /
   operator). This blueprint's job is the plan and the drafts, not the send.

---

## Anchor coverage

| Anchor | Where satisfied |
|---|---|
| **E6** | §2 positioning + landing draft copy (protocol headliner, delivery by-product) |
| **E7** | §3 NLnet dossier plan (eligibility, narrative, budget, timeline) |
| **E56** | §3 (grants) + §4 (B2B pricing refresh) |
| **E57** | §5 usage-PQ-api spec-only + named BUILD-TRIGGER (no implementation scheduled) |
| **E60** | §3 dossier European-dimension section (EU/UA) |
| **V6 (G11)** | §6 G11 recording template + named owner + record-when-true guardrail |

**Boundary — what this phase does NOT do:** it does not submit a grant application; it does not publish
marketing copy to any live surface; it does not build E57 billing/metering (spec-only, trigger-gated);
it does not build Phase 13/16's order path (it only records G11 once they land); it does not run the
public flip (Phase 18) it depends on. It writes no product code — planning blueprint plus in-doc draft
copy only.

---

## 8. Planning-protocol completion appendix (2026-07-17, decorrelated pass)

### 8.1 — Citation verification + new grounding

This blueprint entered this pass with **one** citation (`ac1caba40`, §1/§3) across 295 lines — by a wide
margin the weakest-grounded of the three blueprints in this assignment. Every claim below was
independently checked against the live repo; nothing is carried forward on trust.

**Existing citations verified exact:** `MANIFESTO.md` §0's quoted sentence ("A self-hosted, decentralized
network of autonomous nodes...") matches the live file line-for-line. `ARCHITECTURE.md:4` matches the
"PIVOT 2026-07-16... mesh = FOUNDATION" line exactly. `STRATEGIC-VECTORS:57` (`docs/design/
STRATEGIC-VECTORS-LOCKED-2026-07-16.md`) is confirmed: its V6 section header sits at line 56, with "bebop
PQ-protocol as HEADLINER" on the next line — the headliner/by-product framing §2's draft copy builds on
is genuinely sourced, not invented. `ac1caba40`'s AGPLv3-flip claim (§1, §3) is confirmed an ancestor of
HEAD and present on `origin/main` (same check performed for the sibling P18 blueprint).

**New grounding — LICENSE/ADR-020 state (task-directed check).** `LICENSE` at repo root is the full
AGPLv3 text (`GNU AFFERO GENERAL PUBLIC LICENSE` first line, 33831 bytes) — the "FOSS license mandate —
SATISFIED" claim (§3) holds. `docs/adr/0020-oss-license-tm-dco.md` exists (Status: Accepted,
2026-07-16) and is the authoritative decision record this dossier plan should point a grant reviewer at
for the license claim — this blueprint does not currently cite it by path.

**New grounding — the ADR-0008 SQLite→pgrust correction is itself citing a moving target.**
`docs/adr/0008-local-sqlite-pq-at-rest.md` is confirmed **Status: PROPOSED** (dated 2026-07-13), still
titled and framed entirely around "Per-node local SQLite + PQ at-rest envelope" — confirming §3's own
"⚠ CORRECTED" note that this ADR "is being updated SQLite→pgrust" is accurate as a description of
in-flight, not-yet-landed work. The substrate claim itself is grounded: `kernel/src/backup.rs:29`
`pub trait BlockStore` and `kernel/src/hydra.rs:743 FileEventStore` both exist as claimed, and `pgrust`
is a real, non-vaporware reference across `kernel/src/event_log.rs`, `hydra.rs`, `retrieval/{fixtures,
memory_store,diffusion,mod}.rs`, `kernel/Cargo.toml`, `deploy/pgrust.toml`, and `tools/deep-clean/src/
main.rs` — the correction's replacement claim ("spectral/sqlless BlockStore + pgrust, never SQLite") is
grounded in real code, not merely asserted in reaction to the operator's correction.

**New grounding — an existing in-tree artifact this phase never cites.**
`docs/design/sovereign-roadmap-2026-07-16/OPEN-SOURCE-CREDITS-LIST.md` exists (373 lines, dated
2026-07-16, a full 3-repo dependency/attribution sweep with an explicit "so the operator can star/thank
each one" purpose). This is squarely on-topic for an "ecosystem growth engine" phase — crediting upstream
projects is a standard, low-cost community-goodwill growth action, cheaper than either the grant dossier
or the pricing refresh this phase already covers — and it is never referenced anywhere in this blueprint.
This is the one piece of "what can be grounded" the task asked for that was sitting unused in the same
directory.

**New grounding — the stale brand-bible pointer (§1) is confirmed as described, nothing further found.**
`ls docs/design/dowiz-brand/` → no such directory; `find . -iname "*brand-bible*"` → zero hits; a search
of the memory corpus (`/root/.claude/projects/-root-dowiz/memory/`) for brand-related topic files also
returns nothing that resolves the pointer. §1's finding stands as written — this pass adds no new angle
on it beyond confirming the negative search is genuinely exhaustive, not a single missed `ls`.

**New grounding — `docs/pricing.md` exists but is NOT the stale pricing-v2 this phase supersedes
(checked, cleared, not a gap).** A repo-wide search for "pricing" turns up `docs/pricing.md` (33 lines)
— but it is the **order-level pricing *engine*** (line-item/tax/delivery-fee calculation formula,
Server-Side-Wins, integer minor-units), an entirely different concern from the **business/subscription
tier pricing** (§4's Free/Pro/Business framing) this phase addresses. No conflict, no overlap, no
citation owed — confirmed and cleared rather than left as an open question.

**New grounding — repo-visibility spot-check (the one load-bearing "depends on" edge, P19◄18).** The
task asked for a spot-verification of P19's hard dependency on Phase 18 (a public repo is the surface
grant reviewers/B2B prospects/landing visitors need). Live check (2026-07-17): `gh auth status` shows an
authenticated fine-grained PAT for `SyniakSviatoslav`; `gh repo view SyniakSviatoslav/dowiz` fails
("Could not resolve to a Repository"); `gh api user/repos?affiliation=owner&visibility=all --paginate`
enumerates **29** owned repositories and **`dowiz` is not among them**; meanwhile `git ls-remote origin
HEAD` (SSH) **succeeds**. This does not contradict the dependency — if anything it reinforces why growth
work must wait for a verified, API-visible, public flip — but it means the assumption "the repo simply
becomes gh-API-visible at flip" is itself unverified from this sandbox, and the same token-scope gap
noted in the sibling P18 blueprint's own appendix applies here: whatever confirms the flip took (P18 §4's
post-flip `gh repo view` check) needs a token that can actually see this repo, which the one available
here cannot.

### 8.2 — DECART

No DECART owed. Nothing in §1-§7 or this appendix introduces a new crate, tool, or vendor choice: Modal's
pricing (§4) is reused/cited from prior DECART work elsewhere in the roadmap, not a fresh decision here;
NLnet (§3) is a grant-funding body, not a software dependency; the positioning/pricing/CITATION-adjacent
deliverables are prose documents, not infrastructure. This matches the blueprint's own self-description
("writes no product code — planning blueprint plus in-doc draft copy only").

### 8.3 — 2-question doubt audit

**Q1 — 6 concrete items actually checked, not filler:**
1. `OPEN-SOURCE-CREDITS-LIST.md` exists in the same directory and is directly on-topic but uncited —
   real gap found above, not speculative.
2. The repo-visibility/token-scope gap (§8.1) — checked live via `gh auth status` + `gh api user/repos`
   + `git ls-remote`, not assumed from either direction.
3. ADR-0008's SQLite→pgrust migration is confirmed still-`PROPOSED` — meaning §3's "⚠ CORRECTED" note is
   itself citing an ADR that may need a second correction pass once ADR-0008 actually lands as pgrust.
4. The NLnet "target the next open call" timing claim (§3) was **not** re-verified against nlnet.nl's
   live call calendar in this pass — the assignment's own instruction says this research was "already
   web-verified this session, do NOT re-research," so this remains exactly as carried forward, unchecked
   by me, and should be re-confirmed close to actual submission time since call windows move.
5. `docs/design/PRICING-B2B-MESH-2026.md` (§4's proposed deliverable path) does not yet exist — checked
   (`find . -iname "*PRICING*"` finds only the unrelated `docs/pricing.md`) — consistent with §4
   describing a deliverable still to be written, not a stale pointer.
6. The G11 recording template's "named owner" (§6) defaults to naming the operator — I did not check
   whether any other canon file already names a different accountable person for delivery-track
   ownership that this default could conflict with; left as originally written.

**Q2 — biggest blind spot:** this blueprint (and the R1-E research it draws from) treats "growth" work
as safely startable in parallel with everything else because drafting doesn't require a public repo —
true for the *writing*, but the repo-visibility spot-check above (§8.1) surfaces a sharper version of the
hard dependency than the blueprint states: it's not just that a reviewer "needs a public repo to look
at," it's that the tooling this whole roadmap family (P18's script, this phase's post-flip credibility
checks) leans on to *verify* the flip took is itself unverified against this specific repo from this
environment. A plan that is otherwise careful about separating "agent-preparable" from "operator-only"
(mirroring P18's own discipline) has not yet asked whether the verification step itself has the access
it will need when the moment comes — that's the gap a fresh reader would spot and this pass did not
close, only surface.

### 8.4 — Anu / Ananke check

**Anu (logic):** most of this phase's claims are prose/positioning, not technical assertions, so "Anu"
mostly reduces to: is the *evidence for going ahead* (AGPLv3 satisfied, NLnet eligibility, the mesh-pivot
architectural rationale for rejecting hosted-cloud-only pricing) actually derivable from cited fact rather
than asserted? After this pass's checks, yes — the AGPLv3/ADR-0020 chain, the BlockStore/pgrust substrate
citations, and the STRATEGIC-VECTORS V6 headliner language all trace to live, re-verified sources. The
one place Anu was weakest before this pass — a single citation across 295 lines — is substantially
strengthened by §8.1's new groundings; the remaining ungrounded material (draft marketing copy, pricing
philosophy) is honestly non-code subject matter the assignment itself said to treat as such rather than
force citations onto.

**Ananke (organization):** the phase's own §7 acceptance checklist is a good Ananke structure — each item
is falsifiable (a submission receipt OR a dated non-apply decision; a grep for `BUILD-TRIGGER`; a grep for
forbidden pricing language). But two diligence-reliances surface from this pass: (a) nothing in this
blueprint's structure would have surfaced the uncited `OPEN-SOURCE-CREDITS-LIST.md` sitting in the same
directory — cross-referencing sibling documents in the same roadmap folder is left entirely to a future
reader's initiative, not to any checklist item; (b) the P19◄18 dependency is stated as a phase-ordering
rule ("Depends on: Phase 18 (HARD)") but nothing in either phase's structure requires confirming, at the
moment growth work actually starts, that the verification tooling (a `gh` token scoped to this repo) is
actually in place — it is assumed available when needed, the same way P18's §2.4 script assumes a
`repo:admin` token will simply exist. Both phases share this diligence-reliance; naming it once here
does not close it in either.
