# BLUEPRINT P41 — Three-mode operation: no-AI / local-offline / connected (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). Deepens the roadmap-index DoD for **P41** in
> `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.4 (lines 1002-1013)
> to the standard's depth. Structure/depth template: `BLUEPRINT-P-A-kernel-primitives.md`.
> Sibling: `BLUEPRINT-P40-agent-loop-tool-wiring.md` — P41 proves properties OVER P40's loop;
> it builds no second loop. Backend conventions come verbatim from
> `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md` (§2.2's one-transport-many-Quirks
> split is the swappable-backend half, already shipped; the degradation contract is the half
> this blueprint designs).

**The spine of this phase — the binding invariant, quoted verbatim from the master roadmap
§10.3 item 1 (lines 588-594):**

> **Three-mode operation (no-AI / local-offline-AI / connected-AI).** DELIVERY's core
> order/courier/money flow NEVER requires AI to function — already true by construction (CORE's
> decide/fold Law is pure deterministic Rust; LLM is "a feeling at the edge," never in the
> decision path). If every AGENT phase were deleted tomorrow, orders would still place, couriers
> would still match (deterministic HRW), money would still settle. P41 is the enforcement phase;
> the invariant binds DELIVERY and CORE too — they must never introduce an AI dependency in the
> critical order/money path.

P41's whole job is to convert that "already true by construction" into a **regression-proof,
CI-enforced, falsifiable property** — and to prove modes 2 and 3 differ ONLY in configuration.
Mode 1 therefore requires writing **zero new product code**; finding yourself writing any is a
design smell to reject in review (§1).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree `/root/dowiz`, branch `main` (`f9b2eb9bb`), 2026-07-18.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| `LlmError::Unavailable` exists, typed, fail-closed ("Backend process/endpoint is not reachable (health failed)") | `kernel/src/ports/llm.rs:140-150` (`Unavailable` at `:142-143`) | verified — the degradation primitive is SHIPPED; the missing half is the contract around it |
| Kernel build graph is AI-free by construction: no HTTP client, serde only behind `wasm`/`pq` features, tokio only behind `pgrust` | `kernel/Cargo.toml:17-42` (feature gates) + `kernel/src/ports/llm.rs:3-7` (firewall header) | verified — mode 1 is real today; what's missing is the ENFORCEMENT |
| Engine is offline-clean by mandate: "the GPU/particle `engine` crate is intentionally NOT the consumer — it is offline-clean (kernel-only, no network)" | `llm-adapters/src/compose.rs:18-19` (doc comment) | verified |
| Backend swappability shipped as CODE-side config only: `StackBuilder { base, workers, capacity, refill_rate, cache }`, defaults Ollama local | `llm-adapters/src/compose.rs:75-95` | verified |
| **No environment/file-based mode selection exists anywhere**: `grep -rn "LLM_BACKEND\|LLM_BASE_URL" --include="*.rs"` → **0 hits** | — | verified — HARNESS doc §2.2's `LLM_BACKEND=`/`LLM_BASE_URL=` EnvFile convention is DOCUMENTED but UNIMPLEMENTED; §3.2 builds it |
| `Quirks::managed_api(api_key)` exists (bearer-auth managed-API profile, Tier-0) | `llm-adapters/src/quirks.rs:69-70` | verified — the connected mode's Quirks half is shipped |
| **No `ManagedApiAdapter` struct exists**: `llm-adapters/src/` = `{cache,compose,dispatch,lib,ollama,quirks,telemetry,transport}.rs`; only `ollama.rs` implements `LlmBackend` | directory listing + grep, this pass | verified — connected mode needs one small adapter struct (§3.3), NOT a second transport (transport + quirks are shipped and shared) |
| CI has an unconditional offline kernel+engine test job; **no `cargo tree` firewall job exists**: `grep -n "cargo tree" .github/workflows/*.yml` → 0 hits | `.github/workflows/ci.yml:107-120` (the test job) | verified — DoD-1's CI gate is net-new; the underlying property it locks is already true |
| Offline-proof anchor: full order→delivery flow with ZERO peers, in-process | `/root/bebop-repo/bebop2/delivery-domain/src/intake.rs:408` (`ac6_solo_island_full_flow_no_peers`) | verified — mode 1 is "that plus no-AI" (§10.5.4 DoD-1's own consistency anchor); mode 2's isolation proof (§3.5) follows the same spirit with the network leg made physical |
| Deterministic HRW matcher = sole courier-assignment authority (the anti-scope's protected surface) | §10.3 item 1 (quoted above) + §10.5.4 P41 anti-scope ("the deterministic HRW matcher remains the sole courier-assignment authority in every mode") | roadmap-level invariant, restated as a check in §4.1 |
| P40's loop surface this blueprint consumes: `LoopOutcome::{Answer, AssistantUnavailable, ToolCallingUnsupported, IterationCapExceeded}`; e2e test `agent_reads_order_status_end_to_end` | `BLUEPRINT-P40-agent-loop-tool-wiring.md` §2/§3.4 (sibling blueprint, same directory, this pass) | design-time cite — P41 execution starts after P40's T5-T7 land; DoD-1 (§3.1) is executable TODAY, before P40, per §10.5.4 |
| Ollama daemon live on `127.0.0.1:11434`, v0.30.9; `OLLAMA_MODELS` store on local disk | `HARNESS-LLM-BACKEND.md` §1.2 (live-probed there) | inherited host facts — re-probe in T-execution |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P41 owns vs what it must NOT do

**P41 owns (build items §3):**

| Item | Content |
|---|---|
| C-a | **No-AI proof, CI-enforced**: a `no-ai-firewall` CI job asserting zero llm/agent crates in the CORE (kernel) and engine build graphs, with a committed red-proof; landable TODAY, before P40 |
| C-b | The backend-selection config type: `AiMode` + `BackendConfig::from_env` — mode is chosen by explicit environment configuration, default **Off**, fail-closed on partial config |
| C-c | `ManagedApiAdapter` (one small struct over the SHIPPED transport + `Quirks::managed_api`) so connected mode is a real selectable backend, not a doc claim |
| C-d | **Mode-parity proof**: P40's single-tool e2e test passes with the backend swapped by configuration only — zero source diff in loop or tool port |
| C-e | **Graceful-degradation contract**: Ollama stopped + no network ⇒ typed `AssistantUnavailable`; order/courier flow provably unaffected, in the same test process |
| C-f | **Local-offline proof**: the mode-2 run passes fully network-isolated (network-namespace script), consistent with the solo-island guarantee |
| C-g | **BYO-AI subscription (operator directive 2026-07-18)**: the owner connects THEIR OWN AI subscription — any OpenAI-compatible endpoint + owner-supplied key — through the SAME `ManagedApiAdapter`/`Quirks::managed_api` path as C-c (zero new transport, zero new adapter); plus the default-preset invariant: a fresh venue's WRITTEN preset = mode 2 local Ollama, BYO is the opt-in upgrade path (§3.6) |

**P41 explicitly does NOT do (anti-scope, each a review-rejectable smell):**

1. **NOT writing new product code for mode 1.** Mode 1 already works by construction (§10.3
   quote above; `kernel/Cargo.toml` ground truth §0). C-a adds a CI CHECK and nothing else. Any
   PR claiming "mode-1 support code" is a design smell by definition — reject it in review; the
   correct diff for mode 1 is a test/CI diff only.
2. **NOT a second tool-loop implementation for remote backends.** One `AgentLoop`, one
   `ToolPort`, one `OpenAiCompatTransport`; modes 2 and 3 differ ONLY in which `LlmBackend`
   impl the config selects (`OllamaAdapter` vs `ManagedApiAdapter`). A `#[cfg]`, a
   backend-conditional branch inside the loop, or a "RemoteAgentLoop" is the failure mode this
   phase exists to make impossible — C-d's zero-source-diff assertion is the gate.
3. **NOT auto-escalation from local to connected.** There is no code path that constructs
   `AiMode::Connected` from a failure of local — `from_env` is the ONLY constructor of the mode
   value, and it reads explicit configuration (§4.1's reachability argument). Local backend
   down ⇒ typed unavailable outcome, full stop. Escalation is an operator config change,
   never a fallback.
4. **NOT letting AI touch courier assignment.** The deterministic HRW matcher remains the sole
   courier-assignment authority in EVERY mode; any model output about routing is advisory text
   at most. Structural backstop: P40's `ToolAction` has only `Read`, and no matcher symbol is
   reachable from the loop's namespace (P40 §4.1) — this is inherited, cited, and re-checked
   here (§4.1), not re-implemented.
5. **NOT MCP, NOT tool-catalog growth, NOT streaming** — P42's lane and beyond.
6. **Out of scope, flagged once for awareness — self-mod effector (bebop-repo):**
   `bebop2/core/src/self_mod.rs` + `self_mod_loop.rs` — a code-self-modification actuator
   (dormant; its "operator-authorized" header is self-asserted, not independently
   corroborated), not a delivery-ops assistant; no P41 mode governs it and no P41 code touches
   it.
7. **NOT touching `AgentBridge`/`agent-adapters`** (PROTOCOL's mesh foreign-agent admission,
   `kernel/src/ports/agent/` + `agent-adapters/` — see P40 §4.4's non-conflation check, which
   P41's close-out re-runs).
8. **NOT billing/subscription management for BYO-AI (C-g).** Scope is "how the owner's own
   endpoint+key get configured and used" — payment/plan management for that subscription is the
   provider's problem (and commerce is P47's adjacent lane, not entered). P41 stores a base URL
   and a key-file path, nothing else.
9. **NOT a vendor list (C-g).** "OpenAI-compatible API" is the ONE generalized target —
   `OpenAiCompatTransport` is provider-agnostic by design (HARNESS §2.2); per-vendor adapters or
   provider enumerations are review-rejectable. Provider-side MCP as a connection method is
   flagged for P42's lane (which inherits this three-mode contract), not designed here.

**Dependency posture (from §10.5.4, restated precisely):** C-a is provable TODAY, before P40 —
land it FIRST as the locked baseline (the roadmap's own instruction). C-b/C-c are buildable in
parallel with P40 (different files). C-d/C-e/C-f depend on P40's loop + e2e test existing.
Independent of PROTOCOL P34/P35 by construction (offline-first). Blocks P42 (the MCP surface
must inherit exactly this three-mode contract).

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── llm-adapters/src/compose.rs extension (C-b). Config lives in the adapter
// crate — the kernel NEVER reads environment variables (firewall: the kernel
// doesn't even know modes exist; it sees &dyn LlmBackend or nothing). ─────────

/// The three operating modes. This enum is the operator's three-mode directive
/// AS A TYPE. Constructed ONLY by `BackendConfig::from_env` — there is no other
/// constructor call site, which is what makes silent escalation unrepresentable
/// (§4.1). Default: Off — the fail-closed mode: no model call, no data egress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiMode {
    /// Mode 1 — no AI. The agent surface is absent; the assistant entry point
    /// returns AssistantDisabled without constructing any backend.
    Off,
    /// Mode 2 — local offline. OllamaAdapter at a loopback URL. Never leaves
    /// the node.
    LocalOffline,
    /// Mode 3 — connected. ManagedApiAdapter at an explicit remote URL with an
    /// explicit key. Both must be present or from_env refuses (never a partial
    /// fallback).
    Connected,
}

/// Environment contract (documented here, the single authority):
///   DOWIZ_AI_MODE = "off" (default when unset) | "local" | "connected"
///   DOWIZ_LLM_BASE_URL  — Connected only; required there, refused elsewhere-set-invalid? No:
///                          ignored in Off/Local (Local pins loopback; a non-loopback
///                          base in Local mode is a ConfigError — see invariant below).
///   DOWIZ_LLM_API_KEY_FILE — Connected only; path to key file (never the key
///                          itself in env — process-listing hygiene).
/// INVARIANT (fail-closed, tested §3.2): LocalOffline's base URL must resolve
/// to a loopback host. A non-loopback URL under mode=local is a typed
/// ConfigError — "local" that talks to the network is a lie the type refuses.
#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub mode: AiMode,
    pub base_url: String,        // loopback (Local) or explicit remote (Connected)
    pub api_key: Option<String>, // Connected only, loaded from the key file
}

/// Typed configuration failure. Surfaced at composition time — never a panic,
/// never a silent default-to-something-else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    UnknownMode(String),          // DOWIZ_AI_MODE set to junk
    MissingBaseUrl,               // connected without DOWIZ_LLM_BASE_URL
    MissingApiKey,                // connected without a readable key file
    NonLoopbackLocal(String),     // mode=local with a non-loopback base URL
}

impl BackendConfig {
    /// THE mode constructor. Unset DOWIZ_AI_MODE ⇒ Off (fail-closed default).
    /// Partial connected config ⇒ Err — never "fall back to local", never
    /// "try the default remote" (both would be silent mode changes).
    pub fn from_env() -> Result<BackendConfig, ConfigError>;
}

/// The agent surface's mode-1/absent answer — distinct from Unavailable
/// (which means "configured but unreachable"). Off is a CHOICE, not a failure.
/// Lives beside LoopOutcome consumption in agent-facade's composition fn:
pub enum AssistantEntry {
    Disabled,                     // AiMode::Off — no backend was ever constructed
    Ready(/* composed stack */),  // Local or Connected, health not yet probed
}

