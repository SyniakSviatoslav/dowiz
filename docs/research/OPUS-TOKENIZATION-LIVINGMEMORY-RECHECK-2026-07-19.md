# OPUS — Tokenization vs Living-Memory Retrieval: implementation-state recheck (2026-07-19)

> RESEARCH-ONLY. Re-checks the operator's specific factual push-back against the earlier
> `OPUS-BATCH-TOKENIZATION-SCAN-2026-07-19.md` conclusion. Every claim cites `file:line` from a
> live read of the working tree (not memory recall). Zero code written, zero branches touched.

## The question

The earlier scan concluded: `recall.rs`'s BM25 corpus tokenization is a genuine hundreds-scale
batch, but runs **once per process behind a `OnceLock`, off the hot path** → not worth optimizing
"UNLESS live re-indexing becomes a repeated hot path." The operator pushed back specifically:

> "retrieval tokenization is needed for the living memory retrieval & spectral local node memories"

— i.e. this is the engine behind an *ongoing, repeatedly-queried* living-memory system, not a
one-shot cost. This doc determines the **real current implementation state** to decide whether that
push-back changes the conclusion.

---

## 1. Is corpus tokenization actually re-run repeatedly today? — NO (in the running code)

There are exactly **two** corpus-build (= corpus-tokenization) paths, and I traced every caller of
`Bm25::new` / `PrimaryRecall::new` / `PrimaryRecall::from_dir` across the whole kernel.

### Path A — `PrimaryRecall::new()` over the 12-doc FIXTURE corpus — built once per process
- `recall.rs:210-221` builds `Bm25::new(docs)` over `FIXTURE_CORPUS` — **12 short docs**
  (`recall.rs:45-70`), not the hundreds-scale corpus.
- Wrapped in `static PRIMARY: OnceLock<PrimaryRecall>` (`recall.rs:313`), initialized via
  `PRIMARY.get_or_init(PrimaryRecall::new)` (`recall.rs:315-317`).
- The free function `recall::recall_at_k` (`recall.rs:324-326`) — the surface the self-improvement
  loop and the `living_knowledge` adapter (`living_knowledge.rs:134-135`) delegate to — goes through
  this `OnceLock`. **Genuinely built-once-per-process, and it's only 12 docs** (~110 tokens total),
  not the 200-610-doc batch the earlier scan was worried about.
- The criterion bench (`benches/criterion.rs:233-235`) also uses `PrimaryRecall::new()` (fixture) and
  benches `recall_at_k` only — it does **not** rebuild the index in the hot loop.

### Path B — `PrimaryRecall::from_dir()` over the real 202-file living-memory corpus
- `from_dir` (`recall.rs:266-309`) walks a directory, reads every `*.md`, and builds a **fresh**
  `Bm25::new(docs)` + `TrigramIndex` **on every call** — it is *not* behind any `OnceLock` or cache.
  This is the "hundreds-scale batch" tokenization (the live corpus is **202 `.md` files** today).
- **The only caller of `from_dir` is the `lm` CLI binary** (`bin/lm.rs:76`). `lm` is a one-shot
  process: parse args → `from_dir` (one full corpus tokenize+index build) → one `recall_at_k` →
  `println!` → exit (`bin/lm.rs:76-85`). One process = one corpus build = one query.

### Query-time tokenization is trivial either way
`fusion_rank` / `recall_at_k` tokenize only the **query string** (`recall.rs:147`, `recall.rs:232`)
— a handful of tokens via `bm25::tokenize` (`bm25.rs:86-100`). The expensive batch is *corpus*
tokenization, and that happens at *index-build* time, never per-query.

### The one "wired" live caller is DEAD
`tools/telemetry/governance.sh` is the only place that reaches for the real-corpus path
(`gov_recall`, `governance.sh:237-243`). It is broken two ways:
1. It gates on `[ -f "$GOV_LM" ]` but only `$GOV_LM_BIN` is ever defined (`governance.sh:31-34`);
   `$GOV_LM` is **undefined/empty** → the test is always false → it **always** falls through to
   `gov_precedent`.
2. Even if reached, it calls `python3 "$GOV_LM"` — running the compiled Rust `lm` binary through the
   Python interpreter (a second bug; `lm` is native, replacing the retired `living_memory.py`).
Moreover `gov_recall` itself has **zero callers** anywhere in `tools/`, `scripts/`, `.claude/`. So
the real-corpus retrieval path is **not actually invoked by anything currently running** — not on a
schedule, not per-write, not per-query. (The `lm` binary *is* built on disk — `target/{debug,release}/lm`
— so it can be run manually, but nothing drives it in a loop.)

**Verdict for §1:** The earlier scan is **correct for the code as it runs today**. There is no
repeated corpus re-tokenization on any live path. The self-improvement loop path is a 12-doc fixture
built once; the real 202-file path exists only in a one-shot CLI whose sole wired caller is dead.

