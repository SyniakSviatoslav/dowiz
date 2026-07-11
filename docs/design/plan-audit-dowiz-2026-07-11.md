# Plan Audit — dowiz / DeliveryOS toward a decentralized, PQ-secure, food-vendor delivery protocol

> Read-only inventory produced 2026-07-11 by an independent plan-auditor lane. No code changed.
> Scope: `/root/dowiz` plans, architecture, hub/courier/orders design, crypto/security requirements,
> and living-memory context relevant to unifying dowiz + bebop/bebop2 into ONE blueprint for a
> decentralized, NON-AI, post-quantum-secure delivery protocol for food vendors who want to stop
> relying on third-party courier/aggregator services.
> Sources cited by `file:line`. Living memory read at `/root/.claude/projects/-root-dowiz/memory/MEMORY.md`.

---

## 0. TL;DR for the unifier

- The vendor-protocol goal is **already the stated product thesis**, not a pivot. The Sovereign Core
  MVP is explicitly *"own-channel owner data-hub … 0%-commission direct checkout … honors the
  escape-aggregators thesis"* (`docs/design/sovereign-core-mvp/DECISIONS.md:7-11`).
- **Decentralization is a locked design invariant, deferred as a feature.** Event-sourced pure
  Rust/WASM core + content-hash + signature slot + transport-agnostic sync port are baked NOW;
  libp2p/mesh/CRDT and per-actor Ed25519/PQC auth are Phase 2+ (`DECISIONS.md:13-19`, D2).
- **PQ crypto exists but is quarantined.** PQC (Kyber/Dilithium) was operator-Red-Teamed as
  premature for the dowiz MVP (`DECISIONS.md:41-43`, D6). Real from-scratch PQ primitives (ML-KEM-768,
  ML-DSA-65) live in **bebop2**, are green-on-KAT but **NOT FIPS-interoperable and have live timing
  leaks** (`gap-blueprints-2026-07-11/G09-bebop2-crypto-assurance.md`).
- **Biggest conflict for the unifier:** the dowiz MVP deliberately keeps money on a *single central
  checkout surface* and defers decentralization/PQ, while the unified goal wants decentralized +
  PQ-secure. The seams are designed for it, but the actual crypto/mesh/identity work is unbuilt and
  gated. Also today's prod still runs a **central Fly/Supabase stack with BYPASSRLS** (~103 dormant
  RLS policies) — the opposite of "local-first, owner-run" (`G10`, `MEMORY.md` B3 line).

---

## (a) Plans / arcs and current status

### Sovereign Core MVP — the spine most relevant to the goal
Corpus: `docs/design/sovereign-core-mvp/` (MANIFESTO, DECISIONS, ANALYSIS, GRAND-PLAN, LEAD-REVIEW,
STRUCTURE-UPGRADE, PROGRESS, HANDOFF-2026-07-07-SESSION, IMPLEMENTATION-ROADMAP, PHASE-2-2-CART-TOKEN-SPEC).

| Item | Status | Evidence |
|---|---|---|
| MVP scope = own-channel hub + read-only aggregator view, 0% direct checkout | **DEFINED / locked** | `DECISIONS.md:6-11` (D1) |
| Phase-Zero (wasm32 + clippy purity gate) | **DONE (staged)** — Step 1+2 done, Step 3 partial | `MEMORY.md:28`; `DECISIONS.md:50-53` |
| 0b-1 money boundary `Lek(i64)`, 0b-2 event vocab/Envelope, 0b-3 corridors behind `decide` | **DONE + pushed** | `MEMORY.md:27` (`c10814ab`/`e3e30ac1`/`31520e8a`) |
| 0b-5 keystone (shell flips to `kernel::decide`, deployed-reality RED proof) | **DONE** | `G06:76-77` (`92cc239b`/`69293616`); `MEMORY.md:26` |
| Phase-1/2 build: 0b-4, 1.1, 1.2, 1.5, 2.2, 2.3 | **DONE (staging v266)** — 7 of 12 phases built | `G06-sovereign-core-exit-gate.md:20-21` |
| Phase 1.3 (sync PORT), 1.4 (signed-event envelope), 2.1 (distribution artifacts), 2.4 (aggregator stub), 0b-6 (CI) | **OPEN / not started** (1.4 partially landed via 1.2 columns) | `G06:22, 51-71` |
| MVP exit gate (register channels+QR, real 0% order, dashboard attribution, own+erase customer, money battery green, `/reliability-gate` GO, replay-parity, NOBYPASSRLS, sovereign CI on main) | **NOT CLOSED** — declared "SHIPPING-READY" but gate open; verification debt on the "built" 7 (hub_checkout gates nothing, replay-parity placeholder, cause_hash literal "placeholder", vacuous Playwright) | `G06:16-45`, `G06:30-35` (gate text), `MASTER-EXECUTION-PLAN.md:24` (G06 §2) |

