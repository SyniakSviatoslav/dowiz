# Sovereign Core MVP — GRAND PLAN (2026-07-06)

Authored by Fable 5 (per D4), for adversarial review by the lead. **Authority: `DECISIONS.md` (D1–D7)
— this plan implements it and may not contradict it.** Grounding: `docs/design/dowiz-core-phase-zero/
{PHASE-ZERO,STEP-3-EXECUTION}.md`, live code on `feat/sovereign-core-phase-zero`.

**MVP target (D1):** a modular hub letting a food-business owner control their own data across their
own channels from one module, on a deterministic event-sourced core — own-channel distribution
(web/QR/social/messaging) + ONE direct 0%-commission checkout + owned customer data. Aggregator
orders = READ-ONLY unified view, later phase. Decentralization (D2) and the grail (D3) enter as
seams only.

**Current-state anchor (do not re-plan):** `dowiz-core` = `rebuild/crates/domain` — 10-status machine
(100-pair sweep green), `Lek(i64)` money, `decide`/`fold`/`replay` kernel (machine-only),
`kernel/{policy,idempotency}.rs` + `codec/request_hash.rs` already relocated (commit `1e02d193` +
staged work in tree), wasm32 + clippy disallowed-methods proven, `scripts/sovereign-gate.sh` exists.
NOT closed: pricing extraction, corridor composition behind `decide`, `Envelope`/event vocabulary,
CI wiring, cargo-deny, `sales_channel` entity.

## Conventions used below

- **Gate (D5):** every step names a deterministic gate that drives the REAL deployed/bound/
  un-bypassed surface, plus a **RED proof** — a demonstrated way the gate fails on the actual defect
  (inject → red → revert → green). No mirror-oracles, no psql-literal proofs, no "the call returned".
- **OP** = protect-path (`.claude/settings*.json`, hooks, `.github/`) — staged as a proposal;
  the operator applies via `cp`. The step's DoD includes the staged artifact + apply instructions.
- **Council:** the mandatory hook is disabled (operator 2026-07-05); council on red-line steps below
  is **plan-mandated** (schema / money / RLS / PII per the serious-gate classes), not hook-enforced.
- **Effort:** S ≤ ½ day · M = 1–2 days · L = 3–5 days (lane-days, haiku-doer where routable).
- Ship discipline applies to every shipping step: feature branch → staging deploy (migrations on
  staging DB FIRST) → proof pasted → prod only on explicit approval/merge.
- **Module placement (BINDING — STRUCTURE-UPGRADE A5):** every NEW hub feature below (1.1 channels
  registry, 1.5 channels surface, 2.1 distribution, 2.4 aggregator-view trait, and any Phase-1/2
  hub module) lands as `rebuild/crates/api/src/modules/<name>/` WITH a `module.toml` manifest
  (schema: STRUCTURE-UPGRADE.md §MODULE-CONTRACT), consuming/emitting only `Envelope<Event>` via
  ports — NEVER importing another module's internals. Each step keeps its own scope/gate/council as
  written; this pins only WHERE it lands, and `scripts/module-integrity.mjs` (armed by A1) enforces
  the boundary mechanically (undeclared `use crate::modules::<other>` reds pre-commit). The pilot
  `modules/channel_attribution/` (A2) is the reference shape.

---

# Phase 0 — Optimization & foundation-sealing

Two parallel tracks. 0a is harness work (no product risk); 0b is core work. **Do not serialize 0a
before 0b** — 0a items that are OP-gated must not block the core arc.

## 0a — Token/harness optimization (D7; ranked cheapest-first; each measured before/after)

Baseline measurement first: capture one representative session's token audit (live prefix +
plumbing share, method per REGRESSION-LEDGER row 81) BEFORE any change; re-measure after each item.