---

## 2. Is the 4-layer (trigram/BM25/HNSW/diffusion) living-memory design implemented? — PARTIAL; it's a blueprint

`docs/design/internal-retrieval-living-memory-blueprint.md` is explicitly a **design blueprint**
("Status: v3 (2026-07-14)", roadmap **M0–M8** each "gated by a benchmark vs an honest baseline",
§8/§11.6). Mapping its four layers (§3 table) to actual built code:

| Layer | Blueprint | Built in kernel today? |
|---|---|---|
| **L0 · exact/trigram** | trigram inverted index | **YES** — `retrieval/index.rs` (`TrigramIndex`), fused in `recall.rs:139` |
| **L1/L2 · lexical BM25** | hand-rolled BM25 | **YES** — `retrieval/bm25.rs`, fused in `recall.rs:138` |
| **L2 · semantic (HNSW/embeddings)** | flat SIMD → HNSW, ONNX embed | **NO** — explicitly out of scope: "The semantic (ONNX) signal is intentionally out of scope here… a build-time neural model, not a kernel primitive" (`recall.rs:18-23`) |
| **L3 · relatedness/diffusion** | personalized-PageRank / heat-kernel | **modules exist, NOT wired** (see §3) |
| pgrust living-memory store (§5) | `memory_notes`/`memory_links` SQL | **NOT live** — `retrieval/memory_store.rs` exists but has **zero live consumers** (only the `mod` decl `mod.rs:28` + doc-comment mentions); blueprint itself says "pgrust is a *plan*, not code" (§2) |

So: **L0 + lexical-L1 are built and fused** (that is the recall@5=1.0 path). L2-semantic and the
pgrust store are **not built**. The blueprint's marquee unification (§7 "one diffusion operator
across recall + cache-prefetch + code-relatedness", §11.3 second-order field operator, §12 resolvent
spine) is tagged **SPECULATIVE** by the doc's own author ("prototype and measure, do not assume",
§7). Do not conflate "there is a rich design doc" with "it is running."

`BLUEPRINT-W18-living-knowledge-wiring.md` (the piece that *did* land) scoped only the lexical Rust
recall wiring — "No ONNX/JS spike… Pure Rust deterministic recall" (W18 NON-GOALS). That is exactly
what `recall.rs` implements. Nothing more of the 4-layer stack was in W18's acceptance.

---

## 3. "Spectral local node memories" — the diffusion/PageRank layer is NOT connected to recall

- The L3 modules **exist and are tested**: `retrieval/ppr.rs` (Personalized-PageRank power-iteration
  mirroring `markov.rs`) and `retrieval/diffusion.rs` ("what relates to X" over a **frozen 20-node /
  41-edge wikilink fixture**, `diffusion.rs:9-14`).
- But they have **zero callers outside their own files**. Grepping `ppr::` / `diffusion::` /
  `use …ppr` / `use …diffusion` across `kernel/src` returns nothing — they are standalone, self-tested
  modules, never invoked by `recall.rs`, `bm25.rs`, `living_knowledge.rs`, or the `lm` binary.
- `recall.rs` / `bm25.rs` / `living_knowledge.rs` contain **no** reference to ppr/diffusion/spectral/
  PageRank as *code* — the only "pagerank" occurrences are literal **corpus text** (fixture documents
  that happen to *describe* pagerank, `recall.rs:58-59`, `living_knowledge.rs:260`), not linkage.
- `kernel/src/spectral.rs` has **zero** references to `retrieval` / `recall` / `memory` / `wikilink`.
- `diffusion.rs` runs over a **hand-built 20-node fixture graph**, not the live 202-file corpus or its
  462 wikilinks. There is no extraction of the real wikilink graph into `memory_links` (blueprint §5,
  M4 — unbuilt).

**Verdict for §3:** the "spectral local node memories" connection is **aspirational / blueprint-stage**,
not a built connection. The spectral relatedness engine is not fused with, seeded by, or reachable
from the tokenization/recall path today.

---

## 4. Honest verdict — does the operator's push-back change the earlier conclusion?

**Both statements are true, at different tenses, and they don't contradict:**

- **The earlier scan's conclusion is correct for the code as it runs today.** No live path re-runs
  hundreds-scale corpus tokenization repeatedly. The self-improvement-loop recall is a 12-doc fixture
  behind a `OnceLock` (built once, ~free). The real-corpus path is a one-shot CLI whose only wired
  caller is dead. Query-time tokenization is just the short query string. **Batch/SIMD corpus
  tokenization is NOT a present-tense hot-path win.**