### Gap-blueprints program (2026-07-11) — the current unification/sequencing layer
`docs/design/gap-blueprints-2026-07-11/` G01–G13 + `MASTER-EXECUTION-PLAN.md`. **All plan, zero code**
(`MASTER-EXECUTION-PLAN.md:7`). Waves:
- Wave 0 protect/stop-bleeding, Wave 1 prod vehicle (GDPR trio + checkout fix + /claim), Wave 2
  validation week, Wave 3 secret-history scrub, Wave 4 gated tracks (Sovereign #1, rebuild, Astro,
  security edges, bebop, bebop2). `MASTER-EXECUTION-PLAN.md:35-79`.
- **Status: awaiting operator decisions** (15-item queue `MASTER-EXECUTION-PLAN.md:105-127`). Nothing executed.
- The one metric that matters: *a real order row from a non-operator customer on a claimed venue*
  (`MASTER-EXECUTION-PLAN.md:138-140`).

### Rebuild (TS→Rust/Astro strangler) — courier/orders/auth
`docs/design/rebuild-*` councils. Status: **in-progress, mostly mothball-default**.
- Staging cutover: S1/S3/S5/S7/S9/S10 serve Rust today; behavioral parity is the unknown
  (`MASTER-EXECUTION-PLAN.md:22`, G04). Live state `docs/ops/rebuild-cutover-h_t.json`; `MEMORY.md:17`.
- S7 courier/dispatch port = **DRAFT, NOT APPROVED** council packet (`rebuild-courier-s7-council/proposal.md:3`).
- Rust money newtype Phase A resolved; Phase B pending (`rust-money-newtype-phase-a/resolution.md`).
- Recommendation in G04: **Path B mothball** unless full A0 gate sheet banked (`MASTER-EXECUTION-PLAN.md:74`).

### Payments arc — crypto-first, non-custodial
`docs/design/payments/` — **designed, largely unbuilt**. Provider = Plisio (non-custodial, funds →
merchant wallet, USDT-TRC20 + USDC, stablecoin-only, HMAC signature-verified webhook); card deferred
(`payments/resolution.md:71-73`). Webhook = idempotent, signature-verified, sole writer of
paid/failed/refunded (`payments/research.md:42-44`, `payments/proposal.md:138-140`). **Highly relevant**
to "no third-party" — this is the money-sovereignty half.

### Other active arcs (from MEMORY.md:9-23)
- Cross-agent fallback mesh (Hermes+OpenCode+Goose+Aider+OpenHands) — installed, dormant until keys (`MEMORY.md:10`). *Agent infra, not product.*
- Security incident: creds in git history — rotated; remote scrub = **open operator gate, HARD blocker for any prod push** (`MEMORY.md:20`).
- Open-source goal per ADR-020 (AGPLv3 + TM + DCO) — **gated on secrets scrub + EUTM**; and ADR-020 was *never actually committed* (`MASTER-EXECUTION-PLAN.md:31`, G07 §1). `MEMORY.md:21`.
- Demo preview / outreach upgrades — plan `docs/design/demo-preview-upgrades/PLAN.md`; per-venue unfurl is a RED-LINE override gated on counsel (`MEMORY.md:23`).
- TMA (Telegram Mini App) customer entrypoint — **dark, flag-gated, unlaunched**, CSP+Dockerfile gaps (`channel-hub/TMA-VALIDATION.md:3`, `:28-73`).

### Stale / historical
- Voice control (packages/voice) — flag-dark, council-approved, engine holds zero write capability (`REGRESSION-LEDGER` 62/63). Not core to the goal.
- Token-economy / VSA / codebase-memory / model-routing arcs — **agent-harness infra, orthogonal** to the product protocol (`MEMORY.md:30-37`). The unified NON-AI protocol should not inherit these.

---

## (b) Hub architecture components already designed

