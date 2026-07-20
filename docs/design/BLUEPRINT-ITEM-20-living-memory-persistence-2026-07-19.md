# BLUEPRINT — Item 20: P95 living-memory index persistence (externally ungated, READY)

- **Date:** 2026-07-19 · **Tier:** parallel lane / §F living-memory (roadmap §F) · **Status:**
  BLUEPRINT (planning artifact, no code) — the space-grade **execution binding** of the existing,
  detailed `BLUEPRINT-P95-LIVING-MEMORY-INDEX-PERSISTENCE-2026-07-19.md`; does NOT duplicate it.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §F
  (living-memory lane, lines 407–410), §G proof-line context; `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md`
  §15 (lines 265–276, the one confirmed-open gap = persistence), §9 addendum item 20 (line 293);
  **`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P95-LIVING-MEMORY-INDEX-PERSISTENCE-2026-07-19.md`
  (read in full — the authoritative design)**; live source `kernel/src/retrieval/` (recall.rs, bm25.rs,
  memory_store.rs), `kernel/src/fdr/` (the tier-(b) ring).
- **Upstream:** item 19 (retrieval spectral-routing audit — AUDITED, independent-by-design confirmed).
- **Relationship to item 28:** item 28 (optical compression) is sequenced **after** item 20 "since it
  consumes the same durability machinery" (roadmap §F); item 20 decides *durability*, item 28 decides
  *representation for the archival tiers only*.

---

## 1. Scope / goal (one paragraph)

Sequence the living-memory index persistence layer — "the one confirmed-open gap" of the whole
living-memory subsystem (synthesis §15(c)) — per its existing, detailed P95 blueprint, evaluating
whether it shares durability machinery with the Tier-1 FDR tier-(b) `mmap`-backed ring (synthesis §5
synergy). The persistence work itself is fully specified in the P95 blueprint (persist the built
`Bm25`/`TrigramIndex`; incrementally add one document's terms rather than re-tokenizing the whole
corpus; deterministic std-only codec, no serde). This document's job is the **space-grade binding**:
(a) confirm P95's own NO-GO precondition gate against live HEAD (P95 is `HOLD` — "ready design, do NOT
build yet" — unless a real repeated-write/-query caller exists); (b) pin the roadmap's falsifiable
proof (kill -9 the process, restart, a fixed-corpus retrieval query returns byte-identical results);
(c) rule on the FDR-ring-sharing question; and (d) either resolve P95's HOLD or record its hold reason
as a stated forcing reason (synthesis §9 item 20 proof: "P95's HOLD status resolves, or its hold
reason is recorded as a stated forcing reason").

---

## 2. Verified current state — grounded (re-confirmed against P95's own live cites)

- **The rebuild-every-call hazard is real and precisely located.** `recall.rs:266–309`
  `PrimaryRecall::from_dir` walks the corpus and constructs a **fresh** `Bm25::new(docs)` (`:297`) +
  `TrigramIndex::new(&strs)` (`:298`) on **every call** — no `OnceLock`, no cache, no dirty-check
  (P95 §0.1). The 12-doc fixture path *is* guarded (`recall.rs:313–317` `OnceLock`), so the hazard is
  only on the 202-file real-corpus path.
- **The BM25 state to persist/incrementally-update is defined.** `bm25.rs:117–127` — `docs`, `tf`,
  `avgdl`, `df`, `params`; built by `with_params` (`bm25.rs:136–163`) which **computes and discards
  `total_len`** (P95 §0.2 — exact incremental `avgdl` maintenance needs it retained).
- **The determinism contract any change must preserve is coded.** `bm25.rs:197–241` — `score_doc`
  sorts+dedups query terms (order-independent reduction), `rank` tie-breaks ascending doc-id. Any
  persistence/incremental change must be **byte-identical** (P95 §0.3, §7).
- **P95's own NO-GO precondition is UNMET today.** The only "wired" caller (`gov_recall`,
  `governance.sh:237–243`) is **dead two ways** (undefined `$GOV_LM`; pipes the native Rust binary
  through `python3`) with **zero callers** (P95 §0.4). `from_dir`'s only caller is the one-shot CLI
  `bin/lm.rs:76`. **Zero rebuilds per day on any live path.** So P95-C1 (a real repeated-write OR
  repeated-query caller) is **unmet** at HEAD — re-verify at execution time before building.
