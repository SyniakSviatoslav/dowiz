# Perf Benchmark Coverage Map — dowiz/DeliveryOS + bebop-repo

> Date: 2026-07-18 · Author: Opus research pass · Scope: `criterion`/native perf benches across both repos.
> Goal (operator): *"покрити бенчами… усі репо"* — map what hot/public code IS vs ISN'T benched, then propose
> the highest-value additions. Trigger: two uncovered non-constant-complexity functions
> (`kernel/src/retrieval/ppr.rs`, `kernel/src/absorbing.rs`) — **being handled by sibling passes**, see §0.

All `path:line` and signatures were verified against live source on 2026-07-18 (not memory, not the stale index).

---

## §0 · Sibling-pass carve-out (do not duplicate)

`kernel/src/retrieval/ppr.rs::Ppr::rank` and `kernel/src/absorbing.rs::fundamental_matrix` are owned by two
separate research passes. **Live-truth correction:** both already have a *single-size* bench + committed
baseline on disk today:

| bench id (baseline.json) | target | current shape | gap the sibling pass closes |
|---|---|---|---|
| `ppr/rank_32x32_k20` | `Ppr::rank` (criterion.rs:186) | one fixed n=32, k=20 | O(k·n²) growth curve is invisible at a single point — needs an n-sweep |
| `absorbing/fundamental_matrix_16` | `absorbing::fundamental_matrix` (criterion.rs:205) | one fixed n=16 | O(n³) growth curve invisible at a single point — needs an n-sweep |

This document does **not** redesign those two. Everything below is the *rest* of the surface.

---

## §1 · Existing bench inventory (what IS covered)

Five bench binaries exist across both repos. `dowiz` benches are `criterion` (harness=false) auto-tracked by
`kernel/benches/bench_track.py` → `tools/telemetry/native-trackers` against committed `baseline.json`
(regression gate, default 10%). bebop's `verify_lane.rs` is a zero-dep `std::time` binary (measured number,
not a gate).

### 1a. `dowiz/kernel/benches/criterion.rs` — 13 baseline entries

| bench id | target function (`kernel/src/…`) | bench L# |
|---|---|---|
| `place_order/5_items` | `place_order` (lib re-export → order path) | 16 |
| `fold_transitions/5_hops` | `order_machine::fold_transitions` | 56 |
| `empirical_identify/20k_samples`, `/end_to_end_20k` | `empirical_identify` + `sample_backdoor` (causal) | 73 |
| `token_bucket/try_acquire_permit` | `token_bucket::TokenBucket::try_acquire` | 95 |
| `spectral_cache/slem_cached_10x10_hit` | `spectral_cache::slem_cached` + `csr::NormalizedTile::from_dense` | 109 |
| `spectral_cache/canonical_address_32x32` | `spectral_cache::canonical_content_address` | 133 |
| `graph_rebuild_rank/heap`, `/arena` | `csr::Csr::from_edges(_in)` → `row_normalize(_in)` → `personalized_pagerank(_in)` | 151 |
| `ppr/rank_32x32_k20` | `retrieval::ppr::Ppr::rank` *(sibling pass, §0)* | 186 |
| `absorbing/fundamental_matrix_16` | `absorbing::fundamental_matrix` *(sibling pass, §0)* | 205 |
| `retrieval/recall_at_k_5` | `retrieval::recall::PrimaryRecall::recall_at_k` (BM25+trigram fusion) | 222 |
| `attention/matmul_8x8` | `attention::attention` | 230 |

### 1b. `dowiz/llm-adapters/benches/criterion.rs`

| bench id | target | bench L# |
|---|---|---|
| `cache/exact_hit_decode` | `llm_adapters::cache::CachingBackend::chat` (Exact-hit decode path only; live Ollama is a probe, not a baseline) | 40 |

### 1c. `dowiz/agent-adapters/benches/fuel_bench.rs`

| bench id | target | bench L# |
|---|---|---|
| `fuel_loop_throughput/needs_{1,8,64,256}` | `agent_adapters::fuel::FuelTrancheRunner::run` (prepaid tranche loop; `FUEL_PER_UNIT` B4 placeholder-calibration) | 37 |

### 1d. `bebop-repo/bebop2/core/benches/verify_lane.rs` (zero-dep, gated `--features test_keygen`)

