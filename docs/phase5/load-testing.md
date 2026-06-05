# Load Testing (Stage 34)

## Scenarios

### 1. Read Flood — Public Menu
- **Rate**: 500 RPS constant
- **Target**: `/s/:slug`
- **Assertion**: Cloudflare cache hit >95%, origin connections stable, p95 < 200ms
- **File**: `load/spike.js` → `readMenu()` function

### 2. Burst Orders — Single Tenant
- **Rate**: Ramp 1→20/s over 10s, hold 20s, cooldown
- **Target**: `POST /api/orders` with idempotency key
- **Assertion**: per-tenant rate-limit returns 429 on excess, zero 5xx cascade
- **File**: `load/spike.js` → `placeOrder()` function

### 3. Multi-Tenant Isolation
- **Rate**: 10 RPS constant, both tenants simultaneously
- **Targets**: `/s/${TENANT_A_SLUG}` + `/s/${TENANT_B_SLUG}`
- **Assertion**: Tenant B success rate and p95 do NOT degrade while Tenant A is under burst
- **File**: `load/spike.js` → `multiTenantRead()` function

### 4. Worker Drain
- **Rate**: Burst of durable jobs
- **Assertion**: Backlog drains, no silent worker stop (worker-liveness checker catches any stall)

## Running

```bash
# Prerequisites: k6 installed
k6 run load/spike.js \
  -e BASE_URL=http://127.0.0.1:8080 \
  -e TENANT_A_SLUG=demo \
  -e TENANT_B_SLUG=demo2
```

## Thresholds
| Metric | Threshold | Action |
|--------|-----------|--------|
| `serverError` | <1% | Fix 5xx sources |
| `http_req_failed` | <1% | Fix connection errors |
| `rateLimited` | <100% | Expected under burst |
| `p95_latency` | <500ms | Optimize slow paths |

## CI Integration
- Run weekly in pre-prod
- Block launch if any threshold breached
- Report saved as `load/report-{date}.json`
