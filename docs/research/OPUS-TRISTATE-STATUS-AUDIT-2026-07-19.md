# Tri-State Status Modeling Audit — money / payment / verification

**Date:** 2026-07-19
**Scope:** RESEARCH-ONLY. Zero code written, no branches touched. Audit of whether dowiz's
money/payment/verification code uses proper multi-variant status enums where a genuine
"not-yet-known" state exists, or collapses a real 3-state situation into a bare `bool`.
**Explicitly out of scope (settled red-lines, NOT revisited):** `money.rs`'s exact-i64 amounts;
the crypto oracle's full-strength signature verification.

## Verdict (headline)

**Everything material is already correctly modeled. No genuine bare-bool tri-state collapse
exists anywhere in the money, payment-adapter, or signature-verification surfaces.** Rust's type
system (multi-variant `enum`, `Option`, `Result`) is used idiomatically throughout, and the
"not-yet-known" state is represented **explicitly** wherever it genuinely exists. The one cosmetic
whiff (a redundant derived bool) loses no information and is not a defect. This audit did **not**
manufacture a gap — there isn't one to fix.

---

## 1. `kernel/src/money.rs` — ledger / transaction lifecycle

### Status/state-tracking types found
| Type | file:line | Shape | Verdict |
|------|-----------|-------|---------|
| `EntryKind` | money.rs:137-144 | 2-variant enum `{ Earn, Reversal }` | Correct |
| `LedgerEntry.reverses` | money.rs:155 | `Option<u64>` | Correct |
| `OrderTotalEstimate.fee_known` | money.rs:373 | `bool` | Redundant, not a gap |
| `OrderTotalEstimate.min_not_met` | money.rs:383 | `bool` | Correct (genuinely binary) |
| `delivery_fee` / `tax_total` / `total` | money.rs:375-381 | `Option<i64>` | Correct (Option = "unknown") |

**Ledger lifecycle is event-sourced, not status-fielded.** There is deliberately **no**
`settled/pending/failed` field on a ledger entry. A leg is either an `Earn` or its `Reversal`
(EntryKind, money.rs:137). Whether an earn leg is "reversed" is **derived** by scanning for a
`Reversal` whose `reverses: Option<u64>` (money.rs:155) names it — see `ledger_sum` (money.rs:230-245)
and the "at most one reversal per earn" guard (money.rs:215-219). This is the correct event-sourced
modeling of a double-entry ledger: state is a fold over immutable append-only entries, **not** a
mutable status bool. A bare `bool reversed` would be *worse* here, not better — it would duplicate
truth that the entry log already carries by construction. The conservation invariant
(`ledger_sum == 0` at a compensated terminal, money.rs:124-134) is the falsifier.

**`OrderTotalEstimate` — the only place worth scrutinising, and it holds up.** The genuine
"fee/tax/total is UNKNOWN" state (distance-tiered server-only fee, or an overflowing tax
computation) is modeled by `Option::None` on `delivery_fee`, `tax_total`, and `total`
(money.rs:375-384), with the estimator fail-closing to `None` rather than fabricating a zero
(money.rs:408-425; test `red_tax_overflow_degrades_estimate_to_none`, money.rs:768-784). That is
the tri-state (`Some(n)` value / `Some(0)` known-free / `None` unknown) done **right** via `Option`.

- `fee_known: bool` (money.rs:373) is set to `delivery_fee.is_some()` (money.rs:417) — a **redundant
  mirror** of the Option's discriminant. It loses no information (the `None` already encodes
  "unknown"); it is convenience, not a collapse. Not a defect. If anything, ponytail-cuttable, but
  removing it is a cosmetic call, not a correctness fix.
- `min_not_met: bool` (money.rs:383) is genuinely binary: `subtotal < min`, and `None` min config
  correctly collapses to "met" (money.rs:413-416). No third state exists here.

**Money value types** (`Currency` money.rs:29, `Money` money.rs:59) are values, not status, and
already fail-closed on cross-currency / overflow via `Result` (`checked_add`/`checked_sub`/
`checked_neg`, money.rs:71-121). Not in the status-tracking scope, no bool involved.

