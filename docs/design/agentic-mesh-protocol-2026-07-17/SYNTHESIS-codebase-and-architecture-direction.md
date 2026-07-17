# SYNTHESIS — Codebase Grounding + Architecture Direction for the Agentic Mesh Protocol

> Stream 6 of 6 · 2026-07-17 · planning artifact only (no code written or edited).
> Inputs: R1–R5 (this directory, read in full) + live source reads listed in §1 (every claim
> carries file:line from the actual worktree / bebop-repo, not from memory).
> Canon honored: `docs/design/ARCHITECTURE.md` §0 SCOPE RULE + M1–M12;
> `docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md` (P1–P7, RC-1–RC-4);
> `docs/design/mesh-real/MESH-REAL-PLAN.md` + `BLUEPRINTS-MESH-REAL.md` (incl. the 2026-07-16
> SQLite→sqlless/pgrust correction note at MESH-09).

---

## §1 Live-code grounding

Everything below was read in this session, in full, from the live trees. Where an R-doc made a
code claim, it is confirmed or corrected here.

### 1.1 bebop2 capability + hybrid-signature substrate (`/root/bebop-repo/bebop2/`)

- **`proto-cap/src/hybrid_gate.rs`** — `HybridPolicy::{RequireBoth, ClassicalUntilPqAudit}`
  (`:24-34`); `HybridGate::check` (`:124-209`) enforces, in fixed order: capability freshness →
  anchor-rooted UCAN-subset `verify_chain` (root ∈ roster, narrow-only attenuation, tail binds
  `subject_key`) → optional armed `RedLinePolicy` (deny-by-default on money/auth/secrets/
  migrations scopes, `:150-154`) → `RevocationSet` check on classical key, PQ-key id, and
  capability hash (`:159-168`) → real Ed25519 verify (`:171`) → real ML-DSA-65 verify (`:180-186`)
  → **verify-then-record** nonce insertion (H2 fix, `:193-206`, bounded at `MAX_SEEN_NONCES = 1<<20`).
  A self-signed frame with no anchor chain is `UnknownIssuer` (test `:380-396`). This is a working,
  RED-tested, per-frame real-time verification plane. **It is the thing to build on, verbatim.**
- **`proto-cap/src/node_id.rs`** — `NodeId::from_keys` = `SHA3-256(pq_pub ‖ classical_pub)`
  (`:46-51`, ADR-0007, no CA); fail-closed genesis loader (`load_genesis`, `:116-141`,
  `EmptyRoster` on zero anchors); `RootDelegationPolicy` defaults to `Unspecified` and
  **fails closed until the operator chooses** (`:156-183`). Identity is derived from keys, never
  seeded — the "seeded-owner JWT" mint is structurally dead (tests `:268-345`).
- **`proto-cap/src/revocation.rs`** — `RevocationSet` (`:49-121`): two monotonic namespaces
  (revoked keys incl. `pq_key_id` hashes, revoked capability hashes over canonical TLV,
  `revocation_hash` `:129-131`), `merge` anti-entropy union (`:94-98`), `gossip_payload` sorted
  wire form (`:114-120`), `drop_anchor` (`:105-107`). Surgical (nonce-sensitive) revocation is
  RED-tested (`:463-495`).
- **`core/src/pq_dsa.rs`** — confirmed: ML-DSA-65, FIPS 204, byte-exact pq-crystals port, ACVP
  KAT-verified, zero external crates, RNG-free on the crypto hot path (module header `:1-15`);
  `PUBLICKEYBYTES = 1952` (`:58`), `SIGNATUREBYTES = 3309` (`:64`); public API `keygen` (`:996`),
  `sign` (`:1053`), `verify` (`:1060`), plus `derive_pq_seed` for the hybrid identity (`:1033`).
  This is exactly the implementation R4 §0–§1 budgeted; R4's flag stands: **it has no in-repo
  benchmark** — the 0.2–1 ms/verify prior is unmeasured (R4 §1 P0 action item).
  `proto-cap/src/signed_frame.rs` wires it: `sign_pq` (`:196`), `verify_pq` (`:229`),
  `pq_sig: Option<Vec<u8>>` (`:97`).

### 1.2 dowiz kernel (`/root/dowiz-agentic-mesh/kernel/src/`)

