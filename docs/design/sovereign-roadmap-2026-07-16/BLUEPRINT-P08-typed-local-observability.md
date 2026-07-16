# BLUEPRINT — Phase 8: TYPED LOCAL OBSERVABILITY (M8 made real, not vacuous)

> One phase of the 19-phase master roadmap (`R2-MERGED-PHASE-ROADMAP.md`). This document plans; it
> writes no code. Canon: `ARCHITECTURE.md` (M8, S7/S8/D7, F29/F31/F32/F36/F39/F40). Primary evidence:
> `R1-C-…-gap-analysis.md` §K3 + §0.4 + §1(S7/S8/M8), `R1-E-…-gap-analysis.md` E46/E47.
>
> **Anchors owned:** M8, S7, S8, D7, E46, E47, F29, F31, F32, F36, F39, F40.
> **Depends on:** Phase 1 (CI truth floor — supplies the claim-latency ledger this phase alerts on).
> **Parallel-safe with:** Phase 5, 6, 7. **SCOPE RULE:** every gate here is a canonical-repo /
> operator-build DEV-TIME + local-runtime discipline; at runtime a hub is a sovereign Hydra (M5/M9/M11)
> and may route its own telemetry however it likes. Nothing here is a global control over other hubs.

---

## 1. Current-state evidence — the M8-vs-Telegram exfiltration contradiction (HEADLINE)

**The load-bearing finding: the only metrics pipeline that exists today actively EXFILTRATES, and it
does so unsigned and untyped — a direct, live contradiction of M8's "NEVER exfiltrated" law, which
canon nonetheless writes as if already satisfied.**

`ARCHITECTURE.md:17` (M8) reads: *"Local-only metrics/logging: CPU+GPU telemetry gathered at-process,
typed+filtered, NEVER exfiltrated (no surveillance). OTel-only-if-operator-opts-in, local sink. LOCK."*
Every clause of that sentence is currently false in practice:

- **It exfiltrates.** `tools/telemetry/lib.sh:146` `resource_sample()` samples host load / mem / disk
  from `/proc`, and `bench_run()` (`lib.sh:186-191`) plus `tg_deliver`/`tg_spool` (`lib.sh:46-65`)
  ship that data to the **Telegram Bot API** — `https://api.telegram.org/bot${TOKEN}/sendMessage`
  (`lib.sh:117,121`), a remote third party — with `CHAT_ID` defaulting to `-1003901655568`
  (`lib.sh:10`). This is off-host egress on a routine work path.
- **It is unsigned.** F40 (signed self-report envelope) is NOT BUILT (R1-C §M8). Anyone who can
  intercept or replay the Telegram traffic can spoof a report; the operator has no way to
  authenticate that a self-report actually came from the hub it claims to.
- **It is untyped.** F32 is only partial. `log_event()` (`lib.sh:69-84`) concatenates arbitrary
  `"k":"v"` pairs into an ad-hoc JSON string, stringifying every value (`"ms":"$ms"`). There is no
  schema and nothing rejects a malformed line.
- **It is opt-OUT, not opt-in.** The only guard is `TELEMETRY_NO_TG=1` (`lib.sh:59,188`) — remote
  egress is the *default*, silence is the exception. That is the exact inversion of M8's
  "opt-in only."

The Rust side is empty where it should be full. `kernel/src/telemetry.rs` is **trigram
pattern-surfacing over tool-outcome tokens** (the self-improvement loop) — **not** process metrics
(R1-C §1 S7/S8). There is **no Rust typed per-process CPU/GPU module** anywhere. **OTel is absent
entirely** (grep-clean) — which is why F29/F39 read GREEN, but *vacuously*: nothing enforces the
local-only default, nothing violates it because nothing is built. F36/E47 (claim-latency anomaly
alert) has **zero hits** in repo or tooling (R1-C §M8, R1-E E47). ML-DSA signing exists only in
bebop2 (R1-C §M8) — so even the reporting that *does* exist cannot be authenticated today.

**Reusable substrate that already exists (this phase is mostly wiring, not green-field):**
- `kernel/src/spool.rs` — a pure, Verified-by-Math crash-safe queue state machine
  (`Record{id,payload,claimed}`, `append→Option<u64>`, FIFO `claim_next`, `ack`, `reclaim`,
  backpressure `is_full`; 6 GREEN tests incl. `crash_reclaim_recovers_inflight`). The module doc
  mandates the exact pattern this phase needs: **pure state in the kernel, I/O adapter (JSONL marshal
  + drainer) outside it** behind the pure-std firewall.
