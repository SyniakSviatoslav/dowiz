# DECISIONS — dowiz/bebop

> Operator-confirmed red-line decisions. Source of truth alongside `MANIFESTO.md`.
> Date: 2026-07-12. Status: AUTHORITATIVE.

## D0. Hard invariants (operator, 2026-07-12)
These six words are the lens for EVERY change, in priority order:
**decentralized · local-first · post-quantum · crypto · mesh · reliability-over-latency.**

If a change breaks any of these, it is rejected. They outrank roadmap sequencing,
feature requests, and "MVP-first" pragmatism (C8/YAGNI still applies to *scope*, not to
these invariants — the invariants are non-negotiable; only their *machinery depth* is phased).

## D1. Drop the centralized server (CONFIRMED)
- `server/` (axum + rusqlite centralized deploy) is **DROPPED**. Not refactored, not kept
  as "single-node mode" — removed from the build.
- Rationale: a centralized dispatch/deploy server is the anti-pattern the protocol exists to
  kill (MANIFESTO C13, "DoorDash with extra steps").
- Replacement: peer nodes, each running the Rust/WASM kernel + a local SQLite DB. No
  server process, no central DB, no Supabase, no Fly.
- Action: delete `server/` crate; remove from workspace `Cargo.toml`; keep any genuinely
  reusable pure logic (e.g. `reliability.rs` retransmit queue) by porting it into the node
  crate as a transport-agnostic module.

## D2. Manifest location (CONFIRMED)
- `MANIFESTO.md` lives at repo **root** (`/root/dowiz/MANIFESTO.md`), NOT `docs/design/`.
- The existing `docs/design/MANIFESTO.md` is copied to root and the stale copy removed, so
  blueprint citations (`MANIFESTO.md:28-30`) resolve.
- `DECISIONS.md` (this file) also lives at root.

## D3. Transport selection — spacecraft/lab-grade, reliability > latency (LOCKED, 2026-07-12)
- Requirement: must tolerate long delays, intermittent links, and partial outages
  (couriers drop offline constantly; use case extends to labs/satellites).
- **LOCKED STACK (primary research, PRIMARY sources only):**
  - **DTN / Bundle Protocol v7 (RFC 9171)** = the custody/store-and-forward substrate.
    Store-carry-forward overlay; explicitly targets "intermittent connectivity, sender and
    receiver not concurrently present" (RFC 9171 §1) — EXACTLY the courier use case.
    Rust core: **dtn7-rs** (real, active, Apache-2.0/MIT).
  - **QUIC (RFC 9000) or TCPCLv4 (RFC 9174)** = reliable convergence layer on connected
    segments (carries bundles). quinn = Rust impl. QUIC alone does NOT meet reliability>latency
    (needs a live path) — underlay only.
  - **Link layer** may be SpaceWire/SpaceFibre (sat/lab, ESA/CCSDS), LoRa, or Ethernet (ground).
  - **libp2p-gossipsub REJECTED** as primary: latency-optimized, conflicts with C11.
  - **Zenoh / Reticulum REJECTED** as substrate: Zenoh optimizes latency (not custody);
    Reticulum is Python (no Rust core) + classical crypto (not PQ) + no custody primitive.
- **Custody transfer MUST be real:** BPv7 core only *requests* custody-acceptance reporting
  (flag bit 15). Implement **BIBE (draft-ietf-dtn-bibect)** for actual hop custody handoff.
  Verify dtn7-rs supports BIBE; if not, contribution required. Without BIBE, "custody" is poetry.
- **Replay protection:** enforce bundle lifetime expiry + dedupe on (source EID, Creation
  Timestamp) per RFC 9171; BPSec (RFC 9172) integrity blocks prevent tamper-at-rest.
- **PQ envelope at PROTOCOL layer REGARDLESS of transport:** QUIC/Zenoh/Reticulum are ALL
  classical (X25519/Ed25519/AES/TLS1.3). Our from-scratch ML-KEM-768 + ML-DSA-65 MUST wrap the
  bundle payload (custom BPSec security context, RFC 9172/9173 pattern, or app-layer envelope)
  so PQ holds whether underlay is QUIC, TCPCLv4, Zenoh, or SpaceWire. Do NOT rely on transport
  native crypto for PQ.
