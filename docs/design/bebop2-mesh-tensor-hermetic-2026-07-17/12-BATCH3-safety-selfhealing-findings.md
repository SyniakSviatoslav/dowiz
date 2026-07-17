# BATCH 3 — Safety / Self-Healing Cluster — Research + Audit Findings (2026-07-17)

> Cluster: the safety / self-healing / fault-isolation layer of the Bebop2-mesh dialogue.
> Method: every concept from the dialogue was checked against live local code (dowiz `kernel/`,
> `engine/`; `/root/dowiz-agentic-mesh/`) and against the repo's own two standing doctrines — the
> seven Hermetic Principles (`hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`)
> and the Phase-27 fault-isolation blueprint
> (`BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md`, PROPOSED same day).
> Style contract (inherited): plain evidence-grounded prose, no metaphor; every load-bearing claim
> carries a `file:line` cite or an explicit **(proposal)** / **(stale-memory)** / **(gap)** tag.
> This is a research+audit artifact only — **no blueprint is written here** (per the task brief).
> Verdict vocabulary: **ALREADY-DOCTRINE** (repo already does this, cited) · **REINFORCES**
> (dialogue independently corroborates an existing rule) · **EXTENDS** (adds something real not yet
> built/named) · **CONFLICT** (genuinely contradicts a standing rule). Complexity is never a
> rejection reason — only physics/correctness is.

---

## 0. Headline — the dialogue's safety philosophy IS this repo's own verdict, already written down

The dialogue closes on a thesis: **hard-coded physical/structural invariants beat probabilistic
"ethical bureaucracy" (RLHF-style remembered rules) for safety, because physics is deterministic and
energy-cheap while rule-following is probabilistic and adversarially bypassable.** The task asks
whether this ALREADY IS / REINFORCES / EXTENDS / CONFLICTS with the repo's Hermetic doctrine.

