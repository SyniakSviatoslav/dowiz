# Sovereign Core MVP — Grounding Analysis (2026-07-06)

Three parallel read-only analyses that ground `DECISIONS.md` + `GRAND-PLAN.md`. Preserved verbatim-faithful
so a fresh session inherits the evidence without re-running the agents.

---

## A. dowiz-core + multi-source hub — build-state (reality map)

**Crate:** `rebuild/crates/domain` — package renamed `dowiz-core`, lib name stays `domain`
(`crates/domain/Cargo.toml`; commit `012d9d36`). It IS a pure deterministic crate (no side-effects), and
this is mechanically proven.

**Modules** (`src/lib.rs`): `order_status`, `money`, `error`, `tenant`, `codec` (+`codec/request_hash`),
`kernel` (+`kernel/policy`, `kernel/idempotency`). Public door: `decide, fold, replay, Command, Event,
OrderState, Ts` + `Lek`, `OrderStatus/assert_transition`, `ErrorEnvelope`, `TenantId`.

**Order state machine** (`order_status.rs`) — 10 statuses:
`Pending→{Confirmed,Rejected,Cancelled}` · `Confirmed→{Preparing,InDelivery,Cancelled}` ·
`Preparing→{Ready,Cancelled}` · `Ready→{InDelivery,PickedUp,Cancelled}` ·
`InDelivery→{Delivered,Cancelled,Ready}` · `Delivered/Rejected/Cancelled/Scheduled/PickedUp` = terminal.
`Scheduled` = scaffold/unreachable. Same-status→`SameStatus`; scaffold→`ScaffoldDisabled`. Verified by an
exhaustive 10×10=100-pair sweep (GREEN).

**The Law** `decide` (`kernel.rs`): today enforces **only the state machine** (`assert_transition`), emits
one `Event::StatusChanged{from,to,at}`. `fold` total; `replay` present. Commands carry `Ts(i64)` — time/IDs
enter as DATA, never read from clock/RNG.

**Idempotency** — both pure: `codec/request_hash.rs` = SHA-256 over a canonical **integer-projected**
request (the `f64→i64` projection stays in the shell to keep core float-free); `kernel/idempotency.rs` =
branch decision `Proceed/Replay/Reuse422/DeleteAndRecreate`. Real dup-prevention = DB `UNIQUE(key,
location_id)`; hash only detects a mutated retry.

**Gates:** `unsafe_code=forbid` + `clippy::all=deny` (workspace); `crates/domain/clippy.toml` bans
`SystemTime::now/Instant::now/Uuid::new_v4/env::var`; `scripts/sovereign-gate.sh` runs wasm32 build +
clippy --lib. Hard-Truth proptests: `tests/hard_truth.rs` (money algebra), `tests/kernel_hard_truth.rs`
(determinism, replay-at-every-prefix, terminal absorption). **NOT enforced:** `deny.toml`/cargo-deny does
NOT exist (only in comments); the sovereign gate is **NOT in CI** (`proposed-sovereign-core-ci/APPLY.md` is
a manual proposal; `ci.yml` is Node-only).

**Extraction status:** Step 1 laws/ratchets done; Step 2 identity rename done (`012d9d36`); **Step 3
partial** (`1e02d193`): `policy.rs`+`idempotency.rs`+`request_hash` relocated into core; shell files
`routes/orders/state.rs` + `request_hash.rs` are now thin `pub use domain::...` shims (strangler). **But**
`pricing.rs` (884-line crown jewel) is NOT extracted (still `routes/orders/pricing.rs`, importing
`domain::{Lek,MoneyError}`); and `decide` is NOT yet wired to the corridors (actor-gate, CC-1 strand
guard, `transition_effects`, LC1/conservation live as separate pure fns in `kernel/policy.rs`, not composed
behind the one door). `OrderState` holds only `status`; `Event` has only `StatusChanged`; no
`Envelope{seq,at,cause}`.

