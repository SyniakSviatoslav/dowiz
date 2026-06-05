# Phase 4 — Security Model

## IP Hashing (Invariant)

**Rule:** Raw IP addresses are NEVER stored in the database. Only `client_ip_hash` (sha256 hex) exists.

- `orders.client_ip_hash` = `sha256(ip + ':' + daily_salt)`
- Salt rotates daily via `IP_HASH_SALT_YYYY_MM_DD` env var (or fallback `IP_HASH_SALT`)
- Format enforced by CHECK constraint: `^[a-f0-9]{64}$`
- DB trigger prevents creation of a column named `client_ip` (Phase 0 invariant, not re-created)
- Full KMS rotation integration deferred to E25

**Verification:** `\d orders` must show `client_ip_hash` but NOT `client_ip`.

---

## OTP Seam

### Schema
- `phone_otp.code_hash` stores argon2id hash — raw code NEVER written to DB
- Immutability trigger prevents `code_hash` update after insert
- `attempts` counter is mutable (incremented on failed verify)

### Tenant isolation
- `phone_otp` has `FORCE ROW LEVEL SECURITY`
- Policy: `USING (location_id = current_setting('app.location_id')::uuid)`
- Cross-tenant SELECT/INSERT blocked

### Off by default
- `locations.require_phone_otp` defaults to `false`
- Owner must explicitly enable in settings (E26)

---

## Reputation Semantics (Red Lines)

| Rule | Rationale |
|------|-----------|
| **Soft signals only** | Counters are advisory. No auto-ban, no hard gating. |
| **No cross-column invariants** | `no_show_count` can exceed `completed_count`. Owner judgment, not system enforcement. |
| **Decay** | `last_no_show_at` enables time-weighted decay. Older no-shows weigh less. |
| **Positive counter** | `completed_count` is always incremented on DELIVERED. Balances the picture. |

---

## PII Notes

| Column | Status | Risk | Roadmap |
|--------|--------|------|---------|
| `phone_otp.phone` | Plaintext E.164 | Medium | Encrypt at rest in P4+ (separate from courier PII key) |
| `customers.phone` | Plaintext (Phase 0) | High | Encrypt at rest — open question for P4+ |
| `orders.client_ip_hash` | Hashed only | None | Salt rotation in E25 |
| `locations.require_phone_otp` | Boolean | None | — |
| `onboarding_state` | JSONB, no PII expected | Low | App layer must strip PII before writing |
