# Lens A — Vision Reconcile: bebop2 Unified Plan × dowiz Two-Half-Hub Reality — 2026-07-11

> Research-only session. Nothing in either tree modified; the only file created is this report.
> Repos as found: `/root/bebop-repo` on `feat/wire-native-core` (dirty; untracked design docs +
> uncommitted bebop2 crypto — untouched), `/root/dowiz` on `feat/paleo-dinosaur-digs` (untouched).
>
> **Verification legend.** **VERIFIED-EXEC** = executed this session (test suites run with an
> isolated `CARGO_TARGET_DIR=/tmp/bebop-verify-target`, repo untouched). **VERIFIED** = source/doc
> read this session at the cited `file:line`. **CLAIMED** = stated in a doc/memory, not
> independently re-checked. **CONTRADICTED** = two authoritative sources disagree (both cited).
>
> Executed ground truth this session: `cargo test -p bebop --lib` → **275 passed, 0 failed**
> (matches F3's claim); `cargo test -p bebop2-core` → **91 passed, 0 failed** lib tests (matches
> the blueprint's Ed25519-era count). The blueprint's "385 workspace tests" is CLAIMED (275+91=366
> verified across the two decisive crates; remainder in other workspace crates not run).

---

## 1. The bebop2 unified-plan vision — MVP and long-term, as the docs state it

The newest plan is `/root/bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`
(untracked, dated today; MEMORY.md:30 marks it "CANONICAL SYNTHESIS … supersedes v1/v2" — the v1
in `/root/dowiz/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-2026-07-11.md` is superseded).

### 1.1 The one sentence (v3 §0, quoted — VERIFIED)

> "Build an **open-source, self-hosted owner hub** (local-first, Rust/WASM, pure deterministic
> event-sourced core, **no AI in runtime**) that funnels a food-vendor's **multi-channel /
> multi-device order entrypoints** into one **0%-commission checkout** and **dispatches the
> vendor's own couriers** — with **PQ signatures + mesh/P2P seams baked now** so Phase-2+ can
> switch on without a rewrite, and **the matcher decentralized (not a single dispatch server)** so
> the protocol never re-centralizes." (v3:19-23)

### 1.2 Hard constraints C1–C10 (v3 §2 — the gate for every change; VERIFIED)

No-AI-in-runtime (C1, MANIFESTO:17-19) · pure core, no clock/RNG/env/floats/network (C2) ·
`Intent → decide → Event`, `state = fold(events)` is the law (C3) · **local-first + no central
server as a reachable-free invariant from the signed event log** (C4, MANIFESTO:74-77 + DECISIONS
D2) · integer-only money `Lek(i64)` (C5) · AGPLv3 open-source destination gated on secrets-scrub +
EUTM (C6; memory `open-source-goal-adr020-2026-07-03`) · Verified-by-Math RED+GREEN (C7) ·
**"Over-engineering is the #1 enemy — PQ/mesh/CRDT is roadmap, hard-gated behind MVP (D6)"** (C8)
· ethics charter (C9) · crypto from-scratch, zero-dep, RNG-free hot path (C10).

### 1.3 The layered stack (v3 §3, quoted — VERIFIED)

```
L0 EVENT CORE   dowiz-core: pure Rust/WASM, Intent→decide→Event, fold, integer money, idempotency.
L1 IDENTITY/PoD bebop2 PQ core: self-cert id = H(pq_pub ‖ classical_pub); Ed25519/ML-DSA-65,
                ML-KEM-768, XChaCha20-Poly1305 at-rest, Argon2id. NO issuer, NO phone-home.
L2 MATCHING     OPEN REPLICABLE matcher (pure fn, any node runs it, identical fingerprints) —
                NOT a single dispatch server.
L3 SETTLEMENT   Device-sig THRESHOLD verifier (≥k of n sigs on PoD), NOT a single oracle.
L4 ARBITRATION  Fail-closed dispute machine: OPEN→EVIDENCE→AUTO→ESCALATE→JURY→SETTLE.
L5 ACCESS/SDK   Thin client + reference alt-client (escape "open protocol, closed access").
```

### 1.4 MVP definition (v3 §5, quoted — VERIFIED)

