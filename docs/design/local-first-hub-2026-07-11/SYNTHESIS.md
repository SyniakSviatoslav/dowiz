# Local-First Decentralized Hub — Synthesis — 2026-07-11

> Synthesis of four parallel research lenses (all read-only, both repos left as found) answering the
> operator's directive: collapse dowiz's **two half-hubs into one local-first, decentralized hub**
> running maximally on the devices — kernel (Rust/WASM) + bebop2 protocol + SQLite — with **no single
> server that processes everything**, and the explicit intent to eventually **drop migrations,
> Supabase, Node/TS, and Fly**. Lens reports in this directory: `A-vision-reconcile.md`,
> `B-data-sync.md`, `C-runtime-transport-identity.md`, `D-transition-blueprint.md`. Every load-bearing
> claim is cited + graded in its lens. This doc reconciles them into one verdict.

---

## 1. The one-paragraph answer

**The vision is real, coherent, and mostly unbuilt — and it is the right destination, but not the
right next move.** bebop2's own v3 unified blueprint already describes exactly this (device-resident
hub, "No Supabase. No Fly.", SQLite sync node), and the hard architecture questions have honest
answers: SQLite-per-device is viable, the data cleanly splits into CRDT-safe vs single-writer, and
identity/authz maps onto signed capabilities. But three things are load-bearing and true: (1) "no
central server" honestly means **relay-assisted P2P with an irreducible floor** — push gateways
(APNs/FCM), NAT relays, and one always-on replica per venue cannot be dropped; (2) the money/order
core is **non-commutative** — it needs a single-writer sequencer (the vendor's device), not CRDT
magic; (3) starting the rewrite now, before a single real order, is **serial-pivot #5** against the
project's own deepest risk. The synthesis verdict: adopt local-first as the **committed
destination**, build it as a **reversible strangler ladder** whose early rungs are the operator's own
Wave-0/1 pragmatism, and gate every rung above "one honest door" on **venue validation**.

> **Reconciliation note (2026-07-11 ~20:00, added post-synthesis).** A concurrent session's deep-read
> (`01-bebop2-protocol-state.md`, not authored by this program) + live re-verification caught bebop-repo
> **actively merging while the lenses ran**. Fresher HEAD `57b1c9a` supersedes three points below:
> (1) the previously-uncommitted Argon2id/ML-DSA crypto is now **COMMITTED and clean** — the "protect
> the crown jewels" action (G08-0.2 / P0) is effectively DONE; (2) the **wasm32 no_std build now PASSES**
> with a byte-verified empty import section (G9 closed) — so the P4 gate's "wasm32 green" precondition is
> already met, not future; (3) the workspace suite is now **388/388**, not 366. UNCHANGED: every G09
> crypto constraint still holds at the new HEAD (ML-KEM coefficient-domain / non-FIPS, ML-DSA CBD-sampled
> A + 32-byte challenge, Ed25519 secret-dependent `scalar_mul` branch, no external audit/Wycheproof), so
> §3-Q4's hybrid-only / audited-classical-half rule is untouched. Also confirmed by the deep-read:
> `zenoh.rs` is an **in-process stand-in — zenoh is NOT a dependency**; there is still zero network code,
> zero SQLite/storage, and no runnable node binary. That sharpens §2's "mostly unbuilt": the crypto/
> identity/matcher LIBRARIES are real and now committed; the NODE (wire + storage + runtime) is 0 lines.

## 2. What is actually TRUE today (executed / verified, not claimed)

- **bebop + bebop2 are not stubs.** Lens A executed the suites: `bebop` 275/275, `bebop2-core`
  91/91 pass (**now 388/388 workspace at HEAD `57b1c9a` per the reconciliation note above**). The
  roadmap's "ALL STUBS" line is stale/CONTRADICTED. The PQ crypto core, the protocol primitives
  (`matcher.rs`, `pod.rs`, `reputation.rs`, `ledger.rs`), and the dowiz `kernel::decide` domain logic
  are real, tested Rust. **Caveat (concurrent deep-read):** `zenoh.rs` is an in-process stand-in, not
  a real transport — the node runtime (wire + storage) remains unbuilt.
- **The Rust hub bypasses its own law** — independently re-verified by lens A: `Command::PlaceOrder`
  is never constructed in dowiz's api crate (comments only); `cause_hash = "placeholder"` at
  `pg.rs:863`. This is the hub review's finding #1, confirmed. **Fixing this — making `kernel::decide`
  the one real door — is Phase 1 under EVERY future** (local-first, finish-cutover, or stay-Node),
  so it is the highest-agreement action across all programs.

## 3. The architecture, reconciled (the four hard questions answered)

**Q1 — Datastore (lens B):** rusqlite native on the vendor device (recommended); official
SQLite-WASM+OPFS in browsers but **Safari's 7-day eviction makes browser nodes caches, never
authorities**; cr-sqlite rejected (dormant since Jan 2024); Turso offline-sync still beta. Postgres
RLS → physical per-venue partition + capability tokens (its RLS is largely dormant today anyway —
`dowiz_app` BYPASSRLS). The invariants any SQLite swap MUST preserve: **Lek(i64) integer money, the
byte-frozen 10-status machine, decide/fold, payment monotonicity.**

**Q2 — The crux, sync consistency (lens B, CALM/CALM-theorem-anchored):** the data splits cleanly and
this split IS the architecture —
- **CRDT-safe (commutative):** menus, presence, telemetry, chat → Automerge 3 (alive) over gossip.
- **Single-writer (non-monotone):** orders, dispatch, reservations → a signed per-order event log
  with **the vendor key as sequencer**; money = co-signed double-entry through the conservation
  ledger. bebop's own `ledger` ("in-process only") and `matcher` (deterministic fn, sequencing
  unsolved) already assume exactly this. **You cannot CRDT-merge "order accepted by courier A" vs
  "by courier B" — that is why a sequencer is unavoidable.**

