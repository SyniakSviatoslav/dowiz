# BLUEPRINT — Layer D: Consensus / Trust / Capability — budgeted anchor-rooted issuance (2026-07-17)

> **RECONSTRUCTED 2026-07-17.** The original of this file was lost before commit (root cause
> confirmed: a concurrent consolidation session merged 20 branches onto `main` while this file
> existed only on disk, uncommitted, in an untracked directory — no git recovery possible). This
> reconstruction was rebuilt from the original's completion summary, with **every code citation
> re-verified fresh against the live tree this pass** (dowiz `main` HEAD `caba2203c`; bebop-repo
> worktree `feat/verification-harness`, byte-identical to `openbebop/main` for every cited file —
> `git diff --stat openbebop/main` empty on all three). **Citation drift found: NONE** — all four
> load-bearing spans hold exactly (§1).
>
> **Sibling audit status: ON DISK and reconciled.** `P-D-audit-root-delegation-policy.md` was
> itself reconstructed in parallel and landed while this blueprint was being written; this
> blueprint was cross-checked against it before finalizing. Agreement is total on the load-bearing
> content: the `IssuanceBudget` struct (audit `:170-175` — field-for-field identical to §3.2
> here), the pure-predicate-at-sign-time shape, the A/B/C dispositions (A default / B
> operator-gated / C deferred on Cheng–Friedman), and the P06-independence correction (audit §3).
> Where this blueprint goes further (dedicated `IssuanceError` with 7 poles instead of the audit
> §6.1's lighter "new `CapError`/`GenesisError` variant", the full seam signature, the 10-vs-3
> adversarial test), it *refines* the audit per the lost original's own spec — noted inline. The
> audit's `:118,195` attestation-precondition slots supersede the older `P-D-audit:131-135,107,217`
> cites preserved in `BLUEPRINT-P-E-network-crypto-core.md` §1.
>
> **Contract:** written against all 20 points of
> [`CORE-ROADMAP-STANDARD-2026-07-17.md`](../CORE-ROADMAP-STANDARD-2026-07-17.md) §2 — compliance
> map in §10. **Operator docket:** R-3 (`RootDelegationPolicy`) remains **open**; §3 explains why
> the build is NOT blocked on it (fail-closed by construction until the ruling).

---

## §0. The problem, one paragraph, no metaphor

Sybil-resistance in the bebop2 mesh is already the theorem-permitted asymmetric kind: authority
exists only as an anchor-rooted, narrow-only, signed delegation path (`verify_chain`), so N free
keypairs are inert (`CapError::UnknownIssuer`). Batch 7 proved this PROVEN-VIABLE-WITH-CAVEATS and
isolated the one real residual (§6 there): **the whole defense reduces to how disciplined an anchor
is when it signs a delegation**. If an anchor's signing client is compromised or careless, an
attacker gets N *legitimately-anchored* capabilities and the mechanism is operationally void — a
free-issuance CA rebuilt. The missing piece is small and pure: **a per-anchor issuance budget
checked at delegation-sign time** — a bounded predicate over the existing
`AnchorRoster`/`verify_chain`/`RevocationSet` substrate, no new consensus machinery. This
blueprint specifies that piece exactly (types, constants, functions, file, line placement), plus
the operator-gated attestation overlay (Option B) and the deferred Web-of-Trust branch (Option C)
with its hazard pinned by a test.

---

## §1. Ground truth (contract item 1 — every cite verified THIS pass, 2026-07-17)

Crate: **`bebop-proto-cap`** (`/root/bebop-repo/bebop2/proto-cap/Cargo.toml:2`). All paths below
relative to `/root/bebop-repo/bebop2/proto-cap/src/` unless stated.

| Claim | Cite (fresh) | Status vs lost original |
|---|---|---|
| `RootDelegationPolicy` marker enum (`OperatorSigned` / `WebOfTrust` / `FirstContactQr` / `Unspecified`), `Default = Unspecified` fail-closed, `require_explicit_policy` refusal | `node_id.rs:156-184` (enum 156-166, `Default` 168-174, `require_explicit_policy` 179-184) | **EXACT — unchanged** |
| Module doctrine: "code MUST NOT silently pick one as 'chosen' … Do not 'helpfully' default to a real policy" | `node_id.rs:21-28` | holds |
| `Delegation` struct + canonical-TLV signing input + `Delegation::sign` (real Ed25519, RFC 8032, via `bebop2_core::sign::sign`) | `roster.rs:98-169` (struct 98-118, `canonical_bytes` 126-143, `sign` 148-169) | **EXACT — unchanged** |
| `verify_chain` — anchor-rooted admission: (a) root ∈ roster, (b) chain alignment, (c) narrow-only attenuation, (d) tail-subject binding, (e) effect ⊆ tail scope, (f) per-link Ed25519, (g) expiry | `roster.rs:252-316` | **EXACT — unchanged** |
| NO-COURIER-SCORING structural red-line ("enforced here and NOWHERE ELSE… no score / rating / trust / reputation / rank field"), double-locked by `scripts/ci-no-courier-scoring.sh` | `claim_machine.rs:13-17` | **EXACT — unchanged** |
| `AnchorRoster` — genesis-frozen set; `enroll` 205, `contains` 210, `is_empty` 215, `remove` (MESH-11 drop-anchor) 219-225 | `roster.rs:192-226` | holds |
| `RevocationSet` — monotonic, fact-triggered, never score-triggered; `revoke_key` :69, `is_revoked_key` :81, `merge`/`gossip_payload` :94,:114, `drop_anchor` :105 | `revocation.rs:49-129` | holds |
| Genesis loader fail-closed (missing/malformed/zero-anchor ⇒ error, no authority captured); `GenesisError::PolicyUnspecified` exists | `node_id.rs:84-141` | holds |
| Existing RED tests: empty-roster-no-capture, seeded-owner-cannot-mint, policy-must-be-chosen | `node_id.rs:264-377` | holds |
| Capability shape: `subject_key` (Ed25519), `subject_key_pq` (`Option<Vec<u8>>`, ML-DSA-65), `scope`, `nonce`, `expiry` as caller-supplied monotonic tick | `capability.rs:45-60` | holds |
| C4b — constant-time `mod_l` group-level fix still open on the Ed25519 signing path | `/root/bebop-repo/bebop2/core/src/sign.rs:661-692` ("group-level fix left open: the prior `mod_l` had a secret-bit branch") | holds — see §7 |
| Batch 7 verdict + the design core this blueprint executes: "a per-anchor monotonic issuance epoch/nonce budget checked *at delegation-sign time* (a pure predicate, no monitor)" | `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/16-BATCH7-sybil-proof-capability-mechanism-findings.md` §4, §6 (esp. lines 281-287) | holds |