| # | Step + scope | Files | DoD | Gate (deterministic) | OP | Effort |
|---|---|---|---|---|---|---|
| 0a-1 | Trim `enableAllProjectMcpServers: true` → explicit allowlist `repowise`, `codebase-memory`, `playwright-test` (~28 %/session plumbing) | `.claude/settings.local.json:54` (+ staged proposal under `docs/operating-model/proposed-settings/`) | Setting flipped; the 3 servers still resolve; all other connectors absent from a fresh session's tool list | Fresh-session probe: `/context` (or transcript audit) shows only the 3 servers' tools loaded; before/after plumbing-token delta recorded in the token brief. RED: allowlist emptied → repowise call fails → restore | yes | S |
| 0a-2 | Default read-only lanes to `Explore` agent (−18.8 K/lane, per token brief §2 dispatch floor) | `AGENTS.md` (TOKEN ROUTER section) — rule: any read-only fan-out task routes `subagent_type: Explore`, never general-purpose | Rule text landed; next 5 read-only dispatches in the transcript use Explore | Transcript audit over the following work-week: 0 general-purpose dispatches for read-only tasks; per-lane token delta recorded. RED: intentionally dispatch one general-purpose read-only lane → audit flags it | no | S |
| 0a-3 | Ship the 3 prescribed-but-unshipped hook narrowings: `route-request.sh` context-aware (skip when task already classified), `pre-edit-lessons.sh` narrowed to `docs/**`-relevant edits only, `require-classification.sh` scope-narrowed | `.claude/hooks/{route-request,pre-edit-lessons,require-classification}.sh` via `docs/operating-model/proposed-hooks/` | 3 narrowed hooks staged with header apply-instructions; operator applies | Per-hook RED→GREEN fixture (same style as `context-budget-guard`): hook fires on its target case, stays SILENT on the newly-excluded case — both demonstrated with a captured hook-input JSON | yes | S |
| 0a-4 | REGRESSION-LEDGER compact index — a ≤40-line index (id · one-line root · guardrail pointer) so lanes stop paying for the 183-row full ledger | `docs/regressions/LEDGER-INDEX.md` (new) + pointer swap in whatever loads the ledger | Index exists, generated from the ledger (script, not hand-copied); consumers point at the index | `scripts/` generator re-run is idempotent (`diff` clean); token measurement: lane that previously read the ledger now reads the index (delta recorded). RED: delete a ledger row → regenerated index diff shows it | no | S |
| 0a-5 | Apply the already-proven `context-budget-guard.sh` (session-cap recycle; RED→GREEN proven on the real 513 K transcript per ledger row 81) | `docs/operating-model/proposed-hooks/context-budget-guard.sh` → `.claude/hooks/` + `settings.json` UserPromptSubmit entry (per its header / APPLY-SLIM §3) | Hook registered; fires ≥ budget, silent under; never blocks (advisory-directive) | Re-run its existing proof fixtures post-apply: fires at 25 %/200 K on the 513 K transcript, silent at window=10 M, silent on missing transcript | yes | S |
| 0a-6 | Relax `red-line-doubt-gate.sh` / `guard-bash.sh` from word-in-path matching to money/auth LOGIC paths (e.g. `**/pricing*`, `**/auth/**`, `packages/db/migrations/**`, `routes/orders/pg.rs`) — kill false-positive friction that trains ignoring | `.claude/hooks/{red-line-doubt-gate,guard-bash}.sh` via proposed-hooks | Match rules are explicit glob lists, not substring word hits; documented in the hook header | Fixture matrix: (a) edit to `pricing.rs` → fires; (b) edit to a doc whose path merely contains "money" → silent; (c) migration edit → fires. All three captured as RED/GREEN transcript proof | yes | S |

**0a exit gate:** before/after token report appended to
`docs/research/token-economy-comparison-2026-07-05.md` — measured, not estimated (D7). OP items not
yet applied are listed as staged-awaiting-operator; they do not block Phase 0b.

## 0b — SEAL THE CORE (close Phase-Zero for real)

### 0b-1 · Extract `pricing.rs` into the core — the money crown-jewel
- **Scope:** move the 884-line pure money composition (`apply_tax`, `compute_line_total`,
  `compose_total`, `charged_tax`, `compute_order_pricing`, `delivery_fee_for_order`,
  `resolve_delivery_fee`) into `kernel/pricing.rs`, per the proven strangler loop
  (STEP-3-EXECUTION). **Split finding:** `distance_km` (f64 haversine trig, `pricing.rs:308`) is a
  wasm/replay float-determinism hazard — it STAYS in the shell; distance crosses the boundary as
  integer meters (`DistanceM(i64)` or plain `i64` param on the snapshot), so no float arithmetic
  enters the core.
