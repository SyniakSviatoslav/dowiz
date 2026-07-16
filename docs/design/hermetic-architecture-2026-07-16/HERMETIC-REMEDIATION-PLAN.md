# HERMETIC REMEDIATION PLAN — one map, one sequence, no re-blueprinting (2026-07-16)

> Consolidates the 29-row ranked findings table (`HERMETIC-ARCHITECTURE-PRINCIPLES.md` §3) and its
> four root causes (§2 RC-1…RC-4) into a single wave-sequenced execution plan.
> **Planning artifact only. No code is written or edited by this document.**

---

## §0 — Index and how to read this document

This plan does **not** re-blueprint anything already blueprinted. Its job is three-fold: (1) **map**
every one of the 29 audit findings to the artifact that fixes it — the four NEW blueprints written
this session (`BLUEPRINT-H1`…`H4`, this directory) or the pre-existing sovereign-roadmap blueprints
(`../sovereign-roadmap-2026-07-16/BLUEPRINT-P01/P02/P06/P07/P08/P12`) that already cover several
findings; (2) **build only what is genuinely missing** — which turns out to be the H-series plus six
quick-wins (§4), because five findings were independently pre-designed in the sovereign roadmap
(§2); (3) **sequence** everything into the roadmap's own waves, read from
`R2-MERGED-PHASE-ROADMAP.md` §3, never renumbered. §1 is the complete finding→fix table; §2 an
honest note on the two-method convergence; §3 the wave plan; §4 quick-wins; §5 the backlog in E53
waiver form; §6 the mandatory 2-question doubt audit applied to this plan itself; §7 the
Anu/Ananke check.

---

## §1 — The full finding→fix map (all 29 rows)

Row numbers, severities, and one-liners are from `HERMETIC-ARCHITECTURE-PRINCIPLES.md` §3. Wave
numbers for P-items are R2's own (`R2-MERGED-PHASE-ROADMAP.md` §3: Wave 0 = P1–P5; Wave 1 = P6, P7,
P8; Wave 2 = P9, P11, P12). H-item waves are derived in §3 from real file-surface dependencies.