**Key structural fact for §7:** the entire mint path (`Delegation::sign` → `bebop2_core::sign::sign`,
`roster.rs:167`) and the entire admission path (`verify_chain` → `Delegation::verify_signature`,
`roster.rs:172-184, 275`) are **Ed25519-only**. No ML-DSA, no `key_V`, anywhere on either path.

---

## §2. The three options (from the P-D audit; dispositions ratified here)

### Option A — `OperatorSigned` + per-anchor issuance budget · **RECOMMENDED DEFAULT, fully buildable now** → §3–§6

### Option B — `FirstContactQr` + hardware-attestation precondition, stacked ON TOP of A · **OPERATOR-GATED** → §8 (explicit STOP marker)

### Option C — `WebOfTrust` · **DEFERRED on Sybil risk** → §9 (hazard pinned by a standing test)

Framing per the audit (§4 there): `FirstContactQr` is Option B's enrollment channel — physical
QR commissioning *plus* a device-attestation precondition, additive over A's budget, never a
replacement for it. Until the §8 gate opens, `FirstContactQr` has no wired issuance rule and the
seam refuses it (`PolicyRefused` pole). The audit's key structural finding motivating all of
this (audit §2): the three real enum variants are today **names without mechanism** — nothing in
`proto-cap/src/` branches on which one was chosen; `require_explicit_policy` distinguishes only
`Unspecified` from everything-else. Closing Layer D is therefore a *build* item, not a config
item, and §3 is that build.

---

## §3. Option A — full design

### §3.1 The predicate logic

An anchor may sign a new delegation **iff** all of the following pure predicates hold, evaluated
at sign time against caller-supplied inputs (no clock read, no I/O, no observer — Hermetic P6):

1. `policy == OperatorSigned` — the only policy with a wired budget rule today. Everything else
   refuses (`Unspecified` fail-closed as before; `WebOfTrust`/`FirstContactQr` refuse until their
   own budget rules are specified — never guess).
2. `roster.contains(issued_by)` — the mint-time mirror of `verify_chain`'s check (a): an
   un-enrolled key cannot *mint*, not merely cannot *be admitted*.
3. `!revoked.is_revoked_key(issued_by)` — a revoked anchor mints nothing, even before the roster
   drop propagates.
4. `budget.anchor_id == issued_by` — budgets are per-anchor and non-transferable.
5. `budget.epoch <= issuance_epoch(now_tick)` — a budget record from the *future* means clock
   rollback or a tampered record; refuse (monotonicity is the invariant, not a convention).
6. effective minted count in the current epoch `< max_per_epoch` — **the Sybil pole**. Epoch
   advance re-arms the budget (a *pure function of `now_tick`*, not a scheduled job — nothing has
   to run for the epoch to roll).

Only after all six does `Delegation::sign` run; the budget is charged **after** a successful sign
(a refusal or sign failure never burns budget). **Caller commit rules (the exactly-once
discipline, same law as Layer B's `commit_after_decide`):**

- persist the returned charged budget *before* releasing the delegation bytes to anyone. Crash
  between sign and persist then loses the delegation, not the budget — collapse direction: fewer
  live delegations, never a free mint (Hermetic P4, safe-directed collapse);
- the store's write predicate is **monotonic** (audit DoD-3): accept a new record iff
  `new.epoch > stored.epoch || (new.epoch == stored.epoch && new.minted_count >= stored.minted_count)`
  — a replayed stale (lower-epoch or lower-count) budget record can never re-arm a spent budget.

**Why this binds (and exactly whom):** an attacker who compromises the anchor's *signing client /
automation* is stopped at `max_per_epoch` mints per epoch — the blast radius of a careless or
captured onboarding box drops from "unbounded CA" to a small constant per day. See §6 for the
honest boundary of this claim (a *malicious anchor holding its own seed* is a different threat
with a different, already-built answer).

### §3.2 EXACT types and constants (contract item 4 — these names are the spec)

The `IssuanceBudget` struct below is reproduced **verbatim from the lost original** (load-bearing;
do not restyle). `IssuanceError`'s seven pole *names* are reconstructed-canonical (the original
specified "7 typed refusal poles"; the summary did not preserve the identifiers — if the recovered
P-D audit surfaces different names, reconcile there and update here).

