# BLUEPRINT P101 — Local & Mobile Model Selection + CPU-Only Serving Topology (2026-07-20)

**Status: BLUEPRINT / PLAN — no code written, nothing built, no model pulled.**
**Date:** 2026-07-20
**Component:** AGENT (mobile half also feeds DELIVERY P52/P71)
**Numbering note:** drafted as P97; renumbered same-day after a concurrent docs pass claimed
P97/P98 (AR/VR + audio). **P99/P100 are deliberately skipped, permanently:** both strings are
corpus-wide latency-*percentile* notation (this document's own §4.5 records "P99 wall-clock"),
and an item number that greps into percentile hits is exactly the lexical-collision class the
P-D→Layer-D rename ruling exists to prevent. Hence **P101**.
**Source of facts:** the completed operator-commissioned deep-research pass on (1) small
VLM/agentic models for a mobile courier app, (2) multi-model CPU-only serving topology, and
(3) CPU-only fine-tuning feasibility — synthesized here against the live tree and against
live spot-checks run on the actual deployment box today (§1). Every benchmark number in this
document is quoted verbatim from that research; every claim about dowiz code was re-verified
against the tree at `docs/local-model-synthesis-2026-07-20` (based on `origin/main` @
`09859aaa6`). Where the research has a named GAP, this blueprint either proposes a cheap
empirical closure or says the gap stays open — it never papers over one.

> **Amendment (2026-07-20, same day) — two firm operator corrections, folded in below, no
> further discussion required:**
> 1. **Model count and relationship.** This blueprint originally shipped with an abstract
>    **E/S/G/C multi-tier residency system** server-side (§4, an open-ended "pick tiers
>    empirically" design) and a **primary/fallback** framing for the mobile VLM pick (§3).
>    The operator overruled both: given the measured **max-2-concurrent-decode-stream**
>    ceiling this box actually has (§2), the topology runs on **exactly 2 concrete, named,
>    fully native offline models, not an abstract N-tier system** — **LFM2.5-VL-450M** and
>    **SmolVLM-256M-Instruct**. These two are **not primary/fallback** — they run
>    **concurrently, crosswired** (§2, §3.2, §4.2): SmolVLM's fast first-pass perception
>    feeds LFM2.5-VL's reasoning/tool-selection layer, both loaded and both actively used,
>    never one dormant as a standby for the other. **The same pair serves both the mobile
>    courier pick (§3) and the server topology (§4)** — one artifact family, not two
>    separate picks — filling both of the box's two decode-stream slots.
> 2. **O-1 (LFM Open License) is RULED, not open.** Operator, verbatim: *"clear to ship."*
>    LFM2.5-VL-450M ships despite the LFM Open License's $10M-revenue-cap term. The term
>    itself stays recorded, not erased (§3.3, §8) — per this repo's documentation
>    discipline, decisions get recorded; it is simply no longer a blocking gate.
>
> Execution home for the pair's *native* engine — the same-day R-1 ruling's zero-dependency
> bare-metal build that eventually replaces the Ollama serving path lane-by-lane, with this
> document's Ollama-era topology staying near-term truth until each cut-over gate is green:
> [`../BLUEPRINT-P102-bare-metal-inference-engine-2026-07-20.md`](../BLUEPRINT-P102-bare-metal-inference-engine-2026-07-20.md)
> (its §3 designs the crosswire dataflow/scheduling in full; its §6.1 records the P101↔P102
> sequencing and the E-1 embed / CS-2 code residuals).

**Relationship to the local-model-wiring blueprint
([`BLUEPRINT-LOCAL-MODEL-WIRING-TESTING-USAGE-2026-07-20.md`](BLUEPRINT-LOCAL-MODEL-WIRING-TESTING-USAGE-2026-07-20.md)):**
complementary, not competing. That blueprint owns the *wiring* (AiMode composition switch,
golden tool-calling suite, configurable `TaskClass`→model map) and deliberately left three
questions out of scope: **which** models (it took the four resident ones as given), whether a
**multi-model topology** on the actual box is worth designing, and whether the **training
deferral** survives contact with real CPU-training research. P101 answers those three. Its §1
locked decisions are locked here too and restated in §7 — nothing below reopens them.

## 0. Scope honesty

Three decisions and one measurement plan; no large build. The mobile half (§3) is
forward-provisioning for a courier surface (P52/P71) that is itself still PLAN — so the mobile
deliverable is a *pick + a model-agnostic scenario suite + a license ruling*, not an app build.
The server half (§4) is *configuration + measurement* over infrastructure that already exists
(Ollama, `llm-adapters`), not new serving infra. The training half (§5) is a *ruling that the
existing deferral holds*, plus one optional bounded probe. Anyone padding this into a
model-platform workstream is misreading it.

## 1. Grounding facts — live-verified on the deployment box, today

Spot-checks run 2026-07-20 on the actual Hetzner vServer (the box `llm-adapters` targets):

| Fact | Verified value | How verified |
|---|---|---|
| CPU | AMD EPYC-Milan (Zen3), **4 physical cores × 2 SMT = 8 vCPU**, 1 socket, 1 NUMA node | `nproc`, `lscpu` |
| SIMD / matrix ISA | `avx`, `avx2` present; **zero `amx*` flags** (`grep -c amx /proc/cpuinfo` = 0) | `/proc/cpuinfo` |
| RAM | 30 GiB total, ~28 GiB available | `free -g` |
| GPU | none | (known; nothing in `lspci`-class evidence contradicts it) |
| Ollama | live at `127.0.0.1:11434` | `GET /api/tags` |
| Resident models | exactly 4: `qwen2.5-coder:7b` (Q4_K_M, 4.7 GB, ctx 32768, **`tools` capability**) · `llama3.1:8b` (Q4_K_M, 4.9 GB, ctx 131072, **`tools`**) · `nomic-embed-text` (F16, 274 MB, 768-d) · `qwen3-embedding:0.6b` (Q8_0, 639 MB, 1024-d) | `GET /api/tags` |

*Note (2026-07-20 correction): the row above is the live snapshot of what the box runs
**today**, ahead of this document's own Phase C enactment — it is not the target state.
§4.2's residency plan now targets `SmolVLM-256M-Instruct` + `LFM2.5-VL-450M` (crosswired) in
place of `llama3.1:8b` for the product-facing lanes; `qwen2.5-coder:7b` stays as the unaffected
dev/harness-only model. Phase C (§6) is the still-unbuilt cutover from this table to that plan.*

The AMX check matters: the research's one vendor-backed CPU-training path (IPEX-LLM QLoRA)
requires Intel AMX. This box provably lacks it — that inference is now a locally verified fact,
not a datasheet argument (§5).

**The actual dowiz LLM call sites the topology must serve** (grounded in the tree, not a
generic essay — each verified today):

- **CS-1 — agent-loop executor (P40).** `agent-loop/src/lib.rs`: bounded plan→act→observe,
  `MAX_AGENT_ITERATIONS = 4`, `TaskClass::General` chat with tool-calls over the facade's tool
  surface (`read_order_status`; `web_fetch` behind the off-by-default `web-fetch` feature).
  Served by `agent-loop/src/service.rs`, proxied by `native-spa-server`. This is the "hard
  task" lane: tool selection must be right, latency tolerance is seconds.
- **CS-2 — harness Code lane.** `llm-adapters/src/compose.rs` `Harness` (Dispatcher →
  CachingBackend → OllamaAdapter, `workers: 2`, TokenBucket 64-cap/8-per-s);
  `ollama.rs::route_model` sends `TaskClass::Code` → `qwen2.5-coder:7b`. Dev/harness lane, not
  product-request-path.
- **CS-3 — embedding lane.** `Harness::embed` → `nomic-embed-text`; consumers: the
  semantic-leak gate (`kernel/src/leak_gate.rs`) and the retrieval arc. Always-on, tiny,
  latency-sensitive.
- **CS-4 — intake assist (P48-INTAKE, PLAN).** The intake blueprint's anti-scope §8.7 is
  binding: **no LLM in the intake trust path** — the deterministic `IntentParser` is the
  authority; AI-assist only consumes `Ambiguous` outcomes, under P41's `AiMode::Off`-works
  invariant. So this is a low-rate, classification-grade lane with a *deterministic
  sufficiency oracle* (did the assisted output re-parse into a valid intent, or is it still
  `Ambiguous`?). That oracle is what makes cascading legitimate here and nowhere else (§4.4).
- **CS-5 — comms drafting (D14, PLAN).** Agent-assisted reply drafting is opt-in and
  human-gated per the D14 ruling. Low-rate, classification/drafting-grade.
- **CS-6 — voice (adjacent, NOT an `LlmBackend` consumer).** `engine/src/voice.rs` defines
  `AsrModel` — "Sibling of the kernel `LlmBackend` for a different modality" — with a
  whisper-vs-Moonshine bake-off already mandated by the spatial-storefront synthesis (A4.4:
  numbers recorded on hub hardware, vendor claims "reproduced or not reproduced"). It shares
  this box's RAM and memory bandwidth, so §4's budget counts it even though P101 doesn't design it.

**And the honest statement about vision:** there is **no VLM call site anywhere in the tree
today**. The PoD settlement artifact (`DeliveryClaim`, bebop2 `pod.rs`) has **no photo field**
and settles on k-of-n hub signatures — vision output is structurally incapable of gating
settlement, and P101 keeps it that way (§7). The mobile VLM pick is forward-provisioning for
P52 K4's capture-assist UX (photo sanity/framing help, receipt & menu OCR, label reading), not
a wired consumer.

## 2. The operator's topology idea, re-grounded in this box's physics

The operator's framing — "simple models for easy tasks, one strong reasoning model for hard
tasks" — is architecturally right, and the research confirms it matches an established
pattern (routing/cascading, §4.4). One implied reading had to be corrected before design:
**"8 cores" does not mean 8 independent model streams.** Three grounded reasons:

1. **This is a 4-physical-core box.** 8 vCPU = 4 Zen3 cores × SMT2. llama.cpp's own threading
   guidance (research finding): 4–6 threads is typically optimal for *decode*; extra threads
   help *prefill* far more than decode. One decode stream already wants this box's entire
   real core count.
2. **Decode is memory-bandwidth-bound, not compute-bound** (the research's key bottleneck
   finding). Every concurrently-decoding model multiplies DRAM bandwidth pressure even when
   idle compute exists. The EPYC 9334 datapoint (Q4 7–20B → 20–28 tok/s on 24 threads,
   dual-socket) is from a machine with several times this box's bandwidth — and it still
   frames Q4's benefit as *DRAM headroom for multi-tenancy*, i.e. **residency**, not N
   full-speed parallel decodes.
3. **No source anywhere measured "N small models on 8 cores" vs "1 big model on 8 cores"**
   (research GAP). We do not assume either way; Phase B measures it on this exact box (§4.5).

That measurement produced a firm ceiling — **max 2 concurrent decode streams, full stop** —
and the operator closed the design question directly on top of it, superseding the abstract
"many models resident, few decoding, pick the small tier empirically" framing this section
previously carried: **exactly 2 concrete, named, fully native offline models fill exactly
those 2 slots — LFM2.5-VL-450M and SmolVLM-256M-Instruct — not an open-ended tier system.**
They are **not a primary/fallback pair**: both are loaded and both are actively, concurrently
used — SmolVLM's fast first-pass perception feeds LFM2.5-VL's reasoning/tool-selection layer
in the same request, crosswired rather than one standing dormant as a backup for the other
(§3.2, §4.2). The same pair serves both the mobile courier pick (§3) and the server topology
(§4) — one artifact family, not two separate picks.

One consequence worth stating plainly: both models are far smaller (256 M / 450 M parameters)
than the 7–8 B class this box's physics reasoning above was calibrated against, so the DRAM
bandwidth pressure of decoding both concurrently is almost certainly much lighter than the
llama3.1:8b-class concern that originally motivated "measure before trusting." That is a
reason for optimism, not a license to skip Phase B (§4.5) — the repo does not ship topology on
assumption, so the pair still gets measured on this exact box before being trusted at the
25%-degradation decision rule.

RAM capacity (~28 GiB) is not remotely the constraint for a sub-1 GB pair; concurrent decode
bandwidth, now bounded to exactly these 2 named models, is what Phase B measures. Ollama's
native controls express the residency/concurrency split directly
(`OLLAMA_MAX_LOADED_MODELS` = residency, `OLLAMA_NUM_PARALLEL` = per-model concurrency), which
is one more reason not to build custom serving infra (§4.1).

## 3. Mobile courier-app model — the pick, and the license decision it forces

### 3.1 Requirements, derived from real courier scenarios (not benchmark shopping)

From P52 K4 / P71 R-items: (a) **document-class OCR** — receipts, menus, package labels;
(b) **photo-sanity assist** at the PoD capture step — "is a package visibly at the door in
frame", framing hints; (c) **structured/agentic output** — the assist is useful only if its
output can be consumed as a field or a tool call, not prose; (d) **CPU/NPU phone inference**
on mid-range Android (Albania/EU Wave-0 market, D12 §4-D) — GGUF/llama.cpp-class deployment,
sub-second-feel latency, sub-1 GB memory; (e) **refusal on ambiguity** — a confidently wrong
"package present" is worse than "can't tell" (same negative-case-first-class discipline as
the wiring blueprint's golden suite).

### 3.2 The pair — not a ranked shortlist, two complementary roles

**Decided (operator, 2026-07-20, no further discussion): LFM2.5-VL-450M and
SmolVLM-256M-Instruct, run concurrently and crosswired — not primary/fallback.** Both were
already the strongest two candidates the research surfaced (below); the correction is to how
they relate, not which two models: instead of one dormant standby held in reserve for when
the other proves unusable, both are loaded and both do real work on every request, in a
two-stage pipeline:

- **SmolVLM-256M-Instruct — first-pass perception, always hot.** Fast, cheap, fully
  Apache-2.0. Runs first: fast triage of the raw input (is a package visibly in frame /
  occluded / absent; a first pull of OCR text off a receipt or label; a quick classification
  or context-compression pass for text-only server lanes, §4.4). Its output is *evidence*,
  not the final answer.
- **LFM2.5-VL-450M — reasoning / tool-selection, always hot.** Consumes SmolVLM's first-pass
  output plus the original input and produces the authoritative decision: the structured
  field, the tool call, the refusal. It is the only sub-1B candidate with *measured* agentic
  evidence at all (BFCLv4 **21.08** — modest in absolute terms, but every permissive
  alternative in class has **zero** tool-calling evidence).

Evidence base for each, as researched:

| Model | For | Against / caveats |
|---|---|---|
| **LFM2.5-VL-450M** (Liquid AI, 2026-04-08) | BFCLv4 **21.08** (only sub-1B model with measured tool-calling evidence); OCRBench **684**/1000 ≈ 68.4 (strongest doc-OCR signal in the size class); RefCOCO-M 81.28 (grounding/pointing — useful for framing assist); sub-250 ms edge-inference *claim*; day-one GGUF/llama.cpp + ONNX + LEAP iOS/Android SDK | LFM Open License v1.0 — **RULED, ships anyway (§3.3, O-1 CLOSED)**; BFCLv4 21.08 means "measurable, not reliable" — the scenario suite (§3.5) still has to prove the pipeline clears the bar; sub-250 ms claim is vendor-published on unspecified hardware — reproduce, never trust (Moonshine-precedent discipline) |
| **SmolVLM-256M-Instruct** (Apache 2.0) | Fully permissive; GGUF/ONNX; <1 GB single-image inference; proven phone deployments (llama.cpp, HuggingSnap); cheapest possible first-pass stage | **No demonstrated agentic/tool-calling capability on its own** — the smolagents team itself flags weak VLMs of this class as unreliable for tool use, which is exactly why it is *not* asked to do that job in the pipeline; weaker doc scores standalone (DocVQA 58.3, OCRBench 52.6/100 ≈ 526/1000, trailing LFM's 684 — its role is fast triage, LFM2.5-VL is still the one that finalizes); qualitative "works great" reports only, no hard latency/memory numbers found for the 256M variant |
| Phi-4-multimodal / Phi-4-mini (MIT, 3.8B-class) — considered, not selected | MIT-licensed with built-in function calling — the only *permissive + agentic* combination found, at ~8× the size | 3.8B ≈ 2.2–2.5 GB Q4 — flagship-phone territory, not the mid-range Android floor; no CPU tok/s numbers found anywhere (GAP); kept as background reference only — the operator's 2-model decision is final, this is not an escape hatch to relitigate |
| — (rejected) | Moondream 3 (BSL 1.1 — commercial-competing use needs agreement), PaliGemma 2 (≥3B, not agentic), Gemma 3 270M/1B (**text-only** — no vision encoder below 4B), Florence-2 (MIT but not conversational/agentic), Qwen2.5-VL-3B (**license CONTESTED** in sources — Apache-2.0 vs restrictive Qwen Research License; unusable until verified from the actual repo license file), MiniCPM-V 4.6 (license unconfirmed — research GAP), MiniCPM-V 4.5 (8B — wrong tier), Gemma 3n (not deep-verified — no verdict, honestly) | | |

The honest core of the tradeoff, unchanged by the crosswiring: **the permissive model cannot
do the agentic half of the job on any evidence available, and the capable model is not
open-source.** The crosswired pipeline is precisely the engineering answer to that tradeoff —
use the permissive model for the part it is actually good at (fast perception) and the
capable model for the part that needs real reasoning — rather than picking one loser or
pretending either half away.

### 3.3 O-1 — LFM Open License: RULED 2026-07-20, ship authorized

This was a business decision, not a footnote, and it has now been made: **the operator ruled
"clear to ship."** LFM2.5-VL-450M ships under the LFM Open License v1.0 despite the
$10M-revenue-cap term below. The term is recorded here in full, not erased, per this repo's
documentation discipline (decisions get recorded, never scrubbed) — it is simply no longer a
gate blocking Phase A or Phase C.

- **The term (for the record):** LFM Open License v1.0 is Apache-2.0-*based* but **not
  OSI-compliant**: free commercial use only while the using company's **annual revenue is
  under $10M USD**; above that, a commercial agreement with Liquid AI is required. Nonprofits
  exempt.
- **Who is "the using company" in a decentralized mesh?** Still textually unresolved by the
  license itself — dowiz is plausibly the licensee-distributor if it ships the courier app
  with embedded weights, but hubs are *independent venue businesses*, and a venue group above
  $10M revenue operating its own hub could independently trip the cap. The revenue trigger is
  a **field-of-use restriction that travels with the weights** into every node of a mesh whose
  whole thesis (D0: decentralized · local-first) is that any node can self-host and fork
  without asking anyone. **This ambiguity is unchanged by the ruling** — the operator decided
  to ship despite it, not that the ambiguity was resolved. Any hub/venue operator materially
  above the $10M threshold remains a live open question for *their* compliance, not dowiz's.
- **Repo-policy note, stated exactly:** dowiz itself is AGPL-3.0-or-later, and `deny.toml`
  restricts *crate* licenses to OSI-approved/permissive. Model weights are not crates and
  `cargo-deny` will never see them — LFM2.5-VL-450M is thus the **first non-OSI-licensed
  artifact in the shipped stack**, entering through a gap in the automated gate. That fact is
  now a recorded, ruled-on exception, not an oversight.
- **Practical read (context for the ruling, not a re-litigation):** for Albania/EU Wave-0,
  every realistic participant is far below $10M — the cap costs nothing *today*. The
  alternative the operator declined — permissive-only, cutting scenario classes (b)/(c) until
  the permissive ecosystem catches up — was the honest cost of refusing LFM; the operator
  judged the capability worth the license term.
- **Ruling record (O-1, closed):** adopt LFM under the license. A DECISIONS.md D-entry
  quoting the $10M term and this ruling is the recommended paper trail (not yet filed by this
  blueprint — see §8 for the closed-decision log). No further LFM-license discussion is
  required to ship Phase A or Phase C.

### 3.4 Architecture guard: the pair must stay cheap to reverse

The mobile app necessarily runs its models **in-process** (there is no Ollama daemon on a
courier's phone). That does *not* touch the wiring blueprint's §1 lock — that lock scopes
`llm-adapters`/kernel on the hub, and P101 keeps it verbatim (§7). The mobile discipline is
the **`AsrModel` precedent** from `engine/src/voice.rs`, applied to vision: a small
`VlmModel`-style port (typed errors mirroring `LlmError`/`InferError`, offline fixture stub
satisfying the contract with no model present), with the concrete llama.cpp/GGUF-backed
implementation living at the app edge (`apps/courier` or a sibling edge crate) behind an
**off-by-default Cargo feature** with the standard what-it-pulls-in header. Model identity is
an asset + config value, never a type — this now covers **two** concurrently-loaded model
identities behind the same port shape (a `VlmModel` instance per stage, or one pipeline entry
point wrapping both), not one. Since O-1 is ruled (§3.3), there is no pending license-driven
swap trigger left on this pair; the port discipline still means either stage's weight file is
swappable by construction if future evidence ever warrants it — re-running §3.5's suite on the
replacement, not an architecture change.

### 3.5 Validation — the model-agnostic courier scenario suite

The pair is validated the way this repo validates everything: a suite that can go RED.

- **Fixture set:** N ≥ 12 real courier scenarios across the §3.1 classes — PoD photo sanity
  (package visible / not visible / occluded), receipt line-item OCR, menu OCR, package-label
  read, and ≥ 3 **negative cases** (ambiguous photos where the required output is the refusal
  form, not a guess). Expected outputs are structured fields / expected-substring checks —
  mechanical scoring, **no LLM-as-judge** (wiring blueprint §3.2 ban applies).
- **Agentic half:** the same suite format as the wiring blueprint's golden tool-calling suite,
  with vision-conditioned tool selection cases (e.g. "photo + 'log this delivery'" must select
  the capture tool with the right arg keys, and must select `no_tool` on the ambiguous cases).
  Because the pipeline is two-stage, each fixture asserts at **both** the SmolVLM first-pass
  output (did perception extract the right evidence) and the LFM2.5-VL final decision (did
  reasoning act correctly on that evidence) — a fixture can fail at either stage, and the
  suite records which.
- **Hardware:** at least one real mid-range Android device representative of the Wave-0
  courier fleet. Latency + peak-RSS recorded per scenario, for the pipeline end-to-end and per
  stage; the sub-250 ms LFM claim marked **reproduced or not reproduced** (verbatim A4.4
  Moonshine-precedent wording).
- **Pipeline validation, not a comparative bake-off:** the two models are not being scored
  against each other for a single winner-takes-the-slot decision (O-1 already settled which
  models ship, §3.3) — the suite validates the **crosswired pipeline as a whole**: does
  SmolVLM's first-pass evidence measurably help LFM2.5-VL's final decision (ablation: run
  LFM2.5-VL alone vs. LFM2.5-VL fed SmolVLM's first pass, on the same fixtures) and does the
  combined pipeline clear the bar that neither model alone reliably clears (SmolVLM has no
  agentic evidence; LFM2.5-VL alone is only "measurable, not reliable" per §3.2).

## 4. Server-side topology for the 4-core/30GB no-GPU box

### 4.1 Serving substrate: Ollama's native multi-model support — build nothing

The research confirms Ollama natively serves concurrent models (since 0.2):
`OLLAMA_MAX_LOADED_MODELS` (defaults to **3 on CPU**), `OLLAMA_NUM_PARALLEL` (default **1**;
RAM scales as NUM_PARALLEL × context length), LRU-ish unload under memory pressure. vLLM's
multi-model story is GPU-oriented and still evolving (open issue #3326) — and the wiring
blueprint already treats vLLM as a Quirks preset for *later*, not a deployment. `llm-adapters`
already speaks to Ollama; the entire topology below is **Ollama env vars + the wiring
blueprint's Phase-3 `TaskClass`→model config surface**. Zero new serving infrastructure, zero
new crates, zero new dependencies.

### 4.2 Residency plan (what is loaded), grounded in the call sites

The abstract E/S/G/C tier system this section previously carried (one open bake-off slot, one
"pick empirically" slot) is superseded: the operator named the pair directly, and it is the
**same pair as the mobile pick (§3.2)**, resident and crosswired concurrently on the server:

| Component | Model | Serves | Budget note |
|---|---|---|---|
| Embedding, always hot — a different task shape (encoder, not decode), not one of "the 2 models" | `nomic-embed-text` (274 MB; or `qwen3-embedding:0.6b` after the wiring blueprint's §4 default-flip decision — one, not both, as default) | CS-3 (leak gate, retrieval) | Effectively free; never evicted; does not compete for the box's 2-decode-stream budget |
| **Perception — fast first-pass** | **SmolVLM-256M-Instruct** (Apache 2.0) | CS-1 agent-loop context triage; CS-4 intake assist; CS-5 comms drafting — first stage on every product-facing lane | Sub-1 GB; always hot; fills decode-stream slot 1 |
| **Reasoning / tool-selection** | **LFM2.5-VL-450M** | CS-1 agent-loop final tool call; CS-4/CS-5 final structured output; cascade target for CS-4 escalations (§4.4) | Sub-1 GB; always hot; fills decode-stream slot 2; O-1 ruled (§3.3) — ships |
| Dev/harness lane — out of the 2-model product topology, unaffected by this correction | `qwen2.5-coder:7b` (Q5_K_M upgrade per wiring blueprint §4) | CS-2 harness only | Not product-path; cold-load tolerated on idle-evict, exactly as before |
| (adjacent) | ASR model (whisper-family or Moonshine, CS-6) | voice lane | Counted in the RAM budget, designed elsewhere (A4 voice arc) |

Total *product-topology* resident weights: embedding (0.27–0.64 GB) + SmolVLM-256M (≈0.3–0.6
GB) + LFM2.5-VL-450M (sub-1 GB per §3.2) + KV/context ≈ **1.5–2.5 GB** — an order of magnitude
lighter than the previous llama3.1:8b + Tier-S design's ~11–12 GB, and trivially comfortable
inside 28 GiB available. `qwen2.5-coder:7b` (≈4.7 GB) is not part of this floor — it cold-loads
only when the dev/harness lane needs it, exactly as before this correction.

`OLLAMA_MAX_LOADED_MODELS` set to **3** (embedding + the 2 crosswired models — the entire
product floor, always resident; `qwen2.5-coder:7b` deliberately sits outside this floor and is
left to Ollama's default LRU eviction, since it already tolerated cold-load before this
correction). `OLLAMA_NUM_PARALLEL` stays **1**: combined with the dispatcher's locked
`workers: 2`, the whole system tops out at **exactly 2 concurrent decode streams —
SmolVLM-256M-Instruct and LFM2.5-VL-450M, by name, not by abstract tier**. Both are far
smaller than the llama3.1:8b-class model this budget was originally measured against (§2), so
the bandwidth ask is materially lighter — Phase B (§4.5) still measures rather than assumes.

### 4.3 Why the mobile pair, not a separate server-side text model (resolved, no bake-off)

This section previously ran an open "Tier-S candidate question" — pick a small server-side
text model **empirically** from a thin shortlist, with a separate "Tier-G replacement"
question floated alongside it. **Both questions are superseded, not answered from that
shortlist:** the operator's 2026-07-20 correction locks the server topology onto the **same
SmolVLM-256M-Instruct + LFM2.5-VL-450M pair already selected for mobile (§3.2)**, rather than
running a second, separate selection process server-side. §8 O-2 (the bake-off decision) is
resolved the same way — see §8.

The prior shortlist is kept here for the record, not deleted, in case a future measurement
shows the crosswired pair cannot carry a specific server lane and a different model is needed:

- **LFM2.5-1.2B** (IFEval 86.23, MMLU-Pro 44.35; vendor-claimed 239 tok/s decode on an
  *unspecified* "AMD CPU" — the one CPU number in class, but unverifiable as stated).
- **LFM2.5-350M/230M** — same LFM Open License as LFM2.5-VL-450M; O-1 (§3.3, now CLOSED) would
  cover these too if ever adopted, but they were not selected.
- **Phi-4-mini** (MIT, built-in function calling) — the license-clean candidate at 3.8B ≈
  2.2–2.5 GB Q4; no CPU tok/s found (GAP); not selected because the operator named the mobile
  pair directly.
- **Gemma 3 1B/270M** — text-only, Gemma license (permits commercial use with a
  prohibited-use policy); not selected.
- **LFM2.5-8B-A1B** (MoE, 1.5B active, <6 GB, strongest agentic story in the lineup) — was
  floated as a potential future large-tier replacement; not selected.

Rationale for reuse over a fresh pick: one artifact family across mobile and server means one
license ruling (O-1, closed), one validation suite (§3.5, extended with server-side fixtures
in §4.4), and a resident-weight footprint an order of magnitude lighter than a 7–8B-class
"strong generalist" would have required (§4.2). If Phase B/C measurement ever shows the
crosswired pair genuinely insufficient for a lane (most likely CS-1's tool-calling depth,
given BFCLv4 21.08 is "measurable, not reliable" per §3.2), this shortlist is where the next
candidate comes from — it is not a reason to relitigate the pair today.

### 4.4 The crosswired pipeline, routing, and cascading — applied, not essayed

The research's distinction, applied to dowiz's actual lanes, now with the operator-named pair:

- **The crosswired pipeline is the default shape for every product lane, not a routing
  choice.** CS-1, CS-4, and CS-5 each run SmolVLM-256M-Instruct's fast first-pass followed by
  LFM2.5-VL-450M's reasoning/tool-selection on the same request — both models loaded, both
  invoked, concurrently and complementarily (§2, §3.2), never one held dormant as a standby
  for the other. For **CS-1** specifically (text-only tool-calling — no image is involved):
  SmolVLM performs fast context triage / observation compression on the current agent-loop
  turn (bounded by `MAX_AGENT_ITERATIONS = 4`), and LFM2.5-VL-450M is the model that actually
  decides the next tool call or final answer, grounded in SmolVLM's compressed context. This
  concrete data-flow is a **design proposal at blueprint stage** — nothing is built — and gets
  proven or falsified by re-running the wiring blueprint's golden tool-calling suite against
  the new pair before Phase C ships (§6); it replaces `llama3.1:8b` as CS-1's model, which is
  the one operationally material change this correction makes to a currently-live call site.
- **ROUTING (decide before generation — RouteLLM-class) still exists, but now only picks the
  lane, not the model.** dowiz already has the degenerate form: `TaskClass` static routing in
  `ollama.rs::route_model`, becoming configurable via the wiring blueprint's Phase 3. P101
  extends the *mapping*: CS-1/CS-4/CS-5 all route to the crosswired pair — there is no longer a
  per-lane model choice to route between, since there is only the one pair; CS-2 continues to
  route to the dev-lane `qwen2.5-coder:7b`, unaffected and out of scope for this correction.
  **No new `TaskClass` variant, no kernel enum change** in v1. **Learned routing is explicitly
  rejected here**: RouteLLM trains its router on preference data dowiz does not have, and the
  HK-05/HK-09 arc already owns real-time model routing — the wiring blueprint's §7
  non-duplication ruling stands verbatim.
- **CASCADING (escalate on insufficient quality — FrugalGPT/AutoMix-class) — kept for exactly
  one lane, CS-4 intake assist, as a bounded latency/cost optimization layered on top of the
  crosswired default, not a primary/fallback trust hierarchy between the two models.** CS-4 is
  the only lane with a *deterministic* sufficiency oracle: the assisted output either
  re-parses through the pure `IntentParser` into a valid intent, or it is still `Ambiguous`.
  Cascade contract, restated with the named pair: SmolVLM's first-pass attempt → if still
  `Ambiguous`, one LFM2.5-VL-450M attempt grounded in that first pass → if still `Ambiguous`,
  the existing human-handling path (which P48-INTAKE §8.7 already mandates as the backstop).
  Bounded, two rungs, no retry loop, no LLM-judge arbiter anywhere. **This is not the
  primary/fallback pattern the operator rejected**: both models are resident and already run
  together on every CS-4 request as the crosswired default above; the cascade only bounds how
  much *extra, quality-gated* work happens after that default pipeline's first answer proves
  insufficient — gated by a deterministic oracle, never by distrust of either model. **CS-1
  (agent loop) deliberately does not cascade beyond the crosswired default:** a
  wrong-but-well-formed agent answer has no mechanical detector, so any additional escalation
  trigger there would be judge-shaped — the exact technique the wiring blueprint's §3.2
  evidence base rejects. The loop's existing bounded-iteration + tool-schema fail-closed
  design, now running the crosswired pair, *is* its quality control.
- **Speculative decoding** — noted as distinct from routing (research), single-model-family
  draft-and-verify. Not designed here; Ollama-internal if it ever applies. Out of scope.
- Since P48-INTAKE is itself PLAN, the cascade lands **with** that build; P101's deliverable is
  this contract plus the fixture spec (§6 Phase C), so the intake build consumes a settled
  design instead of re-deriving one.

> **SUPERSEDED IN PART — 2026-07-20 (later, R-2):** once the P102 native pair serves the
> intake-assist lane, the Tier-S→Tier-G escalation rung *inside the model pair* is replaced by
> the P102 §3.2 Mode-B crosswire (S perception draft conditions L's extraction in one pass).
> Everything deterministic here is unchanged and binding: the pure `IntentParser` remains the
> sufficiency oracle, and the human-handling backstop remains the terminal rung. Until the
> P102 Phase-6 cut-over gate for this lane is green, this section's cascade stands as written.

### 4.5 The measurement this whole section stands on (Phase B)

The research could not find a head-to-head "N small vs 1 big on shared cores" benchmark, and
this repo does not ship topology on third-party numbers anyway. Phase B is a scripted,
operator-run measurement matrix **on this exact box** (fixed prompts, temperature 0 where
honored, decode tok/s + prefill tok/s + P99 wall-clock per cell, ≥ 3 runs per cell,
`/api/generate` against the live daemon) — updated to the operator-named, crosswired pair:

| Cell | What it answers |
|---|---|
| B-1: `LFM2.5-VL-450M` solo | the box's single-stream baseline for the reasoning stage |
| B-2: `SmolVLM-256M-Instruct` solo | the box's single-stream baseline for the perception stage |
| B-3: `SmolVLM-256M-Instruct` + `LFM2.5-VL-450M` concurrent (2 dispatch workers) | the actual production shape: the crosswired pipeline itself, both stages decoding together |
| B-4: the concurrent pair (B-3) + `nomic-embed-text` embed burst | does the always-hot embed lane perturb the crosswired pipeline |
| B-5: all 3 product-topology models resident, idle → cold-vs-warm first-token on each | residency plan's reload-thrash claim, quantified for the now much lighter 3-model floor |

Results go in a committed scorecard (`docs/audits/` row or the blueprint's companion `.jsonl`,
same pattern as the wiring blueprint's regression scorecard) — **pass/fail probe, never
baseline-gated CI** (host-noisy LLM measurements stay out of the deterministic gate set, per
standing bench policy). Decision rule, falsifiable: if B-3 degrades LFM2.5-VL-450M's decode by
more than **25%** vs B-1, concurrent crosswired decode is disallowed for latency-sensitive
lanes (dispatcher stays at 2 workers but the composition serializes LFM2.5-VL behind SmolVLM's
completion for product lanes, i.e. the pipeline runs sequentially instead of concurrently) and
that is recorded as *measured*; if under 25%, the concurrent crosswired shape ships as
designed. Both models are roughly two orders of magnitude smaller than the llama3.1:8b
baseline this section originally measured against, so the expectation is that 25% clears
comfortably — but the number, not the expectation, is what ships.

## 5. Fine-tuning / training deferral — reconsidered against the new evidence: **HOLDS, now stronger**

What the research actually found, stated plainly:

- **No real CPU-only training path exists for this box.** Unsloth (the dominant LoRA/QLoRA
  tool) explicitly does not support CPU training — its own LFM2.5 tutorial reaches for a free
  Colab T4. bitnet.cpp is inference-only, no PEFT. The **one** vendor-backed CPU-training
  counter-example, IPEX-LLM QLoRA-on-CPU, requires **Intel AMX** — and §1 *verified on this
  box today* that Zen3 EPYC-Milan exposes zero AMX flags. On-device phone LoRA (QVAC Fabric,
  MobileFineTuner, MobiLLM) all lean on phone GPU/NPU or server-assisted backprop — none is a
  CPU-only demonstration.
- **Memory capacity is plausibly NOT the blocker** (a 2026 result pushes Llama-3.2-3B + LoRA
  training memory to ~1 GB via quantization + efficient checkpointing — trivially inside
  30 GiB). **Wall-clock speed on AMX-less Zen3 is the genuinely unmeasured question** — no
  source anywhere benchmarked it, in either direction.

**Ruling: P54's deferral stands, unchanged in substance.** `TRIGGER-FINETUNE` (≥500 verified
examples AND a measured prompt-only baseline AND that baseline found insufficient AND **a GPU
host**) already encodes the right gate, and the new research *strengthens* its fourth clause:
the fallback fantasy "we could always train slowly on the server CPU" now has three named
tools refusing it and one ISA check disproving the only vendor path. No urgency is
manufactured here because none exists: dowiz's actual LLM tasks (tool-calling, classification
over its own vocabulary) remain squarely in good-small-model + tight-schema + few-shot
territory (wiring blueprint §5, reaffirmed).

**The one cheap empirical step worth offering (Phase D, optional, operator-gated):** the
wall-clock gap is answerable locally for approximately one evening of machine time, and
answering it kills the question permanently instead of leaving it to be re-litigated:

- **Probe:** HF PEFT LoRA fine-tune of a ≤0.6B open model, 100 optimizer steps on a synthetic
  ~500-example corpus, 4 threads, this box. Measure seconds/step; extrapolate to a full
  3-epoch run over a TRIGGER-FINETUNE-scale corpus. Throwaway venv, off the request path, no
  repo dependency changes, no CI wiring — the artifact is one recorded number, not a tool.
- **Falsifiable acceptance:** if the extrapolated full run is **≤ 24 h wall-clock**, amend
  TRIGGER-FINETUNE's fourth clause to "a GPU host *or the measured CPU path*" (with the number
  cited); if **> 24 h** (the expected outcome), the trigger stands verbatim and the recorded
  number becomes the permanent answer to "why not just train on the server". Either outcome
  closes the research's gaps #7/#8 with local data instead of more searching.

## 6. Phasing — RED→GREEN per phase, consistent with the wiring blueprint's build order

Ordering note: P101 consumes the wiring blueprint's Phase 1 (AiMode composition switch) and
Phase 3 (configurable model map) as its config substrate; nothing in P101 blocks them.

- **Phase A — mobile suite.** RED: no vision scenario suite exists (trivially true — no VLM
  anything exists). GREEN: §3.5 suite exists with self-falsification proven (a scrambled
  expected-field must fail), run against the crosswired SmolVLM/LFM2.5-VL pipeline on ≥1 real
  device, scorecard committed, sub-250 ms claim marked reproduced/not, the ablation from §3.5
  recorded. **Gate: none remaining** — O-1 is RULED (§3.3, 2026-07-20, operator: "clear to
  ship"), so Phase A proceeds straight to suite execution.
- **Phase B — the §4.5 measurement matrix.** RED by absence (no concurrency numbers exist for
  this box for the named pair). GREEN: all 5 cells recorded, ≥3 runs each, decision rule
  applied and outcome written back into this document's §4.5.
- **Phase C — residency + routing enactment.** RED: `OLLAMA_MAX_LOADED_MODELS` unset (default
  3) and neither SmolVLM-256M-Instruct nor LFM2.5-VL-450M resident on the server (the box
  today still runs the pre-correction `llama3.1:8b`/`qwen2.5-coder:7b` pair per §1's live
  snapshot — this is the one operationally material cutover this correction implies). GREEN:
  env + model map configured per §4.2's named pair (no bake-off — §4.3, §8 O-2 resolved); one
  wiring-blueprint-golden-suite run recorded under the new map, specifically validating the
  crosswired CS-1 pipeline that replaces `llama3.1:8b`; the CS-4 cascade contract + fixture
  spec handed to the P48-INTAKE build (its RED→GREEN lands there, with a fixture `Ambiguous`
  message that must escalate exactly once and then stop).
- **Phase D — optional CPU-LoRA wall-clock probe (§5).** Operator-gated (O-3); acceptance
  criterion self-contained above.

## 7. Non-goals (explicit, closed list)

- No reopening of the wiring blueprint's §1 locks: `llm-adapters` stays a sync HTTP client;
  no in-process inference in the hub agent lane; 2-worker dispatch and single-backend-per-
  deployment stand. (Mobile in-process inference is a different binary and follows §3.4's
  port discipline — it never enters `llm-adapters` or the kernel.)
- No learned/real-time router — HK-05/HK-09 owns that arc; P101 is static task routing + one
  deterministic cascade, recorded as beneath that arc's threshold by design.
- No LLM-as-judge anywhere: not in the mobile suite, not as a cascade arbiter, not in CI.
- No vision output in settlement authority, ever: `DeliveryClaim` keeps no photo field; VLM
  assist is capture-UX only. (Any future evidence-attachment design belongs to P52 §3.4, and
  even there PoD settles on signatures, not pixels.)
- No new kernel types in v1 (no `TaskClass` variant; per-request `model_id` override suffices).
- No model weights in the git repo; no model-registry/auto-pull system (wiring §2.3 stance).
- No fine-tuning/training build of any kind (§5 — Phase D is a measurement, not infrastructure).
- No custom serving daemon / scheduler — Ollama env + existing config surface only.
- No re-opening O-1 (§3.3) — it is RULED (2026-07-20, "clear to ship"), a closed decision, not
  a discussion item; the license term stays recorded for awareness (§3.3), never relitigated.
- No abstract/pluggable N-tier model system — exactly 2 named models, full stop (§2).

## 8. Decisions log — resolved and open

**Resolved (2026-07-20, operator-directed, recorded not erased):**

- **O-1 — LFM Open License adoption: RULED.** Operator, verbatim: *"clear to ship."*
  LFM2.5-VL-450M ships under the LFM Open License v1.0 despite the $10M-revenue-cap term
  (§3.3 carries the full record: the term, the unresolved "who is the using company" question
  in a decentralized mesh, and the repo-policy note that this is the first non-OSI-licensed
  artifact in the shipped stack). This closes the three-option framing this blueprint
  originally posed (adopt-with-D-entry / permissive-only / defer-VLM-entirely) by choosing
  (i) directly. Recommended follow-up, not yet filed by this blueprint: a DECISIONS.md D-entry
  quoting the term and this ruling, per §3.3's closing note.
- **O-2 — Tier-S bake-off pair: SUPERSEDED, no bake-off ran or runs.** The operator named the
  server-side pair directly — the same SmolVLM-256M-Instruct + LFM2.5-VL-450M pair as the
  mobile pick (§3.2, §4.2, §4.3) — rather than leaving a "pick ≤2 candidates, run a bake-off"
  process open. The prior Tier-S shortlist (LFM2.5-1.2B, Phi-4-mini, Gemma 3, LFM2.5-8B-A1B)
  is kept for the record in §4.3, not deleted, in case a future lane genuinely needs a
  different model — but no bake-off is scheduled.

**Still open:**

1. **O-3 — run Phase D at all.** Costs ~one evening of machine time, closes research gaps
   #7/#8 permanently. Recommendation: yes, precisely because it is cheap and terminal — but it
   changes no near-term plan either way, so "no" is a legitimate answer.

---

*End of blueprint. Everything above is design + measurement plans; nothing is built. O-1 is
RULED (2026-07-20, "clear to ship") and no longer gates the sequence; the load-bearing
sequence is now Phase B (pure local measurement, startable immediately) → Phase C, with
Phase A riding the P52/P71 timeline independently; Phase D is optional and terminal.*
