# BLUEPRINT P42 — MCP port + capability-scoped tool boundary (Skills-pattern discovery) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). Deepens the roadmap-index DoD for **P42** in
> `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.4 (lines 1026-1036)
> to the standard's depth. Structure/depth template: `BLUEPRINT-P40-agent-loop-tool-wiring.md` +
> `BLUEPRINT-P41-three-mode-ai-operation.md` (direct siblings, same directory). P42 is the
> third and final AGENT phase: P40 built the loop and ONE tool behind a `ToolPort` firewall,
> P41 proved the three-mode contract over it — P42 gives that proven pattern a **standard
> exterior (MCP)** and, as this blueprint's central architectural contribution, the
> **discoverability layer that lets the tool catalog grow without growing every prompt**
> (the Agent-Skills pattern, §1.2/§3.1).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree `/root/dowiz`, branch `main` (`f9b2eb9bb`), 2026-07-18. Code rows read from live
files this session; sibling-blueprint rows are design-time cites (their code lands with P40/P41
execution); web rows were researched this pass, not recalled.

| Claim | Fresh cite (this pass) | Status |
|---|---|---|
| IP-08 (MCP-server + agent-as-capability port) is 0% built: `grep -rn "SkillCard\|SkillRegistry\|McpToolServer" --include="*.rs" .` → **0 hits**; no `agent-mcp/`, `agent-facade/`, `agent-loop/` crates exist at repo root | live grep + `ls`, this pass | verified — P42 (and its P40 substrate) are unstarted; this is a first design pass, per the roadmap's own "no existing blueprint" note |
| `agent-adapters/` = PROTOCOL's mesh bridge for **FOREIGN** agents: "MCP's open-world string grammar NEVER enters the signed manifest… an operator-authored tool allowlist maps tool names to closed `(Resource, Action)` scopes (unmapped tools are a fail-closed drop)" | `agent-adapters/src/lib.rs:1-25` | verified — conventions source for P42's closed-enum scope mapping; NOT reused code (§4.4 non-conflation) |
| That bridge already speaks live MCP `tools/list` as a **client**: `self.transport.call("tools/list", json!({}))` | `agent-adapters/src/mcp.rs:50-56` | verified — proves in-repo MCP fluency exists; direction is opposite to P42's (it admits foreign tools IN; P42 serves our tools OUT) |
| Kernel ports convention: `ports/mod.rs` registers `llm` and `agent` with one-line docs; P40 adds `tool` beside them | `kernel/src/ports/mod.rs:1-9` | verified — P42 extends `ports/tool.rs`, creates no second ports home |
| P40's tool surface P42 re-exposes: `ToolPort`/`ToolSpec`/`ToolScope` with **closed** `ToolResource { OrderStatus }` / `ToolAction { Read }`, `agent-facade` re-exports only the two port surfaces, loop imports facade only | `BLUEPRINT-P40-agent-loop-tool-wiring.md` §2, §3.1 (sibling, this directory, read this pass) | design-time cite — P42 executes after P40 T1-T9 |
| P41's contract P42 must inherit: `AiMode { Off, LocalOffline, Connected }` default Off, no auto-escalation; degradation = typed `AssistantUnavailable`, never a hang | `BLUEPRINT-P41-three-mode-ai-operation.md` §2 (`:128`), §3.5 (sibling, read this pass) | design-time cite — §3.6 below is the inheritance proof |
| MCP spec (2025-06-18 revision): tool discovery = `tools/list` with **cursor pagination**; servers declaring the `listChanged` capability emit `notifications/tools/list_changed` on catalog change | modelcontextprotocol.io spec, "Tools" page (web-verified this pass) | verified — §3.3 targets this revision as-specified, no transport invention |
| Agent-Skills pattern (the operator-referenced discoverability model): capabilities ship as self-described bundles; agents load them in three stages — **discovery** (name + short description only, always in context), **activation** (full instructions loaded only when the task matches), **execution** (bundled resources loaded on demand). Rationale stated by the pattern's authors: the context window is the agent's cognitive space; overloading it degrades performance, and description quality directly determines routing accuracy | Anthropic engineering, "Equipping agents for the real world with Agent Skills" (web-verified this pass) | verified — §1.2 is this pattern applied to P40's tool catalog; the reasoning is cited, not asserted (operator requirement) |
| Named future tools that will grow the catalog (the scaling pressure P42 exists to absorb): P22 §11.4 pre-declares `draft_social_post` + `read_post_queue` as "a named FUTURE ToolPort extension, buildable only after P42's pattern lands"; P48 DoD-7 ties hub triage to P40's loop | `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §11.4 (read this pass); master roadmap §11 P48 DoD-7 (`:1282-1284`) | verified — P42's growth rule (§3.1) is what those consumers are gated on |
| `ureq` HTTP discipline (sync, no tokio) is the thrice-DECART'd client class for adapter crates | `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §6.3 (citing HARNESS §5 Decision 2 + `llm-adapters/Cargo.toml:12`) | cited — relevant only if the HTTP transport option in §3.3 is exercised; stdio needs no client |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P42 owns vs what it must NOT touch

### 1.1 Build items

