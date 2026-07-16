# Synthesized Blueprint & Plan — Cross-Report Synthesis (2026-07-16)

> Source documents (all produced same session, 2026-07-16):
> 1. `SYSTEMS-GPU-ML-KERNEL-SYNTHESIS-2026-07-16.md` — 8 parallel Opus research
>    agents + 1 reasoning pass, cloud/data/distributed-systems/GPU-ML/AI-math/
>    git-CI/reference-repos, all grounded in `kernel/src/*`, `engine/src/*`,
>    `bebop2/*`.
> 2. `GAUSSIAN-SPLATTING-ADDRESS-PICKER-SYNTHESIS-2026-07-16.md` — 6 parallel
>    Opus research agents + 1 reasoning pass, on the 2-stage delivery
>    address-picker + reranker/TimesFM/CuPy verdicts.
> 3. `HK05-REALTIME-MODEL-ROUTING-INTEGRATION-2026-07-16.md` — status audit of
>    real-time model-tier routing compute, verified by reading code across a
>    third repo (`/root/hermes-agent-kernel-rewrite/hermes-kernel/`).
>
> This document merges their roadmaps into one prioritized blueprint, preserves
> every explicit rejection, and adds per-P0-item blueprint detail (files,
> acceptance criteria, dependencies). Nothing below is a new idea — every line
> traces to one of the three source documents. Where an item's origin isn't
> obvious from context it's tagged `[SYSTEMS]`, `[GS]`, or `[HK05]`.

---

## 1. Executive summary

All three reports were produced in the same research session, and although they
investigate unrelated surfaces — infra/GPU/ML-kernel architecture, a delivery
address-picker feature, and dev-tooling model routing — they share a method
(multiple independent research agents grounded in real code, then one reasoning
pass that looks for *independent convergence* as the strongest possible
priority signal) and, in one case, land on the **identical architectural
conclusion from two completely different directions**.

