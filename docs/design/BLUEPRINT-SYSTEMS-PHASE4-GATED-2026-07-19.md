> # ⛔ BLOCKED — IMPLEMENTATION FORBIDDEN UNTIL GATE G3 PASSES ⛔
>
> **Nothing in this document may be coded until the Phase 1–3 safety/telemetry gate (gate `G3`
> in `MASTER-ROADMAP-SWARM-SAFETY-TELEMETRY-FIRST-2026-07-19.md`) is measured-green.** This is
> "Blueprint B" — the gated forward plan. It is a **design artifact, review target, and
> threat-model input only** during Phases 1–3. It becomes a work order the moment `G3` passes and
> not one commit sooner. A "small Phase 4 spike" before `G3` is a roadmap violation, not
> initiative (roadmap §6). Operator directive, verbatim: *"first safety & telemetery should be
> planned & implemented, then intensive aggresive testing & injections against it — only after
> these first 3 steps, other can be coded."*

# Blueprint B — Phase 4 Systems: Fluid/Topology Routing, Boson Swarm Mechanics, Distributed Placement, Bayesian Audit

**Companion docs (authorities — this doc does not re-derive them):**
`MASTER-ROADMAP-SWARM-SAFETY-TELEMETRY-FIRST-2026-07-19.md` (sequencing & the gate) ·
`SWARM-SAFETY-DETERMINISTIC-CIRCUIT-BREAKER-SYNTHESIS-2026-07-19.md` (**Synthesis I** — the
breaker primitive and its five signals, which every component below plugs into) ·
`SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md` (**Synthesis II**) ·
`BLUEPRINT-TELEMETRY-SAFETY-2026-07-19.md` (**Blueprint A** — the Phase 1–2 build this depends on).

**Status legend.** Everything here is **PROPOSED**: designed, not built, and — by definition of
the gate — **unmeasured**. Phase 4 has produced zero measurements; do not borrow confidence from
Blueprint A's grounded claims. Where this doc cites a *built* precedent (`field_physics.rs`,
`TokenBucket::release`, the `research-verifier` pattern) that fact is grounded, but every Phase-4
*application* of it is PROPOSED until measured. **🧭 OPERATOR VISION** marks ideas originating in
the operator's own brainstorm, not external literature — carried as design intent, not silently
absorbed as generic architecture.

---

## §A — Fluid / Topology Data-Routing Layer

> ⛔ **BLOCKED until `G3`.** PROPOSED, unmeasured.

**🧭 OPERATOR VISION.** The operator's standing instruction: *"do not reject & miss my vision to
use mechanical deterministic approaches to direct & wrap over the stochastic data flow."* This
section is that vision in rigorous, falsifiable form — an n-dimensional embedding + graph
Laplacian, **not** the "2D algebra" the operator explicitly objected to, and **not** literal
Navier–Stokes/CFD (rejected in Synthesis I §2 as decorative for discrete token/packet routing:
too expensive, no measured precedent of beating graph-Laplacian/flow methods on discrete data).

**What it is.** One graph Laplacian operator `f(L)` unifies memory-recall, salience-decay,
UI-motion, and blur — proven, not analogous: Gaussian blur ≡ heat equation `e^{−tL}`
(Synthesis I §2, citing Chung 2007). The precedent is already built in the sibling bebop-repo:
the discretized wave equation `MÜ + ΓU̇ + c²LU = S` at `crates/bebop/src/field_physics.rs:334`
(`pub fn step_wave`, verified live 2026-07-19). `TokenBucket::release`
(`bebop-repo/bebop2/proto-wire/src/transport_policy.rs`) is already a discretized conservation-law
flow primitive: inflow − outflow = Δstored, the same continuity equation `∂ρ/∂t + ∇·(ρv) = 0`
underlying every fluid PDE, applied to a scalar reservoir. **The operator already has the
operator.** Phase 4 wires it into swarm routing.

Two buildable, measurable sub-components (both PROPOSED):

1. **Conservation-law backpressure accounting.** Model every queue/spool in the swarm (starting
   with the telemetry spool drainer in `tools/telemetry/lib.sh`) as a node with a measured
   inflow rate `λ` and outflow rate `μ`; assert Little's Law `L = λW` holds empirically. Trip
   backpressure when `Δstored` exceeds a bound fit from measured data. This is the cheapest
   Phase-4 experiment — a real, falsifiable number, not a moonshot.
