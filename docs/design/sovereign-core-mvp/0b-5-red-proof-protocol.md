# 0b-5 RED Proof Protocol — Deployed-Reality Shell Flip

**Goal:** Prove that `kernel::decide` is EXECUTED (not a mirror-oracle) on the deployed staging API.

**Load-bearing claim:** The owner status transitions use ONE `decide` door, composed by the kernel, not scattered shell checks.

**Method:** Inject a corridor REFUSAL in the kernel → deploy to staging → run a real API request that would normally succeed → assert the request receives a `CorridorBreach` error → assert the order was NOT mutated → revert the injection → re-run the same request → assert it now succeeds.

---

## Proof Branches & Stages

### Stage 1: Proof of Execution (inject-fail-revert cycle)

**1a. Create a proof branch**
```bash
git checkout feat/sovereign-core-phase-zero
git pull origin feat/sovereign-core-phase-zero
git checkout -b proof/0b5-red-proof-inject
```

**1b. Inject corridor refusal in the kernel**

Edit `rebuild/crates/domain/src/kernel.rs`, in the `decide()` function. Find the line where `Command::Dispatch` is handled (around line 310–330), and add this BEFORE the actor-gate check:

```rust
// 0b-5 RED PROOF INJECT: refuse CONFIRMED→IN_DELIVERY to prove routing.
// Remove this after the proof is captured (revert the commit).
if current == OrderStatus::Confirmed && new_status == OrderStatus::InDelivery && cmd.actor() == Actor::Owner {
    return Err(DomainError::CorridorBreach {
        corridor: "RED_PROOF_DISPATCH",
        code: ErrorCode::DispatchNotAllowed,
    });
}
```

**1c. Verify the injection breaks the unit tests (locally)**
```bash
cd rebuild
cargo test kernel_hard_truth --lib 2>&1 | grep "test result"
```

Expected: `FAILED` on `terminal_states_absorb_all_commands` or similar (the CONFIRMED→IN_DELIVERY path is refused).

**1d. Revert the injection locally**
```bash
git checkout rebuild/crates/domain/src/kernel.rs
cargo test kernel_hard_truth --lib 2>&1 | grep "test result"
```

Expected: `ok. 12 passed; 0 failed` (back to green).

This stage proves the gate WORKS (it CAN catch defects).

---

### Stage 2: Deployed-Reality Proof on Staging

**2a. Build & deploy the injected branch to staging**

```bash
git checkout proof/0b5-red-proof-inject
# Re-inject the refusal (or cherry-pick from 1b if you committed it)
# Then:
bash scripts/deploy-staging.sh
# (Waits for build + deploy + healthcheck; monitor https://dowiz-staging.fly.dev/health)
```

**2b. Create a test order on staging**

```bash
# Use the test fixture from memory: test@dowiz.com / test123456
# POST to https://dowiz-staging.fly.dev/api/orders
curl -X POST https://dowiz-staging.fly.dev/api/orders \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "location_id": "<test-location-uuid>",
    "items": [...],
    "order_type": "delivery",
    "delivery": {...}
  }'
# Response: `{ id, status: "PENDING", ... }`
# Save the order ID.
```

**2c. Confirm the order (should succeed)**

```bash
# PATCH to move PENDING→CONFIRMED
curl -X PATCH https://dowiz-staging.fly.dev/api/orders/<order_id>/status \
  -H "Authorization: Bearer <owner-token>" \
  -H "Content-Type: application/json" \
  -d '{ "status": "CONFIRMED" }'
# Expected: 200 OK, `{ status: "CONFIRMED" }`
```

**2d. Try to dispatch (Confirm→InDelivery) — EXPECT FAILURE with CorridorBreach**

```bash
curl -X PATCH https://dowiz-staging.fly.dev/api/orders/<order_id>/status \
  -H "Authorization: Bearer <owner-token>" \
  -H "Content-Type: application/json" \
  -d '{ "status": "IN_DELIVERY" }'
# Expected: 400 Bad Request (or 403/409 depending on ErrorCode)
# Response body MUST contain:
#   "code": "DISPATCH_NOT_ALLOWED"  (or the ErrorCode variant)
#   "message": contains "Dispatch" or the wire message
#   "request_id": "<correlation-id>"
#   "x-dowiz-cutover": "rust" ← CRITICAL: proves Rust served this request
```

