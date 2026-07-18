# Local-LLM / Agentic-Infra / Mistral Audit — 2026-07-17

> Read-only audit run from isolated worktree `research/dowiz-verify-redteam-2026-07-17`.
> No code changed, no commits. System commands are read-only probes only.
> Method: `which`/`ollama list`/`systemctl`/`ps`/`free`; `grep` over the worktree checkout;
> source read of the real `LlmBackend`/`OllamaAdapter`/`TaskClass`/dispatch/transport files;
> a link-resolution pass over `MEMORY.md`; a real `cargo test` against the live daemon.

---

## 1. What local LLM infrastructure actually exists and runs TODAY

| Probe | Result |
|---|---|
| `which ollama` | `/usr/local/bin/ollama` — installed |
| `systemctl status ollama` | **active (running)** since 2026-07-13 (4 days), Main PID 7716, `ollama serve`, peak mem 19.5 G, service `enabled` |
| Live traffic | GIN access log shows real `/api/generate` (26 s) + `/api/tags` calls today (2026-07-17) — daemon is genuinely being hit |
| Host RAM | **30 GB total, ~26 GB available**; CPU-only (no GPU) |

**Models actually pulled on this host right now (`ollama list`):**

| Model | Size | Pulled | Role in code |
|---|---|---|---|
| `qwen2.5-coder:7b` | 4.7 GB | 2 days ago | `TaskClass::Code` route |
| `llama3.1:8b` | 4.9 GB | 4 weeks ago | `TaskClass::General` route |
| `nomic-embed-text:latest` | 274 MB | 4 weeks ago | `TaskClass::Embedding` route (768-dim) |
| `qwen3-embedding:0.6b` | 639 MB | 4 weeks ago | higher-quality embed option |

**Mistral / Mixtral is NOT among the pulled models, and never has been on this host.** The
only MoE-family model discussed anywhere is a paper idea (Mixtral 8×7B), never downloaded.

---

## 2. Mistral usage — actual grep of the codebase

**Code/config grep (`*.rs *.ts *.tsx *.js *.json *.toml *.yaml *.yml *.sh`, excl. node_modules/.git):
`mistral|mixtral` → ZERO matches. Zero files.**

Mistral/Mixtral is **not wired into any Rust, TypeScript, JSON, or TOML** in the repo. It is
**not** a configured backend, model id, route, or dependency of the kernel LLM stack.

Every occurrence anywhere (8 files total, all **docs or skill fixtures**, never executable
product wiring):

| File | Nature of mention |
|---|---|
| `docs/design/…MASTERWORK-SYNTHESIS-V2.md:505` | The **brainstorm** — "Mixtral 8×7B (MoE) mirrors the mesh", verdict **ADOPT-AS-REFRAME** (reframed to build-time domain oracles, *not* a runtime Mixtral) |
| `docs/design/LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md:132,370` | `mistral.rs` (the Rust server) — explicitly **"Rejected for now"** (duplicates the vetted Ollama daemon) |
| `TOOLING-REGISTRY.md:12`, `docs/operating-model/player-roles.md:10` | Mistral as one link in the **remote OpenRouter** rotation chain (Nemotron→Qwen→DeepSeek→Gemma→Mistral) — legacy dev-tooling, not the local kernel backend |
| `llm_quantization_report.md:29` | Mistral-7B row in a generic quantization-size reference table |
| `.claude/skills/last30days/SKILL.md:705`, `…/lib/categories.py:159` | "Mistral Large" as a string in a third-party skill's generic model list |
| `docs/audit/2026-06-18/deep-check-report.md:116` | Mistral named as a transitive `mem0ai` dependency flagged **for removal** (bloat) |

**Verdict: purely brainstorm/reference. Zero code. "Never became code" is the accurate finding.**

---

## 3. Agentic infrastructure health (the real LLM backend stack)

The stack is real, layered, and zero-`tokio` / zero-network in the kernel:

- **`kernel/src/ports/llm.rs`** (169 LoC) — the `LlmBackend` trait + value types. Compile-firewall:
  no HTTP/JSON/serde in the kernel. `TaskClass{Code,General,Embedding}`, typed
  `LlmError{Unavailable,Unsupported,BadRequest,Timeout}`, `CachePolicy{Exact,SemanticOk,NoCache}`
  as a **type** (gate-critical callers structurally barred from semantic cache), `Caps` fail-closed.
- **`llm-adapters/src/ollama.rs`** — `OllamaAdapter` implements the trait; `route_model` maps
  `TaskClass`→concrete model id (Code→`qwen2.5-coder:7b`, General→`llama3.1:8b`,
  Embedding→`nomic-embed-text`); `:tag` ids pass through; rerank returns `Err(Unsupported)` (fail-closed).
- **`llm-adapters/src/transport.rs`** — one `ureq` (synchronous) OpenAI-compat transport. **Real error
  handling:** 404→`Unsupported`, 5xx→`Unavailable`, other status→`BadRequest`, connection-refused/
  timeout/TLS→`Unavailable`; 120 s chat deadline, 10 s health deadline. Never fabricates a mock.
- **`llm-adapters/src/dispatch.rs`** — `Dispatcher` bounds concurrency with a `TokenBucket` (no tokio):
  budget-exhausted → `Err(DispatchError::BudgetExceeded)` (**degrade-closed**, never silent-queue).
  Emits an H1 harvest row (`track_record.jsonl`) per call, schema-superset of what
  `tools/telemetry/governance.sh::gov_route` consumes (closes the local-vs-managed EV loop).
- **`llm-adapters/src/compose.rs`** — the real composition `Dispatcher<CachingBackend<OllamaAdapter>>`;
  `agent-adapters` builds on the same shape.

**Fallback / unavailability handling: present and typed.** A down/slow/wrong backend surfaces as a
typed `Err` (`Unavailable`/`Timeout`), the dispatcher degrades closed on budget, and `caps()`/`health()`
are fail-closed. `rerank` unsupported is a first-class handled path (caller falls back to cosine).

**Is it exercised by tests?**
- Unit tests in `dispatch.rs` (fake backend, budget-refusal + within-budget + shared-Arc) — no network.
- **`llm-adapters/tests/ollama_roundtrip.rs`** — a **real, non-mocked** integration test hitting the live
  daemon: chat PONG roundtrip, 768-dim embed, rerank-unsupported-fail-closed.

**Caveat (honest): this is library + test code, not yet a product runtime call-site.** No `[[bin]]`/`main()`
consumes the stack; consumers today are the telemetry fold, `agent-adapters`, and `kernel/src/evals.rs` /
`leak_gate.rs` (which name `OllamaAdapter::embed` as the live bridge). It is wired *as a library seam*,
exercised by tests against the live daemon — matching the "Wave 0+1+consumer-wiring DONE" memory note,
but it is not on a shipping product hot path.

**Live build/test result:** _see §6 (run against the live Ollama daemon in this worktree)._

---

## 4. Harness / memory-system health (the system running this initiative)

**`MEMORY.md` size:** 111 lines. Its header claims "Index capped … one-line-per-entry (read-limited)."
**Claim holds** — 111 lines is genuinely compact for the index; detail is correctly pushed into linked
topic files + ATTIC. No runaway growth.

**Broken-link check (72 markdown links in `MEMORY.md`):** **71 resolve, 1 broken.**
- **BROKEN:** `../../../../dowiz/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-2026-07-11.md`
  → `/root/dowiz/docs/design/…` does not exist. Only the **`-v3-`** variant
  (`UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`) exists; the v1 was superseded (memory itself
  elsewhere notes "supersedes v1/v2"). Stale link — should be repointed to the v3 doc or the bebop-repo copy.

**Minor:** `memory/core`, `memory/corpus`, `memory/decisions` subdirs exist but are **empty** (scaffold only).