- **Files:** `rebuild/crates/domain/src/kernel/pricing.rs` (new, via `git mv` + boundary edit),
  `rebuild/crates/api/src/routes/orders/{pricing.rs → shim, mod.rs, pg.rs}`,
  `rebuild/crates/domain/Cargo.toml` (no new deps expected).
- **DoD:** module + its `#[cfg(test)]` tests live in core; shell compiles via shim/`domain::kernel::pricing`
  imports; behavior byte-identical (mechanical move except the distance boundary); `distance_km`
  remains in shell with a comment naming why.
- **Gate (D5):** (1) `bash rebuild/scripts/sovereign-gate.sh` green post-move — wasm32 is the purity
  oracle; (2) `cargo test` core + `cargo check` api; (3) **independent money oracle on the real
  surface**: staging POST of a fixture cart (known items/modifiers/tax-mode/tier) → DB totals
  asserted against hand-computed expected values written in the test, NOT computed by calling the
  pricing fn (non-mirror), with `x-dowiz-cutover` asserted to prove Rust serves the route.
  **RED proof:** temporarily flip `charged_tax` to `tax_total` in the moved code → oracle test and
  LC1 unit test go red → revert.
- **Red-line:** yes (money) — doubt-pass + LC1/integer invariants re-proven; council on any
  behavior-affecting deviation from a mechanical move. · **Effort:** M

### 0b-2 · Event vocabulary + `Envelope { seq, at, cause }`
- **Scope:** grow the kernel's alphabet: `Event` gains `Priced { totals… }`, `RefundObligated { amount }`,
  `BindingTerminalized { … }` (matching `policy::LifecycleEvent`/`TransitionEffects` semantics);
  every event is carried in `Envelope { seq: u64, at: Ts, cause: CommandHash }` (`cause` = the
  codec/request_hash canonical hash — the D2 dedupe/ordering/causality seam). `OrderState` grows a
  money snapshot (`Lek` totals) + `BindingState` so `fold` can accumulate.
- **Files:** `rebuild/crates/domain/src/kernel.rs` (or split `events.rs`),
  `rebuild/crates/domain/src/codec/request_hash.rs` (expose `CommandHash`),
  `rebuild/crates/domain/tests/kernel_hard_truth.rs`.
- **DoD:** `fold` total over ALL variants (exhaustive `match`, no wildcard arm — a new variant
  without a fold arm is a compile error, per "forbidden transitions are compile errors");
  serde round-trip stable; `replay` reconstructs status + money + binding from a log.
- **Gate (D5):** exhaustive-match compile gate (no `_` arm in `fold` — clippy
  `wildcard_enum_match_arm` deny on the fold fn) + Hard-Truth replay test extended to the new
  variants + canonical-bytes round-trip property test. **RED proof:** add a dummy variant without a
  fold arm → compile fails; corrupt one serialized field → round-trip test red.
- **Red-line:** touches money types — invariant-guardian pass. · **Effort:** M

### 0b-3 · Compose the corridors behind the single `decide` door
- **Scope:** `decide` composes, in live-handler order: `assert_transition` (machine) →
  `policy::assert_owner_target_allowed` (actor-gate; `Command` grows an `actor` field) →
  `policy::cc1_strand_guard` → pricing/LC1 conservation corridor (for money-bearing commands,
  incl. new `Command::PlaceOrder { cart, snapshot, at, … }`) → emit enveloped events. Corridor
  refusal = `DomainError::CorridorBreach { corridor, … }` (new variant in `error.rs`).
  `TransitionEffects` becomes an internal emission detail, not a public type.
- **Files:** `rebuild/crates/domain/src/{kernel.rs, kernel/policy.rs, kernel/pricing.rs, error.rs}`.
- **DoD:** the scattered pure fns are no longer the public mutation API — `decide`/`fold`/`replay`
  are the only doors (`policy`/`pricing` internals go `pub(crate)` where the shell no longer needs
  them after 0b-5); nothing invented, only composed.
- **Gate (D5):** Hard-Truth Layer 3 — full `states × command-kinds` enumeration: every illegal pair
  returns `Err`, zero panics; conservation property (`total = subtotal + charged_tax + delivery_fee
  − discount`, all components ≥ 0) and LC1 no-double-tax as proptest corridors over the REAL
  composition (arbitrary carts). **RED proof:** comment out the actor-gate call in `decide` → the
  SYSTEM-only-edge enumeration case goes red → revert.
