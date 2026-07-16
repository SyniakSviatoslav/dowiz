# R1-D — Product-on-Protocol & Delivery Features: Gap Analysis (2026-07-16)

> Cluster D of the sovereign-roadmap R1 fan-out. Anchors owned: **D4, F41–F50, E1, E41–E45**
> (ARCHITECTURE.md §6 F-PRODUCT/DELIVERY; STRATEGIC-VECTORS E41-45; §0 M-series as substrate).
> Canon: `docs/design/ARCHITECTURE.md` (mesh = FOUNDATION; dowiz = delivery service ON TOP of the
> protocol) + `docs/design/STRATEGIC-VECTORS-LOCKED-2026-07-16.md`.
> Grounded in direct code reads (dowiz kernel/engine/web, bebop2, legacy git history) — every
> claim below carries file:line or a named commit. Reuses (does NOT re-derive)
> `GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md` and
> `SYNTHESIZED-BLUEPRINT-PLAN-2026-07-16.md` §2-B/§3.

---

## 0. The single load-bearing ground-truth finding (affects every anchor)

**The centralized product that F41–F50 assume is being "re-plumbed" no longer exists as live
source.** Commit `79ef316f6` / `db766de47` (2026-07-13, "remove legacy JS/TS thin-layer, kernel is
now sole source of truth") deleted `apps/web` (Storefront/Admin/Courier SPA), `packages/ui`
(including i18n), `packages/domain`, `packages/shared-types`; commit `fce5738b0` (branch
`feat/remove-legacy-thin-layer` lineage) quarantined `apps/api`, `apps/worker`, `packages/db`,
`fly.toml` to `attic/`. At current HEAD (`feat/kernel-fsm-graph-analysis`), `git ls-files 'apps/*'`
returns **0 files**; `/root/dowiz/apps/web/` holds only stale `dist/` + `node_modules/`;
`origin/main` is likewise post-deletion.

Consequences for this cluster:

1. "Re-plumbing" is really **rebuild-on-kernel-and-mesh while preserving the feature inventory**.
   The inventory sources are: `DeliveryOS-As-Built-Summary-v1.md` (shipped features, repo root),
   `PRODUCT.md`, `docs/design/dowiz-interfaces/DOWIZ-INTERFACES-PLAN.md` (26-page inventory +
   "no shipped feature is lost" master checklist), and git history
   (`git show db766de47~1:<path>` recovers any legacy file, e.g. the full i18n catalog).
2. Every "today's product assumes centralized" statement below cites the **historical** paths —
   they are the behavioral oracle, not editable code.
3. The replacement substrate already exists and is green: dowiz kernel (167+ tests:
   `order_machine.rs`, `money.rs`, `geo.rs`, `wasm.rs` exports), engine
   (`field_frame.rs` deterministic physics), `web/` kernel-driven shell (20 VbM tests), and
   bebop2 mesh primitives (proto-cap, pq_kem, matcher, iroh transport, delivery-domain, dod gate).

**What the legacy centralized product was** (per `DeliveryOS-As-Built-Summary-v1.md:24-59`):
Albanian-market multi-tenant white-label restaurant delivery; 3 roles (Client/Owner/Courier); 77%
cash; Node22/Fastify5 monolith (~60 route plugins, `apps/api/src/server.ts:585-634` historical),
single Supabase Postgres (`packages/db/src/index.ts:18-51` historical), pg-boss queue, Postgres
NOTIFY/LISTEN bus, in-memory WebSocket rooms explicitly N=1-safe (`apps/api/src/websocket.ts:12-48`
historical), React18+Vite PWA, single Fly.io instance. Central JWT/OTP auth
(`packages/platform/src/auth/jwt.ts:1,55,105` historical). Orders priced server-authoritatively in
`apps/api/src/routes/orders.ts` (POST :65, fee ladder :534-565, `INSERT INTO orders` :597,
historical). Courier assignment = central DB `SELECT…FOR UPDATE` transaction
(`lib/courierAssignmentService.ts:6-61` historical). **All of this is the centralized-architecture
inventory that the mesh target replaces.**

---

## 1. Per-anchor analysis

### D4 — Product UI determinism: dowiz UI = deterministic physics/math wasm

**Current state.**
- Deterministic substrate EXISTS and is proven:
  - `engine/src/field_frame.rs:139-156` — semi-implicit field integrator
    `U_next = (U + dt·(Γ·U̇ + c²·L·U) + dt·S)/(1+dt·M)`, fail-closed CFL bound (`assert_stable`,
    :55-68); `field_frame.rs:299-323` test `compose_returns_deterministic_frame` proves
    **bit-identical RGBA frames** across calls.
  - `engine/Cargo.toml` — zero external deps by mandate (`default = []`); `gpu = []` is an honest
    empty stub ("wgpu uncached… real adapter is an honest Err", W20/W21).
  - `kernel/src/wasm.rs` — product-math surface already exported: `place_order_js` (:276),
    `apply_event_js` (:288), `estimate_order_total_js` (:398), `fsm_graph_report_js` (:414), 9
    `geo_*_js` fns (:474-605), 5 `spectral_*_js` fns (:651+).
  - `web/` — the live kernel-driven shell (`web/README.md`: "This shell **never re-implements**
    geo/spectral/FSM math in JS/TS"); `web/src/app.mjs:1-33` boots the kernel wasm and renders
    ρ / FSM signature / geo route-snap from kernel math only; `web/src/lib/kernel/kernel.test.mjs`
    20 assertions green (W17).
- What does NOT exist: **any product page** on this substrate. The 26-page inventory
  (client cart/checkout/tracking, courier shift/delivery/earnings, owner menu/settlements) lives
  only in design docs — `docs/design/dowiz-interfaces/{RESEARCH-CONSPECT,DOWIZ-INTERFACES-PLAN,
  BLUEPRINTS-DOWIZ-INTERFACES}.md` (Sea & Sheet: WebGL2 Gerstner-wave "sea" ambient/tracking field
  + paper "sheet" brand/menu; DZ-01… work units each with falsifiable RED→GREEN gate; invariants:
  `<Money>` integer-from-kernel, "Money never tweened", local-first render loop that never touches
  the server). Also `field_frame::compose` is **not yet exposed** through `wasm.rs` (grep: no
  field/rgba export) — the physics render can't reach a canvas yet.

**Target state.** Full product UI = deterministic physics/math wasm: every page consumes kernel
wasm math (zero JS re-implementation, greppable), field render drives the ambient layer, Sea &
Sheet design executed per DZ blueprints, local-first (server/mesh = async sync peer).

**Gap.** (a) wasm export for `field_frame::compose` + canvas blit path; (b) all 26 pages rebuilt on
the `web/` shell pattern; (c) a machine gate that keeps determinism (grep/CI: no client-side money/
geo/FSM arithmetic — the pattern `web/src/lib/kernel/kernel_client.mjs` already models).

### E41 — physics/math wasm (product)

Same substrate as D4. Additional gap vs the GS blueprint: `engine/Cargo.toml` has only `gpu = []`;
the pre-declared `webgl`/`webgpu`/`splat` opt-in features from GAUSSIAN-SPLATTING §2.5 do not exist
yet (they land with P1-B2, blocked on GPU-unlock for wgpu itself but the feature scaffolding +
software-rasterizer CI golden tests are buildable when the network unlock happens). No discrepancy
found between GS §4 and SYNTHESIZED §2-B/§3 — SYNTHESIZED only adds explicit dependency edges
(P1-B1 ← P0-B3; P1-B2/B3 ← P1-B1), which this cluster adopts as-is.

### E42 — "openbebop existing design"

**Current state — the anchor's literal referent is EMPTY.** `/root/bebop-repo/delivery/` contains
exactly one untracked, empty placeholder dir `telegram-pending/` (zero files, no git history). The
actual "existing design" is:
- **Code:** `bebop2/delivery-domain/src/lib.rs` — `DeliveryStatus` with pinned wire discriminants
  (:42-85), `OrderTransition` (:92), `assert_transition_local` (:145) mirroring the dowiz kernel
  order machine (re-export behind `kernel-rlib` feature :23-32); tests prove two nodes fold the
  same status and reject forged `Pending→Delivered` (:179-216). `bebop2/mesh-node/src/dod.rs` —
  DoD admission gate (`DodGate::admit` :58-73: payload/id/replay/expiry).
  `bebop2/proto-cap/src/matcher.rs:41,63` — coordination-free courier assignment via HRW
  (structural NO-COURIER-SCORING :30-36 + CI gate `scripts/ci-no-courier-scoring.sh`).
- **Docs:** `bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md` §3
  L0-L4 layered stack (:73-88) + honest gap ledger §6 (:146-157);
  `SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md` (transport/identity/capability/no-scoring —
  explicitly NOT PoD/settlement/escrow).

**Gap.** None to build here per se, but the anchor is **mis-pointed**: E42 should be re-anchored to
`bebop2/delivery-domain` + blueprint v3, and the empty `delivery/` placeholder either populated or
deleted. Flagged for the R2 merge pass.

### E43 — web-first responsive · E44 — WCAG via native-spa

**Current state.** The responsive PWA was deleted with `apps/web`. The new `web/` shell is a
single debug page with no responsive system and no accessibility work (nothing to grep — no
product markup exists). Sea & Sheet plan covers cross-platform (web/laptop/AR) and tokens.

**Target.** Web-first responsive + WCAG AA on the rebuilt native-spa (kernel-driven shell).

**Gap.** Both are **properties of pages that don't exist yet** — they are acceptance criteria of
the UI rebuild phase, not separate builds. Falsifiable: axe/WCAG-AA audit + viewport matrix in the
page-level DZ gates. (E12/E44 CI gates are DEV-TIME canonical-repo fences per §0 SCOPE RULE — a hub
may ship anything.)

### E45 / F47 — wasm demo of a delivery → video after GPU-unlock

**Current state.** `web/src/app.mjs` renders kernel numbers (ρ=1/gap/drift, FSM 10-vertex/9-edge
acyclic report, route snap) — a math demo, **not a delivery scene**. `field_frame::compose`
(engine/src/field_frame.rs:189-196) produces the deterministic RGBA frame a demo would blit, but no
wasm export and no canvas wiring exist. Video is correctly blocked: W21 documented CEILING — wgpu
absent from the offline cargo cache; trigger = network `cargo add wgpu` (ARCHITECTURE §8 "GPU:
wgpu offline-ceiling (W21); GPU-unlock pending network").

**Gap.** Scripted delivery scenario (courier marker driven by `geo::progress_along_route` +
`bridge::geo::CourierMarker`, order FSM ticking through `apply_event_js`, field physics compose)
rendered in-browser from wasm; bit-deterministic for a fixed scenario. Video = post-GPU-unlock
capture of the same scenario (E4 LOCK: demo=wasm now, video AFTER unlock — do not fake it earlier).

### E1 / F41 — order routed over mesh hub-ring

**Current state.**
- Mesh side (bebop2): real iroh-QUIC transport (`bebop2/proto-wire/src/iroh_transport.rs` — real
  Endpoint, rustls+ring :108/:213; replaced the old `unimplemented!()` stub), envelope/framing/
  handshake/discovery siblings, `mesh_consensus.rs` test, DoD gate, delivery-domain fold. **But
  topology is flat sparse-P2P only: "any node is producer/consumer; no central hub"
  (SOVEREIGN-EVENT-EXCHANGE :57); relay/DERP is a future note (iroh_transport.rs:24). No ring/hub
  module exists anywhere** (grep zero).
- Product side (dowiz): order creation was a central Fastify POST + Postgres INSERT (historical,
  §0); the kernel has the order Law (`order_machine.rs:64-78` transition table + golden-signature
  drift gate :502) and `place_order_js`/`apply_event_js` wasm entry points — i.e. the *decide/fold*
  half of mesh order routing already exists; the *transport/ownership* half does not.

**Target.** Order intake → signed envelope → hub-ring routing (region/order ownership) → dod-gated
fold on every interested hub; PQ-safe end-to-end.

**Gap & underspecification flag.** "Hub-ring" is **not defined anywhere in code or canon beyond
the two words** (E1 "hub-ring+sparse-P2P(C)"). The only concrete candidate semantics in the corpus
is SYNTHESIZED §3 P1: "Consistent-hashing ring for order/region ownership on top of the existing
HRW-hashing for couriers". This cluster adopts that reading (ring = deterministic order/region
ownership overlay, not a physical star topology — which would contradict M7 no-SPOF) and flags it
for R2 confirmation. Cross-cluster: the transport/capability substrate itself is Cluster-A
(M-series) scope; this cluster owns the ring-overlay + order-flow-on-top.

### F42 — Proof-of-Delivery signed by edge ML-DSA

**Current state.**
- Product had **NO PoD of any kind**: delivery completion was a courier UI slide →
  `POST /assignments/:id/delivered` (`apps/web/.../DeliveryPage.tsx:202`,
  `routes/courier/assignments.ts:271-342` historical) stamping `delivered_at` + cash fields. No
  photo, no signature, no handover code (repo-wide historical grep: zero PoD hits).
- A PoD primitive EXISTS but is stranded in the legacy agent crate:
  `crates/bebop/src/pod.rs:1-24` — `DeliveryClaim{order_id,courier_id,ts,x,y}`, SHA-512 digest,
  hybrid ML-DSA-65⊕Ed25519 vault signature. **Not wired to bebop2 mesh** (no `pod.rs` under
  `bebop2/`).
- Blueprint v3 specifies the target: L1 self-cert identity + multi-signal PoD (:79-83), L3
  settlement = **threshold verifier ≥k-of-n courier/owner sigs on PoD** (:83-84); gap ledger G7
  "Physical-handoff PoD no trustless anchor" (HIGH).

**Gap.** Port/redesign `DeliveryClaim` into `bebop2/delivery-domain`; edge (courier device) signs
with its self-certifying ML-DSA key (M4); multi-signal evidence (geo fix via kernel
`haversine_meters`/`is_arriving`, optional photo — **synergy: P0-B3 courier photo capture serves
both PoD evidence and the splat bootstrap, one capture flow, two consumers**); k-of-n threshold
verification before settlement. Depends on Cluster-A key/identity plumbing.

### F43 — courier paid via integer-money saga

**Current state.**
- Kernel money is integer + currency-tagged + fail-closed: `money.rs:57-87` (`Money::checked_add`
  rejects cross-currency and overflow), `apply_tax`/`compute_line_total` overflow-safe (BP-17),
  fee-ladder mirror `estimate_order_total` (:216-232). **But there is NO reversal/compensating
  primitive** (checked_add only) and the FSM has **no compensation edges**:
  `order_machine.rs:64-78` — `InDelivery → &[Delivered]`, terminals empty; happy-path only.
  This is exactly SYNTHESIZED **P0-A4** (Cluster-C kernel-correctness scope — consumed here, not
  duplicated).
- Payout logic (historical): cash settlement cycle — `courier_payouts` (deliveries_count,
  total_earned, `courier/settlements.ts:29-32,80`), owner generates/approves/pays
  (`routes/owner/settlements.ts`), period math `lib/settlement-period.ts:1-13`. Central DB only.
- A double-entry primitive EXISTS but is stranded: `crates/bebop/src/ledger.rs` (238 lines —
  sum-of-balances-zero invariant, idempotent Transfer, fails closed). Not in bebop2, not in kernel.

**Target.** Courier payout = event-sourced saga of integer ledger entries driven by PoD-settled
order events, with compensation on cancel/dispute; S9 LOCK (integer + event-sourcing +
saga-compensation).

**Gap.** (1) P0-A4 money-reversal + FSM compensation edges (**cross-cluster dependency: kernel
cluster**, red-line change — deliberate FSM golden-signature update, never silent); (2) port or
rebuild `ledger.rs`'s double-entry law into the canonical path (kernel or bebop2 — DECART: it
already satisfies zero-dep); (3) payout saga wiring PoD→earn→settle→(compensate) over mesh events,
preserving the As-Built cash-settlement feature set (77% cash market — cash reconciliation must
survive the re-plumb).

### F44 — hub disputes order via protocol arbitration + escrow

**Current state — honestly assessed.**
- **Zero implementation**: repo-wide `grep -rniI "escrow|arbitrat|dispute" --include=*.rs` over
  ALL of bebop-repo and dowiz = **0 matches**. The legacy product's only "dispute" was a
  courier↔owner settlement reconciliation status flag (`owner/settlements.ts:206-241` historical)
  — not customer-facing, no funds hold.
- **BUT the design is more than one line** (correcting the prior assumption): 
  `bebop-repo/docs/design/fable-protocol-2026-07-11/F2-dispute-arbitration.md` is a full spec —
  6-state fail-closed machine `OPEN→EVIDENCE→AUTO_ARBITRATE→ESCALATE→JURY→SETTLE` with timeouts
  (T_ev=48h, T_aa=10m, T_em=24h, T_j=72h), invariant "any timeout/ambiguity → escrow HOLD +
  default refund to claimant" (:35), a falsifiable RED test (:78-87), and Kleros/Schelling
  analysis. Blueprint v3 L4 (:85-86) + gap G8 "Dispute resolution unbuilt (MED-HIGH)" (:157).
- **Two unresolved design contradictions inside that spec** (flagged for operator/DECART, cannot
  be built around silently):
  1. F2 maps JURY onto legacy `guard.rs`/`reputation.rs` — but reputation-trust is **permanently
     rejected** (V2, M12 capability-only, NO-COURIER-SCORING structural CI gate). A jury selected
     or weighted by reputation violates canon. Alternative consistent with M12: arbitration as an
     operator-gated **capability** (signed arbiter capability with red-line deny), or pure
     Schelling-point voting among staked capability-holders.
  2. `PROTOCOL-CENTRALIZATION-MAP.md:141` says "use UMA/Kleros, don't build" — an external
     dependency at the trust boundary, which violates M6 (zero protocol deps). Building F2
     in-repo is the canon-consistent path; the UMA/Kleros note predates the mesh pivot.

**Gap.** Implement F2's state machine in bebop2 (protocol messages over envelopes), escrow = HOLD
entries in the F43 double-entry ledger (a hold is just a paired entry to an escrow account —
reuses the same primitive), default-refund on timeout, arbiter-capability model per the DECART
resolution above. Depends on F43 ledger and Cluster-A capability tokens.

### F45 — route computed Dijkstra/A* over geo

**Current state.** dowiz `kernel/src/geo.rs` has **no route computation** — confirmed by full
read: haversine (:15), bearing (:30), EMA (:39), polyline projection `progress_along_route`
(:70-146), ETA (:153), point-in-polygon (:200). It projects onto an *already-given* polyline; it
never computes one (matches SYNTHESIZED §3: "a product gap, not infra"). Repo-wide grep for
dijkstra/astar/A* in dowiz kernel+engine = zero. **However a real implementation exists in the
legacy bebop crate**: `crates/bebop/src/cost_estimate.rs:205` — binary-heap Dijkstra with
admissible Euclidean A* heuristic (:238) + Contraction-Hierarchy shortcuts (:13-17), under the
"k-d filter + BFS guard + A*/Dijkstra + CH" hybrid engine (`crates/bebop/src/lib.rs:22`). It is not
wired to anything delivery-shaped and not in the dowiz kernel.

**Target.** F45 LOCK + wire: route computed over a geo road-graph, feeding
`progress_along_route`/ETA downstream unchanged.

**Gap.** (1) Port/adapt the `cost_estimate.rs` router into the dowiz kernel (zero-dep std-only —
it already qualifies; DECART note either way per the rust-native rule); (2) a road-graph ingestion
port (OSM ways → CSR graph — `kernel/src/csr.rs` deterministic CSR already exists as the natural
container); (3) wire output polyline into the existing courier flow. The GS synthesis names this
gap explicitly; nothing there is re-derived here.

### F46 — partition-tolerant delivery (Union-Find/MST)

**Current state.** **Absent everywhere**: grep for union-find/DSU/disjoint-set/MST/Kruskal/Prim
across dowiz AND bebop-repo `.rs` = zero matches. The named replacement site exists:
`kernel/src/cgraph.rs:171-212` `c_components()` computes connected components via ad-hoc BFS over
bidirected adjacency (plus BFS d-separation :352-377). Partition-merge policy exists only as
canon: F15 (islands merge via HRW), F12 (island mode survives).

**Target.** DSU as a kernel primitive (replacing/backing cgraph's component BFS with parity
proof), MST (Kruskal over the DSU) for the gossip/overlay spanning tree, wired into the mesh
partition-heal path (M7: "mesh heals via Dijkstra/A* + Union-Find/MST").

**Gap.** Full implementation + wiring. Kernel half (DSU/MST primitives, cgraph parity) is
self-contained; mesh half (overlay spanning tree, HRW island-merge) depends on Cluster-A topology
work. Honest note: "partition-tolerant **delivery**" also needs an order-ownership rule during
splits (who may fold Delivered while partitioned) — only the DoD replay-dedup gate
(`dod.rs:58-73`) and the I-FINAL quorum-intersection proof idea (SYNTHESIZED P0-A5, Cluster-A/C)
address this; the dup-risk CON in F46 is real and the falsifier must include a
partition-then-merge double-finalization test.

### F48 — PER-HUB replicated graph-wiki (corrected from single-central)

**Uses the CORRECTED C4 form** (ARCHITECTURE §8: single-graph wiki → PER-HUB REPLICATED, no
central SPOF; supersedes E8/E51's "single-graph wiki" wording — do not build the central version).

**Current state.** Single-node knowledge machinery exists in the kernel: `living_knowledge.rs`,
`spine.rs`, `trigram.rs`, `csr.rs` (deterministic Jacobi PPR), `backup.rs` (sha3
content-addressed BlockStore + Buzhash-CDC dedup), spectral/BD organs; W18 wired PRIMARY recall
(recall@5=1.0). **Nothing is replicated**: no sync protocol, no per-hub instancing, no merge
semantics anywhere.

**Target.** Each Hydra head keeps its OWN graph (BD+spectral+history); opportunistic sync over the
protocol (signed envelopes carrying graph deltas); content-addressed dedup makes replication
convergent for identical content; NO central authority; hub loss loses nothing globally.

**Gap.** The whole replication layer: (1) per-hub graph instance boundary; (2) delta export/import
as envelope payloads (dedup free via BlockStore content addressing); (3) merge policy for
divergent entries — **underspecified in canon** (F48 CON says only "dedup/merge cost"). Flag: no
CRDT/merge semantics are specified, and bebop has a dormant `crdt-fence` pre-commit guard whose
intent (fence CRDTs out, or fence them correct) must be checked before choosing
merge-by-content-address vs CRDT — DECART item, not silently resolvable here.

### F49 — i18n UA/EN/AL for courier app

**Current state.** The entire i18n layer was **deleted** with `packages/ui` (79ef316f6). The
historical implementation (recoverable: `git show db766de47~1:packages/ui/src/lib/i18n.ts`):
`Locale = 'sq' | 'en' | 'uk'` (:3 — Albanian IS `sq`, Ukrainian `uk` displayed "UA"; default
`sq` :8), key-major catalog in `i18n-catalog.ts` as single source (~1291 keys, 631 populated per
locale), locale-major derivation `fromCatalog` (:13-21), `t()/translate()` with dev-loud missing
keys (:26-38), CI parity gate `scripts/i18n-parity.mjs`. Courier surfaces consumed the
`courier.*` namespace exclusively (DeliveryPage/EarningsPage/ShiftPage/… historical). The new
`web/` shell has **zero i18n**.

**Target.** UA+EN+AL on the rebuilt courier app (E12: EN-main, all-locales via OSS; blocking CI
parity gate = canonical-repo DEV-TIME only per SCOPE RULE).

**Gap.** Rebuild i18n for the kernel-driven shell: recover the catalog from git history (asset,
not rewrite — 631×3 translated strings already paid for), serve it key-major as static data to
`web/`, re-institute the parity CI gate. Zero-dep JS lookup is fine (i18n is display, not math —
D4's "no JS math" invariant is untouched).

### F50 — living-organism unbounded (product reading)

Primary ownership is the hub-autonomy cluster (M5/M9/M11). The product-cluster obligation is
narrower and falsifiable: **no rebuilt product feature may reintroduce a mandatory central
service.** The legacy product violated this everywhere (single Postgres, N=1 WebSocket rooms,
central JWT — §0). Target: every product flow functions on a solo hub (F12 island mode) and across
two hubs with no third party. This becomes a standing acceptance criterion on Phases 2-4 below,
not a separate build.

---

## 2. Cross-cluster dependencies (explicit)

| This cluster needs | From | For |
|---|---|---|
| proto-cap capability tokens, self-cert ML-DSA edge identity, transport live on ≥2 nodes | Cluster A (M1-M12, E31-40) | F41 routing, F42 PoD signing, F44 arbitration capability, F48 sync |
| P0-A4 money reversal + FSM compensation edges (red-line) | Cluster C (S9/kernel correctness) | F43 saga, F44 escrow refund |
| I-FINAL quorum-intersection proof (P0-A5) | Cluster A/C | F46 partition double-finalization falsifier |
| E12 i18n CI gate mechanics, E4 demo/video policy | Cluster E (ecosystem) | F49 gate, F47/E45 video timing |
| GPU-unlock (network `cargo add wgpu`) | environment/operator | E45 video, P1-B2 renderer |

This cluster provides to others: kernel Dijkstra/A*+DSU/MST primitives (M7 heal path — Cluster A
consumes), courier photo capture (feeds splat pipeline AND PoD), the product UI shell (Cluster E
demos ride it).

---

## 3. Build phases (ordered, zero exceptions)

### Phase D-1 — Kernel product-math completion (routing, partition, geometry)
**Anchors:** F45, F46 (kernel half), D4 foundation, feeds F42/F43.
**Dependencies:** none external — pure kernel work, parallel-safe with Cluster A. Consumes P0-A4
from Cluster C if landed; otherwise coordinates the red-line FSM/money change with them (one
owner, no duplicate edit).
**Scope:** (1) port `crates/bebop/src/cost_estimate.rs` Dijkstra/A*(+CH) into dowiz kernel with a
road-graph ingestion port onto `csr.rs`; (2) DSU primitive + Kruskal MST; parity-swap cgraph's BFS
`c_components` onto DSU; (3) the six GS §2.6 geo functions (P0-B1: `storey_height_m` …
`los_clear`) in geo.rs's existing style; (4) wasm exports for all of the above +
`field_frame::compose` RGBA.
**Done-test (falsifiable):** route on a hand-oracle 10-node graph = known shortest path; A* result
== Dijkstra result (admissibility check); DSU `c_components` byte-identical to BFS on the existing
cgraph test fixtures; GS P0.1 six-item acceptance list verbatim (pin-drop / floor-slice /
open-space degrade / <1° bearing / 0-360 seam / LOS rectangle); `compose` reachable from node via
wasm with bit-identical frames across two runs.

### Phase D-2 — Delivery-on-protocol spine: order over mesh + PoD + payout saga
**Anchors:** F41, E1, F42, F43, F50 (invariant).
**Dependencies:** Cluster A transport/identity live on ≥2 nodes; D-1 (kernel edges);
P0-A4 landed.
**Scope:** (1) consistent-hash **hub-ring** overlay for order/region ownership on top of HRW
(adopting the SYNTHESIZED §3 reading; R2 to confirm semantics); (2) order intake → signed envelope
→ DoD gate → `delivery-domain` fold, end-to-end on two hubs; (3) PoD: `DeliveryClaim` ported into
bebop2, courier edge ML-DSA signature, multi-signal evidence (geo fix + optional photo via the
P0-B3 capture flow — one flow, two consumers), k-of-n threshold verify per v3 L3; (4) payout saga:
double-entry ledger (port `crates/bebop/src/ledger.rs` law) driven by settled PoD, compensation on
cancel; cash-reconciliation feature parity with the As-Built settlement cycle.
**Done-test:** order placed on hub A folds to identical `DeliveryStatus` on hub B; forged
`Pending→Delivered` rejected (extends delivery-domain :179-216 test); tampered PoD claim rejected,
valid k-of-n claim settles; ledger sums to zero across one delivered order AND one
cancel-after-confirm compensation; **solo-hub island test: the full flow completes with zero
non-hub services running** (F50).

### Phase D-3 — Dispute/arbitration + escrow + per-hub graph-wiki
**Anchors:** F44, F48.
**Dependencies:** D-2 ledger (escrow holds), Cluster A capabilities; operator DECART on the two
F44 contradictions (jury-vs-NO-SCORING; UMA/Kleros-vs-M6) and the F48 merge-semantics question —
**these decisions gate the phase and are flagged now, not mid-build.**
**Scope:** (1) F2 6-state dispute machine as protocol messages, fail-closed, escrow HOLD = paired
ledger entries to an escrow account, default-refund-to-claimant on any timeout; arbiter =
operator-gated capability (M12-consistent), never reputation; (2) per-hub graph-wiki: per-hub
knowledge-spine instance + delta sync via signed envelopes + BlockStore content-address dedup;
merge policy per DECART outcome.
**Done-test:** F2's own RED test verbatim (dispute opened, no evidence, timeouts elapse → escrow
auto-refunds claimant; funds conserved); ledger still sums to zero with an open HOLD; two hubs
diverge offline then sync → both graphs hold the union, no central node consulted; kill either
hub → the other retains a complete graph (no-SPOF falsifier).

### Phase D-4 — Product UI rebuild: deterministic wasm Sea & Sheet + i18n + WCAG
**Anchors:** D4, E41, E43, E44, F49, E42 (re-anchor), F50 (invariant).
**Dependencies:** D-1 (wasm surface incl. compose), D-2 (real order flow to render); independent
of D-3.
**Scope:** execute DZ-01… blueprints on the `web/` shell pattern: 26 pages (client/courier/owner),
all math from kernel wasm; field render as the Sea layer; responsive web-first; i18n rebuilt from
the recovered catalog (sq/en/uk, key-major, parity CI); WCAG-AA per page; address-picker v1
(P0-B1 UI) + courier photo capture (P0-B3) shipped inside the courier pages.
**Done-test:** feature-inventory reconciliation — every As-Built shipped feature maps to a rebuilt
page or an explicit operator-approved drop (the DOWIZ-INTERFACES-PLAN master checklist is the
ledger); Playwright E2E full loop (client order → courier PoD → owner settlement) green against
the wasm UI on a mesh hub; i18n parity gate green ×3 locales; axe WCAG-AA zero critical; CI grep
gate proves zero client-side money/geo/FSM arithmetic.

### Phase D-5 — Delivery demo, splat tiers, GPU-unlock closure
**Anchors:** F47, E45, E4, E41 remainder.
**Dependencies:** D-4 (UI), GPU-unlock trigger (network) for video and wgpu; GS P1 items land here
exactly as written (P1-B1 SplatReconstructionJob+Modal ← P0-B3 photos; P1-B2 tiered renderer;
P1-B3 Tier-C pre-render) — no re-derivation, GS §4/§5 rejections stand (no CuPy, no trained
reranker, no TimesFM-ETA, no satellite, no standing GPU).
**Scope:** scripted wasm delivery demo (kernel-driven scenario: courier along a D-1-computed
route, FSM ticking, field physics); after GPU-unlock: real wgpu adapter behind the existing
`gpu` feature + video capture of the same scenario; splat P1 tier per GS acceptance lists.
**Done-test:** demo runs offline (node + browser), bit-deterministic frames for a fixed scenario;
default `cargo build/test` dependency graph byte-identical to today; post-unlock: video artifact
rendered from the same scenario; one real address reconstructed ≤$2 with $0 cache-hit
re-submission (GS P1.1 verbatim).

---

## 4. Ambiguous / underspecified anchors (honest register)

1. **E1/F41 "hub-ring"** — two words, no spec. Adopted reading: consistent-hash ownership ring
   over HRW (SYNTHESIZED §3). Needs R2 confirmation; a literal star-hub reading would contradict
   M7 and the SOVEREIGN doc's "no central hub".
2. **F44 arbitration+escrow** — spec EXISTS (F2 doc, 6 states + RED test) but carries two canon
   contradictions (jury→reputation vs NO-SCORING/M12; "use UMA/Kleros" vs M6 zero-dep). Operator
   DECART required before D-3.
3. **F48 merge semantics** — "dedup/merge cost" is acknowledged but unresolved
   (content-address-only vs CRDT; dormant `crdt-fence` guard intent unknown). DECART before D-3.
4. **E42 referent** — `/root/bebop-repo/delivery/` is an empty untracked placeholder; the anchor
   should point at `bebop2/delivery-domain` + blueprint v3.
5. **F46 "partition-tolerant delivery"** — DSU/MST gives topology healing; the *order-state*
   half (who finalizes during a split) leans on I-FINAL (unbuilt, Cluster A/C) — the F46 done-test
   must include the partition-then-merge double-finalization case or the anchor is only half
   closed.
6. **F41-F50's premise "existing product being re-plumbed"** — the product source is deleted
   (attic + git history + a legacy deployed instance); the honest framing is rebuild-with-
   feature-inventory-preservation, which D-4's reconciliation done-test makes falsifiable.

---

*R1-D complete. Inputs: ARCHITECTURE.md, STRATEGIC-VECTORS-LOCKED, GAUSSIAN-SPLATTING synthesis,
SYNTHESIZED-BLUEPRINT-PLAN, ROADMAP-GROUND-TRUTH rev-5, direct reads of kernel/engine/web +
bebop2 + git history (`db766de47~1`, `fce5738b0`), two grounding sweeps (dowiz product as-built;
bebop delivery surface). For R2 merge: phases D-1…D-5 above; D-1 is parallel-safe immediately;
D-2 gates on Cluster A transport + Cluster C P0-A4; D-3 gates on two operator DECART decisions.*