- **`event_log.rs`** — pure-Rust FIPS-202 `sha3_256` (`:30`); `MeshEvent` with content-id
  `SHA3-256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)` (`:134-156`); `EventStore` trait whose
  `insert` **returns `Result<(), StoreError>`** (`:188` — the RC-3/P4-V2 fail-open hole from the
  Hermetic audit is fixed on this branch; `StoreError::{Open,Write,Flush,Sync}` `:167-176`);
  `CommitError` keeps the Law-reject pole distinct from the store-fault pole (`:263-268`);
  `commit_after_decide` (decide-before-persist, duplicate = structural no-op, `:339-361`);
  `commit_after_decide_drift_gate` (`:389-419`) — spectral `classify_drift` runs **before**
  `decide`, `Unstable (ρ>1)` rejected pre-persist in the DEFAULT regime, lifted under
  `intervention` per operator directive. Fault-injection tests prove no fabricated `Committed`
  on a failed durability barrier (`:701-745`).
- **`hydra.rs`** — `OrganismState::{Live,Locked}` (`:78`), `integrity_check` (`:180-195`),
  `boot_verify` (`:253-265`), `Hydra::commit` as the single hidden surface (`:214-245`),
  `BreachAlert` (40-byte fixed layout, `:88-120`), `witness_event_id` deterministic no-trust
  re-derivation (`:135-148`), `raise_breach_alarm` WORM self-witness (`:287-318`),
  `ingest_peer_breach` hub convergence (`:332-348`), and the std-only durable
  **`FileEventStore`** — JSONL append + fsync, typed `StoreError`, in-memory index advances only
  after `sync_all` (`:743-902`). **Assessment for this arc's question (usable foundation for
  general real-time verification?): no — it is deliberately narrower.** The G9 machinery carries
  only `node_id + group_size`, bypasses `decide`/drift via `append_raw` (`:313`), has no
  capability scoping and no payload semantics, and per Hermetic Gender V-3 the alarm still
  originates in the audited node's own `integrity_check`. What IS reusable from it: (a) the
  receiver-side pattern — *re-derive the claimed content-id deterministically from the claim's
  own fields and check it against the claimant's WORM log without trusting the sender*
  (`witness_event_id`, test `:626-677`) — which is precisely the shape a work-receipt check
  needs; (b) `ingest_peer_breach` as the market-level "stop trading with a burnt peer"
  convergence R5 §1 identified. General per-message verification is `HybridGate::check`, not hydra.