- **Auditability:** prefer RFC-spec'd protocols over code-as-spec (Reticulum) for space/lab grade.
- Source: `/root/dowiz/RESEARCH-transport-dtn-mesh.md` (PRIMARY: RFC 4838/9171/9172/9174/9000,
  eclipse.dev/zenoh, reticulum.network, ccsds.org, dtn7-rs, quinn).

## D4. Post-quantum is a PROTOCOL (CONFIRMED, MANIFESTO C12)
- Not isolated primitives. The composed scope:
  1. Transit: hybrid KEM `X25519 + ML-KEM-768` (FIPS 203), both-verify, no classical-only fallback.
  2. Signatures: ML-DSA-65 (FIPS 204) — packets, node identity, code-signing.
     CORRECTION: ML-DSA is **FIPS 204**, NOT SP 800-208 (that is LMS/XMSS). Fix all citations.
  3. At-rest: AES-256-GCM volume key, wrapped via ML-KEM encaps (or HPKE RFC 9180) to node
     pubkey, envelope ML-DSA-signed. No "PQ disk encryption" snake-oil.
  4. Code/supply-chain: node update blobs ML-DSA-signed, verified against pinned root BEFORE apply.
  5. In-transit: AEAD inside PQ channel + ML-DSA sig over `(state, seq)`; unsigned ⇒ drop.
- Our from-scratch kernel (ML-KEM-768 + ML-DSA-65, ACVP byte-exact) is the **local reference
  core** (verified, not yet independently audited for production). For production node-to-node
  channels, prefer rustls+aws-lc-rs (X25519MLKEM768) + liboqs ML-DSA until our core is
  independently validated.

## D5. Roles + adapters (CONFIRMED, MANIFESTO §5)
- 3 autonomous node roles: owner/merchant, courier, customer. Each = local SQLite + kernel.
- Adapters/bridges (NOT core transport): NOSTR (messenger/social), ActivityPub (fediverse),
  MCP (tool entrypoint). Every bridged message wrapped in ML-DSA/ML-KEM envelope first.

## D6. Sequence (operator override of C8/D6)
- MANIFESTO C8 gates *over-engineering*; it does NOT block the invariants. Mesh machinery is
  NOW required (operator mandate), not deferred. Seams already exist in L0; L1/L2 machinery is
  in-scope. YAGNI still applies to anything outside the 6 invariants + MVP food-vendor gaps.

## D7. Verification discipline
- Every change ships a RED+GREEN falsifiable assertion (MANIFESTO C7).
- Terminal must stay unblocked: no hanging background builds; verify with fresh `cargo test`
  output, never agent narrative.

## D8. Plan precedence — newest outranks older (operator, 2026-07-12)
- **Conflict rule:** when an older roadmap/blueprint and a newer approved decision conflict, the
  **NEWEST wins**. MANIFESTO.md + DECISIONS.md (dated 2026-07-12) are the live source of truth and
  **SUPERSEDE** the 2026-07-11 `ROADMAP-GROUND-TRUTH` and any `MASTER-BUILD-SEQUENCE` / stale
  blueprint citation.