**Doctrine (D1):** ONE owner hub controls the owner's own data (menu, direct orders, customers)
across their own channels (web/QR/social/messaging), with a single 0%-commission direct checkout.
Aggregator orders appear later as a **read-only** unified dashboard — money + intake stay on the
owner's checkout; **NOT** marketplace order-ingestion (breaks single-money-surface invariant)
(`DECISIONS.md:6-11`).

| Component | Design state | Evidence |
|---|---|---|
| **Owner hub** | Doctrine + ONE primitive today: channel *attribution* only (`orders.metadata.channel`, write-only). No `sales_channel` entity, no adapters, no cart-token (money-council-gated) | `DECISIONS.md:57-59` |
| **Direct 0% checkout** | Built: `kernel::decide` composes machine→actor-gate→cc1→pricing; `rebuild/crates/api/src/routes/orders/checkout.rs` (266 lines); server-priced cart + idempotency (Phase 2.2) | `G06:78-81`; `MEMORY.md:11` |
| **Order entrypoints (channels)** | Phase 2.1 = per-channel link `/s/:slug?ch=<token>` + client QR, `?ch=`→`x-channel` header, channel CRUD UI, flag `hub_channels` default OFF. **Zero artifacts built** (grep-verified); ingredients exist (`modules/channel_attribution/`, `x-channel` reader `checkout.rs:144`) | `G06:60, 68-70` |
| **Aggregator read-only view** | Phase 2.4 = dashboard tab behind `aggregator_view` flag, ONE trait `AggregatorSource::fetch_orders_readonly`, zero impls, empty-state. **Not built** | `G06:61, 71`; `DECISIONS.md:9-11` |
| **Courier management / dispatch** | Fully designed in S7 council packet (courier auth+JWT+session-liveness, assignment state machine w/ actor-gate IDOR fix, honest-dispatch no-orphan, cash-as-proof completion, settlements read, shift lifecycle). **DRAFT/NOT APPROVED**; exists in current TS product, Rust port unstarted | `rebuild-courier-s7-council/proposal.md:1-172` (esp. §1 seams `:63-102`, §4 state machine `:223-283`, §6 cash-as-proof `:333-375`) |
| **Multi-device / multi-browser sync** | **Design invariant, not built.** D2: transport-agnostic `SyncPort` (Phase 1.3, `append`/`read_since`); Postgres impl today = one transport, libp2p peer = another later; CRDT (Automerge/Yjs) deferred to Phase 2+ (only needed for concurrent offline multi-writer). MVP reconciles per-source | `DECISIONS.md:13-19`; `G06:58` |
| **Customer entrypoint: Telegram Mini App** | Dark, flag-gated (`TMA_ENABLED`/`VITE_TMA_ENABLED`), passive `?ch=telegram-tma` tag; CSP blocks the TG bridge script; Dockerfile arg missing; `initData` auth deferred | `channel-hub/TMA-VALIDATION.md:3, 19-73, 162-178` |

**Gap vs "runs locally, open-source, across multiple devices/browsers":** the hub today is a
central Fly/Supabase deployment (`MEMORY.md:53` deploy-topology), not a local owner-run binary. The
"no central server" property is reachable *for free* from the event-log design but is explicitly
**deferred, not delivered** (`DECISIONS.md:13-19`).

---

## (c) Crypto / security requirements stated

**In dowiz core (baked seams, minimal today):**
- Every mutating event carries a **content-hash** (`request_hash`, SHA-256 over codec canonical
  bytes) + a **signature slot** (NULLable `signature bytea`, stays NULL for whole MVP; signing =
  Phase 3) (`DECISIONS.md:17-19`; `G06:59, 65-67`).
- Pure side-effect-free core → WASM; `clippy.toml` bans clocks/RNG/env (purity law) — a wasm build
  already caught a real uuid-v4 entropy dep (`DECISIONS.md:22-25`; `MEMORY.md:28`).
- **PQC (Kyber/Dilithium), formal verification (Coq/Aeneas), mesh/P2P** = Phase 2+, operator
  Red-Teamed as premature for MVP (`DECISIONS.md:41-43`, D6). Per-actor **Ed25519/PQC as auth root**
  = deferred Phase 2+ (`DECISIONS.md:19`).

**Auth / RLS (production security posture, TS product):**
- JWT = **RS256 body-kid**, `alg=none` rejected, dev-kid only in non-prod
  (`docs/design-review/audit-security-2026-07-03.md:42-44`; S7 packet §3 `proposal.md:176-215`).
