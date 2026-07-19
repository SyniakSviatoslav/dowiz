# Synthesis — "Truthfulness" vs "Validity" Disambiguation, Kleene K3 Admission Logic, and Proportionate Open-Source Hardening (Miri / Sentinel / Shadow-Mode / Contribution Gates)

**Date:** 2026-07-19 · **Role:** synthesis + blueprint (Fable pass) over the completed Opus
grounding `RESEARCH-KLEENE-TRUTHFULNESS-OPENSOURCE-HARDENING-2026-07-19.md` (trusted as factual
base — findings cited, not re-derived). **Sources:**
`RAW-PROMPT-6-truthfulness-logical-deduction-kleene-unknown-2026-07-19.md` +
`RAW-PROMPT-7-opensource-hardening-miri-kani-sentinel-shadow-mode-2026-07-19.md` (one combined
dialogue), `SWARM-SAFETY-SYNTHESIS-2-truthfulness-time-metric-2026-07-19.md` (the colliding
earlier arc), `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I items 45–49 (the design
this pass extends). **Epistemic tags:** **GROUNDED** = cited against the research pass /
verified-in-tree · **PROPOSED** = designed here, unbuilt · **RULING** = a decision this pass
makes and records. Continues the arc's numbering: new roadmap items are **50–54** (roadmap §J).

**Repository-state fact, stated up front (Part-4 flag, operator-facing):** a substantial amount
of this session's REAL kernel work — not just docs — still lives ONLY on the unmerged branch
`exec/space-grade-tier0-2026-07-19` (worktree `/root/dowiz-wt-space-grade-exec`, HEAD
`ae2da4a9d` at research time): all of Tier 1 (zero-dep gate, toolchain pin, the entire
`kernel/src/fdr/` module + `Reading<T>`/`Absence`, the `regex` retirement), item 6's
`hardening-gate` + `ct_gate.rs` dudect + `HOT-PATHS.tsv`, the item-16 spectrum collapse, the
item-30 `resume()` defect fix, and more. `main` carries only the planning/audit documents.
**GROUNDED** (research doc, header + §§3–7). This is recorded here for the operator's awareness;
merging is deliberately NOT this pass's scope. Every design below that touches FDR or the
zero-dep gate therefore carries an implicit "after the exec branch merges" prerequisite, stated
per item where load-bearing.

---

## Part 1 — Terminology RULING: the new concept renames to "Validity"; "Truthfulness" stays byte-reproducibility

### 1.1 The conflict is real (GROUNDED, decided by the research pass)

The two "Truthfulness" concepts are genuinely different and orthogonal, not one property from
two angles:

- **Swarm-safety "Truthfulness"** (`SWARM-SAFETY-SYNTHESIS-2` §1, lines 33–41) is **content-free**:
  byte-for-byte identical input ⇒ byte-for-byte identical output, checked across time; "no
  interpretation of the value's *content* is needed." It is a cache-corruption / substitution
  detector. It cannot detect a stably-wrong answer, by design.
- **RAW-PROMPT-6 "Truthfulness"** is **content-based**: an output counts only if it is validly
  derived from stated axioms via a reasoning path the kernel checks against logic rules
  (RAW-6 lines 56, 74–75). It cannot detect a swapped weight file that still emits valid-looking
  derivations, by design.

Counterexamples in both directions exist (research doc, "Orthogonality shown by counterexample").
The research pass refuted the "byte-reproducibility is the mechanism that makes validity
checkable" bridge: validity is checked by a proof-checker over one output's reasoning path,
needing no re-run; what IS shared is that the *checker/Gatekeeper* must itself be deterministic —
a property of the kernel, not of the model output. **GROUNDED.**

### 1.2 The RULING: rename the NEW concept, and the correct new name is "Validity"

**Decision: "Truthfulness" remains, exclusively and untouched, the swarm-safety arc's
byte-reproducibility property. The RAW-PROMPT-6 concept is renamed "Validity" (long form:
derivational validity) everywhere in the space-grade roadmap from item 50 onward.** Three
reasons, in order of force:

1. **Precedence and blast radius.** The swarm-safety definition is earlier-established in this
   session's history and already load-bearing across that arc's own roadmap, blueprints, and
   CORE-ROADMAP-INDEX §7 row ("truthfulness-as-reproducibility" is in the row text itself).
   Renaming it would require edits across another live arc's documents — exactly the
   cross-lane mutation this shared-checkout session must avoid. The new concept, by contrast,
   exists only in RAW-PROMPT-6/7 and the research doc; renaming it costs one terminology table.
2. **The rename is a correction, not a compromise.** In formal logic, the property RAW-PROMPT-6
   describes — "if the system accepts axiom A and rule A→B, then B; emitting C is a logical
   invariant violation" — is the textbook definition of **validity** (conclusion follows from
   premises by the inference rules). "Truthfulness"/soundness would additionally claim the
   axioms are TRUE OF THE WORLD, which the kernel neither checks nor can check — it checks
   derivations from the Axiomatic Context it is handed. Calling the checked property "Validity"
   is strictly more honest than the dialogue's own word. (The dialogue concedes the point
   implicitly: its "Unknown" state is exactly the admission that truth is not established.)
3. **Zero new vocabulary.** Item 47's already-ruled design (roadmap §I) admits advice via
   `admit(Proposal, &Invariants) -> Result<ValidatedProposal, Rejection>` — the type
   `ValidatedProposal` already names this property. "Validity" is the property `admit`
   certifies; the terminology lands on a type that exists in the design today.

### 1.3 Precise definitions going forward (RULING — the disambiguation table future docs cite)

Following the in-repo precedent of maintaining a terminology-collision table
(`CRASH-CONSISTENCY-…-SYNTHESIS` §4: seL4-capability, OTP-supervisor, TMR, `no_std`), the
binding entries are:

| Term | Definition | Plane | Home |
|---|---|---|---|
| **Truthfulness** | Byte-for-byte identical output for byte-for-byte identical input and conditions, checked across time; any divergence is a signal. Content-free. | Anti-substitution / anti-corruption (detects a *changed function*) | Swarm-safety arc (`SWARM-SAFETY-SYNTHESIS-2` §1); replay probes; NOT a space-grade-roadmap term |
| **Validity** (derivational validity) | A proposal is *valid* iff every step of its supplied reasoning/evidence path is accepted by the kernel's deterministic checker against the stated axioms and invariant laws. A missing or incomplete path never defaults to valid — it downgrades to Undecidable (§2.1). Content-based. | Anti-hallucination / anti-invalid-inference (detects an *unsupported conclusion*) | Item 47's admission gate, extended by item 50; certified by `ValidatedProposal` |

The two compose rather than compete: Truthfulness guards the *function* being the one you
deployed; Validity guards each *output* being derivable. A system can want both; neither
implies the other. **Where they meet is one shared requirement — the checker itself is
deterministic** — which the kernel already satisfies as standing law (the whole §H/§I
determinism discipline). Cross-reference only; no shared implementation.

---

## Part 2 — Five scope calls, proportionate, grounded

### 2.1 Kleene / 3-valued logic → NOT a parallel kernel-wide type; a third verdict class inside item 47's existing design (PROPOSED → roadmap item 50)

**GROUNDED baseline:** no 3-valued logic exists anywhere in-tree; `Reading<T>` is two-state
named-absence, `JobStatus::Unknown` is a poll status, not epistemic (research §1). The item-47
design (roadmap §I) already covers the True/False cases: `Ok(ValidatedProposal)` = admitted,
`Err(Rejection)` = refused, deterministic path total either way.

**Design RULING — where "Unknown" lives:**

- **The public seam does NOT grow a third control-flow arm.** The admission signature stays
  `admit(Proposal, &Invariants) -> Result<ValidatedProposal, Rejection>`. Rationale: at the
  decision seam, Kleene-False ("demonstrably invalid") and Kleene-Unknown ("cannot be
  evaluated — evidence incomplete") MUST be behaviorally identical — advice unused,
  deterministic path taken. A third public arm would create a place where "Unknown" could be
  handled more leniently than "False" — precisely the "Unknown dissolving into the system"
  failure RAW-PROMPT-7 warns against. Two-arm control flow also keeps
  illegal-state-unrepresentable intact: `ValidatedProposal` is constructible only through
  `admit`, unchanged.
- **"Unknown" is a classification ON the rejection, not a sibling OF it.** `Rejection` carries a
  two-class cause: `RejectionClass::Refuted` (a named invariant/inference rule demonstrably
  violated — Kleene False) vs `RejectionClass::Undecidable` (the evidence/derivation chain is
  incomplete, absent, or exceeds the checker's bounded budget — Kleene Unknown). The class
  rides on the existing "every `Rejection` emits an FDR event" clause (item 47) so telemetry,
  shadow-mode (§2.4), and item 9's breaker can weight them differently (repeated `Refuted` =
  a misbehaving model; a high `Undecidable` rate = an advice domain the model cannot serve),
  while the decision seam cannot tell them apart.
- **The literal K3 enum exists, but as an INTERNAL combinator type of the admission module.**
  `#[repr(u8)] enum TruthState { False = 0, True = 1, Unknown = 2 }` (the RAW-6 sketch, adopted
  as-is) is the return type of each individual invariant/step check *inside* `admit`; the
  strong-Kleene tables are the fold: any `False` short-circuits to `Refuted`
  (`False & Unknown = False` — the kernel may act on a definite refutation even with gaps
  elsewhere); no `False` but any `Unknown` folds to `Undecidable` (`True & Unknown = Unknown`);
  all `True` admits. This is K3-AND over the conjunction of checks — exactly the propagation
  rule RAW-6 specifies — without exporting a kernel-wide truth type nobody else needs.
