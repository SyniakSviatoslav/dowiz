# BLUEPRINT — Item 48: FDR blind-spot closure — panic forensics + liveness heartbeat

- **Date:** 2026-07-19 · **Tier:** 1-class (reliability seam) · **Status:** BLUEPRINT (planning
  artifact, no code) · **Arc:** §I "Whole-System Determinism & AI-Optional Arc".
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I item 48
  (lines 705–724), items 4+29 (FDR machinery — DONE), item 27 (optional-field discipline);
  `docs/audits/hardening/CHECKLIST.md`. Code ground truth: `kernel/src/fdr/mod.rs`,
  `kernel/src/fdr/schema.rs`, `kernel/src/fdr/ring.rs`, `kernel/src/hub_supervisor.rs`.
- **Dependency status:** after items **4+29 — SATISFIED** (the FDR module exists and was read this
  session). **READY once the FDR branch merges** — the FDR code lives on
  `exec/space-grade-tier0-2026-07-19`; `main` has documents only (roadmap lines 768–771). So item 48
  is *ready-pending-merge*: its prerequisite machinery is DONE but unmerged.

---

## 1. Problem + non-goals

### Problem
The FDR's kill-9 recovery (`fdr/ring.rs`) proves the system recovers *after* process death. It is
structurally BLIND to two failure classes:
- **(a) a panicking process that writes nothing before dying** — the panic unwinds and exits without
  ever appending a record, so recovery finds nothing about *why* it died.
- **(b) a HUNG process that never dies** — it emits no `PostMortem` because it never restarts; the
  one failure class the FDR cannot see. The **k3 span-metrics self-deadlock** (root-caused + fixed
  `67851b2f3`, cited in the roadmap and MEMORY) is the in-repo precedent for exactly this class.

### Non-goals (explicit — roadmap lines 710–718)
- **NOT** a `#[panic_handler]` — this is a `std` kernel; the bare-metal construct does not apply.
  Item 48 uses `std::panic::set_hook`.
- **NOT** register/stack core-dumps — explicitly not pursued.
- **NO** self-kill / self-restart logic in the kernel (`Kernel_Init`-over-`Kernel_Recover`, KISS).
  Liveness JUDGMENT and restart authority stay OUTSIDE the kernel (systemd `WatchdogSec` /
  deployment layer).

## 2. Current-state grounding (verified this session)

### 2.1 The FDR machinery this item builds on (items 4+29, DONE)
- **`Alarm` already fsyncs.** `fdr/ring.rs:134`: `if matches!(ev.kind, Kind::Alarm | Kind::PostMortem)
  { self.file.sync_data()? }`. So an `Alarm`-kind panic record is power-loss durable for free —
  closure (a) needs no new durability code, just an emitter.
- **The record kind is a closed enum.** `fdr/schema.rs:186`: `enum Kind { Event, SpanClose, Alarm,
  PostMortem, Tuning, CleanShutdown }` with `Kind::as_str` at `:197`. Closure (b) adds ONE variant,
  `Heartbeat` — closed-enum growth, the item-27 optional-field discipline (all other records stay
  byte-identical).
- **The stamping constructor + fields vector** exist: `FdrEvent::stamp(seq, level, kind, name,
  StampPolicy, fields)` (`fdr/schema.rs:238`); `fields: Vec<(&'static str, String)>` (`:229`) carries
  the panic message + location.
- **Recovery reads back CRC-valid records** (`fdr/ring.rs:230` `recover`), ordered by seq; a panic
  `Alarm` written before exit is recovered by the same path the kill-9 tests exercise
  (`fdr/ring.rs:333–447`).
- **Post-mortem emission** already exists: `fdr::ring::emit_post_mortem` (`fdr/ring.rs:291`) writes a
  `PostMortem` into a fresh log — but only fires on RESTART after a dirty stop, so it cannot see a
  process that never restarts (the closure-b gap).
- **The FDR sink is never installed on wasm32** (`fdr/mod.rs:52–53,316`). The panic hook + heartbeat
  are non-wasm-only, matching the existing `#[cfg(not(target_arch = "wasm32"))]` gating on the write
  path.

### 2.2 The deploy-granularity precedent for liveness judgment (roadmap line 715)
`kernel/src/hub_supervisor.rs` already owns crash-loop detection at DEPLOY granularity:
`CRASH_LOOP_WINDOW_S = 120` (`hub_supervisor.rs:421`), `CRASH_LOOP_MAX_RESTARTS = 3` (`:422`), and the
`restart_count_since(since_ms) -> u32` port (`:521`, "crash-loop detector input"). This is the model:
the kernel emits evidence (heartbeats), an EXTERNAL layer judges liveness and holds restart
authority. Item 48 does NOT put a timer/judge inside the kernel.