---

## 2. `kernel/src/ports/payment_provider.rs` — P60 payment-adapter core

**This is the exemplar of correct tri-state (in fact N-state) modeling. Reference it, don't
change it.**

| Type | file:line | Variants | Note |
|------|-----------|----------|------|
| `PaymentStatus` | :79-88 | `NoneYet, IntentCreated, Authorized, Captured, Voided, Refunded, Failed(FailReason)` | **`NoneYet` is the explicit "not-yet-known"** |
| `FailReason` | :90-96 | `Declined, Expired, ProviderError, Cancelled` | typed failure cause |
| `LegState` | :131-139 | `Draft, Authorized, AuthFailed(FailReason), Captured, Voided, CaptureStuck` | per-leg lifecycle |
| `NLegOutcome` | :142-156 | `Committed, Aborted{void_set}, NeedsReconciliation{stuck,captured}` | partial terminal is an **explicit variant** |
| `CaptureOutcome` | :398-402 | `Captured, Stuck` | genuinely binary per-leg |
| `PayError` | :195-204 | 7 typed poles | never panic/silent |

The exact "confirmed-failed vs not-yet-checked" distinction the task worries about is modeled
**precisely**: `query_status_by_key` returns `PaymentStatus::NoneYet` for an unseen key —
documented as *"not an error, not a fabricated success"* (:701-710), and `Failed(FailReason)` is a
**separate** variant. A caller can always tell "checked and failed" (`Failed(_)`) from "never
checked" (`NoneYet`). The webhook is the sole writer of Authorized/Captured/Voided/Refunded truth
(:77-88, :712-719).

`NLegOutcome::NeedsReconciliation` (:150-156) makes the genuinely-uncertain "some captured, some
stuck" terminal a **first-class, operator-visible** state — and the doc + `assert_nleg_atomicity`
(:492-534) prove a captured+voided-without-reconciliation terminal is **unrepresentable**. This is
make-illegal-states-unrepresentable, the opposite of a bool collapse.

**Bools present here are all genuinely binary** — no tri-state hiding in any of them:
- `TokenEntry.used: bool` (:590) — a single-use session token is consumed or not. Binary fact.
- `OutstandingIntentGate` / `edge_challenge_ok -> bool` (:538, :557) — gate predicates.
- `MockProvider.captured: bool` (:136,:140) — a **test fixture**, not product state.

`IdemLedger` (:281-369) resolves to `PaymentStatus` with the `NoneYet` fallback (:701-710) — the
"unknown" is threaded end-to-end, never coerced away.

---

## 3. Crypto oracle / signature verification result types

**Clean, deliberate two-layer design. No coercion gap.**

### Layer A — raw crypto primitives return `bool` (correct)
- `dsa.rs::verify(pk, msg, sig) -> bool` and `verify_internal_bytes -> bool` (ML-DSA-65)
  — dsa.rs:1003, :911.
- `SignatureVerifier` trait: `verify_classical -> bool`, `verify_pq -> bool`
  — ports/agent/cap.rs:82-94.
- `HybridSig::verify -> bool` (capability_cert.rs:154-160), `CertDelegation::verify_signature ->
  bool` (:482-499).

A cryptographic signature check has **exactly two mathematical outcomes**: valid or invalid. There
is no third crypto outcome. **"Not yet checked" is not a value the function can return** — it is the
state *before* you call it. `bool` is the idiomatic, correct primitive at this layer (it matches the
NIST ACVP pass/fail semantics the code is KAT-gated against, dsa.rs:4). Promoting these to
`Result<(), E>` would invent failure sub-reasons that do not exist at the raw-verify boundary — a
false richness. This is **not** a bool-collapse; it is the one place bool is exactly right.

### Layer B — policy/chain verification returns `Result<(), Error>` with typed enums (correct)
Where multiple **distinguishable** failure reasons genuinely exist, the code uses `Result` + a
variant enum, not bool:
- `root_delegation.rs::verify_root -> Result<(), RootVerifyError>`, `RootVerifyError { BadRootSignature,
  Unsupported, MaxDepthExceeded }` — root_delegation.rs:63-93. Distinguishes "cryptographically bad"
  from "policy-unsupported" from "depth-exceeded" — the exact distinctions a bool would erase.
