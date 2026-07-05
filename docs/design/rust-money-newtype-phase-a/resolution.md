# RESOLUTION — Rust `domain` money newtype `Lek(i64)` (Phase A scaffold), Round-1

- **Date:** 2026-07-04 · **Architect:** System Architect DeliveryOS · **Round:** STOP-DESIGN-B (RESOLVE)
- **Inputs:** `proposal.md`, `docs/adr/ADR-rust-money-newtype.md`, `breaker-findings.md` (2 HIGH · 5 MED · 3 LOW),
  `counsel-opinion.md` (no ETHICAL-STOP; 3 forward cautions + 1 open question).
- **Verified live-source grounding used in this resolution:**
  - Scale-0 Lek — `apps/api/src/lib/preview-render.ts:47` & `ssr-renderer.ts:304` (`minor_unit ?? 0`),
    `csv-parser.ts:12` (`currencyMinorUnit = 0`), `money.ts:1` (`_minorUnit` dead param). H-1 confirmed.
  - Throw-not-clamp parity — `apps/api/src/lib/money.ts:32-36` `assertNonNegative` throws on `< 0`, does not
    clamp. M-5 parity confirmed.
  - Frame-doc absence — `docs/design/rebuild-plan/**` glob returns **0 files** in this worktree. L-3 confirmed
    a worktree-sync fact (docs live in the main checkout, not this scaffold branch), not a doc-integrity defect.
- **Outcome:** every finding dispositioned; the code plan changed (see "Deltas for the implementer"). No
  ETHICAL-STOP to revise; one **human decision flagged** (O-10, directional money).

---

## Disposition table (10 findings)

| # | Sev | Vector | Disposition | Owner | One-line reason |
|---|---|---|---|---|---|
| **H-1** | HIGH | B-SCALE | **FIX (reframe; keep i64)** | This proposal | Sizing number was 100× wrong (scale-0 Lek → single location-year *fits* int4). Corrected §2/ADR; i64 re-justified on rollups + adversarial intermediates + scale-2-currency future + free headroom. Decision unchanged. |
| **H-2** | HIGH | B-CONSIST/B-DATA | **accept-risk (Phase A) + defer-flag MISSING (Phase-B cutover, O-7)** | S5 orders/money lead | Bare-int `Serialize` is exact Rust-to-Rust but JS `JSON.parse`→f64 rounds >2^53. No JS consumer in Phase A → do not block. Recorded must-solve-before-cutover: string-encode or f64-bound at the first browser-touching JSON boundary. Interim guardrail: >2^53 exact-round-trip test. |
| **M-1** | MED | B-OPS | **FIX** | Crate maintainer | `Overflow` gains `{ op: &'static str, lhs, rhs }`; `Display` renders op + operands → the key alert variant is diagnosable <1 min. |
| **M-2** | MED | B-FAIL | **accept-risk / non-issue (pinned by test + impl constraint)** | Crate maintainer | `abs`-of-`i64::MIN` is unreachable *by design*: `qty < 0` early-returns `NegativeQuantity` before any multiply/abs. Pinned: forbid `.abs()`/`.unsigned_abs()` in impl; DoD test `checked_mul_qty(i64::MIN)` → `NegativeQuantity(i64::MIN)`. |
| **M-3** | MED | B-CONSIST/B-SEC | **FIX (doc)** | This proposal | Clarified: validating `Deserialize` guarantees **sign only, not authority**. A wire `Lek(1)` is well-formed, not a trusted charge; server-authority over amount stays at the order txn. Removed language that could license trusting a wire `Lek`. |
| **M-4** | MED | B-DATA | **defer-flag MISSING (Phase-B write lane, O-9)** | S5 lead | No i32 storage-boundary code exists in Phase A. Phase B: `i32::try_from` only, plus a lint banning `as`-casts on money, landing in the **same** lane as the first write path. |
| **M-5** | MED | B-ANTIPATTERN | **accept-risk (intentional) + add `Lek::ZERO`** | S5 lead (owns over-discount modelling) | No saturating/clamp method: it would silently lose money (the A3 failure the design rejects) and matches production `assertNonNegative` (throws, not clamps). Added `pub const ZERO` so a legitimate over-discount floor is *explicit & greppable* (`.unwrap_or(Lek::ZERO)`), never hidden. |
| **L-1** | LOW | B-OPS/B-SEC | **FIX (doc narrowing) + defer (redaction Phase-B, O-8)** | S5 / egress lead | Dropped the blanket "money is never sensitive" waiver. Scalar is context-free, but amount + downstream context = GDPR-relevant financial data; redaction is a call-site/egress concern, not a type property. `Debug` stays derived (needed for tests/dev). |
| **L-2** | LOW | B-ANTIPATTERN | **FIX (trim `Hash`, keep `Ord`)** | Crate maintainer | Dropped `Hash` (no Phase-A consumer; money-as-map-key is a smell). Kept `PartialOrd/Ord/Eq/PartialEq` — comparing money magnitudes is a near-certain, correct, safe need. Re-add `Hash` only when a keyed collection needs it. |
| **L-3** | LOW | Doc-integrity | **accept-risk (worktree-sync fact, non-bug)** | Merge lead | Frame docs exist in the main tree, absent from this scaffold worktree (glob-confirmed 0 files). Reconciliation is verifiable against the main checkout and re-checkable on merge. Noted in ADR/O-6. |