```rust
/// Per-anchor issuance budget — the pure predicate state for budgeted minting.
/// 48 bytes. Anchor-local record; NOT on the wire (see §5 scaling note).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuanceBudget {
    pub anchor_id: [u8; 32],   // enrolled Ed25519 anchor pubkey, NOT NodeId
    pub epoch: u64,            // monotonic: now_tick / epoch_len_ticks
    pub minted_count: u32,
    pub max_per_epoch: u32,
}

/// The seven typed refusal poles of budgeted issuance. Every refusal is one of
/// exactly these — no stringly-typed reasons, no catch-all (Hermetic P4: named
/// poles; contract item 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssuanceError {
    /// 1. Policy is `Unspecified`, or a policy with no wired budget rule yet
    ///    (`WebOfTrust` deferred §9; `FirstContactQr` gated behind Option B's
    ///    operator ruling §8). Fail-closed — never guess a policy.
    PolicyRefused(RootDelegationPolicy),
    /// 2. The signing key is not an enrolled anchor (mint-time mirror of
    ///    `CapError::UnknownIssuer`).
    AnchorNotEnrolled,
    /// 3. The anchor key is in the `RevocationSet` — a revoked anchor mints nothing.
    AnchorRevoked,
    /// 4. `budget.anchor_id` ≠ the anchor attempting to sign. Budgets are
    ///    per-anchor, non-transferable (this is a mint-count, NOT a currency —
    ///    see R-2 note, §5).
    AnchorMismatch,
    /// 5. `budget.epoch` is AHEAD of `issuance_epoch(now_tick)`: clock rollback
    ///    or tampered record. Monotonicity violated — refuse.
    EpochRegression,
    /// 6. Current-epoch minted count has reached `max_per_epoch` — THE Sybil pole.
    BudgetExhausted,
    /// 7. The inner `Delegation::sign` refused (budget NOT charged). Currently
    ///    unreachable — `canonical_bytes` (roster.rs:126-143) is infallible today;
    ///    the pole exists so a future fallible encoder can never silently charge.
    SignRejected(CapError),
}

/// One epoch = 86 400 ticks (one day at the crate's 1-tick-per-second expiry
/// convention, capability.rs:57-59).
pub const DEFAULT_ISSUANCE_EPOCH_LEN_TICKS: u64 = 86_400;
/// Default: ONE new identity per anchor per epoch. Deliberately parsimonious —
/// the operator raises it per anchor when onboarding demands it, never the code.
pub const DEFAULT_MAX_PER_EPOCH: u32 = 1;
```

### §3.3 EXACT functions (decide/fold shape — mirrors `order_machine`/`claim_machine` law, Hermetic P2)

```rust
/// Pure epoch derivation. `epoch_len_ticks == 0` collapses to ONE ETERNAL epoch
/// (epoch 0 forever ⇒ at most `max_per_epoch` mints EVER) — the safe pole, not
/// a divide-by-zero and not a per-tick epoch (which would neuter the budget).
pub fn issuance_epoch(now_tick: u64, epoch_len_ticks: u64) -> u64 {
    if epoch_len_ticks == 0 { 0 } else { now_tick / epoch_len_ticks }
}

/// DECIDE — pure refusal/allow over predicates 4–6 of §3.1. No mutation, no I/O.
pub fn can_issue(
    b: &IssuanceBudget,
    anchor_id: &[u8; 32],
    now_tick: u64,
    epoch_len_ticks: u64,
) -> Result<(), IssuanceError> {
    if b.anchor_id != *anchor_id {
        return Err(IssuanceError::AnchorMismatch);
    }
    let e = issuance_epoch(now_tick, epoch_len_ticks);
    if b.epoch > e {
        return Err(IssuanceError::EpochRegression);
    }
    // Epoch advance re-arms: count from an older epoch does not carry forward.
    let effective = if e > b.epoch { 0 } else { b.minted_count };
    if effective >= b.max_per_epoch {
        return Err(IssuanceError::BudgetExhausted);
    }
    Ok(())
}

/// FOLD — apply one successful mint. Pure; call only after `can_issue` passed
/// and `Delegation::sign` succeeded. Rolls the epoch forward (resetting the
/// count) before charging.
pub fn charge_issuance(mut b: IssuanceBudget, now_tick: u64, epoch_len_ticks: u64) -> IssuanceBudget {
    let e = issuance_epoch(now_tick, epoch_len_ticks);
    if e > b.epoch {
        b.epoch = e;
        b.minted_count = 0;
    }
    b.minted_count = b.minted_count.saturating_add(1);
    b
}

/// THE SEAM — the first and only consumer of `RootDelegationPolicy` on the mint
/// path. Today the policy enum is produced and validated (`require_explicit_policy`)
/// but consumed by NOTHING; after this lands, every budgeted mint flows through it.
pub fn sign_delegation_budgeted(
    policy: RootDelegationPolicy,
    roster: &AnchorRoster,
    revoked: &RevocationSet,
    budget: IssuanceBudget,
    issued_by: [u8; 32],
    subject: [u8; 32],
    scope: Scope,
    effect: Effect,
    expiry: u64,
    nonce: [u8; 8],
    seed: &[u8; 32],
    now_tick: u64,
    epoch_len_ticks: u64,
) -> Result<(Delegation, IssuanceBudget), IssuanceError> {
    match policy {
        RootDelegationPolicy::OperatorSigned => {}
        other => return Err(IssuanceError::PolicyRefused(other)),
    }
    if !roster.contains(&issued_by) {
        return Err(IssuanceError::AnchorNotEnrolled);
    }
    if revoked.is_revoked_key(&issued_by) {
        return Err(IssuanceError::AnchorRevoked);
    }
    can_issue(&budget, &issued_by, now_tick, epoch_len_ticks)?;
    let d = Delegation::sign(issued_by, subject, scope, effect, expiry, nonce, seed)
        .map_err(IssuanceError::SignRejected)?;
    Ok((d, charge_issuance(budget, now_tick, epoch_len_ticks)))
}
```

