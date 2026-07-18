# OPUS-R6 — Named-technology analysis for dowiz/DeliveryOS (2026-07-18)

> **Research document — writes no product code.** Each of the operator's 8 named items is
> investigated as a real, distinct artifact via live web/GitHub lookups this pass (2026-07-18).
> Where a name could not be verified, that is stated plainly rather than invented. Relevance is
> judged against `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.4/§16.7/§16.26/
> §16.30/§16.42–§16.43, `BLUEPRINT-P42-mcp-agent-skills.md` §11 (the browser-agent fence +
> chromiumoxide-vs-stealth precedent), and `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` (interface/
> rendering research already done this session). The Rust-native constraint (no Node.js/TypeScript
> in product code, ever) and the firm anti-detection-tooling stance (P42 §11.4 leg 4 / §11.7.2
> `chaser-oxide` rejection) are treated as binding filters, not preferences.

---

## 0. One-line verdict table

| # | Named item | What it really is (verified) | License | Maturity / activity | Verdict for dowiz |
|---|---|---|---|---|---|
| 1 | **T3MP3ST** | `elder-plinius/T3MP3ST` — autonomous multi-agent **offensive-security / red-team** meta-harness (turns Claude Code/Codex into 0-day hunters). **NOT** the EM-emanation TEMPEST. | AGPL-3.0 | TypeScript; 4.9k★; created 2026-07-02, pushed 2026-07-17 | **Not relevant** as product tech (TS/Node; dual-use offensive). Marginal interest as *external* security-audit tooling only. |
| 2 | **A Symbolic Analysis of Relay and Switching Circuits** | Claude Shannon's 1937 MIT MS thesis (pub. 1938): Boolean algebra ⇄ relay/switching circuits. Confirmed. | Public domain (historical) | Foundational; "most significant master's thesis of the 20th century" | **Relevant as intellectual grounding** for the math-first-architecture arc, not a dependency. |
| 3 | **m1k1o/neko** | Self-hosted virtual browser: a real desktop browser in Docker, **WebRTC-streamed to humans** (co-browsing, watch-party, remote support). Operator wrote "m1k1lo"; real is `m1k1o`. | Apache-2.0 | Go/Vue; v3.1.0 (2026-04); active | **Not relevant / weakest of the four** (human co-browsing, not agent automation; pulls in the Docker/desktop-VM weight the roadmap is retiring). |
| 4 | **openmontage** | `calesthio/OpenMontage` — agentic **video-production** system (12 pipelines, 52 tools, 500+ skills; Remotion/FFmpeg/Piper), orchestrated by an AI coding assistant. | AGPL-3.0 | Python+JS/TS+HTML; 39.8k★; pushed 2026-07-18 | **Not relevant** as adoptable tech (massive Python/Node stack, off-thesis). Conceptual reference for high-end content generation only. |
| 5 | **colibri** | Most likely `JustVugg/colibri` — pure-**C** LLM **inference engine** running GLM-5.2 (744B MoE) on a 25 GB-RAM consumer machine, experts streamed from disk, no GPU/Python/BLAS. (Name is ambiguous — see §5.) | Apache-2.0 | C; 16k★; created 2026-07-01, pushed 2026-07-18 | **Relevant-with-caveats** — technique reference for local/sovereign inference behind the existing `LlmBackend` port (§16.4). |
| 6 | **260716/_DLACoral** | `ekimroyrp/260716_DLACoral` — Vite+TS+**Three.js native-WebGPU/WGSL** 3D **diffusion-limited-aggregation coral** generative-design tool (GPU compute, GLB/OBJ export). One of ~40 weekly dated generative-physics demos by this author. | MIT | TypeScript; 0★; created 2026-07-16 (2 days old) | **Relevant-as-technique-reference only** — native-WebGPU-compute + DLA growth for R1's procedural-visuals track; TS/Three.js, not adoptable. |
| 7 | **OpenConnector** | **Two** real projects. (a) `ConductionNL/openconnector` — Nextcloud ESB (PHP, EUPL-1.2). (b) `oomol-lab/open-connector` — **AI-agent auth gateway** to 1000+ SaaS via SDK/CLI/**MCP**/HTTP (TS, Apache-2.0, 2.9k★). The prompt named (a); (b) is the one that fits the hypothesis. | EUPL-1.2 / Apache-2.0 | (a) PHP, 3★; (b) TS, 2.9k★, v1.3.0 (2026-07) | **Not relevant for adoption** (both off-thesis); **design reference** for credential-isolation, which dowiz already does via capability-certs. |
| 8 | **Lightpanda** | `lightpanda-io/browser` — from-scratch **headless browser in Zig** for AI agents: DOM+V8, **no rendering engine**, ~9× faster / ~16× lighter than headless Chrome, **CDP (Playwright/Puppeteer)**, **native MCP (JSON-RPC/stdio+HTTP)**, **robots.txt compliance, no stealth features**. | AGPL-3.0 | Zig; Beta; 32k★; nightly builds, active | **Relevant — the single concrete, fence-compatible recommendation.** Candidate engine for P42 §11.7 `browse_extract`'s JS-rendering gap. |

---

## 1. T3MP3ST — `elder-plinius/T3MP3ST`

**What it is (verified).** An open-source, multi-agent **offensive-security** framework by
elder-plinius ("Pliny", a well-known LLM red-teamer). It is an orchestration layer — it does not
ship a model — that repurposes general-purpose AI coding agents (Claude Code, Codex, etc.) as
autonomous red-team operators, coordinating them through a recon → exploit → verify → report kill
chain (an 8-operator design mapped to MITRE ATT&CK / the Cyber Kill Chain). Claimed results: 90.1%
pass@1 on XBEN; 8/10 held-out post-cutoff CVEs pinned to file/line/CWE. Interface is a web
"War Room" + CLI. AGPL-3.0, TypeScript, 4.9k★, created 2026-07-02, pushed 2026-07-17.

**Explicitly NOT the security TEMPEST.** "TEMPEST" (compromising electromagnetic emanations, the
real NSA-originated concept) is unrelated. "T3MP3ST" is leet-styled branding for this red-team
harness; there is no EM-side-channel content in the project. Verified against the actual repo and
multiple independent 2026 write-ups.

**Relevance verdict: NOT relevant as product technology.** It is TypeScript/Node (violates the
Rust-native constraint), and its purpose — autonomous exploit discovery — is dual-use and
governance-sensitive; wiring an autonomous 0-day hunter into a session that also self-manages the
repo is exactly the kind of irreversible/red-line surface the harness gates on. The only defensible
dowiz touchpoint is *external, human-directed* security auditing of dowiz's **own** authorized
surfaces — which is already served by the repo's Playwright E2E + `security-review` conventions and
the read-only `security-sentinel`/`invariant-guardian` reviewers. No adoption. *Sources:*
[github.com/elder-plinius/T3MP3ST](https://github.com/elder-plinius/T3MP3ST) ·
[cybersecuritynews.com/t3mp3st-security-framework](https://cybersecuritynews.com/t3mp3st-security-framework/).

## 2. "A Symbolic Analysis of Relay and Switching Circuits" — Claude Shannon, 1937

**What it is (confirmed).** Claude E. Shannon's MIT master's thesis (written 1937, published 1938),
which proved that **Boolean algebra** is the correct mathematics for describing and simplifying
relay-and-switch networks — the electromechanical logic of telephone exchanges — and, conversely,
that relay networks can compute Boolean functions. It established the two-valued (on/off) algebraic
substrate underneath all subsequent digital circuit design and is routinely called "the most
significant master's thesis of the 20th century." It won the 1940 Alfred Noble Prize.

**Relevance verdict: relevant as intellectual grounding, not a dependency.** This is the historical
root of the exact idea the repo's **math-first-architecture** arc pursues — that an
architecture should reduce to a small algebra over primitives, and that logic and its physical
substrate are one object. It is the direct ancestor of: the `eqc` equation→Rust compiler
(tools/eqc/), the "kernel-as-MCU" north-star, the spectral/Laplacian operator work
(`kernel/src/incidence.rs`, `spectral.rs`), and R1's "*ad fontes* — math/physics primitives over
library dependencies" framing. Cite it as lineage/justification in the math-first design docs; there
is nothing to install. *Sources:*
[Wikipedia](https://en.wikipedia.org/wiki/A_Symbolic_Analysis_of_Relay_and_Switching_Circuits) ·
[historyofinformation.com/detail.php?id=622](https://www.historyofinformation.com/detail.php?id=622).

## 3. m1k1o/neko — self-hosted WebRTC virtual browser

**What it is (verified).** A containerized virtual browser that runs a **real** desktop browser
(Firefox/Chromium/Chrome/Edge/Brave/Tor, plus full XFCE/KDE desktops or any Linux app) inside Docker
and **streams it to humans over WebRTC** — smooth video + audio, multi-user, synchronized. Origin
was co-watching/"watch party" after rabb.it shut down; documented uses are collaboration, interactive
presentations, teaching/support, and privacy (a persistent-cookie browser reachable from anywhere).
Go (37%) / Vue / shell; Apache-2.0; v3.1.0 (2026-04-02); active (11 releases, ~2.1k commits). A
companion `neko-rooms` adds room-management API. **No anti-detection/stealth positioning** — cleanly
legitimate (passes the P42 §11.4 leg-4 filter). Operator's "m1k1lo" is a typo for `m1k1o`.

**Relevance verdict: not relevant / weakest of the four (see §9).** Neko solves "put a real,
human-driven browser on a screen somewhere else." It is **not** an automation or extraction tool —
it is the opposite of a headless engine (it exists to *render and stream*). Its plausible dowiz uses
(owner co-browsing during onboarding support; hosting an owner's authenticated session) are either
served more cleanly elsewhere (chromiumoxide attach-to-local for P42 U1; Playwright for
self-surface QA) or actively off-thesis: Neko is a GB-scale Docker desktop-streaming VM, and the
roadmap's Docker→microVM+WASM arc is *retiring* that class of dependency. It does not compose with a
headless engine (Lightpanda has no display to stream). No adoption. *Sources:*
[github.com/m1k1o/neko](https://github.com/m1k1o/neko) · [neko.m1k1o.net](https://neko.m1k1o.net/).

## 4. openmontage — `calesthio/OpenMontage`

**What it is (verified).** An open-source **agentic video-production** system — "12 pipelines, 52
tools, 500+ agent skills" — that turns an AI coding assistant into a video studio: research →
script → asset generation → edit → render, across image-based (Piper narration + Remotion), local
character animation (SVG rigs/GSAP), and documentary/real-footage pipelines (CLIP-searchable corpus
from Archive.org/NASA/Wikimedia). AGPL-3.0; **Python** (4.0 MB) + HTML + JS/TS; requires Python 3.10+,
**FFmpeg, Node.js 18+, Remotion**; 39.8k★, pushed 2026-07-18 (very hot).

**Relevance verdict: not relevant as adoptable tech.** It has nothing to do with browser automation
or connectors — it is a content-generation domain of its own. Its only conceptual tie to dowiz is
§16.7/§16.26 (a venue's social/marketing content could include short promo videos). But (a) it is a
large Python+Node+Remotion stack, categorically off the Rust-native thesis; and (b) §16.4 is explicit
that routine content work is **deterministic automation scripts + the single in-hub assistant**, not
an agent-per-function studio — a 500-skill video pipeline is precisely the "agent sprawl no one asked
for" that §16.4 rules out. Useful only as a reference for what maximal agentic content generation
looks like. No adoption. *Sources:*
[github.com/calesthio/OpenMontage](https://github.com/calesthio/OpenMontage).

## 5. colibri — most likely `JustVugg/colibri` (name is ambiguous)

**What it most likely is (verified candidate).** `JustVugg/colibri` — a compact, pure-**C** LLM
**inference engine** whose thesis is "tiny engine, immense model": it runs **GLM-5.2 (744B-parameter
MoE)** on a ~**25 GB-RAM consumer machine** with **zero dependencies**, keeping the dense weights
resident and **streaming routed experts from disk**, with compressed attention caches, expert
caching, and speculative decoding. No Python, no BLAS, **no GPU required**. Ships a terminal chat,
an OpenAI-compatible API, and a browser client. Apache-2.0; C; 16k★; created 2026-07-01, pushed
2026-07-18 (currently trending). This candidate fits the list's local-AI/sovereign theme far better
than the other real "colibri"s.

**Honest disambiguation.** "Colibri" is a heavily overloaded name. Confirmed-real but almost
certainly **NOT** meant here: **Toradex Colibri** (SoM hardware modules), **Jitsi COLIBRI** (a
WebRTC videobridge signalling protocol), various "Colibri UI"/semantic-middleware projects, and a
SourceForge `colibr` entry. Given the surrounding items (local inference, agents), the LLM engine is
the best-supported reading — but this identification carries residual uncertainty; confirm with the
operator if a decision hinges on it.

**Relevance verdict: relevant-with-caveats (technique reference).** It maps directly onto §16.4's
"local Ollama or a connected backend" assistant and the existing swappable `LlmBackend` port
(`kernel/src/ports/llm.rs`) — a C engine could sit behind that port as an out-of-process backend
exactly like the Ollama adapter. The *interesting, transferable* idea is **disk-streamed MoE experts
on commodity RAM/CPU**, which is the enabling trick for running a capable assistant on an owner's own
hardware without a GPU — squarely on the sovereign/local-first thesis. Caveats: a 744B target is far
oversized for a per-hub owner assistant (the technique matters, not that specific model); and it is C,
so it enters only as a sidecar behind the port under a `rust-native-bare-metal-decision` DECART, never
in-kernel. *Sources:* [github.com/JustVugg/colibri](https://github.com/JustVugg/colibri).

## 6. 260716/_DLACoral — `ekimroyrp/260716_DLACoral`

**What it is (verified directly).** The operator's "260716/_DLACoral" resolves to
`ekimroyrp/260716_DLACoral` (the "260716" is a `YYMMDD` date-stamp — created 2026-07-16, two days
old). Per its README: a **Vite + TypeScript + Three.js** application for growing and exploring
**3D diffusion-limited-aggregation (DLA) coral forms**, with **native WebGPU compute in raw WGSL and
no WebGL fallback** — GPU walker pools, sparse occupied/frontier hash buffers, indirect instanced
icosphere drawing for large aggregates, eight attachment-neighborhood modes, birth-order color
gradients, ACES tone mapping + studio lighting + bloom, a branchable deterministic simulation
timeline, undo/redo with compressed snapshots, and GLB/OBJ/PNG export. MIT; requires Node 20.19+/
22.12+; Vitest suite; 0★ (brand-new personal repo).

DLA **is** the real physics/math generative-growth algorithm the prompt guessed (Brownian walkers
sticking into a fractal aggregate). The author `ekimroyrp` maintains ~40 such weekly dated
generative-physics demos (Chladni cymatics, slime mold, differential growth, strange attractors,
Voronoi, crystal growth, soap film/minimal surfaces, gyroids, morphogen/reaction-diffusion fields,
etc.) — a rich catalog of exactly the procedural-visual primitives R1 §4 discussed.

**Relevance verdict: relevant-as-technique-reference only.** Two concrete connections to work already
done this session:
1. **R1 §4 procedural/physics-generated visuals + the physics-GPU-UI arc** — DLA is a legitimate
   generative-growth primitive, and this repo is a clean, tested reference for **GPU-compute
   morphogenesis** (walkers + hash grids + indirect instancing) of the kind those arcs want.
2. **A pointed contrast to flag:** `_DLACoral` deliberately makes the **opposite** rendering-fallback
   choice from dowiz — "unsupported browsers show an in-app WebGPU error instead of silently
   switching to another renderer," i.e. **no WebGL2 floor**. dowiz's FE-16 ladder (R1 §6, Risk #9)
   mandates the WebGL2/CPU floor for the ~18% without WebGPU. This is a useful real-world data point
   that a native-WGSL-only path is a live design stance in 2026 — and a reminder of why dowiz's floor
   is a *tested gate*, not an afterthought.

Not adoptable: TS/Three.js/browser-WebGPU + Node build, MIT — it is a technique/reference in the same
category R1 assigned to GPUI/Vello ("mine for technique, do not adopt"). *Sources:*
[github.com/ekimroyrp/260716_DLACoral](https://github.com/ekimroyrp/260716_DLACoral) (README + tree
read this pass).

## 7. OpenConnector — two real projects, disambiguated

**(a) `ConductionNL/openconnector`** (the one the prompt named). An **enterprise service bus / gateway**
delivered as a **Nextcloud app**: REST/SOAP/GraphQL/file-drop/message-queue sources → typed
"registers," configurable mappings, cron/webhook/manual sync, bidirectional push, audit trail; strong
Dutch-government-adapter focus (PDOK, StUF-ZKN/BG, DSO, Berichtenbox). PHP; **EUPL-1.2**; maintained by
Conduction since 2019; the GitHub repo itself has ~3★ (it lives primarily in the Nextcloud app store).

**(b) `oomol-lab/open-connector`** (the better contextual fit). An **open-source auth gateway
connecting 1000+ SaaS providers to AI agents** through SDK / CLI / **MCP** / HTTP / OpenAPI, with
10,000+ prebuilt actions (GitHub, Gmail, Notion, BigQuery, Slack, …). It is explicitly a
**credential/OAuth boundary**: it manages tokens and execution policy so the agent never holds
provider credentials. TypeScript; **Apache-2.0**; 2.9k★; v1.3.0 (2026-07); deploys to Docker/Node,
Fly.io, or Cloudflare Workers.

**Relevance verdict: not relevant for adoption; design reference only.**
- The **Nextcloud ESB** is architecturally inapplicable — dowiz is not a Nextcloud app; that connector
  framework lives inside Nextcloud's runtime.
- The **oomol agent gateway** is the one that matches the hypothesis's "connector/integration
  framework for third-party data sources," and its *pattern* (keep credentials out of the agent
  process; expose third parties as MCP-scoped actions) is genuinely aligned with dowiz doctrine. But
  dowiz **already** realizes that pattern natively and more strictly: capability-certificates
  (proto-cap/ML-DSA) as the auth model, the IP-* integration-ports arc, and P42's `agent-adapters`
  foreign-MCP gate (operator-signed manifest → closed `(Resource, Action)` scopes, fail-closed drop).
  A TS/Node 1000-SaaS gateway is both off-thesis and the "N flat integrations" sprawl the roadmap
  rejects in favor of a small closed set of official adapters. Take the credential-isolation idea as
  confirmation of the existing design; do not adopt the code. *Sources:*
  [github.com/ConductionNL/openconnector](https://github.com/ConductionNL/openconnector) ·
  [openconnector.conduction.nl](https://openconnector.conduction.nl/) ·
  [github.com/oomol-lab/open-connector](https://github.com/oomol-lab/open-connector).

## 8. Lightpanda — `lightpanda-io/browser`

**What it is (verified).** A **headless browser written from scratch in Zig**, purpose-built for AI
agents and automation. Its defining choice: **it does not render** — no graphical engine — so it
implements DOM + HTML5 parse + JavaScript (V8) + Ajax/Fetch + cookies + form input + click, without
the pixel pipeline. Result: ~**9× faster** and ~**16× lighter** than headless Chrome on multi-page
workloads (repo cites ~123 MB peak for 100 pages vs Chrome's ~2 GB). It exposes a **CDP server**
(drop-in for Puppeteer/Playwright via `browserWSEndpoint`), a **native MCP server** (JSON-RPC 2.0 over
stdio and HTTP), a CLI/agent for plain-language navigation+extraction, proxy support, network
interception, and — importantly for the fence — **robots.txt compliance**. AGPL-3.0; Zig (78%); 32k★;
**Beta** ("work in progress, coverage improving"; hundreds of Web APIs still unimplemented); active
nightly builds. **No anti-detection/stealth/fingerprint-spoofing features** are advertised or shipped.

**Relevance verdict: relevant — the one concrete, fence-compatible recommendation (see §10).**
Lightpanda passes every P42 §11.4 fence leg by construction: honest self-identification (no stealth
code path — leg 4 clean), robots.txt-respecting, and it slots behind the same read-only,
denylist-gated, owner-initiated contract as any engine. Critically, its **native MCP + stdio**
transport is the *exact* shape P42 §11.7.3 already designed the `dowiz-browser-mcp` sidecar around
(JSON-RPC 2.0 over child stdio pipes) — but Lightpanda was **not** in the §11.7.2 DECART
(chromiumoxide vs fantoccini vs headless_chrome). That is the gap this report closes below.

---

## 9. The operator's four-way synergy hypothesis — honest evaluation

**Hypothesis (verbatim scope):** *openconnector + lightpanda + neko + openmontage have synergy for
dowiz* — sketched as a lightweight browser-automation stack (Lightpanda) for the assistant's
SEO/content tooling, plus shared/remote browsing (Neko) for owner-support/content-verification, plus
a connector framework (OpenConnector) for third-party data sources.

**Verdict: REFUTED as a single coherent stack; REFINED to one real integration.** Once each item is
understood concretely, the four do **not** build on each other — the thread the operator sensed
("AI-agent tooling") is a *category*, not a *composition*. They split into three unrelated kinds:

1. **Agent web/data access (the real, coherent pair).** **Lightpanda** (headless extraction) and
   **OpenConnector-oomol** (credential-isolated agent→SaaS actions) share a genuine theme: giving an
   agent bounded, legitimate reach to external web/data. This is the only sub-cluster with real
   internal logic. But even here they are *alternatives-for-different-surfaces*, not a layered stack —
   Lightpanda reads arbitrary public/JS pages; OpenConnector routes to APIs that already exist. And
   OpenConnector's contribution is a **pattern dowiz already implements more strictly** (capability
   certs + closed official adapters + `agent-adapters` MCP gate), so it collapses to "confirmation of
   existing design," not a new component.

2. **Human co-browsing (the odd one out).** **Neko** is a different category entirely — it *renders
   and streams a real browser to a person*. It does not compose with a headless engine (nothing to
   stream from Lightpanda), does not serve agent automation, and reintroduces the Docker/desktop-VM
   weight the roadmap is actively retiring. Forcing it into a "browser stack" with Lightpanda is where
   the hypothesis most clearly breaks: they are opposite tools.

3. **Video production (a different domain).** **OpenMontage** has no browser-automation or connector
   surface at all. Its only tie is §16.7/§16.26 content, and even there §16.4 rules out the
   agent-studio shape. It sits alone.

So: **two of the four share a real theme (and one of those two is redundant with existing doctrine);
the other two are each in a category of their own.** There is no honest four-way synergy — presenting
one would be forcing a narrative the evidence does not support. What the exercise *does* yield is one
sharp, concrete, defensible extraction, below.

## 10. The one concrete integration worth proposing — Lightpanda → P42 §11.7

**Where it plugs in.** P42 §11.7 already specifies a first-party **local MCP-server sidecar**
(`dowiz-browser-mcp`) behind the `browse_extract` fence, and §11.7.2 ran a DECART among
`chromiumoxide` / `fantoccini` / `headless_chrome`, choosing **chromiumoxide** on two grounds:
(1) U1 (`MenuImportAssist`) needs **attach-to-the-owner's-running-authenticated-local-browser**, and
(2) chromiumoxide's build-time PDL codegen is the low-maintenance CDP path. **Lightpanda was not
evaluated.** It should be added to that table as a **named candidate for a different leg of the same
capability**, not as a replacement:

- **U2 (`SupplierCatalogRead`) and the SEO/AEO read paths (§16.26)** — public pages, launch-mode, no
  owner session. Here Lightpanda is arguably the *better* fit than chromiumoxide: it runs V8, so it
  closes the **exact gap the §11.2 v0 (capped GET + `LlmBackend` structuring) cannot** — JS-rendered
  menus/catalogs (the named T-B3 trigger: "JS-only menu, handoff impossible") — at ~16× less memory
  and with a **native MCP/stdio** transport that matches §11.7.3's sidecar shape with *less* glue than
  wrapping chromiumoxide's tokio CDP loop. It is fence-clean (robots.txt, honest UA, no stealth).
- **U1 (`MenuImportAssist`)** — Lightpanda **cannot** attach to the owner's already-authenticated real
  Chrome profile (it is its own headless engine). chromiumoxide's `connect`-to-running remains the
  right tool for U1. So the honest outcome is **complementary engines behind one port**, chosen per
  intent, not a single winner.

**Recommended action (research-level, operator-gated — no code):** add a row to the P42 §11.7.2 DECART
for `lightpanda-io/browser` with these falsifiable criteria — *attach-to-running:* NO (eliminates it
for U1, as WebDriver was eliminated for fantoccini); *JS execution:* YES (beats v0 for U2); *native
MCP/stdio:* YES (best sidecar fit); *runtime:* separate Zig process behind the stdio firewall (no
tokio/chromiumoxide in-process — same red-proof as §11.7.3); *maturity:* Beta, partial Web-API
coverage (the risk to price in); *license:* AGPL-3.0 at a **process boundary** (sidecar binary, not
linked). Falsifier/watch-point: Lightpanda's Web-API coverage on real supplier/menu pages — bench it
against v0 and chromiumoxide on ≥3 actual onboarding pages before adopting. This keeps the capability
"follows recorded need, not novelty" (§11.7.1 T-B3) and stays entirely inside the existing fence.

**Non-negotiable filter applied (all clear).** None of the eight is anti-detection/stealth tooling.
The only item in that family that *exists in the neighborhood* is P42 §11.7.2's already-rejected
`chaser-oxide` (a chromiumoxide stealth fork) — not on the operator's list, and correctly excluded.
Lightpanda, Neko, and the OpenConnectors are all legitimate (robots.txt-respecting / co-browsing /
official-API-routing) and pass leg 4; T3MP3ST is offensive-security (authorized-target only), a
different concern handled in §1. Nothing here recommends evasion tooling.

---

## 11. What to carry forward

- **Adopt-candidate (1):** **Lightpanda** as a P42 §11.7 engine candidate for the JS-rendering
  read-path gap — the single actionable output. Add it to the §11.7.2 DECART; keep chromiumoxide for
  the U1 attach case. Bench before adopting.
- **Design confirmations (0 new code):** **OpenConnector-oomol** validates dowiz's existing
  credential-isolation doctrine; **colibri** validates the disk-streamed-MoE local-inference direction
  behind the `LlmBackend` port; **Shannon 1937** is the citable lineage for the math-first arc.
- **Technique references (not adoptable):** **`_DLACoral`** for GPU-compute generative growth (and as
  a live example of the no-WebGL-fallback stance dowiz deliberately rejects); **OpenMontage** as the
  high-end agentic-content-generation reference.
- **Declined:** **Neko** (wrong category + retiring infra class), **T3MP3ST** (dual-use offensive,
  TS/Node), **OpenMontage** and both **OpenConnectors** as *adoptions*.
- **Open item for the operator:** confirm that "colibri" means `JustVugg/colibri` (name is
  overloaded — §5).
