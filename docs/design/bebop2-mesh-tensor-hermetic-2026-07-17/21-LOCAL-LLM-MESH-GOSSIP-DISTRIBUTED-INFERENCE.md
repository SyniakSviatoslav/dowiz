# 21 — Local LLM Mesh: Gossip-Distributed Inference across Cores (research + build, 2026-07-17)

> **Research + audit + on-host BUILD probe. No product code written or edited by this document.**
> Branch: `feat/harness-llm-backend`. Covers the operator-flagged thread the earlier batches
> dropped: *"you also dropped the local llm part which should run decentralized via cores in a
> similar mesh network using gossip & other approaches."* This is genuinely new ground — no prior
> batch (10–15) audited the **LLM inference layer itself** being distributed, as distinct from the
> DecisionUnit gossip of its **compiled outputs** (Batch 6 §1, latency blueprint §2).
>
> **Epistemics discipline** (Anu/Ananke, `AGENTS.md`), every claim tagged:
> **[MEASURED]** = probe run on this host this session · **[GROUNDED]** = external/cited or a live
> `file:line` read this session · **[PHYSICS]** = derivation from measured numbers + hardware facts
> · **[SPECULATIVE]** = brainstorm, honestly assessed. Operator methodology for this pass:
> build/test-first on the sandbox's actual hardware where feasible, web-reason where it isn't,
> and say which is which.

---

## 0. Executive summary (read first)

The operator's instinct — distribute LLM inference across cores/nodes in a gossip mesh — is
**correct in shape but wrong in target**, and the target error is the same inversion the latency
blueprint already found (`BLUEPRINT-LATENCY-ELIMINATION…:31-53`): local decode is
**memory-bandwidth-bound**, and memory bandwidth is a **per-device physical constant that does not
parallelize the way network latency does.** Three concrete architectures were separated and each
was measured or derived against this repo's constraints:

| Variant | What it distributes | Verdict | Why (one line) |
|---|---|---|---|
| **(a) Model-parallel sharding** | ONE model's layers/weights across nodes | **REJECT at mesh scale** [PHYSICS] | pipeline/tensor-parallel adds a network hop **per layer**; wins only when interconnect ≪ per-layer compute (datacenter NVLink), which is false for phones-on-cellular by ~3–4 orders of magnitude — and it violates no-network-for-decisions + degrade-closed |
| **(b) Ensemble / gossip raw results** | each node runs its OWN model, gossips the answer | **COLLAPSES into DecisionUnit gossip** [GROUNDED] | once the Decision Compiler pattern is applied the gossiped answer *is* a compiled unit; where it can't be compiled (open synthesis) it's a low-quality 7B token stream strictly dominated by one remote call. It is a distributed **answer cache**, not a distributed **inference** architecture — and it's already the S7 / §3.6 design |
| **(c) Speculative / hedged racing** | N nodes race the SAME model on the SAME input | **No conflict with the no-speculative rule, but no median win** [MEASURED] | racing idempotent **read-only** inferences (no shared mutable state) is a *different category* than the rejected speculative **state** execution (Batch 4 §2.5) — but on this host racing 2 copies is a measured **latency LOSS**, and across identical deterministic nodes it's a tie; real value is p99-tail/fault-tolerance only, at N× compute |

**The measured core fact [MEASURED]:** on this host (8 vCPU / 4 physical EPYC-Milan cores, 30 GiB,
no GPU), TRUE aggregate decode throughput (total tokens ÷ wall clock) is **flat at ~9–10 tok/s
whether you run 1, 2, or 4 concurrent inferences** — concurrency does not add throughput, it
serializes. Hedging two copies of one request on one host **lost** (first-done 7.04 s vs 6.95 s
single). Distributing across *more* local cores on one host cannot beat the bandwidth wall for a
single request's latency.