- **Evidence-based Unknown, adopted.** The checker never reads model confidence/logits; a
  proposal missing a complete evidence chain to True or False is forcibly `Undecidable`
  (RAW-6's closing recommendation). Self-reported confidence is not an input to `admit`, full
  stop.
- **`None` ≠ `Unknown`.** The seam's `Option<Proposal>` `None` (AI absent/crashed — item 45/47)
  means *no advice was offered*; `Undecidable` means *advice was offered and could not be
  evaluated*. Both take the deterministic path; they are distinct facts and log distinctly.
- **Testing: exhaustive, not property-based.** RAW-PROMPT-7's own observation is correct and
  adopted: 3 states ⇒ the FULL input space of each binary operator is 9 cases (+3 for NOT) —
  written out literally as exhaustive tests (the house 65 536-pair standard, trivially
  satisfied). proptest is NOT needed for the truth tables and adding it would be theater;
  proptest stays where it is — dev-only, 400-case payment invariants (`payment.rs:657`,
  `payment_provider.rs:1122`, `tests/firewall_p47.rs`), compatible with the zero-dep gate iff
  strictly `[dev-dependencies]` (research §2). Any NEW property tests this arc adds inherit
  that same law.
- **Kani for K3 = item 7 scope growth, not a new item.** RAW-PROMPT-7's "prove `TruthState`
  can never reach an unhandled state" is exactly item 7's class of work (Kani is 100% item-7
  scope, untouched by item 6 — research §4). The admission fold joins item 7's named target
  list (Keccak, FSM, NTT, GCRA, + K3 admission fold); no duplicate verification item is
  created.

