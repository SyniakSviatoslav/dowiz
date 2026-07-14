# Wave 2 SDD Spec — A1 (done) / A2 / P7 / P10

> Binding: SDD gate (MEMORY.md § SDD GLOBAL HOOK). Research→analyze→plan→tasks→implement→converge.

## Ground-truth re-anchoring (this turn, grep/git/test, NOT doc claims)
- **A1**: `scripts/automation/tier3-batch.sh` has NO `apps/api` reference; its only `apps/api`
  mention is a USAGE EXAMPLE comment already pointing at `attic/apps-api`. Spec "repoint
  apps/api → attic/apps-api" is ALREADY satisfied. → A1 DONE, no edit (do not fake-edit).
- **A2**: living-knowledge retriever = `createRetriever(files, embedder).search(q)` (JS, on divergent
  branch `recover/stash-1-2994e6c8`). No CLI entry; embedder is bge-small ONNX (network-gated).
  Decision #1 (user): leave spike on branch, wire via ADAPTER (no merge into kernel).
- **P7**: depends on `rebuild/crates/domain/src/kernel.rs::decide` — but `rebuild/crates/domain`
  is ABSENT from current tree. P7 CANNOT be applied until domain crate is restored. Red-line
  money/auth path → must be its own SDD sub-wave after a controlled crate restore.
- **P10**: LICENSE = Apache-2.0 (roadmap mandates AGPLv3). DCO/NOTICE/TM absent. Secrets recoverable
  in history (rotated, not scrubbed). User authorized force-push scrub; P10 GATES 1-3 are reversible
  commits, gate 4 (scrub) is irreversible but authorized WITH backup ref.

## Requirements (EARS)
- REQ-A2-1: Kernel exposes `LivingKnowledge` trait + `SubprocessLivingKnowledge` adapter that
  invokes a bridge command via JSON-over-stdin/stdout, fail-closed if bridge/cmd absent.
- REQ-A2-2: Adapter protocol verified by a RED→GREEN test using a deterministic hash embedder
  (no ONNX/network) over a tiny mock corpus — proves the protocol + fusion parse end-to-end.
- REQ-P10-1: Repo ships AGPLv3 LICENSE, DCO file, root CONTRIBUTING.md (DCO clause), NOTICE, TM policy.
- REQ-P10-2: Pre-scrub backup bundle created; then `git filter-repo` removes credential keys/values
  from ALL branches; force-push to origin. Backup ref retained.
- REQ-P7-1 (deferred): after domain crate restored, `create_order` routes through `kernel::decide`;
  `decide_gateway.sh` gate goes RED→GREEN.

## Risks
- A2: ONNX embedder at runtime is network-gated → adapter test MUST use deterministic embedder, not ONNX.
- P10-scrub: IRREVERSIBLE rewrite of all branches → backup bundle is mandatory; verify no live secret.
- P7: red-line merge of parked domain crate → conflict risk; own sub-wave + gate test.