> "### MVP — shippable NOW (dowiz Sovereign Core)
> - Owner hub: multi-channel/multi-device order entrypoints → ONE 0%-commission direct checkout.
> - `dowiz-core` pure event machine, integer money, idempotency, codec.
> - **Seams baked free:** per-event content-hash + signature slot, transport-agnostic sync port,
>   WASM-pure core + wasm32/clippy disallowed-methods gate.
> - Aggregator order-intake **banned** from MVP (breaks single-money-surface, C5).
> - Couriers: honest single-owner dispatch (`attemptHonestDispatch`) — the vendor runs their own."
> (v3:112-117)

This mirrors the dowiz GRAND-PLAN MVP exit gate verbatim (GRAND-PLAN:340-346): owner registers
channels/prints QR, receives a real 0%-commission order "through the sealed core", sees it
attributed, owns/erases the customer record — money battery + replay-parity + NOBYPASSRLS green.

### 1.5 Long-term strategy (v3 §5 "Protocol — Phase-2+ destination", quoted — VERIFIED)

> "- **Open competitive matcher market** (not a single dispatcher) — permissionless matchers,
>   force-inclusion timeout, attestation aggregation.
> - **Per-actor PQ identity** (Ed25519/ML-DSA/ML-KEM) — deferred choice, seams ready (C4/D2).
> - **Mesh/P2P transport** (libp2p vs Zenoh/Rift) + **CRDT merge** — deferred (C8/D6).
> - **Vendor-owned courier marketplace** (reassignment/auction for 50%-drop resilience) — GAP G2."
> (v3:120-124)

Boundary rule (v3 §5, the reconciliation both audits demanded): *"The owner hub is the **thin,
replaceable access layer** (L5), NOT the chokepoint… the owner's hub is *one* matcher among many;
it never becomes the only one."* (v3:127-131)

### 1.6 Build order (v3 §8, quoted — VERIFIED)

