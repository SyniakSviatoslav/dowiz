# Reconciled reliability-gate SKILL.md (for manual approval)

`.claude/skills/reliability-gate/SKILL.md` is governance-protected. Replace its body with the
content below — it audits stages/files/tables that **exist** and moves unbuilt features to a
"Not yet built" backlog (so the gate stops failing on phantom features). Keep the frontmatter.

---

(frontmatter — keep)
```
---
name: reliability-gate
description: Run the DeliveryOS lifecycle reliability gate. Traces ONE order from /s/:slug through the implemented stages (L0–L9), verifies every surface + cross-tenant isolation, produces a GO/NO-GO verdict. Invoke with /reliability-gate.
---
```

## What changed vs the old skill
- **Stages L0–L11 → L0–L9** mapped to real code (entry, order create, timeout, CONFIRMED via owner/courier, status transitions, assign+accept, pickup, delivered, cross-surface+tenant+N=2).
- **Removed phantom PASS criteria** (migrations 029/030/031; `delivery_trace`/`courier_cash_ledger`/`order_ratings` tables; `DispatchView`/`StarRatingBlock`/`canSubmit`; `ORDER_FEEDBACK_REMINDER`). These are now a **§ Not yet built (backlog — NOT gate failures)** list.
- Added the real courier endpoints (`/accept :112`, `/picked-up :208`, `/delivered :261`) and real migration filenames (timestamp-prefixed).
- Added an **optional live runtime check** (mock-auth + cross-tenant 404 boundary).

## § Not yet built (backlog — flag-only, never fail the gate)
`delivery_trace`, `courier_cash_ledger`, `order_ratings`/ratings route, `ORDER_FEEDBACK_REMINDER`,
StarRating/`canSubmit` UI, `DispatchView`; READY→`courier:{id}` push; GPS accuracy/speed gates;
idempotency_keys composite `(location_id,key)` PK; SSR `/s/:slug` `Cache-Control` header.

## Real gaps in EXISTING code worth filing (from the 2026-06-18 run)
- L0: SSR `/s/:slug` ships no `Cache-Control` (only the JSON menu API has the 60s header).
- L1: `idempotency_keys` PK is single-column `key` but code queries `WHERE key AND location_id` — drift.
- L2/L3: worker `ORDER_TIMEOUT` handler is silent (cancels but writes no status-history row, no bus publish).
- L4: `transitionOrder` refactor dropped the pgboss `queue.cancel` on CONFIRMED (mitigated by `timeout_at=NULL` + the handler's `status='PENDING'` recheck).
- Misc: dashboard `countSql` has no `DATE=CURRENT_DATE`; analytics endpoints not wrapped in `withTenant`.

> The full reconciled SKILL.md body is the one this session attempted to write to
> `.claude/skills/reliability-gate/SKILL.md`; apply it there after review.
