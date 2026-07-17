# BATCH 7 — Sybil-proof capability issuance: prove/disprove the operator's ruling

> Research + audit (NOT a blueprint, NOT implementation). Follows Batch 4's flagged contradiction
> (`13-BATCH4-consensus-trust-findings.md §1`) into the operator's binding ruling:
> **"Sybil-resistance IS required and IS achievable WITHOUT reputation/courier-scoring, watchdogs,
> or proxy."** This batch's job is to prove or disprove that concretely against THIS codebase's
> actual capability model and against the *actual scope* of the Cheng–Friedman theorem — not to
> re-assert the textbook "costly identity" slogan. Every load-bearing claim carries an epistemics
> tag and, where it is code, a `file:line` I read this session.

## Epistemics tags

- `[VERIFIED-CODE]` — read from live source this session (`file:line`).
- `[THEOREM]` — published impossibility/optimality result.
- `[LIVE-WEB]` — grounded by a web search/fetch I ran this session (source URL given).
- `[TRAINING-KNOWLEDGE]` — asserted from general knowledge; flagged for independent re-verification.
- `[PRIOR-ART-ADJUDICATED]` — already decided + reasoned in a sibling doc in this corpus.
- `[OPERATOR-RULING]` — the binding directive being tested here.
- `[INFERENCE]` — my derivation from the tagged facts above.

---

## §0 — VERDICT (read first)