| what | target | note |
|---|---|---|
| ML-DSA-65 verify, scalar vs lane | `pq_dsa::verify_internal_bytes` vs `pq_dsa::verify_internal_bytes_many`, N∈{1,4,16,64} | timed |
| Ed25519 verify, scalar vs batch | `sign::verify` vs `sign::verify_many`, N∈{1,4,16,64} | timed (feature-gated) |
| setup only — **NOT timed** | `pq_dsa::keygen_bytes`, `pq_dsa::sign_internal_bytes`, `sign::keygen`, `sign::sign` | corpus build |

### 1e. `bebop-repo/crates/bebop/benches/criterion.rs`

| bench id | target | bench L# |
|---|---|---|
| `loop_cycle/benign` | `bebop::loop_runtime::LoopRuntime::cycle` | 7 |
| `wire/benign` | `bebop::wiring::wire` | 24 |

### 1f. Crates with ZERO bench coverage

`dowiz/engine` · `bebop2/proto-cap` · `bebop2/proto-wire` · `bebop2/proto-crypto` · `bebop2/delivery-domain` ·
`bebop2/mesh-node` · `bebop2/wasm-host` · `bebop2/ports/*` · all `dowiz/tools/*` (tools are dev-tooling, out of
scope here). Note the dowiz **kernel** has its *own* PQ crypto (`kernel/src/pq/*`) that is **entirely
unbenched** — `verify_lane.rs` lives in the *separate* `bebop2_core` crate and covers a *different*
implementation.

---

## §2 · Prioritized UNCOVERED hot-path functions

Tiering: **T1** = money / order / dispatch / crypto-verify-sign / settlement — a regression is user-visible or
security-relevant. **T2** = spectral / graph / retrieval / physics math with non-constant complexity — silent
perf cliffs. Getters, constructors, config/one-off init, and thin external-crate delegations are excluded.

### 2A · dowiz `kernel` (crate `dowiz_kernel`)

**Tier 1 — money / order / crypto-verify-sign / mesh**

| path:line | signature | complexity | hot caller | why it matters |
|---|---|---|---|---|
| `pq/dsa.rs:911` | `verify_internal_bytes(pk,msg,sig)->bool` | crypto-heavy (13 NTTs, matrix pointwise, 2× SHAKE256 CRH, hint) | `envelope.rs:69`, `root_delegation.rs:71/80`, `mesh.rs:132` | ML-DSA-65 **verify** on every signed envelope, done-gate root attestation, mesh entry — #1 latency anchor of the signed path; kernel's own impl is fully unbenched |
| `pq/dsa.rs:767` | `sign_internal_bytes(sk,msg,rnd)->Vec<u8>` | crypto-heavy + **unbounded reject loop** | `envelope.rs:54` (seal), mesh signer | ML-DSA-65 **sign** per signed message; reject-sampling makes cost variable → a regression is a per-signature stall on writes |
| `pq/kem.rs:332` | `decaps_internal(sk,c)->Vec<u8>` | crypto-heavy, **re-encrypts** for implicit-reject | `hybrid.rs:93`→`volume.rs:171` (open_volume) | ML-KEM-768 **decaps**, heaviest single KEM op, on the volume-open/receive path |
| `pq/kem.rs:285` | `encaps_internal(pk,m)->(Vec<u8>,Vec<u8>)` | crypto-heavy (K×K matrix, NTTs, CBD, compress) | `hybrid.rs:66`→`volume.rs:128`; also *inside* decaps | ML-KEM-768 **encaps** on every seal AND inside every decaps consistency check |
| `pq/hybrid.rs:92` | `hybrid_decaps(&own,&ct)->Result<[u8;32],&str>` | ML-KEM decaps + x25519 DH + 3× SHAKE256 combine | `volume.rs:171` | composite hybrid-open on the settlement/volume receive path |
| `pq/keccak.rs:139` | `shake256(input,out)` | O(n) Keccak-f[1600] sponge | base primitive under dsa/kem/envelope/hybrid | multiplies across every hashing loop of every sign/verify — the base anchor of the whole crypto lane |
| `event_log.rs:30` | `sha3_256(input)->[u8;32]` | O(n) Keccak per 136-B block | `chunker.rs`, `backup.rs`, `spine.rs`, `mesh.rs:151`, `ports/payment.rs`, `ports/customer.rs:144` | content-address hash under every event-id / chunk-id / capability-id / payment ref |
| `mesh.rs:225` | `verify_chain(&self)->Result<(),MeshError>` | **O(n)** (n× ML-DSA verify + n× SHA3 link hash) | mesh-sync chain validation | per-entry verify cost scales linearly — a regression is invisible until a long chain replays |
| `mesh.rs:132` | `verify_sig(&self)->bool` | single ML-DSA verify | `mesh.rs:229` (verify_chain inner loop) | the per-entry unit of `verify_chain` |
| `money.rs:230` | `ledger_sum(ledger)->i64` | **O(n²)** — per non-reversal Earn, a full ledger `any()` scan (confirmed L236-238) | `domain.rs:105` `Order::ledger_balance` | money-conservation probe on every order balance; **quadratic** in ledger length — the highest-signal money finding |
| `money.rs:185` | `ledger_append(ledger,entry)->Result<Vec,String>` | O(n) (dup-id + reversal-target + already-reversed scans) | `domain.rs:98`, `money.rs:262` | every ledger mutation linear-scans the whole ledger fail-closed |
| `money.rs:252` | `reverse_transfer(ledger,earn_id,rev_id)->Result<Vec,String>` | O(n) find + `ledger_append` O(n) | `domain.rs:391` `compensate` | fail-closed refund/cancel driver; money-critical |
| `catalog.rs:61` | `unit_price(&self,product_id,modifier_ids)->Result<i64,String>` | O(m) over modifiers, `checked_add` | `domain.rs:217` `place_order_priced` | authoritative server-side price re-derivation per cart line (ignores client price) |
| `intake.rs:552` | `admit(spec)->Result<Witness,IntakeError>` | Tier-A O(n) + Tier-B **AC-3 arc-consistency fixpoint** | `loops.rs:182` LoopSpec certification | heaviest non-money loop; costliest certification gate |