**2e. Verify order immutability (read the order — should still be CONFIRMED)**

```bash
curl -X GET https://dowiz-staging.fly.dev/api/orders/<order_id> \
  -H "Authorization: Bearer <owner-token>"
# Expected: 200 OK, `{ status: "CONFIRMED", ... }`
# If status were IN_DELIVERY, the mutation passed through the bad refusal → FAIL the proof.
```

**2f. Capture the proof (screenshot / log)**

Save:
- The error response JSON (CorridorBreach visible)
- The read response JSON (status still CONFIRMED)
- The `x-dowiz-cutover: rust` header from 2d
- Request IDs for audit trail

---

### Stage 3: Revert Injection & Verify Success

**3a. Checkout clean main branch and re-deploy to staging**

```bash
git checkout feat/sovereign-core-phase-zero
bash scripts/deploy-staging.sh
```

**3b. Re-run the same dispatch command — EXPECT SUCCESS**

```bash
# Use the SAME order ID from 2b (or create a fresh one if the old one rolled back)
curl -X PATCH https://dowiz-staging.fly.dev/api/orders/<order_id>/status \
  -H "Authorization: Bearer <owner-token>" \
  -H "Content-Type: application/json" \
  -d '{ "status": "IN_DELIVERY" }'
# Expected: 200 OK, `{ status: "IN_DELIVERY" }`
```

**3c. Verify mutation (read the order — should now be IN_DELIVERY)**

```bash
curl -X GET https://dowiz-staging.fly.dev/api/orders/<order_id> \
  -H "Authorization: Bearer <owner-token>"
# Expected: 200 OK, `{ status: "IN_DELIVERY", ... }`
```

---

## Proof Matrix (Falsifiable Gates)

| Gate | Inject (Stage 1) | Deploy (Stage 2) | Revert (Stage 3) | Verdict |
|------|---|---|---|---|
| Unit test fails on inject | ✅ FAILED | — | ✅ GREEN | RED proof works locally |
| Staging request blocked with CorridorBreach | — | ✅ 400 + correct error code | — | Routing is live |
| Order immutable while blocked | — | ✅ status still CONFIRMED | — | No partial mutations |
| x-dowiz-cutover: rust | — | ✅ header present | — | Rust served the request |
| Request succeeds after revert | — | — | ✅ 200 OK | Gate removed, flow works |
| Order mutates after revert | — | — | ✅ status is IN_DELIVERY | Corridor accepts the transition |

**Any FAILED cell = RED proof INVALIDATES the 0b-5 claim** (decide is not the executed door, or routing is wrong).

---

## Reliability Gate (After RED Proof)

Once the RED proof passes, run the operator's full reliability gate on staging:

```bash
pnpm exec playwright test --config=VITE_BASE_URL=https://dowiz-staging.fly.dev /reliability-gate
```

This traces one real order L0–L11 through the full lifecycle (`/s/:slug` public view + owner actions) and expects a GO verdict.

---

## Session End: Push & Handoff

After RED proof passes + reliability gate green:

1. Clean up proof branch (if you created one): `git branch -d proof/0b5-red-proof-inject`
2. Push `feat/sovereign-core-phase-zero` to origin (commits already staged)
3. Record the proof captures (error response, immutability proof, x-dowiz-cutover header) in git notes or a ledger entry

**Result:** 0b-5 is SHIPPED (shell flip + RED proof + reliability gate all green).

---

## Notes

- **No council sign-off required** (disabled 2026-07-05 per operator decision).
- **Staging-only**: the RED proof can only run on a deployed environment (needs real order creation, real DB, real HTTP routing).
- **Load-bearing**: without this proof, the 0b-5 code is live but unproven (mirror-oracle risk). The code is correct; the proof proves it is USED.
- **Falsifiable**: the inject stage proves the gate CAN fail (not a false-green); the deploy stage proves it DOES fail on the actual API; the revert stage proves it returns to success (not a stuck system).