- Courier session-liveness = per-request DB re-read (revoked/expired/membership), no cache
  (`rebuild-courier-s7-council/proposal.md:184`, `:194-197`).
- Money = **integer minor units end-to-end** (`Lek(i64)`, no `From<f64>`); settlement math in
  Postgres SECURITY-DEFINER fn (`proposal.md:292-303`).
- **RLS is the central authz model but currently dormant**: prod runs as `dowiz_app` with BYPASSRLS,
  ~103 RLS policies inert; NOBYPASSRLS flip = latent-critical, XL, red-line, sequenced last
  (`MASTER-EXECUTION-PLAN.md:28`, G10; `MEMORY.md` B3).
- Argon2id password/token hashing; PII encryption at rest (`proposal.md:109`).

**Payments crypto:** HMAC signature-verified webhook as money source-of-truth; stablecoin
non-custodial (`payments/research.md:42-44`).

**PQ crypto (bebop2 — the actual post-quantum implementation, OUTSIDE /root/dowiz at
`/root/bebop-repo/bebop2`, audited here via `G09`):**
- From-scratch zero-dep: SHA-2/3, ChaCha20/XChaCha20-Poly1305, **Ed25519, ML-KEM-768, ML-DSA-65,
  Argon2id** (`G09:13-15`).
- **Interoperable set** {SHA, ChaCha-Poly, Ed25519, Argon2id} = byte-compatible, differentially
  testable (`G09:88-91`).
- **PQ set {ML-KEM-768, ML-DSA-65} is NOT FIPS-interoperable by construction** (coefficient-domain
  keys, CBD-sampled matrix A, 32-byte vs 48-byte challenge) — "bespoke schemes wearing FIPS names",
  cannot validate against ACVP/vectors until re-derived (`G09:92-97`, `:26`/audit item 9).
- **Live timing leaks**: Ed25519 `scalar_mul` secret-scalar branch; ML-KEM `compress`/`decompress`
  divide-by-q = exact **KyberSlash** class; ML-DSA schoolbook secret-branch (`G09:83-85`).
- Value paths (PoD, reputation, escrow) route these identities as **hybrid ML-DSA-65 ⊕ Ed25519**
  (`G09:34-42`). Proposed **4-tier value-bearing policy** (Tier 0 research → Tier 3 sole-guard needs
  external audit + ACVP interop); no primitive is Tier 3 today (`G09:454-483`).
- Two one-line ML-DSA bugs staged & specified (`w1_encode` double-highbits; `make_hint` arg)
  (`MASTER-EXECUTION-PLAN.md:25`, G08).

**Algorithms/curves named:** SHA-256 (event hash), RS256 (JWT), Argon2id (KDF), Ed25519 +
X25519 (classical), **ML-KEM-768 / ML-DSA-65** (PQ, bespoke non-FIPS), ChaCha20/XChaCha20-Poly1305 (AEAD).

---

## (d) Gaps, conflicts, contradictions vs the vendor-protocol goal

1. **Central-money invariant vs decentralization.** MVP D1 forbids marketplace order-ingestion and
   insists on a *single money surface* to protect the checkout invariant (`DECISIONS.md:10-11`). A
   fully decentralized peer/mesh protocol with multiple order entrypoints funneling money must
   reconcile with this — the seam (SyncPort, per-source reconcile) is designed but the "many
   entrypoints → one owner hub money surface" is only attribution-deep today.
2. **PQ is quarantined in the wrong repo and not production-safe.** The real PQ code is in bebop2,
   green-on-KAT but **non-interoperable + timing-leaky** (`G09`). dowiz explicitly defers PQC as
   premature (`DECISIONS.md:41-43`). Unifying "PQ-secure" requires either re-deriving bebop2 PQ to
   FIPS interop (15-25 solo-days, `G09` P2.3) or hybrid-gating it (Tier 2 max) — neither done.
3. **"Runs locally, open-source" vs current central deployment.** Prod = Fly + Supabase, BYPASSRLS,
   secrets-in-history blocker (`MEMORY.md:20, 53`; `G10`). Open-source (ADR-020 AGPLv3) is gated on a
   secret-history scrub that is an **open operator gate** and ADR-020 was never even committed
   (`MASTER-EXECUTION-PLAN.md:31`). Local-first owner binary is an unbuilt invariant.