2. **Graph-Laplacian routing.** Position each data stream as a node in an n-dimensional embedding
   space (reuse the Hilbert-space / inner-product machinery already used for embeddings), and
   route/mix streams via `e^{−tL}` diffusion instead of ad-hoc branching logic. **Deterministic
   because `L` is a fixed, auditable matrix**; the *content* flowing through the pipe stays
   stochastic. This is the literal engineering meaning of "wrap stochastic signaling around
   deterministic streams." `L` is dimension-free — the 2D constraint was only ever in a naive
   adjacency-as-spreadsheet mental model.

**Interface sketch.**
```rust
trait FieldRouter {
    /// Fixed, auditable topology. Hash is logged; any change is an audit event.
    fn laplacian(&self) -> &SparseSym;            // L over the stream-embedding graph
    /// Deterministic mix: out = exp(-t·L) · in. t is policy, L is fixed.
    fn diffuse(&self, streams: &StreamVec, t: f64) -> StreamVec;
    fn topology_hash(&self) -> Blake3;            // pinned per epoch; drift = Role-Mismatch event
}
struct Reservoir { inflow: RateEwma, outflow: RateEwma, stored: f64 }  // continuity check node
```

**Why gated behind Phase 1–3 specifically.** Routing decides *which agent sees which data*. If
the router silently re-mixes streams, the breaker's **behavioral-drift** and **cross-agent
disagreement** signals become uninterpretable — a drift spike could be a compromised agent or
just a topology change. The Laplacian's `topology_hash` must therefore be one of the monitored
inputs *before* routing goes live, and Little's-Law backpressure must feed the same spool the
Phase-2 telemetry pipeline drains. Ship routing before the breaker watches the topology and you
have an unauditable data plane wrapped around an auditable control plane — exactly inverted.

---

## §B — Mirror-Model Chain-of-Thought Observation + Thought Injection

> ⛔ **BLOCKED until `G3`.** PROPOSED, unmeasured.

**What it is.** A decorrelated observer — a *different model/provider* than the agent it watches —
pairs with every reasoning agent to summarize and flag that agent's chain-of-thought. This is
**not hypothetical**: the `research-verifier` agent in this session's own roster already runs
exactly this pattern (decorrelated adversarial verification on a different provider) for research
reports. Synthesis I §1.5 states the gap precisely: the work is not "build a mirror model" — it
is **wire the existing decorrelated-review pattern to swarm *runtime* actions, not just research
docs.** PROPOSED, not built.

**Thought injection.** Plant a synthetic canary fact into an agent's context and verify it is
handled correctly downstream in the CoT — a standard canary-token-equivalent technique, cheap to
build, cheap to measure. The measurable output is a **false-negative rate = canaries missed**.
A missed canary is direct evidence of CoT that is not actually reading its own context (hidden
data, hallucinated reasoning, or an injected instruction overriding the visible chain).

**Interface sketch.**
```rust
struct MirrorPair { agent: ModelHandle, observer: ModelHandle } // observer.provider != agent.provider
struct CoTVerdict { summary: String, flags: Vec<Anomaly>, canary_hit: bool, disagreement: f64 }
fn inject_canary(ctx: &mut Context) -> CanaryId;                 // synthetic fact, unique per turn
fn score(pair: &MirrorPair, trace: &CoT) -> CoTVerdict;         // decorrelated read of the CoT
```

**Why gated behind Phase 1–3 specifically.** `disagreement` from the mirror observer and
`canary_hit` are *inputs to the breaker*, not a parallel safety system. Synthesis I §1.1 already
names **cross-agent disagreement** as a breaker signal fed by shadow execution; the mirror
observer is the natural producer of that signal for CoT. Building the observer before the breaker
exists means its verdicts have nowhere to trip — you would be observing without a control action,
which the operator's control-theory framing explicitly rejects (measurement must drive a
deterministic response, not a report). The canary false-negative rate must also be part of the
Phase-3 red-team corpus (roadmap §3) so its threshold is *fit from attacks*, not guessed.

---

## §C — Swarm / Agent Mechanics ("Boson" architecture)

> ⛔ **BLOCKED until `G3`.** PROPOSED, unmeasured.

