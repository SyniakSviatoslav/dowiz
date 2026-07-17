# P-D Wave-1 Audit — Root Delegation Policy (RootDelegationPolicy issuance closure) (2026-07-17)

> "P-D" here = CORE-ROADMAP **Layer D** (consensus / trust / capability), an altitude lens — NOT
> execution phase P04. Naming ruling per the P-I audit §4; see `CORE-ROADMAP-INDEX.md`.
>
> **Wave 1, Opus, read-only.** Ground-truth audit for CORE-ROADMAP Layer **D**: what closing the
> `RootDelegationPolicy` operator decision actually requires, on today's substrate. Companion Wave-1
> audits in this dir: `P-G-audit-*` (in `BLUEPRINT-P-G` §0–§1), `P-H-audit-*` (in `BLUEPRINT-P-H`
> §0–§1), `P-I-audit-cross-repo-consolidation.md` (on disk). This is the fourth (P-D).
>
> **RECONSTRUCTION NOTE.** The original P-D audit was written and quoted (`BLUEPRINT-P-E` §1 cites
> `P-D-audit:131-135,107,217`) but **lost before commit** — its directory was untracked when a
> concurrent consolidation session merged 20 feat/* branches onto `main`; no git recovery was
> possible (`CORE-ROADMAP-INDEX.md:62`). This document reconstructs it and **re-verifies every
> citation fresh against the live tree this pass** rather than trusting the lost doc's line numbers.
> Result: **every citation still matches** (§0). The reconstruction is faithful, not blind.
>
> **Bottom line up front:** `RootDelegationPolicy` is a 4-variant enum, fail-closed on `Unspecified`,
> with **zero behavioral difference between its three real variants** and **no rate-limit / issuance-
> budget / attestation hook anywhere in `proto-cap/src/`** (grep, §2). Closing it is a **bounded
> policy predicate over the existing `AnchorRoster`/`verify_chain` substrate — no new consensus
> machinery, and it is P06-INDEPENDENT** (§3, correcting the roadmap's "P06 gates P-D" line). Three
> options; **recommend A (OperatorSigned + monotonic IssuanceBudget) as the default that ships today**,
> with **B (FirstContactQr + hardware-attestation) as an operator-gated phone-courier overlay** whose
> sovereignty cost is stated in full (§5). C (WebOfTrust) is deferred on a Cheng–Friedman hazard.

---

## Epistemics tags (inherited from Batch 7, so a reader can weigh each claim)

- `[VERIFIED-CODE]` — read from live source this pass (`file:line`).
- `[THEOREM]` — published impossibility/optimality result.
- `[PRIOR-ART-ADJUDICATED]` — already decided + reasoned in a sibling doc in this corpus.
- `[OPERATOR-DECISION]` — an open choice the code deliberately refuses to make for the operator.
- `[INFERENCE]` — my derivation from the tagged facts above.

---

## §0 — Verified current state (run fresh this pass, not trusted from the lost doc)

| Fact | Verified value | Method / cite |
|---|---|---|
| dowiz branch / HEAD | `main` / `caba2203c` | `git -C /root/dowiz rev-parse` |
| bebop2 crates live in | `/root/bebop-repo` @ `feat/verification-harness` (`f9fea30`) | `git -C /root/bebop-repo` |
| **`node_id.rs` identical on `main` ↔ `feat/verification-harness`** | **yes — 0-byte diff, clean tree** | `git diff main -- …/node_id.rs` → empty. The cites hold on *either* branch |
| `RootDelegationPolicy` location | `bebop2/proto-cap/src/node_id.rs:156-184` (NOT `core/src/node_id.rs`) | `[VERIFIED-CODE]` — read this pass |
| Lost doc's cite `node_id.rs:156-166` (enum) + `:179-184` (`require_explicit_policy`) | **STILL EXACT** | enum `157-166`, `require_explicit_policy` `179-184` |
| `claim_machine.rs:13-17` NO-COURIER-SCORING | **STILL EXACT** — "claim state carries no score / rating / trust / reputation / rank field" | `[VERIFIED-CODE]` |
| P06 blueprint path (roadmap "heavily restructured") | **STILL EXISTS** at `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md` | read this pass |
| Any issuance-budget / rate-limit / attestation hook in `proto-cap/src/` | **NONE** — grep returns only `UnknownIssuer` rejection paths, `issued_by` fields, and `cannot_mint` test names | `grep -rniE 'issue\|mint\|stake\|rate_limit\|budget\|bond\|attest\|epoch'` |

**Citation verdict:** the lost doc's summary was accurate; nothing shifted on the restructured
`main`. The 20-branch merge and the PQ-crypto→`pq`-feature extraction (`GROUND-TRUTH-2026-07-17.md`)
touched the dowiz **kernel**, not the bebop2 **proto-cap** capability layer, which is where every
P-D citation lives. `[INFERENCE]`

---

## §1 — VERDICT (read first)

**CLOSABLE ON TODAY'S SUBSTRATE, P06-INDEPENDENT, NO NEW CONSENSUS MACHINERY.** Batch 7 already
proved the *mechanism* (asymmetric anchor-rooted issuance) is Sybil-proof on the theorem-permitted
branch of Cheng–Friedman and Douceur (`16-BATCH7-…-findings.md` §2, §7) `[PRIOR-ART-ADJUDICATED]`.
This audit closes the one residual it left open: **which `RootDelegationPolicy`, and bounded how.**

The residual is **operational, not cryptographic**: Sybil-resistance reduces entirely to *the
discipline with which an anchor decides to sign a delegation to a new courier* (Batch 7 §6). The
code enumerates the choice and **fails closed until the operator makes it** — but it implements
**none** of the three as behavior. The gap is a *policy predicate at sign-time*, not a protocol. `[INFERENCE]`

---

## §2 — The code as it actually stands (fresh read, `node_id.rs:149-184`)

`[VERIFIED-CODE]` The module header (`node_id.rs:21-28`) states the design intent verbatim: *"The
actual root-delegation model — operator-signed vs Web-of-Trust vs first-contact-QR — is an OPERATOR
decision. This module implements all three as the `RootDelegationPolicy` enum and a `Default` of
`Unspecified`, but the code MUST NOT silently pick one as 'chosen'. … Do not 'helpfully' default to
a real policy."*

The enum itself (`node_id.rs:156-166`):

```rust
pub enum RootDelegationPolicy {
    OperatorSigned,   // :159 — operator-signed root cert(s): offline, audited, pinned
    WebOfTrust,       // :161 — anchors accepted transitively from a trusted seed set
    FirstContactQr,   // :163 — out-of-band key exchange (scanned at commissioning)
    Unspecified,      // :165 — FAIL-CLOSED default; bootstraps no root authority
}
```

**The load-bearing finding — three stub variants, zero behavioral difference.** `[VERIFIED-CODE]`
`[INFERENCE]` Nothing in `proto-cap/src/` branches on `OperatorSigned` vs `WebOfTrust` vs
`FirstContactQr`. The **only** code that inspects a policy value is `require_explicit_policy`
(`node_id.rs:179-184`), and it distinguishes exactly one thing — `Unspecified` (→
`Err(PolicyUnspecified)`) from everything-else (→ `Ok(other)`):

```rust
pub fn require_explicit_policy(p: RootDelegationPolicy) -> Result<RootDelegationPolicy, GenesisError> {
    match p {
        RootDelegationPolicy::Unspecified => Err(GenesisError::PolicyUnspecified),
        other => Ok(other),   // all three real variants are indistinguishable here
    }
}
```

So the three "real" variants are **names without mechanism**: choosing `OperatorSigned` over
`FirstContactQr` today changes *nothing* about what `verify_chain` accepts. The enrollment gate
(`load_genesis`, `node_id.rs:116-141`) reads a flat anchor file and enrolls every listed key
identically regardless of policy. The policy is a **documented seam, not an enforced rule** — which
is exactly why closing it is a *build* item, not merely a *config* item. `[INFERENCE]`

**No scarcity primitive exists.** `[VERIFIED-CODE]` The fresh grep confirms Batch 7 §1: no stake, no
proof-of-work, no rate-limit counter, no per-anchor issuance budget, no hardware-bound key, no
attestation check anywhere in `proto-cap/src/`. Issuance is compute-free; the only scarcity is
*structural* — a capability is authority only if `verify_chain` (`roster.rs:252-316`) finds an
anchor-rooted, signed, narrow-only delegation path to the subject (Batch 7 §1). An attestation
precondition, were one added, would slot in as an **additional pure precondition** at sign-time —
the same shape as the budget predicate below, evaluated before an anchor emits a `Delegation::sign`.

---

## §3 — P06 independence (the correction the roadmap needs)

The canonical index asserts **"P06 `key_V` gates … Layer D's capability issuance"**
(`CORE-ROADMAP-INDEX.md:39-41`, and Layer D "rolls up … P06" at `:32`; echoed in
`CORE-ROADMAP-STANDARD-2026-07-17.md:176-179`). **This audit finds that dependency is
mis-stated. RootDelegationPolicy and P06 are INDEPENDENT.** `[VERIFIED-CODE]` `[INFERENCE]`

| | **P06 (`key_V`)** | **RootDelegationPolicy (P-D)** |
|---|---|---|
| What it governs | a **dev-time** merge fence: a signed key_V verdict over a **git diff / commit** | a **runtime** courier-onboarding rule: which real-world identity earns an anchor-rooted delegation |
| Scope stamp | "canonical-repo **DEV-TIME fence**, not a runtime control … a sovereign hub MAY fork and drop it" (`BLUEPRINT-P06 §5`, `:236-239`) | production capability-issuance path executed on every hub, every enrollment |
| What it blocks | a code merge from landing un-verified | a courier from receiving usable authority |
| Verdict object | `Verdict TLV` signed by `key_V` over `diff_attest_sha3` (`P06 §4`) | `Delegation::sign` by an enrolled anchor over a subject key (`roster.rs:143-181`) |

**They share substrate, not function.** `[VERIFIED-CODE]` Both **reuse** the MESH-12 `load_genesis`
pattern — P06 §2 says its K/V anchor file follows "the MESH-12 `load_genesis` pattern
(`node_id.rs:1-19`) verbatim in shape"; RootDelegationPolicy *is* that loader's sibling in the same
file. Both ride the same Ed25519⊕ML-DSA signing path, so both inherit the **same open C4b
hardening** (`sign.rs mod_l` variable-time nonce; P06 §1 lists it HIGH,
`DECART-P06-bebop2-crypto-dep.md`). But sharing a loader and a signature primitive is **not** a
functional block: `RootDelegationPolicy` closure needs *no* key_V verdict, and P06's merge gate
needs *no* courier-onboarding policy. Neither is on the other's critical path. `[INFERENCE]`

**Consequence for the roadmap:** the "P06 gates P-D" edge should be **removed**. What genuinely
gates *both* (independently) is **C4b** — the shared signing-path side-channel — and that is a P-E /
Phase-3 crypto item, not P06. P-D can ship its policy predicate on today's substrate; if the
operator wants the hybrid-PQ leg on the onboarding signature hardened first, that is the C4b gate,
reached without P06. `[INFERENCE]` This is the single correction this audit makes to the standing plan.

---

## §4 — Three concrete options (each: mechanism · Sybil cost · substrate · constraints honored)

All three honor the three hard constraints Batch 7 §4 fixed: **(a)** no courier-scoring/reputation,
**(b)** structurally enforceable (a predicate, not a watchdog loop), **(c)** deployable on an
unprivileged microVM. Attestation is named as an additional precondition at `:118` (§2 end) and
re-examined in Option B (`:194`).

### Option A — `OperatorSigned` + per-anchor monotonic issuance budget (RECOMMENDED)

**Mechanism.** Bind `RootDelegationPolicy::OperatorSigned` to a **pure predicate checked at
delegation-sign time**: an anchor may emit a delegation only if it has monotonic issuance budget
left in the current epoch. Define the type before implementation (contract item 4):

```rust
/// Per-anchor monotonic issuance budget. Pure predicate at Delegation::sign time.
/// No monitor, no score — a bounded counter the anchor checks against itself.
pub struct IssuanceBudget {
    pub anchor_id:   [u8; 32],  // which enrolled anchor this budget governs
    pub epoch:       u64,       // monotonic; resets budget, never decreases
    pub minted_count: u32,      // delegations already signed this epoch
    pub max_per_epoch: u32,     // operator-set ceiling; sign() refuses at the cap
}
```

`sign_delegation` becomes: `require_explicit_policy(p)? ; assert minted_count < max_per_epoch ;
minted_count += 1`. It is a **pure function of the anchor's own monotonic state** — no observer, no
ranking of couriers, degrade-closed at the cap (Hermetic "self-termination as a hard invariant
boundary," not a supervisor). `[INFERENCE]`

- **Sybil cost.** N Sybil keys ⇒ N delegations ⇒ N draws against a genesis-frozen anchor's bounded
  per-epoch budget. The attacker cannot exceed `max_per_epoch` without the operator raising the
  ceiling out-of-band. Identity stays free; **authorization is bounded and anchor-gated** (Batch 7 §1). `[INFERENCE]`
- **Substrate.** Ships on **today's** `AnchorRoster`/`verify_chain` — pure-Rust, zero hardware, zero
  new dep, no new consensus machinery. `[VERIFIED-CODE]`
- **P06 dependency.** NONE (§3). Fully sovereign — the operator is the only root, no external party.

### Option B — `FirstContactQr` + hardware-attestation precondition (operator-gated overlay)

**Mechanism.** A degrade-closed overlay **on top of A**: before a `FirstContactQr` enrollment mints
a delegation, require a device attestation (Android StrongBox key-attestation / Apple App Attest)
proving the requesting key lives in a genuine secure element on a distinct physical device. The
attestation is an **additional pure precondition** at sign-time (the additional-precondition slot
named at `:118`) — verified
once, folded into the same predicate chain as A, never a standing monitor. `[INFERENCE]`

- **Sybil cost.** Each Sybil now needs a distinct real device with a genuine enclave — the attack is
  priced in **hardware**, not just operator vetting. Strong for a phone-courier fleet. `[INFERENCE]`
- **Substrate.** Phone-only (couriers carry phones; owner hubs do not attest). Pulls **Google/Apple
  attestation roots into the trust base** — a sovereignty cost stated in full in §5. Needs a
  **real-device probe** before adoption (heterogeneous handsets; no guaranteed enclave — Batch 7 §4
  rejected *TPM-passthrough* attestation on Firecracker physics, but **phone-side** StrongBox/App
  Attest is a different surface and is *not* refuted by that finding). `[INFERENCE]` `[PRIOR-ART-ADJUDICATED]`
- **P06 dependency.** NONE. Stacks on A; does not replace it.

### Option C — `WebOfTrust` (DEFERRED)

Transitive delegation from a trusted seed set. **Scales** best but carries the sharpest hazard: it
must stay **asymmetric, path-rooted** (delegation *flow* from anchors). The moment "how many peers
vouch" becomes a **symmetric count**, it re-enters the class Cheng–Friedman proves has *no*
Sybil-proof function (`[THEOREM]`, Batch 7 §2, §6). Naive implementation ⇒ vote-counting ⇒ theorem
violation. **Deferred** until A/B are live and a formal flow-based construction is specified. `[INFERENCE]`

---

## §5 — Recommendation + Descartes-square consequences (fully spelled out)

**Recommendation: default to A. Stack B as an operator-gated phone-courier hardening overlay. Defer C.**

A is the sovereign floor that ships on today's substrate with no new trust root; B prices Sybil
attacks in real hardware where the fleet is phones, at the cost of importing Google/Apple as
attestation roots. The choice between "A alone" and "A+B" is the operator's — so it is laid out as a
full Descartes square (all four quadrants, not just the upside), over the decision **"adopt B on top
of A."**

|  | **What WILL happen** | **What will NOT happen** |
|---|---|---|
| **If we ADOPT B (A+B)** | Sybil attacks priced in hardware — each fake courier needs a distinct genuine secure element; the phone fleet gets device-bound identity; A's operator-vetting ceiling is backstopped by physics. | Sovereignty is no longer self-contained: enrollment now *depends on* Google/Apple attestation roots being reachable and honest; a courier on a rooted/de-Googled/enclave-less handset **cannot** enroll (degrade-closed excludes legitimate low-end devices — the Friedman–Resnick "entry fee excludes newcomers" cost, `[THEOREM]`). |
| **If we DON'T adopt B (A alone)** | The system stays fully sovereign — the operator is the only root, ships immediately, runs on any device including owner hubs with no enclave; zero external dependency. | Sybil cost stays at "operator vetting effort per delegation" (`max_per_epoch`) — **not** priced in hardware; a determined attacker who can pass the operator's out-of-band vetting N times still mints N anchored capabilities up to the budget ceiling. No physical-device rate-limit floor. |

**Reading the square.** The immediate consequence favors **A alone**: it ships today, keeps
sovereignty self-contained, and closes the enum with no new trust root. The long-term consequence is
where **B** earns its place: once the courier base is predominantly phones and Sybil pressure is
real, B adds a **hardware price** to each fake identity that A's operator-vetting alone cannot —
*but* it does so by tying part of the sovereignty story to Google/Apple's attestation infrastructure,
the one thing this architecture otherwise refuses. Hence B is **operator-gated and additive**, never
the default: the operator adopts it consciously, with the enclave-exclusion cost accepted, only when
the phone-fleet Sybil threat is worth that dependency. C stays deferred until its flow-construction
is proven not to collapse into a symmetric vote-count. `[INFERENCE]`

---

## §6 — DoD for the eventual P-D build (falsifiable — contract item 2)

The audit is grounding; the build (Wave 2 `BLUEPRINT-P-D`, still `OPEN`, `CORE-ROADMAP-INDEX.md:50`)
is DONE when:

1. **RED→GREEN — budget cap enforced.** An anchor at `minted_count == max_per_epoch` that attempts a
   further `Delegation::sign` is refused (new `CapError`/`GenesisError` variant); a test that mints
   `max_per_epoch+1` goes RED before the predicate lands, GREEN after. `[VERIFIED-CODE target]`
2. **RED — policy still fail-closed.** `require_explicit_policy(Unspecified)` remains
   `Err(PolicyUnspecified)`; adding a budget must not silently pick a policy (guards `node_id.rs:179-184`).
3. **Monotonicity.** `epoch` never decreases and a replayed lower-epoch budget is rejected (adversarial
   test — contract item 5).
4. **No-scoring invariant intact.** `claim_machine.rs:13-17` unchanged; `ci-no-courier-scoring.sh`
   green — the budget is a counter on the *anchor*, never a rank on the *courier*.
5. **P06-independence witnessed.** The budget predicate compiles, tests, and runs with **no** `key_V`
   / verifier dependency in its build graph (proves §3).

---

## §7 — Citation index (verified this pass, `file:line`)

- `bebop2/proto-cap/src/node_id.rs:21-28,116-141,156-166,168-174,179-184` — module intent ("do not
  helpfully default"); fail-closed `load_genesis`; `RootDelegationPolicy` 4-variant enum;
  `Default=Unspecified`; `require_explicit_policy` (only code that reads a policy value, distinguishes
  only `Unspecified`). **CLEAN, identical on `main`↔`feat/verification-harness`.** `[VERIFIED-CODE]`
- `bebop2/proto-cap/src/roster.rs:143-181,252-316` — `Delegation::sign`; `verify_chain` anchor-rooted
  asymmetric gate (the substrate a budget predicate slots into). `[VERIFIED-CODE]`
- `bebop2/proto-cap/src/claim_machine.rs:13-17` — NO-COURIER-SCORING; no score/rating/trust/rank field.
  `[VERIFIED-CODE]`
- `grep -rniE 'issue|mint|stake|proof_of_work|rate_limit|budget|bond|attest|epoch' proto-cap/src/` →
  only `UnknownIssuer` / `issued_by` / `cannot_mint` — **no issuance-budget / rate-limit / attestation
  hook**. `[VERIFIED-CODE]`
- `dowiz/docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md
  §2(:87-89),§4,§5(:236-239)` — **STILL EXISTS** at path; K/V dev-time merge fence; "canonical-repo
  DEV-TIME fence, not a runtime control"; reuses `load_genesis` shape. `[VERIFIED-CODE]`
- `dowiz/docs/design/sovereign-roadmap-2026-07-16/DECART-P06-bebop2-crypto-dep.md` — C4b the shared
  signing-path side-channel; `key_V` still `signed:false`. `[VERIFIED-CODE]`
- `dowiz/docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/16-BATCH7-sybil-proof-capability-mechanism-findings.md
  §1,§2,§4,§6,§7` — asymmetric anchor-rooted issuance is Sybil-proof; four-candidate evaluation; the
  residual policy caveat this audit closes. `[PRIOR-ART-ADJUDICATED]`
- `dowiz/docs/design/CORE-ROADMAP-INDEX.md:32,39-41,50,62` — Layer-D roll-up; the "P06 gates Layer D"
  edge this audit corrects; P-D blueprint `OPEN`; this file logged `MISSING ON DISK`. `[VERIFIED-CODE]`
- `dowiz/docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-E-network-crypto-core.md §1` — the surviving
  quote of the lost doc (`IssuanceBudget` seam, attestation precondition) this reconstruction
  restores — now at `:170-175` (budget struct) and `:118,195` (attestation); `BLUEPRINT-P-E` §1's
  stale `:131-135,107,217` cite into the lost version is updated to match. `[VERIFIED-CODE]`
- Cheng & Friedman, "Sybilproof Reputation Mechanisms," P2PECON 2005; Douceur, "The Sybil Attack,"
  IPTPS 2002 — no *symmetric* Sybil-proof function; asymmetric path-rooted flow *is* Sybil-proof;
  defense = costly/authorized issuance, not peer reputation. `[THEOREM]` (via Batch 7 §2).

---

*P-D Wave-1 audit reconstructed 2026-07-17. Read-only, no code written. Corrects one standing edge
("P06 gates P-D" → independent; C4b is the real shared gate). Recommendation: A default (ships today,
sovereign, P06-independent), B operator-gated phone overlay (hardware-priced Sybil cost vs Google/
Apple attestation-root dependency), C deferred (Cheng–Friedman symmetric-count hazard). Unblocks the
`CORE-ROADMAP-INDEX.md:62` dead link and the Wave-2 `BLUEPRINT-P-D` build.*