### 2.2 Miri → adopt, scoped to the real unsafe surface; honest about SIMD limits (PROPOSED → roadmap item 52)

**GROUNDED baseline (inventory independently re-verified 2026-07-19 — the research/RAW-prompt
count was wrong and is corrected here):** Miri exists nowhere as a check (zero workflow hits;
aspirational doc-comments only; `ROADMAP-LIVE-STATUS-2026-07-18.md:24` even records "component
absent this toolchain"). The kernel's *real* `unsafe` surface (matching `unsafe fn|impl|trait|{`,
excluding comment mentions) is **19 blocks in only 4 modules**: `arena.rs` (6), `simd.rs` (5),
`fdr/pmu.rs` (5 — `_rdtsc` + raw `syscall5` FFI; exec-branch only, joins the surface post-FDR-
merge), `householder.rs` (3). **CORRECTION — `messenger.rs`, `slot_arena.rs`, `chaos.rs`, and
`bounded_drainer.rs` contain ZERO real `unsafe`** — every `unsafe` token in them is a *comment*
(`slot_arena.rs`'s doc-comment literally states "No `unsafe` in this wrapper. The only `unsafe`
is upstream in thunderdome's [arena]"). The prior "21 blocks across 7 modules" figure
double-counted those comment mentions and *omitted* `fdr/pmu.rs` entirely (verified false at
research HEAD `ae2da4a9d` too, not a since-changed-code artifact). `pq/` (crypto) still has ZERO
unsafe — the raw prompt's crypto guess remains wrong. **GROUNDED (re-verified in-tree).**

**Scope RULING:** one targeted CI job, `miri-gate`, running `cargo miri test` restricted to the
genuinely unsafe-bearing modules' test filters — `arena.rs` (the real bump allocator, 6 blocks,
the classic Miri payoff) plus the scalar paths of `simd.rs`/`householder.rs`. NOT a blanket
miri-everything mandate, and explicitly NOT the four unsafe-free wrappers the old list named
(filtering on them would match zero unsafe and is pure theater). `fdr/pmu.rs` joins the filter
when the FDR branch merges — but its `unsafe` is real `_rdtsc`/raw-syscall FFI, which (like the
SIMD intrinsics below) Miri cannot interpret, so it lands in the same "exercises the fallback /
errors under interpretation" bucket, documented not silently green. Honest limitations, stated now:

- **SIMD intrinsic bodies are largely outside Miri's reach** — `core::arch` AVX2 intrinsics are
  substantially unsupported under interpretation. The house SIMD pattern is runtime-detected
  with a scalar fallback (item 39 / `simd.rs`); under Miri, feature detection reports
  unavailable, so the interpreted run exercises the scalar path. Coverage for the intrinsic
  bodies therefore stays where it already is: the bit-identity differential oracles
  (items 37/39, `ring_mul`-standard `debug_assert`s) + item 7 (Kani/assembly taint, where the
  item-14 baseline already deferred the per-branch proof). The gate documents this split so a
  green `miri-gate` is never misread as "SIMD is Miri-clean." **PROPOSED — exact intrinsic
  support to be confirmed empirically on first run, not asserted.**
- **Toolchain:** Miri requires a nightly component; the build toolchain stays pinned at 1.96.1
  (item 14 — untouched). The `miri-gate` job pins its OWN separate analysis nightly
  (`nightly-YYYY-MM-DD`, recorded in the workflow + a line in `docs/audits/toolchain/`), and
  bumping THAT pin follows the item-14 artifact discipline in spirit (recorded bump, not
  floating). The analysis toolchain never builds shipped artifacts, so the item-14 gate's
  letter is not violated.
- **Proof condition (P7):** a planted UB test (e.g. a deliberate out-of-bounds or
  use-after-free behind `#[cfg(miri_selftest)]`) demonstrably turns the gate RED before it
  counts as landed; the clean run is green; zero-match filters are RED (item-6 anti-forgery
  clause reused).

Priority: medium — `arena.rs`'s bump-allocator logic is the classic Miri payoff, and 6 of the
kernel's 19 real unsafe blocks sit in `arena.rs` alone (the largest single concentration).
Independent of items 50/51; the on-`main` targets (`arena`/`simd`/`householder`) can be
dispatched anytime; `fdr/pmu.rs` folds in when the FDR branch merges.

### 2.3 Sentinel / read-time integrity check for critical LIVE in-memory structs → ADOPT, proportionately scoped (RULING → roadmap item 54)