**🧭 OPERATOR VISION — the entire section.** The homogeneous stateless-agent ("boson") model and
every mechanism below originate in the operator's brainstorm (RAW-PROMPT Part 2, items 5–20), not
external literature. Synthesis I §1.5 lists them all PROPOSED.

- **Boson agents (homogeneous, stateless).** An agent's only identity is the capability token it
  currently holds; between roles it is interchangeable and amnesiac. Amnesia is enforced by Rust
  ownership: the scoped context is a `Drop`-based object, so role-exit auto-revokes it — no
  manual "clear state" step to forget.
- **Capability-token security.** Opaque handles, RAII-scoped context objects, and
  branded/unforgeable token *types* (a `Token<Signing>` cannot be passed where `Token<Ui>` is
  required — a type error, not a runtime check). Signing tradeoff, decided: **HMAC** (fast,
  shared-secret, in-process) for same-process delegation; **Ed25519** (asymmetric) across
  machines. Both already have precedent in the bebop crypto surface.
- **Role triad + bounded delegation.** Owner (root authority, issues policy) / Customer
  (requester, obtains a token) / Courier (stateless executor, presents token only). Recursive
  delegation is allowed but bounded by a **depth/TTL counter** carried in the sub-token, which
  prevents cycles and unbounded fan-out.
- **Orphan-on-supervisor-death.** When a supervisor's token can no longer be renewed, its
  subordinate couriers simply **stop** (chosen over handoff — the safer option for a homogeneous
  system, where any orphaned work is cheaply re-issued to a fresh boson rather than dangerously
  migrated with stale scope).
- **Leased/elected Steward (no permanent Arbiter).** Stewardship is a time-bound lease any peer
  can hold per domain; there is no global fixed arbiter to compromise. The event-sourced kernel's
  **monotone log sequence is itself the fencing token** — etcd-style TTL-lease semantics *without*
  importing etcd/Consul/Kubernetes (standing no-K8s decision). Detailed in §D.
- **Genesis bootstrap.** The first node holds a root token, delegates outward, then
  **self-relinquishes** it. Genesis is a startup role, not a permanent hierarchy.
- **Break-glass recovery — the sanctioned god-function. 🧭 OPERATOR VISION.** Shamir's Secret
  Sharing (`k`-of-`n`) reconstruction of a master recovery key + optional time-lock + immutable
  audit trail, implemented as **one deliberate monolithic "god function"** — the single explicit
  exception to the twelve-anti-patterns rule elsewhere in this system. The justification is
  inversion: god-functions are normally banned because they *sprawl* accidental complexity; this
  one is the opposite by construction — **one entry point, one audit surface, zero composition
  with normal code paths.** Making it monolithic is the security control, not debt. It sits behind
  the human-gated `KILLED` unlock (Synthesis I §1.4).
- **Non-fixed, randomly-assigned Auditor + shadow/dual execution (anti-Goodhart).** A fixed,
  identifiable validator gets gamed — agents learn to perform only when watched. Fix: any peer
  can be handed an Auditor capability for one task; validation is stochastic (random sampling, not
  fixed periodicity) and/or shadow execution (two workers run the same task, neither told which is
  "the check").
- **Context-switch security escalation.** Moving from a low-stakes domain (UI) to a high-stakes
  one (crypto/signing) **invalidates all prior capability tokens and wipes local scratch state**;
  a distinct **`Role-Mismatch`** audit-event type (not a generic "hallucination") fires on any
  attempt to reuse a low-domain resource or logic pattern inside a high-domain task.
- **Context Bridge.** An agent may *propose* a cross-domain idea (e.g., a UI code-quality pattern
  applied to crypto code) but a separate **Validator must approve it against the target domain's
  axioms before it is applied.** This separates beneficial knowledge-*transfer* (good) from
  dangerous runtime/state-*transfer* (bad — e.g., reusing UI retry/timeout logic inside a signing
  operation, which breaks constant-time guarantees).
- **3-layer open-source model (if ever published).** Open protocol / closed data / open
  governance — external review of the protocol and governance without exposing live secrets or
  handing attackers the anomaly-detector's rules.

