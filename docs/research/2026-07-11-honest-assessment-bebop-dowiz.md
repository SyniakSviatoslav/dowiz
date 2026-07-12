# Honest Critical Assessment — bebop (protocol) & dowiz (product) — 2026-07-11

> My own synthesis judgment, grounded in this session's verified work (full audit, 13 gap blueprints,
> hub-architecture review, local-first synthesis A–D + concurrent 01/02, field-sim dissection, bebop
> protocol-state deep-read, the four-lens max-EV program, relay/anonymity/legal deep-dives). Not a
> research fan-out — an opinion, argued. Written to be useful, which means it leads with what's wrong.

> **Correction (added post-write, operator input 2026-07-11).** Two premises below are revised by
> facts the read-only audit could not see: (1) **v1 is being tested on a first client right now** — so
> "zero real orders / zero validation" overstated; market contact IS happening, at the pilot scale
> appropriate to v1. (2) The in-person walk-in isn't available to the operator at the moment (it comes
> later); meanwhile the plan is quality-first — refine the updated version, then send it *remotely* to
> owners, gated on tech + design being stable. Demo/vendor-search/offer pipelines already exist at
> ~80%. This makes the "go validate now / it's elegant procrastination" framing too harsh: there is a
> deliberate, defensible quality-first sequencing, not avoidance. The one residual caution stands —
> "stable enough to send" must be a concrete checklist, not a feeling, or a quality-first person can
> let the line drift. Grades revised accordingly: dowiz-as-business **C → C+/B− (early validation in
> progress)**; the rest holds.

---

## 1. bebop as a protocol

**Now: a disciplined research artifact that is not yet a protocol.** The craft is real — 388/388
tests, a zero-dependency PQ crypto core, a three-model review gate that genuinely caught malleability
bugs, and tested primitives (matcher, pod, reputation, ledger, zkvm-as-hash-commit). But a protocol
earns the name through a wire, a second implementation, and someone speaking it. bebop has none:
`zenoh.rs` is an in-process stand-in, zenoh is not even a dependency, there is **zero network code,
zero storage, no runnable node binary**, and settlement/dispute — the economic heart of any delivery
protocol — is **0 lines**. So today bebop-the-protocol is a **library kit plus a manifesto**, not a
protocol. Three further honest marks against it: (a) the crypto is non-FIPS-interop with
KyberSlash-class and Ed25519 timing hotspots and no external audit — usable only as the hybrid half,
never alone; (b) the "field-sim physics" that some docs lean on as a differentiator has a **verified
sign bug masked by green tests**, and its surrounding theory is half numerology — a credibility
liability if ever pitched as the edge; (c) **nobody can install it** (npm 404, needs clone+cargo),
so trial friction is infinite.

**Long-term: a genuinely good vision that is structurally downstream of dowiz.** Trustless,
local-first delivery coordination with per-actor sovereignty and no aggregator rent is a real and
possibly important idea, and this session's research showed the substrate is *achievable* (relay-
assisted P2P at ~€6/mo, COD-serverless, legal now). But bebop's own cold-start plan admits it needs
a working dowiz with real venues first — so the protocol is **not a parallel bet, it is a
second-order consequence** of the product succeeding. The danger is precise: a protocol is the most
intellectually satisfying place in this whole stack to hide from the market, and bebop's scope has
already sprinted through four identities in four days (coding agent → physics planner → delivery
protocol → PQ crypto lib). **Verdict: keep it parked as a capture-and-protect asset** (commit, push,
one demo, memory bootstrap — cheap, done in an hour), and do not fund it as a protocol until a
product exists to carry it. Its beauty is real; its priority is not.

## 2. dowiz as a product