**Answer: it is the repo's own §4 verdict, verbatim in substance.** The Hermetic synthesis already
concluded (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:315-317`): *"the in-process Rust core largely
**earns** the principles; everything that must survive beyond a single process — across time, across
the author/verifier divide — mostly **aspires** to them."* The line it draws is exactly the
dialogue's physics-vs-bureaucracy line: what the **type system / arithmetic** enforces is
deterministic and unbypassable (physics — earned); what depends on a **remembered ritual** is
probabilistic and forgettable (bureaucracy — aspired). The single sharpest instance is Hermetic
RC-2/Rhythm Finding 5 (`:184-188`): the "MANDATORY" 2-question doubt ritual has **zero firing
mechanism** (hooks empty, all no-op) — a rule that exists only as a probabilistic memory, precisely
the failure mode the dialogue predicts for rule-based safety. Meanwhile Polarity (P4), Cause-and-
Effect (P6), and Gender (P7) are earned in code *because* they are compiler-enforced poles, not
policies.

So: **ALREADY-DOCTRINE + strongly REINFORCES.** The dialogue arrives independently at the Hermetic
Principle of Polarity and at "structural constraint the generator cannot relax" (P7 §1.1). It
**EXTENDS** the doctrine in exactly two ways worth building toward: (a) it names the operator's
own **three-way Self-Healing / Self-Termination / Snapshot-Re-entry split** as a taxonomy the repo
implements unevenly (§6 below), and (b) it elevates **"physical invariant via WASM sandbox / bounded
numeric type"** to a first-class design category the repo does in code but has not named as a rule
(§7). **No genuine CONFLICT** was found; one watch-point is recorded (§6, self-healing must never be
read as license to dissolve the hard boundaries).

---

## 1. Circuit breakers (per-neighbor trip state)

**Verdict: EXTENDS — already fully DESIGNED (Phase 27), not yet BUILT; the per-neighbor variant is
gated on a seam that does not exist yet.**

- The named primitive `kernel/src/breaker.rs` **does not exist** (verified: `ls` → no such file;
  kernel module list has no `breaker`). So as *running code*, circuit-breaking is absent everywhere —
  corroborating Phase 27's own audit line "**no circuit-breaking exists anywhere**"
  (`BLUEPRINT-FAULT-ISOLATION-...-2026-07-17.md:249`).
- But it is **completely designed** in Phase 27 §3.2 as a `CircuitBreaker` sibling of `TokenBucket`:
  `Closed/Open/HalfOpen`, EMA trip via `geo.rs::ema_next`, a `min_calls` floor so a small sample
  cannot trip, `open_cooldown_ms`, and `probe_successes` hysteresis to close
  (`...FAULT-ISOLATION...:337-360`). The **per-neighbor / per-peer** trip state the dialogue asks for
  is item-for-item in the §3.3 bulkhead table: *"per-peer `CircuitBreaker` in the transport with
  policy from `HubPolicy` fields"* (`:392`), explicitly deferred to Wave F3 because it *"needs P9/P10
  work to exist"* (`:471`).
- **Epistemics:** the dialogue adds no new mechanism here — it independently corroborates a design
  the repo produced yesterday from a decorrelated audit. That corroboration has value (two
  independent derivations of the same primitive), but this is **not a new idea**.
- **Real gap (not complexity):** `breaker.rs` (Wave F1a) is buildable now against std only, zero deps
  (DECART already ADOPT, `:451`); the per-neighbor version is genuinely blocked on the P9/P10 mesh
  transport seam being real. The blocker is a missing *seam*, not difficulty.

## 2. Distributed watchdog / hardware hard-stop

**Verdict: SPLIT — the compute-watchdog and teardown-watchdog are BUILT; the restart-intensity
supervisor watchdog is a real designed-not-built GAP.**

- **Compute hard-stop = BUILT (and it is a genuine hardware-enforced physical invariant).** The WASM
  fuel loop terminates a compute-bomb / infinite-spin guest with a typed `BudgetExceeded`, never
  resumed, never queued (`/root/dowiz-agentic-mesh/agent-adapters/src/fuel.rs:91-109`), and the real
  path is a wasmtime `OutOfFuel` trap under `Config::consume_fuel(true)`
  (`fuel.rs:155-208`, `wasmtime-fuel` feature). This is exactly the dialogue's "hardware hard-stop":
  the guest is physically stopped by fuel exhaustion, not by a supervisor deciding to poll-and-kill.
- **Teardown watchdog = BUILT as a port obligation.** `JobPort::teardown` is documented as a
  *"mandatory-teardown watchdog hook so a scale-to-zero job cannot bill indefinitely if the caller
  drops it"* (`kernel/src/budget.rs:74-76`).
- **Operator hard-stop = the M9 kill-switch**, the single acknowledged hard stop
  (`kernel/src/hydra.rs:9,74,286`; `event_log.rs:383` "The only hard stop remains kill-switch").
- **Real GAP (restart-intensity / OTP MaxR·MaxT):** Phase 27 §2.1 finds *"no restart-intensity policy
  anywhere — a crash-looping drainer relaunches forever with no MaxR/MaxT escalation"*
  (`...FAULT-ISOLATION...:217`), designed but unbuilt in §3.4 (`:397-404`, and flagged **(unverified
  whether a systemd unit exists)**). This is the one part of "distributed watchdog" that is neither
  built nor gated on a missing seam — it is buildable now. Complexity is not the reason it is absent.

## 3. Mesh Panic Handler — isolate ONE node without stopping the swarm (vs monolithic kernel panic)

**Verdict: ALREADY-DOCTRINE (built) — the dialogue REINFORCES a panic-discipline finding Phase 27
already made; one honest internal tension surfaced.**

- The **isolate-one-node** mechanism is live as `OrganismState::Locked`. A node that detects core
  tamper (baseline spectral radius shifted to ρ≥1) fails **closed** to `Locked` and refuses commits,
  but the process does **not** stop and stays owner-visible (`hydra.rs:75-79,180-200`). It then
  broadcasts an unforgeable `BreachAlert` (node_id + group_size only, no code) to the hub
  (`hydra.rs:287-318`), and peers durably ingest and converge on the compromise
  (`ingest_peer_breach`, `hydra.rs:332-348`). **One node isolates + warns; the swarm continues** —
  this is the dialogue's Mesh Panic Handler, already implemented as a state transition rather than a
  process abort.
- The **bulkhead that makes "one error can never propagate" structurally true** is the
  process-per-hub tenant boundary (Phase 27 §1.1, citing `DELIVERY-FLOWS...:§4`: zero tenant fields
  in kernel types; M5/M10 make the boundary a process boundary — MMU-level isolation). Phase 27 §2.1
  states the discipline outright: *"process isolation, not `catch_unwind`, is the reliable boundary"*
  (`...FAULT-ISOLATION...:218`), because `catch_unwind` misses `panic = "abort"`. This is precisely
  the dialogue's contrast (isolate one node ≠ monolithic kernel panic).
- **Honest tension (not a conflict):** the same `hydra.rs` module that implements graceful `Locked`
  isolation *also* contains a hard `assert!` panic in `boot_verify` on baseline corruption
  (`hydra.rs:258-263`) — a process-halting panic. This is defensible under the operator directive it
  cites (*"re-seed from golden, not endure … kill-switch is the only safe stop"*, `:262`): a
  tamper-corrupted baseline **should** halt rather than endure. But it is only safe *because* the
  process-per-hub model means that panic isolates one node. Phase 27 independently flags the broader
  class — exported-API panics (A10, `causal.rs:839`), `FileBlockStore::put` panics (A2,
  `backup.rs:198,209,217`), `ParticlePool::new(0)` (A16) — as the exact "monolithic panic crossing a
  boundary" anti-pattern to remove (`...FAULT-ISOLATION...:82-90,143-147,167-168`). **The dialogue's
  concern is already a ranked finding set; it reinforces, adds nothing new.**

## 4. Survival Mode / graceful degradation to a Static Safe Tensor (read-only fallback)

**Verdict: ALREADY-DOCTRINE — implemented THREE independent times; the dialogue REINFORCES and offers
a unifying name ("Static Safe Tensor") the repo lacks.**

The concept — on failure, fall back to a pinned last-known-good read-only state rather than accept a
divergent one — exists in three planes:

1. **Kernel drift-gate (correctness plane):** an `Unstable`-spectrum mutation is rejected
   pre-persist; the organism *"endures by NOT persisting"*, leaving the prior topology intact
   (`event_log.rs:389-419`, the reject message at `:410-411`). The header comment names the doctrine
   literally: *"Survival = endurance, not exclusion"* (`event_log.rs:383`). The last-good tensor is
   the fallback; the bad mutation never lands.
2. **RCI dev-plane (advisory plane):** the STALE-not-wrong ladder — on git-subprocess hang, corrupt
   cache, or lock contention, *"previous snapshot kept, marked `STALE`; hooks pass (fail-open)"*
   (`realtime-change-intelligence-2026-07-17/proposal.md:395-400`, §7 table). A degraded organ serves
   the old read-only snapshot; it never blocks the plane above it. This is the exact "read-only
   fallback state."
3. **Organism Locked state (isolation plane):** refuse writes, remain readable via `state()`
   (`hydra.rs:197-200`) — the safe-tensor pattern applied to trust rather than dynamics.

**Epistemics:** the mechanism is real and triply-instanced. The dialogue's contribution is
vocabulary — "Static Safe Tensor" is a cleaner name than three unrelated descriptions — plus the
observation that all three share one shape (keep last-good, mark degraded, stay readable, never
render "degraded" identical to "healthy" — which is Hermetic P4 Polarity, `:66-78`). **REINFORCES.**

## 5. Hysteresis to prevent oscillation between panic / normal modes

**Verdict: EXTENDS — a genuine small gap. Hysteresis is DESIGNED for the breaker and EXISTS for
admission, but the live `integrity_check` Live↔Locked flip has NONE and can flap near ρ=1.**

- No `hysteresis` token appears anywhere in `kernel/src` or `engine/src` (grep → zero hits).
- It is **designed** for the future breaker: `probe_successes` consecutive HalfOpen successes required
  to close (`...FAULT-ISOLATION...:345,359`), and §2.3 notes *"hysteresis (trip fast, close only after
  k probe successes) … the same shape as P25's admission hysteresis"* (`:281`) — so an admission-side
  hysteresis already exists in the P25 lane (not in this cluster's files).
- **Real gap (the dialogue's idea directly applies):** `Hydra::integrity_check` flips
  `Live`↔`Locked` on the **instantaneous** predicate `rho < 1.0 && rho.is_finite()`
  (`hydra.rs:186-193`) with **no hysteresis band** — a baseline whose ρ dithers around 1.0 would flap
  the organism between refusing and accepting commits every check. The drift-gate uses a ρ>1+ε margin
  (`event_log.rs:406`, `classify_drift`), which is a *single* threshold with a tolerance, **not** true
  hysteresis (no separate trip-up vs release-down thresholds). Adding a two-threshold band (Lock at
  ρ≥1+ε_hi, release only at ρ≤1−ε_lo, or N consecutive healthy checks) is the dialogue's hysteresis
  applied to the one live state machine that needs it. Small, real, buildable now.

## 6. The operator's three-way split — does "degrade-closed" already implement it?

The operator's own synthesis (not the AI's suggestion) split recovery into three organs, each
explicitly **not a supervisor**: **Self-Healing** = emergent property of redundant/error-correcting
math · **Self-Termination** = a hard invariant boundary · **Snapshot Re-entry** = the recovery path.
The task's central question: does the existing **degrade-closed** doctrine already implement this, or
is there a real gap? **Answer: the split is real and the repo honors the "no supervisor" constraint
everywhere — but the three legs are implemented very unevenly.**

### 6a. Self-Termination as a hard invariant boundary — **ALREADY-DOCTRINE, deeply. This IS
"degrade-closed."** (confirms task item #4)

"Degrade-closed" is the repo's exact phrasing for Self-Termination-as-hard-invariant, and it is
enforced by *type/arithmetic*, never a polling supervisor:

- `ComputeBudget::debit` refuses past the ceiling and records **no** spend; `BudgetedJobPort::submit`
  returns `Err(BudgetExceeded)` before any spend; `OfflineJobPort` returns `Err(Offline)` — never a
  fake `Ok`/`JobHandle` (`budget.rs:110-118,149-160,178-183`, doc `:14-17`). The module doc calls
  degrade-closed *"the load-bearing word"* (`budget.rs:14`).
- `BoundedDrainer::tick` — *"cannot pay → stop, do not run unpaid work"* (`bounded_drainer.rs:70-82`,
  doc `:9-11`).
- fuel loop — compute bomb TERMINATED, *"refusal, never silent throttling, and NO fuel is ever loaded
  again after the refusing acquire"* (`fuel.rs:8-10,94-97`).
- H1 event-log — a durability fault is a typed `Err(StoreError)` with in-memory state unadvanced
  (`event_log.rs:293-312`, the `insert?` short-circuits before `set_tip`; verified live in
  `BLUEPRINT-H1-event-log-fail-open-fix.md:220-255`).
- drift-gate — an unstable mutation is a Law-pole reject, pre-persist (`event_log.rs:401-416`).

**The crucial property is that "proceed anyway" is not representable** — the `Result` type and the
`try_acquire → bool` gate make the unsafe path fail to compile / fail to execute, rather than being a
policy a supervisor could forget to enforce. This is precisely the dialogue's "hard invariant, not a
supervisor" **and** its "hard-coded physical invariant beats runtime policy." **The degrade-closed
doctrine fully implements the Self-Termination leg. CONFIRMED, not refuted.**

### 6b. Self-Healing as emergent (error-correcting) math, not a supervisor — **PARTIALLY-DOCTRINE.**

- **Real emergent healing that exists:** (i) spectral contraction — the drift-gate guarantees ρ<1, so
  the organism's dynamics *mathematically decay* perturbations rather than amplify them
  (`hydra.rs:172-195`, `spectral.rs` `classify_drift`); (ii) idempotent content-addressed replay — a
  lost or duplicated event is a *structural no-op*, so the log self-corrects on replay
  (`event_log.rs:349-352` duplicate short-circuit; `boot_verify` replay `hydra.rs:253`); (iii)
  `integrity_check` auto-restores `Live` from `Locked` when ρ returns <1 with **no external
  supervisor** (`hydra.rs:187-190`) — healing emergent from the spectral measurement itself; (iv) the
  fail-open advisory ladder self-heals at the dev plane (dead organ → STALE → next derive catches up,
  no supervisor; RCI §7).
- **Gap:** true *redundant / error-correcting-code* healing (N-of-M redundancy, ECC, Reed-Solomon
  reconstruction) is **absent**, and the **topological** self-heal is unbuilt: Hermetic finding #26
  (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:298`) records that mesh-node has **no topology primitive** and
  **M7 heal (Dijkstra/Union-Find reconnection) is unimplemented** (`ARCHITECTURE.md` M7/F45/F46).
  So "self-healing as a property of redundant math" is real for the *dynamical* and *replay* axes and
  a genuine gap for the *topological / redundancy* axis. **PARTIAL.** (Note the dialogue's "self-
  healing without a watchdog as emergent flow topology" must **not** be read as license to remove the
  hard boundaries in 6a — the operator's own synthesis already forecloses that by making Self-
  Termination a *separate* hard organ. This is the one watch-point, not a conflict.)