1. MVP hub (dowiz-core + owner UI) — *shippable*. 2. Crypto re-point (G11: vault/pod → bebop2,
retire scrypt). 3. wasm32 empty-import gate (G9). 4. ML-DSA NIST-bit-exact via ACVP (G10).
5. Open matcher + threshold settlement (kills DANGER #1/#3). 6. Food-vendor gaps
(storefront/marketplace/liveness/reroute G1/G2/G3/G6). 7. Dispute + PoD-contestability (G7/G8).
8. Economics (1–3%, not 0%) + thin-client access layer (G5, DANGER #2). (v3:171-178)

### 1.7 What v3 explicitly is NOT (v3 §9 — VERIFIED)

Not a business plan; not a formal-verification spec (Coq/Aeneas = Phase-3 grail); **not claiming
"0% fee = moat"** (F1 flags that as poetry — real moat = "earned local reputation graph + credible
neutrality", F1:90-96); not abandoning the MVP for the space-stack (D6 hard-gate).

### 1.8 Where the operator's NEW direction goes beyond v3 (honest finding, not a proposal)

The binding frame for this lane — collapse both half-hubs into ONE device-resident hub
(Rust/WASM kernel + bebop2 protocol + SQLite), making it possible to **drop migrations, Supabase,
Node/TS, and Fly** — is *more radical than any current doc*:

- **Supported by**: MANIFESTO §6 "Local-first data × Local execution (WASM/edge) × P2P protocol =
  decentralized reliability; each node… an autonomous decision center" (MANIFESTO:74-77,
  VERIFIED); D2's seams (content-hash + signature slot per event, transport-agnostic sync port);
  and — notably — bebop's own shipped docs: the optional self-hosted sync node advertises
  **"No Supabase. No Fly. Your keys, your machine."** with **SQLite** as its store
  (`docs/integrations/sync.md:19,31`, VERIFIED). That is the ONLY place SQLite appears in either
  repo's plans — there is **no dowiz-side SQLite/device-store design doc today** (grep-VERIFIED).
- **CONTRADICTED by standing operator decisions** (must be consciously superseded, cited so the
  other lanes see them): (a) rebuild decision 2026-07-04: "**UNCHANGED: Supabase Postgres + RLS
  FORCE (schema kept, data never migrates — code-only rebuild), Fly.io fra**" (memory
  `rebuild-decision-rust-astro-2026-07-04.md:35-38`); (b) D2/D6 defer mesh/P2P/CRDT/per-actor keys
  to Phase 2+ and C8 makes over-engineering the #1 enemy (DECISIONS:13-19,41-43); (c) the hub
  review's own park list: "**1.3 sync port, CRDT, libp2p, signing/bebop2 coupling, PQC — Phase-3
  seams; G06 §2.6 is explicit that bebop2's unaudited crypto must not guard money/identity**"
  (hub review §7.2, VERIFIED); (d) the review's headline: "**nothing that matters about 'one hub'
  is blocked on Rust**" (§3.4). A device-local hub also has no answer yet for the things Postgres
  currently provides: multi-tenant RLS, the courier/customer/owner shared views, WS fan-out
  across parties, and pg-boss durable jobs — none of these has a local-first design doc.

---

## 2. dowiz ↔ bebop concept-correspondence table

Left = the prod/staging dowiz hub concept (Node prod + Rust staging kernel). Right = the bebop
protocol primitive. Status per side; honest GAPs where no correspondence exists.

| dowiz hub concept | Evidence (dowiz) | bebop primitive | Evidence (bebop) | Correspondence — honest verdict |
|---|---|---|---|---|
| **`kernel::decide`** — the ONE door: machine → actor-gate → CC-1 → pricing/LC1; pure, caller-supplied `Ts`, emits `Vec<Event>` | `rebuild/crates/domain/src/kernel.rs:306-371` (VERIFIED; composition + `CorridorBreach` refusals read this session) | **TS bebop kernel** `decide/fold/replay` + Checker gate ("one rule, two scales": local admission == mesh envelope admission); bebop2 `kernel/` dir is *planned* ("deterministic decide/fold/replay — no clock/rng/network", `bebop2/README.md:45-46`) | `docs/features/kernel.md:7-24` (VERIFIED doc; TS impl CLAIMED) | **STRONG — same shape, independently arrived at.** The "as above, so below" Checker is the piece dowiz lacks: dowiz's `validate()` gate (kernel/validate.rs) runs before `decide` locally, but nothing re-runs the same invariant on a *received* event. That Checker IS the local-first sync admission rule the collapsed hub needs. **Caveat:** the dowiz Rust *shell* bypasses its own door — `Command::PlaceOrder` is never constructed in the api crate (grep re-VERIFIED this session: only comments at `checkout.rs:13`, `pg.rs:808`), confirming hub-review finding §3.2.1. |
| **10-status order state machine** (byte-frozen across Node `order-machine.ts:18-40` ≡ Rust `order_status.rs:57-62`) | hub review §3.1/§3.2 (VERIFIED there; kernel doc-comment re-VERIFIED) | **NONE.** bebop's `Order` is `{id, src, dst}` — no lifecycle, no statuses (`matcher.rs:34-38`, VERIFIED). The only bebop state machine is F2's *dispute* machine — design-only, 0 code (`F2:8-10`) | `matcher.rs:34-38` VERIFIED-EXEC (suite green) | **GAP, dowiz-ahead.** The protocol has routing but no order lifecycle. Any unified hub keeps dowiz's machine as L0 and treats bebop's matcher as a *function called during* CONFIRMED→IN_DELIVERY, not a replacement. |
| **`sales_channels` attribution** — 13-value write-only taxonomy; "never read by pricing, state-machine, dispatch, notifications, or authz" | hub review §1.1(2), §3.3; `modules/channel_attribution/` pilot (VERIFIED via review; registry tables Rust-only, zero Node readers) | **NONE.** No source-channel concept anywhere in `crates/bebop`. Nearest neighbours are different concepts: `fingerprint()` attributes a *dispatch decision* to an input; `pod.rs` attributes a *delivery* to a courier | `matcher.rs:100-121`, `pod.rs:8-17` VERIFIED | **GAP, dowiz-only.** Channel attribution is a storefront/analytics concern the protocol deliberately has no vocabulary for. In a collapsed hub it stays an L0/L5 concern (order metadata in the local event log), untouched by L2/L3. |
| **Courier dispatch** — owner-driven assign; `attemptHonestDispatch` (no courier ⇒ order does not advance); durable `courier_dispatch_queue` journal + sweep workers | hub review §4.2 (VERIFIED there; live in prod) | **`matcher.rs::match_orders`** — pure, deterministic, replicable; fail-closed `unmatched` surfacing; `MatcherClient` trait ⇒ no privileged endpoint | `matcher.rs:74-93,127-143,274-290` VERIFIED + VERIFIED-EXEC | **CORRESPONDENCE WITH TOPOLOGY INVERSION.** Both are honest/fail-closed (refuse > silent drop). dowiz = ONE owner dispatches own couriers (the v3 MVP); bebop = ANY node computes the same assignment (the v3 protocol). v3's boundary: the hub is "one matcher among many" (v3:129-131). **Two sub-gaps:** bebop pins each order to one `src` courier — no reassignment/auction (G2, `F4:182-184`), where dowiz's journal-redispatch loop is actually AHEAD; bebop has no liveness input (G3), where dowiz has heartbeats. |
| **deliver-v2 cash-as-proof** — `completeDelivery` primitive; `payment_outcome` enum (`paid_partial` unrepresentable); `paid_full` ⇒ append-only `courier_cash_ledger` 'hold' row; GPS `delivery_trace` | hub review §4.3 (VERIFIED there; live in prod) | **`pod.rs` proof-of-delivery** — claim `order|courier|ts|loc`, SHA512 + hybrid vault sig; misattribution/tamper/wrong-loc replay all refuse | `pod.rs:31-96,139-165` VERIFIED + VERIFIED-EXEC | **STRONG CONCEPTUAL MATCH, different trust anchors.** dowiz proof = operational/accounting (cash amount must equal total, honest CANCELLED tails) but trusts the courier's authenticated app session; bebop proof = cryptographic + pseudonymous but "signature ≠ human received box" — the admitted weakest link (`MAP:143-152`). v3 resolves: PoD is *contestable* evidence routed to L4 arbitration, never ground truth (G7). The collapsed hub wants BOTH: deliver-v2's outcome vocabulary signed as a bebop2 PoD claim. **Neither side has payout release:** dowiz Stage-21 reconciliation ADR = "NO production code" behind a NO-AUTO-DEDUCT red line; bebop G4 payout contract = 0 lines. |
| **(no dowiz counterpart)** courier scoring | Stage-21 invariant test: NO-COURIER-SCORING (hub review §4.3, VERIFIED there) | **`reputation.rs`** — deterministic trust: unknown=0.5, delivery⇒0.75+, suspension⇒0 sticky, `risk_premium=1/trust` feeds routing cost | `reputation.rs:39-93` VERIFIED + VERIFIED-EXEC | **CONTRADICTED — a real doctrine collision.** dowiz has a deliberate, ethics-gated red line *against* courier scoring; bebop makes transparent reputation THE moat (`SYSTEM-ARCHITECTURE-AUDIT.md:111`; F1 §4.1). F4 defends bebop's version as the anti-black-box answer ("trust = 0.83 ⇒ ×1.2 cost, not a hidden rank", F4:121-124). Unifying requires an explicit operator ruling: transparent deterministic reputation ≠ the opaque penalizing algorithm the red line targets — but that ruling has not been made. |
| **WS fan-out** — rooms `location:*`/`order:*`/`courier:*`, per-frame binding re-authz, PgListener bridge, ESLint-enforced guard | hub review §4.4, §5.1 (VERIFIED there) | **`zenoh.rs::Mesh`** — process-local topic pub/sub, join/leave/publish fan-out, delivery log; "the seam, not the wire protocol". Sibling: `portkey.rs` bus. TS side: `mesh.ts` `MeshTransport` port + `torrent.ts` content-addressed verified pieces | `zenoh.rs:1-99` VERIFIED + VERIFIED-EXEC; `docs/features/mesh.md:29-41` VERIFIED | **SEAM-LEVEL MATCH ONLY.** Same shape (topic fan-out to subscribed parties) but dowiz's is production-hardened WITH authz and bebop's is an in-process stand-in with NO network and NO authz. The per-frame binding re-validation discipline is exactly what a real mesh transport will need and bebop hasn't designed. Real p2p = unwritten on both sides (F3: transport = STUB). |
| **Event envelope 1.4** — `Envelope{seq, at, cause: CommandHash, event}`; `order_events(seq, at, cause_hash, payload, content_hash, signature NULL)` — signature slot dormant by D2 | `kernel.rs:215-220` re-VERIFIED this session; GRAND-PLAN 1.2/1.4:202-248 VERIFIED. **Persistence half is broken:** `cause_hash = "placeholder"` literal at `pg.rs:863-864` re-VERIFIED; dual-write partial; replay-parity a placeholder (hub review §3.2.3) | **TS `store.ts` hash-chained log** — `event[n].hash = H(payload ‖ event[n-1].hash)`, `verifyChain()` tamper-evident; bebop2 planned fixed-layout zero-serde codec (`ARCHITECTURE.md:75,88-91`); **`ledger.rs` idempotent content-addressed transfer ids** `H(from‖to‖amount‖nonce)` | `docs/features/kernel.md:26-43` VERIFIED (doc); `ledger.rs:38-49` VERIFIED-EXEC | **STRONG — this is THE junction seam.** dowiz's envelope + content-hash + NULL signature slot was designed exactly so "per-actor PQ identity" can sign canonical bytes later (GRAND-PLAN Phase-3 table: "signing is a shell envelope; kernel keeps consuming plain `Command`"). bebop2 supplies the signer (L1). Missing on the dowiz side today: honest `cause_hash`, canonical-bytes hashing (currently `serde_json::to_vec`), full-coverage dual-write — all named by the review as pre-flip red lines. |
| **Money** — `Lek(i64)`, no `From<f64>`, server-priced carts, BigInt tax, idempotency keys; crypto payments = Plisio (dark, ADR-0017) | kernel `pricing` module VERIFIED; memory `crypto-payments-build-2026-06-30` (CLAIMED) | **`ledger.rs`** — double-entry, Σ balances == 0 ("TigerBeetle law"), fail-closed overdraft/unknown-account/zero-amount, idempotent replay | `ledger.rs:79-113` VERIFIED + VERIFIED-EXEC | **COMPLEMENTARY, not overlapping.** dowiz owns *pricing* (what an order costs); bebop's ledger owns *conservation* (money can't appear/vanish). Type mismatch to reconcile: dowiz `i64` Lek vs ledger `i128` balances. Neither is settlement: "POD → funds released" has no code anywhere (F4 G4). Both cash (courier hold rows) and crypto (Plisio = a central third-party processor) remain centralized-or-manual today. |
| **Identity/auth** — JWT sessions, argon2id, courier separate auth universe, RLS tenancy; per-actor keys DEFERRED (D2) | hub review §4.1 (VERIFIED there); DECISIONS D2 VERIFIED | **`vault.rs` self-cert identity** `id = H(pq_pub ‖ classical_pub)`, no issuer, no phone-home, fail-closed tamper (old crate, RustCrypto/FIPS-reference deps per `docs/features/identity.md:5`); **bebop2 core** = the zero-dep from-scratch replacement (Ed25519 bit-exact, ML-KEM-768 KAT, ML-DSA-65 roundtrip) | `F3:166-231` VERIFIED; bebop2 91/91 VERIFIED-EXEC | **GAP on dowiz side by decision; two crypto cores on bebop side (G11 — v3 resolves: re-point vault/pod at bebop2, retire scrypt).** Sober note the review insists on: G06 §2.6 — bebop2's crypto is unaudited and "must not guard money/identity" yet; v3 agrees implicitly by gating keys on G10 (ML-DSA not NIST-bit-exact — "ACVP oracle before protocol keys minted", v3:147). Key-loss = identity-loss (no recovery design). |
| **Guard rails** — `validate()` invariant gate around `decide`; red-line hooks; corridor refusals | `kernel.rs:31-35` VERIFIED; memory `sovereign-core-mvp-handoff` (CLAIMED) | **`guard.rs`** — `io_guard` proposal envelope (L5 advisor may propose, guard decides) + `KillSwitch` ≥2/3 supermajority peer suspension (no central off-button) | `guard.rs:54-123` VERIFIED + VERIFIED-EXEC | **PARTIAL.** Both enforce "advisor proposes, kernel decides" (ADR-003 shape). The consensus KillSwitch has NO dowiz counterpart — it is a *network*-level admission concept dowiz never needed with one server, and becomes load-bearing the day two nodes exist. |
| **Sync port 1.3** — `SyncPort append/read_since`; impl #1 Postgres, #2 in-memory; "libp2p later = swap not rewrite" | GRAND-PLAN 1.3:222-233 VERIFIED (design; parked per hub review §7.2) | `zenoh.rs`/`portkey.rs` seams; TS `MeshTransport` port; `Transport` trait in `matcher.rs:149-153` | VERIFIED | **SAME IDEA, BOTH UNPROVEN over a real network.** Impl #3 (a peer) exists nowhere. This is the single seam the collapse thesis stands on. |

