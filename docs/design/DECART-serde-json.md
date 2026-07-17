# DECART — serde_json (non-wasm JSON, kernel)

**Integration:** `serde_json` (optional, `wasm`-feature-gated) for JSON (de)serialization
in the kernel when building the wasm/JS bridge boundary.

| Criterion | serde_json (chosen) | manual Parser/Writer (hand-rolled) | simd_json |
|---|---|---|---|
| Fit to sovereign bare-metal core | optional, wasm-feature-gated; default build has no JSON | zero-dep, but re-implements a parser | faster, needs simd |
| Correctness & security | RFC-8259 compliant, fuzz-tested | easy to get wrong (unicode/number edge cases) | compliant |
| Performance — measured | adequate; not on hot path | N/A | faster on large docs |
| Supply-chain & license | MIT/Apache-2.0, deny-clean | none | MIT |
| Maintainability | vetted, no parser to maintain | maintenance burden | extra dep |
| Reversibility | feature-gated adapter; can swap | n/a | possible port |
| Evidence cited | serde ecosystem is the Rust-native default for JSON | — | — |

**DECISION:** `serde_json` (optional, wasm-gated) — chosen as the Rust-native default
for JSON; it is feature-gated so the default deterministic build excludes it.
**Older-as-adapter:** no older tech kept. **Probe:** the honest argument *against* is
"hand-rolled saves a dep" — rejected because a JSON parser is a classic
correctness foot-gun and serde_json is vetted + falsifiably compliant. Commit range:
prior kernel waves.