- **The operator's push-back is correct as a forward-looking architectural claim, and it exposes a
  real latent hazard the earlier scan under-weighted.** Tokenization *is* the front-end of the
  living-memory retrieval engine, and the **current `from_dir` design re-tokenizes and rebuilds the
  ENTIRE corpus from scratch on every call** (`recall.rs:266-309`) — there is **no incremental index
  update, no persisted index, no cache**. So the moment living-memory retrieval is wired as a *real*
  system, corpus tokenization flips into a repeated cost in **two** concrete ways:
  1. **Repeated-write (living memory):** every new/edited memory note that triggers a reindex pays a
     full 202+-doc re-tokenize, because `from_dir` has no incremental path. Living memory is
     write-heavy by definition (append notes, reinforce, demote tiers — blueprint §5/§6), so a
     naive "rebuild on write" would re-tokenize the whole corpus per write.
  2. **Repeated-query via the CLI process model:** the `lm` binary rebuilds the index per invocation
     (`bin/lm.rs:76`). If anything ever drives `lm` per-query in a loop (the governance.sh intent,
     currently dead), *every query* pays a full corpus rebuild.

**Classification: this is a "flag for when the living-memory blueprint (M0-M8 / a real W18 real-corpus
wiring) is actually built" finding — NOT a present-tense optimization.** Nothing waits on tokenization
cost today. But the recommendation should be recorded against the blueprint so it isn't rediscovered
the hard way: **the fix at that point is primarily architectural (persist/incrementally update the
index), and only secondarily SIMD.**

### What the real fix looks like for the repeated-query/repeated-write pattern (when built)

Ordered cheapest-proven-first (matches the blueprint's own §9 guardrail):

1. **Persist the built index; don't rebuild per process/query.** The dominant waste in the CLI model
   isn't slow tokenization — it's re-tokenizing 202 docs to answer one query. Build once, serialize
   the `Bm25` (tf maps, df, avgdl) + `TrigramIndex`, memory-map or load on query. This alone removes
   the "repeated hot path" entirely for the query case. **Biggest win, no SIMD.**
2. **Incremental indexing on write.** Living memory adds/edits one note at a time. Tokenize only the
   changed doc and update `df` / `tf` / `avgdl` deltas, instead of `Bm25::new(all_docs)`. Turns a
   per-write O(corpus) re-tokenize into O(changed-doc). This is the single change that makes
   "live re-indexing" cheap and is the real answer to the operator's concern.
3. **Only then, batch/SIMD the tokenizer** for the (now bounded) work that remains — the initial
   cold build and large multi-doc ingests. `bm25::tokenize` (`bm25.rs:86-100`) is a scalar
   char-by-char `is_ascii_alphanumeric` scan with a per-token `String` allocation; the SIMD-friendly
   rewrite is a classic byte-classification pass (SWAR/`memchr`-style alnum-run detection) emitting
   `&str` slices into one arena instead of `Vec<String>`. Worth it for a large cold ingest; a
   rounding error next to steps 1-2 for the steady state.

The determinism contract (fixed summation order, ascending-doc-id tie-break — `bm25.rs:197-201`,
`bm25.rs:233-239`) must survive any such change; a SIMD tokenizer must produce the byte-identical
token stream the current scalar one does.

---

## Appendix — evidence index (all live-read this session)

- `kernel/src/retrieval/recall.rs` — `FIXTURE_CORPUS` 12 docs (45-70); `new()` build (210-221);
  `from_dir` full rebuild (266-309); `OnceLock PRIMARY` (313-317); free `recall_at_k` (324-326).
- `kernel/src/retrieval/bm25.rs` — `tokenize` scalar scan (86-100); `Bm25::with_params` corpus build
  (136-163); deterministic `rank` (222-241).
- `kernel/src/bin/lm.rs` — one-shot CLI, sole `from_dir` caller (76-85).
- `kernel/src/retrieval/mod.rs` — module wiring; L3 ppr/diffusion declared (mod.rs), memory_store
  declared (28) but unconsumed.
- `kernel/src/retrieval/ppr.rs` / `diffusion.rs` — L3 spectral modules, standalone, **zero external
  callers**; diffusion over a 20-node fixture (diffusion.rs:9-14).
- `kernel/src/living_knowledge.rs` — delegates to `retrieval::recall::recall_at_k` (134-135).
- `kernel/src/spectral.rs` — **no** retrieval/recall/memory references.
- `tools/telemetry/governance.sh` — `GOV_LM_BIN` defined (31-34); `gov_recall` dead + uncalled (237-243).
- `docs/design/internal-retrieval-living-memory-blueprint.md` — v3 blueprint, M0-M8 gated, pgrust
  "a plan not code", §7/§11/§12 SPECULATIVE.
- `docs/design/BLUEPRINT-W18-living-knowledge-wiring.md` — landed scope = lexical Rust recall only.
- Live corpus size: **202 `.md` files** in `/root/.claude/projects/-root-dowiz/memory/`.