**Budget-record loss rule (Snapshot-Re-entry class, contract item 13):** if the persisted budget
record for an anchor is lost/corrupt, recreate it as
`IssuanceBudget { anchor_id, epoch: issuance_epoch(now), minted_count: max_per_epoch, max_per_epoch }`
— i.e. **exhausted until the next epoch boundary**. Recovery direction is always toward fewer
mints, never a fresh budget windfall.

### §3.4 Placement (contract item 18 — agent-executable, zero session context needed)

- **File:** `/root/bebop-repo/bebop2/proto-cap/src/node_id.rs` — this module already owns the
  policy enum and the genesis/fail-closed doctrine; the budget is the policy's enforcement arm
  (reuse-first, contract item 19: extend the existing module, no new file).
- **Insertion point:** immediately **after `require_explicit_policy` (currently ends line 184)**
  and before the `// ── small offline hex helpers` section (currently line 186). Add a new
  section comment `// ── Budgeted issuance (Layer D, Option A) ─────`.
- **New imports at top of file:** `use crate::error::CapError;` is already imported (line 33);
  add `use crate::revocation::RevocationSet;`.
- **Tests:** extend the existing `#[cfg(test)] mod tests` in the same file (§4 below).
- **Do NOT touch** `roster.rs` — `verify_chain` and the wire format are unchanged by construction
  (§5 bulkhead). DoD includes proof of this (§6).
- **Run:** `cd /root/bebop-repo && cargo test -p bebop-proto-cap` (expect current suite green +
  new tests; RED first per §4 discipline).

---

## §4. Spec-driven, event-driven TDD plan (contract items 2, 3, 5, 17)

Order is binding: §3.2's types are the spec (written first — this document *is* that artifact);
the tests below are written second and must FAIL (not compile / assert-fail) before §3.3's bodies
land; code lands third. Tests assert on **outcome sequences** (which pole refused, in which
order), not only end-state — matching the kernel's decide/fold law.

### §4.1 THE adversarial centerpiece (RED today — types don't exist yet): 10 mints vs cap 3

Attacker model: the anchor's onboarding client is captured (or scripted carelessly); it requests
10 delegations for 10 free Sybil keypairs in one epoch. Budget cap under test: 3. Required
outcome: mints 1–3 succeed and are REAL (verify end-to-end); mints **4–10 are each refused on the
same typed pole**; the fold stops at 3.

```rust
// Stays in the suite permanently as regression_budget_cap_refuses_mint_4_of_10
// (REGRESSION-LEDGER row — §6 DoD-5).
#[test]
fn red_attacker_10_mints_against_cap_3_refused_from_4th() {
    let (anchor_seed, anchor_pk) = k(30u8);
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);
    let revoked = RevocationSet::new();

    let epoch_len = DEFAULT_ISSUANCE_EPOCH_LEN_TICKS;
    let now = 10 * epoch_len + 17; // mid-epoch 10, arbitrary offset
    let mut budget = IssuanceBudget {
        anchor_id: anchor_pk,
        epoch: issuance_epoch(now, epoch_len),
        minted_count: 0,
        max_per_epoch: 3,
    };

    let mut outcomes: Vec<Result<(), IssuanceError>> = Vec::new();
    let mut minted: Vec<Delegation> = Vec::new();

    for i in 0..10u8 {
        let (_s, sybil_pk) = k(100 + i); // 10 distinct free keypairs (Douceur: keys are free)
        match sign_delegation_budgeted(
            RootDelegationPolicy::OperatorSigned,
            &roster, &revoked, budget,
            anchor_pk, sybil_pk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            now + 1_000, [i; 8], &anchor_seed, now, epoch_len,
        ) {
            Ok((d, b2)) => { minted.push(d); budget = b2; outcomes.push(Ok(())); }
            Err(e) => outcomes.push(Err(e)),
        }
    }

    // Event-sequence assertion (contract item 3): the refusal boundary is exact.
    assert_eq!(minted.len(), 3, "mints 1-3 succeed, nothing more");
    for (i, o) in outcomes.iter().enumerate() {
        if i < 3 {
            assert!(o.is_ok(), "mint {} must succeed", i + 1);
        } else {
            assert_eq!(
                *o, Err(IssuanceError::BudgetExhausted),
                "mint {} must refuse on the Sybil pole, not any other", i + 1
            );
        }
    }
    assert_eq!(budget.minted_count, 3, "fold charges exactly the successes");

    // The 3 minted delegations are REAL — the budget bounds quantity, it never
    // corrupts legitimate issuance (each admits via the UNCHANGED verify_chain).
    for d in &minted {
        let cap = Capability::new(d.subject, Resource::Route, Action::Send, d.nonce, now + 1_000);
        assert!(verify_chain(&roster, &[d.clone()], &cap, now).is_ok());
    }
    // Subjects 4-10 hold NOTHING: no delegation was ever signed for them, and any
    // self-minted substitute is already pinned dead by
    // red_seeded_owner_fixture_cannot_mint (node_id.rs:311-345).
}
```