**Tier 2 — spectral / graph / retrieval / physics math**

| path:line | signature | complexity | hot caller | why it matters |
|---|---|---|---|---|
| `spectral.rs:225` | `eigenvalues(a)->Vec<Complex>` | **O(n³)** (n≤32 Hessenberg-QR) / **O(n⁴)** (n>32 Faddeev-LeVerrier) | `event_log.rs:434`, `hydra.rs:221`, `spectral_cache.rs:268`, `markov.rs:211` | the shared eigen-solve every spectral quantity funnels through; only the cache-HIT path is benched — regression is silent across drift/mixing/energy, and there is a **complexity step at n=32** |
| `mat.rs:132` | `matmul_contig(a,b)->Mat` | **O(n³)** triple loop | `kalman.rs` predict/update (~15×/cycle), `absorbing.rs:34`, `spectral.rs:39` | real-time Kalman core (courier tracking / ETA); independent of the eigen path, fully unbenched |
| `spectral.rs:679` | `classify_drift(a)->DriftClass` | O(n²) scan + O(n³) spectral_radius | `event_log.rs:434` (commit drift-gate), `spectral_cache.rs:268`, `hydra.rs:60` | drift GATE on every durable mesh-event commit |
| `csr.rs:552` | `laplacian_spmv(&self,x,out,kind)` | O(nnz) | `engine/bridge.rs:125`, `engine/field_energy.rs:79` | per-frame field-UI Laplacian diffusion in the render loop; distinct from the benched PPR spmv |
| `markov.rs:110` | `analyze_detailed(states)->DetailedReport` | O(300·n²) power-iter + O(n³) eigenvalues | `bin/markov_attractor.rs` | full attractor/drift detector run every tool-outcome window in the self-improvement loop; pipeline itself unbenched |
| `kalman.rs:212` | `update(&mut self,z)->bool` | O(n³) Gaussian-elim inverse + ~6 matmuls | `domain.rs` TrustEstimate, `evals.rs` (600-step loop) | measurement fold for trust/ETA; matrix-inverse is the costliest kernel op |
| `kalman.rs:200` | `predict(&mut self)` | O(n²) — 3 matmuls + transpose | `domain.rs` TrustEstimate, `evals.rs` | state-propagation paired with `update` every fold step |
| `retrieval/bm25.rs:222` | `rank(&self,query)->Vec<Scored>` | O(D·\|Q\|) + O(D log D) sort | `retrieval/recall.rs:148/233` | core BM25 pass; `recall_at_k` covers fusion end-to-end but never isolates pure ranking cost |
| `harmonic.rs:26` | `harmonic_centrality(n,edges)->Vec<f64>` | O(n·(V+E)) — one BFS per source | `wasm.rs` (`harmonic_centrality_logic`) | HK-05/06 model-routing + memory ranking; per-source BFS is the cost driver |
| `geo.rs:70` | `progress_along_route(poly,pos)->RouteProgress` | O(n) polyline projection + haversine | `ports/customer.rs` `TrackingView::from_positions` | recomputed per courier position on live customer tracking |

