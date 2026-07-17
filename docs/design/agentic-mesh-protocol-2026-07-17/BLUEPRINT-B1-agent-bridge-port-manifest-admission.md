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

---

> **Expansion pass — 2026-07-17.** Everything above this line is the consolidation-verified body and is
> **unmodified**. The four sections below are *additive*: extended framing, a full Definition of Done
> (wrapping — not replacing — §4's acceptance criteria), the honest event-sourcing treatment, and the
> long-term/safety/scalability analysis the execution body did not carry. Nothing above is weakened.

## Extended Context

AgentBridge is the **keystone of the Agent Exchange Plane**, not one of three peers. B2
(`WorkReceipt`/`Settlement`) and B3 (`ExposureLedger`) both *consume* the admitted identity this
blueprint mints — a B2 receipt binds to "the admitted manifest's `NodeId` and capability chain" (§5),
and B3 keys its per-peer commitments on "the same admitted identity" (§5). Neither can produce a
verified counterparty on its own: without B1 there is no cryptographic answer to *who* did the work
under *which* grant, so B2's receipt binds to nothing and B3's ledger keys on nothing. The consolidation's
sequencing (`AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md` §4) makes this concrete — B2 and B3 both list **B1
as a hard dependency**; B1 is on the critical path for the whole plane, while B4 (the numbers-provider)
is the only genuinely parallel-safe unit. Stall B1 and the plane is a set of primitives with no shared
subject.

The cost of *not* building it is precise and already visible in the repo: node operators stay locked to
**LLM-only bridging** via the existing `LlmBackend`/`llm-adapters` seam, which admits a backend "on
configuration alone: nothing signs the backend's capability claim, nothing scopes what it may touch,
nothing bounds recursive spawning" (§0). Every non-LLM agent — a LangGraph graph, an MCP server, a bare
binary — remains un-bridgeable except by hand-editing config and trusting self-declared bytes, exactly
the trust-the-text posture MAST measures failing 21.3% of the time (R3 §3). "Any node bridges in its own
agent" stays a *trust* decision instead of "an admission decision under existing cryptographic law" (§5).
There is no incremental half-measure: the `AgentManifest` and its admission function are the smallest unit
that turns bridging from configuration into cryptography.

For a reader new to the arc: B1 does **not invent trust machinery** — it routes a new artifact through the
mesh's existing one. Admission is the RED-tested `HybridGate::check` (dual Ed25519 ⊕ ML-DSA-65 under an
unrelaxable `RequireBoth` floor), with the same `RevocationSet` that burns compromised keys, capability
hashes, and PQ key ids checked in the same fixed order it already enforces for every other frame
(`hybrid_gate.rs:124-209`). The only genuinely new code is the *manifest* (a strict canonical-TLV shape),
the *binding* (recomputed `NodeId` must equal the claimed one), and the *caging* (sandbox tier + minted
`TokenBucket` + depth grant). Everything load-bearing about trust is reuse.

## Definition of Done

The §4 acceptance criteria are necessary but not sufficient — they prove the admission *logic* is correct;
the DoD below proves the *deliverable* is complete, integrated, non-regressive, honestly documented, and
unblocked of its two known external gates. **All items must be GREEN; §4 is item 2 of this list.**

1. **Code exists and composes.** `kernel/src/ports/agent.rs` (`AgentBridge` trait + value types +
   `AgentCaps` + policy enum, behind the `ports/llm.rs:3-7` compile firewall), the `agent-adapters` crate
   (`manifest.rs`/`transport.rs`/`quirks.rs`/`dispatch.rs`/`cache.rs`), `AgentQuirks::mcp_server()`, and
   the `admit(...)` function all exist per §2.2/§2.3 and migration steps 1–7, and the reused composition
   `AgentDispatcher<CachingBackend<McpServerBridge, S>>` type-checks.
2. **All §4 acceptance criteria (1–9) pass** — the falsifiable behavioural contract, unchanged.
3. **No test regression.** The pre-B1 `cargo test` pass-counts for `dowiz/kernel`, `dowiz/engine`,
   `bebop2/proto-cap`, and `bebop2/core` are captured at build-start and must not *decrease* (new tests
   only add). Advisory annotation-site counts re-read 2026-07-17 for scale: kernel 423, engine 53,
   proto-cap 93, core 181 `#[test]` sites — **these are annotation counts across both repos, not the
   binding baseline**; the binding number is the actual green pass-count from a clean run on the pre-B1
   tip (MEMORY's 2026-07-16 "kernel 336 green / engine 47-48" snapshot is advisory — re-verify, do not
   trust it as the gate).
4. **Docs match what shipped.** This blueprint is updated wherever the built code *necessarily* diverges
   from the design (e.g. the final `FUEL_PER_UNIT` value once the B4 bench pins it; the actual
   `DEFAULT_MAX_AGENT_DEPTH` once the F10 default is ruled — see item 7). Divergence is annotated in place,
   consolidation-style, never silently.