### §4.2 The remaining RED suite (each one predicate, one pole)

| Test | Pins | Expected pole |
|---|---|---|
| `red_unspecified_policy_mints_nothing` | §3.1-1; extends the existing `green_load_genesis_ok_and_policy_must_be_chosen` (node_id.rs:349-377) to the mint path | `PolicyRefused(Unspecified)` |
| `red_web_of_trust_has_no_budget_rule_yet` | §3.1-1 / §9 | `PolicyRefused(WebOfTrust)` |
| `red_unenrolled_anchor_cannot_mint` | §3.1-2 | `AnchorNotEnrolled` |
| `red_revoked_anchor_mints_nothing` | §3.1-3 — `revoke_key` then attempt; also `drop_anchor` variant (revocation.rs:105) | `AnchorRevoked` |
| `red_budget_not_transferable_between_anchors` | §3.1-4 — anchor B presents anchor A's unexhausted budget | `AnchorMismatch` |
| `red_clock_rollback_refuses` | §3.1-5 — budget from epoch 11, `now` in epoch 10 | `EpochRegression` |
| `red_epoch_rollover_rearms_exactly_max` | §3.1-6 — exhaust epoch e, advance `now` to epoch e+1: exactly `max_per_epoch` more succeed, then `BudgetExhausted` again; and rollover is NOT retroactive (the old epoch's delegations remain valid — expiry, not budget, ends them) | sequence |
| `red_epoch_len_zero_is_eternal_epoch` | §3.3 collapse pole — `epoch_len_ticks = 0` ⇒ at most `max_per_epoch` mints ever | `BudgetExhausted` |
| `red_stale_budget_replay_cannot_rearm` | §3.1 store rule (audit DoD-3) — exhaust the budget, then replay the pre-exhaustion record through the monotonic-write predicate: write refused, next mint still `BudgetExhausted` | sequence |

### §4.3 Standing pin (GREEN today, guards Option C drift — see §9)

`red_vote_counting_never_authority` (§9.2) is green against current `verify_chain` and exists to
go RED the day anyone lands symmetric vouch-counting. Intentionally-adversarial per contract
item 5; it is a drift trap, not a RED→GREEN item — stated honestly to keep test-color truthful.

---

## §5. Isolation, scaling, mesh, memory (contract items 8, 11, 12, 15, 16)

- **Bulkhead (11):** the budget lives entirely on the anchor's mint side. `verify_chain`, the
  wire format (`Delegation` TLV), gossip, and every existing capability are untouched — a budget
  bug can refuse new mints (safe direction) but cannot reject, revoke, or corrupt existing
  authority, and cannot slow the admission hot path (zero diff to `roster.rs` is a DoD item).
- **Scaling axis (8):** record = 48 bytes × |anchors|; anchors are genesis-frozen and human-scale
  (tens, not millions). Epoch counter `u64` at 86 400-tick epochs outlives the protocol. **The
  named point where this shape must change:** the budget is *anchor-side honor-enforced* (§6); the
  day anchors span **more than one operating organization** (semi-trusted federation), budgets
  must become *verifier-enforceable* — countersigned issuance receipts carried in the chain or a
  transparency log, so `verify_chain` itself can refuse over-budget mints. That is a different
  blueprint; this schema deliberately does not pretend to be it.
- **Mesh (12):** node-local. Zero new wire bytes, zero new gossip topics, zero transport-layer
  involvement. Revocation gossip (`revocation.rs:114-120`) is unchanged and remains the only
  mesh-visible consequence of anchor misbehavior.
- **Living memory (15):** the budget is a **fold, not a log** — one current record per anchor;
  past epochs are dead state and are overwritten, never archived. The *audit* record is separate:
  each mint/refusal appends a typed event to the anchor's event log (same append-only discipline
  as Layer B), which is where temporal queries live — cross-ref
  `internal-retrieval-living-memory-arc-2026-07-14`.
- **Tensor/spectral (16): DOES-NOT-TRANSFER** (Linux-adoption verdict vocabulary, reused per
  contract item 9). Budget arithmetic is two integer compares and a division; no spectral or
  equation-compiler machinery applies. Honest N/A, not forced application.
- **R-2 disambiguation:** `IssuanceBudget` counts *signing ceremonies*; it is **not** a currency,
  not transferable (`AnchorMismatch` pole is the type-level enforcement), not a B2 money/compute
  budget. This keeps clear of COUNSEL's "currency you call a budget" hazard (Batch 7 §4) and of
  the money-leg red-line — the refundable-bond hardening from Batch 7 §4 stays REJECTED-for-now
  for exactly that reason (viable only under an explicit operator money-gate; not needed for
  Sybil-resistance).

---

## §6. Honest threat model + DoD (contract items 2, 5, 6, 14, 17)

### §6.1 What the budget binds — and what it does not (the honest boundary)

- **BINDS:** honest-but-careless anchors, compromised signing *clients*, runaway onboarding
  automation, fat-fingered batch scripts. These all reach signing through the seam, and the seam
  refuses at the cap. This is the Batch-7 residual (§6 there) closed.
