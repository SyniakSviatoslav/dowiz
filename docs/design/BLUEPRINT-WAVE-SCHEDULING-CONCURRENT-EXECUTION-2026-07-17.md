# BLUEPRINT — Phase 25: Wave Scheduling & Concurrent Agentic Execution — Resource-Classed Admission Control (2026-07-17)

> Planning document; writes no product code. Built under the Detailed Planning Protocol
> (`AGENTS.md` §"Detailed Planning Protocol"): ground-truth-first, inline DECART, 2-question doubt
> audit, Anu/Ananke check. Style contract: plain prose, no metaphor; every load-bearing statement
> carries a `file:line` cite, a live-command ground, a web citation, or is tagged **(proposal)** /
> **(training-knowledge)**.
>
> **Operator ask (2026-07-17):** organize all work as multi-sequenced concurrent parallel waves
> with concurrent parallel agentic execution, dynamically adjusted in real time for non-breaking
> resource consumption; the host "has 8 cores, so at least 8 concurrent waves should be possible" —
> and research whether CPU-scheduling techniques allow meaningfully more than 8 in practice, rather
> than assuming a naive 1-core-per-task limit.
>
> **Operator clarification (2026-07-17, mid-design, binding):** (a) CPU-bound local compute must
> stay **strictly core-bound** — scheduled onto real cores, not SMT siblings, wherever possible;
> (b) the scheduling decision itself must be computed **natively and locally** — a fast,
> deterministic read of local state — **never** a network round-trip (no LLM call, no remote
> service) to decide admission. Both are named design rules in §3, not implicit assumptions.

---

## 0. Executive answer (the rest of the document is the derivation)

The operator's suspicion is **verified, with one correction and one sharpening**:

1. **Correction — this host does not have 8 cores.** `lscpu` (live, §1.1): 8 vCPUs = **4 physical
   cores × 2 SMT threads**, one socket, one NUMA node. For CPU-bound work the machine is worth
   roughly **4 cores + an SMT uplift of ~15-30%** ≈ 4.6-5.2 core-equivalents — not 8, and never 16.
2. **Sharpening — the 8-hardware-thread limit barely applies to agent dispatch at all.** The work
   this repo's sessions actually fan out (research/planning/code-writing agents calling the Claude
   API) is dominated by **network wait on the LLM response**, not local CPU. A blocked task does
   not occupy a core; the OS runs something else. This is the entire C10K/event-loop literature
   (§2.1) and it is why the defensible concurrency numbers split by **work class**:

| Work class | Bound by | Concrete bound on THIS host |
|---|---|---|
| **C — CPU-bound-local** (cargo build/test/clippy, benches, eqc, wasm builds) | physical cores | **4 strict-core slots** (`taskset -c 0,2,4,6`); Σ build threads ≤ 6 hw-threads if SMT headroom is explicitly allowed (§3.3) |
| **D — I/O-bound-dispatch** (agents waiting on managed LLM API) | memory per agent + API limits, **not CPU** | **16 default**, formula `D_max = min(⌊mem_budget/mem_per_agent⌋, API_limit, 16/workflow)` (§3.4) |
| **L — local-inference** (agents waiting on local Ollama) | the Ollama daemon's CPU use — this IS C-class load at the daemon | `OLLAMA_NUM_PARALLEL` (auto ≤ 4); each in-flight local inference counts against the C budget (§3.5) |

3. **Dynamic adjustment** is a pure, local, µs-scale admission function over PSI + `/proc` gauges
   (the exact surface Phase 24 already builds — consumed, not reinvented), called by the
   orchestrator's dispatch step before each admission (§3.6, §3.7). Admission control, never
   preemption; the OS scheduler (EEVDF, kernel 6.8) backstops any admission error gracefully (§2.3).

---

## 1. Ground truth (live-verified this session)

### 1.1 Host topology — the "8 cores" premise, corrected

```
nproc                    → 8
lscpu                    → Model name: AMD EPYC-Milan Processor
                           Thread(s) per core: 2 · Core(s) per socket: 4 · Socket(s): 1
                           NUMA node(s): 1 · NUMA node0 CPU(s): 0-7
/sys/…/topology/thread_siblings_list → (0,1) (2,3) (4,5) (6,7)
free -g                  → 30 GB total, 27 GB available
uname -r                 → 6.8.0-134-generic   (EEVDF scheduler era, ≥6.6)
/proc/pressure/{cpu,memory,io} → present and live (PSI, kernel ≥4.20)
  cpu:    some avg10=0.05 avg60=1.07 avg300=3.32   (idle-ish at capture)
  memory: some avg10=0.00  io: some avg10=0.00
```

