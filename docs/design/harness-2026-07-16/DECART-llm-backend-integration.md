# DECART — LlmBackend Tier-1 integration (Ollama) + HTTP client crate

> Per `docs/operating-model/integration-decart-rule.md` / AGENTS.md's Integration Decart Rule:
> no silent adoption. Two decisions below, both required before `feat/harness-llm-backend`'s
> first commit (Step (a) of `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND-PLAN.md`).

## Decision 1 — Ollama as the `LlmBackend` Tier-1 local inference service

| Criterion | Ollama (chosen) | Fresh `llama-server` unit | Managed-API-only (no local tier) |
|---|---|---|---|
| Fit to sovereign bare-metal core | Go daemon, external to the Rust core — but consumed *only* through the kernel's own `LlmBackend` trait (compile firewall, `&dyn` boundary); adapter crate is the sole thing that knows Ollama exists | Also external (C++ binary); no material difference in fit | Best fit (zero local process) but forfeits the whole point of M5 hub-autonomy / local-tier cost |
| Correctness & security — falsifiable | Live-probed 2026-07-16: `/v1/chat/completions` returns full OpenAI schema incl. `usage`; `/v1/embeddings` and `/api/embed` both verified 200. `health()` fails closed (typed `Err`) when the daemon is down — falsifiable in Step (a)'s done-check | Same wire shape achievable, but requires standing up + testing a new systemd unit from scratch — more surface to get wrong before first light | N/A |
| Performance — measured | `ollama ps` shows 3 models resident simultaneously (llama3.1:8b 5.6GB + both embedding models) on this 32GB host, confirmed live; `OLLAMA_NUM_PARALLEL` defaults documented (Ollama FAQ) | Unmeasured — would need to be built and benchmarked first | N/A |
| Supply-chain & license | Already running as an existing systemd service (`ollama.service`, enabled, MIT-licensed upstream) — zero NEW supply-chain surface, it predates this integration | New binary + new systemd unit = new supply-chain surface to vet | N/A |
| Maintainability & clarity | One well-known model-management daemon (pull/list/serve) vs. hand-rolling a `llama-server` unit file, model-path management, and a bespoke OpenAI-compat shim ourselves | More moving parts we'd own and maintain | Simplest of all, but is a non-choice given the M5 requirement |
| Reversibility — port/adapter, not core commitment | **Exactly a port**: `OllamaAdapter` is one of three interchangeable `LlmBackend` impls behind `HubPolicy.llm_backend` config; swapping to a fresh `llama-server` or dropping local inference entirely is a config change, never a kernel edit | Same reversibility if built | Already the fallback (Tier 0 stays default) |
| Evidence cited | `systemctl is-active ollama.service` → active since 2026-07-13; `ollama list`/`ollama ps` outputs; live curl probes (see `HARNESS-LLM-BACKEND-PLAN.md` §1) | — | — |

**DECISION: Ollama, wired as `OllamaAdapter` behind the `LlmBackend` port.** Honest falsifiable reason:
it is *already running, already holding the exact model classes needed (chat + code + embeddings), and
already speaks the wire protocol the transport needs* — building a parallel `llama-server` unit would
duplicate a working, already-vetted local inference daemon for no measurable gain.

**Older-as-adapter:** N/A here (Ollama is the newer choice on this axis); the reversibility guarantee
above is what keeps this non-dogmatic — the port abstraction means Ollama is never a hard dependency of
the kernel, only of one pluggable adapter.

**Probe (the honest case against):** Ollama adds a Go daemon and its own model store *outside* the
kernel's sha3 manifest discipline (F3/F27) — a pulled model's integrity is Ollama's responsibility, not
ours, until a future `{url, sha3}` verify-or-deny layer wraps future pulls. This is a real gap, not
dismissed: it's why `OllamaAdapter`'s `health()`/`caps()` treat the backend as untrusted-but-available
(fail-closed on absence, never silently trusted on content) and why F3/F27 remains a named follow-up for
*future* model pulls, not solved by this integration.

