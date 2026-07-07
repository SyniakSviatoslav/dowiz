# System comparison — before vs after (2026-07-07 session)

_Every AFTER value is re-measured by `scripts/probe-system-comparison.mjs` (Verified-by-Math: falsifiable, not asserted). BEFORE values marked [audit] are from docs/operating-model/fable-audit-findings-2026-07-07.md._

## Harness (Fable-audit top-5 + Verified-by-Math)

| Metric | Before | After (measured) |
|---|---|---|
| guard-bash false-positive rate (7 legit cmds) | 2/7 = 29% (sim of old logic) · ~83% [audit] | **0/7 = 0%** |
| guard-bash missed real blocks (5 protected writes) | 0/5 | **0/5** |
| KNOWLEDGE-AS-CIRCUITS enforced in pre-commit | no (0 refs) [audit] | **yes** (6 circuits, wired=true) |
| 0-tool-use degenerate subagent checker | absent [audit] | **armed** (self-test exit 0) |
| Enforced proofs that are falsifiable (VbM) | unenforced [audit] | **13/13** (exit 0) |
| Loop-registry parity violations (lying CERTIFIED + bogus citations) | 12 [audit] | **0** (exit 0) |
| Removed-machinery refs under loops/**·skills/** | 14 files [audit] | **0** |
| Fable dispatch default | warn (purged) | **deny (re-armed)** |

## Islands connected (brain-in-brain: no orphaned guardrail)

No-orphan gate: **all 17 guardrails wired to a runner** (exit 0). Each formerly-unrun guardrail is now live + falsifiable + interconnected:

| Guardrail (was an island) | Interconnected (in a runner) | Live (runs, exit) | Working (reachable RED path) |
|---|---|---|---|
| guardrail-definer-search-path.mjs | ✓ | ✓ (exit 0) | ✓ |
| guardrail-no-set-cookie.mjs | ✓ | ✓ (exit 0) | ✓ |
| guardrail-sandbox-staleness.mjs | ✓ | ✓ (exit 0) | ✓ |

Enforced permanently by `scripts/guardrail-no-orphan-guardrails.mjs` (run-armaments): a guardrail no runner invokes is dead machinery — a false-positive green — and now reds the suite.

## Living-knowledge retrieval — activation vs pure-vector baseline (the "old" approach)

Corpus: 77 files, 172 reference edges, K=5. Backend-agnostic (identical on MemoryStore or the HelixStore mirror).

| Metric | Baseline (pure vector) | Activation (bands) |
|---|---|---|
| recall@5 | 0.813 | **0.875** (Δ 0.062) |
| precision@5 | 0.225 | **0.25** |
| mean top-1 confidence: real vs nonsense query | — | **0.452 vs 0.357** (separable) |
| determinism (bit-identical reruns) | — | **true** |
| cross-layer analysis (brain-in-brain) | not possible | **29 island nodes**,  disconnected layer-pairs over 77 nodes/172 edges |

## HelixDB (Option C: sovereign default + dev-gated real-engine adapter)

- Real engine (`ghcr.io/helixdb/enterprise-dev` v3.0.8) **built + run + smoke-queried** live (see docs/operating-model/… handoff). Wire contract reverse-engineered from the engine's own validation errors (`returns` nested in `query`; properties as `[key,{Value:{String}}]` pairs).
- `spikes/living-knowledge/helix-adapter.test.mjs` (LK_HELIX=1) proves the sovereign store speaks the real engine: readiness 200, AddN 3/3, count 0→3 round-trip.
- Default backend stays sovereign MemoryStore (engine is closed/unlicensed → collides with Sovereign-Core/open-source/ethics; never prod).
