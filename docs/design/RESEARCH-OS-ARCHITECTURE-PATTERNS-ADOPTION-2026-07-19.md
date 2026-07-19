# Research / Feasibility — Real OS Architecture Patterns, Adoption Fit for the dowiz Kernel

**Date:** 2026-07-19 · **Role:** research + feasibility assessment ONLY (Opus). Goes one step past the
earlier grounding pass (`RESEARCH-CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-2026-07-19.md`, which
established that seL4 capability isolation, OTP supervisor trees, and TMR do **not** exist in dowiz):
this pass assesses **whether each pattern actually fits dowiz's real architecture**, and for the ones
that do, sketches a concrete, proportionate adoption enough for a later Fable synthesis pass to turn
into roadmap items. **No blueprint, no full design, no roadmap re-sequencing** — that is the next pass.

**Epistemic rule (this arc's convention):** every claim tagged **GROUNDED** (verified against live
source this pass or the cited prior grounding pass) or **PROPOSED** (a reasoned extension, not built,
not measured). Space-grade quality bar applies regardless of deployment substrate, per the binding
memory note `space-grade-quality-not-deployment-scoped-2026-07-19.md` — **no fit assessment below
scopes a pattern down via "we're not really in space."** Fit is judged on genuine engineering merit:
does the pattern solve a problem dowiz's *actual* architecture has. The confirmed deployment target is
**local, offline-first, consumer-grade hardware (typically no ECC)** — this raises, not lowers, the
transient-fault fit for redundancy patterns.

---

## 0. The load-bearing architectural facts (GROUNDED this pass) — every fit assessment rests on these

Five facts about dowiz's *real* architecture decide every verdict below. All verified against live
`main` source this pass unless a prior-pass citation is given.

- **F1 — The kernel is a single-process, synchronous library of pure functions.** No async runtime in
  the default build. The ONLY `async fn` in `kernel/src` is `retrieval/memory_store.rs:142,157` (the
  `pgrust` Postgres pool, behind the opt-in `pgrust` feature → `dep:tokio`). `tokio` is absent from the
  default kernel graph (`kernel/Cargo.toml` `[features]`: `pgrust = ["dep:sqlx", "dep:tokio"]`,
  off-default; `gpu` pulls `pollster` only to `block_on` a synchronous boundary). The kernel is linked
  *into* host binaries as an rlib/cdylib; it is not itself a running OS mediating other processes.
- **F2 — Concurrency is minimal and coarse-grained; there is no actor model, no thread pool, no
  inter-module message channel.** Grep for `thread::spawn`/`mpsc`/`crossbeam`/`channel()` across
  `kernel/src` returns essentially nothing in production paths: `budget.rs:357` uses `thread::scope`
  (a bench), `token_bucket.rs:146` a test sleep, `core_pinning.rs:53`/`span_metrics/obs.rs:62` read
  `available_parallelism`, `living_knowledge.rs:94` spawns a *subprocess* (not a worker thread). The
  only shared-memory concurrency is fine-grained lock/atomic over POD state: `token_bucket` (rate
  limit), `budget.rs` (a lock-free `AtomicU64` CAS cost accumulator), `spectral_cache`, `arena`. No two
  kernel modules communicate by message-passing — they call each other's pure functions directly.
- **F3 — The agent loop is a bounded, single-threaded, synchronous plan→act→observe state machine.**
  `agent-loop/src/lib.rs`: `AgentLoop::run` drives at most `MAX_AGENT_ITERATIONS = 4` turns, "no
  fourth path and no retry construct," model bytes map onto exactly three continuations, `dowiz-kernel`
  reachable only at depth 2 through `agent-facade` (enforced by a committed firewall test).
  `agent-loop/src/service.rs`: `serve_forever` handles **one** connection at a time, "each accepted
  connection drives exactly one bounded turn … no unbounded state, no self-scheduling." This is a
  deterministic bounded FSM, not a long-lived concurrent actor.
