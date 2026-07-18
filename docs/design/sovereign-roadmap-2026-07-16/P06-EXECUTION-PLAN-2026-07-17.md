# P06 EXECUTION PLAN — live-verified, wave-sequenced (2026-07-17)

> Wraps `BLUEPRINT-P06-v1-split-identity-verifier.md` (this directory, written 2026-07-16) the same
> way `HERMETIC-REMEDIATION-PLAN.md` wrapped `BLUEPRINT-H1..H4`: the blueprint is the design, unchanged;
> this document re-verifies its claims against today's live ground truth, resolves the cross-repo
> architecture question it left implicit, and sequences the build into waves.
> **Planning artifact. Code changes named below are tracked separately (see §2's Wave -1 note — that
> wave is already committed as of this document).**
> Branch: `feat/p06-split-identity-verifier` (worktree `/root/dowiz-p06-verifier`, branched from
> `feat/harness-llm-backend` @ `cc3d5c916`). Companion repo work: `fix/c4b-mod-l-constant-time`
> (worktree `/root/bebop-c4b-fix`, branched from bebop-repo `feat/verification-harness` @ `397b8cd`).

---

## §0 — Why this document exists

BLUEPRINT-P06 is already execution-grade (keygen ceremony §2, diff-signer §3, verifier §4, CI gate
§5, FalseClaimMeter wiring §6, 10 falsifiable acceptance criteria §7). Nothing in this document
revises that design. Three things changed or were left open since it was written, and this document
is where they get resolved rather than silently assumed:

1. **The blueprint's own hard precondition — "Phase 6 MUST NOT begin signing until Phase 3's
   dudect/CT gate is GREEN on mod_l" — was checked live today, found still failing, and closed in
   this session** (§1). This is the one item that gated the whole phase from starting at all.
2. **The blueprint never states which repo the keygen/signer/verifier code lives in.** It cites
   bebop2 primitives (`pq_dsa.rs`, `hybrid_gate.rs`, `node_id.rs`) throughout, but the CI merge-gate
   (§5) is a `dowiz/.github/workflows/ci.yml` job. These are two separate git repos with no cargo
   path dependency between them. §2 resolves this using a precedent already set elsewhere in this
   session's work, not a new invention.