- **`token_bucket.rs`** — F33 bucket (`:27-89`): fractional refill, falsifiable bound
  (granted ≤ capacity + rate·elapsed), degrade-closed `try_acquire → false` (`:46-63`). Confirmed
  as R3 §5 describes: token-denominated, mathematically bounded, degrade-closed — ahead of
  LangGraph/CrewAI step-count guards. **It is a rate limiter that heals with time; it is not an
  exposure limiter that heals only on settlement** (R5 §3's distinction — the gap this arc fills).
- **`ports/llm.rs`** — the port pattern to generalize: zero-HTTP compile firewall (header `:3-7`),
  fail-closed `Caps { chat, embed, rerank, tool_calling }` (`:18-23`), `CachePolicy` as a type not
  a convention (`:79-87`), `Usage::cost()` → bucket units (`:99-102`), `LlmBackend` trait with
  typed `LlmError` and honest `health()` (`:154-169`). Per M5, backend choice is consumer-side
  config, never a kernel recompile.

### 1.3 llm-adapters (`/root/dowiz-agentic-mesh/llm-adapters/src/`) — the existing bridge pattern

- **`dispatch.rs`** — `Dispatcher<B: LlmBackend>` holds `Arc<TokenBucket>`; `dispatch` acquires
  `cost = max_tokens.max(1)` **before** the call, refusing with typed
  `DispatchError::BudgetExceeded` (`:78-118`); every call emits a `TrackRecord` harvest row
  priced by returned `usage.total_tokens` (`:100-110,135-148`).
- **`quirks.rs`** — one `Quirks` struct of per-backend wire deltas (`:12-28`) with
  `ollama()/vllm()/managed_api()` constructors; **`transport.rs`** — one `OpenAiCompatTransport`
  holding zero vendor knowledge (`:16-19`), typed error mapping (`:83-108`); **`cache.rs`** —
  `CachingBackend<B, S: BlockStore>` keyed by `sha3_256` of the BTreeMap-canonicalized request
  (`:57-81`), `NoCache` store to disable structurally (`:156-168`). The composition
  `Dispatcher<CachingBackend<Adapter>>` is the proven stack: **one transport, per-adapter quirks,
  content-addressed cache, budget-bounded dispatch.** This is the closest existing precedent for
  "any node bridges in its own agent," and the synthesis in §3 generalizes exactly this, not
  something new.

### 1.4 Design canon (this worktree)

- **`ARCHITECTURE.md`** — exact wording checked. M6: "ZERO protocol dependencies: no external
  crate at the wire/trust boundary (proto-cap, pq_kem, matcher are zero-dep std-only)."
  M12: "Capability model (proto-cap, in-repo …): ML-DSA-signed, fail-closed, nonce-replay,
  expiry, RevocationSet, red-line deny (auth/money/secrets/migrations). Per-agent scope."
  **Correction to R1/R2's shorthand:** M11's literal text is "Living-organism experiment =
  UNBOUNDED: no caps, only kill-switch + configurable access"; the *fork-freedom* wording lives
  in the §0 SCOPE RULE ("every hub is a sovereign Hydra … it may fork, self-gate locally, ignore
  the upstream gate, change its DB/model/API/port") and F-series anchors. The substance the
  R-docs relied on (divergence is legitimate; no governing layer above hubs except the wire
  format) is correct; the anchor label was imprecise. Also load-bearing here: M3 (QRNG =
  optional, operator-enabled, mixed into nonce/ephemeral-key gen), M5 (hub intra-autonomy), M9
  (kill-switch only), F2 (new bridge ports: deny-by-default + rate-limit), F10 (sub-agent
  recursion: max-depth cap), F44 (disputes: arbitration + escrow).
- **`HERMETIC-ARCHITECTURE-PRINCIPLES.md`** — the seven principles as engineering rules; RC-2
  ("verification organs without independent teeth" — the self-certification class every design
  choice below must be checked against) and the note that the mesh capability path is the
  codebase's *positive* Gender instance ("sign needs the private seed, verify needs only public
  data — structural independence via asymmetric crypto," §1 P7).
- **`mesh-real/MESH-REAL-PLAN.md` + `BLUEPRINTS-MESH-REAL.md`** — the mesh substrate this arc
  extends, not replaces: MESH-01..14, wire→Law→money before state, event-sourcing-not-CRDT for
  money/orders, matcher rendezvous/HRW assignment (MESH-05), Sync·Pull anti-entropy (MESH-07),
  and the **corrected storage stance** (MESH-09 correction, 2026-07-16): sqlless
  content-addressed `BlockStore` + JSONL `FileEventStore` is the MAIN path; **pgrust is the
  uniform SQL-fallback/backup, never SQLite**.
- **`BLUEPRINT-E3`** — the connection the task asked about is real but modest: E3 Phase-A's
  discipline — *configuration as an enumerable, bounded, single-axis lattice with allowlists,
  never free-form* (E3 §2ii) — is the correct discipline for agent-bridge manifests too (a
  bridged agent declares bounded capability/config axes, not arbitrary strings), and any future
  self-tuning of bridge config inherits E3's advisory-only-until-`key_V` gate. No deeper
  structural dependency exists.

---

## §2 Resolution of the five open questions from the earlier (unadopted) draft

### 2.1 Market-based micro-negotiation / rapid-fire auctions → REJECTED AS DRAFTED; narrow sealed-batch form permitted, and mostly unnecessary

R1 §5 found the near-exact structural precedent: Beanstalk (~$182M) — "any mesh mechanism where
transient, acquirable weight decides an outcome is Beanstalk-shaped" — plus the measured
steady-state of transparent low-latency auctions (72,351 sandwich victims, ~$87.7M in one
half-year). R1's verdict is "rejected-unless-sealed." R5 §4 supplies the constructive fix from
finance: Budish–Cramton–Shim frequent batch auctions — discrete windows, sealed bids, uniform
clearing — plus "never use raw arrival time as tie-breaker," and R5 §6 adds the spoofing
exclusion (offers must be binding, with a forfeited deposit on cancel).

**Concrete resolution — "safe micro-negotiation" means all five of these, or no auction at all:**

1. **Default: no auction.** Contested work is assigned by the already-designed deterministic,
   coordination-free rendezvous/HRW hash (MESH-05, `matcher.rs`) under capability authorization.
   Price discovery is only needed where price genuinely varies; assignment mostly doesn't need it.
2. **Non-transient weight:** participation weight = held capabilities under an anchor-rooted
   delegation chain (`verify_chain`). Capabilities cannot be flash-borrowed or bought
   mid-transaction — issuance is a delegation act by an enrolled anchor, not a market purchase.
   This kills the Beanstalk class structurally (R1 §5(b)).
3. **Sealed bids via commit-reveal on the event log:** commit = content-id of the bid appended to
   the bidder's WORM log inside the window; reveal after window close; a reveal whose hash
   mismatches its commit is discarded. No in-flight bid visibility ⇒ no sandwiching (R1 §5(a)).
4. **Batch window ≫ network jitter (order 1 s), uniform clearing, tie-break = deterministic
   function of committed content (hash), never arrival time** (R5 §4).
5. **Binding offers:** a revealed bid is a signed commitment; withdrawal forfeits a deposit
   (escrowed via the §2.4/§3 settlement primitive) — the spoofing exclusion (R5 §6).

And per R1 §3: the moment any such mechanism produces a *price* another decision reads, that
price must be non-atomically-manipulable (windowed/medianized) — a gating precondition, written
now, before any market exists.

### 2.2 Inline self-auditing (agent generates its own proof-of-transition) → REJECTED as a standalone mechanism; replaced by counterparty-verifiable signed receipts

An agent minting its own "proof" that its transition was correct is the textbook RC-2 shape
(Hermetic P7: "the check reduces to the claim restating itself"), and R3 §3's measurement says
this is not hypothetical — MAST attributes 21.3% of multi-agent failures to unverified claims,
and *no surveyed framework verifies agent claims cryptographically*. R3 §3 also gives the
pragmatic middle path ("Tool Receipts, Not Zero-Knowledge Proofs"): don't prove the model's
computation (zkML is 1000×+ overhead — rejected; TEE imports hardware-vendor trust — optional
tier only); **prove the envelope**. R2 §6 (CapTP lineage) and R3 §4 (AP2's Intent→Cart→Payment
signed mandate chain — "the single best pattern in this whole survey") both point to the same
fix: the proof must be a **capability-bound signed artifact that a DIFFERENT party verifies
using only public data**.

**Concrete resolution — the `WorkReceipt`:** a canonical-TLV structure binding
`(capability revocation_hash, input content-id, output content-id, declared budget consumed,
nonce, expiry-tick)`, carried as a `SignedFrame` (hybrid ML-DSA⊕Ed25519, `RequireBoth`), checked
by the **counterparty** through the existing `HybridGate::check` (chain → red-line → revocation
→ both signatures → nonce), then appended to *both* parties' WORM logs via
`commit_after_decide`. Verification consumes only public data (the P7-positive property the
Hermetic audit already certified for this path) and re-derives the content-ids exactly the way
`BreachAlert::witness_event_id` does (`hydra.rs:135-148`) — deterministic, no trust in the
sender. Hydra's peer-witness system is the *template* for that check but is not the mechanism:
it solves breach broadcast (node_id + group_size, no capability scope, bypasses decide). The
honest limit is stated in §4: a receipt proves *authorized delivery of specific bytes under a
specific grant and budget* — it does not prove semantic quality of the work; that judgment
stays with the paying party (and is why settlement is atomic, §2.4).

### 2.3 Speculative/optimistic execution with local challenges → REJECTED; verify-before-persist stands

R1 §6's record: no permissionless fraud proof on any optimistic rollup mainnet for ~3 years;
first successful mainnet fraud proof ever = Kroma, April 2024; documented challenger-censorship
and economic-non-viability ("Hollow Victory") failure modes; and the structural observation that
optimistic execution is *degrade-open* — in direct tension with this architecture's
degrade-closed posture. R2 §2 independently rejects fraud-proof designs for a sparse mesh (the
honest-watcher liveness assumption cannot be met) and names the principle that survives:
verification asymmetry — the asserter pays, every consumer checks in O(ms).

The latency argument for speculation then has to clear R4 §5's numbers, and it doesn't: hybrid
dual-leg verification is ~0.1–1 ms/message (~10³ msg/s/core now, ~10⁴ optimized), while mesh
network RTT is 10–100 ms — verification is **1–2 orders of magnitude below the latency floor
the network already imposes**, and dowiz's actual event rates are orders of magnitude below
1,000 msg/s. Speculating to avoid a 0.3 ms check while waiting 30 ms for the wire is overhead
avoidance for a cost that doesn't matter — while importing the challenge-window machinery,
funded-challenger economics, and censorship-resistance obligations R1 §6 shows nobody has made
work. The kernel already implements the superior alternative: `commit_after_decide_drift_gate`
(`event_log.rs:389`) is validity-first — verify BEFORE persist. **Resolution: keep it; build
nothing optimistic.** One honest caveat carried from R4 §1: the pure-Rust `pq_dsa` verify is
unmeasured — the P0 criterion bench on the deployment host is a prerequisite for stating any
per-message latency budget as an acceptance criterion.

### 2.4 Eventual consistency vs fast finality → the existing structure already answers it; the ONE genuinely new piece is pairwise atomic settlement

R1 §2's prevention rule: don't promise global fast finality; make finality *local and explicit* —
"an event is final for a participant when they hold the signatures they require." That is
already this architecture: each node's log commits locally via decide-before-persist
(local-first, MESH-06), divergence is legitimate (SCOPE RULE / M11-class fork freedom), and
R2 §4 confirmed the mesh "starts from the escape-hatch side" — there is no sequencer to
decentralize and no global head to reorg. Gossip surfaces (revocation `merge`, Sync·Pull
anti-entropy, telemetry/status, breach convergence) are eventually consistent **by design** and
correctly so.

So the "tiering decision" mostly dissolves structurally. The one place eventual consistency is
genuinely NOT acceptable is the one R5 §5 names: **any two-party exchange of value/work** —
neither party may end up having performed without being paid (or paid without delivery). The
answer is not a consistency tier but a primitive: **HTLC-style delivery-versus-payment** —
B's signed work-receipt and A's signed budget/capability transfer locked under the same
hashlock; claiming one side reveals the preimage that unlocks the other; expiry-ticks refund
both on stall. Herlihy (PODC 2018) proves the guarantee: no conforming party ends up worse off
under any deviating coalition. Known honest caveats carried from R5 §5: the timelock asymmetry
gives one side a short-lived free option (~2–3% implicit premium on volatile value), and
capital/budget is griefed-locked until timeout on abort — both acceptable for small
task-denominated exchanges, both to be stated in the blueprint, not hidden. Settlement finality
= "both halves in both WORM logs." Everything else stays anti-entropy.

### 2.5 Priority-tagged dispatcher → NO new kernel machinery; composition of what exists

R5 §3's finding is that finance's containment is *per-counterparty exposure*, not a smarter
queue; R5 §1's refinement is a graduated response ladder (limit-state before halt) and a
designed re-open; R3 §5's recommendation is *hierarchical budgets* (per-peer / per-capability /
per-task envelopes), explicitly noting that industry sophistication is "in layering, not in a
better bucket." The existing `TokenBucket` + `Dispatcher` composition already supports all of
this shape: priority = **which bucket a request draws from**. Concretely: a
`BTreeMap<(PeerId, CapabilityClass), TokenBucket>` of nested envelopes on the dispatch path —
a low-priority peer exhausts *its* envelope and gets the existing typed `BudgetExceeded`
refusal, without starving the node's aggregate (the "ten looping agents = $5,000" mesh case,
R3 §5). A priority *flag* on the wire is therefore just an envelope selector, checked against
the sender's capability scope (a peer cannot self-assign a priority its capability doesn't
grant — otherwise the flag is a self-certified fast lane, RC-2 again). What IS new — and small —
is R5 §3's `ExposureLedger` (§3.3 below), because outstanding-commitment stock is the one thing
no clock-healing bucket can bound. Verdict: **priority dispatch composes; exposure is the new
primitive; no queue machinery enters the kernel.**