- **F4 — The kernel is already a ports-and-adapters (hexagonal) system with compile-time firewalls.**
  `kernel/src/ports/mod.rs`: `llm`, `agent` (AgentBridge), `payment`, `payment_provider`,
  `payment_capability`, `tool`, `mcp`, `customer`, `notification`, `owner_surface` — each a typed trait
  seam "where the kernel meets the outside world without importing it," every one carrying an explicit
  "compile firewall: kernel has NO adapter dependency" invariant (e.g. `payment_provider`: "the
  concrete Stripe adapter lives OUT-OF-KERNEL … No card-data type exists in core"). Cross-boundary
  interaction is *synchronous typed function calls through trait objects*, fail-closed, not async
  messages. `engine/src/lib.rs`: "Pure Rust, zero-dependency, offline-clean. Authoritative compute is
  CPU-side; GPU/wasm is a display surface." Engine is not a concurrent subsystem either.
- **F5 — Isolation today is compile-time (11 feature gates) + the WASM linear-memory sandbox + Rust's
  own type/ownership system; there is no runtime MMU/memory-capability boundary, no scheduler, no
  priority/deadline concept.** `kernel/Cargo.toml` `[features]`: `std`, `json-api`, `wasm`, `chaos`,
  `count-allocs`, `pgrust`, `pq`, `gpu`, `slot-arena`, `p67-adapters`, `telemetry`, `pprof` — each
  gates a whole subsystem *and its dependencies* out of the canonical order/money core (planned
  `inference` gate = item 45). Grep for `priority|deadline|preempt|scheduler|wcet|real.?time` in
  `kernel/src` finds only: `core_pinning.rs` (which *rejects* pinning — "pinning a thread to a core
  buys nothing and risks fighting the OS scheduler"), TTL `deadline`s (session/storefront timeouts,
  degrade-closed), and `router.rs` A* path-cost "priority" (a min-heap key, not a task priority). No
  dedicated `courier`/`dispatch`/`scheduler` module exists in `kernel/` or `engine/` (find returns
  nothing). WCET tooling: absent (prior pass §9; roadmap items 47/48 declare it explicitly out of
  scope). `budget.rs` is a *monthly cost* ceiling (degrade-closed), not a real-time scheduler.

These five facts recur below by number. The single most consequential one is **F2/F3**: dowiz has no
concurrent-actor substrate, which is what several of these OS patterns exist to manage.

---

## 1. seL4-style capability-based memory/module isolation

**(a) What dowiz has (GROUNDED).** Three real isolation mechanisms, none of them seL4's:
- Rust's compile-time type/borrow system is the *only* inter-module boundary inside the single kernel
  process (F1, F2).
- The **WASM linear-memory sandbox** (F5; prior pass §4) — a real memory boundary, but it isolates the
  *whole kernel* from its JS host, not one kernel module from another.
- **Compile-time feature gates** (F5) — 11 of them — plus `isolation/microvm.rs` (a *deployment* gate:
  "a node without KVM refuses `native-process` adapters instead of running them unsandboxed" — prior
  pass §4). `capability_cert.rs` is authority-token capabilities (UCAN/biscuit: *who may act on what*),
  **not** memory-access capabilities (prior pass §4, GROUNDED) — a genuine terminological distinction,
  not a hidden seL4.

**(b) Fit assessment — full seL4 is a category mismatch; a proportionate compile-time subset is real
and mostly already scoped.** seL4's entire value proposition is an **MMU-enforced runtime memory
boundary mediating multiple mutually-untrusted processes**, with machine-checked proofs that a
capability is unforgeable and confinement holds. dowiz is the opposite shape by construction (F1): a
single-process library with no untrusted co-tenant processes inside it. **For a single-process kernel,
Rust's ownership + the WASM whole-kernel sandbox + the feature gates already deliver most of what
seL4's isolation buys** — spatial isolation of the kernel from its host, and compile-time
impossibility of one module reaching into another's private state (no raw shared mutable memory between
modules; F2). Adopting seL4's MMU/page-capability machinery would mean building a multi-process
micro-OS underneath a library that has no need for one — disproportionate, and it would re-import the
platform/dependency surface the zero-dep constraint exists to remove.

**BUT** there is exactly one place the kernel is about to admit *less-trusted, non-deterministic* code
into its own address space: the **AI-inference subsystem (items 33–44)** and the **agent loop (F3)**.
That boundary is the real, proportionate analog of "an untrusted process the kernel must confine," and
the roadmap already scopes most of the right answer:
- **item 45** (`ai-optional-gate`): inference lands behind a non-default `inference` feature with a
  CI-enforced **dependency-direction check** — AI may reference core, core may never reference AI. That
  is a compile-time capability boundary in exactly the seL4 spirit, sized to a single-process kernel.