**Interface sketch.**
```rust
struct Token<Domain> { handle: OpaqueId, scope: Scope, ttl: Deadline, depth: u8, sig: Sig } // branded
impl<D> Drop for ScopedCtx<D> { fn drop(&mut self) { revoke(self.token); wipe(self.scratch); } }
enum AuditEvent { RoleMismatch{from:Domain,to:Domain}, BreakGlass{shares:u8, klock:Deadline}, .. }
fn context_switch(old: Token<Low>, target: Domain) -> Token<High>; // old invalidated, scratch wiped
fn break_glass(shares: [Share; K]) -> RecoveryKey; // the one sanctioned god-function; human-gated
```

**Why gated behind Phase 1–3 specifically.** Every mechanism here *writes to the World Model or
issues authority*, and each such write is precisely what the breaker's **constraint-violation
count** signal (Synthesis I §1.1) must already be monitoring. Concretely: a compromised Steward
that reissues capability tokens is **undetectable** unless the constraint-graph predicate gate is
already counting and rejecting its anomalous writes — so Steward election cannot exist before the
gate watches the write path. `Role-Mismatch` events must route into the Phase-2 audit ring-buffer
(schema: `agent_id, role, node_id, timestamp, drift_score, context_snapshot`) or context-switch
violations vanish. And the break-glass god-function overrides live Stewards — it is the highest-
blast-radius action in the system, so it must sit behind the *already-built, already-tested*
`KILLED` human gate; building break-glass before that gate exists is building the master key
before the lock.

---

## §D — Distributed-Systems Implementation (decided; detail, not relitigation)

> ⛔ **BLOCKED until `G3`.** PROPOSED, unmeasured.

- **World-Model sharding.** Shard key = **geo/spatial cell** (the delivery domain is radius-shaped
  and spatially local, so co-located work lands on the same shard). Placement = **rendezvous (HRW)
  hashing** — a single pure function `argmax_node hash(key, node)`, auditable and deterministic,
  with minimal `1/n` key movement on membership change and no vnode tuning at this shard count.
- **Steward election.** etcd-style **TTL lease + fencing token** semantics implemented **over the
  existing event-sourced kernel log** — the monotone log sequence *is* the fencing token, so a
  stale Steward's writes are rejected by sequence order alone. **Not** an actual etcd/Consul/K8s
  dependency (standing no-K8s decision; Synthesis II §6).

**Interface sketch.**
```rust
fn owner(key: CellId, members: &[NodeId]) -> NodeId {           // HRW: pure, deterministic
    members.iter().max_by_key(|n| blake3(key, n)).copied().unwrap()
}
struct Lease { holder: NodeId, fence: LogSeq, expires: Deadline }  // fence = kernel log sequence
fn renew_or_expire(l: &Lease, now: Deadline, log_head: LogSeq) -> LeaseState; // stale fence => reject
```

**Why gated behind Phase 1–3 specifically.** The lease's `fence` *is* a kernel-log sequence, so
Steward election writes to the same World Model the breaker's **constraint-violation** signal
monitors; a split-brain (two Stewards, stale fence accepted) is only caught if that signal is
already live. HRW re-placement moves keys between shards — which perturbs **behavioral-drift** and
**cross-agent disagreement** baselines — so shard membership must be a monitored input before
placement is dynamic, or every rebalance looks like an attack.

---

## §E — Bayesian Reputation + Probabilistic / Random Auditing

> ⛔ **BLOCKED until `G3`.** PROPOSED, unmeasured. **🧭 OPERATOR VISION** (RAW-PROMPT items 13, 15,
> 16).

**What it is.** Trust is **never binary.** Each node carries a **Bayesian reputation** posterior;
low-reputation nodes get *heavier* verification, and **high-reputation nodes still get random
audits** so trust never becomes a blind spot (the operator named the "echo chamber" risk
explicitly). Reputation is fed by the *merged heartbeat + hallucination telemetry* the operator
specified — latency drift, confidence score, queue depth, constraint-violation count, cross-agent
voting, logical drift — and these feed **the SAME breaker primitive from Blueprint A** (do not
redesign it; cross-reference Synthesis I §1). The response is a **3-tier ladder** mapping directly
onto the breaker states: **warning → quarantine (read-only) → kill-and-replace**.