- **The FDR tier-(b) `mmap`-backed durable ring now exists** (Tier-1 DONE — `kernel/src/fdr/`,
  `FdrRing` with A/B segments, `mod.rs:344`). Synthesis §15(c): "an index that must survive restart is
  exactly the tier-(b) `mmap`-backed durability class the FDR already proposes." This is the §3 ruling
  below — the durability *mechanism* question the P95 blueprint left open ("whether they share
  machinery is a design question for P95's implementation").
- **The persistence sinks P95 targets:** `MemoryStore` trait + `snapshot_root` (`memory_store.rs:24–37`,
  `:31–36`) is **built** (std-only default); the pgrust `PgStore` (`memory_store.rs:129–167`) is a
  **feature-gated stub, zero live consumers** (P95 §0.5). Option A (std-only on-disk) needs neither.

---

## 3. The space-grade ruling — FDR-ring sharing (the one design question P95 left open)

P95 §3.1 offers two serialization targets: **Option A** (std-only deterministic on-disk file,
recommended first) and **Option B** (`MemoryStore`/pgrust). Synthesis §15(c) raises a third
consideration: sharing the FDR tier-(b) ring. **Ruling:** the index persistence and the FDR ring are
**different durability shapes and must NOT share one buffer** — a P2 Correspondence call, decided
against sharing:

- The FDR ring is a **fixed-capacity forensic ring** (last-N-events, overwrite-oldest, segment-fsync-
  amortized) — its whole point is bounded-size continuous capture. The BM25 index is a **whole-object
  snapshot** (persist-once-reload) with an **incremental delta** (add-one-doc). These are the
  ring-vs-snapshot distinction; forcing the index into the ring would corrupt the ring's
  bounded-capacity contract (a 202-doc index does not fit an 8192-entry forensic ring).
- **What they legitimately share is the *discipline*, not the buffer:** the deterministic fixed-order
  encoding (P95 §3.3 mirrors `snapshot_root`'s sorted-key discipline — the same one `fdr/json.rs`
  uses), and the `mmap`-backed std-only durability *class* (both survive process death via the file
  system, no dependency). P95 Option A is the correct first step; it is the tier-(b) durability class
  applied to a snapshot, sharing FDR's *technique* (deterministic encode, mmap-or-file durability) but
  not its *ring*. This resolves synthesis §15(c)'s open "whether they share machinery" as: **share the
  encoding discipline and the durability class; do not share the buffer.**

Everything else about *how* to persist/incrementally-update is P95 §3.1–§3.5 verbatim — this document
does not re-derive it.

---

## 4. Implementation plan — defer to P95, bind the sequencing

The exact edits are P95 §8.1 / §9 (`bm25.rs` `add_document` + retained `total_len`; `index.rs`
`TrigramIndex::add_document`; `recall.rs` `save`/`load`/`save_to`/`load_from` + dirty fingerprint;
`memory_store.rs` reused unchanged). This blueprint adds three space-grade bindings:

1. **Gate on P95-C1 first (NO-GO if unmet).** Before any code: re-confirm at execution time that a
   real repeated-write OR repeated-query caller exists and is wired (the dead `gov_recall` fixed, or a
   self-improvement-loop / agent memory-write path landed). If unmet → **STOP, report HELD, NO-GO**
   (P95 §6, §9 step 1). This is the roadmap's "externally ungated" claim made precise: item 20 is
   *ungated by other roadmap items* but *gated by the existence of a real caller* — the two are not
   the same, and conflating them would build ahead of need (the exact anti-pattern P95 and ponytail
   forbid).
2. **Option A (std-only) before Option B (pgrust).** P95 §3.1 ordering; Option B gated additionally on
   P95-C3 (`memory_store` gaining a real consumer). No new dependency, no DECART event.
3. **Register the persistence codec as a hot path.** The deterministic encoder (P95 §3.3) is
   hand-rolled serialization — an algorithmic path — so it gets a `HOT-PATHS.tsv` row (§5) when the
   code lands.

---

## 5. Tests / proofs — 5-point hardening applicability

The change is a **correctness-preserving optimization** (P95 §7); the 5-point standard maps:

- **Item 1 (oracle):** P95 §7 is the oracle plan — property-based (proptest), **the full rebuild is
  the reference oracle**: P1 append-byte-identity (`prop_incremental_eq_rebuild`), P2 rank-equivalence,
  P3 persistence round-trip (`decode(encode(idx))` byte-identical), P4 tombstone rank-equivalence
  (weaker tier, honestly stated), P5 SIMD stream-identity (only if §4 SIMD is ever built), P-REG
  no-determinism-regression. The full-rebuild-as-oracle is retained forever as the differential target
  — exactly checklist item 1's strong form.
- **Item 3 (debug-differential):** `debug_assert_eq!` the incremental index against a full rebuild in
  debug builds (the per-call reference *exists* here — `Bm25::new` — so the strong form applies, not
  `N/A(corpus-oracle)`).
- **Item 5 (formal):** **N/A / not-warranted** — the properties are byte-equalities better served by
  the proptest oracle than a bounded model checker; the codec has no non-enumerable arithmetic-edge
  space Kani would target. Record `N/A(covered-by-oracle)`.
- **Item 2 (dudect):** **N/A** — no secret-dependent timing in a retrieval index. Record
  `N/A(no-secret-compare)`.
- **Item 4 (asm):** **N/A** — no branch-free constant-time path.

---

## 6. Acceptance criteria (falsifiable) — the roadmap's own proof

Synthesis §9 item 20 / roadmap §F:
1. **Kill -9 the process, restart, and a fixed-corpus retrieval query returns results identical to
   pre-restart** — the load path reconstructs a byte-identical index (or a dirty-tracked incremental
   reload that produces byte-identical `rank`/`top_k` output). This is the primary falsifiable proof.
2. **Every P95 §7 property GREEN** as a proptest (P1 append-byte-identity is the primary guarantee;
   P4 documents the one place byte-identity honestly does not hold — tombstone delete).
3. **`lm --selftest` and the recall@5=1.0 fixture stay GREEN** throughout (P95 §7 P-REG).
4. **P95's HOLD status is resolved OR its hold reason recorded as a stated forcing reason** — if
   P95-C1 is still unmet at execution time, the deliverable is the recorded NO-GO with the forcing
   reason ("no repeated-write/-query caller exists"), which *satisfies* the roadmap proof by its "or"
   clause.
5. **Zero new dependency** (Option A std-only); `cargo tree` unchanged; the FDR-ring-sharing ruling
   (§3) recorded.

---

## 7. Dependency gates

- **Externally ungated by roadmap items** (roadmap §F: "genuinely open, externally ungated, READY
  now") — item 19 (its predecessor audit) is DONE.
- **Gated by a real caller (P95-C1)** — the internal precondition; NO-GO if unmet. This is the honest
  reading of "READY": ready to *design and sequence*, buildable *when a caller lands*.
- **Blocks item 28** (optical compression consumes the same durability machinery — roadmap §F
  sequences 28 after 20).

---

## 8. Open questions (operator ruling)

1. **Is there now a real caller (P95-C1)?** This is a *factual* question the executor resolves by
   grep at execution time, not an operator ruling — but if the answer is "no" and the operator
   nonetheless wants the persistence layer built ahead of a caller (a forward-looking-infrastructure
   call, like the `thunderdome`/`slot-arena` precedent, synthesis §18(a)), **that override is an
   operator decision**, since it reverses P95's own NO-GO and ponytail's build-only-what-has-a-caller
   rule. Flagged: default is HELD/NO-GO absent a caller; only an explicit operator "build it ahead"
   overrides.
