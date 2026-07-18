# BLUEPRINT P56 — Verification-Harness Infrastructure: the shared substrate for P54/P55 (test-matrix dims · extensible probe registry · P25-scheduled execution · content-addressed result chronology on Hetzner · trend layer · META-VERIFICATION of the tests themselves) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9; every point addressed).
>
> **This is Part 3 of a 3-part parallel effort.** Part 1 =
> `BLUEPRINT-P54-llm-agent-verification-harness.md` (LLM/agent-specific probes) and Part 2 =
> `BLUEPRINT-P55-protocol-ecosystem-testing.md` (protocol/chaos/property tests) are **written
> concurrently with this file and did not exist on disk when this pass ran**
> (`ls docs/design/CORE-ROADMAP-2026-07-17/ | grep -i "P54\|P55"` → empty, this pass). They are
> cited by expected name and scope; **cross-reference on next pass**, do not treat their absence
> as a gap. P56 owns the machinery both consume — the dimension model, the probe abstraction and
> registry, the scheduling adapter, the result store and its Hetzner sync, the trend layer, and
> the meta-verification layer. P56 does **not** own any specific probe or test.
>
> **Operator ask this blueprint answers (verbatim intent):** modular, cross-platform,
> multi-device, different environments/parameters (OS/CPU/GPU/network); never lose the ability to
> extend/change/reuse later; run promptless where possible and async/fast/in-waves where prompts
> are needed; separate storage with chronology, regression/improvement detail, saved metrics,
> pattern recognition (topological + chronological), signal-vs-noise; **checks on the tests and
> measurements themselves, catching erroneous conclusions of the instruments, not only their
> results**; feedback-loop-ready for self-improvement; native Rust; plus a dedicated Telegram
> research branch for ongoing testing/chaos-engineering literature.

---

## 1. Ground truth — every cite re-verified live this pass (standard §2 item 1)

| # | Claim | Fresh evidence (this pass) |
|---|---|---|
| 1 | The content-addressed chronological event-log pattern is real, tested, and reusable | `kernel/src/event_log.rs`: `sha3_256` (:30, pure-Rust FIPS 202), `MeshEvent{prev, actor_pubkey, actor_seq, payload}` (:134-143), `event_id()` = hash of the tuple (:148-155), `EventStore` trait (:182-205), idempotent `append` (:302), `verify_chain` read-back integrity walk + typed `ChainDefect` (:475-528). Registered `kernel/src/lib.rs:60` |
| 2 | A std-only **durable** `EventStore` already exists | `kernel/src/hydra.rs:920` `FileEventStore` (impl `EventStore` :1029) — offline, egress-free, no external DB |
| 3 | Statistical primitives for meta-verification already exist in-kernel | `kernel/src/stats.rs` (lib.rs:117): `wilson_interval(k, n, z)` (:100) — doc explicitly names the p̂=1.0 Wald collapse as "the self-certification failure in miniature"; exactly the flake-rate bound §4f needs |
| 4 | A deterministic, seedable, cross-platform PRNG exists | `kernel/src/rng.rs` (lib.rs:98): `Rng` (:33), `new(seed, stream)` (:45), SplitMix64 (:135); P-H audit (`P-H-audit-telemetry-regression-benchmarks.md:136`): "seeded injection schedules → findings reproduce from `(seed, plan)`, no wall-clock flake" |
| 5 | Benchmark convention: criterion + pinned baseline + threshold tracker | `kernel/Cargo.toml:83` `criterion = "0.5"`, bench target :92; `kernel/benches/baseline.json` (flat `name → mean-ns` map, 5 entries); `kernel/benches/bench_track.py` (`--threshold`, delegates to native `native-trackers` binary — "python is never the hot path", :4-9) |
| 6 | P25 wave scheduling exists and already defines the C/D/L class split + admission predicates | `docs/design/BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md`: C = 4 strict-core slots `taskset -c 0,2,4,6`, `nice 10` (§3.2/§3.3); D = `D_max = 16` gated by memory PSI (§3.4); L = Ollama's own governor, counted against C (§3.5); LOCAL-DECISION + CORE-BOUND binding rules (§3.1/§3.2); `kernel/src/admission.rs` is a **proposal, NOT built** (`ls kernel/src/admission.rs` → absent, this pass) — §3.6 prescribes "until GapWire exists, the lead agent applies the same table manually" |
| 7 | P45 alerting/severity/topic machinery exists (design) atop a live send mechanism | `BLUEPRINT-P45-ops-security-monitoring.md` §3: `Severity{S3Ledger,S2Digest,S1Warning,S0Critical}`, `BENCH_REGRESSION_PCT=10.0`, `BENCH_CONFIRM_RUNS=2`, `BENCH_RUNS_PER_SAMPLE=3`, routing table with NEW topics recorded-at-creation; §4b.3 the argued noise floor; §4e.1 message grammar. Live send: `tools/telemetry/lib.sh` `tg_send` (:92-141, `TELEGRAM_TOPIC_ID` default 267 at :118), `tg_deliver`/spool (:57-65), `log_event` (:69-84), `bench_run` (:168-194). Topics in use: 267/291/292/294 (`tools/telemetry/topics/src/main.rs:8-12`) + 257 Reports (P45 §1.1) + P45's three planned Ops topics |
| 8 | The Hetzner S3 remote is LIVE with the expected prefixes | `rclone lsf hetzner:dowiz` → `backups/ cold/ db/ images/` (run this pass); `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` §0: type `s3`, endpoint `fsn1.your-objectstorage.com`, 13.07 GiB present, credentials live |
| 9 | The closed-enum-but-extensible pattern is established precedent | P40 (`BLUEPRINT-P40-agent-loop-tool-wiring.md` §2): `ToolAction { Read }` — "P40 ships exactly one variant. P42 may extend"; P42 (`BLUEPRINT-P42-mcp-agent-skills.md` §2): `SkillCard` (≤ `MAX_CARD_DESCRIPTION_BYTES=200`), `SkillRegistry{cards, resolve}`, `StaticSkillRegistry` panics at construction on duplicate/oversized — malformed catalog unrepresentable at runtime |
| 10 | Baseline-first regression discipline is established | P44 (`BLUEPRINT-P44-cache-layers-scaleout.md` §4.1/§5): "baseline before any layer", every change "arrives with the pinned baseline it targets + a benchmark showing net win"; P45 §4b.3: median-of-3, 2 consecutive breaches, baseline refresh = explicit ledger-row act |
| 11 | The self-improvement loop seam this feeds is real | P32 (`BLUEPRINT-P32-hydraulic-loop-wiring.md` §0 row 1): `kernel/src/intake.rs` (779 lines, lib.rs:85) → `kernel/src/loops.rs:11` `use crate::intake::{admit, …}` — the intake→loops wiring template; `tools/loop-signals/` (Markov attractor, advisory/fail-open) exists on disk (this pass); memory `markov-attractor-loop-signal-2026-07-13.md` |
| 12 | This host is ONE point in the requested matrix | P21 §0 (re-verified against P25 §1.1): 8 vCPU = 4 physical × 2 SMT, AMD EPYC-Milan, AVX2 no AVX-512, 30 Gi RAM (0 B swap), **no GPU** (`nvidia-smi` absent), Linux 6.8 x86_64 |
| 13 | The stale-GREEN failure class is REAL and recurrent — three live worked examples | **P34** §0 row 12: mesh spine claimed "PROVEN"/"DONE", live `cargo test` RED (`E0004` — kernel gained `Refunding`/`CompensatedRefund` after the spine was written; "'Proven' was true when written, is false live"). **P36** §0 rows 2/5: `d23e7aa` broke the wasm32 no_std build AFTER `EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md:31-36` recorded GREEN — "the doc is currently lying about build state". **P06**: `key_V` done-gate still a `signed:false` stub while downstream docs treated it as landed (memory `harness-llm-backend-and-hermetic-remediation-2026-07-17.md`). Same class, smaller: DISK-OPS §0 — memory's "deep-clean tool + cronjobs" claim was stale, nothing scheduled |
| 14 | Chaos-harness mechanics belong to a sibling | `P-H-audit-telemetry-regression-benchmarks.md`: zero chaos/fault-injection harness exists (:29-32); P-H Wave-2 owns building it (:278). P55 is expected to build test *content* on P-H's harness; P56 supplies neither |
| 15 | serde_json is an accepted dependency for tools-tier code | `docs/design/DECART-serde-json.md` (accepted DECART report); `kernel/src/loops.rs:12` already imports `serde::Deserialize` |