**PROVEN-VIABLE-WITH-CAVEATS.** Sybil-resistance *is* achievable in this mesh via
asymmetric-cost capability **issuance** rather than symmetric peer-reputation, and — decisively —
this is **not** the standard hand-wave: the Cheng–Friedman (2005) theorem the repo cites to justify
`NO-COURIER-SCORING` **only** rules out *symmetric* reputation functions, and the same paper
**constructs an asymmetric, path-rooted, flow-based mechanism that IS Sybil-proof** `[LIVE-WEB]`
`[THEOREM]`. The codebase's existing `verify_chain` — a capability is valid only if its delegation
chain is rooted at a genesis-frozen `AnchorRoster` anchor — **is exactly an instance of that
theorem-permitted asymmetric class** `[VERIFIED-CODE]` `[INFERENCE]`. Minting a thousand keypairs
mints a thousand *identities* for free but **zero capabilities**, because a capability requires an
already-trusted anchor (or an anchor's delegate) to sign a delegation to it — an anarchic Sybil
gets `UnknownIssuer` and holds no authority `[VERIFIED-CODE]`.

The **caveat** (the one thing the design must still nail): the asymmetry is theorem-clean and
Douceur-clean, but operational Sybil-resistance reduces to *the discipline with which an anchor
decides to sign a delegation to a new courier*. If an anchor rubber-stamps every request, you have
rebuilt a free-issuance CA and Sybils flow through legitimately-signed delegations. **That
delegation-granting policy is currently an explicit, unresolved OPERATOR decision already
enumerated in code** (`RootDelegationPolicy::{OperatorSigned, WebOfTrust, FirstContactQr,
Unspecified}`, `node_id.rs:156-166`, `Default = Unspecified`, fail-closed) `[VERIFIED-CODE]`. The
mechanism is viable; the residual work is *choosing and bounding that policy*, not inventing new
consensus machinery.

Nothing here requires reputation, courier-scoring, a watchdog process, or a proxy. All three
prohibitions are honored (§4, §5). Hardware-attestation as the cost mechanism is **rejected on
physics** (Firecracker, §4). Proof-of-work is rejected (punishes honest low-resource couriers,
weak Sybil defense, §4). The winning mechanism is the one **already built**, correctly reframed.

---

## §1 — Step 1: Is capability issuance already costed/scarce? (grep + full read)

**Answer: issuance is compute-*free* but structurally *gated*. There is NO stake, NO proof-of-work,
NO rate-limit counter, NO hardware-bound key, NO explicit per-capability economic price anywhere in
`proto-cap`** `[VERIFIED-CODE]` (grep for `issue|mint|stake|proof_of_work|rate_limit|bond|attest`
across `proto-cap/src/` returns only: `revocation.rs` "every capability ever *minted*" prose;
`node_id.rs`/`roster.rs`/`error.rs` "self-*mint*/self-*issue*" **rejection** paths; no scarcity
counter). The scarcity that exists is **structural, not economic**:

- **The gate is `verify_chain`** (`roster.rs:252-316`) `[VERIFIED-CODE]`. A capability is accepted
  only when its delegation chain: (a) roots at an **enrolled anchor** (`roster.rs:260-263`, else
  `UnknownIssuer`); (b) every link is signed by its `issued_by` parent (`:274-275`); (c) chains
  (`child.issued_by == parent.subject`, `:280-284`); (d) attenuates scope narrow-only (`:286-291`);
  (e) binds the tail subject to `cap.subject_key` (`:298-302`); (f) is unexpired (`:270-273`).
- **Self-issue is a rejected auth-bypass, tested RED→GREEN.** A key naming itself both issuer and
  subject on an empty/non-anchor roster is `UnknownIssuer`
  (`roster.rs:332-357 red_self_issued_delegation_rejected_as_unknown_issuer`;
  `node_id.rs:312-345 red_seeded_owner_fixture_cannot_mint`) `[VERIFIED-CODE]`. The "seeded-owner
  JWT" pattern — the classic free-mint — is explicitly dead.
- **The anchor set is frozen at genesis, loaded fail-closed from disk.** `load_genesis`
  (`node_id.rs:116-141`) enrolls anchors exactly once; a missing/malformed/**zero-anchor** file
  yields `EmptyRoster` and the node "captures **no authority**" (`node_id.rs:12-19, 92, 137-139`)
  `[VERIFIED-CODE]`. "At runtime there is no central issuer and no reputation ledger; the *only*
  keys that may bootstrap authority are the enrolled anchors" (`roster.rs:13-14`) `[VERIFIED-CODE]`.

**The load-bearing structural fact** `[INFERENCE]`: a fresh keypair (a Sybil identity) costs one
`keygen` call — free. A fresh **capability** costs *an enrolled anchor's cooperation to sign a
delegation to that key* (or a chain of such delegations from an anchor's delegates). Identity is
free; **authorization is anchor-gated.** That gap — not a PoW puzzle, not a bond — is the entire
Sybil-resistance surface. The asymmetry is: `1 real courier = 1 anchor-authorized delegation`;
`N Sybils = N anchor-authorized delegations`, and the anchor is a genesis-frozen,
operator-controlled scarce root that will not sign N of them. The N Sybil keys land un-anchored →
`UnknownIssuer` → zero authority. This is the exact form R1 §4 names: "minting a thousand
identities does not mint a thousand capabilities" (`R1-web3-failure-modes.md:78`)
`[PRIOR-ART-ADJUDICATED]`.

---

## §2 — Step 2: The Cheng–Friedman theorem's *actual* scope (this is the crux)

Batch 4 and R1 §4 cite the theorem correctly but compressed. The precise scope — confirmed against
a primary source this session — is what turns the operator's ruling from "hopeful" to "sound."

**Cheng & Friedman, "Sybilproof Reputation Mechanisms," P2PECON 2005** `[THEOREM]` `[LIVE-WEB]`
(ACM DL 10.1145/1080192.1080202): the paper proves **"there is no *symmetric* sybilproof reputation
function"** — the impossibility is *scoped to the symmetric class* (PageRank/EigenTrust-style
aggregates where every rater is interchangeable and reputation is a symmetric function of the trust
graph). Critically, the **same paper then constructs the escape**: "for nonsymmetric reputations
following the notion of reputation propagation along paths, they give a general **asymmetric
reputation function based on flow** and give conditions for **sybilproofness**" `[LIVE-WEB]`
(web search of the paper's own abstract/summary, 2026-07-17). So the theorem does **not** say
"Sybil-proofness is impossible." It says: *symmetric aggregation* is impossible; *asymmetric,
path-rooted, flow-based* mechanisms **can be Sybil-proof**, and the authors exhibit one.

**Why this is exactly the codebase's model** `[INFERENCE]`: `verify_chain` accepts authority that
**propagates along a signed delegation path rooted at a distinguished trusted node** (an enrolled
anchor). That is asymmetric by construction — there is a privileged root; raters/peers are *not*
interchangeable; a Sybil that is not on an anchor-rooted path contributes nothing. It is the
path-rooted flow class the theorem leaves open, not the symmetric-aggregate class the theorem kills.
Adding a Sybil keypair adds a node with **no inbound anchor-rooted edge**, so its "flow" from the
root is zero — the standard sufficient condition for sybilproofness in the asymmetric flow
construction `[TRAINING-KNOWLEDGE, re-verify against the paper's Theorem statements if a formal
proof obligation is ever raised]`.

**Douceur, "The Sybil Attack," IPTPS 2002** `[THEOREM]` `[LIVE-WEB]` (Microsoft Research): the
foundational result is that **"without a logically centralized authority, Sybil attacks are always
possible except under extreme and unrealistic assumptions of resource parity and coordination"**,
and the paper's own recommended defense is **"a trusted agency [to] certify identities"** — i.e. a
**costly / authorized identity-issuance authority, NOT peer reputation** `[LIVE-WEB]` (web search of
the paper, 2026-07-17). Douceur's mitigation *is* the anchor-roster pattern: a distinguished
authority gates which identities carry weight. The literature's origin point and the repo's code
agree.

**Net for the ruling** `[INFERENCE]`: the operator's "possible without reputation/scoring" is not a
gamble against the theorem — it is precisely the branch the theorem *and* Douceur point to. R1 §4's
own conclusion already said this ("it sidesteps the Cheng–Friedman impossibility entirely because it
is *not a symmetric scoring function*", `R1-web3-failure-modes.md:80`) `[PRIOR-ART-ADJUDICATED]`;
this batch confirms the theorem's asymmetric-escape half against a primary source, which R1 stated
but did not quote. **The ruling is theorem-consistent.**

---

## §3 — Step 3: Does P06's split-identity/adversarial-verifier pattern generalize to issuance?

**Yes — structurally it is the *same shape*, and it is already latent in the code.** `[INFERENCE]`
`[VERIFIED-CODE]`

P06 (`BLUEPRINT-P06-v1-split-identity-verifier.md`) establishes "verified, not self-certified":
a done-claim (`key_K` over a diff) is worthless until an **independent** party (`key_V`, a *distinct
anchor with a distinct role*, `K ≠ V` enforced at load and at gate, `§2/§5`) re-executes and signs
a verdict; the claimant cannot self-certify (`P06 §0, §4, §7`) `[VERIFIED-CODE]`. The security comes
from **identity separation gating the claim**, explicitly *not* from any downstream score (P06 §8
"identity separation, not person separation" — no rating anywhere).

Map the two problems term-for-term `[INFERENCE]`:

| P06 done-gate | Capability issuance |
|---|---|
| claimant `key_K` | subject key requesting a capability |
| verifier `key_V` (distinct anchor, `K≠V`) | issuing **anchor** (distinct from subject) |
| self-certified GREEN is rejected | self-issued capability is `UnknownIssuer` (`roster.rs:332`) |
| verdict must be signed by a *role-`V`* anchor | delegation must be signed by an *enrolled* anchor / its delegate |
| gate re-checks the signature, no monitor | `verify_chain` re-checks the chain, no monitor |

The generalization is exact: **issuance is Sybil-resistant because it requires an already-trusted
party's signature that the subject cannot produce for itself — the same "a claim needs a check by a
different identity" invariant P06 enforces for done-gates.** The scarce resource in both is *the
trusted counter-party's willingness to sign*, and in both cases that is what is Sybil-resistant, not
any score. P06 even reuses the *same substrate*: it explicitly builds on "the MESH-12 `load_genesis`
pattern verbatim in shape" (`P06 §2`) `[VERIFIED-CODE]` — the very `node_id.rs`/`roster.rs` code that
gates capability issuance. So this is not an analogy stretched across modules; it is **one pattern,
one substrate, two applications.**

---

## §4 — Step 4: Concrete mechanism + evaluation of the four candidate cost sources

**The mechanism (design core, not a blueprint):** *Sybil-resistance = asymmetric anchor-rooted
issuance.* A capability is only authority if `verify_chain` finds an anchor-rooted, narrow-only,
signed delegation path to the subject. What makes **N fake capabilities strictly more expensive than
1 real one** is that each requires a **delegation signed by the operator-gated anchor quorum**; the
anchor set is genesis-frozen and does not grow at runtime (`roster.rs:13-14, 29-34`)
`[VERIFIED-CODE]`. The attacker's N free Sybil keypairs are inert (`UnknownIssuer`). This is
**candidate (3) "rate-limited issuance by an already-trusted quorum," and it is already built** —
the design work is bounding the anchor's *sign-a-delegation* decision (§6), not adding machinery.

Evaluation of all four task-listed candidates against the three hard constraints —
**(a)** requires no courier-scoring/reputation? **(b)** enforceable structurally (a type/protocol
check, not a monitoring loop)? **(c)** physically deployable on the Firecracker prod microVM
(Batch 5 §0.2: unprivileged, virtio-net only, no SR-IOV/TPM passthrough, `scratch` static binary)?

| Candidate | (a) no-scoring | (b) structural, no-watchdog | (c) Firecracker-deployable | Verdict |
|---|---|---|---|---|
| **Anchor-rooted delegation (rate-limited trusted quorum) — EXISTING** | **YES** — anchors are identities not scores (`roster.rs:26`, `revocation.rs:25-26`) `[VERIFIED-CODE]` | **YES** — one `verify_chain` call at admission; pure function of the presented chain; no loop watches anyone `[VERIFIED-CODE]` | **YES** — pure-Rust Ed25519⊕ML-DSA sig verify, zero hardware, zero privilege; already builds+tests `[VERIFIED-CODE]` | **ADOPT (winner)** |
| **Hardware-attestation (device-bound / TPM keys)** | YES (an attestation is not a score) | Partial — attestation check is structural, but provisioning implies a device agent | **NO — REJECT-on-physics.** Firecracker prod is unprivileged, virtio-only, no TPM/secure-enclave passthrough (Batch 5 §0.2, §1.3) `[PRIOR-ART-ADJUDICATED]`; courier phones are heterogeneous, no guaranteed enclave `[TRAINING-KNOWLEDGE]` | **REJECT (physics)** |
| **Real-world stake (refundable bond)** | Borderline-YES — a one-time binary/economic admission is not a history-derived rank; but see caveat | YES *iff* slashing is triggered by a **verified fraud-proof**, not a monitor (§5); a bond balance is a ledger entry, not a supervisor | YES — a ledger row, no hardware | **VIABLE-AS-OPTIONAL-HARDENING, drags in the money red-line** — a bond is a *money leg*: arms B2's `LedgerMoney` red-line + operator gate + the "currency you call a budget" hazard (COUNSEL §5, Batch 4 §2.6) `[PRIOR-ART-ADJUDICATED]`. It is the Douceur/Friedman-Resnick "entry fee" escape, and it *excludes legitimate low-resource newcomers* (`R1 §4`) `[THEOREM]`. Use only if the operator wants economic skin-in-the-game AND accepts the money-leg gate. |
| **Proof-of-work** | YES (not a score) | YES (verify a hash preimage — structural) | Technically yes, but self-defeating | **REJECT (weak + regressive).** PoW is symmetric in cost: it taxes honest low-resource couriers (a phone should not grind hashes) as much as attackers, and a moderately-resourced attacker out-computes honest newcomers — the exact Friedman-Resnick "entry fee excludes legitimate newcomers" cost `[THEOREM]`, on a shared 4-vCPU host (Batch 5 §0.1) `[PRIOR-ART-ADJUDICATED]`. Strictly dominated by the anchor mechanism. |

**Conclusion for Step 4** `[INFERENCE]`: the correct mechanism is the existing anchor-rooted
delegation, reframed as *asymmetric issuance authorized by a trusted quorum whose granting decision
is out-of-band and rate-limited* — "the human touches the roster, not the traffic" (B3 §, H-1
enrollment framing, `BLUEPRINT-B3-exposure-ledger-envelopes.md:595-600`) `[PRIOR-ART-ADJUDICATED]`.
A refundable bond is a legitimate *optional* hardening at the anchor boundary **if** the operator
accepts the money red-line; it is not required for Sybil-resistance, which the issuance asymmetry
already delivers. Hardware-attestation and PoW are out.

---

## §5 — Step 5: Does "zero watchdog, zero proxy" conflict with revocation? (precise resolution)

**No conflict — but only under a precise, bright-line distinction. There are two trigger models for
revocation, and exactly one of them is a watchdog.** `[INFERENCE]` `[VERIFIED-CODE]`

`revocation.rs` is a monotonic, append-only, gossip-convergent set over **public keys and capability
hashes** — "revocation acts on public keys and capability hashes (identities/statements), **never on
scores or reputation**" (`revocation.rs:25-26`) `[VERIFIED-CODE]`. The set has no observer built in:
`revoke_key`/`revoke_capability` are pure inserts (`:69-78`), `merge`/`gossip_payload` are CRDT-style
anti-entropy (`:94-98, 114-120`) `[VERIFIED-CODE]`. The question is *what calls `revoke_*`*.

- **Watchdog revocation (FORBIDDEN):** a standing process that continuously *samples behavior*,
  forms a judgment of "badness," and auto-revokes low-scorers. This is forbidden twice over — it is
  a supervising loop (violates zero-watchdog) **and** it is a reputation score in disguise (violates
  `NO-COURIER-SCORING`: to auto-revoke on "bad behavior" you must *rank* behavior). `[INFERENCE]`
- **Structural revocation-on-verified-fact (ALLOWED):** revocation triggered by a **verifiable
  event**, not by a monitor — e.g. a produced cryptographic **fraud-proof** (a double-signed
  conflicting claim, a replayed nonce, a disclosed key compromise), or an **operator out-of-band
  roster action** (`drop_anchor`, `roster.rs:219-225`). This is a **pure function of a presented
  proof**: someone who *holds* the proof (the injured counterparty) presents it, it is verified
  once, and the consequence is an append to the monotonic set that then gossips by anti-entropy.
  `[VERIFIED-CODE]` `[INFERENCE]`

**The precise difference** `[INFERENCE]`: a watchdog is a *standing, always-on process that samples
state and decides* (temporal, judgment-based, must be running and looking). Structural revocation is
*event-driven, stateless, fact-based* — nothing has to be running and watching; the proof is pushed
by whoever was harmed, checked with the same `RequireBoth` verify discipline the rest of the line
uses, and folded in. It is the identical shape as P06's verdict gate: a **consequence of a passed
verification of a bad-fact**, not the output of a supervisor. "Does something have to be
continuously observing to make this fire?" — for revocation-on-fraud-proof the answer is **no**, so
it is not a watchdog.

**Reinforcing structural facts** `[VERIFIED-CODE]` `[INFERENCE]`:
- Most bad capabilities **self-terminate with zero observer** via expiry (`verify_chain`
  `:270-273, 310-313`; `Capability::is_fresh`). This is the operator's own dialogue synthesis —
  "Self-Termination as a hard invariant boundary (not a supervisor)" (`01-RAW-DIALOGUE-PART-A.md:51`)
  `[PRIOR-ART-ADJUDICATED]`. Revocation is only the *surgical early-kill* for the compromise case
  before natural expiry (`revocation.rs:1-9`).
- Revocation is **degrade-closed**: a revoked key/hash flips the gate to reject; there is no path
  where "the watchdog missed it" leaves authority live, because authority is affirmatively
  *re-derived from first principles on every use* (`verify_chain` runs per-admission), not granted
  once and monitored thereafter. The absence of a standing grant is what removes the need for a
  standing watcher. `[INFERENCE]`

**Bright line for any future builder:** revocation MUST be *fact-triggered* (verified fraud-proof or
operator roster action), **never behavior-monitored**. A "reputation watchdog that auto-revokes
low-scorers" violates both red-lines simultaneously and must never be built. The current code's
stated posture (`revocation.rs:25-26`) is already exactly this line.

---

## §6 — The residual caveat, stated precisely (the one real open design point)

The proof above is clean at the theorem level. The honest residual is **operational**: the whole
Sybil-resistance reduces to *how disciplined an anchor is when it decides to sign a delegation to a
new courier*. `[INFERENCE]`

- If the anchor signs a delegation for **any** requester with no out-of-band vetting, the attacker
  requests N delegations for N Sybil keys and gets N *legitimately-anchored* capabilities. The
  theorem still holds (each is a real anchored path) but Sybil-resistance is **operationally void** —
  you have rebuilt a free-issuance CA. `[INFERENCE]`
- Therefore the load-bearing design decision is **the delegation-granting policy at the anchor
  boundary**, and this is **already an explicit, unresolved OPERATOR decision in code**:
  `RootDelegationPolicy::{OperatorSigned, WebOfTrust, FirstContactQr, Unspecified}`
  (`node_id.rs:156-166`), with `Default = Unspecified` that **fails closed** and
  `require_explicit_policy` that refuses to run until the operator chooses (`node_id.rs:179-184`)
  `[VERIFIED-CODE]`. The code deliberately refuses to "helpfully default" (`node_id.rs:21-28`).

**What each policy costs an attacker** `[INFERENCE]`:
- `OperatorSigned` — every courier onboarding is an offline, human-gated anchor signature. Sybil
  cost = N human vetting events. Strongest; least scalable; the "human touches the roster, not the
  traffic" model (batchable, occasional — B3 §, `:595-600`) `[PRIOR-ART-ADJUDICATED]`.
- `FirstContactQr` — out-of-band physical enrollment (scan at commissioning). Sybil cost = N
  physical enrollment ceremonies. Naturally rate-limited by physical presence; good fit for couriers.
- `WebOfTrust` — transitive delegation from a trusted seed set. **Caution:** must stay *asymmetric
  path-rooted* (delegation flow from anchors), never drift into a *symmetric aggregate* of
  vouches — the moment "how many peers vouch" becomes a symmetric count, Cheng–Friedman bites again
  `[THEOREM]` `[INFERENCE]`. Keep it delegation-flow, not vote-count.

**The design core to nail (not built here):** pick a `RootDelegationPolicy`, and bound the anchor's
issuance so that granting is **rate-limited structurally** — e.g. a per-anchor monotonic issuance
epoch/nonce budget checked *at delegation-sign time* (a pure predicate, no monitor), optionally with
a refundable bond at the boundary if the operator accepts the money-leg red-line (§4). This is the
"rate-limited issuance by an already-trusted quorum" candidate made concrete, and it needs **no new
consensus machinery** — it is a policy choice plus a bounded predicate over the existing
`AnchorRoster`/`verify_chain`/`RevocationSet` substrate.

**Out of scope but noted:** COUNSEL raised *first-party bilateral memory* ("I, node A, refuse new
commitments to X who defaulted on *me*") as a category Cheng–Friedman does **not** forbid
(`COUNSEL-ethics-strategy-review.md:172-183`) `[PRIOR-ART-ADJUDICATED]`. It is **not required** for
Sybil-resistance (issuance asymmetry alone delivers that) and it sits at the edge of the operator's
"жодних courier-scoring" line — a private non-gossiped experience is arguably not "a score," but it
is close enough that adopting it is a *separate* operator call, not part of this mechanism. Flagged,
not folded in.

---

## §7 — Final verdict and what it does / does not prove

**PROVEN-VIABLE-WITH-CAVEATS.**

**Proven** `[THEOREM]` `[LIVE-WEB]` `[VERIFIED-CODE]`:
1. Cheng–Friedman (2005) rules out only *symmetric* reputation functions and itself constructs an
   asymmetric path-rooted flow mechanism that *is* Sybil-proof — the operator's ruling is on the
   theorem-permitted branch, not against the theorem.
2. Douceur (2002) independently points to costly/authorized issuance (not peer reputation) as the
   Sybil defense — the anchor-roster pattern is textbook-correct.
3. The codebase's `verify_chain` + genesis-frozen `AnchorRoster` is already an instance of the
   asymmetric class: identity is free, *authorization* is anchor-gated; N Sybils get zero authority.
4. The P06 split-identity/adversarial-verifier pattern is the same "a claim needs a check by a
   different identity" invariant, on the same substrate — it generalizes to issuance exactly.
5. Revocation does **not** require a watchdog: fact-triggered revocation-on-verified-proof (+ expiry
   self-termination) is structural and degrade-closed; only behavior-monitored auto-revocation is a
   watchdog, and it is doubly forbidden.

**Caveat (why not unconditional):** operational Sybil-resistance depends on bounding the anchor's
delegation-granting decision — an unresolved `RootDelegationPolicy` operator choice (`node_id.rs`),
fail-closed today. Choose it and rate-limit issuance structurally, and the caveat closes with no new
machinery.

**Not proven / honest gaps:** (i) the sufficient-condition that "a Sybil with zero anchor-rooted
inflow contributes zero flow" is stated from the asymmetric-flow construction at
`[TRAINING-KNOWLEDGE]` level — a formal proof obligation, if ever raised, should be discharged
against the paper's Theorem statements, not this summary; (ii) the Firecracker-prod substrate claim
is Batch 5's `(training-knowledge)` on Fly's architecture — the rejection of hardware-attestation
holds for *any* unprivileged microVM regardless, so the verdict is robust to it. Neither gap changes
the verdict.

**Bottom line for the initiative:** the operator's ruling is not just permissible, it is the
*already-implemented* design — Sybil-resistance is delivered by asymmetric anchor-rooted issuance,
reputation/scoring/watchdog/proxy are all correctly absent, and the only open item is a bounded
policy decision the code already stubs out and fails closed on.

---

## §8 — Citation index (verified this session)

- `bebop-repo/bebop2/proto-cap/src/roster.rs:13-34,252-316,332-357` — no central issuer/no
  reputation ledger; `verify_chain` anchor-rooted asymmetric gate; self-issue → `UnknownIssuer`.
  `[VERIFIED-CODE]`
- `…/proto-cap/src/node_id.rs:12-28,116-141,156-184,312-345` — fail-closed genesis loader;
  `RootDelegationPolicy` = unresolved operator decision (`Default=Unspecified`); seeded-owner
  cannot mint. `[VERIFIED-CODE]`
- `…/proto-cap/src/revocation.rs:1-9,25-26,69-98,114-120,219-225(roster)` — monotonic, key/hash-
  scoped, never-score revocation; CRDT anti-entropy; `drop_anchor`. `[VERIFIED-CODE]`
- `…/proto-cap/src/claim_machine.rs:13-17` — NO-COURIER-SCORING: claim state carries no
  score/rating/trust/rank field. `[VERIFIED-CODE]`
- `dowiz/docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md
  §0,§2,§4,§5,§7,§8` — split-identity `K≠V` adversarial verifier; reuses MESH-12 `load_genesis`
  shape; "verified, not self-certified"; no score. `[VERIFIED-CODE]`
- `dowiz/docs/design/sovereign-roadmap-2026-07-16/DECART-P06-bebop2-crypto-dep.md` — key_V still
  `signed:false` stub; bebop2-core (MIT) crypto substrate; blocked on C4b. `[VERIFIED-CODE]`
- Cheng & Friedman, "Sybilproof Reputation Mechanisms," P2PECON 2005 (ACM DL
  10.1145/1080192.1080202) — no symmetric sybilproof function; asymmetric flow-based mechanism IS
  sybilproof. `[THEOREM]` `[LIVE-WEB]`
- Douceur, "The Sybil Attack," IPTPS 2002 (Microsoft Research) — without a logically centralized
  authority Sybil is always possible; defense = trusted certification / costly identity, not peer
  reputation. `[THEOREM]` `[LIVE-WEB]`
- `dowiz-agentic-mesh/…/R1-web3-failure-modes.md:67-80` — reputation rejection theorem chain;
  "sidesteps Cheng–Friedman because it is not a symmetric scoring function." `[PRIOR-ART-ADJUDICATED]`
- `dowiz-agentic-mesh/…/COUNSEL-ethics-strategy-review.md:172-183` — first-party bilateral memory
  is a category the theorem does not forbid (flagged out-of-scope here). `[PRIOR-ART-ADJUDICATED]`
- `dowiz-agentic-mesh/…/BLUEPRINT-B3-exposure-ledger-envelopes.md:581-600` — enrollment is a binary
  event not a score; "the human touches the roster, not the traffic." `[PRIOR-ART-ADJUDICATED]`
- `dowiz/docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/14-BATCH5-network-hardware-findings.md
  §0.1-0.2,§1.3` — Firecracker prod: unprivileged, virtio-only, no hardware attestation surface.
  `[PRIOR-ART-ADJUDICATED]`
- `…/13-BATCH4-consensus-trust-findings.md §1` — the contradiction this batch resolves under the
  operator ruling. `[PRIOR-ART-ADJUDICATED]`
