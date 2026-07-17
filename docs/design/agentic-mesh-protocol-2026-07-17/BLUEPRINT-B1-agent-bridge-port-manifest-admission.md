# BLUEPRINT B1 — `AgentBridge` PORT + SIGNED `AgentManifest` + ADMISSION PATH

> **Anchors:** F2 (deny-by-default + rate-limit on new bridge ports), F10 (sub-agent max-depth cap),
> M5 (backend = config, never a kernel recompile), M6 (zero protocol deps at the trust boundary),
> M12 (per-agent capability scope). Scoped by `SYNTHESIS-codebase-and-architecture-direction.md`
> §3.1/§5.1; discovery grammar per `R3-agentic-infrastructure.md` §1–2; validation-policy-as-data per
> `R2-web3-good-patterns.md` §1 (ERC-4337); config-lattice discipline per
> `docs/design/spectral-energy-flow-evolution-2026-07-16/BLUEPRINT-E3-self-harness-loop-for-llm-harness.md` §2(ii).
> **Depends on:** nothing already-unbuilt — every verification primitive it wires exists and is
> RED-tested. **Parallel-safe with:** B2, B3, B4 (blueprint level; B2/B3 *consume* the admitted-agent
> identity this blueprint defines, they do not modify its files). Out of scope: transport (MESH-09),
> auto-tuning (E3), receipts/settlement (B2), exposure ledger (B3).
> **Planning artifact only. No code is written or edited by this document.**

---

## §0 The problem

Any node must be able to bridge in its own agent — a LangGraph graph, an MCP server, a bare binary —
without the kernel trusting a single self-declared byte. The repo's one existing bridge pattern
(`LlmBackend` + `llm-adapters`) admits a backend on *configuration alone*: nothing signs the
backend's capability claim, nothing scopes what it may touch, nothing bounds recursive spawning.
Every surveyed protocol punts this (R3 §8: MCP has no crypto tool identity, A2A card signing is
optional JWS, both anchor trust in OAuth/DNS). B1 generalizes the proven `LlmBackend` seam into an
`AgentBridge` port whose discovery artifact — the `AgentManifest` — is mandatorily hybrid-signed,
fail-closed, enumerable-only, admitted through the RED-tested `HybridGate::check`, then caged by the
existing `SandboxTier` split and `TokenBucket` envelope.

## §1 Current-state evidence (live re-read, this session)

- **`kernel/src/ports/llm.rs`** — the pattern to generalize. Compile firewall: zero
  network/HTTP/JSON/serde in the port module (`:3-7`). `Caps { chat, embed, rerank, tool_calling }`
  is fail-closed — undeclared = `false` (`:17-23`). `CachePolicy` is a type, not a convention
  (`:78-87`). `Usage::cost()` prices 1 token = 1 `TokenBucket` unit (`:97-102`). `LlmBackend` trait:
  `id/caps/chat/embed/rerank/health`, typed `LlmError`, never a mock (`:154-169`).
- **`llm-adapters/src/`** — the adapter stack to mirror. `dispatch.rs`: `Dispatcher<B>` acquires
  `cost = max_tokens.max(1)` from the bucket **before** the call (`:82`, `:90`), refuses with typed
  `DispatchError::BudgetExceeded` (`:112-114`), and emits a `TrackRecord` row on BOTH success and
  failure — fields `{backend_id, model_id, total_tokens, ms, task, success, value, cost}` (`:37-54`),
  serialized by `append_harvest` with JSON keys `model/task/success/value/cost/backend/tokens/ms`
  (`:135-148`). `quirks.rs`: one `Quirks` struct of per-backend wire deltas with
  `ollama()/vllm()/managed_api()` constructors (`:11-28`, `:45-81`). `transport.rs`: one
  `OpenAiCompatTransport`, zero vendor knowledge (`:16-19`). `cache.rs`: `CachingBackend<B, S>` keyed
  by `sha3_256` of the canonical request (`:31-35`, `:57-81`). `compose.rs`: the composition
  `Dispatcher<CachingBackend<OllamaAdapter, S>>` behind a config-driven `StackBuilder` (`:40-42`, M5).
- **`kernel/src/token_bucket.rs`** — `try_acquire` degrade-closed: `false` on shortfall, no partial
  grant, no silent queue (`:46-63`); bound granted ≤ capacity + rate·elapsed (`:1-5`).
