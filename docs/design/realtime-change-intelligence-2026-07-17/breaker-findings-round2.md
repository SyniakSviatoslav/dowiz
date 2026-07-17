# RCI — Breaker Findings ROUND 2 (RE-ATTACK on Option D′)

> Target: `proposal.md` (RESOLVED-DRAFT, Option D′ — git-as-single-authority, single-writer
> pull-based derive, F_max=30, `RCI_BLOCKING` removed) + `resolution.md`.
> Mandate: this design was **re-derived, not patched** (event chain / compensator rollback /
> RCI_BLOCKING all *removed*). H1–H4 are closed by mechanism-removal and are **not**
> re-attacked here. This round hunts D′'s **own new holes** — regressions the removal
> introduced, and claims the two prior rounds could not see because the design was different.
> Read-only; grounded against live code + git 2026-07-17. No fixes proposed.
> Format: `[SEVERITY] vector · finding · break-scenario/number · violated invariant`.

---

## HEADLINE VERDICT

D′ is genuinely cleaner than Option C on the axes it was re-derived to fix: nothing forks
(H1), nothing compensates (H2), one wide commit no longer saturates (H3), and no code path
grants blessing authority over red-line surfaces (H4). No CRITICAL exists — the
advisory/fail-open + no-runtime-data + no-blocking posture structurally caps blast radius.

**But the re-derivation shipped one over-claim that is false and untested (F-1, HIGH) and
opened four new mechanism-specific gaps (F-2…F-5, MED) plus one growth gap (F-6, LOW).**
The central regression: the resolution asserts D′ makes determinism *"actually true"*
(L2) and *"single authority = durable, append-only, replayable"* (§0) — but that is true only
for the **git-derived structural half**. The **transcript/CI-derived half** (error-EMA + loop
organs) is keyed to a non-git-versioned, machine-local, order-dependent stream, and the DoD
digests only `graph.csr`, so the divergence is invisible.

---

## HIGH

### F-1 — [HIGH] B-CONSIST / B-ANTIPATTERN · "git-as-single-authority, same source state ⇒ same analysis" is FALSE for the error/loop organs; the transcript authority is neither git-versioned nor durable/replayable, and the DoD does not test it

The whole justification for choosing D′ over C is stated as a durability premise:
> "Git + transcript JSONL + CI are all **durable, append-only, replayable**" (proposal §1);
> "all three sources — git, transcripts, CI — are already durable, append-only, replayable"
> (resolution §0); "derive is a **pure function of (git HEAD, tree, transcript files)** ⇒ same
> repo state ⇒ same graph bytes ⇒ same analysis" (proposal §6 / resolution L2).

**Grounded refutation — the transcript source is ephemeral, machine-local, and gitignored.**
RCI ports `transcript_events.py`, which is fed by `check.sh` from
`$HOME/.claude/projects/*/*.jsonl` (verified: `check.sh:20-22` — `ls -t "$HOME"/.claude/projects/*/*.jsonl`).
These are **per-session agent transcripts**, not repo objects. `.gitignore` excludes the whole
derived-stream class (`run-history.jsonl` L21; `e2e/findings/transcripts/` L58; loop-signals
bytecode L48). They are session-scoped and subject to rotation/compaction. Therefore:

- **Not git-versioned.** `rci derive --at <old_sha>` can faithfully reconstruct the *git tree*
  at any sha (that is what git is), but it **cannot** reconstruct "the transcript/CI stream as
  it existed at old_sha" — today's `$HOME` holds sessions from *after* old_sha and no longer
  holds sessions that were pruned. So the §9 claim *"`rci derive --at <sha>` reproduces any
  historical view **bit-identically**… no log archaeology"* is false for `state.json`
  (EMA/innovation) and the loop verdict. It holds only for the CSR structure.