- `tools/telemetry/rust-spool/` (the `telemetry-spool` drainer) + `tools/async-spool/` — existing
  JSONL-file spool adapters to mirror. **No new dependency is required** for the local sink.
- `tracing` + `tracing-subscriber` are already real kernel deps (spans on hot paths, e.g.
  `order_machine.rs:144`) — the S7 half is BUILT; this phase does not rebuild it.
- Phase 1 lands the **claim-latency ledger** (V5-B: per-commit `commit_ts → first_green_claim_ts →
  delta`); this phase *consumes* it (the anomaly detector is the other half of F36/E47).

The resolution strategy, stated once and executed in §5: **do not delete the Telegram path — re-cast
it as F40.** The existing reporting becomes operator-opted-in, ML-DSA-signed self-reporting gated by
an explicit signed marker. That single move makes M8 literally true (no *silent* default egress)
while preserving the operator's visibility.

---

## 2. Typed-metrics module design (S8 / F31)

**Placement.** Split exactly like `spool.rs`: a **pure typed core** (schema + validation, kernel or a
pure-std tool crate, testable, no I/O) and a **`/proc` adapter** (a std-only tool binary — `/proc` is
unavailable in wasm and is I/O, so it lives outside the wasm kernel behind the pure-std firewall).

**Per-process CPU from `/proc/self` (no dependency, `std::fs` only).**
- `/proc/self/stat` — after the `comm` field, `utime` (field 14) and `stime` (field 15) are the
  process's user/kernel CPU in **clock ticks**. Convert with `sysconf(_SC_CLK_TCK)` (typically 100);
  capture the tick rate once at startup. CPU-% is a *derived* consumer quantity from two samples:
  `Δ(utime+stime)/clk_tck ÷ Δwall_seconds`. The module emits **typed samples**, not percentages —
  derivation stays a consumer concern so the raw counters remain lossless.
- Memory: `/proc/self/status` `VmRSS`/`VmHWM` (kB), or `/proc/self/statm`.
- Monotonic wall clock (`std::time::Instant` / `CLOCK_MONOTONIC`) paired with each CPU sample so the
  `Δwall` denominator is immune to wall-clock jumps.

**Struct shapes (design sketch — not code to be written here):**

```
struct ProcCpuSample { pid: u32, utime_ticks: u64, stime_ticks: u64, clk_tck: u64, mono_ns: u128 }
struct MemSample     { vm_rss_kb: u64, vm_hwm_kb: u64 }
struct GpuSample     { util_pct: f32, mem_used_mb: u64, /* … */ }
struct MetricSample {
    ts_unix_ns: u128,
    host_id:    HostId,
    cpu:        ProcCpuSample,
    mem:        MemSample,
    gpu:        Option<GpuSample>,   // None TODAY — typed absence, never a fake 0
}
```

**GPU = typed `Option::None`, deliberately.** There is no GPU on the host and no GPU port yet, so
`gpu` is `None` — a *typed* absence, honest by construction. This is the anti-pattern fix for the bash
path, which would emit `null` or `0` indistinguishably. `Some(GpuSample{…})` is populated only when
the GPU port from the compute phase (Phase 11) materializes; until then the field is unreachable, not
fabricated. F31's "CPU+GPU" is satisfied as "CPU real, GPU typed-none-until-hardware."

**What the module does NOT do:** it does not host-scan (no other-process snooping — that would be the
surveillance M8 forbids); it samples **`/proc/self`** only. `/proc/[pid]` of siblings is out of scope.

---

## 3. Typed log-line schema (F32) + spool-sink design (S8 / D7)

**F32 — a closed, typed schema that REJECTS untyped lines outright.** The contrast target is
`log_event()` (`lib.sh:69-84`), which accepts any `k=v` and stringifies everything. The replacement is
a **closed enum** of known event variants:

```
enum LogEvent {
    Metric(MetricSample),
    ClaimLatency(ClaimLatencyRecord),
    ClaimLatencyAnomaly(AnomalyFlag),
    Bench(BenchRecord),
    // … a fixed, reviewed set; NOT open-ended
}
```

