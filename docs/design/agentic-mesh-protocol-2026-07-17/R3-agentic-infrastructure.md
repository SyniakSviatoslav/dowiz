# R3 — Agentic Infrastructure & Multi-Agent Protocol Research

> Research stream 3 of 6 for the agentic-mesh synthesis (2026-07-17). Question: what exists today
> for agents to talk to each other, to tools, and to money — what is proven, what is hype, and what
> specific gap each piece would need to close to serve a sovereign decentralized mesh
> (crypto-verified, capability-trust, PQ-secure, zero-external-dependency-by-default per M6).
> Planning artifact only. All adoption/maturity claims are cited; secondary-source figures are
> flagged as reported.

Kernel grounding (verified in this worktree): `kernel/src/ports/llm.rs` (`LlmBackend` trait +
fail-closed `Caps`, zero-HTTP compile firewall, M5 backend-as-config), `llm-adapters/src/dispatch.rs`
(`TokenBucket`-bounded dispatcher, typed `BudgetExceeded`, degrade-closed, H1 harvest rows priced by
`usage.total_tokens`), `kernel/src/token_bucket.rs` (F33, verified-by-math refill bound),
`kernel/src/isolation/microvm.rs` (`SandboxTier::{WasmComponent, NativeProcessRequiresKvm}`,
fail-closed KVM probe).

---

## 1. MCP — agent-to-tool, genuinely won; not agent-to-agent, not identity

