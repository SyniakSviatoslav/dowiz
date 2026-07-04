# S2-AUTH Port — Council Packet · STATUS

## ⛔ DRAFT — NOT APPROVED

This packet is the **description input** to the S2-auth Triadic Council. It documents current Node
behavior verbatim and proposes a Rust/axum port design. **It is not an approval to port.** No 🔴 AUTH
row moves to `BUILT` in `traceability.csv` until this packet is council-APPROVED and every
open-question is resolved on the record.

- **Lane:** R3 (complete-rebuild) · **Surface:** S2 auth 🔴 (REBUILD-MAP §3 Phase B, 4th strangler)
- **Date:** 2026-07-04 · **Source:** `fix/audit-remediation@ae9f5360`
- **Status ratchet position:** `MAPPED` → **(this packet)** → `COUNCIL-APPROVED` → `BUILT` → `PROVEN`
  → `CUTOVER`. Currently pre-`COUNCIL-APPROVED`.

## Packet contents
| File | Role |
|---|---|
| `proposal.md` | Port design: claims-extractor type-state, RS256/kid parity, mint & TTL matrix, ADR-0004 revocation parity, dev-login permanent exclusion, transport, quirk register, middleware tower |
| `threat-model.md` | Assets, trust boundaries, 5 carried gaps as accepted-risk rows, JWT-in-URL disposition, token-theft/replay/tenant-confusion scenarios, B3-flip effects |
| `open-questions.md` | 12 numbered decisions, options + R3 recommendation, ordered port-blocking vs fast-follow vs deferrable |
| `packet-status.md` | This page |

## What APPROVAL requires (council DoD)
1. **Every open question resolved** on the record (`open-questions.md` Q1–Q12), each port-blocking one
   (Q2, Q10, Q11, dev-kid segregation) settled before code.
2. **Every quirk-register row dispositioned** (`proposal.md §10`) — carry-verbatim vs FIX-in-port; each
   FIX carries an explicit E2E delta.
3. **The 5 carried gaps + AR-6 signed as accepted-risk** (`threat-model.md §3`) or promoted to a FIX
   with an owner and an E2E.
4. **RETIRE decision on `/api/auth/courier/activate`** with proof-of-deadness (recommend RETIRE, Q2).
5. **Cutover parity gates named:** empty `openapi-diff`; auth E2E slice green (as-is specs,
   `traceability-s1-s2.csv`); AUTH invariant-cluster red→green (ADR-0004 refresh vectors, dev-kid
   prod-rejection, customer-token-no-phone, reuse→family-revoke); shared-token/hash parity test
   (Node↔Rust, Q10); rollback plan (route back to Node behind the proxy).
6. **B3 dependency acknowledged** — the GUC-always-seated and definer-search_path-pin fixes
   (sweep #2/#3) are B3-council rows that gate the NOBYPASSRLS flip's effect on auth
   (`threat-model.md §6`); S2 does not depend on the flip but must not rely on it to fix predicates.
7. **Charter check (counsel):** no auth capability serves harm; dev-bypass stays prod-inert; PII
   minimization (no phone in customer JWT) preserved.

## Proposed council composition
| Seat | Agent | Charge |
|---|---|---|
| Architect | `system-architect` | Ratify the claims-extractor type-state, mint/TTL parity, middleware tower, session-migration seam (Q10) |
| Breaker | `system-breaker` | Prove the port breaks — refresh-race edge, strict-claim bypass, dev-kid leak on prod, insider write-window (T-1..T-11), B3 fail-open interaction |
| Counsel | `counsel` | Ethics/strategy — AR-5 XSS residual honesty, AUTH-GAP-4 no-reset UX, dev-bypass discipline, PII, ETHICAL-STOP surface |
| Human | operator | 🔴 sign-off on Q1/Q2/Q5/Q6/Q10 (irreversible/red-line), RETIRE approval, key-rotation runbook (AR-6), cutover go/no-go |

Optional decorrelation: `security-sentinel` / `Application Security Engineer` read-only pass on the
JWT verifier + dev-kid segregation before the Breaker seat (they authored the 2026-07-02 sweep folded
into `threat-model.md`).

## Non-negotiables carried into every seat (from CLAUDE.md + inventory 14 §4)
- **RS256 only**, alg pinned twice; **zero cookies** (localStorage/Bearer) unless Q5 flips it.
- **Byte-compatible tokens across stacks during overlap** — the strangler's load-bearing seam (Q10).
- **dev-login permanently prod-inert** — four ADR-0003 layers, `#[cfg(feature="dev-routes")]`.
- **Quirks annotated, never silently fixed** — FIX only for 🔴 security-correctness + E2E delta.
- **Parity oracle is the E2E net** — a change is real only with a red→green test (Mandatory Proof Rule).

---
*Nothing in this packet is code, and nothing here authorizes a code change. Approval is a separate,
explicit, human-gated act recorded against `traceability.csv`.*
