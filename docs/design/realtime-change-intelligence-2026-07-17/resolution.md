# RCI — Resolution (Triadic Council RESOLVE, round 2)

> Author: system-architect subagent, 2026-07-17. Companion to `proposal.md` (revised in this
> round), `breaker-findings.md` (Lamach), `counsel-opinion.md` (Counsel).
> Verdict vocabulary: **FIX** (design changed in proposal.md) / **ACCEPT-RISK** (owner +
> rationale) / **DEFER-FLAG** (named trigger, MISSING until trigger fires).
> Facts re-verified live before resolving: `tools/loop-signals/*.py` = 2 files;
> `event_log.rs:204,239,310` `set_tip` is plain last-writer-wins, no compare-and-swap.

---

## 0. Headline: the decision changed. v1 = Option D′, not Option C.

The four HIGH findings are not survivable by patching Option C. Resolving them honestly
forced a re-derivation, and the re-derivation converged with Counsel's steel-man:

- **H1's honest fix** is not "add CAS" (new concurrency machinery on a substrate whose
  documented invariant is single-writer, `event_log.rs:18-19`) — it is to remove the
  multi-producer push chain entirely and make ingestion **pull-based, single-writer,
  lock-serialized**.
- **H2's honest fix** is not "pin head during revert" — it is to notice that once git is
  the authority, the compensator apparatus solves a problem that no longer exists:
  **rollback of a derived cache is re-derivation**, a pure function, idempotent by
  definition. The digest test then proves exactly what it can prove (determinism), and the
  withdrawn claim ("idempotent + safe under concurrent writes") is no longer needed.
- The decisive observation: **round-1 Option C already contained its own refutation.**
  Proposal §7 (chain-corruption row) declared "git is ground truth; the chain is a derived
  cache." A chain that is a derived cache is not a single authority. Option C rebuilt
  git's own content-addressed hash-DAG one level up and called the copy authoritative —
  which is the dual-authority hazard (`markov.rs:1-8`) reborn, the exact hazard Option C
  claimed to kill by construction.

