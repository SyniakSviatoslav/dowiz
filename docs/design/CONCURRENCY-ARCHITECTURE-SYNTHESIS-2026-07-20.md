# CONCURRENCY ARCHITECTURE SYNTHESIS — reconciling D13 with the native-concurrency and end-to-end research (2026-07-20)

**Status: RESEARCH SYNTHESIS / RECOMMENDATION — not yet implemented. Not yet re-confirmed by the operator.** Several recommendations below are deliberately **narrower than a literal "adopt tokio broadly" reading of DECISIONS.md D13**; every such narrowing is flagged inline and collected in §10 as explicit confirmation requests. Nothing here overrides D13 — D13 itself states it is "a directional ruling, not a completed design" and that "implementation requires the scoping pass above first" (`DECISIONS.md`, D13). This document **is** that scoping pass, submitted for confirmation.

Intended landing path if accepted: `docs/design/CONCURRENCY-ARCHITECTURE-SYNTHESIS-2026-07-20.md` (this file).

---

## 0. What this resolves

Three operator statements must be made mutually consistent and turned into buildable per-surface decisions:

1. **D-V1 confirmed** — the voice pipeline's synchronous, thread-based, `cpal`-driven audio path is valuable (`BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md`, §6.1/D-V1).
2. **D13** — reverse the 2026-07-15 no-tokio mandate; "adopt tokio/async broadly across the agent lane," acknowledged as breaking the compile-firewall's current sync guarantees and needing a dedicated redesign pass.
3. **The follow-up directive** — before any redesign, research "native concurrency & exokernel resistent architecture … and interesting old experiments with e2e principles."

Two Opus research passes ran (native-concurrency/exokernel; end-to-end principle). This document reconciles them into one recommendation per concurrency surface, with falsification criteria for each.

---

## 1. Verified current state (live tree, 2026-07-20)

Every claim below was re-checked against the working tree today, not taken from the research passes on trust:

| Surface | Current shape | Evidence |
|---|---|---|
| LLM transport | Synchronous `ureq`, `"stream": false` hardcoded | `llm-adapters/src/transport.rs` lines 1–4 (module doc: "Synchronous (ureq, no tokio)"), line 50 |
| LLM dispatch | Std-only counting semaphore + fixed worker pool over an `mpsc` channel; "Bounds concurrency on LLM calls WITHOUT tokio"; N workers ≤ backend parallelism cap (e.g. 2 for Ollama) | `llm-adapters/src/dispatch.rs` lines 1–8 |
| Agent executor | Bounded synchronous plan→act→observe loop; timeout via "watchdog thread + `recv_timeout` (std-only)" | `agent-loop/src/lib.rs` lines 24–27 |
| Tool boundary | `trait ToolPort { fn spec(...); fn invoke(&self, granted: ToolScope, inv: &ToolInvocation) -> Result<ToolOutput, ToolError>; }` — blocking, defined **inside the kernel** | `kernel/src/ports/tool.rs` lines 129–137; implemented in `agent-facade/src/lib.rs` at lines 96 (`ReadOrderStatusTool`, generic over `OrderStatusSource`) and 200 (`WebFetchTool`) |
| Event log | "Local-first: `append`/`commit_after_decide` runs **before** any network IO. Once persistently committed, the event can be gossiped/synced (MESH-07) and the network layer never re-runs `decide` — it only verifies signatures." | `kernel/src/event_log.rs` lines 296–299 |
| Mesh networking | Not yet built — `mesh-adapter/`'s `Cargo.toml` declares "no HTTP, no transport, no storage"; dependencies are `dowiz-kernel` + `bebop-delivery-domain` + `bebop-proto-cap` only — no tokio, no ureq, no async runtime | `mesh-adapter/Cargo.toml` |
| Voice audio | Blueprint-only; D-V1 = threads + channels, no async, unmodified agent-loop/ToolPort path | `BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md` §6.1 |

Two structural facts matter for everything below:

- **The `ToolPort` trait lives in the kernel** (`kernel/src/ports/tool.rs`), and the default kernel build is pure-`std`, serde-free, WASM-lean (CLAUDE.md, feature discipline). Any `async fn invoke` puts async machinery into the kernel's canonical surface — that is a kernel-purity question, not merely an agent-lane question.
- **The compile firewall is an import topology, not a sync-ness property**: `agent-loop` imports only `agent-facade`, which does not re-export mutation symbols, so `agent-loop` structurally *cannot name* kernel mutation. Async per se does not break the import graph. What D13 correctly flags as "sync guarantees" is a second, subtler property: today the capability scope check in `ToolPort` fails closed **on the same call stack, before the tool body runs**, with no queueing, retry, or reordering layer between check and execution. That same-stack property is what an async rewrite actually endangers, and it is exactly the end-to-end research's framed question (§2, Pass 2, question 1).

---

## 2. What the research established (compressed, load-bearing citations only)

**Pass 1 (native concurrency / exokernel):**

- The genuine walls that motivate async runtimes — memory footprint and scheduler overhead at 10⁴–10⁵ concurrent connections (the C10K/C10M problem, Dan Kegel) — are scales dowiz's single-hub process does not approach. Tokio's work-stealing scheduler is engineered for tens-of-thousands-to-millions of concurrent I/O tasks per process.
- Kobzol's reframe ("Async Rust is about concurrency, not (just) performance," Jan 2025): the honest test for async is *do you need to interleave thousands of concurrent operations per thread*, not raw speed. Function coloring (Nystrom, "What Color is Your Function?", 2015) is the price: `async fn` infects every caller.
- **Rust's `std::thread::scope` (stabilized 1.63, Aug 2022) already provides structured concurrency** — scoped child tasks with guaranteed join at scope exit (the nursery pattern from N.J. Smith's "Notes on structured concurrency, or: Go statement considered harmful," 2018) — with zero async, zero coloring, zero new dependencies.
- **io_uring carries a documented, serious security cost**: ~60% of Google's kernel-exploit VRP submissions targeted it; ChromeOS disabled it; Android gates it; it is off on production Google servers (Google Security Blog, via Pass 1). Thread-per-core frameworks are either effectively unmaintained (glommio — per Apache Iggy's Feb 2026 migration writeup) or tuned for hyperscale (monoio/ByteDance, millions of connections).
- The actor pattern (Armstrong 2003) matches `ToolPort`'s grain conceptually, but mature Rust actor crates (actix, ractor) are tokio-backed; the pattern (supervision, bounded mailboxes, backpressure) is separable from the runtime and keepable over plain threads — which `dispatch.rs` already does.
- Exokernel literature (Engler/Kaashoek/O'Toole SOSP 1995; "Exterminate All OS Abstractions," HotOS 1995; Xok/ExOS SOSP 1997): kernel should securely multiplex, not abstract. It died of complexity, a hard security model, and ecosystem gravity. Its modern viable descendant is **Firecracker-style minimize-the-privileged-surface** (Agache et al., NSDI 2020) — the isolation half of the exokernel bargain without the raw-access half.
- Real-time audio: async inside audio callbacks is discouraged industry-wide (no allocation, no blocking, no `.await` in the callback); `cpal`'s dedicated-thread + lock-free-ring model is mainstream-correct regardless of what the rest of the system does.

**Pass 2 (end-to-end principle):**

- Saltzer, Reed & Clark 1984: a function (reliability, ordering, dedup, integrity) can only be *completely* correctly implemented with endpoint knowledge; a lower layer's version is at best a performance optimization, never a substitute. Clark 1988 adds fate-sharing: state lives at endpoints so only the endpoint's own failure destroys it.
- **Stone & Partridge, SIGCOMM 2000 — the measured confirmation**: 1-in-1,100 to 1-in-32,000 packets failed TCP checksum on links whose CRCs should have caught nearly everything, because corruption entered *outside the link's span* (router memory, host software). Integrity-critical applications need their own end-to-end check. This is measurement, not theory.
- Lower-layer "help" that endpoints had to undo, all documented: link-layer ARQ vs. TCP retransmission (Balakrishnan et al., SIGCOMM 1996), bufferbloat breaking TCP's end-to-end congestion signal (Gettys & Nichols ~2011), Nagle vs. delayed ACKs forcing `TCP_NODELAY` (two independent transport optimizations composing pathologically).
- Middleboxes (Blumenthal & Clark 2001; RFC 3234) and the NAT-traversal lineage (STUN/TURN/ICE — RFC 8445 et al.) are directly dowiz's mesh reality: any P2P mesh must re-assert, at the endpoints, connectivity the network no longer provides.
- **dowiz already embodies the principle correctly in its most load-bearing spot**: `event_log.rs` commits locally before any network IO; the network layer "never re-runs `decide` — it only verifies signatures" (lines 296–299). Content-address-as-idempotency-key is the same move as Saltzer's checksum and IPFS's content-id (Benet 2014), and the local-first posture has the Kleppmann et al. 2019 pedigree.
- Framed test for this synthesis: *"if the endpoint must do this for correctness anyway, is the lower layer doing it a speed-up or a redundant liability?"*

---

## 3. The honest tension: D13's direction vs. the evidence

Stating this plainly, because rubber-stamping either side would be dishonest:

**What D13 says:** adopt tokio/async broadly across the agent lane.

**What the evidence supports:** dowiz's current shape — one hub process, bounded calls to a *local* LLM server capped at ~2 parallel requests by the backend itself (`dispatch.rs`), a sequential agent loop, and a hardware-synchronous audio path — sits far below every threshold at which tokio's design point pays for its coloring cost, dependency tree, and the loss of the same-stack enforcement property. The one dowiz surface where many-concurrent-connections *might* genuinely materialize (`mesh-adapter`) does not exist yet, so no measurement exists either way.

**What the operator may actually have been reaching for.** "Tokio is a must" arose in the context of *streaming a shared LLM-transport adapter*. Candidate underlying goals, and whether the evidence-fit architecture satisfies each:

| Candidate goal behind "tokio is a must" | Satisfied without tokio? | How / why not |
|---|---|---|
| **Streaming LLM responses** (the proximate trigger) | **Yes** | SSE streaming is a property of the HTTP request body, not the runtime. `ureq` returns a blocking `Read`er; a worker-pool thread reading SSE lines and forwarding chunks over a bounded `mpsc` channel delivers token-by-token streaming with the existing dependency set. §5.1. |
| **Timeout/cancellation/`select!` ergonomics** | **Mostly** | `std::thread::scope` + channels + `recv_timeout` covers the patterns dowiz actually uses (the watchdog already is one). Multi-way `select!` over many heterogeneous sources is genuinely uglier over threads — real, but not currently needed anywhere in the tree. |
| **Mesh scale** (many peer connections) | **Unknown — the honest answer** | Below ~10³ concurrent sockets, threads win on simplicity; above ~10⁴, they lose (Pass 1, C10K). Dowiz's launch market (Albania/EU, D12) makes >10³ concurrent peers per hub speculative today. §5.4 makes this a measured gate, not a guess. |
| **Ecosystem access** (QUIC via quinn, WebRTC, most maintained protocol crates are tokio-first) | **No — this is the one tokio genuinely buys** | If the mesh layer wants a mature QUIC or NAT-traversal crate, it will almost certainly be tokio-backed. The confined-async pattern (§6) admits tokio *there* without coloring the agent lane. |
| **Uniformity** ("one runtime everywhere") | **Deliberately not** | Both research passes converge on the opposite: the audio path *must not* be async, the kernel *should not* be async, so full uniformity is unreachable regardless. The system will have a sync/async boundary somewhere; the only question is where it sits and who enforces it. |

**The reconciliation this document proposes** — which is the same one Pass 1's final line identified as available but "a design decision, not a default": tokio enters the tree **at the surface where its design point is real (mesh networking, when built and when measured to need it), confined behind synchronous trait boundaries**, while the agent lane's enforcement spine (`ToolPort`, `agent-loop`, kernel ports) stays synchronous, and streaming — the proximate need — ships now without any runtime change. This honors D13's direction (the mandate reversal stands; tokio is no longer banned and has a designated entry path) while declining the *broadest* reading (async trait signatures through the agent lane), because the evidence is against that reading and D13 explicitly deferred the per-surface scoping to this pass.

**[OPERATOR CONFIRMATION REQUIRED — this is narrower than literal D13. See §10.]**

---

## 4. Standing design constraints adopted here

### 4.1 The end-to-end constraint, stated falsifiably

Adopted as a hard rule for every change proposed below, in the exact form Pass 2 framed:

> **E2E-RULE.** For any function F (retry, dedup, ordering, integrity, capability gating): if an endpoint must perform F for correctness anyway, a lower layer may perform F only as a measured performance optimization, and never in a way the endpoint must detect and undo. Any proposed layer that re-implements F below the endpoint must cite the measurement showing the optimization pays.

Concrete bindings for dowiz:

- **Capability/red-line enforcement** stays entirely at the `ToolPort` scope check — fail-closed, before the tool body, on the same call stack. No transport, queue, or runtime between the check and the body. (Pass 2, question 1.)
- **Idempotency and duplicate-action decisions** at the LLM boundary stay in `agent-loop` (the endpoint). Transport-level retry, if ever added, is a performance optimization only — exactly as `event_log.rs` already refuses to let the network re-run `decide`. (Pass 2, question 2; Stone & Partridge is the measured evidence that middle-layer integrity is never complete.)
- **Buffering is bounded and minimal** on any streaming path. The bufferbloat case (Gettys & Nichols) is the documented failure mode: a generous middle buffer destroys the endpoint's timing signal. For LLM streaming, time-to-first-token *is* the timing signal; an unbounded or large channel between the SSE reader and the consumer would be self-inflicted bufferbloat.
- **No composed hidden optimizations.** The Nagle/delayed-ACK case shows two individually reasonable lower-layer optimizations composing pathologically. Rule: any transport-level batching/coalescing/retry must be individually switch-off-able by the endpoint, and off by default.

### 4.2 The exokernel question, resolved

Pass 1 gave two readings of "exokernel-resistant architecture" and noted they point in opposite directions on io_uring/kernel-bypass:

- **Reading 1 — resistant to *needing* exokernel-style bypass**: keep the kernel and its I/O paths so thin and abstraction-light that nothing ever needs to route around them. Dowiz's std-only, serde-free, minimal-abstraction kernel already fits this.
- **Reading 2 — resistant to the exokernel's own *failure modes***: the exokernel lineage's historical weakness was its security/isolation model; its viable modern descendant is Firecracker's minimize-the-privileged-surface, not raw hardware access.

**This document adopts Reading 1 as the primary lens and Reading 2's isolation evidence as a binding constraint.** The two agree on every actionable conclusion for dowiz:

- **No io_uring, no DPDK/SPDK, no thread-per-core framework anywhere in the tree.** The only payoff of kernel-bypass machinery is latency/throughput at scales dowiz does not have, and its documented cost is attack surface (~60% of Google's kernel-exploit VRP submissions per Pass 1) — for a post-quantum, sovereign, security-first project this is a strictly bad trade. The D0 invariant **reliability-over-latency** is direct, pre-existing evidence that dowiz already weights this axis the same way: when reliability and latency conflict, latency loses. Kernel-bypass is a latency purchase paid in reliability/isolation currency.
- **Isolation-first minimalism is the exokernel lesson dowiz keeps**: the existing microVM/WASM Docker-swap arc is already in the Firecracker family (Pass 1, §B), and the agent lane's compile firewall is the same idea applied at the type level — minimize what the untrusted layer can even *name*.
- Consequence for concurrency: std sockets + threads (or, if measured necessary at the mesh edge, tokio's *portable* epoll-based reactor — not its io_uring backends) are the sanctioned I/O substrate.

---

## 5. Per-surface recommendations

### 5.1 `llm-adapters/src/transport.rs` — LLM transport

**Recommendation: stays synchronous `ureq`; gains real SSE streaming now, with no runtime change.**

- Flip the hardcoded `"stream": false` (line 50) to support `"stream": true`. `ureq`'s response body is a blocking `Read`er; a dispatch worker thread (the pool already exists — `dispatch.rs`) reads SSE `data:` lines as they arrive and forwards parsed chunks over a **bounded** `std::sync::mpsc` channel (small capacity, single-digit chunks — see the bufferbloat binding in §4.1) to the consumer. The consumer sees an iterator of tokens; time-to-first-token drops from full-generation latency to first-chunk latency.
- This directly satisfies the proximate need that triggered the D13 conversation (streaming the shared LLM-transport adapter), and it is the strongest available test of whether "tokio is a must" was about streaming: if this lands and the streaming experience is correct, that motivation is discharged without a runtime.
- **End-to-end placement:** retry, timeout policy, and duplicate-request decisions stay in `agent-loop`/dispatch (the endpoint), exactly where they are now. The transport maps failures to typed `LlmError` and does not retry on its own (its current contract — module doc: "never a mock"). No transport-level retry is added.
- **Concurrency ceiling honesty:** the worker pool caps concurrent streams at N workers (N ≤ backend parallelism, e.g. 2 for Ollama). That is not a limitation today — the *backend itself* is the cap. If dowiz ever fronts a managed API with hundreds of concurrent streams from one process, this design is falsified (§9) and §6 applies.

**Falsifiable claim:** blocking SSE over `ureq` on the existing pool achieves time-to-first-token within measurement noise of an async client against the same local backend, at the backend's own parallelism cap. If a benchmark shows otherwise, this recommendation is wrong.

### 5.2 `agent-loop` executor

**Recommendation: stays synchronous. No tokio, no async trait signatures.**

- The plan→act→observe loop is *sequential by design* — one bounded step at a time, each step's output feeding the next. There is no interleaving of thousands of concurrent operations here (Kobzol's test, Pass 1); an async executor would add coloring and cancellation-point semantics to a loop that deliberately has neither.
- The watchdog-thread + `recv_timeout` pattern already provides the timeout/abort function an async runtime would offer, in ~std-only form, and it has the property async cancellation famously lacks: the step either completes or the watchdog fires — there is no third state where a future is silently dropped mid-side-effect.
- If the loop ever needs bounded *fan-out* (e.g., issuing several independent read-only tool calls in one observe step), use **`std::thread::scope`** — structured concurrency with guaranteed join at scope exit, zero new dependencies, zero coloring (Pass 1, A4). This is the middle path the operator may not have had in view: the ergonomic core of structured concurrency without the runtime.
- **Firewall status:** unchanged in both senses — the import topology (`agent-loop` → `agent-facade` only) is untouched, and the same-stack property (§1) is preserved because `invoke` stays a plain call.
- **End-to-end placement:** the loop remains the sole owner of step idempotency and of "was this action already taken" decisions (Pass 2, question 2). Nothing below it queues, reorders, or retries agent actions.

### 5.3 `ToolPort` (`kernel/src/ports/tool.rs` + `agent-facade` implementations)

**Recommendation: the trait stays exactly as it is — synchronous `fn invoke`, fail-closed scope check before the tool body, same call stack. This is the strongest single recommendation in this document.**

Three independent reasons, any one of which suffices:

1. **Kernel purity.** The trait is defined *inside the kernel*, whose default build is pure-`std`, serde-free, and WASM-lean by standing repo law (CLAUDE.md feature discipline). `async fn` in a kernel port trait either drags runtime machinery into the default kernel graph or forces a feature-gated dual-trait scheme — both worse than the status quo for the canonical surface everything replays on.
2. **End-to-end enforcement (Pass 2, question 1).** Today, mutation is type-unrepresentable (`ToolAction` has a single `Read` variant) and the scope check fails closed *before* the tool body, with no layer in between. Making `invoke` async inserts an executor between check and body: the runtime now owns queueing, scheduling, and cancellation of a capability-checked operation. Per the E2E-RULE, either the endpoint must re-verify after the queue (redundant liability — the Stone & Partridge pattern: corruption/state-change enters in the span between check and use) or the runtime is trusted with a correctness function it cannot completely implement. Keeping `invoke` synchronous keeps check-and-use atomic on one stack.
3. **Coloring containment.** `ToolPort` is the narrow waist of the agent lane. If it turns async, every implementor (`agent-facade`'s `ReadOrderStatusTool`/`WebFetchTool`), every caller (`agent-loop`), and transitively the kernel test surface all turn async — the maximal-blast-radius version of Nystrom's coloring problem, purchased for zero interleaving benefit (tool invocations in a sequential loop).
- **Long-running or I/O-heavy tools** implement concurrency *internally*: the tool body may spawn scoped threads, stream, or block on a channel — the trait boundary remains a synchronous contract, exactly as the audio boundary keeps `cpal`'s callback contract regardless of what sits above it.

### 5.4 `mesh-adapter` — future networking layer

**Recommendation: build the protocol core sans-I/O; choose the I/O substrate by measurement at build time; this is tokio's designated entry path if the threshold is crossed.**

This is the one surface where tokio's actual strength — many concurrent connections — could genuinely apply, and also the surface where nothing exists yet, so the decision can be made correctly instead of retrofitted:

- **Sans-I/O protocol core.** The bebop2 session/sync/gossip logic is written as a pure state machine: bytes/events in, bytes/actions out, no sockets, no runtime, no clocks (it already must be deterministic to live near the kernel). This is the same seam discipline the repo already uses (OpenBebop crypto injected at a seam). Consequences: the protocol is testable without a network, property-testable deterministically, and *runtime-agnostic* — threads today, tokio tomorrow, without touching protocol correctness.
- **I/O substrate, gated on a number:** start with blocking std sockets + a bounded thread pool (identical shape to `dispatch.rs`). **Gate: if a hub node must sustain more than ~1,000–2,000 concurrent peer sockets, the thread-based substrate is falsified** (Pass 1's walls begin at 10⁴–10⁵; the gate is set an order of magnitude conservative because reliability-over-latency prefers early, planned migration over emergency rewrites). Crossing the gate authorizes tokio *inside `mesh-adapter` only*, behind the sans-I/O seam, per §6. This is where D13's direction lands with evidence rather than by default.
- **Ecosystem honesty:** if the mesh design settles on QUIC or standardized NAT traversal, the mature crates are tokio-backed (Pass 1, A5 pattern generalizes). That alone may pull tokio into `mesh-adapter` *earlier* than the connection-count gate — acceptable under §6's confinement rules, and cheaper than reimplementing QUIC.
- **End-to-end placement (already law):** the mesh layer never re-runs `decide`; it verifies signatures and delivers (`event_log.rs` 296–299). The sans-I/O split makes this structurally hard to violate: the I/O substrate literally has no access to `decide`. NAT traversal (STUN/TURN/ICE lineage, Pass 2) is *endpoint* logic re-asserting connectivity through middleboxes — it belongs in the protocol core, not the I/O substrate, and no middlebox-side function may be assumed. Fate-sharing holds: sync state lives in each node's local log; a relay's death loses nothing.
- Per §4.2: whatever substrate is chosen, **no io_uring backend, no kernel-bypass**. Portable epoll-class I/O only.

### 5.5 Voice pipeline audio thread — settled, not relitigated

**D-V1 stands.** `cpal` callback on a dedicated OS thread; no allocation, no blocking, no `.await` in the callback; lock-free ring / bounded channel handing samples upward. Both research passes independently corroborate this (Pass 1: async is discouraged inside real-time audio callbacks industry-wide; Pass 2: "the audio boundary is already correctly end-to-end-shaped … hard real-time correctness stays at the hardware endpoint"), and D13's own text already carves it out as a hardware constraint independent of the ruling. The only design freedom is where the boundary above the audio thread sits — under this document, everything above it is *also* synchronous (agent-loop, ToolPort), so there is no sync/async line to place in the voice path at all. That is a simplification this architecture gets for free.

### 5.6 Correctness-placement table (the E2E-RULE, applied)

| Surface | Correctness function | Lives at (after this recommendation) | Lower layers may add | Forbidden below the endpoint |
|---|---|---|---|---|
| LLM transport | Retry, idempotency, duplicate-action decisions | `agent-loop` / dispatch | Connection pooling, streaming delivery | Silent retry; unbounded chunk buffering; response reordering |
| Agent executor | Step ordering, timeout, "already done" | `agent-loop` (watchdog + loop) | — | Runtime-level task retry/rescheduling of agent steps |
| Tool boundary | Capability/red-line gating, mutation unrepresentability | `ToolPort` scope check, same-stack, fail-closed | — | Any queue/executor between scope check and tool body |
| Mesh | `decide`/`fold`, event identity, dedup, conflict resolution | Kernel + local event log (hash-addressed, structural no-op dedup) | Delivery, signature verification, congestion handling | Re-running `decide`; middle-layer dedup the log must redo; relay-held authoritative state |
| Audio | Real-time deadline correctness | `cpal` callback thread | Everything above the ring buffer | Allocation/blocking/async inside the callback |

---

## 6. The confined-async pattern (pre-approved escalation shape)

If any falsification trigger in §9 fires — or if the operator, after reviewing §3, confirms that the underlying goal *is* ecosystem access or literal broad adoption — the sanctioned shape is:

1. **Tokio as an internal implementation detail of edge crates only** (`llm-adapters` and/or `mesh-adapter`). The crate owns its runtime (a small, explicitly-configured runtime — not ambient), and **no async type crosses its public API**: the boundary functions block (`block_on`/channel hand-off) and present the same synchronous contracts they do today.
2. **`ToolPort`, `agent-loop`, and every `kernel/src/ports/*` signature stay synchronous** regardless. The firewall's import topology and the same-stack enforcement property are non-negotiable under this pattern.
3. **Feature discipline applies as usual**: tokio arrives behind an off-by-default Cargo feature with the standard header comment and a `cargo tree -e no-dev` verification that the default kernel graph stays clean, plus a DECART rationale for the dependency.
4. **Scheduler backend pinned to portable epoll-class I/O** — no io_uring feature flags — per §4.2.

This pattern captures tokio's genuine wins (ecosystem crates, many-connection scaling at the mesh edge, composable `select!` *inside* the edge crate) while paying the coloring cost only inside crates that already are adapters, never in the enforcement spine. It is Pass 1's closing reconciliation ("a thin async boundary confined to the HTTP+mesh edge, threads for audio, firewall preserved") made concrete.

---

## 7. Phased path

**Phase 0 — Operator confirmation (blocks everything else).** Present §10's questions. No code changes; D13 itself says implementation requires this scoping pass to be accepted first.

**Phase 1 — Streaming at the LLM edge, no runtime change.** Implement §5.1: `"stream": true` support, blocking SSE reader on the existing worker pool, bounded chunk channel. RED→GREEN: a test proving time-to-first-token on a long generation is a small fraction of full-generation latency (the RED state is the current design's full-response wait), plus a benchmark number per the "verified, not claimed" culture. This discharges the proximate motivation for D13 and produces the first hard evidence for or against "streaming needed tokio."

**Phase 2 — Structured-concurrency hygiene (cheap, independent).** Where ad-hoc `thread::spawn` exists in the agent lane, migrate to `std::thread::scope` for guaranteed-join semantics; document the pattern as the house style for fan-out. Zero dependencies; strictly improves leak/orphan behavior.

**Phase 3 — Mesh-adapter, sans-I/O first.** Build the bebop2 protocol core as a pure state machine behind the seam (§5.4); start with the thread-pool substrate; instrument concurrent-socket counts from day one so the §5.4 gate is a measurement, not an argument. Tokio (or a tokio-backed QUIC stack) enters here under §6's confinement rules if the gate trips or the protocol choice demands it.

**Phase 4 — Conditional, evidence-gated.** Only if §9 triggers fire: apply §6 to the affected edge crate. Never proceeds by default.

Each phase is independently landable and independently revertible; no phase rewrites a working surface ahead of evidence.

---

## 8. What does NOT change

- **The audio thread** — D-V1 stands as confirmed; `cpal` dedicated thread, sync, forever a hardware constraint (§5.5).
- **The compile firewall's mutation-invisibility guarantee** — `agent-loop` imports only `agent-facade`; `agent-facade` re-exports no mutation symbols; `ToolAction` stays mutation-unrepresentable. No phase touches this.
- **`ToolPort`'s synchronous signature and same-stack fail-closed scope check** (§5.3).
- **The kernel's pure-`std`, serde-free default build** and the feature-discipline rule for any new dependency.
- **`event_log.rs`'s local-first order**: commit-before-network, network-verifies-never-recomputes. This is the repo's existing, correct implementation of the 1984 argument and is the template every new surface above copies.
- **The D13 mandate reversal itself** — tokio is no longer banned; this document assigns it an evidence-gated entry path (§5.4, §6) rather than re-imposing the old prohibition.
- **No io_uring, no kernel-bypass, no thread-per-core frameworks**, per §4.2 — under either exokernel reading.

---

## 9. Falsification criteria — how to prove this document wrong

Per surface, the observable that would overturn the recommendation:

1. **Transport (§5.1):** a benchmark showing blocking-SSE-over-`ureq` materially loses time-to-first-token or chunk cadence vs. an async client against the same local backend at the backend's parallelism cap; **or** a product requirement for more concurrent LLM streams from one process than a worker pool of reasonable size (say, >32 simultaneous streams) can carry. Either → §6 applied to `llm-adapters`.
2. **Agent executor (§5.2):** a confirmed requirement to run on the order of hundreds of *interleaved* agent sessions inside one process (not one-per-thread — genuinely interleaved). Below that, thread-per-session is cheaper than the coloring cost.
3. **ToolPort (§5.3):** a demonstrated tool-invocation pattern that requires runtime-level cancellation semantics threads cannot express (cooperative cancellation flags + watchdog cover every currently known case). This bar is deliberately high because three independent arguments support the status quo.
4. **Mesh (§5.4):** measured sustained concurrent peer sockets on one hub exceeding ~1,000–2,000, or a protocol decision (QUIC, standardized ICE) whose only mature implementations are tokio-backed. Either → tokio inside `mesh-adapter` under §6.
5. **Audio (§5.5):** no falsifier accepted — hardware constraint, settled by D-V1 and both passes.
6. **The E2E-RULE (§4.1):** if a lower-layer duplicate of an endpoint function is proposed *with* a measurement showing it pays and a switch-off path, the rule permits it as an optimization — the rule forbids unmeasured, mandatory middle-layer help, not all middle-layer help.

---

## 10. Operator confirmations required

These go back to the operator as questions, not decisions — each marks a place where this document recommends something narrower than a literal reading of D13:

1. **Scope of "broadly."** D13 says adopt tokio/async "broadly across the agent lane." This document recommends: *no* tokio in `agent-loop`, `agent-facade`, or any kernel port signature; tokio's entry path is `mesh-adapter` (evidence-gated, §5.4) and, on falsification triggers only, confined inside `llm-adapters` (§6). **Confirm or correct.** If the underlying goal was something §3's table missed, naming it changes this analysis — say so and this pass re-runs against the real goal.
2. **Streaming without tokio.** Phase 1 ships LLM streaming with zero runtime change. If streaming was the driver of "tokio is a must," is Phase 1's result (with its benchmark) sufficient evidence to keep the narrow scope, or is tokio wanted at the LLM edge regardless of that measurement? **Confirm the evidence-gated framing or override it.**
3. **The mesh gate numbers.** ~1,000–2,000 sustained concurrent peer sockets as the thread→tokio threshold at the mesh edge (§5.4), set conservatively under reliability-over-latency. **Confirm, adjust, or replace with a different observable.**
4. **The io_uring/kernel-bypass prohibition (§4.2)** — adopted from the research's security evidence and the D0 reliability-over-latency invariant, under both readings of "exokernel-resistant." This one is a recommendation to *bind*, i.e., record alongside D13 so a future agent cannot re-import it casually. **Confirm binding, or leave advisory.**
5. **`ToolPort` stays synchronous permanently (§5.3)** — including under the §6 escalation. This is the strongest constraint proposed and the one most directly in tension with a maximal D13 reading. **Confirm.**

On confirmation, D13's entry in `DECISIONS.md` should gain a one-line pointer to this document as its completed scoping pass, and the stale "no tokio, per operator mandate" comments in `llm-adapters` module docs get updated to reference the new evidence-gated policy — as the migration lands, not retroactively (D13's own instruction).

---

*Sources: all citations are drawn from the two Opus research passes (2026-07-20) — Saltzer/Reed/Clark 1984; Clark 1988; RFC 1958; Blumenthal & Clark 2001; RFC 3234; Stone & Partridge SIGCOMM 2000; Balakrishnan et al. SIGCOMM 1996; Gettys & Nichols (bufferbloat); STUN/TURN/ICE RFCs; Tennenhouse & Wetherall 1996; Isenberg 1997; Benet 2014; Kleppmann et al. 2019; Engler/Kaashoek/O'Toole SOSP 1995; Kaashoek et al. SOSP 1997; Madhavapeddy et al. ASPLOS 2013; Agache et al. NSDI 2020; Kegel (C10K); N.J. Smith 2018; Nystrom 2015; Kobzol 2025; Armstrong 2003; Google Security Blog / Phoronix (io_uring VRP data); Apache Iggy migration writeup (Feb 2026) — and from the live dowiz tree: `llm-adapters/src/transport.rs`, `llm-adapters/src/dispatch.rs`, `agent-loop/src/lib.rs`, `kernel/src/ports/tool.rs`, `kernel/src/event_log.rs`, `agent-facade/src/lib.rs`, `mesh-adapter/Cargo.toml`, `DECISIONS.md` D0/D13, `docs/design/BLUEPRINT-SPATIAL-STOREFRONT-VOICE-HUB-SYNTHESIS-2026-07-20.md` (D-V1). Repo-file line numbers were re-verified against the working tree on 2026-07-20 during this synthesis.*
