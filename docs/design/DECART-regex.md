# DECART — regex (kernel text parsing)

**Integration:** `regex` (optional, wasm-feature-gated) for pattern matching in the
kernel (e.g. event-log / capability-string parsing at trust boundaries).

| Criterion | regex (chosen) | hand-rolled state machine | fancy-regex |
|---|---|---|---|
| Fit to sovereign bare-metal core | wasm-gated; feature-flagged out of default | zero-dep | heavier |
| Correctness & security | fuzz-tested, RFC-528 substring/regex semantics | easy to get wrong | superset of syntax |
| Performance — measured | linear-time (lazy DFA) | N/A | slower (backtracking) |
| Supply-chain & license | MIT/Apache-2.0, deny-clean | none | MIT |
| Maintainability | vetted, no parser to maintain | burden | extra |
| Reversibility | adapter; swappable | n/a | possible |
| Evidence cited | regex is the Rust-native default for pattern matching | — | — |

**DECISION:** `regex` (optional, wasm-gated) — chosen as the Rust-native default;
feature-gated so the default deterministic build stays dependency-light.
**Older-as-adapter:** no older tech kept. **Probe:** the honest argument *against* is
"a hand-rolled matcher saves a dep" — rejected because regex correctness at trust
boundaries is vetted + falsifiably tested, unlike a bespoke matcher. Commit range:
prior kernel waves.
