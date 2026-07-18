# R3 — "Heuristic Arbitration" (Selection = TrustWeight × IntegrityScore) vs the NO-COURIER-SCORING red-line

> Adversarial grounding pass (read-only against product code; write-only into this worktree).
> Cluster: the source dialogue's "Heuristic Arbitration" — the kernel's chosen tie-breaker when two
> data streams conflict (old-but-intact vs new-but-corrupted): `Selection = TrustWeight * IntegrityScore`,
> framed as "не контракт, а вагова функція, математичний вибір найменшого зла в реальному часі"
> (`00-SOURCE-DIALOGUE.md:78-81`). Grounded against this session's already-adjudicated red-line
> (`13-BATCH4 §1`, `16-BATCH7`, masterwork register #33-40) and live code. Method: Descartes-square,
> surface-the-tension-do-not-soften (operator instruction, `00-SOURCE-DIALOGUE.md:90-98`).

## Epistemics tags
- `[VERIFIED-CODE]` — read from live source this session (`file:line`).
- `[THEOREM]` — published impossibility/optimality result.
- `[PRIOR-ART-ADJUDICATED]` — already decided + reasoned in a sibling doc this session.
- `[INFERENCE]` — my derivation from the tagged facts.

---

## §0 — VERDICT (read first)

**REJECT-SAME-CLASS-AS-#33 for the formula as stated, WITH a precise constructive carve-out.**

The arbitration *mechanism* `Selection = TrustWeight × IntegrityScore` is the **same red-line
violation under new terminology** as masterwork register **#33 "Local (not global) Peer Trust
Matrix (Convergence Rate / Latency / **Data Integrity**)"** — which this session already stamped
**REJECT-ON-PHYSICS** (`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md:354`) `[PRIOR-ART-ADJUDICATED]`.
The structural equivalence is not loose: **"IntegrityScore" is literally the "Data Integrity" column
of the exact trust matrix #33 rejected, and "TrustWeight" is its "Trust" axis** — the dialogue has
re-proposed #33's two-of-three columns as a product. #33's own ground note pre-answers the "but it's
scoped differently" defense: *"a peer matrix scored on observed behavior IS courier-scoring … 'Local'
does not exempt it."* By identical logic, **"it's a data stream, not a courier" does not exempt it**,
because in the fail-operational model every stream is sourced by an adapter/node — arbitrating
*between two streams* IS ranking *two sources* (§1). It is also a near-verbatim restatement of
register **#34 "Majority-Metrics Reconciliation (TrustScore-weighted vote)"** and **#37
"Trust-Weighted Dispatcher"**, both REJECT `[VERIFIED-CODE]` (`…V2.md:355,358`).

**The carve-out (why this is not a blanket reject):** the *sound kernel* the dialogue is reaching
for — **per-stream, first-principles integrity as a boolean admission gate** (hash-chain match,
Reed-Solomon-correctable-within-bound, CRC32/CRC64, known-SchemaID, within physical-value-bounds) — is
**already adopted doctrine and is NOT #33.** It is the dialogue's own earlier "Assertion by Panic /
Self-termination / Header-as-Type" (`00-SOURCE-DIALOGUE.md:38-48`) and the kernel's live
verify-before-persist drift-gate (`event_log.rs:419-445`) `[VERIFIED-CODE]`. What is rejected is
**not** checking each stream's integrity; it is **multiplying a per-source weight into a comparative
selector to pick a winner.** The tie between two integrity-valid-but-disagreeing streams must be
broken by a *source-independent checkable fact* (capability authorization, or higher validated
anchored EpochID, or degrade-closed refusal), never by a trust/integrity **rank**.

---

## §1 — The core scope question: does the red-line's enforcement mechanism reach a data-stream weight?

**Task point 1 answered precisely. The red-line's NAME is agent-scoped; its CODE enforcement is
name-based and only *partially* trips on this proposal — so the executable gate does NOT semantically
cover data-stream arbitration. What fully covers it is the DOCTRINE, and the doctrine covers it
completely.** `[VERIFIED-CODE]` `[INFERENCE]`

Three distinct enforcement layers, each checked against live source:

1. **`claim_machine.rs:13-17` (the cited structural constraint)** `[VERIFIED-CODE]` is scoped to the
   `ClaimStatus` enum only — the courier-claim lifecycle record (`Offered/Claimed/Released/PickedUp`,
   `:19-30`). Its guarantee is narrow and literal: *"The claim state carries no score / rating / trust
   / reputation / rank field."* It structurally proves nothing about a *separate arbitration module*
   deciding between two tensor streams. So the enum constraint, by itself, does **not** reach the
   data-stream case.

2. **`scripts/ci-no-courier-scoring.sh` (the executable gate)** `[VERIFIED-CODE]` is a **field-NAME
   grep**, not a semantic guard. It flags struct fields matching
   `\b(score|rating|reputation|rank|trust_score|trust_level|courier_score|agent_rating)\b` in
   `bebop2/` (excluding `bebop2/core/`). Consequence, exactly:
   - A field `integrity_score: f32` → matches `score` → **gate goes RED.** The gate *would* catch the
     "IntegrityScore" half if named naïvely.
   - A field `trust_weight: f32` → matches **nothing** (`trust_weight` is not a listed token; only
     `trust_score`/`trust_level` are) → **gate stays GREEN.** The "TrustWeight" half **evades** the
     current grep, and renaming `integrity_score` → e.g. `sel_coeff` evades the other half too.
   This is a real, code-grounded finding: **the gate is a name-lint that would half-catch this
   proposal and is trivially renamed around.** It is not the thing that actually forbids the pattern.

3. **The DOCTRINE (`SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md:18-20,22-42`)** `[PRIOR-ART-ADJUDICATED]`
   *is* what fully covers it: *"Sovereignty is achieved by making every unit of state a signed …
   event that any peer can independently verify from first principles — **never by trusting, ranking,
   or blacklisting a source.**"* Stream-arbitration-by-weight **is ranking sources** to decide whose
   claim wins. It is inside the doctrine's prohibition regardless of what the field is named or which
   crate it lives in. `revocation.rs:25-26` states the same posture: acts on identities/statements,
   *"never on scores or reputation."* `[VERIFIED-CODE]`

**Net:** the "different scope" defense is *half-true at the grep level and false at the doctrine
level.* The name says "courier"; the principle says "source"; a data stream has a source; therefore
the red-line reaches it. The masterwork already encoded exactly this conclusion at register #33
("'Local' does not exempt it").

---

## §2 — The Sybil/gaming angle: how far does Cheng–Friedman reach into the data level?

**Task point 2 answered. The theorem's reach splits the proposal cleanly in two — and the half it
does NOT reach is precisely the half that is already adopted, while the half it DOES reach is exactly
the arbitration weight.** `[THEOREM]` `[INFERENCE]`

- **Cheng & Friedman (2005)** `[THEOREM]` (`R1 §4`, `16-BATCH7 §2`): no *symmetric* reputation
  function — the class where reputation is a function of a graph of raters/sources and every source is
  interchangeable — can be Sybil-proof. Its escape is *asymmetric, path-rooted, flow-based* trust
  (= capability/anchor authorization), which the mesh already uses (`verify_chain`) `[PRIOR-ART-ADJUDICATED]`.

- **The half OUTSIDE the theorem — `IntegrityScore` as a per-payload predicate.** A first-principles
  integrity *check on one payload in isolation* (does its hash match? is it RS-correctable? CRC-valid?
  SchemaID known? within physical bounds?) is **not a reputation function** — it is a verification
  predicate over content, identical in kind to `verify_chain` or the drift-gate. Cheng–Friedman does
  **not** reach it. This is the sound kernel; keep it, as a **boolean admission gate per stream**, not
  a continuous score.

- **The half INSIDE the theorem — `TrustWeight` and the *comparative* use of any score.** The instant
  you (a) attach a per-*source* `TrustWeight` that accrues from observed history, or (b) use
  `IntegrityScore` *comparatively* to weight one source's claim above another's, you have built a
  symmetric reputation-of-sources function — squarely inside Cheng–Friedman's impossibility and
  identical to register #34/#36/#37. A "diversity/decay factor" bolt-on is the same symmetric-scoring
  tweak the theorem covers (`…V2.md:356`, register #35 REJECT-half) `[PRIOR-ART-ADJUDICATED]`.

- **Second, independent nail (determinism):** register #36 rejected `AtomicF32 TrustScore` *also*
  because `AtomicF32` breaks determinism (`…V2.md:357`) `[VERIFIED-CODE]`. `IntegrityScore` as a
  continuous float multiplied into `Selection` imports float non-determinism into the arbitration
  decision — colliding with the kernel's deterministic-replay requirement and with
  `money_guard.rs` ("money = discrete integer channel, never interpolated", RED-LINE)
  `[PRIOR-ART-ADJUDICATED]`. Even setting aside scoring, the float-weight arbitration is
  determinism-unsafe.

**Conclusion:** the theorem genuinely extends to the data-level case **only through the weighting /
comparative-ranking step**, not through the per-payload integrity check. That is the exact seam
between "adopt" and "reject" below.

---

## §3 — Concrete attack trace (point 3): the forgery WINS arbitration, it does not merely tie

Using **this session's own live finding §1.2** (`V3-red-team-attack-catalog.md:72-85`,
`apply_event_trusts_forged_totals`, HIGH) `[VERIFIED-CODE]`: a real node can already emit a stream
whose `subtotal`/`total` are attacker-chosen — `order_from_in` (`wasm.rs:142-161`) takes them verbatim
and never recomputes from `items`. The RefSigner seam (`kernel/src/ports/agent/cap.rs:102-120`) shows
the same shape: whoever holds a signing identity produces valid-looking signed frames `[VERIFIED-CODE]`,
and the sibling bebop2 pass found unauthenticated revocation (a node acting on the trust set without
authority) among 11 HIGH (`V3-…md:9`). So "a node controls the *content and framing* of its own
stream" is not hypothetical — it is a **confirmed capability** this session already documented.

Now build Heuristic Arbitration on top and trace it `[INFERENCE]`:

1. **Two conflicting streams reach the kernel.** Legitimate node L sends the correct order
   (`items ⇒ total = 100`). Attacker node A sends the forged order (`total = 1`, §1.2). The kernel
   must arbitrate: `Selection = TrustWeight × IntegrityScore`.

2. **IntegrityScore rewards well-formedness — which A fully controls.** IntegrityScore, in the
   dialogue's own definition, is hash-chain-valid + RS-correctable + CRC-valid + SchemaID-known +
   within-bounds. A crafts a forgery that is **internally perfectly consistent**: A computes a valid
   hash over its forged bytes, valid CRC, valid RS parity, a known SchemaID, and picks `total = 1`
   which is trivially "within physical bounds." A's `IntegrityScore = 1.0`. Meanwhile L's authentic
   stream, having traversed a real lossy channel, may carry a genuine bit-flip → RS-corrected but
   `IntegrityScore < 1.0`, or a stale EpochID. **The check that produces IntegrityScore verifies that
   bytes are self-consistent, NOT that they are semantically correct or authorized** — and
   self-consistency is exactly the property the forger has total control over, while the honest party
   is subject to uncontrollable channel noise.

3. **The forgery therefore does not tie — it WINS.** Arbitration selects `total = 1`. The attacker
   has turned "old-intact vs new-corrupted" to their advantage by simply *being the clean stream*: the
   forger is always pristine because they author the bytes; the honest node is the one that looks
   "corrupted." IntegrityScore-weighted selection **actively selects for the adversary.**

4. **TrustWeight compounds it (Cheng–Friedman / whitewashing).** If TrustWeight accrues from "how
   often this source's streams were intact," A (who always emits clean forgeries) climbs TrustWeight
   while L (dinged by real noise) sinks — the classic gameable-metric inflation. A fresh Sybil starts
   clean, farms TrustWeight on honest-looking traffic, then forges once weight is high (whitewashing,
   Friedman–Resnick 2001, `R1 §4`) `[THEOREM]`. The §1.2 forged-total gap becomes **decisive** instead
   of being caught: today §1.2's fix is "recompute from `items` / refuse" (a first-principles check);
   Heuristic Arbitration would instead hand the forger a *weighting mechanism to out-rank the honest
   recomputation.*

**This is strictly worse than no arbitration.** Without it, both streams are checked independently and
the forged one is caught by first-principles recomputation (the §1.2 fix) or self-terminates. With it,
the forger games the very metric meant to protect the kernel. The attack does not require breaking
integrity — it requires *satisfying* it perfectly, which the author of the bytes always can.

---

## §4 — Precise verdict + the constructive path the operator actually wants

**REJECT-SAME-CLASS-AS-#33** for `Selection = TrustWeight × IntegrityScore` as a comparative
arbitration selector. Exact structural equivalences (all `[PRIOR-ART-ADJUDICATED]`, `…V2.md:354-358`):

| Dialogue term | Already-rejected register item | Why identical |
|---|---|---|
| `IntegrityScore` (per-source, comparative) | #33 "Data Integrity" column of the Peer Trust Matrix | it *is* that column, used to rank sources |
| `TrustWeight` | #33 "Trust" axis / #37 "Trust-Weighted Dispatcher" | per-source accrued weight = reputation |
| `TrustWeight × IntegrityScore → pick winner` | #34 "TrustScore-weighted vote, minority penalized" | weighted selection between conflicting sources |
| continuous-float weight | #36 "`AtomicF32` TrustScore" (2nd ground: breaks determinism) | float arbitration is replay-nondeterministic |

**What to build instead (the sound kernel, already doctrine — this is the ADOPT half):**

1. **Per-stream first-principles integrity as a BOOLEAN admission gate**, applied to each stream
   *independently* — hash-chain match, RS-correct-within-bound, CRC, known-SchemaID, physical-bounds.
   A stream that fails **self-terminates** (`00-SOURCE-DIALOGUE.md:38-48`); it is not "scored low," it
   is refused. This is verify-before-persist / drift-gate (`event_log.rs:419-445`) `[VERIFIED-CODE]`.
   Not #33.
2. **Break a genuine tie (both streams integrity-valid but disagreeing) by a source-independent
   checkable fact, never a weight:** (a) **capability authorization** — only the stream from the
   capability-authorized source for that resource is authoritative (`verify_chain`; authorization, not
   scoring); (b) **higher *validated* anchored EpochID / hash-chain height** wins (a checkable fact,
   the dialogue's own `Header-as-Type` + `EpochID` machinery, `00-SOURCE-DIALOGUE.md:43-48`); (c) if
   neither resolves it, **degrade-closed refusal** (mark `DATA_DEGRADED`, escalate/interpolate the
   *non-money* channel only per Header-Based Priority #37/§2.6 capability-scoped envelope — money never
   interpolated, `money_guard.rs`).
3. **Optional gate-hardening finding (not the verdict, but surfaced):** if any arbitration code is ever
   written, extend `ci-no-courier-scoring.sh`'s token list to include `trust_weight`/`weight`-on-source
   and `integrity_score`, since §1 shows `trust_weight` currently evades the grep. Better: keep the
   arbitration *out of the codebase entirely* per this verdict, so the gap never matters.

**Operator adjudication framing (Descartes-square, do-not-resolve-for-them):** this is **not** a
50/50 design question. Adopting the formula reverses register #33/#34/#37 (REJECT-ON-PHYSICS) and the
`NO-COURIER-SCORING` doctrine, and §3 shows it hands a confirmed live forgery (§1.2) a mechanism to
*win* rather than be caught. The honest kernel the dialogue is reaching for — resilient selection
between two streams — is delivered by **per-stream boolean integrity + authorization/epoch tie-break +
degrade-closed**, which needs **no weight, no score, and no new machinery.** The single open operator
decision, if they still want a weighted form, is the same one #33 raised: *do you want to lift
NO-COURIER-SCORING to admit a comparative source-integrity weight, knowing Cheng–Friedman makes it
Sybil-gameable and §3 traces the concrete exploit?* Recommendation as auditor: **surface, keep the
carve-out, do not adopt the weight.**

---

## §5 — Citation index (verified this session)

- `docs/design/fail-operational-layout-versioning-2026-07-17/00-SOURCE-DIALOGUE.md:38-48,78-81,90-98`
  — Heuristic Arbitration formula; Assertion-by-Panic / Self-termination / Header-as-Type / EpochID;
  operator "враховуючи наявні плани та роадмапи" extension framing.
- `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md:354-358`
  — register #33 (Peer Trust Matrix incl. **Data Integrity** column) / #34 / #36 / #37 REJECT-ON-PHYSICS.
- `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/13-BATCH4-consensus-trust-findings.md §1`
  — the reputation-vs-capability contradiction; Cheng–Friedman; CI gate. `[PRIOR-ART-ADJUDICATED]`
- `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/16-BATCH7-sybil-proof-capability-mechanism-findings.md §2,§4`
  — Cheng–Friedman symmetric-only scope + asymmetric escape = capability issuance. `[THEOREM]`
- `/root/bebop-repo/bebop2/proto-cap/src/claim_machine.rs:13-30` — NO-COURIER-SCORING structural
  constraint scoped to the `ClaimStatus` enum. `[VERIFIED-CODE]`
- `/root/bebop-repo/scripts/ci-no-courier-scoring.sh` — field-NAME grep
  `\b(score|rating|reputation|rank|trust_score|trust_level|courier_score|agent_rating)\b`, `bebop2/`
  excl. `core/`; catches `integrity_score`, misses `trust_weight`. `[VERIFIED-CODE]`
- `/root/bebop-repo/bebop2/proto-cap/src/revocation.rs:25-26` — acts on identities/statements, never
  on scores or reputation. `[VERIFIED-CODE]`
- `docs/verification-2026-07-17/V3-red-team-attack-catalog.md:9,72-85` — §1.2 forged-total (HIGH);
  bebop2 sibling's unauthenticated revocation among 11 HIGH. `[VERIFIED-CODE]`
- `kernel/src/ports/agent/cap.rs:80-120` — `SignatureVerifier` seam / `RefSigner` reference signer
  (whoever holds the identity frames valid streams). `[VERIFIED-CODE]`
- `kernel/src/wasm.rs:142-161` — `order_from_in` takes `subtotal`/`total` verbatim (the §1.2 surface).
  `[VERIFIED-CODE]`
- `SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14.md:18-42` — trust = signed capability, *never by
  trusting, ranking, or blacklisting a source.* `[PRIOR-ART-ADJUDICATED]`