- **Red-line:** money + lifecycle — council review of the corridor order vs the live handler
  (a divergence here IS the mirror-oracle failure mode). · **Effort:** M

### 0b-4 · Hard Truth Layers 1–2 (determinism + replay/totality)
- **Scope:** proptest: (L1) arbitrary `Vec<Command>` → run the decide-loop twice → event logs and
  final states identical by canonical bytes; (L2) every prefix k: `state_k == replay(genesis,
  events[..k])`; `fold` never panics under arbitrary decoded events. `cargo-fuzz` on
  decode→decide→fold = optional ratchet, NOT a gate (lean; Kani likewise deferred).
- **Files:** `rebuild/crates/domain/tests/hard_truth.rs`, `Cargo.toml` dev-deps (`proptest`).
- **DoD:** L0–L3 all green in `cargo test` alone — no staging, no DB (that is the point, per
  PHASE-ZERO §4).
- **Gate (D5):** the property suites themselves; determinism proof by canonical bytes (not `Eq` only).
  **RED proof:** inject a `HashMap` iteration (nondeterministic order) into an event-emitting path
  on a throwaway branch → L1 fails → discard.
- **Red-line:** no. · **Effort:** M

### 0b-5 · Shell flips to `kernel::decide` — the deployed-reality step
- **Scope:** the live Rust transition + create handlers stop calling the corridor fns individually
  and pass through the ONE door; `pg.rs::apply_transition` becomes the interpreter of
  `Vec<Envelope<Event>>` (SQL stays in the shell — SQLSTATE/`classify_pg_error` untouched). This is
  the D5 keystone: without it, `decide` is a mirror-oracle that staging never executes.
- **Files:** `rebuild/crates/api/src/routes/orders/{mod.rs, pg.rs, state.rs shims}`.
- **DoD:** grep-proof: no api call-site invokes `assert_transition`/`assert_owner_target_allowed`/
  `cc1_strand_guard`/`compute_order_pricing` directly — only `domain::decide`; behavior identical
  on the full lifecycle.
- **Gate (D5):** (1) `/reliability-gate` on staging — one real order traced L0–L11 through
  `/s/:slug`, GO verdict; (2) `x-dowiz-cutover` asserted on every exercised route (which stack
  serves it — the routed-reality check); (3) bound-param parity: the interpreter's typed inserts
  exercised via the API with the same values a psql literal would use, DB re-read asserts equality.
  **RED proof (the load-bearing one):** on a proof branch, make `decide` refuse a normally-legal
  edge → the DEPLOYED staging route returns the `CorridorBreach` envelope on a real request —
  proving the bound surface executes the core, not a copy → revert.
- **Red-line:** yes (order lifecycle + money path) — council sign-off before staging deploy.
  · **Effort:** M

### 0b-6 · CI: sovereign gate + cargo-deny, required to merge
- **Scope:** add `rebuild/deny.toml` (bans in the core tree: `tokio`, `sqlx`, `axum`, `reqwest`,
  `rand`, `chrono`/`time`, per PHASE-ZERO §3); extend `proposed-sovereign-core-ci/APPLY.md`'s job
  with `cargo deny check bans` + `cargo test` (already drafted) — `.github/` is protect-path, so
  operator applies; then mark `sovereign-core` a required status check on `main`.
- **Files:** `rebuild/deny.toml` (new), `proposed-sovereign-core-ci/APPLY.md` (extend),
  `.github/workflows/ci.yml` (OP).
- **DoD:** CI job green on the branch; required-check flag set; `cargo deny` runs locally green.
- **Gate (D5):** RED proof on a throwaway branch pushed to CI: (a) add `chrono` to core deps →
  cargo-deny job fails; (b) inject `SystemTime::now()` → gate-2 fails (already proven locally per
  APPLY.md — re-prove IN CI, since "runs locally" ≠ "wired remotely" is exactly the D5 failure
  class). · **Red-line:** no (OP). · **Effort:** S

**Phase 0 exit:** 0a report measured · Hard Truth L0–L4 green · pricing in core · one `decide` door
· shell flipped with staging RED proof · CI required check live (or staged-awaiting-operator with
everything else done).

---

# Phase 1 — The deterministic hub core