**Tally:** FIX ×5 (H-1, M-1, M-3, L-1-doc, L-2) · accept-risk ×4 (H-2-PhaseA, M-2, M-5, L-3) ·
defer-flag MISSING ×3 next-stage items (H-2→O-7, M-4→O-9, plus L-1 redaction→O-8) · human-decision flag ×1 (O-10).

---

## Counsel advice — dispositions

| Counsel item | Disposition |
|---|---|
| **§3.1 — pull the raw-`i64`-money lint forward (O-1).** | **Adopted.** O-1 rewritten: the lint lands **with the first Phase-B `Lek` call-site**, not as a later ratchet. Rationale recorded (an open escape hatch = convergence theater; `?`-verbosity is the pressure to route around, so close it early). |
| **§5 / §3.2 — name a home for directional/owed money before the first refund path.** | **Adopted as recommendation + HUMAN DECISION flagged (O-10, §10a).** Recommend a named Phase-B deliverable (`SignedLek`/`Delta`/ledger entry), S5 council owns, designed before the first Rust refund/payout, so "refund exceeds order" is a modelled outcome not a silent floor. The choice "deliverable-now vs accepted-latent-risk" is explicitly left to product + S5 lead. |
| **§3.3 — `Qty` newtype is YAGNI for now.** | **Agreed.** `checked_mul_qty(qty: i64)` keeps a raw `i64` in Phase A; `Qty` noted as a Phase-B candidate so the eventual choice is deliberate. Not built now. |
| **§3.4 — serde asymmetry is correct taste; O-5 test is sufficient.** | **Affirmed, no change.** Validate-in / emit-bare stays; the negative-JSON guardrail test is kept as-is. |
| **§4 — steel-man: A2's ergonomics critique survives its dead enforcement mechanism.** | **Recorded (proposal §10a), no design change.** Result-API is chosen over a *strong* A2, not a strawman: `checked_sub`-below-zero wants to be a handled business error (not a panic), `Result` composes into the one `IntoResponse` enum, one uniform API across four ops. The real `?`-verbosity cost is precisely why the O-1 lint is pulled forward. |

**ETHICAL-STOP:** none raised; none to revise. The one probed red-line (non-negativity vs refunds/amounts-owed)
is a forward caution, now carried as O-10 with a human-decision flag — not a crossed line.

---

## Deltas for the Rust implementer (what changes vs the original proposal)