| Item | Content |
|---|---|
| D-a | `SkillCard` (discovery projection of `ToolSpec`) + `SkillRegistry` trait in `kernel/src/ports/tool.rs` (P40's module, extended not rewritten) + the static registry impl in `agent-facade` |
| D-b | `AgentLoop` re-pointed at the registry: the loop composes its per-request toolset from resolved cards, not from a hardcoded tool reference — **zero behavior change at N=1**, proven by re-running P40's e2e test byte-identical |
| D-c | `agent-mcp/` crate: an MCP **server** (spec revision 2025-06-18, stdio transport) exposing the registry's tools — `initialize`, `tools/list`, `tools/call` — loopback/stdio only |
| D-d | Capability scoping fail-closed at the session boundary: a `GrantSet` fixed at server construction; `tools/list` serves only granted tools, `tools/call` outside the grant is a typed refusal — the roadmap DoD-2 negative test |
| D-e | Firewall, third instance: `agent-mcp` imports `agent-facade` only — `cargo tree -p agent-mcp --depth 1` shows no `dowiz-kernel`, committed red-proof, same choreography as P40 §3.1 (the roadmap's own §10.3 item 5 names this triple: KernelFacade / ToolPort / MCP layer — "Three instances, one pattern") |
| D-f | Three-mode inheritance proof (§3.6): the MCP surface serves **tools**, which are deterministic — `tools/call read_order_status` answers with Ollama stopped and `AiMode::Off`; the exterior can never hang on AI absence |

### 1.2 The central contribution — why "MCP exists" is not the phase

The naive P42 is a transport adapter: wrap `ToolPort` in JSON-RPC, done. That version becomes
unmanageable the moment the catalog grows, and the catalog WILL grow — P22 §11.4 already
pre-declares two tools gated on "P42's pattern", P48 DoD-7 ties hub triage to the loop, and
P43's content lanes will want read-tools of their own. Without a discovery layer, every added
tool is appended to every `ChatRequest.tools` array in every prompt forever: context cost grows
linearly in catalog size, and — per the Skills pattern's authors (§0 row) — an overloaded tool
context measurably degrades selection quality (more hallucinated calls, worse routing). P40
§4.2 already named this exact break point: "the OpenAI `tools` array shape is the break point
to re-examine when P42 grows the catalog (its blueprint's job)". This is that job.

**The design move (Skills pattern, applied):** separate what a tool costs at *discovery* from
what it costs at *activation*.

1. **Discovery tier — `SkillCard`:** every tool ships a card — name, one-line description
   (byte-capped), surface binding, declared scope. Cards are what a context carries by default:
   ~1 line each, so a 20-tool catalog costs ~20 lines, not 20 full schemas.
2. **Activation tier — `resolve()`:** the full `ToolSpec` (argument schema, the verbatim
   model-facing contract) is materialized into `ChatRequest.tools` only for tools selected for
   THIS request. At today's catalog size selection is trivial (§3.2's honesty note); the
   contract is what matters — tool authors write card + spec once, and the selection mechanism
   can harden later without touching any tool.
3. **Deterministic pre-filter — `Surface`:** each card declares which product surface it
   serves (owner / courier / customer / ops). The registry serves only the bound surface's
   cards. This filter is a closed-enum match, not model reasoning — the first cut is free and
   deterministic, before any LLM sees anything.

**Why this composes with P40's firewall instead of duplicating it (cited reasoning, per the
operator's requirement):** P40/P41's `ToolPort` + facade firewall answer *"what can a tool
reach?"* (namespace + closed enums — reachability math, P40 §4.1). The Skills layer answers
*"what does the agent carry and when?"* (context economics — progressive disclosure). They are
orthogonal axes of the same boundary: the firewall keeps the blast radius of any invocation
small; discovery keeps the *prompt* small so fewer bad invocations are attempted at all. The
Skills pattern's own three-stage rationale (discovery/activation/execution, §0 row) maps
one-to-one onto `cards()` / `resolve()` / `invoke()` — P42 is that pattern with Rust types
instead of markdown frontmatter. Together they are what lets the catalog grow: a new tool is
one card + one closed-enum variant + one scope mapping (§3.1's growth rule), never a prompt-tax
increase on unrelated requests and never a new hole in the firewall.

### 1.3 Anti-scope (each a review-rejectable smell)

1. **NOT foreign-agent admission, caging, or mesh exposure.** That is `AgentBridge` +
   `agent-adapters` (PROTOCOL's B1). P42 serves the LOCAL agent and local operator tooling
   only. Zero edits under `kernel/src/ports/agent/` and `agent-adapters/` (P40 §4.4's
   close-out check re-run verbatim). §4.4 states the direction argument.
2. **NOT tool-catalog expansion.** Still exactly ONE tool (`read_order_status`). P42 ships the
   *pattern* proven on one tool; P22 §11.4's two tools land later under P22's number through
   this registry. A second tool in a P42 PR is out of scope by definition.
3. **NOT transport invention.** MCP as-specified (2025-06-18 revision), stdio transport first.
   No custom framing, no WebSocket, no bespoke discovery protocol. The streamable-HTTP
   transport is a named later option (§3.3), loopback-bound if ever built.
4. **NOT a network service.** No non-loopback bind, ever, in this phase. Remote/mesh
   consumption of tools is PROTOCOL's lane (capability-certed, via `AgentBridge`), not a
   listener flag here.
5. **NOT an LLM meta-router.** No embedding index, no learned relevance model for card
   selection at today's catalog size — §3.2 names the threshold const at which a router
   becomes a reviewed follow-up. Building it now is premature optimization against an
   imagined workload (the P44 lesson, one section over).
6. **NOT agent-as-chat-over-MCP.** P42 exposes P40's *tools* to MCP clients; it does not
   expose the *loop* (an MCP client asking our agent to run a conversation). The loop stays
   user-initiated per P40's anti-scope 6; exposing it as a server-side prompt/agent endpoint
   is a separate future decision with its own autonomy implications — named, not scoped.
7. **NOT voice/multimodality.** IP-05's "superposition of intents" stays with DELIVERY's
   DZ-10 (deliberately deprioritized); P42 must not front-run it (roadmap anti-scope,
   restated).
8. **NOT new deps beyond the MCP surface's minimum.** stdio JSON-RPC needs `serde_json` (an
   adapter-layer crate — same discipline as `agent-facade`'s arg parsing, P40 §3.4). No SDK
   crate adoption without a DECART; §3.3's rejected-alternatives row covers it.

**Dependency posture:** depends on P40 (tool port + loop + the one tool) and P41 (the
three-mode contract §3.6 inherits). Needs no PROTOCOL P34/P35 completion: a local MCP endpoint
on a solo offline node is the baseline case (roadmap, verbatim). Blocks the ECOSYSTEM
consumers named in §1.2 — they wait for this pattern rather than importing anything deeper.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/ports/tool.rs — P42 EXTENSION of P40's module (extend, don't rewrite).
// Same firewall header discipline: ZERO network/HTTP/JSON/serde. ──────────────────

/// Which product surface a tool serves. Closed enum — the deterministic
/// discovery pre-filter (§1.2 tier 3). A tool with no surface is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface { Owner, Courier, Customer, Ops }

/// Discovery projection of a tool — the Skills-pattern "frontmatter".
/// This is what a context carries for EVERY tool; the full ToolSpec is
/// materialized per-request for SELECTED tools only (§1.2 tier 2).
#[derive(Debug, Clone)]
pub struct SkillCard {
    pub name: &'static str,        // == ToolSpec.name, the join key
    pub description: &'static str, // ≤ MAX_CARD_DESCRIPTION_BYTES — enforced, §3.1
    pub surface: Surface,
    pub scope: ToolScope,          // declared requirement (P40 type, reused verbatim)
}

/// The two-tier registry. Implemented in agent-facade; consumed by the loop
/// and by agent-mcp — both through this trait, ONE catalog (P2 CORRESPONDENCE).
pub trait SkillRegistry {
    /// Discovery tier: cheap, surface-filtered. Order is stable (registration order).
    fn cards(&self, surface: Surface) -> Vec<SkillCard>;
    /// Activation tier: full tool, by card name. None = never registered
    /// (a card without a resolvable tool is a registry-construction error, §3.1).
    fn resolve(&self, name: &str) -> Option<&dyn ToolPort>;
}

/// Discovery-tier size cap. A description that cannot fit one line is doing
/// activation-tier work in the discovery tier — refused at registration.
pub const MAX_CARD_DESCRIPTION_BYTES: usize = 200;

/// Named growth trigger (NOT built now): when the surface-filtered card count
/// first exceeds this, a relevance-selection pass between tiers becomes a
/// reviewed follow-up design. Below it, all surface cards resolve (§3.2).
pub const CARD_ROUTER_THRESHOLD: usize = 12;

/// Per-request ceiling on ACTIVATED tools (full specs in ChatRequest.tools).
/// The context-economics invariant: prompt tool-cost is O(min(N, this)), not O(N).
pub const MAX_ACTIVE_TOOLS: usize = 4;

// ── agent-facade/src/lib.rs — P42 additions ─────────────────────────────────────

/// The static catalog: (SkillCard, tool) pairs fixed at construction.
/// Construction PANICS (registry build time, not request time) on: duplicate
/// name, card/spec name mismatch, description over the byte cap — a malformed
/// catalog is unrepresentable at runtime.
pub struct StaticSkillRegistry(/* Vec<(SkillCard, Box<dyn ToolPort>)> */);

// ── agent-mcp/src/lib.rs — NEW crate (D-c). Imports agent-facade ONLY. ──────────

/// The set of ToolScopes an MCP session is granted. Fixed at server
/// construction (operator config), immutable for the session's life.
/// Empty set ⇒ the server serves an empty catalog and refuses every call.
#[derive(Debug, Clone, Default)]
pub struct GrantSet(/* Vec<ToolScope> — covers() checked per call */);

/// The server. One session, one grant, one surface, stdio framing.
pub struct McpToolServer<R: /* facade re-export of */ SkillRegistry> {
    registry: R,
    granted: GrantSet,
    surface: Surface,
}
// impl: pub fn serve_stdio(&self) -> Result<(), McpServeError>;
//       (blocking read-eval-write loop over stdin/stdout, JSON-RPC 2.0)

/// Typed server-side failures. Mapped onto JSON-RPC error responses by ONE
/// function (`to_rpc_error`) — the single place wire codes are chosen.
#[derive(Debug)]
pub enum McpServeError {
    Io(String),               // stdio broken — terminate the session, typed
    BadFrame(String),         // unparseable JSON-RPC — per-message error reply
    UnknownMethod(String),
    ScopeDenied { tool: String },       // grant does not cover the tool (D-d)
    UnknownTool { tool: String },       // not in the surface catalog
    Tool(String),             // rendered ToolError from the port (P40 vocabulary)
}

/// Wire constants — spec values, not inventions.
pub const MCP_PROTOCOL_VERSION: &str = "2025-06-18";
/// v1 declares NO listChanged capability: the catalog is construction-static,
/// so advertising change notifications would be a false capability claim.
/// (Honest spec compliance: omit, don't stub — §3.3.)
pub const TOOLS_LIST_CHANGED: bool = false;
```

**Rejected alternatives (DECART-style, one line each):** *cards as a separate
`kernel/src/ports/skill.rs` module* — rejected: the card is a projection of `ToolSpec`, and a
second module would split one tool vocabulary across two files (P40 §2's rejected-alternative
logic, same reasoning). *`GrantSet` as name-based allowlist (strings)* — rejected: names are
the model-facing label; scopes are the closed-enum authority — `agent-adapters/src/lib.rs:10-16`
already established "open-world strings never enter the closed grant" for foreign tools, and
our own exterior holds itself to the same rule in reverse. *Registry inside `agent-loop`* —
rejected: the loop is ONE consumer of the catalog; agent-mcp is the second; the trait must live
where both reach it without importing each other (the ports layer), or P42 re-creates the
"loop-private second grammar" P40 §2 already refused.

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 D-a — `SkillCard` + `SkillRegistry` + the growth rule

`StaticSkillRegistry::new(vec![(card, tool)])` validates at construction: unique names,
`card.name == tool.spec().name`, `description.len() <= MAX_CARD_DESCRIPTION_BYTES`,
`card.scope == tool.spec().scope`. Violation ⇒ panic at composition time (a config bug
surfaces at startup, never mid-request — unrepresentable-malformed-catalog, the §4.3
Self-Termination leg).

**The growth rule this encodes (the thing P22/P43/P48 are gated on):** adding tool N+1 =
1 `ToolResource`/`ToolAction` variant if genuinely new (a kernel-ports diff, type-reviewed —
P40 §4.1's compile-time gate), 1 `ToolPort` impl in the owning phase's facade lane, 1
`(SkillCard, tool)` registration line. Zero edits to the loop, zero edits to agent-mcp, zero
new prompt bytes for surfaces the tool doesn't serve. That is the whole point.

RED→GREEN: RED = `registry_serves_one_card` (cards(Owner) for the P40 tool returns exactly one
card whose fields mirror the spec) fails to compile until the types land; GREEN after.
**Adversarial:** (i) duplicate-name registration panics — asserted with `#[should_panic]`;
(ii) a 201-byte description panics; (iii) `resolve("transfer_money")` → `None` (never a
default tool, never a fuzzy match).

### 3.2 D-b — the loop consumes the registry (zero behavior change at N=1)

`AgentLoop` (P40 §2) currently holds `tool: &dyn ToolPort`. P42 re-points it:
`registry: &dyn SkillRegistry` + `surface: Surface`; step 2's request composes
`tools = cards(surface) → resolve each → ToolDecl` (all cards resolve while
`count ≤ CARD_ROUTER_THRESHOLD`, truncation at `MAX_ACTIVE_TOOLS` is a typed construction
error today since 1 ≤ 4 — the const exists so the ceiling is named, not discovered later).
Tool dispatch on a returned call goes through `resolve(name)` — `None` lands in the existing
`UnknownTool` observation arm (P40 §3.5 case 1, unchanged).

**Selection honesty (stated, not hidden):** with N=1 (and for the near-term catalog ≪ 12),
"selection" = serve all surface cards and let the model pick by name — exactly the Skills
pattern's discovery stage, where selection is model reasoning over descriptions and
description quality determines routing accuracy (§0 row). The two-tier CONTRACT is the
load-bearing deliverable; the relevance pass between tiers has a named trigger
(`CARD_ROUTER_THRESHOLD`) and is deliberately unbuilt until a real catalog crosses it —
building it now would be P44's premature-optimization mistake one lane over.

RED→GREEN: P40's `agent_reads_order_status_end_to_end` re-run against the registry-backed
loop — identical event sequence, identical outcome (the parity assertion is byte-level on the
`ToolDecl` array: one entry, same fields as before). RED = the refactor uncompiled; GREEN =
e2e green with zero assertion edits. **Adversarial:** a scripted backend calls a tool whose
card was served but which a hostile test registry refuses to resolve (test-only inconsistent
registry) → `UnknownTool` observation, loop recovers — proving the loop trusts `resolve()`,
never the card list, at dispatch time (activation is the authority; discovery is advisory).

### 3.3 D-c — `agent-mcp`: the MCP server, spec-as-specified

Methods served (2025-06-18 revision, stdio transport, JSON-RPC 2.0):

- `initialize` → protocol version + capabilities `{ "tools": {} }` — **no `listChanged`**
  declared (§2 const): the catalog is construction-static in v1; advertising a notification
  we'd never emit is a false capability. When a dynamic catalog ever exists, flipping this
  is the same reviewed diff that makes the registry dynamic.
- `tools/list` → the granted, surface-filtered catalog: each entry = name + description (from
  the card) + `inputSchema` (one string property, from `ToolSpec.arg_name` — generated by ONE
  function shared with the loop's `ToolDecl` serialization; two serializers would drift).
  Cursor pagination per spec: served single-page while catalog ≤ threshold (`nextCursor`
  omitted) — the axis is named in §4.2, not silently unbounded.
- `tools/call` → `GrantSet.covers(scope)` check FIRST (fail-closed, before any tool code),
  then `resolve(name)`, then `ToolPort::invoke` with the granted scope — the SAME invoke path
  the loop uses, one door (P2). Result → MCP content (`text`), `isError: true` on rendered
  `ToolError` (per spec: tool-level errors ride results so the model/client can see them;
  protocol-level errors — bad frame, unknown method, scope denial — ride JSON-RPC errors).

**Interior/exterior honesty (DECART-style):** the LOOP does not switch to calling tools over
MCP-on-loopback. In-process `ToolPort` calls stay (typed, zero serialization); MCP is the
exterior door for MCP-speaking clients (an operator's coding agent attached to the venue node
being the concrete first consumer). Routing our own loop through JSON-RPC to itself was
considered and rejected: it adds a serialize/parse round-trip and a process boundary to every
tool call and buys nothing — both doors already converge on the same registry + scope check +
`invoke`. The roadmap's "the agent calls tools via MCP" is satisfied at the boundary that
matters: ANY agent speaking MCP (including ours, if ever externalized) reaches tools only
through the capability-scoped catalog; no door bypasses the scope check. *Rejected dep:* an
MCP SDK crate — the served surface is 3 methods over stdio; hand-rolled `serde_json` framing
(~150 lines) with the spec's own JSON fixtures as tests beats a framework dependency for a
3-method server (same reasoning as P22 §6.4's hand-rolled multipart).

RED→GREEN (DoD-1): `mcp_read_order_status_end_to_end` — spawn the server binary with the
fixture registry (P40's `FixtureOrders`, ord-7 → InDelivery), drive it with REAL JSON-RPC
bytes over stdio (initialize → tools/list → tools/call), assert the result text contains
`IN_DELIVERY` and equals the direct `ToolPort::invoke` output byte-for-byte. RED against an
unimplemented `serve_stdio` stub. No live LLM involved — the exterior serves tools, not chat.

**Adversarial:** (i) a malformed frame (`{"jsonrpc":"2.0","method":`) → JSON-RPC parse-error
reply, session survives, next well-formed call succeeds; (ii) `tools/call` with arguments for
a nonexistent tool name → typed `UnknownTool` error reply, NOT a crash, NOT a fuzzy match;
(iii) a 10 MB single frame → bounded-read refusal (`BadFrame`), the read loop caps frame size
(const in impl) so a hostile client cannot balloon memory.

### 3.4 D-d — capability scoping, fail-closed, proven by negative test

The grant is per-server-construction (operator config), immutable, default **empty** —
`GrantSet::default()` grants nothing, and an empty grant serves an empty `tools/list` (the
discovery tier is itself scope-filtered: a client cannot even SEE tools it cannot call —
no capability oracle).

RED→GREEN (roadmap DoD-2 verbatim, deepened): `scope_denied_is_typed_and_runs_nothing` — a
server granted `{}` receives `tools/call read_order_status` → JSON-RPC error carrying the
`ScopeDenied` rendering, AND a spy `OrderStatusSource` records **zero** invocations (the P40
§3.1 observation-order discipline: refusal provably precedes the tool body). Second negative:
a grant covering a hypothetical different scope pairing (constructed via the P40 §3.1 spy
technique — closed enums forbid inventing variants, so the test asserts cover-check-first by
call counting). **Adversarial:** `tools/list` under the empty grant returns `[]` — asserted,
so discovery leaks nothing.

### 3.5 D-e — the firewall, third instance (committed red-proof)

Checks, verbatim from P40 §3.1's choreography with the crate name swapped:

```
cargo tree -p agent-mcp --depth 1 | grep dowiz-kernel     # → empty (no DIRECT dep)
grep -rn "dowiz_kernel" agent-mcp/src/                    # → empty
grep -rn "ports::agent\|agent_adapters" agent-mcp/src/    # → empty (§4.4 non-conflation)
```

Wrapped in `agent-mcp/tests/firewall.rs` as `#[test]`-wrapped process checks (the repo's
established self-auditing shape). RED-proof: temporarily add `dowiz-kernel = { path =
"../kernel" }` to `agent-mcp/Cargo.toml`, paste the failing output into the commit message,
remove it — the same committed-red ritual as P40. This closes the roadmap's §10.3 item 5
triple: KernelFacade (PROTOCOL) / ToolPort facade (P40) / MCP layer (this) — three instances,
one pattern, each with a red-proof on record.

### 3.6 D-f — three-mode inheritance (P41's contract, held at the exterior)

The inheritance is *stronger* than pass-through, and worth stating precisely: the MCP surface
serves **tools**, and P40's one tool is deterministic Rust — no `LlmBackend` anywhere in its
call path. Therefore:

- **Mode 1 (`AiMode::Off`):** `agent-mcp` constructs NO backend (it never imports the llm
  surface at all — only `ports::tool` re-exports; grep-checkable). `tools/call` works. The
  exterior is fully functional with zero AI — a property, not an accident.
- **Modes 2/3:** identical exterior behavior — the mode distinction lives entirely behind the
  loop, which agent-mcp does not serve (anti-scope 6).
- **Degradation:** with Ollama stopped (P41 §3.5's choreography), `tools/call
  read_order_status` still answers correctly — test `mcp_serves_tools_with_ollama_stopped`,
  run in P41's degradation harness lane. The roadmap's "the MCP surface must degrade exactly
  as gracefully" is exceeded: this surface has nothing to degrade, and the test PINS that
  (a future PR that couples tool serving to a live model turns this test red — that is the
  regression this row exists to catch).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6) — what can a hostile MCP client reach?

- **Reachable set = `granted ∩ surface-catalog`, then `ToolPort::invoke` under P40's types.**
  Every path into tool code goes through the one cover-check-then-resolve door (§3.3). With
  the P40 catalog, the worst any client can do is read the status of orders present in the
  configured source — writes stay UNREPRESENTABLE (`ToolAction::Read` is still the only
  variant; P42 adds no variant), money/auth/RLS/migrations stay out of namespace (facade
  re-export surface unchanged, §3.5 firewall).
- **Transport reachability:** stdio = the client is a local process the operator spawned;
  no listener exists. A network hole cannot be opened by config — there is no bind address
  parameter in v1's types (§2). Absence of a knob is the guarantee (unrepresentable, not
  forbidden).
- **Discovery leaks:** `tools/list` is grant-filtered (§3.4) — capability existence is not
  an oracle for unauthorized clients.
- **Residual risk, named honestly:** (i) a local-process adversary that IS granted the read
  scope can enumerate order statuses for ids it can guess — bounded by read-only + the
  source's id space; when P37's HTTP source replaces the fixture, P40 §4.1's residual
  transfers (capability-cert auth on the source, tracked there). (ii) Card descriptions are
  model-facing text — a malicious tool AUTHOR could prompt-inject via a description; today
  authorship = this repo's review process (cards are `&'static str` compiled in, not loaded
  from config/disk — supply-chain injection would require a reviewed source diff). That
  stays true until anyone proposes dynamic card loading, which this sentence marks as the
  re-review trigger.

### 4.2 Schemas & scaling axes (item 8)

Cards scale by catalog size: N ≤ `CARD_ROUTER_THRESHOLD` (12) serves single-page and
all-cards-in-context (~200 B/card ⇒ ≤ 2.4 KB — bounded); past it, the named router follow-up
and real `tools/list` cursoring activate (spec supports it; wired trivially since the list is
a Vec slice). `ChatRequest.tools` scales by ACTIVATED count, hard-capped `MAX_ACTIVE_TOOLS`
(4) — the P40 §4.2 break point is now governed, not open. `GrantSet` scales by scope count —
bounded by the closed enums' cross-product (currently 1×1). Frame size is read-capped (§3.3
adversarial iii). No axis is unbounded; each names its trigger.

### 4.3 Isolation / bulkhead (item 11), mesh (item 12), rollback (item 13), living memory (item 15)

- **Isolation:** agent-mcp is a separate crate and (at runtime) a separate process from
  anything order-flow-shaped; its crash severs one stdio session and nothing else. The order
  flow never calls it (dependency arrow points outward only — the P41 no-AI invariant's
  geometry, reused). Tool invocations under it inherit P40's `TOOL_TIMEOUT_MS`.
- **Mesh (item 12):** node-local, stdio, no transport dependency, no payload budget — NOT
  mesh-gossiped. Mesh/foreign exposure is `AgentBridge`'s lane, full stop (§4.4).
- **Rollback (item 13, vocabulary used precisely):** Self-Termination leg only —
  construction-time catalog validation makes a malformed registry unrepresentable at request
  time (§3.1), the frame cap + per-call timeout give each session a closed-form worst-case
  (`frame_cap` read + `TOOL_TIMEOUT_MS` per call). Mechanically reversible: delete the
  `agent-mcp` crate + the `ports/tool.rs` additions + the loop's registry re-point (D-b's
  parity test proves the re-point was behavior-neutral, so reverting it is too). No
  Self-Healing or Snapshot-Re-entry claim (nothing here is redundant or epochal).
- **Living memory (item 15):** no new durable channel. Tool calls through MCP produce the
  same per-invocation log entries P40 defined; durable recall remains the H1 harvest lane
  (P40 §4.3's deferral, inherited with its trigger).

### 4.4 Non-conflation with `agent-adapters` (hard constraint, checkable)

Same repo, two MCP dialects in opposite directions — the confusion hazard is real and named:
`agent-adapters/src/mcp.rs` is an MCP **client** that admits FOREIGN agents' tools IN through
operator-signed manifests (PROTOCOL's B1; `lib.rs:1-25`). `agent-mcp` is an MCP **server**
exposing OUR tools OUT to local clients. They share zero code and must continue to: the §3.5
grep (`ports::agent`, `agent_adapters` → 0 hits in `agent-mcp/src`) is in the firewall test,
and P40 §4.4's close-out (`git diff --stat <base>..HEAD -- kernel/src/ports/agent
agent-adapters` → empty) is re-run at P42 close-out. What IS shared is the *discipline* —
closed-enum scopes as the authority, open-world strings never entering a grant — adopted by
citation (§2 rejected-alternatives), not by import.

### 4.5 Linux-discipline verdict framework (item 9)

**ALREADY-EQUIVALENT:** the facade compilation firewall (third instance of the proven
pattern — one gate, one place). **EXTENDS:** the repo's fail-closed capability discipline
(`Caps` probes, grant allowlists) from flat capability flags to a two-tier
discovery/activation contract — with the fail-closed default preserved at both tiers (empty
grant serves nothing; unresolved card dispatches nothing). **REINFORCES:** no-silent-failure —
every exterior fault is a typed JSON-RPC error or typed `McpServeError`, never a dropped
frame.

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Extends §10.5.4's three P42 DoD lines with named tests; none is a prose checkbox.

| Item | RED (fails before) | GREEN (passes after) | Named test / check (permanent, item 17) |
|---|---|---|---|
| D-a registry | types absent — `registry_serves_one_card` uncompilable | card served; 3 constructor-panic adversaries green | `agent-facade/tests/registry.rs::{registry_serves_one_card, dup_name_panics, oversize_description_panics, resolve_unknown_is_none}` |
| D-b loop re-point | P40 e2e vs registry-backed loop uncompiled | P40's `agent_reads_order_status_end_to_end` green with ZERO assertion edits + `ToolDecl` byte-parity | same P40 test (unmodified — that's the point) + `agent-loop/tests/registry_parity.rs::{tooldecl_byte_parity, unresolvable_card_recovers}` |
| D-c MCP e2e (roadmap DoD-1) | `serve_stdio` stub → no reply | real JSON-RPC bytes: init → list → call → `IN_DELIVERY`, byte-equal to direct invoke | `agent-mcp/tests/e2e_stdio.rs::mcp_read_order_status_end_to_end` |
| D-d scoping (roadmap DoD-2) | grant ignored / untyped refusal | empty grant: list `[]`, call refused typed, spy source sees 0 invocations | `agent-mcp/tests/scoping.rs::{scope_denied_is_typed_and_runs_nothing, empty_grant_lists_nothing}` |
| D-e firewall (roadmap DoD-3) | red-proof: `dowiz-kernel` added → check fires (output committed) | tree + both greps clean | `agent-mcp/tests/firewall.rs::{no_direct_kernel_dep, no_agentbridge_import}` |
| D-f mode inheritance | (pins a property already true) coupling tool-serving to a live model turns it red | `tools/call` answers with Ollama stopped, `AiMode::Off` | `agent-mcp/tests/modes.rs::mcp_serves_tools_with_ollama_stopped` |
| exterior robustness | malformed-frame/oversize tests RED vs stub | 3 adversaries green, session survives frame faults | `agent-mcp/tests/adversarial.rs::{malformed_frame_survives, unknown_tool_typed, oversize_frame_refused}` |

Ledger obligation: one row in `docs/regressions/REGRESSION-LEDGER.md` — "MCP tool-serving must
stay AI-independent; guardrail: `mcp_serves_tools_with_ollama_stopped`" (red→green proof per
the ledger's standing ratchet rule). The firewall tests join the P41 `no-ai-firewall` CI job's
lane once that job exists (P41 owns the CI wiring — dependency named, not duplicated).

---

## 6. Benchmark plan (item 10) — existing harnesses only

Criterion harness convention inherited (`llm-adapters/benches/` + `BENCH_HISTORY.md` —
P40 §6's arrangement; agent-lane benches keep the ONE history-file convention). Three
measurements, budgets first:

1. `skills/cards_serve` — `cards(Owner)` on the 1-tool (and a synthetic 12-tool) registry.
   **Budget: ≤ 1 µs** — a Vec filter; the discovery tier must be free.
2. `skills/resolve_hit` — name → `&dyn ToolPort`. **Budget: ≤ 1 µs.**
3. `mcp/roundtrip_overhead` — one `tools/call` over stdio (spawned server, fixture tool)
   minus direct `invoke` on the same tool = serialization + framing + process-boundary cost.
   **Budget: ≤ 5 ms** — generous, honest (process round-trip), and load-bearing: it
   documents WHY the interior loop keeps in-process calls (§3.3's DECART) with a number
   instead of an assertion.

Telemetry: exterior `tools/call` outcomes ride the same per-invocation log vocabulary P40
defined; no second telemetry channel (standard item 19).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.3 item 5 + §10.5.4 P42
(the index entry this deepens) · `BLUEPRINT-P40-agent-loop-tool-wiring.md` (the tool port,
loop, firewall choreography, e2e test reused as the parity gate) ·
`BLUEPRINT-P41-three-mode-ai-operation.md` (`AiMode`, degradation harness §3.5, CI-job
ownership) · `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §11.4 (the first named consumers
of the growth rule) · `BLUEPRINT-P43-external-integration-ports.md` (sibling written this
pass; its future read-tools are catalog consumers, not P42 scope) · master roadmap §11 P48
DoD-7 (hub-triage consumer) · `agent-adapters/src/{lib,mcp}.rs` (conventions by citation,
non-conflation §4.4) · MCP spec 2025-06-18 "Tools" (tools/list, cursor pagination,
listChanged semantics — web-verified this pass) · Anthropic engineering, "Equipping agents
for the real world with Agent Skills" (the discovery/activation/execution pattern + the
context-economics rationale — web-verified this pass) · `docs/regressions/REGRESSION-LEDGER.md`
(§5 row). Memory files: `model-role-division-research-vs-reasoning.md` (n/a-check honored),
`test-integrity-rules-2026-06-27` + `never-bypass-human-gates-2026-06-29` (red-line classes,
inherited via P40 anti-scope) · `verified-by-math-2026-07-07` (§4.1 reachability stance) ·
`rust-native-bare-metal-decision-2026-07-14` (no SDK dep without DECART — §3.3) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (style discipline applied). Supersedes:
nothing — additive over §10.5.4's index entry.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (one concept, one primitive): ONE tool vocabulary (`ports/tool.rs`),
  ONE catalog (`SkillRegistry`), ONE invoke door with ONE scope check — reached by two
  consumers (loop, MCP exterior) that cannot drift because they share the serializer and the
  registry (§3.3). The card is a *projection* of the spec, never a second source of truth.
- **P4 POLARITY** (paired opposites made explicit): discovery vs activation are the two poles
  of one boundary — cheap/always vs full/on-demand — and the design names the axis and its
  crossing point (`resolve()`) instead of blending them into one flat tool list.
- **P7 GENDER** (no self-certification): the exterior is proven by REAL JSON-RPC bytes
  against a spawned server process (§3.3's e2e), not by calling handler functions in-process;
  the firewall by external `cargo tree`/grep process checks; the scope refusal by a spy
  counting zero tool-body invocations — every safety claim is certified by a different
  process than the one making it.

(Other principles are not load-bearing here and are not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (live greps for 0%-claims; web rows researched this pass and marked as such) |
| 2 DoD | §5 |
| 3 spec/event-driven TDD | §2 (types first), §3 per-item RED tests; D-b asserts P40's event SEQUENCE unchanged |
| 4 predefined types/consts | §2 |
| 5 adversarial tests | §3.1 (3 constructor panics), §3.2 (inconsistent registry), §3.3 (3 wire adversaries), §3.4 (2 negatives) |
| 6 hazard-safety as math | §4.1 (reachable set = grant ∩ catalog under closed enums; no-bind-knob unrepresentability) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (every axis + its trigger const) |
| 9 Linux discipline | §4.5 |
| 10 benchmarks+telemetry | §6 (3 budgets; log vocabulary reuse) |
| 11 isolation/bulkhead | §4.3 (process boundary, one-way dependency arrow, inherited timeouts) |
| 12 mesh awareness | §4.3 (node-local stdio, explicitly not mesh — AgentBridge's lane) |
| 13 rollback/self-heal vocabulary | §4.3 (Self-Termination leg only; closed-form session bound; mechanical reversibility) |
| 14 error-propagation gates | §3.1 (construction-time validation), §4.1 (closed enums as compile-time gate), §5 (typed-error tests) |
| 15 living memory | §4.3 (no second channel; P40's deferral inherited with trigger) |
| 16 tensor/spectral + eqc reuse | N/A-honest: no closed-form math in this phase; no decorative claim |
| 17 regression ledger | §5 (AI-independence row) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §2 (3 rejected alternatives), §3.3 (shared serializer, no SDK dep, no second budget/telemetry) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Execute in this order, ONLY after P40 T1-T9 and P41's C-d/C-e land (P42 re-exposes what they
proved). Repo: `/root/dowiz`. No live LLM is needed for any P42 test except re-running P40's
e2e in T3.

1. **T1 (D-a kernel half).** Extend `kernel/src/ports/tool.rs` with `Surface`, `SkillCard`,
   `SkillRegistry`, and the three consts (§2 verbatim). NO serde, NO new deps
   (`git diff kernel/Cargo.toml` → empty). Acceptance: `cd kernel && cargo test --lib` green.
2. **T2 (D-a facade half).** In `agent-facade/src/lib.rs`: add `StaticSkillRegistry` with
   construction-time validation (§3.1); register P40's `ReadOrderStatusTool` under a card
   (`surface: Surface::Owner`, description ≤ 200 B). Write
   `agent-facade/tests/registry.rs` (4 tests, §5 names) — panics RED-first via
   `#[should_panic]` against a validation-free stub. Acceptance: `cd agent-facade && cargo
   test` green.
3. **T3 (D-b).** Re-point `AgentLoop` at `&dyn SkillRegistry + Surface` (§3.2). Write
   `agent-loop/tests/registry_parity.rs` (2 tests). Acceptance: `cd agent-loop && cargo test`
   green INCLUDING P40's `e2e_read_order_status.rs` with **zero edits to its assertions**
   (live Ollama required for that one; `systemctl is-active ollama`).
4. **T4 (D-c).** Create `agent-mcp/` (repo root, standalone crate per the no-workspace
   convention): deps `agent-facade = { path = "../agent-facade" }` + `serde_json` only.
   Implement `GrantSet`, `McpToolServer::serve_stdio` (§2/§3.3): initialize, tools/list,
   tools/call, one shared spec→JSON serializer with the loop's `ToolDecl` path, frame-size
   cap. Acceptance: `cd agent-mcp && cargo build` + T5/T6 tests.
5. **T5 (D-e firewall — BEFORE the e2e, same ritual as P40).** Write
   `agent-mcp/tests/firewall.rs` (§3.5's three checks). Produce the RED-proof: temporarily
   add `dowiz-kernel` to `agent-mcp/Cargo.toml`, paste the failing output into the commit
   message, remove it. Acceptance: `cd agent-mcp && cargo test firewall` green.
6. **T6 (D-c/D-d/robustness tests).** Write `agent-mcp/tests/{e2e_stdio,scoping,
   adversarial,modes}.rs` (§5 names, 8 tests total) — spawn the real server binary, drive
   with real JSON-RPC bytes, spy source for the zero-invocation assertions. RED-first against
   the stub. For `modes.rs`: `systemctl stop ollama`, run, `systemctl start ollama`.
   Acceptance: `cd agent-mcp && cargo test` fully green, deterministic (run twice, identical).
7. **T7 (benches).** Add the three §6 benches; record numbers + budget pass/fail in the
   agent-lane `BENCH_HISTORY.md`. Acceptance: budgets met, numbers committed.
8. **T8 (close-out).** Run: `cd kernel && cargo test --lib`, `cd agent-facade && cargo test`,
   `cd agent-loop && cargo test`, `cd agent-mcp && cargo test`, plus
   `git diff --stat <base>..HEAD -- kernel/src/ports/agent agent-adapters` → **empty**
   (§4.4). Add the §5 ledger row. Verify every §5 DoD row; do not mark P42 done if any test
   was weakened, `#[ignore]`d, or tolerance-inflated. Hand the baton: P22 §11.4's two tools
   and P43's future read-tools now have their extension pattern — each lands under its OWN
   phase number via §3.1's growth rule (card + variant + registration), never by editing the
   loop or agent-mcp.

---

## 11. Pre-declared FUTURE capability — agentic browser (`browse_extract`), fence-first (2026-07-18 operator directive, same-day addendum)

Positioning first, so this section cannot be misread as scope creep on §1.3: **P42's PR still
ships exactly ONE tool** — anti-scope 2 stands verbatim. This section does for the operator's
"agentic browser" directive what P22 §11.4 did for the social tools: pre-declare a future
catalog entry with its scope vocabulary and its enforcement designed BEFORE any code exists.
With one addition specific to this capability, stated as a hard sequencing rule (§11.4): the
**fence lands before the engine**. The failure mode here is not a bug but a drift into a
pattern this project already explicitly declined (§11.0 last row) — so the boundary is designed
as mechanism (denylist in the grant path, unrepresentable verbs, CI grep-gates), never as a
paragraph asking the model nicely.

### 11.0 Ground-truth addendum (rows verified live this pass, 2026-07-18)

| Claim | Fresh cite (this pass) | Status |
|---|---|---|
| `browserbase/stagehand-rust`: official Rust SDK for Stagehand, self-declared "**ALPHA release and is not production-ready**"; architecture = REST client of a **Stagehand API server** (`STAGEHAND_BASE_URL`) + CDP WebSocket to **Browserbase cloud** sessions (`wss://connect.browserbase.com?sessionId=…&apiKey=…`); primitives `act`/`extract`/`observe`/`execute`; deps `tokio` + `reqwest` + `serde` (optional `chromiumoxide` feature for direct CDP); requires `BROWSERBASE_API_KEY`/`BROWSERBASE_PROJECT_ID`; **zero mention of MCP anywhere in the repo** | repo README, WebFetch this pass | verified — the "MCP-native Rust SDK" hope is FALSE; the custody + runtime findings drive §11.2's verdict |
| `browserbase/stagehand` (main repo): TypeScript-primary (83.7%) + a Python port; README documents `act`/`agent`/`extract` primitives; **no MCP server and no REST surface documented there**; setup is Browserbase-credential-shaped | repo README, WebFetch this pass | verified — MCP is not in the SDK family itself |
| `browserbase/mcp-server-browserbase`: the MCP interface DOES exist, one door over — official, self-hostable, **TypeScript (97.5%)**, exposes `act`/`extract`/`observe` among six MCP tools; **requires Browserbase cloud** (`BROWSERBASE_API_KEY` + `BROWSERBASE_PROJECT_ID`), no local-browser mode — "cloud browser automation" by its own description | repo README, WebFetch this pass | verified — protocol fit with our `agent-adapters` foreign-MCP gate is real; custody is what fails it (§11.2) |
| `browser-use/browser-use`: open-source **Python** library turning an LLM into a browser agent, ~89.1% on WebVoyager | operator-supplied research relayed this pass (not re-fetched) | provenance-marked — rejected in §11.2 on the runtime rule regardless of benchmark |
| **Session decline (internal provenance, this session, 2026-07-18):** browser automation designed to mimic human behavior specifically to evade platform anti-bot detection, for posting to social/messenger platforms that have (or are being integrated via) official APIs — **explicitly declined** | this session's own record | binding — inherited as hard anti-scope; §11.5's euphemism clause makes it drift-proof |

### 11.1 Sanctioned uses — a closed set, each named and bounded

| # | Use (intent variant) | What it is | What bounds it |
|---|---|---|---|
| U1 | `MenuImportAssist` | Owner onboarding: extract + structure the owner's OWN menu from a page they name (their old site, a legacy ordering system with no export API) into dowiz types | Owner-present, owner-initiated, once-per-onboarding. Read-only extraction. Authenticated pages: owner handoff in v0 (owner saves/pastes the page from their own browser), owner-LOCAL session attach in the engine future — **the tool never logs in anywhere** (§11.3: no credential field exists) |
| U2 | `SupplierCatalogRead` | Public supplier/ingredient catalog page with no API → structured prices for the owner's own purchasing decisions | Single capped fetch per page, honest self-identifying UA, page-budgeted. Reading a public page once at the owner's direction — not crawling, not a consumer platform's protected surface |
| U3 | QA of dowiz's OWN surfaces | Agent-driven exploratory/E2E testing of our interfaces | **Routed OUT of the product tool loop**: already served by the repo's Playwright E2E conventions as dev tooling. NOT a `ToolPort`, no new engine, nothing to grant — listed so nobody re-imports it as a product capability "for testing" |

An intent outside the enum is unrepresentable, not policy-refused (P40's structural move,
reapplied). **NOT a use case, under any wording:** posting, publishing, messaging, or any
repeated/scheduled interaction with any third-party platform — §11.5.

### 11.2 Engine verdict — Stagehand-rust confirmed as the only candidate, adoption DEFERRED with named triggers — **DEFERRAL SUPERSEDED same day by §11.7** (operator follow-up, 2026-07-18)

> *Supersession note (2026-07-18, second same-day pass): the operator directed that the
> cloud/no-MCP finding below not end in a deferral — §11.7 designs a NATIVE local engine that
> solves both findings instead of waiting on stagehand-rust. Findings 1–3 and the rejected
> alternatives STAND as the reasons that design exists; only the "wait for T-B1" posture is
> retired. §11.7.1 maps exactly what is superseded, revised, and retained. The v0 engine-free
> path below is unchanged and still lands first.*

Findings applied (from §11.0, not recalled):

1. **The MCP claim fails where it matters.** stagehand-rust itself has no MCP surface; the
   real MCP interface is a separate TypeScript server that is Browserbase-cloud-only. So the
   clean "MCP-native engine plugs into P42's port pattern" story does not exist today in any
   Rust artifact.
2. **Custody is the structural disqualifier, independent of maturity.** Both live routes (Rust
   SDK, MCP server) run the browser in Browserbase's cloud. For U1 that would place the
   owner's authenticated session inside a third party's rented browser — failing this
   section's own custody rule (the tool rides the owner's local context or gets handed bytes;
   it never exports a session) and the repo's sovereignty stance generally. Not alpha-jitter;
   a mismatch of shape.
3. **Runtime:** `tokio` + `reqwest` against the thrice-DECART'd `ureq` sync discipline (§0
   last row) — DECART-able only as an out-of-process sidecar, and not worth running while
   findings 1–2 stand.

**Rejected alternatives (DECART-style, one line each):** *Browser Use* — Python runtime for an
adapter-shaped need; the same rejection P43 §3.7 issued for Ghost-Downloader (a foreign runtime
and its supply chain are not priced into a benchmark number). *`mcp-server-browserbase` through
`agent-adapters`' foreign-MCP gate* — protocol fit is genuinely clean (operator-signed manifest
→ closed `(Resource, Action)` scopes, fail-closed drop; near-zero new plumbing) and is the
route to re-examine FIRST if a local-browser mode ever ships; rejected today on custody (cloud
browser) + a Node sidecar. *Writing our own CDP engine* — a browser-automation engine is a
product in itself; out of all proportion to U1/U2's shape.

**v0, engine-free — what U1/U2 actually need today:** `browse_extract` backed by a single
capped native HTTP GET (`ureq`-class, P43 §3.7's media-import fetch shape) + `LlmBackend`
structuring of the fetched page's visible text into typed menu/catalog rows — behind the SAME
contract, denylist, grant, and tests as any future engine (the engine is a swappable binding
behind the port; the fence is engine-independent). Authenticated or JS-only pages in v0: owner
handoff — the owner saves/pastes the page from their OWN browser and the tool structures bytes
it was handed. v0 is deliberately NOT "agentic browsing" (no `act`, no `observe`, no live DOM
session); it is owner-directed page extraction, and it plausibly covers most real onboarding
imports.

**Un-defer triggers for the engine (ALL required; fence-first sequencing on top):**

- **T-B1** — stagehand-rust exits alpha AND documents a local-browser mode (the optional
  `chromiumoxide` direct-CDP feature is the named watch-point). Re-verify the repo live;
  §11.0's rows go stale like any web rows.
- **T-B2** — an operator DECART approves the async-runtime boundary (engine as a separate OS
  process/sidecar; `tokio` never enters in-process adapter code), per
  `rust-native-bare-metal-decision-2026-07-14`.
- **T-B3** — demand: ≥3 real venue onboarding imports where v0 demonstrably failed (JS-only
  menu, handoff impossible). The capability follows recorded need, not novelty.

If T-B1 never fires, the honest end-state is v0 forever — stated now so nobody "helpfully"
adopts a worse-fit engine to close the gap.

### 11.3 The contract — predefined types (land with the owning phase's PR, never P42's)

```rust
// kernel/src/ports/tool.rs — additions at the OWNING phase's build time, via
// §3.1's growth rule (card + variant + registration). Shown here so the scope
// vocabulary is fixed before any code exists.

// ToolResource gains: WebPage      (new closed-enum variant; kernel-ports diff)
// ToolAction stays:   { Read }     — Submit/Fill/Click/Type/Post are NOT added.
//   A write to a third-party site is UNREPRESENTABLE in the grant vocabulary:
//   the same structural move as P40's read-only tool and P22 §11.4's absent
//   Publish/Approve variants. §11.5 pre-commits what adding one would require.

/// The ONLY sanctioned purposes (closed set — §11.1). Carried in every
/// invocation and logged; an un-enumerated purpose cannot be expressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseIntent { MenuImportAssist, SupplierCatalogRead }

/// NOTE what is ABSENT, deliberately: no credential field, no cookie/session
/// import, no schedule field, no submit payload, no UA override. Absence of
/// the knob is the guarantee (§4.1's no-bind-knob argument, reapplied).
pub struct BrowseRequest {
    pub intent: BrowseIntent,
    pub url: BrowseUrl,   // parse-validated: https only, no userinfo, no IP literal
    pub page_budget: u8,  // clamped to MAX_BROWSE_PAGES_PER_INVOCATION
}

/// Compiled-in, closed, label-aligned suffix match (host == entry or host ends
/// with "." + entry) — subdomains covered, lookalikes NOT accidentally caught.
/// NOT config, NOT disk-loaded: changing it is a reviewed source diff that
/// §11.4 leg 1 watches. MEMBERSHIP RULE (list maintained by rule, not vibes):
/// every platform family that P22 §11.5 / P43 route through an official
/// adapter lane is denied here — an official channel existing means the
/// browser is definitionally the wrong door to that platform.
pub const BROWSER_DOMAIN_DENYLIST: &[&str] = &[
    // Telegram (P22 Wave 0/1 official adapter; P43 E-b)
    "telegram.org", "telegram.me", "t.me",
    // WhatsApp (P43 E-d Cloud API adapter)
    "whatsapp.com", "whatsapp.net", "wa.me",
    // Instagram / Facebook / Messenger (P22 Meta lane)
    "instagram.com", "instagr.am", "facebook.com", "fb.com", "fb.me",
    "fb.watch", "messenger.com",
    // YouTube (P22 §12 Wave 2-Y)
    "youtube.com", "youtu.be", "youtube-nocookie.com",
    // SimpleX (P43 E-h sidecar adapter)
    "simplex.chat",
    // Remaining P22 §11.5 official-lane families
    "tiktok.com", "x.com", "twitter.com", "t.co", "viber.com",
];

pub const MAX_BROWSE_PAGES_PER_INVOCATION: u8 = 8;
pub const MAX_BROWSE_FETCH_BYTES: usize = 4 * 1024 * 1024; // 4 MiB per page

/// Honest self-identification — pinned by test. Leg 4's positive half:
/// the agent DECLARES itself; it never dresses as a human.
pub const BROWSE_USER_AGENT: &str = "DowizAgent/1 (+https://dowiz.dev/agent)";

/// Typed refusals — every fence hit is a value, never a silent skip.
#[derive(Debug)]
pub enum BrowseRefusal {
    DeniedDomain { host: String }, // denylist hit — ANY hop, before the engine
    BadUrl(String),                // scheme / userinfo / IP-literal / parse
    PageBudgetExhausted,
    OverFetchCap,
}
```

The catalog entry — the §2 `SkillCard` pattern applied to a fenced capability (description
186 B, under the cap):

```rust
SkillCard {
    name: "browse_extract",
    description: "Read-only extraction from an owner-named web page (menu import, \
                  supplier catalog). Platform domains with official adapters are \
                  refused; no login, no form fill, no posting, no scheduling.",
    surface: Surface::Owner,
    scope: ToolScope { resource: ToolResource::WebPage, action: ToolAction::Read },
}
```

**Grant example (D-d's mechanism, unchanged):** an operator constructing an owner-surface MCP
session that may browse grants exactly `GrantSet([ToolScope { WebPage, Read }])`.
`GrantSet::default()` still grants nothing — a session that was never granted browsing never
even SEES the card (§3.4's discovery-leak rule, inherited). The denylist check runs INSIDE the
tool's invoke path on top of the grant: the grant answers "may this session browse at all?";
the denylist answers "may this URL ever be browsed by anyone?" — two independent fail-closed
layers, neither substituting for the other.

### 11.4 The fence — four independent legs, each mechanical

**Sequencing rule (hard):** the fence tests + CI job land in the SAME PR as the first line of
browse code, before any engine integration — RED-first, like every gate in this family.

| Leg | Runtime mechanism | CI gate | Named tests |
|---|---|---|---|
| 1. Platform denylist | every navigation AND every redirect hop is label-aligned-suffix-checked BEFORE the fetch/engine sees the URL; hit ⇒ typed `DeniedDomain`, session aborts; unparseable ⇒ `BadUrl` (fail-closed). Redirect re-check is why shortener enumeration completeness is not load-bearing — `fb.watch → facebook.com` dies at hop 2 regardless | leg 1: the operator-named families' literals must be present in `kernel/src/ports/` whenever `browser-adapters/` exists | `denied_domain_refused_before_engine_runs` (spy engine records ZERO calls — P40 §3.1 spy discipline), `redirect_hop_to_denied_domain_aborts`, `subdomain_of_denied_domain_refused`, `lookalike_domain_not_denied` (matcher label-alignment, both directions), `ip_literal_and_userinfo_urls_refused` |
| 2. Read-only verb set | `ToolAction` has no write variant — writes are unrepresentable, not forbidden | leg 2: any `ToolAction::(Submit\|Fill\|Click\|Type\|Post\|Publish\|Approve)` token in `*.rs` fails the build (the shape pin; P22's `Draft` writes to OUR review queue and is deliberately not in the banned set) | compile-level — the grep IS the pin |
| 3. No autonomy / repetition | owner-initiated single invocation (P40 anti-scope 6 inherited); `page_budget` clamped; no Spool/queue/cron integration exists to hold a browse | leg 3: `browse_extract` may appear in `*.rs` only under `browser-adapters/`, `agent-facade/` (registration line), `kernel/src/ports/` | `page_budget_exhaustion_typed` |
| 4. No human-mimicry | pinned honest UA; no stealth code path exists to reach | leg 4: anti-detection vocabulary in browse lanes fails the build | `honest_user_agent_pinned` |

Credential absence is not listed as a leg because it is stronger than a leg: `BrowseRequest`
has no field to put a credential in, and the adapter crate holds no token store — unlike
P22/P43 adapters, which hold OFFICIAL tokens for platforms we integrate with by contract. That
asymmetry is the design: official doors get keys; the browser door gets none.

CI job text (normative now, wired by the owning phase; mirrors `no-courier-scoring` (E58) and
`no-pub-raw-matrix-hash` in `.github/workflows/ci.yml`):

```yaml
  # BLUEPRINT-P42 §11.4: agentic-browser fence. Four legs; each REDs independently.
  browser-fence:
    name: agentic-browser fence (P42 §11.4)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: "Leg 1: denylist literals present once the adapter exists"
        run: |
          if [ -d browser-adapters ]; then
            for d in telegram.org t.me whatsapp.com wa.me instagram.com \
                     facebook.com messenger.com youtube.com youtu.be simplex.chat; do
              git grep -q "\"$d\"" -- 'kernel/src/ports/' \
                || { echo "::error::BROWSER_DOMAIN_DENYLIST missing $d"; exit 1; }
            done
          fi
      - name: "Leg 2: third-party write verbs are unrepresentable (shape pin)"
        run: |
          ! git grep -nE 'ToolAction::(Submit|Fill|Click|Type|Post|Publish|Approve)' -- '*.rs'
      - name: "Leg 3: browse tool referenced only from its own lane"
        run: |
          ! git grep -n 'browse_extract' -- '*.rs' \
            ':!browser-adapters/**' ':!agent-facade/**' ':!kernel/src/ports/**'
      - name: "Leg 4: no human-mimicry / anti-detection vocabulary in browse lanes"
        run: |
          ! git grep -nEi 'stealth|humaniz|anti[_-]?bot|undetect|fingerprint[_-]?(spoof|mask|evad)|captcha[_-]?(solv|bypass)' \
            -- 'browser-adapters/**' 'kernel/src/ports/**'
```

Legs 2–4 are trivially green today and MAY be wired dormant ahead of the capability (they pin
the boundary in advance at zero cost); leg 1 self-activates when `browser-adapters/` appears.

### 11.5 Why drift into the declined pattern is a four-diff event

The declined pattern (§11.0 last row) requires, simultaneously: **(a)** reaching a
social/messenger platform surface — leg 1 denies every named family at every hop; **(b)**
writing (posting/submitting) — leg 2 makes the verb inexpressible in the grant vocabulary;
**(c)** repetition/scheduling — leg 3 confines the tool to single owner-initiated invocations
with no queue reachable; **(d)** not looking like an agent — leg 4 pins honest
self-identification. Re-creating the pattern therefore requires **four separate reviewed
diffs, each turning a named CI gate RED** in a PR a human reads. Same doctrine as courier
scoring (E58): prose asks nicely; gates refuse.

**Euphemism clause (binding on future blueprint authors, this one included):** any proposal
whose EFFECT is model-driven posting/messaging on those platforms through a driven browser
session is this declined pattern regardless of its name — "content syndication", "engagement
automation", "channel amplification", "growth tooling" included. The ONLY posting lane is
P22's official-API `SocialPoster` behind its §11.3 `PendingReview`-by-default review queue.
And should a write action on ANY third-party page ever be genuinely proposed (e.g. submitting
a supplier order form), it is pre-committed here to P22 §11.3's pattern: a new `ToolAction`
variant (kernel-ports diff + a deliberate leg-2 RED a reviewer must consciously update),
whose result lands `PendingReview` for the owner to approve — never an autonomous
form-submission loop, at any autonomy level (P22 A6's authority rule, inherited verbatim).

### 11.6 Obligations on landing + links (append-only: §5/§7/§9 above are not edited)

- DoD rows, bench budgets, and wave placement land with the owning phase's blueprint (this
  capability gets its own phase number at roadmap level, like every §3.1 growth-rule consumer
  — never P42's). The regression-ledger row text is fixed now: "Agentic browser stays
  read-only + platform-fenced; guardrails: `browser-fence` CI job +
  `denied_domain_refused_before_engine_runs`."
- Links added by this section: P22 `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md`
  §11.3/§11.4/§11.5 (approval-default, pre-declared-tool precedent, official channel homes =
  the denylist membership rule's source) · P43 §3.7 (foreign-runtime rejection precedent +
  the capped-fetch shape v0 reuses) · `.github/workflows/ci.yml` `no-courier-scoring` +
  `no-pub-raw-matrix-hash` (the grep-gate family `browser-fence` joins) ·
  `agent-adapters/src/{lib,mcp}.rs` (the foreign-MCP admission door considered and rejected
  on custody, §11.2) · `browserbase/stagehand-rust`, `browserbase/stagehand`,
  `browserbase/mcp-server-browserbase` READMEs (web, this pass) · the 2026-07-18 session
  decline (internal, §11.0).
- Re-verify rule: §11.0's web rows go stale like any others — T-B1 explicitly requires
  re-fetching the repositories, not trusting this table.

### 11.7 Native local-browser engine — second same-day addendum (2026-07-18 operator follow-up): design, not deferral

Operator follow-up, applied as directed: §11.2's cloud/no-MCP finding must not terminate in a
deferral — research further and design a NATIVE engine. This section is that design: a
first-party Rust MCP-server binary that drives a browser **locally**, which solves BOTH of
§11.2's structural findings (no MCP surface in any Rust artifact; cloud custody incompatible
with the owner-session use case) rather than working around one of them. Positioning, stated
before anything else: **this is a capability upgrade behind the existing fence, not a boundary
change** — §11.4's four legs, §11.3's types, §11.1's closed use set, and §11.5's euphemism
clause apply to this engine verbatim and unweakened (§11.7.4 restates this mechanically).
Nothing in this section may be read as reopening the declined pattern.

#### 11.7.0 Ground-truth addendum (rows verified live this pass, 2026-07-18, second pass)

| Claim | Fresh cite (this pass) | Status |
|---|---|---|
| In-process runtime discipline is **sync**: `LlmBackend` is a plain sync trait (`fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError>`, no async); `agent-adapters` self-describes as "A synchronous JSON-RPC 2.0 transport" with a byte-IO `RpcChannel` seam whose stated real bindings are "stdio pipe, streamable-HTTP, or an in-memory test double" | `kernel/src/ports/llm.rs:154-169`, `agent-adapters/src/transport.rs:1-24`, live read this pass | verified — corrects this pass's own tasking premise ("async-heavy conventions"), which live code disproves; the DECART below is judged against the REAL convention, and the sync client half of this design already has its seam in-repo |
| `mattsse/chromiumoxide`: async CDP client, "chromiumoxide only supports the tokio runtime"; can "launch a headless or full (non-headless) Chrome or Chromium instance **or connect to an already running instance**"; CDP bindings **auto-generated at build time** from Chromium's own PDL files (Rust rewrite of the chromium-tree Python parser, ~60K generated lines); MIT OR Apache-2.0; v0.9.1 released 2026-02-25; ~2.90M downloads | repo README + crates.io API, WebFetch this pass | verified — connect-to-running + codegen-from-PDL are the two load-bearing rows |
| `jonhoo/fantoccini`: W3C WebDriver client ("uses the WebDriver protocol to drive a conforming (potentially headless) browser"), tokio async, requires a separate running `chromedriver`/`geckodriver`; **no attach-to-existing-session/profile documented**; MIT OR Apache-2.0; v0.22.1 released 2026-02-28; ~3.50M downloads; active (Jon Gjengset) | repo README + crates.io API, WebFetch this pass | verified — most battle-tested of the three by adoption; the attach gap is structural to WebDriver, not a doc omission |
| `rust-headless-chrome` (`headless_chrome` crate): CDP client, **synchronous** API on plain threads, self-described Puppeteer equivalent; supports launch AND attach (`pub fn connect(debug_ws_url: String) -> Result<Self>` — "Allows you to drive an externally-launched Chrome process"); acknowledges unimplemented CDP areas (frames, file picker, touchscreen, network-condition emulation, XHR replay, HTTP Basic Auth, EventSource/WebSocket inspection); MIT; v1.0.22 released 2026-06-11; ~2.67M downloads; 142 open issues | repo README + docs.rs `Browser` page + crates.io API, WebFetch this pass | verified — the freshest release of the three; coverage gaps are self-declared, hand-maintained bindings |
| `chaser-oxide` (`ccheshirecat/chaser-oxide`, mirrored `0xchasercat/`, on crates.io): a chromiumoxide fork whose tagline is verbatim "**Undetectable, high-performance browser automation in Rust. Protocol-level stealth for Chromium**"; ships fingerprint profiles, a "Human Interaction Engine" with "physics-based mouse movements and realistic typing patterns"; sibling project `chaser-cf` self-describes as a "High-performance **Cloudflare bypass** library"; advertises testing against Cloudflare Turnstile/WAF, CreepJS, bot-detection suites | web search + repo taglines, this pass | verified — named in §11.7.2 as REJECTED on principle so no future reader adopts it in ignorance |
| Chrome ≥136 (2025): `--remote-debugging-port`/`--remote-debugging-pipe` are **ignored when pointing at the default user data directory**; a separate `--user-data-dir` is mandatory for CDP debugging (security hardening against cookie/credential exfiltration from the daily profile); Chrome team recommends Chrome for Testing for automation | developer.chrome.com "Changes to remote debugging switches to improve security", WebFetch this pass | verified — forces (and improves) §11.7.3's M-attach profile design |
| MCP spec: current **stable revision 2025-11-25**; a 2026-07-28 release candidate is announced to ship in ten days (protocol-level statelessness for HTTP, `initialize` handshake removed at the protocol layer) — **stdio transport is materially unaffected**; §0's "2025-06-18" row has aged two revisions | modelcontextprotocol.io spec + MCP blog RC post, web this pass | verified — the owning phase pins ONE revision repo-wide together with D-c's `agent-mcp` at its own PR time; this blueprint does not pre-pin |

#### 11.7.1 Supersession map — exactly what changes, what does not

| §11.2 element | Disposition | Reason |
|---|---|---|
| Findings 1–2 (no Rust MCP surface; Browserbase cloud custody fails U1) | **STAND** | They are the problem statement this design answers natively |
| Finding 3 (tokio vs sync discipline → "out-of-process sidecar only") | **RESOLVED BY CONSTRUCTION** | The engine IS a separate OS process (§11.7.3); tokio never enters in-process code — proven by cargo-tree red-proof, not by promise |
| "Adoption DEFERRED", trigger **T-B1** (watch stagehand-rust) | **RETIRED** | We no longer wait on a third party's roadmap; the operator directive replaces watching with building |
| Trigger **T-B2** (operator DECART for the async-runtime boundary) | **SATISFIED BY §11.7.2 + §11.7.3** | The DECART below is that comparison; the sidecar shape is the boundary; operator merge of the owning phase's PR is the sign-off `rust-native-bare-metal-decision-2026-07-14` requires |
| Trigger **T-B3** (≥3 real onboarding imports where v0 failed) | **RETAINED, re-scoped to scheduling** | v1 is now designed and buildable, but it is still BUILT when demand evidence arrives — the capability follows recorded need, not novelty (unchanged doctrine) |
| Rejected alternative "writing our own CDP engine — a product in itself" | **STANDS, and is not what this is** | §11.7.3 adopts a community-maintained CDP client whose protocol layer is generated from Chromium's own PDL; dowiz writes only the MCP server shell + fence glue. The rejection was of implementing CDP ourselves; that rejection is intact |
| v0 (capped native GET + `LlmBackend` structuring) | **UNCHANGED, still lands first** | v0 remains the first binding and the permanent fallback; the engine is a second binding behind the same port (§11.7.3) |
| §11.1 uses, §11.3 types, §11.4 fence, §11.5 clause | **UNCHANGED, verbatim** | §11.7.4 |

#### 11.7.2 DECART — native engine crate (falsifiable, re-runnable; three candidates on merit, one rejection on principle)

Technical comparison first — `chaser-oxide` is deliberately NOT a column here; its rejection
(below the table) is on principle and must not be laundered through a merit table.

| Criterion (falsifiable) | `chromiumoxide` 0.9.1 | `fantoccini` 0.22.1 | `headless_chrome` 1.0.22 |
|---|---|---|---|
| Protocol + depth | CDP direct — full generated surface (~60K lines from PDL) | W3C WebDriver — high-level, browser-portable, **less deep than CDP by design** (no request-interception/coverage-class features) | CDP direct — hand-maintained subset with self-declared gaps (§11.7.0 row) |
| **Attach to a RUNNING local browser** (U1's load-bearing property) | **YES** — "connect to an already running instance" (README) | **NO** — WebDriver creates its own session through a driver binary; no attach path exists in the protocol | **YES** — `Browser::connect(debug_ws_url)` (docs.rs, verified) |
| Extra processes to ship + version-match | none beyond Chrome itself | `chromedriver`/`geckodriver` — a second foreign binary whose version must track the browser's | none beyond Chrome itself |
| Runtime | tokio-only | tokio | sync, plain threads |
| Runtime-discipline fit | in-process: violates sync discipline → **sidecar mandatory**; inside the §11.7.3 sidecar binary: no conflict | same as chromiumoxide | only crate that COULD bind in-process — but fault isolation (§11.7.3) mandates the sidecar anyway, which neutralizes this sole advantage |
| CDP/protocol-drift absorption | **build-time codegen from Chromium's own PDL** — upstream protocol churn is absorbed by regeneration + an active maintainer, not by hand-porting | rides the W3C WebDriver **standard** (genuinely stable) — but drift re-enters via chromedriver↔Chrome version-matching ops | hand-maintained bindings; the self-declared unimplemented list IS the drift debt, visible |
| Maintenance signals (2026-07-18) | v0.9.1 2026-02-25, ~2.90M dl, active | v0.22.1 2026-02-28, ~3.50M dl, active, longest-established maintainer | v1.0.22 2026-06-11 (freshest), ~2.67M dl, 142 open issues |
| License | MIT OR Apache-2.0 | MIT OR Apache-2.0 | MIT |
| MCP-server-role conflict | none — the sidecar binary's single tokio runtime drives both the stdio JSON-RPC loop and the CDP websocket handler task (chromiumoxide's required shape) | none technically, but the driver binary adds a third process to the topology | none — sync engine calls from server loop threads |

**Verdict: `chromiumoxide`.** Two deciding factors, in order: **(1) U1 requires attaching to
the owner's already-running, already-authenticated LOCAL browser — WebDriver cannot express
that operation, which eliminates `fantoccini` for the use case this engine exists for**
(honestly noted: fantoccini is the most battle-tested of the three and would be a defensible
pick if only U2's launch-mode existed); **(2) of the two CDP crates, chromiumoxide's
build-time PDL codegen is the genuinely low-maintenance path for CDP drift** — the protocol
layer regenerates from Chromium's own definition files and an active community absorbs the
churn, whereas `headless_chrome` hand-maintains bindings with a self-declared gap list. Since
§11.7.3's fault-isolation argument mandates a sidecar process regardless of crate,
`headless_chrome`'s sync API — its one structural advantage under this repo's discipline —
buys nothing here. **Falsifier (re-run trigger):** if chromiumoxide breaks
connect-to-running, or goes >12 months without a release across a Chromium CDP break, this
table is re-run; named fallback = `headless_chrome` (attach verified), full retreat = v0
(which never left).

**`chaser-oxide` — REJECTED on principle, separated from merit.** As a chromiumoxide fork it
would nominally match or exceed the winner on every technical row above; it is rejected
BEFORE technical evaluation because its stated purpose is the category this project declined
(§11.0 last row): its own tagline is "Undetectable… Protocol-level stealth for Chromium," it
ships fingerprint-spoofing profiles and a "Human Interaction Engine" that mimics human mouse
physics and typing, and its sibling project is a self-described Cloudflare-bypass library
tested against Turnstile and CreepJS. That is browser automation engineered to defeat
platform anti-bot systems — §11.4 leg 4's exact target. It is named here, with this
reasoning, precisely so a future reader who independently discovers it does not adopt it
without knowing why it was excluded. Mechanical echo: its marketing vocabulary
(undetectable/stealth/fingerprint) is leg 4's grep list — importing it would turn the fence
RED by name alone. Not adoptable under ANY framing, including "it is technically more
capable"; capability is not the axis it fails on.

#### 11.7.3 The design — `dowiz-browser-mcp`, a first-party LOCAL MCP-server sidecar

```
agent loop (sync) ── ToolPort ──► browser-adapters lib (sync; fence leg 1 pre-check;
                                   MCP CLIENT over child stdio pipes — reuses
                                   JsonRpcTransport + a real stdio RpcChannel binding)
                                        │  spawn + JSON-RPC 2.0 / stdio
                              ═══ process boundary (the runtime firewall) ═══
                                        ▼
                          dowiz-browser-mcp bin (tokio + chromiumoxide;
                          MCP SERVER: initialize / tools/list / tools/call;
                          in-engine denylist re-check on every hop)
                                        │  CDP over LOOPBACK websocket only
                                        ▼
                          local Chrome/Chromium (M-launch or M-attach)
```

- **Crate layout (leg 3 holds without editing the gate):** the `browser-adapters/` directory
  owns TWO crates — `browser-adapters` (lib: sync, in-process, implements the `ToolPort`
  binding, speaks MCP as a client to the child) and `browser-adapters/mcp-server` (bin crate
  `dowiz-browser-mcp`: tokio + chromiumoxide, the engine). Both live under `browser-adapters/`,
  so §11.4 leg 3's path confinement applies unchanged. **Runtime firewall, fourth instance of
  D-e's choreography:** `cargo tree -p browser-adapters --depth 1` shows no `tokio` and no
  `chromiumoxide` — committed red-proof. The bin imports `agent-facade` only (for §11.3's
  types + denylist const re-exported through the one firewall door), never `dowiz-kernel`
  directly — same proof obligation as `agent-mcp`.
- **Being an MCP server, concretely:** JSON-RPC 2.0 over **stdio** serving `initialize`,
  `tools/list` (one card: `browse_extract`, §11.3's SkillCard verbatim), `tools/call` —
  exactly D-c's surface, hand-framed with `serde_json` like `agent-mcp` (anti-scope 3/8
  honored: no transport invention, no SDK adoption without DECART; the official `rmcp` SDK is
  the named re-examine option if the MCP surface ever grows past these three methods). Spec
  revision: pinned at the owning phase's PR to the SAME revision `agent-mcp` lands — one
  revision repo-wide (2025-11-25 stable today; the 2026-07-28 RC's stateless-HTTP changes do
  not materially touch stdio — §11.7.0 row). No chromiumoxide/server-role conflict: one tokio
  runtime in the bin drives the stdio read loop and the CDP handler task.
- **Contract reuse, not redefinition:** the tool served is §11.3's `browse_extract` with
  `BrowseIntent` / `BrowseRequest` / `BrowseRefusal` / `BROWSER_DOMAIN_DENYLIST` /
  `MAX_BROWSE_*` / `BROWSE_USER_AGENT` **verbatim** — this section adds zero fields, zero
  variants, zero constants to that contract.
- **Two engine modes, both LOCAL (the property Browserbase structurally could not offer —
  custody solved, not worked around):**
  - **M-launch** (U2 + JS-only public pages): the bin launches a managed headless Chromium
    (Chrome for Testing per the Chrome team's automation guidance) with a throwaway
    dowiz-owned `--user-data-dir`. No owner cookies exist in this mode at all.
  - **M-attach** (U1 authenticated): the owner starts a browser on THEIR machine with
    `--remote-debugging-port` bound to loopback and a **dedicated owner-import profile**
    (`--user-data-dir`), logs into their legacy system there once, and the tool attaches to
    the loopback CDP URL. Chrome ≥136 refuses CDP on the default profile (§11.7.0 row), so
    the dedicated profile is forced — and it is BETTER custody than the naive wish: the
    engine can only ever see the import profile, never the owner's daily browsing profile.
    The debug endpoint is treated as a credential: attach parse-validates **loopback-only**
    (non-loopback ws host ⇒ typed `BadUrl`, fail-closed, the same move as `BrowseUrl`), is
    never persisted, and dies with the session. Browser, profile, and CDP socket all live on
    hardware the owner or dowiz controls; no third party ever holds the session.
- **Extraction stays LLM-shaped, not selector-shaped (the maintenance-cost design decision):**
  the engine's job ends at "rendered DOM text (+ optional screenshot) of the owner-named
  page"; structuring into menu/catalog rows stays with `LlmBackend`, exactly as v0. There is
  deliberately no per-site selector library to rot.
- **Fault isolation (why sidecar even if the crate were sync):** a wedged or crashed browser
  must never hang the agent loop. Child process with kill-on-budget/timeout; engine down ⇒
  typed refusal in P41 §3.5's degradation family, never a hang.
- **v0 relationship:** v0's capped-GET binding remains first and is the permanent fallback;
  the engine is the SECOND binding behind the same port, selected only when v0 cannot serve
  (JS-only rendering, or M-attach need). T-B3's demand evidence schedules the build.

#### 11.7.4 Fence carry-over — unchanged, stated explicitly

Solving cloud custody and the missing MCP surface is a capability upgrade; **the anti-scope
boundary is identical before and after this section.** Mechanically:

- **Leg 1 (denylist), now enforced twice:** `browser-adapters` (lib) checks the request URL
  before the child ever spawns/receives it (§11.4's "before the engine sees the URL",
  unchanged), AND `dowiz-browser-mcp` re-checks **every navigation and redirect hop** inside
  the engine via CDP request interception on document requests — hops happen inside the
  browser, so hop-2 `fb.watch → facebook.com` now dies where it occurs. Same compiled-in
  `BROWSER_DOMAIN_DENYLIST`, same label-aligned matcher, same typed `DeniedDomain` abort.
- **Leg 2 (read-only verbs):** `ToolAction` still has exactly `{ Read }`. chromiumoxide
  exposes click/type/submit APIs — none is wired to any tool verb; a write on a third-party
  page remains unrepresentable in the grant vocabulary, and any future proposal for one is
  already pre-committed to §11.5's `PendingReview` pattern. The leg-2 CI grep is unchanged.
- **Leg 3 (no autonomy/repetition):** single owner-initiated invocation, `page_budget`
  clamped, no queue/cron reachable; the path-confinement grep holds verbatim because both new
  crates live under `browser-adapters/`.
- **Leg 4 (no human-mimicry):** M-launch pins `BROWSE_USER_AGENT`
  (`honest_user_agent_pinned`, unchanged). M-attach drives the owner's genuine browser at the
  owner's direction — the session's identity is truthfully the owner's own browser, and
  ALTERING it would be the misrepresentation; no stealth, humanization, or fingerprint code
  path exists in either mode, and leg 4's vocabulary grep covers both new crates' lanes
  (`browser-adapters/**` already matches them).
- **New named tests (additive, none replacing §11.4's):**
  `denied_domain_hop_aborts_inside_engine` (fixture redirect chain into a denied domain; CDP
  interception log proves zero frames committed past the hop),
  `attach_rejects_non_loopback_ws_url`, `no_tokio_in_browser_adapters_lib` (cargo-tree
  red-proof, committed), `engine_down_is_typed_refusal_not_hang`,
  `page_budget_kills_child_process`.

#### 11.7.5 What this actually commits to (maintenance honesty)

Adopting a native engine is real, ongoing work — named plainly, not priced as free:

1. **The Chromium release train** ships roughly every four weeks and CDP is explicitly not a
   frozen API. Mitigation is genuine but partial: chromiumoxide regenerates its protocol
   layer from Chromium's own PDL and an active maintainer absorbs the churn — dowiz's
   residual cost is riding crate updates on Chromium majors: low, but perpetual. This
   codegen-plus-community property is THE deciding maintenance factor of §11.7.2; the other
   two candidates each carry a heavier residual (hand-ported bindings; driver
   version-matching).
2. **A real browser in production is an ops surface:** memory ceilings, zombie processes,
   sandbox flags, headless-mode behavior changes. The sidecar + kill-on-budget bounds the
   blast radius; it does not erase the surface.
3. **Extraction robustness** is largely dodged by design (LLM structures visible text; no
   selector library), residual: pages that fight rendering.
4. **M-attach's owner workflow** (dedicated profile + debug flag on loopback) is real
   onboarding friction that owner-facing docs must own when the owning phase lands.
5. **Exit ramps are pre-named:** §11.7.2's falsifier (fallback `headless_chrome`, retreat
   v0). And the honest end-state from §11.2 survives in modified form: if T-B3's demand
   evidence never arrives, **v0 forever remains a legitimate outcome** — this section makes
   the engine buildable-by-design, not mandatory-by-momentum.

#### 11.7.6 Build items + DoD deltas (owning phase's PR, never P42's — §11.6 discipline unchanged)

| Item | Content | DoD (falsifiable, RED→GREEN) |
|---|---|---|
| BR-1 | §11.3 types + card land via §3.1's growth rule (unchanged from §11.6) | existing §11.4 named tests green |
| BR-2 | Fence-first: §11.4 legs + `browser-fence` CI in the SAME PR as the first browse line (sequencing rule unchanged) | each leg REDs independently when its guard is violated |
| BR-3 | v0 binding: capped native GET + `LlmBackend` structuring (§11.2, unchanged) | one real menu page → typed rows, offline test with canned bytes |
| BR-4 | `browser-adapters` lib: `ToolPort` binding, leg-1 pre-check, sync MCP client over child stdio (reuses `JsonRpcTransport` + a stdio `RpcChannel`) | `no_tokio_in_browser_adapters_lib` cargo-tree red-proof committed; `denied_domain_refused_before_engine_runs` (spy: zero child spawns) |
| BR-5 | `dowiz-browser-mcp` bin: stdio MCP server (initialize/tools/list/tools/call, revision pinned with `agent-mcp`), chromiumoxide engine, M-launch + M-attach, loopback-only attach validation, in-engine hop re-check | `attach_rejects_non_loopback_ws_url`; `denied_domain_hop_aborts_inside_engine`; `tools/list` serves exactly one card |
| BR-6 | e2e: JS-rendered fixture menu extracted via M-launch; engine-kill path | `page_budget_kills_child_process`; `engine_down_is_typed_refusal_not_hang` |

Regression-ledger row (extends §11.6's fixed text, which stays valid): "…guardrails:
`browser-fence` CI job + `denied_domain_refused_before_engine_runs` +
`denied_domain_hop_aborts_inside_engine` + `no_tokio_in_browser_adapters_lib`."

Links added by this section: `mattsse/chromiumoxide` README + crates.io ·
`jonhoo/fantoccini` README + crates.io · `rust-headless-chrome` README + docs.rs `Browser`
page + crates.io · `ccheshirecat/chaser-oxide` + `chaser-cf` taglines (rejection evidence) ·
developer.chrome.com "Changes to remote debugging switches to improve security" (Chrome ≥136
rule) · modelcontextprotocol.io spec 2025-11-25 + MCP blog 2026-07-28 RC post ·
`kernel/src/ports/llm.rs` + `agent-adapters/src/transport.rs` (live sync-discipline cites).
Re-verify rule inherited: every web row in §11.7.0 goes stale like any other; the owning
phase's PR re-fetches before build.