4. **Signature slot is NULL for the whole MVP.** Event signing (the root of decentralized trust /
   PoD) is Phase 3 (`G06:59`). No per-actor identity yet — the thing a decentralized protocol needs first.
5. **Exit gate never closed + verification debt is false-green.** hub_checkout gates nothing,
   replay-parity + cause_hash are placeholders, staging Playwright vacuous (`G06:24`,
   `MASTER-EXECUTION-PLAN.md:24`). "Shipping-ready" claims contradict own memory (`G06:38-45`).
6. **NON-AI goal vs AI-heavy harness.** The repo is saturated with agent/LLM tooling (VSA,
   codebase-memory, model-routing, THE EYE, circuits — `AGENTS.md`, `MEMORY.md:30-37`). The unified
   *product protocol* is meant to be deterministic/non-AI; the harness is build-time only and must
   not leak into the shipped protocol. The Sovereign Core purity law (`clippy.toml` bans RNG/clock/env)
   is aligned with this and is the right foundation.
7. **Courier/settlement money is split S5(write)/S7(read) and not one atomic surface**
   (`proposal.md:24-26`, §5). A decentralized protocol must define where courier payout authority lives.
8. **Ethics charter constraint (must carry into any unified blueprint):** no military/warfare use,
   AI-as-commons, peace-for-everyone — non-negotiable, overrides everything (`AGENTS.md:1-13`).

---

## (e) Explicit food-vendor / no-third-party / local-first / open-source signals

- **Escape-aggregators thesis is the product's reason to exist:** *"The owner controls THEIR OWN
  data … one direct 0%-commission checkout … Honors the escape-aggregators thesis"*
  (`DECISIONS.md:7-11`). Aggregator (Wolt/Glovo) orders are read-only-dashboard only; money never
  leaves the owner's checkout (`DECISIONS.md:9-11`).
- **0% commission direct checkout** = the vendor-keeps-the-margin lever (`DECISIONS.md:8`; MVP exit
  gate clause 2, `G06:31-32`).
- **Own courier fleet, not third-party couriers:** the entire S7 courier/dispatch plane (owner
  invites couriers, owner dispatches, cash-as-proof, courier payouts) is a first-party fulfilment
  design — `rebuild-courier-s7-council/proposal.md:1-9, 104-131`. This is precisely "vendors who want
  to stop relying on third-party courier services."
- **Non-custodial payments** (funds settle direct to merchant wallet, stablecoin, no processor
  custody) — `payments/resolution.md:71-73`. The money-sovereignty complement to courier-sovereignty.
- **Local-first / decentralized:** design invariant baked as free seams (event-sourced pure WASM
  core, content-hash, signature slot, transport-agnostic SyncPort) — `DECISIONS.md:13-19`. "No central
  server reachable for FREE from an immutable deterministic replayable WASM-pure event log."
- **Open-source:** ADR-020 target = **AGPLv3 + trademark + DCO**, gated on secrets scrub + EU
  trademark (`MEMORY.md:21`; `MASTER-EXECUTION-PLAN.md:31`). Agent-facing OpenWiki already in-repo
  (`AGENTS.md:319-333`).
- **Multi-device/browser owner hub:** implied by SyncPort + "own channels (web/QR/social/messaging)"
  but sync across devices is Phase 1.3 (unbuilt) (`DECISIONS.md:16-19`, `G06:58`).

---

## Recommended unification anchors (for the parent blueprint)

1. **Keep the Sovereign Core event-log + purity law as the protocol kernel** — it already encodes
   determinism, content-hash, signature slot, transport-agnostic sync (the decentralization spine).
2. **Promote the signature slot from NULL to a real per-actor identity** as the first decentralization
   step (currently Phase 3) — this is the hinge for PoD/courier/vendor trust.
3. **Adopt bebop2 PQ only via the 4-tier hybrid policy (`G09` §4)** — never let non-interoperable,
   timing-leaky ML-KEM/ML-DSA solely guard value; re-derive to FIPS interop before Tier 3.
4. **Resolve the central-money invariant vs multi-entrypoint decentralization** — the SyncPort +
   per-source reconcile is the designed seam; specify money authority explicitly.
5. **Sequence behind the gap-blueprints reality**: exit-gate verification debt, secret-history scrub
   (open blocker), NOBYPASSRLS flip, and open-source gate must clear for a credible local-first OSS release.

*End of audit. Read-only; no code, config, or git state changed.*