### 1.1 · `sales_channel` registry entity (RLS, location-scoped)
- **Scope:** promote channel from a metadata string to a typed entity: `sales_channels` table
  (`id uuid`, `location_id` FK, `kind` CHECK-constrained to the 13-value allowlist mirrored from
  `channel.rs::CHANNEL_ALLOWLIST`, `name`, `token` for attribution links, `active`, `created_at`)
  + owner CRUD API. **Doctrine preserved:** attribution stays write-only
  (`orders.metadata.channel`); the registry NEVER feeds pricing/state-machine/authz decisions —
  it is owner-facing configuration only. Platform vocabulary stays in the shell; no core change.
- **Files:** `packages/db/migrations/*` (new migration — red-line glob), new
  `rebuild/crates/api/src/routes/channels/` module, `rebuild/crates/api/src/routes/orders/channel.rs`
  (allowlist becomes the shared source for the CHECK mirror-parity test).
- **DoD:** migration applied staging-first; `FORCE ROW LEVEL SECURITY`; policy scoped by
  `location_id` through the owner's tenancy; CRUD returns only own-location rows.
- **Gate (D5):** **NOBYPASSRLS behavioral test** against the staging DB: connect as the runtime app
  role (no BYPASSRLS), two fixture locations, attempt cross-location SELECT/INSERT/UPDATE → 0 rows
  / denied — behavioral, not policy-text-read. Allowlist parity test: Rust `CHANNEL_ALLOWLIST` ==
  DB CHECK values (single-source drift gate). **RED proof:** in a rolled-back staging transaction,
  drop the policy → the cross-tenant test goes red.
- **Red-line:** YES — schema + RLS → council (system-architect + breaker) before migration.
  · **Effort:** M

### 1.2 · Persistent append-only event log + replay parity (event-sourcing becomes real)
- **Scope:** `order_events` table — the durable form of the kernel's envelope:
  (`order_id`, `seq` monotonic per order, `at`, `cause_hash`, `payload` = canonical event bytes,
  `content_hash`, `signature` NULLable — columns for 1.4/D2 land in this ONE migration).
  Append-only enforced in the DB: `REVOKE UPDATE, DELETE` from the runtime role + a
  raise-exception trigger. The 0b-5 interpreter dual-writes: fold-columns (existing read model)
  stay authoritative for reads; the log is written in the same transaction.
- **Files:** `packages/db/migrations/*`, `rebuild/crates/api/src/routes/orders/pg.rs`
  (interpreter writes), small `rebuild/crates/api/src/ports/` groundwork for 1.3.
- **DoD:** every mutating order action appends exactly its `decide`-emitted events, same txn;
  `(order_id, seq)` unique; append-only proven behaviorally.
- **Gate (D5):** **replay-parity job** (deterministic script, CI-scheduled + runnable on demand):
  for every touched order on staging, `replay(genesis, decode(order_events)) ==` materialized
  row (status + money read model) — a goal-state re-read of the REAL DB, not a unit test.
  Bound-param/`::cast` parity on the typed jsonb/bytea inserts (insert via API, re-read, compare
  against expected literals). **RED proofs:** (a) UPDATE attempt on `order_events` as runtime role
  → SQL error (append-only behavioral); (b) manually flip one staging order's status column →
  parity job goes red → restore.
- **Red-line:** YES — schema + order-of-truth → council. · **Effort:** L

### 1.3 · Transport-agnostic sync PORT (the D2 seam)
- **Scope:** ONE shell trait — `trait EventSink/EventSource` (or a single `SyncPort`):
  `append(order_id, &[Envelope<Event>]) -> Result<Seq>` + `read_since(scope, seq) -> Vec<…>`.
  Impl #1 = Postgres (wraps 1.2). Impl #2 = in-memory, tests only — it exists to prove the seam,
  not to ship. Explicitly NOT: async-trait forests, generic storage abstraction, retry policy.
- **Files:** `rebuild/crates/api/src/ports/sync.rs` (new), `pg.rs` (implements it).
- **DoD:** the interpreter writes events ONLY via the port; contract-test suite (same assertions)
  passes against both impls — that identity is the "libp2p later = swap not rewrite" proof.