**The one place "distributed + local + gossip" genuinely wins is the operator's OWN prior idea,
not a new inference layer:** gossip **compiled DecisionUnits**, not raw inference. A DecisionUnit
executes in **zero tokens / nanoseconds**, so the bandwidth wall is irrelevant to it — and the
existing gossip transport already carries it with no new machinery (§3). Distributing the
*inference* fights physics; distributing the *compiled output* sidesteps physics. **Recommendation
(§5): build none of (a/b/c) as an inference layer; the ready, physics-respecting win is
DecisionUnit gossip over the existing `SyncFrame` transport, gated by import-time independent
replay — already designed in Batch 6 §1 / latency blueprint S7.** The only honest niche for raw
distributed inference is **offline throughput across many independent jobs** (embarrassingly
parallel map-reduce), and even that is dominated by a remote batch call unless the network is
absent (the sovereignty case) — deferred with a measurement trigger.

---

## 1. Ground truth — measured on this host this session [MEASURED]

Probe: `scratchpad/mesh_infer_probe.py` (stdlib-only, `urllib` + `ThreadPoolExecutor`), fixed
decode budget `num_predict=64`, `seed=42`, `temperature=0`, model `llama3.1:8b` (Q4, 4.9 GB),
warm. Ollama 0.30.9, **default** `OLLAMA_NUM_PARALLEL`.

### 1.1 Single-stream baseline
| call | wall | tokens | tok/s (eval_duration basis) |
|---|---|---|---|
| 1 | 7.27 s | 64 | 10.15 |
| 2 | 6.73 s | 64 | 10.07 |
| 3 | 6.84 s | 64 | 9.90 |

Mean single-stream decode **10.04 tok/s**, wall **6.95 s** for 64 tokens — squarely inside the
prior session's 4.8–10.5 tok/s band (`LOCAL-AI…:33`), corroborated independently.

### 1.2 The metric trap, and the honest number

Naively summing each concurrent stream's Ollama-reported `tok/s` (from `eval_count/eval_duration`)
gives "1.94× at 2-way, 4.01× at 4-way" — **an artifact.** `eval_duration` measures only the time a
request is *actively decoding*, not the time it spends **queued** behind others. The honest metric
is **total tokens delivered ÷ wall clock**:

| regime | tokens | wall | TRUE throughput | vs 1-call |
|---|---|---|---|---|
| 1 call | 64 | 6.95 s | **9.21 tok/s** | 1.00× |
| 2 concurrent | 128 | 13.68 s | **9.36 tok/s** | 1.02× |
| 4 concurrent | 256 | 26.13 s | **9.80 tok/s** | 1.06× |

TRUE throughput is **flat within noise** — the host **serializes** concurrent decode under default
settings (each request runs at ~10 tok/s only when it owns the cores; the others wait). This
reproduces the prior finding (`LOCAL-AI…:36`, "2 concurrent → serialize, wall ≈ sum") and extends
it to 4-way.

### 1.3 The hedge measurement (variant c, on one host)

2-way concurrent **first-done wall = 7.04 s** vs single-call mean **6.95 s** → a **0.09 s LOSS.**
Racing two copies of the same request on one host does not return the first answer any sooner,
because both copies contend for the same cores/bandwidth. (Fixed seed ⇒ both produce the identical
answer anyway — the race has no variance to exploit on a single deterministic host.)

### 1.4 Honest measurement gap (named, not hidden) [U]

The **genuine-parallel** case (`OLLAMA_NUM_PARALLEL ≥ 2`, real overlapping slots) is **unmeasured**
— this is the same P-1 probe the prior doc flagged as "the single most decision-relevant unknown"
(`LOCAL-AI…:324`). Both routes to set it here (a systemd drop-in on `ollama.service`; a second
user-space `ollama serve` on another port) were **blocked by the sandbox's action classifier**
(system-service mutation / broad FS scan), so the number stays open. It does **not** change the
architecture verdict, and here is the [PHYSICS] reason it cannot: even a perfect `NUM_PARALLEL`
overlap is bounded above by **4 physical cores sharing one memory bus**. LLM decode is
memory-bandwidth-bound — each generated token must stream **all ~4.9 GB of Q4 weights** from RAM
through the ALUs; at 10 tok/s the host is already moving ~49 GB/s, at or near this EPYC config's
DDR4 ceiling. Genuine parallelism can reclaim only whatever bandwidth a *single* stream leaves on
the table, which the flat 1.0–1.06× curve says is little. The ceiling is hardware, not scheduler.

