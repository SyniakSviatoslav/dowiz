# Batch/SIMD Tokenization Scan — dowiz/DeliveryOS

> Research-only. No code written, no branches touched. Scope: does "tokenization"
> (two distinct meanings) contain a genuine BATCH point large enough (hundreds+ items)
> to justify bit-sliced/SIMD batch processing? Discipline: honest-negative when the
> target is not real. Sibling task covers mesh consensus batch-verification — this
> doc deliberately does NOT duplicate it.
>
> Date: 2026-07-19 · Model: Opus · Method: Read + Grep over `kernel/src`, `apps/`,
> live corpus. Every claim carries a file:line citation.

---

## TL;DR

| Angle | Genuine batch point? | Batch size | SIMD/bit-slice verdict |
|-------|----------------------|-----------|------------------------|
| 1. Capability/auth token validation | **No** | 1 chain/frame at a time; ≤4 links | **HONEST NEGATIVE** — architecturally one-at-a-time + rate-limited; the only multi-item dimension (sigs) is out-of-scope AND already a settled negative (B4). |
| 2. Text tokenization for retrieval | Partial — corpus tokenization at index build (hundreds of docs) | 202 docs / ~1.0 MB (live); up to 610 / ~13.5 MB (docs) | **HONEST NEGATIVE** — the hundreds-scale batch is a **cold, once-per-process** path (OnceLock / one-shot CLI); the per-query path tokenizes a single tiny string. SIMD-amenable *shape*, but off every hot path. |

Neither angle is a real bit-slicing/SIMD target today. Both negatives are load-bearing, not hand-waves — details below.

---

## Angle 1 — Capability / auth token validation

### What the validation path actually is

Three real validators, all **single-item**:

1. **`verify_chain_hybrid`** — `kernel/src/capability_cert.rs:776`. Anchor-rooted, hybrid-signed (Ed25519 ⊕ ML-DSA-65) capability-cert chain verification. The internal loop `for (i, link) in chain.iter().enumerate()` (`capability_cert.rs:798`) walks the links of **one** chain, hard-capped at `MAX_CHAIN_LEN = 4` (`capability_cert.rs:764`) and `MAX_DELEGATION_DEPTH = 1` (`capability_cert.rs:766`). It verifies one `root` + one `chain` + one `cap` per call.

2. **`verify_chain`** (UCAN-subset) — `kernel/src/ports/agent/cap.rs:486`, loop `for link in chain` at `cap.rs:498`. Same shape: one chain at a time.

3. **`AdmissionGate::check`** / **`Admitter::admit`** — `kernel/src/ports/agent/admission.rs:184` and `:394`. Each call takes exactly **one** `frame: &SignedFrame`. The 7-step sequence (freshness → verify_chain → red-line → revocation → classical verify → PQ verify → nonce record) runs per frame. There is **no batch-admit API**.

### Two facts that kill the batch thesis

- **No production caller batches these.** Every one of the ~15 `verify_chain_hybrid` call sites is inside `#[cfg(test)] mod tests` (`capability_cert.rs:1017,1085,1104,1120,1154,...,1674`). This is a BLUEPRINT-P59 deliverable that is not yet wired to any request loop, so there is not even a *single-item* production hot path, let alone a batch one.

- **The design deliberately rate-limits, i.e. de-batches.** `AdmissionLimiter` (`admission.rs:259`) is a mandatory pre-crypto `TokenBucket` gate — `try_admit` does "one O(1) integer decrement … `false` ⇒ drop the frame pre-crypto" (`admission.rs:283-294`). The architecture's explicit goal is to *bound* the number of frames that reach crypto per unit time. Accumulating a large batch to SIMD-verify is the opposite of the stated posture.

### No HTTP-request-level token path exists in this tree

The task hypothesised an HTTP auth path (JWT/bearer over incoming requests). It is **not present**:

- `apps/` contains only `apps/courier` (Rust) — `ls apps` → `courier`. The `apps/api` TypeScript server named in the Repowise index (`.claude/CLAUDE.md`) is **stale**; it is not in the working tree.
- Grep for `jwt.verify | jsonwebtoken | jose | bearer | verifyJwt` across `apps/**/*.ts` returns **zero** hits (no `.ts` sources exist under `apps/` at all).
- No `axum`/`actix`/`hyper::Server`/`express` HTTP server is present in `kernel/src`, `apps`, `engine`, or `web`.

So there is no "batch of incoming requests each carrying a cert, validated in a loop" surface to optimise. It does not exist yet.

### The one multi-item dimension is out-of-scope AND already closed

Within a *single* admission, the RequireBoth floor performs ~10 signature-leg verifications (root self-sig 2 legs + up to 3 links × 2 legs + frame 2 legs). Batching *those* Ed25519/ML-DSA legs is:
- (a) exactly the **mesh capability-chain verification** the task told this scan to exclude (sibling task owns it); and
- (b) a **settled negative**: the B4 pass (`6541ae8`, `openbebop`) proved batch Ed25519 verify gives **no throughput benefit** after the SSR-2020 mixed-order forgery fix, because every batch-accept must re-verify singly (see MEMORY `crypto-safe-first-pass-2026-07-14`). Re-opening it here would contradict a proven result.

### Verdict — Angle 1: HONEST NEGATIVE

Token validation is one-chain / one-frame at a time by construction, has no production batch caller, is guarded by a rate limiter that intentionally caps throughput, and has no HTTP-request auth surface in-tree. The only "many items" axis (multiple signatures) is both out-of-scope and a proven no-win. There is no real SIMD/bit-slicing target here.