- **Not machine-invariant.** Two clones of the identical git state on two machines have
  *different* `$HOME/.claude/projects` → different EMA → different `state.json` → different
  anomaly/innovation verdicts for the byte-identical repo. "Same repo state ⇒ same analysis"
  (the L2 property the resolution says is *"now actually true, because the race-ordered chain
  no longer exists"*) is still false — the non-determinism just moved from chain-arrival-order
  to stream-provenance.
- **The DoD cannot catch it.** DoD (a) tests only `graph.csr` digest; DoD (b) tests
  `derive --at A` equality (again structural). **No DoD item digests `state.json`.** The one
  half of the tool whose determinism actually broke is the one half the verification never
  looks at.

**Break scenario:** operator on machine A sees `rci frame apps/api/src/routes/orders.ts` →
`error: EMA 0.42, innovation HIGH (anomaly)`. Same commit, machine B (or after a session
compaction on A): `error: EMA 0.08, no anomaly`. The "debug = re-derivation, bit-identical"
guarantee that §9 sells as replacing log archaeology silently does not hold for the error
organ — the exact organ the brief added to detect failure.

**Violated invariant:** D′'s load-bearing "single durable/replayable authority ⇒ deterministic
derivation (observation-reproducibility)" — proposal §6, resolution §0/L2. The premise is true
for git and false for the transcript/CI co-authority, and the name "git-as-single-authority"
hides that there are three authorities of unequal durability.

---

## MED

### F-2 — [MED] B-CONSIST · the incremental derive cache (last-seen HEAD + scan hashes) folds the non-commutative EMA in arrival order; it diverges from a cold re-derive, and only the commutative CSR half is DoD-tested

§4.1: *"Incremental: caches last-seen HEAD + per-file scan hashes; cold path re-derives fully."*
The two paths agree only where the fold is **order-independent**:

- **CSR half (safe):** co-change edge weights and import edges are additive / set-membership →
  commutative → incremental == cold. This is what DoD (a) digests. Fine.
- **EMA half (unsafe):** `ema_next(prev, sample, α) = prev + α·(sample − prev) =
  (1−α)·prev + α·sample` (verified `kernel/src/geo.rs:39`) is a **non-commutative recurrence** —
  its value depends on the *order* of the update stream, not the set of samples. The stream is
  **multi-source** (multiple session `.jsonl` + CI results). Cold re-derive replays in some
  canonical order; incremental folds *new* events since `last-seen HEAD` onto the *prior* EMA in
  **arrival order**. A cross-stream event that arrives late (a CI result, or a session line, whose
  logical time precedes an already-folded event) is either applied out-of-order (value drift) or
  skipped past the HEAD watermark (data loss). Either way `state.json`_incremental ≠
  `state.json`_cold for the same final source state.

**The dilemma is unavoidable without a spec the proposal does not give:** either (a) canonical
order = timestamp → incremental late-arrivals diverge (this bug); or (b) canonical order =
arrival/file order → deterministic *per machine* but then EMA is a function of arrival order,
which is **not** part of the git tree state → F-1's machine-to-machine divergence. D′ picks
neither explicitly, so it inherits whichever is worse in practice.

**Break scenario:** `rci derive` runs at HEAD~5 (folds CI batch); five commits later the CI for
HEAD~5 finally reports and is folded at HEAD; a `.rci/` corruption then forces a cold re-derive
(§7) which replays everything in canonical order. The pre- and post-corruption `state.json`
differ — a "recovery" silently changed the anomaly verdicts, with no digest test to flag it.

**Violated invariant:** §6 *"re-running derive at the same HEAD is a byte-identical no-op"* and
the idempotency claim — true for `graph.csr`, unproven and false-in-general for `state.json`.
Shares a root with F-1; rated MED (not HIGH) because a stated total-order canonicalization would
close it — the design simply omitted it.

### F-3 — [MED] B-SCALE / B-DATA · the H3 fix (top-32 recency pruning) opens a new saturation path: ~32 tiny sub-F_max commits fully evict a node's genuine neighbor list, silently (excluded-counter is blind to it)

The H3 fix bounds ONE wide commit (F_max=30 ⇒ ≤29 same-commit neighbors < 32, so
*"no single commit can saturate a node's pruned neighbor list"* — proposal §4.1). Correct for
**one** commit. But the neighbor list is pruned to **top-32 by *decayed* weight** (`ema_next`,
recency-weighted). Recency weighting means a *burst of recent tiny commits* dominates old
genuine coupling.

**Adversarial / automated scenario (numbers):** to seize control of node X's blast-radius
output, author **32 commits, each touching (X, Yᵢ)** for attacker-chosen Y₁…Y₃₂ (2 files each,
trivial edit). Each commit has 2 files ≪ F_max=30 → **not excluded, not counted**. After the
burst, X's 32 decayed-top neighbors are exactly {Y₁…Y₃₂}; every genuine historical neighbor is
evicted by weight. `rci frame X` now reports blast radius = attacker's set. In an
agent-swarm repo where commits are cheap and machine-authored, 32 commits is seconds of work.

**Why the two prior rounds + the prior-art census missed it:** round-1 H3 attacked a *single*
896-file clique; the resolution's 218-commit census classified *existing* commit *styles*
(monolith sweeps vs bundle waves) — neither modeled an actor deliberately emitting many
*narrow* commits. Measured today (last 300 commits): only **9 are >30 files** (F_max excludes
~3%); **192 sit in the 2–30 band that is included**; the filter's whole protection lives in a
band an attacker trivially stays under.

**Observability gap (compounds it):** §7/§9 sell `excluded_commits` as the visibility for the
F_max control (*"surfaced in `rci status`, never silent"*). Sub-F_max poison commits are
**included**, never excluded, so they **never increment that counter** — the "never silent"
guarantee has a blind spot exactly at the evasion path.

**Honest severity note:** natural occurrence is LOW (this repo's genuine features already land
as focused 2–5-file commits, and its wide mechanical sweeps land as *one* excluded commit, not
chunked). This is an *adversarial/automated* MED — the primary organ (consequence prediction) is
attacker-controllable on **non-red-line** paths, but the red-line LOCK (F-4 notwithstanding)
keeps it off money/auth surfaces, and it is advisory/fail-open.

**Violated invariant:** the §2 "PPR yields a *ranked* (non-saturated) impact set" utility premise
and the §7 "excluded → never silent" observability claim.

### F-4 — [MED] B-ANTIPATTERN / B-OPS · the red-line LOCK is technical, but surfacing a low blast-radius *number* on red-line frames is itself the automation-bias channel the LOCK's own rationale forbids — and no gate detects the resulting checklist erosion

H4's LOCK is real and correct as far as *code* goes: RCI can never mechanically clear a red-line
surface; every red-line frame carries `red_line: true` + a fixed disclaimer. The resolution
calls this *"answered structurally… red-line authority structurally unreachable"* (H4). That
over-claims, because the channel that grants de-facto authority is **the operator's attention,
not a code path** — and D′ *added* a new surface that feeds it.

**The leak:** a red-line frame still shows `blast: 2 files (low)` **next to** the disclaimer
"structural ranking cannot clear this surface; run the red-line checklist." The number is what
persuades; the disclaimer is what the eye learns to skip. The H4 rationale itself says this
number is **survivorship-biased and untrustworthy on exactly this class** — yet the design
prints it on exactly this class. Over N correct-looking low-blast readings, the operator's
internal threshold for "do I really need the full money/auth/RLS checklist?" drifts. This is
textbook automation complacency, and it is **precisely C's mechanical-blessing failure mode
re-entering through learned human trust** — the thing the flag-removal was meant to kill.

**No gate catches it:** DoD (h) asserts the flag + disclaimer are *present* on a red-line
fixture — it never asserts the number is *withheld*, nor can any test observe whether the human
actually ran the checklist. The one measurable proxy that could reveal erosion (checklist
completion rate on red-line changes) is not in the DoD at all.

**Break scenario:** three months in, RCI has flagged low blast radius on 40 orders.ts changes,
all fine; the 41st change breaks the integer-cents invariant (a runtime-contract coupling no
import/co-change edge can see — the H4 blindness, intact); the operator, primed by 40 greens,
skims the disclaimer and skips the manual money checklist. RCI never "blocked" or "blessed"
anything — the gate was the human, and the human's threshold moved.

**Violated invariant:** the H4 negative-authority guarantee's *intent* (RCI must never
functionally clear a red-line). Held mechanically, breached behaviorally; the "structurally
answered" framing is stronger than the mechanism delivers.

