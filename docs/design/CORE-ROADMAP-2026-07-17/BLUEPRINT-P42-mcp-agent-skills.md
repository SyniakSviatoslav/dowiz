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
