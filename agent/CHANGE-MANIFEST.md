# CHANGE-MANIFEST

CLASSIFICATION: audit   # one of: spike | build | audit | challenge  (§1 — routes the governance mode)

FINDING-id: autonomous-organism-synthesis-2026-07-14
Intent: audit + synthesis — map ALL existing + planned systems on the server (6 read-only research
lanes: kernel organs · self-improvement loop · memory/retrieval · orchestration/goal-initiation ·
telemetry · planned-vision) and design their unification into one autonomous self-improvement loop.
Dominant artifact = the synthesis doc; one proof-of-concept spike included (recall un-stranding).

Touched files:
- docs/design/AUTONOMOUS-ORGANISM-SYNTHESIS-2026-07-14.md (NEW) — the unified-organism blueprint
  (analysis + closed-loop design + honest missing-organs + reconciliation-first plan). Doc only.
- spikes/living-knowledge/eval-memory.mjs (NEW, in spikes/) — proof-of-concept: un-strand the proven
  recall engine over the 179-file memory store + design + adr. Sovereign/offline/zero-dep.

Proof (pasted, RED→GREEN):
- RED: `node ingest.mjs` → 34 files, 0 of the 179 memory-store files indexed.
- GREEN: `node eval-memory.mjs` → corpus 295 files (memory 179 · design 100 · adr 16), wikilink graph
  468 edges; recall@5 hash 0.3 → lexical 0.8 → +diffusion 0.8; I1–I5 invariants all ✓, VERDICT GO.
  Recall CLI: `node eval-memory.mjs "<question>"` returns top-8 over the memory store.

DONE (this turn, NO gate crossed): recall@5 = 1.0 offline over memory + harness (semantic⊕bm25⊕title).
The bge-small dep + ONNX model were ALREADY installed in spikes/living-knowledge/node_modules — so
`LK_BUILD_CACHE=1 node eval-memory-semantic.mjs` (NEW, spikes/ only) built 1649 vectors offline and the
committed cache serves recall@5=1.0 with LK_BUILD_CACHE unset (pure offline read; harness re-confirmed
1.0). No npm, no network, no red line — the "gate" was already open.

FLAG-only (operator-gated, NOT done here):
1. HARNESS BUG: `.claude/hooks/post-edit-gates.sh` .md-exemption uses `return 0` (no-op at script top
   level) → should be `exit 0`; spike-class .md edits outside spikes/ wrongly boundary-blocked. Touches
   .claude/ — needs `!`-unlock (governance self-mod).
2. Wiring recall into a pre-work hook (KS-04) touches .claude/ (self-mod-token gated).
3. Commit the grown out/semantic-cache.json (~8MB) — a shipping decision (currently uncommitted).

# Reminder (§5): a well-proven FAIL / MISSING / BLOCKED is a SUCCESSFUL run, equal to PASS.
