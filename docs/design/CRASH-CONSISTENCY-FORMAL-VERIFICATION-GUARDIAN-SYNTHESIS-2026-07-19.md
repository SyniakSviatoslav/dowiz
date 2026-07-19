# Crash-Consistency / Formal-Verification / Guardian Synthesis — Whole-System Determinism and the AI-Optional Invariant

**Date:** 2026-07-19 · **Role:** reasoning synthesis + blueprint (Fable), per the operator's process
directive in RAW-PROMPT-5 ("opus для досліджень, Fable синтез і блюпринти") · **Factual base:**
`RESEARCH-CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-2026-07-19.md` (the completed Opus
grounding pass — 11 questions, every claim cited against live source; TRUSTED here, not re-derived)
· **Source dialogue:** `RAW-PROMPT-5-crash-consistency-formal-verification-fail-fast-guardian-2026-07-19.md`
(verbatim) · **Arc this extends:** `DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md` + roadmap
items 33–44 (`191f509b6`). New execution items **45–49** are appended to
`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I.

**Epistemic rule:** every claim tagged **GROUNDED** (verified by the Opus research pass or an
earlier committed artifact, cited) or **PROPOSED** (this document's extension, reasoned but
unmeasured) — same convention as the prior two syntheses.

**Governing directive (operator, verbatim):**

> Продовжуючи тему з безпекою - вона має 100% передбачуваною, математично детермінованою із
> запобіжниками. Окрім цього уся система повинна здатна працювати без AI.

Two binding requirements, applied as the decision criterion throughout: **(a)** whole-system
determinism + predictability + safeguards — NOT scoped to the AI-inference subsystem of items
33–44; **(b)** the system must be able to operate with AI entirely absent — a preserved
architectural **invariant**, not today's accidental state.

---

## 1. Part 1 — The Raw Prompt's Open Questions, Resolved Against Ground Truth

### 1.1 Sequential Log vs Atomic Pointer Swap vs Hybrid → ANSWERED: Sequential Append-only Log. Hybrid parked with a named trigger.

**GROUNDED (research §1):** the kill-9 durability mechanism is a pure **Sequential Append-only
Log** — two alternating fixed-cap segment files (`fdr.a.jsonl`/`fdr.b.jsonl`, 1 MiB each), one
CRC32 per line, `'\n'`-delimited records with the newline escaped inside payloads, recovery
reading BOTH whole segments and merging by monotone `seq`. There is **no pointer updated after CRC
confirmation** — no Atomic Pointer Swap, no hybrid. The module doc states the design choice
explicitly: append-only segments are simpler to prove correct under torn writes than an in-place
cursor. The dialogue's open question is closed by reading the code, not by the abstract comparison.

**PROPOSED — ruling on the dialogue's Hybrid/LSM (WAL + periodic snapshot) recommendation:
Sequential-Log-only is sufficient NOW; the Hybrid is parked, per-surface, with named reopening
triggers.** The Hybrid's sole benefit is bounding startup-replay time. Split by surface:

- **FDR ring: the Hybrid solves a problem the FDR structurally cannot have.** Replay is bounded
  *by construction* — recovery reads at most 2 × 1 MiB of segments, ever (GROUNDED: `DEFAULT_SEG_CAP`,
  A/B ring). Adding a snapshot layer here would add the exact class of recovery-path complexity the
  dialogue's own Fail-Fast/KISS section argues against ("краще Kernel_Init, перевірений на 100%,
  ніж Kernel_Recover з новими прихованими помилками"). **Ruling: Sequential-Log-only, permanent,
  for the FDR.**
- **Durable `EventLog`: genuinely unbounded replay — but the question is premature.** GROUNDED
  (research §2): `event_log.rs` is a pure append-only hash chain with no snapshot; the only
  snapshot-adjacent mechanism (`hub_supervisor`'s `StateSnapshot`) is a log-position pointer for
  UPDATE ROLLBACK, not replay-speedup. But GROUNDED (roadmap item 2, closed-as-defect): **no
  production composition root constructs the durable `FileEventStore` at all** — the wiring gap is
  filed, unfixed. A snapshot layer for a store that is not yet wired would be optimizing an
  unreachable path. **Ruling: park the Hybrid behind item 49 — measure replay time once the store
  is actually wired; reopening trigger = measured startup replay exceeding a stated budget.** This
  is the item-25 park-the-alternative discipline, applied.
- One correctness note carried forward from the dialogue, endorsed: IF a snapshot layer is ever
  built, the data file must be fsynced *before* the pointer swap — the dialogue's own caveat is
  correct, and consistent with `ring.rs`'s existing kill-9-vs-power-loss separation (GROUNDED:
  research §1 — fsync only on Alarm/PostMortem/segment-switch).

### 1.2 Watchdog timer → PARTIALLY redundant. The crash half is solved; the HANG half is not. Scope narrowly to the gap.

Honest decomposition of what the kill-9 test actually proves (GROUNDED, research §1/§11): it
proves **recovery after process death** — a dead process's log is readable, torn tail dropped, a
`PostMortem` summary emitted on the next start. It proves nothing about a process that is **alive
but not making progress** (deadlock, livelock, lost wakeup). A hung process never dies, never
restarts, never emits a `PostMortem` — it is the one failure class the entire FDR machinery is
structurally blind to.

That class is not hypothetical in this codebase: the span-metrics thread-local **self-deadlock
hang** was root-caused and fixed in this session's wiring wave (GROUNDED: k3,
`built-but-unwired-core-surface-wired-2026-07-19`, commit `67851b2f3`). And GROUNDED (research §9):
no unbounded hot-path loops were *found*, but absence-of-finding is not a liveness proof.

**PROPOSED — ruling: a watchdog is warranted, but only the minimal hang-detection shape, and the
restart authority stays OUTSIDE the kernel.** Two deliberate narrowings against the dialogue's
generic watchdog:

1. **In-kernel half = a heartbeat record, nothing more.** A periodic `Heartbeat` FDR event (one
   new closed-enum `Kind` variant) carrying `seq` + monotonic progress counters. The kernel does
   NOT carry self-kill/self-restart logic — that is exactly the "лікувальний код" the dialogue's
   own KISS argument (and the existing `Kernel_Init`-over-`Kernel_Recover` posture) rejects.
2. **Liveness judgment + restart = the platform layer.** The external observer (systemd
   `WatchdogSec`, or the deployment layer, with `hub_supervisor`'s crash-loop detection as the
   existing deploy-granularity precedent — GROUNDED, research §5) declares the process dead on a
   missed heartbeat and kills it — at which point the ALREADY-PROVEN kill-9 recovery path takes
   over. The watchdog does not add a second recovery mechanism; it converts hangs into the crash
   class the system already survives. This is the dialogue's own "AI має впасти швидко, але
   Платформа має приземлитися м'яко" applied to the kernel itself.

A sibling blind spot closed in the same item (research §11): **no panic hook exists** — a panicking
process today writes nothing before dying. A `std::panic::set_hook` that emits ONE fsynced `Alarm`
FDR record (message + location; the `Alarm` kind already fsyncs — GROUNDED) gives the "black box
captures the cause" property the dialogue wants, std-only, ~zero recovery-path complexity. Scoped
as item 48. The raw prompt's register/stack core-dump framing is explicitly NOT pursued — the
`PostMortem` summary + panic-site Alarm + the event trail is the proportionate black box for a
`std` userspace kernel.

### 1.3 "Hard-coded Fallback" structure → neither if-else soup nor function-pointer tables: typed advice, parse-don't-validate gate, named pure static procedure. New item 47, extending item 9 — NOT folding into item 40.

The raw prompt asked generically (if-else blocks vs static named procedures). The idiomatic Rust
shape, given this kernel's actual house standards, is more specific than either:

**PROPOSED — the Guardian shape:**

- **AI advice is DATA, not a code path.** The (future, items 33–44) inference subsystem produces a
  plain `Proposal` struct. The kernel decision function takes `Option<Proposal>` — absence
  (`None`) is a first-class, first-tested input, not an error path. This single signature IS the
  AI-optional invariant expressed in the type system: the deterministic path is the total
  function; advice is an optional refinement.
- **The gate is parse-don't-validate:** `fn admit(p: Proposal, ctx: &Invariants) ->
  Result<ValidatedProposal, Rejection>` where `ValidatedProposal` is a newtype constructible ONLY
  through `admit` — an invalid-but-accepted advice is unrepresentable, the same
  illegal-state-unrepresentable standard as item 9's `Result<Permit, Tripped>` and the §1.5 house
  rule. Invariants are checkable equations (the dialogue's `Result.velocity < MAX_SAFE_SPEED`
  class), written down as a spec, not scattered asserts.
- **The fallback is a NAMED pure function, statically dispatched** — `fn static_procedure(input)
  -> Action`, `match`-based, no `dyn`, no function-pointer indirection mutable at runtime —
  matching `order_machine`'s pure-function FSM style (GROUNDED: research §8 — the FSM is already
  exactly this). Not inline if-else soup: a named procedure is independently testable, and its
  name appears in the FDR record when it fires.
- **Rejection is observable:** every `Rejection` emits an FDR event; when item 9's breaker lands,
  repeated rejections route through the breaker's typed-permit vocabulary (same composition clause
  as item 40 — the design does NOT gate on item 9).

**Why a NEW item and not a fold-in (GROUNDED, research §7):** item 40 is bit-integrity — a golden
checksum whose mismatch is *hardware/memory-fault* evidence, explicitly "not a model error." The
Guardian rejects **well-formed but semantically unsafe advice** — a different predicate, different
false-positive economics, different invariant source (domain law vs pinned vectors). Item 9's
breaker is the safeguard *family's* pivot but is broader than advice-gating. So: item 47 **extends
item 9** (consumes its vocabulary, adds the advice-plane member of the safeguard family) and
**cross-references item 40** (the two checks run on different planes of the same output — bits and
meaning). The closest existing precedent, named so nobody re-derives it: `decision/import.rs`'s
`import_unit` verify-before-persist gate (GROUNDED, research §7) — the same reject-on-any-
disagreement shape, at import granularity; the Guardian is that shape at per-advice granularity.

**A reframing that matters:** since AI-optional is ALREADY TRUE (research §8), the "fallback" is
not a fallback at all — **the deterministic path is the system; AI is the optional overlay.** The
Guardian item is therefore "gate the future AI advice," not "build a deterministic mode." Nothing
about today's decision plane changes.

### 1.4 Testing the static procedures → exhaustive-first, oracle/differential second, proptest as sweep, planted-rejection mandatory. NOT a new framework.

GROUNDED (research §3): the dominant house pattern is hand-rolled deterministic
oracle/differential/exhaustive corpora (TS legacy oracle for the order FSM, golden signature,
power-iteration oracle, MST hand-oracle, P77 differential harness, the FDR fixed-vector tests);
`proptest` is real but narrow (two payment files, 400 cases each); Kani and TLA+ are planned-only,
zero usage. The testing strategy for static procedures follows what this codebase demonstrably
does well, not abstract best practice:

**PROPOSED — the standard, folded into item 47's proof conditions (not a separate item):**

1. **Exhaustive where the domain is enumerable** — the house 65536-pair standard, literally (item
   35 already applies it to i8×i8). Static procedures MUST have bounded, documented input domains
   (that is what makes them static); small domains get full enumeration against a hand-written
   expected-output oracle.
2. **Oracle/differential for larger domains** — a second, obviously-correct reference
   implementation retained forever (the item-37 "Schoolbook" pattern), plus a golden signature so
   drift is a diff, not a debate.
3. **proptest as a supplementary sweep, precedent-shaped** — exactly how item 5's regex-parity
   used it (fixture corpus + synthetic corpus + a proptest sweep on top). Supplementary, never the
   primary proof.
4. **Planted-rejection tests are mandatory (P7)** — the Guardian must demonstrably reject a
   planted invalid `Proposal` (red→green), the same discipline as item 40's planted bit-flip and
   the dudect planted leak: a verifier that has never rejected anything proves nothing.
5. **Bounded-loop clause:** every static procedure's loops must be statically bounded
   (`0..MAX_N`), asserted by the item-42-style source-structure test. Full WCET analysis is
   explicitly OUT OF SCOPE — see §5.

---

## 2. Part 2(a) — Whole-System Determinism: the Float Judgment Call

### 2.1 The precise claim, stated honestly

The raw prompt's "float — це зло" is true only for a specific threat model, and it is important to
be exact about which one, because the remedy's cost varies by orders of magnitude:

- **IEEE 754 arithmetic (+, −, ×, ÷, sqrt) is bit-deterministic for a FIXED binary on FIXED
  hardware** — correctly-rounded by standard, same instruction stream ⇒ same bits, every run. A
  single deployed dowiz kernel binary is NOT unpredictable to itself.
- The non-determinism the dialogue describes lives on the **portability axes**: different
  compiler versions (instruction selection, FMA contraction, autovectorization changing reduction
  order), different ISAs, fast-math flags, thread-scheduling-dependent reduction order, and —
  the empirically dangerous one — **libm transcendentals** (`sin`/`cos`/`exp`/`ln`), whose
  precision is implementation-defined, not standardized.

### 2.2 What this codebase's own evidence says

- **GROUNDED — the one real float-nondeterminism bug this codebase ever shipped was a libm
  transcendental, not arithmetic:** `REGRESSION-LEDGER.md` row 25 — platform-dependent ULP error
  in float `sin`/`cos` inside `empirical_identify`, fixed by the integer Q30 CORDIC
  (`d692c59fc`, `a0a375ad8`; re-verified in the AI-inference synthesis §1.1). Zero ledger
  precedent of basic float arithmetic diverging.
- **GROUNDED — the portability axes are already substantially pinned:** toolchain pinned exact
  (`1.96.1`, item 14, CLOSED) with a structural bump gate requiring a spot-check artifact; zero
  external crates (Tier 1 CLOSED) so no dependency can smuggle in a differently-compiled numeric
  path; `simd.rs` carries the bit-identity design rule (vectorize across rows, never within a
  reduction) with falsifier tests; `attention.rs` documents bit-reproducibility across
  native/wasm32 as a design property; rustc enables no fast-math.
- **GROUNDED (research §10) — the exposure:** the non-AI kernel numeric plane (`spectral.rs` ~91
  float sites, `markov.rs` ~32, `token_bucket.rs` f64 throughout) is f64; fixed-point exists only
  in the CORDIC/eqc/money surfaces.

### 2.3 The judgment call

**PROPOSED — ruling: the pinned-toolchain + zero-dep + bit-identity-SIMD posture already delivers
the deployed-binary determinism guarantee the directive operationally requires. The residual risk
is confined to (i) toolchain bumps re-ordering float code and (ii) libm transcendentals — both are
containable narrowly. A kernel-wide f64→fixed-point rewrite is REJECTED at this time as
disproportionate.** Reasoning:

1. Read literally, "100% математично детермінована" would demand bit-exact portability across all
   hardware forever — a property even seL4 does not claim for numerics. Read operationally — the
   way this codebase already operationalizes P6 (replayed input ⇒ bit-identical output for the
   deployed artifact, native + wasm32) — the guarantee mostly HOLDS today, by construction.
2. The evidence base for a full rewrite is one ledger row, and that row indicts transcendentals
   specifically. Rewriting `spectral`/`markov`'s tested, oracle-covered f64 planes (iterative
   eigensolvers in fixed-point are a research problem: scaling, overflow, convergence-criterion
   redesign) would risk regressions in proven code to fix a bug class with zero observed instances
   there — the opposite of safety-first.
3. What DOES warrant work is making residual float drift **detectable at the gate instead of
   silently shipped** — a "запобіжник" in exactly the operator's sense. That is item 46:
   - inventory every libm-transcendental call site in the deterministic kernel plane (the
     CORDIC-precedent class — `sin`/`cos`/`exp`/`ln`/`powf`; `sqrt` is correctly-rounded and
     exempt); disposition each: migrate to integer/CORDIC-class, or pin under a golden test;
   - require every value that feeds a **cross-version/cross-host comparison surface** (golden
     signatures, oracle pins, `wire_code()`s, `DRIFT_BAND`-class classification constants) to be
     either integer-domain or covered by a golden test. **Precise mechanism (verified against the
     item-14 gate on the exec branch): the toolchain-bump gate itself does NOT run the golden
     tests — it structurally requires a `docs/audits/toolchain/spot-check-<new>.md` artifact
     (with `## Assembly spot-check` + `## Full-suite re-run` headings) in the same diff on a
     `channel` bump. The actual re-execution under the new compiler is delivered by (i) the
     always-on `cargo test (kernel + engine, offline, unconditional)` job — which runs the full
     suite under the `rust-toolchain.toml`-pinned channel, so a bump PR re-runs every golden test
     under the new rustc — and (ii) item 6's `hardening-gate`, which UNCONDITIONALLY re-runs the
     oracle rows (golden signature, drift gate, wire pins) every build.** Net safety property:
     a compiler-induced float divergence in a golden-covered surface turns the bump PR RED with a
     named failing test instead of shipping — provided this item first adds the golden coverage
     for the currently-uncovered f64 surfaces (that coverage is exactly scope (ii));
   - the full fixed-point conversion is parked as an explicitly-flagged-LARGE item with a named
     reopening trigger: a reproduced cross-version golden divergence in basic float arithmetic,
     or a multi-ISA deployment requirement. Until the trigger fires, the narrow containment is
     the whole scope.

