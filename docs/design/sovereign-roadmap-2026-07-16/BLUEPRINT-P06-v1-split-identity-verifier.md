# BLUEPRINT — Phase 6: V1 SPLIT-IDENTITY + ADVERSARIAL VERIFIER (epistemic bedrock)

> **Anchors:** V1 (whole), E9 (verifier half). **Depends on:** Phase 1 (unsigned V5-C re-exec
> harness + claim-latency ledger), Phase 3 (PQ trust-root hardened — hybrid K/V keys put the
> still-open C4b side-channel on the V1 signing path). **Parallel-safe with:** Phase 5, 7, 8.
> Canon: `ARCHITECTURE.md` §0/§4/§8, `STRATEGIC-VECTORS-LOCKED-2026-07-16.md` §V1/§V5.
> Primary evidence: `R1-B-hub-autonomy-agent-infra-gap-analysis.md` §V1 (file:line, code-verified).
>
> **Planning artifact only. No code is written or edited by this document.** This blueprint
> designs the phase; the build lands under the Phase-1 CI floor and the Phase-6 done-tests below.

---

## 0. The problem this phase solves (one paragraph, no metaphor)

Every "GREEN" in this repo is currently self-certified. The three-model review
(`scripts/three-model-review.sh`) is orchestrated by the *same session* that authored the diff —
no identity separation, no re-execution (R1-B §V1 B-fragment 1). HK-03
(`hermes-kernel/kernel/src/verification.rs:1-29`) is a `SessionState` FSM where `Complete` is legal
only from `Verified`, but the same agent both claims and checks — its own header cites "~30
false-'done' corrections over 8 days" (R1-B §V1 B-fragment 3). BRAIN-TOPOLOGY records the failure
mode directly: 97.8% single authorship ("no independent second party"), and a **52-second GREEN on
a 1,610-insertion diff** — "a claim-shaped artifact replaces a check-shaped artifact." Meanwhile the
entire crypto substrate needed to fix this — ACVP-verified ML-DSA-65, a real two-leg hybrid gate,
canonical TLV encoding, an operator-gated genesis loader — **sits finished and idle**, built for the
delivery protocol and unused for exactly this purpose (R1-B §V1: "the ML-DSA infrastructure already
built for the delivery protocol is sitting unused for exactly this"). Phase 6 wires that idle
substrate into a signing + re-execution loop so that a claim can no longer stand in for a check.

---

## 1. Current-state evidence: what already exists and is reusable

All claims below are code-verified in R1-B §V1 (file:line). This phase **reuses**, does not rebuild.

**A-side substrate (identity + signing primitives — all BUILT):**
- **ML-DSA-65 from scratch, zero external crates, FIPS-204 byte-exact, ACVP KAT-verified** —
  `bebop2/core/src/pq_dsa.rs:1-16` + `core/src/pq_dsa/acvp_tests.rs`. This is the signature engine
  for key_K / key_V.
- **Hybrid gate, both legs REAL** — `proto-cap/src/hybrid_gate.rs:1-12`: Ed25519 leg via
  `bebop2-core::sign`, PQ leg via `signed_frame::{sign_pq,verify_pq}`; `RequireBoth` enforces real
  ML-DSA-65 verification; missing/invalid PQ ⇒ `HybridIncomplete`/`PqVerifyFailed`, never a
  fabricated pass. This is the verification path a verdict signature rides on.
- **`derive_pq_seed` domain separation (C6)** — referenced in `pq_dsa.rs` (commit `15f0b24`). This
  is the seed-derivation primitive the K/V keygen ceremony reuses.
- **Canonical TLV encoding (C7b)** — `revocation.rs:20-23` hashes `Capability::canonical_bytes_tlv`.
  This is the deterministic byte layout the diff-signer and verdict-signer encode over.
- **Operator-gated genesis loader (MESH-12)** — `proto-cap/src/node_id.rs:1-19`: fail-closed
  anchor-file loader ("empty/absent list authorizes nothing"); `roster.rs:187-204` enrolls trust
  anchors at genesis only, then freezes them. This is the exact pattern the K/V anchor file follows.
- **SHAKE256-J (C8)** `core/src/pq_kem.rs`; **CT `scalar_mul` (C4)** `core/src/sign.rs`;
  operator-cert tests `wss_transport.rs:864-909`.

**Open follow-ups this phase inherits from Phase 3 (hard dependency):**
- **C4b (HIGH)** — `sign.rs:612 mod_l` is still variable-time on the secret nonce (biased-nonce →
  lattice key-recovery class). If key_K/key_V are hybrid (Ed25519⊕ML-DSA per E10), **C4b is on the
  V1 signing path** — every key_K diff-signature and every key_V verdict-signature would exercise the
  leaky Ed25519 leg. Phase 6 MUST NOT begin signing until Phase 3's dudect/CT gate is GREEN on
  `mod_l`.
- **C3** — `pq_dsa::keygen(seed)` is unconditionally `pub`; should be feature-gated. Fixed as part of
  key hygiene here (keygen behind a `ceremony` feature so a random build cannot mint an anchor key).
- **Stale docstring** — `proto-cap/src/lib.rs:12-16` still calls the ML-DSA leg "a marked TODO"
  (contradicts `hybrid_gate.rs` + `pq_dsa` tests). Fixed in passing.

**B-side (harness + meter — the re-exec runner is BUILT UNSIGNED in Phase 1):**
- Phase 1 builds the **V5-C re-execution harness** (fresh checkout, re-execute the suites, emit
  RED/GREEN + rationale) and the **claim-latency ledger** (V5-B: one entry per commit). Phase 6 adds
  ML-DSA K/V signatures + the merge gate *on top of* that harness (R2 §1 "Major merges": "Phase 1
  builds it unsigned, Phase 6 adds ML-DSA K/V signatures + merge gate on top"). **The runner is not
  rebuilt here.**
- **`FalseClaimMeter`** — `hermes-kernel/kernel/src/governance.rs:1-25`: a claimed-vs-verified audit,
  computed but **not fed** by a per-commit latency log (R1-B §V1). Phase 6 wires the Phase-1 ledger
  into it.

**What does NOT exist (the whole gap):** grep `key_K|key_V|split_identity` across all three repos =
zero hits outside docs. Nothing signs a git diff or commit with any key; dowiz CI
(`.github/workflows/ci.yml`) has no verifier job and no merge-block tied to any verifier signature.

---

## 2. Keygen ceremony design (K/V split identity)

**Goal:** two distinct ML-DSA-65 (hybrid) anchor keys — `key_K` (author/diff-signer) and `key_V`
(verifier/verdict-signer) — created under operator control, provably distinct, with private material
never in-repo.

**Reuse:** the MESH-12 `load_genesis` pattern (`node_id.rs:1-19`) verbatim in shape. A new
`config/kv-genesis.txt` anchor file lists exactly two public-key anchors, tagged `role=K` and
`role=V`. Empty/absent ⇒ authorizes nothing (fail-closed inherited).

**Seeds:** each private key derives from operator-supplied entropy via `derive_pq_seed` (C6) with a
distinct domain-separation label — `"dowiz.v1.key_K"` vs `"dowiz.v1.key_V"` — so the two seeds are
independent by construction, not by luck. Keygen runs behind the new `ceremony` feature gate (closes
C3): a normal `cargo build` cannot mint an anchor key.

**Hybrid keys ⇒ Phase-3 gate:** because E10 mandates hybrid identity (Ed25519⊕ML-DSA), each anchor
carries both a classical and a PQ public key, and signing touches the Ed25519 `mod_l` path. **The
ceremony is blocked on Phase 3 closing C4b.** This is recorded as a hard precondition, not a
recommendation.

**Ceremony steps (operator-run, one-time, output committed = public anchors only):**
1. Operator provides two independent entropy inputs (offline).
2. `cargo run --features ceremony -- keygen --role K` and `--role V` derive seeds via
   `derive_pq_seed` with the role-labels, emit `(pub_K, priv_K)` and `(pub_V, priv_V)`.
3. Private keys are written to an operator-controlled EnvFile / secret store (S3 discipline: NEVER
   in-repo, gitleaks-gated by Phase 1). Public anchors are written to `config/kv-genesis.txt`.
4. `load_genesis` on the anchor file at gate-start refuses to run if the file lists fewer than two
   anchors, if either role tag is missing, or if `pub_K == pub_V` (the `K ≠ V` invariant is checked
   at load, not only at merge).

**Rotation:** re-run the ceremony, append a new anchor with a monotonic epoch, revoke the old via the
existing `RevocationSet` (`revocation.rs:35-55`, monotonic/irreversible). Verdicts record the epoch
they were signed under.

---

## 3. Diff-signer design (key_K over the diff)

**What is signed:** a canonical TLV structure (C7b `canonical_bytes_tlv` style, so encoding is
deterministic and re-derivable byte-for-byte) binding the commit to its content. The signed message
is the **hash tuple**, not the raw diff bytes, so the artifact is small and stable:

```
DiffAttestation TLV (canonical, C7b layout):
  T=0x01 commit_sha3      : 32 bytes   (sha3-256 of the commit object)
  T=0x02 diff_sha3        : 32 bytes   (sha3-256 of the canonical unified diff vs merge-base)
  T=0x03 base_sha3        : 32 bytes   (sha3-256 of the merge-base commit — pins what the diff is against)
  T=0x04 key_K_anchor_id  : 32 bytes   (which K anchor; supports rotation epochs)
  T=0x05 redline_touch    : 1 byte     (0/1 — set iff the diff touches a red-line path; see §5)
  T=0x06 timestamp        : 8 bytes    (author-claimed sign time, big-endian unix — feeds §6)
```

**Signing flow:**
1. Author computes `commit_sha3`, canonicalizes the unified diff against the merge-base and hashes it
   (`diff_sha3`), records `base_sha3`.
2. Author sets `redline_touch` by running the Phase-1 red-line path matcher (same matcher the merge
   gate uses — computed once, signed, re-checked by the gate).
3. Author encodes the TLV and signs it with `priv_K` (hybrid: Ed25519 leg + ML-DSA-65 leg, both
   real, via the existing `sign` / `sign_pq` paths).
4. The signature + the TLV are stored as a **git note** on the commit (`refs/notes/v1-diff-attest`),
   not in the tree — so re-signing after an amend does not churn the working tree, and the note is
   fetchable in CI.

**Determinism requirement:** the diff canonicalization must be stable (fixed context lines, sorted
file order, LF-normalized) so that the verifier, re-hashing the same commit range independently,
derives an **identical `diff_sha3`**. A mismatch is a hard RED (the attestation does not describe the
diff the verifier sees).

---

## 4. Independent-verifier design (key_V over the verdict)

**What it does:** takes a commit that carries a valid key_K DiffAttestation, **re-executes the
Phase-1 V5-C harness in an independent context**, and emits a key_V-signed RED|GREEN verdict with a
free-text rationale — never trusting the author's claim, only the re-execution result.

**Verdict TLV (canonical, signed with key_V):**
```
Verdict TLV:
  T=0x01 diff_attest_sha3 : 32 bytes  (sha3 of the key_K DiffAttestation it is judging — binds V to a specific K claim)
  T=0x02 recomputed_diff_sha3 : 32 bytes  (V's own recomputed diff_sha3 — MUST equal K's T=0x02)
  T=0x03 verdict          : 1 byte    (0x00=RED, 0x01=GREEN)
  T=0x04 suite_results    : var       (per-suite pass/fail: kernel cargo test, engine test, node, playwright)
  T=0x05 key_V_anchor_id  : 32 bytes
  T=0x06 context_descriptor : var     (isolation evidence — see O9 below)
  T=0x07 rationale         : var (utf-8, bounded)   (why RED/GREEN; on RED, the first failing suite/test)
  T=0x08 claim_latency_ms  : 8 bytes  (verify start→verdict, feeds §6)
  T=0x09 residue           : fixed string "enforced approximation: identity != person"  (§8, ALWAYS present)
```

**Isolation mechanism — concrete proposal, FLAGGED PENDING PHASE-2 RULING O9.**
> Per R2 §4, the V1-B context-isolation bar (fresh worktree / separate machine / different model
> family) is **operator decision O9, ruled in Phase 2**. This blueprint MUST NOT invent the bar. The
> following is a **recommendation only**, to be ratified, tightened, or replaced by O9:
>
> **Recommended tier (until O9 rules otherwise): fresh detached `git worktree` + fresh process +
> different model family.** Rationale: it is the strongest bar achievable with zero standing
> infrastructure (no second machine to provision, consistent with M6 zero-dep and the single-host
> reality), it defeats the dominant observed failure (same-session self-certification) because the
> verifier process shares no memory/context with the author session, and pairing it with a
> *different model family* for the rationale step decorrelates the judgment from the author model
> (the same decorrelation principle the `research-verifier` agent uses). The `context_descriptor`
> (T=0x06) records concrete isolation evidence: worktree path hash, PID, and the model-family id used
> for the verdict, so the isolation actually applied is auditable, not asserted. **If O9 rules for
> "separate machine" or "different model family, mandatory," the descriptor schema already carries
> the field; only the gate's acceptance predicate on T=0x06 changes.** Recorded as an enforced
> approximation regardless (see §8).

**Verification flow:**
1. Fetch the commit + its `refs/notes/v1-diff-attest`; verify the key_K signature via the hybrid gate
   (`RequireBoth`). Invalid/missing K signature ⇒ the verifier does not even run the suites; it emits
   RED with rationale "no valid author attestation."
2. Check `key_K_anchor_id` resolves to a `role=K` anchor in `kv-genesis.txt`. A signature by a
   `role=V` anchor on a diff attestation is rejected here (defense in depth for the gate's `K ≠ V`).
3. Materialize the commit in a **fresh isolated worktree** (per O9 tier), recompute `diff_sha3`
   independently, and assert it equals K's `T=0x02` (mismatch ⇒ RED).
4. Run the Phase-1 V5-C harness (the already-built unsigned runner) to completion. Record per-suite
   results and total `claim_latency_ms`.
5. Set `verdict` = GREEN iff every suite the harness selected passed; else RED with the first failing
   suite/test in `rationale`.
6. Encode the Verdict TLV **including the standing residue string (T=0x09)**, sign with `priv_V`
   (hybrid), and publish as a git note `refs/notes/v1-verdict` on the commit.

The verifier is **stateless and re-runnable**: given a commit, anyone can reproduce the verdict. It
never mutates product code and never writes to the tree (notes only).

---

## 5. CI merge-gate design

A new Phase-1-hosted CI job (`v1-verifier-gate` in `.github/workflows/ci.yml`) that **blocks merge**
unless ALL of the following hold. Any failure ⇒ CI RED, merge blocked.

1. **Both signatures present.** The commit carries a valid key_K DiffAttestation note AND a valid
   key_V Verdict note. Missing verifier signature ⇒ RED (this is done-test case #3).
2. **`key_K ≠ key_V`.** The two anchor ids differ AND resolve to distinct roles (`role=K` vs
   `role=V`) in `kv-genesis.txt`. A verdict signed with key_K — the author's own key, not the
   verifier's — is rejected here (done-test case #2). The check is by anchor id, not just by key
   bytes, so an author cannot self-verify even holding both privates unless the operator genuinely
   enrolled them as separate roles.
3. **Hash-binding intact.** Verdict `T=0x01 diff_attest_sha3` equals sha3 of the actual K
   attestation, and both attestations' `diff_sha3` equal CI's own independent recomputation. Any
   drift ⇒ RED (the verdict does not describe this diff).
4. **GREEN required on red-line-touching diffs.** If the diff touches a **red-line path** — money /
   orders (P0-A4 saga + reversal), auth, RLS, migrations (the categories Phase 3 maps in `redline.rs`
   beyond Money) — then `verdict == GREEN` is **mandatory** to merge. This is V5-C: independent
   re-execution enforced on red-line diffs (STRATEGIC-VECTORS §V5-C). Non-red-line commits still
   require both signatures + a verdict, but the merge policy for a RED non-red-line diff follows the
   Phase-2 ruling (default: RED verdict blocks; the meter still records it).
5. **Residue present.** Every verdict note MUST contain the exact residue string (T=0x09). A verdict
   without it is malformed ⇒ RED. This makes the honesty statement non-optional and machine-checked.
6. **`redline_touch` honesty.** CI recomputes the red-line-path match and compares to the author's
   signed `redline_touch` bit; if the author signed `0` but CI matches a red-line path, RED (an
   author cannot dodge the GREEN requirement by mis-tagging).

**SCOPE RULE stamp (ARCHITECTURE §0):** this gate is a **canonical-repo DEV-TIME fence**, not a
runtime control. A sovereign hub (M5/M9/M11) MAY fork and drop it. The gate governs the operator's
own build; it does not and cannot bind an autonomous hub. This is written into the job's header
comment so it is never mis-read as global control (matches HYDRA-CONTRADICTION sweep C8).

---

## 6. FalseClaimMeter / claim-latency-anomaly design (V5-B)

**Input:** Phase 1's claim-latency ledger (one entry per commit: `time(diff-landing → GREEN-claim)`)
plus the verifier's own `claim_latency_ms` (T=0x08). **Sink:** the existing `FalseClaimMeter`
(`hermes-kernel/kernel/src/governance.rs:1-25`), which today computes a claimed-vs-verified audit but
is **not fed** by a per-commit latency log (R1-B §V1). Phase 6 connects the ledger to the meter — no
new meter is built.

**Anomaly rule (the documented pattern):** flag a commit for sample-audit when the observed
claim-latency is implausibly small for the diff size — the **52-seconds-GREEN-on-a-1,610-insertion
diff** signature from BRAIN-TOPOLOGY. Concretely, compute a per-commit ratio
`latency_ms / diff_line_count` and raise a flag when it falls below a floor calibrated so the
52s/1610-line exemplar trips it (a human/independent re-execution of a 1600-line diff cannot complete
its suites in 52s; a GREEN emitted that fast is a claim, not a check). The flag is **observation, not
a block** (STRATEGIC-VECTORS §V5-B: "anomalies flagged for sample-audit. Observation, not block") —
it routes the commit to the sample-audit queue and increments the FalseClaimMeter's suspicion count
for the signing identity. F36 (claim-latency anomaly alert, ARCHITECTURE §6) is the alert surface;
this feeds it.

**Two latencies, one meter:** the *author* claim-latency (ledger) detects fast self-green; the
*verifier* `claim_latency_ms` (T=0x08) is a floor witness — a verifier that emitted GREEN faster than
its own suites could physically run is itself anomalous and is flagged. This closes the loop: neither
role can fast-green without leaving a measurable trace in the meter.

---

## 7. Acceptance criteria (numbered checklist — falsifiable)

The phase is DONE when every item is demonstrated (the first three are the mandated RED cases, then
one GREEN, then structural checks):

1. **RED — failing test cannot go GREEN.** A diff containing a deliberately failing test is submitted
   with a valid key_K attestation. The verifier re-executes in the isolated context, the suite goes
   RED, and **no key_V GREEN verdict can be produced** — the merge gate blocks. (Falsifies "claim
   replaces check.")
2. **RED — wrong-key verdict rejected.** A GREEN verdict signed with **key_K** (the author's key, not
   the verifier's) is presented to the merge gate. The gate rejects it on the `key_K ≠ key_V` /
   role-mismatch check (§5.2). Merge blocked.
3. **RED — no verifier signature cannot merge.** A PR carrying only the key_K attestation and **no
   key_V verdict** is blocked by the gate (§5.1). Merge blocked.
4. **GREEN — clean diff merges.** A clean diff obtains a valid key_K attestation, an independent
   key_V GREEN verdict from re-execution, both signatures verify, `key_K ≠ key_V`, hash-binding
   intact, red-line policy satisfied — and the gate goes GREEN. Merge allowed.
5. **Residue present on every verdict.** Cases 1–4 each carry the exact string
   `enforced approximation: identity != person` in the verdict note (T=0x09); a verdict lacking it is
   RED (§5.5).
6. **K ≠ V enforced at load and at gate.** `load_genesis` refuses a `kv-genesis.txt` with fewer than
   two anchors or `pub_K == pub_V`; the gate independently re-checks role distinctness.
7. **Hybrid signatures real.** key_K and key_V signatures verify through the existing `RequireBoth`
   hybrid gate (both Ed25519 and ML-DSA-65 legs); a 1-bit corruption of either leg fails
   verification.
8. **Claim-latency anomaly fires.** A synthetic 52s-GREEN-on-a-1,610-line-diff pattern raises the
   FalseClaimMeter anomaly flag and enqueues a sample-audit (§6).
9. **Phase-3 precondition honored.** No signing path is exercised until Phase 3's dudect/CT gate on
   `mod_l` is GREEN (C4b closed); C3 keygen is feature-gated; `proto-cap/src/lib.rs:12-16` stale
   docstring corrected.
10. **SCOPE RULE stamped.** The merge-gate job header declares it a canonical-repo DEV-TIME fence
    (M5/M9/M11 hub may override), consistent with the HYDRA-CONTRADICTION sweep.

---

## 8. Residual limitation — honest paragraph (identity ≠ person)

This mechanism enforces **identity separation, not person separation**, and it says so on every
verdict. key_K and key_V are two ML-DSA anchors; nothing here guarantees two different *humans* or
two independent *judgments* stand behind them. Today, in a 97.8%-single-authorship repo, the same
operator holds both privates and the same model family may drive both the authoring and (absent an O9
"different model family" ruling) the verifying context. The V1 escape clause is explicit about this:
"identity-separation ≠ person-separation … If a second human reviewer becomes available, they take
the V-role. Until then: A+B is the enforced approximation, logged as such" (STRATEGIC-VECTORS §V1).
The residue line (T=0x09), mandatory and machine-checked, is precisely that log — the system refuses
to *pretend* it has achieved true reviewer independence. **It is still worth building**, for three
falsifiable reasons: (1) it defeats the dominant, documented failure — same-session self-green — by
forcing re-execution in a process that shares no context with the author, so a claim can no longer
substitute for a check; (2) it makes the *cost of a false GREEN* measurable and attributable (the
FalseClaimMeter now has a per-identity, per-commit latency signal, so a 52-second GREEN leaves a
trace instead of vanishing); (3) it is **forward-compatible** — the moment a genuine second party
(human or a decorrelated model family under O9) is available, they simply take the V-role with zero
schema change, upgrading the enforced approximation to real independence. The honest framing is the
feature, not a caveat: this is the cheapest, highest-leverage step from *self-certified* toward
*independently verified*, and it names exactly how far short of true independence it still stands.

---

*Blueprint P06 complete. Scope: V1 + E9. Reuses the finished-and-idle PQ substrate (pq_dsa ACVP,
hybrid_gate, canonical TLV, node_id genesis loader) per R1-B §V1 file:line. Builds the K/V signing +
independent re-execution + merge gate on top of Phase 1's unsigned V5-C harness; blocked on Phase 3's
C4b closure for the hybrid signing path; the isolation bar is a flagged recommendation pending
Phase-2 ruling O9. No code written by this document.*
