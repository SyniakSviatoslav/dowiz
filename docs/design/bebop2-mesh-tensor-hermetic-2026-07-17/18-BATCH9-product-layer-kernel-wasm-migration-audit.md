# Batch 9 — Product-Layer → Kernel/WASM Migration Audit (apps/api · apps/web · packages/*)

> **Scope escalation batch.** The operator directed that the *entire* codebase — including the live
> product (`apps/api`, `apps/web`, `packages/*`), not just kernel/engine — move toward kernel/Rust/WASM
> as the execution substrate, with Node/TS/JS/Python demoted to adapters/bridges and the UI reconceived
> as a real-time render of kernel state. This document is **audit + staged plan only** (zero edits, no
> blueprint). Fable writes the blueprint next. Rejections are permitted **only** on hard
> physics/correctness grounds (money/RLS/auth), never on rewrite-cost — per the standing framing rule.
>
> Method: read `kernel/src/wasm.rs` in full; extracted the real hotspot source (see §0 — it is *not*
> in the working tree of this branch); read `orders.ts` (974 L), `courier/shifts.ts` (408 L),
> `customer/otp.ts` (213 L), the pure libs `money.ts`/`geo.ts`/`order-machine.ts`, the `web/` kernel-wasm
> beachhead, the RLS migrations + `withTenant` adapter, and `docs/ops/P8-NOBYPASSRLS-FLAG.md`. Two
> decorrelated sub-audits (apps/web split; RLS boundary) fed §C/§D.

---

## §0 — Ground-truth correction: where the product source actually lives (READ THIS FIRST)

The CLAUDE.md / Repowise hotspot list (`apps/api/src/routes/orders.ts`, `.../courier/shifts.ts`,
`apps/web/src/pages/admin/MenuManagerPage.tsx`, …) is **stale relative to the active branch**. Verified
git facts on `feat/harness-llm-backend` (HEAD):

- `apps/api/` — **0 files tracked**. `apps/web/src/` — **0 files tracked** (only `apps/web/dist/` build
  output survives in the working tree).
- `origin/main` — also **0** `apps/api/src/` files. `main` does not resolve as a local ref.
- The TS product layer was **moved to `attic/apps-api/src/routes/…` and then dropped** from the active
  branches. A `backup/pre-drop-js-20260715-161134` branch and `feat/remove-legacy-thin-layer` /
  `feat/rw-02-delete-channel-js` branches confirm a deliberate "drop-JS" operation on 2026-07-15.
- The **real (non-attic) product source** still lives on `feat/pq-crypto-tier1` and the `recover/*`
  branches. This audit read the hotspot files from `feat/pq-crypto-tier1` (58 route files under
  `apps/api/src/routes/`). Sizes: `orders.ts` 42 127 B / 974 L, `shifts.ts` 15 838 B / 408 L,
  `otp.ts` 8 629 B / 213 L, `MenuManagerPage.tsx` 1 244 L, `MenuPage.tsx` 1 275 L.

**Deployment reality (do not mistake the attic for a completed migration):** the production image
(`Dockerfile`) still `COPY packages ./packages && COPY apps ./apps`, runs `pnpm -r build` +
`scripts/build-apps.ts`, and serves `apps/web/dist` as the storefront and `apps/api` as the API. The
`test:phase*` scripts all target `apps/api/tests/*`. **So the legacy TS is CI-wired and production-live
— it is NOT confirmed-dead code and must not be deleted on the strength of the attic alone.** The "drop"
on the sovereign branches is aspirational ahead of a working replacement; prod still runs the TS.

There are **two frontends**, and conflating them is the single biggest planning trap:
| Path | Stack | Status | Consumes kernel WASM? |
|---|---|---|---|
| `apps/web/` | React 18 + Vite + react-map-gl + react-router | **LIVE in prod** (`dowiz.fly.dev`) | **No** |
| `web/` (top-level) | Astro 5 + Svelte 5, zero-npm kernel shim | **Beachhead, not deployed** | **Yes** — `web/src/lib/kernel/kernel_client.mjs` |