**Explicitly excluded (verified: no live prod caller today):** `router.rs::route`/`build_shortcuts` (Dijkstra/CH,
staged not wired), `pq/x25519.rs::x25519` + `hybrid_encaps` (thin dalek delegation), `spectral_laplacian.rs`,
`spectral::eigh`, `householder::eigh_contig`, `spectral::topk_symmetric`, `dsu::kruskal_mst`,
`cgraph::d_separated_bi` (zero non-test callers), `causal.rs` do-calculus (transitively covered via
`empirical_identify`).

### 2B · dowiz `engine` (crate `dowiz-engine`) — ZERO benches today

**Tier 1 — per-frame field/motion (runs every animation frame) + money guard**

| path:line | signature | complexity | hot caller | why it matters |
|---|---|---|---|---|
| `field_frame.rs:198` | `FieldFrame::step(&mut,source,eq)` | O(w·h) stencil + integrate | `wasm/lib.rs:92` (per rAF frame) | THE core per-frame integrator; runs continuously all session — top hot path, zero coverage |
| `field_frame.rs:140` | `laplacian_into(u,w,h,out)` | O(w·h) 5-point stencil | `field_frame.rs:202` (every frame) | pure inner diffusion kernel inside `step` |
| `field_frame.rs:229` | `FieldFrame::frame_rgba(&self)->Vec<u8>` | O(w·h) + w·h·4 alloc/frame | `wasm/lib.rs:97` | per-painted-frame; per-pixel hue map + fresh Vec alloc |
| `field_frame.rs:255` | `compose(scene,eq,w,h,steps)->Vec<u8>` | O(w·h·(shapes+steps)) | `wasm/lib.rs:35,57` FFI | the single full-frame pipeline the GPU blit consumes |
| `motion.rs:50` | `Spring::step(&mut,dt)` | O(⌈ω·dt/0.1⌉) substep loop × props | per animated property/frame | critically-damped easing; ω-dependent cost |
| `scene.rs:122` | `Scene::render_frame(&self,w,h)->Vec<f32>` | O(w·h·shapes) SDF fold | `wasm/lib.rs:80`, `compose` | builds the SDF source buffer; O(n²) in resolution |
| `bridge.rs:121` | `VertexBridge::apply_field(&mut,x)` | O(nnz) SpMV + `vec![0.;n]`/call | `wasm/lib.rs:112` FFI | per-frame graph-Laplacian physics; per-call heap alloc in a documented no-alloc loop |
| `zerocopy.rs:85` | `write_into_linear(mem,offset,buf)->usize` | O(n) f32→LE copy into WASM mem | FE-01 upload boundary | Rust→GPU upload leg every frame |
| `money_guard.rs:60` | `TweenGuard::present_money(amount_minor)->Result<i64,String>` | O(1) | money-presentation call sites | 🔴 RED-LINE money-never-tween guard; cheap but pin its baseline |

*(`field_energy.rs` is `#[cfg(test)]`-only — a reference oracle, correctly not a bench target.)*

### 2C · bebop `bebop2_core` — verify benched, **sign/KEM/seal are NOT**