5. **The Wasmtime DECART is written** (owed per consolidation §5 Q1.4; the Detailed Planning Protocol
   requires it *in the planning artifact*, and B4 wrote one for `criterion` while B1 had not). It is below,
   in the canonical `integration-decart-rule.md` table+DECISION+probe form:

   | Criterion | **Wasmtime 46.0.1** (component model + cranelift) — chosen | wasmer | WAMR / wasm3 (C) | native-process only (skip WASM) |
   |---|---|---|---|---|
   | Rust-native fit (adapter-side; kernel port stays `no_std`+alloc behind the `ports/llm.rs:3-7` firewall) | pure Rust; lands in `agent-adapters`, zero in the M6 boundary | pure Rust | C runtime — fails Rust-native default | Rust, but forfeits the WASM integrity tier entirely |
   | Correctness & security — falsifiable | component-model capability isolation; oss-fuzz + CVE tracking; proven here by §4 crit. 4 (fuel-exhaustion ⇒ typed `BudgetExceeded`) and crit. 6 (no-KVM ⇒ no native) | similar API, smaller fuzz corpus | not exercised; larger C attack surface | N/A — no in-process sandbox to prove |
   | Performance — *measured, not assumed* | fuel-metering overhead + `FUEL_PER_UNIT` **calibrated by a B4 criterion bench** (step 7), not estimated; cranelift JIT is native-adjacent | comparable, unmeasured here | interpreter — slower, unmeasured | no runtime cost, but no WASM workloads run |
   | Supply-chain & license | Apache-2.0-WITH-LLVM-exception; large tree (cranelift ≈ 30 crates) **but offline-present** — `wasmtime-46.0.1`, `cranelift-0.133.1`, `wasmtime-internal-component-{macro,util}-46.0.1` all in `~/.cargo/registry` (verified this session; the wgpu/W21 network-block failure mode does **not** apply) | offline status unverified | C build (`cc`) — would flag `cargo-deny` | none added |
   | Maintainability | `component model` maps 1:1 onto `SandboxTier::WasmComponent`; Bytecode Alliance active | active | smaller community | simplest, but no capability model |
   | Reversibility — port/adapter/fallback, not a core commitment | lands in `agent-adapters` (a port); `NativeProcessRequiresKvm` tier already exists as the alternative sandbox (`microvm.rs`); `pulley` (wasmtime's interpreter, no cranelift) is a no-JIT fallback | swappable behind the same trait | swappable | it *is* the fallback tier |
   | Evidence (number/probe, NOT social proof) | offline registry probe this session; component-macro/util crates extracted = component model available | — | — | `microvm.rs:52-63` KVM probe |

   **DECISION: Wasmtime 46.0.1 (component model, cranelift backend), adapter-side in `agent-adapters`** —
   Rust-native, offline-available (no new network trust), and its `store.set_fuel` + `epoch_deadline` APIs
   are exactly the primitives §2.2's prepaid tranche loop requires; the M6 kernel port is untouched.
   **Older-as-adapter:** the `NativeProcessRequiresKvm` (microVM) tier is kept, not purged, as the
   confidentiality-grade alternative for workloads the WASM integrity boundary cannot serve; `pulley`
   stays available as a no-cranelift fallback. **Probe (strongest case against):** the cranelift codegen
   tree (~30 crates) is the single largest supply-chain surface this whole arc introduces. It did not win
   the argument because the isolation is *structural, not aspirational* — the tree lands only in
   `agent-adapters`, and `kernel/src/ports/agent.rs` sits behind the same zero-HTTP/serde compile firewall
   as `ports/llm.rs:3-7` (verifiable by the migration-step-2 `cargo tree` done-check), so the M6 trust
   boundary's `Cargo.toml` stays byte-clean exactly as `llm-adapters` keeps transport deps out of
   `ports/llm.rs`. If bebop2-grade zero-dep purity is ever demanded of the *adapter* too, `pulley` or the
   KVM tier is the documented retreat.

6. **A demonstrated run exists — not unit tests alone** (mirroring how H1–H2's fault-injection proof
   produced an *artifact*, a captured run, rather than a green checkmark). A small end-to-end harness
   (`agent-adapters/examples/admit_demo.rs` or an integration test that prints and captures) exercises the
   *real* admission path against a *real* `FileEventStore`, and its output is captured to a
   `DEMO-RUN.md`-style doc (host + both commit fingerprints, `BENCH_RESULTS.md` convention):
   - **Accepts a real valid manifest:** a freshly generated Ed25519 ⊕ ML-DSA-65 keypair signs a manifest
     whose `agent_node_id` correctly equals `SHA3-256(pq_pub‖classical_pub)`, `validation_policy =
     RequireBoth`, `execution_model = WasmComponent`; `admit(...)` returns `Ok`, an `AdmissionEvent`
     lands in the WORM log (its content-id printed), a `TokenBucket` is minted at the granted envelope,
     tier = `WasmComponent`, and one subsequent bridged `invoke` succeeds and appends **exactly one**
     `TrackRecord` row.
   - **Rejects a real invalid case:** the *same run* feeds a manifest whose `agent_node_id` does **not**
     hash from its own carried keys (or, second variant, a `config_axes` pair with an out-of-domain
     `value_index`); `admit(...)` returns the typed `IdentityMismatch` (resp. `ManifestParseError`), and
     the harness asserts the admission store is **byte-identical before and after** (§4 crit. 1's posture)
     and, for the parse-fail variant, that `HybridGate::check`'s call-count is 0 (§4 crit. 2). The rejected
     error type and the unchanged store digest are printed into the artifact.
   This is the "manifest admission path actually rejecting a real invalid case and accepting a real valid
   one" that unit assertions alone cannot demonstrate.
7. **The `0x12` discriminant collision is resolved before "done."** Per consolidation §4/§5 Q1.1, B1 pins
   `Resource::AgentBridge = 0x12` and B2 independently pins `Resource::WorkReceipt = 0x12` — a genuine
   collision, both having taken the next byte after the live high-water mark `Resource::Migration = 0x11`
   (`scope.rs`, verified this session). **B1 cannot ship its `Resource::AgentBridge` value until the
   lead-agent Wave-0 discriminant-allocation ruling assigns distinct bytes across both enums** (the ruling
   also numbers B1's currently-unnumbered `Action::{AdmitAgent, InvokeAgent}` against B2's pinned
   `0x19–0x1E`). The MESH-03 wire-stability pin test (migration step 1) catches a violation mechanically,
   but the *allocation itself* is an integration act that must precede this DoD's item 1. Marking B1 done
   with an unratified `0x12` is a hard fail.

## Event-Driven Architecture Treatment

**Event-sourced (WORM `MeshEvent`): the admission *decision* itself — yes, by design.** §2.2 step 7
already specifies it: `admit(...)` appends an `AdmissionEvent` "to the WORM log via
`commit_after_decide`. No capability is usable before this commit succeeds." This is the correct call —
admission is a *structural mutation of the node's trust roster*, exactly the class the event log exists
to make durable, ordered, and replayable. Concretely, the event uses the live `MeshEvent` shape
(`event_log.rs:129-152`): `prev` chains to the node's current log tip (local-first, bound at append
time), `actor_pubkey` = the **admitting node's** identity (the local operator making the admission
decision, not the bridged agent — the agent is the *subject*, the node is the *actor*), `actor_seq` = the
node's per-actor monotonic counter, and `payload` = a canonical TLV of `{kind = AgentAdmitted, manifest
content-id (sha3_256 of the canonical manifest bytes), granted TokenBucket envelope (capacity ‖ refill,
integers), sandbox tier, granted_depth}`. The manifest content-id is the natural content-addressed key
(R2 §4). Admission goes through plain `commit_after_decide`, **not** `commit_after_decide_drift_gate`,
and that distinction is deliberate: the drift gate rejects spectrally-`Unstable(ρ>1)` *energy-flow*
events; an identity admission is not a flow mutation, so subjecting it to a stability discriminant would
be a category error (B2/B3 use the drift-gate slot because settlement/exposure *are* flow). Admission's
inverse — burning an admitted agent — is likewise event-sourced (a `RevocationSet` drop, durable and
gossiped), keeping the rule crisp: **structural trust changes are events; runtime metering is not.**

**Idempotency — two honest layers.** (i) *Crash/exact-replay* idempotency is free from the existing
`Duplicate` pattern: re-appending the identical `(prev, actor_pubkey, actor_seq, payload)` tuple returns
`AppendOutcome::Duplicate(id)` as a no-IO no-op (`event_log.rs:294-305`), so a replayed admission event
is structurally absorbed — no new code. (ii) *Semantic re-admission* (the same manifest bytes presented
again later, at a new tip and seq — which would **not** collide as a byte-`Duplicate`) is handled one
level up, at the pure `decide` closure `commit_after_decide` already wraps: `decide` is a function over
`(current admitted-set, manifest content-id)`; if that content-id already maps to a *live* (unexpired,
unrevoked) admission with an identical granted envelope, `decide` yields `NoChange` and **no event is
appended at all**. A new event is emitted only when something actually changed (envelope, expiry, tier) —
i.e. a genuine re-grant, which *should* be a distinct event. So "re-admitting an already-admitted manifest
with unchanged bytes is a structural no-op" holds — via `Duplicate` for exact replay, via a content-id
`decide` short-circuit for later re-presentation — without ever silently overwriting a live grant.

**NOT event-sourced, by design: live runtime metering — following the `TokenBucket`/`Dispatcher`
precedent, not a noisier one.** The kernel today does **not** emit an event per token acquisition
(`TokenBucket::try_acquire` mutates in-memory state only), and `Dispatcher` emits **one `TrackRecord`
harvest row per call** into the JSONL telemetry stream (`append_harvest`) — which is *not* a WORM
`MeshEvent`, it is the `gov_route` fold's separate append-only stream. B1 mirrors this exactly:
- **Per-tranche Wasmtime fuel** (`bucket.try_acquire(TRANCHE_UNITS)` → `store.set_fuel(...)` per slice,
  §2.2) is **in-memory state, not events.** Emitting a `MeshEvent` per fuel tranche would be the noisy
  invention the task warns against — it would flood the WORM log with metering churn that carries no
  durable trust meaning. Fuel is a CPU-instruction meter; the log records trust transitions.
- **Per-invocation outcome** (success, backend error, *and* budget refusal) lands as **exactly one
  `TrackRecord` row** in the existing schema (§2.3, §4 crit. 8) — same JSONL, same `gov_route` fold as
  LLM calls. This is "recorded" but on the telemetry plane, not the event chain.
- **Budget exhaustion** (`try_acquire → false` ⇒ typed `BudgetExceeded`, instance terminated) surfaces as
  a `TrackRecord` row with `success = false` and is returned to the caller; it does **not** mint a
  distinct `MeshEvent`. The durable trust artifact was the *admission* that set the envelope; the
  exhaustion is that envelope doing its job in memory.

Net: one WORM event at admission (and one at revocation); zero WORM events during execution; the entire
runtime story rides the pre-existing `TrackRecord` harvest stream. This is strictly the kernel's current
event/telemetry split, extended additively — no new event cadence is introduced.

## Long-Term Consequences, Safety, Scalability

**(a) Scalability — the admission path is *not* the thing that scales; the invoke path is.**
`HybridGate::check` runs **once per admission** (a rare, per-operator, uncontested act), so even at 100×
more bridged agents per node the admission cost — one hybrid verify at B4/R4 §5's ~0.1–1 ms — is
amortized to nothing; agents are admitted seldom and invoked often. The per-frame cost that actually
compounds is on the **invocation** path: every bridged-invoke frame's delegation chain is `verify_chain`'d
(each `Delegation` link = one more Ed25519 verify, B4 §1.2) plus the terminal ML-DSA leg under
`RequireBoth`. At R4 §5's measured envelope (~10³ verify/s/core now, ~8×10³ across `dowiz-dev`'s 8 cores;
~10⁴ optimized) the ceiling is reached only if *aggregate invocation-frame rate* — not agent count —
approaches that band. The natural batching point already exists in the arc: B4 §2.3's
`HybridGate::check_batch` batches the classical (Ed25519) legs across a burst, but honestly trims only
~15–20% under `RequireBoth` because the unbatchable ML-DSA leg dominates; its strongest application is
B1's own **boot-time re-admission of the stored manifest set** (a natural batch) and B2's settlement /
Sync·Pull bursts. Two real 100×-regime caveats the bench must confront, not paper over: (1) the
verify-then-record **nonce insert is under a shared `Mutex`** (`hybrid_gate.rs:193-206`) — B4 evidence
item 2b flags that single-threaded benching *understates* this contention, and it is precisely where
concurrent high-churn admission/invocation serializes; (2) `MAX_SEEN_NONCES = 1<<20` (~1.05 M,
`hybrid_gate.rs:56`) is a **fixed ring** — at 100× churn the replay-protection *window shrinks in
wall-clock terms*, so the anti-replay horizon is a function of traffic, not time. The structural fix for
raw throughput is out of scope and correctly gated: the AVX2/NTT ML-DSA port (~3×, B4 §5), triggered only
by *measured* traffic exceeding the *measured* ceiling — never speculatively.

**(b) Safety — the WASM boundary is integrity, not confidentiality, and here is exactly where that bites.**
§2.4 names it plainly ("WASM is an *integrity* boundary, not confidentiality"), and honesty demands
spelling out both sides. From inside a WASM-sandboxed **malicious** bridged agent, an attacker CAN:
(i) return wrong or adversarial *answers* — the sandbox guarantees the guest cannot escape, **not** that
it computes honestly (this is precisely why B2's `WorkReceipt` proves "authorized delivery of specific
bytes under a specific grant, never semantic quality"); (ii) burn its **entire** granted fuel + epoch
budget on a compute-bomb or spin — but the blast radius is capped to its *own* minted `TokenBucket`
envelope, and the epoch deadline preempts wall-clock overruns, so it cannot starve co-resident agents
(each holds a distinct bucket) or the host; (iii) — the **single most concrete honesty point** — mount
**micro-architectural timing / cache side-channel inference (Spectre-class) against whatever shares its
physical core**: the WASM component boundary provides memory-safety and control-flow integrity, it does
**not** provide micro-architectural isolation, and it can measure the timing of its own host-import calls.
What saves the design is *not* that this channel is closed — it is that there is **nothing inside the
guest's reach worth exfiltrating to**: node signing keys never enter the guest address space (§2.4 — "the
host signs, the guest only computes"), so the highest-value secret is structurally absent from the side
channel's endpoint. What the attacker CANNOT do: read the Ed25519/ML-DSA private keys (out of guest
memory), escape to native code or issue arbitrary syscalls (component model = capability-scoped host
imports only), open arbitrary network egress (`resource_needs` T=0x06 hosts are u16 *indexes into the
operator allowlist*, never hostname strings), or reach any `(Resource, Action)` outside its allow-listed
scope (unmapped MCP tools are a fail-closed drop). When true *confidentiality* isolation is required — a
guest that must hold a secret co-tenant data cannot be allowed to time — the `NativeProcessRequiresKvm`
microVM tier, not WASM, is the answer, and `register_adapter` makes that an operator tier choice, not a
code fork.

**(c) Long-term / lock-in — the closed-enum TLV buys safety and *charges* for extensibility; name the
price.** The `AgentManifest` field list runs `T=0x01–0x0F` with **no reserved/extension T and no
`manifest_version` field**, and the decoder is strict fail-closed ("unknown T ⇒ decode error"). This is
the same property that makes free-form values *unrepresentable* (§2.1) — and it is the same property that
**forbids graceful forward-compatible extension**: you cannot have both "unknown T is rejected" (the
safety guarantee) and "unknown T is ignored for forward-compat" (the extensibility affordance) from one
decoder. The arc deliberately chose safety. The honest consequence: the *only* built-in headroom for a
genuinely unanticipated capability without a wire change is the **`AgentCaps` byte — 6 of 8 bits used, so
2 spare boolean capabilities fit** (an undeclared bit reads `false` = fail-closed, so old manifests stay
safe under a new bit). Everything larger — a new field, a new resource-need *kind*, a new config-axis
domain beyond what an old decoder's registry knows — requires a **new manifest version and a coordinated
decoder upgrade across the mesh**, because an old decoder fail-closes on the unknown T / discriminant /
axis-id. That is a real friction, not a cosmetic one: the *first* schema evolution beyond the 2 spare bits
is a hard fork, since there is no version byte to branch on today. **Cheap pre-ship hedge worth an
operator ruling:** add a single `manifest_version` TLV (or a versioned outer envelope) *now*, before B1
ships — it is nearly free at this stage and converts every future breaking change into a version-gated
decoder branch, whereas retrofitting it later is itself the one breaking change you most want to avoid.
This does **not** weaken the closed-enum discipline (unknown *values within a version* still fail-closed);
it only gives the schema a forward door. Stated as the genuine tradeoff it is: closed-enum
unrepresentability (strong safety, verified) versus zero forward-compat headroom beyond 2 capability bits
(real extensibility friction) — the arc should pick consciously, not discover it at the first upgrade.

## Safety Hardening (post-adversarial-review)

> Additive, 2026-07-17. Written in direct response to `SYSTEM-BREAKER-safety-stress-test.md` findings
> **F2 [HIGH]** and **F6 [MEDIUM]**, and `COUNSEL-ethics-strategy-review.md` **§8 safeguard 4**.
> Everything above this line is **unmodified and un-weakened** — these are three new defenses and one new
> *blocking* acceptance criterion layered on top of the existing design. No existing guarantee (the
> unrelaxable `RequireBoth` floor, deny-by-default red-line, closed-enum TLV, host-held keys, the
> post-admission per-agent `TokenBucket`, F10 depth cap) is relaxed. Where a defense reuses an existing
> primitive it says so; the arc's "reuse verbatim, one new primitive" restraint is honored — SH-1 and SH-2
> introduce **no new cryptographic primitive** and no new trust code, only new *placement* of the
> `TokenBucket` and `CachingBackend` types already in the tree.

### SH-1 — Pre-cryptographic admission-attempt limiter + hard chain-length pre-check (fixes F2)

F2's mechanism is a **cost asymmetry**: a manifest frame is free to mint but `HybridGate::check` is real
crypto to reject (per-link Ed25519 across the delegation chain + Ed25519 + ML-DSA-65, B4's ~0.1–1 ms), and
the only rate limit B1 has (the per-agent `TokenBucket`, §2.2 step 5) is **minted at admission — it does
not exist for an unadmitted identity**, so every *failed* attempt pays no toll. The flood also contends the
shared verify-then-record nonce `Mutex` (`hybrid_gate.rs:193-206`), degrading legitimate verification
node-wide. B1 today has **no limiter on the pre-admission path** (§2.4 bounds only post-admission dispatch).
SH-1 closes this with **two independent guards, both cheap and both strictly *before* any signature
verification runs.** They are inserted into `admit(...)` (§2.2) as new steps **0a** and **0b**, ahead of the
existing step 1 (parse) and step 2 (`HybridGate::check`); nothing downstream changes.

**Guard A — coarse pre-crypto admission-attempt bucket (new step 0a, runs first).** Before *any* work —
before TLV parse, before the gate — `admit(...)` consults a **raw-admission-attempt limiter** built from the
**existing `kernel/src/token_bucket.rs` `TokenBucket`, reused verbatim** (no new primitive). One
`try_acquire(1)` per inbound admission frame; on `false` the frame is **dropped with a typed
`AdmissionThrottled` and zero further work** — it never reaches parse, never reaches `HybridGate::check`, and
therefore never touches the nonce `Mutex`. This is a single integer decrement + monotonic-refill compare: no
crypto, no allocation, O(1), and it is the *first* thing the admission entry point does.

- **Keying — and the honest MESH-09 dependency.** The limiter must bucket on something an unadmitted
  attacker cannot cheaply multiply. B1 **does not own the transport (MESH-09 is explicitly out of scope,
  see header)**, so the admission path sits *above* a transport this blueprint does not define. The correct
  source key is the transport's own coarse connection/peer identity — call it `ConnId` — which MESH-09 must
  surface up to `admit(...)`. **This is a named cross-layer dependency, stated honestly: SH-1's per-source
  fairness is only as good as MESH-09's connection accounting, and the transport layer is expected to carry
  its own connection-establishment rate-limit as the true first line (a limiter above an unbounded
  connection firehose is fairness, not a ceiling).** Two concrete shapes, in preference order:
  1. **Load-bearing property (transport-independent): one global coarse bucket** sized to the node's total
     tolerable pre-crypto verify rate (e.g. a small multiple of R4 §5's ~10³ verify/s/core envelope). This
     alone bounds **total** admission-time crypto work node-wide *regardless of source cardinality or
     spoofing* — it is the ceiling that makes the DoS survivable. It cannot be exhausted by minting fake
     source ids because it does not key on source at all.
  2. **Fairness refinement (bounded-memory, DoS-safe): a fixed-size array of `N` sharded buckets** indexed
     by `hash(ConnId) mod N`. Fixed `N` ⇒ **bounded memory by construction** — there is deliberately *no*
     per-source `BTreeMap` that an attacker could grow by spoofing source ids (that map would itself be an
     unbounded-memory DoS). Sharing a shard with an attacker degrades only that shard; the global bucket (1)
     still caps the aggregate. Precise per-`ConnId` buckets are available *only if* MESH-09 guarantees
     bounded concurrent-connection cardinality — otherwise the sharded array is the safe default.
  The global bucket (1) is mandatory; the sharded array (2) is a fairness add-on. Both refill slowly and are
  sized by operator config (M5 — config, not recompile), never by the attacker.

**Guard B — hard chain-length cap checked at decode, before the first `verify_signature()` (new step 0b,
inside parse).** F2 notes the depth cost multiplier: `verify_chain` (`roster.rs:252`) does `for link in
chain { … link.verify_signature()? … }` — **one real Ed25519 verify per link, with no length check before
the loop begins** (live-read this session: the function's only length handling is `chain.first().ok_or(
UnknownIssuer)` for emptiness; there is no upper bound). B1's `DEFAULT_MAX_AGENT_DEPTH = 3` does **not** cover
this — it governs `(AgentBridge, InvokeAgent)` *dispatch* depth (§2.4 F10), a different and smaller quantity
than the raw count of `Delegation` links a frame may present for verification (a chain can carry non-invoke
links). SH-1 adds a **separate, dedicated constant `MAX_VERIFY_CHAIN_LINKS`** (a small operator-config bound,
e.g. 16 — must be ≥ the mesh's legitimate maximum delegation-chain length, which is ≥ `DEFAULT_MAX_AGENT_DEPTH`
because invoke-depth ⊆ chain-length). Enforcement is **purely a length check on raw bytes, spending zero
crypto**:
- **O(1) claimed-length rejection:** the `SignedFrame`/chain TLV carries a link *count* (or the decoder can
  count `Delegation` record headers as it streams). The instant the *claimed* or running count exceeds
  `MAX_VERIFY_CHAIN_LINKS`, decode aborts with a typed `ChainTooLong` — **before allocating the full chain
  vec and before the first `link.verify_signature()` is ever called.** An attacker claiming 10⁶ links is
  rejected on the length field, not by verifying 10⁶ signatures.
- This runs as part of step 1's strict TLV decode (§2.2), so it is *before* step 2's `HybridGate::check`
  exactly as the existing "free-form dies at parse" property (§4 crit. 2) already places `config_axes`
  domain rejection before the gate. It is the same discipline extended to chain length.

**Net effect on F2.** Guard A bounds the *rate* of frames that can reach crypto at all (protecting both the
verify-CPU budget and, transitively, the shared nonce `Mutex`, since throttled frames never enter the gate).
Guard B bounds the *per-frame* crypto cost so a single admitted-through-the-bucket frame cannot carry an
unbounded verification bill. Neither guard weakens admission: a *legitimate* frame under the rate and within
`MAX_VERIFY_CHAIN_LINKS` reaches the unchanged `HybridGate::check` and is judged exactly as before. Both
guards are fail-closed (bucket empty ⇒ drop; over-length ⇒ decode error), matching the design's posture.

### SH-2 — Per-agent cache partitioning, default-deny (fixes F6)

F6 is real and currently unaddressed for the multi-tenant case: §2.3 reuses `CachingBackend<B, S>`
"verbatim," keyed by `sha3_256` of the canonical request. If two bridged agents from **different, possibly
mutually-distrusting operators** share one cache store, agent X submits request R and times the response — a
**hit (fast)** proves some co-resident agent (or X earlier) issued the identical R. That is a cross-tenant
request-content **existence oracle via pure timing**, no memory read required.

**Default (fail-closed): a genuinely separate `CachingBackend` instance per admitted agent — NOT a shared
store with a keyspace prefix.** Each `AdmissionEvent` already constructs that agent's own dispatcher stack
(§2.3: `AgentDispatcher<CachingBackend<McpServerBridge, S>>` is built *from* the admission record), so giving
each admitted agent its **own `CachingBackend<_, S>` instance with its own store `S`, its own capacity, and
its own lock** is the natural construction — no new machinery, just per-agent instantiation instead of a
shared singleton.

**Why separate instance, not keyspace-prefix — stated explicitly because the task asks.** A shared store with
an agent-identity key prefix (`sha3_256(agent_node_id ‖ canonical_request)`) *would* close the **content**
oracle (X's key and Y's key differ, so Y can never produce a hit on X's key). But it leaves two channels
open, because the store is still physically shared: (i) **eviction cross-talk** — Y filling the shared store
evicts X's entries, so X observes a miss where it expected a hit, a coarse "another tenant is busy caching"
signal; (ii) **lock/allocator contention** on the one shared store, a timing coupling of its own. A separate
instance has **no shared mutable state at all** between tenants — it removes the content oracle *and* the
eviction and contention channels — so it is the strictly stronger, fail-closed choice, and it is the one that
matches the default-deny discipline this design already follows everywhere else (deny-by-default red-line,
unrelaxable floor, no-unsandboxed-fallback). Keyspace-prefixing is retained only as **defense-in-depth
*within* the opt-in shared instance below** (so even a co-scoped store still binds identity unless the group
also opts into shared dedup).

**Opt-in exception — operator-co-scoped mutually-trusting agents only (never agent-declared).** The single
legitimate reason to share a cache is dedup benefit between agents the operator *knows* are mutually
trusting. SH-2 permits exactly that, and **only the operator can grant it**: a signed operator config axis
(M5 lattice — a `cache_group_id` enumerated value, in the same closed-domain family as §2.1's `config_axes`)
names a set of agents that share one `CachingBackend` instance. Membership is **checked at admission against
operator-signed config, never taken from the manifest** — an agent **cannot** place itself into a shared-cache
group, because self-selection into a co-tenant's keyspace is precisely the **confused-deputy** risk (an agent
opting to share cache with a victim to read its hit-timing). Absent an explicit operator co-scope, the answer
is always a separate instance. Fail-closed: unknown/absent `cache_group_id` ⇒ private instance, not shared.

### SH-3 — Poly-Network invariant as a BLOCKING RED-first acceptance criterion (COUNSEL §8 safeguard 4)

The consolidation already records the R1 §1(b) Poly-Network invariant as arc design law and proposes it for
canon as **CD-8** (`AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md` §5 Q2a / §7): *no `(Resource, Action)` scope may
authorize mutation of the `AnchorRoster`/genesis or the `RevocationSet`-drop path; anchor mutation is an
out-of-band operator act, never a capability-authorized frame; the delegation graph is acyclic w.r.t.
trust-anchor mutation.* Counsel §8.4 argues — correctly — that a canon-diff the operator *might* merge into
`ARCHITECTURE.md` later is **not** a mechanism, and that **B1 is exactly the blueprint that adds new
`(Resource, Action)` scopes to `scope.rs`** (`Resource::AgentBridge`, `Action::{AdmitAgent, InvokeAgent}`).
Therefore the invariant is hereby promoted, **on this blueprint specifically**, from a design note to a
**blocking, failing-test-first acceptance obligation**: it is added as **§4 acceptance criterion 10** and as
**Definition-of-Done item 8** below, and B1 **cannot be marked done** until it is GREEN.

**The precondition, stated as a hard gate:** the *first* migration step that touches `scope.rs` to add the
new `Resource`/`Action` variants (migration step 1) **must not land without** a RED-first test asserting that
**no `(Resource, Action)` pair introduced by B1 can reach any code path that mutates `AnchorRoster`
(`enroll`/`remove`, `roster.rs:205,222`), genesis (`load_genesis`, `node_id.rs`), or the `RevocationSet` drop
path (`RevocationSet::drop_anchor`, `revocation.rs:105`).** Anchor/revocation mutation remains an out-of-band
operator act only.

**Concrete test shape (an implementer cannot hand-wave past this).** Three layers, RED before GREEN:

1. **Behavioural — drive the real path, assert no mutation, for every new scope.** Define an explicit,
   exhaustive array `B1_NEW_SCOPES = [(AgentBridge, AdmitAgent), (AgentBridge, InvokeAgent)]` (the test fails
   to compile if a future B1 scope is added and not listed — the array is the enumeration contract). For each
   pair: construct a **fully valid, admittable** frame carrying that scope (correctly anchor-rooted delegation
   chain, both signature legs valid, so it genuinely *passes* `HybridGate::check` — the test is not relying on
   the gate to reject it). Thread a **mutable** `AnchorRoster` and a **mutable** `RevocationSet` into the real
   `admit(...)` **and** the real invoke/dispatch entry point. Snapshot both before (anchor-membership set +
   the revocation hash set) and assert **byte-identical after**: the admitted, authorized scope reached no
   handler that called `enroll`/`remove`/`drop_anchor`/`load_genesis`. Assert for *both* the admission path
   and the post-admission invoke path, since a scope could in principle be exercised at either.
2. **Negative control — prove the assertion has teeth (this is what stops the hand-wave).** Temporarily wire
   a **test-only poison handler** that *does* call `roster.remove(anchor)` when it sees a designated test
   scope, and assert the layer-1 before/after check **FAILS** against it. This proves the equality assertion
   actually detects roster mutation rather than being vacuously true. Then remove the poison wiring; layer-1
   must go GREEN. (RED→GREEN: the check must be shown capable of going red before it is trusted green.)
3. **Structural — the mutators are unreachable *by type*, not just unreached in this test.** Assert the
   capability-authorized surface can never obtain the `&mut` needed to mutate. Concretely: `admit(...)` and
   the invoke/dispatch handlers take the roster and revocation set as **`&AnchorRoster` / `&RevocationSet`
   (shared, immutable)**, never `&mut`; the only `&mut AnchorRoster` / `&mut RevocationSet` call sites in the
   tree are the genesis loader and the out-of-band operator/CLI enrollment+revocation path — **none of which
   consumes a `Capability` or `SignedFrame`**. A compile-time-adjacent guard (a `cargo tree`/grep done-check
   in the migration-step-2 style, or a call-graph assertion) records that the set of functions reachable from
   a capability-bearing input to `{enroll, remove, drop_anchor, load_genesis}` is **empty**. This makes the
   invariant a property of the type-plumbing, so it survives future refactors, not just a single test input.

Layer 1 is the falsifiable behavioural core; layer 2 guarantees it is not vacuous; layer 3 makes it durable.
All three ship RED-first with migration step 1 and are GREEN before B1 is done.

### SH-4 — New acceptance criteria (extend §4; all BLOCKING via the Definition of Done)

These are **added to** §4's list 1–9 (none of which is weakened) and folded into the DoD:

10. **[BLOCKING — SH-3] Poly-Network invariant holds by RED-first test.** The three-layer test of SH-3
    exists, shipped RED-first alongside the `scope.rs` variant addition (migration step 1), and is GREEN: no
    B1 `(Resource, Action)` pair reaches `AnchorRoster::{enroll, remove}`, `load_genesis`, or
    `RevocationSet::drop_anchor` from any capability-bearing input, proven behaviourally (layer 1), with a
    passing negative control (layer 2) and the immutable-borrow structural guard (layer 3). **B1 is a hard
    fail if this is not GREEN** — this is the counsel §8.4 precondition, promoted from canon-diff to gate.
11. **[SH-1 Guard A] Pre-crypto flood is throttled before crypto.** A burst of forged/garbage admission
    frames exceeding the coarse limiter's rate is dropped with `AdmissionThrottled`, and the test asserts
    `HybridGate::check`'s call-count stays at the bucket's grant (not the flood size) — i.e. the flood never
    reaches the gate or the nonce `Mutex`. The global bucket bounds total pre-crypto verify work independent
    of source count/spoofing.
12. **[SH-1 Guard B] Over-length chain dies at decode, zero crypto.** A frame presenting a delegation chain
    longer than `MAX_VERIFY_CHAIN_LINKS` (or claiming an absurd length) is rejected with `ChainTooLong` at
    TLV decode, provably before any `link.verify_signature()` runs (verify call-count = 0), and distinct from
    the `DEFAULT_MAX_AGENT_DEPTH = 3` dispatch cap (both bounds tested independently).
13. **[SH-2] Cache is per-agent by default; sharing is operator-only.** Two admitted agents issuing the
    identical canonical request get **no cross-agent cache hit** by default (separate instances — the second
    agent's identical request is a miss, verified by the store's own hit/miss counter or a timing-independent
    presence check, not by timing), and a shared cache exists **only** when an operator-signed `cache_group_id`
    co-scopes them; a manifest that self-declares a shared group is refused (the group is read from operator
    config, never the manifest).