---

## §A — Existing kernel↔WASM bridge inventory

### A.1 The seam already exists, is compiled, is parity-tested, and has a working JS client

`kernel/src/wasm.rs` is a **complete, feature-gated (`#[cfg(feature = "wasm")]`), JSON-in/JSON-out
bridge**. Every export is a thin `#[wasm_bindgen]` wrapper over a pure `Result<String,String>` logic
function that is unit-tested on the native host (43 tests in-file). It touches **no DB, no I/O, no
float-on-money** (module header, `wasm.rs:27`). Exposed surface today:

| Domain | Exports | Kernel authority module |
|---|---|---|
| Order FSM | `place_order_js`, `apply_event_js`, `boot_verify_fsm_js`, `fsm_graph_report_js` | `domain.rs`, `order_machine.rs` |
| Money | `estimate_order_total_js` | `money.rs` |
| Analytics | `channel_ledger_js`, `reduce_anomalies_js` | `analytics.rs` |
| Geo/route | `geo_haversine_js`, `geo_lerp_js`, `geo_bearing_js`, `geo_progress_js`, `geo_progress_flat_js`, `geo_eta_js`, `geo_should_snap_js`, `geo_is_arriving_js`, `geo_point_in_polygon_js`, `geo_is_out_of_order_js` | `geo.rs` |
| Spectral | `spectral_eigenvalues_js`, `spectral_radius_js`, `spectral_gap_js`, `spectral_algebraic_connectivity_js`, `spectral_classify_drift_js`, `spectral_flat_js` | `spectral.rs` |
| Graph | `harmonic_centrality_js` | `harmonic.rs` |

- **Build:** `kernel/Cargo.toml` — `crate-type = ["cdylib", "rlib"]`; the `wasm` feature pulls
  `wasm-bindgen` + `serde*`; the native rlib build stays serde-free. `scripts/build-kernel-wasm.sh`
  compiles `wasm32-unknown-unknown` and emits **two** glue targets: `kernel/pkg/` (`--target nodejs`,
  usable by `apps/api` Node) and `kernel/pkg-web/` (`--target web`, ES module, for `web/`). Compiled
  artifacts (`dowiz_kernel.js`, `dowiz_kernel_bg.wasm`) are already checked in.
- **Consumers today:** (1) `engine/src/bridge.rs` — Rust, consumes the `*_flat` numeric protocol
  (no JSON); (2) `web/src/lib/kernel/kernel_client.mjs` — a **66-line, zero-npm-dependency** shim that
  `WebAssembly.instantiate`s the kernel and calls `spectral_radius_js` / `geo_progress_flat_js` /
  `fsm_graph_report_js`, decoding the multivalue return fail-closed (`{ok:false}` on kernel reject);
  (3) `wasm/demo/`. **None of the live product (`apps/api`, `apps/web`) imports it.**

### A.2 Load-bearing conclusion for §A

The "TS-as-thin-adapter-over-kernel-WASM" pattern the operator wants **is not hypothetical — it already
exists in `kernel_client.mjs` and is the exact template.** The kernel is the declared single source of
truth (`Cargo.toml:5`: *"The TS app is the legacy oracle; this is the canonical kernel"*). The gap is
**wiring**, not construction: the live product re-implements in TS what the kernel already exports and
parity-tests. Every geo/money/FSM/spectral primitive the routes need has a kernel equivalent **today**.

**Dual-authority note (money):** `wasm.rs:332-335` states the design explicitly — the server TS fee
ladder *"stays authoritative for what is CHARGED; this mirror drives what the client SEES."* So the
money surface currently has **two authorities** (TS charge, kernel display). Collapsing them to one
kernel authority is the intent of the `feat/rw-03-kernel-money-authority` branch and is a money red-line
move (§B Tier 2).

---

## §B — apps/api business-logic-in-TS inventory (with risk-tiered migration order)