**Hub state:** design = `docs/design/rebuild-plan/07-channel-hub-adoption.md` +
`docs/research/2026-07-04-customer-distribution-channels.md`. Head families (marketplace · direct-web/app ·
kiosk+QR · social/messaging · phone/AI-voice · POS) governed as `render|conversational|feed|agentic`;
**single-money-surface / cart-token invariant** — every conversational/social/agentic channel terminates
in ONE signed cart-token handoff into the ONE web checkout (no channel gets a payment surface); token =
signed `{slug,items,channel,iat,exp≤15min,nonce}`, no prices/PII, server re-validates fresh. Ingestion =
Deliverect-shaped MINUS order-intake. **Today only ONE hub primitive exists:** channel *attribution*
(`routes/orders/channel.rs`, `CHANNEL_ALLOWLIST` 13 values, `x-channel` header → **write-only**
`orders.metadata.channel`, never read by pricing/state/dispatch/authz). No `sales_channel` entity, no
adapter interface, no cart-token (money-council-gated). **Mismatch flagged:** the design explicitly does
NOT ingest Wolt/Glovo orders — resolved by D1 (own-channel hub + read-only aggregator view later).

**Cross-pattern (core ↔ hub):** core's single door `decide` mirrors the hub's single money surface; both
**refuse rather than work around** (`CorridorBreach` ↔ "no channel gets a payment surface"); both use
additive-only immutable taxonomies; channel attribution being **write-only, never read by a decision** =
the Manifesto's "UI is a mirror, never a truth source." **Integrity gap:** the modular boundary is
designed + partly enforced, but NOT closed (corridors + money proven in isolation, not behind the one
door; no CI runs the ratchet).

---

## B. Error / issue history — failure patterns

Sources: `REGRESSION-LEDGER.md` (#1–#81), 19 curated reflections + `RETRO/2026-07-06`, lessons INDEX.

**Top recurring failure classes:**
1. **Live-surface / contract-parity-invisible-until-driven** — a proxy (unit test, psql literal, staging,
   mirror-oracle, `#[ignore]` DB test, the *Node* source when *Rust* serves the route) passes while the
   deployed/routed/bound target fails. Refs #77, #78, #80, og-preview reflection (`x-dowiz-cutover`).
2. **Cross-tenant IDOR / forgotten RLS predicate** — per-author discipline dropped across a wide surface;
   RLS inert under BYPASSRLS pool → each miss is a LIVE leak. #57 (15 sites), #58, #16, #39/#40, #50.
3. **GDPR/PII under-erasure & wrong-context anonymization** — #76 (LIVE Art.17), #74, #26, #61, #60.
4. **Wrong-seat / wrong-family binding** — GUC family, DEFINER search_path, bound SQL type. #73, #33, #77.
5. **Money composition / display parity** — #56 (inclusive-tax double-charge, certified green by a
   mirror-oracle), #17, #66 (100× minor-unit), #59, #72/#7b.
6. **Paper gates & false-green proofs** — gate never runs / release-state rots open / assertion can't go
   red. #47 (gates disarmed 11 days), #46, #21/#11b, #67, #64.
7. **Discipline-triggered steps die; only hook-enforced artifacts survive** — reflection→lesson,
   merge-back, ledger-row all rot within a week. #48, #69, #68, #47.
8. **Partial-adoption seam** — logic in the read path only, never mirrored to write. #65, #20, #43, #34.

**Dominant cross-pattern (THE root):** **"the live/deployed/routed/bound reality ≠ the
edited/assumed/literal model"** — a proof measured a PROXY, not the surface under load. Subsumes classes
1, 4, and much of 2/3/5. Cause-critic confirmed all 19 reflection WHYs; 7 of 19 are this exact shape.
Lesser roots: (a) proxies drift from ground truth & are acted on un-reverified; (b) discipline dies
without a hook; (c) shared mutable state without ownership → silent collisions.

**Red-line concentration:** classes 2 (IDOR/RLS), 3 (erasure/PII), 4 (wrong-seat), 5 (money) are ENTIRELY
red-line; the live-surface root is what makes them ship undetected. The red-line surfaces and the
"invisible-until-driven" root are the SAME fire.