| path:line | signature | cost | hot caller | why it matters |
|---|---|---|---|---|
| `pq_dsa.rs:1155` | `sign(sk,msg,rnd)->MlDsa65Sig` (wraps `sign_internal_bytes:836`) | variable (reject loop) | `proto-cap/kv_signer.rs:136`, `signed_frame.rs:202` | ML-DSA-65 **sign** — hybrid-gate sign half is *completely untimed*; single most expensive per-message op |
| `sign.rs:892` | `sign(seed,msg)->[u8;64]` | fixed (SHA-512 + scalar-mult) | `kv_signer.rs:131`, `signed_frame.rs:186` | Ed25519 **sign** — classical leg of every hybrid frame; setup-only in bench |
| `pq_kem.rs:703` | `decaps(dk,ct)->SharedSecret` | heaviest KEM (decrypt + full re-encrypt) | proto-crypto ladder, `vault.rs` | ML-KEM-768 decaps — per-handshake receive side |
| `pq_kem.rs:674/693` | `encaps_internal(ek,m)` / `encaps(ek,rng)` | fixed (NTT + matrix) | proto-crypto ladder | per-handshake session-key establishment |
| `aead.rs:278` | `aead_xchacha20_poly1305_encrypt(key,nonce24,pt,aad)->(Vec<u8>,[u8;16])` | **size-dependent** | `at_rest.rs:121` | SEAL for the at-rest store — every persisted record; length-scaling unmeasured |
| `aead.rs:300` | `aead_xchacha20_poly1305_decrypt(…)->Option<Vec<u8>>` | **size-dependent** | `at_rest.rs:155` | OPEN for at-rest reads (CT tag check); read throughput unmeasured |
| `hash.rs:344` | `sha3_256(msg)->[u8;32]` | **size-dependent** | `event_log.rs:61`, `proto-cap/tlv.rs:124`, `proto-wire/handshake.rs:30`, `revocation.rs:130` | highest-frequency crypto primitive — once per event AND per signed message; unbenched at any size |
| `x25519.rs:356` | `x25519(k,u)->[u8;32]` | fixed Montgomery ladder | `x25519::encaps/decaps` | sovereign DH baseline for the hybrid handshake (swap-in vs external dalek) |

*(Not a gap: `key_v_verifier::verify_key_v_verdict` — trivial TLV parse over the already-benched
`pq_dsa::verify_internal_bytes`.)*

### 2D · bebop `bebop2_proto_cap` + `bebop2_proto_wire` — ZERO benches today

| path:line | signature | complexity | hot caller | why it matters |
|---|---|---|---|---|
| `proto-cap/hybrid_gate.rs:124` | `HybridGate::check(&self,frame,roster,chain,revocations,now)->CapResult<()>` | O(chain) + 2–3 sig verifies | `iroh_transport.rs:395`, `stdio_transport.rs:272`, `wss_transport.rs:616`, `facade.rs:126` | **THE per-frame authorization gate** — runs on every inbound frame; highest-value hot path in the crate |
| `proto-cap/signed_frame.rs:229` | `SignedFrame::verify_pq(&self)->CapResult<()>` | 1× ML-DSA verify + domain rebuild | `hybrid_gate.rs:181` | dominant crypto cost per frame under `RequireBoth` |
| `proto-cap/signed_frame.rs:208` | `SignedFrame::verify_classical(&self)->CapResult<()>` | 1× Ed25519 + domain rebuild | `hybrid_gate.rs:171` | always-verified classical leg; re-derives TLV domain per call |
| `proto-cap/tlv.rs:81` | `tlv_signing_input(domain_tag,struct_tag,wire_version,fields)->Vec<u8>` | O(fields) + `sort_by_key` | under every sign AND verify | the canonical leaf allocation that all signing-domain construction bottoms out in |
| `proto-cap/roster.rs:252` | `verify_chain(roster,chain,cap,now)->CapResult<()>` | O(chain), per-link Ed25519 + subset | `hybrid_gate.rs:142` | delegation-chain validation; scales with chain length (isolates cost `check` hides) |
| `proto-cap/scope.rs:167` | `Scope::is_subset_of(&self,super)->bool` | **O(n·m)** `Vec::contains` | `roster.rs:289/306` per link | attenuation lattice check per delegation link; quadratic on grant-count, no HashSet |
| `proto-cap/matcher.rs:63` | `assign(order,candidates,max)->Vec<CourierKey>` | O(n) HRW + O(n log n) sort | `matcher.rs:81`; `hrw_weight` reused by `delivery-domain/hub_ring.rs:24` | per-order courier assignment — the dispatch scan |
| `proto-wire/wire_codec.rs:198` | `encode_frame(frame)->WireResult<Vec<u8>>` | O(fields)+O(chain) + sort | `iroh_transport.rs:347`, `bpv7.rs:382` | serializes every outbound frame to canonical wire bytes |
| `proto-wire/wire_codec.rs:240` | `decode_frame(buf)->WireResult<SignedFrame>` | O(fields)+O(chain) parse | `iroh_transport.rs:364`, `wss_transport.rs:561` | deserializes every inbound frame **before** verify — hostile-input hot path |
| `proto-wire/framing.rs:25/41` | `encode(env)->…` / `decode(&mut buf)->Option<Envelope>` | const + `drain` memmove | `iroh_transport.rs:349/362` | length-prefix framing on every carrier read/write |
| `proto-wire/envelope.rs:41/50` | `to_bytes(&self)` / `from_bytes(bytes)` | serde_json O(payload) | `framing.rs:26/53` | the **only serde_json on the carrier path** — quantify JSON overhead vs the hand-rolled TLV it wraps |