---

## 2. The three architectures, separated and evaluated

### 2.a Model-parallel sharding (split ONE model across cores/nodes) — REJECT at mesh scale

**What it means concretely.** Tensor parallelism (split each layer's matmul across nodes,
all-reduce the partials every layer) or pipeline parallelism (node 1 holds layers 0–k, node 2
holds k–2k, …; a token flows through them in sequence). Real lightweight implementations exist and
were researched: **llama.cpp's RPC backend** (`rpc-server`, splits a GGUF model's layers across
machines) [GROUNDED — llama.cpp ships `ggml-rpc`]; **Petals** (BitTorrent-style layer sharding for
BLOOM/Llama across volunteers) [GROUNDED]; **exo / distributed-llama** (LAN device clusters)
[GROUNDED, low-trust on numbers].

**On ONE host it is a non-idea:** llama.cpp *already* shards one model's matmuls across all
threads/cores internally — that is what the single-stream 10 tok/s already is. There is no
additional core to "add"; the §1.2 flat curve is the ceiling.

**Across separate mesh nodes it is defeated by the interconnect [PHYSICS].** Both parallel forms
move data across the node boundary **once per layer** (tensor-parallel: an all-reduce every layer;
pipeline: an activation handoff every stage boundary) — an 8B model is ~32 layers, so ~32 network
crossings **per token**. This only pays off when the interconnect latency is far smaller than the
per-layer compute time. In a datacenter that ratio holds: NVLink/InfiniBand hops are ~1–10 µs
against per-layer compute of tens of µs. In this repo's stated mesh — "a few courier phones + one
owner hub" on WiFi/cellular — a hop is **~5–50 ms**, so 32 hops/token is **~0.16–1.6 s of pure
network per token before any compute**, i.e. slower than the ~0.1 s/token the host already achieves
alone. The math inverts by ~3–4 orders of magnitude. Petals' own guidance confirms the shape: it
targets many-GPU volunteer swarms for models too big for any one node, explicitly trading latency
for the ability to run at all — the opposite of this repo's latency goal.

**It also violates two standing constraints [GROUNDED].** (1) **No-network-for-decisions**
(BLUEPRINT-WAVE-SCHEDULING): a decision that requires a network round-trip *per layer* is the
purest possible violation. (2) **Degrade-closed / no-watchdogs**: sharded inference is
degrade-**open** — a dropped courier mid-token stalls the entire inference with no local fallback,
exactly the fault mode Batch 3 / the hermetic principles forbid. **Verdict: REJECT on physics +
architecture, not on complexity.** Sharding is a datacenter-interconnect technique wearing a mesh
costume; the mesh's links are 3–4 orders too slow.

### 2.b Ensemble / gossip raw inference results — COLLAPSES into DecisionUnit gossip

**What it means concretely.** Each node runs its *own* small model independently on the *same or
related* input and gossips the **result** (not weights). The task's sharp question: is this
meaningfully different from DecisionUnit gossip, or does it collapse once the Decision Compiler is
applied?

**It collapses — and the collapse is the point [GROUNDED].** Gossip does not make any single
inference faster; each node still pays its own full bandwidth-bound decode to produce a result. So
(b)'s only value is **sharing an answer so a peer need not recompute it** — which is a *distributed
answer cache*, not a *distributed inference engine*. And the repo already has the correct form of a
distributed answer cache: the latency blueprint's **signed decision records** (`§3.6:423-434`) and
its endpoint **S7 — gossip compiled DecisionUnits** (`§5 S7:509-515`), confirmed by Batch 6 §1 as
the same object with epoch-versioning + import-time replay. The moment you compile the recurring
question-shape's decision boundary into a DecisionUnit (yesterday's primary recommendation), the
"result" you gossip *is* the compiled unit, executed everywhere in ns — and raw distributed
inference becomes **unnecessary**, exactly as the task hypothesized.

