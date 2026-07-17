# RCI — Resolution, RESOLVE loop 2 (Крок 6 — петля до жорсткого виходу)

> Author: system-architect subagent, 2026-07-17. Companion to `breaker-findings-round2.md`
> (re-attack on D′: 1 HIGH, 4 MED, 1 LOW) and `counsel-opinion-round2.md` (re-examine: both
> ETHICAL-STOPs structurally resolved, no new ones; 2 non-blocking asks + minor items).
> Numbering note: the first RESOLVE lives in `resolution.md` (internally titled "round 2");
> this document is the **second RESOLVE loop** and uses the `-round2` file suffix to match
> the breaker/counsel round-2 artifacts. Verdict vocabulary unchanged: **FIX** /
> **ACCEPT-RISK** / **DEFER-FLAG**.
> Facts re-verified live this loop: `tools/loop-signals/check.sh:17-21` (transcript source =
> `$HOME/.claude/projects/*/*.jsonl` — machine-local, session-scoped, outside git);
> ADR grep for `64` → only the intentional `{30, 64}` sensitivity-check mention remains
> (Counsel's F_max doc-sync item is already satisfied; recorded below).

---

## 0. Headline

Round 2 found no CRITICAL and re-attacked none of H1–H4 (all closed by mechanism removal).
It found **one real over-claim the re-derivation itself introduced (F-1, HIGH)** — treating
the machine-local transcript stream as part of a "durable, replayable single authority" and
shipping a DoD that only ever digests the git-derived half — plus four mechanism-specific
MED gaps and one growth LOW. **All six are resolved below: five FIX, one ACCEPT +
DEFER-FLAG.** The FIXes follow this design's own spine where possible: narrow the claim to
what is true and key it explicitly (F-1), remove the divergent second path rather than
synchronize it (F-2), make the silent path non-silent (F-3), remove the biasing surface
rather than disclaim it (F-4), name the primitive whose OS semantics dissolve the failure
class (F-5). No transcript-versioning, no heartbeat machinery, no windowing was added —
each was considered and rejected as over-engineering against a measured need of zero.

After this loop: 0 unresolved CRITICAL/HIGH, 0 unresolved ETHICAL-STOP, back-of-envelope
re-converges (one added cost line, budget unchanged), all Council artifacts exist. The
design meets the hard exit and goes to the operator's human final.

---

## A. F-1 [HIGH] — "same source state ⇒ same analysis" is FALSE for the error/loop organs → **FIX (claim narrowed + dual-key determinism + DoD boundary test)**

**Accepted in full.** The breaker is right on every leg, and the ground is verified this
loop: transcripts come from `$HOME/.claude/projects/*/*.jsonl` (`check.sh:17-21`) —
session-scoped, machine-local, gitignored, rotated. `derive --at <old_sha>` cannot
reconstruct the stream as of that sha; two clones of the same sha legitimately hold
different streams; and the DoD digested only `graph.csr` — the half whose determinism never
broke.

**Fix (what changed in proposal + ADR):**

1. **The authority claim is narrowed to what is true.** "Git-as-single-authority" now
   names the **structural spine only**. The design declares **two authorities of unequal
   durability**:
   - **git** — the single git-durable authority for the structural half (`graph.csr` +
     structural meta): keyed to **(HEAD, tree)**, machine-invariant, reproducible at any
     historical sha.
   - **transcript JSONL + CI** — a **machine-local co-authority** for the stream half
     (`state.json`: error EMA + loop verdict): deterministic **only for a FIXED input byte
     set** under the canonical fold order (F-2); **not** git-versioned, **not**
     reconstructible at a historical sha, **not** machine-invariant. Every blanket
     "durable, append-only, replayable" sentence covering all three sources is rewritten.
2. **Every reproducibility claim now carries its key.** §6 splits determinism into the two
   keys above; §9's "reproduces any historical view bit-identically" is narrowed to the
   structural half; a frame produced by `derive --at <sha>` labels its error/loop fields as
   computed from the *current* machine stream, never presented as historical.
3. **Provenance is visible, not mysterious:** frames carry `derived_at` (structural key)
   **and** `state_input_digest` (stream key) — when two machines disagree on an error
   verdict, the differing key says why in one glance.
4. **DoD closes the blind half — new item (i):** (α) a FIXED transcript/CI fixture ⇒
   byte-identical `state.json` digest, invariant under input-file arrival order (RED =
   mutate one event ⇒ digest mismatch); (β) the **boundary test**: same (HEAD, tree) with
   two *different* transcript sets ⇒ **identical `graph.csr` digest AND differing
   `state.json`** — the test asserts the boundary exists exactly where the design now
   claims it, instead of the guarantee silently not holding.

**Rejected fix (named):** making transcripts git-versioned (committing session logs) —
over-engineering per the breaker's own steer, and it would pull session content toward the
repo (a new PII/noise surface) to buy a historical-error-replay capability no measured need
asks for. Refused.

**Effect:** per the breaker's own closing line, F-1 downgrades to a documented residue —
the residue being "the error organ is a machine-local instrument", which is now a declared,
keyed, DoD-tested property instead of a false guarantee.

## B. F-2 [MED] — non-commutative EMA fold, incremental ≠ cold → **FIX (canonical order + the incremental path for the stream half is REMOVED)**

**Accepted.** `ema_next` (`geo.rs:39`) is order-dependent; the proposal specified neither a
canonical order nor which fold path is authoritative.

**Fix, two parts (mechanism removal again, not synchronization machinery):**

1. **Canonical total order, stated:** all stream events (every transcript `.jsonl` + CI
   results) are merged and sorted by **(timestamp, stream_id = source file path,
   line_index)** before folding — a total order with no tie left to arrival.
2. **`state.json` is NEVER incrementally folded.** Every derive re-folds the EMA state
   from scratch over the full canonical stream. The incremental cache (last-seen HEAD +
   per-file scan hashes) now applies **only to the commutative CSR half**. Cost is
   re-derived in §2: the stream is session-scoped and small (≤ ~10⁵ events), the fold is
   O(events) < 10 ms — the incremental path bought nothing for this half, so the honest
   fix is to delete it. With exactly one fold path, incremental-vs-cold divergence
   **cannot exist**; the late-arrival dilemma (watermark skip vs out-of-order fold)
   dissolves because there is no watermark for the stream half.

DoD (i α) proves the result: fixed inputs ⇒ byte-identical `state.json`, invariant under
arrival order. Breaker's break-scenario (corruption-triggered cold re-derive silently
changing anomaly verdicts) is now impossible for same-input-bytes, and attributable via
`state_input_digest` when the input bytes themselves changed (F-1).

