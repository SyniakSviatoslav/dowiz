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
   in-repo FIPS 204/203 KATs, self-certifying node identity (ADR-0007), local SQLite PQ-at-rest
   (ADR-0008), the eqc VERIFIED-BY-MATH proof discipline. **If Phase 14 has landed, add the two-hub
   per-hub-wiki delta-exchange demo** as evidence of working replication (this is the soft dependency's
   payoff).
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