### 6c. Snapshot Re-entry as the recovery path — **PARTIALLY-DOCTRINE; the durable half is a real GAP.**

- **Works in-process:** `boot_verify` replays the WORM log after restart and re-checks invariants
  (`hydra.rs:247-265`); RCI cold-re-derives the whole projection from git+transcripts on any
  `.rci/` corruption (`proposal.md:397`, <10 s); the golden re-seed is the corruption path
  (`hydra.rs:262`).
- **Gap (durable snapshot + restore-drill):** Hermetic finding #4 (HIGH) — the COLD backup
  restore-drill has **never run**; no restore-verify subcommand exists
  (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:276`). Phase 27 A6 — every append-only store
  (`MemEventStore`, `FileEventStore`, `KnowledgeSpine`) grows forever with **zero compaction /
  snapshot / retention** (`...FAULT-ISOLATION...:114-126`); durable snapshot semantics are owned by
  P12 and not yet built. So Snapshot-Re-entry is designed and works for in-memory replay, but its
  *durable snapshot + verified restore* half is unbuilt. **PARTIAL.**

**Summary of the three-way split:** the "no supervisor" architectural constraint is honored on all
three legs (nothing here is a bolted-on watchdog). **Self-Termination is fully built** (= degrade-
closed). **Self-Healing** is real on the dynamical/replay axes, a gap on the topological/redundancy
axis (M7). **Snapshot Re-entry** is real in-memory, a gap on the durable/restore-drilled axis (P12 +
Hermetic #4). The dialogue's contribution is to make the three legs an **explicit taxonomy** so the
two partial legs are finished deliberately rather than drifting — that is the genuine EXTENSION.

## 7. Hard-coded physical invariants as Rust type/API constraints (WASM sandbox, bounded numeric ranges)

**Verdict: ALREADY-DOCTRINE (sandbox + fuel BUILT) + one stale-memory correction + one real EXTENSION
(numeric bounds at the boundary).**

- **WASM sandbox as a physical invariant = BUILT.** `microvm.rs` types the two tiers so the unsafe
  path is unrepresentable: `SandboxTier::WasmComponent` (always accepted, no KVM dependency) vs
  `NativeProcessRequiresKvm`, and `register_adapter("native-process")` returns `Err(AdapterRejected)`
  on a host without `/dev/kvm` **with no unsandboxed fallback**
  (`/root/dowiz-agentic-mesh/kernel/src/isolation/microvm.rs:20-26,68-90`). The admission path
  enforces it (`ports/agent/admission.rs:423-429`, and the RED test
  `crit6_native_without_kvm_is_rejected` at `:679-694`). This is the dialogue's "physical invariant":
  the host *physically cannot* run un-isolated native code — a hardware fact, not a policy check.
- **Fuel metering = BUILT** (see §2): CPU is a physically-bounded resource via wasmtime fuel; a
  compute bomb is stopped by fuel exhaustion, deterministic and energy-bounded (`fuel.rs`).
- **Stale-memory correction (flag):** MEMORY records *"B1 step 7 (Wasmtime fuel wiring) as NEXT / not-
  yet-done."* This is **partly stale.** The fuel *primitive* + the *real* wasmtime-backed meter are
  built and tested today — `DeterministicFuelMeter` models wasmtime's `set_fuel`/consume/trap exactly
  and the real `WasmtimeFuelMeter` compiles+tests behind the `wasmtime-fuel` feature
  (`fuel.rs:112-152,155-208,251-263`). What remains genuinely unbuilt: (a) `FUEL_PER_UNIT` is a
  documented **B4-pending placeholder** (`= 100_000`, `ports/agent/admission.rs:50-56`) awaiting the
  criterion bench that pins it; (b) the fuel loop is **not yet wired into any agent-invoke execution
  path** — `FuelTrancheRunner::run` is exercised only by its own tests (grep: no non-test caller;
  `admission.rs` `admit` assigns a `SandboxTier` and mints a budget bucket but never invokes the fuel
  loop, because admission is a trust transition, not execution, `admission.rs:15-16,454-455`). So the
  accurate status is: **fuel sandbox primitive DONE; value-pinning + invoke-time wiring PENDING** —
  not "fuel wiring not-yet-done" wholesale.
- **Real EXTENSION (numeric bounds at the type boundary):** the dialogue's "bounded numeric ranges
  enforced at the type level" is only partially present. The physics core does it well — CFL stability
  assert before the integrator (Hermetic P3 A3, `field_frame.rs:55-68`), `DT_STABLE` pinned
  (`kernel/src/lib.rs:180`), drift ρ<1+ε. But Phase 27 A9 (`...FAULT-ISOLATION...:136-139`) flags
  `compose(scene, eq, w, h, steps)` iterating `steps` and allocating `w*h` with **zero clamps**,
  exposed straight at the `#[wasm_bindgen]` port (`engine/src/field_frame.rs:218-225`,
  `wasm/src/lib.rs:57-59`), and A16 (`ParticlePool::new(0)` panics). Clamping caller-controlled
  compute at the port (Phase 27 Wave F1c) is the dialogue's "bounded range as a hard type/API
  constraint" applied to the one boundary that lacks it. Buildable now; complexity is not the blocker.