> **Disposition reversed by operator ruling (2026-07-19), recorded in full.** An earlier draft of
> this pass REJECTED the Sentinel pattern on the argument that the kernel runs on "commodity ECC
> cloud hardware, so the space-grade framing is aspirational." The operator rejected that reasoning
> on two independent grounds: (i) genuine space-grade *engineering quality* is the standard for this
> whole arc regardless of the literal substrate — a soft deployment assumption is not a valid reason
> to skip real hardening; and (ii) the deployment premise was **factually wrong** — the actual
> target for this kernel arc is **local, offline-first, consumer-grade hardware, which typically
> LACKS ECC**. Under the corrected premise the in-memory single-/multi-bit-flip fault class is
> *higher*, not negligible, so the mechanism's justification is genuinely stronger, not merely
> "build it anyway." Both corrections are applied below; the old rejection is superseded.

**GROUNDED baseline:** the exact pattern (a live in-memory struct carrying an integrity checksum,
recomputed and compared at read/use time, mismatch ⇒ Safe State) exists nowhere; everything close
is AT-REST — `backup.rs` content-addressed verify-on-get (`:122/:252/:689`), `event_log.rs`
chain-walk `verify_chain` (`:212`), FDR ring per-line CRC32 (exec branch) (research §5). The
live-struct read-time case is a genuine gap. **GROUNDED.**

**Proportionality RULING — build it, right-sized on three axes** (the discipline that keeps this
real hardening rather than blanket theater):

1. **Which structs qualify as "critical" (scope axis).** NOT every struct — that *would* be the
   theater the reversed draft feared. The qualifying set is narrow and enumerable: a struct is a
   Sentinel candidate iff it is **(a) long-lived, (b) an authority input to a money/safety/decision
   path, and (c) has no at-rest backing that already verifies it.** Concrete candidate registry
   (each justified at build time): item 47's loaded **`Invariants` table** (a bit-flip in an
   invariant bound silently mis-certifies *every* subsequent `admit` — the highest-value target);
   item 21's **gain-schedule / decision-configuration** state; the **`ActiveAIContext`-class live
   inference config** RAW-PROMPT-7 named — *distinct* from item 40's read-only weights (see axis 3).
   Explicitly EXCLUDED: transient hot-loop scratch, and anything already at-rest-verified
   (`event_log` chain, `backup` CAS) or already covered by item 40.
2. **What primitive + how often (cost axis — zero-dep, reuse-only).** The check REUSES an existing
   in-kernel primitive: the **hand-rolled CRC32 already built for the FDR module this session**
   (the same one item 40 reuses — P2, no second CRC, no new algorithm, no external crate; consistent
   with the dual-Keccak dedup discipline). CRC32, not a cryptographic hash: the threat model is a
   *hardware memory fault*, not an in-memory adversary, so fault-detection strength is what's needed
   and cryptographic cost would be waste. Frequency is **at defined transition points, NOT every
   field read** (that per-read hot-path tax was the one sound half of the old objection, kept):
   (a) once per *authority-use* — e.g. once per `admit` call over the `Invariants` about to be
   checked, amortized across the whole admission, not per-field; (b) recompute-and-store on
   mutation, which for these long-lived structs is rare and centralized (the invariant table is
   effectively immutable-after-init → pure read-time check, zero re-hash burden — the ideal case
   with no false-trip surface). The "missed re-hash site manufactures a false trip" risk is thereby
   bounded to a handful of centralized write sites, not smeared across the codebase.
3. **Relationship to item 40 (redundancy check — genuine, and it defines the boundary).** Item 40
   (roadmap §H) IS the sentinel pattern for read-only embedded **weights** (build-time golden CRC32
   over static data). That overlap is real — so item 54 does NOT re-cover weight integrity; it
   scopes to the **live MUTABLE critical structs item 40 structurally cannot touch** (the
   `Invariants` table, gain-schedule, live inference config). Same primitive, same Safe-State
   semantics, complementary surfaces, one CRC implementation shared. The overlap is a boundary, not
   a reason to skip the mechanism.

