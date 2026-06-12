# RECON-CATALOG.md — Nightly Reconciliation Checks

> Generated: 2026-06-12 · Scope: data/money drift detectable only in aggregate
> Status: All checks defined · Read-only (zero mutations)

---

## Check Definitions

| ID | Invariant | Query (essence) | Tolerance | Severity | Source |
|---|---|---|---|---|---|
| **M1** | `total = subtotal + delivery_fee + tax_total - discount_total` | `SELECT id FROM orders WHERE total != subtotal + delivery_fee + tax_total - discount_total` | 0 rows | 🔴 | `orders.ts:499-501` |
| **M2** | No negative monetary fields | `SELECT id FROM orders WHERE total < 0 OR subtotal < 0 OR delivery_fee < 0 OR tax_total < 0` | 0 rows | 🔴 | DB CHECK constraints |
| **M3** | Cash amount ≥ total for cash orders with pay_with | `SELECT id FROM orders WHERE payment_method='cash' AND cash_pay_with IS NOT NULL AND cash_pay_with < total` | 0 rows | 🔴 | `orders.ts:504-506` |
| **M4** | Delivered cash amount matches total | `SELECT a.id FROM courier_assignments a JOIN orders o ON o.id=a.order_id WHERE a.cash_collected=true AND a.cash_amount IS NOT NULL AND a.cash_amount != o.total` | 0 rows | 🔴 | `assignments.ts:217` |
| **M5** | No unresolved cash discrepancies without alert | `SELECT * FROM settlement_audit_log WHERE action='disputed' AND NOT EXISTS (SELECT 1 FROM notification_outbox_audit WHERE event='cash.reconcile_discrepancy' AND status='delivered' AND target_id::text = payout_id::text)` | 0 rows | 🟠 | `settlements.ts:203` |
| **O1** | Orders in non-terminal state beyond age threshold | `SELECT id, status FROM orders WHERE status IN ('PENDING') AND created_at < now() - interval '1 hour'` | ≤5 | 🟠 | order lifecycle |
| **O2** | Open shifts > 24h | `SELECT id, courier_id FROM courier_shifts WHERE status IN ('available','on_delivery') AND started_at < now() - interval '24 hours'` | 0 rows | 🟠 | `shifts.ts` |
| **O3** | Failed pg-boss jobs above threshold | `SELECT name, count(*) FROM pgboss.job WHERE state='failed' AND created_on > now() - interval '24h' GROUP BY name HAVING count(*) > 10` | 0 rows | 🟠 | pg-boss infra |
| **N1** | Critical events without delivered audit within 24h | `SELECT event, location_id FROM notification_outbox_audit WHERE event IN ('order.created','order.confirmed','order.rejected') AND created_at > now() - interval '24h' GROUP BY event, location_id HAVING bool_and(status != 'delivered')` | 0 rows | 🔴 | notification pipeline |
| **R1** | PII retention: customers not anonymized past limit | `SELECT c.id, c.location_id, l.retention_days FROM customers c JOIN locations l ON l.id=c.location_id WHERE c.anonymized_at IS NULL AND c.created_at < now() - (l.retention_days || ' days')::interval` | 0 rows | 🟠 | P5 policy |
| **F1** | FK orphans: orders with missing location | `SELECT o.id FROM orders o LEFT JOIN locations l ON l.id=o.location_id WHERE l.id IS NULL` | 0 rows | 🔴 | FK constraint |
| **F2** | FK orphans: assignments with missing courier | `SELECT a.id FROM courier_assignments a LEFT JOIN couriers c ON c.id=a.courier_id WHERE c.id IS NULL` | 0 rows | 🔴 | FK constraint |
| **T1** | Cancellation rate anomaly vs 7d baseline | `SELECT (SELECT count(*) FROM orders WHERE status='CANCELLED' AND created_at > now() - interval '1d')::float / NULLIF((SELECT count(*) FROM orders WHERE created_at > now() - interval '1d'),0) AS today_rate` | < 2× 7d avg | 🟡 | trend baseline |