---

## Angle 2 — Text tokenization for retrieval (recall.rs / BM25)

### The actual tokenizer

`kernel/src/retrieval/bm25.rs:86-100`, `pub fn tokenize(text: &str) -> Vec<String>`:

```
for ch in text.chars() {
    if ch.is_ascii_alphanumeric() { cur.extend(ch.to_lowercase()); }
    else if !cur.is_empty() { out.push(take(cur)); }
}
```

A scalar, char-by-char, lowercase+alnum word-boundary scan. It processes **one string at a time**. This is the single tokenization primitive; `Document::from_text` (`bm25.rs:60`) and every query path call it.

### Where it is invoked — two contexts, both wrong-shaped for SIMD

**(a) Per-query tokenization — one tiny string, no batch.**
`recall.rs:147` (`fusion_rank`) and `recall.rs:232` (`recall_at_k`) call `super::bm25::tokenize(query)` on a single query string (a natural-language question, tens of bytes). One short string per recall call. No batch, nothing to vectorise across.

**(b) Corpus tokenization at index build — a genuine hundreds-scale batch, but COLD.**
`Bm25::new` (`bm25.rs:131`) tokenizes every document via `Document::from_text`; `PrimaryRecall::from_dir` (`recall.rs:266-309`) reads every `*.md` in a dir and tokenizes each; `PrimaryRecall::new` (`recall.rs:210`) tokenizes the fixture. This *is* many documents processed together.

### Real corpus scale (measured, not guessed)

| Corpus | Docs | Bytes | Source |
|--------|------|-------|--------|
| Fixture (`FIXTURE_CORPUS`) | **12** | ~1 KB | `recall.rs:45` |
| Live living-memory (`from_dir` target) | **202** | **995,692 (~1.0 MB)** | `bin/lm.rs:10` → `/root/.claude/projects/-root-dowiz/memory/*.md` |
| `docs/` corpus | **610** | ~13.5 MB | `find docs -name '*.md'` |
| Repo-wide `*.md` | 3391 | — | (theoretical ceiling) |

So the corpus batch is 202–610 documents / 1–13.5 MB — comfortably in the "hundreds+" band where SIMD *could* matter, and the alnum-classification scan is a textbook SIMD-classification shape (parallel `is_ascii_alphanumeric` + boundary detection, à la simdjson-style byte classification).

### Why it is still a negative: the batch is cold, once-per-process

- `PrimaryRecall::new()` sits behind a `OnceLock` static — `static PRIMARY: OnceLock<PrimaryRecall>` with `get_or_init` (`recall.rs:313-317`). The corpus is tokenized **once per process**, then reused for every query.
- `from_dir` is a one-shot CLI ingest (`bin/lm.rs` — "ingest … in-process, zero subprocess", `bin/lm.rs:4-10`), not a per-request loop.
- Real consumers of the BM25 recall path are `bin/lm.rs` (one-shot) and the **wasm-gated** `living_knowledge` adapter (`recall.rs:334`). No in-tree hot loop re-tokenizes the corpus.

Tokenizing ~1 MB once at startup is sub-millisecond-to-low-millisecond scalar work, off every request path. SIMD would shave a cold startup cost nobody waits on — negative ROI, and it would add `unsafe`/target-feature complexity to a `pure-std, deterministic` module whose determinism is load-bearing (`bm25.rs:1-6`).

### Adjacent note (not tokenization, flagged to avoid a wrong turn)

`Bm25::rank` (`bm25.rs:222`) scores **all** docs per query — O(N_docs · |Q|). At N=202 this is trivial; at N=3391 it grows. But this is HashMap-gather BM25 float arithmetic (`score_doc`, `bm25.rs:187`), **not** tokenization, and its scatter/gather over per-doc `HashMap<String,u32>` term-frequency maps is not bit-sliceable. If a future perf pass targets recall latency at large N, the lever is the scoring data layout (e.g. postings/columnar tf), not SIMD tokenization. Out of scope for this scan; recorded so it is not mistaken for a tokenization target.

### Verdict — Angle 2: HONEST NEGATIVE

The per-query tokenization is a single small string (no batch). The corpus tokenization is a real hundreds-scale batch with a SIMD-amenable *shape*, but it runs once per process at index build, off any hot path — so vectorising it optimises a cost that is not on any latency budget. Not a real SIMD/bit-slicing target today.

**Conditional (documented, not endorsed):** IF corpus re-indexing ever became a *repeated* hot path — e.g. a live re-index fired on every living-memory write, re-tokenizing 1–13.5 MB each time — then `bm25::tokenize`'s alnum byte-classification would become a legitimate SIMD candidate. That hot path does not exist in the tree today; there is currently nothing to accelerate.

---

## Bottom line

Both "tokenization" meanings are honest negatives for bit-sliced/SIMD batch processing:

1. **Auth/capability tokens** — validation is one-chain/one-frame, rate-limited, has no production batch caller, and no HTTP auth surface exists in-tree; the multi-signature axis is out-of-scope and already proven no-win (B4).
2. **Retrieval tokenization** — the only real hundreds-scale batch (corpus index build) is cold/once-per-process; the per-query path is a single tiny string.

Consistent with today's discipline: no manufactured target. If a batch-tokenization SIMD win exists anywhere in this codebase, it is not in either place "tokenization" points to.