---

## 8. Does the dialogue add anything genuinely NEW to the safety layer? (honest ledger)

| Dialogue concept | Status vs live repo | New? |
|---|---|---|
| Circuit breaker, per-neighbor trip | Designed in full (Phase 27 §3.2/§3.3), not built; per-peer gated on P9/P10 | No — corroborates |
| Distributed watchdog / hardware hard-stop | Compute (fuel) + teardown BUILT; restart-intensity designed-not-built | Partly — restart-intensity is a real unbuilt gap |
| Mesh Panic Handler (isolate one node) | BUILT as `Locked`+`BreachAlert`; process-per-hub bulkhead | No — reinforces existing finding set |
| Survival Mode → Static Safe Tensor | BUILT ×3 (drift-gate non-persist, RCI STALE, Locked) | No — adds a unifying *name* |
| Hysteresis (anti-oscillation) | Designed for breaker; **absent** on live `integrity_check` flip | **Yes — small real gap** |
| Self-Termination = hard invariant | = degrade-closed, fully BUILT (budget/drainer/fuel/H1/drift) | No — confirms |
| Self-Healing = emergent math | Real on dynamical+replay axes; **M7 topological heal unbuilt** | Partly — topological gap |
| Snapshot Re-entry | In-memory replay works; **durable snapshot+restore-drill unbuilt** (P12, Hermetic #4) | Partly — durable gap |
| Physical invariant via WASM sandbox | BUILT (microvm tiers + fuel); value-pin + invoke-wiring pending | No — corrects stale memory |
| Bounded numeric range at type boundary | Core does it; **wasm boundary A9/A16 unclamped** | **Yes — real gap** |

**Net:** the dialogue is ~80% corroboration of doctrine the repo already holds and has partly built,
and ~20% genuinely actionable — concentrated in four unbuilt items: (1) `integrity_check` hysteresis,
(2) restart-intensity supervision, (3) the two partial legs of the three-way split (M7 topological
heal; durable snapshot/restore), (4) numeric-range clamps at the wasm boundary. None of these is
rejected for complexity; each is a physics/correctness improvement.

---

## 9. Prioritized build-order list (smallest kernel abstractions first, per the operator's mandate)

Ordered by (dependency-freedom × leverage), smallest/lowest-level first. **This is a research
recommendation, not a blueprint** — each item still earns its own planning pass.

1. **`integrity_check` hysteresis band (§5).** Smallest, zero-dep, kernel-local. Replace the single
   instantaneous `rho < 1.0` flip (`hydra.rs:186-193`) with a two-threshold band + N-consecutive-
   healthy release. Closes the one live oscillation risk in the safety state machine. RED test:
   ρ dithering around 1.0 must not flap `Live`↔`Locked`.
2. **`breaker.rs` primitive (Phase 27 F1a, §1).** Buildable now, std-only, DECART already ADOPT.
   Pure `step()` core is table-testable; `probe_successes` gives the breaker the hysteresis item 1
   adds to `integrity_check` — same shape, one idiom. Unblocks per-adapter failure exposure.
3. **Numeric-range clamps at the wasm boundary (Phase 27 A9/A16, §7).** Clamp `w`, `h`, `steps` at
   the `#[wasm_bindgen]` port; reject zero-capacity pools at construction. The dialogue's "bounded
   type as physical invariant" applied to the one boundary that lacks it.
4. **Restart-intensity supervision (Phase 27 §3.4, §2).** The distributed-watchdog gap: MaxR/MaxT
   (systemd `StartLimitBurst`/`StartLimitIntervalSec` or a relaunch-counter file) so a crash-looping
   drainer stops relaunching and surfaces one Blocker line. Resolve the systemd-vs-lib.sh unknown
   first (Phase 27 flags it **(unverified)**).
5. **Fuel loop invoke-time wiring + `FUEL_PER_UNIT` pin (§7, B4 bench).** Connect the built fuel
   primitive to the real agent-invoke path once that path exists; pin the placeholder constant with
   the B4 criterion bench. (Primitive is done; this is wiring + calibration, not new construction.)
6. **M7 topological self-heal (Hermetic #26, §6b).** Land the mesh-node topology primitive +
   Dijkstra/Union-Find reconnection, sharing (or parity-binding) the hub's graph representation.
   Completes the emergent-self-healing leg's topological axis. Larger; depends on the mesh seam.
7. **Durable snapshot + restore-drill (P12 + Hermetic #4, §6c).** Compaction/retention/snapshot for
   the append-only stores, plus a `restore-verify` subcommand drilled on a timer. Completes the
   Snapshot-Re-entry leg's durable axis. Largest; owned by P12.

Items 1–3 are independent and parallel-safe (different files, no shared mutable state). Items 4–7
carry seam/ownership dependencies and are sequenced after.

---

## 10. Two-question doubt audit

**Q1 — what would make this cluster's read wrong?** The strongest risk is that I treated Phase 27
(PROPOSED, `...FAULT-ISOLATION...:3`) and RCI (RESOLVED-FINAL awaiting sign-off, `proposal.md:2`) as
*doctrine* when both are design artifacts not yet merged. I distinguished built-vs-designed on every
line (breaker absent, fuel present, M7 absent) by reading the actual code, so the "already built"
claims are code-grounded; but the "already designed" claims rest on documents the operator has not
ratified — if either blueprint is rejected, items 2/4 lose their design backing (the code gaps
remain real regardless). Second risk: I read the agentic-mesh fuel/admission code but did **not**
compile or run its test suite this pass, so "built and tested" rests on reading the tests, not
executing them (dowiz kernel H1 tests *were* independently run green per
`BLUEPRINT-H1...:249`, 367/422/49).

**Q2 — least-verified load-bearing claim?** That the fuel loop is "not wired into any invoke path" —
I grep-confirmed no non-test caller of `FuelTrancheRunner::run` and read `admit` end-to-end, but I did
not exhaustively trace every module in the agentic-mesh worktree for a future/partial invoke seam
(`mcp.rs`, `dispatch.rs` were listed but not fully read for a fuel consumer). If a wiring exists in a
path I did not open, §7's "invoke-time wiring PENDING" would be an under-claim. The direction of any
error is conservative (I claim *less* is wired than might be), not an over-claim of completeness.

---

## 11. Anu / Ananke check

**Anu (does it follow?):** every verdict derives from a specific `file:line` read, not from the
pattern catalog — "circuit breakers are good" appears as a reason nowhere; the *absence* of
`breaker.rs`, the *presence* of `fuel.rs`, the *unclamped* `compose` do. The physics-over-bureaucracy
thesis is mapped to the repo's own §4 verdict by quotation, not assertion
(`HERMETIC-ARCHITECTURE-PRINCIPLES.md:315-317`). The one place a formula could mislead — "self-healing
without a supervisor" — is guarded explicitly (§6b watch-point: it must not dissolve the §6a hard
boundaries), matching the operator's own synthesis rather than the AI collaborator's looser framing.

**Ananke (is the good outcome structural?):** the Self-Termination leg is structural today (compiler-
enforced `Result` poles — a swallow no longer compiles, `budget.rs`/`event_log.rs`/`fuel.rs`); the
WASM sandbox is structural (host physics + `register_adapter` refusal). The gaps named in §9 are gaps
precisely *because* they are not yet structural — `integrity_check` hysteresis is arithmetic not a
type, restart-intensity is a policy not a compiler invariant, durable snapshot is unbuilt. The build-
order deliberately puts the items that *become* structural (type/arithmetic-enforced: 1, 2, 3) before
the ones that remain policy/ops (4, 7). No governance friction was re-introduced (suspended per
operator directive 2026-07-15); these are all deterministic-gate improvements, not remembered rituals.

---

*Batch 3 complete. Research + audit only — no blueprint written, no code edited. The safety/self-
healing philosophy of the Bebop2 dialogue is the repo's own already-written Hermetic §4 verdict:
type-enforced hard invariants (physics) are earned; remembered rituals (bureaucracy) aspire. The
operator's three-way Self-Healing / Self-Termination / Snapshot-Re-entry split is honored on the
"no-supervisor" axis everywhere, fully built on the Self-Termination (degrade-closed) leg, and
partially built on the other two — the genuine actionable residue is four unbuilt items (§9),
none rejected for complexity.*