---

## §3 The architecture direction — one shape, three new layers

**Name: the Agent Exchange Plane — three thin layers over the existing mesh substrate.** The
core insight from R3 §8 is that everything surveyed (MCP, A2A, x402, AP2, TEE) punts the
decentralized PQ trust plane — "that part must be built, and the survey says it is the only
part that must be." Everything else here is deliberate reuse.

### 3.1 Layer 1 — `AgentBridge`: the generalized bridge port (bring-your-own-agent)

The `LlmBackend` pattern (`ports/llm.rs`), generalized. A new kernel port —
**`AgentBridge`** — with the same compile firewall (zero HTTP/serde in the kernel port module;
all wire code in an adapter crate, mirroring `llm-adapters`):

- **`AgentManifest`** — the discovery artifact. Shape borrowed from MCP's three-primitive
  discovery grammar + A2A's Agent Card (R3 §1–2), with the trust plane those protocols punt:
  the manifest is **mandatorily signed** (hybrid, canonical TLV — never optional JWS), carries
  the agent's `NodeId`-anchored identity, a fail-closed capability set (the `Caps` pattern:
  what the agent does NOT declare is `false`), the **cost unit and budget denominations** it
  will be metered in, and bounded/enumerable config axes only (E3 Phase-A lattice discipline —
  allowlists and discrete domains, never free-form strings). Per R2 §1 (ERC-4337's actual
  innovation), the manifest also declares the actor's *validation policy* as data
  (`RequireBoth` today; threshold-classical ⊕ single-PQ later per R2 §3) — verifiers evaluate
  the declared policy rather than one hard-coded scheme, with `RequireBoth` as the floor the
  policy may narrow but not relax.