### F-5 — [MED] B-FAIL / B-OPS · the single-writer lock's crash-safety is unspecified; if lockfile-style, a killed derive wedges RCI at STALE forever with no liveness probe — and crash-mid-write atomicity of `.rci/` is undefined

The single-writer invariant (H1's fix) rests entirely on *"an exclusive advisory lock on `.rci/`
(concurrent invocation fails fast **or waits**; never interleaves)"* (proposal §4.1 /
resolution H1) with *"lock wait > 5 s → skip this derive"* (§7). The design **names no lock
primitive**, and the crash semantics differ decisively by choice:

- **`flock(2)` on an fd (kernel-associated):** OS releases on process death → a crashed derive
  self-heals. Safe, zero-dep, but **not stated**.
- **Lockfile / PID-file:** survives the holder's death → a `derive` killed by OOM / SIGKILL /
  power-loss **while holding the lock** leaves it held forever. Every subsequent invocation hits
  "wait 5 s → skip" (§7) permanently; RCI freezes at the last snapshot, every frame `stale:
  true`. This is a **new failure class C did not have** (C had no lock).

**The observability claim does not cover it.** §9 promises *"Observe in <1 min: `rci status`"* —
but that is a **pull** a human must initiate; **nothing probes writer liveness**. A wedged lock
produces no alarm; RCI just quietly stops updating and keeps answering `stale: true` (which
hooks treat as pass, fail-open). So the failure is *silent to the automation* and detectable only
by a human who happens to run `status` — the "<1 min visibility" is not delivered for this class.

