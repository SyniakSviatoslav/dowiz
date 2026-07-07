# Sovereign Core MVP — Session Handoff (2026-07-07 late)

**Status**: Phase 2.2 + 2.3 COMPLETE and DEPLOYED TO STAGING  
**Branch**: `feat/sovereign-core-phase-zero`  
**Deployment**: https://dowiz-staging.fly.dev (health: 200)  
**Testing**: 18/18 tests PASS (4 adversarial + 14 e2e)

---

## What Was Done This Session

### Phase 2.2: Direct Checkout (POST /api/orders with kernel::decide)

**Deliverables**:
- Route handler: `rebuild/crates/api/src/routes/orders/checkout.rs` (266 lines)
- Forbidden-field validation: subtotal/tax_total/delivery_fee/total/discount_total → 400
- Server pricing: DB lookup → kernel::decide → conservation invariant
- Customer upsert: Capture phone/name at checkout
- Request-hash idempotency: UNIQUE(location_id, request_hash) constraint
- 3 adversarial RED proofs (all verified):
  - RED PROOF 1: Client price injection rejected
  - RED PROOF 2: Duplicate request hash yields COUNT=1 order
  - RED PROOF 3: Conservation invariant (total = subtotal + tax_charged + delivery_fee - discount)

**Commits**:
- `56f1f872`: feat(Phase 2.2 + 2.3) Direct checkout + customer ownership

**Tests**:
- `rebuild/crates/api/tests/phase_2_2_adversarial_money_suite.rs` — 4/4 PASS (release build verified)

### Phase 2.3: Customer Ownership (NOBYPASSRLS + Erasure Oracle)

**Deliverables**:
- Migration: `1780350000002_customers-consent-flags.ts` (consent fields, indexes)
- Module: `rebuild/crates/api/src/modules/customer_management/` (hub-module with manifest)
- Trait: `CustomerRepo` (list, get, delete, verify_erased)
- Implementation: `PgCustomerRepo` (RLS-scoped, transaction-wrapped, membership-checked)
- Routes: `rebuild/crates/api/src/routes/owner/customers.rs`
  - GET `/api/owner/locations/:locationId/customers` (list, paginated, searchable)
  - GET `/api/owner/locations/:locationId/customers/:customerId` (single)
  - DELETE `/api/owner/locations/:locationId/customers/:customerId` (erasure + oracle)

**Behavioral Gates**:
- NOBYPASSRLS: `assert_active_owner_membership` as first statement in every repo method
- Erasure oracle: `verify_customer_erased` re-reads customers + orders tables post-delete
- RLS scoped: location_id WHERE clause + transaction-level GUC seating via `with_user`

**Commits**:
- `03a7031a`: feat(Phase 2.3) Migration + module foundation + CustomerRepo trait
- `162ef1ec`: feat(Phase 2.3) Route handlers + router wiring + state binding
- `56f1f872`: feat(Phase 2.2 + 2.3) Consolidated commit (both phases complete)

### Comprehensive Testing

**Unit Tests** (Rust):
- File: `rebuild/crates/api/tests/phase_2_2_adversarial_money_suite.rs`
- Status: ✅ 4/4 PASS (debug + release builds verified)
- Coverage: Price authority, idempotency, conservation invariant + hand-derived vectors

**E2E Tests** (Rust behavior):
- File: `rebuild/crates/api/tests/sovereign_core_e2e.rs`
- Status: ✅ 14/14 PASS
- Coverage:
  - Phase 0b-5: Kernel pricing integrity + conservation invariant
  - Phase 1.1: Multi-channel routing
  - Phase 1.2: Event log dual-write + replay parity
  - Phase 1.5: Channel attribution + dashboard aggregation
  - Phase 2.2: Server-priced checkout, idempotency, conservation
  - Phase 2.3: NOBYPASSRLS, erasure oracle, customer capture
  - Full lifecycle: End-to-end MVP integration

**Playwright Tests** (Staging validation):
- File: `apps/web/tests/sovereign-core-mvp-e2e.spec.ts`
- Status: ✅ Compiles, passes linting, ready for staging trace
- Coverage: 10 test cases validating:
  - Server price computation (no client injection)
  - Idempotency (duplicate request returns same order)
  - Customer capture at checkout
  - NOBYPASSRLS gate (cross-location denial)
  - Erasure oracle (customer deletion verification)
  - Event logging
  - Channel attribution
  - Full MVP lifecycle integration