- **Admission** = `HybridGate::check` on the manifest frame (anchor-rooted chain, revocation,
  red-line) + sandbox tier assignment per the existing
  `SandboxTier::{WasmComponent, NativeProcessRequiresKvm}` split (`kernel/src/isolation/`,
  confirmed by R3 §6 as the industry-converged architecture): WASM component as the default
  isolation boundary (integrity boundary, NOT confidentiality — node signing keys never share
  the guest address space), microVM for native-code frameworks. Wasmtime fuel wires to the
  agent's `TokenBucket` envelope. Deny-by-default + rate-limit per F2; recursion depth capped
  per F10.
- **Adapter crate** (`agent-adapters`, sibling of `llm-adapters`): one generic transport,
  per-framework `Quirks` (a LangGraph agent, a CrewAI agent, an MCP server, a bare binary — each
  is a quirks profile, not a new protocol), `Dispatcher<Caching<…>>` composition reused
  verbatim; every bridged call emits the existing `TrackRecord` harvest row.

### 3.2 Layer 2 — `WorkReceipt` + `Settlement`: the exchange primitives

As resolved in §2.2/§2.4. Two proto-cap-level artifacts, both riding `SignedFrame` and both
landing in the WORM event log through `commit_after_decide`:

- **`WorkReceipt`** — canonical-TLV, capability-bound, counterparty-verified (never
  self-certifying); content-ids re-derived receiver-side per the `witness_event_id` pattern.
  New `Resource`/`Action` scope variants (additive, closed-enum-pinned, exactly as MESH-03 did
  for `Order`/`Claim`).