// ── llm-adapters/src/managed.rs — NEW, small (C-c) ─────────────────────────────
/// The Tier-0 managed/remote adapter the HARNESS doc's table already names.
/// ~a screenful: OpenAiCompatTransport::new(base_url, Quirks::managed_api(key)),
/// route_model = pass-through (remote decides), caps() probed fail-closed like
/// Ollama's (tool_calling from a live probe where the endpoint offers one;
/// otherwise false — never assumed). NO new transport, NO new wire code.
pub struct ManagedApiAdapter { /* transport: OpenAiCompatTransport */ }
// impl LlmBackend for ManagedApiAdapter — the crate's SECOND impl, proving the
// port's "&dyn LlmBackend behind a config-selected constructor" promise
// (kernel/src/ports/llm.rs:9-11) with real code for the first time.

// ── tools/ci/offline-proof.sh — NEW script (C-f), spec'd in §3.5 ───────────────
// ── .github/workflows/ci.yml — NEW job `no-ai-firewall` (C-a), spec'd in §3.1 ──
```

**Rejected alternatives (DECART-style, one line each):** a config FILE (`ai-mode.toml`) —
rejected for P41 because no config-file convention exists in the Rust crates yet (grep §0) and
inventing one for three values is machinery ahead of need; env vars match the repo's existing
EnvFile/systemd deployment shape (HARNESS §2.2 names exactly this) and a later HubPolicy file
can wrap `from_env` without changing consumers. Auto-detect ("use local if Ollama answers") —
rejected as the definition of silent mode selection; the operator directive requires modes to
be explicit. Putting `AiMode` in the kernel — rejected: the kernel must not know modes exist
(the §10.3 invariant is that the kernel works with AI *absent*; a mode enum in the kernel would
be an AI concept in the no-AI core).

---

## 3. Build items — spec → RED test → code, adversarial cases (items 3, 5)

### 3.1 C-a — No-AI proof, CI-enforced (landable TODAY, before P40)

**The property is already true** (§0: kernel/engine graphs are AI-free). C-a makes it
**stay** true mechanically. New CI job in `.github/workflows/ci.yml` (same offline discipline
as the existing test job, `ci.yml:107-120`):

```yaml
no-ai-firewall:
  name: no-AI firewall (mode-1 invariant, §10.3 item 1)
  # cargo tree over the CORE and engine graphs must show zero AI/agent/HTTP crates.
  # This is the blueprint's existing firewall done-check (HARNESS §6 WAVE-0)
  # promoted to a mode-1 invariant gate.
  steps:
    - run: |
        set -e
        for m in kernel engine; do
          if cargo tree --offline --manifest-path $m/Cargo.toml \
            | grep -Ei "llm-adapters|agent-loop|agent-facade|ureq|reqwest"; then
            echo "FIREWALL BREACH: AI/HTTP crate in $m build graph"; exit 1
          fi
        done
