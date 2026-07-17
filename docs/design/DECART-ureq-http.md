# DECART — ureq (pure-Rust HTTP client)

**Integration:** `ureq` (blocking, `default-features=false` + `tls` + `json`) as the
HTTP client in `llm-adapters`, `tools/telemetry/*`, `tools/async-spool` — the
adapter/transport layer that talks to external LLM/telemetry endpoints.

| Criterion | ureq (chosen) | reqwest (tokio/async) | hyper (low-level) |
|---|---|---|---|
| Fit to sovereign bare-metal core | pure-Rust, **no async runtime** needed | pulls tokio (async surface) | async, lower-level |
| Correctness & security | rustls TLS, deny-clean | tokio/native-tls risk | rustls |
| Performance — measured | blocking is fine for adapter call-outs; no runtime overhead | async overhead when unused | n/a |
| Supply-chain & license | MIT, deny-clean, no C build | can drag native-tls/openssl-sys | MIT |
| Maintainability | simple blocking API, easy to read | larger async surface | boilerplate |
| Reversibility | adapter behind a trait; swappable | n/a | possible |
| Evidence cited | B4 (2026-07-14): C-built crypto confined; ureq avoids native-tls | — | — |

**DECISION:** `ureq` — chosen as the Rust-native, **no-async-runtime**, deny-clean
HTTP client for the adapter layer (avoids tokio/native-tls C-build surface per B4).
**Older-as-adapter:** if an async transport is later needed, reqwest stays a port
behind the same adapter trait. **Probe:** the honest argument *against* is "reqwest
is more popular / async-first" — rejected as appeal to authority; the sync adapter
needs no async runtime and ureq's supply-chain is cleaner. Commits: llm-adapters +
telemetry waves.