**Commit**:
- `a6ea2001`: feat(testing) Comprehensive e2e test suite for all phases
- `2dd72a99`: test(Playwright) MVP end-to-end validation suite for staging

### Staging Deployment

**Status**: ✅ DEPLOYED  
**Command**: `bash scripts/deploy-staging.sh`  
**URL**: https://dowiz-staging.fly.dev  
**Health**: ✅ 200 OK

**Build Verification**:
- ✅ Pre-commit gates: All 17 governance armaments PASS
- ✅ Typecheck: 13/13 workspace projects green
- ✅ Build: Full SPA + apps built successfully
- ✅ Docker: Fly.io config validated (local Docker disk constraint, cloud build works)
- ✅ Cargo check: `cargo check -p api` green (only 9 dead-code warnings from stubs)

---

## What's Ready for Next Session

### Immediate Next Steps (Low effort)

1. **Run /reliability-gate on staging** — Manual trace across L0–L11:
   - L0: `GET /s/demo` (entry page)
   - L1: Menu read via Rust
   - L2: Order create via Phase 2.2 checkout (new!)
   - L2: Idempotent double-POST (new!)
   - L4: Owner CONFIRM
   - L4: Double-CONFIRM race guard
   - L5: Status transitions (PREPARING → READY)
   - L5: Illegal transition guard
   - L6: READY → PICKED_UP (pickup terminal)
   - L11: Owner order list (shows status)
   - L7: Courier DELIVERED (delivery path)

   **Gate doc**: `docs/ops/reliability-gate-cutover-2026-07-05.md`

2. **Run Playwright tests against staging** — Execute `apps/web/tests/sovereign-core-mvp-e2e.spec.ts`:
   ```bash
   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test apps/web/tests/sovereign-core-mvp-e2e.spec.ts --reporter=list
   ```

3. **Verify feature flag `hub_checkout`** — Confirm default OFF, can be toggled ON:
   - Spec requirement: Phase 2.2 only active when feature flag is ON
   - Current status: Wired in handler but needs feature-flag gate verification

4. **Verify replay-parity job** — Phase 1.2 dual-write + replay:
   - Events logged to `events` table
   - Replay from event log matches current order state
   - CI job green (if configured)

### Pending Work (Before prod merge)

1. **Red-line gates** (user/council review):
   - Phase 0b-5: Shell flip (already RED-proved on 0b-3 kernel)
   - Phase 1.2: Persistent event log (red-line: replay parity)
   - Phase 2.2: Money conservation invariant (red-line: hard-truth proptests + adversarial suite)
   - Phase 2.3: Customer PII + RLS (red-line: NOBYPASSRLS behavioral gate)

2. **Staging full lifecycle validation** (via /reliability-gate):
   - Trace a full order from creation (Phase 2.2) through delivery
   - Verify customer data capture (Phase 2.3)
   - Verify events logged (Phase 1.2)
   - Verify channel attribution (Phase 1.5)
   - Verify no cross-location leaks (Phase 2.3 NOBYPASSRLS)

3. **Production merge readiness**:
   - All phases 0b, 1.1, 1.2, 1.5, 2.2, 2.3 validated on staging
   - Feature flag `hub_checkout` OFF by default (can ramp ON gradually)
   - Migrations on staging DB FIRST (before prod)
   - Prod merge only on explicit operator approval

---

## Branch State

**Current branch**: `feat/sovereign-core-phase-zero`

**Recent commits** (newest first):
```
2dd72a99 test(Playwright): MVP end-to-end validation suite for staging
a6ea2001 feat(testing): Comprehensive e2e test suite for all phases (0b-2.3)
56f1f872 feat(Phase 2.2 + 2.3): Direct checkout + customer ownership (kernel::decide integration)
162ef1ec feat(Phase 2.3): Customer routes + repo wiring + erasure oracle integration
03a7031a feat(Phase 2.3): Customer management module foundation + consent flags migration
1fef0e30 feat(Phase 2.2): Cart-token spec + adversarial test suite structure
032c0c58 docs(MVP): update PROGRESS for Phase 1.1/1.2/1.5 completions
```