- **`/root/bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs`** — `HybridGate::check` (`:124-209`)
  enforces, in fixed order: capability freshness (`:134-136`) → anchor-rooted `verify_chain`
  (`:143`) → armed `RedLinePolicy` deny-by-default (`:150-154`) → `RevocationSet` on classical key,
  capability hash, and `pq_key_id` (`:159-168`) → real Ed25519 `verify_classical` (`:171`) → real
  ML-DSA-65 `verify_pq` under `RequireBoth` (`:180-186`) → verify-then-record nonce insertion,
  bounded at `MAX_SEEN_NONCES = 1<<20` (`:56`, `:193-206`). A self-signed frame with no anchor chain
  is `UnknownIssuer` (test `:380-396`).
- **`proto-cap/src/capability.rs` / `scope.rs` / `node_id.rs`** — `Capability
  {subject_key, subject_key_pq: Option<Vec<u8>>, scope, nonce, expiry}` (`capability.rs:45-59`) with
  deterministic `canonical_bytes_tlv` (`:106-118`). `Resource`/`Action` are CLOSED enums with pinned
  discriminants; the `Claim` variant documents the additive precedent: "pinned discriminant … do NOT
  renumber existing variants" (`scope.rs:17-62`). `NodeId::from_keys = SHA3-256(pq_pub ‖
  classical_pub)` (`node_id.rs:45-50`).
- **`kernel/src/isolation/microvm.rs`** — `SandboxTier::{WasmComponent, NativeProcessRequiresKvm}`
  (`:19-27`); `register_adapter` accepts `"wasm-component"` unconditionally, accepts
  `"native-process"` only when `kvm_available()` (`/dev/kvm` + vmx/svm probe, `:52-63`), refuses
  everything else with **no unsandboxed fallback** (`:76-92`).
- **`docs/design/ARCHITECTURE.md`** — F2 exact wording: "Hub opens a NEW inbound port for a bridge.
  … LOCK + deny-by-default+rate-limit" (`:63`). F10: "Hub delegates to a sub-agent that opens its
  own sub-hub. … FUT: depth blowup. … LOCK + max-depth-cap" (`:71`).
- **What does NOT exist:** no `AgentBridge`, no `agent-adapters` crate, no manifest type, no
  Wasmtime embedding behind the probe. Grep for `AgentManifest|AgentBridge` = docs only.

## §2 Target-state design

### 2.1 `AgentManifest` — canonical TLV (C7b layout, deterministic, re-derivable)

The manifest is the payload of a `SignedFrame` whose `Capability.scope` is the new pair
`(Resource::AgentBridge, Action::AdmitAgent)` — two additive, pinned-discriminant enum variants
(`Resource::AgentBridge = 0x12`; `Action::{AdmitAgent, InvokeAgent}`), exactly the MESH-03/`Claim`
precedent. Fields:

```
AgentManifest TLV (canonical, fixed T order; unknown T ⇒ decode error, fail-closed):
  T=0x01 agent_node_id    : 32 B   NodeId = SHA3-256(pq_pub ‖ classical_pub) (ADR-0007)
  T=0x02 subject_key      : 32 B   Ed25519 public key (classical leg)
  T=0x03 subject_key_pq   : 1952 B ML-DSA-65 public key — MANDATORY (no Option here)
  T=0x04 agent_caps       : 1 B    fail-closed bitmap (see below)
  T=0x05 action_scopes    : var    list of (Resource, Action) pairs — CLOSED enums only
  T=0x06 resource_needs   : var    list of ResourceNeed variants (closed enum; egress hosts are
                                   u16 INDEXES into the operator's allowlist, never hostname strings)
  T=0x07 cost_denomination: 1 B    closed enum; only value today = TokenBucketUnits(0x01)
  T=0x08 budget_request   : 16 B   capacity u64 ‖ refill_milli_units_per_sec u64 (integers, no floats)
  T=0x09 validation_policy: 1 B    closed enum; floor = RequireBoth(0x01) — see below
  T=0x0A execution_model  : 1 B    WasmComponent(0x01) | NativeProcess(0x02)
  T=0x0B config_axes      : var    list of (axis_id u8, value_index u8) — E3 lattice (see below)
  T=0x0C depth_request    : 1 B    requested sub-delegation depth
  T=0x0D quirks_profile   : 1 B    closed registry: McpServer(0x01) | … (never a string)
  T=0x0E nonce            : 8 B
  T=0x0F expiry           : 8 B    monotonic tick, as Capability.expiry
```

**`AgentCaps`** (the `Caps`-shape generalized; undeclared bit = `false`, caller must never assume):
`invoke_tool` (executes tool calls), `read_resource` (serves resource reads), `render_prompt`
(serves prompt templates), `delegate` (may request sub-agent invocation — `false` ⇒ granted depth
is 0), `long_task` (async task lifecycle), `streaming` (partial results).

**Enumerable config only (E3 Phase-A lattice, cited above):** `config_axes` carries
`(axis_id, value_index)` pairs where `axis_id` names an axis in a fixed in-code registry (e.g.
`transport ∈ {stdio, streamable_http}`, `protocol_epoch ∈ {pinned set}`, `cache_policy ∈ {Exact,
NoCache}`, `max_concurrent ∈ {1,2,4}`) and `value_index` indexes into that axis's fixed bounded
domain. A free-form value is **unrepresentable**: an out-of-range index or unknown axis fails TLV
decode — rejection happens at parse time, structurally, before any gate runs.

**Validation policy as data (R2 §1):** the manifest declares the actor's acceptance predicate and
the verifier evaluates the *declared* policy — the ERC-4337 move. The floor is a real constraint,
not a comment: the decoder's policy enum has **no variant weaker than `RequireBoth`** (no
`ClassicalOnly`/`ClassicalUntilPqAudit` code point exists; such a byte fails decode), and
`effective_policy(declared) -> Result<HybridPolicy, AdmissionError>` can only return `RequireBoth`
or a future *narrowing* variant (threshold-classical ⊕ single-PQ per R2 §3 — both legs plus more).
Relaxation below the hybrid floor is impossible at three layers: wire (no code point), decode
(unknown byte = error), admission (return type).

### 2.2 Admission path

Kernel-side `admit(frame, roster, chain, revocations, now)`:

1. **Parse (fail-closed):** strict TLV decode of the payload — unknown T, out-of-domain
   `config_axes`, missing mandatory field, or a policy byte below floor ⇒ `ManifestParseError`.
   Nothing else runs.
2. **`HybridGate::check`** on the frame, gate constructed via `new_redlined(RequireBoth,
   DenyByDefault)` — the exact existing order: freshness → `verify_chain` (anchor-rooted, narrow-only)
   → red-line (a manifest whose `action_scopes` touch money/auth/secrets/migrations is rejected
   unless operator-allow-listed) → revocation (classical key, cap hash, `pq_key_id`) →
   `verify_classical` → `verify_pq` → verify-then-record nonce. No new verification code is written.
3. **Identity binding (new):** recompute `NodeId::from_keys(T=0x03, T=0x02)` and require equality
   with `T=0x01`; also require frame `subject_key/subject_key_pq` == manifest `T=0x02/0x03`. A
   manifest signed by keys that don't hash to its claimed `NodeId` is rejected.
4. **Sandbox tier assignment (new):** `execution_model` maps to
   `register_adapter("wasm-component" | "native-process")` verbatim — `WasmComponent` always admits;
   `NativeProcess` admits only under `kvm_available()`, else `AdapterRejected`, never a downgrade
   (microvm.rs `:76-92`). WASM is an *integrity* boundary, not confidentiality (R3 §6): node signing
   keys never enter the guest address space — the host signs, the guest only computes.
5. **Budget envelope (F2 rate-limit):** mint a dedicated `TokenBucket` with
   `granted = min(budget_request, operator per-peer cap)`; denomination is `TokenBucketUnits`
   (`Usage::cost` parity: 1 unit ≈ 1 token-equivalent).
6. **Depth grant (F10):** `granted_depth = min(depth_request, DEFAULT_MAX_AGENT_DEPTH = 3)`, and 0
   when `!agent_caps.delegate`.
7. **Record:** append an `AdmissionEvent` (manifest content-id, granted envelope, tier, depth) to the
   WORM log via `commit_after_decide`. No capability is usable before this commit succeeds.

**Wasmtime fuel ↔ `TokenBucket` — the mechanism.** Fuel is NOT 1:1 with budget units: fuel meters
CPU instructions, units meter billing. One pinned constant `FUEL_PER_UNIT` (initial 100_000
fuel/unit, calibrated by a B4-style criterion bench, mirror-pinned kernel↔adapter per the
synthesis's P3/`DT_STABLE` treatment) converts. Execution is **prepaid, tranche-wise**: before each
guest slice the host calls `bucket.try_acquire(TRANCHE_UNITS)` (default 8 units); on grant it
`store.set_fuel(TRANCHE_UNITS × FUEL_PER_UNIT)` and resumes; on Wasmtime's out-of-fuel trap it
attempts the next tranche; if `try_acquire` returns `false`, the instance is terminated with typed
`BudgetExceeded` — refusal, never silent throttling. An epoch-deadline is armed alongside as the
wall-clock backstop (fuel bounds CPU, epochs bound elapsed time). Consumed units are what the
`TrackRecord` row prices.

### 2.3 `agent-adapters` crate skeleton

Sibling of `llm-adapters`, same structure: `lib.rs`, `manifest.rs` (TLV encode/decode — the kernel
port module holds only plain value types behind the same zero-HTTP compile firewall as
`ports/llm.rs:3-7`; a new `kernel/src/ports/agent.rs` defines `AgentBridge` trait —
`id/caps/manifest/invoke/health` with typed `AgentError` — and the value structs),
`transport.rs` (one generic JSON-RPC 2.0 transport, zero framework knowledge, mirroring
`OpenAiCompatTransport`), `quirks.rs` (`AgentQuirks` + per-framework constructors), `dispatch.rs`
(reuses the `Dispatcher` pattern: pre-acquire, typed refusal, harvest row), `cache.rs` (reuse
`CachingBackend`; idempotent `read_resource` calls cacheable under `Exact`, tool invocations default
`NoCache`). Composition reused verbatim: `AgentDispatcher<CachingBackend<McpServerBridge, S>>`.
**Every bridged call emits the existing `TrackRecord` row** (`dispatch.rs:37-54` shape, unchanged
schema): `backend_id = "mcp:<server-id>"`, `model_id` = quirks-profile id + mapped tool name,
`task = "agent.invoke_tool" | "agent.read_resource" | …`, `total_tokens`/`cost` = budget units
consumed, `success` recorded on both poles — same JSONL, same `gov_route` fold.

**Reference `Quirks` profile: `AgentQuirks::mcp_server()`.** It translates MCP's discovery grammar
(R3 §1): `initialize` capability negotiation + paginated `tools/list`, `resources/list`,
`prompts/list` → `AgentCaps` bits (`tools` ⇒ `invoke_tool`; `resources` ⇒ `read_resource`;
`prompts` ⇒ `render_prompt`; MCP has no delegation ⇒ `delegate = false` always for this profile; MCP
tasks ⇒ `long_task`). The crucial decision: **MCP's open-world string grammar never enters the
manifest.** Discovery output is untrusted input; the bridge produces a *draft*, and the operator's
keys sign the final manifest. Tool names are free-form strings in MCP, so the quirks profile carries
an operator-authored **tool allowlist map** `tool_name → (Resource, Action)`; unmapped tools are
unreachable (fail-closed drop, never string passthrough). The manifest carries only the closed-enum
scopes plus `sha3_256` of the canonical sorted tool-map, so post-admission drift (`listChanged`,
server substitution — R3's registry-poisoning class) is detectable: digest mismatch on invoke ⇒
refuse and require re-admission. Further quirks fields: transport kind (stdio | streamable-http, an
enumerated config axis), pinned protocol epoch, JSON-RPC error → typed `AgentError` mapping, and
**server-initiated `sampling/createMessage`/elicitation requests are always refused** with a typed
error — an admitted agent must never drive the host's LLM (RC-2-shaped control inversion).

### 2.4 F2 / F10 enforcement points

- **F2 deny-by-default:** no admitted manifest ⇒ no dispatch path exists (the dispatcher is
  constructed only from an `AdmissionEvent`); within an admitted bridge, only allow-listed
  `(Resource, Action)` pairs are reachable; red-line scopes need explicit operator allow-listing.
  **F2 rate-limit:** the per-agent `TokenBucket` minted at admission, enforced at the same slot as
  `dispatch.rs:82-90` — acquire before call, typed `BudgetExceeded` refusal.
- **F10 depth cap:** depth is **not a mutable counter the agent reports** — it is the number of
  `(Resource::AgentBridge, Action::InvokeAgent)`-scoped links in the invocation frame's delegation
  chain, already verified by `verify_chain`, so depth is cryptographically witnessed and survives
  cross-node hops. The dispatcher computes it per invocation and refuses with typed
  `DepthExceeded` when it reaches `granted_depth` (global default cap **3**; per-manifest grant may
  only narrow). Checked at admission (grant) and at every dispatch (enforcement).

## §3 Migration steps

1. Add pinned `Resource::AgentBridge` + `Action::{AdmitAgent, InvokeAgent}` variants (additive,
   MESH-03 discipline; wire-stability test on discriminants).
2. Create `kernel/src/ports/agent.rs` (trait + value types + `AgentCaps` + policy enum; compile
   firewall check mirroring the WAVE-0 `cargo tree` done-check).
3. Implement `AgentManifest` canonical TLV in proto-cap style + strict decoder (RED-first: free-form
   axis, unknown T, weak-policy byte all fail decode).
4. Implement the admission function wiring `HybridGate::check` + NodeId binding + `register_adapter`
   + bucket mint + depth grant + `commit_after_decide` record.
5. Scaffold `agent-adapters` (transport, quirks, dispatch with `TrackRecord`, cache reuse).
6. Implement `AgentQuirks::mcp_server()` + tool-allowlist map + discovery-digest pinning.
7. Wasmtime embedding behind `SandboxTier::WasmComponent`: fuel-tranche loop + epoch backstop;
   `FUEL_PER_UNIT` pinned after a B4 bench.

## §4 Acceptance criteria (numbered, falsifiable)

1. **Unsigned = nothing.** A manifest frame with a missing or 1-bit-corrupted signature (either leg)
   is rejected by `HybridGate::check` and NO capability, bucket, sandbox, or `AdmissionEvent` exists
   afterward — verified by asserting the admission store is byte-identical before/after.
2. **Free-form dies at parse.** A manifest whose `config_axes` carries an unknown axis id or
   out-of-domain value index (or any string-bearing TLV) fails at TLV decode with
   `ManifestParseError` — provably *before* `HybridGate::check` runs (gate call-count = 0).
3. **Floor is unrelaxable.** No decodable byte sequence yields an effective policy weaker than
   `RequireBoth`; property test over all 256 policy bytes: each either decodes to
   `RequireBoth`-or-stronger or errors.
4. **Fuel exhaustion = refusal.** A WASM guest that exhausts its granted tranches (bucket
   `try_acquire → false`) is terminated with typed `BudgetExceeded`; it is never resumed at reduced
   rate and never queued — assert no further fuel is ever set after the refusing acquire.
5. **F10 fires.** An invocation whose delegation chain carries more `InvokeAgent` links than
   `granted_depth` (and any chain deeper than 3) is refused with `DepthExceeded`; an agent with
   `delegate = false` is refused at depth 1.
6. **No KVM, no native.** A `NativeProcess` manifest on a host where `kvm_available() == false` is
   rejected (`AdapterRejected`), never downgraded — reuses microvm.rs test R2's posture.
7. **Identity binds.** A manifest whose `agent_node_id ≠ SHA3-256(pq_pub ‖ classical_pub)` of its own
   carried keys is rejected at step 3 even with valid signatures.
8. **Harvest is total.** Every bridged call — success, backend error, budget refusal — appends
   exactly one `TrackRecord` row in the existing schema; a `gov_route` fold over mixed LLM+agent
   rows parses with zero schema errors.
9. **Digest drift re-admits.** Changing the MCP server's tool list after admission causes the next
   invoke to fail the tool-map digest check and refuse until re-admission.

## §5 What this unblocks

B2 (`WorkReceipt`/`Settlement`) gains a *verified counterparty*: receipts bind to the admitted
manifest's `NodeId` and capability chain, so "who did the work under which grant" is already
answered. B3 (`ExposureLedger`) keys its per-peer commitments on the same admitted identity and
zeroes envelopes for burnt peers. B4's bench pins the `FUEL_PER_UNIT` and per-message verify numbers
this blueprint cites as calibration constants. More broadly: after B1, "any node bridges in its own
agent" is an admission decision under existing cryptographic law, not a trust decision.

---

*Blueprint B1 complete. All §1 citations re-read live 2026-07-17 in `/root/dowiz-agentic-mesh`
(branch `feat/agentic-mesh-protocol-2026-07-17`) and `/root/bebop-repo/bebop2`. No code written.*