This is the same shape as every closed item in this arc: don't rewrite what is proven; gate what
could drift; name the trigger that would change the answer.

---

## 3. Part 2(b) — AI-Optional: From Accidental Truth to Enforced Invariant

**GROUNDED (research §8):** AI-optional is ALREADY TRUE — `attention.rs`'s own doc pins "the
kernel stays non-AI"; `order_machine`, `decision/import`, `decision/mod`, and `hydra` import zero
AI-adjacent modules; the decision plane is fully deterministic today. The job is purely to make
this an invariant a future regression cannot silently break — which becomes live the day items
33–44's inference subsystem lands.

**PROPOSED — item 45, the `ai-optional-gate`, deliberately small (one law + one CI job + one
red-proof):**

1. **Structural law, amended into the 33–44 arc:** the inference subsystem lands behind a
   **non-default cargo feature** (e.g. `inference`) — the exact compile-time surface-control
   pattern the kernel already uses for `pq` and `slot-arena` (GROUNDED, research §4). The default
   build never contains AI code. This is one sentence of law, zero new machinery.
2. **CI job:** (a) build the kernel default-features (AI absent) and run the FULL test suite —
   proving the system does not merely compile without AI but passes every behavioral guarantee
   without it; (b) a dependency-direction check, `zero-dep-gate.sh`-precedent-shaped: no core
   decision module (`order_machine`, `decision/`, `hydra`, `event_log`, `markov`, `spectral`,
   `fdr`) may reference the AI module paths outside the feature gate — AI may depend on core,
   never core on AI.