The central finding: **the pure computation is already factored out of the routes into ~5 KB of tiny
pure libs, every one of which the kernel already mirrors.** The route files are overwhelmingly I/O
orchestration that must stay as *some* thin adapter regardless of language.

### B.1 `orders.ts` (974 L) — the POST /orders monolith (lines 64-795) + GET/:id + PATCH/:id/status

Bucketed by nature (approximate line share of the 974-L file):

- **I/O glue that MUST stay (~85%)** — Fastify route + rate-limit config; `db.connect()` +
  `BEGIN`/`COMMIT`/`ROLLBACK`; `SET LOCAL statement_timeout=4500` (write-hold bound, `:112`); ~25 SQL
  queries (location config, menu version, product/modifier availability, idempotency, inserts of
  order/items/modifiers/velocity/track-grant); `withTenant(...)` on the owner/courier read+write paths
  (`:828`, `:904`); `queue.enqueue` transactional outbox (`:678`, `:688`); post-commit `messageBus`
  publishes; JWT issuance (`:746`). None of this is "primitive computation"; it is transaction
  choreography and is the correct place for a **thin Rust or TS adapter** to live.
- **Pure computation already imported from tiny libs — kernel parity EXISTS:**
  - `assertTransition, OrderStatus` from `@deliveryos/domain` (`:3`) ← `packages/domain/src/order-machine.ts`
    (1 470 B): the 10-status transition table. **Exact kernel mirror** = `apply_event_js` /
    `order_machine.rs` (kernel test `fsm_graph_report_js_shape` asserts `vertices==10`).
  - `applyTax, assertNonNegative, computeLineTotal` from `../lib/money.js` (`:19`) ←
    `apps/api/src/lib/money.ts` (1 988 B): BigInt-only, *"RED LINE: zero float arithmetic on money."*
    **Kernel mirror** = `estimate_order_total_js` / `money.rs` (parity tests
    `estimate_flat_exclusive` … in `wasm.rs`).
  - `distanceKm` from `../lib/geo.js` (`:20`, used at `:540`) ← **kernel** `geo_haversine_js`.
  - **Kernel-portable but NOT yet in kernel:** `evaluatePreflight` (`../lib/preflight.js`, `:332`) — a
    pure decision function over `{lines, signals, acknowledgedCodes}` → `hard_block|soft_confirm|clean`;
    the request-hash canonicalization (`:184-204`, deterministic sorted-modifier JSON + sha256); the
    signal-state reduction (`:308-329`); the timeout rule `busy_mode ? t*2 : t` (`:592`); the delivery
    fee-tier ladder (`:528-560`); `cash_pay_with < total` guard (`:568`); `min_order_value` guard
    (`:519`); modifier-group min/max-select validation (`:494-504`) + duplicate-modifier check (`:472`).
  - **Stays I/O-bound (partial glue):** `computeSignals(db, …)` (`:271`) takes the pool → its DB reads
    stay; only its scoring arithmetic is kernel-portable.

  **Verdict:** the *charged* pricing path (`computeLineTotal` `:506` → `applyTax` `:563` → `total`
  `:565` → `assertNonNegative` `:566`) is pure and kernel-ready. Its **inputs** (`product.price`,
  `modInfo.price_delta`) come from the in-transaction MVCC snapshot read (`:388-392`) — i.e. the
  DB/RLS-scoped read stays in the adapter; the kernel just computes on the snapshot. This is the clean
  separation that makes the migration safe (§D).

### B.2 `courier/shifts.ts` (408 L)

- **~90% I/O glue.** Every handler is `db.connect()` → `BEGIN` → `SELECT set_config('app.current_tenant',
  $1, true)` (`:24`, `:77`, `:120`, `:193`, `:337`) → SQL → audit-log insert → `messageBus.publish`.
