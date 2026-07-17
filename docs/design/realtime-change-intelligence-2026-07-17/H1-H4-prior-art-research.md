# H1–H4 Prior-Art Research (external grounding for the resolve step)

> Input to the design-council resolve pass for `proposal.md` / `breaker-findings.md`.
> Method note: this session's WebSearch budget was exhausted before this task started, so
> grounding was done by direct WebFetch of primary sources (papers, official docs, tool READMEs);
> two paywalled/403 sources are cited by name and explicitly labeled "not fetched".
> Standing rule applied throughout: YAGNI / smallest-correct-fix (per the operator's session rule
> and the C4b precedent — the simpler existing idiom beat the sophisticated rewrite).

---

## H1 — Concurrent multi-producer chain fork

### What real systems do

1. **Optimistic concurrency (CAS-with-retry on the tip).** The standard event-store answer.
   EventStoreDB/Marten append with an *expected version*: "When appending to a stream, a writer
   specifies an expected version and the server verifies it matches the current state before
   allowing the write. … the second writer receives a conflict error … and must retry by
   re-reading the current state." Dudycz: "Optimistic concurrency is also fundamental in ensuring
   the order of events in Event Sourcing."
   ([event-driven.io — Optimistic concurrency for pessimistic times](https://event-driven.io/en/optimistic_concurrency_for_pessimistic_times/))
   Git itself — the closest structural analog to this hash-chain-with-a-tip-ref — implements
   exactly this as `push --force-with-lease`: "This option overrides this restriction if the
   current value of the remote ref is the expected value. git push fails otherwise."
   ([git-scm.com/docs/git-push](https://git-scm.com/docs/git-push))

2. **Single-writer funnel.** Martin Thompson's Single Writer Principle: funnel all mutations
   through one writer; "For highly contended data it is very easy to get into a situation whereby
   the system spends significantly more time managing contention than doing real work." Queues +
   one consumer (actor style) serialize without CAS retry loops at all.
   ([Mechanical Sympathy — Single Writer Principle](https://mechanical-sympathy.blogspot.com/2011/09/single-writer-principle.html))
   Kafka's per-partition total order (one leader serializes each partition's appends) is the same
   principle at distributed scale (kafka.apache.org design docs; standard reference, ordering
   section not directly quotable via fetch this session).

3. **CRDTs / vector clocks.** A G-Set/OR-Set of deltas keyed by content-id would make the log an
   unordered set and dissolve the tip entirely (Shapiro et al., "Conflict-free Replicated Data
   Types", INRIA 2011 — cited by name, not fetched). Honest assessment: CRDTs solve
   **multi-replica convergence**, which is not this problem (one host, one store). Worse, they
   are only correct here if the fold is order-independent — true for `AddEdge` weight-sums
   (commutative), false once `RemoveEdge` interleaves with `AddEdge`. Same for Lamport/vector
   clocks: they buy causal ordering *across nodes* at the price of redefining `fold` over a
   partial order. Both are strictly more machinery than the problem needs.

### Smallest correct fix for THIS system

A plain serialization point — **yes, it is sufficient, and simpler than all of the above** —
with one nuance the brief's "plain mutex" phrasing hides: the four producers span **processes**
(the git post-commit hook is not a thread of a daemon), so an in-process `Mutex<EventLog>` alone
does not cover the file-backed `.rci/` store. Two correct minimal shapes:

- **(a) flock-per-append (recommended v1):** every producer invokes the same `rci ingest` binary,
  which takes an exclusive `flock` on `chain.jsonl`, re-reads the tip *inside* the critical
  section, appends, releases. The lock IS the serialization; no CAS retry loop is needed because
  no interleaving is possible. At µs-scale appends and ~1 event/s peak (§2 of the proposal),
  contention is nil. This is the mutex answer, made cross-process.
- **(b) single daemon owns the chain** (Thompson's funnel), other producers hand events over a
  spool dir / socket. Correct, slightly better under sustained load, but adds daemon lifecycle —
  defer until (a) measurably contends (YAGNI).

An expected-tip assertion inside the lock (EventStoreDB's `expectedVersion`, git's
`--force-with-lease`) is a cheap tripwire worth keeping as a debug assert, but under a held
exclusive lock it cannot fire; it is defense-in-depth, not the mechanism.

**Why the original proposal didn't just do this:** it reused `EventLog` "as-is" and inherited its
documented ceiling verbatim — "`MemEventStore` is non-durable and not shared across processes —
single-node local-first only" (`kernel/src/event_log.rs:18-19`) — while mistaking
**content-address idempotency for concurrency control**. Content-ids dedup *identical* replays
(`event_log.rs:302-306`); two *different* events racing on the same tip are not duplicates, so
the dedup never engages and last-writer-wins forks the chain (`event_log.rs:293-311`). The
proposal's "idempotency for free" claim was true and irrelevant to H1.

**Recommendation:** flock-serialized append (option a) for all four producers; keep `EventLog`
unchanged; document the lock as the chain's single-writer invariant. Reject CAS-retry, partitions,
CRDTs, and vector clocks as over-machinery for a single-host tool.

---

## H2 — Rollback-safety digest invariant is false under concurrency

### What the literature actually promises

The breaker's proof matches the founding literature exactly — compensation was **never** state
restoration:

- **Sagas** (Garcia-Molina & Salem, SIGMOD 1987): compensation is *semantic undo*, not physical
  restoration; when a compensating transaction executes, "other transactions may have observed
  and acted upon the effects of the transaction being compensated" — the database does **not**
  return to its original state, only the compensated subtransaction's direct effects are
  reversed. ([sagas.pdf, Cornell mirror](https://www.cs.cornell.edu/andru/cs711/2002fa/reading/sagas.pdf))
- **Azure Compensating Transaction pattern**: "You might think that you can simply restore the
  system to its original state, but this approach can overwrite changes from other concurrent
  application instances. Instead, the compensating transaction must intelligently account for
  concurrent work." And: "A compensating transaction doesn't necessarily return the system data
  to its state at the start of the original operation." Steps must be **idempotent commands**
  because compensation itself can fail and be retried.
  ([learn.microsoft.com — Compensating Transaction pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction))
- **Fowler, Retroactive Event**: reaching a branch point is done by *reversal* (requires all
  events reversible) or *rebuild* (snapshot + replay forward, skipping rejected events); naive
  reversal is wrong when subsequent events made decisions based on the reverted one.
  ([martinfowler.com/eaaDev/RetroactiveEvent.html](https://martinfowler.com/eaaDev/RetroactiveEvent.html))

So the standard formulation the design should adopt: **a compensator undoes the edge-diff of the
reverted range; it does not pin the projection to the target state against future/concurrent
deltas.** The proposal invented a stronger invariant than the literature claims, and the breaker
correctly falsified it.

### Smallest correct fix for THIS system

H1's serialization dissolves most of H2. Concretely:

1. **Run `revert(to)` inside the same exclusive lock as appends** (quiescent window by
   construction, not by hope). Under the lock: compute `diff(fold(0..head), fold(0..target))`,
   append the one compensating delta, and assert `digest(fold(0..new_head)) ==
   digest(fold(0..target))` **inside the critical section**. The digest-equality check is
   retained — but re-scoped from a global invariant to a *precondition-protected post-condition
   of the atomic revert operation*. That is exactly the quiescent-window constraint the brief
   asked about, made explicit instead of assumed.
2. **Idempotency by recomputation, not by content-id matching.** The proposal's
   `content-id = f(head, target)` retry story breaks on head-advance (breaker is right). The
   robust idiom — matching Azure's "design each step as an idempotent command" — is: a retried
   rollback re-enters the lock and **recomputes the diff**; if the first compensator already
   landed, the recomputed diff is empty ⇒ append nothing ⇒ exactly-once effect without any
   content-id cleverness. (If a stable content-id is still wanted, `append_raw`
   (`event_log.rs:321-329`) already exists for tip-independent ids — but the empty-diff no-op
   makes it unnecessary.)
3. **Rename the claim in the ADR:** "rollback = compensating delta whose application under the
   append lock restores the target projection at that instant; subsequent deltas evolve it
   further" — the Saga-standard semantic-undo guarantee, not permanent state-pinning.

**Recommendation:** keep compensating deltas (right pattern, wrong invariant); serialize revert
under the H1 lock; make retry idempotent by diff-recomputation; demote digest-equality to an
under-lock post-condition and fix the DoD text accordingly. Reject snapshot-plus-replay-with-
exclusion (Fowler's rebuild) for v1 — it mutates the meaning of history and is only needed when
you cannot serialize, which we can.

---

## H3 — Wide commits blow the co-change cap 20×

### Established practice

This is a solved, literally-parameterized problem in the change-coupling literature:

- **ROSE** (Zimmermann, Weißgerber, Diehl, Zeller — "Mining Version Histories to Guide Software
  Changes", ICSE 2004): "In order to detect coupling within transactions, one must take into
  account all branches, **but avoid the large merge transactions. ROSE does so by ignoring all
  changes that affect more than 30 entities.**" Signal strength is then measured by
  *support* (number of transactions a rule derives from) and *confidence*
  (`frq(T, x1∪x2)/frq(T, x1)`), not raw co-occurrence.
  ([zimmermann-icse-2004.pdf](https://thomas-zimmermann.com/publications/files/zimmermann-icse-2004.pdf))
- **code-maat** (Adam Tornhill's mining tool behind *Your Code as a Crime Scene*, Pragmatic
  Bookshelf 2015): ships `--max-changeset-size MAX-CHANGESET-SIZE`, **default 30** — "Maximum
  number of modules in a change set if it shall be included in a coupling analysis"; large
  commits represent bulk maintenance/refactoring, not meaningful logical coupling.
  ([github.com/adamtornhill/code-maat](https://github.com/adamtornhill/code-maat))
- CodeScene's hosted docs were 403-blocked this session (not fetched); code-maat is the same
  author's open implementation and carries the identical filter, so it stands as the primary
  citation.

So: **yes, hard exclusion above a file-count threshold is the established practice**, and the
industry-default threshold is **30 files**, independently in the canonical academic tool (ROSE)
and the canonical industrial one (code-maat).

### Smallest correct fix for THIS system

Exclude commits touching more than `max_changeset_size = 30` files from co-change edge
derivation (configurable; default mirrors the literature). Effect on the breaker's measured
numbers: the 896-file commit (400,960 potential edges), the 507-file commit, and every top-20
wide commit contribute **zero** edges; the ≤20k cap and the PPR saturation problem both dissolve
without any pruning-that-changes-semantics. The operator's wide native-port/format sweeps are
exactly the "bulk maintenance" class the literature excludes on purpose — losing them is the
point, not a regression; coupling signal comes from focused commits. A softer `1/C(F,2)`
down-weighting exists in some variants but is **not** the established default and adds a knob;
YAGNI. If ranked-rule quality later matters, adopt Zimmermann's support/confidence weighting —
also deferred.

**Recommendation:** hard exclusion at >30 files per commit in the co-change extractor; cite
ROSE + code-maat in the ADR; delete the bespoke "≤20k design cap" as the load-bearing control
(it becomes a monitoring assertion, not the mechanism).

---

## H4 — Import graph blind to money/auth/RLS runtime-contract coupling; blind graph can earn gating power

### What the literature says about coupling invisible to import graphs

1. **Historical co-change as complement:** ROSE's stated purpose includes detecting "coupling
   between items that **cannot be detected by program analysis** — including coupling between
   items that are not even programs" (ICSE 2004, above). So co-change is the standard complement
   to structural graphs — but H3 shows it is noise-prone, and this repo's wide-commit style
   thins it further after the H3 filter. It mitigates, never guarantees.
2. **Sound dependency graphs + safety nets, never scraped ones:** Google's TAP ("Taming
   Google-Scale Continuous Testing", Memon et al., ICSE-SEIP 2017) gates on a *reverse dependency*
   graph derived from **declared BUILD dependencies enforced by the build system** (missing dep =
   build failure — the graph is sound by construction, unlike regex import scraping), and even
   then treats selection as an approximation with safety nets: milestones periodically run *all*
   affected targets, and the paper explicitly abandoned fine-grained selection heuristics as
   unreliable at their scale.
   ([TAP paper PDF](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/45861.pdf))
   The classical safe-RTS principle (Rothermel & Harrold, "Analyzing Regression Test Selection
   Techniques", IEEE TSE 1996 — cited by name, not fetched) is the same stance: a *safe*
   technique must select every test that may behave differently, and where safety cannot be
   established, the fallback is retest-all.
3. **Criticality classification dominates any heuristic:** DO-178C assigns verification rigor by
   **consequence of failure** (Level A: 71 objectives, 30 with independence … Level E: 0);
   "the software level establishes the rigor necessary to demonstrate compliance." No measured
   heuristic, cost argument, or analysis result can downgrade a Level-A component's verification.
   ([en.wikipedia.org/wiki/DO-178C](https://en.wikipedia.org/wiki/DO-178C))
   That is precisely the pattern the brief asks about: *critical path always fully verified,
   regardless of any cheaper predictor* — and yes, it is standard practice in safety-critical
   systems, as a **structural classification evaluated before any analysis**, never as
   "trust the analysis unless X".
4. Fancier detectors — dynamic/runtime call-graph tracing, mutation testing to surface hidden
   coupling — exist but are heavyweight (new deps, long runtimes), still sampling-based (blind
   spots remain), and unnecessary once (3) is in place. Explicit invariant-group tagging (a file
   declares membership in an invariant group) is a reasonable idea — and the repo **already has
   the coarse version of it**.

### Smallest correct fix for THIS system

The repo already ships the DO-178C-shaped control: `is_redline()` in
`/root/dowiz/tools/ci-truth/src/main.rs:237-242` matches `money.rs` / `order_machine.rs` /
`event_log.rs` / `auth|otp|jwt` (case-insensitive), and the v5c-reexec gate
(`main.rs:383-391`) already forces full kernel+engine `cargo test` re-execution on any red-line
touch, with an explicit SKIP verdict otherwise. The correct fix is exactly the brief's candidate:

- **Structural override, evaluated before the graph:** if any changed path matches the red-line
  matcher, RCI's blast-radius prediction is **bypassed entirely** for gating purposes and full
  verification is hard-forced (the existing v5c-reexec behavior — reuse it, do not re-implement
  the matcher). The predictor may still annotate red-line changes advisorily, but its output
  carries zero gating power there, permanently.
- **`RCI_BLOCKING` is structurally scoped out of red-line paths:** promotion-to-gating via
  measured precision@k applies only to the non-red-line population. This closes the breaker's
  survivorship-bias hole exactly: precision measured on ordinary imports (the population the
  graph can see) can never license gating over the population it is structurally blind to —
  the same way DO-178C's level assignment precedes and constrains all analysis, and the same
  way TAP only lets a *sound-by-construction* graph gate, with periodic run-everything nets.
- The E1 validation case (field_frame.rs ↔ csr.rs sign convention, no import edge) should be
  re-labeled in the DoD as a **known-blind-spot regression witness** (the predictor is expected
  to miss it; the red-line/full-verify layer is expected to catch its consequences), not as a
  blast-radius success criterion.

**Recommendation:** adopt the structural override on the existing `is_redline` matcher; reject
mutation testing, dynamic tracing, and new tagging schemes for v1 (the matcher IS the v1 tagging
scheme); keep co-change (post-H3-filter) as an advisory complement, citing ROSE for why it sees
what imports cannot.

---

## Source list

Fetched this session: [Sagas (Garcia-Molina & Salem 1987)](https://www.cs.cornell.edu/andru/cs711/2002fa/reading/sagas.pdf) ·
[Azure Compensating Transaction pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction) ·
[Fowler, Retroactive Event](https://martinfowler.com/eaaDev/RetroactiveEvent.html) ·
[Thompson, Single Writer Principle](https://mechanical-sympathy.blogspot.com/2011/09/single-writer-principle.html) ·
[git push --force-with-lease](https://git-scm.com/docs/git-push) ·
[Dudycz, optimistic concurrency in event stores](https://event-driven.io/en/optimistic_concurrency_for_pessimistic_times/) ·
[Zimmermann et al., ICSE 2004 (ROSE)](https://thomas-zimmermann.com/publications/files/zimmermann-icse-2004.pdf) ·
[code-maat README](https://github.com/adamtornhill/code-maat) ·
[Memon et al., Taming Google-Scale Continuous Testing (ICSE-SEIP 2017)](https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/45861.pdf) ·
[DO-178C (Wikipedia)](https://en.wikipedia.org/wiki/DO-178C).
Cited by name only (not fetched — paywalled/blocked): Rothermel & Harrold, TSE 1996 (safe RTS);
Shapiro et al., INRIA 2011 (CRDTs); Tornhill, *Your Code as a Crime Scene* (2015); CodeScene
hosted docs (403); Kafka design docs (nav page only).