## 3. Options considered (≥2)

**Option A — panic hook emitting one `Alarm` + a `Heartbeat` variant judged externally (RECOMMENDED,
the roadmap design).**
- Closure (a): `std::panic::set_hook` emits one fsynced `Alarm` carrying message + location, chained
  to any prior hook.
- Closure (b): a `Kind::Heartbeat` variant emitted periodically by the HOST loop (seq + progress);
  the external liveness check (systemd `WatchdogSec` / `hub_supervisor`) converts a missed heartbeat
  into the kill-9 crash class the system already survives.
- Concept: *BITE (built-in test equipment)* — the kernel records, the deployment layer judges.
- Tradeoff: minimal kernel surface, KISS, reuses the fsync + recovery machinery. Relies on an
  external watchdog for the hang case (correct — liveness judgment is a deploy concern).

**Option B — an in-kernel liveness thread that self-detects a hang and self-restarts.**
- Concept: *self-healing kernel*.
- Tradeoff: **rejected** — violates the KISS / no-self-kill rule (roadmap line 718), a liveness
  thread cannot reliably observe its own deadlock (the k3 case deadlocked the very machinery that
  would report it), and it duplicates `hub_supervisor` + systemd. Recorded to show it was considered.

## 4. Decision + rationale (ADR-format)

**ADR-048: Option A — panic hook (`Alarm`) + `Heartbeat` variant, liveness judged externally.**

Rationale: closure (a) is a pure add — `Alarm` already fsyncs and recovery already reads it back, so
a `set_hook` emitter closes the "panic wrote nothing" gap with no new durability code. Closure (b) is
one closed-enum variant + an emit fn; the JUDGE stays external because a process cannot reliably
observe its own hang (the k3 precedent proves it), and `hub_supervisor` + systemd `WatchdogSec`
already own restart authority at the right granularity. This keeps the kernel `Init`-over-`Recover`
(no self-kill) and reuses the proven FDR path end-to-end.

## 5. Implementation plan (numbered)

1. **Panic hook (closure a):** add `fdr::install_panic_hook()` (non-wasm), called from the FDR init
   path (`fdr/mod.rs:317` `init`, or a dedicated entry). On panic it builds ONE
   `FdrEvent{ kind: Kind::Alarm, level: Level::Error }` whose `fields` carry `message` (the
   `PanicInfo` payload) and `location` (`file:line:col`), and appends it to the ring sink (which
   fsyncs `Alarm` — `ring.rs:134`). **Chain to the previous hook** (capture `std::panic::take_hook()`
   and call it after) so a test harness's hook is not clobbered. NOT a `#[panic_handler]`. No
   register/stack dumps.
2. **Heartbeat variant (closure b):** add `Kind::Heartbeat` to `fdr/schema.rs:186` and its arm to
   `Kind::as_str` (`:197`). Add `fdr::emit_heartbeat(seq: u64, progress: &[(&'static str, String)])`
   (non-wasm) that writes a `Heartbeat` record carrying a monotonic seq + progress counters. The
   record uses the `Cheap` stamp policy (high-frequency; same rationale as `Event` at
   `fdr/mod.rs:375`). **The kernel provides only the record type + emit fn — the CADENCE and the
   JUDGMENT live in the host / `WatchdogSec` / `hub_supervisor`**, never a kernel timer.
3. **Clean-shutdown interplay:** clean shutdown already writes a `CleanShutdown` marker
   (`ring.rs:161`, fsynced) that makes recovery `clean` (`ring.rs:282`). Item 48 ensures a final
   heartbeat (or the existing `CleanShutdown` marker) means the external liveness check sees an
   orderly stop and raises **no false alarm**.
4. **No self-kill:** `emit_heartbeat` is emit-only; the external layer decides a missed heartbeat is a
   hang and (via `hub_supervisor.restart` / systemd) converts it into the kill-9 class already
   provably survived (`ring.rs` kill-9 tests).
5. **Byte-identity discipline (item 27):** the `Heartbeat` variant is additive; every non-heartbeat
   record serializes byte-identically to before (the `pmu: Option` optional-field precedent,
   `schema.rs:223–228,276–279`). No existing record's JSON changes.

## 6. Failure + degradation (failure-first)

- **Panic during panic-hook execution:** the hook must be panic-safe (avoid allocation-heavy paths
  that could re-panic; a best-effort append, errors swallowed like `emit_event` at `mod.rs:389–393`).
  A double-panic aborts — acceptable, the OS still leaves the fsynced `Alarm` from the first append
  attempt if it reached the page cache.