**Where it does NOT collapse, it is dominated [PHYSICS].** For genuinely non-compilable outputs
(open-ended synthesis — the >1K-token class that is 80% of decode work, latency blueprint §2.6),
gossiping a 7B courier-phone token stream means propagating **lower-quality tokens produced 5–15×
slower** than one remote API call (`LOCAL-AI…:34`). A quality-weighted "ensemble consensus" over
several small models is both slower (each runs a full decode) and unable to exceed the best single
model — and any *quality voting* immediately hits the repo's `NO-COURIER-SCORING` / reputation
red-line (Batch 4 §1), since "which node's answer is better" is a quality score. So for the
non-compilable class, (b) is strictly dominated by "call the remote API once."

**Verdict: (b) is buildable on the existing transport (§3) but is not a distinct inference
architecture — it is a distributed answer cache that, for the compilable class, IS DecisionUnit
gossip, and for the non-compilable class is dominated by a single remote call.** No new build; it
folds into the S7 design already on the shelf.

### 2.c Speculative / hedged racing (N nodes race the SAME model on the SAME input) — category-OK, but no median win

**Does it conflict with the adopted "verify-before-persist, no speculative execution" rule?
NO — different category [GROUNDED].** Batch 4 §2.5 rejected speculative **STATE** execution:
optimistically mutating shared, persisted state before consensus and rolling back on mismatch —
rejected as degrade-**open** in a degrade-closed architecture, and undermined by the SSR-2020
batch-verify finding (`sign.rs:971`, `6541ae8`). Racing N **independent, read-only** inferences
with **no shared mutable state** is a categorically different thing: there is nothing to roll back,
no state to diverge, no persist step. First-to-finish gossips a signed answer over `SyncFrame`;
the losers' outputs are discarded (idempotent — content-addressed, so a duplicate is a no-op,
`sync_pull.rs:606`). This is Dean & Barroso tail-latency **hedging** ("The Tail at Scale," CACM
2013), which the latency blueprint already discusses for the remote-vs-remote case (`S2:479-485`).
**So (c) is permitted** — it is not the rejected pattern.