- **Gate (D5):** shared contract suite green on both impls; staging still green via
  `/reliability-gate` (the port refactor cannot regress the real route — `x-dowiz-cutover`
  asserted). **RED proof:** make the in-memory impl drop the last event → contract suite red on
  that impl only. · **Red-line:** no (pure refactor behind 1.2's gates). · **Effort:** S/M

### 1.4 · Signed-event envelope: content-hash live, signature slot dormant
- **Scope:** wire `content_hash` = SHA-256 over the codec canonical event bytes (reusing the
  `request_hash` canonicalization discipline — integer-projected, stable field order) computed in
  Rust at append time. `signature` stays NULL for the whole MVP (D2: slot only; Ed25519/PQC is
  Phase 3 machinery).
- **Files:** `rebuild/crates/domain/src/codec.rs` (canonical event bytes),
  `rebuild/crates/api/src/routes/orders/pg.rs` (populate at append).
- **DoD:** every appended event row carries a verifiable `content_hash`; core exposes
  `canonical_event_bytes` (pure, wasm-clean).
- **Gate (D5):** **independent non-mirror oracle:** a verification query using pgcrypto
  `digest(payload, 'sha256')` over the STORED bytes must equal the Rust-computed stored
  `content_hash` for all rows — two independent implementations agreeing on real data.
  **RED proof:** mutate one stored payload byte in a rolled-back staging txn → mismatch detected.
  · **Red-line:** no. · **Effort:** S

### 1.5 · Owner "control your channels" module surface (read-only first)
- **Scope:** `/admin/channels` in the existing owner dashboard: list registered channels (1.1) with
  per-channel order attribution counts (read from `orders.metadata.channel` — the one sanctioned
  reader per the channel doctrine). Read-only in this phase; CRUD UI arrives with 2.1. i18n al+en.
- **Files:** existing Astro+Svelte admin app (extend, don't create a parallel surface),
  attribution-count endpoint in `routes/channels/`.
- **DoD:** owner sees their channels + counts; other-location channels invisible (RLS end-to-end);
  al/en strings complete.
- **Gate (D5):** Playwright vs DEPLOYED staging (`VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm
  exec playwright test … --reporter=list`), real DOM assertions: create an order with
  `x-channel: qr` via the public API → the dashboard count for `qr` increments — a goal-state
  re-read THROUGH the UI, not an API echo. `x-dowiz-cutover` on the endpoints. **RED proof:**
  point the count query at a wrong column on a proof branch → Playwright assertion red.
  · **Red-line:** no (read-only surface over gated data). · **Effort:** M

**Phase 1 exit:** registry live under NOBYPASSRLS proof · event log append-only with green replay
parity on staging · port contract suite green on 2 impls · content-hash oracle green · owner sees
channels in the deployed dashboard.

---

# Phase 2 — The owner hub MVP (Trojan-horse), behind flags

### 2.1 · Multi-channel distribution artifacts
- **Scope:** per registered channel, generate the owner's distribution artifact: attribution link
  (`/s/:slug?ch=<token>`) + client-side QR render; storefront maps `?ch=` → the `x-channel` header
  on order POST (normalization already hardened in `channel.rs::normalize_channel` — never blocks
  an order). Channel CRUD UI completes 1.5. Feature flag `hub_channels` (default OFF).
- **Files:** admin channels UI (extend 1.5), storefront `/s/:slug` param→header plumbing in the
  existing Astro+Svelte app, QR via a self-contained client lib.
- **DoD:** owner creates a channel → gets link + QR; an order via that link lands with the right
  attribution; malformed/forged `ch` degrades to `other`/`web-direct`, never a failed order
  (design-for-failure).
- **Gate (D5):** end-to-end Playwright vs staging: open `/s/:slug?ch=qr` → place a real fixture
  order → DB `metadata.channel = 'qr'` (goal-state re-read) AND dashboard count increments; forged
  `?ch=<junk>` case asserts `other` and a successful order. **RED proof:** break the param→header
  mapping on a proof branch → attribution test red while order-success still green (proves the
  never-block property is independently asserted). · **Red-line:** no. · **Effort:** M

### 2.2 · The direct checkout through the sealed core (THE MVP centerpiece)
- **Scope:** the single 0%-commission checkout on the existing storefront, order placement flowing
  `Command::PlaceOrder{cart,…}` → `decide` (machine + actor + CC-1 + pricing/LC1 corridors) →
  `Priced`+`Placed` events → interpreter. Cart-token spec-v0 (memory: money-council-gated):
  server-priced cart — the client submits item/modifier IDs + quantities ONLY; every price/total is
  computed server-side by the core; client-supplied money fields are refused (`.strict()` schema).
  Idempotent create via `request_hash` (already core). Flag `hub_checkout` default OFF.
- **Files:** storefront checkout flow (existing), `routes/orders/` create path, core
  `Command::PlaceOrder` (0b-3), cart-token spec doc → implementation.
- **DoD:** a real customer path — menu → cart → checkout → order placed → owner sees it live —
  entirely on owned rails; tampered-price submissions provably ignored/refused; duplicate submits
  collapse to one order.
- **Gate (D5):** the full money battery on DEPLOYED staging, all with `x-dowiz-cutover`:
  (1) **independent money oracle** — fixture carts with hand-computed expected totals asserted
  against the DB (never by re-calling pricing); (2) **adversarial cases** — client-injected
  price/total fields → refused or ignored with correct server total (assert the DB, not the
  response); (3) **idempotency goal-state re-read** — same `request_hash` twice → COUNT(*) = 1;
  (4) conservation invariant SQL check over all staging orders created by the suite;
  (5) `/reliability-gate` GO on the new path. **RED proofs:** tamper-accept bug simulated on a
  proof branch → gate (2) red; double-insert by nulling the idempotency key → gate (3) red.
- **Red-line:** YES — money on a public surface → **money council** (architect + breaker + counsel)
  on the cart-token spec BEFORE code; test-integrity rules (no false-greens) apply in full.
  · **Effort:** L

### 2.3 · Owned customer data in the owner dashboard
- **Scope:** the "own-the-customer" half of D1: customer records (name, phone, consent flags)
  captured at checkout, listed/searchable in the owner dashboard, strictly location-scoped;
  erasure path (owner deletes a customer → gone from every surface). i18n al+en.
- **Files:** migration if a dedicated `customers` table is missing for the rebuild path (red-line
  glob), `routes/customers/`, admin UI extension.
- **DoD:** checkout populates the record; owner lists/searches own customers only; erasure
  complete; consent stored explicitly.
- **Gate (D5):** NOBYPASSRLS behavioral cross-location test (as 1.1); **erasure goal-state
  re-read** — delete → re-read via list API, search API, and the order-detail surface → absent
  everywhere (the D5 erasure oracle); Playwright vs staging for the surface. **RED proof:** leave a
  denormalized copy un-deleted on a proof branch → erasure re-read red.
- **Red-line:** YES — PII + RLS (+ schema if migrating) → council. · **Effort:** M

### 2.4 · Read-only aggregator-view stub (seam, not machinery)
- **Scope:** dashboard tab "All orders" behind flag `aggregator_view` (default OFF): renders the
  unified read-only view over ONE trait (`AggregatorSource::fetch_orders_readonly`) with zero real
  impls (empty-state only). This pins the D1 later-phase seam and the UI contract without touching
  money or ingestion.
- **Files:** admin UI tab, one shell trait + empty impl.
- **DoD:** flag OFF in prod config; ON in staging shows the empty-state; the trait's contract doc
  states the single-money-surface invariant (money + intake NEVER flow through this view).
- **Gate (D5):** config assert in CI that prod config has the flag OFF (deterministic, file-level);
  Playwright: flag ON on staging renders empty-state, flag OFF hides the tab. **RED proof:** flip
  prod config in a PR → CI config-assert red. · **Red-line:** no (read-only, no data source).
  · **Effort:** S

## MVP exit gate (the shippable definition)

All on staging first, then prod on explicit approval: an owner can (1) register channels and print
QR/links, (2) receive a real direct order end-to-end at 0% commission through the sealed core,
(3) see it attributed in their dashboard, (4) own and erase the customer record — with the full
money battery green, `/reliability-gate` GO, replay-parity green, NOBYPASSRLS suites green, and the
sovereign CI check required on `main`. Everything not launch-ready stays flagged OFF.

---

# Phase 3+ — Roadmap (machinery deferred; each seam already makes it swap-not-rewrite)

| Item | Gate to start | The seam already placed |
|---|---|---|
| Aggregator READ-ONLY ingestion (Wolt/Glovo view data) | Operator + money council (single-money-surface invariant, D1) | `AggregatorSource` trait + view UI (2.4); attribution doctrine keeps it decision-free |
| libp2p / mesh transport | D6; demand for offline/multi-node; operator | Sync PORT (1.3): a peer transport is impl #3 of the same contract suite |
| CRDT merge (Automerge/Yjs) | Concurrent offline multi-writer edits of the SAME entity actually observed (D2 — MVP reconciles per-source) | Deterministic `fold` over a totally-ordered log IS the replication primitive; ordering is the transport's problem |
| Per-actor Ed25519 → PQC (Kyber/Dilithium) identity | Security council; D6 | `signature` column (1.4) + canonical bytes (codec) — signing is a shell envelope; kernel keeps consuming plain `Command` |
| Energy-aware consent compute — `ProofOfContribution` (D3 grail) | Phase 3, operator; core NEVER learns what a battery is | Signed-event log: contribution/consent = shell-emitted signed events in the SAME log; extensible `Event` enum with exhaustive-fold gate |
| Canvas/vello UI terminal | Perf demand proves DOM insufficient (D6) | wasm32 CI gate: the same core crate already links for the browser; Canvas = a second subscriber folding the same events into pixels |
| Coq / Aeneas / Kani formal layer | Kani ratchet (0b-4 optional) demonstrates value first; D6 | Pure, total, IO-free kernel is the exact shape formal tools consume; nothing to restructure |

---

# How this plan could break (adversarial self-critique)

- **0b-5 is the riskiest single step and the plan deliberately skips a shadow-compare window.**
  Flipping the live order route onto `decide` in one move relies on staging traffic being
  representative; if it isn't, a corridor-ordering divergence from the old handler reaches prod.
  The lean choice (no dual-run diffing) is defensible but it is a bet — if `/reliability-gate`
  misses a path, this is where a red-line bug ships.
- **Dual-write (1.2) creates two truths, and parity jobs rot.** If replay-parity ever goes
  advisory ("known-flaky, skip"), the event log decays into decoration — the exact failure the
  manifesto's log-primacy exists to prevent. The parity job must be a required check with an owner,
  or 1.2 was theater.
- **"pricing.rs is pure" is partially assumed.** `distance_km` is already a known impurity-adjacent
  hazard (f64 trig); there may be more platform residue (locale/format/serde quirks) discovered
  mid-extraction. Budget exists for a shell-remainder, but if >30% of the file must stay behind,
  the "crown jewel in the core" claim weakens and the conservation corridor tests must be
  re-scoped to what actually moved.
- **Float determinism is only partially fenced.** clippy/deny ban clocks, RNG, and `From<f64>` for
  money — nothing bans general `f64` arithmetic entering `fold` state. One leaked float and
  native-vs-wasm replay silently diverges. Mitigation is type discipline (integer boundaries), which
  is a convention, not a compiler property, until a dedicated lint lands.
- **Phase 0a is hostage to operator-apply latency.** Five of six items are protect-path. The plan
  parallelizes 0a/0b so this doesn't block, but the D7 "measured before/after" claim quietly
  degrades to "staged, unmeasured" if applies lag — and estimated savings from one brief may not
  reproduce.
- **Over-engineering temptations are named but magnetic:** the sync PORT wants to become a storage
  abstraction; the signature slot wants signing; the aggregator stub wants ingestion; the channel
  registry wants to feed pricing/menus per channel (which would breach the write-only attribution
  doctrine and drag platform vocabulary toward the core). Every one of these must lose in review;
  the YAGNI clause only works if enforced.
- **The MVP's center of gravity is 2.2, and it is the longest, most gated step.** Money council +
  cart-token spec + the adversarial battery could exceed all other Phase-2 work combined. Slippage
  pressure will suggest shipping the read-only dashboards as "MVP" — that is NOT the Trojan horse;
  without the checkout there is no 0%-commission escape and the thesis is untested.
- **Sequencing hazard: 1.1's CHECK-constraint mirrors a Rust const.** Two sources for the channel
  allowlist (DB CHECK + `CHANNEL_ALLOWLIST`) drift unless the parity test is required; adding a
  14th channel will fail in whichever surface someone forgets — the gate must fail loudly in CI,
  not in a lane's local run.
