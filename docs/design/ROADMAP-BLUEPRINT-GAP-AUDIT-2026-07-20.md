# Roadmap-wide Blueprint-Gap + Landed-Status Audit — 2026-07-20

> **Scope.** A delta audit against `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` (read that first —
> it explains the method in detail and this doc doesn't repeat it), covering the full additional
> day of work that landed on `main` since (product-surface wave, remaining-queue wave, space-grade
> item execution). Two independent read-only passes: (1) blueprint-link coverage across
> `CORE-ROADMAP-INDEX.md` and the space-grade track, (2) fresh `cargo test` counts + commit-hash
> verification against live `main`. Audit-only where noted; a handful of small, low-risk
> documentation fixes were applied directly rather than just reported, per this session's
> established pattern of closing what it finds. Zero product code touched.

---

## 0. Headline

**Coverage is still effectively complete.** GAP-A1 — the one gap the 2026-07-19 audit left
open — turns out to have been closed the same day it was flagged (`GAP-A1-DISPOSITION-AUDIT-2026-07-19.md`
exists, dispositions every named unit). The 2026-07-19 audit's own text just went stale the moment
it was committed, because the disposition doc landed in a later commit the same day. No new
phase-numbering gaps were introduced by today's work. What today's fast pace DID produce: 4
orphaned/under-linked docs, 2 stale status cells, and one label bug — all fixed in this pass — plus
one real (non-blueprint) finding: two space-grade items have real standalone scripts that aren't
wired into CI despite their own spec text describing a CI gate.

## 1. Blueprint-link findings (fixed in `CORE-ROADMAP-INDEX.md`)

| # | Finding | Fix |
|---|---|---|
| 1 | Dangling link at the WIRING WAVE row: pointed at `../../.claude/projects/-root-dowiz/memory/...`, which resolves under `docs/design/` to a path inside this repo that doesn't exist. The real file lives outside the repo entirely, at `/root/.claude/projects/-root-dowiz/memory/`. | Converted to plain text with an explicit note that it's a memory-system file, not a resolvable repo-relative link. Same edit also corrected the row's stale "Push is operator-gated — not pushed" — `17d65f315` is now a confirmed ancestor of current `origin/main`. |
| 2 | `BLUEPRINT-P40-agent-loop-tool-wiring.md` (2026-07-18, the executor build) and `BLUEPRINT-P40-AGENT-EXECUTOR-PRODUCT-WIRING-2026-07-19.md` (the product-wiring follow-up) were both zero-referenced anywhere in `docs/design/`, despite P40 being heavily cited in prose across P41/P42/P43/P44/P48/P54 and the crosswalk. | Added a new P40 row in §10. |
| 3 | `SPACE-GRADE-VERIFIED-STATUS-LEDGER-2026-07-20.md` (the real execution ledger for the operator's "autopilot to completion" authorization — items 8,9,20-24,26,48,50,54-56+ closed with acceptance-filter evidence) had zero references anywhere in the corpus. | Added a row in §7, including the items-45/73/74 CI-wiring caveat (§3 below). |
| 4 | `LIVING-MEMORY-WAVE-PROPAGATION-FINISHING-LAYER-SYNTHESIS-2026-07-20.md` and `OPTICAL-COMPRESSION-DECISION-2026-07-19.md` were each reachable only via a 3-hop path (index → intermediary doc → target), violating this index's own stated ≤2-hop rule. | Added a direct row. |
| 5 | The "item 5/26" mislabel on the product-surface-wave row: item 5 (regex retirement) was already closed 2026-07-19 and is unrelated; the cross-mesh-replication landing is tracked by the space-grade roadmap doc itself as out-of-band §M ("not one of the original 78 items"), not item 5. | Corrected to cite §M + item 26 by name. |
| 6 | The item-26 index row still read "CLOSED — measurement-only, NO batching code landed (scope law held)" — accurate as of 2026-07-19, but the operator later authorized the batching this same measurement pass had flagged BATCH-WORTHY-but-gated, and it landed as real code the next day (`kernel/src/hydra.rs`, commit `85022e49d`). | Added a superseded-by note citing the commit and the verified-status ledger's own confirmation. |

No other dangling links were found across §0–§10. No 2026-07-20 P-numbered work needed a new
phase slot — the remaining-queue wave's payment/omnichannel docs correctly self-frame as
extensions of existing P60/P48/P43, not new phases (confirmed both already exist as phase
blueprints and the new docs use "residual"/"INTAKE" naming rather than duplicating a number).

## 2. Main-branch landed-status verification (fresh, 2026-07-20)

Read-only verification, not a fix — reported here since "status of done on actual main" was
explicitly asked for.

- **HEAD**: `499869e55`, exactly matching `origin/main` (zero divergence either direction).
- **Fresh test counts, zero failures across the board**: kernel default **1137 passed / 8 ignored**
  (was 894 on 2026-07-19); kernel `--features pq` **1310 passed / 9 ignored** (was claimed 1131
  mid-day); engine **128 passed** (was 121). Growth is monotonic and consistent with the day's
  claimed landings — no regressions found anywhere.
- **13 spot-checked commit hashes** (spanning the group-commit/mesh-replication/Kani/KnowledgeSpine/
  product-surface-wave/remaining-queue-wave landings) all exist, are ancestors of HEAD, and their
  file-change lists match what memory/docs claimed for each. None fabricated, none mislabeled.
- **No unmerged product work found reaching main is missing.** Dozens of `exec/*`/`sg-wt*`/
  `gov-item*` branches show as "unmerged" by a naive `git log main..branch` check, but their real
  content already landed via the squash-import commit `cb00706b1` — the same pattern
  `GROUND-TRUTH-2026-07-19-FINAL.md` already documented for a different batch of branches.
  `recover/stash-1/2` remain intentionally unmerged (standing operator ruling, unchanged).

## 3. Real (non-blueprint) finding: items 45 and 73/74 claim CI enforcement they don't yet have

Spot-checking the space-grade roadmap's "25 items now REAL CODE" claim against 5 sampled items
(1, 7, 26, 45, 73) found 3 fully confirmed (zero-dep gate, Kani proofs, group-commit — all real,
tested, and CI-wired) and 2 partial discrepancies:

- **Item 45** (`ai-optional-gate`): `scripts/ai-optional-gate.sh` exists (169 lines, substantial,
  landed via `cb00706b1`) but is **not referenced in any `.github/workflows/*.yml` job**. The
  item's own spec text describes "New CI job... (a) default-features build must compile AND pass
  the FULL kernel test suite; (b) a dependency-direction check" — that CI job does not exist yet.
- **Items 73/74** (Gate-Root Invariant + red-line registry): `scripts/gate-root-invariant.sh`,
  `scripts/red-line-classifier.sh`, `scripts/red-line-monotonicity.sh`, `scripts/verify-item-74.sh`
  all exist (56–239 lines each, real logic, same commit) and are likewise **not wired into any
  workflow**. Neither item carries a `✅ DONE` marker in the roadmap doc — unlike items 1–31, which
  consistently do — so the doc's own annotation convention correctly stays silent here; the risk
  was the *ledger's* summary line ("25 items... landed+tested") reading as if CI enforcement were
  live for all of them, which it isn't for these two.

**Disposition: reported, not silently fixed.** Wiring 4 new CI jobs is a real, separate,
higher-blast-radius change (shared pipeline, needs its own test/verification pass this audit
didn't do) — out of scope for a documentation-correctness pass. Status annotations were added
directly to items 45/73/74 in `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` stating plainly
that the scripts exist standalone but the gate is not yet live, so the doc stops overstating its
own status. Actually wiring them into `ci.yml` is a named, scoped, operator-decidable follow-up.

## 4. Operational note: disk-space + git-collision incident found and resolved mid-audit

Not a roadmap finding, recorded for continuity. Mid-audit, `/` hit 100% full (0 bytes free),
caused by ~99 registered git worktrees accumulated by the autopilot execution swarm — each
carrying its own multi-GB `target/` build cache, almost all for branches already fully merged
into `main`. 77 were removed immediately (zero unique commits vs `main`, zero uncommitted
changes — unambiguously safe by the same method `GROUND-TRUTH-2026-07-19-FINAL.md` already used),
freeing 25GB (100%→66% used). A further 21 were left untouched per operator instruction ("leave
them, just report the finding") rather than removed unilaterally.

Separately, while committing this audit's own findings, a genuine live git collision occurred on
the shared `/root/dowiz` checkout: a concurrent Hermes autopilot process reset the checked-out
branch's ref mid-edit and, independently, had switched the working directory to a different
branch (`swave/integrate`, itself carrying real in-progress work implementing pieces of this
session's own blueprints — an LLM `AiMode` composition-switch wire-up and two Telegram-blueprint
phases) entirely outside this session's own git commands. Recovered via an isolated
`git worktree add` off the verified-good `origin/main`, redoing this document's edits there in
isolation rather than continuing to contend for the shared checkout — matching this repo's
standing worktree-collision-avoidance practice. Hermes's `swave/integrate` work and its stashed
uncommitted WIP were restored to their pre-interference state before this recovery proceeded.

---

*Audit performed 2026-07-20 against the working tree and live `main`. Sources: this session's own
2 parallel read-only verification passes (blueprint-link machine-check; fresh `cargo test` +
commit-hash verification); `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md`; `GAP-A1-DISPOSITION-AUDIT-2026-07-19.md`;
`GROUND-TRUTH-2026-07-19-FINAL.md`; `SPACE-GRADE-VERIFIED-STATUS-LEDGER-2026-07-20.md`.*