- **Pure computation:** `isWithinGeofence` (`:344`), `roundCoordinate` (`:373`) ← already in kernel
  `geo.rs`. A **small embedded shift-FSM** (`offline ↔ available ↔ on_delivery`, `:206-294`) with guards
  ("can't go offline with active order" `:212`; idempotent same-state `:206`; invalid transition `:253`)
  — kernel-portable exactly like `order_machine`, but **not yet in kernel**.
- Migration payoff here is small (mostly glue) but the shift-FSM belongs in the kernel next to the order
  FSM for a single state-machine authority.

### B.3 `customer/otp.ts` (213 L)

- **~95% I/O glue** — location resolve, rate-limit windows, `phone_otp` / `customer_otp_sessions`
  insert/consume, `messageBus`. **Notably uses bare `db.query` with `WHERE location_id=$1` / `slug=$1`
  scoping and sets NO tenant GUC** — these tables lean on application-level scoping (§D).
- **Pure crypto primitives** in `../../lib/otp.js` (`generateOtpCode`, `hashOtpCode`, `hashPhone`,
  `hashOrderIntent`, `verifyOtpCode`, `generateOpaqueToken`, `maskPhone`) — kernel-portable (bebop2 has
  the crypto), but **low leverage / OTP is globally disabled** (`OTP_ENABLED` false until an SMS gateway
  exists). Defer.

### B.4 Risk-tiered migration order for apps/api (read-only/reporting before money/RLS)

| Tier | What | Risk | Why this order |
|---|---|---|---|
| **T0 — done/free** | geo, money-display-mirror, order-FSM already have kernel parity + a working `kernel_client.mjs` template | none | wiring only |
| **T1 — read-only / telemetry** | `channel_ledger_js`, `reduce_anomalies_js`, `fsm_graph_report_js`, `spectral_*`, `geo_progress_*`/`eta` for tracking display | low — no charge, no RLS write | prove the Node-target `kernel/pkg` glue in the live API on a path where a bug is cosmetic |
| **T2 — money DISPLAY → then AUTHORITY** | (a) `estimate_order_total_js` for client preview (safe); (b) collapse dual-authority so the **server** charge path calls the kernel too | **money red-line** | must be **bit-identical, parity-gated** before flipping charge authority; parity tests already exist in `wasm.rs`. Never flip (b) before (a) is proven in prod |
| **T3 — state-machine authority** | order FSM (`apply_event_js` already mirrors `assertTransition`) + port shift-FSM; keep `updateOrderStatus` as the DB-writing adapter around the kernel decision | medium | decision moves to kernel; DB write + RLS scoping stays TS/adapter |
| **T4 — money-RLS-PII write paths** | `POST /orders`, `PATCH /:id/status`, courier assignment | **highest — money + RLS + PII** | kernel computes the *decision*; the transaction envelope + tenant GUC + `WHERE location_id` + idempotency **stay in the thin adapter, never deleted** (§D) |

Top migration-risk item: **T2(b) — flipping money charge-authority to the kernel** and **T4 — the
order-write transaction**, because a divergence between the TS oracle and the kernel changes what a
paying customer is charged, and because the write path is where tenant isolation is established.

---

## §C — apps/web: rendering-vs-logic split + physics-UI migration path

Sub-audit measured the two hotspot React components on `feat/pq-crypto-tier1`:

| Component | Lines | UI rendering (renderer-specific) | Business/validation (kernel-portable) | I/O glue |
|---|---|---|---|---|
| `MenuManagerPage.tsx` (admin) | 1 244 | ~72% (main `return` alone is 713 L / 57%) | ~7% | ~15% |
| `MenuPage.tsx` (client) | 1 275 | ~66% (main `return` 727 L / 57%) | ~13% | ~10% |

- **Business logic is already pure-function-shaped** (data-in → view-model-out), so it can move to WASM
  and feed the React tree as props with no JSX change: admin `getAllProducts` (search/availability/sort,
  `:501-517`), `totalDishes` (`:522-528`); client `displayCategories` (filter+global-sort+re-bucket,
  `:180-216`), `bomToNutrition` (`:99-114`), `toggleModifier` (min/max/radio constraint enforcement,
  `:438-454`), `canAdd` (required-group validation, `:519-528`), `allAllergens`/`allTasteAxes`.