---

## 3. Centralization audit, reconciled (F3 + MAP × dowiz prod reality)

F3's method: grep for hidden endpoints — "**zero hardcoded operator endpoints** in the executable
path… The real centralization surface is in the *design contract*, not the code" (F3:110-114).
F3's split verdict: "~70% real architecture, ~30% poetry" (F3:303). Below, every place a central
server still lurks, what bebop's docs propose to remove it, and what dowiz-prod adds to the list.

| # | Central point | bebop status (F3/MAP, verified) | Proposed removal (docs) | dowiz-prod reality today (the collapse target) |
|---|---|---|---|---|
| D#1 | **Matcher / dispatch sequencer** — "whoever orders courier↔order controls the network… even if settlement is on-chain" (MAP:75-89) | 🟢 **Killed in code**: pure `match_orders`, `matcher_is_replicable_no_hidden_server` fingerprint agreement, `MatcherClient` trait not a hostname (`matcher.rs:74,274-290`; VERIFIED-EXEC) | Already done at algorithm level; deployment risk devolves to D#2 | The Node API process on Fly IS the single dispatcher (`attemptHonestDispatch` + sweep workers in the API web process). MVP-accepted per v3 boundary; becomes DoorDash-with-extra-steps only if it stays the ONLY matcher. |
| D#2 | **SDK / bootstrap / access layer** — "open protocol, closed access" (MAP:91-102) | 🔴 **THE genuine unresolved risk** — "admitted, not de-risked in code" (F3:148-164, 316). "If our hosted backend is the only way in, we re-centralized at the access layer" | "Thin client over the open `MatcherClient` trait + ship a reference alternative client" (F3:330-331; v3 L5, build-order #8). Zero code | This is *exactly what the dowiz hub is today*: storefront, checkout, owner console, courier app all reachable only through the operator's Fly-hosted backend. The operator's collapse direction (hub runs ON the vendor's device) is the strongest possible D#2 kill — stronger than the docs' own thin-client escape. |
| D#3 | **Settlement oracle** — who attests "delivered" to release money (MAP:104-112) | 🟠 Design-mitigated ("PoD = threshold of device signatures… the crypto proof IS the oracle, not a server"), **code absent — 0 executable lines** of escrow/DLT (F3:131-146). "This is the single most important follow-up gap" | Device-sig **threshold verifier** (≥k of n courier/owner/customer sigs), never a single service (v3 L3, G4) | dowiz: cash settlement = manual owner approve/pay over `courier_cash_ledger` holds; Stage-21 reconciliation is deliberately NO-CODE (ethics gate). Crypto = Plisio, a central third-party (memory `crypto-payments-build-2026-06-30`, dark). Payments stay a central dependency in ANY topology until L3 exists. |
| D#4 | **Identity root-of-trust** (MAP:114-123) | 🟢 De-centralized in code: self-cert `vault.rs`, no issuer, no directory, no phone-home (F3:166-231). ⚠️ Bot-proof/Sybil open (stake/PoP deferred, soft signal only); recoverable ❌ (key-loss = identity-loss) | Self-cert id from bebop2 primitives; Sybil bounded by reputation, later stake bonds (MAP:121-123) | dowiz identity = central Postgres session store + JWT + argon2id (central by construction, and fine for MVP). D2 keeps per-actor keys deferred; the signature slot (1.4) is the pre-baked seam. |
| D#5 | **Liquidity/sequencer** | = D#1 (MAP:125-128) | same | same |
| — | **Menu / storefront hosting** | ❌ **POETRY**: "IPFS menu cache — 0 refs in src/" (F3:314); no menu concept at all (G1, `F4:180-181`) | v3 G1: build `Order={id,src,dst,items,price}` + menu module; IPFS remains unspecified | Menus live in the hub's Postgres. Local-first: menu becomes device-local state that must *distribute* (customers must read it without the vendor's device being a 24/7 server) — the one problem the docs have only an IPFS word for. |
| — | **Dispute oracle** | Design-only F2 fail-closed machine (any timeout/ambiguity ⇒ escrow HOLD + refund claimant, never silent approval, F2:35); "L5 as judge" flagged reification unless routed through the Neuro-Symbolic Gate (F2:100-111) | Build F2's machine OR integrate external UMA/Kleros (G8) — note: UMA/Kleros are external services, a *chosen* centralization | dowiz: the support desk is the owner + operator. No dispute machinery either. |
| — | **PoD physical anchor** | 🔴 Admitted weakest link: "signature ≠ human received box… NO trustless production anchor" (MAP:143-152) | Design for contestability, route to L4; never treat sig as ground truth (G7) | deliver-v2's cash-equality check + GPS trace is the *operational* analog of the missing anchor — evidence, equally contestable. |
| — | **Transport bootstrap / rendezvous** | Not yet addressed: zenoh/portkey are in-process; a real mesh needs peer discovery, and whoever runs the bootstrap node re-centralizes. F3 counts transport as STUB (F3:315) | Unwritten (libp2p vs Zenoh/Rift deferred, v3:123) | dowiz equivalent: Fly + Supabase are the rendezvous. Also note prod's *platform* dependencies that survive any hub move: Telegram (owner notifications), ORS (routing polylines), web-push endpoints — each an external central service to inventory in the collapse plan. |
| — | **Kill-switch** | 🟢 consensus ≥2/3 of known nodes, one node cannot kill another (`guard.rs:107-113`, VERIFIED-EXEC) | already code | dowiz: feature flags + Fly = operator off-button (appropriate for one operator; a protocol needs guard.rs's shape). |

**Reconciled headline** (F3:319-327, VERIFIED): the engine has no hidden operator node; the one
real exposure is D#2 — and dowiz-prod *is* a D#2 instance by construction. The centralization
risk "lives in what gets bolted on later" (settlement oracle, bootstrap) "not in what exists
today". The operator's device-resident-hub direction attacks D#2 head-on but inherits D#3 (no
settlement), the menu-distribution hole, and the transport-bootstrap question unsolved.

