# BLUEPRINT — Roadmap Item 30: State-Machine Proliferation Audit (Tier 0)

> Planning artifact, NOT the audit itself. Scopes the executor's work for
> `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §A "Item 30" / §G.2, per
> `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §22 + Addendum 30.
> Proof condition (verbatim intent): a table, one row per module, citing file:line for
> shared-vs-independent state-machine logic; every independent one gets a
> collapse-or-parity-pin ticket per §10/P2, with RC-4 as the precedent entry format.

## 1. Confirmed real paths (verified this session, main checkout)

The four candidate files and the reference module all live where the synthesis said —
no moves since indexing. (Worktree copies under `.worktrees/p61` and
`.worktrees/p62-catalog-multivendor` exist for `hydra.rs`/`order_machine.rs` only; the
audit targets the main checkout.)

| Module | Path | Lines |
|---|---|---|
| reference | `kernel/src/order_machine.rs` | 1199 |
| candidate | `kernel/src/capability_cert.rs` | 1678 |
| candidate | `kernel/src/hub_provisioning.rs` | 1822 |
| candidate | `kernel/src/hub_supervisor.rs` | 1287 |
| candidate | `kernel/src/hydra.rs` | 1225 |

### Reference pattern — `order_machine.rs` (hardened this session, commit `94c29146b`
"fix(order_machine): const adjacency + idx_of dedup (roadmap item 3)")

- State def: `OrderStatus` enum `:8`; `LIFECYCLE_STATES` const `:199`; canonical `idx_of` `:217`.
- Transition: `allowed_next(from) -> &'static [OrderStatus]` `:78` (const adjacency); `all_edges()` `:173`.
- Proof machinery: `topological_order()` `:243` (Kahn), `FSM_SPECTRAL_RADIUS` `:334`,
  `FSM_GOLDEN_SIGNATURE` `:465`, live-vs-golden verify `:492-499`, `has_cycle()` `:542`,
  cyclomatic number `~:615`, red/green tests `:789+`.

## 2. Preliminary classification (state defs + transition fns read; verdict = executor's)

**Machinery-sharing check (hard finding):** `grep` for `order_machine`, `has_cycle`,
`topological_order`, `GOLDEN_SIGNATURE` across all four candidates → **zero hits**. All
four are preliminarily INDEPENDENT of the shared proof machinery. However, §22's worst
fear — "five copies of the same machinery" — does **not** appear to materialize: these
are four structurally *different* state shapes, not four reinventions of the order FSM.

