# BLUEPRINT — Phase 2: CANON REPAIR + OPERATOR DECISION BATCH (2026-07-16)

> **Type:** decision memo + canon-diff proposal. **Not code.** **Anchors:** D5, D8, V4, V6, E35, E42, E53, E55.
> **Depends on:** — (Wave 0). **Parallel-safe with:** Phase 1, 3, 4, 5.
> **Sources:** ARCHITECTURE.md (HEAD `574f05604`), R2-MERGED-PHASE-ROADMAP §1/§3/§4, R1-A…E,
> DECISIONS.md (root, 2026-07-12, AUTHORITATIVE), live greps re-run this session.
>
> **§0 STANDING NOTE — this blueprint does NOT edit canon.** ARCHITECTURE.md and
> STRATEGIC-VECTORS-LOCKED are governed by their own rule (line 3): *"Single source of truth. Merge,
> never append."* Merging is the operator's act (or a delegated-back merge task). Everything below is
> a **proposal**: exact before/after diffs (§1), a decision docket the operator rules on (§2), the
> ADR-020 draft (§3), and the V4 closure template (§4). Nothing here is applied. Where a ruling is
> **load-bearing** (O1 D5/D8, O3 F44, O4 F48) the recommendation is explicitly overridable and the
> phase **cannot close** until the operator rules or dates a deferral. Where a ruling is
> **mechanical** (rewordings, D2 phrasing, S7/S8 split) a default is recommended for adoption.

---

## 1. CANON CORRECTIONS — before/after diffs against ARCHITECTURE.md

Each block quotes the **exact current line** (verified at HEAD this session) and the proposed
replacement. The operator merges these by hand or delegates the merge.

### C-1. S3 stale license/scrub line — `ARCHITECTURE.md:41`

**BEFORE (current):**
```
- Secrets (S3): systemd EnvFile + NEVER in-repo + gitleaks. (ADR-020 LICENSE mismatch: Apache-2.0 vs AGPLv3; force-push scrub BLOCKED red-line; EUTM pending.)
```
**AFTER (proposed):**
```
- Secrets (S3, D3-legal): systemd EnvFile + NEVER in-repo + gitleaks. (LICENSE = AGPLv3 since ac1caba40 (2026-07-14), on origin/main; NOTICE+DCO+TRADEMARK.md+MANIFESTO in tree. Scrub RESOLVED: H8 runbook CLOSED 2026-07-13 + P10 decision 2026-07-16 (origin already at scrubbed tip 6c7212b5; SHA-rewrite declined as redundant). D3 remaining = EUTM filing + explicit public-flip go + one all-origin-refs gitleaks sweep. See ADR-020.)
```
**Rationale:** the "Apache-2.0 vs AGPLv3" mismatch and "scrub BLOCKED red-line" are both **false at
HEAD** (R1-E §0.1/§0.2, re-verified: `/root/dowiz/LICENSE` is 660-line AGPLv3). Canon may not carry
known-false statements. Adds the `D3-legal` label back (see C-4).

### C-2. §8 honest-gaps ADR-020 bullet — `ARCHITECTURE.md:128`

**BEFORE (current):**
```
- ADR-020: LICENSE Apache-2.0 vs AGPLv3; force-push scrub BLOCKED (red-line); EUTM pending.
```
**AFTER (proposed):**
```
- ADR-020 (docs/adr/ADR-020-oss-license-tm-dco.md): LICENSE = AGPLv3 (LANDED ac1caba40); scrub RESOLVED (H8 CLOSED + P10 2026-07-16). OPEN operator gates only: EUTM brand+filing (O16), public-flip go (O17, one-way door), all-origin-refs gitleaks sweep. CONTRIBUTING.md:17 DCO-CI claim is FALSE until Phase 1 lands the DCO job.
```

### C-3. §7 anchor total — `ARCHITECTURE.md:123` (depends on O1 ruling)

**BEFORE (current):**
```
## 7. Total locked: V1-6 + D1-8 + S1-9 + E1-62 + M1-12 + F1-50 = 147 anchors.
```
**AFTER — variant A (operator DEFINES D5/D8, ratifies E10≡E36 as alias):**
```
## 7. Total locked: V1-6 + D1-8 + S1-9 + E1-62 + M1-12 + F1-50 = 147 nominal IDs (146 distinct; E36 is an alias of E10). All D1..D8 now defined (D5/D8 ratified this phase — see ADR/decision-log).
```
**AFTER — variant B (operator RENUMBERS: strike D5/D8):**
```
## 7. Total locked: V1-6 + D1,2,3,4,6,7 + S1-9 + E1-62 + M1-12 + F1-50 = 145 nominal IDs (144 distinct; E36 alias of E10). D-series renumbered — D5/D8 never existed; every "147" corrected.
```
**Note (concrete, load-bearing):** `"147"` appears in **exactly two tracked files** —
`ARCHITECTURE.md:123` and `STRATEGIC-VECTORS-LOCKED-2026-07-16.md:124`. Both must move together to
whatever count O1/O2 ratify. This is the entire blast radius of the count-consistency done-test.

### C-4. Restore dropped D3/D4 labels (mesh-pivot rewrite `0d1935d96` dropped them)