---

## 4. The honest delta — exists (tested Rust) vs design-only vs aspiration, for the local-first hub

### 4.1 EXISTS as tested code today

| Asset | Where | Proof |
|---|---|---|
| Protocol primitives: matcher, PoD, reputation, ledger Σ=0, consensus kill-switch, zenoh/portkey seams, content-addressed registry, hybrid A*/CH routing (`cost_estimate`), graceful degradation (`reconnect`) | `/root/bebop-repo/crates/bebop/src/` | **VERIFIED-EXEC: 275/275** `cargo test -p bebop --lib` this session |
| bebop2 PQ crypto core: Ed25519 RFC 8032 §7.1 bit-exact, ML-KEM-768 FIPS 203 KAT, ML-DSA-65 roundtrip+tamper (NOT NIST-bit-exact — G10), Argon2id RFC 9106 KAT, XChaCha20-Poly1305, SHA-512/SHA3, ChaCha20 CSPRNG — zero-dep, RNG-free hot path; + math/spectral kernel | `/root/bebop-repo/bebop2/core/src/` (uncommitted, PRECIOUS) | **VERIFIED-EXEC: 91/91** `cargo test -p bebop2-core` this session. Roadmap's "ALL STUBS" (`bebop2-roadmap-2026-07-10.md:9-11`) is **CONTRADICTED/STALE** — plan-audit-bebop:39-44 + execution confirm implemented |
| dowiz-core kernel: `decide` composing machine→actor-gate→cc1→pricing, exhaustive `fold`, `replay`, `Envelope{seq,at,cause}`, `validate()` layer, integer `Lek(i64)`, idempotency decision, wasm32+clippy purity gates (the gate caught a real uuid-v4 entropy dep — memory `sovereign-core-phase-zero-2026-07-05`) | `/root/dowiz/rebuild/crates/domain/` | **VERIFIED** by code read (`kernel.rs:306-371,215-220`); test counts CLAIMED (118 lib per skill; 132–137 per handoff) |
| dowiz prod Node hub: one canonical intake, transactional integer-money pipeline, courier invite→shift→assign→deliver-v2 cash-as-proof loop, per-frame WS authz, notification fan-out with audit ledger | `/root/dowiz/apps/` | **CLAIMED-VERIFIED** by hub review (fresh, same-day, file:line-cited); spot findings re-verified here (PlaceOrder absence, cause_hash placeholder) |
| TS bebop local-first skeleton: kernel+Checker, hash-chained ContentStore, VSA memory, torrent/mesh ports, optional self-hosted **SQLite** sync node ("No Supabase. No Fly.") | `/root/bebop-repo/src/`, `docs/features/*`, `docs/integrations/sync.md` | **VERIFIED** docs read; impl CLAIMED (tests named in docs, not run this session) |