### 2E · dowiz adapters (beyond the two already benched)

| path:line | signature | complexity | why it matters |
|---|---|---|---|
| `agent-adapters/cache.rs:58` | `AgentCache::key(canonical_request)->[u8;32]` | O(len) sha3_256/request | agent-side cache-key hashing; agent-adapters has NO cache bench (only fuel) |
| `agent-adapters/cache.rs:63` | `AgentCache::get(&self,key)->Option<Vec<u8>>` | O(1) lock+get+clone | agent analog of the ONE llm path that IS benched (exact-hit decode) |
| `agent-adapters/dispatch.rs:57` | `decode_track_record(line)->Option<TrackRecord>` | O(len) serde_json/row | per-row parse folded over the whole EV telemetry ledger |

---

## §3 · Proposed criterion harness structure (top-priority first)

Design rules matched to the existing harness: `[[bench]]` `harness=false`; deterministic inputs only (no live
daemon / network / RNG-from-entropy — seed everything); use `BenchmarkGroup` + `bench_with_input` for the
size-sweeps so `bench_track.py` auto-seeds each `<group>/<size>` id into `baseline.json`. **The sweeps are the
point** — a single fixed size cannot reveal an O(n²)/O(n³) regression; picking 3–4 sizes that straddle any
complexity step (e.g. eigenvalues at n=32) turns the baseline into a growth-curve gate.

### Wave 1 — extend `dowiz/kernel/benches/criterion.rs` (no new crate wiring)

```rust
// GROUP: money_ledger — exposes the O(n²) ledger_sum + O(n) append/reverse. Highest money signal.
//   ledger_sum(&[LedgerEntry]) -> i64                 sweep n ∈ {8, 64, 256, 1024}   // O(n²) reversal scan
//   ledger_append(Vec<LedgerEntry>, LedgerEntry)      sweep n ∈ {8, 64, 256, 1024}
//   reverse_transfer(Vec<LedgerEntry>, u64, u64)      at   n ∈ {64, 1024}
//   catalog::PriceTable::unit_price(&str, &[String])  sweep m modifiers ∈ {0, 4, 16}
// Fixture: build a synthetic ledger of n alternating Earn/Reversal legs from a fixed seed.

// GROUP: kernel_crypto_pq — the kernel's own (unbenched) PQ lane. Fixed-size, one pre-generated corpus.
//   pq::dsa::verify_internal_bytes(&pk,&msg,&sig)     one valid (pk,msg=32B,sig)     // verify anchor
//   pq::dsa::sign_internal_bytes(&sk,&msg,&rnd)       fixed sk, msg=32B, rnd fixed   // variable reject loop
//   pq::kem::encaps_internal(&pk,&m)                  fixed pk, m=32B
//   pq::kem::decaps_internal(&sk,&c)                  fixed sk, c from encaps
//   pq::hybrid::hybrid_decaps(&own,&ct)               composite open
//   pq::keccak::shake256(&in, &mut out)  |  event_log::sha3_256(&in)  sweep len ∈ {64B, 1KiB, 64KiB}  // Throughput::Bytes

// GROUP: mesh_verify — O(n) chain growth.
//   mesh::MeshLog::verify_chain(&self)   sweep chain len n ∈ {1, 8, 64, 256}
//   mesh::…::verify_sig(&self)           single (per-entry unit)

// GROUP: spectral_math — the shared O(n³)/O(n⁴) surface + Kalman core.
//   spectral::eigenvalues(&[Vec<f64>])   sweep n ∈ {8, 16, 32, 48}   // STRADDLES the n=32 QR↔Faddeev step
//   mat::matmul_contig(&Mat,&Mat)        sweep n ∈ {8, 16, 32, 64}   // O(n³)
//   kalman::Kalman::update(&mut,&z) / predict(&mut)   at state dim ∈ {2, 4, 6}
//   spectral::classify_drift(&[Vec<f64>])             at n ∈ {8, 32}
//   csr::Csr::laplacian_spmv(&self,&x,&mut out,kind)  at n ∈ {256, 1024}, nnz≈2n

// GROUP: retrieval_geo
//   retrieval::bm25::Bm25Index::rank(&[String])       sweep D docs ∈ {100, 1000}   // isolate pure BM25 from fusion
//   geo::progress_along_route(&poly,pos)              sweep polyline ∈ {16, 128, 1024}
//   harmonic::harmonic_centrality(n,&edges)           at n ∈ {16, 64}   // per-source BFS
```
Register the new `criterion_group!` functions in the existing `criterion_main!`. `intake::admit` is a strong T1
target but its `EtalonSpec` fixture is non-trivial — defer to Wave 2 with a representative LoopSpec corpus.