- the **Wasmtime-fuel pattern** (synthesis §3, roadmap): any path executing agent-supplied/less-trusted
  logic carries a pre-committed step budget with a deterministic trap — a *runtime execution
  capability*, the one seL4-flavored control that survives into a single process.

**(c) The genuinely NEW slice worth adopting (PROPOSED).** Make the inference/agent→core boundary a
**first-class typed capability, not merely a feature gate**: the AI subsystem holds an unforgeable
in-process token (a zero-sized capability type constructible only by the composition root, §7 below)
that it must present by *signature* to call a kernel port. A port method that requires
`cap: &CoreWriteCapability` cannot be invoked by code that was never handed one — the
illegal-state-unrepresentable house standard (synthesis §1.5) applied to *authority to touch the
deterministic core*. This reuses the **existing** `capability_cert.rs` attenuation/scoping machinery
*internally* (no new crypto, no new dependency) rather than inventing a memory-capability system. It is
strictly additive over item 45 (which stops cross-references at *compile* time; this also stops
*runtime* authority a compiled-in-but-untrusted path might otherwise exercise). **Slots into:** extends
item 45; composes with the fuel pattern; is the object-capability half of the "capability-per-call"
idea §4(a) below reaches independently. **Verdict: full seL4 = NO (category mismatch, F1); the
compile-time + fuel + typed-in-process-capability subset = YES, ~70% already scoped (item 45 + fuel),
one proportionate new slice.**

---

## 2. Erlang/OTP supervisor trees ("let it crash")

**(a) What dowiz has (GROUNDED).** `hub_supervisor.rs` is a **deploy-rollout** state machine
(A/B-slot promote/rollback, `CRASH_LOOP_MAX_RESTARTS = 3` at *release* granularity — prior pass §5),
NOT an actor supervisor. The FAIL-FAST + PostMortem + recover pattern exists (FDR, items 45–49). Item
48 explicitly adds a `std::panic::set_hook` panic-forensics record + a `Heartbeat` liveness variant,
and — decisively — **places restart authority OUTSIDE the kernel** (systemd `WatchdogSec` / deployment
layer; "the kernel carries NO self-kill/self-restart logic").

**(b) Fit assessment — an in-kernel OTP supervisor tree is a poor fit; dowiz has no actor model to
supervise, and the OTP *insight* is already adopted at the correct granularity.** An OTP supervisor
tree presupposes **many concurrent, independently-failing, individually-restartable
processes/actors** arranged in a hierarchy, each with private state, communicating by messages. dowiz's
concurrency model is the categorical opposite (F2/F3): single-threaded synchronous request/response,
no long-lived concurrent workers, no actors, no inter-module messages. **There is no population of
crashing children to supervise.** A literal `Supervisor{ Worker, Worker, … }` hierarchy would be
scaffolding around a system that runs one bounded synchronous turn at a time and then returns.

