# ROUND-2 — Confidence-Weighted Reconciliation vs Heuristic Arbitration, and the Critical-Tier Priority Resolution (2026-07-17)

> **Status: nuance-recovery pass over the same source dialogue, per the operator's explicit
> instruction "має бути враховним та додатково дослідженим, щоб не загубити нюанси." This is NOT
> a re-litigation of R1–R4** — no verdict is reversed by default and none is defended reflexively.
> The job: find genuine nuance the first pass may have flattened; say plainly where none exists.
> Read in full for this pass: `00-SOURCE-DIALOGUE.md` **(both Part 1 and the mid-pass-appended
> Part 2, `:109-148` — hybrid determinism / aviation Kalman analogy / ConfidenceLevel header)**,
> `R2`, `R3`, the R1–R4 synthesis blueprint,
> `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` §0 (ema_next ground-truth row),
> and live source `kernel/src/geo.rs:39-41` + `kernel/src/kalman.rs` (whole file).
> Part 2 arrived while §1–§4 were being written; it is folded in as §5 without altering the
> §1–§4 analysis it independently corroborates. The operator's Part-2 response ("готувати
> інфраструктуру наперед як основа всього — не чекати поки розвалиться", `:139-148`) is weighed
> in §5.3 as a build-ahead bias per its own framing — not as a blanket verdict-flip.
> No product code touched; no commits.

---

## 0. TL;DR — six answers

1. **(a)/(b)/ambiguous read (§1):** the dialogue's *mechanism* is unambiguously **(a)-shaped**
   (winner-take-all *selection* between two streams, with a per-stream *trust* term) — R3's
   REJECT-SAME-CLASS-AS-#33 **stands as-is for the formula**. The *identity scope* of the two
   streams (same lineage vs different sources) is genuinely ambiguous in the text — the phrase
   "same EpochID lineage" appears nowhere in the dialogue; it is a charitable construction. But
   §1.3 shows the verdict is robust across **every** reading of the ambiguity: all three possible
   meanings of `TrustWeight` fail as a selector.
2. **A legitimate (b)-shaped pattern exists — but it DISSOLVES the arbitration rather than
   rescuing it (§2).** Named **Confidence-Weighted Reconciliation (CWR)**: variance-weighted
   fusion of one already-admitted stream's measurement with the kernel's own prediction — the
   Kalman pattern already in-tree (`geo.rs::ema_next` = scalar steady-state 1-D KF;
   `kalman.rs::KalmanFilter` = the full form, its own header says so). CWR *fuses*, never
   *selects*; it has no per-source weight; it is structurally unreachable by an unauthenticated
   source. Under the narrow scope, `Selection = TrustWeight × IntegrityScore` degenerates to
   nothing — what remains is boolean refusal + Kalman fusion, both already in the corpus.
3. **Critical-tier resolution (§3):** `00`-critical/money on stream failure =
   **refuse-and-escalate + last-valid-read-only + typed observable flag** — *neither* of the
   dialogue's two poles. "Ignored"-as-silently-proceed is rejected (fail-open, worse than
   refusing for money); "rescued-via-interpolation" is rejected (operator ruling + category
   error). The root of the dialogue's self-contradiction is a **missing axis**: it conflates
   *criticality* with *interpolability* (§3.3).
4. **Fault Domain Partitioning / Header-Based Priority (§4):** substantially captured by R1
   Pattern B′/UT-LAW + the bulkhead adoption and by R2 §4.1 respectively (note: the corpus names
   are **A′/B′**, not "Pattern C′" — no C′ document exists; the substance the caller meant is
   §3.2 UT-LAW + synthesis ledger row "Data Containment"). Three small, genuine gaps found:
   the **eviction predicate** for an alive-but-garbage adapter (§4.1), the **unassigned `11`
   codepoint** in the 2-bit tier field (§4.2), and a one-line adjudication of the
   `Health<Threshold` self-check that the synthesis row 67 left open (§4.3).
5. **Part 2's `ConfidenceLevel` header field (§5.2): REJECT-AS-CARRIER, ADOPT-AS-LOCAL-STATE.**
   A sender-set confidence field in the packet header is a self-reported metric — the same
   gameable class as the self-assigned tier flag (#15's exact threat) and Pattern A′
   self-certification (`key_K`); a forger sets `ConfidenceLevel = 1.0` on forged bytes for free.
   The legitimate carrier of the dialogue's idea **already exists inside CWR**: the receiver's
   *own* filter state — covariance `P`/`trace(P)` (staleness) and innovation/`last_surprise`
   (measurement disagreement) — computed locally, never transmitted, never read from the wire.
   The kernel is structurally indifferent to anything the sender claims about its own quality.
6. **CWR build timing (§5.3): the infrastructure half is upgraded DEFER → ADOPT-NOW
   (pre-built), the consumer half stays demand-driven** — the same inversion shape as the
   operator's FEC ruling. Reasoning, not reflex: the math substrate is already built and tested
   (`kalman.rs`), the missing piece is a thin type-boundary whose *absence* is precisely the
   future break ("someone wires naive extrapolation without the boundary"), and a first
   non-stranded consumer exists on day one (the existing `ema_next` smoothing path re-expressed
   through the boundary). Heuristic Arbitration's REJECT is explicitly untouched by all of §5
   (§5.4 keeps the boundary sharp).

---

## 1. Close reading: what does the dialogue's Heuristic Arbitration actually describe?

### 1.1 The exact text (the only text there is)

`00-SOURCE-DIALOGUE.md:78-81`, verbatim:

> "Heuristic Arbitration — коли ядро вибирає між двома потоками (напр. старий-цілий vs
> новий-з-помилками), формула Selection = TrustWeight * IntegrityScore, описана як «не контракт,
> а вагова функція, математичний вибір найменшого зла в реальному часі»."

