# WEB3 SYNTHESIS — INVISIBLE, AGENTIC, LOCAL INFRASTRUCTURE (2026-07-17)

> **Research/design artifact only. No code is written or edited by this document.**
> Branch: `feat/harness-llm-backend` (`/root/dowiz`).
>
> **Operator directive this document executes (recorded as directive, not inferred):** synthesize
> the web3/agentic research so its documented mistakes are not repeated, and make the technology
> **invisible and seamless for users** — explicitly NOT a CRM, NOT typical SaaS UX (dashboards,
> forms, settings panels as the primary interface); that category is "no longer relevant." What
> matters is fundamentally correct, reliable infrastructure for (a) agentic infrastructure,
> (b) local AI, (c) local databases — connected through adapters/bridges/thin transition layers,
> never by absorbing an external ecosystem wholesale.
>
> **Relation to prior work (read in full this session):** this document builds ON
> `dowiz-agentic-mesh/docs/design/agentic-mesh-protocol-2026-07-17/` R1–R5 +
> `SYNTHESIS-codebase-and-architecture-direction.md` + `AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md`,
> and on today's `docs/design/LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md` and
> `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md`. The mesh SYNTHESIS read R1–R5 through
> the axis "what protocol must the mesh speak" (its answer: the Agent Exchange Plane —
> AgentBridge / WorkReceipt+Settlement / ExposureLedger). **This document reads the SAME evidence
> through a different axis: what product/interface shape the failures condemn and the successes
> license.** It duplicates neither the mesh blueprints nor the local-AI gap register; §2 and §4
> mark explicitly where it extends prior conclusions and where (in one confined case, §6) it
> surfaces a tension for the operator to rule on. No divergence from any adopted ADR/M-anchor was
> found or introduced.
>
> **Non-goals:** no product/GTM roadmap; no migration plan for the existing TypeScript SaaS surface
> (apps/web, apps/api) — per the standing roadmap ground-truth, `origin/main` is the frozen anchor
> and the canonical Rust stack lives on feature branches; how (or whether) the TS surface is
> retired is an operator-direction call this document names but does not make. No new mesh
> blueprints — the four existing ones (B1–B4) stand. No code.

---

## §1 Ground truth (live-verified this session, 2026-07-17)

### 1.1 Documents read in full

R1–R5 + SYNTHESIS + CONSOLIDATED (mesh worktree, all dated today), HARNESS-LLM-BACKEND.md,
LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md, AGENTS.md (Detailed Planning Protocol, Integration
Decart Rule, Anu/Ananke doctrine). HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md was read §1 +
section map (relevant to the §6 Anu check; its D1–D5 addendum headers verified, bodies skimmed —
flagged honestly in §7).

### 1.2 Live repo checks (commands run this session, this branch)

- **Local-AI port + adapter exist as documented:** `kernel/src/ports/llm.rs` + `ports/mod.rs`
  present; `llm-adapters/src/` contains exactly `{lib,transport,quirks,ollama,cache,compose,
  dispatch,telemetry}.rs`. This is the reference adapter shape the operator named.