The decision (`Lek(i64)`, integer minor units, checked-Result, validating `Deserialize` + bare `Serialize`, no
`From<f64>`, no `Add`/`Sub`) is **unchanged**. These are the precise build-deltas the resolution introduces:

1. **`MoneyError::Overflow` is no longer a unit variant** — it is
   `Overflow { op: &'static str, lhs: i64, rhs: i64 }`. `checked_add` passes `op: "add"`; `checked_mul_qty`
   passes `op: "mul_qty", lhs: self.0, rhs: qty`. `Display` renders op + both operands (M-1).
2. **`checked_mul_qty` negativity detection is pinned:** `if qty < 0 { return Err(NegativeQuantity(qty)); }`
   **before** any multiplication. **No `.abs()` / `.unsigned_abs()` anywhere** in the type (M-2).
3. **Add `pub const ZERO: Lek = Lek(0);`** — the only sanctioned clamp-to-zero primitive is an explicit
   `.unwrap_or(Lek::ZERO)` at the call site. **Do not** add `saturating_*`/`clamp_*` methods (M-5).
4. **Drop the `Hash` derive.** Final derive set: `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord`
   (+ `Serialize`, hand-written `Deserialize`) (L-2).
5. **Source doc-comments to add** (so the invariants survive later edits):
   - on `Deserialize`: "sign-checked, NOT authority-checked — a valid `Lek` is a magnitude, not a trusted
     charge; server owns the amount at the order txn" (M-3).
   - on `Serialize`: "KNOWN LIMITATION — bare integer is exact Rust-to-Rust but JS `JSON.parse`→f64 rounds
     values > 2^53; safe only because no JS consumer touches this type in Phase A; see O-7 before any
     browser-touching cutover" (H-2).
6. **DoD test additions (all red→green with the code):**
   - `checked_mul_qty(i64::MIN)` → `Err(NegativeQuantity(i64::MIN))` (M-2 landmine pin).
   - `Overflow` `Display` contains the op name **and** both operands (M-1).
   - serde exact round-trip of a value **above 2^53** (`Lek(9_000_000_000_000_000)`) via
     `to_string`→`from_str` (H-2 interim guardrail; Rust-only — the JS hazard is O-7).
   - `Lek::ZERO == Lek::new(0).unwrap()` (M-5).
   - (kept) construction 0/1/`i64::MAX` ok & `-1` rejected; add-overflow; sub-goes-negative; negative-qty;
     bare-int round-trip; deserialize rejects float/string/**negative**.

**No code ships in this round** — Phase A remains an isolated, un-wired scaffold; the above lands at STOP-CODE-A.

---

## Deferred / next-stage items (MISSING — owned, not silently dropped)

| Ref | Item | Stage | Owner |
|---|---|---|---|
| **O-1** | raw-`i64`-money clippy/dylint rule | Phase B — with the **first** `Lek` call-site | S5 lead |
| **O-7** | f64-cliff mitigation (string-encode or f64-bound) + test | Phase B — first browser/Node-touching JSON boundary | S5 lead |
| **O-8** | amount-redaction at AI/queue/analytics egress | Phase B — egress boundary | Egress lead |
| **O-9** | `i32::try_from`-only write path + `as`-cast-on-money lint | Phase B — first write path (S5) | S5 lead |
| **O-10** | directional/owed money type (`SignedLek`/`Delta`/ledger) | Phase B — before first refund/payout path · **HUMAN DECISION** | Product + S5 lead |

---

## Human decision required (single item)

**O-10 — is directional/owed money a named Phase-B deliverable with an owner *now*, or an accepted latent risk
until it bites?** Architect recommendation: **named deliverable, S5 council owns**, designed before the first
Rust refund/payout, so a refund exceeding an order is always a modelled business outcome and never floored to
zero (which would silently cost the customer or courier money they are owed — the parties least able to
notice). This does not change the Phase-A scalar. Awaiting product + S5-lead ruling.