Ground truth is non-discussible; everything below builds on this section only.

---

## 2. Scope — what P56 owns, what its consumers own, what it must not touch

**P56 owns (the shared machinery):**
1. The environment-dimension model (`EnvDims`) every probe run is stamped with (§4a).
2. The `Probe`/`ProbeRegistry` extensibility abstraction + versioned result schema (§4b).
3. The execution adapter that maps probe kinds onto **P25's existing** admission classes (§4c).
4. The content-addressed, chronological result store + the Hetzner sync policy (§4d).
5. The trend/query layer ("is this metric getting worse over the last N runs?") (§4e).
6. The meta-verification layer — checks on the tests/measurements themselves (§4f).
7. One new Telegram forum topic for testing-literature research digests (§4g).

**Consumers (cited by expected name; written concurrently — cross-reference on next pass):**
- **P54** (`BLUEPRINT-P54-llm-agent-verification-harness.md`): the LLM/agent probes themselves —
  adversarial prompt sets, agent-loop conformance, model-behavior checks. Every P54 probe is a
  `ProbeKind::LlmDispatch` (or `LocalInference`) implementor of §4b's trait; P54 defines WHAT is
  probed, P56 defines how it runs, where results live, and how its own honesty is checked.
- **P55** (`BLUEPRINT-P55-protocol-ecosystem-testing.md`): protocol/chaos/property tests — all
  `ProbeKind::DeterministicNative`; P55 builds test content on P-H's chaos harness (§1 row 14),
  registers it through P56's registry, inherits P56's storage/meta layers.

**Anti-scope (each names its owner):**
- No specific LLM probes (P54's) and no specific protocol/chaos/property tests (P55's).
- No chaos-injection *mechanics* — failpoints, `feature = "chaos"` gating, injection schedules
  are P-H Wave-2's (§1 row 14). P56 stores and meta-checks their results, nothing more.
- **No second scheduler.** Probe execution admits through P25's class table and (once built)
  `kernel/src/admission.rs` — §4c is an adapter, with a named cutover, not a fork.
- **No second alerting mechanism.** Every harness alert is a P45 `AlertEvent` at a P45 severity
  through the existing `tg_send`/spool lane and P45's routing table. P56 adds exactly ONE topic
  (research digests, §4g) to that table, by P45's own record-id-at-creation convention.
- **No multi-OS/GPU pretense.** §4a names what this single machine can and cannot exercise.
- **No local result accumulation.** The Hetzner sync policy (§4d) is a hard requirement with a
  hard local byte-bound, per the operator's explicit instruction and the disk-crisis history
  (DISK-OPS §0: 90% full, 2026-07-18).
- No product-code mutation from any probe: the harness reads, builds, and executes test targets;
  it holds no write capability toward product state (§5.2).

---

## 3. Predefined types & constants (standard §2 item 4 — named before implementation)

