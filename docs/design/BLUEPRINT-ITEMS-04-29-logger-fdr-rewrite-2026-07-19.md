# BLUEPRINT — Items 4+29 (+§1.2 JsonWriter): Hand-Rolled Logger / Flight-Data-Recorder, `tracing` Pair Retired

**Status:** BLUEPRINT (planning artifact, not execution). Tier-1 keystone item per
`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §B; proof conditions §G.9.
**Mandates bound in:** synthesis §21 (energy/hardware fields first-class from day one),
synthesis §10/P2 (JsonWriter absorbs BOTH `esc()` and the logger's field serializer in the
same change), and `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` (BINDING per its
§3 — the 10-step walk is §5 of this doc).
**Every file:line below was verified against live HEAD (`e10ea4e54`) this session.**

---

## 0. Executive summary

The `tracing`/`tracing-subscriber` pair is the kernel's last-but-one external dependency
surface (with `regex`, item 5). Mechanically verified this session: the default kernel tree is
**25 unique crates** (`cargo tree -e no-dev --locked --offline`); removing the pair drops
**exactly 19 crates** (verified by subtree set-difference — list in §5 step 3), leaving 6
(kernel + regex's 5). The §G.9 proof "drops 13+ crates" is exceeded with margin.

The honest headline: **the call-site surface is far smaller than the item's "largest Tier-1"
framing suggests — the real work is the NEW build, not the cutover.** Total `tracing` API
usage in the entire workspace is ~14 source lines across 7 kernel files (nothing in `engine/`,
`apps/`, `tools/`, or any other crate — swept, §1). The genuinely new work is: the FDR
durable ring + kill-9 recovery, the first-class energy/hardware event schema (no RAPL/joules
code exists anywhere in the kernel today — swept, §3.2), and the JsonWriter consolidation.
A large fraction of the machinery already exists in-kernel and must be REUSED, not rebuilt
(§3): `/proc` CPU/RSS readers, the typed-absence `gpu: Option<GpuSample>` precedent, the
`LogBucket`/`JsonlWriter` metric machinery, and `FileEventStore`'s fsync discipline.

Recommended scope: **single cutover pass for the core** (it is small enough), with four
explicitly deferred surfaces (§7). Cutover is three isolated commits (build → flip → remove),
each revertable alone, per procedure step 6.

---

## 1. Usage map — exhaustive, by pattern

Sweep method: `grep -rn "tracing" --include="*.rs"` over the whole repo (excluding
`.worktrees/`, `target/`), plus all `Cargo.toml` manifests. Result: **the entire tracing
surface lives in `kernel/`**. `engine/Cargo.toml` mentions tracing only in a comment (line
20); no other crate (`agent*`, `mesh-adapter`, `bebop2`, `tools/`, `apps/`) references it.

### Manifest sites
- `kernel/Cargo.toml:134` — `tracing = "0.1"` (resolves 0.1.44). Unconditional, NOT optional.
- `kernel/Cargo.toml:139` — `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
  (resolves 0.3.23). Unconditional.
- `kernel/Cargo.toml:93-102` — the `telemetry` feature comment documenting the
  `SpanMetricsLayer` reuse of the pair (P83).

### Pattern P1 — in-body `info_span!(...).entered()` + `debug!` with structured fields (3 functions, 7 lines)
- `kernel/src/domain.rs:175-181` — `place_order`: span fields `id = %id, n_items = items.len(), channel = ?channel`.
- `kernel/src/domain.rs:183` — `tracing::debug!(subtotal_cents = subtotal, "order subtotal computed")`.
- `kernel/src/domain.rs:219-225` — `place_order_priced`: same span shape.
- `kernel/src/domain.rs:239-242` — `debug!` (catalog-authoritative subtotal).
- `kernel/src/order_machine.rs:161-162` — `fold_transitions` span (`start = ?start, n_steps = ...`).
- `kernel/src/order_machine.rs:167` — `debug!(final_status = ?cur, ...)`.

Uses the `%` (Display) and `?` (Debug) field sigils and the fields-then-message grammar. These
three are the "already-placed spans" the P83 layer consumes. **No `info!`/`warn!`/`error!`
call exists anywhere** — the macro surface actually used is exactly `info_span!` + `debug!`.

### Pattern P2 — `#[instrument]` proc-macro thin wrappers (6 sites, all `#[cfg(feature = "telemetry")]`)
- `kernel/src/span_metrics/instrument.rs:31,44,60,71` — `route`, `commit_after_decide`,
  `decide_settlement`, `cap_verify_chain` (`skip_all, level = "info"`).