## C. F-3 [MED] — top-32 recency pruning saturable by ~32 tiny sub-F_max commits, invisible to `excluded_commits` → **FIX (second signal: neighbor-churn alarm) + named residue**

**Accepted, including the observability point:** poison commits in the 2–30 band are
*included*, so the `excluded_commits` counter is structurally blind to this path — the
"never silent" claim had a hole exactly at the evasion route.

**Fix — control 4 in the co-change extractor (thresholds named):**

- `meta.json` keeps each node's top-32 neighbor-set fingerprint per derive. When a node's
  current neighbor set has **Jaccard similarity < 0.5** against its own set **W = 32
  commits earlier** (i.e. ≥ 16 of 32 neighbors replaced within the window), the derive
  raises a **churn alarm**: `rci status` surfaces `churn_alerts=<n>`, and the node's frame
  carries `churn_alert: true` with the plain-language why ("28 of 32 neighbors replaced
  within the last 32 commits — treat this ranking as suspect"). DoD (k): a synthetic burst
  of 32 two-file commits against one node MUST trip the alarm.
- Threshold rationale: under organic development decayed weights move slowly — overtaking
  a genuine neighbor requires *repeated* co-changes, so flipping half a neighborhood
  inside 32 commits is either the F-3 burst signature or a real mass refactor; both
  deserve one advisory line, and the false-positive cost is exactly that line.

**ACCEPT-RISK residue (R9, owner: operator):** an adversary pacing the attack beneath the
churn threshold can still shape rankings. Accepted because: (1) a repo-write-capable
adversary already holds a strictly stronger capability — editing the code the operator
runs; (2) the payoff is bounded to misdirecting *advisory* attention on **non-red-line**
paths (red-line frames suppress rankings entirely per F-4, and RCI has no blocking path);
(3) the breaker's own measurement: natural narrow-burst frequency is low (192/300 commits
sit in the organic 2–30 band; features land focused). The fast version of the attack is
now non-silent; the slow version is named, bounded, and owned.

