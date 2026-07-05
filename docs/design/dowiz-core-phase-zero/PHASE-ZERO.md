# Phase Zero — `dowiz-core`: the Sovereign Engine blueprint

- Input: `MANIFESTO.md` (this directory). Scope per operator directive 2026-07-05: **Sovereign Core
  only — legacy compatibility is explicitly out of scope for this document.**
- Author role: Systems Architect / Lead Rust Engineer. No approvals modeled; engineering constraints only.
- Repo ground truth verified against `fix/audit-remediation` HEAD, 2026-07-05.

---

## 0. Ground truth — the core is not greenfield

The Sovereign Core already exists in embryo and **already serves production traffic** through the
shell. Phase Zero is a consolidation + kernel formalization, not a blank-slate build. Verified state:

| Asset | Where | Purity status |
|---|---|---|
| 10-state `OrderStatus` machine, exhaustive 10×10=100-pair test | `rebuild/crates/domain/src/order_status.rs` | PURE. No IO. Total. |
| `Lek(i64)` integer money, checked arithmetic, no floats | `rebuild/crates/domain/src/money.rs` | PURE. |
| `EngineError` taxonomy (`DomainError`, `ErrorCode`, envelope) | `rebuild/crates/domain/src/error.rs` | PURE. |
| `TenantId` identity newtype | `rebuild/crates/domain/src/tenant.rs` | PURE. |
| Transition fold-decisions (`TransitionEffects`), actor-gate, CC-1 strand guard, idempotency decision | `rebuild/crates/api/src/routes/orders/state.rs` | PURE — but **trapped in the shell crate**. |
| Money composition (`compute_order_pricing`, `delivery_fee_for_order`, LC1 no-double-tax) — 884 lines | `rebuild/crates/api/src/routes/orders/pricing.rs` | PURE — trapped in the shell crate. |
| Canonical request hashing | `rebuild/crates/api/src/routes/orders/request_hash.rs` | PURE — trapped in the shell crate. |
| SQL interpreter of the fold decisions (`apply_transition`) | `rebuild/crates/api/src/routes/orders/pg.rs:768` | SHELL. Correctly placed. Stays. |

Audited invariants that already hold in the candidate core (`crates/domain`):

- **Zero IO dependencies** — no tokio, no sqlx, no axum. Deps: serde, serde_json, thiserror, uuid,
  utoipa (compile-time schema derive only, documented exception).
- **Zero clock calls. Zero entropy calls** in production code (`Uuid::new_v4` appears only inside
  `#[cfg(test)]`, `tenant.rs:37-65`).
- `unsafe_code = "forbid"`, `clippy::all = deny` at the workspace level (`rebuild/Cargo.toml`).

Conclusion: the Manifesto's Hard Rule №1 ("no side-effects in Core") is already true of
`crates/domain`. What is missing is (a) the trapped pure modules, (b) the **kernel entrypoint**
`decide` / `fold`, (c) the **event vocabulary**, and (d) **mechanical enforcement** of the laws so
purity is a compiler property, not a discipline.

---

## 1. Sovereignty boundary (Extraction Path)

Under the sovereign lens there are exactly three dispositions. Nothing else exists.

| Asset | Disposition | Reason |
|---|---|---|
| `crates/domain/src/{order_status,money,error,tenant}.rs` | **CORE — keep in place** | Already pure math. These files ARE dowiz-core v0. |
| `api/.../state.rs` — actor-gate, `TransitionEffects`, CC-1, idempotency decision | **CORE — relocate** → `kernel/policy.rs`, `kernel/idempotency.rs` | Side-effect-free decisions; their placement in the shell is an accident of the port, not a design. |
| `api/.../state.rs::classify_pg_error` | **SHELL — stays** | SQLSTATE is Postgres vocabulary. Platform words never enter the core. |
| `api/.../pricing.rs` | **CORE — relocate** → `kernel/pricing.rs` | Pure function of (cart, snapshot) → money. The crown jewel; it belongs inside the Law. |
| `api/.../request_hash.rs` | **CORE — relocate** → `codec/` | Canonical bytes of a command = the future signing surface (Phase Three). |
| `api/.../pg.rs::apply_transition` + all repos | **SHELL — becomes the interpreter** | The only legitimate role of IO code: replay kernel output against storage. |
| axum / sqlx / tokio / anything async | **SHELL — forever** | The core has no runtime. `async fn` in core is a rejected concept, permanently. |
| Everything Node (`apps/api`, `apps/web`, `packages/domain/*.ts`) | **OUT OF SCOPE by decree** | Its pure fragments were already transcribed into the Rust modules above. Nothing further is ported. It is not the core's concern. |