- `kernel/src/span_metrics/instrument.rs:84` AND `kernel/src/span_metrics/mldsa.rs:12` —
  **`mldsa_verify` is wrapped TWICE**, two byte-near-identical wrappers emitting the same span
  name, both behind `telemetry`+`pq`. Pre-existing duplication (P2-correspondence smell);
  cutover deletes one (§4.6).

This is the only consumer of `tracing-attributes` → the whole `proc-macro2`/`quote`/`syn`
chain exists to serve 6 one-line wrappers whose bodies are 1:1 forwards.

### Pattern P3 — `tracing_subscriber::Layer` implementation (the P83 `SpanMetricsLayer`)
- `kernel/src/span_metrics/obs.rs:255-299` — `impl Layer<S> for SpanMetricsLayer`
  (`on_new_span`/`on_enter`/`on_close`/`on_record` against `Subscriber + LookupSpan`).
- `obs.rs:236-253` — thread-local `ENTER_AT`/`CURRENT_SPAN_NAME` workaround for a REAL
  self-deadlock in tracing-subscriber's per-span `Extensions` lock (documented in-file; gdb-
  confirmed by the P83 work, landed `67851b2f3`). Note for §5 step 3: this workaround also
  makes the layer **drop the outer span's measurement under nesting** — `on_enter` of an inner
  span clobbers the single thread-local stamp, so the outer's `on_close` finds `None` and is
  skipped.
- Everything else in `obs.rs` is already hand-rolled std-only and carries over UNCHANGED:
  `LogBucket` histograms (`obs.rs:74-137`), `JsonlWriter` (`obs.rs:142-165`), `SpanMetrics`
  registry (`obs.rs:168-215`), `metric.jsonl` row format (`obs.rs:109-122`),
  `normalized_load1()` (`obs.rs:58-71`).

### Pattern P4 — Registry / global-subscriber wiring
- `kernel/src/span_metrics/mod.rs:53-63` — `init()`: `Registry::default().with(layer)` +
  `set_global_default`.
- `kernel/src/span_metrics/mod.rs:78-85` — `init_scoped()`: `set_default` → `DefaultGuard`
  (test-scoped install).

### Pattern P5 — `fmt` + `EnvFilter` dev subscriber
- `kernel/src/lib.rs:386-406` — `init_tracing()` (`#[cfg(not(target_arch = "wasm32"))]`):
  telemetry branch → `span_metrics::init`, else `tracing_subscriber::fmt()` with
  `EnvFilter::try_from_default_env()` fallback `"info"`.
- **Zero production callers.** Repo-wide sweep: `init_tracing` is called only from
  `kernel/tests/span_metrics_init_wire.rs:174`. Neither kernel bin (`lm`, `markov_attractor`)
  calls it. The `env-filter` feature exists to serve a function nothing ships with.

### Pattern P6 — test usage (1 file)
- `kernel/tests/span_metrics_init_wire.rs` — `init_scoped` (:90, :138), synthetic
  `tracing::info_span!("p83_synthetic")` (:143), `init_tracing()` D2 default-branch test
  (:163-184). Feature-split with in-file `#[cfg(feature = "telemetry")]` guards (:76-133).

### Pattern P7 — latent linkage
`tracing` is a non-optional dep, so the wasm cdylib and every rlib consumer link all 19 crates
even though spans are inert without a subscriber. This is the §0.1 "13 transitive crates to
emit a log line" inconsistency in live form.

### Output consumers (the REAL byte-compat surface)
- `tools/telemetry/telemetry:83-108` — `kernel-spans` subcommand: runs a target under
  `DOWIZ_SPAN_METRICS=1` + `DOWIZ_SPAN_METRICS_DIR`, then parses `metric.jsonl` rows and folds
  them into governance metrics ("byte-compatible: kind=<x> + double-serialized sample").
- `tools/telemetry/governance.sh:279` — `GOV_METRIC=$DIR/logs/metric.jsonl`.
- `tools/telemetry/lib.sh:233` — `alert.jsonl` grep-reader.
- `check.sh` + hook consumers of the markov CLI JSON contract (documented
  `kernel/src/bin/markov_attractor.rs:1-12`).
- The stderr `fmt` output has **no machine consumer and no production caller** (P5).