Crucially, the OTP *principle* — "let it crash; isolate the failure; restart at the right granularity"
— **dowiz already implements correctly, just not with OTP's machinery**: fail-fast + PostMortem
forensics (item 48a), external liveness + restart authority at the process/deploy layer (item 48b,
`hub_supervisor`'s crash-loop rollback). Erlang puts the supervisor *inside* the VM because the VM *is*
the OS; dowiz correctly puts it *outside* the kernel because the kernel is a library and systemd/the
deployment layer *is* its supervisor. Moving that authority inward would violate the item-48 KISS
finding and re-create the self-restart logic that pass deliberately rejected.

**(c) The one proportionate slice, and it is not a tree (PROPOSED, small).** Where OTP's *fault-domain*
idea does apply is **per-port fault isolation**: one failing adapter (e.g. a payment provider timing
out) must fail *closed* for that request without taking down the process or corrupting the deterministic
core. dowiz largely has this already via typed `Result` + degrade-closed ports (F4; `budget.rs`
"degrade-closed", `payment_provider` fail-closed). The additive nudge is to make that contract
*uniform and asserted* across every port — a "one crashing adapter cannot escalate past its own port
boundary" property test — rather than a per-port convention. **Slots into:** composes with item 9's
breaker (a tripped breaker is the fault-containment receiver) and item 48. **Verdict: in-kernel OTP
supervisor tree = NO (no actor substrate, F2/F3; insight already adopted at process/deploy
granularity); uniform per-port fail-closed isolation = a small YES, mostly already present.**

---

## 3. TMR (Triple Modular Redundancy) — redundant compute + vote

**(a) What dowiz has (GROUNDED).** Only integrity *detection*, never redundant-compute-and-vote (prior
pass §6): FDR CRC32, event_log SHA3 chain, item 40 (read-only weight checksum), **item 54 Sentinel**
(CRC32 read-time integrity for critical *live mutable* structs — items 47 `Invariants`, item 21
gain-schedule, live inference config — at transition points). Item 12 (SIHFT triple-vote pilot) is
PURSUE-**design-only**. The critical decision paths are cheap and pure: `money.rs` integer-only,
overflow-safe checked arithmetic; the order FSM zero-float/zero-I/O (~µs); `decision/import.rs`
`import_unit` measured **0.87 µs p50** (item 26).

**(b) Fit assessment — be precise about what TMR protects.** TMR defends against **transient hardware
faults (single-event upsets / bit-flips)** corrupting a *computation or the data feeding it*. It gives
**zero** protection against software bugs — identical buggy code produces the identical wrong answer
three times and the vote agrees on garbage. So the honest scope questions are two:
1. *Does item 54 already cover the real dowiz risk?* Item 54/Sentinel covers **at-rest / transition-time
   corruption of stored struct bytes**. It does **not** cover a bit-flip that occurs *during* the pure
   compute itself — a flipped register/ALU bit mid-`applyTax`, mid-FSM-transition, mid-`event_id` hash.
   That is a distinct, genuinely-uncovered fault class.
2. *Is that residual class worth defending, and cheaply?* On the confirmed **non-ECC consumer target**,
   transient compute-path SEUs are a real (if rare) class, not negligible. And the candidate functions
   are **µs-scale pure functions** (money gate, event-id hash, FSM transition) — so recomputing them
   2–3× and voting is nearly free. This is exactly item 12's scope, and it *is* additive over item 54.

**(c) The right-sized form is TEMPORAL TMR, and the honest caveat is carried (PROPOSED).** Classic
spatial TMR (three separate cores/dies + vote) is unavailable to a single-process kernel and, on shared
silicon, a single SEU in a shared cache/ALU corrupts all three replicas at once (synthesis §6 caveat,
GROUNDED-in-literature). The form that *does* fit F1 is **temporal TMR**: run the pure function 2–3×
**sequentially on one core** over the same inputs and vote. This catches a *transient* flip during any
one run (the runs disagree) at ~2–3× a tiny cost, with zero new hardware and zero new dependency — and
it stacks with item 54 (Sentinel guards the *inputs* at rest; temporal-TMR guards the *evaluation*).
It is honestly **partial** (a *permanent* fault or a bug corrupts all runs identically; a flip in the
voter itself is unguarded — mitigate by keeping the vote a trivial equality). Scope to the two or three
most critical cheap pure functions only (money gate, event-id hash), vote-mismatch → item 9 breaker
trip + FDR `Alarm` (synthesis §6 already prescribes this composition), never a claim of SEU immunity.
**Slots into:** refines item 12 from "triple-vote pilot" to specifically *temporal* triple-run, and
names item 54 as the complementary at-rest half. **Verdict: general TMR = NO for software bugs and
redundant vs item 54 for at-rest memory; a NARROW temporal-TMR on the cheapest-most-critical pure
computations = YES, genuinely additive (compute-time transient faults), cheap, already scoped as
item 12.**

---

## 4. Other real OS patterns

### 4(a) Microkernel message-passing IPC discipline

**What dowiz has (GROUNDED):** the ports-and-adapters layer (F4) IS already a microkernel-flavored
boundary discipline — a minimized trusted core, all cross-boundary interaction through **typed, minimal,
fail-closed seams** with no dependency leakage. But the transport is **synchronous trait-object function
calls**, not asynchronous message-passing (F1/F2).

**Fit:** the microkernel *insight* (mediate every cross-boundary interaction through a typed minimal
capability-checked seam; keep the trusted core small) is **already strongly present**. Converting the
synchronous trait-call ports into an actual message-passing bus (cFS software-bus / F´ typed-port
runtime) would add serialization, queuing, and latency to a **single-process synchronous** kernel for
**no isolation benefit** — the synthesis §3 already flagged "if messaging is already minimal and
direct, do not add a bus for ceremony." **Verdict: literal message-passing IPC bus = NO (ceremony
without benefit, F1/F2); the discipline it exists to enforce is already adopted via ports.** The one
worthwhile borrow from F´'s typed ports is **not** the bus but the **capability-checked call** — every
port call carries the §1(c) in-process capability token, so cross-boundary calls are capability-checked,
not just type-checked. That is the same PROPOSED slice as §1(c); it does not need a second item.