---

## Decision 2 — HTTP client crate for `OpenAiCompatTransport`

| Criterion | `ureq` (chosen) | `reqwest` |
|---|---|---|
| Fit to sovereign bare-metal core | Synchronous, no runtime requirement — matches this repo's existing network-adapter pattern exactly | Requires a `tokio` runtime; heavier dependency graph for a use-case (one blocking call per dispatch task) that doesn't need it yet |
| Already used in this project | **Yes — twice.** `tools/telemetry/rust-spool/Cargo.toml` and `tools/async-spool/Cargo.toml` both already depend on `ureq = { version = "2", default-features = false, features = ["tls", "json"] }` with the rustls+ring backend, under an explicit **operator mandate (2026-07-15): "rustls with ring everywhere possible."** Reusing it here is the direct application of "harness patterns already used in the project," not a new choice | Not used anywhere in this repo today — would be the first instance |
| Correctness & security | rustls+ring (pure-Rust TLS, no OpenSSL) — the same provider this repo's own DECART worked example (§ in the rule doc) already chose for the bebop transport, for the same reason | Also supports rustls, but pulls in tokio's async runtime surface as a correctness-relevant dependency this use-case doesn't need |
| Performance — measured | Sufficient for Step (a)'s one-request-per-call shape; Step (d)'s concurrency is achieved by dispatching multiple blocking calls across threads/`spawn_blocking`, not by async I/O multiplexing (this workload is CPU/model-bound on the Ollama side, not I/O-bound — async gives no real throughput win here) | Would give async I/O concurrency the workload doesn't need (the bottleneck is Ollama's own `OLLAMA_NUM_PARALLEL`, not socket multiplexing) |
| Supply-chain & license | Already `cargo-deny`-clean in this repo (used twice already); adding a third consumer changes nothing | New dependency tree to vet |
| Maintainability & clarity | Same client, same TLS backend, same feature set (`tls`, `json`) as the two existing spool crates — one pattern to know across the codebase | A second, different HTTP-client pattern for future readers to learn |
| Reversibility | The `OpenAiCompatTransport` is the seam; swapping the underlying client later (if a genuinely async workload appears) is an internal change behind that seam, never a kernel change | — |

**DECISION: `ureq = { version = "2", default-features = false, features = ["tls", "json"] }`** —
the exact dependency spec already vetted twice in this repo. Honest falsifiable reason: it is the
established, operator-mandated pattern for HTTP-calling adapter crates here, and this workload
(one blocking request per dispatch, concurrency bounded by `OLLAMA_NUM_PARALLEL` on the server side, not
by client-side socket multiplexing) has no correctness or performance need for an async runtime.

**Older-as-adapter:** N/A (ureq is the current, actively-used choice, not a legacy fallback).

**Probe (the honest case against):** Step (d)'s `TokenBucket`-bounded concurrent dispatch was originally
scoped around `tokio` tasks in `HARNESS-LLM-BACKEND-PLAN.md` §4.2. Choosing `ureq` here means Step (d)'s
dispatcher achieves concurrency via OS threads (`std::thread::spawn` + a bounded pool, or
`std::sync::mpsc` for the overflow queue) instead of `tokio::spawn`/`tokio::sync::mpsc` — a real design
change from that plan, not a free substitution. This is accepted now and flagged explicitly for Step (d)'s
own implementation to resolve consciously (thread-pool-bounded dispatch is a legitimate, simpler
alternative to the tokio design given ureq's synchronous nature, and avoids adding tokio as a
non-optional dependency of the adapter crate — tokio stays confined to the kernel's already-optional
`pgrust` feature, never pulled into this crate).

---

*Both decisions apply to `feat/harness-llm-backend`. Filed 2026-07-16, prior to the branch's first
implementation commit, per the Integration Decart Rule's "no silent adoption."*
