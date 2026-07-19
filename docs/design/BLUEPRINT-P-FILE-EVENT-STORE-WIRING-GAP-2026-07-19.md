# BLUEPRINT P-FILE-EVENT-STORE-WIRING-GAP — the durable event store has no production composition root (2026-07-19)

> **Standalone WIRING-CORRECTNESS defect (dowiz-kernel `kernel/src/hydra.rs` / `kernel/src/event_log.rs`).**
> Closes SPACE-GRADE roadmap Item 2 as a filed defect. Verification source (re-confirmed adversarially
> against live `HEAD`, not trusted): `BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md`.
> Original finding: `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §1.4/§9.2/§10-P4
> ("a wired store that swallows IO is arguably worse than an unwired one"). Format precedent:
> `BLUEPRINT-P91-kem-ring-correction-2026-07-19.md`, `BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md`.
> Tier 0, read-only audit — **this doc files the defect and scopes the fix; it does NOT build it.**
>
> **One sentence:** the durable `FileEventStore` is fully implemented and now IO-safe (sub-question (b)
> PASSES — the H1 fail-open fix landed), but it is **constructed in zero production code paths** — every
> one of its six construction sites is inside a `#[cfg(test)]` module or an integration-test binary, and
> **no binary crate builds any durable `Hydra`/`EventLog` composition root at all** (sub-question (a)
> FAILS). A correct, tested, unreachable audit trail is materially different from "not yet built": the
> code exists and passes, so a reader assumes durability is available, when in fact any running binary
> would have to wire it first.

---

## VERDICT (stated up front, per session research discipline)

**FILE-AS-DEFECT — real wiring gap, latent not live. (a) FAILS, (b) PASSES.** Independently re-verified
this session against live `HEAD` `02ab817c9` (note: the source blueprint was written at `e10ea4e54`;
`main` has advanced under concurrent writers, but every cited line still matches).

- **(b) `Result`-typed insert — PASS (independently confirmed).** `EventStore::insert` at
  `kernel/src/event_log.rs:188` returns `Result<(), StoreError>`. The `FileEventStore` impl
  (`kernel/src/hydra.rs:1036`, inside `impl EventStore for FileEventStore` at :1032) routes **every** IO
  step through a typed error and `?`: `.open(...).map_err(|e| StoreError::Open(...))?`,
  `.write_all(...).map_err(StoreError::Write)?`, `.flush().map_err(StoreError::Flush)?`,
  `.sync_all().map_err(StoreError::Sync)?`. The in-memory index (`self.by_id.insert / self.tip /
  self.count`) advances **strictly after** `sync_all()?` succeeds (the H1 §2.2 ordering invariant,
  hydra.rs:1062-1067). There is **no `let _ =` on any IO call** in the insert body — the only textual
  `let _ =` in that region is inside an explanatory comment at hydra.rs:1051. Regression test at
  `kernel/src/hydra.rs:1188-1218` (`mod file_store_tests`) points the store at an unwritable path and
  asserts `Err(StoreError::Open(_))` — a durability failure now surfaces instead of a fabricated
  `Committed`. The fix is real: commit **`4dec042186280b40c161aa1411079670cbbe28fe`** —
  "fix(hermetic): H1 event-log fail-open fix + H2 kernel<->engine mirror-pin sweep" (Jul 16 2026,
  SyniakSviatoslav), touching `kernel/src/event_log.rs` (+185) and `kernel/src/hydra.rs` (+133),
  among others. **Nothing to do for (b).**

- **(a) Production wiring — FAIL (independently confirmed).** `FileEventStore` (struct at
  `kernel/src/hydra.rs:923`, `::open` at :934) is **never constructed outside test code.** All six
  construction sites, verified by full-workspace grep:

  | Site | Classification |
  |---|---|
  | `agent-adapters/tests/e2e_admission.rs:92` | integration-test binary (under `tests/`) — not production |
  | `kernel/src/hydra.rs:1107` | inside `#[cfg(test)] mod file_store_tests` (attr :1084, mod :1085) |
  | `kernel/src/hydra.rs:1123` | same test module |
  | `kernel/src/hydra.rs:1154` | same test module |
  | `kernel/src/hydra.rs:1175` | same test module |
  | `kernel/src/hydra.rs:1207` | same test module |

  Every hydra.rs site sits at a line number **greater than 1085** — the `mod file_store_tests` boundary.
  No production line constructs the durable store. Confirmed further: **no binary crate constructs any
  `EventLog`, `Hydra`, or `FileEventStore` at all** — `agent-loop/src/main.rs`,
  `tools/native-spa-server/src/main.rs`, `tools/ci-truth/src/main.rs`, `kernel/src/bin/lm.rs`,
  `kernel/src/bin/markov_attractor.rs`, and the telemetry/eqc-rs tool binaries contain zero references
  to `EventLog`/`Hydra`/`hub_supervisor`. **There is no durable composition root anywhere in the
  workspace.** This exactly matches the source blueprint's §4 gotcha: "no hub binary at all builds a
  `Hydra`/`EventLog`."