### Wave 1 — NEW `dowiz/engine/benches/criterion.rs` (add `criterion` dev-dep + `[[bench]]` to `engine/Cargo.toml`)

```rust
// GROUP: field_frame — the continuous per-frame render cost. Grid sweep is the whole story.
//   FieldFrame::step(&mut,&source,&eq)   sweep grid ∈ {64×64, 128×128, 256×256}
//   field_frame::laplacian_into(&u,w,h,&mut out)      same grid sweep (isolate diffusion)
//   FieldFrame::frame_rgba(&self)        at 128×128 (per-pixel hue + alloc)
//   field_frame::compose(&scene,&eq,w,h,steps)        end-to-end at 128×128, steps=8
//   scene::Scene::render_frame(&self,w,h)             sweep shapes ∈ {1, 8, 32} at 128×128
//   motion::Spring::step(&mut,dt)         sweep ω to expose the substep-loop count
//   bridge::VertexBridge::apply_field(&mut,&x)        sweep nnz
//   money_guard::TweenGuard::present_money(f64)       single (RED-LINE baseline pin)
```

### Wave 2 — bebop crates

- **`bebop2/core/benches/verify_lane.rs`** (extend the existing zero-dep binary, or add a criterion sibling):
  add **sign** timing (`pq_dsa::sign`, `sign::sign` — currently untimed setup), **KEM**
  (`pq_kem::encaps`/`decaps`), sovereign `x25519::x25519`, and **size-swept** `aead_*_encrypt`/`decrypt` +
  `hash::sha3_256` over plaintext ∈ {64B, 1KiB, 64KiB}.
- **NEW `bebop2/proto-cap/benches/criterion.rs`** — group `gate`: `HybridGate::check` swept over chain len
  {0,1,4,16} (the per-frame auth gate); `SignedFrame::verify_pq`/`verify_classical` single; `tlv_signing_input`
  over field-count; `roster::verify_chain` swept over chain len; `matcher::assign` over candidates {8,64,256}.
- **NEW `bebop2/proto-wire/benches/criterion.rs`** — group `codec`: `encode_frame`/`decode_frame` swept over
  chain len; `framing::encode`/`decode`; `envelope::to_bytes`/`from_bytes` (serde_json cost).

### Priority order (biggest untimed cost on a money/auth/frame path first)

1. `kernel money_ledger` (esp. **`ledger_sum` O(n²)** — a real algorithmic cliff on every order balance).
2. `kernel kernel_crypto_pq` + `mesh_verify` (the entire kernel PQ sign/verify/KEM lane is unbenched).
3. `engine field_frame` (continuous per-frame cost, zero coverage today).
4. `bebop2 proto-cap gate` (`HybridGate::check` — per-frame auth) + `bebop2_core` sign/KEM/seal.
5. `kernel spectral_math` (the shared eigen/matmul/Kalman surface, straddle the n=32 step).
6. `proto-wire codec` + `kernel retrieval_geo` + adapter cache paths.

> Sibling passes own the `ppr.rs` / `absorbing.rs` n-sweeps (§0) — coordinate the sweep-size convention
> (`<group>/<n>` bench-id + `Throughput` where byte-sized) so all baselines share one growth-curve schema.