- **Money is small and already mostly centralized.** Currency *formatting* is isolated in the shared
  `PriceDisplay` (`@deliveryos/ui`) → `formatMoney` (`@deliveryos/shared-types`) — no inline formatting
  in either component. The **only inline money arithmetic** is the modifier-delta line-total **preview**
  in `MenuPage`: `calcModifierDelta()` (`:456-468`), `price + delta` (`:1011`), `(price+delta)*qty`
  (`:1227`). A dedicated client mirror `packages/ui/src/lib/money.ts` (`applyTax`/`computeDeliveryFee`/
  `estimateOrderTotal`, ADR-0005 "server is authoritative for what is CHARGED") exists but neither menu
  component imports it — it is a checkout-path lib. This is exactly the arithmetic `estimate_order_total_js`
  already covers.

### C.1 Two honest paths — evaluate both

**Path 1 — WASM-computed state feeds the *existing* React tree (incremental).** Extract the pure
view-model functions into the kernel/WASM, pass outputs as props into the unchanged JSX. Low-risk,
shippable on a live paying product, reuses the `kernel_client.mjs` pattern. Leaves ~66-72% of each file
(framer-motion, Tailwind, `PriceDisplay`, `ProductCard`) untouched.

**Path 2 — no-DOM physics-UI render layer (the `physics-ui-capture` / `field-ui-engine` target).**
Replace the JSX render layer with the wgpu/SDF/Slug-text/AccessKit stack, driven by the same kernel
state. This is the operator's stated end-state and is **not rejected** — but two correctness facts from
the standing arcs bound it, and neither is a rewrite-cost objection:
  1. The physics field **provably does not hold** for exact-alignment (measure-zero), crisp band-limited
     text, and **discrete money/selection** (RED-proven in `field-ui-engine-arc`); the target is
     therefore explicitly a **hybrid** — constraint-solver + SDF layer + kernel state machine — with the
     **money-never-tween boundary preserved** (the field presents integer cents from the kernel, never
     interpolates them). So even the end-state keeps a discrete/DOM-ish layer for money, text-input, IME
     and a11y (AccessKit web backend is a ~multi-year line item).
  2. `VertexBridge` is built+tested but **never reaches a real GPU buffer** yet (`upload_once` only
     counts a hypothetical `writeBuffer`) — the FE-01 wire-it gap. There is zero wgsl/wgpu in-repo today.

**Honest recommendation:** Path 1 first (extract logic → WASM → feed React), Path 2 second and
separately. "No fear of rewrite" licenses committing to Path 2 as the destination; it does **not** make a
big-bang render-layer replacement the honest first move on a live storefront. The realistic substrate for
Path 2 is the **`web/` Astro/Svelte beachhead**, which already loads the kernel via `kernel_client.mjs` —
grow the no-DOM renderer there island-by-island (by Gain−Loss), never a flag-day cutover of `apps/web`.

---

## §D — RLS / money-PII security boundary (explicit; not hand-waved)

**Answer to the operator's question:** moving order/pricing/state logic into kernel Rust/WASM is
**orthogonal to RLS and does not, by itself, weaken it** — *provided the thin DB-session adapter is
preserved.* The danger is not the kernel; it is deleting that adapter along with the "TS business logic."

### D.1 Where RLS is actually enforced — the Postgres layer, not TS business logic