- Concretely: mesh machinery is NOW in-scope (D6 overrides C8's deferral); transport = DTN/BPv7
  (RFC 9171) + QUIC/TCPCLv4 + BIBE (D3), NOT libp2p; PQ is a protocol not primitives (C12/D4).
- Before acting on a roadmap tier, check the date + D-series. If conflict → follow the newer artifact.
- Mirrored in `bebop-repo/docs/RULES.md` (precedence setting, Anu line).

## D9. Anu QRNG wiring — native entropy is DEFAULT + FALLBACK (operator, 2026-07-12)
- **Remote quantum entropy IS wired in**: `kernel/src/pq/entropy.rs` `provider` module pulls REAL
  vacuum-fluctuation noise from **ANU QRNG** (`qrng.anu.edu.au`) behind the `qrng` feature, mixed via
  `SHAKE256(quantum ‖ os)` (NIST SP 800-90B: never raw quantum alone).
- **Native (OS) entropy is the DEFAULT and the FALLBACK.** The sanctioned entry point
  `entropy::master_seed()` returns OS `/dev/urandom` seed by default; when `qrng` is on AND ANU is
  reachable it upgrades to quantum⊕os; on ANY failure (unreachable / parse error / offline) it
  **transparently falls back to pure OS entropy** — no error, no broken boot.
- **Rule:** a node MUST boot and produce entropy identically offline (OS-only) and online
  (quantum-boosted). The application must NEVER hard-depend on the remote QRNG being available.
- Production note: the bundled `mini_get` reaches ANU over plain TCP (ponytail stub). Real deployments
  MUST gate a TLS client (reqwest/rustls) behind a `qrng-tls` feature — tracked, not silently shipped.
- Cross-repo: bebop's `bebop2` entropy seam follows the same DEFAULT+FALLBACK contract.

## D10. R-3 RootDelegationPolicy = Option A (OperatorSigned + per-anchor IssuanceBudget) — RULING RECORDED (operator, 2026-07-18)

- **Ruling:** adopt **Option A** — `RootDelegationPolicy::OperatorSigned` bound to a per-anchor
  monotonic `IssuanceBudget` predicate checked at delegation-sign time (`can_issue` / `charge_issuance`
  / `sign_delegation_budgeted`). Dated **2026-07-18**.
- **Authority / FLAG for override:** recorded under the **expanded autopilot mandate** (operator-
  authorized red-line/decision execution, dated 2026-07-18). **FLAGGED — the operator MAY OVERRIDE at
  any time.** This is a recorded ruling, not a lock; a future operator choice of B (`FirstContactQr` +
  hardware attestation), C (`WebOfTrust`), or a named hybrid supersedes it. Cross-ref:
  `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-D-consensus-capability.md` §11;
  `docs/design/CORE-ROADMAP-2026-07-17/P-D-audit-root-delegation-policy.md`.
- **Why A:** the P-D audit's recommended default — ships on today's `AnchorRoster`/`verify_chain`
  substrate, fully sovereign (operator is the only root; no Google/Apple attestation dependency),
  P06-independent, and it closes the Batch-7 Sybil residual (bounded per-epoch issuance). Options B and
  C remain unwired stubs and are **NOT adopted**.
- **Mechanism already built — no code change in this ruling:** `bebop-repo` commit `e08eb07`
  (`bebop2/proto-cap/src/node_id.rs:187-372`; `IssuanceBudget` / `IssuanceError` / `can_issue` /
  `charge_issuance` / `sign_delegation_budgeted`; 10 RED→GREEN tests; CI gate
  `scripts/ci-budgeted-issuance.sh`). This D10 entry is a **ruling RECORD**, not a code change. Per
  `BLUEPRINT-P-D-consensus-capability.md` §11 anti-scope, B's attestation overlay and C's flow-based
  construction are explicitly NOT adopted by this ruling.
- **Operator deployment actions still required (ops, not code):** (i) generate + Ed25519-sign the real
  anchor root cert(s) and populate a production `genesis.example.txt`-shaped anchor file
  (`load_genesis`, `node_id.rs:117-142`); (ii) set the production node's runtime `RootDelegationPolicy`
  to `OperatorSigned` **explicitly** — keep `Default = Unspecified` (`node_id.rs:169-174`, never flip the
  code default); (iii) confirm or override `DEFAULT_MAX_PER_EPOCH` (currently `1`, `:203`) against real
  onboarding throughput.
- **Standing anti-scope (unchanged):** do NOT silently change `DEFAULT_MAX_PER_EPOCH` or write a real
  production genesis/anchor file without this ruling on record; do NOT build B/C speculatively; do NOT
  re-couple R-3 to P06 `key_V` (independent per the audit §3).

## D11. Governed Self-Evolution (items 73-78) — apply-token, boundary & sequencing rulings (operator, 2026-07-20)

Rules the 7 open governance questions filed in
`docs/design/BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md` §9. Items 73-78 build the
channel by which the product's own AI proposes changes to its own kernel — the single most
red-line-adjacent capability in the space-grade roadmap. These are spec-plane rulings; no code
exists yet against them.

- **Q1 — apply-token holder:** each node's own operator, fully local. No delegated authority, no
  mesh-wide root — matches D0 (decentralized/local-first/mesh) most literally. A given node's
  self-change proposals are approved only by whoever runs that node.
- **Q2 — what the apply-token physically is:** a **2-factor** design —
  1. a `capability_cert` (ML-DSA-65) scoped to a new `apply:change-proposal` capability, reusing the
     existing `AnchorRoster`/`capability_cert.rs` enrollment substrate (the enrolled key already
     binds to a specific device — no separate device-id field needed);
  2. a live, time-boxed rotating code, TOTP-shaped (RFC 6238 semantics — a code that changes every
     ~30s off a shared secret + time) but HMAC'd with **SHA3-256** (the kernel's existing Keccak
     module) instead of classical HMAC-SHA1, so the second factor stays PQ-consistent and adds zero
     new hash primitives (one-escaper-implementation discipline, same rule as the kernel::json/
     fdr::json dedup ticket).
  Rationale: a stolen long-term signing key alone must not be sufficient; the rotating factor proves
  live human presence at the moment of approval. **Open sub-question, deliberately NOT ruled here:**
  the rotating-secret enrollment ceremony — defer to item 75/64's design.