**Companion gap — write atomicity is undefined.** §6 promises *"on fault, keep the previous
snapshot and mark STALE"* via typed `StoreError` discipline — but a SIGKILL/OOM/power-loss is
**not a catchable StoreError**; the "keep previous snapshot" code never runs. Whether a
half-written `graph.csr`/`state.json` is safe then depends on temp-file-then-rename vs in-place
mutation — **also unspecified**. If in-place, recovery falls to §7's "corrupt → delete + cold
re-derive," which is a real path (bounded by F-6), but the design asserts crash-safety it has not
pinned.

**Violated invariant:** H1's "single-writer enforced *by construction*" and §9's
degraded-vs-down / <1-min-visibility taxonomy. Fail-open caps blast radius (commit path
untouched) → MED, not HIGH; but "by construction" is unproven until the primitive + stale-lock
recovery are pinned in the design, not left to the implementer.

---

## LOW

### F-6 — [LOW] B-SCALE · cold re-derive reads `git log --name-only` over ALL history with no window; it is the recovery path that fires on corruption/crash (F-5), so recovery cost grows unbounded as the fast-committing repo ages — and the single-writer lock starves RCI to STALE exactly during high-velocity waves

Two coupled small things, both bounded by fail-open:

1. **Unwindowed cold derive.** The co-change extractor runs `git log --name-only` (§4.1) with
   **no history window stated**; §2 pins "<10 s" at "~5k commits." History is **847 commits
   today** (`git rev-list --count HEAD`), so it is fast now — but this repo commits in bursts
   (300 commits are recent-era), the cost is O(total history), and **cold derive is the
   recovery path** triggered by every `.rci/` corruption (including the F-5 crash-mid-write).
   So the degraded-mode recomputation gets slower precisely as the repo you most need it on
   grows, with no window and no bound.

2. **Lock starvation during waves.** The single-writer lock serializes all derives; §7 handles
   contention by "wait 5 s → skip … next invocation catches up." Under a sustained agent-swarm
   wave (this repo's normal mode — "WAVE0", "W17-W20", "P08/P04/P11 §2/§4/§7"), there is no
   quiet window, so every derive skips and RCI sits at STALE — i.e., RCI is **least fresh
   exactly when change-velocity (and consequence risk) is highest**. It catches up only after
   the wave settles. Advisory value → 0 during the window that most wants it.

**Violated invariant:** the §2 "<10 s cold restore" budget (asserted at a commit count the repo
will exceed) and the implicit "advisory freshness tracks activity." LOW because both are
bounded by fail-open and self-correct post-wave; flagged because the recovery-cost/growth
coupling (2·1) and the freshness/velocity inversion (2·2) are new to D′'s pull+lock model — C's
streaming model did not serialize on one writer lock.

---

## Regression ledger (round 2 vs round 1 — what the re-derivation changed)

| Round-1 finding | Status in D′ | New hole the fix opened |
|---|---|---|
| H1 chain fork | **CLOSED** (chain removed) | F-5 — the replacement single-writer *lock* has unpinned crash-safety |
| H2 rollback not idempotent | **CLOSED** (compensator removed) | F-1/F-2 — "determinism now true" over-claims: false for the stream-derived EMA/loop half, untested by the DoD |
| H3 wide-commit saturation | **CLOSED** (F_max=30 + top-32) | F-3 — top-32 *recency* pruning is saturable by ~32 tiny sub-F_max commits, invisible to `excluded_commits` |
| H4 red-line blessing via RCI_BLOCKING | **CLOSED mechanically** (flag removed + LOCK) | F-4 — same blindness re-enters *behaviorally* via the low-blast number shown on red-line frames; no gate detects checklist erosion |
| — | — | F-6 — pull+lock model introduces unwindowed-recovery growth and wave-time STALE starvation |

**Net:** D′ is a real improvement, not cosmetics — every HIGH from round 1 is genuinely
dissolved by removal, and no CRITICAL survives (fail-open + no-authority + no-runtime-data caps
severity). The one HIGH that remains (F-1) is not a re-attack: it is the specific over-claim the
re-derivation introduced — treating a machine-local, gitignored, session-scoped transcript
stream as a "durable, replayable, single authority" and shipping a DoD that only ever digests the
git-derived half. Fix the authority/durability wording and add a `state.json` determinism test to
the DoD and F-1 downgrades to a documented residue; leave it and D′ ships a reproducibility
guarantee that is false for its own error organ.
