# BLUEPRINT P-I / Layer I — Cross-Repo Consolidation (Wave 2/3, Fable, 2026-07-17)

> **Layer I of the CORE-ROADMAP** (`docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §3, row P-I),
> written against the §2 twenty-point contract (compliance map: §9). Formalizes and — uniquely among
> the layer blueprints — **executes in the same pass** the plan established by the Wave-1 audit
> `P-I-audit-cross-repo-consolidation.md` (same directory, read in full, adopted not re-derived).
> This is the phase that makes the operator's "don't revisit the same thing twice" guarantee real:
> one canonical roadmap, one navigation index, five superseded masters bannered, six would-be-lost
> items dispositioned, one numbering collision killed.
>
> **NAMING RULING (binding for all future docs): the letter axis is `Layer A..I`, not `P-A..P-I`.**
> Per audit §4: the letters are an orthogonal **altitude axis** grouping clusters of numeric phases
> (`P01–P30` — the sole execution numbering, never renamed), NOT a renumbering. On-disk filenames
> keep their `P-X` provenance names; every header and all future prose says "Layer X". The crosswalk
> table lives in `docs/design/CORE-ROADMAP-INDEX.md` and is the single anti-double-numbering artifact.

---

## Why this layer exists (context for a reader with zero session history)

Layer I is the only layer that maps to **no numeric phase** — and that is the point. It exists to
answer a navigation problem, not a build problem: this repo accumulated **six** overlapping "master
roadmap" documents across a week of planning rounds, none bannered, each plausibly claiming to be
*the* plan. A future reader (human or agent) landing on the wrong one would re-derive against stale
structure and re-litigate settled decisions — the exact waste the operator's "don't revisit the
same thing twice" rule forbids. Layer I makes that structurally impossible: it designates **one**
canonical roadmap (SOVEREIGN), banners the other five as superseded with a pointer at their very
top, disposition every would-be-lost item so nothing evaporates, and builds a single navigation
index (`CORE-ROADMAP-INDEX.md`) from which every planning doc is reachable in ≤2 hops. It also
killed a real naming collision — the "P-A..P-I" letters were renamed to "Layer A..I" so they stop
colliding lexically with the P01–P30 execution numbers (the letters are an *altitude axis* over
clusters of numbered phases, never a renumbering).

Uniquely among the layer blueprints, this one **executed in the same pass it was written** — the
banners, the index, and the fold-ins are already on disk. So a future agent's job here is
verification and maintenance (§8), not construction: add exactly one index row per new doc, never
rename P01–P30, and keep the single-canonical-root invariant true. The problem Layer I solves, in
one line: **guarantee there is exactly one place to start, and that everything is reachable from
it — so no plan is ever re-derived from a stale copy.**

---

## 1. Ground truth (contract item 1 — every claim verified live THIS pass, 2026-07-17)

> **Correction (2026-07-18, session verification pass): G5 below is now STALE — both missing layer
> blueprints have since been written.** `BLUEPRINT-P-D-consensus-capability.md` (Layer D, budgeted
> anchor-rooted issuance — reconstructed after its own pre-commit loss) and
> `BLUEPRINT-P-F-local-ai-mesh.md` (Layer F, DecisionUnit-family mesh — likewise reconstructed) are
> **both on disk** in this directory as of this pass. The `CORE-ROADMAP-INDEX.md` §2 Layer-D row
> that still reads "OPEN — not written" is corrected in place this pass to link the file (see the
> index's own dated correction). G5's "D was blocked on R-3, F on the MoE redo" remains true as
> *history* — both shipped as blueprints that are fail-closed until their respective operator gates
> (R-3 for D's production policy; the money/P06 gates for F) — but the blueprints themselves exist.
> No other G-row changed. The three lost Wave-1 audits noted in G6 have also since been
> **reconstructed on disk** (`P-D-audit-*`, `P-G-audit-*` content in BLUEPRINT-P-G §0–§1,
> `P-H-audit-*`), per the index §3 status column.

Branch `feat/p19-growth-engine`, HEAD `b64a2c1c6` (the audit ran at `f01f9bb6b`; five commits have
landed since, incl. the agentic-mesh and spectral-evolution worktree merges — re-verified, none
touches the consolidation surface).

| # | Fact | Evidence (fresh) |
|---|---|---|
| G1 | All **6** master docs exist at their audit-cited paths (root `MASTER-ROADMAP-MVP-2026-07-12.md`; `docs/design/MASTER-{BUILD-SEQUENCE-UPDATED-2026-07-11, INTEGRATION-PLAN-2026-07-14, ROADMAP-10-PHASES-2026-07-14, EXECUTION-PLAN-2026-07-13, ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16}.md`) | `ls` this pass |
| G2 | **None of the 5 older docs carried a superseded banner** before this pass | `head` of each, this pass |
| G3 | SOVEREIGN's own header names only 4 docs it supersedes — **`MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` was missing from its list** (audit §0 found the inverse omission in CORE-STANDARD §0: `MASTER-EXECUTION-PLAN` missing there) | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:11-12` pre-edit |
| G4 | Numbered blueprint files span **P01–P19** (`sovereign-roadmap-2026-07-16/BLUEPRINT-P01..P19-*.md`, 19 files); **P20–P30 exist only as standalone `BLUEPRINT-*-2026-07-17.md` / masterwork files indexed from SOVEREIGN §8.1–§8.12** — audit finding (b) independently re-verified: §8.1 (P20–P23), §8.6–§8.12 (P24–P30), every referenced file present on disk | SOVEREIGN §8 read in full this pass + `ls` |
| G5 | Wave 2 produced **6** layer blueprints on disk: `BLUEPRINT-P-{A,B,C,E,G,H}-*.md`. **Layer D and Layer F blueprints DO NOT EXIST** (repo-wide + worktree `find`, zero hits). D was blocked on the R-3 `RootDelegationPolicy` operator ruling; F on the pending MoE-specific mesh-masterwork redo | `ls`/`find` this pass |
| G6 | **NEW FINDING, this pass:** of the 4 Wave-1 audits, only `P-I-audit-cross-repo-consolidation.md` survives on disk. `P-D-audit-root-delegation-policy.md`, `P-G-audit-product-ui-post-decommission.md`, `P-H-audit-telemetry-regression-benchmarks.md` are **gone everywhere** (repo, worktrees, scratch — `find /root` zero hits) despite being quoted with line numbers by the Wave-2 blueprints written 19:44–19:48 today (`BLUEPRINT-P-E` §1 quotes `P-D-audit:131-135`; `P-G` adopts its audit's scope; `P-H` cites its audit's Areas 3–4). The dir is untracked — no git recovery path. Their load-bearing content **survives only as those embedded quotes** | `find` + blueprint headers, this pass |
| G7 | L1's primitive is **live on this branch**, stronger than the audit's "dowiz-pq lineage" hedge: `kernel/src/pq/codesign.rs` exists with `codesign_keypair` + `PinnedRoot` + tests | `kernel/src/pq/codesign.rs:99,140,162` this pass |
| G8 | L2's target `docs/transport-research-2026-07-12.md` was **absent from the working tree** (referenced by MVP `:92`, present only as git blob at `94e257fe9`, 139 lines) — **restored to the tree this pass** so the P09 cross-link resolves | `git show 94e257fe9:…` → file restored, `wc -l` = 139 |
| G9 | Both worktree-arc consolidated docs + `living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md` + `bebop2-mesh-tensor-hermetic-2026-07-17/INDEX.md` exist at their cited paths | `ls` this pass |

Ground truth is non-discussible; everything below acts on the fresh column only.

## 2. Scope — what Layer I owns (and executed)

Exactly the audit's plan, no additions:

1. **Banner pass** — prepend a SUPERSEDED banner to the **5** older masters (not 4 — audit finding
   (a)); never delete or truncate content. SOVEREIGN gets no banner (it is the target) but gets the
   G3 list correction.
2. **Targeted canonical edits** — SOVEREIGN gains an appended §9 (Layer crosswalk pointer +
   fold-in ledger), matching its own §8 append-only precedent; CORE-STANDARD §0/§3 gain the
   inventory correction + Layer-naming note. No wholesale rewrites.
3. **Fold-in execution (L1–L6)** — per audit §3's ledger, dispositions in §4 below.
4. **`docs/design/CORE-ROADMAP-INDEX.md`** — the master navigation index (8 mandated sections),
   including the Layer↔phase crosswalk.

**NOT owned:** writing the missing Layer D / Layer F blueprints (D waits on R-3; F waits on the MoE
redo — both are recorded as OPEN rows in the index, never silently linked); re-deriving any Wave-2
blueprint's content; renumbering anything in P01–P30.

## 3. Predefined types (contract item 4) — the consolidation domain model

Doc-phase types are still types; stated in Rust so a future `tools/ci-truth` docs-check can lift
them verbatim:

```rust
/// The altitude axis. Groups numeric phases; NEVER replaces them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer { A, B, C, D, E, F, G, H, I }

/// Navigation status of any planning document in the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocStatus {
    Canonical,       // exactly ONE doc may hold this: SOVEREIGN
    LayerBlueprint,  // Wave-2 output, indexed under a Layer
    Superseded,      // bannered; historical/audit-trail only
    OpenNotWritten,  // Layer D, Layer F — listed, never dead-linked
    MissingOnDisk,   // the 3 lost Wave-1 audits — recorded, never fabricated
}

/// One would-be-lost item disposition (audit §3).
pub struct FoldIn {
    pub id: &'static str,          // "L1".."L6"
    pub source: &'static str,      // old-doc cite
    pub target: &'static str,      // file edited, or index pointer
    pub executed: bool,            // this pass makes all six true
}

/// Invariant: single canonical root.
/// forall d in corpus: d.status == Canonical  =>  d.path == SOVEREIGN_PATH
pub const SOVEREIGN_PATH: &str =
    "docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md";
```

## 4. The six fold-ins — dispositions as executed (audit §3 ledger, closed)

| ID | Item | Disposition executed this pass |
|---|---|---|
| **L1** | Node self-update **code-signing** (ML-DSA vs pinned root; `kernel/src/pq/codesign.rs` live, G7) | Appended a boot/update-integrity unit note to `sovereign-roadmap-2026-07-16/BLUEPRINT-P10-hub-runtime-kill-switch-boot.md` (Phase 10 owns boot + M9 signing path) |
| **L2** | Transport bake-off **rationale** (Zenoh/Reticulum/TCPCLv4/libp2p-rejected/BIBE) | Restored `docs/transport-research-2026-07-12.md` from blob `94e257fe9` (G8) + appended one cross-link line to `BLUEPRINT-P09-confidential-self-healing-wire.md` |
| **L3** | Courier **out-of-app notification**/wake path (`NotifyHub`/VAPID, BUILD-SEQ `:132-141`) | Appended a courier-leg sub-unit note to `BLUEPRINT-P13-delivery-on-protocol.md`: dissolved-by-mesh for delivery semantics (courier node receives `MeshEvent`s directly); out-of-band device-wake remains a real P13 sub-unit, xref P08 alerting sink |
| **L4** | Anonymous **`.onion`/Tor tier** (BUILD-SEQ `:88`) | E53-form waiver entry in SOVEREIGN §9 ledger — what: anonymity/Tor access tier; why-suspended: no vendor-node tier, no demonstrated anonymity demand; trigger: vendor-node tier ships AND a venue requires anonymity |
| **L5** | "**Lost reports**" honesty ledger (13 + ~20 reports, RESOLVED-AS-LOST) | One line in SOVEREIGN §9 + index: closed-as-lost, decisions survive in `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`; no resurrection (re-creating them would violate ground-truth discipline) |
| **L6** | Self-development **research queue** (causal/do-operator, category-theory functorial, info-geometry, integer laws) | **NOT folded into P01–P30** (audit: different axis). `CORE-ROADMAP-INDEX.md` carries an explicit cross-track pointer to MEMORY → `physics-math-exploration.md` as the parallel always-running track |

Plus the G6 remediation this pass adds on its own authority: the three lost Wave-1 audits are
recorded in the index as `MissingOnDisk` with their surviving-quote locations, so no future reader
chases a dead filename or, worse, re-runs the audits believing they never happened.

## 5. DoD (contract item 2) — falsifiable, machine-checkable

All GREEN as of this pass; each is a one-liner any agent can re-run:

- **D1 (banners):** `for f in <5 old docs>; do head -4 "$f" | grep -q "SUPERSEDED" || echo RED:$f; done`
  → no RED lines. Content preserved: `git diff --stat` shows only insertions on those 5 files, zero
  deletions of existing lines.
- **D2 (index exists + 8 sections):** `grep -c '^## ' docs/design/CORE-ROADMAP-INDEX.md` ≥ 8 and it
  contains a crosswalk row for each of Layers A–I.
- **D3 (no dead links):** every relative path named in `CORE-ROADMAP-INDEX.md` outside the
  explicitly-marked `MissingOnDisk`/`OpenNotWritten` rows resolves via `ls` — including the restored
  `docs/transport-research-2026-07-12.md`.
- **D4 (single-canonical invariant):** `grep -l "canonical" docs/design/MASTER-*.md` — only SOVEREIGN
  asserts canonical status without a SUPERSEDED banner above it.
- **D5 (fold-ins executed):** L1/L2/L3 grep-able in their target blueprints
  (`grep -n "codesign" BLUEPRINT-P10*`, `grep -n "transport-research" BLUEPRINT-P09*`,
  `grep -n "out-of-band\|NotifyHub" BLUEPRINT-P13*`); L4/L5 grep-able in SOVEREIGN §9; L6 grep-able
  in the index (`grep -n "physics-math-exploration"`).

## 6. Adversarial test (contract item 5) — "no old-doc item unreachable from the new index"

**The reachability sweep (the intentionally-breaking test).** Witness set = the audit §3 ledger
(L1–L6) plus one item per old doc's covered-set spot-check (audit §2). For each witness: start at
`CORE-ROADMAP-INDEX.md`, follow ≤2 links, require arrival at a doc that states the item. The
**deliberately-failing case that proves the sweep has teeth:** before this pass, running the sweep
on L1 ("codesign") failed — `grep -rn codesign docs/design/sovereign-roadmap-2026-07-16/` had only
incidental hits and no index existed — i.e. the sweep was RED on the real corpus, and this pass
turned it GREEN. A second synthetic RED: delete the index's Layer-D row and the sweep loses the
only path to the R-3 ruling context — the row's existence is load-bearing, not decorative.
**Chaos case (link-rot):** the sweep MUST fail loudly on a dead path (that is why G6's missing
audits are typed `MissingOnDisk` instead of linked — a fabricated link would pass a grep sweep and
lie; the type makes the honest state representable).

## 7. Hazard analysis (contract item 6) — argued from structure, not policy

The hazard class of a consolidation pass is **navigation-authority ambiguity**: two documents each
plausibly claiming to be "the roadmap" (the exact state this repo was in — 6 masters, 0 banners,
G2/G3). The unsafe state is made structurally unreachable, not discouraged: after this pass every
master doc is either (a) SOVEREIGN, or (b) opens with a banner whose first line points at SOVEREIGN
+ the index — a reader physically cannot load a superseded doc without passing the pointer. The
`DocStatus` invariant (§3) has exactly one `Canonical` inhabitant; D4 is its checkable form. Per
the Monocoque/authority-boundary doctrine (`HERMETIC-ARCHITECTURE-PRINCIPLES.md` + doc 19's
finite-anchored-authority finding): authority here is finite and anchored — one root, all other
paths are references to it, and the failure mode (silent second root) is caught by a grep, not by
diligence.

## 8. Instructions for a zero-context agent (contract item 18)

Everything in §2 is already executed; a future agent's job is **verification + maintenance**:

1. Run D1–D5 (§5). Any RED → fix the drifted file, never the check.
2. Adding a new planning doc? Add exactly one row to `CORE-ROADMAP-INDEX.md` (choose Layer via the
   crosswalk semantics: which cluster of numeric phases does it serve?). A doc not in the index does
   not exist for navigation purposes.
3. Writing Layer D or Layer F for real? Flip the index row from `OPEN` to a link, cite R-3's ruling
   (D) or the MoE redo (F), and reuse the embedded P-D-audit quotes in `BLUEPRINT-P-E` §1 — do not
   re-run the lost audit from scratch without first harvesting those quotes.
4. Never rename `P01–P30`. Never reuse the "P-" prefix for a non-numeric axis (audit §4; the Layer
   ruling in this doc's header).

## 9. Contract compliance map (all 20 points, honest N/A where doc-phase)

| # | Item | Where |
|---|---|---|
| 1 | Ground truth, fresh cites | §1 (G1–G9, two NEW findings beyond the audit: G6, G8) |
| 2 | Falsifiable DoD | §5 D1–D5 |
| 3 | Spec/event-driven TDD | Adapted: §3 types precede §4 execution; the "events" are git-visible edits, each independently checkable (D1–D5 assert on the edit sequence's result) |
| 4 | Predefined types | §3 (`Layer`, `DocStatus`, `FoldIn`, invariant const) |
| 5 | Adversarial/breaking test | §6 (real pre-pass RED on L1; synthetic Layer-D-row deletion; link-rot chaos case) |
| 6 | Hazard from structure | §7 (single-`Canonical`-inhabitant invariant) |
| 7 | Links to docs & memory | Throughout; the index IS this item, promoted to an artifact; MEMORY pointers: `physics-math-exploration.md` (L6), `sovereign-architecture-19-phase-roadmap-2026-07-17.md` (P06 blocker) |
| 8 | Scaling axis | Index scales in doc-count: flat tables to ~50 rows/section; past that, split per-Layer index files and demote this to a root index — stated breakpoint, not timeless |
| 9 | Linux-discipline verdict | REINFORCES: this is the kernel-tree `MAINTAINERS`/`Documentation/index` pattern — one authoritative index, historical docs never deleted |
| 10 | Benchmarks + telemetry | N/A-with-reason: no hot path. The regression surface is D1–D5 (cheap greps, CI-liftable) |
| 11 | Bulkhead/isolation | Edits are append/prepend-only per file; a bad banner cannot corrupt content below it; SOVEREIGN §9 is appended, matching §8's own precedent — no shared-mutable rewrite |
| 12 | Mesh awareness | N/A — node-local docs; nothing gossip-propagated |
| 13 | Rollback/self-healing math | Rollback = `git checkout` per file (every change additive ⇒ trivially reversible); Snapshot Re-entry class: the superseded docs ARE retained snapshots of prior planning epochs |
| 14 | Smart index / mistake gate | D1–D5 as a future `ci-truth docs-check` (named option, not built — YAGNI until first drift); the `DocStatus` type turns "fabricated link" into a representable, catchable state |
| 15 | Living-memory awareness | L6 disposition: the self-development axis lives in MEMORY, index points across, never duplicates |
| 16 | Tensor/spectral reuse | N/A — no numeric content |
| 17 | Regression tracking | D1–D5 are the named permanent checks; ledger row proposed for `docs/regressions/REGRESSION-LEDGER.md` under "docs-nav" class if drift ever recurs |
| 18 | Zero-context executability | §8 |
| 19 | Reuse-first | SOVEREIGN §8 append pattern reused for §9; MEMORY's existing arc lines reused for EXECUTION-PLAN's 9 sub-plans (audit §2.5) instead of a new sub-index; banners reuse SOVEREIGN's own supersession language |
| 20 | Hermetic principles | §7 — finite anchored authority (one canonical root); correspondence (index mirrors the corpus's real structure incl. its holes: `OpenNotWritten`, `MissingOnDisk`) |

---

*Written and executed 2026-07-17 on `feat/p19-growth-engine` (HEAD `b64a2c1c6`) as the Wave-3
consolidation pass. Companion artifacts landed the same pass: 5 banners, SOVEREIGN header fix + §9,
CORE-STANDARD §0/§3 corrections, L1/L2/L3 blueprint fold-ins, the restored transport-research doc,
and `docs/design/CORE-ROADMAP-INDEX.md`.*