- **Ingest = parse-or-reject.** A candidate line is deserialized into `LogEvent`. If it does not match
  a known variant, or a field has the wrong type (a string where a `u64` is required — precisely what
  the bash path would let through), it is **rejected**: returned as `Err`, counted in a
  `rejected_untyped_total` counter, and the offending bytes are diverted to a `rejects` sink for
  debugging. It is **never silently coerced** and never reaches the metrics JSONL. The rejection is
  therefore *observable and testable* (see acceptance §6.2).
- Serialization is **deterministic** (fixed field order) so two identical samples produce byte-identical
  lines — required by the crash and reproducibility tests.

**Local JSONL sink over `spool.rs` (no new dependency).** Reuse the kernel's existing spool machinery
exactly as its module doc prescribes:

1. **Producer** (the metrics emitter / any subsystem) validates a `LogEvent`, serializes it, and calls
   `Spool::append(line)` — microseconds, fire-and-forget, never blocks the work path. Backpressure:
   `append` returns `None` at capacity (bounded queue; the producer drops-and-retries rather than
   growing unboundedly).
2. **Drainer** (a std-only adapter binary modeled on `tools/telemetry/rust-spool/`) `claim_next()`s in
   strict FIFO, appends the line to the local sink file `…/observability/metrics.jsonl`, **`fsync`s**,
   then `ack()`s. The spool's own records are WAL-marshaled to a spool JSONL so a crash before drain
   loses nothing either (the existing telemetry-spool pattern).
3. **Ordering that makes `kill -9` lossless:** `ack` happens **only after** the sink write is
   `fsync`-durable. Therefore any *acked* record is on disk; any *in-flight* (claimed, not-yet-acked)
   record survives the crash in the spool and is `reclaim`ed on drainer restart
   (`spool.rs::reclaim`, already GREEN in `crash_reclaim_recovers_inflight`). **Zero acked records are
   lost.** This is a wiring-and-fsync-ordering task on proven state-machine code, not new correctness.

**Local-only by construction.** This sink writes to a local file and opens **no socket**. Remote is a
separate, gated adapter (§5) — never the default path.

---

## 4. Claim-latency anomaly alert (F36 / E47 — one build, two anchor names)

F36 and E47 are near-duplicate anchors describing the same feature; this phase builds it **once**.
Phase 1 owns the *ledger* (V5-B appender: per commit `commit_ts, first_green_claim_ts, delta_seconds,
diff_size_lines`). Phase 8 owns the *anomaly detector* that consumes it.

**The pattern to catch (documented, real):** the BRAIN-TOPOLOGY self-certification residue — *"52s
GREEN on a 1610-line diff"* — where a large change is declared verified faster than any real
verification could have run (claim replaces check). The detector encodes a **falsifiable minimum
plausible verification time** as a function of diff size (and, where available, test count):