**Staged work** (not ours, concurrent):
- `rebuild/Cargo.lock` — domain + api crate updates
- `rebuild/crates/domain/` — kernel finalization (0b-5 already complete)

**To push**:
- All commits above ready to push
- Tests passing
- Linting passing
- Pre-commit gates passing

---

## File Manifest

**New files**:
- `rebuild/crates/api/src/routes/orders/checkout.rs` (266 lines) — Phase 2.2 route
- `rebuild/crates/api/tests/phase_2_2_adversarial_money_suite.rs` (448 lines) — Phase 2.2 tests
- `rebuild/crates/api/tests/sovereign_core_e2e.rs` (363 lines) — E2E behavioral tests
- `apps/web/tests/sovereign-core-mvp-e2e.spec.ts` (345 lines) — Playwright staging tests

**Modified files**:
- `packages/db/migrations/1780350000002_customers-consent-flags.ts` — Phase 2.3 migration (new)
- `rebuild/crates/api/src/modules/customer_management/mod.rs` — Phase 2.3 module
- `rebuild/crates/api/src/modules/customer_management/pg.rs` — Phase 2.3 implementation
- `rebuild/crates/api/src/routes/owner/customers.rs` — Phase 2.3 routes
- `rebuild/crates/api/src/routes/orders/mod.rs` — Router wiring
- `rebuild/crates/api/src/routes/orders/pg.rs` — Bug fix (Lek constructor in tests)
- `rebuild/crates/api/src/openapi.rs` — OpenAPI registration

---

## Key Invariants & Proofs

### Phase 2.2 Money Conservation (RED-proved)
```
total = subtotal + tax_charged + delivery_fee - discount
```
- ✅ RED PROOF 1: Remove validation → client price accepted → test FAILS
- ✅ RED PROOF 2: Remove UNIQUE constraint → COUNT > 1 → test FAILS
- ✅ RED PROOF 3: Compute tax incorrectly → invariant diverges → test FAILS

### Phase 2.3 NOBYPASSRLS (RED-proved)
```
Every repo method:
  1. assert_active_owner_membership(txn, owner_user_id, location_id) first
  2. All WHERE clauses include location_id
  3. Transaction-level GUC seating via with_user()
```
- ✅ RED PROOF: Remove membership check → cross-location access succeeds → test FAILS

### Phase 2.3 Erasure Oracle (RED-proved)
```
After delete_customer():
  1. SELECT NOT EXISTS(SELECT 1 FROM customers WHERE id=? AND location_id=?)
  2. SELECT NOT EXISTS(SELECT 1 FROM orders WHERE customer_id=? AND location_id=?)
  3. Both must return true
```
- ✅ RED PROOF: Skip the NULL update on orders → second re-read FAILS

---

## Memory & Documentation

**Memory updated** (via auto-memory):
- Session handoff status
- Phases 2.2–2.3 completion
- Testing status
- Staging deployment state

**Docs created**:
- This handoff file: `docs/design/sovereign-core-mvp/HANDOFF-2026-07-07-SESSION.md`
- Spec already exists: `docs/design/sovereign-core-mvp/PHASE-2-2-CART-TOKEN-SPEC.md`
- Progress updated: `docs/design/sovereign-core-mvp/PROGRESS.md`

---

## Rollback Plan (if needed)

All changes are on feature branch `feat/sovereign-core-phase-zero`:
- Revert to `032c0c58` (docs update before 2.3 started) to roll back customer module
- Revert to `1fef0e30` (before impl) to roll back both phases
- Main branch `main` unchanged; prod safe

---

## Token Economy

**This session**: ~95K tokens (Haiku doer throughout)
- Checkout implementation + adversarial tests: ~40K (agent)
- E2E tests writing: ~15K
- Deployment + validation: ~10K
- Handoff prep: ~5K

---

## Sign-Off

✅ **Ready for next session handoff**:
- All code committed
- All tests passing (18/18)
- Staging deployed (health: 200)
- Pre-commit gates passing (17/17 armaments)
- Feature ready for /reliability-gate validation

**Next actor**: Run full L0–L11 trace on staging, verify red-line gates, prepare for prod merge.
