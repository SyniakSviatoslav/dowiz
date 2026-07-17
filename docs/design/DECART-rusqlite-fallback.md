# DECART — rusqlite (SQL fallback adapter, M4)

**Integration:** `rusqlite` added to `kernel` as the **optional** SQL persistence
fallback behind the `pgrust` native adapter (M4 living-memory: pgrust is default,
SQL is opt-in, never the default path).

| Criterion | rusqlite (chosen, opt-in) | pure-Rust sqlite (rusqlite is the only mature binding) | in-kernel vectorless (default) |
|---|---|---|---|
| Fit to sovereign bare-metal core | opt-in only, behind feature flag; default path stays `no_std`-clean | same (rusqlite IS the binding) | native default, zero-dep |
| Correctness & security | audited binding over SQLite (battle-tested engine, FFI) | n/a | verifier-actually-rejects tests in kernel |
| Performance — measured | local file DB; acceptable for opt-in fallback | n/a | deterministic, no I/O |
| Supply-chain & license | MIT, deny-clean | MIT | none |
| Maintainability | thin FFI, well-documented | n/a | native |
| Reversibility | **port/adapter/fallback — NOT core** (M4: native index is default) | n/a | default |
| Evidence cited | M4 decision (AGENTS.md memory): pgrust ONLY optional adapter; SQL opt-in | — | — |

**DECISION:** `rusqlite` as opt-in fallback — chosen because it is the only mature
SQLite binding and is gated behind a feature flag so the deterministic core stays
default. **Older-as-adapter:** it is a fallback bridge, not the primary store.
**Probe:** risk is FFI surface + the temptation to make SQL the default — mitigated
structurally by M4 (native index is the default; SQL is opt-in, never the default
path). Commit range: feat/p07-money-reversal.