Confirmed via `git show 4aa71b725:docs/design/ARCHITECTURE.md` (R1-E): the prior revision labelled
**D3 = AGPLv3+TM legal** and **D4 = "dowiz UI = deterministic physics/math wasm"**. The
`0d1935d96` rewrite kept the content but dropped the labels. Proposed re-anchoring (minimal, in-line):

- `ARCHITECTURE.md:36` (Legal line) — append `(D3)` after "AGPLv3 + TRADEMARK".
- `ARCHITECTURE.md:41` (S3) — already carries `D3-legal` via C-1.
- `ARCHITECTURE.md:116` (F47 "Demo = wasm physics/math render") and the S-series UI reference — add
  `(D4)` where the deterministic-wasm-UI property is asserted, so D4 is enumerable again.

### C-5. Apache-2.0 grep — IMPORTANT REFINEMENT to the done-test

The done-test as written (`grep -n "Apache-2.0" ARCHITECTURE.md → 0`) is **too strong**. Four hits
exist; only **two are stale** (`:41`, `:128`, fixed by C-1/C-2). The other two —
`:7` and `:34`, both *"vLLM 86k★ Apache-2.0"* — are **correct facts** (vLLM's real license) and MUST
remain. Corrected done-test: **`grep -n "Apache-2.0 vs AGPLv3" ARCHITECTURE.md → 0`** (the stale
*mismatch* phrasing is gone; the legitimate vLLM license mentions survive).

### C-6. Minor stale-fact relays (fold into any canon touch, not blocking)

- Memory/prose "kernel 152 tests" → **337 passed / 0 failed** (R1-C, `cargo test --offline`). Canon
  should never hard-code a test count; if it must, cite the command, not the number.
- E10≡E36: record E36 as an **alias** of E10 in the accounting, not a second anchor (O2).

---

## 2. THE DECISION DOCKET (O1–O19, Descartes-quadrant format)

Format per ARCHITECTURE.md §6: **SIT** (possible/impossible) · **NOW** · **FUTURE** · **PRO** · **CON**
· **REC** (recommendation — operator may override). **LOAD-BEARING** items carry no silent pick.

### O1 — D5/D8: define or renumber `[LOAD-BEARING — no silent pick]` (Phase 2, blocks all "147" arithmetic)
- **SIT:** possible either way. D5/D8 have **zero occurrences in any revision** of either canon doc
  (R1-E §D5/D8, re-verified across `8180b03eb`/`4aa71b725`/`0d1935d96`/`574f05604`). The count "8 (D)"
  was overstated by 2 at inception (`4aa71b725`) and carried forward — a live self-certification
  instance (claim replaced check).
- **NOW:** every "147" is off by two undefined IDs. **FUTURE:** left open, the arithmetic corrupts
  every downstream doc that cites the total.
- **Option A — DEFINE from DECISIONS.md** (root, operator-confirmed 2026-07-12): D5 := *roles +
  adapters bridge law* (3 node roles owner/courier/customer; NOSTR/ActivityPub/MCP as bridges never
  core transport; every bridged msg ML-DSA/ML-KEM-enveloped first); D8 := *plan precedence,
  newest-outranks-older*. **PRO:** both themes are otherwise absent from the canon D-series and are
  already ratified elsewhere. **CON — the sharp one:** DECISIONS.md is a **different, colliding**
  D-numbering (its D3 = DTN transport, its D4 = PQ-protocol) — importing its D5/D8 into the canon
  scheme yields canon-D3(legal)≠DECISIONS-D3(transport) while canon-D5=DECISIONS-D5. The operator
  must accept that partial collision explicitly.
- **Option B — RENUMBER:** declare the D-series = the 6 real anchors (D1,D2,D3,D4,D6,D7), strike
  D5/D8, correct the total to **145** everywhere (2 files, C-3 variant B). **PRO:** no invented
  content, no scheme collision. **CON:** D-series is non-contiguous (a reader wonders where D5/D8
  went — mitigate with a one-line "D5/D8 retired 2026-07-16, never defined" note).
- **REC (overridable):** **Option A**, because the two DECISIONS.md themes are genuinely load-bearing
  and currently unlabelled in canon — but **only if** the operator is comfortable stamping the
  scheme-collision note. If not, Option B is the honest fallback. **This phase cannot close without
  A or B chosen (or a dated deferral recorded in canon §8).**

### O2 — E10 ≡ E36: ratify merge or write the distinction `[cheap — REC to adopt]`
- **SIT:** as-is. STRATEGIC-VECTORS:89 (`E10 ML-DSA hybrid`) and :98 (`E36-40: ML-DSA hybrid`) are
  the identical three words, no distinguishing text (R1-A #7, re-verified). Substance BUILT
  (`RequireBoth`/`ClassicalUntilPqAudit`, `hybrid_gate.rs:24-34`).
- **NOW:** 147 double-counts one anchor. **FUTURE:** Phase 3 bookkeeping inherits the ambiguity.
- **PRO (merge):** truthful count. **CON:** if E36 was *meant* to carry distinct scope (e.g. E36=hybrid
  KEM vs E10=hybrid signature) that intent is lost — but no evidence it was.
- **REC:** **ratify as one anchor; E36 = documented alias of E10.** Count becomes 146 distinct / 147
  nominal. If the operator recalls a real distinction, write it instead; absent that, alias.

### O3 — F44 dispute/arbitration DECART `[LOAD-BEARING — no silent pick]` (Phase 2 ruling; blocks Phase 14)
- **SIT:** the only spec (`bebop-repo/docs/design/fable-protocol-2026-07-11/F2-dispute-arbitration.md`)
  is a real 6-state fail-closed machine (`OPEN→EVIDENCE→AUTO_ARBITRATE→ESCALATE→JURY→SETTLE`,
  timeouts, default-refund invariant, RED test) but carries **two canon contradictions** (R1-D §F44):
  (1) it maps JURY onto `reputation.rs` — but reputation-trust is **permanently rejected** (V2, M12
  capability-only, NO-COURIER-SCORING CI gate); (2) `PROTOCOL-CENTRALIZATION-MAP:141` says "use
  UMA/Kleros, don't build" — an external dep at the trust boundary, violating M6 (zero protocol deps).
- **NOW:** Phase 14 cannot start without a chosen arbiter model. **FUTURE:** picking wrong bakes a
  reputation system or an external oracle into the trust boundary — both irreversible-ish.
- **Option A — operator-gated arbiter capability:** arbitration is a signed, red-line-scoped
  **capability** (M12-consistent), issued per-dispute, revocable. **PRO:** no reputation, no external
  dep, reuses proto-cap. **CON:** centralizes arbitration on capability-holders the operator anoints.
- **Option B — Schelling-point voting among staked capability-holders:** no reputation weighting; vote
  weight = staked capability, tie→default-refund. **PRO:** decentralized, M12-clean. **CON:** more to
  build; sybil/stake mechanics need their own falsifier.
- **REC:** **do not pick here.** Both A and B are canon-consistent; the choice is a genuine
  architecture value-call (centralized-but-simple vs decentralized-but-complex) reserved for the
  operator. What this phase *does* rule: **F2's jury→reputation and UMA/Kleros legs are struck as
  canon-violating** regardless of A/B. Record the strike now; date the A/B choice into Phase 14's gate.

### O4 — F48 per-hub graph-wiki merge semantics `[LOAD-BEARING — no silent pick]` (Phase 2 ruling; blocks Phase 14)
- **SIT:** single-hub knowledge substrate BUILT (`living_knowledge.rs` PRIMARY recall@5=1.0), zero
  replication. Merge policy for divergent per-hub entries is **unspecified** (F48 CON = only
  "dedup/merge cost"). A **dormant `crdt-fence` pre-commit guard exists in bebop** whose intent
  (fence CRDTs *out* vs fence them *correct*) must be read before choosing (R1-D §F48).
- **Option A — content-address-only:** sync = signed envelopes carrying sha3-addressed deltas;
  identical content converges free via BlockStore dedup; divergent content = union, no merge.
  **PRO:** trivial, zero-dep, deterministic. **CON:** no last-writer-wins / no field-level merge —
  two hubs editing "the same" logical node keep both.
- **Option B — CRDT (e.g. OR-Set / LWW-register):** true convergent merge. **PRO:** clean merges.
  **CON:** new merge machinery, possible new dep (DECART), and the `crdt-fence` guard may *forbid*
  exactly this — must resolve the guard's intent first.
- **REC:** **do not pick here** — but flag that the `crdt-fence` guard's intent is the tie-breaker and
  **must be resolved before Phase 14.** If the fence forbids CRDTs, Option A is forced. Read the
  guard, then the operator rules.

### O5 — D2 / iroh inversion `[cheap — REC to adopt]` (Phase 2 pre-ruling; executed as Phase 9 DECART)
- **SIT:** canon says "iroh-QUIC primary, quinn fallback"; **code is inverted** — iroh is deliberately
  NOT a dependency (offline build + `ed25519-dalek 3.0.0-rc.0` pin conflict), quinn is the only
  carrier (R1-A #5). F20 ("quinn if iroh down") is **impossible as written** — no iroh to fall back
  from.
- **REC:** **amend canon to "quinn primary; iroh promoted via DECART on a named network-unlock
  trigger" and amend F20/E34 with it** (proposed trigger: same `cargo add`-needs-network gate as the
  wgpu GPU-unlock, O18). The actual DECART doc lands in Phase 9; Phase 2 only ratifies the direction
  so the canon stops lying. Low stakes — mechanical.

### O6 — E35 "3-tier locality": define or strike `[REC to strike-pending-def]`
- **SIT:** zero code hits; **no definition of the three tiers exists anywhere** in canon (R1-A #8).
  Unbuildable and unfalsifiable as written.
- **NOW:** an anchor that cannot be tested. **FUTURE:** if defined (candidate tiers: edge / hub /
  region, or on-device / hub-local / mesh-global) the impl attaches to Phase 9.
- **REC:** **strike-pending-definition** — mark E35 `UNDEFINED (canon gap)` in §8 now; the operator
  either writes the three tiers in one sentence (then it lands in Phase 9) or it stays struck and the
  count drops by one more. Do not leave it silently "locked". Low stakes but must not be papered over.

### O7 — E1/F41 "hub-ring" semantics `[cheap — REC to adopt]` (blocks Phase 13 design)
- **SIT:** "hub-ring" is two words with no spec. The only concrete reading in the corpus (SYNTHESIZED
  §3) is a **consistent-hash order/region-ownership overlay on top of the existing HRW courier
  hashing**. A literal star/central hub would **contradict M7 (no-SPOF)**.
- **REC:** **ratify the consistent-hash ownership-ring reading** (deterministic order/region
  ownership, not a physical topology). Add a one-line canon gloss on E1/F41. Mechanical — the
  alternative reading is already ruled out by M7.

### O8 — F10 sub-hub max-depth-cap value `[operator sets a number; REC a default]` (blocks Phase 15)
- **SIT:** sub-hub recursion needs a hard depth cap (F10 LOCK); no value exists in canon.
- **NOW:** without a number Phase 15's refusal test has no threshold. **FUTURE:** too low kills
  legitimate nesting; too high permits depth-blowup DoS.
- **REC:** **default cap = 8** (depth carried in the child capability; refuse at `depth > 8`). Rationale:
  generous for any real agent-spawns-subagent nesting, still bounded well below stack/resource blowup;
  operator overrides with any integer. Cheap — it's a single constant the operator picks.

### O9 — V1-B context-isolation bar `[operator sets one sentence; REC a default]` (blocks Phase 6)
- **SIT:** V1-B's "independent context" verifier is unpinned — fresh worktree? separate machine?
  different model family? (R1-B §2.5). The identity≠person escape is already logged; the *minimum
  isolation bar* is not.
- **REC (one sentence of canon):** *"V1-B independence bar = fresh clone/worktree + re-executed test
  suites in a separate process, signed by key_V (K≠V); different model family is preferred-not-required
  and logged as an enforced approximation."* Rationale: worktree+re-exec is achievable today (Phase
  1's V5-C harness is exactly this); a hard "separate machine / different model" bar would strand
  Phase 6 on infra it doesn't have. Operator may raise the bar.

### O10 — S7 vs S8 split `[cheap — REC to adopt]` (Phase 8 documentation)
- **SIT:** canon never defines S7 and S8 separately — only the joint line "Observability
  (S7/S8/D7/M8)" (R1-C §S7/S8).
- **REC:** **S7 = tracing (spans/events), S8 = typed numeric metrics (per-process CPU, GPU-as-Option).**
  Add both as one-line canon entries. Mechanical; matches the Phase 8 build split exactly.

### O11 — M12/F25 replay-ledger bound `[REC to adopt]` (blocks Phase 9)
- **SIT:** the nonce/replay ledger is **per-gate-instance, in-process** (`Mutex<HashSet>`, bounded
  `MAX_SEEN_NONCES=1<<20`, verify-then-record) — explicitly not distributed (R1-A F25).
- **PRO (bless the bound):** simple, fast, no persistence dep. **CON:** a process restart forgets seen
  nonces within the still-valid expiry window — a bounded replay window.
- **REC:** **bless the in-process bound as canon-correct for a single hub**, and add the mitigation
  note: *"expiry windows must be short relative to restart cadence; persistence is a hub's optional
  choice (M5), not a protocol requirement."* Low stakes — the expiry gate already fences the window.

### O12 — "BD" expansion `[cheap — REC to adopt]` (blocks Phase 14 E16)
- **SIT:** "BD" in "spectral+BD memory" (E8/E16) is **never expanded** anywhere in canon or arcs
  (R1-B §2.2). Unfalsifiable until named.
- **REC:** **BD := "Bounded Diffusion"** (the diffusion-search retrieval layer,
  `kernel/src/retrieval/diffusion.rs`, the personalized-PageRank/heat-diffusion recall organ). This is
  the only "BD"-shaped organ in the code. If the operator meant something else (e.g. "Block-Diagonal"
  from the spectral solver), substitute — but Bounded Diffusion is the grounded reading. Cheap.

### O13 — M9 subtree-kill semantics `[REC to sequence]` (blocks Phase 10/15)
- **SIT:** M9 says the operator may hard-kill a "hub/subtree", but **no hub hierarchy exists** until
  F10 sub-hubs land (Phase 15). "Subtree" is undefinable pre-F10 (R1-B §2.3).
- **REC:** **sequence, don't redefine.** Rule now: *"M9 kill applies to a single hub today; subtree-kill
  is defined once F10 sub-hub hierarchy exists (Phase 15) as 'kill a hub and all capability-descendant
  sub-hubs by depth-scoped revocation'."* Add the forward-reference to canon so M9 isn't read as
  already-supporting a tree it can't address. Mechanical sequencing note.

### O14 — E13–E20 per-item numbering ratification `[cheap — REC to adopt]` (bookkeeping)
- **SIT:** the per-item Descartes text for E13–E20 is "in session dialog", not on disk (R1-B §2.1).
  R1-B inferred a mapping.
- **REC:** **ratify the inferred mapping:** E13/E14 = LLM-infra (self-host llama.cpp/vLLM GOAL +
  managed-advisory-until-GPU); E15 = harmonic+kelly adaptive tiering; E16 = spectral+BD memory; E17 =
  MCP; E18 = per-agent capability tokens; E19 = TokenBucket; E20 = paired-debate. Write these eight
  one-liners into STRATEGIC-VECTORS so the mapping is on disk. Cheap; unblocks bookkeeping.

### O15 — Cheap canon rewordings (bundle) `[cheap — REC to adopt all]` (doc truth)
Five mechanical edits, no stakes; adopt as a batch:
- **E39:** "signed event_log" → **"hash-chained (SHA3-256) event log fed only by hybrid-verified
  frames"** — there is no single signed-log type; signing is one layer up at `SignedFrame`/`HybridGate`
  (R1-A E39). Prevents a reader from hunting a missing primitive.
- **F21:** amend "ML-DSA holds" → **"ML-DSA holds *once delegation links + genesis anchors are hybrid*
  (today they are Ed25519-only — the PQ frame leg is vacuous against the F21 adversary until Phase 3
  closes this)."** Names the real trust-root PQ requirement (R1-A #4).
- **F20:** amend with O5 — "quinn primary; iroh via DECART on network-unlock trigger."
- **E42 re-anchor:** repoint E42 from the empty `/root/bebop-repo/delivery/` placeholder to
  **`bebop2/delivery-domain` + `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`** (R1-D §E42).
  Either populate or `rm` the empty `telegram-pending/` dir. **REC:** delete the empty dir, re-anchor
  the text.
- **M3-vs-M6 beacon boundary:** add one note — *"a network-touching QRNG beacon client lives OUTSIDE
  the zero-dep wire boundary; entropy is injected as caller-supplied bytes"* (R1-A #6). Resolves the
  apparent M3/M6 tension without weakening M6.

### O16 — EUTM brand + filing `[LOAD-BEARING operator — Phase 18, relayed for visibility]`
- **SIT:** "DeliveryOS" is a weak descriptive mark; "dowiz" is the stronger mark (ADR-020 §6.6,
  R1-E E59). EUIPO e-filing = €850 one class, Fast Track free, SME-Fund 2026 voucher may reimburse.
- **REC:** **not this phase's ruling** — belongs to Phase 18. Relayed here so the docket is complete.
  Operator picks the brand BEFORE e-filing. No Phase-2 action beyond recording it in ADR-020 as OPEN.

### O17 — Public-flip go `[LOAD-BEARING operator — Phase 18, one-way door]`
- **SIT:** standing rule — the public flip is a one-way door, **never autonomous**.
- **REC:** **not this phase's ruling.** Recorded in ADR-020 as an OPEN operator gate. Phase 2 only
  ensures the canon around it (C-1/C-2) is truthful so the flip decision rests on facts.

### O18 — GPU-unlock `[external trigger — relayed]`
- **SIT:** wgpu is uncached offline; unlock = network `cargo add wgpu` (W21 ceiling). Gates Phase 17
  video + Phase 15 E13-execution.
- **REC:** **no Phase-2 ruling** — environment/operator trigger. Relayed so the dependency is visible;
  O5's iroh trigger should reference the same unlock mechanism for consistency.

### O19 — I-FINAL proof home `[REC to adopt]` (Phase 13, blocks F46 full closure)
- **SIT:** SYNTHESIZED P0-A5 cites `eqc-proofs/lambda_max_of_d.rs` — **this file/dir does not exist
  anywhere in dowiz** (R1-C §2.5); eqc proofs are emitted ephemerally in CI.
- **REC:** **home the I-FINAL quorum-intersection proof in `bebop-repo`'s consensus path** (it is a
  mesh-consensus property, not a dowiz product-math property), emitted through the same eqc pattern.
  If the operator prefers dowiz `tools/eqc`, note that dowiz has no persistent proof dir today, so
  bebop is the lower-friction home. Low-medium stakes; recommend and let Phase 13 confirm.

---

## 3. ADR-020 DRAFT OUTLINE — `docs/adr/ADR-020-oss-license-tm-dco.md`

The decision is referenced in 5+ docs and MANIFESTO C6 but **no `docs/adr/ADR-020*.md` exists**
(verified: `find -iname '*ADR-020*'` → 0; the `docs/adr/` dir has 0001–0009 + named ADRs but no 020).
Proposed structure (write as a real ADR, matching the repo's existing ADR prose style):

- **Title:** ADR-020 — Open-Source License, Trademark & DCO Policy.
- **Status:** Accepted (license + DCO + NOTICE landed `ac1caba40`); OPEN operator gates: EUTM (O16),
  public-flip go (O17), all-origin-refs gitleaks sweep.
- **Context:** sole-copyright repo (~97.8% single authorship → relicense is legally sound; Apache-2.0→
  AGPLv3 is one-way compatible; the one dependabot commit is non-creative). Prior secrets incident
  (2026-07-03) rotated + scrubbed; H8 runbook CLOSED 2026-07-13; P10 decision 2026-07-16 declined
  SHA-rewrite as redundant.
- **Decision:** (1) **AGPLv3** for canonical code; (2) **TRADEMARK.md** brand leash (protocol + runtime
  free per M11 — TM is not a mesh control; a hub MAY fork code, drop brand, keep protocol); (3) **DCO
  1.1** sign-off required — enforced by the Phase-1 CI job (until then CONTRIBUTING.md:17 is a false
  claim, flagged); (4) **NOTICE + MANIFESTO** in tree; (5) per-tool MIT carve-outs
  (`async-spool`, `native-spa-server`) documented in NOTICE, or converted to AGPL — operator's call
  (record kernel `Cargo.toml` gains `license = "AGPL-3.0-or-later"`).
- **Consequences:** copyleft protects the commons; TM protects the brand; DCO gives provenance.
  Public-flip remains operator-gated (one-way). EUTM brand decision (dowiz > DeliveryOS) precedes filing.
- **Supersedes/relates:** MANIFESTO C6; supersedes the stale ARCHITECTURE §8/S3 lines (C-1/C-2);
  relates to P10-OSS-READINESS-AUDIT (gates 1–3 now stale, superseded by newest evidence per D8).

---

## 4. V4 — CLOSURE-CRITERION TEMPLATE + split-track (codify + retrofit)

V4 is **practiced, not codified** (R1-E §V4): `origin/main` = frozen anchor, canonical stack on
feature branches, main-merge = operator gate — but no repo file defines the split-track or a standard
closure criterion. Proposed canon addition (one section) + the reusable template:

**Split-track law (proposed canon text):**
> Two tracks. **Stable** = product + ADR-020-prep, gated by V3 CI, merges to `main` only by operator
> gate. **Experimental** = kernel-growth / self-development (PRIMARY per operator directive
> 2026-07-13), lives on feature branches, **never merges to `main` without an explicit promotion
> ruling.** Promotion = operator act, recorded with a closure criterion.

**Closure-criterion template (every arc carries these three fields):**
```
### <arc name> — <status>
- DONE-WHEN:      <falsifiable predicate — a grep/test/command that returns a definite yes/no>
- EVIDENCE:       <the artifact that proves it — commit SHA, test count from `cargo test`, CI link>
- STRAND-IF:      <the condition under which this arc is archived/stranded rather than finished>
```
**Retrofit onto currently-active arcs** (exemplars — the FSM arc is the model, "DONE 2026-07-14
`99c7698f`"):
- *FSM graph-analysis* — DONE-WHEN: `has_cycle/μ/topo/reachable/ρ` tests green; EVIDENCE: `99c7698f`,
  kernel suite; STRAND-IF: n/a (closed).
- *HK-05/HK-09 routing* — DONE-WHEN: same task classified `Complex` routes differently than `Simple`
  (Phase 5 done-test); EVIDENCE: `governance.sh` calls + `bucket` column; STRAND-IF: GPU-unlock never
  arrives AND managed-advisory is ruled permanent.
- *Math-first architecture* — DONE-WHEN: S0..S7 invariants each bound to a passing eqc proof;
  EVIDENCE: eqc CI job green; STRAND-IF: spectral-operator foundation fails a falsifiable bench.
- *Knowledge Spine* — DONE-WHEN: two hubs exchange wiki deltas with no central authority (Phase 14);
  EVIDENCE: two-hub delta test; STRAND-IF: O4 rules content-address-only AND cross-hub divergence
  proves unmergeable in practice.

**V6 metaphor discipline (audit result + rule).** Rule: every "emergent/swarm/organism/self-heal/
unbounded/anarchy" occurrence in canon must sit **within 3 lines of a named computed criterion**
(SLEM, escape-mass, ρ, Lyapunov Σx², recall@k, drift-gate) or be reworded to "designed coordination".
**Audit result (this session, `grep -niE` over ARCHITECTURE.md):** **6 violations, none adjacent to a
computed criterion** — lines **20** (M11 "Living-organism … UNBOUNDED"), **68** (F7 "pure anarchy …
experiment"), **70** (F9 "self-heal"), **71** (F10 "emergent"), **117** (F48 "emergent knowledge"),
**119** (F50 "Living-organism … emergent"). Proposed fixes: anchor each to a criterion —
e.g. F10 "emergent" → "emergent (bounded by max-depth-cap O8 = 8)"; F48 "emergent knowledge" →
"convergent-by-content-address knowledge (dedup criterion)"; M11/F50 "living-organism unbounded" →
"unbounded except the M9 kill-switch + noether Σx² floor gate (`self_mod.rs`)". Either reword in place
or add the adjacent criterion; the audit above is the falsifiable checklist for the done-test.

**E53 (rsa-triage → standard suspension/waiver template).** The `innovate:` marker at
`kernel/Cargo.toml:31` (RUSTSEC-2023-0071, named owner + checkable revisit condition: *"`cargo tree
-i rsa` shows a real path OR a patched rsa release ships"*) is already the exemplar. **Codify it as
THE reusable form** (proposed one-pager `docs/design/SUSPENSION-WAIVER-TEMPLATE.md`): every future
gate-suspension/vuln-waiver MUST carry `{what, why-suspended, named-owner, falsifiable-revisit-trigger,
date}`. Phase 1's `deny.toml` waivers consume this form.

**E55 (Manifesto).** **Confirmed BUILT** (`/root/dowiz/MANIFESTO.md`, C1–C13; R1-E E55). Light-touch
action: ensure it's linked from ADR-020 and README; no rebuild needed.

---

## 5. FALSIFIABLE DONE-TEST FOR PHASE 2 (what "merged" means)

1. `grep -n "Apache-2.0 vs AGPLv3" docs/design/ARCHITECTURE.md` → **0** (C-1/C-2 merged; the two
   legitimate vLLM "Apache-2.0" mentions at :7/:34 remain — see C-5).
2. Both `"147"` sites (`ARCHITECTURE.md:123`, `STRATEGIC-VECTORS:124`) reflect the **O1/O2-ratified
   count** and agree with each other.
3. D-series **enumerable**: D1..D8 all defined (O1-A) OR renumbered to D1,2,3,4,6,7 everywhere (O1-B);
   D3/D4 labels restored (C-4).
4. Each of O1–O19 has a **written ruling or an explicit dated deferral** in canon §8 (O16/O17/O18 may
   defer to Phase 18/external with a date).
5. Every emergent/swarm/organism hit in canon sits within 3 lines of a named computed criterion (the
   6-line audit checklist in §4 all cleared).
6. `docs/adr/ADR-020-oss-license-tm-dco.md` **exists** as a real ADR (§3).
7. The closure-criterion template exists and **every active arc** carries DONE-WHEN + EVIDENCE +
   STRAND-IF.
8. E42 re-anchored (empty `delivery/` dir deleted or populated); E53 template codified; E55 link
   confirmed.

---

## 6 — Planning-protocol completion appendix (2026-07-17, decorrelated pass)

> Independent verifier pass. **Per this task's special rule: this pass does NOT resolve or recommend
> rulings on any O1–O19 item beyond what §2 already proposes.** It verifies evidence, checks
> citations against the live tree, and applies the 2Q/Anu-Ananke checks to the DOCKET DOCUMENT itself,
> not to the merits of any individual O-item. Verdict: **DEEPENED, content still
> BLOCKED-ON-OPERATOR-DECISION** (§2's own framing — this pass changes that status for zero items).

### (i) Citation-verification results

**Major correction — §3's central claim is STALE, in the good direction.** §3 states *"no
`docs/adr/ADR-020*.md` exists (verified: `find -iname '*ADR-020*'` → 0)."* This is no longer true:
**`docs/adr/0020-oss-license-tm-dco.md` now exists** (following the repo's own `NNNN-name.md`
convention used by 0001-0009, not the `ADR-020-name.md` form the blueprint's outline sketch used —
so a literal `find -iname '*ADR-020*'` would still print 0, and Done-test §5 item #6 as *literally
worded* would still fail even though the deliverable it's checking for is real; the done-test's
filename pattern needs updating to `*0020*` or `*0020-oss*`). Read in full, the landed ADR:
- Follows §3's proposed structure closely (Status/Context/Decision/Consequences, the same five
  decision points) and explicitly cites `BLUEPRINT-P02-canon-repair-operator-decisions.md §3` as its
  source in its own "What this ADR closes" section — direct confirmation this blueprint's draft
  outline was used, not superseded.
- Is honest about what it does NOT close: it states plainly that `ARCHITECTURE.md`'s stale S3/§8 lines
  (C-1/C-2) remain **unmerged** ("this ADR does not touch `ARCHITECTURE.md` itself"), and that
  `CONTRIBUTING.md:17`'s DCO claim is still false pending Phase 1's `dco-check` job.
- Confirms independently, re-verified live by the ADR's own author: `LICENSE` is 660-line AGPLv3
  (matches §3's citation exactly), the two MIT tool crates are still unflipped, `kernel/Cargo.toml`
  still has no `license` field — i.e. every OTHER §1/§5 done-test this pass re-checked is still
  correctly described as open.

**Re-verified, unchanged (not stale):** `ARCHITECTURE.md:41` and `:128` still read verbatim as C-1/C-2
quote them (the "Apache-2.0 vs AGPLv3 mismatch... force-push scrub BLOCKED" line is still live);
`ARCHITECTURE.md:123` and `STRATEGIC-VECTORS-LOCKED-2026-07-16.md:124` both still say "= 147 anchors"
(C-3's "exactly two tracked files" claim holds); grep for `D5`/`D8` as literal tokens in either canon
file still returns zero (O1's premise holds); `kernel/Cargo.toml:31`'s rsa `innovate:` marker is
byte-accurate at the cited line. So Done-test items #1-#5 and #7 in §5 remain genuinely unmet — the
canon-diffs are proposed, not merged, exactly as §0's standing note says.

**A citation-precision issue, not a staleness issue.** O2 cites `hybrid_gate.rs:24-34` for
`RequireBoth`/`ClassicalUntilPqAudit` — the lines are byte-accurate, but the file is
`/root/bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs`, a **different git repository** than the one
this blueprint's canon lives in. A reader in `/root/dowiz` following this citation literally (as a
relative or repo-root path) will not find it. Worth a one-line repo-qualifier in the citation.

**New evidence relevant to O4 (not a ruling — reporting a fact the docket's own text asks for).** O4
names "the `crdt-fence` guard's intent... must be resolved before choosing" as its tie-breaker. The
live guard (`/root/bebop-repo/scripts/ci-crdt-fence.sh`) is scoped: it fails a build only when a crate
*whose source touches* `order_machine|money|ledger|claim_machine|MeshEvent|assert_transition` also
depends on `automerge`/`cr-sqlite`. It does **not** blanket-forbid CRDTs — a wiki/knowledge crate with
no money/order-state coupling is outside its fence. A sibling document already reached the same reading
(`BLUEPRINT-P14-dispute-escrow-graph-wiki.md:94,250,314`: "the `crdt-fence` guard's intent is now
concrete... permitted for the wiki"). This is offered as evidence for whoever rules O4, not as this
pass overriding the "do not pick here" instruction — Option A vs B is still the operator's call; what
changes is that the "must resolve the guard's intent first" precondition O4 itself names now has an
answer on record, in a place O4's own text doesn't point to yet.

### (ii) DECART

**No DECART owed by this blueprint's own text.** P02 is a canon-repair/decision-docket document; it
adds no new dependency, crate, or external service itself. Where a downstream choice could imply one
(O4 Option B's "possible new dep (DECART)" for a CRDT merge crate), the blueprint already correctly
defers it rather than deciding — exactly the discipline the Detailed Planning Protocol asks for
(DECART inline *when a choice is made*; flagging a future DECART obligation when it is deliberately
not made is the correct alternative, not a gap).

### (iii) 2-question doubt audit (of the docket document itself)

**Q1 — least confident about (concrete):**
1. I did not re-derive the "97.8% single authorship" figure ADR-0020 §Context cites (nor did §3
   originally) — took it as carried from R1-E without an independent `git shortlog -sn` re-count.
2. I did not check whether STRATEGIC-VECTORS-LOCKED-2026-07-16.md's other cited lines (beyond 89/98/124,
   which I did check) still match — O10, O11, O12, O13's citations to specific line ranges in that file
   were not individually re-verified this pass.
3. O19's claim that "eqc proofs are emitted ephemerally in CI" — I confirmed the eqc CI job exists
   (`ci.yml`'s `eqc-proofs` job, still Python+sympy, unchanged) but did not confirm no proof artifact
   is ever persisted anywhere (e.g. as a CI artifact) — "ephemeral" is a slightly stronger claim than I
   independently checked.
4. The DECISIONS.md D-numbering collision O1 names (canon-D3=legal vs DECISIONS-D3=DTN transport) — I
   spot-read DECISIONS.md's D1/D2 but did not read its D3-D8 to independently confirm the exact collision
   shape O1 describes; I am relying on O1's own characterization.
5. I did not verify O16/O17/O18's "relayed, not ruled here" framing against Phase 18's actual blueprint
   text (whether Phase 18 in fact restates and owns these, closing the loop O2 opens) — out of scope for
   my assigned files, flagged rather than silently assumed consistent.
6. Whether the ADR-0020 filename mismatch (`0020-oss-license-tm-dco.md` vs the done-test's literal
   `ADR-020-oss-license-tm-dco.md` pattern) is itself worth a correction to §5's done-test wording, or
   whether the done-test's grep was always meant loosely — I judged it a real gap (a literal `grep`/`find`
   as written would false-negative) but did not find a stated intent either way.

**Q2 — biggest thing this pass might be missing:** the docket's own "cannot close without a ruling"
framing for O1/O3/O4 is honest, but the ADR-0020 discovery reveals a *pattern* worth naming: **this
docket's mechanical items (O2, O5-O16 marked "cheap — REC to adopt") have a real risk of silently
executing piecemeal**, the same way ADR-0020 partially executed §3 without the operator having
formally "ruled" on §2's docket as a whole. That is not necessarily bad — mechanical items were always
meant to ship on the recommended default — but it means "the docket is still open" (this appendix's
verdict) is a document-level truth that individual line items can quietly stop being true one at a
time, with no single event that marks the docket "closed." Nothing in §5's done-test currently
distinguishes "fully closed" from "closed except D5/D8/F44/F48" — a state this repo may already be
approaching.

### (iv) Anu & Ananke check

**Anu.** Every load-bearing "cannot close without X" claim in §2 (O1, O3, O4) is correctly marked
as such and none is silently resolved by this pass, per the special rule. The one place Anu is worth
naming a gap: O4's "must resolve the guard's intent first" precondition is now *answerable from
evidence already sitting in this repo* (the guard script + BLUEPRINT-P14's independent reading) — §2's
own text does not yet point to that evidence, so a future reader ruling O4 could easily not know it
exists and re-derive it from scratch, or worse, rule without it. Recorded above in (i) precisely so
that gap is closed by citation, not by this pass overriding the operator.

**Ananke.** The docket's structure is honest about what depends on a future reader: §5's done-test #4
("each of O1–O19 has a written ruling or an explicit dated deferral") is the correct falsifiable
mechanism — a grep over canon §8 either finds all 19 or it doesn't. What does NOT yet survive on
structure alone: nothing currently checks that a *partial* execution (ADR-0020 landing while §2's
ruling is still open) gets recorded as such rather than being mistaken for full closure — this appendix
had to manually reconcile "the artifact exists" against "the docket says BLOCKED" by reading both. A
cheap structural fix (not built here, flagged for the Phase-2 implementer): have ADR-0020 (or a
successor edit to it) carry a literal `BLUEPRINT-P02: OPEN — O1,O3,O4 unresolved` line the way
`docs/adr/README.md`'s honest-dating convention already asks ADRs to self-date — so the next reader of
the ADR alone, without cross-referencing this blueprint, sees the same "not fully closed" truth this
pass had to reconstruct by hand.

---

*Blueprint P02 complete. This is a proposal, not an edit. ARCHITECTURE.md and STRATEGIC-VECTORS remain
untouched by this document per their "merge, never append" rule — the operator (or a delegated merge
task) applies §1's diffs, rules O1–O19 in §2, and lands §3/§4 as new files. Load-bearing rulings
(O1 D5/D8, O3 F44, O4 F48) are left to the operator by design; all mechanical items carry a
recommended default the operator may override.*