```rust
// ── tools/test-harness/src/ — NEW crate (pure std + serde_json per DECART-serde-json.md;
//    rclone shelled out for sync ONLY, §4d; kernel imported for sha3_256/stats/rng shapes).

/// Schema version stamped on EVERY result event. Evolution rules (§4b.3): new fields
/// are optional-with-default; existing fields are never renamed or repurposed; readers
/// accept every version ≤ current. Bump = a fixture-corpus row + CI read-back test.
pub const RESULT_SCHEMA_VERSION: u16 = 1;

// ── §4a — the test-matrix dimension model ────────────────────────────────────
/// Closed enum, extensible by the P40/P42 precedent (§1 row 9): ships with the
/// variants distinguishable TODAY; a new platform is a new variant + a schema-
/// version note, never a stringly-typed slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform { LinuxX86_64, LinuxAarch64, MacOsAarch64, WindowsX86_64, WasmWasi, WasmBrowser }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsaClass { X86_64Avx2, X86_64Avx512, Aarch64Neon, Wasm128 }

/// CPU dims are measured numbers + an ISA class, not a marketing tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuDims { pub phys_cores: u16, pub hw_threads: u16, pub isa: IsaClass }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPresence { None, WebGpu, Cuda, Metal, Rocm }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCondition {
    Full,                              // unshaped egress
    HighLatency { added_ms: u16 },     // tc netem delay (root available on this box)
    Lossy { loss_pct: u8 },            // tc netem loss
    Offline,                           // no egress (env-enforced; the mesh's own local-first case)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvDims {
    pub platform: Platform,
    pub cpu: CpuDims,
    pub mem_gb: u16,
    pub gpu: GpuPresence,
    pub net: NetworkCondition,
    pub emulated: bool,   // true when cpu/mem are cgroup/taskset-restricted or qemu-run (§4a.2)
}
// env_fingerprint() = kernel::event_log::sha3_256(canonical encoding of EnvDims).

// ── §4b — probe abstraction ──────────────────────────────────────────────────
/// Maps 1:1 onto P25's admission classes (C/D/L) — the load-bearing split for §4c.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind { DeterministicNative, LlmDispatch, LocalInference }

/// Discovery projection — the SkillCard pattern (P42 §2) applied to tests.
pub struct ProbeCard {
    pub id: &'static str,             // "p55/proto/settlement_replay"
    pub description: &'static str,    // ≤ MAX_CARD_DESCRIPTION_BYTES (200, P42's constant reused)
    pub kind: ProbeKind,
    pub version: u32,                 // bumped on any behavior change; part of the flake identity key
    pub requires: EnvRequirement,     // dims this probe needs (e.g. Gpu(Cuda)) — unmet ⇒ Verdict::Skip
    pub grounds: &'static [Ground],   // §4f.3 — what this probe's GREEN is contingent on
}

/// A content-addressed assumption the probe was written against (§4f.3).
pub enum Ground {
    FileHash { path: &'static str, sha3: [u8; 32] },   // fixture / contract file
    SymbolShape { path: &'static str, grep: &'static str, sha3: [u8; 32] }, // e.g. an enum's variant list
    DocAnchor { path: &'static str, sha3: [u8; 32] },  // the cited design claim
}

pub trait Probe {
    fn card(&self) -> &ProbeCard;
    /// Deterministic probes get NO backend parameter — a model call from a
    /// deterministic probe is type-unrepresentable (the P40 one-variant trick).
    fn run(&self, env: &EnvDims, seed: u64) -> ProbeOutcome;              // DeterministicNative
}
pub trait LlmProbe {                                                       // P54's surface
    fn card(&self) -> &ProbeCard;
    fn run(&self, env: &EnvDims, backend: &dyn LlmBackend, seed: u64) -> ProbeOutcome;
}

/// Static registry; construction PANICS on duplicate id, oversized description,
/// kind/trait mismatch (P42's StaticSkillRegistry discipline verbatim).
pub struct ProbeRegistry(/* Vec<ProbeEntry> */);

// ── §4d — the result event (event_log.rs pattern applied to test runs) ───────
#[derive(Debug, Clone, PartialEq)]
pub struct TestRunEvent {
    pub schema_v: u16,                 // RESULT_SCHEMA_VERSION
    pub prev: [u8; 32],                // chain link (zero = segment genesis), exactly MeshEvent::prev
    pub wave_id: u64,
    pub probe_id: String,              // ProbeCard.id
    pub probe_version: u32,
    pub env: EnvDims,                  // + env_fingerprint derivable
    pub git_sha: String,               // repo HEAD the probe ran against
    pub ts_unix: u64,
    pub seed: u64,                     // kernel::rng reproduction key (§1 row 4)
    pub metrics: Vec<(String, f64)>,   // "latency_ms", "tokens_in", "tokens_out", "rss_mb", …
    pub verdict: Verdict,
    pub meta: Option<MetaVerdict>,     // §4f — verdict ABOUT the verdict
}
// run_id = sha3_256(canonical serialization minus `meta`) — content-address = idempotency
// key (a re-reported run is a structural Duplicate, event_log.rs:302's exact property).

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict { Pass, Fail { reason: String }, Skip { missing: String }, Inconclusive { why: String } }

/// §4f — the meta-layer's typed findings about the INSTRUMENT, not the product.
#[derive(Debug, Clone, PartialEq)]
pub enum MetaVerdict {
    FlakyProbe   { pass: u64, runs: u64, wilson_low: f64 },   // same identity key, differing verdicts
    InstrumentTooNoisy { noise_pct: f64, threshold_pct: f64 },// noise band ≥ the threshold it polices
    StaleGround  { ground_path: String },                     // a Ground hash no longer matches live tree
    DeadProbe    { canary: String },                          // reported GREEN on the known-RED canary
}

// ── Thresholds & policy (single authority; P45 values REUSED where they exist) ─
pub const REGRESSION_THRESHOLD_PCT: f64 = 10.0; // = P45 BENCH_REGRESSION_PCT (one number, one meaning)
pub const CONFIRM_RUNS: u32 = 2;                // = P45 BENCH_CONFIRM_RUNS
pub const RUNS_PER_SAMPLE: u32 = 3;             // = P45 BENCH_RUNS_PER_SAMPLE (median-of-3)
pub const NOISE_GATE_FACTOR: f64 = 1.0;         // §4f.2: noise band ≥ 1.0× threshold ⇒ InstrumentTooNoisy
pub const FLAKE_MIN_RUNS: u32 = 5;              // §4f.1: minimum same-key runs before a flake verdict
pub const WILSON_Z: f64 = 1.96;                 // 95% — stats.rs wilson_interval's z
pub const CANARY_EVERY_N_WAVES: u32 = 8;        // §4f.4: each probe's known-RED twin re-fired on this cadence
pub const TREND_WINDOW: usize = 32;             // per-probe rolling samples kept in the local index
pub const LOCAL_RESULTS_MAX_MB: u64 = 64;       // HARD local bound: pending segments + index (§4d.3)
pub const SYNC_REMOTE_PREFIX: &str = "hetzner:dowiz/test-results/"; // NEW prefix beside backups|cold|db|images
pub const ORPHAN_SWEEP_MIN: u64 = 60;           // catch-up cron cadence for crash-orphaned segments
```

**Telegram routing addition (P45 §3 table convention — id recorded at creation):**

| Route | Lane | Chat | Topic | Purpose |
|---|---|---|---|---|
| `TEST_RESEARCH` | S2-class digests only | existing `-1003901655568` | NEW topic `Testing-Research` (id assigned at creation, recorded here) | §4g literature digests |
| harness regression/meta alerts | S1/S2 per §4f | existing | P45's `OPS_ALERTS`/294 Benchmarks — **no new topic** | reuse, not fork |

---

## 4. Build items — spec → RED test → mechanism, each with an adversarial case (items 3, 5)

### 4a. The test-matrix dimension model — and the honest single-machine gap