**But it does not win the median on this hardware [MEASURED + PHYSICS].** On one host, racing 2
copies **lost** (§1.3). Across *separate identical deterministic nodes* (fixed seed ⇒ identical
output at identical speed), the race is a **tie** — no node finishes first by design. Hedging only
pays when nodes have **variance** (heterogeneous load/hardware/models), and then it buys a **p99
tail** improvement (a busy node doesn't stall the answer) at the cost of **N× the compute** — a
fault-tolerance lever, not a speedup of the typical case. That is a real but narrow niche (offline
resilience: if the hub's model is busy, a courier's idle phone can answer), not a primary latency
strategy, and it is worth building only after there is a measured tail problem to hedge and a second
live node to hedge with (neither exists in a one-host session).

**Verdict: (c) does not conflict with the no-speculative-STATE rule (idempotent read-racing is a
different category), but it delivers no median-latency win on bandwidth-bound identical nodes
(measured LOSS on one host, tie across identical nodes); its only value is p99-tail/fault-tolerance
across heterogeneous nodes at N× compute — a deferred niche, not a build-now lever.**

---

## 3. Does the EXISTING gossip transport support any of these, without new machinery?

Concrete answer from reading the live transport (`bebop-repo/bebop2/proto-wire/src/discovery.rs`,
`sync_pull.rs`, this session [GROUNDED — file:line below]). The transport is **full-roster
anti-entropy over real QUIC**, carrying **opaque, content-addressed, Ed25519-signed `Vec<u8>`
payloads with idempotent dedup and Merkle-root convergence**. Key facts:

- `SyncFrame` carries an **opaque `payload: Vec<u8>`** (`sync_pull.rs:147`), content-addressed by
  `content_id = sha3_256(prev‖actor‖seq‖payload)` (`:300-307`), signed over a canonical domain
  (`:166-174`, real Ed25519 verify `:357`), deduped by content-id (a re-sent frame is a no-op,
  `:606-608`), and reconciled by comparing **Merkle roots** (`MerkleLog`, `:457-481`; N-node
  convergence is a proven graph fixed-point, `:1163-1180`).
- **Hard bound: `MAX_SYNC_PAYLOAD = 1 << 20` = 1 MiB** (`sync_pull.rs:159`), enforced
  before allocation on decode (`:259-261`).
- `GossipAgent` (`discovery.rs:148`) does the same for the peer roster — opaque payload, hybrid
  (classical+PQ) signed frames (`:259-286`), deterministic BTree ordering.

| Cargo | Fits existing transport? | Concrete reason |
|---|---|---|
| **(a) sharded model weights** | **NO** | Weights are **GB-scale**; the payload cap is **1 MiB** (`sync_pull.rs:159`). Weights are not event *facts* — no `prev/actor/seq`, nothing to fold, no convergence semantics. Re-gossiping GB every epoch would thrash anti-entropy. This needs entirely new machinery (chunked BitTorrent-style transfer, not anti-entropy). **Reject on the existing transport.** |
| **(b) inference results** | **YES, natively** | A signed inference-result record = an opaque ≤1 MiB content-addressed payload — exactly what `SyncFrame` is built to carry. Zero new machinery. (But per §2.b this is a distributed answer cache, and worth carrying only in its compiled DecisionUnit form.) |
| **(c) DecisionUnits** | **YES, natively — this is the fit the transport was made for** | A compiled DecisionUnit (its provenance-stamped Rust source or a small artifact) is a signed, content-addressed opaque payload; gossip → dedup → Merkle-converge is exactly right. **One constraint:** the unit must fit **1 MiB** (trivially true for every §2.3-class shape; a large *model* artifact would not). Import is gated by the **independent-replay** check (Batch 6 §1.3) — the transport delivers bytes; the receiving hub re-executes the harvested instance set before trusting the unit (`verify`-before-persist shape, `event_log.rs:389`). Epoch-versioning (Batch 6 §1.2a) rides the existing per-actor `seq`/Merkle identity. |

**Concrete answer to item 3: the transport supports (b) and (c) with zero new machinery and rejects
(a) on its 1 MiB payload bound + fact-vs-blob mismatch.** And since (b) collapses into (c), the
single supported, worthwhile cargo is **compiled DecisionUnits** — which is precisely the
already-designed S7 endpoint, now confirmed transport-ready.

---

## 4. Honest verdict — does distributing close the 5–15× local-vs-remote gap?

**No — not for the latency of one request, and the reason is structural, not tunable [PHYSICS].**

The 5–15× gap (local 4.8–10.5 tok/s vs remote 50–90 tok/s, `LOCAL-AI…:34`) is **memory bandwidth
per token**. Distributing across nodes does not touch it, and the three variants show why:

- **Network latency parallelizes; decode does not.** Independent network requests use independent
  links, so more nodes = more aggregate link capacity, linearly. A single token's decode is a
  **serial dependency chain** — layer N needs layer N-1's activations, and every layer must stream
  the full weight set from RAM. You cannot "add lanes" to a dependency chain: sharding it (variant
  a) inserts a network hop into the chain **per layer**, making one request *slower*; racing it
  (variant c) runs the whole chain N times in parallel but each copy is still bounded by one node's
  bandwidth, so the first answer is no earlier (measured LOSS, §1.3); ensembling it (variant b)
  still pays a full chain per node.
- **On one host, more cores don't help** — §1.2's flat 1.0–1.06× curve is the bandwidth wall; the
  4 physical cores already share the one memory bus a single stream nearly saturates.
- **Across nodes, you add bandwidth — but only for THROUGHPUT, never for one request's latency.**

**Where it genuinely wins (stated because the math says so, not softened):** **throughput across
MANY independent requests.** If the hub has 100 independent, schema-constrained classification jobs
(not one big generation), spraying them across 10 courier phones — each running its own small model,
embarrassingly parallel, gossiping signed results back over `SyncFrame` — yields ~10× aggregate
throughput, because you have summed 10 devices' memory bandwidth. This is real, it fits variant (b)
+ the existing transport, and it is the **only** honest distributed-inference win. Its caveats are
equally honest: it is the **least latency-sensitive** workload (batch, not interactive); it is
**dominated on quality-per-token** by a single remote batch API call (50% cost, ~1 h, latency
blueprint §3.7.4) **unless the network is absent** — i.e. its true justification is
**offline/sovereign operation**, not speed. So even the win is a sovereignty play, not a
performance play.

**Bottom line:** distributing the **inference** cannot close the gap — the bottleneck is per-device
compute-per-token, which does not parallelize like network latency. Distributing the **compiled
output** (DecisionUnit gossip) closes it completely for its class, by making execution cost **zero
tokens** — the bandwidth wall simply does not apply to a native `match` that runs in nanoseconds.
The operator's own DecisionUnit-gossip idea is not "the closest existing idea" to this thread; it is
the **resolution** of it.

---

## 5. Recommendation — what (if anything) to build first

1. **Build none of (a/b/c) as a distributed-inference layer.** (a) is rejected on physics +
   architecture; (b) collapses into (c)/DecisionUnit gossip; (c) has no median win on this
   hardware. Building a "mesh inference engine" now would be capability theater against a measured
   bandwidth wall.
2. **The ready, physics-respecting win is DecisionUnit gossip over the existing `SyncFrame`
   transport** — already designed (latency blueprint S7, Batch 6 §1), already transport-supported
   (§3), gated by import-time independent replay (Batch 6 §1.3), epoch-versioned on the existing
   Merkle identity (Batch 6 §1.2a). **Nothing new in the transport is required.** What is missing is
   upstream of the mesh and already scoped elsewhere: the DecisionUnit registry + compile-with-
   verification protocol + the import-replay gate (latency blueprint §2.4–§2.7). Sequencing:
   single-host Decision Compiler pilot (shape C1) **first**; gossip of units is a mesh-arc consumer
   that lights up only when a **second live hub** exists (today's one-host session makes it
   premature — the same honesty as `§3.6:432-434`).
3. **Defer raw distributed inference (variant b, offline throughput map-reduce) behind a
   measurement trigger** [DEFER-WITH-TRIGGER]: adopt only if (i) a real workload of ≥N independent,
   schema-constrained, latency-insensitive local jobs appears, **and** (ii) the network is genuinely
   absent (sovereign/offline requirement), **and** (iii) a second live node exists to distribute to.
   Until all three hold, a single remote batch call dominates. When they hold, it rides variant (b)
   over the existing transport with **no** new machinery — so there is nothing to pre-build.
4. **Variant (c) tail-hedging: not now.** Revisit only with a measured p99 tail problem and ≥2
   heterogeneous live nodes; it is a fault-tolerance lever (N× compute for a tail-only win), never a
   median-latency lever, and it must stay clear of any answer-quality *scoring* (NO-COURIER-SCORING
   red-line).

---

## 6. Anu / Ananke check

- **Anu (logic, not authority):** the central claim — distributing inference cannot beat the
  bandwidth wall for single-request latency — is arithmetic on **measured** numbers (§1.2 flat
  throughput; §1.3 hedge LOSS) plus the weight-size/bandwidth identity (4.9 GB × 10 tok/s ≈ 49 GB/s,
  at the DDR4 ceiling) and the layer-count × hop-latency derivation for sharding (32 hops × 5–50 ms
  ≫ 0.1 s/token). Every transport claim is a live `file:line` read (`sync_pull.rs:147,159,300,357,
  606`; `discovery.rs:148,259`), not memory. The one number I could not obtain — genuine
  `NUM_PARALLEL≥2` overlap — is named as unmeasured (§1.4, classifier-blocked) with the reason it
  cannot change the verdict (4-core shared-bus ceiling), not hand-waved.
- **Ananke (structural necessity):** the recommendation **adds no machinery** — it routes every
  worthwhile case to an existing authority (DecisionUnit gossip = S7/Batch 6; the transport already
  carries it; the import gate is the same verify-before-persist / key_V pole the hermetic audit
  names). It rejects (a) by a structural constraint (1 MiB payload cap + no-network-for-decisions),
  not by taste; and it keeps (b)/(c) from drifting into the reputation-scoring and speculative-state
  forms the repo's CI gates and theorems already forbid. The good outcome is forced by reusing
  proven poles, not by inventing a mesh-inference subsystem.

## 7. Doubt audit (2 questions)

**Q1 — least confident (ranked):**
1. **Genuine `NUM_PARALLEL≥2` throughput is unmeasured** (§1.4) — both measurement routes were
   classifier-blocked. Bounded above by the 4-core shared-bus ceiling; would refine the exact flat
   curve, cannot overturn "concurrency ≠ single-request speedup."
2. **The 32-hops-per-token sharding figure uses a nominal layer count and a WiFi/cellular RTT band**
   — the *direction* (3–4 orders too slow) is robust to any plausible layer count or mesh-link
   latency; the exact multiplier is illustrative, not a benchmark. No llama.cpp-RPC-over-cellular
   number was obtainable this session.
3. **The "10× throughput across 10 phones" figure (§4) is arithmetic on summed bandwidth**, not a
   measured cluster — real-world gossip overhead, heterogeneous phone bandwidth, and result-collation
   would shave it; it is an upper bound to justify the *niche*, not a forecast.
4. **Petals/exo/distributed-llama numbers are secondary/low-trust** — cited for the *shape* of the
   technique (many-node swarms for too-big models), and no decision rests on their specific figures.

**Q2 — the biggest thing possibly missed:** the honest blind spot is that **this whole thread's
premise (make local inference fast by distributing it) is the wrong optimization target**, and I
may be over-serving it by analyzing three variants when the measured answer is "the token is the
enemy, not the node count." The latency blueprint already established that *eliminating* tokens
(compiled decisions) beats *relocating* them; this document's real contribution is proving the mesh
transport can't rescue distributed *inference* from that verdict, while it *can* carry distributed
*compiled decisions*. A second, named miss: I treated the courier-phone mesh as the target hardware
per the repo's stated scale — if the operator's real mesh is a cluster of *owner hubs* on a LAN
(µs-scale interconnect, not cellular), variant (a) sharding moves from "rejected by 3–4 orders" to
"worth a measured trial for models too big for one hub" — a different regime that this analysis
rejects only under the phones-on-cellular assumption, stated so the operator can correct the target
if it's wrong (Descartes-square: the verdict flips only if the interconnect assumption flips).

---

*Written 2026-07-17 on `feat/harness-llm-backend`. On-host probes this session:
`scratchpad/mesh_infer_probe.py` (1/2/4-concurrent decode, hedge first-done, TRUE
tokens-÷-wall throughput), `ollama list`/`ps`/`/api/version`/`/api/generate` (warm 8B). Live code
read: `bebop-repo/bebop2/proto-wire/src/{discovery.rs,sync_pull.rs}` (gossip transport, payload
cap, content-address, Merkle convergence). Companion authorities: LOCAL-AI-LOCAL-AGENTS-RESEARCH
(measured decode band, the two built planes), BLUEPRINT-LATENCY-ELIMINATION (the decode-dominates
inversion + Decision Compiler + S7 gossip endpoint), 15-BATCH6 (DecisionUnit gossip = Decision
Compiler; epoch/PoQ-import-gate/Merkle-rollback new ground; import-time replay survives), Batch 4
§2.5 (speculative-STATE rejection, distinct from idempotent read-racing), BLUEPRINT-WAVE-SCHEDULING
(no-network-for-decisions), hermetic principles (degrade-closed, RC-2 key_V). Two measurement
routes to genuine `NUM_PARALLEL≥2` were classifier-blocked and are flagged unmeasured (§1.4). No
product code written or edited.*