| # | Finding (one line) | Fix location | Wave |
|---|---|---|---|
| 1 | `EventStore::insert` infallible; `FileEventStore` swallows IO, reports `Committed` (RC-3) | NEW: BLUEPRINT-H1 | 0 (gated on `hydra.rs` quiescence, see §3) |
| 2 | "Done" gate reads author-supplied evidence — self-certified completion (RC-2) | EXISTING: BLUEPRINT-P06 full (key_V verifier; precondition = P01 §2.8 harness) | 1 |
| 3 | ADR-020 is a phantom cited by 15+ docs (RC-1) | EXISTING: BLUEPRINT-P02 §3 (ADR-020 draft outline, ready to land) | 0 |
| 4 | COLD backup restore-drill has never run — Schrödinger's backup | EXISTING: BLUEPRINT-P12 §3 (`restore-verify` subcommand + drill log) | 2 |
| 5 | Hermes cron gateway DOWN, all jobs `last_run=None`, MEMORY says "running" (RC-1) | EXISTING: BLUEPRINT-P12 §6 (gateway revival; the one-command `sudo hermes gateway install --system` is also an immediate ops action — see §3 note) | 2 |
| 6 | Schedule lives outside the repo — hygiene rhythm unreproducible from canon | EXISTING: BLUEPRINT-P12 §6 (in-repo systemd units + timers) | 2 |
| 7 | `FalseClaimMeter` unfed AND self-fed (RC-2) | EXISTING, three legs: P01 §2.7 (ledger appender = the feed) + P06 §6 (route `verified` through key_V) + P08 §4 (anomaly consumer) | 0 → 1 |
| 8 | "ONE Laplacian" refuted; `csr::laplacian_spmv` has zero callers (RC-1) | BACKLOG (see §5; doc-half — mark aspirational in ARCHITECTURE.md — rides P02's canon-repair batch, Wave 0) | — |
| 9 | PPR implemented 3×, R-LM pointer wrong (RC-1) | BACKLOG (see §5; same doc-half note as #8) | — |
| 10 | `DT_STABLE=0.02` vs `field_frame` default `dt=0.016`, unlinked (RC-4) | NEW: BLUEPRINT-H2 §2.1 | 0 |
| 11 | Money tax `unwrap_or(0)` — failure rendered as tax-free order | EXISTING: BLUEPRINT-P07 §6 (fail-closed degrade `total → None`, RED test already designed) | 1 |
| 12 | wasm `funnel` leaks `HashMap` order into emitted JSON | NEW: BLUEPRINT-H2 §2.4 (bundled; `BTreeMap`) | 0 |
| 13 | Two canon docs mis-cite `eqc-proofs/lambda_max_of_d.rs`; real home is `/root/bebop-repo/rust-core/eqc-proofs/` (RC-1) | EXISTING: BLUEPRINT-P02 O19 (proof-home ruling; corroborates its bebop REC) + cite-with-probe corrections to both docs | 0 (ruling; O19 executes at P13) |
| 14 | Retroactive ADR-writing habit (0007–0009 post-hoc) | QUICK-WIN (§4: doctrine sentence + date-the-reasoning field) | any |
| 15 | Breach detection originates only in the audited party (RC-2) | NEW: BLUEPRINT-H3 (peer-initiated probe on top of G9) | 2 |
| 16 | 2Q doubt ritual MANDATORY with zero firing mechanism (RC-2) | NEW: BLUEPRINT-H4 (proposal-only; operator applies — `.claude/` is protected) | 0 (operator-gated) |
| 17 | Dead one-shot cron `bebop-library-star-list` still `[active]` | QUICK-WIN (§4) | any |
| 18 | `TG_MIN_GAP_S=3.5` cross-repo, comment-only (RC-4) | NEW: BLUEPRINT-H2 §2.3 | 0 |
| 19 | "Byte-identical" claim broader than same-process double-call tests | QUICK-WIN (§4: second-process serialize→re-read tests) | any |
| 20 | Reproducibility banner over-covers transcendental floats | QUICK-WIN (§4: one doctrine sentence) | any |
| 21 | `fold` reducer hand-rolled per subsystem | BACKLOG (§5; trigger may fire at P7 §5 — named there) | — |
| 22 | Graph `from_edges` hand-rolled per module; `mat.rs` half-retired | BACKLOG (§5; natural trigger = P4's csr ingestion port) | — |
| 23 | `DriftClass` enum declared twice, comment-bound (RC-4) | NEW: BLUEPRINT-H2 §2.2 (`wire_code()` + round-trip pin test) | 0 |
| 24 | Linear un-jittered backoff duplicated at ~7 sites | NEW: BLUEPRINT-H2 §2.5 (bundled; shared jittered helper or ADR line) | 0 |
| 25 | Eigen*vector* output has no second-party check (values do) | BACKLOG (§5) | — |
| 26 | Hub↔mesh-node topology gap; M7 heal unimplemented — a feature gap, not drift | BACKLOG (§5; owned by the sovereign roadmap itself: math = R2 P4, heal = R2 P9) | — |
| 27 | Claim-latency ledger absent; designed-in half-pendulum | EXISTING: P01 §2.7 (appender) + P08 §4 (consumer) shipped as a pair; H4 §2(b) is the explicit interim stopgap that retires into P01 (H4 §3.5) | 0 → 1 |
| 28 | Quarantined ad-hoc-easing UI tree retrievable | QUICK-WIN (§4: delete + optional CI grep) | any |
| 29 | Stray `kernel/=5` zero-byte artifact | QUICK-WIN (§4: `rm`) | any |

**Completeness check:** 17 rows map to NEW/EXISTING blueprints (1–7, 10–13, 15, 16, 18, 23, 24,
27), 6 to quick-wins (14, 17, 19, 20, 28, 29), 6 to backlog (8, 9, 21, 22, 25, 26) = **29/29, zero
silent drops** (re-counted in §6).

**Leverage restated:** per the audit's own reading note (`HERMETIC-ARCHITECTURE-PRINCIPLES.md` §3),
four actions close the fourteen ★RC rows — H1 (RC-3), P06+feeds (RC-2 core, with H3/H4 as its
peer-probe and interim-ritual limbs), the P02 cite-with-probe canon pass (RC-1), and H2 (RC-4).

---

## §2 — An honest note on convergence

Five findings were already designed for **before this audit existed**: row #3 (ADR-020 → P02 §3,
written 15:32 today), #4 (restore-drill → P12 §3), #5/#6 (cron gateway + in-repo timers → P12 §6),
#11 (tax overflow → P07 §6, complete with its RED test), #13 (eqc-proofs homelessness → P02 O19,
whose "home it in bebop" recommendation the audit then corroborated by *finding the file already in*
`/root/bebop-repo/rust-core/eqc-proofs/`). The two passes used genuinely different methods — the
sovereign roadmap did anchor-based gap analysis (147 canon anchors → 19 phases), the Hermetic audit
did principle-based live inspection — and arrived at the same gaps. That is **suggestive
corroboration**: two search strategies with different starting points hitting the same holes is
evidence the holes are real, not artifacts of one method's blind spot.

It is **not** independent verification in the Gender-principle sense, and this plan refuses to
claim it as such. Both passes were authored within the same session lineage, same author identity,
same day, with overlapping context (the audit reports cite the roadmap blueprints by name — the
second pass read the first). PRINCIPLE-7's own test — *"who supplies the inputs the checker
reads?"* — answers "the same party" here. The audit's central diagnosis is that this repo lacks a
structurally independent second party (97.8% single authorship, `HERMETIC-ARCHITECTURE-PRINCIPLES.md`
§4); crediting same-lineage agreement as independence would be a fresh instance of the very
self-certification pattern (RC-1) this plan exists to close. What the convergence honestly buys:
increased confidence these five fixes are correctly targeted, and zero duplicated design work.
What it does not buy: proof. Proof arrives when P06's key_V re-executes them.

---

## §3 — Wave-sequenced execution plan

Wave numbers are R2's, unrenumbered (`R2-MERGED-PHASE-ROADMAP.md` §3). This plan adds the H-series
into those waves and *activates* the specific P-sections named below; it does not touch the rest of
the phases' scope.

### Wave 0 — H1 ∥ H2 ∥ H4, alongside the already-scheduled P01 §2.7/§2.8 and P02 §3/O19

**Members:** NEW H1, H2, H4; EXISTING P01 §2.7 (claim-latency appender) + §2.8 (v5c-reexec
harness, the P06 precondition), P02 §3 (write ADR-020) + O19 (proof-home ruling + fix the two bad
cites), plus the §4 quick-wins (all sub-hour, no dependencies).

**Parallelism rationale (file surfaces, verified against each blueprint's own evidence sections):**
H1 touches `kernel/src/event_log.rs` + `kernel/src/hydra.rs` (H1 §1). H2 touches
`engine/src/field_frame.rs`, `kernel/src/spectral.rs`, `kernel/src/wasm.rs`, `engine/src/bridge.rs`,
and the two spool tools (H2 §1). H4 touches only `.claude/hooks/`, `.claude/settings.json`, and
`docs/ledger/` — and only via operator application (H4 §3). P01 touches `.github/workflows/`,
`scripts/`, `deny.toml`, `docs/ledger/`; P02 touches canon docs and `docs/adr/` (P01 §5). These are
pairwise disjoint, so H1 ∥ H2 ∥ H4 ∥ P01 ∥ P02 is genuinely collision-free — with **two honest
caveats**, stated rather than papered over: (a) H2 and R2's P4 (also Wave 0) both edit
`kernel/src/wasm.rs` — H2 at the `LedgerOut`/`spectral_flat_logic` regions (H2 §1 sites 2, 4), P4
adding new wasm exports (R2 phase 4 row). Different regions, low merge risk, but it is one hot
file; if P4 runs concurrently, sequence the two `wasm.rs` edits within one lane rather than
claiming clean parallelism. (b) H4 and P01 both create files under `docs/ledger/` — different files
(`ritual-run.jsonl` vs `claim-latency.jsonl`), no conflict, but H4 §3.5 defines a retirement
dependency (H4's ledger folds into P01's when it lands) that implementers must honor.

**H1's entry gate (real constraint, not fake parallelism):** H1 §2.4 rewrites the signatures of
`raise_breach_alarm`/`ingest_peer_breach` — functions the G9 breach-witness arc landed *this
session*. H1's own re-verification says all G9 commits predate HEAD and `FileEventStore::insert`
is untouched by them (H1 §0), so H1 may start now — but only after confirming no further G9 work
(memory records gaps G3–G8 still open in that arc) has an open edit on `hydra.rs`. That
confirmation is H1's step 0, not an assumption.

**H4's wave label is honest-with-an-asterisk:** it has zero build dependencies, but every step is
operator-applied (`.claude/` is protected; H4 header). "Wave 0" means *can be offered now*, not
*will land now*.

**Immediate-ops note on row #5:** the gateway-revival command is one line
(`sudo hermes gateway install --system`, P12 §6) with no code dependency. Its blueprint home is P12
(Wave 2), but running the command — and correcting the false MEMORY.md "created/running" record —
should not wait for Wave 2. Flagged as an operator ops action executable today; the *reproducible*
half (in-repo timers) stays in P12 where it belongs.

### Wave 1 — P06 (full), P07 §6, P08 §4

**Members:** P06 in full (closes row #2 and, via §6, the independence half of #7); P07 §6
(tax-overflow fix, row #11) inside the P7 money phase; P08 §4 (claim-latency anomaly consumer,
the reader half of #27/#7).

**Dependency reasoning (from R2 §3, not invented):** P6 ← P1 (it wraps P01 §2.8's unsigned harness
with ML-DSA signatures — "the same runner", P01 §5) and ← P3 (C4b side-channel must close before
the hybrid K/V keys sign anything, P06 §2). P7 ← P1 (its RED tests are only *enforced* once P01's
`cargo-test` CI job exists). P8 ← P1 (its §4 detector consumes P01 §2.7's ledger — a hard data
dependency: no ledger, nothing to alert on). **Parallelism:** P6/P7/P8 are declared mutually
parallel-safe by R2 itself (P6 row: "Parallel-safe with 5, 7, 8") and the file surfaces confirm it:
P06 builds keygen/signer/gate machinery + a CI job; P07 §6 edits `kernel/src/money.rs`; P08 §4
builds a detector over `docs/ledger/claim-latency.jsonl`. Disjoint. One flagged softness: binding
the standalone `money.rs` one-liner (#11) to P7's phase discipline delays a MED money fix behind
CI work — accepted here for roadmap integrity, but noted in §6 as convenience-adjacent.

### Wave 2 — H3, P12 §3/§6

**Members:** NEW H3 (row #15); P12 §3 (restore-verify, row #4) + §6 (timers + gateway, rows #5/#6).

**H3's sequencing is a real constraint chain, stated explicitly:** H3 and H1 both touch
`kernel/src/hydra.rs` in different functions — H1 changes `FileEventStore::insert` and the
signatures of `raise_breach_alarm`/`ingest_peer_breach`; H3 adds `attest_integrity`/
`verify_attestation`/`record_probe_timeout` and *calls* `ingest_peer_breach` on its Locked path
(H3 §3.5). Because H1 changes the very signature H3 consumes (`ingest_peer_breach` becomes
`Result<(), StoreError>`, H1 §2.4), H3 must build **after H1 lands**, against the post-H1 API —
this is a data-shape dependency, not just a merge-conflict avoidance. H3's own header additionally
gates on: all in-flight G9 work settled (same file), P06's key_V seam existing (its verdicts are
key_V-signed — "or stub the key_V-role anchor behind the P06 seam", H3 §3.1), and P03/P09 for
ML-DSA transport and gossip. Net: H3's pure-kernel primitives start once H1 + Wave-1 P06 are in;
its transport legs ride P9, which R2 places in Wave 2. **P12 parallelism:** P12 ← P7 (durable
EventStore only after the dedup fix, R2 §3) and — a dependency R2 could not have known —
**P12 §4.2 ← H1**: its fsync-before-ack contract "is *impossible* on today's `-> ()` trait" (H1 §5),
so H1 is P12 §4.2's silent precondition, now named. P12 §3/§6 (archives, timers) share no files
with H3 (kernel) — genuinely parallel.

### Later waves — unchanged

Nothing in this plan touches R2 Waves 3–6. Row #13's O19 ruling (Wave 0) is *executed* at P13
(Wave 4) per R2 §4; row #26's real fix lives in R2 P4/P9 as already scheduled.

---

## §4 — Quick-wins checklist (no blueprint needed)

One line each; none blocks or is blocked by anything above.

- **#14 — retroactive-ADR doctrine note.** Add one sentence to `docs/adr/` (README or the template
  P02 §4's V4 work creates): "An ADR records a decision before the knowledge is lost — date the
  reasoning and the code separately; post-hoc ADRs must say so." One sentence of doctrine; folds
  naturally into P02's canon batch.
- **#17 — dead one-shot cron.** Expire/remove `bebop-library-star-list` via the `hermes cron` CLI
  (host-side, one command), or adopt fire-on-revive semantics when the gateway comes back (P12 §6).
- **#19 — second-process reproducibility tests.** One serialize→re-read (write output, re-read in a
  fresh assertion path) test per claimed surface: `ppr.rs`, `diffusion.rs`, `csr.rs`, `recall.rs`,
  `rng.rs` (audit row #19 evidence lines). A few lines each, kernel test modules.
- **#20 — transcendental-float doctrine.** One sentence next to the reproducibility banner
  (`rng.rs:4-6` / `spectral.rs` header): "transcendentals (`ln/sin/atan2/hypot`): reproducible
  per-target, not cross-target; only the integer RNG earns cross-platform bit-identity."
- **#28 — quarantined UI tree.** `rm -rf apps/web/node_modules/@deliveryos/.ignored_ui` (untracked,
  unimported); optionally add a CI grep forbidding `.ignored_ui` imports to P01's job batch. One
  command + one optional CI line.
- **#29 — stray artifact.** `rm 'kernel/=5'`. One command. ⚠ ADDED (decorrelated verification
  2026-07-16, `REMEDIATION-PLAN-VERIFICATION.md`): the file is covered by a dead `.gitignore:77`
  entry — delete that line too, or the ignore rule outlives the file it was written for.

---

## §5 — Backlog with named triggers (E53 waiver form)

Form per `BLUEPRINT-P02-canon-repair-operator-decisions.md` §4 (the rsa-triage exemplar,
`kernel/Cargo.toml:31`): `{what, why-suspended, named-owner, falsifiable-revisit-trigger, date}`.

- **#8 — ONE-Laplacian unification.** *What:* parity-bind the two live dense Laplacians
  (`spectral.rs:287`, bebop `field.rs:82`) or route both through `csr::laplacian_spmv`.
  *Why-suspended:* only two live implementations, no third consumer forcing unification; the
  canonical operator has zero production callers, so unification now is speculative wiring. Interim
  doc-fix (mark ONE-`L` aspirational in `ARCHITECTURE.md`) rides P02's Wave-0 canon batch.
  *Owner:* operator (kernel lane). *Revisit-trigger:* a third production `L=D−A` construction
  appears anywhere, OR `csr::laplacian_spmv` gains its first production caller — parity test lands
  in that same commit. *Date:* 2026-07-16.
  ⚠ TRIGGER FIRED (2026-07-16, `feat/spectral-energy-flow-evolution`): `csr::laplacian_spmv` now
  has a production caller — `engine/src/bridge.rs:125` (`VertexBridge::apply_field`, W20), wired
  public API though only test-reached today (not yet on a live runtime loop). The parity test this
  trigger calls for is now designed: `docs/design/spectral-energy-flow-evolution-2026-07-16/
  BLUEPRINT-E1-gradient-lyapunov-and-laplacian-parity.md` — a discrete grad/div incidence
  factorization pinning `csr.rs`/`spectral.rs` (agree, `+(D−A)`) against `field_frame.rs` (found
  live-diverging, `−(D−A)` — an unpinned sign-convention split this backlog entry did not know
  about in the original audit). Scope note: E1 deliberately does NOT merge all Laplacian call
  sites into one operator (three hot-path representations are kept, parity-tested instead) —
  narrower than this backlog item's original "route both through csr::laplacian_spmv" framing,
  and the honest reason is in E1 §2. Status: blueprinted, not yet implemented.
- **#9 — PPR triplication.** *What:* collapse `retrieval/ppr.rs` / `diffusion.rs` / `markov.rs`
  PPR math to one engine or parity-gate them; fix the R-LM design pointer to the survivor.
  *Why-suspended:* all three are live and individually tested; no consumer is currently misrouted
  in code (the wrong pointer is in a design doc, correctable in the P02 cite pass).
  *Owner:* operator (kernel lane). *Revisit-trigger:* the R-LM/physics-UI viz build begins (it must
  then pick one engine), OR a fourth PPR implementation is attempted. *Date:* 2026-07-16.
- **#21 — shared fold/replay primitive.** *What:* extract one replay/projection primitive from the
  per-subsystem `fold`s (`event_log.rs:300-319`, `order_machine.rs:137-151`, `intake.rs:47`).
  *Why-suspended:* the audit's own fix direction defers to "when the third consumer appears."
  *Owner:* whoever implements P7. *Revisit-trigger:* **named and near** — P07 §5 routes money
  ledger-entry events through `commit_after_decide`, which is plausibly the third fold consumer;
  the P7 implementer must check this trigger at build time, not after. *Date:* 2026-07-16.
- **#22 — graph-conversion hub + `mat.rs` retirement.** *What:* one `from_edges` conversion hub;
  finish the retirement `mat.rs:6-9` already declares. *Why-suspended:* divergence is latent, not
  live; consolidation without a forcing consumer is churn. *Owner:* whoever implements P4.
  *Revisit-trigger:* P4's road-graph ingestion port onto `csr.rs` (R2 phase 4) — the natural moment
  a new conversion is written; do the hub then. *Date:* 2026-07-16.
- **#25 — eigenvector second party.** *What:* second-solver *vector* parity (values already have
  one). *Why-suspended:* vectors are currently validated by residual, and no P0/P1 effect consumes
  eigenvector direction. *Owner:* operator (kernel lane). *Revisit-trigger:* any gate/signature/
  idempotency effect starts consuming eigenvector components (not just values/residuals), OR a
  second eigensolver is added for any other reason. *Date:* 2026-07-16.
- **#26 — hub↔mesh-node topology / M7 heal.** *What:* a topology primitive shared (or
  parity-bound) between the hub graph (`hydra.rs:41`) and mesh-node. *Why-suspended:* this is a
  **feature gap already owned by the sovereign roadmap**, not remediation debt — the math lands
  once in R2 P4, the heal layer in R2 P9 (R2 §1 "Major merges", Phase 9 row); building it here
  would duplicate that plan. *Owner:* P9 implementer. *Revisit-trigger:* P9 lands — verify at its
  done-test that the heal representation is shared/parity-bound with the hub graph per this
  finding's fix direction, and close #26 in the same review. *Date:* 2026-07-16.

---

## §6 — The 2-question doubt audit, applied to THIS plan

Per `AGENTS.md` (Detailed Planning Protocol step 6 + the 2Q ritual, mandatory at the planning
stage), answered for this document specifically.

**Q1 — least confident about (concrete):**

1. **The H2 ∥ P4 `wasm.rs` collision.** H2's header claims parallel safety only against H1/H3/H4;
   I extended Wave-0 parallelism across P-phases and found `kernel/src/wasm.rs` is shared with P4.
   I judged the regions disjoint from the blueprints' line citations — I did not re-read the live
   file to confirm P4's export additions won't touch `LedgerOut` or `spectral_flat_logic`.
2. **`hydra.rs` quiescence for H1 is asserted from H1's text, not re-verified live here.** H1
   checked G9 commits predate HEAD, but the hydra arc has open gaps (G3–G8 per memory); if any is
   in flight, H1's Wave-0 start collides. Mitigated by making it H1's explicit step-0 gate, but
   this plan did not run the `git log kernel/src/hydra.rs` check itself.
3. **Quick-win facts carried, not re-probed.** `kernel/=5`, the `.ignored_ui` tree, and the dead
   cron's `[active]` state are cited from the same-day audit, unverified in this pass — a small,
   ironic RC-1 residue inside the RC-1 remediation plan.
4. **Row #7's one-cell mapping is a simplification.** Its fix spans three blueprints across two
   waves; a reader implementing only P01 §2.7 could believe #7 closed when only the feed exists.
   The table says "three legs," but a table cell cannot enforce that all three land.
5. **#11's Wave-1 placement is roadmap-faithful but not technically forced.** The `money.rs` fix
   is standalone; parking it behind P1 is phase discipline, not dependency. If a money bug bites
   before Wave 1, this sequencing was the wrong call.
6. **H4's "Wave 0" conflates *offerable now* with *lands now*** — it is operator-gated and may
   never land; the RC-2 closure story leans on it more than its probability warrants.
7. **I did not exhaustively re-derive H2's five bundled sites for collisions with P5/P8 telemetry
   work** (the spool tools appear in both H2 §2.5 and P08's spool-sink design); I believe P08 §3
   consumes `spool.rs` as an adapter rather than editing the spool mains, but that is a reading,
   not a diff-level check.

**Q2 — the biggest thing this plan might be missing:** completeness first — I re-audited §1
against the audit table row-by-row: all 29 rows appear exactly once across the four buckets
(17 blueprint-mapped + 6 quick-wins + 6 backlog), no silent drops. The real miss is structural:
**nothing in this plan makes the audit recur.** RC-1's stated antidote is a *periodically re-run*
mechanical probe (`HERMETIC-ARCHITECTURE-PRINCIPLES.md` §2 RC-1: "making that antidote recur is
itself a Rhythm problem"), and this plan schedules fixes but no re-audit cadence — the §1 table
will drift stale exactly the way MEMORY.md's cron record did. By the audit's own vocabulary, this
plan is an outward swing with no return swing: a half-pendulum about pendulums. Cheapest honest
fix: when P12 §6's timers land, add one timer that re-runs the cite-with-probe checks (resolve
ADR paths, grep `csr` callers, read `last_run`) and diffs against this table — flagged here for
the P12 implementer, not silently absorbed.

---

## §7 — Anu (logic) & Ananke (organization) check

**Anu.** The sequencing is derived, not convenient, in the places that matter: every P-wave number
was read from R2 §3 rather than re-invented; H3-after-H1 rests on a named data-shape dependency
(H1 changes the `ingest_peer_breach` signature H3 calls) plus a named shared-file constraint —
both checkable by any reader against the two blueprints' §2 sections. Where an ordering is *not*
logically forced, the plan says so instead of dressing it up: #11 riding P7 is phase discipline
(§6.5), #5's Wave-2 home has an immediate-ops escape hatch (§3), H4's wave label carries an
asterisk. The honest Anu weakness is §6.1: one parallelism claim (H2 ∥ P4) was extended beyond
what any source document asserts and rests on region-level reasoning I did not verify at
diff-level. That claim is therefore marked as a lane-sequencing instruction rather than a
parallelism guarantee — a decision downgraded to match its evidence, which is what Anu demands
when derivation runs out.

**Ananke.** Partially passes, and says where it fails. What survives without anyone remembering
this document: the underlying blueprints' falsifiable acceptance criteria (H1 §4, H2 §4, H3 §4,
H4 §4, P01 §4, P06 §7, P07 §7, P12 §8 — each a command or test that passes or doesn't), and the
§5 backlog entries, which are deliberately written in the E53 form the repo already treats as its
canonical waiver shape, each with a trigger a future session can evaluate cold ("did
`laplacian_spmv` gain a caller?") without knowing this plan exists. What does *not* survive on
structure alone: the §1 mapping itself and the multi-leg closure of row #7 — both depend on a
future reader consulting this file, and §6.Q2 already convicted the plan of having no re-audit
return swing. The two mitigations are named, not hoped: H4's hook (if the operator applies it)
makes plan-artifact residue mechanically observable, and the P12-timer re-probe flagged in §6.Q2
would turn this table from a document into a checked invariant. Until one of those lands, this
plan's organization is better than memory but short of necessity — recorded here so the gap is a
known debt with an owner, not a silent assumption.

---

*Sources: HERMETIC-ARCHITECTURE-PRINCIPLES.md (§2, §3, §4); BLUEPRINT-H1/H2/H3/H4 (this
directory); ../sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md (§2, §3, §4);
BLUEPRINT-P01 (§2.7, §2.8, §5), P02 (§3, §4, O19), P06 (§2, §6, §8), P07 (§6), P08 (§4),
P12 (§3, §4.2, §6); AGENTS.md (2Q ritual, Detailed Planning Protocol, Anu/Ananke doctrine).
No code or non-plan file was written or edited.*
