# Execution Roadmap — Space-Grade Kernel Synthesis, Items 1–32, Dependency-Ordered

**Source:** `docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` (commit `10164bd74`).
**Sorting rule:** actual technical dependency, lowest first — not the document's topic order. The
two dependencies the source states explicitly — **item 21 strictly after item 9** (§16(c)) and
**item 23 after item 22** (§17(b) addendum) — are preserved verbatim and never resequenced. Every
other ordering choice below that is not explicit in the source is flagged **[new ordering choice]**
with its reason.

---

## 0. Operator rulings — recorded 2026-07-19, same day as the synthesis

All five open decision gates were presented and ruled on the same day:

| Gate | Source | Ruling |
|---|---|---|
| GCRA lock-free TokenBucket swap | §1.3 / item 8 | **ADOPT** — gated behind the differential oracle + Kani interleaving proof already scoped in item 8; built and tested before it ships. |
| Mesh integration approach | §17(d) / item 22 | **REIMPLEMENT IN DOWIZ, ZERO-DEP** — bebop's proven mesh-node/proto-wire/proto-cap serves as design reference/parity oracle only, not a linked dependency. The same ruling covers `agent-governance-wasm`'s `bebop2-core` path-dep and `mesh-adapter`'s sibling paths per §25's table. |
| Optical/pixel context compression | §20(c) / item 28 | **PURSUE** — model-weight dependencies are ruled outside §0's compiled-Rust-crate scope; archival/display-plane content only, never the P0/P1 determinism planes (§10/P6). |
| ARINC-653-style scheduler | item 11 | **PURSUE, design-only** — Phase 0 (design doc + TLC model), no code until the breaker (item 9) exists, per the source's own restriction. |
| SIHFT triple-vote pilot | item 12 | **PURSUE, design-only for now** — needs the breaker + FDR to exist first regardless of the ruling; design/scoping work can start. |
| eqc indexed-summation IR extension | item 32 | **PURSUE** — extend eqc's `Expr` language to support the Laplacian's neighbor-sum operator, not just scalar control laws. |

The Laplacian reimplement-vs-vendor fork (§14(d)) is **not a live gate** — §26(d)'s correction found
`laplacian_spmv` already exists in-kernel at `csr.rs:552`; the surviving work is the parity pin
(item 18, Tier 0 below) and reconciling bebop's `step_wave` as a third representation, not a
build-vs-vendor choice. The §27 frequency/wave-domain communication idea remains parked — no
research has been done, and the source document itself requires a research pass before its own
gate applies.

---

## A. Tier 0 — zero prerequisites, read-only or self-contained on already-tested surfaces. READY NOW.

Nothing here depends on any other item; each is pure investigation or a small change under existing
test coverage.

