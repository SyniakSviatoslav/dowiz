# Phase 4 — Blockers (Open Questions / Debt)

This document tracks unresolved items that are out of scope for P23 but must be addressed in later Phase 4 stages.

---

## High Priority

| ID | Item | Stage | Notes |
|----|------|-------|-------|
| P4B-H1 | Customer PII encryption at rest | P4+ | `customers.phone`, `customers.name` are plaintext. Requires separate key from courier PII. Needs migration, app-layer encrypt-on-write, decrypt-on-read. |
| P4B-H2 | `phone_otp.phone` encryption at rest | P4+ | Currently plaintext E.164. Should use same key as customer PII encryption once implemented. |
| P4B-H3 | IP hash daily salt rotation | E25 | Stub exists in `lib/ip-hash.ts` but full KMS integration deferred. Without rotation, hashes are linkable across days. |

---

## Medium Priority

| ID | Item | Stage | Notes |
|----|------|-------|-------|
| P4B-M1 | `customer_otp_required` deprecation | P4 cleanup | Legacy column on `locations` (Phase 0). `require_phone_otp` replaces it. Drop in a future cleanup migration. |
| P4B-M2 | `phone_otp.phone` format validation | E26 | DB doesn't validate E.164 format. App layer must reject non-E.164 at write time. |
| P4B-M3 | Onboarding state JSONB schema validation | E29 | DB checks only `typeof = 'object'`. Per-key validation (v, step, etc.) is app-only. |

---

## Low Priority

| ID | Item | Stage | Notes |
|----|------|-------|-------|
| P4B-L1 | Reputation counter stale data cleanup | P4+ | `no_show_count` may drift from reality. Periodic recomputation job optional. |
| P4B-L2 | Dwell threshold edge cases | E25 | App layer should clamp negative values before writing JSONB. |
| P4B-L3 | `customers_no_show_idx` maintenance | — | Partial index where `no_show_count > 0`. Index bloat minimal but should be monitored. |

---

## Resolved During P23

| ID | Item | Resolution |
|----|------|------------|
| P4B-R1 | `customers.no_show_count` already exists | Column was created in Phase 0 migration. P23 added CHECK constraint and COMMENT. |
| P4B-R2 | `locations.customer_otp_required` vs `require_phone_otp` | Both exist. `customer_otp_required` from Phase 0, `require_phone_otp` added as Phase 4 seam. Consolidated in future cleanup. |