- **Embedded-DB ground truth (grep `sqlite|sled|redb|rocksdb|lmdb|pgrust`, case-insensitive,
  over `kernel/` and `engine/src/`):** ZERO hits for sqlite/sled/redb/rocksdb/lmdb as
  dependencies anywhere. What exists instead:
  - `kernel/src/hydra.rs:743` — `FileEventStore`: std-only durable JSONL append + fsync; its
    in-memory index advances only after `sync_all`; fault-injection tested (`:1008` "no longer
    swallows").
  - `kernel/src/backup.rs:29` — `BlockStore` trait (content-addressed, `Hash = [u8;32]`,
    idempotent put) + `MemStore`; `BackupOrgan<S: BlockStore>` at `:111`.
  - `kernel/src/event_log.rs:179-207` — `EventStore` trait; `MemEventStore` as the offline
    stand-in; module header (`:1-19`) states the production upgrade path is a `PgEventStore`
    wired in the node binary, NOT in the kernel.
  - `kernel/Cargo.toml:24-29` — `pgrust = ["dep:sqlx", "dep:tokio"]`: the ONLY SQL adapter,
    optional, non-default, feature-gated; `kernel/src/retrieval/memory_store.rs:118-129` is the
    real sqlx-backed living-memory adapter behind that flag.
  - Canon: the MESH-09 correction (2026-07-16, cited via mesh SYNTHESIS §1.4, re-confirmed there
    this session): **sqlless content-addressed `BlockStore` + JSONL `FileEventStore` is the MAIN
    path; pgrust is the uniform SQL fallback/backup, never SQLite.**
- **`engine/src/` has no embedded-DB usage at all** (zero grep hits).

### 1.3 Facts carried from today's already-verified documents (not re-measured here)

Ollama 0.30.9 live with four models; measured decode 4.8–10.5 tok/s, prefill ~636 tok/s
(prefill:decode ≈ 130:1) on this 8-vCPU/30GiB no-GPU host (LOCAL-AI §1.1–1.2, probes run today).
`AgentBridge` (signed `AgentManifest`, admission, `SandboxTier` caging, Wasmtime fuel tranches)
is BUILT on the mesh branch (`/root/dowiz-agentic-mesh/kernel/src/ports/agent/`, landed
`f30189262` per memory + LOCAL-AI §1.4). B2 `WorkReceipt`/`Settlement` and B3's ledger half are
blueprint-only. The kill-switch (P10) is grep-verified absent (LOCAL-AI §4 G8).

---

## §2 The re-reading: web3 failures as product-shape failures

R1/R5 root-caused each incident technically. Read the same incidents for *where the design put
human attention and visible surfaces*, and a second, product-shaped pattern emerges: **every
failure class below is a case where a visible surface (dashboard, config panel, watcher role,
open order flow) stood in for a structural mechanism** — which is precisely the CRM/SaaS shape
the operator is excluding. This section is the extraction the task asked for; each row cites the
incident evidence, the generalized failure, its CRM/SaaS analog, and the structural replacement
dowiz already has or has already blueprinted.

| # | Incident evidence | Generalized failure shape | CRM/SaaS-shaped analog | Structural replacement (status in dowiz) |
|---|---|---|---|---|
| F1 | Knight Capital: ~$460M in 45 min; "humans watching dashboards could not shut it down fast enough" (R5 §1) | **Dashboard-as-safety-mechanism** — monitoring is advisory, harm outruns attention | An admin panel with alerts as the risk control | Pre-commit gates IN the path: `commit_after_decide_drift_gate` (pre-persist reject), `noether.rs` invariant checks, planned `ExposureLedger::try_commit` in the same slot (R5 §3's 15c3-5 "direct and exclusive control"). BUILT / B3 blueprinted |
| F2 | Ronin: a temporary signing grant "never revoked" enabled $624M (R1 §1); IPFS: persistence is an ops convention, unpinned data silently dies (R2 §5) | **Ops-memory-as-guarantee** — correctness depends on an admin remembering | Settings panels + runbooks; "the operator will clean that up" | Grants carry expiry and are revocable as data (`RevocationSet`, capability expiry in `HybridGate::check`); replication policy belongs in the protocol (3-2-1-1-0 as per-object pin counts, R2 §5). BUILT (caps) / doctrine (pins) |
| F3 | Optimistic rollups: ~3 years to the first real fraud proof, whitelisted challengers, censorable challenges (R1 §6); MAST: 21.3% of multi-agent failures = nobody checked the claim (R3 §3) | **Watcher-as-verifier** — safety assumes someone is looking and can act | Human review queues; "a manager approves it in the dashboard" | Verify-before-commit (validity-first), counterparty-verified `WorkReceipt` on public data — never a human queue, never a self-certificate. BUILT (gate) / B2 blueprinted (receipts) |
| F4 | Poly Network: the verification path could rewrite its own trust root; Nomad: an upgrade seeded a `0x00` trust anchor and everything validated (R1 §1) | **Config-surface-as-attack-surface** — free-form privileged mutation reachable at runtime | The settings panel as product; live config editors | Trust roots out of the mutation path (fail-closed `load_genesis`, `RootDelegationPolicy` defaults to refuse); config as bounded enumerable lattice, never free-form (E3 discipline, adopted into `AgentManifest`). BUILT |
| F5 | MEV: 72,351 sandwich victims, ~$87.7M/half-year — pending state visible to adversaries IS the extraction surface (R1 §5) | **Visibility-as-extraction** — showing in-flight state creates the attack | "Real-time activity feeds" / open order books as a feature | Sealed commitments (commit-reveal on the WORM log), batch windows, hash tie-breaks — the five-precondition dormant law (mesh SYNTHESIS §2.1). Doctrine, deliberately unbuilt |
| F6 | Terra: stability rested on confidence, and the corrective mechanism amplified the run (R1 §7); Solana: recovery = validators hand-coordinating restarts off-chain (R1 §2) | **Attention/confidence-backed stability** — the system holds only while humans believe and attend | Growth dashboards as health; war-room ops as recovery | Degrade-closed refusal + designed re-open conditions (R5 §1's LULD graduated ladder, adopted for B3); no mechanism whose backstop is belief. Doctrine + B3 |

The inversion in F5 deserves one sentence because it contradicts SaaS instinct directly: in an
adversarial setting, **more visibility is not more trust — visible in-flight state is a
manipulable instantaneous quantity** (R1's second meta-pattern). A product built as "watch
everything happen live" builds the extraction surface in.

### 2.1 What worked in web3, read as invisibility mechanisms

The strongest positive evidence in R2/R3 is uniformly about *removing* surfaces users had to
operate:

- **Account abstraction (ERC-4337/EIP-7702, R2 §1).** The killer feature was gas sponsorship +
  pluggable validation — i.e., the user stopped seeing gas, seed phrases, and signature schemes.
  Adoption clustered exactly where the machinery became invisible (paymasters, passkeys, session
  keys). Transferable split: **the party who authorizes ≠ the party who pays ≠ the scheme that
  validates**, all bound in one signed envelope — none of it a user-facing surface.
- **Capability possession = authority (seL4/HACMS, Cloudflare Workers — R2 §6).** No ACL admin
  screens, no role-management UI: you hold an unforgeable, attenuable reference or you are
  "mathematically stuck." A red team with in-flight access could not escalate. The permission
  *panel* disappears because permission is a held object, not a configured row.
- **Escape hatch over decentralization theater (rollup forced-inclusion, R2 §4).** Users never
  see the sequencer; they don't demand it be distributed because the slow path is *guaranteed*,
  bounding fast-path failure to latency, never safety. Lesson: make guarantees structural and
  the operational topology becomes invisible — the exact opposite of exposing infrastructure
  status pages as a product feature.
- **Verification asymmetry (R2 §2, R4 §5).** Hybrid signature verification costs ~0.1–1 ms/msg
  against 10–100 ms network RTT — verification is cheap enough to be *always on and never seen*.
  There is no "pending review" state a user must watch.
- **Signed mandate chain (AP2, R3 §4).** Human intent → bounded delegated authority → verifiable
  execution record. The human appears once, at intent; everything after is machine-verified.
  This is the invisible-agent pattern payment incumbents already accepted.
- **Deterministic budgets in infrastructure (R3 §5).** Nobody watches a spend dashboard; the
  `TokenBucket` refuses with a typed error. dowiz is already ahead of framework practice here.

**Extension vs. prior synthesis, stated exactly:** the mesh SYNTHESIS used these same items to
design wire-level primitives. This document's addition is the claim that they also fix the
*interface economics*: each one converts a surface a human had to operate (approve, configure,
watch, reconcile) into a structure that needs no operator attention in the steady state. That
claim is the bridge from the web3 evidence to the operator's "invisible and seamless" directive.

---

## §3 The invisibility doctrine — five engineering rules

Plainly: **a human should touch the system at exactly three points — expressing intent, handling
a surfaced exception, and making a governance decision. Everything between intent and exception
runs headless.** This is M5 hub-autonomy applied to the human surface, and each rule below is
derived from a §2 row, not asserted.

- **I1 — Authority travels with the request.** Every actionable message carries its own signed
  authorization (capability/envelope), verified in the kernel; therefore no login-session
  machinery, role matrix, or permission screen is load-bearing. (From F2/F4 + R2 §6; mechanism:
  `HybridGate::check`, capability expiry, `RevocationSet` — built.)
- **I2 — Safety is pre-commit or it does not count.** Every risk control lives in the commit
  path with a typed reject; a monitoring view may *duplicate* a control but may never be its
  only home. Review discipline: any proposed "alert/dashboard" mitigation must name its kernel
  commit-path twin or be rejected. (From F1; mechanism: drift gate, noether, B3 `try_commit`.)
- **I3 — Verification is always-on and never queued to a human.** Claims between parties are
  verified cryptographically at message cost (~0.1–1 ms, R4 §5) or by deterministic validators;
  agent output that matters gets an independent check (Mirror, schema validator, counterparty
  receipt), never a human approval queue and never the claimant's own word. (From F3; RC-2/P7.)
- **I4 — Recovery is designed, not staffed.** Refusals are typed and degrade-closed; anomaly
  halts have a graduated limit-state and a *defined automatic re-open condition*; only tamper
  (`Locked`) and red-line scopes require an operator act. Every transient spike that costs an
  operator intervention is a design bug. (From F6 + R5 §1.)
- **I5 — The audit surface is the log.** Byte-identical replay of the WORM event log is the
  audit; any report, chart, or admin page is an optional *read projection* over it, rebuildable
  and deletable, never a source of truth and never a control surface. (From F1/F6 + R5 §2
  matching-engine convergence.)

Consequence for product shape, stated once and plainly: dashboards, forms, and settings panels
are demoted from *product* to *optional projections*. The load-bearing human surfaces are:
intent capture (an order, a goal, a message — Telegram-primary per the IP-* integration arc fits
this directly), exception surfacing (a typed refusal or breach alert pushed to the human, not a
panel the human polls), and governance acts (capability grants, kill-switch, operator `!`
decisions). This does not delete the existing TS admin UI; it rules that no NEW canonical-stack
capability may *depend* on such a surface to be correct or safe.

---

## §4 Architecture by pillar

### 4.1 Agentic infrastructure — how agents operate with no one watching

**What already stands (extend, don't redesign):** the three-plane picture (LOCAL-AI §3) —
model plane BUILT (`LlmBackend` + `llm-adapters`), foreign-agent plane BUILT on the mesh branch
(`AgentBridge` admission → `SandboxTier` cage → fuel/`TokenBucket` envelope), resident-agent
plane ABSENT (gap G1, the `AgentLoop`). The mesh Agent Exchange Plane (B1–B3) supplies the
inter-hub trust fabric. Nothing in this document changes those designs.

**What this synthesis adds — the headless operating loop, composed entirely from named existing
parts:**

1. **Trigger = event, not human.** An agent step is caused by an event on the WORM log (new
   order event, failed-validation event, timer tick from a named in-repo scheduler — the P5
   dead-pendulum gap the mesh SYNTHESIS §4.3 already flags must be closed by B2's sweep
   mechanism, not by a cron convention). No agent waits for a human to click.
2. **Coordination = blackboard, not supervisor.** Concurrent loops append typed step events and
   read each other's ≤200-token status entries from the same log (LOCAL-AI §2.4: blackboard
   beats supervisor 13–57% for small-model teams, and the WORM log already is one). No
   orchestrator service, no orchestration UI.
3. **Containment = budgets + exposure + named exploits, all structural.** Flow bounded by
   `TokenBucket`/fuel (built); stock bounded by the B3 `ExposureLedger` (blueprinted);
   contested allocation, if ever needed, only under the five sealed-batch preconditions (dormant
   law). The R5 §6 RED-list (spoofing/wash/stuffing) is the checklist any new coordination
   mechanism must answer.
4. **Human surface = exceptions and governance only.** A loop's outcomes are: committed events
   (silent), typed refusals (surfaced to the human as an exception with the refusal reason), or
   breach alerts (`BreachAlert` → peer convergence + operator). Autonomy tier is capped by law
   until P10's kill-switch exists (G8 — restated here because it is the governance touchpoint
   that makes headless operation permissible at all: invisible operation without a kill-switch
   would itself repeat F6, stability resting on hope).

**Anti-requirement, from F1/F3:** no "agent activity dashboard" may be a correctness or safety
dependency of any wave. Observability remains available as projections (the `TrackRecord`/
`Telemetry` ledger already provides it) — consulted when debugging, load-bearing never.

### 4.2 Local AI — built; the web3 synthesis adds three rulings

The plane itself is done and documented (HARNESS-LLM-BACKEND.md; LOCAL-AI §1.4); its gap
register (G1–G8) and economics constraint (prefill-heavy/decode-light at 4.8–10.5 tok/s) are
today's work and are not duplicated here. What the web3 evidence adds on top:

1. **Telemetry-based model routing is NOT reputation — with the distinction argued, not
   assumed.** R1 §4 (Cheng–Friedman impossibility, Sybil, whitewashing) condemns *symmetric
   scores aggregated from peer reports in an adversarial identity space*. The G3 router consumes
   `Telemetry`/`ModelStats` — the hub's OWN measurements of ITS OWN backends' success/latency,
   with no peer-supplied input, no cross-actor aggregation, and no identity minting surface.
   None of the three attack preconditions (cheap identities, peer-reported scores, symmetric
   aggregation) exists, so the NO-SCORING gate is not triggered. The gate's vigilance clause
   (R1 §4b: no smuggled reputation) still binds the boundary: the moment routing input includes
   *another hub's* claim about a model or agent, it crosses into reputation and is refused —
   cross-hub trust is capabilities and receipts only.
2. **Model outputs that matter get envelope proof, not model proof.** zkML is rejected (1000×+
   overhead, R3 §3), TEE stays an optional tier; the verifiable unit is the signed envelope —
   request cid, tool receipts, output cid, budget consumed — checked by deterministic gates.
   For the resident loop this concretely means: schema-constrained output (`format`, live-verified
   working) + application validator + Mirror-checked exit, per LOCAL-AI Wave B, and any
   cross-hub delivery of AI work rides B2's `WorkReceipt`. A receipt proves authorized delivery
   of bytes under a grant — never semantic quality; that honest limit carries over verbatim.
3. **Budgets stay in infrastructure; prompts are never a control.** Confirmed best practice
   (R3 §5) and already built stronger than the frameworks; the invisibility rule I2 adds only
   the review discipline: a spend *view* is a projection, the `TokenBucket`/ledger refusal is
   the control.

### 4.3 Local databases — decided by ground truth; DECART inline

**The recommendation is to change nothing structural: the sqlless pair — content-addressed
`BlockStore` + fsync-gated JSONL `FileEventStore`, with in-memory read projections rebuilt by
replay, `pgrust` as the only optional SQL adapter — is the local-database architecture.** It is
already decided (MESH-09 correction), already implemented (§1.2 citations), already
fault-injection-tested, and the web3 evidence *strengthens* it: hash-as-identity is the one
layer that never failed (R2 §5); deterministic replay over a WORM log is the exchange-grade
audit discipline (R5 §2); and every store crossing is a port (`EventStore`/`BlockStore` traits)
with adapters behind it — the same seam shape as `LlmBackend`.

**Back-of-envelope (the load this must actually hold):** dowiz-domain event rates are small —
at an assumed 100 locations × 2 orders/min × ~10 events/order ≈ 33 events/s peak, roughly 1 KB
each. A JSONL append + fsync is single-digit-ms class on this host's NVMe; sha3-256 of a 1 KB
event is ~3 µs (R4 §4). Headroom is ≥1 order of magnitude with zero tuning. Cold-start replay is
the only O(N) cost; `BackupOrgan` snapshots bound it before any new storage engine could be
justified. (Assumed volumes, not production measurements — production traffic today is
demo-scale, so the assumption is conservative in the right direction.)

**DECART (Integration Decart Rule — the one live technology choice in this document):**

| Candidate | Bare-metal fit | Falsifiable correctness | Measured perf need | Supply chain | Reversibility | Evidence |
|---|---|---|---|---|---|---|
| **Incumbent: sqlless (`FileEventStore`+`BlockStore`+replay projections)** | zero-dep, std-only, in kernel | fsync-gated, fault-injection RED-tested (`hydra.rs:1008`) | covers §4.3 envelope ×10 | none | trait-ported already | live code, this repo |
| redb (pure-Rust embedded B-tree) | good (pure Rust) | ACID, maintained | **no demonstrated need** at current scale | +1 crate at the storage boundary; would break the zero-dep kernel unless adapter-crated | high (fits behind `EventStore`/`BlockStore`) | would be the lead candidate IF the re-open trigger fires |
| sled | pure Rust | long-standing beta, documented recovery instability history | no need | risk | high | technical case against, not social proof |
| SQLite (via rusqlite) | C dependency — contradicts the Rust-native default | mature (not a permitted deciding reason) | no need | C toolchain + FFI | medium | **excluded by canon**: MESH-09 correction says never SQLite |
| pgrust (sqlx/Postgres, existing optional feature) | server-class, not embedded | already in-repo, DB-gated test exists | fallback only | already vetted | already a feature flag | keeps its existing role: the uniform SQL fallback when a genuine SQL shape appears |

**DECISION:** keep the sqlless main path; adopt no embedded database. Falsifiable reason: no
query shape exists that a replay-built in-memory projection cannot serve within the §4.3
envelope, and every candidate adds supply-chain surface for zero measured benefit.
**Probe (strongest honest argument against):** replay projections are O(N) at startup and
RAM-resident; a large event log (order 10⁶+ events / multi-GB) or a required secondary-index
query that exceeds RAM would break this. **Re-open trigger (falsifiable):** if measured
cold-start replay of a live log exceeds ~5 s despite snapshotting, or a required projection
cannot fit the host RAM budget, run a full DECART with redb as lead candidate — implemented
strictly as an adapter crate behind the existing `EventStore`/`BlockStore` traits, never inside
the kernel. **Older-as-adapter note:** pgrust is retained, unchanged, as the bridge to SQL — not
purged, not promoted.

### 4.4 The adapter doctrine — the cross-pillar seam rule (the load-bearing new synthesis)

The operator's connective requirement ("перехідники, адаптери, мости — never absorb an
ecosystem") and R1's bridge dossier meet in one rule. Every catastrophic web3 bridge failure
(R1 §1) had the same anatomy: **the bridge stopped translating and started deciding** — it held
the validator quorum (Ronin), stood in for signature checking (Wormhole), owned the trust anchor
(Nomad), could rewrite the keeper set (Poly), or held all the keys (Multichain). A bridge that
mints authority becomes the whole attack surface.

dowiz's existing seam shape is the structural negation of that anatomy, and it must be named as
doctrine so every future integration inherits it:

> **An adapter translates representation; it must never mint, hold, or verify authority.
> Authority crosses a seam only as a signed artifact, and only the kernel verifies it.**

The reference implementation the operator cited, decomposed into the five properties every
adapter must copy:

1. **Port in the zero-dep kernel** — trait + plain value types, compile firewall (no HTTP/serde
   in the kernel module): `kernel/src/ports/llm.rs` (`LlmBackend`, fail-closed `Caps`).
2. **Adapter in a sibling crate** — all wire knowledge outside the kernel, `ureq`+`serde` only:
   `llm-adapters/` (`OpenAiCompatTransport` + per-backend `Quirks` + `CachingBackend` +
   `Dispatcher`).
3. **Fail-closed discovery** — what the far side does not prove, the port reports as absent
   (`Caps` bits pinned false until probed; `health()` returns typed errors, never a mock).
4. **Budget at the seam** — every crossing draws from a `TokenBucket`/fuel envelope before the
   call, refusing typed (`BudgetExceeded`), and emits a harvest row (`TrackRecord`).
5. **No authority in the adapter** — the adapter carries bytes; capability verification
   (`HybridGate::check`), red-line scopes, and admission stay kernel-side. Compromising an
   adapter yields a broken translator, not a signer.

Conformance census (evidence the doctrine is descriptive, not aspirational): `LlmBackend` /
`llm-adapters` (built); `EventStore`+`BlockStore` / `FileEventStore`·`MemStore`·pgrust (built —
the local-DB pillar already IS this shape); `AgentBridge` / `agent-adapters` (built, mesh
branch — with the F2 deny-by-default + rate-limit anchor); HUB-DESIGN's proposed
`ChannelBridge` and `DispatchProvider` ports (blueprint addendum, same declared shape). Future
integrations from the IP-* ports arc (Telegram, payments, QRNG) bind to the same five
properties. Checklist rule for review: a new integration PR that adds wire code inside
`kernel/`, verifies a signature outside it, or ships without a typed-refusal budget path fails
the doctrine on sight.

---

## §5 Consequences for existing plans (dependencies, no new waves invented)

This document deliberately creates no new build wave. The existing sequenced work already
implements the doctrine; what it adds are acceptance constraints and one operator flag:

1. **LOCAL-AI Waves A/B/C** (port tool extension → `AgentLoop` → router) proceed unchanged;
   Wave B additionally inherits I3/I4 as done-check language (typed refusal surfaced as the
   exception artifact; no step depends on a human viewing anything). Bounded by G8 until P10.
2. **Mesh B2/B3/B4** proceed unchanged; B3's graduated limit-state + auto-reopen is the I4
   mechanism; B2's sweep-firing decision closes the P5 scheduler gap that headless operation
   requires.
3. **Local DB:** no action; the §4.3 DECART records the decision and its re-open trigger. The
   read-projection spec flagged in mesh SYNTHESIS §4.1 ("open commitments with peer X") inherits
   this decision — it is a projection over the log, not a database.
4. **Operator flag (the one surfaced tension, see §6 Anu):** HUB-DESIGN's kernel addendum
   (D1–D5: menu-as-data, `ChannelBridge`, `DispatchProvider`, `StoreState`, statement
   projection) is fully compatible with this document — those are data entities and ports. The
   tension is confined to **P16's owner-UI framing** (16 owner pages: MenuManager, Analytics,
   Settings). Under today's directive, the same kernel entities should surface through
   intent/exception channels (e.g., Telegram-primary conversational ops + pushed exceptions)
   rather than a page-per-concern cockpit, with pages demoted to optional projections. That is a
   presentation-layer re-ruling with zero kernel impact — **flagged for operator confirmation,
   not silently applied**, per the never-bypass-human-gates rule.

---

## §6 Anu / Ananke check (per decision, plainly)

**Anu — do the decisions follow from evidence in front of this document?**

- The six failure-shape rows (§2) each cite a named, root-caused incident from R1/R5; the
  mapping to CRM/SaaS analogs is an interpretive layer and is falsifiable: one counterexample —
  a cited incident whose root cause was structural despite a dashboard-free design, or a
  dashboard-dependent system in this class that survived adversarial stress — would weaken the
  corresponding row. None was found in the R-docs.
- The routing-vs-reputation ruling (§4.2.1) is argued from the attack preconditions in R1 §4
  (cheap identities / peer-reported / symmetric aggregation), each checked absent — not from
  "it feels different."
- The local-DB decision (§4.3) is derived from live code citations + an explicit arithmetic
  envelope + canon (MESH-09), with a falsifiable re-open trigger; the strongest counter-argument
  (O(N) replay) is stated inside the decision, not omitted.
- Sibling-document contradiction check, run deliberately: mesh SYNTHESIS — no contradiction
  (different axis, §0); LOCAL-AI — no contradiction (its G-register and waves are adopted
  verbatim); HUB-DESIGN — one real tension found (its market bar is a "vendor operations
  cockpit"; today's directive excludes cockpit-as-product), resolved by argument in §5.4:
  kernel entities compatible, presentation framing flagged to the operator rather than left
  standing or silently overridden.

**Ananke — is the good outcome structural, not hoped for?**

- Already structural: I1 (capability verification is in the commit path), I2 partially (drift
  gate, noether), I3 (typed errors, RC-2 rule), I5 (WORM log exists; projections are
  rebuildable by construction). The adapter doctrine's properties 1–4 are enforced by existing
  compile firewalls, feature gates, and typed refusal paths.
- Honestly NOT yet structural, named rather than hidden: (a) nothing mechanically prevents a
  future safety check living only in a UI layer — I2's review discipline is a standing rule
  today, and a CI grep-gate (e.g., new `*BLUEPRINT*`/`*ROADMAP*` naming a commit-path twin for
  every alert-class mitigation) is the legitimate enforcement follow-up, which touches
  `.claude/`-adjacent config and is therefore the operator's unlock, not this document's act;
  (b) the P5 scheduler gap means "event-driven, headless" partially rests on B2 specifying its
  sweep mechanism — a named dependency, not an assumption; (c) G8: headless autonomy above
  advisory tier is structurally forbidden until P10's kill-switch exists — this document leans
  on that law rather than weakening it.

---

## §7 The 2-question doubt audit (mandatory)

**Q1 — least-confident items, in order:**

1. **The CRM/SaaS mapping is an interpretive layer.** The incidents are cited facts; reading
   them as "product-shape failures" is this document's argument. It is falsifiable (§6) but it
   was not independently red-teamed this session.
2. **HUB-DESIGN was read §1 + section map + addendum headers, not every body paragraph.** The
   §5.4 compatibility claim (D1–D5 are data + ports) rests on its own header declarations and
   its stated Trait-as-Port constraint; a full-body read could surface a dashboard-dependent
   detail inside D1–D5 that would move it from "compatible" to "needs the same re-ruling."
3. **Carried-forward measurements were not re-run** (Ollama state, decode rates, mesh-branch
   build status): all sourced from documents written and live-verified earlier today on these
   same branches — the freshest available evidence short of re-probing, but still one step
   removed.
4. **The §4.3 volume envelope uses assumed order rates**, not measured production traffic
   (production is demo-scale, so the assumption overstates load — the safe direction — but the
   re-open trigger exists precisely because assumptions age).
5. **Interpretation of "no longer relevant":** applied here as "the canonical Rust stack must
   not make SaaS surfaces load-bearing," NOT as "delete/stop the existing TS product." If the
   operator meant the stronger reading, §0's non-goal (no migration plan) becomes the next
   task; the architecture in §2–§4 is unchanged either way.

**Q2 — the biggest thing possibly being missed:** the transition seam. This document defines
the end-state (three human touchpoints, everything else headless) and shows the substrate
supports it, but real users today interact through exactly the surfaces being demoted — and the
document deliberately does not design the intermediate product (which exceptions surface where,
what the Telegram-primary intent grammar is, what happens to the 16 owner pages in the
meantime). That is withheld on purpose — per the standing rule, the operator sets direction and
§5.4 is the flag requesting it — but it should be named as the single largest unplanned area
between this synthesis and anything a user touches. Secondary miss, inherited and restated:
LOCAL-AI §7 Q2's economics warning applies to invisibility too — an invisible agent that takes
10 minutes per loop at 5 tok/s is invisible AND useless; the decode-economics probes (P-1/P-2)
gate this document's §4.1 just as hard as they gate the resident-agent plane.

---

*Written 2026-07-17 on `feat/harness-llm-backend` in `/root/dowiz`. Live checks this session:
glob of `kernel/src/ports/*.rs` and `llm-adapters/src/*.rs`; case-insensitive grep for
`sqlite|sled|redb|rocksdb|lmdb|pgrust` across `kernel/` and `engine/src/`; targeted greps of
`kernel/src/{event_log,hydra,backup}.rs`, `kernel/Cargo.toml`, `kernel/src/retrieval/
memory_store.rs`; reads of AGENTS.md (planning protocol, DECART rule, Anu/Ananke) and the ten
documents listed in §1.1. No code written or edited.*