## D. F-4 [MED] — printing the blast number on red-line frames is itself the automation-bias channel → **FIX (structural suppression of the ranking on red-line frames)**

**Accepted, and fixed at the root rather than socially.** The breaker's framing is exact:
the LOCK held mechanically but the channel that grants de-facto authority is operator
attention, and a low number printed beside a disclaimer feeds it.

**Fix — one-directional rendering, enforced in the frame contract:**

- On `red_line: true` frames the ranking number is **not shown at all**: the frame prints
  `blast: suppressed — ranking withheld on red-line surfaces; run the red-line checklist`,
  and the co-change top-k list is suppressed likewise.
- Rendering rule (mirrors the LOCK's own "may add friction, never remove it"): on
  red-line frames **only concern-raising signals may render** (error innovation above
  threshold, `churn_alert`); **reassurance-capable numbers never render**. A low EMA is
  omitted, not printed — nothing on a red-line frame can read as "safe".
- This removes the habituation channel **structurally**: there is no number to habituate
  on; the red-line frame is constant-shape across 40 benign changes and the 41st
  dangerous one.
- **DoD (h) strengthened:** the red-line fixture test now asserts the blast/ranking
  values are **ABSENT**, not merely that the flag + disclaimer are present.

**ACCEPT-RISK residual (R11, owner: operator):** no test can observe whether the human
actually runs the checklist. The specific drift mechanism Lamach named (number-beside-
disclaimer) is deleted; measuring operator checklist compliance would mean instrumenting
operator behavior — disproportionate and declined by name, not silently skipped.

## E. F-5 [MED] — lock primitive unnamed; SIGKILL-wedge + crash atomicity unspecified → **FIX (primitive + crash protocol pinned in the design)**

**Accepted:** "by construction" was unproven while the construction was unnamed.

**Fix — the primitive and both crash semantics are now design text, not implementer
choice:**

1. **Lock primitive: OS advisory lock** via `std::fs::File::try_lock()` (stable since
   Rust 1.89; `flock(2)` on Linux) held on `.rci/lock` for the duration of the derive.
   Kernel-associated: the OS releases it on process death **including SIGKILL/OOM/power
   loss** — the wedged-forever lockfile class **cannot exist**, so no heartbeat/takeover
   machinery is needed (considered, rejected as machinery duplicating an OS guarantee).
   PID + started-at are written into the lockfile **as observability metadata only**
   (`rci status` reports holder + age); liveness never depends on them.
2. **Crash-atomicity of `.rci/`:** every artifact is written tmp-file → fsync → atomic
   same-directory `rename(2)`; **`meta.json` renames last** and is the commit point. A
   crash at any instant leaves either the complete previous snapshot or the complete new
   one — never a torn mix. The loader validates version + digests; on mismatch it falls
   to the §7 cold re-derive path (which F-6 bounds).
3. **Liveness visibility (partial push, cheap):** beyond pull-only `rci status`, the
   advisory hook prints one alarm line when snapshot lag exceeds 50 commits — a wedge or
   starvation becomes visible to the automation path, not only to a human who happens to
   ask.
4. **DoD (j):** SIGKILL a derive mid-write ⇒ next invocation acquires the lock
   immediately (OS-released flock) and loads an intact previous snapshot.

## F. F-6 [LOW] — unwindowed cold re-derive (the recovery path) grows O(history); wave-time lock starvation → **ACCEPT-RISK + DEFER-FLAG (named trigger, pre-planned fix)**

**Accepted as a real growth coupling; windowing NOW is rejected as premature
optimization** — history is 847 commits today and the < 10 s budget holds with ~5×
margin at the 5k-commit estimate.

- **DEFER-FLAG (R10), trigger named:** when measured cold-derive time exceeds **K = 30 s**
  (or the `git log` stage alone exceeds 15 s), introduce co-change **windowing at the
  decay horizon**. The fix is pre-planned so the trigger cannot force an improvised
  patch, and it is semantics-preserving by construction: decayed weights already send
  commits beyond the horizon to ε-contribution, so a horizon window changes results only
  within ε. Snapshot-as-base is the companion mechanism (already exists).
- **Wave starvation — ACCEPT:** the pull model self-corrects post-wave; during a wave
  every frame is truthfully labeled `stale: true` — RCI is late, never wrong-and-fresh-
  looking. Mitigation shipped with F-5: the hook's lag > 50 commits alarm line makes
  starvation visible during the wave, and `rci status` now shows lock holder + age.
  Serving the advisory during high-velocity windows better than STALE would require the
  streaming model this Council already removed for cause; the trade is named and taken.

---

## G. Counsel round-2 items (non-STOP)

### G-1. (a) §G addendum — the earned dividend + self-reversal as integrity → **FIX (text added, proposal §3 + this section)**

Recorded explicitly, closing the ~30% Counsel named:

> **Option C was not waste — it was the necessary probe.** H1/H2 are findable only in a
> design specified well enough to break; the small v1 is the **earned dividend of that
> research**, not its refutation. One cannot know minimalism is *sufficient* without
> having designed the larger thing well enough to watch it fail. The architect's reversal
> of their own round-1 decision under evidence is recorded as the highest-integrity move
> available to an author (anti-sunk-cost, anti-Goodhart) — not as a corrected blunder.

And the inverse watch-item Counsel asked for, against "removal = trophy" drift:
**removal earns its case per-instance exactly as additions must** — cutting is a
discipline with a burden of proof, not a prestige move. (Watch on the pattern; this
instance's removals were each individually forced by a grounded finding.)

### G-2. (b) §5 sharpening — deliberation, not checkbox → **FIX (D2 precondition upgraded in proposal §10 + ADR)**

The deferred cascade/drift organ's precondition is upgraded from *declaration* to
*deliberation*: any future proposal for it must (1) **declare which claim it makes** —
code topology or swarm behavior — AND (2) **record the operator's explicit
want/don't-want decision on reifying swarm mood as a repo-health number at all**. A
passing backtest (efficacy) can never swallow the values question (purpose); the trigger
fires the deliberation, it does not replace it.

### G-3. Minor items

- **F_max ADR sync:** **verified already synced this loop** — the ADR carries 30
  everywhere; the only remaining "64" is the intentional `{30, 64}` sensitivity check.
  No edit needed; recorded so the item is not silently dropped.
- **Physician-seam on the backtest (per-author leak):** **FIX** — DoD (g) now explicitly
  covers **the backtest report output**, not only frames: no per-author aggregation in
  any RCI output, backtest included.
- **Committed-vs-running structure honesty:** **FIX** — one line added to the §9 DoD
  preamble: guards are committed structure at design time and become running structure
  only when DoD (f)/(g)/(h) are green; "structurally resolved" is not to be read stronger
  than that before implementation.

---

## H. Hard-exit checklist (Крок 6)

| Gate | Status |
|---|---|
| Unresolved CRITICAL/HIGH | **0** — round-2 had no CRITICAL; the single HIGH (F-1) is FIXED (claim narrowed + dual-key determinism + DoD boundary test); breaker's own text: with that fix "F-1 downgrades to a documented residue" |
| Unresolved ETHICAL-STOP | **0** — Counsel round 2: both prior STOPs structurally resolved, **no new STOPs**; both non-blocking asks (G-1, G-2) FIXED anyway |
| Back-of-envelope converges | **Yes** — one line added (stream full-fold O(events) < 10 ms at ≤ ~10⁵ events); nnz ≈ 52k, CSR < 1 MB, PPR 3–6 ms, cold < 10 s at 847-commit reality all unchanged; F-6 growth carries a named trigger (K = 30 s) with a pre-planned ε-preserving fix |
| Artifacts exist | `proposal.md` (final revision), `docs/adr/ADR-realtime-change-intelligence.md` (final revision), `resolution.md`, `resolution-round2.md` (this), `breaker-findings.md`, `breaker-findings-round2.md`, `counsel-opinion.md`, `counsel-opinion-round2.md` |

**Score-keeping (loop-2 honesty check):** claims withdrawn this loop: "git + transcripts +
CI are all durable/replayable" as a blanket (F-1); "same repo state ⇒ same analysis"
unqualified (F-1); "incremental == cold" for the stream half (F-2 — resolved by deleting
the incremental path, not by proving the equality); "excluded → never silent" as full
coverage of commit-shaped distortion (F-3 — churn alarm added where the counter is blind).
Nothing in this resolution is marked resolved without this loop's Lamach/Counsel round —
this document is that loop's output. The design now goes to the operator's human final.