**Spec.** Every probe run is stamped with the full `EnvDims` (§3) and its `env_fingerprint`.
Probes declare `requires: EnvRequirement`; the runner evaluates it against the live host dims —
unmet ⇒ `Verdict::Skip { missing }`, recorded like any other event. **A skipped cell is data**
(the matrix's uncovered surface is queryable), a silently-absent cell is a lie.

**4a.1 What THIS machine can exercise today (live dims, §1 row 12):** exactly one native point —
`{LinuxX86_64, CpuDims{4, 8, X86_64Avx2}, mem 30, GpuPresence::None, net Full, emulated: false}`.

**4a.2 What it can additionally exercise honestly, marked `emulated: true`:**
- **Downward CPU classes:** `taskset` to 1/2 cores (a budget-device proxy for scheduling/latency
  behavior). Downward only — you can emulate a weaker CPU, never a stronger one or a different ISA.
- **Downward memory classes:** cgroup-v2 `memory.max` (2 GB / 8 GB cells) — real OOM behavior.
- **All four `NetworkCondition` values:** `tc netem` delay/loss (root available) + egress-denied
  offline runs — the one FULLY exercisable non-trivial dimension on this box, and the one the
  local-first mesh design cares most about (event_log's own offline-write property, :770-798).
- **Wasm as a platform cell:** `wasm32-unknown-unknown` build gates (the exact lane whose absence
  let the P36 regression live, §1 row 13) and wasmtime execution where a runtime is present
  (P35's territory; functional verdicts only).
- **Emulated foreign-arch (qemu-user), functional-only:** verdicts count, all timing metrics are
  auto-tagged `Inconclusive` — perf numbers under emulation are noise by construction.

**4a.3 The gap, named plainly:** macOS, Windows, Android/iOS, real Aarch64 silicon, and every
`GpuPresence` other than `None` are **not testable here at all**. They require CI runners and
hardware that do not exist in this environment — provisioning them is **[OPERATOR]**. What P56
ships now is the schema half: because every event carries `EnvDims`, a future remote runner just
reports different dims into the same store — **zero schema change** when the matrix grows. Until
then, cross-platform coverage = build-gates (compilation targets) + the emulated cells above, and
this blueprint says so rather than pretending an 8-vCPU GPU-less Linux box is a matrix.

**RED tests:** (i) two consecutive runs on this box → identical `env_fingerprint`; a
`taskset -c 0`-restricted run → different fingerprint with `emulated: true`; (ii) a probe with
`requires: Gpu(Cuda)` on this host → `Skip`, never `Pass`; (iii) a qemu-emulated run's latency
metric arrives `Inconclusive`, never enters a trend baseline.

**Adversarial case:** an agent "fills in" a matrix cell by running natively and hand-editing the
dims. Countered structurally: dims are *measured* at runner start (`lscpu`/`/proc/meminfo`/
`nvidia-smi` probe — never caller-supplied), and the event's content-id covers `env`, so an
edited record no longer matches its `run_id` and fails the §4d chain walk.

### 4b. Extensibility — the probe registry and the versioned result schema

**Spec — the P40/P42 pattern applied to tests, not a new philosophy (§1 row 9):**
1. **Closed-but-extensible enums.** `ProbeKind`, `Platform`, `GpuPresence` etc. ship with today's
   variants; extension = new variant + version note. No stringly-typed kinds, ever.
2. **Card/trait/registry trio** (§3): `ProbeCard` is the discovery tier (one line, ≤200 bytes);
   `Probe`/`LlmProbe` is the activation tier; `ProbeRegistry` is the static catalog whose
   construction panics on malformed entries — a broken catalog is unrepresentable at run time
   (P42's `StaticSkillRegistry` discipline verbatim).
3. **The growth rule (P42 §3.1's, restated for tests):** adding a probe = implement the trait +
   ONE registration line. Zero edits to the runner, scheduler adapter, store, trend, or meta
   layers. P54 and P55 grow the suite without ever touching P56's crates — that seam is the DoD.
4. **Versioned results (the "old results stay readable" half):** `RESULT_SCHEMA_VERSION` on every
   event; evolution rules in §3; enforced by a **fixture corpus** — one committed serialized
   event per historical schema version under `tools/test-harness/fixtures/schema/`, with a CI
   test that the current reader parses every one. A schema bump without its fixture row is RED.

**RED tests:** (i) register a trivial new probe in a scratch branch — diff touches exactly one
registration line outside the probe's own file; (ii) duplicate probe id → registry construction
panics (test asserts the panic); (iii) parse the full fixture corpus green; delete a fixture row
→ CI RED (the drift-gate on schema honesty).

**Adversarial case:** schema evolution by field *reinterpretation* (same name, new meaning) —
invisible to a parse test. Countered: the fixture test asserts *values*, not just parse success
(each fixture carries its expected decoded struct), so a reinterpreted field breaks the fixture's
equality assertion.

### 4c. Execution — promptless where possible; wave-scheduled through P25 where not

**Spec — reconciling the operator's "без промптів де можливо, з промптами — асинхронно, швидко й
хвилями" with the machinery that already exists:**

| ProbeKind | LLM prompt? | P25 class | Admission (P25's, reused verbatim) |
|---|---|---|---|
| `DeterministicNative` (all of P55; property/regression/matrix-build) | **never** — structurally: `Probe::run` has no backend parameter (§3) | **C** | 4 strict-core slots, `taskset -c 0,2,4,6`, `nice 10`, `Σ threads ≤ 4` (P25 §3.3) |
| `LlmDispatch` (P54's managed-API probes) | yes | **D** | `D_max = 16`, memory-PSI-gated, `admit_dispatch()` (P25 §3.4) — waves fan to full width immediately |
| `LocalInference` (P54's Ollama probes) | yes (local) | **L** | Ollama's own governor; each in-flight inference counts against the C budget (P25 §3.5) |

- Zero model-call cost for deterministic lanes is **by construction, not policy**: the type
  signature admits no backend, mirroring P40's "mutating tool invocation is type-unrepresentable".
- **A wave is sized by class, not one number** (P25 §3.7, adopted whole): a mixed wave fans its
  D-probes to `D_max` while its C-probes queue through the 4 slots; the two never share a
  bottleneck. Results post per-probe as they land (async), the wave closes when its last probe does.
- **No new scheduler — the named cutover:** `kernel/src/admission.rs` is not built yet (§1 row 6).
  v1 of the runner therefore embeds P25's §3.3/§3.4 predicates with P25's own constants — exactly
  the "apply the table manually" interim P25 §3.6 prescribes — behind one function
  (`fn admit(kind, gauges) -> Verdict`) whose body is REPLACED by a call to
  `kernel::admission::admit()` the moment P25 W3 lands. Same constants, same predicate shapes,
  one-commit cutover; drift between the two is a `SymbolShape` ground (§4f.3) on the runner itself.
- LOCAL-DECISION rule inherited: admission reads procfs/PSI only; `grep -c "ureq\|http"` on the
  admission module == 0 (P25 W3's own CI check, applied here too).

**RED tests:** (i) a mixed 20-probe wave: at no instant do >16 D-probes or >4 C-threads run
(observable from the runner's own event stream + `/proc/<pid>/status` affinity); (ii) the
deterministic lane completes with the LLM backend never constructed (a counter on backend
construction == 0); (iii) inject `psi_cpu avg10 = 20` fixture → C-probe deferred, D-probe admitted
(P25 W4's observable, reused).

**Adversarial case:** a "deterministic" probe that shells out to something that calls a model
(hidden D-load in a C-slot). Countered two ways: C-probes run with egress denied by default
(`NetworkCondition::Offline` is their standard cell — deterministic tests have no business on the
network), and any C-probe requesting `net: Full` must say why in its card description — a
reviewable, greppable declaration.

### 4d. Result storage — content-addressed chronology, shipped to Hetzner, never accumulating locally

**Spec — event_log.rs's pattern applied to test runs (§1 rows 1-2), not a new storage paradigm:**

1. **The chain.** Every `TestRunEvent` is content-addressed (`run_id = sha3_256(...)`, §3) and
   `prev`-chained per host, exactly `MeshEvent`'s shape: chronology is structural (the chain IS
   the timeline), duplicates are structural no-ops (a re-reported run dedups on content-id,
   event_log.rs:302's property), and integrity is checkable by the same `verify_chain`-style walk
   (recompute each id from its body; typed `ChainDefect` on mismatch — :475-528).
2. **Local = a bounded buffer + a small index, never the history.**
   - Pending wave segments: `wave-<seq>-<wave_id>.jsonl` written during the wave (append-only,
     JSONL — one serialized `TestRunEvent` per line; segments are KB-to-MB scale, compression is
     a named growth trigger, not v1).
   - The index: per-probe ring buffer of the last `TREND_WINDOW=32` samples (metric medians,
     verdicts, git_shas) + the chain tip + per-probe pinned baselines. This is ALL the trend and
     meta layers need hot (§4e/§4f); everything else lives remote.
   - **Hard bound: `LOCAL_RESULTS_MAX_MB = 64`.** At the bound (i.e. sync failing repeatedly) the
     harness goes fail-closed: **no new waves start**, S1 alert through P45's lane. A harness that
     cannot persist results must stop producing them, silently dropping nothing — and it must
     never re-create the 90%-disk incident (DISK-OPS §0).
3. **The sync policy (hard requirement, trigger chosen and justified):** **immediately after each
   wave completes** — `rclone moveto <segment> hetzner:dowiz/test-results/<host>/<yyyy-mm>/<seq>-<wave_id>-<first8>-<last8>.jsonl`
   (`moveto` deletes the local file only after a verified transfer — fail-closed by the tool's own
   size/hash check). Why per-wave, not interval: (a) wave close is the only moment a segment is
   complete and immutable — interval sync ships half-written segments or needs extra bookkeeping;
   (b) crash loss is bounded at ≤ 1 in-flight wave; (c) the filename encodes sequence + first/last
   event ids, so `rclone lsf` alone reconstructs chronology and chain continuity (each segment's
   genesis `prev` = previous segment's last id). A **catch-up sweep** (cron, every
   `ORPHAN_SWEEP_MIN`) uploads any orphaned segments from crashed runs — idempotent because names
   and content-ids are deterministic.
4. **`test-results/` is a NEW prefix** beside the live `backups/ cold/ db/ images/` (§1 row 8) —
   same bucket, same credentials, zero provisioning.
5. **The rclone boundary, argued per DECART culture (requirement stated, not assumed):** the
   harness, storage logic, trend math, and meta-verification are **native Rust** — the logic
   layer. `rclone` is shelled out to for exactly one thing: moving finished bytes to S3. That is
   a *thin external-tool boundary* — a process boundary around an already-provisioned, separately-
   verifiable transport (checkable by `rclone lsf`, replaceable by any S3 client without touching
   harness logic) — categorically different from a *foreign-runtime violation* (logic living in
   Python/Bash). Reimplementing S3 auth/multipart natively would add an HTTP+crypto surface with
   zero falsifiable win over a tool already trusted for `backups/`/`db/`. Same verdict class as
   bench_track.py's own delegation posture ("python is never the hot path") inverted: Rust is the
   logic path, rclone is the dumb pipe.

**RED tests:** (i) wave completes → segment present in `rclone lsf hetzner:dowiz/test-results/…`
AND absent locally; (ii) `kill -9` mid-wave → orphan segment exists, next sweep ships it, chain
walk over the re-downloaded pair is green; (iii) flip one byte in a downloaded segment → walk
reports `HashMismatch` (the byte-flip drill, P45 D-a3's pattern); (iv) block egress → after the
bound is hit, the runner refuses new waves and the S1 arrives (fail-closed proven by observing
refusal).

**Adversarial case:** the remote store silently diverges (a segment uploaded, then lost/altered
remotely). Mitigation: monthly drill re-downloads a random month and runs the chain walk against
it (the P45 §4a.3 "restore drill" discipline applied to results); freshness of the newest remote
segment is a metric on P45's digest (`dowiz_ops_testresults_last_sync_seconds`-shaped).

### 4e. Trend & query layer — regression/improvement over chronology, signal over noise

**Spec — P45 §4b.3 + P44's baseline-first rule REUSED as the statistics, not a new method:**
- Per probe metric: rolling **median of `RUNS_PER_SAMPLE=3`** per wave; breach = median >
  pinned baseline × (1 + `REGRESSION_THRESHOLD_PCT`/100); a regression is flagged only after
  **`CONFIRM_RUNS=2` consecutive** wave-level breaches; recovery posts a digest line, not a page.
  The noise-floor argument is P45 §4b.3's verbatim (±3-5% single-run noise on this host, median
  compresses the tail, 2 consecutive breaches ⇒ sustained shift). Improvements are tracked with
  the same math, opposite sign — a sustained −10% is a digest-visible win, feeding item §4f's
  baseline-refresh discipline honestly.
- Baselines: per-probe pinned file (the `kernel/benches/baseline.json` flat-map convention,
  extended per-probe); **refresh = explicit act with a REGRESSION-LEDGER row** (P45 §4b.3's
  anti-gaming rule verbatim; boiling-frog counter: the weekly digest shows Δ vs *baseline date*).
- The query the operator asked for — "is this metric getting worse over the last N runs" — is a
  pure fold over the local index's ring buffer (N ≤ `TREND_WINDOW`); deeper history questions
  re-download the needed months from `test-results/` into the scratchpad on demand (never a
  standing local mirror).
- **Pattern recognition, scoped honestly:** v1 ships (a) *chronological* — the per-probe trend
  above; (b) *cross-probe co-movement* — breaches grouped by first-breach commit range (two
  probes breaking in one range = one shared-root-cause hint, one digest line — cheap and real).
  The *topological* layer (co-failure clustering over the probe graph — `kernel/src/spectral.rs`
  is the named reuse target) is an extension point with a trigger (≥50 registered probes AND a
  recurring multi-probe incident), NOT built now — per standard item 16, cited where applicable,
  not decoratively.

**RED tests:** planted `sleep(1ms)` in a bench probe → exactly one flag after 2 waves with the
numbers in the message (P45 D-b3's arm, reused); 30 synthetic ±5% noise waves → **zero** flags
(the cry-wolf falsifier); a baseline refresh without a ledger row → digest drift-check flags it.

**Adversarial case:** trend math over a *changed environment* (a taskset-restricted emulated run
polluting the native baseline). Countered structurally: the baseline key is
`(probe_id, probe_version, env_fingerprint)` — a different fingerprint is a different series by
construction; emulated timing never merges with native timing (§4a.2's `Inconclusive` rule).

### 4f. META-VERIFICATION — checks on the tests and measurements themselves (the hardest ask)

The operator's precise words: recognize erroneous conclusions **of the measurements and tests
themselves**, not only their results. This session's own recurring find (§1 row 13) is the target
failure class: P34/P36/P06 were all *a claim reported GREEN because it was checked against a
referent that had silently moved*. Humans caught each one by hand. This layer is the mechanism
that catches them automatically. Four detectors, each a typed `MetaVerdict` (§3), each an event
in the same chain (chronology of instrument health is itself queryable), each alert-routed
through P45's severities.

**4f.1 Flake detector — same key, different answers ⇒ the instrument is broken, not the product.**
Identity key = `(probe_id, probe_version, env_fingerprint, seed)`. Every probe runs under a
kernel-`Rng` seed recorded in its event (§3) — P-H's "findings reproduce from `(seed, plan)`, no
wall-clock flake" doctrine (§1 row 4) — so for a deterministic probe the key pins *everything* an
honest run depends on. If runs sharing one identity key disagree on verdict:
`FlakyProbe { pass, runs, wilson_low }`, computed with `stats.rs::wilson_interval` (k passes, n
runs, `WILSON_Z`) after `FLAKE_MIN_RUNS=5` — the Wilson bound quantifies "how flaky" without the
p̂=1.0 self-certification collapse the function's own doc warns about (§1 row 3). **Routing rule,
load-bearing:** a flaky probe's Pass/Fail verdicts are excluded from trend baselines and from
regression alerts until the flake is fixed — a harness bug must never page as a product
regression, and vice versa. Severity: S2 (digest) rising to S1 if the probe stays flaky for a week.

**4f.2 Instrument-noise sanity — a benchmark noisier than its own threshold cannot detect it.**
Per metric per identity key, the meta layer maintains the measured run-to-run noise band
(p95−p5 spread of the last `TREND_WINDOW` medians, as % of median). Gate: noise band ≥
`NOISE_GATE_FACTOR × REGRESSION_THRESHOLD_PCT` ⇒ `InstrumentTooNoisy { noise_pct, threshold_pct }`
— the probe's regression verdicts are **suppressed to `Inconclusive`** (explicitly flagged, never
a silent false regression OR a silent false all-clear) until the probe is fixed (more iterations,
tighter isolation, or an honestly-raised per-probe threshold recorded in the ledger). This makes
P45 §4b.3's design-time noise argument a *standing per-probe machine check*: the argument is
re-proven live on every instrument, forever, instead of asserted once at design time.

**4f.3 Stale-ground detector — the P34/P36/P06 class, caught structurally.**
Every `ProbeCard` carries `grounds` (§3): content-addressed hashes of what the probe's GREEN is
*contingent on* — its fixture files, the shape of the contract it encodes (e.g. a grep-extracted
enum-variant list), the design-doc claim it verifies. At harness start, a ground-truth pass
re-hashes every ground against the live tree. Any mismatch ⇒ every dependent probe is demoted:
its verdict cannot be reported as plain `Pass` — it carries `StaleGround { ground_path }` until a
human (or the probe's owner-agent) re-grounds it, bumping `probe_version`. Worked example, the
P34 case replayed (§1 row 13): the spine probe's ground = `SymbolShape` over
`kernel/src/order_machine.rs`'s `OrderStatus` variants; the P07 commit adding
`Refunding`/`CompensatedRefund` changes that hash ⇒ the probe is flagged **the first run after
the commit** — before it can emit one more GREEN checked against the 10-variant world. The doc
half of P36's case is the same mechanism pointed at a `DocAnchor`: a probe verifying "wasm32
builds green" grounds on the claiming doc's paragraph; either the build breaks (normal Fail) or
the doc changes (StaleGround) — the "doc says GREEN while reality is RED" gap can no longer
persist silently in the harness's own domain. Severity: **S1** — a test checking the wrong thing
is worse than a failing test, because it manufactures false confidence.

**4f.4 Canary (negative-control) — does the smoke detector still detect smoke.**
Every registered probe ships a **known-RED twin input** (a canary: the deliberately-broken
fixture, the planted regression, the invalid protocol frame — the probe author declares it at
registration; registration *panics* without one, same construction-time discipline as §4b). Every
`CANARY_EVERY_N_WAVES=8` waves, the runner feeds each probe its canary: a probe that reports
GREEN on its canary is `DeadProbe { canary }` — it has stopped checking anything, whatever its
history says. This is the REGRESSION-LEDGER's red→green ratchet (a guardrail must be *proven* RED
before its GREEN is trusted) converted from a one-time landing ritual into a **standing cadence**,
and P45 D-a3's flip-one-byte drill generalized to every probe in the registry. Severity: S1.
Dead/flaky/noisy/stale probes appear as a dedicated block in P45's digest ("instrument health:
N probes, N trusted, N flagged") — instrument health gets the same visibility as product health.

**Meta-meta honesty (one sentence, structural):** the detectors are themselves registered probes
— each has a canary (e.g. the flake detector's canary is a synthetic coin-flip stream it MUST
flag; the noise gate's is a ±15% stream against a 10% threshold), so the layer that checks the
instruments is checked by the same mechanism, not exempt from it.

**RED tests:** (i) a fixture coin-flip probe → `FlakyProbe` within `FLAKE_MIN_RUNS`, Wilson bound
in the flag, and its verdicts verifiably absent from trend input; (ii) synthetic ±15% noise vs
the 10% threshold → `InstrumentTooNoisy`, zero regression flags emitted from that series; (iii)
mutate a probe's grounded fixture on a scratch branch → next run carries `StaleGround`, plain
`Pass` unreportable (assert the type-level impossibility: the reporter refuses an event with a
stale ground and no meta field); (iv) register a probe whose body is `Ok(())` (checks nothing) →
first canary cadence marks it `DeadProbe`; (v) the P34 worked example encoded as a fixture:
replay the enum-gaining commit → flag fires on the first post-commit run.

**Adversarial case (gaming the meta layer):** an author sets a trivial canary (input so broken
anything catches it) — DeadProbe never fires, dead probe lives. Not fully solvable structurally;
mitigated by (a) canary review at registration (the card's one-line canary description is
greppable/reviewable), (b) the flake/stale detectors still bound the damage, and (c) honestly
NAMED as the residual: meta-verification narrows the trust gap, it cannot close it — the last
reviewer is still a reader. No mechanism claims otherwise.

### 4g. The Telegram research branch — "окрема гілка в телеграм каналі"

**Spec:** ONE new forum topic, `Testing-Research`, in the existing chat `-1003901655568` —
created via the bot API exactly like P45 §10-W0 item 2's three Ops topics, **id recorded in §3's
routing table at creation** (the one sanctioned edit to this document). No new bot, no new chat,
no new send mechanism — `tg_deliver`/`tg_send` with `TELEGRAM_TOPIC_ID=<new id>` (lib.sh already
parameterizes this per-message, :118). Purpose: the standing literature lane — chaos-engineering
practice, property-based-testing techniques, flaky-test research, benchmark-methodology papers —
distinct from the operational lanes (257 reports, 294 benchmarks, P45's Ops topics), so research
reading never buries a page and pages never bury research.

**Posting convention (one shape, mandatory fields):**
`📚 [test-research] <theme> — <≤5 bullet digest> | sources: <links> | applies-to: <P54|P55|P56 item, or "none — archived">`
— one message per digest (never a thread-flood), weekly cadence default, S2-class always (research
is never a page). The `applies-to` field is mandatory for the same reason P45 §4e.1 makes "next
action" mandatory: a digest that binds to no roadmap item is noise by its own admission — "none"
is an allowed, honest value, but it must be said.

**RED test:** topic created, id recorded in §3; one real digest posted through `tg_deliver` with
the new topic id; a post missing `applies-to` is rejected by the posting helper (a 3-line format
check in the harness crate, not vigilance).

---

## 5. Cross-cutting design obligations (standard items 6, 8, 11, 13, 15)

**5.1 Hazard-safety from structure (item 6).** The harness's primary self-defeat mode is
*manufactured confidence* — exactly the P34/P36/P06 class. The counters are structural, not
procedural: content-addressed grounds (a stale referent changes a hash, not a reviewer's mood),
canaries at a fixed cadence (RED-provability is re-earned, never grandfathered), the
type-unrepresentable model call in deterministic probes, construction-panic registries, and the
fail-closed chain (`verify_chain`'s degrade-closed posture: integrity unprovable ⇒ `Unreadable`,
never assumed green — event_log.rs:474). Second mode: alert fatigue — inherited answer, P45
§5.1's taxonomy/dedup/digest discipline, consumed not duplicated.

**5.2 Isolation / bulkhead (item 11).** Probes hold zero write capability toward product state
(read/build/execute-under-test only); C-probes default to `Offline`; the harness's failure
degrades to "no test signal" (P45's dead-man lane notices the silence via the results-freshness
metric, §4d adversarial) — the product depends on the harness for nothing. Money/auth/RLS/
migration surfaces are never probe-mutable, red-lines preserved (memory
`test-integrity-rules-2026-06-27.md`).

**5.3 Schemas with scaling axes (item 8).** Events: ~10²-10³/wave, KB-scale JSONL — breaks at
~10⁵ events/wave (segment size), named upgrade = compression + segment splitting. Local index:
`TREND_WINDOW × probes × metrics` floats — breaks around 10⁴ probes; upgrade = per-probe index
files. Remote: monthly prefixes scale indefinitely on S3; the chain walk is O(month) per drill by
construction (segment-scoped, not whole-history). Matrix growth: new dims/platform variants are
schema-versioned enum extensions (§4b.4), remote runners merge with zero schema change (§4a.3).

**5.4 Rollback/self-healing, as math (item 13).** The store is append-only content-addressed
chain — "rollback" of a bad result is a new event superseding it, never a mutation (Snapshot
Re-entry does not apply; nothing here is a state machine to re-enter). Self-healing is claimed
for exactly one thing: the orphan sweep (idempotent re-upload from deterministic names/ids —
redundancy math, §4d.3). Self-termination: the fail-closed no-new-waves bound at
`LOCAL_RESULTS_MAX_MB` — a hard invariant boundary, not a supervisor's judgment call.

**5.5 Living-memory awareness (item 15).** The result chain is a temporal stream with a hot
window (local index) and a cold body (Hetzner) — move-not-delete: nothing is ever deleted from
history, only shipped colder. Mirrors the living-memory tiering doctrine
(`internal-retrieval-living-memory-arc-2026-07-14`: demote-never-delete) without building its
machinery.

**5.6 Feedback-loop readiness (operator ask 6 — the data shape, not the loop).** The chain is
directly foldable by the existing self-improvement machinery: verdict/metric sequences per probe
are the same JSONL-event shape `tools/loop-signals/` (Markov attractor, §1 row 11) already
consumes for tool outcomes, and the intake→loops seam (`intake.rs` `admit` → `loops.rs`, P32's
wiring template) is the named consumer for any future self-tuning rule ("if probe-class X trends
worse, propose Y") — P40's agent loop reads the same events through the trend layer's query API.
P56 ships the shape and the query; the loops that act on it are P32/P40's, cited not rebuilt.

---

## 6. DoD — falsifiable, per sub-section (item 2)

| # | Item | Falsifier (RED unless proven) |
|---|---|---|
| D-1 | dims model | identical fingerprint across 2 native runs; taskset-restricted run → distinct fingerprint + `emulated: true`; `requires: Gpu(Cuda)` on this host → `Skip` recorded, never `Pass`; emulated timing tagged `Inconclusive`, absent from baselines |
| D-2 | extensibility | new probe = 1 registration line outside its own file (diff-counted); duplicate id → construction panic (asserted); schema fixture corpus parses with value-equality; corpus row deletion → CI RED |
| D-3 | scheduling | mixed-wave drill: ≤16 D in flight, ≤4 C threads, C under `taskset 0,2,4,6`+`nice 10` (procfs-observed); deterministic lane with backend-constructor counter == 0; PSI-fixture defer/admit split (P25 W4's observable); admission module `grep "ureq\|http"` == 0 |
| D-4 | storage/sync | wave close → segment on `hetzner:dowiz/test-results/` AND absent locally; `kill -9` orphan shipped by sweep; re-downloaded chain walk green; 1-byte flip → `HashMismatch`; egress blocked → no new waves + S1 (refusal observed) |
| D-5 | trend | planted 1 ms sleep → exactly one flag after `CONFIRM_RUNS` waves with numbers; 30×±5% noise waves → zero flags; baseline refresh without ledger row → digest drift flag |
| D-6 | **meta-verification** | coin-flip probe → `FlakyProbe` + Wilson bound + exclusion from trends; ±15% noise vs 10% threshold → `InstrumentTooNoisy` + suppression; mutated ground → `StaleGround`, plain `Pass` unreportable; no-op probe → `DeadProbe` at first canary cadence; P34 worked-example fixture fires on first post-commit run; each detector's own canary green |
| D-7 | research topic | topic created + id recorded in §3; one real digest delivered via `tg_deliver`; format check rejects a digest without `applies-to` |

Every landed item adds its `docs/regressions/REGRESSION-LEDGER.md` row with red→green proof, per
the standing ratchet rule; the meta-detectors register under the ledger's guardrail taxonomy
(P45 §6's `alert-gate` word + a new one-word entry `meta-gate`).

---

## 7. Benchmark plan — the harness's own overhead budget (item 10)

A harness that perturbs what it measures corrupts §4f.2's own noise accounting, so its budget is
a gate, measured with the existing machinery only:

| Surface | Budget | Measured how |
|---|---|---|
| Runner dispatch + admission check | ≤ 1% of wave wall time; admission µs-scale (P25's own bar) | `bench_run` wrapper (lib.sh:168) around a fixture wave |
| Local index ops (ring-buffer fold, trend query) | µs-scale, zero alloc on the append path | criterion micro-bench in the harness crate (kernel/benches conventions) |
| Chain hash per event | ≤ 50 µs (sha3_256 over KB-scale events) | criterion, same file |
| rclone sync | fully off the critical path (post-wave, async); wall time logged per segment | `log_event metric` per sync |
| Meta-layer pass (grounds re-hash + canary wave) | grounds pass ≤ 5 s at 100 probes; canary wave counted as a normal wave in the C/D budget | `bench_run` at registry scale 10/100 (synthetic) |
| **End-to-end falsifier** | a probe's measured latency with the harness's telemetry ON vs a bare `cargo bench` run of the same body differs < 2% | criterion A/B, recorded in the ledger row |

---

## 8. Links to docs & memory (item 7)

- Consumers (concurrent, re-cite next pass): `BLUEPRINT-P54-llm-agent-verification-harness.md`,
  `BLUEPRINT-P55-protocol-ecosystem-testing.md`.
- Extended, not duplicated: `BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md` (P25 —
  scheduling), `BLUEPRINT-P45-ops-security-monitoring.md` (severity/topics/noise floor/digest),
  `BLUEPRINT-P-H-ops-telemetry.md` + `P-H-audit-telemetry-regression-benchmarks.md` (chaos
  mechanics boundary; rng doctrine).
- Patterns reused: `kernel/src/event_log.rs` (chain), `kernel/src/hydra.rs` `FileEventStore`,
  `kernel/src/stats.rs` (Wilson), `kernel/src/rng.rs` (seeds), P40/P42 (closed-enum + card/
  registry), P44 §4.1 (baseline-first), P32 + `tools/loop-signals/` (feedback seam),
  `kernel/benches/` (criterion/baseline/bench_track), `tools/telemetry/lib.sh` (send/spool/log).
- Operational ground: `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md` (live `hetzner:dowiz`, disk
  discipline), `docs/regressions/REGRESSION-LEDGER.md` (ratchet), `DECART-serde-json.md`.
- Memory: `test-integrity-rules-2026-06-27.md` (banned test classes — the meta layer's cultural
  ancestor), `verified-by-math-2026-07-07.md`, `markov-attractor-loop-signal-2026-07-13.md`,
  `internal-retrieval-living-memory-arc-2026-07-14.md` (tiering), `metacognition.md` (RED→GREEN
  as tool — §4f.4 is its mechanization), `harness-llm-backend-and-hermetic-remediation-2026-07-17.md`
  (P06 stale-DONE), `environment-and-ops-facts-2026-07-16.md`.

---

## 9. Standard-compliance map (all 20 points)

| # | Point | Where |
|---|---|---|
| 1 | Ground truth, file:line, this pass | §1 (15 rows, incl. live `rclone lsf` and the admission.rs-absent check) |
| 2 | Falsifiable DoD | §6 |
| 3 | Spec→test→code, event-driven | §4a-4g each spec→RED→mechanism; results ARE events on a hash chain |
| 4 | Predefined types & constants | §3 |
| 5 | Adversarial incl. intentionally-failing | every §4 item; §4f.4 makes intentional failure a standing cadence |
| 6 | Hazard-safety from structure | §5.1 (manufactured confidence countered by hashes/canaries/types) |
| 7 | Links | §8 |
| 8 | Scaling axes | §5.3 |
| 9 | Linux-discipline verdicts | inline: P25/P45 EXTENDS; event_log/stats/rng REUSE (ALREADY-EQUIVALENT); rclone boundary argued §4d.5; no GAP tool added |
| 10 | Benchmarks + telemetry | §7; every result event is telemetry by construction |
| 11 | Isolation/bulkhead | §5.2 |
| 12 | Mesh awareness | node-local only; the store syncs via S3, never the mesh transport; offline cell is first-class (§4a.2) |
| 13 | Rollback/self-heal as math | §5.4 (append-only supersession; idempotent sweep; hard local bound) |
| 14 | Error-propagation + smart index | §4f entire (the meta layer IS the smart index for instrument error); §4b.4 fixture drift-gate |
| 15 | Living-memory | §5.5 |
| 16 | Tensor/spectral where applicable | honest: not in v1; named trigger + reuse target (§4e) |
| 17 | Regression tracking | §6 closing rule (`meta-gate` taxonomy word) |
| 18 | Zero-context instructions | §10 |
| 19 | Reuse-first | scheduler/alerting/statistics/chain/registry all extensions of named existing machinery; the one new external surface (rclone) is already-provisioned and argued §4d.5 |
| 20 | Hermetic principles | Correspondence: results mirror runs via content-address, cannot drift silently. Cause-and-effect: every flag carries its measured cause (§4f typed variants). Polarity: fail-closed at every detector break. Rhythm: canary cadence + wave-boundary sync. Vibration: hysteresis via CONFIRM_RUNS, never threshold-naive |

---

## 10. Clear instructions for agentic workers (item 18 — zero session context assumed)

You are implementing P56 from this blueprint. Read §1-§5 first. Hard rules: **never** invent a
scheduler (use §4c's adapter; cut over to `kernel/src/admission.rs` when P25 W3 lands), **never**
send alerts except through `tools/telemetry/lib.sh` + P45's routing/severities, **never** let
pending results exceed `LOCAL_RESULTS_MAX_MB` (fail closed), **never** report a `Pass` that
carries a stale ground. P54/P55 register probes through §4b's registry — if their blueprints now
exist as siblings, reconcile card shapes with them before freezing the trait.

Build order (each lands with its §6 falsifier + a ledger row):
1. `tools/test-harness/` crate: §3 types + `EnvDims` measurement + fingerprint (D-1).
2. Registry + `Probe`/`LlmProbe` traits + construction panics + schema fixture corpus (D-2).
3. Runner + §4c admission adapter with P25's constants; C-lane egress-off default (D-3).
4. Chain store on the `event_log.rs` pattern + wave-segment writer + `rclone moveto` sync +
   orphan-sweep cron + the hard local bound (D-4). Create the `test-results/` prefix by first
   upload; nothing to provision.
5. Trend layer over the local index; per-probe baselines in the `baseline.json` convention (D-5).
6. Meta layer: flake (Wilson) → noise gate → grounds pass → canary cadence, in that order; each
   detector self-registered with its own canary (D-6). Encode the P34 worked example as a fixture.
7. Create the `Testing-Research` topic; record its id in §3's table (the one sanctioned edit
   here); ship the posting helper with the `applies-to` format check (D-7).

**[OPERATOR] items — prepare, never execute:** multi-OS/GPU runner provisioning (§4a.3); any
change to red-line surfaces. **What NOT to do (the failure modes of this phase):** no probe
content (P54/P55's); no chaos mechanics (P-H's); no second sender/scheduler/statistics stack; no
local history mirror; no un-canaried probe; no GREEN without a live ground.