**What it standardizes.** MCP (Anthropic, Nov 2024) is a JSON-RPC 2.0 contract between a *client*
(the agent host) and a *server* (a tool/data provider): discovery and invocation of three
primitives — tools, resources, prompts — plus transport (stdio / Streamable HTTP) and OAuth 2.1
authorization ([spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25),
[GitHub](https://github.com/modelcontextprotocol/modelcontextprotocol)). It is the Language Server
Protocol move applied to AI tooling.

**Adoption is real, not announcement-stage.** Adopted by Anthropic, OpenAI, Google DeepMind,
Microsoft, AWS within ~a year ([Wikipedia](https://en.wikipedia.org/wiki/Model_Context_Protocol));
donated Dec 2025 to the Agentic AI Foundation under the Linux Foundation (co-founded Anthropic,
Block, OpenAI). Third-party trackers report ~97M monthly SDK downloads and >10,000 public MCP
servers as of early 2026 ([digitalapplied](https://www.digitalapplied.com/blog/mcp-adoption-statistics-2026-model-context-protocol)
— aggregator figures, treat as order-of-magnitude). The
[2026-07-28 release candidate](https://blog.modelcontextprotocol.io/posts/2026-07-28-release-candidate/)
moves to a **stateless core** (plain HTTP load-balancing), a formal **extensions framework**,
**tasks** for long-running operations, and OAuth/OIDC hardening — i.e., the protocol is maturing
toward enterprise fleets, not toward decentralized trust.

**What it does NOT solve.** MCP is agent-to-tool inside one trust domain. The security literature
is blunt: MCP has no protocol-level mechanism cryptographically binding a tool's identity to its
provider — identity resolves via non-unique names and client heuristics
([arXiv 2512.03775](https://arxiv.org/pdf/2512.03775), a scale study of cryptographic misuse in MCP
servers; also [arXiv 2511.20920](https://arxiv.org/pdf/2511.20920)). A server authenticates its
endpoint (TLS/OAuth) but not *the code running behind it*; there are no signed execution records
([Diagrid](https://www.diagrid.io/blog/why-mcp-gateways-are-not-enough),
[CSA best-practices](https://labs.cloudsecurityalliance.org/agentic/agentic-mcp-security-best-practices-v1/)).
Registry poisoning / server substitution is a live threat class.

**Verdict for the mesh.** Borrow the **shape**, not the trust model. The tool-manifest +
typed-invocation + capability-discovery contract is the proven pattern for "any node bridges in its
own agent," and the kernel is already isomorphic to it: `Caps { chat, embed, rerank, tool_calling }`
is a fail-closed capability manifest; `LlmBackend` is a discovery-then-invoke seam. A mesh-native
"agent bridge manifest" that is *signed* (ML-DSA ⊕ Ed25519 hybrid, per the bebop2 stance) and
*capability-scoped* would be MCP's discovery contract with the missing identity layer added.
**Gap to close:** no crypto identity, no execution attestation, PQ nowhere in sight, and OAuth 2.1
assumes an external IdP (violates M6). Do not adopt MCP wire-format wholesale; adopt its
three-primitive discovery grammar behind the mesh's own signed envelope.

## 2. A2A and the agent-to-agent protocol field — one credible standard, trust explicitly out of scope

**What A2A standardizes.** Google's Agent2Agent protocol (announced Apr 2025, donated to the Linux
Foundation 2025-06-23): **Agent Cards** (JSON metadata advertising capabilities/skills/endpoints,
discovered at runtime), a **task lifecycle** (delegation, status, artifacts), and structured
message exchange over HTTP/JSON-RPC/SSE ([IBM overview](https://www.ibm.com/think/topics/agent2agent-protocol),
[Linux Foundation one-year announcement](https://www.prnewswire.com/news-releases/a2a-protocol-surpasses-150-organizations-lands-in-major-cloud-platforms-and-sees-enterprise-production-use-in-first-year-302737641.html)).

**Maturity, honestly.** Real consolidation happened: IBM's ACP (BeeAI) merged into A2A under the
Linux Foundation in Aug 2025 ([LF AI & Data](https://lfaidata.foundation/communityblog/2025/08/29/acp-joins-forces-with-a2a-under-the-linux-foundations-lf-ai-data/)),
so the field has one flagship instead of four. At the April 2026 one-year mark: 150+ member
organizations, 22k+ GitHub stars, SDKs in 5 languages, v1.0 stable with signed Agent Cards
([AIwire](https://www.hpcwire.com/aiwire/2026/04/09/linux-foundation-a2a-protocol-marks-one-year-with-broad-enterprise-and-cloud-adoption/)).
That said, "150 organizations support" is a membership metric, not a deployment metric; production
evidence is thin relative to MCP's server ecosystem, and most public A2A material is still
platform-vendor integration guides. The rest of the field is weaker: ANP is near-final draft,
Agora is concept-stage ([Katonic taxonomy](https://www.katonic.ai/blog/agent-protocols);
[arXiv 2602.11327](https://arxiv.org/pdf/2602.11327) threat-models all four).

**Trust: the honest part of the spec is that it punts.** A2A is "a communication protocol first" —
trust establishment, credential handling, and identity verification sit **outside its scope by
design** ([Tyk guide](https://tyk.io/learning-center/a2a-security-the-developers-complete-guide/)).
v0.3+ defines `AgentCardSignature` — a JWS (RFC 7515) over JCS-canonicalized JSON (RFC 8785) — but
signing is **optional and unenforced**, so card spoofing via DNS/CDN compromise remains routine
([analysis](https://dev.to/kanywst/a2a-protocol-auth-taken-apart-why-the-spec-is-thin-and-where-that-leaves-holes-22ii)).
Auth between agents is delegated to OAuth/OIDC bearer tokens — centralized IdPs again. Research
proposals exist to fill this (e.g. [AIP, arXiv 2603.24775](https://arxiv.org/pdf/2603.24775),
verifiable delegation across MCP+A2A; DID-based cards for no-single-authority settings) but they
are papers, not deployments.

**Verdict for the mesh.** Borrow: (a) the **Agent Card** as the discovery artifact — but make
signature *mandatory* and the signature a mesh capability-certificate (signed capability, never
reputation, per the OpenDDE stance), (b) the explicit **task lifecycle** state machine — it maps
cleanly onto the kernel's event-sourced `decide` path and makes delegation auditable. **Gap to
close:** A2A's entire trust plane (optional JWS, OAuth IdPs, DNS-anchored discovery) must be
replaced with mesh-native signed capabilities; JWS today is ES256/RS256 — no PQ signature suite is
even drafted for Agent Cards; and JWTs cannot be attenuated offline, whereas a mesh wants
delegatable, narrowing capabilities.

## 3. Multi-agent failure modes — measured, and the verification gap is real

The failure literature graduated from anecdote to measurement in 2025-26:

- **MAST** ([arXiv 2503.13657](https://arxiv.org/abs/2503.13657)): first empirical taxonomy — 7
  frameworks, 200+ tasks, 1600+ annotated traces (κ=0.88). 14 failure modes in 3 classes:
  **specification issues 41.8%**, **inter-agent misalignment 36.9%**, **task verification
  21.3%**. I.e., over a fifth of observed failures are precisely "nobody checked what an agent
  claimed."
- **Infinite loops** ([arXiv 2607.01641](https://arxiv.org/abs/2607.01641)): "Infinite Agentic
  Loops" arise from framework feedback paths with no effective bound; static analysis (IAL-SCAN)
  over 6,549 agent repos confirmed 68 IAL defects across 47 projects (91.9% precision). AutoGPT's
  documented history is the canonical case: natural-language "is the goal complete?" checks default
  to "more work needed" forever ([case study](https://github.com/vectara/awesome-agent-failures/blob/main/docs/case-studies/autogpt-planning-failures.md)).
  Cost blow-ups are documented: one widely-cited July 2025 report describes a Claude Code recursion
  loop consuming 1.67B tokens in 5 hours ([secondary source](https://sanj.dev/post/llm-cost-control) —
  anecdote, but the mechanism is uncontested).
- **Cascading error/hallucination**: small early-stage errors propagate silently and compound into
  confident wrong outputs in agentic RAG pipelines ([CHARM, arXiv 2606.04435](https://arxiv.org/html/2606.04435));
  recursive context reuse amplifies error cascades in collaboration graphs
  ([arXiv 2603.04474](https://arxiv.org/html/2603.04474v1)). One honest counter-datapoint:
  claim-level tracking across 3-agent chains found downstream agents can *reduce* hallucination
  scores (0.422→0.272) when the chain functions as review ([arXiv 2606.07937](https://arxiv.org/abs/2606.07937))
  — cascades amplify *or* filter depending on topology; the mesh should treat chain topology as a
  design variable, not assume decay.

**The load-bearing finding: no framework verifies agent claims.** AutoGPT-lineage, CrewAI,
LangGraph, AutoGen — all are uniformly trust-the-text-output. The strongest guarantees any of them
offer are structural (typed state channels, step caps), never cryptographic; none binds "agent X
produced output Y from input Z via computation C" to anything checkable. The nearest real
mechanisms live *outside* agent frameworks:

- **TEE remote attestation** is the only production-feasible "verify the compute" path in 2026:
  AWS Nitro, Intel TDX, NVIDIA Hopper confidential computing; deployed by crypto-adjacent projects
  (Phala, Marlin Oyster, Automata) ([overview](https://eco.com/support/en/articles/14796365-tees-for-ai-agents-verifiable-compute);
  [Attestable Audits, arXiv 2506.23706](https://arxiv.org/html/2506.23706v1)). Cost: hardware trust
  in Intel/AMD/NVIDIA/AWS + side-channel history. Niche, but real.
- **zkML is not viable**: proving inference remains orders of magnitude slower than running it
  (zkLLM: thousands of times slower) ([survey](https://www.blockchain-council.org/blockchain/verifiable-ai-inference/)).
- **Signed tool receipts** — sign the *inputs/outputs of deterministic tool calls* rather than the
  model computation — is the pragmatic middle, argued explicitly in
  [arXiv 2603.10060](https://arxiv.org/pdf/2603.10060) ("Tool Receipts, Not Zero-Knowledge Proofs").

**Verdict for the mesh.** This is the gap the mesh design fills that nothing existing does. The
kernel's event-sourced core + drift gate + signed event log is already the right substrate: don't
try to prove the LLM's computation (zkML dead end, TEE = imported hardware trust); prove the
**envelope** — signed request, signed tool receipts, signed output, hash-chained into the event
log, checked by deterministic gates (schema, budget, drift ρ) before commit. That is exactly the
"verification" class MAST measures as absent, made cryptographic.

## 4. Agent economies — x402 is real traffic with a centralized verifier; AP2's mandate pattern is worth stealing

- **x402** (Coinbase, 2025): revives HTTP 402 — a server answers `402 Payment Required` with terms;
  the agent pays in stablecoin (USDC on Base/Solana) and retries; a **facilitator** verifies and
  settles ([Coinbase docs](https://docs.cdp.coinbase.com/x402/welcome), [x402.org](https://x402.org/)).
  Governance moved to a foundation (Coinbase+Cloudflare, Sept 2025; Linux Foundation, Apr 2026,
  with 22 members incl. Visa, Mastercard, Stripe, AWS) ([BlockEden](https://blockeden.xyz/blog/2026/03/05/x402-foundation-ai-payment-internet/)).
  Traffic is genuinely beyond demo — reported ~69k active agents and 165M cumulative transactions
  at ~$50M cumulative volume by late Apr 2026 ([RZLT](https://www.rzlt.io/blog/agentic-payments-2026-x402-explainer));
  note another source claims ~$600M *annualized* ([Sherlock](https://sherlock.xyz/post/x402-explained-the-http-402-payment-protocol))
  — the figures are inconsistent; what is safe to say is: high transaction count, tiny average
  ticket (~$0.30/txn implied — machine micropayments, not commerce). **Trust model:** settlement is
  on-chain, but verification runs through facilitators, and Coinbase's facilitator holds ~70%
  share — a de-facto single point of failure/censorship ([Datawallet](https://www.datawallet.com/crypto/x402-protocol-explained),
  [QuestFlow](https://blog.questflow.ai/p/x402-at-a-crossroads-infrastructure)). Concrete attacks
  are already catalogued ([Five Attacks on x402, arXiv 2605.11781](https://arxiv.org/pdf/2605.11781)).
- **AP2** (Google, Sept 2025, 60+ partners incl. Mastercard, PayPal, Amex): three signed
  **Mandates** — Intent, Cart, Payment — carried as W3C Verifiable Credentials, forming a
  cryptographic chain from human intent to settlement ([Google Cloud](https://cloud.google.com/blog/products/ai-machine-learning/announcing-agents-to-payments-ap2-protocol),
  [spec site](https://agentpaymentsprotocol.info/docs/introduction/)). Maturity: launch/pilot
  stage — partner list, spec, reference code; no independent production-volume evidence found.

**Verdict for the mesh.** x402 proves demand for machine-to-machine metered payment and proves the
flow works at volume; its gap is the facilitator (centralized verifier — exactly what the mesh
refuses) and chain dependence (violates M6 zero-external-dependency-by-default; on-chain settlement
is an *optional port*, never core). AP2's **signed mandate chain is the single best pattern in this
whole survey for the mesh**: it is capability-delegation-as-VC — human intent → bounded agent
authority → verifiable execution record — structurally identical to the mesh's signed-capability
stance, and it demonstrates large payment incumbents accepting that shape. Gap in both: signature
suites are classical (ES256/secp256k1); no PQ story anywhere in agent payments.

## 5. Budget bounding — the kernel's TokenBucket is at (in one respect ahead of) best practice

Industry consensus post-AutoGPT is exactly the kernel's posture: **enforce resource bounds in
deterministic infrastructure, never in prompts.** What frameworks actually ship: LangGraph — a
configurable step/recursion bound and tool-call caps; CrewAI — `max_iter` (default 25),
`max_execution_time`, `max_rpm` ([framework comparison](https://www.speakeasy.com/blog/ai-agent-framework-comparison/)).
These are *step-denominated* guards. Notably, **neither ships token/cost-denominated budget
enforcement natively**; practitioners route spend caps through gateways or SDK-level hard caps
([SupraWall](https://www.supra-wall.com/en/learn/ai-agent-runaway-costs),
[LeanOps](https://leanopstech.com/blog/agentic-ai-cost-runaway-token-budget-2026/) — agent workloads
reported at ~50x chat token consumption). LangGraph itself has shipped loop bugs that run to the
recursion limit ([issue #6731](https://github.com/langchain-ai/langgraph/issues/6731)) — the bound
is the last line, and it fires in practice.

Against that baseline, `TokenBucket` (F33) + the dispatcher's typed `BudgetExceeded` is *stronger*
than mainstream framework practice on three axes: it is token/cost-denominated (not step-count),
it has a falsifiable mathematical bound (grants ≤ capacity + rate·elapsed over any window), and it
is degrade-closed (refuse, never silently queue-then-downgrade). Worth adding from the literature,
in order of value: (1) **hierarchical budgets** — per-peer / per-capability / per-task envelopes
that nest, so one remote agent exhausting its envelope cannot starve the node (this is the mesh
version of "ten looping agents = $5,000"); (2) **static loop analysis** on bridged agent graphs
(IAL-SCAN shows framework-independent loop-dependence analysis is feasible) as an admission-time
complement to runtime buckets; (3) **stopping rules distinct from budgets** — a budget bounds
damage, it does not detect the loop; pairing the bucket with the kernel's existing Markov/attractor
loop-signal machinery is ahead of anything the frameworks ship. No more-sophisticated *primitive*
than the token bucket was found in production use; sophistication in the field is in *layering*
(per-principal envelopes + circuit breakers + anomaly alerts), not in a better bucket.

## 6. Sandboxing untrusted agent code — the two-tier design matches where the industry converged

2026 industry convergence is explicit: **agent code needs hardware-or-better isolation, not
containers** ([Northflank survey](https://northflank.com/blog/best-code-execution-sandbox-for-ai-agents);
in one quarter: E2B raised $33M, Google shipped Agent Sandbox on GKE, AWS shipped Lambda MicroVMs
([AgentConn](https://agentconn.com/blog/sandbox-agent-code-aws-lambda-firecracker-microvms-2026/))).

- **WASM / Wasmtime is mature enough to be the default isolation boundary today.** Fastly and
  Shopify run untrusted *tenant* code on Wasmtime in production — the exact threat model of "run
  someone else's agent adapter" ([status overview](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/)).
  The capability model is deny-by-default (a component touches only what it is explicitly handed —
  this *is* capability-trust, mechanized), fuel metering bounds CPU deterministically, and the
  Component Model gives typed, language-agnostic interfaces (WIT) — i.e., the bridge ABI for "any
  framework, any implementation" agents. WASI 0.3 (async I/O) shipped ~Feb 2026, WASI 1.0
  standardization is targeted within 2026 ([Java Code Geeks](https://www.javacodegeeks.com/2026/04/webassembly-in-2026-where-it-has-landed-what-wasi-0-2-changes-and-why-java-and-kotlin-developers-should-pay-attention-now.html))
  — the interface-stability risk that plagued WASI for years is closing but not fully closed; pin
  the WASI version per mesh epoch.
- **MicroVMs for what WASM can't hold.** Native-code adapters (Python frameworks, GPU access —
  WASM has no mature GPU story) need Firecracker-class isolation: <125ms boot, <5MiB overhead
  ([Spheron](https://www.spheron.network/blog/ai-agent-code-execution-sandbox-e2b-daytona-firecracker/)).
  E2B, Modal (gVisor), Daytona sell exactly this; it hard-requires KVM.
- **Caveat:** WASM's isolation is memory-safety + capability isolation in a shared process —
  treat it as an **integrity** boundary, not a **confidentiality** boundary against side-channel
  attackers (Spectre-class); secrets (node signing keys) must never share an address space with a
  guest component.

**Verdict for the mesh.** `SandboxTier::{WasmComponent, NativeProcessRequiresKvm}` with the
fail-closed KVM probe is precisely the industry-converged architecture, implemented at probe depth.
The honest gap list is the kernel's own: the WASM host (Wasmtime embedding, WIT world for the agent
bridge, fuel wired to `TokenBucket`) and the actual VMM launcher behind the probe are unbuilt.
Nothing researched suggests a different shape — only that the WASM tier should carry the default
path (per the Rust/WASM-native preference and M6: Wasmtime embeds as a Rust crate, zero external
service).

## 7. Gap table — what each piece must close for THIS mesh

| Piece | Proven at | Specific gap for a sovereign mesh |
|---|---|---|
| MCP | Massive agent-to-tool adoption | No crypto tool identity; no execution records; OAuth = external IdP (M6); no PQ; single-trust-domain assumption |
| A2A | Consolidated standard, v1.0, membership-heavy | Trust plane out of scope; card signing optional (classical JWS); OAuth/DNS anchors; production evidence thin |
| ANP / Agora / AIP | Draft / concept / paper | Not adoptable; mine for ideas only |
| CrewAI / LangGraph / AutoGPT-lineage | Orchestration ergonomics | Zero verification of agent claims (MAST: 21.3% of failures); step-bounds not cost-bounds; single-org trust |
| TEE attestation | Feasible in production (Nitro/TDX/Hopper CC) | Imports hardware-vendor trust; side channels; ill-suited to heterogeneous sovereign nodes — optional tier, not foundation |
| zkML | Research | 1000x+ overhead — rejected |
| x402 | 100M+ machine micropayments | Facilitator centralization (~70% one vendor); chain dependence (M6); classical crypto |
| AP2 | Launch-stage, top-tier partners | Centralized issuer/verifier ecosystem; classical VC suites — but the signed mandate-chain *pattern* is the keeper |
| Wasmtime/WASI | Untrusted tenant code at Fastly/Shopify | Confidentiality vs side channels; GPU absent; WASI 1.0 not yet final — pin versions |
| Firecracker microVMs | AWS Lambda-scale | Needs KVM (probe already fail-closed); launcher unbuilt |

## 8. Bottom line

Three patterns are worth borrowing outright: **MCP's tool-manifest/discovery grammar** (the kernel's
`Caps`/`LlmBackend` seam already speaks it), **A2A's Agent Card + explicit task lifecycle** (made
mandatory-signed with mesh capability certificates), and **AP2's signed mandate chain** (intent →
bounded authority → verifiable execution). Two of the kernel's existing primitives are independently
validated as best practice: token-denominated degrade-closed budgeting (ahead of what LangGraph/
CrewAI ship) and the WASM-default/microVM-escalation isolation split (where the whole industry
landed in 2026).

The gap nothing existing closes: **a decentralized, cryptographically verifiable, PQ-capable trust
plane for agent-to-agent work** — every surveyed protocol either punts trust to OAuth/DNS/CA
authorities (MCP, A2A), a dominant facilitator (x402), a platform issuer (AP2), or hardware vendors
(TEE), and every orchestration framework is trust-the-text with, at best, step caps. Signed
capability certificates + signed tool receipts + hash-chained event logs checked by deterministic
gates — the mesh's design — has no existing implementation to adopt. That part must be built, and
the survey says it is the only part that must be.