**Now: technically excellent, systematically unvalidated — the two facts that matter most are in
tension.** The engineering is top-decile for a solo effort: a live hardened Node/React product,
integer money, FORCE-RLS available, RS256, 162 migrations, a *genuinely enforced* CI/meta-controller
harness, and the best part — a hardened courier backend (invite → honest dispatch → deliver-v2
cash-as-proof → journal redispatch → per-frame WS authz), which is the hardest thing to build and
the thing it built best. And against all of that: **zero real orders, zero claimed venues.** After a
month that produced a full Rust rewrite, an event-sourced kernel, and a hand-rolled crypto library,
there is no evidence of one real non-operator paid order. That is the whole assessment in one line.

The supporting failures all point the same direction — **the product avoids its own revenue edge.**
The one working door leaks at every hinge: `/claim` 404s everywhere (every claim link ever minted is
dead — one line), 3 of 6 messengers 400 at checkout, `/courier-invite` missing from routes, the prod
worker stopped since 07-03, demos live only on staging. "Many channels" is attribution-true and
transport-false — no messenger or bot can actually create an order; the hub is **~1/3 real**, one
intake wrapped. It runs as **two half-hubs on one spine** — prod 100% Node, the Rust kernel dark with
gates that are theater (`hub_checkout` gates nothing, replay-parity is a placeholder, and the
reference implementation bypasses its own `kernel::decide`). GDPR fixes sit un-shipped in prod (live
legal exposure). And four "authoritative" futures compete with none closed.

**Long-term: a coherent, differentiated, and — rarely — achievable vision.** Commission-free,
own-courier, local-first, multichannel, anonymous-by-construction, sovereign — and the market read is
real: Albania is cash-first, aggregator commissions bite (Wolt 25% next door), InstaPorosi already
validates the exact wedge, and the own-couriers model dodges gig-labour law (a real moat). This
session's research kept returning the same surprising result: the ambitious destination is not a
fantasy — it is buildable at ~€6/mo and legal to pilot today. That is a genuinely strong position.

## 3. The honest synthesis (both together)

The two projects share one pathology and one cure. **The pathology: both substitute engineering
depth for market contact.** dowiz is over-built and under-validated — the exact inversion of what a
pre-revenue product should be; bebop is a protocol with no users, no wire, and a cold-start that
depends on dowiz having the users it doesn't have yet. Every layer added since the project's own
Business-Value-Sort warned of "elegant procrastination" — the rewrite, the kernel, the crypto, the
local-first research, even this session's beautiful interface direction — raised the cost of learning
the one answer without changing its likelihood. **The stack has out-run the market test by a month,
and the gap is widening.**

**The cure is embarrassingly small relative to what's been built:** fix `/claim` (one line), walk
into ArtePasta in the 15:30–17:30 lull with a printed QR card that shows the owner their own
restaurant live, and find out. The max-EV program priced this at +€1,155 and, more importantly, found
the real risk is **non-execution, not rejection** — the pre-committed "10 contacts, 0 claims → stop"
is itself worth ~€1,800 in information. Nothing above P1 of any roadmap — not the local-first
substrate, not the audited crypto, not the protocol, not the tide-over-bedrock surface — should move
before that one data point exists.

**Why the critique is sharp precisely because the work is good:** the discipline (a living-memory
corpus most funded teams lack), the honesty (build-dark is never called shipped in the corpus itself),
the courier backend, the security posture, and the repeated research finding that the vision is
*achievable* — these are real and rare. This is not a project that is failing on quality. It is a
project one real order away from being real, that keeps building instead of taking it. The single
highest-leverage change is not technical. It is to point a week at a restaurant, not a repo.

**One-line grades (my opinion, honestly held):**
- bebop as a protocol: **A− as research, D as a protocol** (no wire/users/settlement; correctly parked).
- dowiz as a product: **A− as engineering, C as a business** (zero validation is the ceiling until order #1).
- The combined long-term vision: **genuinely strong and, unusually, achievable — bottlenecked entirely on execution of the smallest step.**

*Assessment written 2026-07-11. My judgment, not a delegated finding; every factual claim traces to a
verified item in this session's docs.*