3. **Red-proof (P7):** a planted core→AI import must demonstrably turn the gate RED before the
   gate counts as landed — same standard as every gate this session shipped.
4. **Runtime half, owned by item 47, not duplicated here:** the Guardian's `Option<Proposal>`
   signature makes AI-absence a first-class tested runtime input (the `None`-path test proves
   output equality with the deterministic baseline). Compile-time absence = item 45; runtime
   absence = item 47; together they are the invariant.

Explicitly NOT built (over-design guard): no runtime kill-switch service, no dual-binary release
pipeline, no AI-health monitor. The feature gate + the CI job + the Guardian's `None` path are
sufficient and each is independently testable.

---

## 4. Terminology Collisions and Non-Mapping Analogies — Flagged So They Are Never Re-Imported

The raw prompt's space-grade precedents are inspiring but four of them do NOT map onto this
codebase the way the words suggest. Recording the collisions (all GROUNDED, research §§4–6, 9, 11)
so future docs cite this section instead of re-deriving:

| Raw-prompt term | What exists here under that name | Verdict |
|---|---|---|
| seL4 "capability" (memory-access token) | `capability_cert.rs` = UCAN/biscuit-lineage **authority-token** chain (who may act on what), NOT memory isolation | **Genuine terminology collision — never conflate.** In-kernel per-module memory isolation does not exist and is not proposed; the real isolation boundaries are Rust's type system, the wasm32 sandbox (whole-kernel), and the microVM deployment gate. No item. |
| Erlang/OTP "supervisor tree" | `hub_supervisor.rs` = A/B-slot software-UPDATE rollback machine (crash-looping *release* → revert), not per-actor restart | No OTP-style actor hierarchy exists or is warranted for a single-process kernel; the platform-restart contract (item 48) + hub_supervisor cover the two real granularities. No item. |
| TMR (triple-vote) | Single-copy integrity checks (CRC32/SHA3/AEAD) — detection, not vote-and-recover | Already owned by **item 12** (SIHFT triple-vote, design-only, operator-ruled). Cross-reference only; no new item, no competing mechanism. |
| `no_std` / `#[panic_handler]` / static-alloc / WCET | Kernel is `std`; arena is fixed-cap-with-fallback; no WCET tooling anywhere | The bare-metal stack does not apply to a `std` userspace kernel on Linux — WCET is not even well-defined under a preemptive scheduler + page cache. Adopted instead: the bounded-loop clause (§1.4.5) and the panic **hook** (`set_hook`, not `panic_handler`) in item 48. WCET tooling: OUT OF SCOPE, stated. |