```

RED-proof (committed evidence, the §10.3-item-5 discipline): on a scratch branch, add
`llm-adapters = { path = "../llm-adapters" }` to `kernel/Cargo.toml`, run the job's script
locally, paste the `FIREWALL BREACH` output into the landing commit's message, revert the
scratch edit. The gate must be seen to bite before it is trusted.

**Consistency anchor (from §10.5.4 DoD-1, honored not restated):** PROTOCOL's
`ac6_solo_island_full_flow_no_peers` (`intake.rs:408`) already proves the full delivery flow
with zero peers; mode 1 is that plus no-AI, and that test **must stay green untouched** — C-a's
close-out re-runs it in bebop-repo and asserts no diff to it landed in this phase.

**Adversarial:** the grep list is itself tested against under-matching: a second scratch-branch
proof adds `agent-loop` (not just `llm-adapters`) to `engine/Cargo.toml` and confirms the job
fails on the ENGINE leg too — both graphs, both crate families, both proven, not assumed. Also
the feature-gate trap: run the same tree check with `--features pgrust` and `--features wasm`
and record the result — tokio/serde legitimately appear there (feature-gated, `Cargo.toml:24,35`);
the job pins the DEFAULT (canonical order/money) build only, and this scoping decision is
written into the job's comment so a future reader doesn't "fix" it into a false positive.

### 3.2 C-b — the backend-selection config type (mode = explicit configuration)

`BackendConfig::from_env` (§2) + `StackBuilder::from_config(cfg)` wiring
(`compose.rs:75-95`'s builder gains one constructor; existing builder methods untouched).
Composition behavior per mode: `Off` ⇒ `AssistantEntry::Disabled` — **no backend is
constructed at all** (not "constructed then unused"); `LocalOffline` ⇒
`OllamaAdapter::new(base)` with the loopback invariant checked; `Connected` ⇒
`ManagedApiAdapter::new(base, key)`.

RED→GREEN table tests (pure, no daemon, no network — env-var permutation matrix):

```rust
// llm-adapters/src/compose.rs tests mod:
#[test] fn unset_mode_is_off() { /* no DOWIZ_AI_MODE ⇒ Off — fail-closed default */ }
#[test] fn junk_mode_is_typed_error() { /* "turbo" ⇒ ConfigError::UnknownMode */ }
#[test] fn connected_without_base_url_refused() { /* ⇒ MissingBaseUrl, NEVER local fallback */ }
#[test] fn connected_without_key_refused() { /* ⇒ MissingApiKey */ }
#[test] fn local_with_remote_url_refused() {
    // DOWIZ_AI_MODE=local + DOWIZ_LLM_BASE_URL=https://api.example.com
    // ⇒ ConfigError::NonLoopbackLocal — "local" that egresses is refused by type.
}
```

**Adversarial (the anti-escalation teeth):** a test proving no-auto-escalation structurally —
`grep -rn "AiMode::Connected" llm-adapters/src agent-facade/src agent-loop/src` must show the
variant constructed in exactly ONE non-test location (`from_env`'s parse arm). This is a
grep-CI-gate-shaped guard (ledger vocabulary) turning "silent escalation" into a structural
impossibility: to escalate, code must construct `Connected`, and there is one audited place
that can. Committed as a `#[test]`-wrapped process check beside the firewall tests.