**Systematically-missing guardrail (per class, same shape):** *a deterministic gate that drives the real,
deployed, bound, un-bypassed surface, with proof it can go RED on the actual defect.* Money → independent
(non-mirror) oracle. RLS → NOBYPASSRLS behavioral leak test. Erasure → goal-state re-read of the subject
graph. Wrong-seat → bound-param/`::cast`/GUC-pin parity. Live-surface → run against real Postgres + drive
the routed stack (`x-dowiz-cutover`). Paper-gates → armament test that simulates DENY + red-on-real-bug +
actually wired into CI. Discipline → a hook-enforced artifact + a checker.

---

## C. Token economy — state & optimization targets

**Already implemented:** VSA codec (`tools/vsa/src/codec.mjs`) 34.3% aggregate, live A/B whole-dispatch
−35.2%; VSA-VIZ/macro/delta −87…−99% at scale (crossover ~25-30 entities); blind orchestration
(`orchestrate.mjs`) −82% cold/≈−100% steady, 68% auto-resolve at $0; route/dispatch crossover-aware router
+ `integrity.mjs` $0 circuit-breaker; TOKEN ROUTER (−20…−55%); MODEL ROUTING v3 (Fable off doer-lanes,
opus pinned, haiku doer); CLAUDE.md slim (~2.3K); MEMORY.md ~1.3K; council/serious-gate DISABLED.

**Biggest remaining sinks (ranked):**
1. **Per-lane ~42K floor (MEASURED)** — CLAUDE+MEMORY only ~8,550; **~33K = tool schemas + MCP connectors +
   base prompt.** Global claude.ai connectors (Gmail/Notion/Calendar/Drive/Sentry/Figma/Consensus/Harmonic/
   Scholar/Synapse/Common-Room/Learning-Commons + browser/playwright) load into EVERY lane. ~17 lanes/
   session ≈ **714K (~28%) before any work.** Biggest, unaddressed — an **operator claude.ai-settings**
   action (not a repo file; `settings.json` only allow-lists repowise/playwright/browser/figma).
2. **general-purpose vs Explore** — read-only lanes pay 35,753 vs 16,960 floor; router documents
   Explore-default but nothing enforces it. −18.8K/lane.
3. **Hook injection friction (4.2 d):** pre-edit-lessons 491 injects (~95K), route-request 538 nudges,
   require-classification 441 pass + 6 block. Prescribed narrowings still UNSHIPPED (protect-path).
4. **`require-classification` Stop gate** — fires every turn; 6 blocks each forced a new
   manifest/reflection doc + a Stop round-trip. Biggest autonomy-flow friction.
5. **REGRESSION-LEDGER full-read 28,352 tok**, grows forever; no compact index.
6. **Long-session quadratic rot NOT enforced** — 80K lane / 300K session thresholds documented but
   `context-budget-guard.sh` **does not exist** (phantom). Recycle is advisory only.
7. **VSA stack is a library, not wired into the loop** — codec/viz/blind-orch invoked manually; product
   runtime blind-orch ($12,283→$58/mo, −99.5%) is a banked bench, not live in the Rust dispatch tick.

**Ranked "maximum optimization" candidates (impact-per-effort):** (1) trim global MCP connectors
[operator, claude.ai]; (2) Explore-default read-only lanes [−18.8K/lane]; (3) ship route-request
context-aware nudge [protect-path]; (4) narrow pre-edit-lessons `docs/**` trigger [protect-path]; (5)
REGRESSION-LEDGER compact index [autonomous]; (6) wire dispatch/route as default prompt composer
[autonomous]; (7) build `context-budget-guard.sh` [protect-path]; (8) relax require-classification Stop +
red-line-gate glob to money/auth LOGIC not any path with the word [protect-path]. **Bottom line:** the
compression libraries are built + measured; un-captured savings are almost all **plumbing + enforcement**
— MCP-connector floor + Explore-default + the prescribed-but-unshipped hook narrowings. Many are
protect-path → operator-apply.

**Measured numbers:** per-lane floor 42,000 (8,550 CLAUDE+MEMORY, ~33K plumbing); Explore 16,960 vs GP
35,753; VSA codec 34.3% / whole-dispatch −35.2%; viz −87…−99%; blind-orch runtime −99.5%; skeleton −90%;
distill −92%; blended dev-agent ~30-40% saved as-practiced; unshipped levers ≈ another 5-10%/session.