- `capability_cert.rs::verify_self / verify_chain_hybrid -> Result<(), CertError>` — :276, :776-784.
- `hybrid.rs::hybrid_decaps -> Result<[u8; 32], &'static str>` — hybrid.rs:92-100 (`"key-confirmation-
  failed"`); the RED gate rejects any degraded leg, no classical-only fallback.
- `ports/payment.rs::SettlementOutcome { Recorded, Rejected(SettlementReject) }` (:90-99) with a
  6-variant `SettlementReject` (:104-125) — `CourierCertRejected` carries the cert-verify failure
  reason instead of a bare false.

### "Not yet checked" is carried by Option/absence, by construction
- `PaymentStatus::NoneYet` (payment_provider.rs:79).
- `OrderSettle.settled: Option<i64>` — `None` = not yet settled, `Some(amount)` = settled + amount,
  used as the idempotency key (ports/payment.rs:170-171). Textbook tri-state via `Option`.
- `query_status_by_key` → `NoneYet` for an unseen key (payment_provider.rs:701-710).

### Is a rich result ever coerced to a bare bool, losing the distinction?
**No.** Every `-> bool` consumer audited (envelope.rs:69; capability_cert.rs:292, :358, :495, :680;
hub_provisioning.rs:398, :430, :496) consumes a **leaf** crypto primitive whose only two outcomes
are valid/invalid — it is not throwing away a Result's error detail. The higher layers that *have*
multiple failure reasons already return `Result`/enum and propagate with `?` (capability_cert.rs:680,
:724). There is no point where a `Result<_, SomeError>` or a multi-variant status is squashed into a
bool and the "unknown vs failed" distinction is lost.

---

## 4. Adjacent lifecycle types (sanity sweep, all correct)
- `order_machine.rs::OrderStatus` — 12-variant enum incl. `Pending`, `Refunding`,
  `CompensatedRefund` (order_machine.rs:8-25); unknown strings rejected, never silently mapped (:29-45).
- `wallet/transfer.rs::TransferState` — 8-variant enum with `Failed(TransferError)` and a mandatory
  `AwaitingConfirmation` gate (transfer.rs:73-99); sealing without confirmation is unrepresentable.
- `wallet/outbox.rs::ReconnectOutcome` — 3-variant enum carrying `PaymentStatus` (outbox.rs:96-104).
- `wallet/record.rs::WalletStoreError`, `wallet/draft.rs::DraftState/DraftFoldError` — all enums.

---

## 5. Would-be enums, for completeness (NOT recommended changes)

No change is recommended. If a maintainer ever wanted to remove the single redundancy, the only
candidate is `OrderTotalEstimate.fee_known: bool` (money.rs:373) — it is exactly `delivery_fee
.is_some()` (money.rs:417) and could be dropped so callers read the `Option` directly. This is a
**ponytail cosmetic cut, not a correctness fix**, and touching money.rs's public estimate struct
carries more risk than the ~1 line of redundancy it removes. Leave it.

There is **no** place where a proper `enum TransactionStatus { Pending, Settled, Failed }` or
`enum VerificationState { Unverified, Valid, Invalid }` needs to *replace* a bool, because:
- transaction/settlement lifecycle is already event-sourced (`EntryKind` + derived state;
  `SettlementOutcome`; `PaymentStatus` with `NoneYet`);
- verification already splits correctly into `bool` at the 2-outcome crypto leaf and
  `Result<_, enum>` at the N-outcome policy layer, with "unverified" carried by `Option`/`NoneYet`.

## Tool calls
7 (4 Read, 3 Bash). Real source inspected: `money.rs` (full), `ports/payment_provider.rs` (full),
`pq/hybrid.rs` (full), `ports/payment.rs`, `order_machine.rs`, `wallet/transfer.rs`,
`wallet/outbox.rs`, `pq/dsa.rs`, `pq/root_delegation.rs`, `capability_cert.rs`,
`ports/agent/cap.rs` (via grep + targeted reads).