### 3.3 C-c — `ManagedApiAdapter` (the connected mode becomes real, minimally)

One struct (§2), reusing the SHIPPED `OpenAiCompatTransport` + `Quirks::managed_api`
(`quirks.rs:69-70`) — zero new wire code, zero new deps (standard item 19; this is the
"one transport, three adapters" table of HARNESS §2.2 getting its second row implemented).
`caps()` fail-closed like Ollama's (P40 §3.2 discipline): tool-calling is probed where the
endpoint exposes model metadata, otherwise `false` — never assumed `true` because the vendor
is big.

RED→GREEN: a transport-double test (scripted OpenAI-envelope responses) proving
request/response mapping and bearer-header injection; a live smoke test against the
operator's endpoint exists but is env-gated (below). **Adversarial:** a 401-response double ⇒
typed `LlmError` (BadRequest carrying the status), never a retry loop; a key file that doesn't
exist ⇒ `ConfigError::MissingApiKey` at composition, the adapter is never constructed.

### 3.4 C-d — mode-parity proof (one loop, swapped by config only)

**The claim:** P40's `agent_reads_order_status_end_to_end` passes under mode 2 and mode 3 with
**zero source diff** in `agent-loop/`, `agent-facade/`, and `kernel/src/ports/` between the
runs — the backend swap is entirely `DOWIZ_AI_MODE`/`DOWIZ_LLM_BASE_URL` environment.

Mechanics: the e2e test gains a sibling `mode_parity.rs` that composes the stack via
`BackendConfig::from_env()` instead of a hard-coded builder, then runs the IDENTICAL assertion
body (shared fn in the test support mod — the test body is literally one function called
twice-by-config, so "zero loop-code diff" is enforced by there being no per-mode code to
diff). Parity is a **behavioral contract, stated precisely, not byte-equality**: both runs
must produce (i) the event subsequence `ToolCallParsed{read_order_status, ord-7} →
ToolResult`, (ii) outcome `Answer` containing `IN_DELIVERY`. Answer prose may differ between
models; the tool-use behavior may not.

**The connected leg's honesty protocol (named precedent, not a silent skip):** the mode-3 run
requires an operator-supplied OpenAI-compat endpoint with tool-calling. The test is written,
named `mode_parity_connected`, and `#[ignore]`d with a doc comment stating the exact
activation condition (`DOWIZ_AI_MODE=connected` + endpoint + key present) — the SAME accepted
honesty convention as P-A's cross-arch CORDIC seam (`BLUEPRINT-P-A` §3.6/T8: "an explicitly-
open checklist line, not silently claimed"). The mode-2 leg runs unconditionally. P41's DoD
row for C-d is GREEN-local + declared-open-connected until the operator supplies the endpoint;
claiming full C-d closure without a real connected run is forbidden.

**Adversarial:** the anti-parity mutation — a scratch-branch proof inserting
`if mode == Connected { /* different prompt */ }` into the loop must be caught: the §3.2 grep
guard fires (a second `AiMode` reference inside `agent-loop/` — the loop crate must contain
ZERO mode references, checked by `grep -rn "AiMode" agent-loop/src` → empty, added to P40's
firewall test file). The loop cannot even SEE the mode — parity by blindness, not by
discipline.

### 3.5 C-e + C-f — graceful degradation and the network-isolated proof

**C-e (degradation contract):** with Ollama stopped AND no network:

```rust
// agent-facade/tests/degradation.rs:
#[test] fn assistant_down_orders_still_flow() {
    // Precondition choreography (documented in the test header, HARNESS §6 style):
    //   systemctl stop ollama
    // 1. Compose mode-2 stack via from_env → AssistantEntry::Ready.
    // 2. loop.run("status of ord-7?") → LoopOutcome::AssistantUnavailable
    //    (typed — from health() Err(Unavailable), llm.rs:142-143). Assert the
    //    time-to-refusal < DEGRADE_DEADLINE_MS (§6 budget) — never a hang.
    // 3. IN THE SAME TEST PROCESS: drive the kernel order flow — place → confirm
    //    → … → Delivered via decide/fold — and assert the full event sequence.
    //    This is the "order flow provably unaffected" leg made literal: same
    //    process, assistant dead, Law green.
}
```