3. **O9 (the verifier's isolation bar) is still an open operator decision**, exactly as the blueprint
   flagged. §3 carries the blueprint's own recommended default forward as a DECART rather than
   re-deciding it.

---

## §1 — Live re-verification of BLUEPRINT-P06's evidence (2026-07-17)

Re-checked against the live trees, not carried forward from the blueprint's 2026-07-16 text:

| Claim (BLUEPRINT-P06 §0/§1) | 2026-07-16 (blueprint text) | 2026-07-17 (checked live, this session) |
|---|---|---|
| `mod_l`'s dudect/CT gate | Failing — `sign.rs:712-723` documents the residual secret-bit branch, "NOT fixed here" | **CLOSED.** `bebop2/core/src/sign.rs` `mod_l` rewritten fixed-32-byte, branch-free (`ct_add_be32`/`ct_sub_be32`/`fe_cselect`). New `mod_l_op_count_is_constant` (call-count invariant, proven non-vacuous by reintroducing the exact prior branch and confirming it goes RED — 512 vs 1024 add calls — while the RFC 8032 KAT still passes GREEN) + `mod_l_reduces_known_values`. 249 bebop2-core tests green, full workspace `cargo test --workspace` exit 0. Committed on `fix/c4b-mod-l-constant-time` (worktree `/root/bebop-c4b-fix`), not yet merged to a roadmap branch — that merge is an operator action, named in §2. |
| ML-DSA-65, hybrid gate, canonical TLV, genesis loader "sit finished and idle" | Asserted from R1-B §V1 | **Re-confirmed live**: `bebop2/core/src/pq_dsa.rs`, `bebop2/proto-cap/src/hybrid_gate.rs`, `bebop2/proto-cap/src/node_id.rs` all present, all still covered by the same 249/782-test green baseline this session's workspace run touched. Not re-audited line-by-line (not this document's job) but nothing in this session's work altered them. |
| Phase 1 (unsigned V5-C re-exec harness + claim-latency ledger) — hard dependency | Listed as a precondition | **DONE**, verified in the prior conversation turn this session: `tools/ci-truth` (native Rust) implements `claim-latency` and `v5c-reexec` subcommands, wired into `dowiz/.github/workflows/ci.yml`'s `claim-latency-ledger` and `v5c-reexec` jobs. `tools/ci-truth/src/main.rs:423` currently emits the exact placeholder P06 is meant to replace: `"signed":false` / `"note":"UNSIGNED (Phase 1); Phase 6 wraps this runner with ML-DSA key_K/key_V signatures"`. This is the literal insertion point for P06's signature wrapping (§2 below), found by grep, not assumed. |
| C6b (test fixtures co-derive both hybrid legs) | Open, test-only, low-risk | **Still open**, confirmed via `BLUEPRINT-P03` §1.3/§3.3 (not touched this session — out of scope for the `mod_l` fix, which only closes the item P06's own text names). Not a P06 blocker per the blueprint's own dependency wording ("Phase 3's dudect/CT gate... on mod_l" — C6b isn't that gate), but flagged here so a future reader doesn't assume "Phase 3 fully closed." |
| Phase 3 (PQ Trust-Root Hardening) as a whole phase | Listed as a dependency | **Partially open** (M2 canonical-KEM decision, C6b) — but P06 depends on Phase 3 for exactly one reason per its own §0/§1: the hybrid signing path's constant-time gate. That gate is `mod_l` (now closed). The KEM-stack dual-authority issue (M2) is `pq_kem.rs`, never touched by `sign`/`pq_dsa` — P06 signs with ML-DSA-65 (`pq_dsa.rs`) and Ed25519 (`sign.rs`), not KEM. Verified by reading BLUEPRINT-P06 §1's own file citations: no `pq_kem` reference anywhere in it. **Conclusion: P06's specific hard precondition is satisfied; the rest of Phase 3 remains independently open and does not gate P06.** |

**Net effect of this section: the one thing that made P06 un-startable is closed. Everything else the
blueprint already knew about (O9, C6b, Phase 3's other items) is unchanged and was already correctly
scoped as non-blocking or explicitly-flagged-open.**

---

## §2 — Resolving the unstated cross-repo architecture (bebop2 ↔ dowiz)

BLUEPRINT-P06 cites bebop2 primitives throughout (§1, §2, §3, §4) but specifies the merge-gate as a
`dowiz/.github/workflows/ci.yml` job (§5). dowiz and bebop-repo are separate git repositories with
independent `Cargo.lock`s and no path dependency between them (confirmed: `grep -r "bebop2" dowiz/*/Cargo.toml`
= zero hits). The blueprint does not say how a dowiz CI job verifies a bebop2-issued signature — this
is a real gap, not a stylistic omission, and this document resolves it using a precedent **already
set in this exact session's other work**, not a new design decision:

> `feat/agentic-mesh-protocol-2026-07-17`'s B1 `AgentBridge` (commit `f30189262`, kernel 403 tests
> pass) hit the identical problem — "dowiz kernel does not link bebop2's proto-cap directly (separate
> repos)" — and resolved it with a `SignatureVerifier` **trait seam**
> (`kernel/src/ports/agent/cap.rs:82-95`): a pluggable interface with `classical_public`/
> `sign_classical`/`verify_classical`/`pq_public`/`sign_pq`/`verify_pq`, injectable with either a real
> crypto backend or a test double, decoupling the *port* from bebop2 without a cargo dependency.

**CORRECTION (found re-verifying this section live, not carried forward from the first draft):** B1's
concrete implementation of that trait, `RefSigner` (`cap.rs:97-165`), is **not** a verified-parity
clone of bebop2's real Ed25519/ML-DSA-65 — its own doc comment says so directly: *"NOT production
crypto — a SHA3 commitment scheme... Production replaces this with the real bebop2 Ed25519 +
ML-DSA-65 verifier."* It is a deliberately simpler placeholder (XOR-masked SHA3 commitments) used to
exercise B1's admission logic in tests, never checked byte-for-byte against `hybrid_gate.rs`'s actual
verification order. **What B1 actually establishes as precedent is the trait SHAPE (a pluggable
verification seam avoiding a cross-repo cargo dependency), not a working cryptographic implementation
P06 can reuse as-is.** This is a materially different, larger task than the first draft of this
section implied — flagged here rather than left as a wrong "already solved" claim (see revised §4 Q1).

**P06 adopts the seam SHAPE, but needs REAL crypto behind it — a genuinely new build item:**

- **bebop-repo side** (where signing happens — private keys never leave here): a new `v1-ceremony`
  binary/module implementing BLUEPRINT-P06 §2 (keygen), §3 (diff-signer). Lives in bebop2, reuses
  `pq_dsa`/`hybrid_gate`/`node_id`/`revocation` directly (no seam needed — same repo).
- **dowiz side** (where verification + the merge gate happen): a new `tools/ci-truth` submodule (or
  sibling crate, matching the existing `tools/ci-truth` Rust-native convention) implementing
  BLUEPRINT-P06 §4 (independent-verifier) behind a `SignatureVerifier`-shaped trait (B1's interface
  shape, reused), but backed by a **real, from-scratch Ed25519 + ML-DSA-65 *verify-only* implementation**
  — not signing (no private key material ever needs to exist on the verifier side), just the public
  math, which is the well-specified, KAT-testable half (RFC 8032 §5.1.7 for Ed25519 verify, FIPS-204
  for ML-DSA-65 verify) — matching this repo's own "zero external crates, from scratch, KAT-green"
  convention already used for `pq_dsa.rs`/`sign.rs`. This is genuinely new code (Wave 0 scope, sized
  accordingly — not a thin adapter), and never by shelling out to a bebop2 binary (that would be a
  supply-chain/build coupling BLUEPRINT-P06 never asked for and M6 zero-dependency discipline would
  flag).
- **Cross-repo trust anchor**: `config/kv-genesis.txt` (BLUEPRINT-P06 §2) must exist **identically**
  in both repos (or in a location both can read — e.g. committed to dowiz only, since dowiz is where
  the gate enforces it, with bebop2's ceremony tool taking the anchor file path as a CLI argument
  rather than assuming a fixed in-repo location). This is a **new decision this document is making**,
  flagged as such: BLUEPRINT-P06 §2 step 3 says anchors are "written to `config/kv-genesis.txt`"
  without naming a repo. **Resolution: dowiz is canonical** (it's where the gate lives and where
  `origin/main` is the trust root per `ROADMAP-GROUND-TRUTH-2026-07-14.md`), bebop2's ceremony tool
  takes `--genesis-file <path>` pointing at a checked-out copy of dowiz's `config/kv-genesis.txt`.

**Honest gap this creates, not papered over — sharper than the first draft of this section stated:**
a real dowiz-side Ed25519/ML-DSA-65 verifier is a second, independent codebase that must agree with
bebop2's `hybrid_gate.rs` byte-for-byte forever — exactly the "dual-authority" hazard BLUEPRINT-P03
(§1.4) already names as a real failure class elsewhere in this project (the three parallel ML-KEM
stacks). **Unlike what the first draft claimed, B1 does not already demonstrate a solved parity test**
— its `RefSigner` is intentionally non-cryptographic, so there is no existing byte-for-byte agreement
to point to. Required mitigation, now correctly scoped as new work: a cross-repo parity test — sign a
fixture message with bebop2's real `hybrid_gate`/`pq_dsa`/`sign`, verify it with dowiz's new
from-scratch verifier, assert both PASS and FAIL cases agree (including the standard RFC/FIPS known-
answer test vectors both sides can independently check against) — checked into whichever repo can run
both (likely bebop2, since it can build the fixture; dowiz's CI job then re-verifies the same fixture
independently as its own test). This parity test is Wave 0 work (below) and is now understood to be
substantial, not a formality.

---

## §3 — DECART: O9 isolation-bar default (carrying the blueprint's recommendation forward)

BLUEPRINT-P06 §4 already filed this as "recommendation only, pending Phase-2 ruling O9" — this
section does not re-decide it, it records that the recommended default is what Wave 1 will build
against, per the same "recommended-default, operator-can-override" pattern BLUEPRINT-P02 already
established for its own O-decisions.

| Criterion | Fresh worktree + fresh process + different model family (recommended) | Separate physical machine | Same-process re-check |
|---|---|---|---|
| Defeats same-session self-certification | Yes — verifier shares no memory/context with author | Yes, more strongly | **No** — this is the exact failure mode V1 exists to close |
| Standing infrastructure cost | Zero (worktrees are already the pattern this whole session uses) | Requires provisioning a second host | Zero, but worthless |
| Auditable | Yes — `context_descriptor` (T=0x06) records worktree path hash, PID, model-family id | Yes, with a hostname/attestation field added | N/A |
| Fits M6 (zero-dependency, single-host reality) | Yes | No — new standing infra | Yes |

**DECISION (provisional, operator-overridable per O9):** proceed with fresh-worktree + different
model family, exactly as BLUEPRINT-P06 §4 recommended. If O9 rules otherwise, only the gate's
acceptance predicate on `context_descriptor` (T=0x06) changes — the schema already carries the field
(BLUEPRINT-P06 §4's own text), so this is not a re-design, just a predicate swap.

**Probe (honest case against):** "different model family" for the verifier's rationale step is
real decorrelation for judgment calls, but the RED/GREEN verdict itself is deterministic (test
pass/fail) — the model-family diversity mainly buys something on the free-text `rationale` field, not
on the machine-checkable parts of the gate (§5 items 1-6 are pure logic, no model judgment involved).
Worth naming so nobody overstates what O9 actually protects.

---

## §4 — The 2-question doubt audit, applied to THIS plan

**Q1 — least confident about (concrete):**

1. **RESOLVED, not just flagged — and the resolution changes Wave 0's real size.** This item started
   as "I didn't re-verify B1's reference implementation agrees with `hybrid_gate.rs`." It got checked
   (`kernel/src/ports/agent/cap.rs:80-165` in the agentic-mesh worktree, live read this session) and
   the finding is sharper than "might have drifted": B1's `RefSigner` was **never** cryptographically
   equivalent to bebop2's real verifier — its own doc comment says so ("NOT production crypto... a SHA3
   commitment scheme"). §2 above is corrected accordingly. The residual uncertainty now is different:
   whether a from-scratch dowiz-side Ed25519/ML-DSA-65 *verify-only* implementation is actually the
   right call versus reconsidering a controlled cross-repo dependency (e.g. vendoring bebop2's
   `pq_dsa`/`sign` verify functions as a git-pinned path dependency, accepting the coupling M6 usually
   avoids, in exchange for zero dual-authority risk). This document did not evaluate that alternative
   — a real DECART gap for whoever starts Wave 0, not silently resolved by picking "build fresh."
2. **The `kv-genesis.txt` canonical-repo decision (§2) is a real, non-trivial choice this document
   made, not one BLUEPRINT-P06 specified.** An operator could reasonably prefer bebop2 as canonical
   instead (it's where the private keys and ceremony live) — I chose dowiz because the gate lives
   there, but the blueprint itself is silent, so this is this document's judgment call, flagged as
   such, not blueprint-derived.
3. **§1's claim that Phase 3's other open items (M2, C6b) don't gate P06 rests on one grep** (no
   `pq_kem` reference in BLUEPRINT-P06) — a real check, but a single negative grep is weaker evidence
   than the file:line citations the rest of this document relies on.
4. **The dudect-vs-op-count substitution** (the C4b fix used a deterministic call-count CT proof,
   not the wall-clock dudect-style statistical test BLUEPRINT-P03 §3.2 explicitly names) is a
   conscious deviation, justified by this file's own stated preference for deterministic proofs
   (`sign.rs` comment: "Deterministic constant-time proof (no flaky wall-clock timing)") — but it is
   a different concrete artifact than what P03's text says, and no dudect-style test was added
   alongside it. If an operator wants literal dudect coverage as well (timing-based, not just
   op-count), that is still open.

**Q2 — the biggest thing this plan might be missing:** this document resolves the cross-repo
question and the C4b blocker, but it does **not** address who runs the keygen ceremony operationally
(BLUEPRINT-P06 §2 says "operator-run, one-time" — this plan doesn't schedule that human action, only
notes it's a precondition for Wave 1). Without the ceremony actually running, Wave 0's scaffolding
(TLV types, CI gate skeleton, `SignatureVerifier` parity test) can be built and tested against
synthetic fixture keys, but the real `kv-genesis.txt` with real anchors cannot exist until the
operator does this — named here so Wave 1 doesn't silently stall waiting for an action nobody
scheduled.

---

## §5 — Anu (logic) & Ananke (organization) check

**Anu.** The one hard, load-bearing dependency this plan had to check — C4b's closure — was verified
by running the actual test suite (249 + workspace-wide green), not asserted from the blueprint's
text. The cross-repo architecture gap (§2) is resolved by citing a real precedent (B1's
`SignatureVerifier` seam) that was itself built and tested this session, not invented fresh. Where
this plan makes a genuine new call not derivable from either blueprint (the `kv-genesis.txt`
canonical-repo choice, §2), it says so explicitly rather than presenting it as blueprint-derived.

**Ananke.** What survives without this document being remembered: BLUEPRINT-P06's own 10 falsifiable
acceptance criteria (§7) and the `mod_l` fix's own tests (`mod_l_op_count_is_constant`,
`mod_l_reduces_known_values`, both committed with the code they guard). What does **not** survive on
structure alone: the `kv-genesis.txt` canonical-repo decision and the dudect-vs-op-count
substitution note — both are judgment calls recorded only here. A future reader implementing Wave 0
without reading this file could put the genesis file in the wrong repo or assume dudect coverage
exists when it doesn't; flagged so that gap is a known, owned debt, not a silent assumption.

---

## Wave plan (summary)

- **Wave -1 — C4b closure.** DONE this session. `fix/c4b-mod-l-constant-time` (`/root/bebop-c4b-fix`),
  awaiting operator merge into a roadmap branch (not this document's call to make).
- **Wave 0 — dowiz-side scaffolding, no real signing yet (parallel-safe, no crypto keys needed):**
  TLV type definitions (§3/§4's `DiffAttestation`/`Verdict` structs, behind whatever feature gate
  keeps them inert without real keys), the `SignatureVerifier` reference implementation + its parity
  test against bebop2 fixtures (§2), the `v1-verifier-gate` CI job skeleton (§5, logic only —
  hash-binding/role-checks/residue-string checks can all be unit-tested against synthetic fixture
  signatures without a real ceremony), and wiring Phase-1's `tools/ci-truth` ledger into
  `FalseClaimMeter` (§6, already partially fed per the Wave-1 P08 work from the hermetic-remediation
  arc — verify overlap before duplicating).
- **Wave 1 — real signing (blocked on the operator running the keygen ceremony, §4 Q2):** keygen
  ceremony execution, diff-signer wired into the actual commit flow, verifier running for real against
  live commits, full CI gate enforcement per BLUEPRINT-P06 §7's 10 acceptance criteria.

*Sources: BLUEPRINT-P06-v1-split-identity-verifier.md (unchanged); BLUEPRINT-P03-pq-trust-root-hardening.md
§1.3/§1.4/§3.2/§3.3; AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md + commit `f30189262` (B1 SignatureVerifier
seam precedent); `tools/ci-truth/src/main.rs:423` (live grep, the P06 insertion point);
`bebop2/core/src/sign.rs` (live, post-fix, this session). No product code in dowiz was written by
this document — only the C4b fix (a separate repo, already committed) and this plan.*