**Q3 — Runtime + the honest floor (lens C):** only the **vendor's own device can be an always-on
node**. Decisive constraint: **iOS runs no background network node, ever** (suspended = no sockets;
BGTask ~30s); **Android only with a 6h-capped foreground service + battery fights**. Phones are
**intermittent, push-woken signers/verifiers, not peers.** Installed PWA dodges Safari's 7-day cap;
mobile pattern = Rust-core-as-native-lib via UniFFI (Signal/Bitwarden proven). Transport: **iroh 1.0**
(June 2026; Ed25519 NodeId dialing aligns with bebop's self-cert identity; self-hostable stateless
relay) behind bebop's `MeshTransport` port, + Zenoh multicast for same-premises LAN. **"Decentralized"
honestly = relay-assisted P2P; the floor is one dumb untrusted encrypted-forward relay** — a far
smaller trusted surface than Fly, but not zero.

**Q4 — Identity/authz (lens C):** self-cert PQ-hybrid keypairs issue **signed capability tokens**
replacing RS256 JWT + RLS; the courier per-frame WS re-authz (ADR-0013) maps ~1:1 onto per-frame
signature+`exp` and becomes *more* robust offline. **Hard gate (G09):** bebop2's hand-rolled PQ is
non-FIPS-interop with KyberSlash-class + Ed25519 timing hotspots — **may guard value only as the PQ
half of a hybrid, never alone.**

## 4. The irreducible-server floor (what "no central server" really means)

All lenses converge here — be honest about it up front:
- **Push delivery: APNs/FCM** — the one unavoidable Big-Tech dependency (UnifiedPush helps Android
  only). Order-while-courier-offline REQUIRES push-wake; this is also the hub review's #1 courier gap.
- **NAT relay** — ~90% of connections go direct, the rest need a relay (CGNAT on cellular).
- **≥1 always-on replica per venue** — the "vendor's phone dies at dinner rush" answer; a cheap
  always-on node (mini-PC in the venue, or a thin hosted replica) is required for durability.
- **An HTTPS endpoint per venue** for non-cash payments; **cash is the only truly serverless rail**
  (which fits the Albanian cash-first reality from the EV market lens).
- Net: the trusted/central surface shrinks from "Fly runs everything" to "dumb relays that carry, not
  decide" — a real and meaningful decentralization, but not zero-infra.

