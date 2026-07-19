# BLUEPRINT P85 — NTT red-line process remediation (the `--no-verify` A4 hook bypass) (2026-07-19)

> **Standalone PROCESS-REMEDIATION blueprint (bebop-repo `bebop2/core`).** One coherent,
> independently executable unit against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. This is **not** a build-of-new-code unit — it is the
> unit that restores the exact protection a `git commit --no-verify` bypassed, so that a
> technically-excellent crypto commit stops being a *blocked* item and becomes a *reviewable* one.
> Scope source: `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §3/§4.1 (Tier A4), refreshed
> by `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §4. The master ledger judged this scope **ADEQUATE /
> near-executable** (`MASTER-STATUS-LEDGER-2026-07-19.md` §4 item 10): this pass **formalizes** the
> existing recipe and tracks the C3 precondition — it does **not** expand scope.
>
> **One sentence:** commit `986646a` (a correct, exhaustively-proven ML-KEM-768 NTT) was landed with
> `git commit --no-verify`, bypassing all **five** bebop-repo pre-commit gates including the mandatory
> 3-model peer review built precisely for red-line crypto — P85 re-runs the four deterministic gates
> and executes a *real* independent 3-model review over the NTT diff (or records an explicit operator
> retroactive sign-off), which is the only way to lift the quarantine on the entire downstream lane.

---

## VERDICT (stated up front, per session discipline)

**MUST-DO, BLOCKING, and cheap — but gated on one operator decision it shares with C3.** This is the
one live *procedural* emergency of the 2026-07-18→19 wave. The NTT code is the strongest technical
artifact of the day (a complete 0 / 65 536 basis-pair proof, §0.4); its **process** state is RED and
the two must never be conflated. Three things are true simultaneously and all three matter:

1. **Technical status: GREEN.** Exhaustive bit-identity proof vs schoolbook `poly_mul`, non-wired,
   schoolbook path unchanged, 11/11 `pq_kem` tests pass (§0.4). Nothing here questions the math.
2. **Process status: RED / OPEN VIOLATION.** Until P85's DoD is met, `986646a` is a *blocked* item,
   never a *completed* one, in every citation. No wire-in, no Montgomery layer, no dependent work.
3. **This blocks the ENTIRE bebop-side dependency lane, not just the NTT wire-in (§2.4 — the
   load-bearing sequencing fact).** `986646a` now sits at the **base** of the shared branch
   `perf/bus-contention-2026-07-18`, and that tree is *additionally* in a pre-existing **C3 HARD-law
   red** state (ungated keygen, §0.5). Together, **P85 (bypass unremediated) + C3 (HARD-law red)
   freeze every hook-respecting commit on the bebop working branch** — M1, the P76 bus-fix patch,
   P78, P82, P91.1, P93, P92, P94, and the D-9 wire-in all sit behind this gate-0. Resolving P85 + C3
   is the freeze-breaker for the whole lane, co-equal in priority with P75 on the dowiz lane
   (`MASTER-STATUS-LEDGER-2026-07-19.md` §3 finding 2, §3 priority line).

The single operator decision this needs (OD-6 / W3-6): **real 3-model review vs recorded retroactive
sign-off.** Default if unruled: **quarantine holds** — nothing lands on `986646a`. This blueprint
specifies both closure paths fully so whichever the operator picks is turn-key.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from the live tree
> (`/root/bebop-repo`, HEAD `986646a`) **this pass**, not inherited from the synthesis sketch.

### 0.1 The commit and its unpushed, shared-base state

- `git log --oneline -1` → **`986646a feat(pq_kem): re-derive correct ML-KEM-768 NTT alongside
  schoolbook (NOT wired), exhaustively proven bit-identical`** — verified at HEAD of branch
  **`perf/bus-contention-2026-07-18`**.
- `git rev-parse --abbrev-ref --symbolic-full-name @{u}` → **no upstream (unpushed)**. The commit has
  never left the local machine.