- **Item 2** — `FileEventStore` wiring verification. Dual check: (a) is the durable store constructed
  anywhere, (b) has the `Result`-typed `insert` fix landed since 07-16 (§10/P4 — "a wired store that
  swallows IO is arguably worse than an unwired one"). Highest consequence-per-cost in the roadmap.
  **✅ RESOLVED-AS-DEFECT-FILED 2026-07-19** (re-verified adversarially against live `HEAD`): **(b) PASSES**
  (typed `StoreError` propagation confirmed, fix `4dec04218`, regression test `hydra.rs:1188-1218`);
  **(a) FAILS** — no production composition root constructs the durable store (all 6 `FileEventStore::open`
  sites are test-only; no binary builds a durable `Hydra`/`EventLog`). Defect filed:
  [`BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md`](BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md)
  (verification: [`BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md`](BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md)).
  Fix scoped there as a follow-up Tier-1 build item (§B territory) — NOT built (Tier-0 read-only audit).
- **Item 3** — `order_machine` const-adjacency + `idx_of` dedup. Golden signature and 1e-12 oracle
  already cover it.
- **Item 30** — state-machine proliferation audit (`capability_cert.rs`, `hub_provisioning.rs`,
  `hub_supervisor.rs`, `hydra.rs`). Read-only table. **✅ CLOSED 2026-07-19** —
  `AUDIT-ITEM-30-state-machine-final-2026-07-19.md`: all 4 modules INDEPENDENT (0 shared with the
  FSM proof kit), 4 PARITY-PIN tickets (I30-T1..T4, 0 collapses forced). **1 confirmed silent
  defect** (I30-D1, `resume()` owner-zeroing) fixed with a red→green guard on
  `exec/space-grade-tier0-2026-07-19` (`707848dfd`); the in-session "2 confirmed silent defects"
  phrase confirmed UNSOURCED.
- **Item 15** — eigen-surface entry-point + parity-scope verification. Read-only; defect filed only
  if found. **✅ AUDITED 2026-07-19** — single eigen-surface HOLDS (`spectral.rs:225 eigenvalues` →
  `householder::eigenvalues_contig`, no `lowrank.rs`); gap = R3 parity is values + dominant-residual
  only (`spectral.rs:1254 let _ = dvecs;`). Ticket **I15-T1** (vector-scope cross-solver pin) filed
  in [`AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md`](AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md);
  not built (new scope).
- **Item 16** — `GraphSpectrum` single-spectrum audit. Read-only unless a P2 defect forces collapse.
  **✅ RESOLVED-BY-REFACTOR 2026-07-19** — P2 CONFIRMED (`graph_spectrum` computed the adjacency
  spectrum 3×, `graph_energy_report` 4×, both claiming "single pass"). Collapse LANDED, option (b)
  (internal, zero public-signature change): `classify_drift_with_rho` + shared `drift_guards_ok`/
  `drift_band`; `graph_spectrum` now = exactly 2 passes (adj + Laplacian), `graph_energy_report`
  4→2. Proof = thread-local `EIGEN_CALLS` exactly-2 counter + field-consistency test. Kernel suite
  **902 / 0 / 3** (was 899). Committed `e125f0c97`, pushed to `exec/space-grade-tier0-2026-07-19`.
  Resolution note: `AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md` §0.
- **Item 17** — `engine` thick/thin classification table (RC-4's three mirrored items as first
  entries). **✅ AUDITED 2026-07-19** — RC-4 triple: `DriftClass` + `dt` CLOSED (pinned post-H2);
  **L-operator OPEN** (`engine/src/field_frame.rs:10-40` engine-side 5-point Neumann Laplacian
  unpinned to kernel `csr.rs:552 laplacian_spmv`). Ticket **I17-T1** (engine-boundary Laplacian pin;
  cross-references item 18's intra-kernel pin, does NOT duplicate it) filed in the tickets doc; not
  built (new scope).
- **Item 19** — retrieval spectral-routing audit (`diffusion.rs`/`ppr.rs`). Read-only.
  **✅ AUDITED 2026-07-19** — independent-by-design (zero `spectral`/`GraphSpectrum` refs;
  `ppr.rs:6-7` "No eigendecomposition"), correctly so — NOT the second GraphSpectrum consumer. New
  smell: `ppr.rs:3-5` is a comment-bound unpinned mirror of `markov.rs:162-170`'s inner loop, no
  test pin. Ticket **I19-T1** = parity-pin (NOT collapse — `retrieval/mod.rs:14` red-lines touching
  `markov.rs`) filed in the tickets doc; not built (new scope).
- **Item 22 (verification half only)** — read `mesh.rs`, classify real-port vs stub. The ruling is
  now recorded (§0 above: reimplement), so this verification informs HOW MUCH of `mesh.rs` is
  reusable scaffolding versus needs building from scratch, not whether to proceed.
  **✅ VERIFICATION COMPLETE 2026-07-19** — proof filed
  [`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`](AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md)
  (classification table, one row per public symbol, file:line + caller-or-NONE + verdict; blueprint
  independently re-verified and CONFIRMED). Finding: `mesh.rs` (387 lines, `#[cfg(feature="pq")]`,
  `pq` NOT default) is a real, tested ML-DSA-65 signed-log primitive with **ZERO production kernel
  callers** — `MeshLog`/`MlDsaSigner`/`Signer` bench-only, `SignedEntry`/`MeshError`/`HubTransport`
  uncalled, protocol layer absent; even `mesh-adapter` bypasses it (bebop path-deps, no `pq`).
  **Scoping handoff to item 23 (gated strictly after — NOT started here):** it is **"mostly stub
  above the log layer"** — reuse `SignedEntry`/`MeshLog`/`MlDsaSigner` + the `HubTransport` seam
  as-is (keep, don't rewrite), but sync/consensus/capability/gossip start near-scratch; gossip
  admission must extend `decision/import_unit()`, never fork a parallel importer (synthesis §17(b)).
- **Item 18 (narrowed)** — the Laplacian parity pin: dense `laplacian()` ↔ `csr.rs:552
  laplacian_spmv`, plus a `step_wave` reconciliation note. **[new ordering choice]** — promoted from
  mid-roadmap to Tier 0 because §26(d) shrank it to one parity test against an oracle already in-tree.
- **Item 31 (investigative half)** — `rusqlite` usage read + reclassification; pin the
  `cosmic-text = "*"` wildcard; verify `sha2`-vs-kernel-keccak on the body digest.
  **✅ INVESTIGATIVE HALF COMPLETE 2026-07-19** — findings filed
  [`AUDIT-ITEM-31-dependency-findings-2026-07-19.md`](AUDIT-ITEM-31-dependency-findings-2026-07-19.md)
  (blueprint independently re-verified). **(a) rusqlite** → KEEP-and-contain (cat-2 foreign format,
  Hermes `state.db` only, no default build path) — docs-only ruling. **(b) cosmic-text `*`** →
  DEFECT CONFIRMED + FIXED: pinned to already-resolved `0.19.0` (`engine/Cargo.toml:30`), lockfile
  unchanged, `cargo check --features text` green, committed `c2d0f306a` on
  `exec/space-grade-tier0-2026-07-19`. **(c) sha2 vs keccak** → NOT a defect, KEEP `sha2`; blueprint
  CORRECTED (`pub mod pq` is `#[cfg(feature="pq")]`-gated, not "already linked" — the swap would
  pull `aes-gcm`+`curve25519-dalek`, a net dep increase). Bonus flag confirmed: **dual in-kernel
  Keccak-f[1600]** (`event_log.rs:67` vs `pq/keccak.rs:156`) — dedup ticket owed to item 25, filed
  not fixed. Enactment half (Tier 2) allowlists rusqlite+sha2.

## B. Tier 1 — foundational builds. ✅ COMPLETE (2026-07-19) — all items DONE; the kernel's default no-dev build has ZERO external crates.

- **Items 1 + 13 combined** — the CI zero-dep gate, born deterministic:
  `cargo tree -e no-dev --locked --offline` + lockfile-hash assertion, 3-crate allowlist shrinking
  monotonically. **[new ordering choice — bundling]**: item 13 hardens item 1's own mechanism;
  building it nondeterministic first is two passes over one CI job.
  **✅ DONE (2026-07-19)** — `kernel/ZERO-DEP-ALLOWLIST.txt` + `scripts/zero-dep-gate.sh` (3 gates:
  tree⊆allowlist, monotonic-shrink, `Cargo.lock` sha256) + `zero-dep-gate` CI job under `unshare -n`;
  all §G.7 clauses red-proven; `01acd673e` on `exec/space-grade-tier0-2026-07-19`. See §G.7 for detail.
- **Item 14** — `rust-toolchain.toml` pin + structural compiler-bump trigger. Independent, parallel.
  **✅ DONE 2026-07-19** (commit `bb1e9e8dc`, `exec/space-grade-tier0-2026-07-19`) — root
  `rust-toolchain.toml` pins `channel="1.96.1"` (exact, verified = dev-box toolchain; no pin existed
  pre-change, CI floated on runner stable); `toolchain-bump-gate` job added to `ci.yml` (always-runs,
  required-check safe, enforcement fires only on a `channel`-value change and then requires
  `docs/audits/toolchain/spot-check-<new>.md` w/ both mandated headings in the same diff — pin's own
  intro = `<absent>→1.96.1`, so it carries the baseline `spot-check-1.96.1.md`). Baseline artifact is
  HONEST: real source-level constant-time audit of all 6 pq surfaces (flags the pre-existing,
  compiler-independent variable-time `!=` FO tag-compares in `kem.rs`/`hybrid.rs`, owed to P91.2),
  assembly audit PARTIAL with the full per-branch taint proof DEFERRED to Tier 2 item 7 (Kani) — no
  fabricated clean claim. Proofs: kernel `cargo test` 902/0/3, engine 117/0, gate logic 6/6 +
  end-to-end `git show BASE:$FILE` extraction test (maps 1:1 onto §G.8). Owed (G5): flip the gate to
  a required status check in branch protection (server-side).
- **Item 25 (procedure doc first) — ✅ DONE (2026-07-19).** The slot-arena/qrng standing procedure
  is codified and independently re-verified in
  [`PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md`](PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md)
  — all citations checked against live HEAD, both precedents (slot-arena override, QRNG
  never-replace) confirmed. **This doc is now BINDING**: Items 4+29 (`tracing`/`tracing-subscriber`
  logger/FDR rewrite, incl. the `telemetry` `SpanMetricsLayer` consumer) and Item 5 (`regex`
  retirement) MUST run its numbered 10-step ruling per crate before cutover (§18(a)'s "under this
  exact procedure, not a bespoke one"). **Re-verification finding (new scope, owed ticket, not an
  item-25 fix):** the `qrng` feature is undeclared in `kernel/Cargo.toml`, so the QRNG provider
  (incl. its sanctioned `master_seed()`) is dead code that never compiles — a
  standing-rule-vs-reality inconsistency filed in the procedure doc §3.
- **Items 4 + 29 combined, with the §1.2 `JsonWriter` absorbed in the same change — ✅ DONE
  (2026-07-19).** The hand-rolled logger/FDR tier-(b) buffer with the energy/hardware field set
  first-class in the schema from day one. Both bundlings are the source document's own explicit
  mandate (§21, §10/P2). The largest Tier-1 item; the keystone of the tier. Landed as three isolated
  commits on `exec/space-grade-tier0-2026-07-19` (`f04142f89` build → `4f4872a54` flip →
  `eb350464e` remove): `kernel/src/fdr/` (json/schema/ring/macros/mod) coexisted, then the 13 call
  sites + `SpanMetricsLayer`→`SpanMetricsObserver` (a kernel `fdr::SpanObserver`) flipped, then
  `tracing`/`tracing-subscriber` removed. Proofs discharged: `cargo tree -e no-dev` 25→6 crates (**19
  dropped**, exceeds ≥13); `metric.jsonl` + markov CLI JSON byte-identical before/after (golden-pinned);
  kill-9→restart→recover test (real child SIGKILLed, 300/300 events recovered + PostMortem emitted);
  `hw` first-class with `joules_uj` reporting `unavailable:no_rapl_interface` (named absence) on this
  RAPL-less host; duplicate `mldsa_verify` wrapper deduped; wasm32 cdylib green (`Instant` gated off
  wasm); full kernel suite 938 passed / 0 failed; `scripts/zero-dep-gate.sh` GREEN (5 external crates,
  allowlist shrunk by 19). Ruling recorded in `fdr/mod.rs` doc + `kernel/Cargo.toml` + the blueprint
  ([`BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite-2026-07-19.md`](BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite-2026-07-19.md)).
- **Item 5 — retire `regex`, after the logger exists. ✅ DONE (2026-07-19) — CLOSES ALL OF TIER 1.**
  The kernel's last external crate. Its entire production surface was one function
  (`TrigramIndex::query_regex`) with **zero production callers** (re-verified by full-workspace
  grep across `kernel/ engine/ apps/ tools/ agent-loop/ agent-adapters/`); the only pattern ever
  compiled anywhere was `note-.*-recall`. Ruling per item 25's procedure = terminal state (a)
  removed outright, replaced by a kernel-owned restricted matcher for the used subset
  ({literal, `.`, `.*`}, unanchored contains-match, greedy leftmost segment placement — no
  backtracking exists ⇒ no pathological blowup), with typed rejection (`PatternError::UnsupportedMeta`)
  of every other metacharacter (degrade-closed). Landed as three isolated commits on
  `exec/space-grade-tier0-2026-07-19` (`18152ef84` build → `c6b5d2176` flip → `6605166cd` remove):
  `kernel/src/retrieval/pattern.rs` + `query_pattern` coexisted, then the seam flipped, then
  `regex = "1"` was removed. Proofs discharged: parity proven BEFORE cutover — differential vs the
  live `regex` crate over the 20-doc FIXTURE + 2000-doc synthetic corpus + a proptest sweep (random
  subset patterns × ASCII docs), all bit-identical; a permanent independent naive recursive
  reference matcher + a frozen golden (`query_pattern("note-.*-recall") == vec![7]`) carry the
  guarantee post-removal; rejection tests assert typed errors with byte positions.
  `cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked --offline` = **`dowiz-kernel`
  root ONLY, ZERO external crates** (regex's whole subtree — regex, regex-automata, regex-syntax,
  aho-corasick, memchr — dropped; regex survives only as a `criterion` dev-dep transitive in
  `Cargo.lock`, outside the no-dev proof surface). `ZERO-DEP-ALLOWLIST.txt` shrunk 5 → 0;
  `scripts/zero-dep-gate.sh` GREEN "0 external crates" (also fixed a latent gate abort at the true
  zero-dep end state — its filter greps returned exit 1 when they filtered every line out, aborting
  under `set -euo pipefail`; now `|| true`-guarded, gate A/B/C semantics unchanged). Full kernel
  suite green (925 lib unit tests / 0 failed / 3 ignored, +22 integration). Ruling recorded in
  `pattern.rs` module doc + `kernel/Cargo.toml` tombstone + allowlist header + `fdr/mod.rs` +
  `lib.rs`/`retrieval/mod.rs`, and the blueprint
  ([`BLUEPRINT-ITEM-05-regex-retirement-2026-07-19.md`](BLUEPRINT-ITEM-05-regex-retirement-2026-07-19.md)).
  **With this, every §B Tier-1 item (1+13, 14, 25, 4+29, 5) is DONE: the kernel's default build has
  genuinely zero external dependencies.**

## C. Tier 2 — process/verification layer. Parallelizable.

- **Item 6** — §4 hardening checklist codified + CI enforcement, with §10/P7's correction built in:
  CI must re-execute oracles and dudect self-tests, never presence-check artifacts.
  **✅ DONE 2026-07-19** (`ae4964e61`, branch `exec/space-grade-tier0-2026-07-19`). Real CI config +
  a new dudect harness landed. Three deliverables: `docs/audits/hardening/CHECKLIST.md` (standing
  law), `docs/audits/hardening/HOT-PATHS.tsv` (machine-read manifest — 14 rows seeded from the real
  surfaces: pq/dsa+kat, pq/keccak, event_log Keccak-copy-B, pq/x25519, pq/kem, pq/hybrid,
  order_machine FSM, householder+spectral eigen, token_bucket, retrieval/pattern, fdr/json, ct_gate),
  and the `hardening-gate` CI job (`scripts/hardening-gate.sh`). The gate **re-executes, never
  presence-checks** (§10/P7): every verdict is a live `cargo test` exit code + the PARSED `N passed`
  count asserted `>= min_tests`; a filter matching **zero** tests is RED (anti-forgery core). **RED/
  RED/GREEN proven with real output:** (a) a diff touching a hot ZONE with no manifest row → exit 1;
  (b) a manifest row whose filter matches zero tests → exit 1; (c) my own commit's diff (touching 3
  registered rows) → exit 0. **Independent-verification CORRECTION to the blueprint's premise:** the
  cited pq KATs (ACVP/Keccak/x25519/KEM/hybrid) do **NOT** re-execute in the default `cargo-test` job
  — `pq` is not a default feature, so `cargo test --offline` never compiles them; they were **dark in
  CI**. The gate's unconditional oracle floor now runs them with `--features pq` every build, closing
  that gap. **dudect (honest gap — built):** `kernel/src/ct_gate.rs`, a zero-dep Welch-t harness + a
  reusable `ct_eq` constant-time primitive + a **planted-leak self-test** (variable-time `naive_eq`
  detected at |t|≈300+, `ct_eq` |t|<1.3, separation >290×) run in release in the gate step. **item 3
  (debug_assert differential):** wired for `order_machine::assert_transition` (slice-vs-`FSM_ADJ`
  dual-representation) and `householder::eig2x2` (Vieta trace/det) as the pattern; corpus-oracle rows
  carry `N/A(corpus-oracle)`. **Scoped vs deferred (ledgered in the manifest's own `gap` column):**
  dudect crypto-surface coverage → items 7/8; `kem.rs`/`hybrid.rs` variable-time tag compares are
  `KNOWN-RED(P91.2)` (NOT fixed here — the CT fix is the gate's first customer); `token_bucket` GCRA
  differential oracle → item 8; item-4 exhaustive assembly → item 7 (Kani). Full kernel suite
  **955/0/8** at the commit. Docs (this roadmap + CORE-ROADMAP-INDEX) pushed to `origin/main`.
- **Item 7** — Kani wiring (Keccak, FSM graph algorithms, NTT arithmetic, GCRA transition — now
  applies to the adopted GCRA, §0 above).
- **Item 8** — GCRA decision package. **Ruling: ADOPT (§0 above).** Differential oracle + Kani
  interleaving check now execute toward a real swap, not just an evidence package.
- **Item 31 (enactment half)** — per-crate allowlist CI gate + shared kernel-side JSON-parse
  primitive for the serde carriers + manifest-recorded rulings. Depends on items 1 and 25.
  **✅ DONE 2026-07-19 — real CI config + kernel module landed** on `exec/space-grade-tier0-2026-07-19`
  (`ae2da4a9d` gate → `dd6876a73` json+oracle → `c64ca923b` cutover). **Four blueprint claims
  independently re-verified, TWO corrected:**
  - **Workspace = 26 crates** (not the synthesis's 20; the six `tools/telemetry/*` were missed) —
    confirmed. **12 already zero-external-dep** by default.
  - **Gate**: `scripts/zero-dep-gate.sh` parametrized `[<crate-dir>]` (no-arg = kernel,
    backward-compatible); path-dep filter generalized to `grep -v ' (/'` (verified against real
    `cargo tree --prefix none` — root + every path dep render with an abs path in parens). Added
    `scripts/zero-dep-crates.txt` (24-crate roster) + `<crate>/ZERO-DEP-ALLOWLIST.txt` × 25 (12 empty
    floors, 13 frozen closures with item-25 ruling headers). CI `zero-dep-gate` job loops the roster
    under one `unshare -n`; **mesh-adapter** gate rides its existing dual-checkout job (relative bebop
    path); **agent-governance-wasm EXCLUDED** (absolute-path `/root/bebop-repo` dep — CI-unresolvable,
    filed as its own portability defect). **Proof**: full roster GREEN 24/24 (5×); Gate A RED on an
    injected unlisted dep (`cfg-if`→`tools/eqc-rs`), GREEN on revert; Gate C lockfile-hash stable.
    (Also regenerated 10 downstream `Cargo.lock`, removals-only — pruning the regex/tracing closure the
    kernel dropped in items 4/5/29 so `--locked` resolves.) A subtle CI-poison bug was root-caused +
    fixed: any FAILING `git origin/main:<untracked-path>` access corrupts cargo's next `rustc -` target
    probe in a shared `.git`; Gate B now probes with a no-pathspec `git ls-tree`.
  - **Serde carriers = NINE** (not seven: + `rust-spool`, + `topics`) — confirmed.
  - **JSON primitive — HONEST SCOPE-DOWN**: built `kernel::json` (always-compiled, pure-std, bounded
    recursive-descent RFC 8259 parser + serializer, degrade-closed), SEPARATE from `fdr::json`
    (serialize-only). `serde_json` kept as a **dev-dep differential oracle** (outside the `-e no-dev`
    surface → kernel allowlist stays empty). Oracle: 50-item real-carrier corpus (all 50 agree, 31
    accept / 19 reject, 31 round-trip) + a 2000-case proptest fuzz over the carriers' real number/
    string/nesting distribution. **Phase-A cutover of the carriers that BOTH shrink the tree AND are
    a sound cutover: `agent-facade` (11→0 ext deps) + `skillspector-rs` (15→5).** Serde carriers
    **9 → 7** (a real decrease). **Correction to the blueprint's projected 3rd (wasm)**: verified NOT
    Phase-A — its `a11y_build_mirror` site (de)serializes the SHARED `dowiz-engine` `SemanticScene`/
    `A11yTree` through engine's `serde` feature; cutting it would couple `kernel::json` to engine's
    schema. **wasm deferred to Phase-B**, reopening trigger: engine exposes a serde-free codec.
    `native-spa-server`/`llm-adapters`/`async-spool` deferred — **verified via `cargo tree -i
    serde_json` that removing the direct dep shrinks NOTHING** (axum's default `json` + ureq's `json`
    feature retain `serde_json`); reopen only if those framework json features go optional.
  - **`rust-spool` deletion — DEFERRED (corrects the blueprint's "referenced by nothing")**:
    independent grep found `tools/telemetry/lib.sh:37` hardcodes + `tg_spool_ensure` LAUNCHES
    `rust-spool/target/release/telemetry-spool` as the LIVE Telegram telemetry drainer. Deleting it
    would break the live pipeline. Retire only after `async-spool` is deployed + `lib.sh` cut over.
- **Item 26** — batching research pass. Zero prerequisites; scheduled low-priority, measurement-only.
  **✅ DONE 2026-07-19** — real measurements landed:
  [`AUDIT-ITEM-26-batching-measurements-2026-07-19.md`](AUDIT-ITEM-26-batching-measurements-2026-07-19.md).
  Inventory re-verified (all §1 citations accurate). M1 event-log commit: p50 637 µs / p99 1343 µs /
  1,513 ev/s, **exactly 1 fsync+open+close per event** (strace) — group-commit worth **~53×** at
  batch-64 but changes the crash contract ⇒ *operator-gated opt-in, not a default*. M2 FDR ring:
  normal 2.56 µs vs alarm-fsync 571 µs (~148×); 1 MiB→4 MiB cap buys only ~11% ⇒ **KEEP AS-IS**
  (design already amortizes fsync over a segment; baseline now on record). M3 `import_unit`:
  0.87 µs p50 / ~0.6 ns-per-case marginal ⇒ **measured DON'T-BATCH**. M4 skipped per its own gate
  (allocation is noise). **PMU unavailable** (`perf_event_paranoid=4`, no `perf`) — wall-clock +
  `strace -c` fallback, no fabricated counters. No batching code landed (scope law held).
  Scaffolding (bench + `#[ignore]` probe) on `exec/space-grade-tier0-2026-07-19`.
- **Item 27 (classifier-input half)** — ✅ **DONE** (`03887462a`, branch
  `exec/space-grade-tier0-2026-07-19`). PMU counters now ride alongside every `Verdict`/`DriftClass`
  emission as an FDR companion, WITHOUT touching either classifier. New `kernel/src/fdr/pmu.rs`:
  `PmuStamp` (all `Reading<u64>`), a sibling of `HwStamp` on the same `Reading<T>`/`Absence`
  machinery. **Tier A** (`rdtsc` + `/proc/self/stat` minflt/majflt/nswap + `/proc/self/status`
  ctxt-switches) reads real data with zero permissions. **Tier B** (instructions/cycles/
  cache-misses/branch-misses) via a hand-rolled zero-dep `perf_event_open(2)` raw syscall (`asm!`),
  every failure mode degrading to a named `Absence` (new `NoPmuInterface` variant; EPERM/EACCES →
  `PermissionDenied`) — never a fabricated 0, never a panic. Wired via `PmuStation::bracket`: the
  `markov_attractor` bin window-brackets `analyze_detailed` and logs ONE `markov_verdict` `FdrEvent`
  carrying `verdict_str()` + the PMU delta on the SAME record (optional `pmu` field, absent
  elsewhere so all other FDR records stay byte-identical). `analyze_detailed`/`classify_drift` stay
  pure (P6 preserved). Diagnostic-grade; NO CI gate keyed to any PMU value. 6 `fdr::pmu` unit tests
  + 1 end-to-end integration test (spawns the real bin, recovers the real FDR ring) green; full
  kernel suite 955 passed / 0 failed.
  - **Independent-verification correction to §C/line 212's "PMU unavailable" premise:** the
    self-management agent process runs as **root with `CAP_PERFMON`/`CAP_SYS_ADMIN`**, which
    **bypasses `perf_event_paranoid=4` entirely** — so Tier B `perf_event_open` actually SUCCEEDS in
    that context and returns real hardware counters (measured live: IPC ≈ 3.7, instructions/cycles/
    cache-miss/branch-miss all real, hardware-plausible). A genuinely *unprivileged* process on this
    host would still see `permission_denied`; that named-absence path is proven deterministically
    (errno-table + forced-absence serialization tests) rather than relying on the live privilege
    level. **Operator note (informational, non-blocking):** for the unprivileged production path,
    `sysctl kernel.perf_event_paranoid=2` OR granting `CAP_PERFMON` (kernel ≥5.8) to the kernel's
    process would unlock Tier B's real IPC/cache-miss data there too — a host-level knob, flagged
    here for awareness, no decision required for this half to stand.
  - The autonomic-**response** half stays routed to Tier 4 (below) per the source's own requirement —
    gated on item 9 (breaker) + item 21 (gain-scheduling); untouched here.

## D. Tier 3 — THE PIVOT.

- **Item 9** — build `kernel/src/breaker/` from Blueprint A under the §1.5/§10-P4 standard (typed
  `Result<Permit, Tripped>`, unconstructible tripped-but-permitting state, `CommitError` alarms
  routed in). **The pivot point of the entire roadmap** — items 11, 12, 21, 27(response), and 32
  (control-law half) all sit behind it. Best entered after item 2's finding and Tier 1's FDR.
- **Item 10** — TLA+ spec of decision-import + order FSM. No structural dependency on the breaker;
  same-tier verification of the same state-machine family, runs in parallel with item 9.

## E. Tier 4 — gated on the breaker.

- **Item 21** — autonomic gain-scheduling module. Explicit stated dependency: strictly after item 9.
- **Item 11** — ARINC-653 scheduler Phase 0 (design doc + TLC model only). **Ruling: PURSUE,
  design-only (§0 above)** — can start now as a design artifact; the model itself doesn't need the
  breaker to exist, only the eventual code does ("code comes only after the breaker exists").
- **Item 12** — SIHFT triple-vote pilot. **Ruling: PURSUE, design-only for now (§0 above)** — the
  pilot itself needs breaker + FDR; scoping/design work can start immediately.
- **Item 27 (response half)** — after item 21.
- **Item 32 (split)** — Laplacian half already lands with item 18 (Tier 0). **Ruling: PURSUE the IR
  extension (§0 above)** — this can start as its own eqc-rs capability work, independent of the
  breaker; only the §16 pilot-control-law half needs items 9 + 21.

## F. Parallel lanes

- **Spectral/physics lane:** item 18 (Tier 0, narrowed) → item 32's Laplacian half (also Tier 0/now).
  eqc IR extension (item 32, ruled PURSUE) runs alongside, independent.
- **Living-memory lane:** item 19 (audit) → **item 20** (P95 persistence — genuinely open,
  externally ungated, READY now) → **item 28** (optical compression — **ruled PURSUE**, pilot scoped
  to the archival plane only, sequenced after item 20 since it consumes the same durability
  machinery).
- **Mesh/gossip lane:** **item 22** (verification, READY) → reimplementation work (per the §0 ruling,
  not a vendor integration) → **item 23** (explicit stated dependency: after item 22 — preserved
  exactly; extends `import_unit()`, no parallel importer) → **item 24** (crypto surfaces under §4 —
  depends on item 6's re-executing CI machinery and item 14's trigger).

---

## G. Garden of Eden — Recommended First Execution Batch, hand to Opus now, in this order

1. **Item 2** — Proof: a cited line constructing the durable store in production, or a filed defect;
   plus the §10/P4 check on `Result`-typed `insert`. **✅ DONE 2026-07-19 — defect filed** (no production
   construction site exists; (b) `Result`-typed insert confirmed): `BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md`.
2. **Item 30** — Proof: a table, one row per module, citing file:line for shared-vs-independent
   state-machine logic; every independent one gets a collapse-or-parity-pin ticket.
   **✅ DONE** — proof table + 4 parity-pin tickets in `AUDIT-ITEM-30-state-machine-final-2026-07-19.md`;
   1 confirmed defect (`resume()` owner-zeroing) fixed (`707848dfd`).
3. **Items 15, 16, 19, 17** (read-only audits, any order) — Proofs verbatim from the source: single
   backend + named parity test cited by file:line or P2 defect filed; one eigenvalue computation
   feeding all functionals; shared backend cited or defect filed; every public `engine` item
   classified with RC-4 as first three entries.
4. **Item 22 (verification half)** — Proof: the classification cited by file:line — typed boundary
   plus real kernel caller, or no-caller finding filed. Now feeds directly into reimplementation
   scoping (§0 ruling), not a decision package. **✅ DONE 2026-07-19** —
   [`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`](AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md):
   no-caller finding filed (zero production callers; `MeshLog`/`MlDsaSigner`/`Signer` bench-only,
   `SignedEntry`/`MeshError`/`HubTransport` uncalled). "Mostly stub above the log layer" — see §A.
5. **Item 3** — Proof: zero heap allocations under a counting allocator test; one `idx_of`
   definition; golden signature and 1e-12 oracle both green.
6. **Item 18 (narrowed)** — Proof: a parity test computing Lu via dense `laplacian()` and via
   `laplacian_spmv` — exhaustive over small graphs plus a large randomized corpus — green to float
   epsilon; `cargo tree` unchanged.
7. **Items 1+13** — Proof: CI fails on any new dependency, allowlist shrinks monotonically; gate
   verdict identical with networking disabled, lockfile hash unchanged.
   **✅ DONE 2026-07-19** — baseline re-verified (`cargo tree -e no-dev --locked --offline` = exactly
   24 external crates, matches the blueprint). Landed `kernel/ZERO-DEP-ALLOWLIST.txt` (24 names),
   `scripts/zero-dep-gate.sh` (Gate A tree⊆allowlist / Gate B `comm -13` monotonic-shrink vs
   `origin/main` / Gate C `Cargo.lock` sha256 stable), and the `zero-dep-gate` CI job running under
   `unshare -n`. All four §G.7 clauses red-proven: Gate A RED on a throwaway `libc` dep, Gate B RED on
   a grown allowlist + GREEN on a shrink, `unshare -r -n`/`unshare -n` identical 24-crate verdict.
   Committed `01acd673e`, pushed to `exec/space-grade-tier0-2026-07-19`. Blueprint:
   `BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md`. Scope held to `dowiz-kernel` (item 31 = Tier 2).
8. **Item 14** — Proof: a toolchain-bump diff without the spot-check artifact fails CI; a non-bump
   diff never triggers the job. **✅ DONE 2026-07-19** (`bb1e9e8dc`) — proof discharged: gate logic
   unit-tested 6/6 (non-bump → vacuous-green exit 0; bump-without-artifact → RED exit 1;
   bump-with-artifact → GREEN; malformed-artifact → RED; `<absent>→1.96.1` with/without baseline)
   plus an end-to-end `git show BASE:$FILE` extraction test against the real committed pin. Live
   GH-Actions run of the introduction PR is the `<absent>→1.96.1` end-to-end green; G5 (required-check
   registration) still owed server-side.
9. **Item 25 (procedure doc)** — then **Items 4+29 (+JsonWriter)** — **✅ DONE 2026-07-19**
   (`f04142f89`, `4f4872a54`, `eb350464e`). All proofs discharged: `cargo tree -e no-dev` 25→6
   (19 dropped ≥13); `metric.jsonl` + markov CLI JSON byte-compatible; post-mortem readback test
   (kill -9, restart, recover — 300/300 events + PostMortem); event schema `hw` first-class,
   RAPL-less host shows `unavailable:no_rapl_interface` (named absence, not silent omission);
   `zero-dep-gate.sh` GREEN; wasm32 green; 938 tests pass.
10. **Item 5** — **✅ DONE 2026-07-19** (`18152ef84`, `c6b5d2176`, `6605166cd`). Proofs discharged:
    `cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked --offline` = `dowiz-kernel`
    root ONLY (**0 external crates**); pre-cutover parity of the kernel-owned {literal, `.`, `.*`}
    matcher vs the live `regex` crate (20-doc + 2000-doc + proptest, bit-identical) + permanent
    independent naive-reference differential + frozen golden; `zero-dep-gate.sh` GREEN "0 external
    crates" (empty allowlist; latent zero-state gate abort fixed); existing parsing tests green
    (925 lib unit / 0 failed). **This closes ALL of Tier 1.**

Everything in this batch is now unblocked — no operator ruling stands between it and execution.

---

## H. Items 33–44 — Deterministic AI Inference Arc (appended 2026-07-19, second wave)

**Source:** `DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md` (grounding + five resolved
decisions) and `RAW-PROMPT-4-deterministic-ai-inference-self-verifying-code-2026-07-19.md`
(verbatim source dialogue). **Governing ruling, recorded:** *"безпека і передбачуваність понад
швидкість"* (safety and predictability over speed) — applied in the synthesis §2 to resolve all
five of the dialogue's open questions: **own-kernel zero-dep engine** (not TVM/Burn),
**inference-only** (training stays edge/build-time), **embedded weights** (generated committed
Rust static, `#[repr(align(64))]`, SHA3 init self-check, codesign; objcopy/link-section deferred
with named trigger), **i8-symmetric per-tensor quantization** (integer domain end-to-end,
`div_half_up` requantization), and **fold_transitions pinning moot pending re-measurement**
(item 33; separate-core rejected on the `core_pinning.rs` DECART precedent). Same sorting rule
as items 1–32: actual technical dependency, lowest first. **Standing law for the whole arc:**
zero new external crates (the live empty `ZERO-DEP-ALLOWLIST.txt` gate makes violation a CI
failure); every hot path ships under the §4 hardening checklist (item 6's machinery) — no
parallel checklist; dependency questions, if any arise, follow item 25's BINDING procedure.
Planning only — no item below starts before the operator dispatches it.

- **Item 33 — bench ground-truth re-measurement (Tier-0-class, zero prerequisites, NOT
  gated on item 34).** The raw prompt's telemetry numbers (+30% wire, 3.02x ML-DSA @N=64,
  +16.6% `fold_transitions`, +14.3% `empirical_identify`, "123 passed" engine, MISSING
  `fundamental_matrix_16`) match **no committed artifact in their claimed context** in either repo
  (synthesis §1.2) — names real, numbers unverified. (Lone near-match, corrected on re-verify: the
  figure "123 passed" *is* a real committed count, but for `bebop-proto-cap`
  (`WAVE-CLOSEOUT-P57-P74-2026-07-19.md:36`, P65), NOT `engine` — engine is 112/116/117/121 across
  committed docs, never 123; a cross-wired attribution, which strengthens the "real numbers from a
  different session" reading rather than weakening it.) Run the full tracked bench set (all baseline.json keys, both
  repos' perf branches reconciled) against committed baselines; confirm or refute each claimed
  regression; close the `_cur.json` partial-run gap so MISSING→RED cannot be produced by an
  incomplete run. **Proof:** a dated results doc with per-bench delta vs `baseline.json`; each
  raw-prompt number explicitly CONFIRMED (with the reproducing command) or REFUTED; a full-key
  run recorded with zero MISSING rows; any confirmed regression gets its own follow-up ticket
  (static-data-layout-first per the Q2 resolution — item 3's const-adjacency is the named fix
  shape; separate-core stays rejected).
- **Item 34 — pilot workload selection + scope ruling (`RESOLVED 2026-07-19` — operator ruled;
  gates items 35–44).** No model exists in-repo, so the arc must not start as an engine in search
  of a workload. Candidate real-product surfaces were presented (synthesis §3: retrieval reranker
  head, `Verdict`/`DriftClass` anomaly scorer, ETA-adjacent regressor — each KB–MB-scale,
  bounded-domain). **Operator's ruling (recorded, CLOSED — not an open gate): SYNTHETIC/TOY PILOT
  FIRST.** A small hand-built synthetic classifier — a toy MNIST-style or hand-authored pattern
  classifier, weights hand-written or fit offline at KB scale, **zero product data, zero PII, zero
  product risk** — that exercises the WHOLE determinism pipeline end-to-end (quantization → arena →
  SIMD kernels → reference oracle → golden checksum → embedded weights) and proves it works BEFORE
  any real product workload is attempted. Explicitly **NOT** a real-product classifier (the three
  §3 surfaces are DEFERRED to a follow-on second pilot, itself gated on this toy pilot landing
  green); **NOT** design-only/deferred (the toy pilot is *built*, it is the concrete vehicle for
  items 35–44); and — restating the arc-wide non-goals — not an LLM, not training, not GPU.
  **Scope consequence threaded to the downstream items:** the toy pilot's input plane is
  **public/synthetic by construction** (no capability/crypto/secret-adjacent inputs, no
  product/PII data anywhere in items 35–44), so item 43's constant-time gate takes its
  cheap-but-optional branch for THIS pilot — the mandatory dudect branch and any PII/secret-plane
  handling activate only for the deferred real-product pilots (item 43's named reopening trigger).
  **Proof (ruling half — DONE):** this ruling recorded here and in synthesis §3. **Proof (spec
  half — owed on dispatch):** a one-page spec fixing the toy classifier's bounded input domain D
  (synthetic, enumerable or tightly bounded) and the output-tolerance guarantee the engine must
  prove — the pure-function `f(x)=y` contract of the source dialogue's part 3.
- **Item 35 — fixed-point number-format + rounding-law spec (after 34).** The Q5 ruling made
  concrete: i8-symmetric weights, per-tensor scale (power-of-two shift preferred), i32
  accumulators with per-layer proven no-overflow bounds, `div_half_up` requantization,
  saturating-clamp semantics, refuse-never-fall-back on any unprovable bound. **Proof:** a spec
  doc with every law as a checkable equation; the i8×i8 multiply-accumulate law exhaustively
  proven over all 65 536 pairs (the house 65536-pair standard, literally); overflow-bound lemma
  stated falsifiably per layer shape.
- **Item 36 — eqc-rs indexed-summation IR extension, quantized-dot target (after 35; extends
  the already-ruled item 32 IR work — one extension, two consumers, never two IRs).** Grow
  `Expr` with the Σ-over-index construct needed by BOTH the Laplacian neighbor-sum (item 32)
  and the quantized dot-product inner law; `emit_fixed_rust` learns the i32-accumulator Q-format
  path. **Proof:** `emit_proof_program` harness green on an emitted quantized dot (compiled with
  real rustc, self-asserted against the tree-walking evaluator); the fixed emitter demonstrably
  refuses an inexpressible node; item 32's Laplacian consumer still green — one IR serves both.
- **Item 37 — reference oracle implementation (after 35; parallel with 36).** The "Schoolbook"
  of this arc: scalar, obviously-correct integer-domain matmul + activation set (i64/i128
  shadow accumulation — std-only, no dependency), retained forever as the test-only
  differential target, per the §4 checklist's oracle clause and the NTT schoolbook precedent.
  **Proof:** exhaustive small-dimension cases + large randomized corpus, oracle vs
  wide-accumulator shadow, zero divergence; the oracle module documented as permanent (never
  deleted on optimization).
- **Item 38 — static tensor workspace on the Arena (after 34; parallel with 35–37).** The
  dialogue's part-5 shape on the existing `BumpArena` precedent: one preallocated workspace,
  tensors as fixed offsets computed at build time from the pilot graph, zero mid-inference
  allocation, zero-copy layer-to-layer reads. **Proof:** a counting-allocator test (item 3's
  own proof machinery reused) shows zero heap allocations across a full inference; offsets are
  `const`; a deliberately-overlapping layout fails to construct (illegal state unrepresentable,
  §1.5 house standard).
- **Item 39 — SIMD quantized kernels via `core::arch` (after 36+37+38).** AVX2
  `_mm256_maddubs_epi16`/`_mm256_madd_epi16`-class integer paths, runtime-detected with the
  scalar oracle as fallback — `simd.rs`/`householder.rs` house pattern. Named dividend of Q5:
  integer arithmetic is associative, so within-row vectorization is *legal* here (unlike the
  f64 lanes' across-rows-only rule) — but the chosen lane order is still fixed and documented,
  and debug builds carry `debug_assert_eq!` against item 37's oracle (the `ring_mul` standard).
  **Proof:** differential corpus vs oracle bit-exact on both paths; the §4 checklist artifacts
  present and CI-re-executed; bench added to `baseline.json` so the bench-gate guards it.
- **Item 40 — per-layer golden-checksum oracle + hard-fail (after 39).** Build-time golden
  CRC32 per layer over pinned test vectors (reusing `fdr`'s hand-rolled CRC32 — P2, no second
  CRC), runtime spot-check, hard-fail to safe state on mismatch — a checksum mismatch is
  hardware/memory fault evidence, not a model error. Until item 9's breaker exists the fail is
  a typed trap + FDR entry; when the breaker lands, it routes through `Result<Permit, Tripped>`
  (composition named in synthesis §3 — design does NOT gate on item 9). **Proof:** a planted
  single-bit corruption (weights AND activation, separately) demonstrably trips the fail path
  and writes the FDR entry; an uncorrupted run is checksum-silent; CI re-executes the planted
  fault (P7 — the verifier proves it can reject).
- **Item 41 — embedded weight pipeline (after 35; parallel with 39–40).** The Q4 ruling made
  real: generator emits committed `static WEIGHTS: [i8; N]` Rust source (eqc_gen precedent),
  `#[repr(align(64))]` wrapper (first in-repo use — flagged as new surface), SHA3-256 golden-
  hash self-check at init (reusing `event_log`'s Keccak), ML-DSA codesign via `pq/codesign.rs`
  for update-blob shipping. The objcopy/`link_section` alternative is parked with its named
  reopening trigger (weights > ~1–2 MB committed-source practicality, or measured build-time
  regression) per item 25's procedure. **Proof:** init self-check demonstrably fails on a
  tampered byte (red→green); alignment asserted by test; the parked alternative + trigger
  recorded in the module doc (slot_arena format); zero-dep gate untouched.
- **Item 42 — fixed-sequence scheduler (after 38+39+41).** The engine's spine: a `const`
  function-pointer array / straight-line layer sequence, cyclomatic complexity 1, no dynamic
  graph traversal, no hash-map dispatch — the whole model as one compiled call sequence.
  **Proof:** a source-structure test asserts the sequence is `const` and branch-free at the
  dispatch level; an assembly spot-check of the dispatch path filed under item 14's toolchain-
  keyed audit format; end-to-end inference reproduces bit-identical outputs and (via item 40)
  identical per-layer checksums across repeated runs and across native/wasm32.
- **Item 43 — constant-time inference gate (after 42; scope decided by input-plane
  classification first).** Classify the pilot's input plane per §10/P6 plane-ranking: if inputs
  are secret-adjacent (anything fed from capability/crypto surfaces), the full dudect-style
  gate with planted-leak self-test (the `ntt_ct_gate` template) is mandatory and ReLU-class
  branches become mask/cmov per the dialogue's part 4; if provably public-plane, record that
  ruling and ship the gate as cheap-but-optional. **For the item-34 synthetic/toy pilot the
  classification is already settled — inputs are public/synthetic by construction, so this pilot
  takes the cheap-but-optional branch; the mandatory dudect branch activates only for the deferred
  real-product pilots (item 34's reopening trigger).** **Proof:** the plane classification recorded
  with its reasoning; if gated — Welch |t| < 4.5 across input classes AND the planted leak
  demonstrably caught; if not gated — the recorded ruling names the reopening trigger (any new
  secret-adjacent consumer).
- **Item 44 — arc-wide CI integration + retroactive checklist pass (after 40+42; final).**
  The inference hot paths join item 6's designated-hot-path list; the §4 CI job re-executes
  (never presence-checks) the oracle corpus, the planted-fault checksum test, and (if gated)
  the dudect self-test; benches join the bench-gate baseline; the FDR carries per-inference
  cycles + (where RAPL exists) joules per item 29's schema — a token-count-only cost report
  fails review per §21. **Proof:** a deliberately artifact-less test diff touching an inference
  hot path fails CI; the full suite green; `cargo tree -e no-dev` still resolves to the kernel
  root alone — the arc lands with the allowlist still empty.

**Dependency graph, one line:** 33 ∥ 34 → 35 → {36 ∥ 37 ∥ 38} → 39 → 40 → 42 → 43 → 44, with
41 branching off 35 and merging before 42; item 9 (breaker) composes with 40's fail path when
it exists but gates nothing here.

---

## I. Items 45–49 — Whole-System Determinism & AI-Optional Arc (appended 2026-07-19, third wave)

**Source:** `CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md` (Fable
synthesis) over `RESEARCH-CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-2026-07-19.md` (Opus
grounding, 11 findings) and
`RAW-PROMPT-5-crash-consistency-formal-verification-fail-fast-guardian-2026-07-19.md` (verbatim
dialogue). **Governing directive, recorded:** *"вона має 100% передбачуваною, математично
детермінованою із запобіжниками. Окрім цього уся система повинна здатна працювати без AI"* —
(a) whole-system determinism + safeguards, broader than the items-33–44 AI subsystem; (b)
AI-optional as a preserved architectural INVARIANT (GROUNDED already-true today: `attention.rs`
"the kernel stays non-AI"; `order_machine`/`decision`/`hydra` import zero AI modules). Ground
truth honored throughout: the kill-9 mechanism IS a Sequential Append-only Log (not pointer-swap,
not hybrid); Kani/TLA+ remain planned-only (items 7/10/11, unchanged); Coq/Lean-class full
formal verification is OUT OF SCOPE per the synthesis §5 proportionality ruling (BITE/runtime-
verification primary — where the source dialogue's own self-correction landed). Same standing
laws as §H: zero new external crates, §4 hardening checklist via item 6's machinery, item-25
procedure for any dependency question. Planning only — no item starts before the operator
dispatches it.

- **Item 45 — `ai-optional-gate`: AI-optional as an enforced compile-time invariant (Tier-0/1-
  class, zero prerequisites, READY NOW — asserts today's truth, gains teeth when items 33–44
  land).** Structural law amended into the §H arc: the inference subsystem lands behind a
  **non-default cargo feature** (e.g. `inference`) — the exact `pq`/`slot-arena` surface-control
  pattern already in the kernel. New CI job (zero-dep-gate/toolchain-bump-gate precedent shape):
  (a) default-features build (AI absent) must compile AND pass the FULL kernel test suite; (b) a
  dependency-direction check — no core decision module (`order_machine`, `decision/`, `hydra`,
  `event_log`, `markov`, `spectral`, `fdr`) may reference the AI module paths outside the feature
  gate (AI depends on core, never core on AI). Explicitly NOT built: runtime kill-switch service,
  dual-binary pipeline, AI-health monitor (over-design guard; the runtime half is item 47's
  `None` path). **Proof:** a planted core→AI import (or a planted default-features AI reference)
  demonstrably turns the gate RED before the gate counts as landed (P7); the default-features
  full suite runs green inside the job; the feature-gate law is recorded in §H's header and the
  AI module's own doc when it lands.
- **Item 46 — float-determinism containment, evidence-scoped (READY NOW; composes with item 14's
  closed bump gate).** NOT a kernel-wide f64→fixed rewrite — rejected as disproportionate
  (synthesis §2.3: the one real float-nondeterminism bug ever shipped was libm `sin`/`cos` ULP
  drift, fixed by the Q30 CORDIC, `REGRESSION-LEDGER.md` row 25; basic IEEE-754 arithmetic is
  bit-deterministic for a fixed binary on the pinned 1.96.1 toolchain). Scope: (i) inventory
  every libm-transcendental call site (`sin`/`cos`/`exp`/`ln`/`powf`; `sqrt` exempt —
  correctly-rounded) in the deterministic kernel plane (`spectral.rs`, `markov.rs`,
  `token_bucket.rs`, `attention.rs`), disposition each as migrate-to-CORDIC-class or
  pin-under-golden; (ii) every value feeding a cross-version/cross-host comparison surface
  (golden signatures, oracle pins, `wire_code()`s, `DRIFT_BAND`-class constants) must be either
  integer-domain or covered by a golden test. **Re-execution mechanism (verified precise): the
  toolchain-bump gate itself only requires a `spot-check-<new>.md` artifact on a `channel` bump;
  the golden tests are actually re-run under the new compiler by the always-on full-suite
  `cargo test` job (pinned via `rust-toolchain.toml`) plus item 6's `hardening-gate` unconditional
  oracle re-run** — so a compiler-induced float divergence turns the bump PR RED, never a silent
  ship (once this item adds the missing golden coverage); (iii) the full fixed-point
  conversion is parked as an explicitly-flagged-LARGE item with named reopening triggers: a
  reproduced cross-version golden divergence in basic float arithmetic, or a multi-ISA deployment
  requirement. **Proof:** the inventory doc with per-site disposition and zero unclassified
  transcendental sites; the new golden float surfaces sit in the always-on full-suite /
  `hardening-gate` oracle set (a deliberately perturbed golden value turns CI RED under the pinned
  toolchain — red-proven), and a `channel` bump is additionally gated on the `spot-check-<new>.md`
  `## Full-suite re-run` artifact; the parked rewrite + triggers recorded in the doc and the
  relevant module docs.
- **Item 47 — Guardian: semantic advice gate + deterministic-primary path (spec after item 35;
  full wiring after item 42; EXTENDS item 9, cross-references item 40 — no competing breaker, no
  fold-in).** The kernel's decision seam takes `Option<Proposal>` — advice is DATA; `None`
  (AI absent/crashed/rejected) is a first-class tested input, and the deterministic path is the
  total function (the "fallback" IS the system — AI-optional expressed in the type system).
  Admission is parse-don't-validate: `admit(Proposal, &Invariants) -> Result<ValidatedProposal,
  Rejection>` with `ValidatedProposal` constructible only through `admit`
  (illegal-state-unrepresentable, the item-9 `Result<Permit, Tripped>` standard); invariants
  written as checkable equations (the `Result.velocity < MAX_SAFE_SPEED` class). Static
  procedures are NAMED pure functions, statically dispatched, `match`-based (the `order_machine`
  style), every loop statically bounded (`0..MAX_N`, item-42-style source-structure assertion;
  WCET tooling explicitly out of scope). Distinct from item 40 by plane: 40 rejects corrupted
  BITS (hardware-fault evidence), 47 rejects well-formed-but-unsafe MEANING; both hard-fail
  observable. Every `Rejection` emits an FDR event; when item 9 lands, repeated rejections route
  through the breaker (same composition clause as item 40 — design does NOT gate on item 9).
  Named precedent to extend, never fork: `decision/import.rs::import_unit`'s
  verify-before-persist replay gate — the same shape at import granularity. **Proof:** the
  invariant spec doc with every law as a checkable equation; planted-invalid-advice red→green
  (the gate demonstrably rejects — P7); the `None`-path test proving bit-identical output vs the
  deterministic baseline; exhaustive enumeration where the advice domain is enumerable +
  oracle/differential corpus otherwise + a proptest sweep (the item-5 regex-parity testing
  stack, reused not reinvented); the source-structure bounded-loop assertion green.
- **Item 48 — FDR blind-spot closure: panic forensics + liveness heartbeat (after items 4+29 —
  satisfied; READY once the FDR branch merges).** The kill-9 test proves recovery AFTER process
  death; it is structurally blind to (a) a panicking process that writes nothing before dying
  and (b) a HUNG process that never dies (no PostMortem is ever emitted — the one failure class
  FDR cannot see; the k3 span-metrics self-deadlock, root-caused+fixed `67851b2f3`, is the
  in-repo precedent). Two narrow closures, both BITE-shaped: **(a)** `std::panic::set_hook`
  emitting ONE fsynced `Alarm` FDR record (message + location; `Alarm` already fsyncs) — a panic
  hook, NOT a `#[panic_handler]` (`std` kernel; the bare-metal construct does not apply);
  register/stack core-dumps explicitly not pursued. **(b)** a periodic `Heartbeat` `Kind`
  variant (closed-enum growth) carrying seq + progress counters; liveness JUDGMENT and restart
  authority stay OUTSIDE the kernel (systemd `WatchdogSec` / deployment layer;
  `hub_supervisor`'s crash-loop detection is the deploy-granularity precedent) — a missed
  heartbeat converts a hang into the kill-9 crash class the system already provably survives.
  The kernel carries NO self-kill/self-restart logic (`Kernel_Init`-over-`Kernel_Recover`,
  KISS). **Proof:** a test child that panics yields a recovered `Alarm` record carrying the
  panic site (red→green: without the hook, nothing is recovered); a test child that deliberately
  hangs (loop + no heartbeat) is flagged by the external liveness check WHILE producing no
  PostMortem — demonstrating exactly the gap closed; all other FDR records byte-identical
  (optional-field discipline, item-27 precedent); clean-shutdown emits a final heartbeat and no
  false alarm.
- **Item 49 — event-log replay-bound measurement + Hybrid/LSM park (after item 2's wiring fix
  lands — currently gated: no production composition root constructs the durable store).** The
  raw prompt's Hybrid (WAL + periodic snapshot) recommendation, dispositioned per surface: for
  the FDR ring it is REJECTED permanently (replay bounded by construction at 2×1 MiB segments);
  for the durable `EventLog` (genuinely unbounded hash-chain replay; `hub_supervisor`'s
  `StateSnapshot` is an update-rollback epoch pointer, NOT replay-speedup) it is PARKED behind
  measurement — measuring an unwired store would optimize an unreachable path. Once wired:
  measure startup replay time vs event count (item-26 measurement-only discipline: real numbers,
  no code landed), state a replay budget, and record the parked snapshot design with its named
  reopening trigger (measured replay exceeding budget at realistic event volume). Carried-forward
  correctness note if ever built: data-file fsync strictly BEFORE pointer swap (the dialogue's
  caveat, endorsed; consistent with `ring.rs`'s kill-9-vs-power-loss separation). **Proof:** a
  dated measurement doc (replay µs at N ∈ {1e3, 1e4, 1e5} events, methodology stated); the
  budget + trigger recorded; zero snapshot code landed (scope law, item-26 precedent); the FDR
  permanent-rejection rationale recorded in `fdr/ring.rs`'s module doc when next touched.

**Dependency graph, one line:** 45 ∥ 46 ∥ 48 ready now (48 pending the FDR branch merge);
47 spec after 35, full wiring after 42, composes with item 9's breaker when it exists;
49 strictly after item 2's wiring-gap fix. No item here gates any §H item; item 45's feature-gate
law binds §H's build items when they land.

## J. Items 50–54 — Validity (K3 Admission), Live-Struct Sentinel & Proportionate Open-Source Hardening Arc (appended 2026-07-19, fourth wave)

**Source:** `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` (Fable synthesis) over
`RESEARCH-KLEENE-TRUTHFULNESS-OPENSOURCE-HARDENING-2026-07-19.md` (Opus grounding) and
`RAW-PROMPT-6-…-kleene-unknown` + `RAW-PROMPT-7-…-sentinel-shadow-mode` (one combined verbatim
dialogue). **Terminology RULING, binding from here on (synthesis §1):** "Truthfulness" =
byte-reproducibility, exclusively the swarm-safety arc's property, NOT a term of this roadmap;
the RAW-PROMPT-6 content-based concept is renamed **"Validity" (derivational validity)** — a
proposal is valid iff its supplied reasoning/evidence path checks against the stated
axioms/invariants; incomplete evidence downgrades to Undecidable, never to assumed-valid.
**Dispositions recorded here (synthesis §§2.3/2.5, Part 3):** the Sentinel read-time integrity
check for critical LIVE in-memory structs is **ADOPTED as item 54**, proportionately scoped —
an earlier draft of this pass rejected it on a "commodity ECC cloud hardware" argument that the
operator **reversed on 2026-07-19** on two grounds: (i) genuine space-grade engineering quality
is the standard for this arc regardless of substrate, and (ii) the deployment premise was
factually wrong — the target is **local, offline-first, consumer-grade hardware, which typically
LACKS ECC**, so the in-memory bit-flip fault class is *higher* not negligible, strengthening the
mechanism's justification. Item 54 reuses the in-kernel CRC32 (zero new primitive), checks at
transition points (not per-field-read), and is scoped to the live mutable authority structs item
40's read-only weight checksum structurally does NOT cover (item 47's `Invariants` table, item 21
gain-schedule, live inference config) — genuine overlap with item 40 is a boundary, not a reason
to skip. Kani-for-K3 is item-7 target-list growth, not a new item. proptest stays strictly
dev-only (zero-dep-gate law). **Operator-facing repository-state flag (Part 4):
items 1–49's actual CODE (all Tier 1, item 6's gate + `ct_gate.rs`, the FDR module, fixes from
items 16/30/31) still lives ONLY on the unmerged `exec/space-grade-tier0-2026-07-19` branch —
`main` has documents only.** Items below that touch FDR or item 47 inherit that merge as a
prerequisite. Same standing laws as §§H–I: zero new external crates, item-6 hardening machinery,
item-25 procedure for any dependency question. Planning only — no item starts before the
operator dispatches it.

- **Item 50 — K3 admission-verdict extension + Validity terminology binding (spec-level
  amendment to item 47 — same gating: spec after 35, wiring after 42; EXTENDS item 47's
  `admit`, never a parallel type).** The public seam stays exactly item 47's
  `admit(Proposal, &Invariants) -> Result<ValidatedProposal, Rejection>` — Kleene-False and
  Kleene-Unknown MUST be behaviorally identical at the seam (advice unused, deterministic path
  taken), so no third control-flow arm exists for "Unknown" to be handled leniently through.
  `Rejection` gains a two-class cause, `RejectionClass::{Refuted, Undecidable}`: Refuted = a
  named invariant/inference rule demonstrably violated (K3 False); Undecidable = evidence
  chain incomplete/absent/over-budget (K3 Unknown — RAW-6's Evidence-based Unknown adopted
  verbatim: model confidence/logits are NEVER an input to `admit`). The literal
  `#[repr(u8)] enum TruthState { False=0, True=1, Unknown=2 }` lands as an INTERNAL combinator
  type of the admission module: each sub-check returns `TruthState`; the strong-Kleene fold
  governs (any False short-circuits to Refuted — `False & Unknown = False`; else any Unknown
  folds to Undecidable — `True & Unknown = Unknown`; all True admits). `None` ≠ `Unknown`:
  the seam's `Option<Proposal>` None (advice absent, items 45/47) and Undecidable (advice
  present but unevaluable) both take the deterministic path but log as distinct facts. The
  class rides item 47's existing per-`Rejection` FDR event so item 9's breaker and item 51 can
  weight Refuted vs Undecidable differently. **Proof:** exhaustive truth-table tests — all 9
  cases per binary operator + 3 for NOT, the full state space enumerated literally (RAW-7's
  own exhaustive-beats-random point; NO new proptest use); planted incomplete-evidence
  proposal demonstrably lands `Undecidable` and planted rule-violation lands `Refuted`
  (red→green, P7); the item-47 `None`-path bit-identity test still green with the extension in
  place; the K3 fold joins item 7's Kani target list (recorded there, executed under item 7).
- **Item 51 — shadow-mode divergence telemetry at the decision seam (after item 47's wiring +
  item 50; FDR branch merge prerequisite — genuinely NEW pattern, full design in synthesis
  §2.4).** No second execution lane: item 47's deterministic decision D is already total and
  always computed, so on `Some(proposal)` the comparison is nearly free. New FDR
  `Kind::ShadowDivergence` variant (closed-enum growth — item-48 `Heartbeat` precedent)
  carrying decision-site id, Admitted/`RejectionClass`, agreement bit, and short DIGESTS of D
  and the proposed action (never full payloads; records without the surface stay
  byte-identical — item-27 optional-field discipline). Emission policy: every disagreement and
  every Admitted-but-differs logged; Undecidable-while-D-decides at a bounded rate (the
  "model adds nothing on this domain" signal); agreement SAMPLED at a low fixed rate for the
  base-rate denominator — bounded emission preserves the FDR ring's replay-bounded-by-
  construction property (item 49's rationale). Advisory by definition AND by test: no build
  fails, no decision changes, no breaker trips on a shadow event alone (aggregated
  Refuted-class counts still reach item 9 via item 47's own rejection events — shadow mode
  adds observation, never authority). Distinct from every existing differential in-tree: those
  all fail/reject on disagreement (`decision/import.rs` ReplayDisagreement rejects;
  pq/spool/spine differentials are tests); nearest advisory kin is `metrics.rs`'s
  merge-plane anomaly flag — different plane, cited not extended. **Proof:** deterministic
  output bit-identical with shadow logging on vs off (item-47 `None`-path test pattern
  reused); a planted disagreeing proposal yields exactly one recovered `ShadowDivergence`
  record with correct class + digests (red→green through the real FDR ring); emission-rate
  bound asserted under a flood of planted disagreements; all non-shadow FDR records
  byte-identical before/after.
- **Item 52 — `miri-gate`: targeted UB detection over the real unsafe surface (independent —
  zero prerequisites on items 47/50/51; dispatchable now).** GROUNDED baseline: Miri runs
  nowhere (aspirational doc-comments only; `ROADMAP-LIVE-STATUS-2026-07-18.md:24` "component
  absent this toolchain"). **Inventory corrected by independent re-verification 2026-07-19 (the
  research/RAW figure was wrong):** the real unsafe surface is **19 blocks in only 4 modules** —
  `arena.rs` (6), `simd.rs` (5), `fdr/pmu.rs` (5 — `_rdtsc`/raw-`syscall5` FFI, exec-branch only,
  joins post-FDR-merge), `householder.rs` (3). `messenger.rs`/`slot_arena.rs`/`chaos.rs`/
  `bounded_drainer.rs` contain **ZERO real unsafe** — every `unsafe` token in them is a *comment*
  (`slot_arena.rs`'s doc-comment literally says "No `unsafe` in this wrapper"); the old "21 blocks /
  7 modules" list counted those comment mentions and omitted `fdr/pmu.rs`. `pq/` has ZERO unsafe
  (the raw prompt's crypto guess was wrong). Scope: ONE CI job running `cargo miri test` filtered to
  the genuinely unsafe-bearing modules — `arena.rs`'s bump-allocator raw-pointer logic (where UB
  actually hides) plus the scalar paths of `simd.rs`/`householder.rs`; NOT the four unsafe-free
  wrappers (filtering them matches zero unsafe — theater), NOT miri-everything. Honest limitation,
  recorded in the gate's own doc: `core::arch` AVX2 intrinsic bodies AND `fdr/pmu.rs`'s
  `_rdtsc`/syscall FFI are largely unsupported under Miri; the house runtime-detection +
  scalar-fallback pattern means the interpreted run exercises the scalar paths of
  `simd.rs`/`householder.rs`, and intrinsic/syscall-body coverage stays with the items-37/39
  differential oracles + item 7 — a green `miri-gate` is never read as "SIMD/PMU is Miri-clean"
  (exact intrinsic support confirmed empirically on first run, not asserted). Toolchain: Miri
  needs a nightly component; the BUILD pin (item 14, 1.96.1) is untouched — the job pins its
  own analysis nightly, recorded in the workflow + `docs/audits/toolchain/`, bumps recorded
  not floating. **Proof:** a planted UB self-test (out-of-bounds / use-after-free behind a
  test-only cfg) demonstrably turns the gate RED before it counts as landed (P7); clean run
  green; a filter matching zero tests is RED (item-6 anti-forgery clause reused); build
  toolchain pin byte-unchanged.
- **Item 53 — `lint-gate`: clippy + fmt (+ miri-required promotion) contribution gates (LOW
  priority, LAST in this arc, blocks nothing — sequenced behind 50–52 by explicit RULING).**
  GROUNDED: none of the triad exists in CI (zero clippy/fmt/miri workflow hits; real gates
  today = cargo-test, dco-check `ci.yml:210-226`, decart-dep-lint, v5c-reexec, gitleaks,
  supply-chain, bench-regression); AND open-sourcing is NOT imminent — ADR-0020 Accepted but
  public-flip + EUTM are operator-gated and unauthorized — so the raw prompt's "any PR is an
  attack vector" urgency presumes a contribution surface that is not authorized to exist yet.
  Scope when dispatched: one cheap job — `cargo clippy --deny warnings` + `cargo fmt --check`
  (both components ALREADY pinned by item 14's `rust-toolchain.toml`
  `components=[rustfmt,clippy]`); miri-required = promoting item 52's job to a required check,
  no new machinery. Inherits item 14's owed G5 caveat: advisory until marked required in
  branch protection (server-side). **Named escalation trigger:** operator authorization of
  public-flip preparation (ADR-0020's gate) promotes this item to a pre-flip BLOCKER alongside
  the ADR-recommended all-origin-refs gitleaks sweep; until then it stays last. **Proof:** a
  planted clippy warning and a planted fmt divergence each turn the job RED (P7); clean tree
  green; the escalation trigger recorded here and in the job's comment header.
- **Item 54 — Sentinel: read-time integrity check for critical LIVE in-memory structs (after
  {item 47 wiring (post-42) + item 50} + the FDR branch merge; registry enumeration startable
  now; full design in synthesis §2.3 — operator-reversed 2026-07-19 from an earlier draft
  rejection).** Deployment premise, corrected and load-bearing: the target is **local,
  offline-first, consumer-grade hardware that typically LACKS ECC**, so a single-/multi-bit
  in-memory flip is a real fault class — NOT a cloud/ECC context, and the "space-grade" standard
  binds regardless of substrate. GROUNDED baseline: the live-struct read-time pattern is genuinely
  absent (all existing integrity machinery is AT-REST — `backup` CAS, `event_log` chain-walk, FDR
  ring CRC32). Proportionate on three axes: **(scope)** only structs that are long-lived AND a
  money/safety/decision authority input AND lack at-rest backing qualify — the enumerable registry
  is item 47's `Invariants` table (a flipped bound silently mis-certifies *every* `admit`), item 21
  gain-schedule/decision-config, and the live inference config (distinct from item 40's read-only
  weights); transient scratch and already-at-rest-verified state are excluded. **(primitive)**
  REUSES the in-kernel CRC32 already built for the FDR module (P2 — no second CRC, no new
  algorithm, no external crate; CRC32 not crypto — the threat is a hardware fault, not an in-memory
  adversary). **(frequency)** checked at defined transition points (once per authority-use, e.g.
  per `admit` over the `Invariants`; recompute-and-store on the rare centralized mutation) — NOT
  per-field-read, so the hot-path tax and the missed-re-hash false-trip surface are both bounded;
  an immutable-after-init struct is a pure read-time check with zero re-hash burden. On mismatch:
  ONE fsynced FDR `Alarm` (hardware-fault evidence, item-40 semantics) + fail-closed deterministic
  path (a corrupted `Invariants` table REFUSES admission), composing with item 47's `Rejection`
  seam and item 9's `Result<Permit, Tripped>` when it lands (does NOT gate on item 9). Distinct
  from item 40 by plane: 40 guards read-only static WEIGHTS, 54 guards live MUTABLE authority
  structs — complementary surfaces, one shared CRC. **Proof:** a planted single-bit corruption of a
  registered struct (behind a test-only cfg raw-pointer flip, mirroring item 40's planted-fault
  test) demonstrably trips the Safe-State path and writes the `Alarm` (red→green, P7); an
  uncorrupted run is checksum-silent; mutate-then-read passes (re-hash correctness); CI re-executes
  the planted fault; the critical-struct registry is enumerated with per-struct justification (why
  critical, why no at-rest backing); `cargo tree -e no-dev` byte-unchanged (existing CRC32 reused,
  zero new dependency and zero new algorithm — max-nativeness law).

**Dependency graph, one line:** 50 rides item 47's gates (spec after 35, wiring after 42);
51 after {47-wiring + 50} + the FDR/exec branch merge; 52 independent (on-`main` targets
`arena`/`simd`/`householder` dispatchable now, `fdr/pmu` folds in post-FDR-merge); 53 last by
ruling, trigger-promoted on public-flip authorization; 54 parallel with 51 (same {47-wiring + 50}
+ FDR-merge prerequisite; registry enumeration startable now). Nothing here gates any §H/§I item;
items 50 and 54 amend/extend item 47's surface in place (one admission gate, one shared integrity
primitive, never a fork).