**Totals: 7 distinct patterns; ~14 API lines in `kernel/src` + 1 test file; 8 span names**
(`place_order`, `place_order_priced`, `fold_transitions`, `route`, `commit_after_decide`,
`decide_settlement`, `cap_verify_chain`, `mldsa_verify`).

---

## 2. The §1.2 JsonWriter — finding

**It does not exist yet.** Synthesis §1.2 PROPOSES it (~50-line writer owning escaping at the
write boundary); §10/P2 mandates it absorb *both* hand-rolled JSON sites in the same change.
The sites, located:

1. `kernel/src/bin/markov_attractor.rs:75-88` — `esc()` (escapes `"` `\` `\n` `\r` `\t`),
   applied manually at :46, :51, :60 inside `format!` strings that hand-assemble the CLI's
   JSON contract (exact spacing matters — `check.sh` parses it).
2. `kernel/src/span_metrics/obs.rs:119` — `LogBucket::to_jsonl` escapes the span name via
   Rust's `{:?}` Debug formatting — **a second, different escaping primitive already in the
   tree** (`{:?}` also emits `\u{..}` for non-printables, unlike `esc()`). Live P2 violation,
   exactly the shape §10/P2 predicts.
3. (New, this item) the §0.1 logger's field serializer — the third would-be site.

**Absorption plan:** `kernel/src/fdr/json.rs` hosts the one escaping authority:
- `pub fn escape_into(out: &mut String, s: &str)` — semantics of today's `esc()` (byte-
  compatible for every string the markov CLI can emit; golden-pinned).
- `JsonWriter` builder: `obj().field_str(k, v).field_u64(k, v).field_f64_fmt(k, v, prec)…` —
  field methods that cannot emit unescaped bytes (illegal state unrepresentable, §1.5 applied
  to serialization).
- The markov CLI keeps its exact output format (spacing included) but every escape routes
  through `fdr::json::escape_into` — `esc()` body deleted, call sites byte-identical (proof:
  golden test over the CLI's 12-case `--selftest` corpus + adversarial reason strings).
- `LogBucket::to_jsonl` switches its span-name field to the JsonWriter — byte-identical for
  all 8 real span names (all `[a-z_]`, escaping never fires; golden test pins the exact row).

Role change under absorption: the JsonWriter stops being a markov-CLI nicety and becomes the
**kernel's single JSON write authority** — logger events, FDR records, `metric.jsonl` /
`alert.jsonl` rows, and the CLI all emit through it. (Parse-side JSON remains item 31's
scope — `json_api.rs`/serde carriers are untouched here.)

---

## 3. Existing in-kernel assets the design MUST reuse (verified, not assumed)

### 3.1 Telemetry schema + /proc readers (P08) — two halves, one pre-existing duplication
- `kernel/src/typed_metrics.rs` (335 lines): `ProcCpuSample::sample()` reads
  `/proc/self/stat` (utime/stime ticks, clk_tck, `mono_now_ns()` via `OnceLock<Instant>` —
  :21-27); `MemSample` reads `VmRSS`/`VmHWM` from `/proc/self/status` (:81-84). Pure std,
  degrades to `None` off-Linux.
- `kernel/src/metrics.rs` (543 lines): the closed `LogEvent` enum (:103) with deterministic
  pipe-delimited `to_line()`/`from_line()` parse-or-reject, and the **typed-absence
  precedent**: `gpu: Option<GpuSample> = None` — "typed absence, never a fabricated 0"
  (:57-61), serialized as literal `null` (:151-155). Real production consumer:
  `decision/import.rs:39,89-95` emits `DecisionImportRecord` through this lane (D8).
- **Flag (owed ticket, not this item's fix):** `metrics.rs:38-61` and
  `typed_metrics.rs:28-120` define near-duplicate `ProcCpuSample`/`MemSample`/`GpuSample`
  structs — an internal P2 duplication in the same class as the dual Keccak already filed by
  item 31. The FDR reuses `typed_metrics.rs`'s *readers*; the dedup gets its own ticket.

### 3.2 Energy: nothing exists
Sweep for `rapl|joule|energy_uj|powercap|perf_event|rdtsc` across `kernel/`, `engine/`,
`tools/`: **zero hits** (the only "energy" matches are spectral `graph_energy` math). Joules
is genuinely new code. The only system-hardware read today is `normalized_load1()`
(`obs.rs:58-64`, `/proc/loadavg`, degrades-closed off-Linux) — the pattern to copy.

### 3.3 Durability precedents
- `kernel/src/hydra.rs:923-1063` — `FileEventStore`: append + `sync_all` **before**
  in-memory state claims the event (:1061-1063), typed `StoreError::Sync` on fsync failure
  (`event_log.rs:172-186`). This is the house fsync discipline; the FDR's alarm-class flush
  copies it.
- `kernel/src/spool.rs` — crash-safe pure state machine (append/claim/ack), the
  "producer never blocks" pattern. Not directly reused (FDR is a ring, not a queue) but its
  pure-state/IO-adapter split is the module discipline to match.
- Item-2 finding (roadmap §A): NO production composition root constructs the durable event
  store — so the §5 synthesis line "post-mortem event into the durable event log" cannot be
  fully honored yet; §7 defers that routing behind item 2's fix.

---

## 4. Replacement module design — `kernel/src/fdr/`

Name: `fdr` (flight data recorder) — the logger IS the recorder (synthesis §5: "the hand-
rolled tracing replacement and the FDR share one buffer").

### 4.1 API shape: mechanical rename, tracing's macro grammar subset — NOT a drop-in shim

Decision: **keep tracing's exact macro grammar** (fields-then-message, `%`/`?` sigils, span
name first) implemented as kernel `macro_rules!` — so every call-site change is a path-prefix
rename (`tracing::info_span!` → `fdr::info_span!`, `tracing::debug!` → `fdr::debug!`), a
~13-line mechanical diff. Rejected alternative: a facade crate literally named `tracing`
(true drop-in) — name-shadowing via `[patch]` is harder to audit than a 13-line rename and
would hide the cutover from `grep`; rejected as higher-risk for zero benefit at this call-site
count. Rejected alternative 2: hand-rolled `#[instrument]` proc-macro — would itself require
`syn`/`quote`, defeating the item. The 6 attribute wrappers become one explicit guard line
each (`let _g = fdr::info_span!("route").entered();` as the wrapper body's first line).

Surface (all in `fdr/mod.rs` + `fdr/macros.rs`):
- `Level` (Error..Trace) + `static LEVEL: AtomicU8` — one Relaxed load is the disabled-path
  cost, matching tracing's dispatch-check cheapness. Init from `DOWIZ_LOG` env (level-only
  grammar: `error|warn|info|debug|trace`; default `info`, mirroring today's
  `EnvFilter::new("info")` fallback at `lib.rs:404`). Full `RUST_LOG` target-filter grammar is
  an accepted loss (§5 step 3) — nothing in the repo uses it.
- `fdr::event!(level, fields…, "msg")` + `debug!`/`info!`/`warn!`/`error!` sugar.
- `fdr::info_span!(name, fields…)` → `SpanHandle` with `.entered() -> SpanGuard`.
  `SpanGuard { name: &'static str, t0: Option<Instant> }`; `Drop` computes elapsed and (i)
  notifies the observer, (ii) emits a `span_close` FDR record when a sink is installed.
  Each guard owns its own `t0` ⇒ **nesting is measured correctly by construction** — fixing
  the incumbent layer's outer-span-dropped-under-nesting behavior AND deleting the entire
  `obs.rs:236-253` thread-local deadlock workaround (no registry, no Extensions locks exist
  to deadlock on).
- **wasm32 trap (load-bearing):** `std::time::Instant::now()` panics on
  `wasm32-unknown-unknown`. Today's macros are inert on wasm only because no subscriber is
  installed. The guard must therefore take NO `Instant` when disabled (`t0: None` on the
  fast path — check LEVEL/sink first). A naive always-stamp guard would break the cdylib.
- `fdr::init(config)` (`#[cfg(not(target_arch = "wasm32"))]`, replacing `init_tracing()`):
  installs sink (stderr and/or ring) into a `OnceLock`; uninstalled = macros stay inert.
  `fdr::set_span_observer(…)` — see 4.5. `init_scoped()`-equivalent for tests via a
  thread-local override, preserving the `span_metrics_init_wire.rs` test pattern.

### 4.2 Event schema — energy/hardware first-class from day one (§21)

Every FDR record carries a fixed envelope; `hw` is a **non-optional struct field**, so
schema-level omission is unrepresentable:

```
FdrEvent {
  seq: u64,              // monotonic per-process
  ts_unix_ns: u128,      // wall clock (forensic/display plane per §10/P6)
  mono_ns: u128,         // typed_metrics::mono_now_ns() — replay-ordering key
  level: Level,
  kind: Kind,            // Event | SpanClose | Alarm | PostMortem | Tuning (closed enum;
                         //   Tuning reserved for item 21's FDR-logged adjustments, §16(ii))
  name: &'static str,
  hw: HwStamp,           // FIRST-CLASS — always present, never Option
  fields: …              // via JsonWriter
}

HwStamp {
  cpu_ticks: Reading<u64>,   // utime+stime, /proc/self/stat (typed_metrics reader reused)
  rss_kb:    Reading<u64>,   // VmRSS, /proc/self/status (reader reused)
  joules_uj: Reading<u64>,   // NEW: RAPL /sys/class/powercap/intel-rapl:*/energy_uj (µJ)
}

Reading<T> = Value(T) | Unavailable(Absence)
Absence = NonLinuxHost | NoRaplInterface | PermissionDenied | ReadError | SamplingDisabled
          // closed enum, serialized by name
```

**"Named absence, not silent omission," mechanically:** the serialized event ALWAYS contains
the field — `"joules_uj":12345` or `"joules_uj":{"unavailable":"no_rapl_interface"}`. This
upgrades the existing `gpu: None → "null"` precedent (present-but-empty) by adding the
*reason*. §G.9 proof test: on this host (no RAPL exposure), assert the emitted record
contains the literal `unavailable` reason string — greppable, not a missing key.

**Honest cost control:** stamping 2–3 `/proc`/`/sys` reads per event is µs-scale syscall work;
unconditional stamping would tax hot paths. Policy (part of the schema, not a bolt-on):
`Alarm`/`PostMortem`/`SpanClose`-of-the-8-instrumented-functions get full stamps; high-
frequency `Event`-kind records default to `Unavailable(SamplingDisabled)` — first-class,
truthful, cheap. The RAPL delta (joules-per-span) is computed by the consumer from successive
stamps; the kernel emits raw counters only (same losslessness rule as
`metrics.rs:35-37`'s "CPU-% is a derived consumer concern").

### 4.3 Write path
- Format: one NDJSON line per record via JsonWriter (deterministic field order — fixed by
  builder call order, no map iteration; mirrors `metrics.rs`'s determinism doc).
- Sync: synchronous, lock-scoped small writes (a `Mutex<Sink>`; the producer path is
  format-into-stack-buffer → one `write_all`). No async runtime, no background thread in
  phase 1 — matches the "few hundred lines of std-only Rust" §0.1 estimate and the P83
  best-effort-writer stance (`obs.rs:24-26`: observability never poisons the caller).
- Dev sink: plain stderr lines (deterministic format, no ANSI). NOT byte-compatible with
  tracing-subscriber's `fmt` output — see §6.1 for why that is the honest reading.

### 4.4 FDR durability (tier (b) of synthesis §5) + the kill-9 proof
**Correction to the synthesis, stated plainly:** §5 calls tier (b) "`mmap`-backed file ring —
pure std". **std has no mmap.** A literal mmap needs `libc`/`memmap2` — a new dependency,
disqualified by the item itself. The honest pure-std tier (b):

- **A/B alternating segment files** (`fdr.a.jsonl`, `fdr.b.jsonl`, preallocated cap, default
  1 MiB each): append records to the active segment; on reaching cap, truncate the other and
  switch. Bounded size, last-N-seconds retention, and append-only semantics — simpler to
  prove correct under torn writes than an in-place byte-ring cursor. Each line carries a
  CRC32 suffix (hand-rolled table CRC, ~25 lines — well inside the Keccak precedent class);
  recovery accepts CRC-valid lines only and tolerates exactly one torn tail line per segment.
- **Durability tiers, honestly separated:** surviving `kill -9` (process death) requires only
  that `write(2)` reached the page cache — no fsync needed; that is the §G.9 test. Surviving
  power loss requires fsync cadence: `sync_data` on every `Alarm`/`PostMortem` record and on
  segment switch (copying `FileEventStore`'s sync-before-claim discipline,
  `hydra.rs:1061-1063`), typed error surfaced not swallowed (per the item-2 §10/P4 lesson).
- **Recovery protocol:** `fdr::init` writes a `clean_shutdown` marker on orderly drop; on
  init, if the tail of the newest segment lacks the marker → read back both segments, CRC-
  filter, order by `seq`, and emit a `PostMortem` record (count recovered, first/last seq,
  last 5 event names) into the fresh log. Routing that PostMortem into the durable
  `EventLog` is DEFERRED behind item 2's composition-root fix (§3.3) — the FDR exposes
  `recovered_events()` for that wiring to consume later.
- **§G.9 kill-9 test design (make it CI-stable):** parent spawns a child bin that writes N
  marked events then touches a ready-file; parent waits on the ready-file (not a sleep),
  `SIGKILL`s, re-runs the child in recover mode, asserts ≥ N−1 CRC-valid events recovered and
  a `PostMortem` record emitted naming the recovery.

### 4.5 The SpanMetricsLayer port (P83 keeps working)
The layer's VALUE is 100% hand-rolled already (§1 P3); only its *hook* is tracing-shaped.
Replace the hook with a kernel-owned port:

```rust
pub trait SpanObserver: Send + Sync { fn on_span_close(&self, name: &'static str, dur_us: u64); }
```

`SpanMetricsLayer` → `SpanMetricsObserver`: same `SpanMetrics::record()` →
`LogBucket` → `metric.jsonl` chain, byte-identical rows (golden-pinned). `span_metrics::init`
/`init_scoped` re-wire onto `fdr::set_span_observer`; `DOWIZ_SPAN_METRICS=1` +
`DOWIZ_SPAN_METRICS_DIR` env contract unchanged, so `tools/telemetry kernel-spans` works
without edits. `obs.rs` sheds its `use tracing*` imports, the `Layer` impl, and the
thread-local deadlock block (~80 lines net deletion). `breach.rs`/`pprof.rs` are already
tracing-free — untouched.

### 4.6 Cutover inventory (complete, from §1)
1. `domain.rs` 2 spans + 2 `debug!` → prefix rename (4 lines).
2. `order_machine.rs` 1 span + 1 `debug!` → prefix rename (2 lines).
3. `instrument.rs` 5 wrappers → attribute deleted, explicit guard line added (5×1 line);
   `mldsa.rs` duplicate wrapper DELETED, its one caller (if any — sweep found none beyond
   re-export) pointed at `instrument.rs`'s.
4. `span_metrics/mod.rs` + `obs.rs` → §4.5 port.
5. `lib.rs:386-406` `init_tracing()` → `fdr::init()` (name kept as a deprecated alias for one
   release if desired; it has zero production callers, so a hard rename is also safe).
6. `span_metrics_init_wire.rs` → same assertions against the new init path (the
   hard-join-timeout harness stays — it exists to catch hangs, still valuable).
7. `Cargo.toml:134,138-139` → both dep lines deleted, replaced by the step-9 ruling comment.

---

## 5. The standing procedure, walked (10 steps — BINDING per procedure §3)

Ruling covers both crates; they enter and leave together (subscriber is useless without the
facade; the facade's only consumer configuration is the subscriber).

1. **Trigger:** the zero-dep push — synthesis §0.1's live-violation finding (verified
   `cargo tree`, this doc §0), roadmap §B items 4+29, and the item-1 CI gate whose allowlist
   must shrink 3 → 1 in this change. Named before work: this blueprint.
2. **Sweep:** §1 above — every call site enumerated file:line; this list IS the cutover's
   test surface (procedure's retirement clause). Notable sweep facts: zero usage outside
   `kernel/`; zero `info!`/`warn!`/`error!`; `init_tracing` has zero production callers;
   `env-filter`'s grammar is used by nobody.
3. **Claimed edge, verified in-house:** what the pair actually provides *to this kernel*:
   (a) macro ergonomics at 13 lines; (b) span timing for 8 spans — via a Layer that we had to
   deadlock-workaround (`obs.rs:236-253`) and that mis-measures nested spans (§1 P3); (c) a
   global dispatch check (~one atomic load) — matched by our `LEVEL` load; (d) `fmt` dev
   output nobody ships. Cost, measured: 19 transitive crates (`cfg-if, lazy_static, log,
   matchers, nu-ansi-term, once_cell, pin-project-lite, proc-macro2, quote, sharded-slab,
   smallvec, syn, thread_local, tracing, tracing-attributes, tracing-core, tracing-log,
   tracing-subscriber, unicode-ident` — mechanical set-difference this session), including a
   full proc-macro toolchain serving 6 one-line wrappers. What removal genuinely LOSES,
   honestly: third-party ecosystem interop (any future dep emitting tracing events lands
   nowhere — today that set is empty by construction of the zero-dep push; note sqlx under
   the opt-in `pgrust` feature still pulls `tracing` transitively for itself — that graph is
   out of this ruling's default-build scope and its internal events were never consumed by
   us anyway); `#[instrument]` sugar; `RUST_LOG` per-target filter grammar; span
   hierarchy/context propagation (no current consumer — the incumbent layer is explicitly
   single-span). No loss touches a shipping behavior.
4. **In-kernel alternative compile-checked BEFORE ruling:** execution order mandated below
   (§7 commit 1): `fdr/` lands complete with tests while both systems coexist; the ruling is
   recorded in that commit; call-site flip only after green. Loss accounting: the §5-step-3
   list, quantified (grammar subset; no hierarchy; no ecosystem bridge).
5. **Terminal state: (a) removed outright** — both crates, from `[dependencies]`. Not (b):
   an opt-in tracing feature would keep the `SpanMetricsLayer` fork alive in a feature branch
   of the tree, the exact outcome procedure §3 warns about. Not (c): a logger is not a
   syscall/wire/ABI boundary.
6. **Fallback/rollback:** call sites bind only to kernel-owned `fdr::` macros (the seam) —
   no site names `tracing::*` after cutover, so any future re-adoption is an fdr-internal
   change, not a call-site sweep. Three isolated commits (build / flip / remove), each
   revertable alone; the last release artifact with the pair is the rollback. The proven
   incumbent carries load until commit 2 — and commit 2 lands only with step-7 proofs green.
7. **Test coverage before cutover (all red→green before commit 2):**
   - Golden byte-compat: `metric.jsonl` row for fixed samples (exact bytes, extending
     `obs.rs:362-370`'s shape test); markov CLI JSON over the selftest corpus + adversarial
     escape strings; `alert.jsonl` row shape.
   - Kill-9 readback per §4.4 (the §G.9 proof, CI-stable design).
   - Named absence: RAPL-less host → literal `"unavailable"` reason in the emitted record
     (the §G.9 second proof).
   - Span parity: the 8 spans driven under old layer and new observer → identical row
     *structure* and span-name/count fields (durations differ by definition); the
     `span_metrics_init_wire.rs` D1/D2/D3 assertions green on the new path; nested-span case
     documented as an intentional divergence (new: both spans measured; old: outer dropped).
   - Perf guard: criterion pre/post on `place_order` (bench exists per `Cargo.toml:163-165`
     lane) — disabled-path cost within noise of tracing's dispatch check.
   - Full suite green pre/post, `--features telemetry` on AND off, wasm cdylib builds.
8. **Mechanical absence:** `cargo tree -e no-dev --locked --offline | grep -c tracing` → 0;
   full default tree = exactly 6 lines (kernel + regex subtree) until item 5 lands; command
   written into the Cargo.toml comment at the removal site; item-1 allowlist shrinks
   {regex, tracing, tracing-subscriber} → {regex} in the same commit.
9. **Ruling recorded, three places:** (i) `fdr/mod.rs` module doc — "Why this exists,"
   the verdict, this blueprint's path, the loss list; (ii) `kernel/Cargo.toml` comment where
   the dep lines died — invariant + the step-8 one-liner; (iii) this blueprint gains an
   UPDATE section on landing (deep-dive §6 format).
10. **Reopening trigger (concrete, observable):** a real deployment requirement to export
    kernel telemetry into an external tracing/OpenTelemetry collector, OR a mandatory (not
    opt-in) kernel dependency that requires a live tracing subscriber for its own
    diagnostics. Either event reopens the ruling through this same procedure; nothing else
    does.

---

## 6. Honest risk & complexity assessment

1. **"Log output byte-compatible" is ambiguous — interpretation recorded here, flag to the
   operator.** Byte-compat with tracing-subscriber's `fmt` stderr output is unsatisfiable
   even by tracing itself (timestamps differ every run) and pointless (zero parsers, zero
   production callers of `init_tracing` — §1 P5). The proof is read as: **every
   machine-parsed log artifact byte-compatible** — `metric.jsonl` (parsed by
   `tools/telemetry:83-108` + governance), `alert.jsonl`, and the markov CLI JSON contract.
   That is provable and §5-step-7 pins it. If the operator meant the stderr format too, that
   is a scope change to raise before commit 2 — not silently absorbed.
2. **The item is call-site-small but build-new-medium.** Cutover risk is genuinely low
   (13 lines, 8 span names, 1 Layer port, golden-pinned artifacts). The new-code risk
   concentrates in the segment-ring recovery path (torn-line handling, CRC, marker protocol)
   and the kill-9 test's CI stability — which is why §4.4 specifies a barrier-file design,
   not sleeps.
3. **wasm Instant panic** (§4.1) — the one trap that could break the shipping cdylib. The
   disabled-path-takes-no-Instant rule is load-bearing; a wasm build + a
   place_order-through-wasm test guard it.
4. **Nested-span semantics change** — an improvement (incumbent drops outer spans), but any
   downstream consumer that implicitly relied on at-most-one-row-per-nested-pair would see
   more rows. Sweep found no such consumer; named anyway.
5. **Synthesis §5's "mmap … pure std" is wrong** — corrected here (§4.4) rather than built
   as specified. Tier (b) is delivered with plain std file I/O; nothing of the durability
   guarantee is lost for the kill-9 proof; power-loss durability is fsync-cadence, stated.
6. **Not a risk, recorded as owed tickets:** the dual `mldsa_verify` wrapper (§1 P2, deleted
   in cutover), the `metrics.rs`/`typed_metrics.rs` sample-struct duplication (§3.1, reused
   not expanded, dedup ticket filed separately), and PostMortem→EventLog routing (blocked on
   item 2's composition-root defect).
7. **Concurrent-writer discipline:** Tier-1 execution happens on its own branch (Tier-0 used
   `exec/space-grade-tier0-2026-07-19`); this shared checkout has active writers today —
   the executor must not build in the shared tree.

**No phased-compat verdict needed for tracing's ecosystem:** unlike the worry in the task
brief, full "byte-compatible replacement of tracing's ecosystem" is NOT required — the
kernel consumes a verified-small subset (no error/warn/info events, no hierarchy, no
third-party layers, one Layer we own). A span/instrument *compatibility layer* phase-2 is
unnecessary; the compatibility that matters (parsed artifacts + the 8 span names + env
contract) is fully covered in one pass.

---

## 7. Handoff scope for the Opus executor — definition of done for THIS pass

Three commits, in order, each green and revertable alone:

**Commit 1 — build (no behavior change):** `kernel/src/fdr/` = `mod.rs` (init/Level/sink/
observer), `macros.rs` (tracing-grammar subset), `json.rs` (JsonWriter + `escape_into`),
`schema.rs` (`FdrEvent`/`HwStamp`/`Reading`/`Absence`, reusing `typed_metrics` readers; new
RAPL reader), `ring.rs` (A/B segments + CRC + recovery + PostMortem). All §5-step-7 tests
in-tree and green (golden tests initially pinned against the INCUMBENT's live output).
Ruling text (steps 1–5) in `fdr/mod.rs`. tracing still present; both coexist.

**Commit 2 — flip:** the §4.6 inventory (13 call lines + wrapper rewrite + `SpanMetricsLayer`
→ `SpanMetricsObserver` + `init_tracing` → `fdr::init` + test port + markov `esc()` →
`json::escape_into` + `to_jsonl` `{:?}` → JsonWriter). All goldens still green (now proving
byte-compat across the swap). Duplicate `mldsa.rs` wrapper deleted.

**Commit 3 — remove:** both dep lines out of `Cargo.toml` (+ ruling comment, step-8
one-liner), `Cargo.lock` regenerated, item-1 allowlist shrunk, `cargo tree` 6-line proof
captured in the commit message, blueprint UPDATE section written (steps 8–10).

**Done =** §G.9 verbatim: `cargo tree` drops 19 crates (≥13 ✓); parsed log artifacts
byte-compatible (interpretation §6.1, flagged); kill-9 → restart → recover test green; event
schema shows `hw` first-class; RAPL-less host shows named absence. PLUS procedure §2's
closing line: terminal state (a) reached, step-7 proofs green, step-8 command → 0, step-9
records in all three places, step-10 trigger named.

**Explicitly NOT this pass (do not build):** tier (c) reserved-RAM/ramoops (host-config-
dependent per synthesis §5); PostMortem→`EventLog` routing (blocked on item 2);
`RUST_LOG` per-target filter grammar; span hierarchy/context propagation; any
tracing-facade bridge for opt-in features; the `metrics.rs`/`typed_metrics.rs` struct dedup
(own ticket); item 5 (`regex` — strictly after, roadmap §B).