- **`Settlement`** — the HTLC-style pairwise DvP: hashlocked pairing of receipt and
  budget/capability transfer, expiry-tick refunds, both halves in both logs. Settlement events
  are what decrement Layer 3's exposure ledger. Money-scoped settlements remain behind the
  armed red-line gate (`new_redlined`, deny-by-default) and integer-money law (S9) — a
  validly-signed money settlement still requires operator allow-listing. Disputes escalate per
  F44 (arbitration + escrow), not per reputation.

### 3.3 Layer 3 — `ExposureLedger`: per-counterparty containment

R5 §3's sketch adopted nearly as written: a typed
`ExposureLedger { per_peer: BTreeMap<PeerId, Commitment>, per_peer_cap, aggregate_cap }` whose
`try_commit` runs **pre-persist in the same commit-path slot as the drift gate** (15c3-5's
"direct and exclusive control, in the path, not advisory") and whose balances decrement **only
on settlement events** (§3.2), never by a clock. Composes with, and does not replace,
`TokenBucket` (flow vs stock). Two refinements from R5 §1 carried in: a **graduated
limit-state** — a named intermediate pole (pause new commitments, let in-flight settle) between
accept and `Locked`, satisfying Hermetic P4's "every intermediate degree is a named variant on
one axis" — and a **defined auto-reopen condition** for exchange-anomaly halts (tamper-`Locked`
still requires owner re-seed/M9). Peer-breach convergence reuses `ingest_peer_breach`: a burnt
peer's exposure cap drops to zero.

### 3.4 Reused verbatim (named)

`HybridGate::check` + `HybridPolicy` + `RedLinePolicy` · `verify_chain`/`AnchorRoster`/
`Delegation` · `RevocationSet` (+ gossip/merge) · `NodeId`/`load_genesis`/`RootDelegationPolicy`
· `pq_dsa` ML-DSA-65 + `signed_frame` · `sha3_256`/`MeshEvent`/`EventLog`/
`commit_after_decide(_drift_gate)` · `FileEventStore` (JSONL, fsync-gated) + `BlockStore` ·
`TokenBucket` + `Dispatcher`/`Quirks`/`CachingBackend`/`OpenAiCompatTransport` (as the pattern
and partially as code) · `SandboxTier` + fail-closed KVM probe · `matcher.rs` rendezvous/HRW
assignment · `Hydra` G9 breach machinery (for breach, unchanged) · `EntropyRng` + the
QRNG-seeded-never-replace doctrine (R4 §7).

### 3.5 Rejected from the earlier draft (with citations)