### 4(b) RTOS scheduling classes / priority inheritance

**What dowiz has (GROUNDED):** nothing scheduler-shaped (F5). No preemptive multitasking of kernel
work, no priority, no deadline (RT sense), no priority inversion surface (the few locks are over POD
with no nested lock ordering — F2), no WCET tooling, and `core_pinning.rs` *rejects* affinity pinning.
Courier dispatch is not even a separately-scheduled path — it is part of the deterministic order/decision
machine. `budget.rs` is a cooperative *cost* budget; the Wasmtime-fuel pattern is a cooperative *step*
budget.

**Fit:** classic RTOS priority scheduling and **priority inheritance** solve contention among concurrent
tasks of differing urgency sharing a *preemptive* CPU, and inversion when a low-priority task holds a
lock a high-priority task needs. dowiz has neither preemptive kernel-task multitasking nor a
priority-inversion surface (F2/F5), so **priority inheritance is N/A** and **preemptive priority
scheduling is solving a problem the kernel layer does not have.** What *is* real in dowiz's workload is
a **latency-class distinction**: request-path work that must return in bounded time (courier-dispatch
decision, order transitions) vs deferrable background work (telemetry drain, index maintenance,
backups). The right-sized adoption for that is a **cooperative per-path execution budget/deadline** —
which the kernel *already* has the primitives for (`budget.rs`, the fuel pattern) and the roadmap
already scopes the temporal-partitioning version of via **item 11** (ARINC-653-style two-level
scheduler, design-only, PURSUE). **Verdict: preemptive RTOS scheduling + priority inheritance = NO
(no preemptive/inversion surface, F2/F5); a cooperative request-path-vs-background execution-budget
discipline = YES and already scoped (budget.rs + fuel + item 11).** WCET tooling stays out (roadmap
items 47/48); bounded-loop source-structure assertions are the substitute.

### 4(c) Journaling-filesystem techniques (checksummed, copy-on-write metadata)

**What dowiz has (GROUNDED):** the FDR ring (A/B append-only segments + per-record CRC32 + torn-tail
detection, kill-9-safe — prior pass §1), the event_log (append-only SHA3-256 hash chain,
content-addressed — prior pass §2), `hub_supervisor` epoch snapshot (a log-position pointer, not a
state dump). Item 49 parks the Hybrid/LSM (WAL + periodic snapshot) design behind a measured
replay-budget trigger.

**Fit:** the genuinely-useful journaling-FS subset — (i) **checksummed records**, (ii) **atomic append
with torn-write detection**, (iii) **recovery that skips corrupt tails** — is **already captured** by
FDR (CRC32 + torn-tail) and event_log (SHA3 chain). Two ext4/btrfs/ZFS techniques are *not* yet
captured, and only one is a real gap:
- **COW + atomic-pointer-swap commit to bound replay** — the grounding was explicit that FDR is a
  Sequential Append-only Log, *not* pointer-swap, and the durable EventLog has genuinely unbounded
  replay. But **item 49 already parks exactly this** (WAL+snapshot behind a measured trigger; the
  carried-forward correctness note "data-file fsync strictly before pointer swap" is the btrfs/ZFS
  ordering rule verbatim). Nothing new to propose.