**Safe State on mismatch:** a checksum mismatch is hardware/memory-fault evidence (item-40
semantics). Hard-fail: emit ONE fsynced FDR `Alarm` record (the fault evidence) and take the
deterministic fail-closed path — for the `Invariants` case that means REFUSE the admission (a
corrupted invariant table cannot certify anything), composing with item 47's `Rejection`/
deterministic seam and, when item 9's breaker lands, routing through `Result<Permit, Tripped>`
(the exact composition item 40 uses — the design does NOT gate on item 9).

**Justification is present-tense, not deferred.** The reversed draft's "reopening triggers"
(non-ECC/edge deployment; a long-lived mutable safety-critical struct) are now *already satisfied*
by the corrected deployment premise (local consumer hardware without ECC) and the `Invariants`
table's existence — so the item is dispatched, not parked. **PROPOSED (design here) → roadmap
item 54.** Dependency: the strongest instance rides {item 47 wiring (after item 42) + item 50} +
the FDR branch merge (for the `Alarm` record) → parallel with item 51; the critical-struct
registry enumeration can begin now; the gain-schedule instance rides item 21.

### 2.4 Shadow mode → adopt, properly scoped; the highest-value genuinely-new item (PROPOSED → roadmap item 51)

**GROUNDED baseline:** genuinely new — every existing differential mechanism treats
disagreement as fail/reject (`decision/import.rs` `ReplayDisagreement` rejects even though it
emits telemetry; the pq/spool/spine/stats differentials are tests). The only advisory
precedents are narrower: `metrics.rs:85/:382` advisory anomaly flag (deterministic, merge-plane)
and `leak_gate.rs` fail-closed advisory (research §6). Swarm-safety's replay-probe concept is
design-only and lives in the OTHER plane (Truthfulness, §1.3) — no overlap.

