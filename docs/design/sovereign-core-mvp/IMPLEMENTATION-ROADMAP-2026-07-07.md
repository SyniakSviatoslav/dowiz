# Sovereign Core MVP Implementation Roadmap
## Fresh Session — 2026-07-07 Late (Phase 1.1 kickoff)

**Status:** Phase 0b complete (0b-1 through 0b-5 + reliability gate). Starting Phase 1 implementation.

### Completed This Session
- ✅ Phase 1.1 schema: `sales_channels` migration (1780350000000) with RLS policy
- ✅ Phase 1.2 schema: `order_events` migration (1780350000001) with append-only enforcement
- ✅ Phase 1.1 Rust API skeleton: `routes/channels/mod.rs` with CRUD handlers + trait
- ✅ Commits pushed to feat/sovereign-core-phase-zero
- ✅ Explore agent analysis: comprehensive readiness report for all 5 MVP phases

### Critical Path (Must Complete in Order)

```
Phase 1.1: channels registry (80% ready)
    ├─ ✅ Migration created
    ├─ ⏳ Postgres impl (PgChannelsRepo) — NEXT
    ├─ ⏳ Route registration in main.rs
    ├─ ⏳ Allowlist parity test (Rust `CHANNEL_ALLOWLIST` == DB CHECK)
    └─ ⏳ NOBYPASSRLS behavioral test

Phase 1.2: event log (60% ready, BLOCKING 2.2)
    ├─ ✅ Migration created
    ├─ ✅ Core Event enum + fold logic (0b-2)
    ├─ ⏳ Interpreter dual-write: routes/orders/pg.rs::apply_events
    │      └─ Insert each Event into order_events table (seq, at, cause_hash, payload, content_hash)
    ├─ ⏳ Replay-parity job: deterministic CI script
    │      └─ for every staging order: replay(genesis, decode(order_events)) == {status, totals, binding}
    └─ ⏳ Bound-param parity: jsonb/bytea encoding tests

Phase 1.5: owner channels dashboard (70% ready)
    ├─ ⏳ GET /api/owner/locations/:locationId/channels (reuse 1.1 repo)
    ├─ ⏳ Attribution count query: SELECT COUNT(*) GROUP BY metadata.channel
    ├─ ⏳ Admin UI tab: "Channels" in dashboard
    ├─ ⏳ i18n: al+en strings
    └─ ⏳ Playwright test: order via x-channel → dashboard count increments

Phase 2.2: direct checkout (75% ready, REQUIRES 1.2)
    ├─ ✅ Order create flow (0b-5 flipped to kernel::decide)
    ├─ ✅ Money composition + conservation (0b-1/0b-3)
    ├─ ⏳ Cart-token spec v0 documentation (BEFORE code — money-council gate)
    ├─ ⏳ Adversarial test suite:
    │      ├─ Client-injected price fields → refused or ignored + correct server total asserted
    │      ├─ Double-create via same request_hash → COUNT(*) = 1 verified
    │      └─ Conservation invariant: SELECT SUM(total) = SUM(subtotal + tax + fee - discount) for all orders
    ├─ ⏳ /reliability-gate deployment proof (L0–L11 on staging, new checkout path)
    ├─ ⏳ x-dowiz-cutover assertion on /api/orders POST (proves Rust handles it)
    └─ ⏳ Feature flag `hub_checkout` (default OFF)

Phase 2.3: customer data + erasure (70% ready)
    ├─ ⏳ POST /api/owner/locations/:locationId/customers/:customerId/erase (trigger deletion)
    ├─ ⏳ GET /api/owner/locations/:locationId/customers (list/search)
    ├─ ⏳ Customer list UI in admin dashboard (searchable table + erase button)
    ├─ ⏳ Erasure oracle test: delete → re-read via list API + search API + order-detail → absent everywhere
    ├─ ⏳ NOBYPASSRLS behavioral test: cross-location erasure attempt → denied
    ├─ ⏳ i18n: al+en for customer UI
    ├─ ⏳ Consent flag: if not in schema, add via migration
    └─ ⏳ Playwright test: create order → customer appears → delete → re-read absent
```

