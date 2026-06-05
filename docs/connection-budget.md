# Database Connection Budget

DeliveryOS uses PostgreSQL connection pools against the Supabase database. Since all pools share the same underlying Postgres backend connections, it is critical to adhere to this budget.

## Pool Distribution

### Operational Pool (Transaction Mode - port 6543)
- **Max Connections:** 8
- **Usage:** Hot path for API requests. Transaction mode multiplexes connections efficiently.
- **Constraints:** No named/cached prepared statements.

### Session Pool (Session Mode - port 5432)
- **Max Connections:** 3
- **Usage:** Workers, analytics, migrations, advisory locks.
- **Constraints:** 1:1 mapping with backend connections. Keep this pool small.

### Backup Pool (Session Mode - port 5432)
- **Max Connections:** 2
- **Usage:** Backup manifest generation, row count queries, restore validation.
- **Constraints:** Separate from operational pool to avoid starvation during backup.

### Transient / Migrations (Session Mode - port 5432)
- Migrations (`node-pg-migrate`) run transiently and use session mode connections during deploy. They will cleanly exit.

## 1. Budget Breakdown (Phase 3, Post-Eтап 20)

*Note: These limits were validated successfully under the Supabase Free Tier load spike (ADR 0001).*

| Client / Component | Connection String | Pool Size (`max`) | Purpose / Notes |
| :--- | :--- | :--- | :--- |
| `@deliveryos/api` (hot path) | `***REDACTED***` (6543) | **8** | Fast, short-lived CRUD, courier dispatch, shift management. |
| `@deliveryos/api` (session pool) | `***REDACTED***` (5432) | **3** | PII decryption reads, export, WebSocket room management. |
| `pg-boss` Worker | `***REDACTED***` (5432) | **3** | Background jobs: courier dispatch, stale check, GPS purge, settlement generation, Telegram notifications. |
| Backup Worker | `***REDACTED***` (5432) | **2** (`BACKUP_POOL_SIZE`) | Backup dump encryption, manifest generation, restore dry-run. |
| Settlement Worker | `***REDACTED***` (6543) | **shared with API** | Settlement cron uses operational pool for payout generation. |
| CLI / Migrations | `***REDACTED***` (5432) | **1** | Transient connection for `node-pg-migrate`. |

## 2. Phase 3 Pool Usage Details

### Courier-Specific Queries
- `courier_shifts` dispatch scan (`FOR UPDATE SKIP LOCKED`) — operational pool
- `courier_assignments` CRUD — operational pool
- `courier_positions` GPS insert — operational pool
- `courier_audit_log` insert — operational pool

### Settlement-Specific Queries
- Payout generation scan — operational pool (transactional)
- Settlement item inserts — operational pool
- Settlement audit log inserts — operational pool

### Backup-Specific Queries
- `pg_dump` runs via separate process (no pool), connects via `***REDACTED***`
- Manifest row count queries — backup pool (session)
- Metadata/audit log updates — operational pool

**Total Peak Backend Connections:** ~17 (Within safe operating range for Supabase Free Tier pooling).