**Design (concrete, hooks into item 47's seam):**

- **Where it hooks.** Item 47's decision seam already computes the deterministic decision D as
  the total, primary function; advice arrives as `Option<Proposal>`. Shadow mode is therefore
  nearly free: no second execution lane is built — the comparison object already exists on
  every decision. On `Some(proposal)`, after `admit` returns, compare the proposal's proposed
  action against D and log the triple (admission verdict class, agree/disagree bit, digests).
  On `None`, nothing (absence is already logged by item 47's own path).
- **What gets logged — a new FDR `Kind::ShadowDivergence` variant** (closed-enum growth, the
  exact item-48 `Heartbeat` precedent), carrying: decision-site id, `RejectionClass` or
  Admitted (from §2.1), the agreement bit, and short digests of D and the proposed action —
  digests, not payloads (minimal-statistic discipline; FDR lines stay small and the
  byte-identical-elsewhere rule holds via the item-27 optional-field precedent: records
  without the field are untouched). So yes — disagreement DOES get its own FDR event schema
  surface, as a new `Kind` variant + its payload, NOT as a new field smeared across existing
  kinds.
- **Emission policy.** Log every disagreement and every Admitted-but-differs case; log
  `Undecidable`-while-D-decides at a bounded rate (it is the "model adds nothing on this
  domain" base-rate signal RAW-PROMPT-7 flags as PostMortem-worthy); agreement is SAMPLED at a
  low fixed rate (base-rate denominator), never logged per-event. Bounded emission keeps the
  FDR ring's replay-bounded-by-construction property (item 49's rationale) intact.
- **Non-gating, by definition and by test.** The variant is advisory: no build fails, no
  decision changes, no breaker trips on a shadow event alone. Aggregated `Refuted`-class
  counts still route to item 9's breaker via item 47's OWN rejection events — shadow mode adds
  observation, never authority. A test asserts the deterministic output is bit-identical with
  shadow logging on vs off (the item-47 `None`-path test pattern, reused).
- **What it buys.** This is the measurement infrastructure that answers "is the AI advice worth
  anything, per decision site, in the field" BEFORE any advice is ever trusted — the honest
  prerequisite to ever widening AI authority, and the field-data source item 33-class
  re-measurement passes will want. It is deliberately scoped as the full design now (not
  underscoped for novelty): variant, policy, and proof are all specified above; the build
  waits only on item 47's wiring plus the FDR branch merge.

### 2.5 CI contribution gates (clippy / fmt / miri-required) → real, LOW priority, sequenced behind everything above (PROPOSED → roadmap item 53)

**GROUNDED baseline:** none of the raw prompt's triad exists (zero clippy/fmt/miri occurrences
in any workflow); the real existing gates are cargo-test, dco-check (`ci.yml:210-226`, on
main), decart-dep-lint, v5c-reexec, gitleaks, supply-chain, bench-regression, etc.; item 6's
hardening-gate partially prefigures "proof of PR" for hot paths but is on the unmerged exec
branch. AND: open-sourcing is NOT imminent — ADR-0020 is Accepted (AGPLv3 landed) but the
public flip + EUTM are explicitly operator-gated and unauthorized (research §§7–8).

**Priority RULING:** the raw prompt's urgency ("any PR is an attack vector") presumes an open
contribution surface that does not exist and is not authorized to exist yet. De-prioritized
accordingly: item 53 is sequenced LAST, explicitly behind items 50–52, blocking nothing.
Scope when it runs: one cheap `lint-gate` job — `cargo clippy --deny warnings` +
`cargo fmt --check` (both components are ALREADY pinned by item 14's `rust-toolchain.toml`
`components=[rustfmt,clippy]`, so the job costs minutes to add); miri-as-required simply
promotes item 52's existing job to a required check (no new machinery). One honest caveat
carried from item 14's own ledger: every `ci.yml` gate is advisory until marked a required
status check in branch protection (server-side, G5-owed) — item 53 inherits that owed step.
**Named escalation trigger:** the moment the operator authorizes public-flip preparation
(ADR-0020's gate), item 53 jumps the queue and becomes a pre-flip blocker, alongside the
ADR-recommended all-origin-refs gitleaks sweep. Until that trigger, low priority is the
grounded call.

---

## Part 3 — Disposition summary (what became an item, what didn't)

| RAW-PROMPT-6/7 proposal | Disposition |
|---|---|
| Truthfulness-as-logical-deduction | Renamed **Validity** (§1 RULING); concept adopted into item 47's admission gate via item 50 |
| Kleene K3 `TruthState` enum | Adopted as INTERNAL combinator of `admit` + `RejectionClass::{Refuted,Undecidable}`; no parallel public type → **item 50** |
| Evidence-based Unknown | Adopted verbatim (confidence/logits never an input) → **item 50** |
| PBT (proptest) for K3 truth tables | Superseded by exhaustive 9-case enumeration (RAW-7's own better idea); proptest stays dev-only where it is → folded into item 50's proof |
| Kani for `TruthState`/Guardian | Item 7 target-list growth, NOT a new item (Kani is wholly item-7 scope) |
| Miri as mandatory | Adopted, scoped to the real **19-block / 4-module** unsafe surface (`arena`/`simd`/`fdr-pmu`/`householder`; the old 21-block/7-module list corrected — 4 named modules are unsafe-free), SIMD/PMU intrinsic limits stated → **item 52** |
| Sentinel / read-time `integrity_hash` on critical LIVE structs | **ADOPTED, proportionately scoped** (§2.3; operator-reversed from an earlier draft rejection) — reuses the in-kernel CRC32, checks at transition points, scoped to live mutable authority structs item 40's weight checksum does NOT cover → **item 54** |
| Shadow-mode differential logging | Adopted, full design (FDR `Kind::ShadowDivergence`, non-gating, bounded emission) → **item 51** |
| clippy/fmt/miri contribution gates | Adopted at LOW priority, trigger-escalated on public-flip authorization → **item 53** |
| "Which kernel regions feel least verified?" (dialogue's closing question) | Left as a genuine operator question (per RAW-7's own capture note) — flagged, not guessed. The §2.2 unsafe-density ranking is the mechanical answer; the *felt*-uncertainty answer is the operator's |

New roadmap items: **50, 51, 52, 53, 54** — appended as §J of
`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (item 54 = the operator-reversed Sentinel).
Planning only; no item starts before the operator dispatches it.