**Interface sketch.**
```rust
struct Reputation { alpha: f64, beta: f64 } // Beta posterior; audit_prob = f(mean, floor>0 always)
fn audit_probability(r: &Reputation) -> f64 { (BASE / r.mean()).clamp(FLOOR, 1.0) } // never 0
enum Tier { Warning, QuarantineReadOnly, KillAndReplace } // == breaker Closed-trip / Open / Killed
```

**Why gated behind Phase 1–3 specifically.** Reputation is a *weighting* on the breaker's
existing signals — it has no independent trip authority and no independent primitive. If it ships
before the breaker, it is a trust score with nothing to gate. Worse, a reputation system that
*replaced* rather than *fed* the breaker would let a high-reputation compromised node escape
scrutiny — exactly the echo-chamber failure the design forbids. The `FLOOR > 0` random-audit rate
must be validated against the Phase-3 red-team corpus, or "random audit" degrades to "no audit
for trusted nodes."

---

## §F — Dependency List: which Phase-1/2 breaker signals each Phase-4 component requires

> ⛔ **This table is what makes the gate real rather than arbitrary.** Each Phase-4 component is a
> *consumer* of a Blueprint-A breaker signal (Synthesis I §1.1) or audit primitive. Until that
> signal is built **and** measured-armed at `G3`, the consumer has nothing to plug into — so the
> ordering is a hard data dependency, not a policy preference.

| Phase-4 component | Depends on (Blueprint A / Synthesis I §1.1) | Failure if built before the gate |
|---|---|---|
| §A Laplacian routing | Behavioral-drift EWMA + cross-agent disagreement; `topology_hash` as a monitored input | Silent re-mix makes drift/disagreement uninterpretable — unauditable data plane |
| §A Backpressure | Constraint-violation count; Phase-2 telemetry spool (Little's Law `L=λW`) | Backpressure trips with no shared spool to drain into; no measured baseline |
| §B Mirror-CoT observer | **Cross-agent disagreement** signal (the observer is its CoT producer) | Verdicts with no breaker state to trip — observation without control action |
| §B Thought injection | Confidence gap; Phase-3 red-team corpus (canary FN-rate threshold) | Canary threshold guessed, not fit from attacks |
| §C Capability tokens / delegation | Constraint-violation count on the World-Model write path | Anomalous token writes uncounted — unforgeable in name only |
| §C Steward reissue | Constraint-violation count + audit ring-buffer | Compromised Steward reissuing tokens is undetectable |
| §C Context-switch / Role-Mismatch | Audit ring-buffer schema (`agent_id, role, node_id, timestamp, drift_score, context_snapshot`) | `Role-Mismatch` events have nowhere to land — violations vanish |
| §C Break-glass god-function | **`KILLED`-state human gate** (Synthesis I §1.4), unfiltered log ship | Highest-blast-radius action with no tested human gate — master key before the lock |
| §D HRW sharding | Behavioral-drift + disagreement baselines (membership as monitored input) | Every rebalance reads as an attack; baselines corrupt |
| §D Steward lease / fencing | Constraint-violation count over the event-sourced kernel log | Split-brain (stale fence accepted) uncaught |
| §E Bayesian reputation | **The entire breaker primitive** (§1) + merged heartbeat/hallucination telemetry (§1.1, all five signals) | A weighting with nothing to gate; high-rep compromise escapes — echo chamber |

**Breaker states every §E tier and §C kill path bind to (Synthesis I §1.2), unchanged, not
redesigned here:** `CLOSED → OPEN (quarantine/read-only) → HALF_OPEN (canary) → CLOSED`, and
`OPEN → KILLED`. Thresholds `θ_open`, `θ_kill`, `W`, `T` are **fit from measured FP/FN rates at
`G3`**, never picked by feel. The `KILLED` state for red-line-classed actions
(money/auth/RLS/migrations) never auto-resumes — human gate, unfiltered logs to the operator —
through every phase, Phase 4 included.

---

> # ⛔ REMINDER: BLOCKED — IMPLEMENTATION FORBIDDEN UNTIL GATE G3 PASSES ⛔
>
> This blueprint is complete as a *plan*. It is a review target and threat-model input during
> Phases 1–3 and nothing more. When `G3` produces its measured artifacts (roadmap §3: fitted
> thresholds, 1000/1000 bitwise-identical batch-invariant baseline, red-team TPR/FPR per signal),
> and only then, does §F's dependency list turn from a blocker into a build order.
