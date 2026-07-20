# BLUEPRINT — Item 60: Engine Frame-Loop + Voice Instrumentation (gaps G3 + G11)

- **Date:** 2026-07-19 · **Tier:** code (roadmap §K, item 60) · **Status:** BLUEPRINT (planning
  artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 60
  (lines 1036–1047); `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` (gaps G3/G11);
  `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` step 9 (wasm leg); item 58 blueprint;
  item 62 blueprint (`BLUEPRINT-ITEM-62-…`, the shared wasm-clock design); ground-truth code:
  `engine/src/engine_loop.rs`, `engine/src/bridge.rs`, `engine/src/voice.rs`.
- **Prerequisites:** **item 58** (`(work, cost)` pair + `WorkloadKind::FramesRendered`). The engine
  currently has **ZERO `Instant::now`** (grep-verified this session).

---

## 1. Scope & goal

**Goal.** The physics render engine measures nothing about its own time. Add (a) frame-time
instrumentation against a **named frame-budget constant**, (b) wake-word/ASR latency (wiring the
already-declared-but-dead `InferError::Timeout`), and (c) a stated wasm clock leg — so the engine's
"battery lever" claims stop being unmeasured and its frame budget stops being aspirational.

**Non-goals.**
- NOT a change to the physics/field math (timing is P3 observation, not a field input).
- NOT a GPU dependency (the engine is offline-clean, zero external crates by default; timing uses
  std `Instant` on native, a `performance.now()` import on wasm).
- NOT money/decision-adjacent (the engine loop already excludes consequential commands,
  `engine_loop.rs:101–104`).

## 2. Current-state grounding

### 2a. Frame loop — no time (gap G3)

- `engine/src/engine_loop.rs:58–71` — `EngineLoop::frame(surface, profile) -> usize` is the
  production caller of `InputRouter::tick`; it polls/classifies/applies and returns the intent count.
  It takes **no `Instant`** and has **no budget check** — frame *cost* is invisible.
- `engine/src/bridge.rs:19–25` — `FrameProfiler { json_parse_calls, write_buffer_calls }` counts the
  two FE-01 costs the zero-copy gate names, but carries **no time** field. `bridge.rs` owns the
  `VertexBridge` upload path (`profiler`/`profiler_mut` at `:189–195`).
- Grep confirmed: **no `Instant::now` anywhere in `engine/src`** — the engine is temporally blind.

### 2b. Voice — a dead timer (gap G11)

- `engine/src/voice.rs:5` — module doc names `WakeWordSpotter` as "the battery lever" — an explicit
  efficiency claim (wake-gate blocks ASR to save battery) with **zero measurement**.
- `engine/src/voice.rs:111` — `fn feed(&mut self, pcm: &[i16]) -> Result<Vec<AsrDelta>, InferError>`
  (the ASR hot call). `:121–125` `enum InferError { …, Timeout }` — **`Timeout` exists but no timer
  ever produces it** (dead variant).
- `engine/src/voice.rs:129–132,341,345` — `WakeWordSpotter`; `feed_mic`; `asr_feed_calls()` (the
  battery-lever *count* proof: ASR feed must be 0 before wake, `:440`) — but the *latency* of a feed
  is never measured.

### 2c. wasm leg — must be stated (gap G4 shared with item 62)

- The engine compiles to a wasm cdylib. `Instant::now()` **panics on `wasm32-unknown-unknown`** (the
  same trap the FDR module guards, `fdr/schema.rs:236–237`, `fdr/mod.rs:216–224`). Any engine timer
  must state its wasm leg — a `performance.now()` import or a named absence — **one design, shared
  with item 62's wasm clause**, not two.

## 3. Implementation plan (numbered)

1. **(a) Named frame-budget constant, one authority site (P3 rate discipline).** Declare
   `const FRAME_BUDGET_US: u64 = …` (e.g. 16_667 for 60 fps) as the **single** authority, with a pin
   test asserting its value (mirroring the FDR `DEFAULT_SEG_CAP` one-authority pattern,
   `fdr/ring.rs:33`). No magic frame-time numbers scattered in the loop.
2. **(a) `EngineLoop::frame()` measures frame time.** Wrap the `frame()` body with a wasm-safe clock
   (native `Instant`, wasm `performance.now()` import — step 4) and compare elapsed against
   `FRAME_BUDGET_US`. Extend `FrameProfiler` (`bridge.rs:20`) with a time field (`last_frame_us`
   and/or a p50/p99 accumulator) alongside its existing call counts. Workload-kind `FramesRendered`;
   the `(work: FramesRendered Δcount, cost: Δframe_us)` pair is item 58's schema, emitted under the
   `telemetry` feature.
3. **(b) Voice latency — wire the dead timer.** In `voice.rs`, bracket `AsrModel::feed`
   (`:111`) and the `WakeWordSpotter` spot with the wasm-safe clock; on exceeding a named ASR budget
   constant, return `InferError::Timeout` — making the **currently-dead `Timeout` variant reachable
   from a real timer** (red→green: today it is unreachable). The battery-lever efficiency claim gains
   a measured basis (feed-latency + the existing `asr_feed_calls` count = a real energy-proxy pair).
4. **(c) One shared wasm clock design (with item 62).** All engine timing states its wasm leg per
   procedure step 9: native `std::time::Instant`; wasm `performance.now()` via a single imported
   binding (or a named `Absence` where a surface genuinely cannot time on wasm). This is **the same
   `performance.now()` design item 62 specifies for the FDR wasm leg** — coordinate to one binding,
   not two. The wasm cdylib must stay green (no `Instant::now()` on the wasm path).
5. **Feature gating.** Frame-time p50/p99 emit under the `telemetry` feature (the item-57 G9 posture:
   cheap floor always compiled, heavy stamps feature-gated). The default engine build stays
   offline-clean and untimed-but-accounted (its HOT-PATHS `eff` cells name the workload or a `gap:`).

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 60 |
|---|---|
| 1. **Oracle** | **Frame-time p50/p99 emitted under the telemetry feature with a budget-breach test:** a planted slow frame (sleep/spin past `FRAME_BUDGET_US`) is flagged (red→green). The budget constant is pinned by a value-assertion test. **Voice:** `InferError::Timeout` is demonstrably **reachable from the real timer** (a planted slow feed returns `Timeout` — red→green, today the variant is dead). |
| 2. **Dudect** | **N/A** — frame/voice latency is public UX timing, not secret-dependent; no CT branch introduced. |
| 3. **Debug cross-check** | **N/A** — timing is a measured value, no per-call arithmetic reference. |
| 4. **ASM spot-check** | **N/A** — no branch-free hot path. |
| 5. **Kani/formal** | **N/A** — oracle-class property (budget-breach flagged, absence named). |

**wasm-cdylib-stays-green proof (procedure step 9, load-bearing):** the wasm build compiles and the
timing path takes no `Instant::now()` on wasm — either the `performance.now()` import is used or a
named absence is recorded. This is a *shipping-break* guard, not a style check (an unguarded
`Instant::now()` panics the cdylib).

**Budget-constant single-authority proof:** grep confirms `FRAME_BUDGET_US` has exactly one
declaration; the pin test asserts its value (P3 rate discipline).

## 5. Falsifiable acceptance criteria

1. `EngineLoop::frame()` measures frame time against `FRAME_BUDGET_US`; a planted slow frame is
   flagged; p50/p99 are emitted under the `telemetry` feature.
2. `FRAME_BUDGET_US` has one authority site, pinned by a value test.
3. `InferError::Timeout` is reachable from the real ASR timer (a planted slow feed returns it —
   today the variant is unreachable).
4. The wasm cdylib stays green with the stated clock leg (`performance.now()` import or named
   absence); no `Instant::now()` on the wasm path.
5. Frame/voice records carry the item-58 `(FramesRendered, Δframe_us)` pair; ratios are consumer-side.
6. The default (non-`telemetry`) engine build stays offline-clean (`cargo tree -e no-dev`
   byte-unchanged), and every engine hot zone has an `eff` cell (workload-kind or `gap:`).

**Falsifier:** an unguarded `Instant::now()` breaking the wasm cdylib; `InferError::Timeout` still
dead; a frame budget with multiple authority sites or no pin; a ratio field in the record.

## 6. Dependency gates

- **Upstream:** **item 58** (`(work, cost)` pair + `WorkloadKind::FramesRendered`). Transitively item
  57 (procedure) → the FDR merge for any FDR-recorded emission. The engine's own frame-profiler
  timing (step 2) can be developed against the engine's local `FrameProfiler`, but its
  `WorkloadKind`-tagged emission aligns to item 58.
- **Coordination:** **item 62 (wasm clock leg)** — steps 4 here and item 62's wasm clause are **one
  design**. Land the shared `performance.now()` binding once; both the engine and FDR wasm surfaces
  consume it. Do NOT invent two wasm clocks.
- **Crate-boundary note (build model):** the engine is a standalone crate (`engine/`), path-depends on
  `kernel/`. Build/verify with `cd engine && cargo test` and the wasm build per
  `scripts/verify-kernel-engine.sh`. The engine is offline-clean by default — the timer must not pull
  a dependency (std `Instant` / a single wasm import only).
- **Downstream:** none — item 60 is a leaf consumer of item 58.

## 7. Operator-decision points & accepted risks

- **[OPERATOR] The frame-budget value.** `FRAME_BUDGET_US` sets the engine's frame SLO (e.g. 60 fps
  = 16_667 µs vs 30 fps = 33_333 µs). This is a product/UX decision, not an engineering default —
  flagged for the operator to set the number; the *mechanism* (one authority site, pin test,
  breach-flag) is fixed regardless. **Owner:** operator.
- **[OPERATOR] The ASR timeout budget.** The constant that makes `InferError::Timeout` fire is a
  battery-vs-latency tradeoff (the module's own "battery lever" framing). Flagged for the operator
  to set; the wiring (dead variant → live timer) is the engineering deliverable. **Owner:** operator.
- **[ACCEPTED] Timing under a feature flag.** Heavy frame p50/p99 stamps are `telemetry`-gated (G9
  posture); the default engine ships untimed-but-accounted. This is the honest cost posture, not a
  gap. **Owner:** arc lead.
