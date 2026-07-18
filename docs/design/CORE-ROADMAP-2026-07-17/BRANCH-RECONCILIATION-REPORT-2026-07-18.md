# Branch Reconciliation Report — 2026-07-18

Branch: `fix/reconcile-diverged-branches-2026-07-18` (off `main` @ `0a87db928`, which carries the
full §16–§18 axis incl. §18.4's priority ruling). Scope: the four diverged branch groups named in
§18.4's commit-time pass. **Nothing here is merged into `main` — that decision is the operator's.**

## 0. First finding: §18.4's divergence numbers were stale at reconciliation time

§18.4 (and the task brief derived from it) records "agentic-mesh 226 ahead / ~0-1 behind" etc.
Live `git rev-list --count` at reconciliation start showed the **inverse**: `main` had already
absorbed the bulk of all three small groups via the same-day merge wave, leaving each branch only
its final post-merge docs commit(s) unique:

| Branch group | §18.4 claim | Live (this pass) |
|---|---|---|
| `feat/agentic-mesh-protocol-2026-07-17` (+ snapshot twin) | 226 ahead / ~0-1 behind | **1 ahead / 231 behind** |
| `feat/spectral-energy-flow-evolution` (+ snapshot twin) | 228 ahead / ~0-1 behind | **1 ahead / 233 behind** |
| `research/dowiz-verify-redteam-2026-07-17` (+ snapshot twin) | 162 ahead / 2 behind | **2 ahead / 167 behind** |
| `feat/kalman-organ` (+ snapshot twin) | 1052 ahead / 677 behind | **677 ahead / 1057 behind** |

Every snapshot twin points at the **identical SHA** as its primary (verified local == origin for
all six small-group refs), so each merge below covers its twin automatically.

## 1. `feat/agentic-mesh-protocol-2026-07-17` — MERGED CLEAN

- Unique work: one docs-only commit `cb4fe19f3` — resolves B1's wasmtime-dependency DECART flag
  inline (dependency landed optional/default-off) and appends B4's missing per-blueprint
  2-question doubt audit (independently re-derives the 3.26× batch/single ratio from raw ledger
  data at `6541ae8`). 4 files, +398 lines, zero `.rs` touched.
- §16–§18 check: kernel-side research docs; no DOM/UI surface; no P57–P74 duplication. Not
  superseded.
- **Merge commit: `4447e09f4`** (`--no-ff`, zero conflicts).

## 2. `feat/spectral-energy-flow-evolution` — MERGED CLEAN

- Unique work: one docs-only commit `f1548aaaf` — decorrelated §10 completion appendices for the
  E1/E2 blueprints, verifying their DoD claims against the live tree post-`6bd181a02`, including
  one real correction (the Normalized-Laplacian branch called "unbound" is actually parity-bound).
  2 files, +373 lines, zero `.rs` touched.
- §16–§18 check: kernel-math audit docs; not superseded.
- **Merge commit: `72b66a186`** (`--no-ff`, zero conflicts).

## 3. `research/dowiz-verify-redteam-2026-07-17` — MERGED, 3 CONFLICTS RESOLVED

- Unique work: two docs-only commits `b31cb488d` + `8494167b1` — enriches CORE-ROADMAP Layer
  C/D/E/F/G/H/I blueprints with the 2026-07-18 verification/red-team + fail-operational round-2
  findings, refreshes the index, backs up the session's source docs
  (`fail-operational-layout-versioning-2026-07-17/`, `verification-2026-07-17/`,
  `repo-maintenance-2026-07-17/`, `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md`), and repoints
  §1.2/§7 to the regenerated bebop2 cross-repo synthesis. 26 files, ~7.5k lines, docs only.
- **Merge commit: `e10c6ba63`**. Conflicts (all "two concurrent same-day appendices to the same
  file", not contradictions — resolved by keeping BOTH sides):
  1. **`BLUEPRINT-P-D-consensus-capability.md`** — `main` appended §11 (R-3 ruling record:
     Option A adopted, operator-overridable); the branch appended its own "§11" (red-team
     fold-in: A5/B-3/A7/B-6). Kept main's §11; branch fold-in renumbered **§11→§12** with an
     inline merge note flagging that §12's "R-3 remains the sole operator gate" is superseded by
     §11's recorded ruling. The branch also carried a **second, EOF-truncated duplicate** of the
     same fold-in (cut mid-sentence in the branch's own file) — dropped; the complete version is
     kept. The branch's §0 orientation paragraph was preserved (re-applied after the conflict
     rebuild briefly dropped the auto-merged hunk — caught and restored during resolution).
  2. **`BLUEPRINT-P-E-network-crypto-core.md`** — `main` appended §13 (Kalman SoA readiness
     spec); the branch appended its own "§13" (FEC + `LaneFrameHeader` fold-in). Kept main's
     §13; branch fold-in renumbered **§13→§14** with a merge note; branch's §0 context opener
     preserved; the fold-in's internal `§13's` self-reference fixed to `§14's`.
  3. **`CORE-ROADMAP-INDEX.md`** (2 hunks) — Layer-table rows took the branch's strictly richer
     versions with fold-in references corrected to the renumbered sections; the D row gained the
     R-3-recorded-closed reconciliation note; the **G row gained an explicit §16.30/§18.4
     supersession pointer** (see below). Second hunk: both sides' disjoint table-row additions
     kept (main's pgrust-rebuild row + branch's fail-operational/session-synthesis/hermes rows).
- §16–§18 check: the branch enriches the **pre-dialogue Layer A–I axis**, which §18.4
  deprioritizes but does not invalidate — the content is verification findings (money-recompute
  CRITICAL, Sybil/red-line gaps, FEC/wire-format rulings) that remain true regardless of UI
  paradigm. One exception handled explicitly: Layer G's "first real DOM surface" build-out
  conflicts with §16.30's wgpu-only/zero-DOM mandate — the merged INDEX row now carries a
  `⚠ SUPERSEDED as product-UI track (§16.30/§18.4)` marker so no future swarm builds on it,
  while its money-recompute findings are kept as valid.

## 4. `feat/kalman-organ` — NOT MERGED. Recommendation: **(b) archive**, with two small carve-outs

**No merge/rebase was attempted or executed**, per instruction. Investigation findings:

### What the branch actually is
- Merge-base with `main` is `129f73a42` (early June "Merge branch 'main'"). The branch carries an
  **entire alternative repo history since ~2026-06-05**: 677 commits spanning the TS-era product
  (src/screens HTML mocks, e2e Playwright, packages/, eslint-plugin-local), the July-14
  math-first wave (markov/absorbing/eqc/living-knowledge/kalman B1), plus exactly two fresh
  commits dated 2026-07-18: `f8c396080` (Layer-E N-courier Kalman SoA consumer, AVX2 +
  bit-identity tests + criterion bench + telemetry probe, "164 kernel tests") and `5e5177b75`
  (a `backup(...)` working-tree snapshot).
- Diff scope vs `main`: **2,667 files, +539k/−17.5k lines**, dominated by `attic/` (498 files),
  `graphify-out/` (427), `docs/` (375), **`.claude/` (364) and `.agents/` (300) config trees**,
  `e2e/` (202), plus output junk (`qa-shots/`, `dogfood-output/`) that `main` deliberately
  de-indexed in `f5358358`-era cleanup.
- **The branch name itself is ambiguous**: local `feat/kalman-organ` (`5e5177b75`) and
  `origin/feat/kalman-organ` (`8030e5bd8`) share **only the June merge-base** — 677 vs 675
  commits of two parallel rewrites of the same history (same subjects, different SHAs; a
  rebase/rewrite happened on one side and was never force-pushed/pulled through).
  `origin/feat/kalman-organ-snapshot-2026-07-18` == the local variant's tip, so both variants
  are safe on the remote.

### Why (b) superseded/archive, not (a) manual reconciliation
1. **Its only genuinely fresh work is already on `main`, independently.** `main` implemented the
   same BLUEPRINT-P-E §13 item as WAVE D: `kernel/src/simd.rs:296 kalman_batch_step` +
   `:359 kalman_batch_step_trust`, written against `main`'s `TrustEstimate`/`domain.rs` ownership
   boundary with the `kalman_batch_bit_identical` parity test. The branch's version targets its
   own stale fork's `CourierKalman` and cannot land without dragging the fork in.
2. **Its kernel is a strict-subset stale fork.** Every kernel module it adds (kalman, markov,
   absorbing, simd, spectral, …) exists on `main` in a far more evolved state (~40+ modules,
   561-test suite vs the branch's 22 modules / 164 tests); the whole `kernel/` tree would be
   add/add conflicts resolving to "take main" ~everywhere.
3. **Its unique non-kernel surfaces are exactly what §16–§18 and the Rust-native mandate
   retired**: 40+ DOM/HTML screen mocks (`src/screens/*.html`) vs §16.30 zero-DOM; Node/TS
   tooling (eslint-plugin, e2e Playwright, Python `tools/eqc` vs main's Rust `tools/eqc-rs`);
   and `.claude/`/`.agents/` governance trees, which must never be bulk-merged from a feature
   branch regardless.

### Carve-outs worth re-deriving onto `main` (cheap, no merge needed)
- `kalman_lane_telemetry` (branch `simd.rs:38-62`): dispatch-pole (simd/scalar) + stepped-courier
  counters — a mandatory-telemetry nicety `main`'s WAVE-D implementation lacks. ~25 lines,
  trivially re-implementable against `main`'s `simd.rs`.
- The branch's criterion bench shape for `kalman_batch_step` AVX2-vs-scalar, if `main`'s
  `kernel/benches/` doesn't already cover it.

### Recommended disposition (operator's call)
Keep both pushed refs (`origin/feat/kalman-organ`, `origin/feat/kalman-organ-snapshot-2026-07-18`)
as the archival truth — optionally retag as `archive/kalman-organ-{origin,local}-2026-07-18` —
and stop treating the branch as pending-merge work. Do **not** delete: it is the only surviving
record of the June-era alternative history. Do **not** attempt reconciliation: everything of
value either already landed on `main` through other branches or costs less to re-derive than to
excavate from a 539k-line divergence.

## 5. Not done here (deliberately)
- No merge of `fix/reconcile-diverged-branches-2026-07-18` into `main` — operator gate.
- No deletion or force-push of any investigated branch.
- No code changes outside conflict resolution; all merged content is documentation.