---

## 2. Crate blueprint

### Identity

`rebuild/crates/domain` **is** `dowiz-core`. Claim the name with a zero-churn rename:

```toml
# rebuild/crates/domain/Cargo.toml
[package]
name = "dowiz-core"
[lib]
name = "domain"          # import paths unchanged; full lib rename is cosmetic, do it post-cutover
```

```toml
# rebuild/crates/api/Cargo.toml
domain = { package = "dowiz-core", path = "../domain" }
```

Three lines. Identity now, churn never.

### Module hierarchy

```
rebuild/crates/domain/            # package "dowiz-core"
└── src/
    ├── lib.rs                    # THE public surface: State, Command, Event, EngineError, decide, fold
    ├── state/
    │   ├── mod.rs                # OrderState — the immutable aggregate (value object, Clone, Eq)
    │   ├── status.rs             # ← order_status.rs (as-is: matrix is byte-frozen)
    │   ├── money.rs              # ← money.rs (as-is)
    │   └── tenant.rs             # ← tenant.rs (as-is)
    ├── kernel/
    │   ├── mod.rs                # decide(&OrderState, &Command) -> Result<Vec<Event>, EngineError>
    │   ├── policy.rs             # actor-gate, CC-1 strand guard, safety corridors  (← api state.rs)
    │   ├── pricing.rs            # money composition, LC1                           (← api pricing.rs)
    │   └── idempotency.rs        # idempotency decision                             (← api state.rs)
    ├── events/
    │   ├── mod.rs                # Event enum + Envelope { seq, at: Ts, cause: CommandHash }
    │   └── fold.rs               # fold(&OrderState, &Event) -> OrderState — TOTAL, panic-free
    ├── codec/
    │   └── mod.rs                # canonical bytes + CommandHash (← request_hash.rs); rkyv = future additive feature
    └── error.rs                  # ← error.rs; grows EngineError::CorridorBreach
```

### The Law — kernel contract

```rust
/// The ONLY door. Everything the business "does" is a value that passes through here.
pub fn decide(state: &OrderState, cmd: &Command) -> Result<Vec<Event>, EngineError>;

/// TOTAL. An Event is a fact; folding a fact cannot fail. If it could, `decide` was wrong —
/// that is a kernel bug, caught by the replay property, never a runtime branch.
pub fn fold(state: &OrderState, event: &Event) -> OrderState;
```

Note vs the Manifesto's `transition(State, Command) -> Result<State, Error>`: the kernel returns
**events, not the next state**, and the next state is `events.iter().fold(state, fold)`. This is
strictly stronger — it forces the Immutable Log (Manifesto §2 Storage) to be the primary output by
construction. A kernel that returns only `State` lets the log rot into an afterthought.

### The Six Laws (mechanically enforced, §3)

1. **No clock.** `Ts(i64)` millis arrives inside `Command`. `SystemTime::now` is a compile failure in core.
2. **No entropy.** All IDs arrive inside `Command`. `Uuid::new_v4` is a compile failure in core (non-test).
3. **No IO.** No tokio/sqlx/axum/reqwest/rand in the dependency graph — cargo-deny bans, CI-enforced.
4. **State is a value.** No `&mut self` transitions anywhere in the public surface. New state per fold.
5. **Money is `Lek(i64)` checked.** Float arithmetic is a compile failure in core.
6. **`fold` is total.** No `panic!`, no `unwrap`, no `Result` — proven by the replay + fuzz layers.

### Safety Corridors (Manifesto §3 → concrete)

Corridors are `decide`-internal checks that return `EngineError::CorridorBreach { corridor, .. }` —
the system refuses, it does not "work around". Initial corridor set, all already existing as
scattered pure checks, consolidated behind the single door:

- **LC1** — inclusive-tax never double-charged: `total` uses `charged_tax`, never `tax_total`.
- **Conservation** — `total = subtotal + charged_tax + delivery_fee − discount`, every component `≥ 0`.
- **Terminal absorption** — no command produces an event out of DELIVERED / PICKED_UP / REJECTED / CANCELLED.
- **Actor-gate** — machine-legal ≠ actor-legal: SYSTEM-only edges (dispatch-exhausted cancels) refuse owner commands.
- **CC-1 binding consistency** — no DELIVERED/PICKED_UP while a courier binding is active; no orphaned bindings.

---

## 3. Dependency strategy — platform decoupling as a compiler property

**Allowed in core:** `serde` (+derive), `thiserror`, `uuid` (type + serde only), `sha2` (codec),
`utoipa` (compile-time schema derive — the one documented exception). Dev-only: `proptest`,
`serde_json`.

**Banned in core, permanently:** `tokio`, `sqlx`, `axum`, `reqwest`, `rand`, `chrono`/`time`
(timestamps are the `Ts(i64)` newtype — the core does not know what a calendar is).

Enforcement — three ratchets, all deterministic, all CI-fatal:

```toml
# rebuild/clippy.toml
disallowed-methods = [
  "std::time::SystemTime::now",
  "std::time::Instant::now",
  "uuid::Uuid::new_v4",          # core non-test code; tests opt out with #[allow]
]
```

```toml
# rebuild/deny.toml  — cargo-deny
[bans]
deny = [ { name = "tokio" }, { name = "sqlx" }, { name = "axum" },
         { name = "reqwest" }, { name = "rand" }, { name = "chrono" } ]
# scoped to the dowiz-core dependency tree
```

```yaml
# CI job — the single most information-dense gate in this document:
cargo build --target wasm32-unknown-unknown -p dowiz-core
```

If the core compiles to `wasm32-unknown-unknown`, it provably has no OS, no clock, no sockets, no
threads, no Postgres. That one command is the machine-checkable definition of "sovereign" — and it
is simultaneously the Phase One/Two readiness gate (browser = the second interpreter of the same crate).

Interoperability without coupling: the shell (`api` crate) depends on the core; the core depends on
nothing of the shell. Wire formats live in `codec/` behind serde today; `rkyv`/protobuf later is an
**additive feature flag** — core types don't change, an encoder is bolted on.

---

## 4. Hard Truth suite — verification path

Layer 0 exists and stays byte-frozen. Layers 1–4 are the Phase Zero deliverable. Each layer is a
falsifiable assertion — no layer is "review" or "confidence".

| Layer | Property | Mechanism | Status |
|---|---|---|---|
| 0 | Transition matrix total & exact | 100-pair exhaustive sweep (`order_status.rs` tests); fold-effect, actor-gate, CC-1, idempotency unit tests | **GREEN today** |
| 1 | **Determinism** | proptest: arbitrary `Vec<Command>` → run `decide`-loop twice → event logs and final states identical by canonical bytes | to build |
| 2 | **Replay / totality** | every prefix k: `state_k == fold(genesis, events[..k])`; `fold` never panics — proptest + `cargo-fuzz` on decode→decide→fold | to build |
| 3 | **Corridors** | property tests: conservation, LC1, non-negativity, terminal absorption; full `states × command-kinds` enumeration — every illegal pair returns `Err`, zero panics | to build |
| 4 | **Sovereignty** | wasm32 build gate + cargo-deny bans + clippy disallowed-methods, all CI-fatal | to build (1 day) |
| 5 (optional) | Bounded proofs | Kani on `Lek` checked ops and `fold` totality | ratchet, not a gate |

On formal verification (Manifesto Level 3, Coq): the 10-state machine is **already totally proven**
— exhaustive enumeration of a finite relation IS a proof, not a sample. Formal methods only buy
anything on the unbounded parts: the money algebra and fold totality. Kani (bounded model checking)
covers both at ~zero cost; Coq/Creusot stays a Level 3 item and is not on the Phase Zero critical path.

**Phase Zero Definition of Done:** Layers 0–4 GREEN + the three ratchets CI-fatal + `decide`/`fold`
as the only public mutation surface of the crate. No staging, no browser, no database is involved in
this DoD — the core proves itself in `cargo test` alone. That is the point.

---

## 5. Implementation steps — START NOW order