- **Periodic background scrub** (ZFS-style re-verification of *at-rest* checksums to catch latent bit-rot
  before a read needs the data) — this is a **small genuine gap**. Item 54 Sentinel does the *live-struct*
  analog; event_log/backup verify at *walk time*; but no periodic proactive scrub of the durable log
  exists. On non-ECC local storage this is a proportionate defense-in-depth. **PROPOSED:** a cheap
  scrub pass that walks the durable EventLog/FDR segments on an idle cadence, re-verifies CRC/SHA, and
  emits an FDR `Alarm` on any latent mismatch — reusing the *existing* CRC32/SHA3 (no new primitive).
  **Slots into:** gated on item 2's durable-store wiring fix (scrubbing an unwired store is pointless),
  composes with item 54 (same integrity-alarm seam), and reuses item 49's measurement discipline.
  **Verdict: the useful journaling subset = ALREADY CAPTURED (FDR + event_log); COW-snapshot =
  already parked (item 49); periodic scrub = a small real gap worth a proportionate item, gated on the
  item-2 wiring fix.**

### 4(d) Capability-secure init system (dependency-ordered startup + declared resource limits)

**What dowiz has (GROUNDED):** flat, ad-hoc composition roots. `native-spa-server/src/main.rs` wires
`ApiState::build_default()` → `build_router` → `serve` with no declared init-order graph, no per-module
resource declaration, no staged fail-closed bring-up. `agent-loop` and the kernel bins (`lm.rs`,
`markov_attractor.rs`) are similarly flat. Item 2 **proved** the concrete consequence: **no production
composition root constructs the durable event store at all** — every `FileEventStore::open` is
test-only, so the "genuine hash chain" evaporates on process exit in production. `budget.rs` declares
one resource ceiling for one module; nothing coordinates init order or resource declaration across
modules.

**Fit — this is the strongest-fitting new pattern of the whole pass.** systemd/F´/cFS init systems
solve a problem dowiz *demonstrably has*: modules with **real ordering and capability dependencies** at
startup. The durable EventLog must exist before `decision/import` can persist; the capability roots
(`capability_cert` self-signed roots) must load before `verify_chain`; the FDR must be *recovered*
(read back last moments — synthesis §5) before normal operation begins. Today these are implicit in
whatever `main()` happens to wire, and item 2 shows the wiring is actually **missing** for the most
consequential one. A **declarative, dependency-ordered composition root** is a direct, proportionate
fix:
- **(i) explicit init order** derived from a declared module-dependency graph (the same DAG discipline
  `order_machine.rs` already proves over — `has_cycle`/`topological_order` — reused for *module*
  startup ordering, so a cyclic init dependency is a compile/startup-time caught error, not a runtime
  surprise);
- **(ii) each module declares the capabilities/ports it requires**, and **fails closed** if a required
  capability is absent — generalizing the existing `isolation/microvm.rs` "refuse the adapter if the
  host capability is absent" pattern from deployment gating to *module init*;
- **(iii) the composition root is the sole minter of the §1(c) in-process capability tokens** — which
  makes (4d) and (1) one coherent design: the init system is *where* capabilities are granted, and the
  typed-capability boundary is *what* it grants. (This is precisely seL4's "the init task holds all
  capabilities and delegates" pattern, sized to a single process.)

This directly closes item 2's finding and the synthesis's standing "no production composition root"
gap, and it is where F´'s typed-component-topology and cFS's app-init model genuinely map onto dowiz.
**Slots into:** subsumes/formalizes the item-2 fix, composes with §1(c) and item 45, and gives item 48's
FDR-recover-before-normal-operation a declared place to live. **Verdict: capability-secure declarative
dependency-ordered init/composition root = YES, genuine gap (item 2 is the proof), proportionate,
strong fit — arguably the single most valuable new adoption in this pass.**

---

## 5. Summary table (fit verdicts, GROUNDED architecture basis, roadmap slot)

| # | Pattern | Fit | Basis | Roadmap slot |
|---|---------|-----|-------|--------------|
| 1 | seL4 capability isolation | **NO full / YES subset** | F1 (single-process library) makes MMU capabilities a category mismatch; Rust+WASM+feature-gates already deliver the single-process equivalent | extends item 45 + fuel; **new slice**: typed in-process capability reusing `capability_cert` |
| 2 | OTP supervisor tree | **NO in-kernel / adopted externally** | F2/F3 (no actor substrate); insight already at process/deploy layer (items 48, hub_supervisor) | no new in-kernel item; small: uniform per-port fail-closed (composes item 9) |
| 3 | TMR (compute + vote) | **NO general / YES narrow temporal** | µs-scale pure critical fns (money/FSM/event-id); item 54 covers at-rest, NOT compute-time flips; non-ECC target | refines **item 12** to *temporal* triple-run; pairs with item 54 |
| 4a | Message-passing IPC bus | **NO** | F4 (ports already are the discipline); F1/F2 (no async/multi-process to justify a bus) | none; capability-per-call = same slice as §1(c) |
| 4b | RTOS priority scheduling / inheritance | **NO preemptive / YES cooperative budget** | F5 (no preemptive/inversion surface); latency-class split is real | already scoped: budget.rs + fuel + **item 11** |
| 4c | Journaling-FS techniques | **CAPTURED + one small gap** | FDR CRC + event_log chain capture the subset; COW-snapshot parked (item 49) | **new small item**: periodic scrub, gated on item 2 wiring |
| 4d | Capability-secure init system | **YES — strongest fit** | Item 2 proved no production composition root; real init-order + capability deps exist | **new item**: declarative dependency-ordered + capability-declaring + fail-closed composition root; subsumes item-2 fix, unifies with §1(c) |

---

## 6. The 2–3 strongest cases for real adoption

1. **(4d) Capability-secure declarative init / composition root** — the standout. It is the only
   pattern here backed by a *proven* concrete defect (item 2: the durable store is never constructed in
   production), it fits dowiz's real architecture cleanly (a DAG of module init dependencies the kernel
   already has the proof machinery to validate), and it unifies with the seL4 capability slice (the init
   root is the sole capability minter). Highest value-per-effort.