Load-bearing observations, strictly from the words:

- **"вибирає між двома потоками"** — the kernel *chooses between two streams*. The operation is
  **selection** (argmax; one winner, one loser), not combination. A Kalman filter never selects
  between inputs; it *fuses* them into one estimate. So the *mechanism* named is not the
  Kalman/(b) pattern under any reading — the verb itself is (a)-shaped.
- **"TrustWeight"** — a per-stream *trust* term. Trust carries information **only if it can
  differ between the two streams**. If both signals came from the same already-authenticated,
  same-capability source, trust is identical by construction and the term is vacuous. The
  presence of a trust axis in the formula is the strongest textual evidence that the dialogue
  imagines streams whose trustworthiness *differs* — i.e., different sources. That is (a).
- **"(напр. старий-цілий vs новий-з-помилками)"** — old-but-intact vs new-but-corrupted. *This*
  clause is the genuinely ambiguous part. It admits two readings:
  - **(a)-reading:** an old node's stream (via a bridge/adapter, per the immediately preceding
    exchanges `:50-58`) vs a new node's stream — two *sources*, possibly two signers. The
    surrounding context (the whole exchange is about adapter failures and version bridges)
    supports this.
  - **(b)-reading:** the last valid snapshot (epoch N) vs the fresh damaged frame (epoch N+1) of
    the *same* stream, same signer — the task's "same EpochID lineage" construction. The text
    neither states nor excludes this; "EpochID" is never mentioned in the arbitration sentence.
- **"IntegrityScore"** and **"TrustWeight"** are given **no definition, no accrual mechanism, no
  type** anywhere in the dialogue. R3 §3.4's "if TrustWeight accrues from observed history" was
  one (reasonable, worst-case) interpretation, not a quote.

**Honest summary of the read:** mechanism = (a) (comparative selection with a trust term);
identity scope = ambiguous between (a) and (b). The dialogue did not clearly mean (b) — but it
did not clearly exclude the (b) scenario either, and the *scenario* it gives as its example
(old-intact vs new-corrupted) is exactly the scenario the (b) pattern answers. So the right
treatment is: reject the mechanism (§1.3 shows this is robust), then answer the scenario
properly (§2).

### 1.2 Was R3 right to treat it as (a)? — yes, and not merely "by necessary caution"

R3's structural-equivalence argument keys on the two named terms (`TrustWeight` = #33's Trust
axis, `IntegrityScore` = #33's Data Integrity column, product = #34's weighted vote) and on the
forger-wins trace. Both are properties of the **formula as stated**, independent of the identity
ambiguity: even in the same-lineage reading, a *selection* weighted by well-formedness selects
for whoever authors the cleanest bytes (R3 §3 step 2 applies to a compromised same-lineage
emitter identically). R3's rejection was therefore not an over-cautious flattening — it was
correct on the text's own terms. What R3 did *not* do (and this pass adds) is (i) show the
verdict is stable under every reading of the undefined `TrustWeight` (§1.3), and (ii) give the
(b) scenario its own legitimate, non-scoring answer (§2), so the dialogue's underlying *need*
is met rather than merely refused.

### 1.3 Robustness check: all three readings of the undefined `TrustWeight` fail as a selector

The dialogue never defines `TrustWeight`. Enumerate the possibilities exhaustively:

| Reading of `TrustWeight` | What it is | Why it still fails as a comparative selector |
|---|---|---|
| (i) Accrued from observed source history | per-source reputation | R3's full argument verbatim: #33/#34/#37; Cheng–Friedman; whitewashing; forger-wins trace |
| (ii) Static per-source configuration ("the new adapter is trusted 0.9, the old 0.6") | static trust ranking | Not Sybil-*farmable*, but still *ranking sources* — inside the doctrine's "never by trusting, ranking, or blacklisting a source" verbatim; and the forger-wins trace still runs (a compromised high-static-weight source wins outright); also still a continuous float in an arbitration decision (R3 §2's determinism nail) |
| (iii) Identical for both streams (vacuous) | degenerates to `Selection = IntegrityScore` | Pure well-formedness ranking — the forger-wins trace in its sharpest form: the adversary *authors* well-formedness (R3 §3 step 2); an honest stream crossing a real lossy channel loses to a pristine forgery |

Every branch fails. **R3's REJECT-SAME-CLASS-AS-#33 stands as-is for the formula, under all
readings of the ambiguity.** No verdict change; the nuance recovered is that the rejection is
*stronger* than R3 stated — it does not depend on the accrual assumption.

---

## 2. The genuine (b)-shaped pattern: Confidence-Weighted Reconciliation (CWR)

### 2.1 What it is, and why it is not Heuristic Arbitration renamed

**Name:** Confidence-Weighted Reconciliation (CWR) — deliberately distinct from the rejected
"Heuristic Arbitration," because the operation is categorically different:

| | Heuristic Arbitration (REJECTED) | CWR (this section) |
|---|---|---|
| Operation | **select** a winner between two streams' claims | **fuse** one stream's measurement with the kernel's own prediction |
| Inputs | two streams, possibly different sources | exactly one admitted stream + the kernel's own state — never two peers |
| Weight | per-*source* trust/integrity rank | per-*model* variance (Q, R) — carries no identity |
| Weight origin | accrued/assigned per identity | fixed at construction; a property of the channel physics, not of who sends |
| Reachable by unauthenticated data | yes (that is the attack) | no — structurally (§2.4) |
| Corpus status | REJECT-SAME-CLASS-AS-#33 | **already in-tree**: `geo.rs:39-41` + `kalman.rs` |

The mathematical form is exactly the in-tree Kalman update (`kalman.rs:212-250`):
`K = P·Hᵀ·(H·P·Hᵀ + R)⁻¹`, `x ← x + K·(z − H·x)` — a variance-weighted average of prediction
and measurement. `geo.rs::ema_next` (`prev + alpha*(sample-prev)`) is its scalar steady-state
special case — stated by `kalman.rs`'s own module doc (":3-6") and pinned by the in-tree test
`scalar_kf_equival_ema_next` (`kalman.rs:357-389`). This pattern is **already accepted doctrine
in this codebase** (P-A ground-truth row A2, Kalman-first integration arc), which is precisely
why the (a)/(b) distinction matters: rejecting (a) must not be allowed to creep into a
prohibition of (b), or the existing `ema_next`/`KalmanFilter` substrate becomes accidentally
suspect. It is not: the anti-#33 argument never touches it (§2.3).

### 2.2 Precise definition (predefined types, per contract §2.4)

CWR is admissible **iff all five clauses hold**; each is type-level, not reviewed-for:

1. **Single-stream key.** A CWR instance is keyed to exactly one
   `(PeerId, CapabilityClass, StreamId)` — one signer, one capability scope, one lineage. Typed
   as a phantom parameter: inputs are `AdmittedFrame<S>` and `Predicted<S>` for the *same* `S`;
   fusing across two stream keys is a **compile error**, not a runtime check.