- **No sink installed:** `emit_heartbeat`/the panic append are no-ops (the `SINK.get()` early-return
  at `ring`/`sink`), exactly like every other FDR emit — zero cost, no crash.
- **wasm32:** no sink is ever installed (`mod.rs`), the hook/heartbeat are gated off wasm — inert.
- **Missed heartbeat (the hang):** degrades to the kill-9 crash class by external restart — a class
  with an existing, tested recovery. No new failure mode introduced.

## 7. Required tests / proofs (per CHECKLIST.md 5-point standard + roadmap 719–724)

1. **Oracle (differential, subprocess):**
   - **Panic-child:** a test child process that panics yields, on `recover()` of its ring dir, an
     `Alarm` record carrying the panic site (red→green: WITHOUT the hook, nothing is recovered — the
     RED). This is the direct analogue of the kill-9 recovery tests (`ring.rs:340`).
   - **Hang-child:** a test child that deliberately hangs (loop, no heartbeat) is flagged by the
     external liveness check (simulated: assert the last `Heartbeat` seq stopped advancing) WHILE
     producing no `PostMortem` — demonstrating exactly the gap closed.
2. **dudect gate:** N/A — no secret-dependent timing. Record `N/A(no-secret-input)`.
3. **Debug cross-check:** the byte-identity assertion (below) is the differential cross-check.
4. **Assembly spot-check:** N/A — not a branch-free crypto path.
5. **Byte-identity (item 27):** all non-heartbeat FDR records serialize byte-identically before/after
   the `Heartbeat` variant is added (assert a corpus of `to_json()` outputs is unchanged).

**Falsifiable acceptance criteria (roadmap 719–724):**
- Panic-child → recovered `Alarm` carrying the panic site; RED (nothing recovered) without the hook.
- Hang-child → flagged by the external liveness check with **zero `PostMortem`** emitted.
- All other FDR records byte-identical (optional-field discipline).
- Clean shutdown → emits a final heartbeat / `CleanShutdown` marker and raises **no false alarm**.

## 8. Security + tenant isolation

- Panic messages can contain arbitrary payload — the hook's `message` field must be treated as
  potentially sensitive; keep it in the local FDR ring (already local, `ring_dir`), never gossiped.
  No PII/menu content is intentionally logged; a panic string is forensic, local-only.
- No tenant/money/auth surface — the FDR ring is a per-node forensic log, outside RLS scope.

## 9. Operability

- **Health (degraded-vs-down):** a missed heartbeat = *hang detected* (down, external restart); a
  recovered panic `Alarm` = *died, recovering* (degraded → recovered). The two are now distinguishable
  in the FDR, which they were not before.
- **Observability (<1 min):** panic → one fsynced `Alarm` with `file:line:col`; hang → heartbeat seq
  flatlines (external watchdog sees it within `WatchdogSec`). Both surface in `alert.jsonl`/the ring.
- **Rollback:** the panic hook is installed at FDR init; not installing it reverts to today's
  behaviour. The `Heartbeat` variant is inert if never emitted.
- **Flag/scaling gate:** heartbeat cadence is host config (`WatchdogSec`), not a kernel flag.

## 10. Open / accepted risks + operator-decision points

- **[GATE — ready-pending-merge] FDR branch merge.** Items 4+29 (the FDR module) are DONE but live on
  `exec/space-grade-tier0-2026-07-19`; `main` has docs only (roadmap 768–771). Item 48 cannot land on
  `main` until that branch merges. *Owner: operator (merge decision).*
- **[OPERATOR-DECISION] Heartbeat cadence + progress semantics.** The interval is a deployment
  concern (systemd `WatchdogSec`); the kernel provides only the record + emit fn. Confirm the host
  owns the cadence and the progress-counter meaning (seq monotonicity + which counters). *Owner:
  operator + deployment layer.*
- **[DESIGN NOTE] Panic-hook chaining vs owning.** Recommend chaining to the prior hook (don't
  clobber a harness hook). Minor; recorded so the executor does not silently replace an existing
  hook. *Owner: item-48 executor.*
- **[ACCEPTED] wasm32 exclusion.** The hook/heartbeat are non-wasm-only (no FDR sink on wasm). The
  wasm cdylib is unaffected. *Owner: item-48 executor.*
- **[ACCEPTED] Panic-in-hook double-panic aborts.** Kept best-effort; the OS retains any bytes that
  reached the page cache. *Owner: item-48 executor.*