The compile-graph argument does the heavy lifting (the kernel cannot be affected by an absent
daemon it never talks to — C-a's enforced property); step 3 demonstrates it at runtime anyway,
because "provably unaffected" should be shown in one place a reviewer can run, not only argued.

**Adversarial (C-e):** a half-dead daemon double — a listener that accepts TCP and then stalls
(never sends bytes) — must yield `LlmError::Timeout` within the transport deadline, surfacing
as `AssistantUnavailable`, never a hang (this is the nastier real-world failure than
connection-refused; connection-refused is the easy case).

**C-f (local-offline proof, network-isolated):** `tools/ci/offline-proof.sh` — the mode-2 e2e
run inside a **network namespace with only loopback**, making non-loopback egress
*kernel-impossible* rather than merely unobserved (the same spirit as ac6's zero-peers proof,
with the network leg physical):

```sh
#!/usr/bin/env sh
# offline-proof.sh — P41 DoD-4. Requires root (netns). Exit 0 = proof holds.
set -e
unshare -n sh -c '
  ip link set lo up                       # loopback only; no other iface exists here
  OLLAMA_HOST=127.0.0.1:11500 ollama serve &   # fresh daemon INSIDE the netns,
  SERVE_PID=$!                            # model store shared read-only from disk
  for i in $(seq 1 60); do curl -sf 127.0.0.1:11500/v1/models && break; sleep 1; done
  DOWIZ_AI_MODE=local DOWIZ_LLM_BASE_URL=http://127.0.0.1:11500 \
    cargo test --offline --manifest-path agent-loop/Cargo.toml \
    --test e2e_read_order_status -- --nocapture
  kill $SERVE_PID
'
```

Inside the namespace there is no route to any remote endpoint — if the mode-2 path secretly
needed one, the test FAILS. `cargo --offline` closes the crates.io leg too. Fallback if the
in-netns daemon is too heavy for a CI runner (model load ≈ seconds + RAM): a scoped nftables
egress-drop of non-loopback traffic around the live-daemon test — documented as the weaker
substitute (host-state mutation risk), the netns form is canonical.

**Adversarial (C-f):** the proof must be able to fail — run the SAME script with
`DOWIZ_AI_MODE=connected DOWIZ_LLM_BASE_URL=https://api.openai.com` and assert the run
produces `AssistantUnavailable`/typed connect failure, NOT a pass — demonstrating the
namespace actually severs egress (the isolation has teeth; a proof that cannot fail proves
nothing — `verified-by-math` discipline).

### 3.6 C-g — BYO-AI subscription: owner's own endpoint + key, default preset local (operator directive 2026-07-18, appended)

**The directive (verbatim intent):** open-source third-party services AND adapters for the most
popular paid ones are permitted — e.g. API/MCP/direct-connect with the client's OWN AI
subscription; convenient settings so the owner can change what fits vs what doesn't; but WITH a
ready DEFAULT PRESET that includes the local agent.

**Confirmed: the mechanism already exists — cite, don't re-derive.** "Owner brings their own
subscription" is EXACTLY mode 3 as already designed: `AiMode::Connected` +
`DOWIZ_LLM_BASE_URL=<the provider's OpenAI-compat endpoint>` +
`DOWIZ_LLM_API_KEY_FILE=<owner's key>` through the SHIPPED `OpenAiCompatTransport` +
`Quirks::managed_api` (HARNESS §2.2's one-transport-many-Quirks split; C-c's
`ManagedApiAdapter` is the one struct still to land, §3.3). Any provider exposing an
OpenAI-compatible API is thereby supported with zero new architecture — that is the generalized
target, deliberately not a vendor list (§1 anti-scope 9).

**What C-g actually adds (small, configuration-provenance only):**

1. **A named sub-distinction under Connected — NOT a fourth enum variant.** `Connected` splits
   at the CONFIG level into `connected/managed-default` (a dowiz-operated default endpoint,
   if/when one exists) vs `connected/byo` (owner-supplied endpoint + key). Same
   `AiMode::Connected`, same `ManagedApiAdapter`, same code path — the distinction is WHO
   supplies `base_url`+key, recorded as `DOWIZ_LLM_PROVENANCE = "managed" | "byo"` in the §2
   env-contract doc so telemetry/H1 rows attribute cost to the right party. A provenance branch
   inside the loop or adapter is the same §1-item-2 smell — provenance is attribution metadata,
   never behavior.
2. **The default-preset invariant, reconciled with fail-closed — two layers, both kept.**
   `from_env`'s CODE default stays `Off` (unset env ⇒ no AI, §2 — never weakened). The PRODUCT
   default is a provisioning-time preset: a fresh venue install WRITES an explicit
   `DOWIZ_AI_MODE=local` EnvFile — so zero-OWNER-config = local Ollama agent, while the mode
   remains explicit written configuration (the preset file IS the explicit config; no silent
   default enters the code). BYO is the opt-in upgrade path FROM that preset — local-first is
   the zero-config default, never the other way around.
3. **The settings surface lives in P48's owner hub — cross-reference ONLY.** The owner-facing
   "connect your own AI" settings (endpoint, key file, local/connected toggle) belong to P48's
   owner/admin operational surface (master roadmap §11 / §10.5.x P48 — the owner's management
   view, per its 2026-07-18 hub ruling). P48 is specified in its own parallel lane; this
   blueprint deliberately designs NO UI and touches NO P48 section. The whole contract is one
   sentence: P48's settings write the SAME env/EnvFile configuration that `from_env` (or the
   future `from_policy(hub)`, §4.2) reads — one writer, one reader, no second config channel.
4. **MCP as a connection method: deferred to P42, flagged not designed.** Where a provider
   exposes MCP rather than an OpenAI-compat API, that connection rides P42's MCP lane (which
   already must inherit this three-mode contract); designing it here would front-run P42.

RED→GREEN (config-level, extends the §3.2 matrix): `byo_connected_is_plain_connected` — env set
to a BYO endpoint+key composes the IDENTICAL `ManagedApiAdapter` stack as the managed-default
case (same type, same quirks profile — provenance changes nothing); and
`preset_file_is_explicit_local` — the provisioning preset parses to `LocalOffline` through the
normal `from_env` path, no special-casing. **Adversarial:** the provenance-neutrality grep
guard — `DOWIZ_LLM_PROVENANCE` read in exactly ONE place (telemetry attribution), zero hits in
`agent-loop/` or adapter dispatch; a second read site is a behavior fork and fails the guard.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

The hazards of THIS phase are mode-boundary hazards; each is argued from structure:

- **Silent data egress (the worst case):** order text/prompts leaving the node without
  operator intent. Reachability: egress requires a non-loopback base URL in a constructed
  backend; a backend is constructed only from a `BackendConfig`; `Connected` is constructible
  only in `from_env`'s parse arm from an explicit `DOWIZ_AI_MODE=connected` (§3.2's
  one-constructor grep guard), and `LocalOffline` with a non-loopback URL is a typed refusal
  (`ConfigError::NonLoopbackLocal`). Therefore egress without explicit connected-mode config
  is unreachable through the composition layer. Residual, named: a hostile EDIT to `from_env`
  itself — caught by the grep guard + review, and by C-f's namespace proof for the mode-2
  path (an egress attempt inside the netns fails hard).
- **AI in the decision path (the invariant's own failure mode):** requires the kernel/engine
  build graph to gain an AI crate (C-a's CI gate goes red) or a runtime call edge from
  decide/fold into the assistant — which has no representation: the kernel cannot name any
  assistant symbol (it has no such dependency, enforced). The HRW matcher's sole authority is
  the same argument one level up: no matcher symbol is importable in `agent-loop`'s namespace
  (P40 §4.1, re-checked at P41 close-out).
- **Mode confusion:** every ambiguous configuration is a typed `ConfigError`; the only silent
  default is `Off` — the mode in which nothing runs, no data moves, and the §10.3 invariant is
  vacuously safe. Fail-closed means: when in doubt, the assistant is absent, never remote.

### 4.2 Schemas & scaling axes (item 8)

`BackendConfig` is O(1) — three fields, read once per process at composition; no growth axis.
The env contract's scaling point, named: when nodes need per-hub multi-backend policy (many
hubs, one process — the M5 HubPolicy future), `from_env` becomes `from_policy(hub)` and the
enum survives unchanged; that trigger is "more than one hub identity per process," not before.
`offline-proof.sh` scales by model-load time (~seconds per run) — acceptable as a nightly/
release gate rather than per-commit if CI minutes demand; the decision point is recorded in
the job comment, not silently chosen.

### 4.3 Isolation / bulkhead (item 11), mesh (item 12), rollback (item 13), living memory (item 15)

- **Isolation:** the mode boundary IS the bulkhead — mode-2/3 failures terminate in
  `AssistantUnavailable` on the assistant surface; the order/money surface has no call edge
  into it (C-a) and demonstrably keeps folding while the assistant is dead (C-e step 3). The
  degradation deadline (§6) bounds how long the assistant surface can even stall its OWN
  caller.
- **Mesh (item 12):** mode selection is **node-local configuration. Not mesh-gossiped, not
  negotiated with peers, no transport dependency.** A hub's mode is its operator's business
  (M5 hub-autonomy); no protocol message carries `AiMode`.
- **Rollback (item 13, vocabulary used precisely):** P41 claims the **Self-Termination /
  unrepresentable-state leg** only: partial connected config, non-loopback "local," and
  silent escalation are refused states, not monitored ones; degradation is a typed terminal
  outcome within a deadline. Mode change is an env edit + process restart — mechanically
  reversible in seconds, no state migration (the config is stateless). No Self-Healing or
  Snapshot-Re-entry claims (nothing here has state to heal or re-enter).
- **Living memory (item 15):** N/A-honest — mode config has no temporal/topological access
  pattern; no decorative cross-reference made. The H1 harvest rows (which P40's calls emit in
  every mode) are where mode-differentiated cost/latency history accumulates for
  `gov_route`-style pricing — existing channel, nothing new.

### 4.4 Linux-discipline verdict framework (item 9)

Per `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s categories: C-a is
**ALREADY-EQUIVALENT** (the kernel's config-gated feature discipline, `Cargo.toml:17-42`,
extended by one CI assertion — "don't regress what construction already gives you"); C-b's
explicit-config-no-magic-detection is **REINFORCES** (the repo's fail-closed/no-silent-default
rule applied to deployment config); C-f's netns proof is **EXTENDS** — physical-isolation
testing is new machinery for this repo, justified because a grep cannot prove the absence of
network need, only a namespace can (item 19: the existing pattern — done-check greps — was
shown insufficient before new machinery was added).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Extends §10.5.4's four P41 DoD lines with real test/check names. P41 is DONE iff every row is
demonstrably true.

| Item | RED (fails before) | GREEN (passes after) | Named test / check (permanent, item 17) |
|---|---|---|---|
| C-a no-AI | scratch-branch red-proof: `FIREWALL BREACH` on kernel AND engine legs (output committed) | `no-ai-firewall` CI job green on main; `ac6_solo_island_full_flow_no_peers` untouched-green in bebop-repo | `.github/workflows/ci.yml` job `no-ai-firewall` |
| C-b config | 5 env-matrix tests RED (type absent) | matrix green incl. `local_with_remote_url_refused`; one-constructor grep guard green | `llm-adapters` compose tests: `{unset_mode_is_off, junk_mode_is_typed_error, connected_without_base_url_refused, connected_without_key_refused, local_with_remote_url_refused}` + `connected_single_constructor_guard` |
| C-c managed | transport-double tests RED (struct absent) | mapping + bearer + 401-typed tests green | `llm-adapters/tests/managed_adapter.rs::{envelope_mapping, bearer_header_present, http_401_is_typed_error}` |
| C-d parity | `mode_parity_local` RED before from_env wiring | mode-2 leg green with the SHARED assertion fn; `grep AiMode agent-loop/src` → empty; connected leg written + `#[ignore]`d with declared activation condition | `agent-loop/tests/mode_parity.rs::{mode_parity_local, mode_parity_connected(#[ignore], declared)}` + the `AiMode`-blindness grep in `agent-loop/tests/firewall.rs` |
| C-e degradation | `assistant_down_orders_still_flow` RED vs missing typed path | daemon-stopped run: typed `AssistantUnavailable` within deadline + full order fold green in-process; stalling-listener double yields `Timeout` not hang | `agent-facade/tests/degradation.rs::{assistant_down_orders_still_flow, stalling_backend_times_out}` |
| C-f offline | isolation-teeth run (connected-in-netns) must FAIL typed — proving the namespace severs egress | `offline-proof.sh` exit 0: mode-2 e2e fully green inside loopback-only netns | `tools/ci/offline-proof.sh` (nightly/release CI gate; per-commit decision recorded in the job comment) |
| C-g BYO | byo/preset tests RED (contract absent) | `byo_connected_is_plain_connected` + `preset_file_is_explicit_local` green; provenance single-read grep guard green; P48 settings-surface cross-ref recorded, NO UI built here | `llm-adapters` compose tests: `{byo_connected_is_plain_connected, preset_file_is_explicit_local}` + provenance-neutrality grep guard |

Ledger obligations (`docs/regressions/REGRESSION-LEDGER.md`, ratchet rule `:9-16`): one row for
the no-AI firewall ("AI crate in CORE/engine graph — guardrail: CI-gate `no-ai-firewall`,
red-proof committed") and one for the escalation guard ("silent local→connected escalation —
guardrail: grep-CI-gate `connected_single_constructor_guard`"). Both rows land with their
red→green proof BEFORE P41 is called done.

---

## 6. Benchmark plan (item 10) — budgets, then measurements, no estimates shipped as facts

1. **Mode-selection overhead:** `BackendConfig::from_env` — pure env reads + one file read
   (key). Budget: ≤ 1 ms, paid once per process. Bench `compose/from_env_local` in
   `llm-adapters/benches/criterion.rs`; number recorded in `BENCH_HISTORY.md` (RED-commit
   baseline seeding, existing `bench_track` convention).
2. **Time-to-typed-refusal (the degradation deadline):** `DEGRADE_DEADLINE_MS` — the C-e
   assertion bound. Connection-refused path (daemon stopped): budget ≤ 250 ms (loopback RST is
   immediate; the budget is generous for CI noise). Stalling-listener path: bounded by the
   transport read deadline (`LlmError::Timeout`, `llm.rs:148-149`) — MEASURE the shipped ureq
   deadline first and record it; if no explicit deadline is currently set on the transport,
   that is a real finding to fix in this phase (an unbounded read would make `Timeout`
   unreachable and C-e's stall test RED — the test finds it either way, which is the point).
3. **Mode-1 overhead: zero by construction** — no code runs, nothing to measure; recorded as
   exactly that sentence in `BENCH_HISTORY.md` rather than a fabricated number (the P-A §6
   "no measured consumer" honesty precedent).
4. **Telemetry hook:** per-mode cost/latency separation falls out of the existing H1 harvest
   rows (`TrackRecord` carries `backend_id` — "ollama" vs the managed id), aggregated by the
   shipped `telemetry.rs` fold. Zero new channels; the regression surface is the bench-gate +
   the CI jobs above.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.3 item 1 (quoted verbatim above —
the spine), item 2 (offline-first/solo-island — C-f's mandate), item 5 (firewall pattern) +
§10.5.4 P41 (the index DoD this deepens) · `BLUEPRINT-P40-agent-loop-tool-wiring.md` (the loop
this phase proves properties over; its §4.1 reachability inheritance; its firewall test file
hosts the `AiMode`-blindness grep) · `HARNESS-LLM-BACKEND.md` §2.2 (one-transport/Quirks
split = the shipped swappable-backend half; the `LLM_BACKEND=` EnvFile intent §3.2-implemented
here as `DOWIZ_*`), §5 Decision 2 (ureq discipline), §6 (done-check choreography) ·
`BLUEPRINT-P-A-kernel-primitives.md` (template; the `#[ignore]`-with-declared-condition
honesty precedent §3.6/T8 reused for the connected parity leg) ·
`ac6_solo_island_full_flow_no_peers`
(`/root/bebop-repo/bebop2/delivery-domain/src/intake.rs:408`) ·
`docs/regressions/REGRESSION-LEDGER.md` (§5 rows) ·
`BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md` (§4.4 verdicts). Memory
files: `verified-by-math-2026-07-07` (§3.5's proof-must-be-able-to-fail) ·
`never-bypass-human-gates-2026-06-29` (connected leg is operator-supplied, never
self-provisioned) · `ground-truth-over-proxy-2026-07-07` (behavioral parity defined on real
runs, not mocked ones) · `harness-llm-backend-and-hermetic-remediation-2026-07-17` (substrate
status) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style discipline). Supersedes:
nothing — additive over §10.5.4's index entry; the "small design pass needed here (a policy
note)" that §10.5.4 requested for the degradation contract IS §3.5/§4.1 of this document.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (one concept, one primitive): ONE loop, ONE transport, ONE mode
  constructor — modes are one enum consumed at one composition point, never a scattered
  `if remote` idiom (C-d's blindness grep makes the scattered form detectable).
- **P6 CAUSE-AND-EFFECT** (determinism as law): mode is a pure function of explicit
  configuration — same env, same composition, every time; no probing, no racing a daemon's
  liveness to pick a mode. The degradation path is deadline-bounded, so even failure timing
  is a stated bound, not an emergent behavior.
- **P7 GENDER** (paired creation, no self-certification): the no-AI claim is certified by CI
  running `cargo tree` — an external tool over the real build graph, not the code's own claim;
  the offline claim by the OS's namespace isolation — an authority OUTSIDE the tested process;
  the parity claim by running the same test body against two real backends, not by asserting
  the loop "should" be backend-agnostic.

(Other principles are not load-bearing here and are not claimed decoratively, per the
Anu/Ananke discipline.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites incl. 2 found gaps: env-config unimplemented, ManagedApiAdapter absent) |
| 2 DoD | §5 |
| 3 spec/event-driven TDD | §2 (types first), §3 RED-first per item; C-e asserts the order-flow EVENT sequence in-process |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.1 (under-match proof), §3.2 (escalation guard), §3.4 (anti-parity mutation), §3.5 (stalling listener; isolation-teeth run that must FAIL) |
| 6 hazard-safety as math | §4.1 (egress/decision-path/mode-confusion, each a reachability argument) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 |
| 9 Linux discipline | §4.4 |
| 10 benchmarks+telemetry | §6 (budgets + the honest possibly-missing-deadline finding) |
| 11 isolation/bulkhead | §4.3 (mode boundary as bulkhead; deadline-bounded stall) |
| 12 mesh awareness | §4.3 (node-local config, explicitly never gossiped) |
| 13 rollback/self-heal vocabulary | §4.3 (Self-Termination leg only; env-revert rollback) |
| 14 error-propagation gates | §3.1 CI gate, §3.2 grep guard, §5 named checks |
| 15 living memory | §4.3 (N/A-honest; H1 rows named as the existing channel) |
| 16 tensor/spectral + eqc reuse | N/A-honest: no closed-form math in this phase; no decorative claim |
| 17 regression ledger | §5 (two rows named with guardrail types) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §2 (3 rejected alternatives), §3.3 (transport/Quirks reuse), §4.4 (netns justified against the weaker existing pattern) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repo: `/root/dowiz`. Order matters only where stated; T1 is independent of P40 entirely — do
it first, today. T4-T6 need P40's T5-T7 (see `BLUEPRINT-P40-agent-loop-tool-wiring.md` §10)
landed.

1. **T1 (C-a — before P40, the locked baseline).** Add the `no-ai-firewall` job to
   `.github/workflows/ci.yml` (§3.1 script verbatim; reuse the offline cargo-fetch preamble
   from the existing test job at `ci.yml:107-120`, and reproduce the standing SCOPE RULE
   banner comment that every job in that file carries). Produce BOTH red-proofs (kernel leg
   via `llm-adapters` dep, engine leg via `agent-loop` dep — if `agent-loop` doesn't exist
   yet, use `llm-adapters` for both and note it), paste outputs into the commit message,
   revert scratch edits. Add the ledger row (§5). Acceptance: job green on main; red-proof
   text present in `git log -1`.
2. **T2 (C-b).** In `llm-adapters/src/compose.rs`: add `AiMode`, `BackendConfig`,
   `ConfigError`, `from_env`, `StackBuilder::from_config` (§2 verbatim, incl. the loopback
   invariant and the doc-comment env contract). Write the 5 matrix tests + the
   one-constructor grep guard (§3.2) RED-first. Env-var tests must serialize env mutation
   (`std::env` is process-global — use a test mutex, note it in the file header). Acceptance:
   `cd llm-adapters && cargo test compose` green.
3. **T3 (C-c).** Create `llm-adapters/src/managed.rs` (§2's `ManagedApiAdapter` — transport +
   `Quirks::managed_api`, pass-through routing, fail-closed caps) + `pub mod managed;` in
   `lib.rs`. Write `llm-adapters/tests/managed_adapter.rs` (3 named tests, §5 row C-c)
   against a scripted transport double. NO new deps. Acceptance:
   `cd llm-adapters && cargo test managed` green.
4. **T4 (C-d — after P40 T7).** Create `agent-loop/tests/mode_parity.rs`: extract P40's e2e
   assertion body into a shared fn (same file or a `tests/support` mod — do NOT modify the
   loop source to do this; if the extraction seems to need a loop change, stop — that is the
   §1 item-2 smell); `mode_parity_local` composes via `from_env` (set
   `DOWIZ_AI_MODE=local`), runs the shared body; `mode_parity_connected` same body,
   `#[ignore]`d, doc comment stating the activation condition (§3.4). Add
   `grep -rn "AiMode" agent-loop/src` → empty to `agent-loop/tests/firewall.rs`. Acceptance:
   `cd agent-loop && cargo test mode_parity_local` green; firewall suite green.
5. **T5 (C-e — after P40 T7).** Create `agent-facade/tests/degradation.rs` (§3.5's two
   tests). The stalling-listener double: `std::net::TcpListener` on an ephemeral loopback
   port, accept-then-sleep. FIRST measure whether the shipped transport sets a read deadline
   (§6 item 2) — if not, fix the transport to set one in this task (that fix is in-scope: it
   is the degradation contract) and record the before/after in the test header. Acceptance:
   with `systemctl stop ollama`: both tests green within budget; `systemctl start ollama`
   after.
6. **T6 (C-f — after T4).** Create `tools/ci/offline-proof.sh` (§3.5 script verbatim; make
   executable). Run it locally as root; then run the isolation-teeth variant
   (`DOWIZ_AI_MODE=connected` + remote URL inside the netns) and confirm typed failure. Wire
   it as a nightly/release CI job (or document in the job comment why per-commit was chosen/
   rejected — decide, record, don't default silently). Acceptance: script exit 0 on the
   mode-2 leg; teeth-run demonstrably fails typed.
7. **T7 (benches).** Add `compose/from_env_local` bench; measure and record the
   connection-refused and stall-path refusal times against the §6 budgets in
   `llm-adapters/benches/BENCH_HISTORY.md`; record the mode-1 zero-overhead sentence
   verbatim. Acceptance: budgets met or the miss recorded + investigated, never silently
   accepted.
8. **T9 (C-g — alongside/after T2+T3; listed before close-out, numbering appended 2026-07-18).**
   Extend §2's env-contract doc comment with `DOWIZ_LLM_PROVENANCE` (attribution-only field);
   add the two §3.6 compose tests + the provenance-neutrality grep guard RED-first; write the
   provisioning preset as an EnvFile template (`tools/preset/dowiz-ai.env.default`, containing
   `DOWIZ_AI_MODE=local` + a comment naming the invariant) and point
   `preset_file_is_explicit_local` at it. Do NOT touch P48's blueprint or roadmap section
   (parallel lane — cross-reference only, §3.6 item 3). Acceptance:
   `cd llm-adapters && cargo test compose` green including both new tests.
9. **T8 (close-out).** Run: `cd kernel && cargo test --lib`, `cd engine && cargo test`,
   `cd llm-adapters && cargo test`, `cd agent-loop && cargo test`, the C-e choreography, and
   `offline-proof.sh`. Re-run P40's non-conflation check
   (`git diff --stat <base>..HEAD -- kernel/src/ports/agent agent-adapters` → empty). In
   bebop-repo: confirm `ac6_solo_island_full_flow_no_peers` untouched-green. Verify every §5
   DoD row; both ledger rows present with red→green proof. The connected parity leg's open
   status must be DECLARED in the closing summary if still open — never claimed closed
   without a real remote run (ledger ratchet rule, `REGRESSION-LEDGER.md:9-16`).
