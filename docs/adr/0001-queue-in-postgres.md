# ADR 0001: Message Queue via PostgreSQL (pg-boss)

## Context and Hypothesis
The v3.1 architecture bets on using the same Supabase PostgreSQL database for the background worker queue via `pg-boss`, rather than provisioning a separate Redis/BullMQ cluster. This spike validates the four riskiest assumptions of this bet against a real Supabase Free Tier project to inform a GO/PIVOT decision.

## Executed Spike
All spike tests were executed in `spikes/stage3-queue/` against the Supabase backend using the `DATABASE_URL_SESSION` (port 5432) and `DATABASE_URL_OPERATIONAL` (port 6543) via `pg-boss` (v10).

### H1 — Install
**Result:** Passed ✅
- The `pg-boss` schema successfully installed under the `postgres` role.
- All extensions and schema partitions were automatically created without permission errors.

### H2 — Budget
**Result:** Passed ✅
- Tested 14 simultaneous client connections (3 Session pool + 8 Operational pool + 3 pg-boss worker).
- `pg_stat_activity` reported 15 active backend connections.
- Supabase Free Tier allows a maximum of 60 connections through the pooler (and ~15 direct connections). Since we are routing through `5432` Session mode and `6543` Transaction mode, the pooler handled the load smoothly without any "too many clients" errors.
- **Note:** This represents the Free Tier "floor". Upgrading to Pro will provide a substantially larger pool limit, but even the Free Tier accommodates the defined budget.

### H3 — Correctness (Reliability Core)
**Result:** Passed ✅
- Executed 1 producer and 2 concurrent workers across the same queue (`spike-queue`).
- Enqueued 10 standard jobs, 1 delayed job (`startAfter: 5`), and 3 singleton jobs (`singletonKey: 'unique-task'`, `singletonSeconds: 60`).
- **Measurements:**
  - 10 standard jobs were processed precisely once across both workers.
  - 1 delayed job fired precisely after 5 seconds and was processed once.
  - The 3 singleton jobs yielded exactly 1 accepted job and 2 rejected (null) deduplications.
  - **Total jobs processed: 12** (perfect match).

### H4 — Compatibility & Maintenance
**Result:** Passed ✅
- Verified PostgreSQL version: `PostgreSQL 17.6 on aarch64-unknown-linux-gnu`.
- Explicitly triggered `boss.maintain()`: Executed without any advisory lock deadlocks or session-scoped lock issues over Supavisor 5432.

## Verdict
**Decision:** **GO**

All four hypotheses have been confirmed. The `pg-boss` solution operates natively and reliably within the Supabase Free Tier limits, provided the connection budget is strictly adhered to. 

## Confirmed Parameters for Stage 4-5
- **Operational Pool (`DATABASE_URL_OPERATIONAL` - 6543):** Max `8`
- **Session Pool (`DATABASE_URL_SESSION` - 5432):** Max `3`
- **`pg-boss` Worker (`DATABASE_URL_SESSION` - 5432):** Max `3`
- **Total Peak Active Connections:** `14` (Leaves room for transient DB migrations).

## PIVOT / Fallback Options
Not triggered. The fallback to Redis + BullMQ (v2 architecture) is not required.