**Final decision: Option D′ — "git-as-single-authority, derived-projection ranker."**
Counsel's Option D (pure co-change ranker) extended with exactly two cheap organs whose
need is already established by the brief: the import graph (covers new files with no
co-change history — Counsel's own honest counter-balance) and the per-node failure EMA
(the brief's error-detection ask, `geo.rs:39` re-pointed). Everything else Option C built
speculatively — the GraphDelta chain, compensator rollback, spectral drift organ, full n-D
Kalman, the RCI_BLOCKING flag — is **removed from v1** and parked behind named measured
triggers (§F below), per Counsel N2's "earn-the-power for analyzers, not just for flags."

What D′ keeps from C (nothing of measured value is lost): PPR blast-radius ranking over
the β-combined import+co-change operator (`csr.rs:228-264`), the one-call `AnalysisFrame`
simultaneous view, the recall/precision oracle loop (`csr.rs:387-427`), the zero-Python
port, advisory/fail-open discipline. What D′ loses vs full C, honestly: sub-commit
latency, ingestion of non-replayable sources, chain-level tamper evidence — none of which
any measured need requires today (all three sources — git, transcripts, CI — are already
durable, append-only, replayable).

---

## A. HIGH findings

### H1 — concurrent multi-producer chain fork → **FIX (structural removal)**
- **Decision:** v1 has no event chain and no producers. Ingestion is pull-based: one
  `rci derive` invocation reads git + transcript JSONL + import scan and rebuilds the
  projection, under an exclusive advisory lock on `.rci/` (concurrent invocation fails
  fast or waits; never interleaves). Single-writer is enforced by construction, not by
  protocol discipline.
- **Rationale:** the cited substrate documents itself as single-writer
  (`event_log.rs:18-19`); `set_tip` has no CAS (re-verified, `event_log.rs:204,310`).
  Pointing four cross-process producers at it was a design error, and adding CAS would be
  new concurrency machinery to preserve a chain that (per §0) was never the true
  authority. Removing the mechanism removes the fork class entirely.
- **Residue pinned:** if a v2 chain is ever built (trigger in §F), CAS-or-single-writer
  is a precondition recorded in the ADR — it cannot be skipped silently.

### H2 — rollback not idempotent; digest DoD unsatisfiable under concurrency → **FIX (structural removal) + claim withdrawn**
- **Decision:** the compensating-delta rollback is deleted from the design. Two distinct
  operations replace it, named separately so the overload Counsel flagged (STOP-2) cannot
  recur:
  1. **Projection reset** = re-derivation at a target commit: `rci derive --at <sha>`.
     Pure function of git state ⇒ idempotent trivially; `digest(derive(sha))` is
     reproducible by construction. The digest RED+GREEN test now proves exactly the
     property it is capable of proving (derivation determinism), and no stronger claim is
     made.
  2. **Revert-candidate suggestion** (renamed from "Correction proposal") = advisory
     ranking of recent commits by overlap with anomalous nodes; **always human-confirmed,
     human-executed `git revert`**. RCI never executes it (§D STOP-2).
- **Claim withdrawn explicitly:** "rollback is idempotent and safe under concurrent
  writes" is removed from proposal + ADR. Lamach is right that the round-1 test verified
  fold determinism, not concurrent-rollback safety; the revised DoD labels the digest test
  as a determinism test.
- **Rationale:** with git as authority and `.rci/` as cache, there is no authoritative
  RCI history to compensate. The Saga apparatus was solving Option C's self-inflicted
  problem. If a v2 chain ever exists, its rollback must be convergent
  (check-digest-then-append under the writer lock) and its DoD must include a
  concurrent-load test — pinned as a precondition in §F, not hand-waved.

### H3 — 896-file commit = 400,960 edges, 20× the cap; PPR saturates → **FIX**
- **Decision:** three explicit controls in the co-change extractor:
  1. **Wide-commit exclusion:** commits touching > `F_max = 30` files contribute zero
     co-change pairs (revised 64 → 30 post-resolve; see "H3 reconciled against
     prior-art" below). Excluded commits are logged and counted in `rci status` —
     explicit and observable, never silent pruning.
  2. **Per-node neighbor pruning:** keep the top-32 co-change neighbors per node by
     decayed weight.
  3. **Cap re-derived, not asserted:** worst included commit = C(30,2) = 435 pairs;
     with top-32 pruning, co-change edges ≤ 1.2k × 32 ≈ 38.4k — the retained-edge bound
     is set by the pruning and is **independent of F_max**; total budget unchanged:
     nnz ≈ 52k (imports + co-change), CSR < 1 MB, PPR ≈ 3–6 ms (§2 of revised proposal).
- **Rationale:** wide sweeps (format/rename/mass-port — the operator's measured 896- and
  507-file commits) are mechanical co-occurrence, not logical coupling; excluding
  over-wide transactions is the standard, published practice in co-change mining
  (Zimmermann et al., ROSE), so this is a semantics-preserving fix, not a lossy dodge:
  the signal being discarded is noise for the question being asked. The clique-saturation
  and spurious-spectral-drift consequences disappear with the cliques.
- **Accepted residue (owner: operator):** F_max=30 is a stated tunable, same status as β
  (R4); a genuinely-coupled 31-file commit loses its pairs. Bounded: such commits still
  produce Touch/error signal, the import channel still covers them, and genuine coupling
  recurs in focused commits, where it is re-learned.

#### H3 reconciled against prior-art (post-resolve addendum, 2026-07-17)

`H1-H4-prior-art-research.md` landed after this resolve and exposed a real discrepancy:
my `F_max = 64` was an ad-hoc back-of-envelope number, while the literature carries a
**parameterized default of 30** in both the canonical academic tool (ROSE: "ignoring all
changes that affect more than 30 entities" — Zimmermann et al., ICSE 2004) and the
canonical industrial one (code-maat `--max-changeset-size`, default 30). Reconciliation
was done against this repo's data, not by citation-deference alone:

1. **Budget invariance (verified by re-derivation):** the retained co-change edge bound
   ≤ 1.2k × 32 ≈ 38.4k comes entirely from the per-node top-32 pruning — F_max does not
   appear in it. So 64 bought zero budget headroom over 30; there never was a capacity
   argument for 64. The only number that moves is the worst included commit:
   C(64,2) = 2,016 → C(30,2) = 435 candidate pairs.
2. **Repo commit-style census (218 commits, current-era reflog of this working tree —
   subjects classified):** the wide-commit population splits into (a) mass ports / purges
   / format sweeps ("chore: drop ALL JS/TS", "remove legacy JS/TS thin-layer", "purge
   dead TS trees", "native Rust ports replace python/bash…") — the class both thresholds
   exclude; and (b) **wave/bundle commits** ("WAVE0 docker-swap DK-04/05/06/07/08 +
   DK-10", "finish waves W17-W20", "hydra G3-G8", "P7/P8/P10 waves") — 4–6 *independent*
   work items shipped in one commit. Intra-commit pairs in class (b) are
   shipping-schedule coincidence, not logical coupling: they violate the
   "one transaction ≈ one logical change" assumption that gives co-change its meaning.
   This repo's genuine features land as focused conventional commits (feat(kernel)/
   fix(harness)/single-module + tests). So the 31–64 band here is dominated by exactly
   the noise class the filter exists for — keeping 64 would *admit* bundle-cliques, the
   H3 failure in miniature.
3. **Structural harmony with top-32 (THIS-system argument, not in the literature):**
   F_max=30 ⇒ at most 29 same-commit neighbors < 32, so **no single commit can saturate
   a node's pruned neighbor list**; at F_max=64 one included bundle-commit fully
   determines the neighborhood of any low-history node, evicting nothing by weight only
   because there is nothing else — pure clique noise as a node's entire profile.
4. **Honest measurement limitation:** a per-commit file-count histogram
   (`git log --numstat`) of the 31–64 band was NOT run this round (no shell in this
   session; breaker's round-1 measurement covered only the top-20, all >100 files, all
   mechanical). The decision therefore rests on (1)–(3) plus the safety net, and carries
   a named confirmation step: **the first backtest-oracle run (M4) must include an F_max
   sensitivity check {30, 64} and a census (count + subjects) of excluded 31–64-file
   commits** — the default is confirmed or moved by measurement, not by taste. Recorded
   in proposal R4.

**Final: `F_max = 30` — the literature default adopted, because the repo evidence points
the same way and 64 had no argument left** (no budget gain, band dominated by bundle
noise, structural top-32 harmony). Boring & proven > bespoke: when prior art supplies a
published, twice-independent default and nothing in this repo contradicts it, the bespoke
number loses. Remains a stated tunable (R4) with an observable exclusion counter (§7) —
if the sensitivity check shows 30 measurably hurts recall here, the knob moves with
evidence.

### H4 — module graph structurally blind to money/auth/RLS coupling; precision@k could earn RCI_BLOCKING on that blindness → **FIX (three-part, the deepest cut)**
- **Decision:**
  1. **`RCI_BLOCKING` is removed from v1 entirely.** No drift-gate feed, no blocking
     code path, no flag to flip. The name is reserved in the ADR with preconditions
     (§F) so a future proposal cannot introduce it more weakly than this round demands.
  2. **Permanent negative invariant (LOCK, survives any future flag):** RCI **never**
     acquires blocking or blessing authority over red-line surfaces (money / auth / RLS /
     migrations globs). On those surfaces RCI output is one-directional: it may **add**
     friction (flag a concern), it may **never remove** it (a low blast-radius score can
     never be read as "safe"). Any `AnalysisFrame` for a red-line path carries
     `red_line: true` plus a fixed disclaimer: structural ranking cannot clear this
     surface; run the red-line checklist. This holds **even if** a future precision@k
     baseline clears any aggregate threshold — because Lamach proved the aggregate is
     survivorship-biased exactly on this class.
  3. **DoD restructured to make the blindness measurable instead of hidden:**
     precision/recall are scored **stratified** — red-line-glob incidents reported as a
     separate stratum, never blended into the headline number; and the E1 sign-split case
     (`field_frame.rs:92` vs `csr.rs:307`, no import edge — Lamach verified) is kept in
     the DoD as a **documented negative control**: the import channel is *expected to
     miss it*, and the test asserts + records that miss. A benchmark that proves the
     tool's blind spot is worth more than one curated to hide it.
- **Rationale:** Lamach's strongest finding, accepted in full. Cross-cutting invariants
  (integer-cents, tenant-scope, auth-path) couple by runtime contract, not by `import`;
  no import/co-change graph can see them, and co-change sees them only when history
  happens to co-change them. The honest posture is a tool that knows and declares what it
  cannot see, with red-line authority structurally unreachable rather than
  threshold-guarded.

---

## B. MED findings

### M1 — 40 top-level dirs > 32-supernode "structural invariant" today → **FIX-by-removal + DEFER-FLAG**
- **Decision:** the spectral drift organ is not in v1 (per §0 / N2), so no quotient graph
  exists to violate the cap. **Deferred organ precondition (recorded):** if/when built,
  the supernode partition is a *curated, checked-in explicit map* of ≤ 32 buckets
  (workspace-level grouping), never auto-derived from top-level dirs; and the RCI-side
  call path must n-check before `eigenvalues()` because the kernel auto-selects the dense
  path with no refusal (M5).
- **Trigger (named):** the oracle backtest (§C M4) shows a measured gap that a
  cascade/drift verdict would close — i.e., a class of incidents ranked poorly by
  co-change+import PPR that quotient-spectrum analysis demonstrably ranks well.

### M2 — `TestResult`/`meta` free-form text = secret/PII leak vector → **FIX**
- **Decision:** every persisted RCI artifact (`.rci/state.json`, frames, snapshots) uses
  a **closed schema — no free-form text fields anywhere**. Test results persist as
  `{test_id, status, duration_ms}` only; tool outcomes persist as the existing anonymized
  4-token alphabet (`markov.rs:30-39`); assertion diffs, stack traces, stderr, env values
  **never enter `.rci/`** — error text lives and dies in the ephemeral local log of the
  run that produced it. Claim-check discipline is restated to cover *contents and error
  text*, not just file contents.
- **Rationale:** Lamach is right that "metadata" was a hole exactly where error text
  lives. Closing the schema closes it; there is no legitimate v1 consumer of raw failure
  text inside RCI (the ranker needs only pass/fail + location).

### M3 — cross-tenant v2 inherits the tenant-blind kernel event model → **FIX (wording) + DEFER-FLAG**
- **Decision:** the proposal now states the strong form: **cross-tenant folding into one
  graph is a rejected construction**, not a v2 — a single CSR that has merged hubs has no
  tenant discriminant to re-partition on, and RLS-at-rest cannot fix an in-RAM merge
  (Lamach's point, accepted verbatim). Any per-tenant variant is per-tenant *derivation*
  (scope-per-hub, mirroring the audit's process-per-hub), with a tenant discriminant in
  the schema from birth, a new ADR, and RLS ENABLE+FORCE if stored — R7 upgraded from
  "precondition" to "rejected-construction + preconditions."
- **Trigger (named):** any proposal to point RCI at runtime error streams or at more than
  the single canonical repo.

### M4 — escalation floor unbootstrappable (flood vs months-inert) → **FIX**
- **Decision:** two-part bootstrap:
  1. **`RCI_ESCALATE` flag, default OFF.** A cold RCI emits **zero** `ESC-` records;
     findings live only in the local `AnalysisFrame`. The flood horn is dead by default.
  2. **Retrospective backtest bootstraps the floor from day one:** replay git history —
     at commit *t*, predict top-k impact; score against files touched by subsequent
     fix-commits within a window; the floor = Wilson lower bound on measured precision
     (same statistical discipline as the E2 recall-oracle work). The months-inert horn is
     dead because the baseline is measured from history, not awaited from future
     incidents.
- Stratification from H4 applies: the backtest reports red-line strata separately and can
  never earn red-line authority.

### M5 — fold mis-costed (µs claim false); `energy()` full-graph = latent O(n⁴)/11.5 MB trap → **FIX**
- **Decision:** (a) the latency table is corrected — there is no per-event fold in D′ at
  all; `rci derive` does one honest full CSR rebuild per invocation, O(E log E) ≈ 5–10 ms
  at nnz≈52k, inside a per-commit budget of 1 s (not a fictional µs line inside a 100 ms
  streaming budget that no requirement demanded). (b) The O(n⁴) trap is noted as **not a
  property of the kernel code** (`eigenvalues()` auto-selects with no refusal —
  `spectral.rs:206-213`): in v1 it is unreachable because RCI calls no spectral function;
  the deferred-organ precondition (M1) requires an explicit RCI-side n≤32 check before
  any `eigenvalues()`/`energy()` call, verified by test, because the "hard cap, refuse"
  must live in code, not prose.

---

## C. LOW findings

### L1 — DoD "zero .py" vs 2 files incl. an e2e test → **FIX**
- DoD (e) amended: the port covers **both** `transcript_events.py` **and**
  `test_transcript_e2e.py` — the Rust replacement e2e test (same fixtures, equivalent
  coverage) must be green **before** the `.py` files are deleted. Deleting a test to go
  green is forbidden (test-integrity red-line, restated in the DoD itself).

### L2 — chain order race ⇒ "same repo state ⇒ same analysis" false → **FIX (structural, via D′)**
- In D′ determinism is re-keyed to **source state**, not arrival order: `derive` is a
  pure function of (git HEAD, tree, transcript files). Same repo state ⇒ same graph bytes
  ⇒ same analysis — the property round 1 wrongly implied is now actually true, because
  the race-ordered chain no longer exists. The proposal states the distinction Lamach
  drew (replay-reproducibility vs observation-reproducibility) explicitly.

### L3 — path-hash "confidentiality" is hollow → **FIX (claim withdrawn)**
- Accepted: SHA3 over an enumerable path set is dictionary-invertible; the claim added
  false assurance. `.rci/` now stores **plain paths** (it is a local derived cache in the
  same trust domain as the git working tree itself); the "leaks nothing if exfiltrated"
  sentence is deleted. The honest security statement: `.rci/` contains repo structure +
  closed-schema counters, nothing else (M2), and that is the accepted exposure.

---

## D. Counsel ETHICAL-STOPs

### STOP-1 — missing SCOPE-RULE banner; RCI_BLOCKING → fail-closed drift gate without intervention-lift link → **FIX (revise)**
- The banner is written **verbatim** into proposal §0 and the ADR:
  > **SCOPE RULE — RCI is a canonical-repo dev-time fence, NOT a runtime/global control;
  > at runtime every hub (M5/M9) MAY ignore it.**
- The explicit lift-link is recorded: any future blocking coupling is **subordinate to
  the drift-gate's intervention-lift + kill-switch** (`event_log.rs:380-383` — "ALL
  safeties are LIFTED"); even a hypothetically-enabled RCI gate is never a permanent
  block — the conscious operator always lifts all safeties.
- Because v1 now contains **no blocking flag at all** (H4), the STOP's trigger condition
  cannot arise in v1; the recorded-human-decision requirement is pinned in §F as a
  precondition for any future `RCI_BLOCKING` proposal, alongside the banner and the
  red-line LOCK. Counsel's epistemic point (the scope must survive an operator who never
  read this proposal) is answered structurally: the banner sits in the ADR, and the
  mechanism it scopes does not exist to be misread.

### STOP-2 — "Correction/revert" naming overload in a money/auth-adjacent repo → **FIX (revise)**
- Explicit negative guarantee, written verbatim (UA as Counsel drafted, EN mirror) into
  proposal §8 and the ADR:
  > **«RCI НІКОЛИ авто-не-виконує revert реального коду й НІКОЛИ авто-не-емітить
  > компенсуючу подію в продакшн event_log; correction — завжди human-confirmed
  > suggestion, тим паче money/auth-суміжне.»**
  > (EN: RCI never auto-executes a revert of real code and never auto-emits into the
  > production event_log; correction is always a human-confirmed suggestion — above all
  > on money/auth-adjacent surfaces.)
- Naming fixed: "Correction proposal / suggested revert target" → **"Revert-candidate
  suggestion (human-confirmed)"** everywhere.
- Guard test added to the DoD (fence test, mirroring the bebop kernel-fence guards): the
  RCI binary has **no write path to the production event log** — no import of the kernel
  commit API; RCI writes only under `.rci/`. In D′ this is structurally trivial (there is
  no RCI chain and no compensator emitter at all), and the test keeps it true.

---

## E. Counsel non-blocking advice

### N1 — NO-AGENT-SCORING guard (mirror of NO-COURIER-SCORING) → **FIX (structural + guard), not deferred**
- In D′ the fix is stronger than a guard: the derived schema **has no actor dimension** —
  git author fields are dropped at parse; nodes are paths, edges are coupling; there is
  no `actor_pubkey` because there is no MeshEvent reuse. Plus the guard Counsel asked
  for: a DoD test asserting no per-actor/per-author aggregation exists in any RCI output
  schema — the NO-AGENT-SCORING pulse, visible, mirroring `event_log.rs:22-23`.
  "Rank changes, never actors" is now unviolable without failing a test.

### N2 — earn-the-power for analyzers, not just flags → **ACCEPTED — this IS the decision**
- Resolved by §0: v1 ships the minimum that carries the value (co-change + import PPR +
  EMA + frame); every further organ (spectral drift, full Kalman, chain, symbol-level)
  must be pulled in by a **measured gap against the same backtest oracle**, not by budget
  headroom. Round 1's "fits in budget" reasoning is retired as a justification — capacity
  is not need.

### N3 — legibility of block-capable verdicts → **FIX-by-narrowing + standing rule**
- v1's verdicts are naturally legible ("changed together N times in M commits, last at
  <sha>"; "imports X"; "failure EMA 0.4 over last 12 runs"). Standing rule recorded for
  any future organ: **a verdict that cannot carry a plain-language "why" may not surface**
  — spectral numbers without a named supernode + named edges stay internal.

### N4 — exit plan pinned next to the banner → **FIX**
- Proposal §0 banner block now ends with the exit line: removal = delete the hook line +
  the binary + `.rci/`. One glance shows a future reader this is removable before they
  can believe it is foundation.

### Counsel §5 (unasked question: code seismograph vs swarm EEG) → **RECORDED, binds the deferred organ**
- Accepted as an open question for the operator, attached to the deferred cascade/drift
  organ's trigger: if that organ is ever proposed, its proposal must declare **which claim
  it makes** — code topology or swarm behavior — and NO-AGENT-SCORING (N1) binds it
  either way. Not resolvable inside this round; explicitly not silently dropped.

---

## F. Deferred organs — named triggers (all MISSING until trigger fires)

| Deferred | Trigger (named) | Pinned preconditions |
|---|---|---|
| GraphDelta event chain (Option C machinery) | a measured need for sub-commit granularity OR a source that is not durable/replayable (none exists today) | CAS or lock-proven single-writer (H1); convergent rollback + concurrent-load DoD (H2) |
| Spectral drift / cascade organ | backtest shows a measured incident class PPR ranks poorly that quotient-spectrum ranks well | curated ≤32 partition map, never auto-dirs (M1); explicit n≤32 code check before `eigenvalues()` (M5); plain-language why (N3); code-vs-swarm claim declared (Counsel §5) |
| Full n-D Kalman | measured EMA false-positive/negative rate insufficient for the anomaly job | — |
| `RCI_BLOCKING` (any blocking coupling) | operator explicitly proposes it — never implied | recorded human decision (STOP-1); SCOPE-RULE banner + intervention-lift subordination; stratified precision incl. red-line strata; **red-line LOCK: never authority over money/auth/RLS/migrations, no threshold overrides this** (H4) |
| `RCI_SYMBOL_LEVEL` | module-level stratified precision measured < 0.6 useful floor (R1, unchanged) | tree-sitter dep decision = DECART report |
| pgrust storage move | Living-Memory arc lands and RCI state outgrows files | forward-only atomic migration; RLS ENABLE+FORCE if multi-scope (R7, unchanged) |
| Per-tenant / runtime-stream variant | any proposal to point RCI at runtime error streams | rejected-construction rule: no cross-tenant fold; per-tenant derivation + tenant discriminant + new ADR + RLS FORCE (M3) |

## G. Score-keeping (honesty check)

- Findings fixed by **removing mechanism** rather than patching it: H1, H2, M1, L2,
  STOP-2-mechanism — this is the Counsel steel-man doing real work, not cosmetics.
- Claims **withdrawn** (round 1 overclaimed): "rollback idempotent + safe under
  concurrent writes" (H2); "same repo state ⇒ same analysis" as stated (L2);
  "leaks nothing if exfiltrated" (L3); "O(n⁴) unreachable by construction" (M5 — the
  construction was prose, not code).
- Residual accepted risks, owner = operator (SyniakSviatoslav): F_max/β/top-32 as stated
  tunables (H3 residue, R4); regex import false edges (R2, unchanged — advisory noise
  bounded by the oracle); co-change = correlation not causation (R3, unchanged).
- Nothing in this resolution was marked resolved without a Lamach/Counsel round: this
  document **is** that round's output and awaits the operator's human final per the
  Counsel summary.