**The shared thread, confirmed by reading both docs closely.** `SYSTEMS` §11
explicitly rejects "literal GPU/CUDA adoption" — reached by auditing the kernel
code and finding `field_frame.rs` says "wgpu OUT OF SCOPE" and `attention.rs` is
"reference scalar... kernel stays non-AI"; the value transferred is *design
principles* (fusion, batching, KV-cache, locality tiers), never a GPU substrate.
`GAUSSIAN-SPLATTING` §2.3 independently states this is "the strongest
cross-confirmation in the set": reports 03 (library-fit analysis) and 06 (cost
analysis) arrive from opposite ends at "GPU work is offline, batch, behind a
port; never in-kernel, never in the request path." These are not two different
opinions that happen to agree loosely — they are **the same stance**
("GPU as an external, ported, offline capability — never a first-class kernel
dependency") independently re-derived by two separate research efforts that did
not reference each other going in. That makes it the single most load-bearing
architectural constraint across all three documents, and it governs every P1
splat-rendering item below (the `SplatReconstructionJob` port, the
`webgl`/`webgpu`/`splat` cargo features gated behind `default = []`).

**Other cross-report patterns worth naming:**
- **Trait-as-Port and content-addressing, applied verbatim across domains.**
  `SYSTEMS` §2.1–2.2 names these as the house style (`BlockStore`, `EventStore`,
  `Transport`, all cloud/GPU/external capability behind a trait; `sha3_256` as
  the universal cache/dedup/audit key). `GAUSSIAN-SPLATTING` §2.4 applies both
  literally: the `SplatReconstructionJob` trait *is* Trait-as-Port for GPU
  rental, and the splat asset cache key
  (`sha3(geohash(address) ⊕ hash(imagery_set) ⊕ params)`) into the existing
  `backup.rs` BlockStore *is* content-addressing "applied verbatim," in the
  source document's own words.
- **Falsifiability discipline, applied identically.** All three reports refuse
  to accept a hypothesis without a stated falsifier: `SYSTEMS` §7 operationalizes
  proof-by-contradiction as machine-checked `eqc` harnesses; `GAUSSIAN-SPLATTING`
  gives every P0 item a numbered falsifiable acceptance list and gates every P2
  deferral on a specific future test (beat a classical baseline, win a
  PSNR/cost bake-off); `HK05` closes with an explicit RED→GREEN falsifiable
  test for the one remaining wiring gap.
- **Refusal to manufacture connections.** `GAUSSIAN-SPLATTING` §3 explicitly
  refuses to force a TimesFM↔Gaussian-Splatting bridge ("share no mathematical
  surface... no connection is manufactured"). `HK05` §5 explicitly refuses to
  force HK-05/HK-09 routing into being a dowiz *product* feature ("forcing that
  connection would be the same overclaiming this project already rejects
  elsewhere"), correctly scoping it as dev-tooling for agent sessions working
  *on* dowiz, not a delivery-platform capability. This synthesis follows the
  same discipline below: **no artificial link is drawn between HK05 and the
  other two roadmaps** — a check for code-path overlap found none (HK05 touches
  `tools/telemetry/governance.sh` + the separate `hermes-kernel` repo; the
  Systems roadmap's nearest neighbor, spool-based bounded-substep draining
  [P1], touches `kernel/spool.rs`/`engine/loop_.rs` — a different surface).
- **All three are "P0 is cheap, already-diagnosed, and often already-half-built"
  documents.** Every P0 item across all three reports is small in code size
  relative to its architectural leverage — a cache key, a wiring call, a fixed
  hash count, six pure geo functions. None require new infrastructure to start.

**What this synthesis does NOT find:** no roadmap item in any of the three
reports touches the same file as an item in another report. The merge below is
therefore a **union with reordering by corroboration strength**, not a
line-item dedupe — the one apparent overlap candidate (HK05 wiring vs. Systems'
spool/drainer item) was checked and is a false match (different files, different
purpose: one is LLM-tier routing for agent sessions, the other is CPU
substep-budgeting for kernel numeric ops).

---

## 2. Unified P0 blueprint — prioritized, with per-item detail

Ordered by corroboration strength within each cluster, then grouped by surface
area (kernel/infra correctness → product feature → dev-tooling) since none of
these clusters gate each other.

### Cluster A — Kernel/infra correctness (source: `SYSTEMS` §10, items 1–7)

**P0-A1. Cache the spectral decomposition of `L`** `[SYSTEMS §6.2, §10.1]`
- **What:** `field_frame.rs`'s graph Laplacian `L` is recomputed every frame
  (`laplacian()`, matvec) even though topology — and hence `L` — is unchanged
  frame-to-frame. Decompose once, cache the eigenbasis keyed by the existing
  `snapshot_root` hash (`memory_store.rs`), and do decay (`exp(-λt)`), motion
  (`exp(iωt)`), and layout as point-wise multiplication in the cached basis.
- **Files touched:** `engine/src/field_frame.rs` (cache lookup + invalidation
  on topology change), `kernel/src/memory_store.rs` (expose `snapshot_root` as
  the cache key), `kernel/src/spectral.rs` (the decomposition being cached).
- **Why P0:** the single most corroborated finding in the entire research
  session — independently reached by the kernel-synthesis cluster (code-level:
  recomputed every step) and the AI/math cluster (physics-level: QHO↔field
  operator structural equivalence implies the same spectral-caching recipe),
  *and* it matches `bebop2/ARCHITECTURE.md`'s own stated but unimplemented goal
  ("store the operator as its spectrum... never the dense tensor").
- **Acceptance criteria:** for an unchanged topology, decay/motion/layout
  update cost drops from O(n³) recompute to O(n) point-wise multiply in the
  cached basis; a snapshot_root change correctly invalidates and recomputes.
- **Dependencies:** none on other P0 items; a natural second-order candidate is
  applying the same cache-key pattern to PPR (§6.2's "second candidate," not
  separately roadmapped as P0).

**P0-A2. Fix double-hashing in `event_log.rs`** `[SYSTEMS §6.1, §10.2]`
- **What:** `commit_after_decide` and `append` both compute `event_id()` on the
  same bytes — hash once, thread the value through.
- **Files touched:** `kernel/src/event_log.rs`.
- **Why P0:** cheap, concrete, found directly in code; no research risk.
- **Acceptance criteria:** `event_id()` invoked exactly once per commit path;
  existing event-log tests stay green (byte-identical event IDs).
- **Dependencies:** none.

**P0-A3. Secrets management** `[SYSTEMS §8, §10.3]`
- **What:** gitleaks CI gate at minimum; secrets exclusively via systemd
  `EnvironmentFile` / Fly secrets, never in-repo or in CI logs.
- **Files touched:** CI config (new gitleaks job), deploy scripts referencing
  `.env` today.
- **Why P0:** triple-confirmed independently (git/CI cluster, cloud/IaC
  cluster's externalized-config finding, and roadmap-gap-analysis cluster), and
  directly tied to a **real, already-recorded incident** (`SECURITY INCIDENT:
  creds in git history`, 4 unrotated pasted creds including a CF API token, per
  project memory).
- **Acceptance criteria:** gitleaks CI job fails the build on a planted test
  secret; no plaintext secret reachable via `git grep` on current HEAD.
- **Dependencies:** none; pairs naturally with P0-A5 (git discipline) since
  both are CI/git-hygiene, but neither blocks the other.

**P0-A4. Saga compensation edges + money reversal primitive** `[SYSTEMS §4, §10.6]`
- **What:** `allowed_next` in the order state machine encodes only the
  happy path (`InDelivery → [Delivered]`); there is no compensating transition
  (e.g. `InDelivery → CompensatedRefund`) and `money.rs` has no reversal
  primitive (`checked_add` only, no paired compensating credit). A
  multi-node order (merchant→courier→customer) is a distributed transaction
  today with no 2PC and no compensation.
- **Files touched:** `kernel/src/order_machine.rs` (new compensating states +
  transitions), `kernel/src/money.rs` (reversal/compensating-credit primitive).
- **Why P0:** correctness gap in the project's own declared red-line domain
  (money/orders) — this is not a nice-to-have, it's closing a hole in a domain
  the project treats as non-negotiable elsewhere.
- **Acceptance criteria:** a cancel-after-confirm flow reaches a terminal
  compensated state with a matching reversal entry in `money.rs` that nets to
  zero; existing FSM golden-signature drift-gate stays green with the new
  states added deliberately (not silently).
- **Dependencies:** none technically, but this is a red-line/money change —
  treat with the same scrutiny the project already applies to money code
  regardless of any suspended process gates.

**P0-A5. I-FINAL invariant as an eqc-style machine-checked proof** `[SYSTEMS §7, §10.7]`
- **What:** new invariant for bebop2 — "two mesh nodes never finalize
  conflicting delivery state for the same order" — provable via
  quorum-intersection (two signed quorums `Q_A, Q_B` at `n > 3f` must overlap
  in one honest node, which can't have signed two different finalizations for
  the same `(order, epoch)` without contradiction).
- **Files touched:** new `eqc-proofs/i_final.rs` (or equivalent) alongside the
  existing `eqc-proofs/lambda_max_of_d.rs` pattern; bebop2 certificate
  verification path it constrains.
- **Why P0:** cheap first step (a proof sketch, not a rewrite), continues the
  project's own established VERIFIED-BY-MATH discipline exactly the way
  `lambda_max_of_d` already does.
- **Acceptance criteria:** self-asserting harness compiles and runs, asserting
  the quorum-intersection contradiction is unreachable given `n > 3f`.
- **Dependencies:** none; independent of P0-A4 despite both touching
  order/delivery-state correctness (different repos: `order_machine.rs` is
  dowiz kernel, I-FINAL is bebop2 mesh consensus).

**P0-A6. IaC first step: OpenTofu + one `libvirt_domain` module** `[SYSTEMS §3, §10.4]`
- **What:** zero `.tf`/IaC files exist today (imperative shell + manual
  `systemctl enable`). First step: one `opentofu/` directory, one
  `libvirt_domain` module declaratively reproducing one Firecracker/KVM
  microVM node, `backend "pg"` remote state on the already-running pgrust
  instance (not S3+DynamoDB).
- **Files touched:** new `opentofu/` directory (net-new, no existing file
  conflicts).
- **Why P0:** closes the one domain that is *fully* absent (not partially —
  zero IaC files exist), and is licence-clean (OpenTofu = MPL fork, no BSL
  vendor lock, consistent with the project's own dependency-choice discipline).
- **Acceptance criteria:** `tofu plan`/`tofu apply` reproducibly creates one
  microVM node from the module; state persists in pgrust via `backend "pg"`.
- **Dependencies:** none.

**P0-A7. Git discipline: short-lived branches + daily rebase** `[SYSTEMS §8, §10.5]`
- **What:** process rule, not code — branches live hours-to-days not weeks,
  `git fetch && rebase origin/main` daily. dowiz↔bebop-repo coordination stays
  via versioned interface contract (already exists:
  `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT`), never submodules or lockstep branch
  names.
- **Files touched:** none (process rule); optionally a CI check for branch age.
- **Why P0:** directly explains an already-recorded incident (the
  `rebase --onto` divergence recovery at `@e275dbce`, per project memory) and
  the current state (~15 `feat/*`/`recover/*` branches).
- **Acceptance criteria:** N/A (process adoption); optional: CI warning on
  branches >N days without a rebase.
- **Dependencies:** none.

### Cluster B — Address-picker product feature (source: `GAUSSIAN-SPLATTING` §4, items P0.1–P0.3)

**P0-B1. `geo.rs` geometry extension + no-splat address-picker v1** `[GS §2.6, §4 P0.1]`
- **What:** six new pure functions on `kernel/src/geo.rs`, splat-independent:
  `storey_height_m`, `floor_slice_height_m`, `arrow_screen_rotation_deg`,
  `angular_diff_deg`, `in_field_of_view`, `los_clear`. Plus OSM
  `building=*`/`building:levels`/Overture-fallback footprint ingestion with
  graceful ground-floor degrade (never fabricates a floor count).
- **Files touched:** `kernel/src/geo.rs` (six new functions, existing
  RED→GREEN unit-tested style, zero new dependencies), new footprint-ingestion
  module, address-picker UI consuming it.
- **Why P0:** buildable now with zero new dependencies; this whole layer is
  explicitly independent of the splat pipeline (draws over Tier-C static
  panorama, bare OSM footprint, or nothing), which is what makes it shippable
  before any GPU work exists.
- **Acceptance criteria (verbatim from source, six-item falsifiable list):**
  (1) pin-drop works everywhere, no external data dependency; (2) address with
  OSM footprint+levels → correct floor-selector range + north-up slice at
  `floor_slice_height_m`, verifiable against the raw tag; (3) address with no
  footprint → graceful open-space degrade, no crash, no fabricated floor;
  (4) arrow bearing on the north-up slice matches `bearing_deg` to <1°;
  (5) `in_field_of_view` correct across the 0/360° seam; (6) `los_clear` false
  across a known rectangular footprint, true routing around it.
- **Dependencies:** none; this is the foundation P0-B3 (photo capture) and
  P1-B1/B2/B3 (splat pipeline) later layer on top of.

**P0-B2. Hand-tuned fusion reranker** `[GS §3, §4 P0.2]`
- **What:** fixed-weight linear fusion over BM25 (`bm25.rs`) + PPR graph mass
  (`ppr.rs`) + exact/trigram match (`index.rs`) + recency + status/tier weight
  — fixed weights, per-signal normalization, fixed summation order, ascending-
  id tie-break. No trained model.
- **Files touched:** new fusion module composing `bm25.rs`/`ppr.rs`/`index.rs`
  outputs; extends the existing 29-query retrieval oracle with
  BM25-vs-graph-disagreement queries.
- **Why P0:** right-sized for the data the project actually has (no
  MS-MARCO-scale labelled corpus); `no_std`-compatible, bit-reproducible,
  matches the knowledge-spine blueprint's existing "status-aware rerank" spec.
- **Acceptance criteria:** nDCG@10 gap vs. an offline ms-marco-MiniLM
  measuring-stick (never shipped, used only to validate) stays within ~2-3
  points on the extended oracle — the standing falsifier that would trigger
  reconsideration.
- **Dependencies:** none.

**P0-B3. Courier photo capture flow (consent + upload + geohash staging)** `[GS §2.1, §4 P0.3]`
- **What:** consent-gated photo capture at delivery time, staged into the
  existing content-addressed `BlockStore` (`backup.rs`) keyed by address
  geohash from day one.
- **Files touched:** new capture/upload flow (courier app surface), `backup.rs`
  BlockStore consumer (existing port, no new adapter needed for this step).
- **Why P0:** costs nothing GPU-wise, and must start accreting data before P1's
  splat reconstruction has anything to reconstruct from — data bootstrap is
  the long pole, not compute.
- **Acceptance criteria:** photo sets stage into BlockStore keyed correctly by
  geohash; dedup is free by construction (same content hash = same block).
- **Dependencies:** feeds P1-B1 (SplatReconstructionJob); does not depend on
  P0-B1/P0-B2.

### Cluster C — Dev-tooling model routing (source: `HK05` §3–4)

**P0-C1. Wire `governance.sh`'s `gov_route()` to the already-built routing engine** `[HK05 §3–4]`
- **What:** the entire compute for adaptive real-time model-tier routing is
  already built, tested, and compiled into a binary in a **third repo**
  (`/root/hermes-agent-kernel-rewrite/hermes-kernel/`) — `classify_complexity`,
  `rank_models_for_bucket` (which itself calls `harmonic_centrality`, already
  built+tested in `dowiz/kernel/src/harmonic.rs`, wasm-wired), `ev`,
  `kelly_fraction`, `ruin_prob`, `lane_size`, `pid_parallelism` (13 tests, all
  dispatchable via CLI ops). The live `gov_route()` in `governance.sh` never
  calls `op_classify_complexity` or `op_rank_models`, and lane width (N=4/N=8
  in the benchmark) is a hardcoded constant, never adaptive via
  `lane_size`/`pid_parallelism`.
- **Files touched:** `dowiz/tools/telemetry/governance.sh` (three new calls:
  `op_classify_complexity` on task entry, `op_rank_models` folding
  `track_record.jsonl` by `(bucket, model)` instead of just `task_type`,
  `op_gov_lane`/`lane_size`/`pid_parallelism` fed by
  `resource_sample()`/`bench_run()` telemetry already collected),
  `track_record.jsonl` (new `bucket` column, backward compatible — missing
  field defaults to `Simple`).
- **Why P0, and scope caveat:** this is a wiring task with a clear boundary,
  not research — but it is honestly **dev-tooling for agent sessions working
  on dowiz, not a dowiz/DeliveryOS product feature**. Include it in this P0
  tier for completeness and because it's cheap and already fully speced, but
  do not conflate it with the product-facing P0 items in Cluster B, nor claim
  it advances a shipped delivery-platform capability. The source doc is
  explicit: "dowiz today doesn't run its own LLM agents in production that
  would need this router."
- **Acceptance criteria (RED→GREEN, verbatim from source):** the same task,
  classified as `Complex`, must get a different (wider/more expensive) route
  than the same task classified as `Simple` — a falsifiable test that either
  passes or doesn't.
- **Dependencies:** none on Clusters A or B (confirmed no code-path overlap).

---

## 3. P1 — merged, larger or conditional

**From `SYSTEMS` §10 (items 8–15):**
- Core-pinning + cgroups/CAT + NUMA-binding for tenant isolation (the accurate
  MIG analog, more precise than cgroups alone) `[§6.4]`.
- SIMD-batch the non-critical numeric lane (N Kalman filters across couriers
  as struct-of-arrays + `f64x4`; `attention.rs::softmax` SIMD-reduction;
  `money.rs` integer SIMD) — explicitly never on the single-order critical path
  `[§6.6]`.
- Route heavy kernel one-shot operations (spectral decomp, backup, recall
  re-index) through `spool.rs` with a bounded-substep drainer modeled on
  `engine/loop_.rs::MAX_SUBSTEPS` `[§6.5]`.
- Consistent-hashing ring for order/region ownership on top of the existing
  HRW-hashing for couriers `[§4, §5]`.
- Complete the pgrust projection: persisted read model, keyset pagination on
  `actor_seq` (not OFFSET), real B-tree indexes, synthesized `updated_at`
  `[§4]`.
- Frontend minimum: debounce/throttle utilities, a server-state layer
  (stale-while-revalidate, dedup, refetch-on-focus), `BroadcastChannel`
  cross-tab sync, correct CORS+httpOnly cookie auth — addresses a concrete bug
  class (stale order status in a second tab; stale-tab-after-logout) `[§8]`.
- Algorithmic gaps for routing/mesh-partitioning: Dijkstra/A* for courier
  routing (flagged as a **product** gap, not infra — `geo.rs` today only
  projects position onto an already-given polyline, it doesn't compute a
  route), Union-Find/DSU (currently ad-hoc BFS in `cgraph.rs`), Bloom
  filter/Count-Min/HyperLogLog for the mesh path, MST for gossip/overlay
  spanning tree `[§9]`.
- Distributed tracing — `Envelope.trace` already carries a correlation ID,
  only a sink is missing `[§5, §9]`.

**From `GAUSSIAN-SPLATTING` §4 (items P1.1–P1.3):**
- **P1-B1. `SplatReconstructionJob` port + Modal adapter.** Port trait, Modal
  default backend (~$1.25–1.67/job, per-second billing, true scale-to-zero),
  content-addressed result cache (`sha3(geohash ⊕ imagery_set ⊕ params)`),
  mandatory-teardown watchdog, monthly budget ceiling (degrade-closed),
  TokenBucket submission bulkhead, 7k-iteration preview default. Container:
  COLMAP → PyTorch+gsplat (brush bake-off deferred to P2). *Acceptance:* one
  real address reconstructed end-to-end for ≤$2; artifact lands in BlockStore;
  re-submission of identical inputs is a $0 cache hit; a killed job provably
  tears down its rental. **Depends on P0-B3** (needs photos to reconstruct
  from).
- **P1-B2. Tiered client renderer.** mosure/bevy_gaussian_splatting behind
  `webgl`/`webgpu`/`splat` cargo features, `default = []` unchanged in
  `engine/Cargo.toml` (zero external crates in the canonical build); runtime
  capability cascade `navigator.gpu` → WebGL2 probe → static fallback; CI
  golden-image shader tests on the software rasterizer. *Acceptance:* default
  `cargo build/test` dependency graph byte-identical to today; frontage scene
  renders interactively on a WebGL2-only budget-Android device; WebGPU absence
  falls back silently with the same asset. **Depends on P1-B1** (needs the
  splat asset to render).
- **P1-B3. Tier-C pre-render batch job.** Same splat crate + wgpu on the
  self-hosted box, offline, emitting panorama/fixed-viewpoint images to CDN;
  P0-B1's vector overlay draws on top unchanged. Completes the A/B/C rendering
  floor. **Depends on P1-B1.**

---

## 4. P2 — deferred, correctly (not dropped, gated on stated falsifiers)

**From `SYSTEMS` §10 (items 16–20):**
- Storage-engine layer under the log/backup store: WAL→LSM/B-tree→MVCC (today:
  log is WAL-adjacent, `backup.rs` is content-addressed blob store, but no
  on-disk indexed structure, `fsync`/checkpoint protocol, or MVCC exists).
- Mesh membership/discovery: SWIM/HyParView gossip + DHT (currently explicitly
  "out of scope" in `iroh_transport.rs:23`).
- Formalize the 3-tier locality model (§6.3: within-process/within-host-IPC/
  cross-host-mesh) as an explicit engineering rule with per-tier bandwidth
  budgets.
- Nomad for multi-node scheduling, once the fleet exceeds ~5 nodes.
- DECART-gated search of awesome-rust/awesome-wasm/awesome-distributed-systems/
  awesome-post-quantum before any manual reinvention.

**From `GAUSSIAN-SPLATTING` §4 (P2 items):**
- brush (Rust-native trainer) bake-off — same imagery, compare PSNR/time/$;
  swap the adapter backend only on a win; DECART report either way.
- TimesFM aggregate demand forecasting — gated on beating a classical seasonal
  baseline (Holt-Winters/ETS, STL, Theta) on a backtest of dowiz's own held-out
  demand history; out-of-kernel ops adapter only if it wins.
- Hetzner GEX44 standing GPU — gated on sustained volume that would saturate a
  standing box; a backend swap behind the existing port, zero kernel change.
- LOD machinery (CLoD-GS budget-based rendering, progressive streaming) — a
  WebGPU-tier scaling nicety; the frontage crop already makes it unnecessary
  for the budget tier that matters first.
- Facade "straightening" (PCA/min-area-box view rotation) — `arrow_screen_
  rotation_deg` already accepts `view_rotation_deg`, so the seam exists for
  later.
- ColBERT-style reranker — gated on (a labelled corpus exists) AND (the fusion
  gap is proven large on the extended oracle).

---

## 5. Explicit rejections (preserved verbatim in substance — load-bearing, not to be re-litigated casually)

**From `SYSTEMS` §11:**
- **Managed cloud (AWS/RDS/EKS) as default** — adapter-only, behind existing
  traits, never the architecture.
- **Kubernetes** — the zero-OCI rule (`check-zero-oci.sh`, `Dockerfile` final
  stage `FROM scratch`) architecturally excludes it; Nomad/systemd instead.
- **GraphQL as a mesh/inter-node protocol** — edge/client-facing only; a
  trusted-resolver model contradicts the mesh's deny-by-default capability
  trust.
- **IAM-style centralized reputation/roles, or any reputation/blacklist trust**
  — capability tokens only; already a closed architectural decision
  (`NO-COURIER-SCORING`).
- **Literal GPU/CUDA adoption** — no GPU in the stack; value transfers as
  design principles onto CPU, never as hardware. *(This is the rejection that
  independently matches the Gaussian-Splatting report's cross-confirmed
  "GPU is always offline/behind-a-port" finding — see §1 above.)*
- **"Digital microcontroller" as a description of current code** — accepted
  only as a north star; the numeric layer is allocation-heavy today, which
  directly contradicts the model until P0-A1/P1 core-pinning items close that
  gap.

**From `GAUSSIAN-SPLATTING` §5:**
- **Full satellite/aerial reconstruction pipeline** — rejected on three
  independent walls (legal ToS prohibitions on derivative datasets; nadir
  imagery is the wrong input shape for multi-view GS; upfront-global-coverage
  economics). Any one wall suffices; the legal one is absolute.
- **Apartment-level indoor identification** — rejected from the roadmap
  entirely; open global indoor floor-plan data at apartment resolution does
  not exist. A data-existence problem, not a funding problem — must not appear
  on a roadmap as if money unlocks it.
- **Persistent GPU rental as the starting point** — €184-889/mo standing idle
  vs. ~$0.10-1.67/job; break-even needs hundreds-to-thousands of
  reconstructions/month, which one-time-per-address-cached-forever will not
  produce.
- **CuPy as trainer, default preprocessor, or kernel dependency** — kernel
  verdict unchanged (Tier 3, no hook, N≤32 eigensolve 2-3 orders below GPU
  break-even); no mainstream 3DGS trainer uses CuPy as its core.
- **Trained ML reranker (cross-encoder/ColBERT/LLM-listwise) now** — no
  labelled corpus; fusion is right-sized; LLM-listwise is additionally
  non-deterministic, the opposite of a bit-reproducible kernel primitive.
- **TimesFM for per-order ETA** — Kalman/EMA are already the optimal linear
  estimator for this problem; a 200M-parameter transformer loses on latency,
  determinism, footprint, and explainability simultaneously.
- **A manufactured TimesFM↔Gaussian-Splatting bridge** — no shared math
  surface; the only adjacency is a queue boundary already better served by the
  delivery-volume heuristic.
- **Software GPU emulation for training** — 2-3+ orders of magnitude too slow;
  kept strictly as a CI shader-correctness/smoke-test tool, never a training
  fallback.
- **SaaS rendering APIs for Tier C** — violates no-cloud-by-default /
  self-hosted-first; pre-render on the project's own box needs no always-on
  GPU at all.
- **graphdeco-inria reference implementation as a shipped dependency** —
  permanently rejected (non-commercial licence); Apache-2.0 alternatives
  (mosure, web-splat, brush, gsplat) cover every role.

**From `HK05` §5 (a scope boundary, not a rejection, but load-bearing the same way):**
- **HK-05/HK-09 routing as a dowiz product feature today** — explicitly
  declined. It is agent-session dev-tooling; dowiz runs no production LLM
  agents that would consume this router. The one legitimate future hook named
  in the source doc: *if* dowiz ever ships an AI-agent product feature (e.g. an
  owner-panel AI assistant), EV-driven tiered routing is already a proven
  pattern to reuse under the same `bebop mcp` port — but that is a conditional
  future hook, not present-day roadmap scope, and forcing it onto the roadmap
  now would be the same overclaiming this project rejects elsewhere.

---

## 6. Blueprint template applied consistently above

Every P0 item in §2 states: **files touched** (so "blueprint" means a concrete
diff surface, not a vague direction), **why P0** (the corroboration or
incident that earns it priority), **acceptance criteria** (falsifiable —
matches the project's own VERIFIED-BY-MATH discipline referenced in `SYSTEMS`
§7), and **dependencies on other P0 items** (mostly none — the three source
reports' roadmaps are largely parallel work, confirmed by the no-overlap check
in §1).

---

*Synthesized from the three 2026-07-16 source documents listed at the top of
this file. No roadmap item here was invented; every item traces to a numbered
section in one of the three sources.*
