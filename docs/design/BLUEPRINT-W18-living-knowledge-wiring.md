# BLUEPRINT W18 — living-knowledge Rust wiring

## WHY
`kernel/src/living_knowledge.rs` exists (4 pub items: eval/search/recall adapter) but
ORGANISM-STATUS 07-15: the JS living-knowledge engine was purged; the Rust adapter is NOT
consumed by the self-improvement loop. Recall@k must run through the Rust path (no JS).

## WHAT (acceptance)
- `LivingKnowledge` trait/adapter wired into `retrieval/mod.rs` as a PRIMARY recall source
  (mirrors `wire living-memory as PRIMARY retrieval` from 07-13 commits).
- `recall_at_k(query, k) -> Vec<(doc_id, score)>` deterministic, no float nondeterminism.
- Surface a `kalman`/`trigram`-fed pattern signal back into the loop (see W19).

## RED→GREEN
- RED: `living_knowledge` module compiles but no caller invokes recall (grep: 0 usages outside tests).
- GREEN: a `retrieval/mod.rs` integration test calls `recall_at_k` and asserts top-k matches a
  known fixture (recall@5=1.000 on the 324-file corpus, deterministic). 0 JS.

## FILES (Owns — disjoint)
- Modify: `kernel/src/retrieval/mod.rs` (register adapter), `kernel/src/living_knowledge.rs` (expose recall API)
- Test: `kernel/src/retrieval/tests.rs` (new recall integration test)

## RISKS
- Corpus path: Rust adapter needs a corpus source. Reuse `retrieval/fixtures.rs` or an in-repo
  indexed corpus; do NOT shell out to deleted JS. If no corpus on disk, build from `retrieval/index.rs`.
- Keep it std-only (M4: native store default, pgrust opt-in).

## NON-GOALS
- No ONNX/JS spike (that was the purged A2 branch). Pure Rust deterministic recall.