- **Q3 — Pending-approval TTL:** dynamic and operator-configurable per node, **default 24 hours**.
  Not a fixed constant — item 75's `Pending` state must carry a configurable expiry, not a compiled-in
  number.
- **Q4 — the meta-governance boundary (item 74's registry, the AI's own safety perimeter):** the AI
  MAY reach and edit `agent scope` (`kernel/src/ports/agent/scope.rs`) and the governance module
  itself (items 73-76's own code) — self-evolution is allowed there, gated by the human apply-token
  per D11's Q1/Q2. The ONLY hard, non-negotiable line is **core kernel authority + the circuit
  breaker** (`kernel/src/breaker/`, item 9; the order/money/decide-fold core). Everything else is
  configurable/customizable by the human operator, who may narrow the boundary further at any time —
  the boundary itself is operator-drawn, never agent-drawn, per the blueprint's own Q4 framing.
- **Q5 — red-line registry row-removal ruling format:** a `DECISIONS.md` D-entry (this file) — matches
  existing precedent (D0-D10), keeps one canonical place for every red-line ruling, old and new. No
  separate ruling-doc format or commit-trailer convention.
- **Q6 — item 77 self-heal threshold:** **3 consecutive** adverse windows before a fix proposal may
  be opened. Still just a proposal — Q1/Q2's apply-token is required regardless of threshold.
- **Q7 — build sequencing:** item 77 (self-healing) before item 78 (self-upgrading), the roadmap's
  original numeric order — **operator override of the blueprint's own recommendation** (the architect
  suggested 78-before-77 to battle-test the human-gate on lower-urgency, operator-initiated changes
  first). Recorded per D8 (newest/operator ruling outranks the design doc's suggestion).
- **What this ruling does NOT do:** it does not authorize dispatching items 73-78 to code. Per the
  blueprint's §10, the transition from spec to code for this arc remains its own explicit decision;
  this D11 entry resolves the *content* of that future code's design, not the *go* to start writing
  it. Item 75's `apply` seam specifically stays gated on items 64/65 (unbuilt) regardless of this
  ruling.
- **FLAG for override:** recorded under the same operator-ruling authority as D10. The operator MAY
  override any clause here at any time; this is a recorded ruling, not a lock.

## D12. DeliveryOS launch blockers §4-A–D — the four operator decisions from SYNTHESIS-LAUNCH-BLOCKERS (2026-07-20)

Rules the four operator-only decisions flagged in
`docs/design/CORE-ROADMAP-2026-07-17/SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §4. All four block
blueprint-writing for their named surfaces; nothing here reopens any other closed ruling in that
document (§0.2's two rulings, or any §16/§17 decision it cites).

- **§4-A — Card-capture surface (web + desktop): Path A — scoped provider-iframe overlay.** One
  narrow, documented DOM exception at the card moment only, keeping the user inside the immersive
  wgpu canvas experience for the rest of the flow. Mobile stays Path B (native provider SDK sheet,
  zero DOM) as the source doc already recommended — unaffected by this ruling.
  **Real architectural consequence, not a side detail:** per the source doc's X3 dependency table,
  choosing Path A **forces the desktop shell to host a live webview** (Tauri) for the card moment —
  it rules out the alternative the doc flagged as "cleaner" (pure `winit`+`wgpu`+AccessKit with no
  webview at all). P39-rev (app shell/installability) must record Tauri-with-webview as the
  desktop shell decision, not present it as still-open pending the P63 spike; P63's shell spike
  narrows to *validating* the Tauri-webview approach, not choosing between it and a webview-free
  alternative.
- **§4-B — Self-custody severity: absolute, both surfaces, no exceptions.**
  (i) Backup break-glass: **no `dowiz_break_glass_pubkey` in any backup recipient set, ever.**
  dowiz cannot read backup plaintext by construction — a vendor who loses their key loses every
  backup for that hub. (ii) Owner-root loss: **no recovery path.** A lost owner root eventually
  strands its hub fleet as certs expire under the short-TTL revocation mechanism; this is accepted
  as the cost of (i)'s guarantee applied consistently, not a separate looser policy for keys that
  happen to gate more blast radius. One philosophy, both surfaces — exactly the consistency the
  source doc asked for. P59's revocation section and P68's backup-recipient section must both cite
  this ruling; neither may introduce a recovery mechanism independently.
- **§4-C — Abandoned-claimed-hub power-down: no suspension, every claimed hub stays hot
  indefinitely.** A long-inactive claimed hub is NOT suspended/compute-released — it remains fully
  provisioned and running for as long as it is claimed, consistent with §16.57's "stays the
  vendor's forever" read the most literal way. **Real cost consequence, not a side detail:** the
  warm-pool is net-consumed with zero recycling from inactive hubs; P67's pool-economics section
  must size the pool (and its refill cadence) assuming claimed capacity never returns to the pool,
  not assuming a suspend-and-reclaim offset.
- **§4-D — Food-court Wave-0 market scope: Albania / EU**, matching `PRODUCT.md`'s existing primary
  market (Albania + EN/UK speakers). P72's per-vendor provider-account onboarding and provider
  matrix are scoped to providers that support per-vendor split/connect charges in EUR/ALL — not a
  global-from-day-one matrix. §16.20's requirement that the *architecture* stay market-agnostic is
  unaffected; this only scopes where the *feature* is proven first.
- **Unblocked by this ruling:** blueprint-writing may now proceed for P39-rev, P59, P60 (client
  leg), P63, P67, P68, P69, P72 — all four were named blockers in the source doc's swarm-dispatch
  summary (§5 "Before W1 writing starts, raise §4-A–D with the operator").
- **FLAG for override:** recorded under the same operator-ruling authority as D8/D10/D11. The
  operator MAY override any clause here at any time; this is a recorded ruling, not a lock.

## D13. Async/tokio adoption across the agent lane — DIRECTIONAL RULING, migration NOT scoped (operator, 2026-07-20)

> **Scoping pass CONFIRMED 2026-07-20** —
> [`CONCURRENCY-ARCHITECTURE-SYNTHESIS-2026-07-20.md`](docs/design/CONCURRENCY-ARCHITECTURE-SYNTHESIS-2026-07-20.md)
> resolves this directional ruling into a per-surface recommendation, informed by two Opus research
> passes (native-concurrency/exokernel-inspired architecture; the end-to-end principle). Operator
> confirmed the narrower-than-literal-D13 scope explicitly, after asking "why not async everywhere
> with concurrency intended?" and receiving the full reasoning (function-coloring cost for zero
> interleaving benefit in a sequential agent-loop; the same-call-stack fail-closed `ToolPort` check
> as a security property an inserted executor would weaken; `ToolPort` living inside the pure-std
> kernel; nothing in dowiz today has the connection-count scale async's design point targets except
> the not-yet-built mesh layer) — resolved as **"async only where it brings value."** Ruling:
> `ToolPort`/`agent-loop`/every kernel port stays synchronous **permanently**; tokio's *only* entry
> path is the not-yet-built `mesh-adapter` networking layer, gated on the doc's proposed
> ~1,000–2,000 concurrent-peer-socket threshold (accepted as-is, not adjusted). The io_uring/
> kernel-bypass prohibition (§4.2 of the synthesis) is treated as **binding** (recorded here, not
> merely advisory) given it rests on real security evidence (io_uring's documented exploit history)
> plus D0's reliability-over-latency invariant. Implementation still awaits the phased build order
> in the synthesis's §7 — this entry authorizes the scope, not a specific PR.

While reviewing `BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md`'s open decisions,
the operator confirmed the blueprint's synchronous, thread-based voice architecture (D-V1) as
valuable, then separately stated tokio/async adoption is "a must across the system." Asked to
resolve the apparent contradiction explicitly (the existing 2026-07-15 mandate — "no tokio,
per operator mandate" — is load-bearing in `llm-adapters`, `agent-loop`'s watchdog-thread design,
and the `ToolPort` trait's synchronous `fn invoke`), the operator chose: **reverse the mandate —
adopt tokio/async broadly across the agent lane**, explicitly acknowledging this "breaks the
compile-firewall's current sync guarantees and needs its own dedicated redesign pass."

- **This is a directional ruling, not a completed design.** No migration plan exists yet. The
  2026-07-15 no-tokio mandate is superseded in principle; every place that currently documents it
  as a constraint (`llm-adapters`' module docs, the blueprint's C3, this file if it names the old
  mandate elsewhere) needs updating as the actual migration lands, not retroactively rewritten now
  on the strength of a directional answer alone.
- **Reconciling with the voice-sync confirmation (not actually contradictory once scoped):** the
  operator's two statements are compatible if read as two different layers. Real-time audio
  capture/VAD/STT/TTS is a hardware-driven constraint independent of this ruling — `cpal` (the
  audio crate the voice research identified) is callback-based on a dedicated OS thread regardless
  of what runtime the rest of the system uses; async inside a realtime audio callback is
  discouraged industry-wide, not just under dowiz's old mandate. So D-V1's audio-thread design can
  stand even after this reversal. What changes is the layer ABOVE the audio boundary — the LLM
  transport (`llm-adapters/src/transport.rs`, today hardcoded `ureq` blocking HTTP), `agent-loop`'s
  executor, and `ToolPort`'s trait signature — which this ruling opens to an async rewrite. The
  next required step is a dedicated scoping pass that states explicitly, for each surface, whether
  it goes async, and specifies exactly where the sync/async boundary sits (almost certainly at or
  above the audio thread, never inside it) — **not assumed here.**
- **NOT yet authorized by this entry alone:** any actual code change. This records the operator's
  directional intent so it isn't lost or silently reversed by a future agent defaulting back to
  the old mandate; implementation requires the scoping pass above first.
- **FLAG for override:** recorded under the same operator-ruling authority as D8/D10/D11/D12. The
  operator MAY override, narrow, or reverse this at any time; this is a recorded ruling, not a lock.

## D14. Offline-resilience + media/comms/agentic-autonomy synthesis rulings (operator, 2026-07-20)

Resolves the open-decision sections of
[`OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md`](docs/design/OFFLINE-RESILIENCE-SYNTHESIS-2026-07-20.md)
§6 and
[`MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md`](docs/design/MEDIA-COMMS-AGENTIC-AUTONOMY-SYNTHESIS-2026-07-20.md)
§10. Each item below is the ruling; unlisted items from those sections proceed per the synthesis
document's own stated recommendation (lower-stakes implementation detail, not escalated).

- **Service Worker + IndexedDB doctrine exception — RATIFIED.** Joins `<model-viewer>` (the AR/
  voice blueprint's O3 ruling) as the second sanctioned JS exception in `web/`, on the same
  "shell/infrastructure, zero application logic, zero external deps" reasoning. Phase A of the
  offline-resilience synthesis is unblocked.
- **Agent autonomy shipped default — Human-only, with Agent-assisted as an opt-in feature flag.**
  Not default-on; a hub owner turns Agent-assisted on if they want it. Matches the synthesis's own
  "Human-only default, Agent-assisted offered at onboarding" recommendation.
- **Customer-leg chat encryption (Leg B/C) — native PQ-hybrid double ratchet on dowiz's own
  KAT-gated primitives**, not the `simplex-chat` CLI sidecar. Real, substantial new crypto-surface
  work is accepted deliberately in exchange for full sovereignty (no non-Rust runtime component,
  no dependency on SimpleX's implementation) — consistent with dowiz's existing X25519+ML-KEM-768
  hybrid and the kernel's KAT-gated discipline (never a stubbed primitive). This gates Phase C2 of
  the media/comms synthesis; the ratchet design itself is not yet specified — a follow-on blueprint
  is required before C2 can start.
- **Relay topology for legs B/C — per-hub operator-configurable.** Each hub owner chooses
  self-hosted vs. third-party relay routing; no dowiz-wide default topology is imposed.
- **Staff-role vocabulary — Owner / Kitchen / Counter-Manager**, exactly the synthesis's proposed
  3-preset trio.
- **Media storage locality — local hub disk only; off-hub backup/CDN is the hub owner's own
  responsibility to connect, not a dowiz-provided default stream.** Diverges from the synthesis's
  own recommendation (which proposed folding media into `backup.rs`'s existing off-hub-backup
  design intent) — the operator's stated reason is to keep the dowiz-provided surface minimal and
  put storage-redundancy choices in the hub owner's hands. `FileBlockStore` on local disk is the
  complete v1 media storage story from dowiz's side; documentation should tell owners plainly that
  local-disk media has no dowiz-side off-hub copy unless they connect one themselves.
- **Async/tokio scope for the agent lane — see D13** (this session's same-day companion ruling):
  confirmed narrower-than-literal, "async only where it brings value."

- **FLAG for override:** recorded under the same operator-ruling authority as D8/D10/D11/D12/D13.
  The operator MAY override, narrow, or reverse any item here at any time; these are recorded

---

## D14. Operator decision-ratification batch (2026-07-20)

Source of truth for the consolidated queue in `docs/design/OPERATOR-DECISION-REGISTRY-2026-07-20.md`.
Ratified by operator in one pass. RED-LINE items (C1–C5, OD-3, OD-7, OD-8) carry a *high-level*
ruling here; each concrete code change STILL gets per-change confirmation before commit (autonomy gate).

### Tier A — Launch decisions (gate first real order M1)
- **C1 — Ordering surface: STOREFRONT + custom app.** Build the web storefront (P69) AND a custom
  app client; Telegram-first is NOT the primary channel. (Diverges from the synthesis's Telegram-first
  suggestion — operator chose storefront+custom app.)
- **C2 — Roster/provider/currency → DEPLOY CONFIG.** Move trusted-anchor roster, payment provider,
  and currency out of compile time into deployment configuration; add a real enrollment path so a
  production server does not boot rejecting every request.
- **C3 — Durability spine (default accepted):** wired persistence + versioned format + off-node
  encrypted snapshot reserved. `Mutex<HashMap>` is not launch-grade.
- **C4 — Payment rail: cash-on-delivery AND Stripe.** Real `StripeAdapter` (vendor=Stripe, geography
  operator-set, fee model = Stripe's) for card; cash-on-delivery as a pilot path. Red-line: provider
  wiring per-change-confirmed.
- **C5 — Courier delivery client: YES.** Ship a minimal installable courier `[[bin]]` for the pilot.

### Tier B — P75–P96 wave
- **OD-1 (GCRA swap): KEEP DEFAULT — not shipped.** Mutex+clock-hoist stands.
- **OD-2 (contention bench push): KEEP DEFAULT — stays local.**
- **OD-3 (bebop C3 ungated-keygen): RESOLVE.** Operator rules to resolve the ungated-keygen red
  state (do not leave the bus patch as a frozen file). Concretely: the bebop C3 freeze is lifted by
  this ruling; the bus patch may be applied as landed-work, not re-implemented. (bebop-side action.)
- **OD-4 (push unpushed main + slot_arena): PUSH.** Push the local main line above origin/main
  (incl. P57–P74, `a857cd71a` slot_arena) AND the slot_arena commit. Action: verify main green, then
  push. NOT yet executed this pass — green-verification gate must pass first (no fake-green).
- **OD-5 (P91.0 false-FIPS header): FIX.** Remove the false FIPS-203 claim from `kem.rs` header
  (comment-only, ahead of P91.1). Red-line: per-change-confirmed.
- **OD-6 (P85 closure): RETROACTIVE SIGN-OFF.** Close P85 via recorded retroactive sign-off (the
  `--no-verify` NTT gate remediation is accepted with a recorded rationale), not a fresh 3-model review.
- **OD-7 (D-1 golden digest gate): YES — propose P84.** Register P84 (golden state-digest regression
  gate). Red-line (money/FSM). Per-change-confirmed.
- **OD-8 (D-2 reputation.rs): DELETE.** Remove `reputation.rs` (courier-scoring red-line divergence).
  Red-line. Per-change-confirmed.
- **OD-9 (pq_kem NTT wire-in): WIRE.** Triple-gate now satisfiable (OD-6 retroactive sign-off + OD-3
  resolved + P82 bench required first). Wire after P82 bench evidence lands.
- **OD-10 (PPR determinism relaxation): RELAX.** Standing default REJECTED is reversed — relax PPR
  determinism (operator ruling; recorded so it is a deliberate decision, not silent adoption).
- **OD-11 (GPU field-state): START.** Begin P86/P87 build (P38 §4.2 GPU decision taken).
- **OD-12 (D-93-A privacy): RECORD BOTH.** Blueprint records both plaintext ReceiverID and blinded
  tag options; no default taken.
- **OD-13 (D-93-C broadcast): PER-RECIPIENT SIGNED COPIES.** Not wildcard-sentinel defer.
- **OD-14 (P92 proceed): AGREE.** Run D-BENCH measure-first gate; arrange mandatory independent
  adversarial review for M1 + fast-path; NO-GO if bench doesn't clear.
- **OD-15 (restore 11 research docs): AGREE.** Restore+commit the 11 recovered `docs/research/` files
  from the scratchpad `recovered/` dir.

### Tier C — Sovereign-architecture O-series
- **O1 (D5/D8 / D-series renumber): ACCEPT DEFAULT** (BLUEPRINT-P02 diff).
- **O3 (F44 dispute/escrow): STAKED SCHELLING VOTING.** (Not a courier-scoring mechanism — distinct
  from M12; a staked, game-theoretic arbiter for dispute/escrow only.)
- **O4 (F48 merge semantics): AGREE** (content-address-only for money/order; CRDT fenced out of those,
  open for knowledge-wiki).
- **O5 (D2/iroh): USE EXISTING IF iroh MISSING.** Canon claims iroh exists; it does not — amend canon
  to "quinn primary + named unlock trigger" until iroh actually lands.
- **O7 (E1/F41 hub-ring): NO-SPOF (consistent-hash).** Ratify the consistent-hash reading; literal
  star-hub is rejected (contradicts M7).
- **O9 (V1-B verifier isolation): DELEGATED TO ENGINEER.** (fresh worktree vs machine vs model family —
  operator defers the call to the implementing engineer's judgment.)
- **O19 (I-FINAL proof home): DOWIZ tools/eqc.** The proof lives in dowiz `tools/eqc`, not bebop.
- **O18a (graphics-unlock): AGREE** — stays external/environment-gated.
- **O18b (model-weights-unlock): GO.** Approve llama.cpp CPU-tier GGUF fetch + local server (GREEN on
  host); requires a DECART report + this go, not an external trigger.
- **O8 (F10 sub-hub recursion depth): DELEGATED TO ENGINEER** (numeric value).

### Tier D — parked red-line (operator: "fix, save the decisions")
- **D-money (money-leg settlement scope): DECIDED — bounded-scope pilot.** Settlement leg is scoped to
  the single-hub pilot surface only (no cross-hub auto-settlement in v1); the red-line money surface
  stays deny-by-default. Concrete adapter per-change-confirmed.
- **D-fuel (Wasmtime fuel policy): DECIDED — fuel IS the gas, kept enforce-on.** Wasmtime fuel metering
  remains the enforced compute-cap (no relaxation); document the per-call fuel budget as config.
- **D-batch (53× event-log batching): DECIDED — batch the 53× hot path.** Event-log batching of the
  53× path is approved where determinism permits (per-event commit retained for saga-critical legs);
  batching gate is DoD-tested.
  rulings, not locks.

## D13 — 2026-07-23: Quality Gates
- All tests must pass (0 failures tolerated)
- No self-referencing gates (forbidden tokens in separate file)
- All public APIs sanitize f64 inputs
- Idempotency: insert/push must guard against duplicates
- Status: ENFORCED, 2427 tests green

## D14 — 2026-07-23: Post-7.7 Quality Drive
- 131→0 kernel compiler warnings enforced
- 37-invariant fuzzer (5 real bugs found & fixed)
- 11 idempotency gates across all modifiable paths
- Native telemetry harvest ledger (HarvestLedger + 21 probes)
- All bebop proto-crypto stubs filled (ladder, ct, fips, wycheproof)
- HybridPolicy canonical deduplication
- 50-language stemmer with auto-detect (Unicode script + suffix scoring)
- 20 bebop property tests (determinism, tamper, roundtrip, DTN, wave, TriCap)
- SIGMOD battle-test (20 e2e courier tests)
- Actual wgpu texture render+readback on VirtIO GPU
- Voice DSP: FFT, MFCC, phoneme classification
- Status: ENFORCED, 2637 tests green, 0 warnings, 0 blocked