2. **(1) The typed in-process capability boundary around the AI/agent subsystem** — the correct,
   proportionate reading of seL4 for a single-process kernel. ~70% already scoped (item 45 + fuel); the
   new slice (an unforgeable in-process capability the AI path must present to touch a core port,
   reusing existing `capability_cert` machinery) is small, zero-dependency, and hardens the one place
   less-trusted non-deterministic code enters the kernel's address space.
3. **(3) Temporal TMR on the cheapest-most-critical pure functions** — the one honestly-additive
   redundancy slice over the already-adopted integrity detection (item 54). Cheap because the targets
   are µs-scale pure functions, genuinely additive because it guards *compute-time* transient flips that
   at-rest checksums structurally cannot see, and well-matched to the confirmed non-ECC consumer target.
   Already scoped as item 12; the contribution is naming *temporal* (single-core sequential) as the
   right-sized form and pairing it with item 54.

Everything else is either a category mismatch dowiz's architecture rules out (full seL4, in-kernel OTP
tree, IPC bus, preemptive RTOS scheduling) or already captured/parked in the existing roadmap
(journaling subset, cooperative budgets, item 49's COW-snapshot park) — with periodic scrub (4c) the
one small residual gap worth a proportionate item.

---

## Index of primary citations (all paths absolute, `main` unless noted)

- `/root/dowiz/kernel/src/ports/mod.rs`, `/root/dowiz/kernel/Cargo.toml` (`[features]`),
  `/root/dowiz/kernel/src/budget.rs`, `/root/dowiz/kernel/src/core_pinning.rs`,
  `/root/dowiz/kernel/src/money.rs`, `/root/dowiz/kernel/src/router.rs`,
  `/root/dowiz/kernel/src/retrieval/memory_store.rs`, `/root/dowiz/kernel/src/decision/` (import.rs, mod.rs)
- `/root/dowiz/agent-loop/src/lib.rs`, `/root/dowiz/agent-loop/src/service.rs`
- `/root/dowiz/engine/src/lib.rs`
- `/root/dowiz/tools/native-spa-server/src/main.rs`
- Prior grounding (GROUNDED there, cited not re-verified): `docs/design/RESEARCH-CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-2026-07-19.md`
  (§4 capability_cert=authority-tokens, §5 hub_supervisor=deploy-rollout, §6 no TMR, §1 FDR mechanism, §9 no WCET)
- Roadmap slots: `docs/design/SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (items 2, 9, 11, 12,
  40, 45, 47, 48, 49, 54), `docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` (§3
  F´/fuel, §6 SIHFT caveat)
- Absent (GROUNDED DOES NOT EXIST): any `courier`/`dispatch`/`scheduler` module, any
  priority/deadline (RT)/preempt construct, any in-kernel actor/thread-pool/message-channel, any
  redundant-compute-vote, any dependency-ordered init graph, `kernel/src/fdr/` (worktree-only, absent on `main`).