- The branch is **shared**: the contention-bench lane (P90) targets the same branch for its bebop-side
  work, so `986646a` is no longer an isolated tip — it is the base other verified work wants to land on
  (`SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §4 fact 2). *Live nuance (honest):* the working tree at this
  pass additionally shows uncommitted `M bebop2/core/src/pq_kem.rs` plus other dirty files and an
  untracked `.worktrees/`; P85 targets the **committed `986646a` tree** (well-defined via git), not the
  dirty working copy — the reviewer works from `git show 986646a` / `git diff <parent>..986646a`.

### 0.2 The five pre-commit gates that were bypassed (read verbatim from `.git/hooks/pre-commit`)

The hook is live (`-rwx--x--x`, 3199 bytes) and runs, in order, **five** gates — a `--no-verify`
commit skips **all five**:

| # | Gate | Script | What it enforces |
|---|---|---|---|
| G1 | Doc-claim verification | `scripts/verify-doc-claims.mjs` | no doc/README statement unbacked by a live probe/test ("Constant Doubt") |
| G2 | Falsifiable-proof guardrail | `scripts/guardrail-falsifiable-proof.mjs` | every test file has a RED/failure path (no tautological/assertion-free tests) |
| G3 | Global Logic Laws (truth gates) | `scripts/logic-gate.mjs` | LOGIC-LAWS.md; RC=1 HARD refuse, RC=2 escalate-to-human (allowed, tracked in `ESCALATIONS.md`) |
| G4 | Logic Laws as practice (real tools) | `scripts/law-hooks.mjs` | HARD laws proven by real tools (clippy/deny/audit/falsifiable) — **this is where C3 fails, §0.5** |
| G5 | **3-model peer review** | `scripts/three-model-review.sh` | builder ≠ reviewer ≠ overlap; no self-certification (operator standing rule 2026-07-11) |

### 0.3 Why G5 exists and why bypassing it on crypto is the worst kind of miss

`scripts/three-model-review.sh` (5073 bytes, dated 2026-07-11) states its own rationale verbatim: *"a
single agent building and reviewing its own work produces false-greens (the §A.3.1 tag was 'green' on a
roundtrip test that shared the same broken path both ways). Independence of the reviewer is the only
reliable antidote."* The gate is **falsifiable, not advisory**: the builder runs `prepare <builder-id>`
to drop `.review/staged.json`; two *other* agents run `attest reviewer <agent> <findings>` and
`attest overlap <agent> <findings>`; the hook refuses the commit unless **both** attestations exist,
each names a **different** agent than the builder and from each other, with **non-empty** findings.

- **`.review/` has NO attestation for `986646a`.** Verified: the newest genuine findings files are
  dated **2026-07-11** (Ed25519/sign work: `findings-reviewer-ed25519.md`, `findings-overlap-ed25519.md`,
  etc.). The only newer file, `.review/staged.json` (2026-07-17), is a **stale P13 delivery-domain
  record** (`"builder": "hermes-p13-builder"`, reviewer `hermes-p13-rev1`, overlap `hermes-p13-ovl`) —
  it is **not** an NTT review and would be consumed/`rm`'d by the next hook-respecting commit regardless.
- **Non-hypothetical value on this exact codebase.** Memory `crypto-safe-first-pass-2026-07-14.md` (B4):
  an independent reviewer *built and ran* an Ed25519 mixed-order SSR-2020 forgery that the pre-fix
  `verify_batch` wrongly accepted and that *passed the unit tests*; it was caught only because a reviewer
  built an actual forgery. "The builder's own tests are green" is precisely the false-green pattern G5
  answers — reviewer independence, not test volume, is the control. The exhaustive 0 / 65 536 proof is
  strong evidence but does not substitute for independence (an exhaustive proof of the *wrong property*
  is still a false-green; see the kernel wrong-ring case, §0.6).

### 0.4 The artifact under review (what a reviewer must actually check — verified in the committed tree)

`git show 986646a:bebop2/core/src/pq_kem.rs` contains the new, non-default-active symbols (line cites
are in the committed blob):

| Symbol | Cite (in `986646a`) | Role |
|---|---|---|
| `const fn brv7(x)` | `:359` | 7-bit bit-reversal for the ζ-table index |
| `const fn modpow_c(base,e,m)` | `:368` | compile-time modular exponentiation |
| `const ZETAS_KEM: [i32;128]` | `:380` | ζ table, `17^{brv7(i)} mod 3329`, built at compile time |
| `fn ntt_fwd_kem(&mut [i32;256])` | `:392` | forward incomplete (7-layer) NTT |
| `fn ntt_inv_kem(&mut [i32;256])` | `:416` | inverse (Gentleman-Sande) + ×128⁻¹ |
| `fn basemul_kem(a0,a1,b0,b1,ζ)` | `:445` | `(a0+a1x)(b0+b1x) mod (x²−ζ)` |
| `fn poly_mul_ntt(a,b)` | `:455` | full ring multiply `intt(basemul(ntt(a),ntt(b)))` |
| 6× `ntt_kem_*` tests | `:996+` | round-trip, negacyclic-wrap, schoolbook-match, **exhaustive basis proof** |

Schoolbook `poly_mul` (`:296`) and the live `keygen`/`encaps`/`decaps` call sites are **unchanged** —
verified. The proof (`ntt_kem_exhaustive_basis_proof`) exploits that both maps are Z_q-bilinear, so
agreement on all 256×256 monomial basis pairs implies agreement on all of `(Z_q²⁵⁶)²` — a *complete*
proof, not a KAT sample (`OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` §3).

### 0.5 The C3 HARD-law red state on the very same tree (the shared precondition)

`scripts/ci-no-ungated-keygen.sh` (the C3 guard, 2026-07-14, invoked as a HARD law via G4
`law-hooks.mjs`) requires `pq_dsa::keygen` **and** `pq_kem::keygen_internal` to be preceded by
`#[cfg(any(test, feature = "dangerous_deterministic", feature = "test_keygen"))]`. On the `986646a`
tree this gate is **RED** — a clean worktree at HEAD with zero crypto edits still trips it
(`SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §2, §4 fact 3). This C3 red is **pre-existing and unrelated**
to the NTT commit, but the `--no-verify` bypass *sailed past it without surfacing it*. **Consequence:**
even after P85's 3-model review is filed, a hook-respecting commit on this tree will still be refused at
G4 until C3 is resolved. So **C3 resolution (OD-3 / W3-3, operator/council-gated) is a co-precondition
of P85's practical goal** — lifting the lane freeze — even though C3 is not itself the NTT's review debt.

### 0.6 The carried flag (NOT fixed here) — the kernel's wrong-ring KEM

`OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` §1.2 + `OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md`:
dowiz's *own* `kernel/src/pq/kem.rs` (a **separate** codebase from bebop2's `pq_kem.rs`) implements the
**cyclic** ring `Z_q[x]/(x²⁵⁶−1)` (complete 8-layer NTT, ROOT=17, `(i+j)%N` mul) with **η1=3** (the
ML-KEM-512 value) while its header claims FIPS-203 / ML-KEM-768. It is internally self-consistent, so its
own round-trip tests pass in the wrong ring. **This is P91's lane** (`P91.0` header defusal + `P91.1`
port, itself gated on P85). P85 only **files it as a downstream link** — it does NOT fix it and must not
ride this remediation.

---

## 1. Prior-art map — this is process machinery, adopt the repo's own gate (standard §2 item 19)

| Prior art (in-repo) | What it is | How P85 uses it — and what it does NOT invent |
|---|---|---|
| `scripts/three-model-review.sh` `prepare`/`attest` | the exact, falsifiable 3-model gate the bypass skipped | **Run it for real** over the NTT diff. P85 invents no new review mechanism — it uses the one already wired into the hook (§0.3). |
| `.review/` attestation records (`findings-reviewer*.md`, `findings-overlap*.md`) | the on-disk format independent reviewers already used for the 2026-07-11 Ed25519 work | **Same format, new subject.** The NTT attestations drop into `.review/` exactly as the Ed25519 ones did — a proven pattern, not a new artifact type. |
| `docs/design/ESCALATIONS.md` | the sanctioned human-arbiter record for G3/G4 RC=2 escalations | **The only other legal exit.** A recorded operator retroactive sign-off is filed here, per the hook's own escalation convention — not a new bypass path. |
| B4 / SSR-2020 forgery precedent (`crypto-safe-first-pass-2026-07-14.md`) | an independent reviewer built a *working forgery* the unit tests missed | **Sets the reviewer's bar:** produce an adversarial input or a proof of impossibility, not a read-and-approve (§4.2). |
| the bebop regression/escalation ledger | permanent, findable record of a resolved gate event | **File the bypass + resolution** so it has a permanent paper trail (§4.4). |

P85 **adds no dependency, no script, no gate.** It is the act of *running* protection that already
exists and was skipped. That is the whole unit.

---

## 2. Scope — what P85 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P85 OWNS

1. **Re-running the four deterministic gates** (G1–G4) against the `986646a` tree, green (or their
   failures fixed in a follow-up commit that itself goes through the hooks) — **except** the C3 sub-part
   of G4, which is the shared operator precondition (§2.3).
2. **Executing G5 (the 3-model review) for real** over the NTT diff: builder `prepare`, an independent
   reviewer attestation, and a third overlap attestation, all in `.review/` per the script contract.
3. **OR** — failing (1)–(2) — **escalating for an explicit operator retroactive sign-off** recorded in
   `docs/design/ESCALATIONS.md` (the only other sanctioned exit for a red-line gate).
4. **Recording the resolution** in the bebop regression/escalation ledger either way, so the bypass has
   a permanent, findable trail.
5. **Naming the C3 co-precondition** (OD-3) and the P91.1 downstream consumer as tracked links — without
   fixing either.

### 2.2 P85 does NOT own (anti-scope)

- **The NTT math / any code change to the NTT.** The artifact is frozen as-is; P85 reviews it, it does
  not edit it. (If the review *finds* a defect, that defect's fix is a new commit through the hooks — a
  successor, not part of P85's remediation act.)
- **Wiring `poly_mul_ntt` into `keygen`/`encaps`/`decaps`.** That is D-9, triple-gated (P82 bench
  evidence AND operator sign-off AND **P85 complete**) — downstream, not in scope.
- **The Montgomery/Barrett follow-up layer.** Same quarantine; not in scope.
- **The C3 keygen-gating fix.** Operator/council-gated crypto work predating this wave (OD-3); P85
  depends on it but does not perform it.
- **The kernel `pq/kem.rs` wrong-ring fix.** P91's lane (§0.6); filed as a link only.
- **Pushing `986646a` to any remote.** Push hygiene is an operator decision (the branch is quarantined
  until P85 closes); flagged, not performed.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** the pre-commit hook + its five scripts (`.git/hooks/pre-commit`,
`scripts/{verify-doc-claims.mjs,guardrail-falsifiable-proof.mjs,logic-gate.mjs,law-hooks.mjs,
three-model-review.sh}`); the `986646a` diff (`git diff <parent>..986646a -- bebop2/core/src/pq_kem.rs`);
`.review/` (attestation sink); `docs/design/ESCALATIONS.md` (sign-off sink); the bebop regression ledger.

**Shared operator precondition:** **C3** (`ci-no-ungated-keygen.sh` red, OD-3) — must be resolved (or an
explicit `--no-verify` ruling recorded) before *any* hook-respecting commit lands on this tree, P85's
follow-ups included.

**Consumers (what unblocks when P85 closes):** D-9 NTT wire-in; the Montgomery follow-up; **P91.1**
(kernel KEM port — its source *is* bebop2's now-reviewed NTT, so it cannot start until P85 clears the
source); and, jointly with C3, the whole hook-respecting bebop lane (M1, P76 bus patch, P78, P82, P93,
P92, P94).

### 2.4 The blocking topology, stated explicitly (the fact the task demands be unmissable)

```
                    ┌─────────────────────────────────────────────┐
   OD-6 (P85 path)  │  986646a  (NTT, --no-verify, review debt)    │  ← P85 clears review debt
   OD-3 (C3 fix) ───┤  tree ALSO C3 HARD-law red (ungated keygen)  │  ← C3 fix clears G4
                    └───────────────────┬─────────────────────────┘
                                        │  BOTH required to commit through hooks
                                        ▼
     ══════════════ ENTIRE bebop hook-respecting lane is FROZEN until both clear ══════════════
        M1 (exporter) · P76 bus patch · P78 · P82 · P91.1 · P93 · P92 · P94 · D-9 wire-in
```

**P85 + C3 = gate-0 of the bebop lane.** This is not "clean up before wire-in"; it is "clean up before
the branch can carry anything else" (`SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §4). Co-highest priority
with dowiz-lane P75 (`MASTER-STATUS-LEDGER-2026-07-19.md` §3).

### 2.5 Honest reconciliation (standard §2 item 6)

Nothing here re-litigates the NTT's correctness or the value of shipping it eventually — the synthesis
verdict (technically GREEN, wire-in gated) is binding and P85 upholds it. P85 also does not weaken the
gate: the *fix for the bypass is to run the gate*, never to lower it. The one philosophical point the
operator may weigh (OD-6): a complete machine proof is unusually strong evidence, so a *recorded
operator retroactive sign-off* is a legitimate exit here in a way it would not be for un-proven crypto —
but that judgment is the operator's, and the review path remains the default (§5.3).

---

## 3. Predefined artifacts & constants — named BEFORE execution (standard §2 item 4)

No new Rust types (this is a process unit). The named, non-magic artifacts are:

```
# The subject under review (frozen)
NTT_COMMIT            = 986646a
NTT_BRANCH            = perf/bus-contention-2026-07-18            # bebop-repo, unpushed
NTT_DIFF              = git diff 986646a~1..986646a -- bebop2/core/src/pq_kem.rs
BUILDER_ID            = <the agent/model that authored 986646a>   # e.g. "opus-p85-ntt-builder"

# The gate scripts (must all be green for a hook-respecting close)
GATES_DETERMINISTIC   = [ verify-doc-claims.mjs, guardrail-falsifiable-proof.mjs,
                          logic-gate.mjs, law-hooks.mjs ]         # G1..G4
GATE_REVIEW           = scripts/three-model-review.sh            # G5

# The 3-model review records (drop into .review/)
REVIEW_STAGE_FILE     = .review/staged.json                      # from `prepare` (overwrites stale P13 record)
REVIEW_FINDINGS       = .review/findings-reviewer-ntt.md,
                        .review/findings-overlap-ntt.md
REVIEWER_ID / OVERLAP_ID  = <two DISTINCT agents, both ≠ BUILDER_ID and ≠ each other>

# The escalation exit (only if the review path is declined)
SIGNOFF_RECORD        = docs/design/ESCALATIONS.md               # operator retroactive sign-off entry

# The permanent trail (either path)
LEDGER_ENTRY          = <bebop regression/escalation ledger>     # P85 resolution record

# The shared precondition (operator-owned, not performed here)
C3_GATE               = scripts/ci-no-ungated-keygen.sh          # OD-3; must clear for G4
```

**Attack surfaces the reviewer MUST target** (named so the review is concrete, not a rubber stamp):
`basemul_kem` adversarial inputs; `ZETAS_KEM` edge indices (0, 63, 64, 127) and the `brv7` mapping;
i64 intermediate-bound re-derivation (max basemul ≈ 3.7e10 ≪ i64::MAX — re-verify, don't trust);
the `128⁻¹ = 3303 mod 3329` inverse scaling in `ntt_inv_kem`; and the *property* the exhaustive test
proves (bit-identity to schoolbook) vs the property it does **not** prove (FIPS-203 wire interop — the
KEM stores coefficient-domain, §0.4 / research §1.1).

---

## 4. Remediation steps — each a spec → check → GREEN, with the adversarial cases named (standard §2 items 2, 3, 5)

Modeled as an event sequence (item 3): `BypassRecorded → GatesReRun → ReviewAttested(or SignOffRecorded)
→ LedgerEntryWritten → QuarantineLifted`. Each step has a falsifier.

### 4.1 R1 — re-run the four deterministic gates (G1–G4) against `986646a`

- **Spec:** from a clean checkout of `986646a`, run `verify-doc-claims.mjs`, `guardrail-falsifiable-proof.mjs`,
  `logic-gate.mjs`, `law-hooks.mjs`. All must exit 0 (or escalate at RC=2 with an `ESCALATIONS.md`
  entry). Any HARD failure (RC=1) is fixed in a **follow-up** commit that itself goes through the hooks —
  the NTT commit is not amended in place (its history stays honest, including the bypass).
- **Check (falsifier):** running each script prints its ✓ line; a RED exit names the failing claim/law.
- **Expected reality:** G1–G3 are expected green (the NTT added only non-default code + falsifiable
  tests + no doc claims to drift). **G4 is expected RED via the C3 sub-gate** (§0.5) — that RED is the
  pre-existing keygen issue, *not* the NTT; it is handed to OD-3, and R1's own DoD is "G1–G3 green + G4
  green-except-C3, with C3 explicitly routed to OD-3."

### 4.2 R2 — execute the 3-model review (G5) for real over the NTT diff

- **Spec:** the builder runs `bash scripts/three-model-review.sh prepare <BUILDER_ID>` (this overwrites
  the stale P13 `staged.json`). A **genuinely independent** reviewer (a different agent/model — e.g.
  `system-breaker`/`security-sentinel` or a decorrelated model) reviews `NTT_DIFF` and runs
  `attest reviewer <REVIEWER_ID> .review/findings-reviewer-ntt.md`; a **third** agent runs
  `attest overlap <OVERLAP_ID> .review/findings-overlap-ntt.md`. The hook then passes G5
  (builder ≠ reviewer ≠ overlap, non-empty findings).
- **The bar (B4 precedent, §0.3):** each attestation must be an **active attempt to break the code** —
  the reviewer tries the §3 attack surfaces (adversarial `basemul_kem` inputs, ζ-edge indices, i64-bound
  re-derivation, the proven-property-vs-claimed-property gap) and either **produces a defect** or
  **argues its impossibility**. A restatement of the builder's tests is a FAIL of this step even if the
  hook mechanically accepts a non-empty string — the reviewer independence, not the file's existence, is
  the control.
- **Adversarial self-check on the review itself (`red_review_is_not_a_rubber_stamp`):** the overlap
  agent's explicit job is to catch a blind spot the reviewer and builder might share (the §A.3.1 Poly1305
  shared-path lesson) — e.g. does the exhaustive test prove the *ring* is ML-KEM's negacyclic ring, or
  only that two implementations of *whatever ring the code uses* agree? (The kernel wrong-ring case,
  §0.6, is the live proof that "two impls agree" ≠ "correct scheme".) The overlap MUST address this.
- **Falsifier:** with no `staged.json` or with reviewer/overlap == builder or empty findings, a
  hook-respecting commit is **refused** at G5 (verified behavior, §0.3) — the review is provably present
  only when the commit is *accepted*.

### 4.3 R3 (alternative to R2) — recorded operator retroactive sign-off

- **Spec:** if the 3-model review path is declined (OD-6 → sign-off), the operator's explicit retroactive
  sign-off for `986646a` is recorded in `docs/design/ESCALATIONS.md` with: the commit hash, the fact of
  the `--no-verify` bypass, the specific gates skipped (G1–G5), the justification (e.g. the complete
  machine proof), and the operator identity/date. This is the hook's own sanctioned RC=2 human-arbiter
  convention applied to a red-line gate.
- **Falsifier:** a grep of `ESCALATIONS.md` for `986646a` returns the entry; absence = quarantine holds.
- **Honest constraint:** R3 satisfies the *review-debt* half only. **C3 (G4/OD-3) is orthogonal** and
  still blocks hook-respecting commits on the tree regardless of R3 — R3 does not substitute for C3.

### 4.4 R4 — write the permanent ledger entry (either path)

- **Spec:** record in the bebop regression/escalation ledger: the bypass event (`986646a`, `--no-verify`,
  five gates skipped), the closure path taken (R2 attestations or R3 sign-off), the C3 co-precondition
  status, and the P91.1 downstream link. This makes the bypass permanently findable so no future session
  re-discovers it as a surprise.
- **Falsifier:** the ledger contains a `986646a` remediation row; a REGRESSION-LEDGER cross-reference
  exists (standard §2 item 17).

### 4.5 R5 — lift the quarantine (the state transition this whole unit exists to enable)

- **Spec:** once R1 (G1–G3 green, G4 routed to C3), R2-or-R3, and R4 hold **and** C3 (OD-3) is resolved,
  the D-9 wire-in precondition "P85 complete" flips true, and the bebop lane freeze lifts (jointly with
  C3). Emit the `QuarantineLifted` event by updating the D-9 trigger and the master ledger P85 row to
  CLOSED.
- **Falsifier:** the D-9 trigger still reads "P85 incomplete" until this step; the master-ledger P85 row
  stays SKETCH/BLOCKED until CLOSED is written with the attestation/sign-off cite.

---

## 5. The two closure paths in full (the decision the operator owns — OD-6)

### 5.1 Path A — real 3-model review (default, strongest)

Restores the *exact* control that was bypassed. Cost: two independent agents actively attacking a
~120-line, fully-specified diff — cheap. Yields a permanent, independent attestation that the NTT is not
just self-proven but adversarially reviewed. This is the path the B4 lesson argues for.

### 5.2 Path B — re-run deterministic gates + recorded operator sign-off

If the operator judges the complete machine proof sufficient and declines a full independent review,
R1 + R3 + R4 close the *procedural* debt with an explicit, recorded human decision. Legitimate here
(and only here) because the artifact carries a *complete* proof of its stated property — but it trades
away independent adversarial review, so it is the weaker path and must be a deliberate operator call,
never a default drift.

### 5.3 What is NOT a closure path

- **Another `--no-verify`.** Bypassing the gate to "fix" a bypass is incoherent; explicitly forbidden.
- **Amending `986646a` to hide the bypass.** History stays honest; the bypass is recorded, then
  remediated.
- **Wiring the NTT "to prove it works".** The proof already exists; wire-in is D-9, downstream of P85.
- **Treating C3 as closed by R3.** C3 is a separate HARD law; only OD-3 clears it.

---

## 6. Definition of Done — falsifiable, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (check) |
|---|---|---|
| D1 | G1–G3 run green on the `986646a` tree | each script exits 0 with its ✓ line; a follow-up commit (if any fix needed) itself passes the hooks |
| D2 | G4 is green **except** the pre-existing C3 sub-gate, which is explicitly routed to OD-3 | `law-hooks.mjs` output; C3 RED is attributed to `ci-no-ungated-keygen.sh`, not the NTT diff |
| D3 | **(Path A)** `.review/` holds a reviewer AND overlap attestation for the NTT diff, each a distinct agent ≠ builder, non-empty, adversarial | `three-model-review.sh` accepts a hook-respecting commit; findings target the §3 surfaces, not a test restatement |
| D3′ | **(Path B, if chosen)** `docs/design/ESCALATIONS.md` holds an explicit operator retroactive sign-off for `986646a` | grep `986646a` in `ESCALATIONS.md` returns the signed entry |
| D4 | the bypass + resolution are in the bebop regression/escalation ledger + REGRESSION-LEDGER | ledger row for `986646a` exists |
| D5 | the D-9 wire-in trigger's "P85 complete" precondition flips true; master-ledger P85 row → CLOSED | D-9 trigger text updated; P85 row cites the attestation/sign-off |
| D6 | the C3 co-precondition (OD-3) status is recorded (resolved, or explicitly ruled) so the lane-unfreeze condition is unambiguous | OD-3 disposition noted alongside D5 |
| D-NOREG | the NTT commit's own 11/11 `pq_kem` tests still pass; schoolbook path untouched | `cargo test -p bebop2-core --lib pq_kem` green on `986646a` |

**Quarantine invariant (holds until D1–D5 all green):** no `poly_mul_ntt` wire-in, no Montgomery layer,
no dependent work on `986646a`, no P91.1 port (its source is this diff).

---

## 7. Cross-cutting obligations (standard §2 items 6–20 — applied honestly, N/A marked where true)

- **Hazard-safety as structure (item 6):** the unsafe state — *an unreviewed red-line crypto commit
  silently becoming a "done" base other work builds on* — is made unreachable by the gate itself: a
  hook-respecting commit on this tree is **impossible** until G1–G5 pass, so the quarantine is enforced
  by the same machinery being restored, not by prose. The one residual reachability (a second
  `--no-verify`) is closed by §5.3 + the permanent ledger entry that makes a repeat immediately visible.
- **Links & memory (item 7):** §8.
- **Schemas / scaling axis (item 8):** N/A — no data schema. The only "scale" is number of gates (fixed
  at 5) and reviewers (fixed at 3); neither grows.
- **Linux discipline (item 9):** **REINFORCES** the existing fail-closed hook chain; **EXTENDS** nothing
  (adds no gate); **DOES-NOT-TRANSFER** — no daemon, no new tool. The move is "run the existing gate,"
  the most Linux-kernel-discipline act available (you do not merge crypto past review).
- **Benchmarks + telemetry (item 10):** N/A for the remediation itself (no hot path). The *downstream*
  D-9 wire-in is bench-gated (P82 KEM bench) — P85 only records that gate, does not run it. (Honest: the
  research already benched nothing here; the NTT's ~100× multiply speedup only matters per-handshake and
  is D-9's measure-first obligation, not P85's.)
- **Isolation / bulkhead (item 11):** the quarantine **is** the bulkhead — the unreviewed artifact is
  fenced from every consumer (D-9, Montgomery, P91.1, the whole lane) until reviewed; its failure mode
  (a latent NTT defect) cannot propagate because nothing may depend on it while quarantined.
- **Mesh awareness (item 12):** N/A — a local commit/review event, node-local, never gossiped.
- **Rollback / self-heal as math (item 13):** **Self-termination** = the hook refuses the commit
  (unrepresentable "committed-but-unreviewed" state on a hook-respecting path), not a supervisor choice.
  **Snapshot re-entry** = if the review finds a defect, the schoolbook `poly_mul` is the untouched valid
  epoch to fall back to (it was never replaced). Self-healing is NOT claimed.
- **Error-propagation / smart index (item 14):** the bug class ("crypto merged without independent
  review") is turned into a **commit-time** refusal by G5 + the C3 HARD law at G4 — already compile/commit
  gates, not runtime surprises. P85's contribution is ensuring they actually run.
- **Living-memory awareness (item 15):** the ledger entry (R4) is the durable, findable record — the
  bypass is written into project memory so its lesson is time-persistent, not a one-session artifact
  (the exact `MASTER-STATUS-LEDGER §0` "untracked docs vanish" lesson, applied to a process event).
- **Tensor/spectral (item 16):** N/A — a review/process unit, no linear-algebra kernel. (The *reviewed
  artifact* is NTT math, but reviewing it is not itself a spectral task; forcing `spectral.rs` here would
  be nonsense.)
- **Regression tracking (item 17):** R4 + the REGRESSION-LEDGER cross-reference; D-NOREG keeps the NTT's
  own suite green.
- **Worker instructions (item 18):** §9.
- **Reuse-first (item 19):** §1 — P85 adds nothing; it runs the existing gate. Extension was considered
  and rejected: no new machinery is needed to remediate a skipped gate.
- **Hermetic principles (item 20):** §7.1 below.

### 7.1 Hermetic principles honored (item 20 — load-bearing only)

- **Cause & Effect:** every landed crypto artifact must have a *cause* that is an independent review or a
  recorded human sign-off — nothing is trusted because "its own tests are green" (that is correlation, the
  false-green). P85 restores the causal chain the bypass broke.
- **Polarity / no-middle:** a commit is either fully-gated-and-reviewed or it is refused — there is no
  "mostly reviewed" middle state a `--no-verify` could smuggle. P85 removes the illegitimate middle the
  bypass created.
- **Correspondence:** "as above, so below" — the protection the operator declared (the 3-model rule,
  2026-07-11) must *correspond* to what actually ran on the wire; the bypass broke correspondence between
  declared policy and executed reality, and P85 restores it.

---

## 8. Links to docs & memory (standard §2 item 7)

**Scope / status sources:**
- `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §3 (Tier A4), §4.1 (the P85 recipe), §7 D9′.
- `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §4 (live status refresh), §2 (C3-blocks-verified-work), §6 (OD W3-3/W3-6).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P85 / I1 rows), §3 (bebop lane gate-0 finding), §4 item 10, §5 OD-3/OD-6.
- `docs/research/OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` (the artifact under review; §3 proof, §1.2 kernel flag).
- `docs/research/OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md` (the P91 downstream flag).

**Gate machinery (bebop-repo, the bypassed protection):**
- `.git/hooks/pre-commit` (the five-gate chain, §0.2), `scripts/three-model-review.sh` (G5 contract, §0.3),
  `scripts/ci-no-ungated-keygen.sh` (C3, §0.5), `scripts/{verify-doc-claims,guardrail-falsifiable-proof,
  logic-gate,law-hooks}.mjs` (G1–G4), `.review/` (attestation sink), `docs/design/ESCALATIONS.md` (sign-off sink).

**Memory:**
- `crypto-safe-first-pass-2026-07-14.md` (B4 / SSR-2020 — why independent review beats test volume; the review bar).
- `never-bypass-human-gates-2026-06-29.md` (blanket permission ≠ per-change approval — why OD-6 is the operator's).
- `worktree-remote-push-collision-avoidance-2026-07-18.md` (push hygiene for the quarantined branch, flagged not performed).

**Standard:** `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract). **Format precedent:**
`BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.

---

## 9. Instructions for the executing worker (zero prior context — standard §2 item 18)

This is a **bebop-repo** unit (NOT dowiz). Execute in `/root/bebop-repo`, branch
`perf/bus-contention-2026-07-18`, against commit `986646a`.

1. **First, get the OD-6 ruling** (operator): Path A (real 3-model review) or Path B (recorded sign-off).
   Default = Path A. Do not proceed on assumption — this is a red-line gate the operator owns.
2. **Run R1** (§4.1): `verify-doc-claims.mjs`, `guardrail-falsifiable-proof.mjs`, `logic-gate.mjs`,
   `law-hooks.mjs` on a clean `986646a` checkout. Expect G1–G3 green; expect G4 RED **only** via C3
   (`ci-no-ungated-keygen.sh`) — attribute that RED to OD-3, do NOT touch the keygen gating here.
3. **Path A → run R2** (§4.2): `three-model-review.sh prepare <BUILDER_ID>`; route the diff
   (`git diff 986646a~1..986646a -- bebop2/core/src/pq_kem.rs`) to an **independent** reviewer and a
   **third** overlap agent (`system-breaker`/`security-sentinel` or decorrelated models). Their mandate:
   **attack the §3 surfaces and produce a defect or a proof of impossibility** — not restate the tests.
   File `findings-reviewer-ntt.md` + `findings-overlap-ntt.md` via `attest`.
   **Path B → run R3** (§4.3): file the operator's retroactive sign-off in `docs/design/ESCALATIONS.md`.
4. **Run R4** (§4.4): write the bypass + resolution into the bebop regression/escalation ledger +
   REGRESSION-LEDGER.
5. **Run R5** (§4.5): flip the D-9 "P85 complete" precondition and mark the master-ledger P85 row CLOSED,
   citing the attestation (Path A) or sign-off (Path B).
6. **Do NOT** wire the NTT, add Montgomery, start P91.1, amend `986646a`, or `--no-verify` anything.
7. **Remember the lane:** P85 closing + **C3 (OD-3) resolving** together unfreeze M1 / P76 patch / P78 /
   P82 / P93 / P92 / P94 / D-9. Neither alone lifts the freeze — say so in the close-out.

---

*This blueprint writes ZERO product code, touches no git branches, pushes nothing. It specifies the
remediation of a process violation; execution is a separate, operator-gated act.*
