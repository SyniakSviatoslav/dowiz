# BLUEPRINT P95 — Living-memory index persistence + incremental BM25 update (2026-07-19)

> **Standalone HELD blueprint (dowiz `kernel/src/retrieval`).** One coherent, independently
> buildable unit. Research source: `docs/research/OPUS-TOKENIZATION-LIVINGMEMORY-RECHECK-2026-07-19.md`
> §4 (the one forward-looking latent hazard the tokenization recheck surfaced). Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree: `/root/dowiz/kernel` at HEAD,
> read live this pass.
>
> **One sentence:** the moment living-memory retrieval gets a *real* repeated-write or repeated-query
> caller, `PrimaryRecall::from_dir` — which re-tokenizes and rebuilds the entire corpus from scratch on
> **every call** with no cache and no incremental path (`recall.rs:266-309`) — flips from harmless to a
> per-write / per-query O(corpus) tax; this blueprint is the **ready-when-needed** architectural fix
> (persist the index + incrementally add one document's terms rather than re-tokenizing all of them),
> held until such a caller actually exists.

---

## VERDICT (stated up front, per session research discipline)

**HOLD — ready design, do NOT build yet. No urgency exists today; this is a latent-hazard flag, not a
live bug.** The research recheck is unambiguous on tense (research §4): *both* the earlier "not worth
optimizing" conclusion and the operator's "this is the living-memory engine" push-back are true, at
different tenses, and they do not contradict.

- **Today (do nothing):** there is **no live path** that re-runs hundreds-scale corpus tokenization
  repeatedly. The self-improvement-loop recall is a **12-doc fixture behind a `OnceLock`** (built once
  per process, ~free — `recall.rs:210-221`, `:313-317`). The real 202-file corpus path exists only in a
  **one-shot CLI** (`bin/lm.rs:76-85`) whose sole "wired" caller (`gov_recall`, `governance.sh:237-243`)
  is **dead two ways** (§1.3). Query-time tokenization is just the short query string (`recall.rs:147`,
  `:232`). **Batch/SIMD corpus tokenization is not a present-tense win, and neither is this blueprint.**

- **When a real caller lands (build this):** `from_dir` re-tokenizes and rebuilds the **entire** corpus
  on every call (`recall.rs:266-309` → `Bm25::new(docs)` + `TrigramIndex::new(&strs)` at `:297-298`).
  There is **no incremental update, no persisted index, no dirty-tracking**. So a real integration pays
  the full 202+-doc rebuild in **two** concrete ways (research §4): a **repeated-write** living-memory
  (append note → naive "rebuild on write" = full re-tokenize per write) and a **repeated-query** CLI
  process model (rebuild per invocation → every query pays a full corpus rebuild).

**Two hard preconditions gate any build (NO-GO if neither holds, §6):** (P95-C1) a real repeated-write
*or* repeated-query caller for living-memory must exist and be wired (the dead `gov_recall` fixed, or a
self-improvement-loop / agent memory-write path landed); and (P95-C2) the change must sit behind a
persistent process or a stable ingest order — the persistence axis is meaningless for a pure one-shot
CLI whose whole lifetime is one query. Absent P95-C1, the per-rebuild cost is paid **zero** times per
day; building ahead of it is exactly the "build ahead of real need" the research warns against.

The ordering is non-negotiable and matches the research's own cheapest-proven-first list (research §4):
**(1) persist + don't rebuild → (2) incremental add-on-write → (3) only then SIMD/batch tokenize.**
Steps 1–2 are the architectural fix and remove the hazard entirely; step 3 is a rounding error next to
them and is explicitly secondary (§4).

---

## 0. Ground truth — every cite re-verified live this pass

> Every claim below was read from source **this pass** (`/root/dowiz/kernel`, HEAD), not inherited from
> the research sketch or memory recall.

### 0.1 The rebuild-every-call hazard (the avoidable work)

`PrimaryRecall::from_dir` (`recall.rs:266-309`) walks a directory, reads every `*.md`, and constructs a
**fresh** index **on every call** — it is behind **no `OnceLock`, no cache, no dirty-check**:

| Step | Cite | Cost per call |
|---|---|---|
| `read_dir` + collect `*.md` paths | `recall.rs:273-288` | I/O, O(files) |
| `paths.sort()` (stable doc order) | `recall.rs:290` | O(files log files) |
| `read_to_string` every file | `recall.rs:291-294` | I/O, O(corpus bytes) |
| `Document::from_text` every file (**tokenize**) | `recall.rs:295` | **O(corpus tokens)** — the batch |
| `Bm25::new(docs)` (build `tf`/`df`/`avgdl`) | `recall.rs:297` | **O(corpus tokens)** — full reindex |
| `TrigramIndex::new(&strs)` | `recall.rs:298` | **O(corpus bytes)** — full trigram build |

The 12-doc fixture path (`PrimaryRecall::new`, `recall.rs:210-221`) is wrapped in
`static PRIMARY: OnceLock` (`recall.rs:313-317`) and served by the free `recall_at_k` (`recall.rs:324-326`)
— **built once, ~110 tokens, genuinely free.** `from_dir` is the *only* path over the real 202-file
corpus, and it has **no such guard**.

### 0.2 The BM25 state that must be persisted / incrementally updated (`bm25.rs:117-127`)

```rust
pub struct Bm25 {
    docs:  Vec<Document>,                    // doc-id = position in this Vec
    tf:    Vec<HashMap<String, u32>>,        // per-doc term-frequency map
    avgdl: f64,                              // mean document length (tokens)
    df:    HashMap<String, u32>,             // term -> document frequency n_t
    params: Bm25Params,
}
```

Built by `Bm25::with_params` (`bm25.rs:136-163`): one pass over `docs` incrementing a per-doc `tf` map
and the global `df`, accumulating `total_len`, then `avgdl = total_len / docs.len()` (`:151-155`).
**Note `total_len` is computed and discarded** — only `avgdl` is retained; exact incremental `avgdl`
maintenance needs it kept (§3.2).

### 0.3 The determinism contract any change MUST preserve (`bm25.rs:197-241`)

- `score_doc` sorts + dedups query terms before summation so the reduction order is HashMap-iteration
  independent (`bm25.rs:199-201`).
- `rank` sorts descending score, **tie-broken by ascending doc-id** (`bm25.rs:233-239`).
- `tokenize` (`bm25.rs:86-100`) is a scalar `char`-by-`char` `is_ascii_alphanumeric` scan, lowercasing,
  splitting on non-alnum runs, one `String` allocation per token.

Same input ⇒ same bytes. **Any persistence/incremental/SIMD change is a correctness-preserving
optimization: it must produce a byte-identical index and byte-identical `rank` output** (§7 proves it).

### 0.4 The only "wired" caller is DEAD — this is why it is not urgent

`tools/telemetry/governance.sh` `gov_recall` (`governance.sh:237-243`) is broken two ways: it gates on
`[ -f "$GOV_LM" ]` but **only `$GOV_LM_BIN` is ever defined** (`governance.sh` var block) →
`$GOV_LM` is empty → the test is always false → it **always** falls through to `gov_precedent`; and even
if reached it calls `python3 "$GOV_LM"` — running the native Rust `lm` binary through the Python
interpreter (a second bug). Moreover `gov_recall` has **zero callers** anywhere in `tools/`, `scripts/`,
`.claude/`. **Nothing drives the real-corpus path in a loop today** — not on a schedule, not per-write,
not per-query.

### 0.5 The persistence sinks that exist (and their honest state)

| Sink | Cite | State |
|---|---|---|
| `MemoryStore` trait — `put/get/keys/snapshot_root` content-addressed KV | `memory_store.rs:24-37` | **built**, std-only default `InMemoryStore` (`:45-76`) |
| `PgStore` pgrust SQL adapter | `memory_store.rs:129-167` | **feature-gated stub, OFF by default, zero live consumers** (research §2) |
| `ppr.rs` / `diffusion.rs` (L3 spectral relatedness) | `mod.rs:20-30` | **standalone, self-tested, zero external callers** — NOT wired to recall (research §3) |

`memory_store.rs` is declared (`mod.rs:26-28`) but has **zero live consumers**. The pgrust path is a
plan, not code. The spectral/PageRank "local node memories" connection is **aspirational**, not built
(research §3). None of this is a dependency this blueprint may assume as real.

---

## 1. Problem statement — precisely what breaks, and exactly when

### 1.1 What is fine today (so we do not manufacture urgency)

The self-improvement-loop recall path is a 12-doc fixture behind a `OnceLock` — one build per process,
~110 tokens, off every hot path. Query-time work is the short query string only. The 202-file corpus is
touched **only** by a one-shot CLI whose only wired caller is dead (§0.4). **Zero rebuilds per day on any
live path.** The earlier "not worth optimizing" scan is correct *for the code as it runs today.*

### 1.2 What breaks, and the trigger that breaks it

`from_dir` has no persisted index and no incremental path (§0.1). The instant a **real** caller wires
living-memory retrieval, the full-corpus rebuild becomes a repeated cost in two disjoint ways:

1. **Repeated-write (living memory is write-heavy by definition).** The living-memory design is
   append-notes / reinforce / demote-tiers (`internal-retrieval-living-memory-blueprint.md` §5-6). A
   naive "reindex on write" calls `from_dir` (or `Bm25::new(all_docs)`) per new/edited note → an
   **O(corpus) re-tokenize + full reindex for a one-document change.** At 202 docs today and growing,
   every note write re-tokenizes all 202.
2. **Repeated-query via the CLI process model.** `lm` rebuilds the index per invocation
   (`bin/lm.rs:76`). If anything ever drives `lm` per query in a loop (the `gov_recall` *intent*,
   currently dead), **every query pays a full corpus rebuild** to answer one question — the dominant
   waste is re-tokenizing 202 docs to serve a handful of query tokens.

### 1.3 Classification — latent hazard, not a live defect

This is a **"flag for when the living-memory blueprint (M0-M8) or a real `from_dir` wiring is actually
built"** finding, **not** a present-tense optimization. Nothing waits on index-build cost today. The
value of recording it now is that it is not rediscovered the hard way once a caller lands, and that the
fix is captured while the code is fresh: **primarily architectural (persist / incrementally update),
only secondarily SIMD.**

---

## 2. Prior-art map — adopt, don't invent

| Prior art | What it is | How P95 uses it — and what it does NOT take |
|---|---|---|
| **Lucene / Tantivy segment + incremental indexing** | new docs go to a fresh in-memory segment; postings merged lazily; index persisted between processes | **Adopt the shape** — persist the built index; add one doc's postings without re-tokenizing the corpus. **NOT taken:** multi-segment merge policies, deletes-by-generation, mmap'd codecs — over-engineering for a low-hundreds-doc flat corpus (ponytail); a single incrementally-maintained `Bm25` + a stable id map is sufficient. |
| **Incremental / online BM25 (streaming `df`/`avgdl` maintenance)** | maintain `df`, doc-length sum, and `avgdl` as running aggregates so adding a doc is O(doc), not O(corpus) | **Adopt verbatim** (§3.2). This is the exact operation that makes "reindex on write" cheap. |
| **Content-addressed / dirty-tracked build cache** | fingerprint inputs; skip rebuild when the fingerprint is unchanged; reconcile only the delta | **Adopt** (§3.1) — the existing `MemoryStore::snapshot_root` (`memory_store.rs:31-36`) is already a reproducible content root; reuse it as the dirty-check, don't invent a new one. |
| **SIMD / SWAR byte-classification tokenizers (`memchr`-style)** | classify alnum runs on whole byte spans, emit `&str` slices into an arena instead of per-token `String` | **Sketch only, explicitly deferred** (§4). Worth it for a large cold ingest; a rounding error next to persistence for the steady state. Must emit the **byte-identical** token stream the scalar tokenizer does. |
| **serde / bincode** | derive-based (de)serialization | **NOT taken** — the kernel is pure-`std`, no serde (`memory_store.rs:3` red line). Persistence uses a hand-rolled **deterministic** encoder (fixed field + sorted-key order), mirroring the existing `snapshot_root` discipline (§3.3). Adding serde would be a new dep → a DECART event, unjustified here. |

**P95 adds no dependency and invents no primitive** — it composes the existing `Bm25`/`TrigramIndex`
build with the existing `MemoryStore` seam and a std-only deterministic codec.

---

## 3. Primary fix — persistence + incremental update (the architectural fix)

This is *the* fix. Steps §3.1–§3.3 remove the hazard entirely; §4 (SIMD) is a separate, lesser concern.

### 3.1 Persist the built index; don't rebuild per process/query (biggest win, no SIMD)

The dominant waste in the CLI/repeated-query model is not slow tokenization — it is **re-tokenizing 202
docs to answer one query.** Build once, serialize, reload on query:

- **New API on `PrimaryRecall`** (name before impl): `save(store: &dyn MemoryStore) -> Result<(),String>`
  and `load(store: &dyn MemoryStore) -> Result<Option<PrimaryRecall>,String>`, plus a std-only
  on-disk equivalent `save_to(path)` / `load_from(path)` for the pure-`std` red line (Option A below).
- **Dirty-tracking:** persist a **corpus fingerprint** = the sorted list of `(stem, content_hash)` pairs
  (content hash via the same primitive `snapshot_root` uses). On `from_dir`-equivalent load: hash the
  live directory, compare to the stored fingerprint; **identical ⇒ load the cached index, zero rebuild;
  differ ⇒ reconcile only the delta** (add new stems, tombstone removed stems, re-add changed stems)
  via the incremental path (§3.2). This is the single change that removes the repeated-query rebuild.

**Two serialization targets, cheapest-dependency first:**

- **Option A (recommended first — std-only on-disk cache).** A single deterministic file next to the
  corpus (or in a cache dir). No new dependency; honors the kernel's pure-`std` red line
  (`memory_store.rs:3`). This is the lowest-risk first step and is sufficient for both the CLI and a
  persistent-process caller.
- **Option B (upgrade — `MemoryStore` / pgrust).** Store the serialized index under a well-known key and
  each doc's `tf` map under a per-stem key, so an incremental write re-`put`s **one** key, not the whole
  index; `snapshot_root` gives tamper-evidence + merge ordering for free. **Honest caveat:** the pgrust
  adapter is a stub with **zero live consumers** (§0.5) — Option B rides on infrastructure that is not
  yet real, so it is the *second* step, gated on `memory_store` gaining a real consumer (§6-P95-C3).

### 3.2 Incremental indexing on write — what "incremental" means for BM25 specifically

Living memory adds/edits **one note at a time.** The difference this blueprint turns on:

- **Full rebuild (today):** `Bm25::new(all_docs)` re-tokenizes and re-aggregates **every** document —
  O(corpus tokens) per write.
- **Incremental add (the fix):** tokenize **only the changed doc** and mutate the aggregates in place —
  O(changed-doc tokens) per write.

Concretely, `Bm25::add_document(doc)` (new method; illustrative spec, not code to ship now):

```
1. id = docs.len()                              // stable, monotonic — id is NOT the sort position (§3.4)
2. let m = per-doc tf map from doc.tokens        // tokenize the ONE new doc only
3. for term in m.keys():  df[term] += 1          // global df: increment, order-independent
4. total_len += doc.tokens.len()                 // running integer sum (needs the retained total_len field, §0.2)
5. docs.push(doc);  tf.push(m)
6. avgdl = total_len as f64 / docs.len() as f64  // single division — byte-identical to full-build (§0.2)
```

**Why this is byte-identical to a full rebuild over the same sequence:** `df` values are counts —
incrementing in doc order yields the same map a full build's in-order pass yields (order-independent);
each per-doc `tf` map is the same tokenization; `total_len` is an exact integer sum; `avgdl` is the same
single float division. Therefore `docs`, `tf`, `df`, `avgdl` are **all** byte-identical, hence `rank`
output is identical for every query (proven in §7-P1/P2). The `TrigramIndex` gets the analogous
`add_document` (append the new doc's trigrams to the inverted posting lists, keeping each list sorted).

### 3.3 Deterministic serialization (no serde, std-only)

The encoder emits fields in **fixed order** and maps in **sorted-key order** (df by term, docs by
doc-id, each `tf` by term) so the byte stream is reproducible run-to-run and platform-to-platform —
exactly the discipline `snapshot_root` already enforces (`memory_store.rs:31-36`). `params` and `avgdl`
serialize as fixed-width little-endian; `total_len` is persisted (so `avgdl` is reconstructable exactly
on load without float drift). Round-trip is an equality: `decode(encode(idx)) == idx` byte-for-byte
(§7-P3).

### 3.4 Edit / delete semantics — the honest hard case

Living memory's rule is **move-not-delete** (tier-demote is metadata, not corpus removal), so the hot
path is **append + tier-demote**, which §3.2 covers with strict byte-identity. Hard delete/edit is rare
but must be defined:

- **doc-id = Vec position** (`bm25.rs` comment `:105`), so **compacting** on delete would renumber every
  later doc → breaks stable ids and breaks the append-equivalence guarantee.
- **Fix: tombstone.** Mark the slot deleted (empty its `tf`, flag `docs`), **decrement `df`** for its
  distinct terms, subtract its length from `total_len`, and track a `live_count`. `rank`/`score_doc`
  skip tombstoned ids and use `live_count` (not `docs.len()`) as `n` for IDF. doc-ids stay stable; an
  edit = tombstone-old + append-new.
- **Equivalence tier for delete is honestly weaker** (§7-P4): a tombstoned index is **not** byte-identical
  to a full rebuild over only the live docs (the rebuild would renumber), but its **`rank`/`top_k` output
  — the observable — is identical after mapping doc-ids through the stem table.** State both tiers; do not
  over-claim byte-identity where it does not hold.

### 3.5 The id-order caveat (must be stated, or the equivalence claim is false)

`from_dir` sorts paths (`recall.rs:290`), so a *full rebuild* renumbers doc-ids whenever a new file sorts
into the middle. The incremental/persisted model therefore **assigns doc-ids monotonically by ingest
order** and decouples id from filename via the existing `ids: Vec<String>` stem map (`recall.rs:299-308`).
The strict byte-identity property (§7-P1) is stated over **the same ingestion sequence**, not "the sorted
directory." This is a precise, load-bearing honesty note — without it the equivalence test would be
testing the wrong invariant.

---

## 4. Secondary consideration — SIMD / batch tokenization (NOT the priority)

**Explicitly secondary.** It only matters *after* §3 makes tokenization the (now bounded) remaining work
— the initial cold build and large multi-doc ingests. It does **nothing** for the steady state, where
§3.2 already reduces per-write tokenization to one document.

- **What it would look like:** `bm25::tokenize` (`bm25.rs:86-100`) is today a scalar `char`-by-`char`
  `is_ascii_alphanumeric` scan with a per-token `String` allocation. The SIMD-friendly rewrite is a
  classic **byte-classification pass** (SWAR / `memchr`-style alnum-run detection) that emits `&str`
  slices into **one arena** instead of `Vec<String>`, over whole byte spans.
- **Hard constraint:** it must produce the **byte-identical token stream** — same lowercasing, same
  split points, same order — the scalar tokenizer produces, or it breaks the determinism contract
  (§0.3). This is a proptest target (§7-P5), not an assumption.
- **When (if ever) to do it:** only if a benchmark shows cold-build / bulk-ingest tokenization is a
  measured bottleneck *after* §3 lands. Absent that measurement, **do not build it** — it is a rounding
  error next to persistence + incremental update, and premature SIMD is exactly the kind of complexity
  the research (and ponytail) says to skip.

---

## 5. Explicit non-goal / do-not-build-yet

**This is a ready design, held for when living-memory gets a real integration. It is NOT a task to
schedule now.** Consistent with today's research discipline of not building ahead of real need:

- **The only current caller is dead code.** `gov_recall` gates on an undefined `$GOV_LM` and pipes the
  native Rust `lm` binary through `python3`, and has zero callers anywhere (§0.4). Nothing rebuilds the
  index in a loop. There is **no repeated cost to eliminate today.**
- **The L2 semantic layer (HNSW/ONNX) is explicitly out of scope** in the current design
  (`recall.rs:18-23`, research §2) — this blueprint does not touch it.
- **The spectral / personalized-PageRank "local node memories" connection is aspirational, not built.**
  `ppr.rs` / `diffusion.rs` are standalone, self-tested, and have **zero external callers**; `diffusion`
  runs over a hand-built 20-node fixture, not the live corpus (research §3). This blueprint makes **no**
  claim to wire them and **must not** be read as authorizing that work.
- **Do not build `save`/`load`/`add_document`/tombstone/SIMD until §6's preconditions hold.** Landing
  them ahead of a real caller adds surface area, a new persistence format to maintain, and test burden
  for a cost that is currently paid **zero** times per day.

---

## 6. Dependencies / preconditions — what must be true first (NO-GO gate)

| # | Precondition | Why it gates | Evidence it is unmet today |
|---|---|---|---|
| **P95-C1** | A **real repeated-write OR repeated-query caller** for living-memory exists and is wired — e.g. the self-improvement loop writing memory notes on a schedule, an agent memory-write path, or a fixed `gov_recall` that actually drives `lm` per query. | Without a caller, the full-rebuild cost is paid **zero** times/day; the fix optimizes nothing. | `gov_recall` dead + zero callers (§0.4); `from_dir`'s only caller is a one-shot CLI (`bin/lm.rs:76`). |
| **P95-C2** | The caller runs behind a **persistent process** or a **stable ingest order** (not a pure one-shot CLI whose whole lifetime is a single query). | For a one-shot process the relevant axis is cross-process *persistence* (§3.1), not in-process incremental; the incremental half only pays off across writes within a living index. | `lm` is one-shot: parse → `from_dir` → one query → exit (`bin/lm.rs:76-85`). |
| **P95-C3** | *(Option B only)* `memory_store` / pgrust gains a **real live consumer**. | Option B persists into a sink with zero consumers today; the std-only on-disk Option A does not need this. | `memory_store` declared but unconsumed (§0.5). |

**Rule:** if **P95-C1 is unmet, this blueprint is NO-GO** — report "held, no caller yet." When C1 lands,
prefer Option A (std-only, no new dep) first; escalate to Option B only when C3 also holds.

---

## 7. Test / verification plan — prove incremental ≡ full-rebuild (correctness-preserving)

This is a **correctness-preserving optimization**; the test plan's entire job is to **prove the
optimized path returns exactly what the rebuild returns.** Property-based (proptest), not example-based.

| # | Property | Falsifier |
|---|---|---|
| **P1** | **Append byte-identity.** For any random sequence `[d0..dn]`, an index built empty-then-`add_document(d_i)` in order is **byte-identical** (`docs`, `tf`, `df`, `avgdl`, `total_len`) to `Bm25::new([d0..dn])`. | `prop_incremental_eq_rebuild`: proptest random token sequences; assert full field equality. RED before `add_document` exists. |
| **P2** | **Rank equivalence (the observable).** Same corpus built both ways ⇒ for any random query, `rank`/`top_k` return **identical** `(doc_id, score)` vectors, including the ascending-doc-id tie-break (`bm25.rs:233-239`). | `prop_incremental_rank_eq`: random corpus × random query; assert vector equality. |
| **P3** | **Persistence round-trip.** `decode(encode(idx))` is byte-identical to `idx`; `rank` output identical through the serialize→deserialize boundary; a **dirty-tracked incremental load** (cache + one new doc) equals a full rebuild over cache+new (P1/P2 through persistence). | `prop_serde_roundtrip` + `prop_dirty_load_eq_rebuild`. |
| **P4** | **Delete/tombstone rank-equivalence (weaker tier, stated honestly).** After a tombstone delete, `top_k` id-lists **mapped through the stem table** match a full rebuild over the live-doc set. NOT byte-identity (ids differ by design, §3.4). | `prop_tombstone_rank_eq`: assert mapped id-list + score equality, and explicitly assert byte-identity does **not** hold (documents the tier). |
| **P5** | **SIMD tokenizer stream-identity (only if §4 is ever built).** The batch/SIMD `tokenize` emits a **byte-identical** token `Vec` to the scalar `tokenize` for any input (incl. UTF-8, punctuation runs, single-char tokens). | `prop_tokenize_simd_eq_scalar`. Gate SIMD on this being GREEN. |
| **P-REG** | **No determinism regression.** The existing recall@5=1.0 fixture and `lm --selftest` (`bin/lm.rs:91-127`) stay green; the incremental/persisted path reproduces the **same** fixture ranking, in the same sorted-term summation order (`bm25.rs:199-201`). | existing fixture tests + `lm --selftest` + a new incremental-vs-fixture cross-check. |

**Acceptance:** every P-property GREEN as a proptest with a fixed seed and a shrink-checked
counterexample budget; P1 is the primary guarantee (append is the hot path per §3.4); P4 documents the
one place byte-identity honestly does not hold. A single failing shrink means the optimization is wrong
— **fix the optimization, never the property** (test-integrity rule).

---

## 8. Scope — what P95 owns vs deliberately does NOT

### 8.1 P95 OWNS (when built)
1. `Bm25::add_document` + retained `total_len` field + `TrigramIndex::add_document` (incremental, §3.2).
2. Tombstone delete/edit + `live_count` for IDF (§3.4).
3. `PrimaryRecall::save/load` (Option A std-only on-disk; Option B `MemoryStore`) + deterministic codec
   + `(stem, content_hash)` dirty fingerprint (§3.1, §3.3).
4. The property-based equivalence suite (§7).

### 8.2 P95 does NOT own (anti-scope)
- **The L2 semantic layer (HNSW / ONNX embeddings)** — out of scope by the current design
  (`recall.rs:18-23`).
- **Wiring `ppr.rs` / `diffusion.rs` (spectral / PageRank relatedness) to recall** — aspirational,
  zero callers (research §3); a separate future blueprint, not this one.
- **Building the pgrust `PgStore` for real** — it is a stub (§0.5); Option B only *targets* the
  `MemoryStore` trait, it does not implement the SQL backend.
- **Fixing `gov_recall` / choosing the real caller** — that is P95-C1's precondition, owned by whoever
  wires living-memory for real, not by this optimization.
- **Any SIMD work before §7-P5 is GREEN and a benchmark justifies it** (§4).

---

## 9. Links to docs & memory + instructions for a future worker

**Depends on / cites:**
- `docs/research/OPUS-TOKENIZATION-LIVINGMEMORY-RECHECK-2026-07-19.md` §1 (dead caller), §2 (L2/pgrust
  not built), §3 (spectral not wired), §4 (the fix ordering: persist → incremental → SIMD).
- `docs/design/internal-retrieval-living-memory-blueprint.md` §5-6 (living-memory write model), M0-M8
  (gated roadmap this fix attaches to).
- `docs/design/BLUEPRINT-W18-living-knowledge-wiring.md` (the landed lexical-recall scope).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- Memory: `internal-retrieval-living-memory-arc-2026-07-14.md`, `pattern-ledger-2026-07-18.md`
  (build-only-what-has-a-caller discipline).

**Exact targets (dowiz kernel — DO NOT build until §6-P95-C1 holds):**
- **EDIT** `kernel/src/retrieval/bm25.rs` — add `total_len` field + `add_document` + tombstone support;
  keep `score_doc`/`rank` determinism (`:197-241`) byte-for-byte.
- **EDIT** `kernel/src/retrieval/index.rs` — `TrigramIndex::add_document` (append postings).
- **EDIT** `kernel/src/retrieval/recall.rs` — `PrimaryRecall::{save,load,save_to,load_from}` + dirty
  fingerprint; keep `from_dir` (`:266-309`) as the cold-build fallback.
- **REUSE unchanged** `kernel/src/retrieval/memory_store.rs` (`MemoryStore` trait + `snapshot_root`) —
  Option B target; **DO NOT** implement `PgStore` here.
- **DO NOT TOUCH** `ppr.rs` / `diffusion.rs` / `spectral.rs` (out of scope, §8.2); `markov.rs`
  (red-line: never modified, `mod.rs:14`).

**For a worker with zero session context — acceptance path:**
1. **Confirm §6-P95-C1 first.** If no real repeated-write/-query caller exists, **STOP — report HELD,
   NO-GO.** Do not write code ahead of the caller.
2. Land the incremental `add_document` (§3.2) with §7-P1/P2 GREEN as proptests **before** persistence —
   incremental is the correctness core; persistence rides on it.
3. Add Option A std-only persistence + dirty fingerprint (§3.1, §3.3) with §7-P3 GREEN. Escalate to
   Option B only if §6-P95-C3 also holds.
4. Do **not** build SIMD tokenization (§4) unless a benchmark after step 2-3 shows cold-build/bulk-ingest
   tokenization is a measured bottleneck and §7-P5 is GREEN.
5. Keep `lm --selftest` and the recall@5=1.0 fixture GREEN throughout (§7-P-REG). A failing equivalence
   property means the optimization is wrong — fix the code, never the property.