1. **Laws before territory (day 0, zero collision):** add `clippy.toml` disallowed-methods,
   `deny.toml` bans, the wasm32 CI job, and the proptest scaffold to the existing `crates/domain`.
   All four land green against today's code (audit §0 says so) — from then on, purity regressions
   are compile failures, and every later step happens under the ratchet.
2. **Identity:** the 3-line `dowiz-core` package rename. `git mv` the module re-org
   (`state/`, `kernel/`, `events/`, `codec/` skeletons; existing files move, zero logic edits).
3. **Relocate the trapped modules:** `state.rs` → `kernel/{policy,idempotency}.rs` (minus
   `classify_pg_error`, which stays in the shell), `pricing.rs` → `kernel/pricing.rs`,
   `request_hash.rs` → `codec/`. Pure moves; the shell's imports flip; behavior byte-identical.
   *Sequencing note (engineering, not process): these three files are actively touched by the
   in-flight S5 money batch on this branch — land Step 3 after that batch merges, or rebase it on
   top. Steps 1–2 collide with nothing and start immediately.*
4. **Vocabulary:** define the value types —
   `Command` (PlaceOrder, Confirm, StartPreparing, MarkReady, Dispatch, Deliver, PickUp, Cancel,
   Reject, courier Accept/Pickup/Deliver/Abort — each carrying `Ts`, actor, and all IDs),
   `Event` (Placed, Confirmed, Preparing, Ready, Dispatched, Delivered, PickedUp, Cancelled,
   Rejected, BindingTerminalized, RefundObligated, Reconciled), `OrderState` aggregate
   (status + money snapshot + binding state).
5. **The kernel:** implement `decide` by composing what already exists —
   `assert_transition` (machine) → `assert_owner_target_allowed` (actor) → `cc1_strand_guard` →
   corridors → emit events; `TransitionEffects` becomes an internal detail of event emission, not a
   public type. Implement `fold`. Nothing here is invented; it is the existing proven functions
   put behind the single door.
6. **Hard Truth Layers 1–3 red→green.** Phase Zero exits here.
7. **Phase One boundary (out of scope, stated for direction only):** `pg.rs` becomes an interpreter
   of `Vec<Event>`; WS fan-out consumes `Event` directly; the browser gets the same crate via wasm32.

---

## 6. Future-proofing — why no core rewrite later

- **Phase One/Two (Hybrid UI / Canvas Terminal):** the wasm32 gate (§3) guarantees the same crate
  runs in the browser. The Canvas layer is just a second subscriber folding the same `Event`
  stream into pixels instead of rows. "UI is a mirror" (Hard Rule №2) is structural: the UI cannot
  compute truth because the truth-computing functions live in a crate it can only call, not modify.
- **Phase Three (PQC — Kyber/Dilithium):** signing needs stable bytes. `codec/` produces canonical
  command bytes from day one (that is why `request_hash` moves into the core, §1). `SignedCommand`
  is then an **envelope in the shell** — signature verification is a platform concern; the kernel
  keeps consuming plain `Command` values and never changes. The seam already carried now:
  `Envelope { seq, at, cause: CommandHash }` on every event — dedupe + ordering + causality,
  which is everything a signed/replicated transport will need.
- **Phase Three (Mesh):** a deterministic `fold` over a totally-ordered event log **is** the
  replication primitive. Ordering/consensus is the transport's problem (CRDT or otherwise); the
  core's contract — same log in, same state out, on every node — is exactly Layer 1/2 of the Hard
  Truth suite. Nothing to pre-build; determinism IS the mesh-readiness.
- **Agents/Voice (Whisper):** `packages/voice` exists as a future producer. Agents and humans are
  indistinguishable to the kernel — both are `Command` sources. Equal citizens by construction
  (Manifesto Phase Three), zero core impact.
- **YAGNI clause (binding):** no dead envelope fields, no trait forests, no `async` in core, no
  persistent-data-structure crates until a profile demands them. The core stays small enough that
  one engineer can hold `decide` + `fold` in their head. That is a feature, not a limitation.

---

## Constitutional filter (from the Manifesto, restated as the review checklist)

For every future change to this crate, three questions, all must be "No":

1. Does it mutate state in place, or let state exist that is not `fold(genesis, log)`?
2. Does it introduce anything `cargo test` alone cannot verify (clock, entropy, IO, network)?
3. Does it let any consumer (UI, agent, shell) hold a truth the kernel didn't emit?