The guest presents 4 cores with SMT sibling pairs; the **physical-core CPU list is `0,2,4,6`**
(one thread per sibling pair). Caveat, named: under QEMU the guest topology may not map 1:1 to the
hypervisor's real SMT siblings — the guest-visible pairs are still the only actionable handle for
core-binding from inside the VM, and Hetzner's vCPU-to-hardware mapping is not verifiable from
here **(flagged, not assumed)**.

`HARNESS-LLM-BACKEND.md` §1.1 recorded this host as "8-vCPU AMD EPYC Milan / 32GB RAM, no GPU" —
correct as far as it went; the 4c/8t split is the refinement this blueprint adds.

### 1.2 Existing concurrency policy inventory (reconciled against, not contradicted)

| Where | What it says | Status vs. this design |
|---|---|---|
| Workflow tool (this environment's own doc) | "Concurrent agent() calls are capped at min(16, cpu cores − 2) per workflow" → **min(16, 6) = 6** on this host | KEPT. §3.4 shows the `cores − 2` term is CPU-derived and conservative for D-class work, but the per-workflow cap stands; the global governor is the admission function, and multiple workflows/waves may run concurrently under it |
| `CLAUDE.md` "Spawn parallel subagents for independent work" | fan out 2+ independent units concurrently; no numeric cap | KEPT; this blueprint supplies the missing numbers |
| `.claude/skills/dispatching-parallel-agents/SKILL.md` | one agent per independent domain; no numeric cap | KEPT; same |
| `CONCURRENCY-ANALYSIS-2026-07-11.md` §4 | "max parallel = 3 concurrent per delegate cap" | Historical (a 2026-07-11 delegate cap, pre-dating current tooling); superseded by this document for wave sizing — noted, not silently dropped |
| `HARNESS-LLM-BACKEND.md` §1.2 | `OLLAMA_NUM_PARALLEL` default 1, auto-selects ≤ 4 by memory; `OLLAMA_MAX_QUEUE` 512 then HTTP 503 | KEPT verbatim as the L-class bound (§3.5) |
| `BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR-2026-07-17.md` §2 | "8-agent swarms are the recorded max"; auto-dispatch budget ≤ 5/day (TokenBucket) | The 8-agent record is an observed high-water mark, not a derived limit; the TokenBucket budget is a *spend* bound orthogonal to this *resource* bound — both apply |
| Memory: token-lifecycle thresholds | lane KILL @ >80K tokens | The per-agent *context* budget — the memory-side reason D_max is finite (§3.4) |
| `AGENTS.md` shared-working-tree hazard | concurrent code-writing agents MUST use `isolation: "worktree"` | KEPT; wave fan-out of code-writing units inherits it unchanged |

### 1.3 The two sibling blueprints this design consumes (operator-directed seams)

- **P24** (`BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md`): §3.4 extends
  the existing gauge surface with `/proc/pressure/{cpu,memory,io}` `some avg10` + `total`,
  two-snapshot `/proc/self/stat` CPU deltas, and VmRSS — **exactly the admission signal set**.
  §2.1 establishes the discipline this blueprint inherits: "never re-measure what the OS already
  accounts"; PSI is the kernel's own cause-attribution/oversubscription primitive
  ([PSI kernel doc](https://docs.kernel.org/accounting/psi.html): pressure data enables systems to
  be "managed dynamically using techniques such as load shedding … or strategically pausing or
  killing low priority or restartable batch jobs"). This blueprint adds **no third monitoring
  mechanism** — the admission function reads P24's gauges (or, pre-P24, the same three procfs
  files directly, which is what P24's gauges wrap).
- **Orchestrator/GapWire** (`BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR-2026-07-17.md`): §4.3's dispatch
  loop step 5 (`AutoResearch → bucket.try_acquire → dispatch`) is the one mechanical place
  admission plugs in: the call becomes `bucket.try_acquire && admit(class, gauges)`. The
  `TriagePolicy`-as-data pattern (§4.3) is reused for admission thresholds (§3.6).

---

## 2. Research — which techniques are real levers here, and which are not

Provenance note: this session's WebSearch budget was exhausted before this task; all sources below
were either fetched live via direct URL fetch this session (marked *fetched*) or are canonical
literature cited from training knowledge (marked **(training-knowledge)**, per the style contract,
with the numeric claims to be re-verified when search reopens). None of the design's load-bearing
*numbers* rest on an unfetched source alone — the host numbers are live, and the one
training-knowledge number (SMT uplift %) has a proposed on-host falsifier (§7 W2).

### 2.1 The load-bearing fact: blocked tasks don't occupy cores (VERIFIED, this is the real lever)

The operator's key insight is correct and it is not a "CPU trick" — it is the blocking profile of
the work:

- **C10K** (*fetched*: [kegel.com/c10k.html](http://www.kegel.com/c10k.html), 1999-2001): "It's
  time for web servers to handle ten thousand clients simultaneously, don't you think?" — the
  founding catalog of I/O strategies showing event-driven/nonblocking servers handling 10,000+
  concurrent connections on single-digit-core machines, because a connection waiting on the
  network costs a file descriptor and some bytes, not a core. One-thread-per-client is listed as
  the memory-hungry strategy, not the CPU-impossible one — the constraint was *per-connection
  memory*, exactly the constraint that reappears here as per-agent context (§3.4).
- **Tokio** (*fetched*: [tokio.rs/blog/2019-10-scheduler](https://tokio.rs/blog/2019-10-scheduler)):
  one worker thread per core, per-worker run queues, work stealing — "when a processor becomes
  idle, it checks sibling processor run queues and attempts to steal from them" — with "many user
  land tasks … multiplexed on a few operating system threads" (M:N). The work-stealing deque is
  Chase & Lev, "Dynamic Circular Work-Stealing Deque" (SPAA 2005) **(training-knowledge)**; Go's
  runtime is the same M:N shape (`GOMAXPROCS` = cores, goroutines in the thousands)
  **(training-knowledge)**. The relevance is *conceptual*, not a code dependency: these runtimes
  are the proof that logical concurrency is bounded by cores only for the *runnable* fraction of
  tasks.
- **The sizing formula** — Brian Goetz, *Java Concurrency in Practice* (2006) §8.2
  **(training-knowledge)**: `N_threads = N_cpu × U_cpu × (1 + W/C)` (W = wait time, C = compute
  time per task). Applied to a dispatched research agent: a turn spends tens of seconds waiting on
  the LLM API for well under a second of local compute (JSON assembly, file reads) → W/C ≥ 50 →
  the CPU-derived ceiling is `8 × 1.0 × 51 ≈ 400+`. **CPU arithmetic is not what bounds D-class
  work on this host.** Memory and API limits bind about an order of magnitude sooner (§3.4).
- **Note on dowiz's own no-tokio DECART** (`HARNESS-LLM-BACKEND.md` §0/§1.4: `ureq`, blocking,
  "NO tokio"): orthogonal, not a constraint. That decision governs the HTTP client *inside one
  adapter process*. Wave-level concurrency here is process-level (N agent processes, each mostly
  blocked in `ureq`'s socket read) — the OS multiplexes blocked processes exactly as an async
  runtime multiplexes blocked tasks, at a cost of one process's memory instead of one task's.
  No workflow-level scheduler dependency on tokio is introduced or needed.

### 2.2 SMT/Hyper-Threading — real but small for CPU-bound; the reason "8" was already generous

- Intel's own historical claim for Hyper-Threading was **up to ~30%** throughput on multithreaded
  workloads, not 2× **(training-knowledge; Intel's page returned 403 to live fetch this
  session)**. Independent compile-workload benchmarks (Phoronix SMT on/off passes on EPYC/Ryzen)
  typically land in the **15-30%** range for build-like integer work, with regressions possible on
  cache-thrashing loads **(training-knowledge, same flag)**.
- Mechanism: two hardware threads share one core's execution resources; the second thread only
  adds throughput when the first stalls (cache miss, branch). This is why SMT helps
  memory-stall-heavy work more than dense compute — and why for THIS host, CPU-bound capacity is
  honestly "4 cores + ~a fifth to a third of a core each," ≈ **4.6-5.2 core-equivalents**, not 8.
- Consequence for the operator's framing: "8 cores → at least 8 waves" was **optimistic for
  C-class work** (the machine has 4 real cores) and **pessimistic for D-class work** (which needs
  almost no cores at all). The class split, not an SMT trick, is where the real concurrency comes
  from.
- On-host falsifier (W2, §7): time `cargo build` in kernel/ at `-j4` pinned to `0,2,4,6` vs `-j8`
  unpinned; the measured ratio replaces the training-knowledge percentage in this document.

### 2.3 Linux CFS→EEVDF, nice, cgroups — the OS already multiplexes; what app-level admission is FOR

- *Fetched*: [sched-design-CFS](https://docs.kernel.org/scheduler/sched-design-CFS.html) — "CFS
  basically models an 'ideal, precise multi-tasking CPU' on real hardware"; picks the task with
  smallest `vruntime`; "no notion of 'timeslices' in the way the previous scheduler had"; the doc
  notes "CFS is making room for EEVDF" (merged 6.6; this host runs 6.8). Oversubscribed runnable
  threads are time-sliced fairly at ms granularity; context-switch direct cost is µs-scale (~3-5 µs
  measured in Li/Ding/Shen, "Quantifying the Cost of Context Switch", ExpCS 2007
  **(training-knowledge)**). **Moderate CPU oversubscription degrades throughput by low single-digit
  percent; it does not deadlock or collapse.**
- So is explicit core-counting even necessary? **Yes, but for exactly three reasons, none of which
  is "the kernel can't cope":** (1) **memory is not gracefully multiplexed** — oversubscribed RAM
  means OOM-kill or swap-thrash, and agents hold 100s-of-MB contexts; (2) **latency** — a `-j8`
  build at default weight steals interactive latency from the dispatch/lead plane; (3) **remote
  budgets** — API rate limits and token spend are invisible to the kernel. Admission control
  targets those three; CPU fairness itself is delegated to EEVDF.
- Complementary mechanism, adopted (§3.3): run C-class jobs at `nice 10` (or cgroup v2
  `cpu.weight` low) so that even when admission math is wrong, builds yield to the interactive
  plane — cgroup v2's weight model gives contending groups proportional CPU without hard quotas
  ([cgroup-v2 doc](https://docs.kernel.org/admin-guide/cgroup-v2.html), *cited in P24 §2.4*).
  Load-average caveat (*fetched*: [Brendan Gregg, "Linux Load Averages: Solving the Mystery"](https://www.brendangregg.com/blog/2017-08-08/linux-load-averages.html)):
  Linux `load1` counts **uninterruptible D-state (disk/lock-wait) tasks as well as runnable
  ones** — by design since a 1993 patch — so it conflates CPU demand with I/O wait. This is why
  the admission predicates below lead with **PSI**, and use `load1` only as a secondary bound.

### 2.4 NUMA — verified irrelevant

`lscpu`: 1 socket, 1 NUMA node, node0 = cpus 0-7 (§1.1). All memory access is uniform; NUMA
pinning, interleaving, and locality policy have no effect on this host. Excluded from the design —
stated once here so its absence is a decision, not an oversight.

### 2.5 io_uring — scoped correctly, not adopted

io_uring batches syscall submission/completion through shared SPSC rings (io_uring(7), *cited in
P24 §2.3*); its win is syscall-heavy file/storage I/O. Outbound HTTPS to an LLM API is a few
long-lived sockets per agent — epoll-class blocking/readiness I/O is already fine, and dowiz's
`ureq` adapters are blocking-by-design. The one place io_uring could matter locally is compiler/
linker file I/O inside cargo — which is rustc's business, not this scheduler's. **Not adopted;
named as out of scope** rather than silently missing.

---

## 3. Design — the wave admission model

### 3.1 Named rule: LOCAL-DECISION (operator directive 2026-07-17, binding)

**The admission decision is a deterministic, native, local computation.** Its inputs are reads of
local state only — `/proc/pressure/*`, `/proc/loadavg`, `/proc/meminfo`, P24's gauge snapshot, and
the orchestrator's own in-memory count of in-flight work. Its cost is microseconds (a few procfs
reads + arithmetic). It **never** performs a network round-trip — no LLM call, no remote service
check — to decide whether to admit a task. Rationale, restated from the operator: the decision
must be as fast as the thing it gates, or it becomes the bottleneck itself — the same principle as
P24's hot-path contract (no lock, no allocation, no I/O on the instrumented path; P24 §0). A
corollary: admission math failure mode is **fail-open-to-defer** (if a gauge file is unreadable,
defer C-class admission, admit D-class at the floor rate) — never "ask something remote."

### 3.2 Named rule: CORE-BOUND (operator directive 2026-07-17, binding)

**C-class work is scheduled onto real cores, strictly, wherever possible.** On this host that
means `taskset -c 0,2,4,6` (the one-thread-per-sibling-pair list, §1.1) or a cgroup `cpuset` with
the same four CPUs, giving **4 strict-core slots**. SMT siblings (1,3,5,7) are deliberately left
to the interactive/dispatch plane, which is nearly idle in CPU terms and benefits most from SMT's
stall-filling (§2.2). Escape hatch, named: when the host is otherwise idle (PSI cpu ≈ 0, no
D-class latency-sensitive work in flight), an uncapped `-j8` build may use all 8 threads for the
~15-30% SMT throughput bonus — this is an explicit admission-function branch, not a default.

### 3.3 C-class (CPU-bound-local) admission

- **Slot model:** 4 strict-core slots (§3.2). One *uncapped* `cargo build`/`cargo test` consumes
  all 4 (cargo defaults `-j` to nproc and parallelizes internally); admitting it means admitting
  the whole C budget. Finer-grained sharing = cap jobs (`-j2` × 2 concurrent, `-j1` × 4).
  Admission accounting is in **threads requested**, not "jobs": `Σ job_threads ≤ 4` (strict) or
  `≤ 6` (SMT-headroom branch, matching the environment's own `cores − 2` formula on nproc).
- **Niceness:** every C-class job runs `nice 10` (§2.3) — structural latency protection
  independent of admission correctness.
- **Dynamic predicate** (named constants, tunable via policy-as-data, §3.6):

```
admit_cpu(req_threads) ⇔
      psi_cpu_some_avg10  < CPU_PSI_HIGH   (= 15.0 %)
  AND load1               < N_HW_THREADS   (= 8.0)
  AND inflight_c_threads + req_threads ≤ C_SLOTS (= 4; 6 on the idle-host SMT branch)
```

### 3.4 D-class (I/O-bound-dispatch) admission

- **What bounds it (derived, §2.1):** not CPU (Goetz arithmetic gives 400+). The real bounds:
  (1) **memory per agent** — each dispatched lane is a live process holding context (the 80K-token
  lane-kill threshold in standing memory is the context-side expression of the same budget);
  (2) **provider-side API rate/concurrency limits** — invisible locally, surfaced only as 429/529
  responses; (3) the environment's own per-workflow cap `min(16, cores − 2) = 6`, which stays.
- **Formula:**

```
D_max = min( ⌊MEM_AGENT_BUDGET / MEM_PER_AGENT⌋ , API_CONCURRENCY , WORKFLOW_CAP × N_WORKFLOWS )
```

- **Defensible default numbers for THIS host:** `MEM_AGENT_BUDGET` = 16 GB (27 GB available minus
  ~6 GB Ollama residency (`HARNESS-LLM-BACKEND.md` §1.2: three models resident) minus 4 GB C-class/
  page-cache headroom minus margin). `MEM_PER_AGENT` ≈ 0.5-1 GB **(estimate, flagged — the single
  most important number to measure; W1 in §7 measures it from `/proc/<pid>/status` VmRSS of live
  lanes)**. That gives 16-32 by memory; API concurrency is unverified this session (flagged); so
  the standing default is **D_max = 16 concurrent dispatched agents** — 2× the hardware threads,
  ~2.7× the per-workflow CPU-derived cap — raiseable to 24+ once `MEM_PER_AGENT` is measured and
  memory PSI stays clean at 16.
- **Dynamic predicate:**

```
admit_dispatch() ⇔
      mem_pct             < MEM_HIGH        (= 80 %)
  AND psi_mem_some_avg10  < MEM_PSI_HIGH    (= 5.0 %)
  AND inflight_agents     < D_MAX           (= 16 default)
```

  Memory pressure, not CPU pressure, is deliberately the throttle here — PSI memory `some` > 0 on
  this host means reclaim/swap activity has begun, which is the earliest honest overcommit signal
  ([PSI kernel doc](https://docs.kernel.org/accounting/psi.html)); `psi_cpu` does not appear in
  this predicate at all, because D-class work is entitled to run when builds have the cores busy.

### 3.5 L-class (local-inference) — the class the naive framing misses

An agent waiting on **local Ollama** looks I/O-bound from its own process, but its wait time IS
this host's CPU doing llama.cpp inference. Dispatching 16 "I/O-bound" agents at local models would
be a C-class storm wearing a D-class label. Rules: (1) inference concurrency is delegated to the
daemon's own governor — `OLLAMA_NUM_PARALLEL` (default 1, auto ≤ 4 by memory) + `OLLAMA_MAX_QUEUE`
(512, then 503), already live per `HARNESS-LLM-BACKEND.md` §1.2 — do not fight it from outside;
(2) each in-flight local inference is **counted against the C budget** (it occupies cores exactly
like a build does); (3) mixed waves (some lanes on managed API, some on Ollama) therefore admit
their lanes under different predicates — the class attaches to the *work unit*, not the wave.

### 3.6 Where the admission function lives (proposal)

- `kernel/src/admission.rs` **(proposal)** — a pure, std-only module mirroring `intake.rs`'s shape
  (deterministic, fail-closed, testable without I/O): `pub enum WorkClass { CpuBoundLocal { threads: u8 },
  IoBoundDispatch, LocalInference }`, `pub struct Gauges { psi_cpu_some_avg10, psi_mem_some_avg10,
  load1, mem_pct, inflight_c_threads, inflight_agents }`, `pub fn admit(class: &WorkClass, g: &Gauges,
  p: &AdmissionPolicy) -> Verdict` with `Verdict::{Admit, Defer { retry_after_ms }}`. All
  thresholds live in `AdmissionPolicy` (policy-as-data, the GapWire `TriagePolicy` pattern —
  orchestrator §4.3), so P15's future self-revision path can tune them through the same
  propose→mirror→apply gate, never by code edit.
- **Callers:** (1) the GapWire drainer's dispatch step (orchestrator §4.3 step 5) — the line
  becomes `bucket.try_acquire(n) && admit(class, &gauges, &policy).is_admit()`; (2) until GapWire
  exists, the lead agent applies the same table manually when sizing a fan-out — the numbers in
  §0's table are exactly what to apply by hand. Gauges come from P24 §3.4's extended gauge surface
  (or the same three `/proc/pressure` files + `/proc/loadavg` + `/proc/meminfo` read directly
  pre-P24 — identical bytes, no new mechanism).
- **Backoff on Defer:** exponential with jitter — recheck at 5 s, 10 s, 20 s, 40 s, cap 60 s
  (named constants). **Admission-only, never preemption:** running work is never killed by this
  mechanism (PSI's own doc lists "pausing or killing … restartable batch jobs" as an option; this
  design deliberately takes only the admission half — killing a half-done build or agent lane
  wastes everything already spent, and EEVDF + nice already contain a running overload).
- **Hysteresis:** thresholds are stated as high-water marks for *admission*; a deferred class
  re-admits only when the gauge falls below `0.8 × threshold` (low-water), preventing oscillation
  on a noisy avg10.

### 3.7 What "multi-sequenced concurrent parallel waves" means operationally

A wave is a set of work units with no mutual dependency (the roadmap's existing definition, §2 of
the master roadmap). This blueprint adds: **a wave is sized by class, not by a single number.** A
wave's D-class units fan out up to `D_max` immediately; its C-class verification steps (each
phase's `cargo test` done-check) queue through the 4-slot C budget; its L-class units queue
through Ollama's governor. Two waves may overlap in time when their classes don't contend — e.g.
Wave-1 planning agents (D) run at full width while Wave-0's builds (C) still occupy the cores.
"Multi-sequenced" is therefore free: sequencing constraints live in the dependency graph, width
constraints live in the admission function, and the two never share a bottleneck except when two
C-class steps collide — which is precisely the collision the C budget exists to serialize.

---

## 4. DECART — the admission mechanism (new integration class: scheduling discipline)

| Candidate | Native fit | Falsifiable correctness | Cost | Supply chain | Reversibility | Verdict |
|---|---|---|---|---|---|---|
| **Rely on Linux EEVDF alone** (spawn everything, let the kernel multiplex) | ✓ zero code | kernel-proven for CPU | zero | none | — | REJECT as sole mechanism: gracefully handles CPU only; OOM/swap (D-class memory) and API budgets are invisible to it (§2.3). ADOPTED as the backstop layer |
| **cgroup v2 hard quotas** (`cpu.max` per class) | ✓ kernel | ✓ | idle-capacity waste (quota caps even when host idle) | none | easy | REJECT for quotas; **ADOPT the weight half** (`nice`/`cpu.weight` on C-class, §3.3) — proportional, work-conserving |
| **External job scheduler** (systemd-run properties, nomad, slurm) | ✗ new service/dep; slurm-class tools assume clusters | — | heavy for 1 host | new dep (and crates.io/apt egress constraints recorded in P15 §9) | poor | REJECT |
| **LLM-in-the-loop admission** (ask an agent "should I dispatch?") | ✗ violates LOCAL-DECISION rule outright — network latency deciding µs-scale admissions | not falsifiable | seconds per decision | — | — | REJECT, categorically (operator directive, §3.1) |
| **Pure native admission fn over PSI/procfs + nice, EEVDF backstop** ← | ✓ std-only pure fn, mirrors `intake.rs`; reads P24's gauges | every predicate a table-driven unit test; thresholds named constants | µs per decision | zero new deps | trivial (delete module; behavior reverts to today's unthrottled fan-out) | **ADOPT (build)** |

**DECISION:** build the pure native admission function, because (falsifiable reason) it is the
only candidate that covers all three genuinely-unmanaged resources (agent memory, API budget,
build/interactive latency) while satisfying the LOCAL-DECISION rule, at zero dependency cost, on
telemetry that P24 already commits to producing. **Mandatory probe (strongest honest argument
against):** admission on lagging averages (avg10) can oscillate or under-admit — a 10-second-old
signal gates a decision about the next 60 seconds. Mitigations: hysteresis (§3.6), the
fail-open-to-defer posture, and the decisive one — the EEVDF backstop means a wrong *admit* costs
low-single-digit-% throughput (§2.3), and a wrong *defer* costs seconds of latency; neither is
correctness-affecting. If measured oscillation exceeds nuisance level, the named upgrade is PSI
kernel-side triggers (`poll()` on `/proc/pressure/*`, P24 §2.1's noted upgrade path), still
local-only.

---

## 5. Retroactive classification of the master roadmap's waves

Applied into `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §2 directly (append-only note);
summary of the reasoning here:

- **Every implementation phase is two-faced:** its *agent work* (reading, designing, writing code
  text) is D-class — parallelizes to `D_max`; its *verification steps* (each blueprint's
  falsifiable `cargo test`/`cargo build` done-check) are C-class — serialized through 4 slots.
  The wave diagram's width was never wrong; it was underspecified about which face it was
  measuring.
- **Predominantly C-class at execution time** (Rust build/test-heavy done-checks): P1 (CI truth
  floor — kernel 337 + engine 47 tests), P3 (bebop2 crypto: 5 crates, CT tests), P4, P5, P6, P7,
  P8, P9, P10, P11, P12, P13, P24 (ring + criterion benches + `bench_track.py` runs — the
  AGENTS.md mandatory-benchmark doctrine makes every wave partly C-class by law).
- **Predominantly D-class** (research/planning/doc output, no heavy local build): P2 (canon
  repair), P18-prep (readiness docs), P14's ruling-dependent design half, P20's asset/doc units,
  P22's blueprint-stage work, all future blueprint/research fan-outs (the 25-agent roadmap
  production run and the recorded 8-agent swarms were D-class in exactly this sense — which is why
  they worked on "4 cores" without anyone noticing a core limit).
- **L-class:** P21 (resident-agent plane — its runtime loop consumes Ollama CPU) and any P15
  E13-cpu work once O18b unlocks — these count against the C budget while inferring, per §3.5.

## 6. Proposed AGENTS.md addition (clearly marked — NOT applied by this blueprint; operator merges)

> ### Wave classification rule (proposed, 2026-07-17)
>
> Every future blueprint/roadmap wave-or-sequencing section MUST classify each parallel work unit
> as one of: **CPU-bound-local** (local build/test/lint/bench — admission bound: Σ threads ≤ 4
> strict-core slots on this host, `taskset -c 0,2,4,6`, `nice 10`), **I/O-bound-dispatch**
> (agent lanes waiting on a managed LLM API — admission bound: D_max = 16 default, gated by
> memory PSI, never by core count), or **local-inference** (lanes waiting on local Ollama —
> bounded by `OLLAMA_NUM_PARALLEL`, counted against the CPU budget). A bare "N parallel units"
> claim with no class label is an incomplete plan under the Detailed Planning Protocol. Admission
> decisions are computed locally and natively (procfs/PSI/P24 gauges) — never via a network
> round-trip (LOCAL-DECISION rule). Numbers cite `BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md`
> and must be re-derived if the host topology changes (`lscpu` is the check).

## 7. Build plan — falsifiable done-checks

| # | Unit | Depends on | Falsifiable done-check |
|---|---|---|---|
| W1 | Measure `MEM_PER_AGENT`: capture VmRSS of ≥3 live dispatched lanes at mid-task | — | Three `/proc/<pid>/status` VmRSS readings recorded in this doc's §3.4, replacing the 0.5-1 GB estimate; D_max re-derived from the measurement |
| W2 | SMT uplift on-host falsifier | — | `cargo build` (kernel/, cold) timed at `-j4` pinned `0,2,4,6` vs `-j8` unpinned, 3 runs each; measured ratio recorded in §2.2, replacing the training-knowledge 15-30% band |
| W3 | `kernel/src/admission.rs` pure fn + table tests | — | `cargo test -p kernel admission::` green: every predicate row from §3.3/§3.4 as a fixture, incl. hysteresis (defer at high-water, re-admit only below low-water) and gauge-unreadable → C-defer/D-floor behavior; `grep -c "ureq\|http" kernel/src/admission.rs` == 0 (LOCAL-DECISION rule, CI-greppable) |
| W4 | Wire into GapWire dispatch step | W3 + orchestrator W1 | Dispatch loop refuses a C-class dispatch under an injected `psi_cpu_some_avg10 = 20` gauge fixture and admits a D-class one under the same fixture (the class split is the observable) |

W1 ⊥ W2 ⊥ W3 (mutually independent, all startable now); W4 needs W3 and the orchestrator's own W1.

## 8. 2-question doubt audit

**Q1 — least confident about (concrete):**
1. `MEM_PER_AGENT` is an estimate, not a measurement — W1 exists because of this; D_max = 16 is
   defensible but its raiseable-to-24 claim rests on the unmeasured number.
2. SMT uplift %, Chase-Lev, Goetz, and context-switch-cost figures are training-knowledge (search
   budget was exhausted); each is flagged inline, W2 replaces the one that carries a design number.
3. Anthropic API-side concurrency limits were not verified this session — `API_CONCURRENCY` enters
   the D_max formula as an unknown that currently never binds below 16; if the provider throttles
   lower, 429s will surface it and the term becomes real.
4. Whether the Workflow tool's "cpu cores − 2" reads nproc (8→6) or physical cores (4→2) is
   unverified — the reconciliation in §1.2 assumes nproc; if it reads physical cores the
   per-workflow cap is 2 and the case for classing D-work separately gets *stronger*, not weaker.
5. The guest's SMT-sibling topology may not map to the hypervisor's real siblings (§1.1) — pinning
   to `0,2,4,6` is the best available action and is correct on the guest's own scheduler, but the
   physical-core claim is one virtualization layer less certain than it reads.
6. PSI thresholds (15%/5%) are chosen from the signal's semantics, not tuned from this host's
   measured behavior under a real mixed load — the hysteresis + backstop design makes mis-tuning
   cheap, but the constants are v1 guesses and say so.
7. Ollama's actual thread usage per parallel request was not measured — §3.5 counts an in-flight
   inference as "occupies the C budget" without a thread-exact number.

**Q2 — biggest thing possibly missed:** the binding global constraint on D-class fan-out may not
be local at all — session token budget (the 300K save+push threshold and the headroom proxy in
standing memory) and provider rate limits could saturate before 16 concurrent lanes ever stress
30 GB of RAM. If so, this design still does no harm (its bounds simply never bind), but the
operator should know the honest ordering: **spend budget is probably the true frontier, local
resources second** — this blueprint governs the second so that the first is the only one anyone
has to think about.

## 9. Anu / Ananke check

**Anu (derivable, not asserted):** the 4-core correction derives from live `lscpu`, not from any
doc; the C/D split derives from the fetched C10K/Tokio/CFS sources plus the Goetz arithmetic; the
D_max default derives from a stated memory budget over a stated (and flagged) per-agent estimate;
the reconciliation table (§1.2) checks this design against every prior concurrency statement found
in the environment rather than contradicting any silently — the one superseded item
(CONCURRENCY-ANALYSIS's "max 3") is named as superseded. Weakest Anu links, named: the two
training-knowledge numeric bands (SMT %, context-switch µs) — both flagged, one falsified-on-host
by W2, the other decorative to the design (µs vs ms is all the argument needs).

**Ananke (structural, not hoped):** the LOCAL-DECISION rule is enforceable by grep (W3's
no-network-symbols check), not by review vigilance; thresholds are policy-as-data so tuning never
requires a code edit; the classification requirement becomes structural only if the operator
merges §6 into AGENTS.md — until then it is convention, and that gap is named here rather than
assumed closed. The admission function cannot become load-bearing for safety because the EEVDF/
nice backstop bounds the cost of any admission error at throughput-%, never at correctness — the
design degrades, by construction, to "what the host already does today."

---

## Appendix — phase-table registration

Registered in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8 as **Phase 25** (25
confirmed free at registration time: §8.1 ends at P23, §8.2 explicitly declined a number, §8.6
took P24 — re-read fresh this session, not assumed). Depends on: **24** (gauge surface — PSI +
`/proc/self` deltas are P24 W1b; pre-P24 the same procfs files are read directly, so the
dependency is soft: it shapes *where* the gauges come from, not *whether* admission can compute).
Off-critical-path lane, same class as P5/P8/P11/P12/P24. Wave-classification note applied
retroactively to §2's Waves section (append-only).

---

## Addendum (2026-07-17, append-only — Phase 26 cross-reference)

Phase 26 (`BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md`) takes up this blueprint's
flagged unknown — `MEM_PER_AGENT` is an estimate (§3.4, W1) — and supplies the mechanism that
replaces it: a `MemoryBudget` primitive (`TokenBucket`'s byte-budget sibling; reserve/release, no
time-refill) so D-class admission calls `try_reserve(mem_per_agent_ewma)` per lane and `release`
on lane exit, with the estimate seeded by W1's VmRSS measurements and maintained as an EWMA of
completed-lane peak VmRSS from P24's gauges. **The formula in §3.4 is unchanged**; what changes is
that its memory term becomes measured + enforced instead of assumed, making the static
`D_MAX = 16` the secondary bound and the byte reservation primary (PSI-memory stays in the
predicate as the kernel-ground-truth backstop). Consequence stated there, not here: if measured
`MEM_PER_AGENT` ≈ 0.5 GB, the same 16 GiB budget admits ~32 lanes — the "raiseable to 24+" claim
in §3.4 gets its mechanism; if ≈ 1 GB, D_max honestly stays ~16. P26 also bounds the two
unbounded native memory surfaces that would otherwise erode `MEM_AGENT_BUDGET` under long
16-lane sessions (the exact-match LLM cache and `FileBlockStore`'s eager whole-store RSS mirror).