## 5. Proportionality Ruling — Where the "Verify Everything" Escalation Lands

The dialogue escalated to Coq/Lean-as-primary-artifact, code-as-derivative, TCB minimization,
verified compilers — then its own source assistant walked it back ("Вибач, я, мабуть, занадто
сильно 'академізував' процес"). This synthesis lands exactly where that self-correction landed,
and the codebase's own record is the evidence it is the right landing:

- **GROUNDED:** this session's entire proof discipline has been runtime-verification-first — CRC
  recovery, kill-9 differential tests, golden signatures, oracle corpora, dudect planted leaks,
  binary/assembly spot-checks, re-executing (never presence-checking) CI gates. That IS the BITE
  posture the dialogue's corrected turn describes, already operational.
- **Ruling (PROPOSED, but continuous with the standing item-6/7/10 scoping):**
  **BITE/runtime-verification is PRIMARY.** Narrowly-scoped Kani (item 7's list: Keccak, FSM
  graph algorithms, NTT arithmetic, GCRA — panics/overflow/bounds, not functional correctness)
  and proptest-as-sweep are SECONDARY. TLA+ stays exactly where item 10/11 put it
  (decision-import + order FSM spec; ARINC-653 TLC model, design-only). **Coq/Lean, proof-carrying
  code, verified compilers, and full-TCB minimization are OUT OF SCOPE** — for a ~1000-test,
  zero-dep, single-team kernel, the proof-assistant path would consume the development budget the
  dialogue itself estimated (80% proving / 20% writing) to defend against bug classes the runtime
  gates already catch at the moment they matter, on the real hardware, including the bit-flip
  class no static proof can address (the dialogue's own hardware-entropy argument). Reopening
  trigger, named: an external certification requirement (DO-178C-class) or a formally-verified
  downstream consumer demanding machine-checked functional-correctness artifacts.

## 6. Deliverable

Items **45–49**, dependency-ordered with real proof conditions, appended as §I of
`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md`. Planning only — same law as the whole
roadmap: no item starts before the operator dispatches it. One-line map:

- **45** — `ai-optional-gate`: feature-gated AI + default-build full-suite CI + dependency-direction
  check + red-proof. READY (asserts today's truth; gains teeth when 33–44 land).
- **46** — float-determinism containment: transcendental inventory + golden coverage wired into
  item 14's bump gate; full fixed-point rewrite parked with named trigger. READY.
- **47** — Guardian: typed advice gate (`Option<Proposal>` → parse-don't-validate →
  `ValidatedProposal`) + named static procedures + `None`-path equality test; extends item 9,
  cross-references item 40. After item 35 (spec) / item 42 (full wiring).
- **48** — FDR blind-spot closure: panic hook → fsynced Alarm; heartbeat `Kind` + external
  liveness contract (platform restarts, kernel never self-heals). After items 4+29 (done).
- **49** — event-log replay-bound measurement + Hybrid/LSM park with reopening trigger. After
  item 2's wiring fix lands.
