# S2-AUTH Port — Council Packet · STATUS

## 🟢 COUNCIL-APPROVED (2026-07-04) — cleared to BUILD DARK to the cutover DoD

**Triadic council ran 2026-07-04** (architect + breaker + counsel + decorrelated security-sentinel).
Verdict: **PROCEED-WITH-REVISIONS, no ETHICAL-STOP**. Seat verdicts: architect RATIFIED-WITH-REVISIONS ·
breaker "the port breaks" (1 CRIT / 6 HIGH — strict-claim parse + Q10 body-`kid`) · counsel
PROCEED-WITH-REVISIONS · security-sentinel PASS-with-one-HIGH (RETIRE courier-activate). Full record in
`resolution.md`.

**OPERATOR SIGN-OFF: APPROVED ALL 🔴 items (2026-07-04, explicit, in-session).** Q1 (rotation runbook +
dual-kid fast-follow), **Q2 RETIRE courier-activate**, Q5 (localStorage for the port; customer-token
httpOnly is the lead fast-follow), Q6 (courier per-location re-check as fast-follow), **Q10 no-migration
canary cutover**, AR-6 key-rotation runbook, cutover canary plan — all approved.

**Scope of the approval (precise):** cleared to (1) fold REV-1…REV-5 as binding build specs, and
(2) **BUILD the Rust S2 auth surface DARK** to the cutover DoD (below). The **live production cutover
flip** (routing real auth Node→Rust) remains a **separate future go/no-go** once the surface is built,
the DoD is green, and the canary has run — building dark is not flipping. AUTH rows advance
`MAPPED → COUNCIL-APPROVED`; `BUILT` on green DoD; `CUTOVER` only at the future flip.

**Binding on the build (from `resolution.md`):** RETIRE `/api/auth/courier/activate` · REV-1 courier
`has_location` per-request re-check + `(jti, activeLocationId, sub)` key · REV-2/C1 body-`kid`
round-trip + cross-verifiable/identical-hash-format contract · REV-3/T-12 customer-scope FIX-IN-PORT ·
REV-4 middleware tower order (rate-limit/OPTIONS/NO_AUTH) · REV-5 Q11 collapse only after the 4
divergences are contracted · four Q10 cutover gates · claim-set frozen for the overlap.

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