| Draft idea | Verdict | Why (R-doc) |
|---|---|---|
| Rapid-fire transparent auctions / free-form micro-negotiation | **Rejected as drafted**; only the §2.1 sealed-batch form, and only if HRW assignment is shown insufficient | R1 §5 (Beanstalk, sandwich-attack steady state, "rejected-unless-sealed"); R5 §4 (FBA), §6 (spoofing) |
| Inline self-generated proof-of-transition | **Rejected**; replaced by counterparty-verified `WorkReceipt` | Hermetic RC-2; R3 §3 (MAST 21.3%, tool-receipts paper); R2 §6 (CapTP); R3 §4 (AP2 mandates) |
| Speculative/optimistic execution + local challenges | **Rejected outright** | R1 §6 (3-yr fraud-proof record, censorship, Hollow Victory; degrade-open); R2 §2 (no honest-watcher assumption); R4 §5 (verify ≪ RTT) |
| Eventual-vs-fast-finality tiering as new machinery | **Dissolved structurally**; one new pairwise DvP primitive instead | R5 §5 (HTLC/Herlihy); R1 §2 (finality local & explicit); R2 §4 |
| Priority-flag dispatcher as new kernel machinery | **Rejected**; envelope-selection composition over existing bucket + new exposure ledger | R5 §3 (exposure ≠ rate), §1 (graduated ladder); R3 §5 (hierarchical envelopes) |
| (From R-docs, pre-emptive) per-message ZK, BLS aggregation, PQ threshold signing, GG18/GG20 MPC, reputation anywhere | **Rejected / deferred** | R2 §2/§7/§3 anti-recommendations; R4 §3/§6; R1 §4 (Cheng–Friedman impossibility — the standing NO-COURIER-SCORING confirmation) |

---

## §4 Requirement satisfaction — checked, not asserted

### 4.1 Local spectral-indexed databases (sqlless; pgrust = fallback only)

Every new artifact (manifest, receipt, settlement halves, exposure snapshots) is a `MeshEvent`
in the content-addressed WORM log (`FileEventStore` JSONL + fsync) and/or a block in
`BlockStore` — the same sqlless pattern the kernel already runs, matching the MESH-09 correction
verbatim (append-only sqlless store; pgrust only if a SQL shape is genuinely needed, never
SQLite). Retrieval over these events reuses the existing kernel retrieval organs
(`kernel/src/retrieval/`: bm25, diffusion, ppr, spine — the spectral index) — receipts and
manifests are just more content-addressed rows to those readers; no new database enters the
design. Honest gap: an *index schema* for "find all open commitments with peer X" is a read
projection to be specified in the Layer-3 blueprint, not something that exists today.

### 4.2 PQ crypto + ANU quantum randomness

All new frames are hybrid-signed and verified under `RequireBoth` (real ML-DSA-65 + real
Ed25519, both legs, per `hybrid_gate.rs:180-186`); identities are `H(pq_pub ‖ classical_pub)`;
revocation covers both legs (`pq_key_id`). Per R4 §3, the design budgets **one full ML-DSA
verify per message** (no PQ aggregation exists to design around) and treats the ~3.4 KB/message
hybrid signature as the real PQ tax — batching is an envelope-design question (one hybrid
signature over a settlement batch), not an aggregation assumption. QRNG per R4 §7 and M3
(optional layer): verification consumes **zero** randomness by construction; ANU AQN bytes are
fetched on a slow background cadence and **mixed, never replacing** —
`new_state = SHAKE256(old_state ‖ qrng_bytes ‖ getrandom_bytes)` into the fail-closed
`EntropyRng`; the internal fallback when ANU is unreachable is `EntropyRng`/`getrandom` (the
default state, not an incident); the kernel's deterministic simulation PRNG (`kernel/src/rng.rs`)
is red-lined out of cryptographic use. The QRNG can never sit on a hot path (100 req/month free
tier; ~250–330 ms RTT from Hetzner-EU — R4 §7) and the design never asks it to.

### 4.3 The seven Hermetic principles, one honest sentence each