**Anti-scope:** this doc does not construct the composition root, does not choose which binary owns it,
does not touch money/auth/RLS/orders (the event log is upstream of all of them but this change adds no
handler), and does not alter `MemEventStore`'s status as the correct in-memory default for tests.

---

## 1. Adversarial correction to the source blueprint (the one place it overstated)

The verification blueprint's §2(a) states: *"The only NON-test store construction in the repo is
`MemEventStore`: `kernel/src/hub_supervisor.rs:366` (`MemStateStore::new`, production code — its
`#[cfg(test)]` starts at :710)."* This is **imprecise, and the correction strengthens the FAIL.**

`kernel/src/hub_supervisor.rs:366` — `log: EventLog::new(MemEventStore::new())` — is inside the **body**
of `MemStateStore::new()`, a `pub fn` defined outside the `#[cfg(test)]` boundary (which is at
hub_supervisor.rs:710). But every **call site** of `MemStateStore::new()` is inside that test module:
hub_supervisor.rs:998, 1009, 1176, 1253 — all `> 710`. `MemStateStore` is a **reference/stub**
`StateStore` impl, not a live production store: it carries a `restore_called: bool` test-observation
flag (struct at hub_supervisor.rs:352), and its own doc comment says "the real impl re-folds the
projection from the log under the old code." So it is never constructed on a genuinely-reached
production path either.

Net: there is **no production construction of ANY concrete `EventStore`** in the workspace — not the
durable `FileEventStore`, and not even `MemEventStore` via a truly-invoked path. The only two
concrete-or-generic constructions in non-`cfg(test)` `pub` bodies are library plumbing no running binary
calls: `Hydra::new` at hydra.rs:203 (generic over its `store` parameter — it picks nothing), and
`MemStateStore::new()` at :366 (a stub, called only from tests). The defect shape is precisely and
only: **no durable composition root exists.**

## 2. Why it matters (latent, not live — but a real trap)

- **Correct + tested + unreachable ≠ not-built.** The naive reading of "the durable store has a
  regression suite and passes" is "durability is available." It is not. A future author wiring the hub
  who greps for `FileEventStore`, sees green tests, and assumes they can rely on an existing durable
  path would be wrong — they must build the composition root first. The gap is invisible to the test
  suite by construction (tests *are* the only callers).
- **It is genuinely latent, not an incident.** With no binary constructing any store, nothing is
  currently persisting to `MemEventStore` in production and silently losing data on restart — because
  nothing is running the kernel as a durable service at all yet. There is no data to migrate, no
  incident to declare. This is a **fix-before-wiring** item, in the same latent-defect class as P91.
- **It is the highest consequence-per-cost item in the roadmap's own Tier 0** (SPACE-GRADE §A) — the
  fix is small and the failure mode (a hub that boots on a non-durable store and loses the audit trail
  across restarts) is severe, precisely because (b) already guarantees that *if* wired, the durable
  store will surface IO failures honestly rather than swallow them.

## 3. Scoped recommendation — a follow-up build item (NOT this doc's scope)

File a Tier-1 build item (SPACE-GRADE roadmap §B territory) — deliberately separate scope from this
Tier-0 audit. Because (b) is already closed by `4dec04218`, the remaining work is wiring-only, which
shrinks it well below a from-scratch build. Recommended shape:

1. **Pick the composition root.** The first binary that runs the kernel as a durable service owns it.
   Today no such binary exists; the natural home is the hub supervisor entry point when it is built
   (`hub_supervisor.rs` already defines the `StateStore` port and the `Hydra` it would drive). Do **not**
   retrofit a store into a tool binary (`ci-truth`, `native-spa-server`, telemetry spools) that has no
   durability contract.
2. **Config the log path.** A single config value (event-log path) threaded into
   `FileEventStore::open(path)` at that root. Fail closed if the path is unwritable — (b) already makes
   `open` return `Err(StoreError::Open(_))`, so the root must propagate it, not `.unwrap()` it.
3. **Keep `MemEventStore` the test/default in-memory store.** The generic `Hydra::new(store, ...)` /
   `EventLog::new(store)` seam already supports injecting `FileEventStore` in production and
   `MemEventStore` in tests with zero API change — the wiring is purely a choice at the root.
4. **Prove it red→green.** A wire test that boots the real root, writes N events, kills the process,
   restarts from the same path, and asserts the chain tip is recovered (the `e2e_admission.rs` and
   hydra.rs `mod file_store_tests` boot-reopen tests are the ready-made pattern — promote one to run
   against the real composition root instead of an ad-hoc `TempDir`).

**Sequencing note:** the SPACE-GRADE roadmap places the breaker (Item 9, `kernel/src/breaker/`) as its
pivot and says Item 9 is "best entered after item 2's finding." This defect **is** that finding: the
durable-root wiring is a clean prerequisite candidate to bundle with, or immediately precede, the
breaker's own composition work, since both live at the same not-yet-existing hub entry point.