- A single named, reviewed floor constant — e.g. `MIN_SECONDS_PER_100_LINES` (VERIFIED-BY-MATH style:
  a documented, tunable, reviewable constant, never a magic number). If the recorded `delta_seconds`
  falls below `floor(diff_size_lines)`, raise the flag. Worked example at a 5 s/100-line floor:
  1610 lines ⇒ ~80 s plausible minimum; the recorded 52 s < 80 s ⇒ **FLAG**. (The exact constant is
  tuned against the ledger's own history; the reproduction test in §6.5 pins the 52 s/1610-line case.)
- The flag is emitted as a typed `LogEvent::ClaimLatencyAnomaly` into the **local** sink (M8 — it is
  telemetry, not a remote alert) and can be surfaced at session close.

**Advisory, not blocking** — consistent with the ground-truth-over-proxy and Markov-attractor
posture: the detector *signals* deterministically; it does not gate a merge (that is Phase 6's
signed-verifier job). It answers "was this GREEN claimed implausibly fast?" and records the answer.

---

## 5. Signed-envelope (F40) + opt-in-marker (F29/F39) — THE resolution mechanism

This is where the M8-vs-Telegram tension is resolved. Two mechanisms, both fail-closed.

**(a) ML-DSA-signed self-report envelope (F40).** Every report bound for *any* remote sink is wrapped
in an envelope `{ host_id, sink_kind, issued_ns, body, sig }` where `sig` is an **ML-DSA-65**
signature over the canonical serialization of the other fields. The signer reuses the **existing
zero-dep, ACVP-verified ML-DSA-65 primitive** (the bebop2 / pq crate leg — M2/M6 std-only, already
KAT-checked; per R2 §1.4 the ML-DSA leg is real, not a TODO). Because Phase 8 is parallel-safe with
Phase 6, it must **not** depend on Phase 6's split-identity ceremony: the envelope key is a dedicated
**operator ML-DSA keypair**, private half provisioned via the S3 EnvFile pattern
(`EnvironmentFile=`, never in-repo), public half distributed to whoever verifies. Sharing that
zero-dep pq crate into the dowiz build is an in-repo sovereign-crate wiring (M2/M6-consistent), noted
for the Phase-1 DECART-dep lint — not a new external dependency.

- Verification: the recipient (or a local self-check) verifies `sig` against the operator public key.
  A **single-bit corruption** of `body` or `sig` must fail verification (acceptance §6.4). This makes
  the Telegram report authenticatable — the exact property missing today.

**(b) Explicit opt-in marker for ANY remote sink — precisely how it is checked/enforced.** An env
toggle is too weak (it can be set silently or by accident — the current `TELEMETRY_NO_TG` polarity is
itself the bug). The marker is therefore a **signed capability**, consistent with M12:

- The marker is a file at a well-known path (e.g. `deploy/remote-sink.optin` /
  `/etc/dowiz/remote-sink.optin`) containing an **operator ML-DSA signature over a canonical opt-in
  statement**: `{ host_id, sink_kind, issued_ns, expiry_ns }`. `sink_kind` ∈ `{telegram, otel-remote,
  grafana}`.
- A remote-sink adapter emits **only if ALL hold**, else fail-closed (no egress, metrics stay in the
  local JSONL sink): (1) marker present; (2) signature verifies against the operator public key;
  (3) `now < expiry_ns`; (4) `sink_kind` matches this adapter. Missing / expired / wrong-kind /
  bad-signature ⇒ **zero remote egress**.
- **Default (no marker) = provably zero remote egress.** This is what upgrades F29 ("no remote OTel by
  default") and F39 ("Grafana export opt-in only") from *vacuously* true to *enforced* true.
- The legacy `TELEMETRY_NO_TG=1` is demoted to an additional belt-and-suspenders **local kill**; it is
  no longer *sufficient to enable* remote — only a valid signed marker is. Polarity is inverted from
  opt-out to opt-in.

**Telegram re-cast as F40.** The existing `tg_*` path stays, but each message is (1) wrapped in the
signed envelope and (2) gated by a valid `telegram` opt-in marker. The Telegram chat becomes the
operator's authenticated self-report channel — no longer a silent default, no longer spoofable. M8's
"NEVER exfiltrated" now reads truthfully as "never exfiltrated *without an explicit operator-signed
opt-in*, and every opt-in report is authenticated."

**OTel (F29 / E46).** OTel is wired as **opt-in and local-sink-only by default**, **feature-gated**
(`otel = []`, off by default) so the default `cargo build` dependency graph is **byte-identical** to
today (honest, like the `gpu` Err stub — no `opentelemetry` crates pulled unless the feature is on).
The adapter maps typed `MetricSample`/spans to OTLP into a **local** collector/file sink. A **remote**
OTLP endpoint requires a `otel-remote` signed marker — same gate as Telegram. Landing the
`opentelemetry` crate at all is a DECART item enforced by Phase 1's new-dep lint.

---

## 6. Acceptance criteria (numbered checklist — all falsifiable)

1. **Zero-egress-by-default (the load-bearing test).** With **no** opt-in marker present, run a
   metrics-emitting workload while capturing **all** outbound connections (strace on `connect(2)`, or
   a no-route network namespace, or `tcpdump`/`ss`). Assert **zero** connections to any remote host.
   Then install a valid operator-signed `telegram` marker and assert **exactly one** authorized
   connection (to `api.telegram.org`) and none elsewhere. RED without the fix (today's default egress
   passes traffic), GREEN after.
2. **Untyped line rejected, not coerced.** Feed an untyped / wrong-typed log line (e.g. a `u64` field
   arriving as a string, the bash `log_event` shape) to the schema ingest. Assert it returns `Err`,
   increments `rejected_untyped_total`, lands in the `rejects` sink, and **never** appears in
   `metrics.jsonl`. A well-typed line passes.
3. **`kill -9` mid-write loses zero acked records.** Drive the spool sink, `kill -9` the drainer mid
   write, restart. Assert every record `ack`ed before the kill is present and byte-identical in
   `metrics.jsonl`, and every in-flight record was `reclaim`ed and re-drained — **zero acked loss**.
4. **Signed envelope verifies and fails on 1-bit corruption.** A Telegram/self-report envelope's
   ML-DSA-65 signature verifies against the operator public key; flipping a single bit of `body` or
   `sig` makes verification **FAIL**.
5. **Claim-latency anomaly fires on the documented pattern.** A synthetic reproduction of "52 s GREEN
   on a 1610-line diff" fed through the ledger raises the `ClaimLatencyAnomaly` flag; a plausible-latency
   commit of the same diff size does **not**.
6. **GPU absence is typed.** `MetricSample.gpu == None` on the current host (typed absence, not `0` /
   `null`); the field is populated **only** behind the compute-phase GPU port.
7. **Opt-in fail-closed matrix.** Missing marker, expired marker, wrong `sink_kind`, and a
   bad-signature marker EACH independently yield **zero** remote egress; only an in-date,
   correct-kind, valid-signature marker permits it.
8. **Default build unchanged.** `cargo build`/`cargo test` default dependency graph is **byte-identical**
   to today (OTel and any remote deps feature-gated off; local sink is `std`-only over `spool.rs`).

---

## 7. S7 vs S8 split documentation (per O10; adopted-pending-ratification)

`ARCHITECTURE.md:45` defines S7 and S8 **only** as the joint line *"Observability (S7/S8/D7/M8): local
tracing+typed CPU/GPU metrics; OTel opt-in local-only."* They are never individually defined (R1-C
§2.1). **Operator decision O10 is still open** — no `BLUEPRINT-P02` exists on disk at authoring time,
so Phase 2 has **not** fixed a value. This blueprint therefore adopts the **R1-C/R2 proposed split
verbatim** (it does not invent a competing one) and flags it as pending O10 ratification:

- **S7 = tracing** (spans/events). **Already BUILT** — `tracing` + `tracing-subscriber` (env-filter)
  are real kernel deps with spans on hot paths (`order_machine.rs:144`). This phase does not rebuild
  S7; it inherits it.
- **S8 = typed numeric metrics** — the new per-process CPU/GPU module + typed schema + spool sink
  built in §2–§3. This is the phase's primary deliverable.
- **D7** = the observability *pattern* anchor (the umbrella under which S7+S8 sit).
- **M8** = the governing *law*: local-only, typed+filtered, never exfiltrated without signed opt-in.

**Per-surface mechanism nuance (not a contradiction).** `KERNEL-OBSERVABILITY-DECART-2026-07-15.md`
**rejected** `tracing` for the *bebop rust-core* (the empty-import wasm gate) and chose C-ABI counter
exports + a ring-buffer upgrade path. That is a different crate under different constraints (wasm, zero
external imports), so S7's "tracing" applies to the **dowiz kernel dev/CLI surface**, while
**bebop-core** carries C-ABI counters per its own DECART. This phase records which surface gets which
mechanism and does not force one across both. If O10 is later ratified with a different split, the
labels — not the built artifacts — are what change.

---

## Anchor coverage

| Anchor | Where satisfied |
|---|---|
| **M8** | §1 (contradiction stated) + §5 (signed opt-in makes "never exfiltrated" literally true) |
| **S7** | §7 (tracing, already built, inherited) |
| **S8** | §2 (typed metrics module) + §3 (typed schema + spool sink) |
| **D7** | §7 (observability umbrella) |
| **E46** | §5 OTel opt-in local-sink, feature-gated |
| **E47** | §4 claim-latency anomaly (one build with F36) |
| **F29** | §5 remote OTel denied by default (enforced, not vacuous) |
| **F31** | §2 per-process CPU real, GPU typed-None |
| **F32** | §3 strict typed schema, reject-on-untyped |
| **F36** | §4 claim-latency anomaly alert (one build with E47) |
| **F39** | §5 Grafana/remote export opt-in only (signed marker) |
| **F40** | §5 ML-DSA-signed self-report envelope |

**Boundary — what this phase does NOT do:** it does not build Phase 1's ledger appender (consumed
here, owned there); it does not build Phase 6's split-identity verifier (the anomaly alert is advisory,
not a merge gate); it does not add a GPU adapter (typed-None until the compute-phase port); it does not
host-scan other processes (M8 anti-surveillance). It writes no product code — this is a planning
blueprint only.