- **P1 Mentalism:** each layer ships as a blueprint with falsifiable done-checks before code
  (this document + §5's wave), and every artifact cited here was resolved live (file:line) —
  the cite-with-probe discipline RC-1 demands.
- **P2 Correspondence:** one signing/verifying primitive (`SignedFrame`+`HybridGate`) serves
  manifests, receipts, and settlements; one hash (`sha3_256`) serves content-ids, cache keys,
  and hashlocks; the `AgentBridge` port is the *same* seam pattern as `LlmBackend`, not a second
  mechanism — and where the receipt check re-derives ids, it copies `witness_event_id` rather
  than inventing a sibling.
- **P3 Vibration:** every new cadence gets a named, single-authority, tested rate — batch-window
  length, settlement expiry-ticks, exposure-ledger reopen condition, QRNG reseed cadence — with
  the `DT_STABLE` mirror-pin treatment wherever a value crosses the kernel↔adapter seam.
- **P4 Polarity:** all new checks return typed two-pole results in the existing enums
  (`CapError`, `CommitError`, `DispatchError`); the exposure ledger adds its intermediate pole
  (limit-state) as a named variant on one axis, and no `unwrap_or`-style pole collapse is
  permitted on any settlement or money path.
- **P5 Rhythm:** honest gap — HTLC expiry sweeps and QRNG reseeds are periodic processes, and
  this repo currently has **no structurally-guaranteed scheduler** (the Hermetic audit's dead-
  pendulum finding); the Layer-2 blueprint must specify what fires the timeout sweep (in-repo
  timer unit, or sweep-on-commit piggyback) rather than assuming a cron that history shows
  doesn't fire.
- **P6 Cause-and-Effect:** receipts, hashlocks, and batch tie-breaks are pure functions of
  canonical bytes (TLV, fixed-order concatenation, BTreeMap folds); the only randomness is
  named, seeded, and port-isolated (`EntropyRng`), and no wall-clock enters any content-id
  (expiries are monotonic ticks, as `HybridGate::check(now)` already models).
- **P7 Gender:** the receipt/settlement design exists *because* of this principle — every
  generative claim (work done, budget spent, bid placed) is paired with a verifier the claimant
  cannot supply (counterparty verification on public data); honest residual: a receipt proves
  authorized delivery, not work *quality*, and the repo-level independent verifier (`key_V`)
  remains unbuilt, so any future auto-acceptance of bridged-agent output inherits E3's
  advisory-only gate.

---

## §5 What this arc needs next — the blueprint wave

Four blueprints, scoped to be independently executable. Each must carry RED-first contracts and
the M-anchor + Hermetic checklists; none may write code before its blueprint lands.

1. **B1 — `AgentBridge` port + signed `AgentManifest` + admission.** Scope: kernel port module
   (trait + value types, zero-HTTP firewall), manifest TLV schema + mandatory hybrid signature,
   admission path through `HybridGate` (incl. new manifest `Resource`/`Action` scopes), sandbox
   tier assignment + Wasmtime-fuel↔`TokenBucket` wiring, `agent-adapters` crate skeleton with
   one reference `Quirks` profile (an MCP-server bridge is the highest-leverage first profile
   per R3 §1). Out of scope: transport (MESH-09 owns it), any auto-tuning (E3 owns it).
2. **B2 — `WorkReceipt` + `Settlement` (pairwise DvP).** Scope: receipt/settlement TLV schemas,
   hashlock + expiry-tick protocol (Herlihy conformance argument written down, free-option and
   grief-lock caveats stated with bounds), counterparty verification path (the
   `witness_event_id`-style re-derivation), commit-path integration via `commit_after_decide`,
   red-line arming for money-scoped settlements, and — per P5 — the explicit firing mechanism
   for timeout sweeps. Out of scope: multi-party swaps (two-party only in v1), disputes beyond
   the F44 escrow hook.
3. **B3 — `ExposureLedger` + hierarchical budget envelopes.** Scope: the typed ledger, per-peer/
   per-capability envelope map over `TokenBucket`, pre-persist `try_commit` in the drift-gate
   slot, graduated limit-state + defined auto-reopen, settlement-driven decrement (depends on
   B2's event shape — the one inter-blueprint dependency; B3 can land its rate-envelope half
   before B2 finalizes), read projection for open commitments, burnt-peer zeroing via
   `ingest_peer_breach`. Out of scope: any pricing/market logic.
4. **B4 — Crypto ground-truth bench + batching (R4's P0).** Scope: criterion benches of
   `pq_dsa::verify` / `sign::verify` / full `HybridGate::check` on the deployment host
   (pins the acceptance-criterion numbers every other blueprint may cite); Ed25519 batch
   verification with the cofactored-equation consistency pin (R4 §3); envelope size budget for
   the ~3.4 KB hybrid tax (when to sign batches vs single events). Out of scope: AVX2 porting
   (recorded as the known ~3× upgrade path, triggered only if measured throughput demands it).

**Deliberately NOT blueprinted now:** the sealed-batch auction (§2.1) — its five preconditions
are recorded here as design law, but no blueprint until a concrete allocation problem is shown
that rendezvous/HRW assignment cannot solve; checkpoint validity proofs (R4 §6 — settlement-layer
future option, never latency-coupled); threshold-FROST-Ed25519 custody (R2 §3 — composes later
without protocol change); TEE attestation tier (R3 §3 — optional, imports vendor trust).

---

*End of synthesis. No code was written or edited. All file:line citations resolved live on
2026-07-17 in `/root/dowiz-agentic-mesh` (branch `feat/agentic-mesh-protocol-2026-07-17`) and
`/root/bebop-repo/bebop2`.*