### Implementation Notes

**Phase 1.1 Postgres Impl**
```rust
// rebuild/crates/api/src/routes/channels/pg.rs
pub struct PgChannelsRepo { pool: PgPool }

#[async_trait]
impl ChannelsRepo for PgChannelsRepo {
    async fn create(...) {
        // Validate kind IN ('web-direct', 'qr', ...)
        // Generate UUID token
        // INSERT into sales_channels RETURNING *
    }
    // list/update/delete follow owner pattern
}
```

**Phase 1.2 Dual-Write**
```rust
// In routes/orders/pg.rs::apply_events, add:
for event in events {
    let payload = serialize_to_bytes(&event)?;
    let content_hash = sha256(&payload);
    sqlx::query(
        "INSERT INTO order_events (order_id, seq, at, cause_hash, payload, content_hash)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(order_id)
    .bind(event_seq)  // must increment per order
    .bind(now)
    .bind(cause_hash)
    .bind(payload)
    .bind(content_hash)
    .execute(&mut **txn)
    .await?;
}
```

**Replay-Parity Job (CI Script)**
```bash
# scripts/replay-parity-check.sh
for order_id in $(psql -c "SELECT DISTINCT order_id FROM order_events"); do
    state_from_log=$(psql -c "SELECT replay(...) from order_events WHERE order_id = $order_id")
    state_from_db=$(psql -c "SELECT status, totals, binding FROM orders WHERE id = $order_id")
    [ "$state_from_log" == "$state_from_db" ] || exit 1
done
```

### Effort Remaining (Haiku Doer Lane)

| Phase | Work | Effort | Blocker | Risk |
|-------|------|--------|---------|------|
| 1.1 | PgChannelsRepo + route registration + tests | M (4-6h) | none | LOW |
| 1.2 | Dual-write + replay-parity job + tests | L (8-12h) | 0b-5 ✅ | MEDIUM |
| 1.5 | Channels dashboard endpoint + UI + Playwright | M (4-6h) | 1.1 | LOW |
| 2.2 | Spec + adversarial suite + staging proof | L (8-12h) | 1.2 | HIGH (money) |
| 2.3 | Customer CRUD + erasure oracle + Playwright | M (4-6h) | none | MEDIUM (PII) |

**Total:** ~24-42 Haiku hours. Can parallelize 1.1/1.5 once 1.1 schema is migrated. 2.2 and 2.3 can run parallel after 1.2 event log is implemented.

### Session Continuity Notes

1. **Next session entry point:** Phase 1.1 Postgres implementation (PgChannelsRepo in `routes/channels/pg.rs`). Schema + API skeleton in place.
2. **Staging deployment:** After 1.1 + 1.2 impl complete, run `scripts/deploy-staging.sh` and execute `/reliability-gate` for proof.
3. **Red-line gates (enforce in order):**
   - 1.2: Replay-parity job must be required CI check (never "known-flaky")
   - 2.2: Money-council sign-off BEFORE code (spec review)
   - 2.3: PII + schema (council review)
4. **Feature flags:** `hub_checkout` and `hub_channels` (default OFF until MVP complete and gated)
5. **Memory update:** Commit this roadmap, update PROGRESS.md cursor to "Phase 1.1 Postgres impl NEXT".

### Files Created/Modified This Session

```
✅ packages/db/migrations/1780350000000_sales-channels.ts
✅ packages/db/migrations/1780350000001_order-events-log.ts
✅ rebuild/crates/api/src/routes/channels/mod.rs
```

**Not yet created (NEXT session):**
- rebuild/crates/api/src/routes/channels/pg.rs (PgChannelsRepo impl)
- Route registration in rebuild/crates/api/src/main.rs
- Test fixtures for RLS/allowlist parity
- Event log dual-write in routes/orders/pg.rs
- Replay-parity CI job
- Customer API endpoints
- Dashboard UI wiring
- Playwright test suites