Four database-layer facts, all in migrations:
1. **Policy predicates over a GUC.** `packages/db/migrations/1780310071220_core-identity.ts:70-101`:
   `app_current_user()` = `current_setting('app.user_id', true)::uuid`; `app_member_location_ids()` =
   `SELECT location_id FROM memberships WHERE user_id = app_current_user() AND status='active'` (SECURITY
   DEFINER); `CREATE POLICY tenant_isolation ON locations USING (id IN (SELECT app_member_location_ids()))`.
   The same `tenant_isolation` predicate is applied to `orders`/`order_items`/`customers`/
   `idempotency_keys` (`1780310074262_orders.ts:76-98`), `products`/`categories`, etc. A **second GUC**
   `app.current_tenant` governs the courier-era tables (`couriers`, `courier_shifts`,
   `courier_positions`, `settlement_items`, … `USING (location_id = current_setting('app.current_tenant')::uuid)`).
2. **`ALTER TABLE … FORCE ROW LEVEL SECURITY`** on every tenant table (owner is also subject).
3. **A restricted role** — `1790000000015_operational-pool-role.ts:22` creates the operational user
   `WITH LOGIN NOBYPASSRLS INHERIT`, DML-only.
4. **A per-transaction GUC set by a 20-line adapter** — `packages/platform/src/auth/tenant.ts`
   (`withTenant`): `BEGIN → SELECT set_config('app.user_id', $1, true) → fn(client) → COMMIT`. Courier
   routes set `app.current_tenant` directly (`shifts.ts:24/77/120/193/337`). Anonymous checkout sets **no**
   GUC on purpose, so `app_current_user()` is NULL and the `anonymous_insert WITH CHECK (app_current_user()
   IS NULL)` policy admits it. A defence-in-depth pool guardrail in `packages/db/src/index.ts` destroys any
   connection whose `current_user === 'postgres'` ("SECURITY FAULT … bypasses RLS").

### D.2 The kernel WASM path sits OUTSIDE the RLS boundary

`kernel/src/wasm.rs` is pure JSON-in/JSON-out (`wasm.rs:27`); it opens no connection, runs no SQL, has no
`Pool`. A kernel-wide grep for `pg|postgres|.query(|set_config|DATABASE_URL` finds nothing in the compute
path (the only DB code is an optional `pgrust` living-memory KV, unrelated to tenant tables). **Because it
never touches the database, the kernel can neither enforce nor bypass RLS.** RLS is enforced at the
SQL/connection layer that *any* caller — today's TS, or a future Rust adapter — must still traverse to
read inputs into the kernel and to persist its output. Same policies, same role, same GUC, different
language doing the arithmetic in between.

### D.3 The honest risk — what a naive TS→kernel deletion breaks, and what must survive

