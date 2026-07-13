# D6 — Business Value Teardown: dowiz / DeliveryOS / bebop

> Red-team due-diligence review, 2026-07-13. Author: adversarial VC-partner / incumbent-strategy lens.
> Method: read PRODUCT.md, MANIFESTO.md, DECISIONS.md, DeliveryOS-Business-Value-Sort.md,
> DeliveryOS-Product-Goal-Alignment-Audit.md, DeliveryOS-As-Built-Summary-v1.md,
> MASTER-ROADMAP-MVP-2026-07-12.md, docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md,
> BRAND-BIBLE.md (recovered from git 330ff4ed — the file no longer exists on the active branch),
> README.md (origin/main), docs/design/payments/*; plus live probes of https://dowiz.fly.dev and
> https://dowiz-staging.fly.dev on 2026-07-13. CLAIMED and SHIPPED are separated throughout.

---

## 1. Bottom line — PASS (venture kill; viable only as a bootstrapped lifestyle tool)

Pass. The stated product — a $19–59/mo commission-free branded-ordering SaaS for small Albanian
restaurants — attacks a real pain (25–35% aggregator commission) but sells a wedge that Oracle's
GloriaFood already gives away free, into a market whose entire in-country revenue ceiling is a
rounding error, with no billing code, no live acquisition funnel (`/claim` 404s on prod today), no
savings/ROI surface (its own #1-ranked lever), and — after ~1,400 commits in six weeks — **zero real
customer orders and zero paying restaurants**. Worse, the project has formally pivoted away from the
revenue product: the entire production stack (API, worker, DB, Fly config) sits quarantined in
`attic/` on the active branch, and the new constitution (MANIFESTO.md C1–C13, DECISIONS.md D0/D1)
subordinates "MVP-first pragmatism" to six ideological invariants — decentralized, local-first,
post-quantum, from-scratch crypto, mesh, reliability-over-latency — none of which any restaurant
owner has ever asked for. The team's own strategy doc (Business-Value-Sort, Tier 4) diagnosed this
exact failure mode ("the process apparatus is becoming an elegant substitute for the only thing that
validates the business: a real venue placing a real paid order") — and the response, verifiable in
git, was to build a delay-tolerant satellite-grade post-quantum delivery protocol instead. As an
incumbent I would not even track this; as an investor I pass and watch the founder as engineering
talent.

---

## 2. Value-proposition teardown

### 2.1 What is CLAIMED
- README.md (origin/main): "A 0-commission, data-ownership food-delivery platform for small
  vendors." Pricing table: Free ≤50 orders, $19/$39/$59 tiers, "no per-order commission on any tier."
- Business-Value-Sort (header): wedge = price ("aggregator 25–35% commission = the enemy; flat
  $19/39/59 subscription, ROI pays back in 3–4 days") + own branded channel ("your delivery, your
  customers, your data"). Target: 1–50 venues currently taking orders via Instagram DM/phone; 77% cash.
- MANIFESTO.md §0 (2026-07-12, "AUTHORITATIVE"): the product is now "a self-hosted, decentralized
  network of autonomous nodes… running a NON-AI, post-quantum-secure delivery protocol."

### 2.2 What is SHIPPED (live probes, 2026-07-13)
| Claim | Live reality | Evidence |
|---|---|---|
| Branded storefront `/s/:slug` | EXISTS. SSR of meta tags only; body is an empty `<div id="root">` — a diner on the stated "flaky connection" gets a blank page until a multi-hundred-KB React bundle loads; 7 Google-Fonts families render-block from a third-party CDN | curl `/s/demo` prod: 200, 3.7 KB HTML, no menu markup |
| Venue acquisition funnel `/claim` | **404 on prod.** `ClaimPage.tsx` exists on main (main.tsx:55) but `/claim` is absent from server SPA_ROUTES (server.ts:840) — every shared claim link dies server-side | curl prod `/claim` → 404; ROADMAP-GROUND-TRUTH §1 marks this "DONE (verified) → prod (f0bd9966)" — **false**: f0bd9966 is "P3 wire storage into AnonymizerService (GDPR photo purge)" |
| Marketing landing / pricing page | NONE on prod. `/` redirects to `/start`; the Warm Cosmo-Noir cinematic landing (commit 330ff4ed) never merged to main; no pricing surface anywhere in the app | main.tsx:50; grep for pricing strings in apps/web → zero hits |
| Billing / ability to charge money | NONE. "Stripe/Billing: Stub (post-MVP)" — there is no code path by which any restaurant can pay dowiz anything | As-Built-Summary §2 "Shims vs Real"; grep origin/main for stripe/billing → push-subscription hits only |
| Savings/ROI display ("you saved $X vs Glovo commission") | ABSENT. Their own Business-Value-Sort Tier 5 ranks this the cheapest, highest-leverage retention lever ("подвоїти"); the Alignment-Audit §A calls it the "найбільша можливість." A month later: zero hits in admin UI or API | grep origin/main apps/web/src/pages/admin + apps/api/src for savings/commission → nothing |
| Clean, premium production surface | Prod sitemap serves Google `debug-loc-1781513997559`, `gp-e2e-1781588122647`, `rg-tenantb` test venues — the exact "visible test-data clutter" PRODUCT.md lists as the anti-reference to kill | curl `/sitemap-locations-1.xml`; PRODUCT.md "Anti-references" |
| Staging integrity | Staging sitemap index leaks an internal address: `https://dowiz-rust-staging.flycast/sitemap-locations-1.xml` — broken for crawlers, exposes infra topology | curl staging `/sitemap.xml` |
| "First real order" (G11) | NOT ACHIEVED. MASTER-ROADMAP §3 marks "S4 · FIRST REAL ORDER — DONE (G11 GREEN)" — but it is a **cargo-test simulation** (`node/src/sim.rs`, "simulated first real order end-to-end"). Tier-3 G11 (a real non-operator order) is explicitly still open (ROADMAP-GROUND-TRUTH Tier 3: "external, not code") | MASTER-ROADMAP §3 vs ROADMAP-GROUND-TRUTH §2 Tier 3 |

### 2.3 Vitamin or painkiller?
The commission pain is real. But dowiz's actual offer to the vendor decomposes into:
(a) a branded ordering page — **free elsewhere** (GloriaFood, or a Linktree + Instagram DM, which is
what the target segment uses today at zero cost and zero training);
(b) "no commission" — but commission is not rent, it is a **bundled price for demand generation and
courier logistics**. Removing it hands both costs back to a 1-location owner who has neither
marketing budget nor dispatch capacity. dowiz's answer (owner-managed couriers, own channel) only
fits venues that *already* self-deliver and have brand pull — the exact venues already served by
free tools;
(c) "your data" — small Albanian restaurateurs do not wake up wanting data portability. It is a
founder value, honorable, and a vitamin.

The kernel of a painkiller exists — "never miss an order at 19:00, run it from your phone,
cash-loop that reconciles" — but it is buried, unmonetizable (no billing), and now unmaintained
(stack in `attic/`).

### 2.4 The pivot vs the product
MANIFESTO C13: "`server/` (axum/rusqlite centralized deploy) is DROPPED… replaced by peer nodes with
local SQLite." DECISIONS D1 repeats it: "a centralized dispatch/deploy server is the anti-pattern
the protocol exists to kill." D0: the six invariants "outrank roadmap sequencing, feature requests,
and 'MVP-first' pragmatism." Commit e1505e1d ("chore(declutter C2)") physically moved `apps/api`,
`apps/worker`, `packages/db`, `fly.toml` into `attic/`. Meanwhile MANIFESTO §6 admits: "Not a
business plan… Not '0% fee = moat' (poetry)." The constitution of the company now states, in
writing, that it is not a business. Take it at its word.

---

## 3. Competitive & moat analysis

### 3.1 Named competitive set
- **GloriaFood (Oracle).** Free commission-free restaurant ordering site/widget, menu management,
  order app, ~30 languages; paid add-ons around $29/mo each. This is dowiz's wedge, given away free
  by a company with infinite distribution. dowiz's own Alignment-Audit cites GloriaFood twice as the
  evidence base for its design choices — the team knows the free anchor exists and prices against it anyway.
- **Wolt (DoorDash).** Live in Tirana since 2024. Brings what dowiz structurally cannot: demand. A
  restaurant paying 25% to Wolt receives customers; a restaurant paying $39 to dowiz receives a URL.
- **Glovo / Delivery Hero Balkans; local players (e.g. Baboon in Albania).** Same asymmetry.
- **The real incumbent: Instagram DM + phone + a guy on a moped.** Free, zero training, already
  where the customers are. Business-Value-Sort itself defines the target as venues "that today take
  orders via Instagram/phone." Displacing free-and-familiar with $19/mo-and-new requires either
  demand-gen (absent) or 10x workflow gain (a menu page is not 10x over a pinned Instagram story
  for a 20-order/day venue).
- **Shopify/Square + WhatsApp** for the upmarket tail.

### 3.2 Moat audit — claim by claim
| Claimed moat | Verdict |
|---|---|
| Deterministic Rust/WASM kernel, event-sourced law, integer money | Supplier-side virtue. No restaurant owner has ever chosen a POS because state = fold(events). Correctness is table stakes delivered invisibly by every incumbent at scale. Not a moat; a hiring brag. |
| Post-quantum crypto (from-scratch ML-KEM-768/ML-DSA-65, C10 "zero-dep") | Negative moat. Home-rolled crypto handling money/PII is a DD red flag, not an asset; NIST-track PQ will be a commodity library flag in every TLS stack before dowiz has 100 customers. Threat model (harvest-now-decrypt-later of… kebab orders in Durrës) does not survive contact with a buyer. |
| Decentralized / no-central-server protocol (C4, C13) | Anti-moat. It removes the operator's own control point ("open protocol, closed access" is the only way protocols monetize — MANIFESTO §2 explicitly engineers that escape hatch shut). C6 mandates AGPL open source: the code moat is donated by design. |
| "No AI" (C1) | Indifferent to buyers; forecloses the one feature class (menu import OCR, demand prediction, support deflection) their own Tier-1 list calls a core enabler ("ШІ-імпорт меню б'є найбільший фрикшн"). The constitution now contradicts the core funnel. |
| "Money never tweens" (trust cue) | A CSS animation preference elevated to doctrine. Zero willingness-to-pay attached. |
| Network effects | Explicitly renounced: no cross-venue discovery, no marketplace (Alignment-Audit §B — "нема крос-закладного discovery" is a *design goal*). Single-tenant white-label = zero demand-side network effect. Couriers are owner-managed = zero supply-side effect. |
| Data moat | Renounced by principle: "diner data belongs to the restaurant," export/erasure mandatory (§B). Ethically admirable; commercially it is a promise of frictionless churn. |
| Switching cost | Low by design (portability is the pitch). The moat pitch and the product pitch are mutually exclusive and the team chose the pitch. |
| The honest residual moat | Local fit: sq/en/uk i18n, cash-HOLD reconciliation loop, map-pin for weak Albanian geocoding, Telegram-ops alerts. Real, thoughtful, and worth something — against global SaaS that ignores Albania. Worth nothing against Wolt's country team or one local reseller white-labeling GloriaFood. |

---

## 4. Strategic-coherence critique

This is the fatal axis. The repo simultaneously contains, all active within the same 6 weeks:

1. **A food-delivery SaaS** (the attic'd TS/Fastify/Supabase stack, 92 Playwright tests, 67 migrations).
2. **A from-scratch post-quantum cryptography suite** — Keccak/SHAKE, ML-KEM-768, ML-DSA-65, X25519
   hybrid, at-rest volume encryption, ML-DSA code-signing (MASTER-ROADMAP §0: kernel 144 lib tests).
3. **A decentralized delay-tolerant delivery network** — DTN/BPv7 (RFC 9171) custody transfer, BIBE,
   QUIC/TCPCLv4 convergence layers, "satellite/lab-grade" transport, with SpaceWire/SpaceFibre named
   as candidate link layers (DECISIONS D3) — for couriers carrying sushi across a city of 175k people.
4. **NOSTR / ActivityPub / MCP adapters** wrapped in PQ envelopes (MANIFESTO §5).
5. **An agent-governance harness** with gender/profanity/archetype axes, a hard-banned "voodoo"
   archetype, and a default "serves God" relation (docs/design/BEBOP-GOVERNANCE-PORT-TO-DOWIZ-2026-07-12.md,
   `agent-governance/index.ts`) — inside a restaurant-ordering repo.
6. **A meta-engineering apparatus**: six quality gates, councils, critics, reflections, regression
   ledgers, lessons INDEX, doubt-escalation ladders, token-economy proxies (docs/harness, loops/,
   docs/reflections, docs/regressions).
7. **A second repo** (/root/bebop-repo) hosting the protocol twin, with its own manifesto, brand,
   and memory system.

The team's own documents convict the strategy:
- Business-Value-Sort Tier 4 (the founder's hired honest broker): "будувати й полірувати
  якість-апарат стає витонченою заміною єдиного, що валідує бізнес: реальний заклад, який провів
  реальне платне замовлення… закапи апарат… виведи заклад №1 наживо." Verdict line: "обмеження це
  невалідований попит."
- MANIFESTO C8 lists "over-engineering is the #1 enemy"… and then D6 overrides C8 ("Mesh +
  post-quantum are NOT deferred") and D0 declares the six ideology-words outrank MVP pragmatism.
  The YAGNI clause was amended to exempt precisely the things YAGNI exists to stop.
- ROADMAP-GROUND-TRUTH §0.1: ~20 research/design reports cited as the basis of the plan "do not
  exist on local disk in either repo" — the plan was built on documents that were never written.
- The stack was rewritten mid-pivot: TS/Node → "DROPPED from the build target" (§0.5), React app
  demoted to "LEGACY ORACLE," new canonical stack = Rust/WASM + Astro/Svelte + hand-rolled WebGL
  particle cloud (≤7 kB gz chunk gate). Then the freshly built Rust `server/` was itself dropped
  (D1) three days later for being too centralized. Two full-stack rewrites inside one week, both
  before the first customer.

Is this a product or a playground? The commit record answers: 858 commits on main since 2026-05-31
plus 604 on the pivot branch, one author, and the newest main commits are documentation about plans.
It is a solo founder pair-programming with an agent swarm at extraordinary velocity — in circles.
The playground is world-class. The product is unattended: its funnel 404s and its billing does not exist.

---

## 5. Unit-economics & GTM holes

- **Revenue mechanics: none.** No billing integration (stub), no pricing page, no upgrade path, no
  invoice. Today the company cannot accept money even from a willing customer. Everything downstream
  of this is hypothetical.
- **TAM/SAM realism.** Albania: ~2.7M people; realistically low-thousands of venues that deliver, of
  which the addressable slice (self-delivering, smartphone-comfortable owner, not exclusive to an
  aggregator) is in the hundreds. At the blended ~$30/mo price: 300 paying venues ≈ **$108k ARR** —
  the in-country ceiling of the entire stated GTM. i18n (uk/en) hints at Ukraine/diaspora expansion,
  which multiplies competitors (Glovo/Bolt Food are entrenched in Ukraine) faster than TAM.
- **Willingness to pay.** Segment anchored at zero by GloriaFood-free and Instagram-DM-free; 77%
  cash economy; ROI story ("pays back in 3–4 days") requires the venue to already have order volume
  being taxed by an aggregator AND the ability to self-deliver — a narrow intersection.
- **CAC vs LTV.** This segment is sold door-to-door in Albanian, venue by venue, with hand-held
  onboarding (the docs' own "concierge mode"). A solo technical founder is the only salesperson and
  is currently occupied implementing RFC 9171 custody transfer. Even granting $500 LTV (14+ months at
  $39 with charitable churn for a business category with high mortality), CAC in founder-hours makes
  payback irrelevant because sales capacity ≈ 0 hours/week on observed allocation.
- **Cold start.** Vendor side: unsolved (funnel 404). Customer side: structurally unsolved — no
  discovery surface, no JSON-LD Restaurant/Menu markup in the live storefront HTML (their own
  Alignment-Audit §J flags this as claimed-but-unbuilt; live curl confirms absent), OG image support
  landed on staging only. Every venue must generate 100% of its own demand; the product does not help.
- **Regulatory tail.** Fiscalization (Albania's mandatory real-time e-invoice regime) is *known* but
  parked as an open human-gate (payments docs NH-5, "carried"; counsel-opinion.md:166-167) — it gates
  any legal card-payment launch. The bebop settlement design ("threshold-signature verifier,"
  MANIFESTO §2) walks toward money-transmission/PSD2-adjacent licensing with no counsel engaged.
  GDPR posture is genuinely decent (anonymizer, claim-check, erasure) — credit where due — but a
  2026-07-03 secrets-in-git-history incident with the remote scrub still an open gate is a DD fail
  on its own.
- **Support economics.** The docs' own "19:00 phone call" principle concedes each feature carries
  support cost in a language/market where the founder is the entire support org.

---

## 6. What kills this — ranked

1. **Founder attention is constitutionally allocated to the playground.** DECISIONS D0 makes six
   ideology-words outrank shipping; D1 deleted the deployable server; the revenue stack is in
   `attic/`. This is not drift that a board fixes — it is codified. The company's operating documents
   now define the business plan as out of scope (MANIFESTO §6). Probability-weighted, the product
   dies of neglect before any market force touches it.
2. **Zero demand validation.** ~6 weeks, ~1,400 commits, 0 real orders (G11 open; the "FIRST REAL
   ORDER — DONE" line is a simulation in `sim.rs`), 0 paying venues, 0 ability to be paid. The one
   experiment that matters has never been run.
3. **The wedge is free elsewhere.** GloriaFood (Oracle) gives the commission-free branded-ordering
   product away; Instagram DM is free and native to the segment. There is no pricing power at
   $19–59/mo against a free anchor without demand-gen, which dowiz explicitly does not do.
4. **No demand-side story at all.** Aggregators are hated *and paid* because they deliver customers.
   dowiz delivers a URL and hands acquisition back to a 1-location owner. No discovery, no SEO
   structured data live, no marketplace (by principle). The anti-marketplace stance is the moat
   renunciation and the growth renunciation in one move.
5. **Execution-integrity record fails DD.** The "ground truth" doc contains a false verification
   (/claim "DONE, verified, f0bd9966" — live 404, wrong commit); memory records three consecutive
   FALSE-GREEN agent rounds and agents escaping worktrees in three waves; ~20 foundation reports
   never existed on disk; prod sitemap ships debug fixtures; credentials sat in git history. A
   process that manufactures unverified "DONE"s will manufacture them about revenue too.
6. **Solo bus-factor + double rewrite.** One author, two stack rewrites pre-revenue (TS→Rust/Astro,
   then centralized-Rust→peer-nodes), a second repo, and an agent-governance metaverse. Velocity is
   real; direction integral over six weeks is approximately zero.
7. **Market ceiling.** Even flawless execution of the stated GTM caps around low-hundreds of $k ARR
   in-country — a fine bootstrapped business, not a venture outcome, and not worth the PQ-protocol
   optionality story used to justify the detour (protocols monetize worse than SaaS, and this one is
   AGPL + anti-capture by charter).

---

## 7. Steelman — the strongest honest case FOR, and why it still loses

**The steelman.** The commission revolt is real and global; the EU restaurant own-channel movement
has genuine tailwind. The market empathy embedded in this product is far above tourist grade:
cash-HOLD reconciliation for a 77%-cash economy, map-pin ordering because Albanian geocoding is
weak, Telegram as the owner alert rail, sq/en/uk i18n, "confirmation is mandatory — an order without
owner confirmation does not exist," phone-number fallback when the platform dies. That is a product
thesis written by someone who has actually watched an Albanian restaurant at 19:00 — Wolt's Tirana
country team has not. The engineering quality bar (RED-proven tests, integer money, RLS,
idempotency, event-sourcing) is exactly the reliability religion that SMB trust is built on, and it
is rare. The self-diagnostic capacity is rarer still: Business-Value-Sort and the Alignment-Audit
are better, more honest strategy documents than most funded seed companies produce. A real pilot
venue shape exists (Dubin & Sushi, Durrës, live at `/s/demo` with correct per-venue meta). If the
founder froze the protocol work today, spent 90 days doing concierge onboarding of 10 Durrës venues
with a Stripe checkout and the savings counter, this could be a profitable €3–8k MRR regional
business within a year — and the PQ/DTN work, judged purely as R&D, is legitimately impressive.

**Why it still loses.** Because the counterfactual just described was written down *by this team, in
this repo, a month ago* — Tier 4/5 of Business-Value-Sort is precisely that memo — and the observable
response was to ratify the opposite in MANIFESTO/DECISIONS and physically attic the product. Revealed
preference beats stated strategy: given a free choice each morning for six weeks, the founder chose
custody-transfer semantics over customer #1 every single day. Even in the repaired world, the
business hits the Albania ceiling and the GloriaFood free anchor, and the only expansion assets —
protocol, kernel, brand — are respectively donated (AGPL), invisible to buyers, and unshipped (the
Warm Cosmo-Noir landing never reached main). The most probable terminal states, in order: (a) an
exquisite, formally verified, post-quantum protocol with zero nodes run by strangers; (b) a
half-maintained pilot that quietly churns when the founder's attention completes its orbit; (c)
acqui-hire of the founder, with the repo as portfolio. None is an investment.

---

## Appendix A — CLAIMED vs SHIPPED ledger (spot checks, 2026-07-13)

| # | Claim (source) | Status | Proof |
|---|---|---|---|
| 1 | "/claim 404 fix → prod (f0bd9966)" (ROADMAP-GROUND-TRUTH §1 "DONE (verified)") | FALSE | live `curl https://dowiz.fly.dev/claim` → 404; origin/main server.ts:840 SPA_ROUTES lacks `/claim`; f0bd9966 = GDPR photo-purge commit |
| 2 | "FIRST REAL ORDER — DONE (G11 GREEN)" (MASTER-ROADMAP §3 S4) | SIMULATION ONLY | `node/src/sim.rs` cargo test; real-order G11 still "external, not code" (ROADMAP-GROUND-TRUTH Tier 3) |
| 3 | Pricing $0/19/39/59 (README.md) | UNBILLABLE | Stripe "Stub (post-MVP)" (As-Built §2); no pricing surface in app |
| 4 | Savings/ROI counter = top retention lever (Business-Value-Sort Tier 5) | ABSENT | grep origin/main admin+api for savings/commission → 0 hits |
| 5 | "storefront SSR" (As-Built §1) | META-ONLY | `/s/demo` HTML: og/twitter tags + empty `#root`; no menu markup, no JSON-LD |
| 6 | "kill visible test-data clutter" (PRODUCT.md anti-reference) | VIOLATED IN PROD | sitemap-locations-1.xml lists debug-loc-*/gp-e2e-*/rg-tenantb |
| 7 | Warm Cosmo-Noir brand shipped (BRAND-BIBLE) | NOT ON MAIN / NOT ON PROD | landing commit 330ff4ed unmerged; prod `/` → `/start`; brand-bible file absent from active branch |
| 8 | ~20 foundation research reports (2026-07-11 brief) | NEVER EXISTED ON DISK | ROADMAP-GROUND-TRUTH §0.1, "headline risk" |
| 9 | Product stack maintained as "legacy oracle" | QUARANTINED | commit e1505e1d moved apps-api/apps-worker/packages-db/fly.toml to `attic/`; only `apps/web` remains |
| 10 | Staging healthy public surface | LEAKY | staging sitemap index points to internal `dowiz-rust-staging.flycast` |

*Confidential — red-team work product. Judgments falsifiable against the citations above; re-run the curls before reuse.*