| Module | State def | Transition logic | Preliminary read |
|---|---|---|---|
| `capability_cert.rs` | `RotationState` `:542-551` (2 data-carrying states: `Stable`, `Overlapping`) | `accepts()` `:556-571` — time-windowed acceptance predicate; rotate/retire transitions described only in doc comment `~:538-540` | INDEPENDENT, but not a graph FSM — a 2-state time predicate. Const-adjacency/cycle machinery structurally inapplicable (states carry data, transition is clock-driven). Likely verdict: no collapse; small parity pin (exhaustiveness + overlap-window boundary test) if anything. |
| `hub_provisioning.rs` | `PoolSlotState` `:152-168` (4 data-carrying states: `Provisioning`, `Warm`, `Claimed`, `Suspended`; "§16.57 no-reclaim" invariant lives in a comment) | **Scattered** inline assignments across four methods: `refill` `:633` (→`Warm` `:658`), `claim` `:757` (guard `:775`, →`Claimed` `:811`), `suspend` `:826` (→`Suspended` `:838`), `resume` `:846` (→`Claimed` `:858-864`) | INDEPENDENT and the **highest-risk shape** of the four: transitions live in call sites with `matches!` guards, no single transition fn, the no-reclaim invariant enforced only by convention. Prime collapse-or-pin candidate. **Plus one preliminary defect candidate — see §4b.** |
| `hub_supervisor.rs` | `Slot` `:433`, `UpdateState` `:442-478` (7 states, data-carrying; doc `:439-441` states promote-without-snapshot/health is UNREPRESENTABLE per §5.1) | Pure decide fns: `decide_promote` `:536`, `decide_rollback` `:568` (+ `PromoteStep` `:526`, `RollbackStep` `:559`); M8 kernel driver below `~:585` | INDEPENDENT but *principled*: a linear pipeline enforced by type-level unrepresentability + pure functions ("no I/O, fully testable (item 3)" — its own comment already cites the order_machine hardening item). A DAG chain; graph machinery arguably overkill. Likely verdict: parity pin (a test asserting the linear order is cycle-free / exhaustive), not collapse. |
| `hydra.rs` | `OrganismState` `:76-80` (2 states: `Live`, `Locked`) + `HysteresisBand` `:85-93`, `INTEGRITY_BAND` `:94` with **compile-time const asserts** `:103-115` | `integrity_check()` `:219-241` — ρ-driven hysteresis flip (trip in one check; release after `healthy_checks` consecutive healthy samples) | INDEPENDENT — a 2-state hysteresis toggle. It already carries its own proof machinery (const asserts on band invariants) and consumes `crate::spectral::spectral_radius` on the *topology* graph — spectral machinery reused, just not the FSM proof kit (which doesn't fit a 2-state toggle). Likely verdict: no collapse; note const-asserts as the in-situ parity pin. |

## 3. EXACT proof-table schema the executor must fill

One row per module (4 rows + `order_machine.rs` as row 0 for the shared baseline).
Columns, in order:

| # | Column | Content rule |
|---|---|---|
| 1 | `module` | filename, `kernel/src/…` |
| 2 | `state-def` | `file:line-line` of the enum/struct defining states |
| 3 | `transition-fn` | `file:line` of EVERY function that assigns/returns a new state (scattered sites each cited — no "various") |
| 4 | `shared-or-independent` | `SHARED` only with a cited line routing through `has_cycle`/`topological_order`/`FSM_GOLDEN_SIGNATURE`; otherwise `INDEPENDENT` |
| 5 | `ticket` | required iff column 4 = `INDEPENDENT`. RC-4 precedent entry format (synthesis `:237`): **finding-id · the independent construct · file:line · resolution** = either `COLLAPSE(→ shared machinery, named target fn)` or `PARITY-PIN(named test + stated forcing reason why collapse doesn't fit)` |

Acceptance: every cell cited by file:line against the live tree (not this blueprint —
re-verify, lines may drift); a `PARITY-PIN` without a forcing reason is a fail;
"structurally inapplicable" (e.g. 2-state data-carrying predicate) IS a valid forcing
reason when stated and cited.

## 4. Prior-findings check

**(a) The phrase "state-machine proliferation with 2 confirmed silent defects" —
UNVERIFIED, do not repeat as fact.** Searched: `docs/design/**` (both SPACE-GRADE docs
+ all other design docs), `git log --all --grep` for "silent defect" and
"proliferation", and the memory corpus. **Zero occurrences found anywhere.** The
synthesis §22 itself is explicit it verified *existence only* ("Contents were **not
read**; nothing is asserted"). If that phrase circulated in-session, it has no written
source; the executor must treat any defect count as unestablished until row-by-row
confirmed.

**(b) One PRELIMINARY silent-defect candidate found while scoping this blueprint**
(needs executor confirmation with a red test, not asserted as confirmed):
`hub_provisioning.rs` `resume()` `:846-866` sets
`slot.state = Claimed { owner: OwnerId([0u8;32]) }` (`:858-860`) and then attempts to
restore the real owner by matching `Suspended` on the *already-overwritten* state
(`:862-864`) — that `if let` can never match, so the restore is dead code and a
resumed hub's owner appears to be left zeroed. The existing test
(`green` resume test `~:1634`) asserts only `matches!(…, Claimed { .. })` — it never
checks the owner id after resume (the suspend test `:1655` does check owner, resume
does not). Executor: write the red test first; if it fires, this is ticket #1.

**(c) RC-4 precedent** (ticket format source): synthesis `:237` — engine's mirrored
`DriftClass` / `dt = 0.016` vs `DT_STABLE = 0.02` / own L-operator, each cited
file:line with a move-or-pin resolution. Item 17's table takes those as its first
three entries; this audit's tickets use the same shape.

## 5. Handoff note (executor)

1. **Index row (lead integrates — shared hot file, concurrent writers today):** per
   `CORE-ROADMAP-INDEX.md` `:213` maintenance rule ("new planning doc ⇒ one new row"),
   add — ready to paste:
   `| Item 30 — state-machine proliferation audit (blueprint) | CORE | [BLUEPRINT-ITEM-30-state-machine-audit-2026-07-19.md](BLUEPRINT-ITEM-30-state-machine-audit-2026-07-19.md) | Tier-0 read-only audit blueprint: 4 candidate modules pinned by file:line, all preliminarily INDEPENDENT of order_machine proof machinery (94c29146b baseline); proof-table schema fixed; 1 preliminary silent-defect candidate (resume() owner-zeroing, hub_provisioning.rs:858-864) awaiting red-test confirmation; "2 confirmed silent defects" phrase found UNSOURCED |`
   This row was deliberately NOT added by the blueprint author (fan-out discipline:
   registration is the lead's integration point).
2. Fill §3's table against the **live tree** — re-cite every line, don't inherit §2's.
3. §4b red test first (cheapest possible truth about the only defect candidate).
4. Tickets per §3 column 5; RC-4 format. Expected outcome from the preliminary read:
   0 collapses forced, 1–2 parity pins, hub_provisioning the only module where a real
   restructuring (single transition fn) is worth arguing for.
5. Audit is read-only except tests/tickets; any code change routes through the normal
   gate (order_machine golden signature must stay green — `94c29146b` baseline).