> **Independent corroboration (concurrent L-series, 2026-07-11 ~20:00).** A parallel session's
> architecture lens (`02-local-first-architecture.md`, not this program's work) reached the SAME
> topology — per-vendor sovereign node + one dumb stateless relay — scoring it against sovereign-only
> and full-P2P-mesh, and forcing the choice on the same physical fact (a one-shot mobile-web customer
> behind cellular CGNAT physically cannot peer with a CGNAT'd vendor node; browsers can't open raw
> sockets, Safari refuses `serverCertificateHashes`, hole-punching tops ~70-80%). Two programs
> converging independently is the confidence signal. It adds three concrete facts worth folding in:
> (1) **the relay costs ~€4/month** (Hetzner VPS SNI/TCP passthrough, or free Tailscale Funnel) with
> **TLS terminating on the vendor node** (Cloudflare Tunnel rejected — it terminates TLS at the edge);
> order traffic ≈1.5 GB/mo — so "no single server" is literally a €4 line item, not Fly. (2) **COD is
> the superpower**: because no digital money moves, the system keeps double-entry books of *obligations*
> settled by counter-signed custody hand-offs — collapsing the unsolvable offline-double-spend problem
> into signed bookkeeping (fits the Albanian cash-first reality). (3) **Fiscalization (Law 87/2019) is
> the one legally-unavoidable central endpoint**, but it is offline-first by law (48h grace) and lives
> node-only — the hub is never the fiscal system of record. All three strengthen §4's floor without
> changing the verdict.

## 5. The transition — one reversible ladder (lens D, refined by A/B/C)

| Rung | What | Infra dropped | Gate to ENTER | Reversible? |
|---|---|---|---|---|
| **P0** | Wave-0/1 + 3 fixes (`/courier-invite`, `c.name`, alerts) + QR/venue #1 + protect bebop2 WIP | none | now | n/a |
| **P1** | Fix `kernel::decide` bypass = ONE real door (G06 Opt-B) | none | P0 | yes |
| **P2** | SQLite read-replica dual-run beside Postgres | none yet | Wave-1 merged AND P1 landed AND **G11 GREEN (first real order)** | yes (drop replica) |
| **P3** | Menu/presence device-authoritative (CRDT-safe subset) | some read load | ≥20 orders/wk×4wk OR 3 venues | yes (repoint) |
| **P4** | Money single-writer on vendor device + store-and-forward relay | Postgres write-path for orders | ≥3 paying venues + **bebop2 Tier-2 hybrid** + wasm32 gate green | per-surface |
| **P5** | Drop Supabase / Node / Fly | the substrate | all above stable + soak | hard cutover |

Total ~30–45 sessions (~2–3× the remaining G04 cutover). Each rung is independently valuable, has a
VbM RED case, and is per-surface reversible (strangler).

## 6. The verdict (lens D, three-way EV — take the real position)

- **Local-first rewrite:** MAX-EV **as a destination**, but only via the ladder with venue-validation
  gating. Full-start-now = serial pivot #5 (G07) against zero real orders (§7.8) — the exact failure
  pattern that stalled everything before.
- **Finish the G04 Rust cutover:** **dominated** — its server tail is precisely the architecture P4/P5
  would dismantle; keep only the kernel-honesty slice (P1), discard the rest.
- **Stay Node-prod forever:** best paper EV, but ~0 execution probability given revealed operator
  attention (the corpus shows attention has already left Node).

**Start trigger:** P2 opens on `Wave-1 merged AND P1 landed AND G11 GREEN (first real non-operator
order)`. Until then, P0+P1 only — and P0+P1 are things you want under every future anyway, so they
carry no pivot risk.

## 7. Contradictions the collapse must CONSCIOUSLY supersede (lens A)

This arc reverses prior operator-signed decisions — do it with eyes open, not by drift:
- `rebuild-decision-2026-07-04`: "Supabase Postgres UNCHANGED, Fly.io" — **directly reversed.**
- `G06`: "bebop2's unaudited crypto must NOT guard money/identity" — the local-first identity puts it
  exactly there → enforced by the **hybrid-only / Tier-2** rule (§3 Q4) + the G09 assurance ladder.
- The hub review's "nothing is blocked on the Rust rewrite" — still true for P0/P1; P2+ changes that.
- **A genuine doctrine collision needing an operator ruling:** bebop's `reputation.rs` (courier
  scoring / KillSwitch) vs dowiz's standing **NO-COURIER-SCORING red line**. These cannot both hold —
  the operator must rule before any protocol-reputation code touches dowiz couriers.
- Settlement/payout is **0 lines everywhere** (dowiz deliver-v2 AND bebop pod.rs both prove delivery,
  neither pays out); menu-over-IPFS and DLT settlement are still "poetry," not design.

## 8. What this synthesis asks the operator to decide

1. **Ratify the destination + the gate:** local-first is the committed end-state, but rungs above P1
   are gated on `G11 GREEN`. (Yes = the ladder; No = which alternative.)