**Hook / config health (this worktree's `.claude/`):**
- `.claude/settings.json` has **no `hooks` key** — consistent with the operator's 2026-07-15 "all hooks
  emptied / no-op pass-through" directive. The 7 scripts in `.claude/hooks/` are all 321-byte no-op stubs.
- **`repowise-augment` is not installed** (`which repowise` / `which repowise-augment` → both empty; the
  binary is absent from PATH). No `repowise-augment` reference exists in `.claude/` in this worktree, so
  any `repowise-augment: not found` error observed at runtime comes from a hook/config **outside** this
  checkout (e.g. a user/global-scope hook) invoking a binary that isn't on this host. Harmless if the hook
  is fail-open, but it is a real broken invocation worth removing at its source.

---

## 5. Recommendation on further Mistral-specific investment

**Do NOT pull Mixtral 8×7B to re-run the local-inference benchmark. Not justified.** Reasons, grounded
in this host's measured numbers:

1. **The bottleneck is memory bandwidth, not FLOPs — and MoE only saves FLOPs.** The mesh-masterwork
   benchmark measured `llama3.1:8b` at **~9.2–10.0 tok/s single-stream, flat across 1/2/4 concurrent**
   (9.21 / 9.36 / 9.80 tok/s), because at ~10 tok/s the CPU host already moves ~49 GB/s — at/near this
   EPYC config's bandwidth ceiling. Mixtral's MoE trick (activate 2-of-8 experts) reduces *compute* per
   token but its **~13B active params per token still stream from RAM**, so on a bandwidth-bound CPU host
   Mixtral would be **slower per token than the 8B dense model**, not faster. The architecture the
   brainstorm praised does not help the metric that actually limits this host.
2. **Fit is marginal and hostile.** Mixtral 8×7B Q4 ≈ 26–28 GB; this host has 30 GB total / ~26 GB free.
   Loading it would consume essentially all RAM (the ollama service already peaked at 19.5 G), risking OOM
   against everything else, for a model that is architecturally *worse* on this workload.
3. **The repo already reached the same conclusion twice, on evidence.** doc21 measured that local
   inference "does not parallelize the way network latency does" and can't beat remote-API latency
   regardless of model; the masterwork already **reframed** Mixtral away from a runtime model into
   *build-time domain-expert oracles* (compilable → ns-runtime DecisionUnit gossip) — which needs **no
   Mixtral download at all**. `mistral.rs` (the server) was likewise explicitly rejected as duplicating
   the vetted Ollama daemon.

**Bottom line:** Mistral/Mixtral investment is a **dead end for local runtime inference on this host.**
The only re-open condition already written into the research stands: revisit *only* if a measured table
shows Ollama's coarse knobs cost >30% aggregate throughput vs tuned alternatives — and even then the fix
is tuning/`llama-server`, not an MoE model. Keep the existing dense-model + typed-fallback stack; spend
effort on the genuine open gap (the answer-cache / build-time-compile path), not on a new model pull.

---

## 6. Live build + test result (this worktree, vs the running daemon)

`cargo test --tests` in `llm-adapters/` — **PASS, exit 0**, compiled clean in 13.9 s:

- Unit tests (`dispatch.rs` fake-backend budget/degrade-closed etc.): **12 passed, 0 failed.**
- **Integration `ollama_roundtrip` against the LIVE daemon: 3 passed, 0 failed** — real chat roundtrip,
  768-dim embed, and rerank-fail-closed all green against `127.0.0.1:11434`.

So the agentic LLM stack **builds and its real (non-mocked) roundtrip against the live local model
works today.** Health = good at the library+test layer; the only gap is the absence of a product
runtime call-site (§3 caveat).

---

## Appendix — commands run (all read-only)
```
which ollama; ollama list; systemctl status ollama; ps aux | grep ollama; free -g
grep -rIni 'mistral|mixtral' <worktree>            # code/config → 0 ; all files → 8 docs/skills
grep -rIln 'LlmBackend|OllamaAdapter|TaskClass'    # located kernel/ + llm-adapters/ sources
python3 <link-resolution over MEMORY.md>           # 71/72 ok, 1 broken
cargo test --tests   (llm-adapters, vs live daemon)  # see §6
```