- **DOES NOT BIND:** a **malicious anchor holding its own seed**, which can call
  `Delegation::sign` (`roster.rs:148-169`) directly and skip the seam. No local predicate can
  cryptographically bound a signer who owns the key — claiming otherwise would be a false safety
  claim. The malicious-anchor answer is the *already-built, separately-tested* pair:
  **drop-anchor** (`AnchorRoster::remove`, roster.rs:219-225; `RevocationSet::drop_anchor`,
  revocation.rs:105) + **key revocation** (`revoke_key` :69, gossip-convergent `merge` :94), and
  the structural fact that authority is **re-derived per admission** by `verify_chain` — there are
  no standing grants, so dropping the anchor instantly ends every future admission of every chain
  it ever minted. Fact-triggered, never behavior-monitored (Batch 7 §5 bright line; NO scoring).
- **Reachability argument (item 6, math/type-structure not prose):** an unbudgeted mint by a
  *compliant* caller is unreachable because the seam is the only public budgeted path and the
  refusal poles are total over its inputs (every predicate failure has a typed pole; there is no
  `_ => Ok` arm). An unbudgeted mint by a *bypassing* caller inside the workspace is caught by
  the deterministic gate in §6.3 (CI-time), with a follow-up visibility ratchet
  (`Delegation::sign` → `pub(crate)`) that upgrades the gate to **compile-time** once external
  test callers are migrated — proposed as the R-step after landing, since it is a breaking API
  change.

### §6.2 DoD — falsifiable, RED→GREEN

1. **RED first:** §4.1 + §4.2 committed failing (types absent ⇒ compile-fail counts as RED).
2. **GREEN:** all §4 tests pass; `cargo test -p bebop-proto-cap` fully green; suite count grows
   by ≥ 9.
3. **Hot path untouched:** `git diff --stat -- bebop2/proto-cap/src/roster.rs` is **empty** for
   the landing commit (bulkhead proof; also the perf claim — no admission-path regression is
   possible with zero admission-path diff; issuance itself is a human-ceremony cold path, `can_issue`
   is O(1) integer ops — benchmark honestly N/A, telemetry is the watch instead).
4. **Deterministic gate live (item 14):** §6.3's CI script added and RED-proven (a scratch commit
   with a bare `Delegation::sign` call in non-test, non-seam code must fail CI) before merge.
5. **Regression ledger:** row added to `docs/regressions/REGRESSION-LEDGER.md` naming
   `regression_budget_cap_refuses_mint_4_of_10` (permanent, item 17).
6. **Telemetry (item 10):** the seven refusal poles emit typed counters into the anchor's event
   log — hook shape per `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md`;
   a spike on `BudgetExhausted` is the *signal* an attack or a too-small cap is live (a report,
   never an actor — no watchdog).
7. **Roadmap correction applied:** the remaining edits of §7.2 (the INDEX edit already landed
   with the audit).
8. **R-3 remains open:** production policy stays `Unspecified` (fail-closed) until the operator
   rules; tests pass the policy explicitly. Landing this blueprint does NOT pre-empt the ruling —
   it makes every branch of the ruling already-enforced-by-type.
9. **No-scoring invariant intact (audit DoD-4):** `claim_machine.rs:13-17` unchanged,
   `ci-no-courier-scoring.sh` green — the budget is a counter on the *anchor*, never a rank on
   the *courier*.
10. **P06-independence witnessed (audit DoD-5):** the budget predicate compiles, tests, and runs
    with zero `key_V`/verifier dependency in its build graph — `cargo tree -p bebop-proto-cap`
    shows no dowiz-kernel edge; proves §7 structurally, not by assertion.

### §6.3 The deterministic gate (mirrors the existing pattern — reuse-first)

`scripts/ci-budgeted-issuance.sh` (bebop-repo), same shape as `scripts/ci-no-courier-scoring.sh`
(which double-locks claim_machine.rs:13-17): grep for `Delegation::sign(` call sites outside
(a) `node_id.rs`'s seam and (b) `#[cfg(test)]` modules; any hit fails CI with a pointer to this
blueprint. Wire into the same CI lane as the no-courier-scoring gate.

---

## §7. Correction of the stale "P06 gates Layer D" assumption (verified, not asserted)

### §7.1 The finding (audit §3 is the authority; this section adds corroborating code evidence)