2. **Rule the reputation collision** (courier scoring red line vs protocol reputation).
3. **Accept the irreducible floor** (relays + push + one always-on replica-per-venue is not "zero
   server" — is that acceptable as "decentralized"?).
4. **Bind bebop2 to hybrid-only** for any value-bearing identity until the G09 external-audit ladder
   clears (non-negotiable given the KyberSlash-class findings).
5. **Sequence vs the EV program:** P0/P1 run alongside the July validation; the substrate rewrite
   (P2+) does not start until a real venue is transacting — the two programs share the same
   `G11 GREEN` trigger.

## 9. Session addenda (2026-07-11 evening — operator deep-dives)

Follow-up research after operator rulings (local-first ratified, no courier scoring, COD mandatory,
anonymity a stated value). Companion docs:

- **Transport / relay** (`docs/research/2026-07-11-relay-hetzner-tailscale-mesh.md`): **Hetzner CX23
  primary** (~**€6/mo** — the earlier €4.15 is stale, Hetzner raised prices 2026-06-15; 20 TB incl.
  vs ~1.5 GB/mo actual), nginx `stream` SNI-passthrough, own domain, TLS on node. **Correction:**
  Tailscale Funnel does NOT terminate TLS (both keep "relay never decrypts"); the real reason to
  prefer Hetzner is the **domain** (Funnel only serves `ts.net` — a printed QR would hand Tailscale
  the front door). Tailscale kept for dev/ops-tailnet/break-glass (courier+admin legs only). **Mesh
  layered, not either/or:** iroh for vendor↔courier (self-host relay on the same box), plain WSS for
  the customer leg, Zenoh/LAN for same-premises. libp2p/IPFS rejected ("poetry"); i2p rejected on
  performance + a Dec-2025 deanonymization paper (NOT on the discredited "attribution forbids
  anonymity" premise). Whole stack ≈ **€6/mo**, zero PCI surface (COD).
- **Anonymity** (`03-anonymity-architecture.md`): **4 of 5 anonymity layers are already FREE** under
  local-first + COD. VERIFIED that Albanian Law 87/2019 records **only the vendor's sale, no buyer
  field** — so the buyer is anonymous by default and the earlier "fiscalization vs anonymity" conflict
  was wrong. Design: **default-anonymous data layer always** (per-order PII envelope, only a hash in
  the signed log, **crypto-shred** the key after the dispute window — EDPB 02/2025-blessed erasure;
  `pod.rs` already gives pseudonymous proof-of-delivery). **Network-metadata layer by latency-tolerance:**
  order placement can go over a **vendor-node Tor .onion mirror** (~1–1.5 s, no new cost, adds
  censorship resistance re: Albania's TikTok ban); real-time courier↔vendor stays on iroh. **Honest
  floors:** a normal mobile browser can't reach .onion (no production Tor-in-WASM 2026) so the default
  one-shot web customer gets every layer *except* network-metadata anonymity; the delivery ADDRESS
  must reach the courier; and Albanian mandatory SIM registration + telecom retention bind
  anonymity-from-the-state regardless of design. Nym mixnet / anonymous credentials (KVAC to drop the
  phone) / on-device ZK are earn-it/future.
- **Anonymity — multichannel revision** (`04-anonymity-mesh-messenger-revision.md`, supersedes 03's
  customer-leg limit + SIM floor): operator ruling — **multichannel stays, NO dedicated app**. The
  browser/.onion limit and the customer SIM floor were properties of the *browser sandbox*, not
  physics; a multichannel hub accepts channels the customer already has (Tor Browser→`.onion`,
  no-phone messenger), so they dissolve for anyone who *picks* an anonymous channel — with no dowiz
  app ever. Anonymity splits into (a) an always-on, channel-independent **hub guarantee** (no profile,
  per-order PII envelope + crypto-shred, vendor-node data, cash) and (b) **per-channel network
  anonymity = the customer's honest pass-through choice** (dowiz labels each funnel's privacy level
  but cannot enforce it). The "private tier" is an anonymous **channel-adapter** — the vendor-node
  `.onion` web mirror or a no-phone messenger contact — never an install. Surviving floors: the
  **iOS push/background wall** (reliable wake ⇒ APNs/FCM ⇒ Big-Tech de-anon — the sharpest), the
  **courier's registered SIM**, physical/behavioral correlation, and label≠enforce.
- **Launch legality** (`docs/research/2026-07-11-launch-without-lawyer-albania.md`): **order #1 is
  legal now, no entity, no lawyer** — venue fiscalizes on its own POS, dowiz relays only, cash pilot
  free. The **no-courier-scoring ruling is legally load-bearing** (keeps couriers as venue staff →
  dowiz dodges platform-work law). Hard lawyer triggers named: taking payments, employing couriers,
  scale contracts, equity. Do-now: EU-region pin, template ToS/notice, accountant (not lawyer) for
  fiscal, entity only to *charge*.
- **bebop field-sim** (`docs/design/bebop-field-sim-2026-07-11/SYNTHESIS.md`): **park for delivery** —
  static heat-kernel/FFT/VSA correct, but iterative diffusion has a VERIFIED sign bug the green tests
  mask, the physics is orphaned, benchmarks are dishonest, and `reputation.rs`'s courier-scoring path
  is red-line-blocked. Not on the hub critical path.

*Synthesis produced 2026-07-11 from four parallel read-only lenses + the living-memory corpus, with
evening addenda from five follow-up deep-dives. Companion program (max-EV adoption,
`docs/research/2026-07-11-MAX-EV-SYNTHESIS.md`) synthesized separately; all of it shares one trigger
— the first real order.*