2. **Post-admission only.** `AdmittedFrame<S>` has one constructor: the kernel's own inline
   boolean gate for stream `S` — signature/capability verify + hash-chain/epoch + value-bounds
   (the identical gate raw network input passes; UT-LAW's substrate). A frame that fails any
   boolean check is **never constructed** and therefore can never reach `update()`. CWR sits
   strictly *behind* the gate; it never influences, replaces, or softens admission.
3. **Variance weights only, identity-blind.** The only weights are the model's `Q`/`R`/`P`
   (process noise, measurement noise, covariance). None is keyed by identity; none accrues from
   observed *source behavior*; the only mutation channel is the already-guarded
   `set_q_scaler`-class knob (`kalman.rs:283-288`, guard-gated). Structurally: the CWR state
   carries **no field whose value depends on who sent the data**.
4. **Fusion, never selection.** The output is one estimate `Fused<S>`; there is no branch that
   discards one input "because the other scored higher." (The boolean gate may have *refused* a
   frame — but refusal happened upstream, on first principles, not on a comparison.)
5. **Tier restriction.** Available only on `CapabilityClass::Telemetry`/`::Optional` envelopes,
   under R2 §4's already-adopted boundary (never self-assigned tier; compile-fail on
   `Critical`/money — synthesis §3.1's trybuild DoD covers CWR unchanged).

### 2.3 Why the R3 rejection does NOT apply (the structural argument, not a vibe)

#33-class scoring requires two properties simultaneously: **(P1)** a mutable per-identity state
that accrues from observed behavior, and **(P2)** comparative use of that state across
identities to decide whose claim wins. CWR has neither:

- ¬P1: clause 3 — no identity-keyed state exists; `Q`/`R` are channel-model constants.
- ¬P2: clause 1 + 4 — there is never a second identity in scope to compare against, and the
  operation has no winner/loser branch.

Cheng–Friedman (the theorem behind #33's rejection) is a statement about *reputation functions
over a graph of sources*; a variance-weighted average of two signals from one source is outside
its object language entirely — the same seam R3 §2 itself drew ("the theorem does not reach the
per-payload predicate; it reaches exactly and only the comparative weighting step"). CWR lives
wholly on the non-reached side of R3's own seam. R3's rejection is untouched; CWR was never
inside it.

### 2.4 The concrete adversarial test (asked for verbatim by this pass's brief)

**Q: can an attacker who does NOT hold a valid capability for stream `S` ever get data into
`S`'s reconciliation step?**

**A: No — structurally.** The only path into `update()` is an `AdmittedFrame<S>`, whose only
constructor is the capability/signature verify for `S`. A frame not signed under `S`'s
capability fails verify → the value is never constructed → there is nothing to fuse. This is
the same shape as UT-LAW (the side-car's output re-enters through the identical inline gate):
CWR adds **zero** new admission surface; it consumes only what the gate already admitted.

**Honest caveats, stated plainly:**
- The guarantee is exactly as strong as the admission gate itself. A gap in the gate
  (G11-class) propagates to CWR as to every other post-admission consumer — CWR neither
  widens nor narrows it.
- An attacker **with** the capability (compromised authorized source, RefSigner finding: whoever
  holds the identity frames valid streams) can feed plausible lies into `S`'s telemetry. CWR
  grants such an attacker **no new authority**: they could already poison last-valid (#145) or
  raw pass-through; fusion cannot exceed the trust the capability already conferred, cannot
  affect any other stream (clause 1), and cannot touch command/money paths (clause 5). The
  innovation/surprise signal (`last_surprise`, `kalman.rs:271-276`) makes such poisoning *more*
  observable, not less — but per doctrine it stays **advisory/observable only**; it must never
  accrue into a per-source score (that would re-import #33 through the back door). The only
  deterministic actuator it may feed is the §4.1 circuit-breaker on *boolean gate refusals*,
  never on innovation magnitude.
- **If this narrowness is ever relaxed** — a second peer's data admitted into one filter, an
  identity-keyed weight added, a winner-take-all branch introduced — CWR becomes exactly what
  R3 rejected, and the honest name for it would again be Heuristic Arbitration. The five
  clauses are the pattern; without them there is no pattern, only #33.

### 2.5 The dissolution result: under the (b) scope, the arbitration question disappears

Run the dialogue's own example ("старий-цілий vs новий-з-помилками") under the narrow scope
(same signer, same lineage), and watch the formula evaporate:

1. "Новий-з-помилками" (new-but-corrupted) → fails the boolean gate → **refused,
   unrepresentable post-admission** (Assertion-by-Panic — the dialogue's *own earlier* concept,
   `:38-48`). There is no corrupted candidate left to weigh.
2. What remains is not a *choice between two claims* but a *state-estimation question*: "the
   stream is stale — what do I serve?" Answer, by tier:
   - **Critical:** hold last-VALID, read-only (#145) + refuse new mutations (§3).
   - **Telemetry:** KF `predict()` forward (dead-reckoning), fuse via `update()` when a valid
     frame resumes — CWR.
3. Nowhere does a score appear. **The most charitable (b)-reading of the dialogue needs no
   IntegrityScore** — which is the cleanest proof the formula was never load-bearing even for
   its own best-case scenario.

**The legitimate ghost of "IntegrityScore":** the intuition the dialogue was reaching for — "how
much do I still believe this stale estimate?" — maps onto the Kalman covariance `P`, which
grows with every `predict()`-without-`update()`. A **staleness bound** — refuse/degrade when
`trace(P)` exceeds a fixed threshold — is per-stream, non-comparative, deterministic, and
boolean at the decision point: a *value-bound on uncertainty*, same class as the adopted
"exit criteria as hard value-bounds," not a trust metric. That is the correct home for the
dialogue's instinct, and it is degrade-closed (bound exceeded → refuse, not → pick another
source).

### 2.6 One refinement to R2's deferred item (strengthens, does not reverse)

R2 row 10 deferred "the temporal interpolator" with the dialogue's phrasing "лінійна/квадратична
апроксимація." This pass's finding: **when the trigger fires, do not build a bespoke
linear/quadratic extrapolator — the interpolator and CWR are one module, already in-tree.**
`KalmanFilter::predict()` *is* temporal extrapolation (dead-reckoning during outage — the 2-D
constant-velocity test `kalman.rs:392-420` is literally courier-position dead-reckoning);
`update()` *is* the reconciliation when the stream resumes. One mechanism, deterministic
fixed-order, zero new deps, plus the covariance-staleness bound of §2.5 for free (a bare
linear extrapolator has no uncertainty tracking and would need one bolted on). The build shape
is now pinned: reuse `kalman.rs`, wrap with the §2.2 five-clause typing. *(Timing note: as
originally written this section kept the synthesis §4.4 deferral unchanged; Part 2's operator
ruling arrived mid-pass and §5.3 supersedes the timing for the boundary/infrastructure half
only — ADOPT-NOW — while the consumer trigger stands as stated here.)*

---

## 3. The critical-tier priority ambiguity — resolved precisely (R2's flagged gap)

### 3.1 The two poles, exact text

- **Pole 1 — "ignored" (`00-SOURCE-DIALOGUE.md:64-66`):** "Graceful degradation: замість
  зупинки — прапорці статусу в заголовку тензора (DATA_DEGRADED/ADAPTER_WARNING), ядро вирішує
  на основі контексту (**навігаційні дані ігноруються при деградації**, телеметрія
  використовується частково)."
- **Pole 2 — "rescue at any cost" (`:71-77`):** "операторський вибір «рятувати» критичні потоки
  навіть при збоях адаптера … **00=критичний/навігація «рятувати будь-якою ціною»** … Temporal
  Interpolation як головний інструмент порятунку — ядро бере останній валідний тензор і
  екстраполює стан (лінійна/квадратична апроксимація) замість зупинки."

The dialogue never reconciles them (R2 §3 established this). The operator has since ruled
directly this session: **"fail-operational can be used but not for red line items like money"**
[OPERATOR-RULING, quoted per this pass's brief]. That closes the *interpolation* half: critical/
money does **not** interpolate — Pole 2 is dead for the money tier. What R2 left unresolved is
whether Pole 1's "ignored" is itself safe. It is not, unrefined:

### 3.2 "Ignored" splits into two behaviors, and one of them is a fail-open

- **ignored-as-silently-proceed:** the operation continues *without* the failed input. For an
  avionics control loop dropping one bad sensor sample, this is the intended (and sane)
  reading. For money it is catastrophic: "proceed with the order, skipping price validation"
  is a **fail-open strictly worse than refusing** — it is the G11 hole generalized (accept
  without the authoritative check). REJECTED for the critical tier.
- **ignored-as-refuse:** the degraded/missing input is discarded AND every operation whose
  correctness depends on it returns a **typed refusal**. This is degrade-closed. ADOPTED.

### 3.3 The root cause of the dialogue's self-contradiction: a missing axis

The 2-bit priority conflates two orthogonal properties:

| | **Sampled continuous signal** (position, speed, ETA) | **Discrete authoritative fact** (total, price, order state) |
|---|---|---|
| **Critical** | avionics navigation: interpolation is *correct* (dead-reckoning) | money: interpolation is *category-inapplicable* (R2 §3's note) — refuse |
| **Non-critical** | telemetry smoothing: CWR/interpolate freely | e.g. applied-discount log: never fabricate; drop or refuse |

The dialogue's avionics frame lives in the left column only, where "critical ⇒ rescue by
interpolation" happens to be right *because every critical datum there is a physical signal*.
Transplanted to a delivery kernel, the most critical datum (money) sits in the right column,
where the same rule becomes fabrication. **Interpolation eligibility is decided by the COLUMN
(data-kind: signal vs fact), not the ROW (priority tier).** R2's category note saw this; the
round-2 precision is that the boundary law should key on **both bits, both type-level**:
`interpolable ⟺ (data-kind = SampledSignal) ∧ (CapabilityClass ∈ {Telemetry, Optional})`.
Priority tier alone (even correctly capability-scoped per #15) is not a sufficient license —
tier says how much you *want* the data, kind says whether a substitute can *exist*.

### 3.4 The one precise resolution (what `00`-critical/money actually does on stream failure)

1. **Never admit, never substitute.** The corrupted/missing frame fails the boolean gate; no
   interpolation, no extrapolation, and no treating last-valid *as if fresh* for a mutating
   decision. (Operator ruling + money RED-LINE "discrete integer channel, never interpolated.")
2. **Dependent operations REFUSE, typed.** Any operation whose correctness depends on the
   failed input returns a typed refusal — not a silent skip, not a fabricated value, not a
   proceed-without-check. For money specifically the refusal has one escape, and it is
   **derivation, not fabrication**: server-side recompute from authoritative inputs
   (`compute_order_total`, `domain.rs:129-145`) — recomputing a fact from its true inputs is
   first-principles verification, the exact opposite of interpolating it from its own history.
3. **Reads of the last VALID committed state stay servable, read-only, marked stale** (#145
   Survival-Mode "Locked-readable") — endurance without fabrication. No NEW money-mutating
   decision consumes the stale value.
4. **The failure is observable, never silent:** `DATA_DEGRADED`-class typed flag + Escalate
   (mesh #20/#22 — Escalate, not death, at the semantic layer). The dialogue's status-flag idea
   survives intact — it was only its "ignore-and-proceed" *consumer* that had to be corrected.

One sentence: **critical-tier failure = refuse-and-escalate, serve last-valid read-only,
recompute-from-truth where a recompute path exists — never proceed-without, never interpolate.**
This is neither of the dialogue's poles; it is the only behavior compatible with the operator
ruling, the money RED-LINE, and the degrade-closed corpus simultaneously.

---

## 4. Fault Domain Partitioning + Header-Based Priority — confirmation and three genuine gaps

**Naming correction first (honesty):** this pass's brief attributed the bridge-containment work
to "Fable-B's Pattern C′." No Pattern C′ exists in the corpus — the patterns are **A′**
(trusted-proxy, REJECT) and **B′** (untrusted-translator, ADOPT) in R1 §4.2, codified as
**UT-LAW** in synthesis §3.2, with Fault Domain Partitioning adopted separately as bulkheading
("ADOPT as fail-operational bulkheading, the P-C bulkhead class #11 — orthogonal to, never a
substitute for, the trust argument," synthesis supplementary ledger + R1 §4.5). Substance
confirmed fully captured under those names: memory sandbox per adapter, read-only-on-old,
adapter self-termination as defense-in-depth, kernel inline re-verify as the authority,
adapter-death → empty buffer → observable disable. Header-Based Priority likewise confirmed
captured by R2 §4.1 (#15 `CapabilityClass`, never self-assigned). Three genuine gaps neither
pass named:

### 4.1 The eviction predicate — the alive-but-garbage adapter (real gap)

R1 §4.2 covers adapter **death** ("empty buffer → kernel disables that adapter" — observable).
The dialogue's "«відрізання» хворого адаптера від шини даних" (`:77-78`) also covers adapter
**sickness**: alive and emitting, but every frame fails the gate — a compute-DoS on gate
evaluation that "empty buffer" detection never triggers on. *Who decides it is sick, and by
what?* Unspecified in both the dialogue and R1/synthesis. The safe form, pinned now so nobody
later fills the hole with a health score:

- **Deterministic circuit-breaker on boolean gate refusals:** an integer counter of
  *consecutive* gate failures per adapter; at a fixed bound N → disable the adapter's intake
  (typed, observable, re-enable on operator action or a deterministic cool-down). This is
  **TokenBucket-class, not #33-class**: (i) it consumes only boolean outcomes the kernel's own
  gate computed — never self-reported health, never innovation magnitude, never a float;
  (ii) it is per-adapter and non-comparative — no adapter is ranked against another;
  (iii) it decides *intake on/off*, never *which of two conflicting payloads wins*.
- Explicitly REJECTED for this slot: any continuous per-adapter health/quality score
  (that is #33's "Convergence Rate" column reborn), and any use of CWR's surprise signal as
  the disable input (§2.4's caveat).

### 4.2 The unassigned `11` codepoint (small, real, free to fix)

The dialogue assigns `00`/`01`/`10` (`:72-74`) and leaves `11` undefined. A 2-bit field has
four states; an undefined discriminant that is not hard-rejected becomes the next silent
fail-open. Law (one line, mirrors the existing `unknown_version_is_rejected_on_decode` red-team
test, `framing.rs:102-114`): **unknown tier discriminant → typed decode reject.** In the #15
`CapabilityClass` mapping this is nearly free — a `#[non_exhaustive]`-style match with an
explicit reject arm, same shape as R1 §3.2's tagged-enum law.

### 4.3 `Health<Threshold` self-termination — one-line adjudication (partial closure of ledger row 67)

The bracketed exchange `:33-35` gives Compute-Units "самознищуються при Health<Threshold" — a
health *score* — which synthesis row 67 honestly left NOT-ADJUDICATED. As it bears on fault
domains, one line closes the dangerous half: a unit's **self**-check against a **fixed bound on
its own observables** is the value-bound/self-termination class (admissible — it is
non-comparative and self-referential, same family as `!rho.is_finite()` → self-terminate). The
same quantity **exported to the kernel as a trust input, or compared across units**, is
#33-class (inadmissible). Self-bound: yes; health-as-signal-to-others: never. The rest of the
State-Keepers/Compute-Units exchange remains honestly unadjudicated.

---

## 5. Part 2 fold-in — Hybrid Determinism, the Aviation Analogy, and the `ConfidenceLevel` Header

> Part 2 (`00-SOURCE-DIALOGUE.md:109-137`) arrived mid-pass. Its aviation example — deterministic
> PID control loop + stochastic sensor fusion via Kalman filter (`:121-125`) — is, verbatim, the
> (a)/(b) seam §1–§2 had already drawn independently: the dialogue's own best argument now
> *names the Kalman filter* as the correct home for stochasticity, which is exactly CWR. This
> section adjudicates the two genuinely new items Part 2 adds: the `ConfidenceLevel` header
> proposal (`:136-137`) and the operator's build-ahead ruling (`:139-148`).

### 5.1 Hybrid determinism — corpus alignment, plus the one precision the dialogue misses

Part 2's split — "Детермінізм там, де це фізика … Стохастичність там, де це реальність"
(`:117-120`) — is not a new doctrine for this codebase; it is a *description of the corpus as it
already stands*: deterministic law on the command path (event_log, money integer channel, state
machines, replay), stochasticity *tolerated as input* on the telemetry/eventual lane (R2 §4,
synthesis §3.1's honest determinism note). The "culture of determinism trap" critique (`:126-130`)
therefore does not land on this architecture — the corpus never demanded deterministic sensor
noise; it demands deterministic *verification of claims*, a different object.

**The precision Part 2 blurs, stated once:** the dialogue slides between "stochastic input" and
"non-deterministic computation." They are independent. The in-tree Kalman filter is
**deterministic code over a stochastic model**: fixed-order f64 operations, same inputs → same
bits (`kalman.rs:17-18`, "all operations are fixed-order and deterministic"). Accepting the
noise of reality does not require surrendering replayability of the computation — and the
corpus keeps both. Slogan form, so no future reader flattens it: **stochastic model,
deterministic code.** (This also answers Part 1's closing determinism question `:85-88` more
sharply than R4 did: the system never "works on damaged data" in the command sense — it works
on *estimates with tracked uncertainty* in the telemetry sense, computed reproducibly.)

**One straddle-item flagged (keeping the §1 boundary sharp):** Part 2 lists "передбаченні
поведінки вузлів мешу" (predicting mesh-node behavior, `:120`) among the legitimate stochastic
domains. That phrase straddles the (a)/(b) line and must be scoped now: stochastic estimation
may target **channels and signals** — link loss rate, RTT, congestion, node *availability* for
scheduling/transport parameter choice (the `:133` compression/routing heuristic is this, and it
is the TCP-congestion-control class: deterministic algorithms over stochastic channel
observations, deciding *transport parameters*, never *whose claim wins*). It may **never**
target the **authority of claims** — a per-node behavioral model consulted when deciding
whether to accept a node's data is #33 under a Part-2 vocabulary. Channel physics: yes.
Identity trust: never. Same seam as §1, third appearance, same verdict.

### 5.2 The `ConfidenceLevel` header field — REJECT-AS-CARRIER, ADOPT-AS-LOCAL-STATE

The dialogue's closing proposal (`:136-137`): every node transmits "тензор + ConfidenceLevel,"
the field living in the packet header as "the bridge between the deterministic foundation and
the stochastic network reality." Split the idea from the carrier:

**The carrier — a SENDER-set header field — is rejected, on three already-established grounds:**

1. **It is a self-reported metric, attacker-priced at zero.** The forger-wins trace (R3 §3)
   transfers verbatim: the party that authors the bytes authors the confidence claim;
   `ConfidenceLevel = 1.0` on forged frames costs nothing, while an honest sender crossing a
   noisy channel honestly reports less. A comparative consumer of the field selects *for* the
   adversary — the exact gameable-self-reported-metric problem, third form (after
   IntegrityScore and the self-assigned tier bit).
2. **It is the #15 threat, re-proposed.** The corpus's single load-bearing correction to
   Header-Based Priority was "checked against capability scope, **never self-assigned**"
   (R2 §4.1). A wire confidence field is a self-assigned quality claim — the same hole #15
   exists to close, one field over.
3. **It is `key_K` self-certification.** A sender attesting the quality of its own output, with
   the kernel consuming that attestation, is the Pattern A′ shape UT-LAW exists to forbid: the
   kernel must be **structurally indifferent to what the sender claims about itself** — the
   only admissible attestations on the wire are the ones the kernel re-verifies from first
   principles (signature, hash-chain, epoch), and "how confident I am" is not first-principles
   verifiable by construction.

Corollary: **do not put the field on the wire at all** — not even as "advisory." Every wire
field eventually finds a consumer; an advisory confidence byte is a standing invitation for
some future branch to read it (the same reasoning that keeps arbitration code out of the tree
entirely, R3 §4.3). A sender-side *sensor-metadata* variant (e.g. GPS HDOP as claimed
measurement noise `R`) fails the same test: sender-controlled `R` manipulates the Kalman gain
directly (claim tiny `R` → my measurement dominates the fuse). `R` is a **receiver-side model
constant per channel class** — calibrated locally, never negotiated with the counterparty.

**The idea — "the system should know how much it believes its current estimate" — is correct,
and CWR already carries it, locally:**

| Dialogue's want | The legitimate, receiver-local carrier | Where it already lives |
|---|---|---|
| "how degraded is this stream's data right now" | covariance `P` / `trace(P)` — grows with every `predict()`-without-`update()`; the §2.5 staleness bound refuses past a fixed threshold | `kalman.rs` state; §2.5 |
| "how much did the last frame disagree with expectation" | innovation / `last_surprise = ‖y‖/√tr(S)` — observable anomaly signal, advisory-only | `kalman.rs:271-276`; §2.4 caveat |
| "a bridge between deterministic core and stochastic reality" | the CWR type boundary itself: `AdmittedFrame<S>` (deterministic admission law) → fusion (stochastic model, deterministic code) → `Fused<S>` + local confidence | §2.2 clauses 1–5 |

So the Part-2 question "чи не здається вам, що саме цей ConfidenceLevel … і є той міст?" gets a
precise answer: **yes to the bridge, no to the header.** The bridge exists; it is the
receiver's own filter state. Confidence is a property of the receiver's model, never a field
of the sender's packet. (Note the symmetry with §2.5's "legitimate ghost of IntegrityScore" —
Part 2 independently re-derived the same instinct and reached for the same wrong carrier;
covariance-as-local-state answers both.)

### 5.3 Build timing under the operator's ruling — the explicit call: split ADOPT-NOW / demand-driven

The operator's response is binding as a *bias*, by its own text: build resilience infrastructure
ahead of time ("основа всього — не чекати поки розвалиться"), read together with the FEC
inversion ("reed-solomon will be used, add FEC too"), while "не силентно фліпати кожен DEFER"
(`00-SOURCE-DIALOGUE.md:139-148`). Weighing CWR against that honestly, not reflexively, in
either direction:

**For ADOPT-NOW (infrastructure half):**
- **Near-zero marginal cost.** The math substrate is *already built and tested* (`kalman.rs`
  full KF with fail-closed singular-S, EMA-equivalence pin, 2-D dead-reckoning test;
  `geo.rs::ema_next` in production use). What is missing is only the thin boundary layer:
  `AdmittedFrame<S>`/`Predicted<S>`/`Fused<S>` types, the `CapabilityClass` tier gate with its
  trybuild compile-fail, the `trace(P)` staleness bound, the five-clause placement law. Zero
  new dependencies, zero product-behavior change until a consumer wires in.
- **The boundary IS the "не чекати поки розвалиться" item.** The failure the operator's ruling
  targets is exactly what happens *without* this layer: the first telemetry consumer that
  needs dead-reckoning gets built in a hurry, wires a naive extrapolator with no admission
  typing, no tier gate, no staleness bound — and the break has happened. Pre-building the
  boundary makes the wrong wiring unrepresentable before any consumer exists. Safety
  infrastructure whose value is preventing a future mis-build is the *strongest* case for
  build-ahead; a feature nobody uses is the weakest. CWR's boundary is the former.
- **Non-stranded from day one.** The stranded-organ risk (the honest counter-argument — this
  corpus has 9/11 stranded organs on record) is answered concretely: the existing production
  `ema_next` smoothing path is already CWR's degenerate case (α = steady-state gain, proven
  equivalent by the in-tree test). Re-expressing that one existing consumer through the CWR
  boundary at adoption time gives the layer a live, tested consumer immediately, at zero
  feature cost. The wrapper is born wired, not shelved.
- **Consistency with the FEC inversion.** FEC moved to pre-built because the operator wants the
  resilience floor to exist before the carrier that needs it; CWR's boundary is the same shape
  one layer up (the estimation floor before the consumer that needs it). Treating structurally
  identical cases identically is the non-arbitrary reading of the ruling.

**Against flipping the consumer half:** the dead-reckoning *feature* (courier-position
smoothing during outages, ETA fusion) adds resilience only when a product surface consumes it;
building the feature with no surface is the stranded-organ pattern with extra steps, and
nothing in the operator's text demands features ahead of surfaces — it demands
*infrastructure* ("інфраструктуру наперед"). The infrastructure/feature line is therefore
where the ruling itself cuts.

**Verdict (explicit, superseding §2.6's deferral for the boundary only):**
- **CWR boundary + local-confidence state (the five §2.2 clauses, types, tier compile-fail,
  `trace(P)` bound, `ema_next`-path first consumer): ADOPT-NOW — pre-built infrastructure.**
  DoD carries over from §2.2/§3.1-synthesis unchanged (trybuild compile-fail on `Critical`,
  self-assigned-tier chaos test, cross-stream-fusion compile error, staleness-bound refusal
  test — all RED-first).
- **Dead-reckoning/fusion consumers beyond the day-one `ema_next` re-expression:
  demand-driven** (trigger unchanged from synthesis §4.4: the courier-position/ETA feature is
  actually built). When they come, they reuse `kalman.rs` predict/update per §2.6 — no bespoke
  extrapolator.
- **Unchanged by the ruling:** every REJECT in this document and in R1–R4. A build-ahead bias
  accelerates *admissible* infrastructure; it has no force on inadmissible mechanisms
  (sender-set ConfidenceLevel §5.2, Heuristic Arbitration §5.4, critical-tier interpolation
  §3). The ruling's own text says as much.

### 5.4 The boundary, kept sharp (unchanged by Part 2)

Nothing in Part 2 touches the Heuristic Arbitration rejection, and one sentence keeps the two
ideas from ever being conflated downstream: **ConfidenceLevel-of-a-signal is the receiver's
locally-computed uncertainty about its own estimate of one admitted stream;
TrustWeight-of-a-peer is a comparative claim about identities.** The first is CWR (§2, adopted,
now pre-built); the second is #33 (rejected, R3, §1.3 — robust under every reading). The
aviation analogy itself respects this line: a drone's Kalman filter fuses *its own sensors'*
noisy signals; it does not rank *other aircraft* by trustworthiness to decide whose position
report is true. Part 2, read carefully, argues for (b) — and (b) is what was adopted. The
rejection of (a) stands untouched.

---

## 6. Citations (live-verified this pass)

- `00-SOURCE-DIALOGUE.md:33-35` (Health<Threshold), `:38-48` (Assertion-by-Panic/Header-as-Type),
  `:50-58` (bridges context), `:64-66` (Pole 1 "ігноруються"), `:71-77` (Pole 2 "рятувати
  будь-якою ціною" + interpolation + 2-bit tiers + fault partitioning), `:78-81` (Heuristic
  Arbitration, quoted in full §1.1), `:85-88` (Part 1 closing determinism question).
- `00-SOURCE-DIALOGUE.md` Part 2: `:117-120` (determinism-as-law / stochasticity-as-adaptivity
  split; "передбаченні поведінки вузлів мешу" straddle-item), `:121-125` (aviation PID +
  Kalman sensor-fusion analogy), `:126-130` (over-determinism trap), `:133` (channel-noise
  routing heuristic), `:136-137` (ConfidenceLevel header proposal, quoted §5.2), `:139-148`
  (operator build-ahead ruling, verbatim, incl. the no-silent-flip qualifier and the FEC
  precedent).
- `kernel/src/geo.rs:39-41` — `ema_next(prev, sample, alpha) = prev + alpha*(sample-prev)`.
- `kernel/src/kalman.rs:1-26` (module doc: ema_next = scalar steady-state 1-D KF special case),
  `:212-250` (`update`: gain, innovation, fail-closed singular-S), `:255-262` (`gain`),
  `:271-276` (`last_surprise`), `:283-288` (`set_q_scaler` guarded knob), `:357-389`
  (`scalar_kf_equival_ema_next`), `:392-420` (2-D constant-velocity dead-reckoning test).
- `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` §0 row 1 (ema_next ground-truth
  MATCH), row A2; `domain.rs:129-145` money law (via P-A §0, re-verified there this session).
- `R3-heuristic-arbitration-vs-courier-scoring-grounding.md` §0-§4 (the standing rejection;
  forger-wins trace; carve-out; the Cheng–Friedman seam §2 this pass builds on).
- `R2-fail-operational-vs-degrade-closed-grounding.md` §3 (the 64-66/71-77 contradiction; the
  category note), §4 (#15 tier boundary; the three composing mechanisms).
- `R1-layout-versioning-bridges-grounding.md` §4.2 (Patterns A′/B′; adapter-death disable),
  §4.5 (bulkhead adoption).
- `BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md` rows 8/9/10/12; supplementary
  rows "Data Containment," "Header-Based Priority," row 67 (NOT-ADJUDICATED); §3.1 (tier DoD),
  §3.2 (UT-LAW), §5.1, §5.3.
- Operator ruling "fail-operational can be used but not for red line items like money" —
  [OPERATOR-RULING], relayed in this pass's brief; treated as binding for §3.