RLS breaks **not because the kernel bypasses it, but because the deleted TS carries the
session-establishing plumbing.** A naive "rip out the TS business logic" would take with it:
- **the GUC set** — lose `withTenant`/`set_config` and `app_member_location_ids()` returns ∅ ⇒ every
  `tenant_isolation` policy matches zero rows ⇒ owner/courier flows silently fail-closed (the P8 "KNOWN
  TRAP"); or, worse,
- **the role/pool selection** — a Rust adapter that opens its own connection with the wrong role
  (superuser / BYPASSRLS) and doesn't re-implement the `current_user==='postgres'` guardrail **silently
  bypasses RLS** (cross-tenant reads/writes succeed with no error — the dangerous mode);
- **the transaction envelope + parameterized SQL + `WHERE location_id` discipline** policed by
  `apps/api/tests/phase5/rls-adversarial.test.ts` and the `check-rls.mjs` skill script.

**Must be preserved regardless of compute language (this is I/O adapter, NOT business logic):**
(1) the restricted **NOBYPASSRLS** role on the operational connection string; (2) the per-transaction GUC
set — **both** `app.user_id` (core) and `app.current_tenant` (courier), which are not interchangeable;
(3) the `withTenant` `BEGIN → set_config → COMMIT` envelope (GUC is transaction-local); (4) the pool
guardrail rejecting superuser connections; (5) parameterized SQL + `WHERE location_id` filters. A Rust
adapter that persists kernel output must reproduce this envelope exactly; the kernel's pure functions
cannot and should not.

### D.4 Production caveat that inverts the "Postgres enforces it" story TODAY (hard gate)

Per `docs/ops/P8-NOBYPASSRLS-FLAG.md` (verdict 🔴 CONFIRMED), the prod operational role **`dowiz_app`
still runs with BYPASSRLS**, leaving **~123 policies across 53 migrations DORMANT**; enforcement is gated
behind an **un-applied** `ALTER ROLE dowiz_app NOBYPASSRLS` "Phase-3 flip." **So right now in prod, RLS is
the dormant backstop and the LIVE isolation is exactly the TS-layer `app_member_location_ids()` /
`WHERE location_id` filters + membership checks** (documented as defence-in-depth in
`apps/api/src/lib/storefrontService.ts:9-19`). Until the flip lands, **isolation genuinely leaks into TS
app code**, and a naive TS→kernel deletion would remove the only *live* guard while the Postgres backstop
is still switched off.

**This flag GATES the whole migration.** Also flag a **role-name drift / operational-truth gap**: three
names appear for "the API role" — `deliveryos_api_user` (SKILL.md), `deliveryos_operational_user`
(migration 015), `dowiz_app` (P8, the one prod reportedly uses with BYPASSRLS). Whoever executes a
TS→kernel move must first **confirm the live role and land the NOBYPASSRLS flip**, or the "Postgres
enforces it" guarantee is not yet true in prod and the T4 write-path migration is unsafe.

---

## Prioritized build-order (audit recommendation; Fable turns this into the blueprint)

0. **Pre-gate (blocking, security): land the `NOBYPASSRLS` flip and resolve role-name drift** (§D.4).
   Until prod RLS is actually enforced, tenant isolation lives in TS app code and no write-path logic may
   be deleted. This is the highest-priority item and is independent of the kernel work.
1. **Wire the existing Node-target glue (`kernel/pkg`) into the live API on a T1 read-only path**
   (analytics `channel_ledger`/`reduce_anomalies`, `fsm_graph_report`, geo route-progress). Proves the
   seam in prod where a bug is cosmetic. Reuse the `kernel_client.mjs` decode pattern.
2. **Frontend Path-1 extraction:** move `MenuPage`/`MenuManagerPage` pure view-model functions to WASM,
   feed the unchanged React tree. Ship the money **preview** via `estimate_order_total_js`.
3. **Collapse money dual-authority (T2b, money red-line):** make the server charge path call the kernel,
   bit-identical + parity-gated against the TS `money.ts` oracle before the flip. (`feat/rw-03` intent.)
4. **State-machine authority (T3):** promote order FSM to the kernel decision; add the shift-FSM to the
   kernel; keep `updateOrderStatus` / shift routes as DB-writing adapters.
5. **T4 write paths (`POST /orders`, `PATCH /:id/status`, courier assignment):** kernel computes the
   decision; the tenant-GUC + transaction envelope + `WHERE location_id` stay as the thin adapter, never
   deleted. Gated on step 0.
6. **Frontend Path-2 (no-DOM physics-UI):** grow island-by-island in the `web/` Astro/Svelte beachhead
   (already kernel-wired), never a flag-day cutover of `apps/web`; keep the money-never-tween and
   text/IME/a11y discrete layer per the field-UI RED proofs.

**Kernel gaps to fill for the above (not-yet-exposed, kernel-portable):** `evaluatePreflight` decision
function; the request-hash canonicalization; the delivery fee-tier ladder; the shift-FSM; (deferred) OTP
crypto primitives.

**One-line answer to the RLS question for the operator:** the kernel move is *orthogonal* to RLS — the
kernel never touches the DB, so it cannot weaken row-level security; what would weaken it is deleting the
thin TS session-adapter (tenant GUC + restricted role + `WHERE location_id`) that is I/O glue, not
business logic, and **must survive in whatever language** — and, critically, prod RLS is **not actually
enforced today** (BYPASSRLS), so that TS adapter is currently the *only live* guard and the NOBYPASSRLS
flip must land before any write-path logic is removed.