### 4.2 DESIGN-ONLY (written shape, zero or placeholder code)

- **Settlement / escrow / payout contract** — "0 lines" (plan-audit-bebop:51; F3:131-146). The
  device-sig threshold verifier (G4) is the highest-stakes unbuilt piece on both sides.
- **Dispute/arbitration state machine** — F2's table + fail-closed law + Schelling-jury math:
  "applicable theory, absent code" (F2:67).
- **Open matcher *market*** — the pure fn exists; permissionless deployment, force-inclusion
  timeout, attestation aggregation exist only in strategy briefs (plan-audit-memory §4 OPEN).
- **Thin client / reference alt-client** (D#2 escape) — specified, not coded (F3:148-164).
- **Courier marketplace / reassignment / auction (G2), node liveness (G3), mid-route reroute
  (G6), storefront/menu module (G1)** — v3 gap ledger, all unbuilt.
- **Reputation merge on partition rejoin** — local HashMap only; "no merge/sync protocol in code"
  (F3:281-284).
- **dowiz event log honesty set** — real `cause_hash` (today the literal `"placeholder"`,
  `pg.rs:863-864` re-VERIFIED), canonical-bytes `content_hash`, full dual-write coverage, real
  replay-parity, `hub_checkout` gate that gates, and routing the Rust checkout through
  `Command::PlaceOrder → decide` (hub review §3.2, findings 1-5 — the review's red-line amendment).
- **Sync port impl #3 (real peer transport), CRDT merge, per-actor key wiring** — D2-deferred
  seams with in-memory stand-ins only.
- **wasm32 empty-import gate** — FAILS (~94 errors); "the only honest 'runs as machine code / no
  reachable clock/RNG/socket' proof… aspirational until it compiles" (plan-audit-bebop:77-80).
  v3 marks G9 + G10 as IN-FLIGHT via parallel agents (v3:196-197) — CLAIMED, not verified here.
- **SQLite-on-device hub store** — no design doc exists anywhere; sole precedent is bebop's
  optional sync node `BEBOP_DB` (sync.md:31).

### 4.3 ASPIRATION / poetry (flagged as such by the corpus itself)

- **IPFS menu cache** — 0 refs in src (F3:314). **DLT settlement** — doc-prose only (F3:313).
- **"0% fee = atomic bomb"** — "(c) poetry… fee is a subsidy tactic; liquidity is the war"
  (F1:21,44); v3 adopts 1–3% + value-added sinks (G5).
- **"L5 neuro-symbolic as judge"** — reification unless advisor-proposes/kernel-decides (F2:104).
- **Energy-aware consent compute (the grail)** — Phase 3, "the core never learns what a battery
  is" (D3). **Unikernel fortress tier** — packaging prose. **Coq/Aeneas** — Phase-3 grail (v3 §9).
- **The full collapse itself** ("drop migrations, Supabase, Node/TS, Fly") — today an operator
  intent with strong doctrinal anchors (MANIFESTO §6; D2 seams; bebop's "No Supabase. No Fly.")
  but **no plan document, and standing decisions it must consciously supersede** (§1.8 above).
  The freshest full-system review's counter-position should be quoted alongside it: "nothing
  about 'one hub, many sources, own couriers' is blocked on the Rust rewrite" (hub review §0) —
  the two-half-hub problem is real, but its named root causes are verification theater and a
  bypassed door (§3.2), not the existence of Node/Postgres per se.

### 4.4 Cross-repo housekeeping facts the other lanes need

- Two crypto cores (old `crates/bebop` vault: RustCrypto + scrypt per `identity.md:5,15` vs
  zero-dep bebop2) — v3 G11 resolves: re-point vault/pod at bebop2, retire scrypt (v3:100-102).
- `/root/dowiz/rebuild/crates/bebop` exists and is **pre-existingly broken** (missing
  `pricing::PriceInputs` export — dowiz-operating-system SKILL.md pitfalls, CLAIMED) — a stale
  cross-repo seam; the living-memory rule says bebop-referencing files belong in
  `/root/bebop-repo` (dowiz-living-memory SKILL.md; MEMORY.md:7).
- Canonical memory = `/root/.claude/projects/-root-dowiz/memory/` (dowiz-living-memory SKILL.md);
  the "202/202" bebop test count in it is stale — ground truth this session is 275.
- Open-source flip remains hard-gated: secrets-history scrub + EUTM + explicit human go (memory
  `open-source-goal-adr020-2026-07-03`) — a local-first self-hosted hub is the ADR-020 self-host
  story's strongest form, but the gates are unchanged.

---

*Prepared 2026-07-11, lens A of 4. Sources: v3 blueprint + plan-audit-bebop + plan-audit-memory +
fable F1–F4 + delivery-protocol docs + bebop2 ARCHITECTURE/README + docs/features + crates/bebop
source (suite executed) + bebop2-core (suite executed) + dowiz hub-architecture-review +
MANIFESTO/DECISIONS/GRAND-PLAN/REBUILD-MAP + kernel.rs/checkout.rs/pg.rs (re-verified) + living
memory corpus + dowiz-operating-system / dowiz-living-memory skills. Both repos left as found;
build artifacts went to /tmp only.*