The stale claim — "P06 `key_V` gates Layer D's capability issuance" — appeared in
`CORE-ROADMAP-STANDARD-2026-07-17.md:176-179`, the INDEX §1 closing paragraph, and MEMORY
`sovereign-architecture-19-phase-roadmap-2026-07-17.md`. The audit's §3 withdraws it on the
governance axis: **P06 is a dev-time merge fence** (a signed `key_V` verdict over a git diff;
"canonical-repo DEV-TIME fence, not a runtime control" — BLUEPRINT-P06 §5:236-239), while
`RootDelegationPolicy` is a **runtime courier-onboarding rule**; they share substrate (the
`load_genesis` loader shape, the Ed25519⊕ML-DSA signing path, the open C4b hardening) but
neither is on the other's critical path. This blueprint adds the code-path corroboration,
verified this pass (§1): the mint path (`Delegation::sign` → `bebop2_core::sign::sign`,
roster.rs:167) and the admission path (`verify_chain` → per-link Ed25519 `verify_signature`,
roster.rs:275) contain **zero ML-DSA and zero `key_V`**; the PQ half of capability *identity*
(`subject_key_pq`, capability.rs:49-52) is bebop2's own ACVP-verified `pq_dsa.rs`, not P06's
dowiz-kernel verifier. What P06 *does* still touch inside Layer D is the **DecisionUnit
signed-import leg** (P29/P30 — SOVEREIGN §8.12 names it P06's 4th consumer), which is why the
Layer-D roll-up keeps its P06 row. The only genuinely shared item on the issuance path is
**C4b**: the still-open constant-time `mod_l` group-level fix (`core/src/sign.rs:661-692`) sits
on the same Ed25519 signing path every budgeted mint uses — a **shared-hardening dependency, not
a gate**, and a P-E/Phase-3 crypto item, not P06. Option A builds now.

### §7.2 Wording corrections — status ledger (DoD-7)

- **`CORE-ROADMAP-INDEX.md` §1 — DONE** (landed with the audit, verified this pass): the closing
  paragraph now removes Layer D from P06's gate list and carries the explicit withdrawal note
  ("Correction (P-D audit §3, 2026-07-17) … Layer D ships P06-independent").
- **`CORE-ROADMAP-STANDARD-2026-07-17.md:176-179` — STILL STALE, edit pending.** Replace
  "it gates P-C's independent-verification leg, P-D's capability issuance, and P-G's
  product-safety story" **with** "it gates Layer C's independent-verification leg, Layer D's
  *DecisionUnit signed-import leg* (P29/P30), and Layer G's product-safety story; Layer D's
  *capability issuance* is INDEPENDENT of P06 — see P-D audit §3 / BLUEPRINT-P-D §7; only C4b
  `mod_l` hardening is shared".
- **MEMORY `sovereign-architecture-19-phase-roadmap-2026-07-17.md` — STILL STALE, edit pending:**
  same substitution in the P06-blocks list, one line.

This *narrows* P06's blocker set honestly without shrinking P06's priority: it remains the
highest-leverage build (three other consumers unchanged).

---

## §8. Option B — hardware-attestation overlay

> ## ⛔ STOP — REQUIRES OPERATOR DECISION
> Nothing below this line may be built, scaffolded, or "prepared" in code before an explicit
> operator ruling. It modifies the trust topology (a third-party platform enters the issuance
> preconditions). Design is recorded so the ruling can be made once, with full information.

**Shape (audit §4 Option B, `P-D-audit:190-206`, precondition slot at `:118,195`; budget
integration per BLUEPRINT-P-E §4.2-§4.3):** the enrollment channel is `FirstContactQr` (physical
QR commissioning) and the attestation is an **additional pure precondition slot** in the same
predicate chain as A — verified once at sign-time, never a standing monitor, never a parallel
mechanism. Budget integration: split the cap — `max_per_epoch_unattested` (small baseline, e.g.
the default 1) vs `max_per_epoch_attested` (larger). Valid phone-side evidence (Android
StrongBox key-attestation / Apple App Attest; `AttestationEvidence` with `Software`
**unrepresentable** as attested — P-E §4.3) unlocks the larger cap for minting. **Degrade-closed
polarity (Hermetic P4):** attestation-service outage never revokes, blocks, or bricks an
existing capability and never blocks renewal — it only holds *new* minting to the unattested
baseline. Note Batch 7 §4 REJECTED *hub-side* TPM attestation on Firecracker physics; the audit
(§4-B) confirms phone-side StrongBox/App Attest is a different surface, not refuted by that
finding, but requires a **real-device probe before adoption** (heterogeneous handsets, no
guaranteed enclave). The hub never needs an enclave; X.509 chain verification runs
anchor/hub-side only; zero new deps in `bebop2-core`.

**Descartes square (per the standing DECART rule; consistent with the audit's §5 square — that
one additionally names the enclave-exclusion cost, folded in here):**

| | it is adopted (A+B) | it is not adopted (A alone) |
|---|---|---|
| **what happens** | Sybil mint cost rises from "N vetting ceremonies" to "N vetting ceremonies × N real attested devices" — the attack is priced in hardware; onboarding can scale past human-ceremony throughput | Trust topology stays fully sovereign (operator + math only), ships immediately, runs on any device including enclave-less owner hubs; Sybil cost remains anchor-vetting throughput up to `max_per_epoch` |
| **what does NOT happen** | Sovereignty does NOT survive intact: Google/Apple attestation roots enter the issuance preconditions (their outage/policy shifts touch mint *rate*, never existing authority — the split-budget polarity is the containment); a courier on a rooted/de-Googled/enclave-less handset does NOT get the attested cap (the Friedman-Resnick "entry fee excludes legitimate newcomers" cost — held to the unattested baseline, not excluded outright: that softening is exactly why B must stack on A) | Onboarding does NOT scale beyond ceremony throughput; the larger attested cap and its physical-device-cost floor never materialize — no hardware price on a fake identity |

---

## §9. Option C — `WebOfTrust`: DEFERRED, hazard pinned

### §9.1 Why deferred (theorem, not taste)

Cheng & Friedman ("Sybilproof Reputation Mechanisms," P2PECON 2005; primary source re-verified in
Batch 7 §2): **symmetric** reputation aggregation cannot be Sybil-proof; only **asymmetric,
path-rooted, flow-based** mechanisms escape. `WebOfTrust` is safe *only* while it remains
delegation-flow from anchors (exactly what `verify_chain` already computes). The hazard is
**drift**: the moment acceptance becomes a function of *how many* peers vouch — a symmetric
count — the theorem bites and the mesh is Sybil-open. Batch 7 §6: "the moment 'how many peers
vouch' becomes a symmetric count, Cheng–Friedman bites again. Keep it delegation-flow, not
vote-count." Deferral trigger (numeric, operator-tunable): revisit only when onboarding demand
exceeds `OperatorSigned`+`FirstContactQr` ceremony throughput (placeholder: > 50 new
couriers/week/anchor sustained — the operator sets the real number at revisit time).

### §9.2 The RED trap that catches naive drift (lands WITH Option A, stays forever)

```rust
// GREEN today; goes RED the day anyone lands vote-counting. Companion CI grep
// (ci-budgeted-issuance.sh) additionally refuses any fn signature taking a
// count-of-vouchers into an authority decision.
#[test]
fn red_vote_counting_never_authority() {
    let mut roster = AnchorRoster::new();
    let (anchor_seed, anchor_pk) = k(40u8);
    roster.enroll(&anchor_pk);
    let (_s, subject_pk) = k(41u8);
    let cap = Capability::new(subject_pk, Resource::Route, Action::Send, [9u8; 8], 9_999);

    // 64 distinct, correctly-SIGNED, mutually-consistent vouches from non-anchor
    // keys, all naming the subject. A symmetric aggregator would count these.
    // Authority must refuse EVERY one, identically, regardless of k.
    for i in 0..64u8 {
        let (vs, vpk) = k(128 + i);
        let vouch = Delegation::sign(
            vpk, subject_pk,
            Scope::single(Resource::Route, Action::Send),
            Effect::single(Resource::Route, Action::Send),
            9_999, [i; 8], &vs,
        ).unwrap();
        assert!(
            matches!(verify_chain(&roster, &[vouch], &cap, 0), Err(CapError::UnknownIssuer)),
            "vouch #{}: a COUNT of vouchers must never become authority", i
        );
    }

    // Control: ONE anchor-rooted path suffices — asymmetry, not signature scarcity,
    // is the mechanism (Cheng–Friedman's permitted branch).
    let real = Delegation::sign(
        anchor_pk, subject_pk,
        Scope::single(Resource::Route, Action::Send),
        Effect::single(Resource::Route, Action::Send),
        9_999, [200u8; 8], &anchor_seed,
    ).unwrap();
    assert!(verify_chain(&roster, &[real], &cap, 0).is_ok());
}
```

---

## §10. Compliance map (contract §2, all 20) · links (item 7) · Hermetic citations (item 20)

| # | Where | # | Where |
|---|---|---|---|
| 1 ground truth | §1 | 11 bulkhead | §5 |
| 2 DoD | §6.2 | 12 mesh | §5 |
| 3 spec/event TDD | §3.2→§4 order; sequence asserts §4.1 | 13 rollback/self-term math | §3.3 loss rule; §6.1 expiry/no-standing-grants |
| 4 types/consts first | §3.2 | 14 smart index | §6.3 gate + visibility ratchet §6.1 |
| 5 adversarial tests | §4.1 (RED), §9.2 (trap) | 15 living memory | §5 fold-vs-log |
| 6 hazard-by-structure | §6.1 reachability | 16 tensor/spectral | §5 — honest DOES-NOT-TRANSFER |
| 7 links | below | 17 regression | §6.2-5 |
| 8 scaling axis | §5 (federation trigger) | 18 agent instructions | §3.4 |
| 9 Linux-verdict reuse | §5 (verdict vocabulary) | 19 reuse-first | §3.4 (extend node_id.rs; Batch 7 §6 "no new consensus machinery") |
| 10 bench+telemetry | §6.2-3/-6 (zero-diff hot path; pole counters) | 20 Hermetic | below |

**Hermetic principles honored** (`hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`):
**P1 Mentalism** — this spec precedes and generates the code; **P2 Correspondence** — decide/fold
mirrors `order_machine`/`claim_machine`'s one law-shape, no second idiom; **P4 Polarity** — one
budget mechanism, seven *named* refusal poles, every collapse safe-directed (epoch-len-0 →
eternal epoch, record-loss → exhausted, attestation-outage → baseline cap); **P6 Cause-and-Effect**
— `can_issue`/`charge_issuance` are pure functions of passed-in ticks, no ambient clock, no
effect without a declared cause; **P7 Gender/Paired-Creation** — the mint (creation) is paired
with an independent check it cannot influence (`verify_chain` re-derives authority per admission;
the adversarial §4.1 test is the anti-self-certification artifact).

**Docs:** `P-D-audit-root-delegation-policy.md` (the Wave-1 grounding authority — ON DISK,
reconciled; its §3 owns the P06 correction, its §6 DoD items are absorbed into §6.2 here) ·
`CORE-ROADMAP-STANDARD-2026-07-17.md` (contract; §3 paragraph still carries the stale P06 edge —
§7.2 edit pending) · `CORE-ROADMAP-INDEX.md` §1/§3 (Layer D row → this file closes the "OPEN —
not written" entry) ·
`16-BATCH7-sybil-proof-capability-mechanism-findings.md` (theorem + mechanism authority) ·
`13-BATCH4-consensus-trust-findings.md` · `BLUEPRINT-P-E-network-crypto-core.md` §1/§4 (surviving
audit quotes; attestation types) · `sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md`
(what P06 actually is — §7 here) · `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §8.12
(R-2/R-3 dockets; Batch-7 verdict ratified) · `docs/regressions/REGRESSION-LEDGER.md`.
**Memory:** `sovereign-architecture-19-phase-roadmap-2026-07-17.md` (carries the §7 correction) ·
`crypto-safe-first-pass-2026-07-14.md` (C4b open status) ·
`internal-retrieval-living-memory-arc-2026-07-14.md` · `never-bypass-human-gates-2026-07-29`-class
rules (R-3 and §8 gates are operator's, not the agent's).
**Supersedes:** nothing. **Superseded by:** nothing — this is the Layer D blueprint the index's
"OPEN — not written" row awaited.
