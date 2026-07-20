# BLUEPRINT P97 — Local & Mobile Model Selection + CPU-Only Serving Topology (2026-07-20)

**Status: BLUEPRINT / PLAN — no code written, nothing built, no model pulled.**
**Date:** 2026-07-20
**Component:** AGENT (mobile half also feeds DELIVERY P52/P71)
**Source of facts:** the completed operator-commissioned deep-research pass on (1) small
VLM/agentic models for a mobile courier app, (2) multi-model CPU-only serving topology, and
(3) CPU-only fine-tuning feasibility — synthesized here against the live tree and against
live spot-checks run on the actual deployment box today (§1). Every benchmark number in this
document is quoted verbatim from that research; every claim about dowiz code was re-verified
against the tree at `docs/local-model-synthesis-2026-07-20` (based on `origin/main` @
`09859aaa6`). Where the research has a named GAP, this blueprint either proposes a cheap
empirical closure or says the gap stays open — it never papers over one.

**Relationship to the local-model-wiring blueprint
([`BLUEPRINT-LOCAL-MODEL-WIRING-TESTING-USAGE-2026-07-20.md`](BLUEPRINT-LOCAL-MODEL-WIRING-TESTING-USAGE-2026-07-20.md)):**
complementary, not competing. That blueprint owns the *wiring* (AiMode composition switch,
golden tool-calling suite, configurable `TaskClass`→model map) and deliberately left three
questions out of scope: **which** models (it took the four resident ones as given), whether a
**multi-model topology** on the actual box is worth designing, and whether the **training
deferral** survives contact with real CPU-training research. P97 answers those three. Its §1
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
  this box's RAM and memory bandwidth, so §4's budget counts it even though P97 doesn't design it.

**And the honest statement about vision:** there is **no VLM call site anywhere in the tree
today**. The PoD settlement artifact (`DeliveryClaim`, bebop2 `pod.rs`) has **no photo field**
and settles on k-of-n hub signatures — vision output is structurally incapable of gating
settlement, and P97 keeps it that way (§7). The mobile VLM pick is forward-provisioning for
P52 K4's capture-assist UX (photo sanity/framing help, receipt & menu OCR, label reading), not
a wired consumer.

## 2. The operator's topology idea, re-grounded in this box's physics

The operator's framing — "simple models for easy tasks, one strong reasoning model for hard
tasks" — is architecturally right, and the research confirms it matches an established
pattern (routing/cascading, §4.4). But one implied reading must be corrected before design:
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

So the design target is: **many models *resident*, few models *decoding*.** RAM capacity
(~28 GiB vs ~11 GB of currently-resident weights) is provably not the constraint; concurrent
decode bandwidth is. Ollama's native controls express exactly this split
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

### 3.2 The shortlist, ranked

| Rank | Model | For | Against |
|---|---|---|---|
| **1 (primary)** | **LFM2.5-VL-450M** (Liquid AI, 2026-04-08) | The only sub-1B candidate with *measured* agentic evidence at all (BFCLv4 **21.08**) — modest in absolute terms, but every alternative in class has **zero** tool-calling evidence; OCRBench **684**/1000 ≈ 68.4 (the strongest doc-OCR signal in the size class); RefCOCO-M 81.28 (grounding/pointing — useful for framing assist); sub-250 ms edge-inference *claim*; day-one GGUF/llama.cpp + ONNX + LEAP iOS/Android SDK | **LFM Open License v1.0 — NOT OSI; $10M revenue cap (§3.3)**; BFCLv4 21.08 means "measurable, not reliable" — the scenario suite (§3.5) decides whether it clears the bar; sub-250 ms claim is vendor-published on unspecified hardware — reproduce, never trust (Moonshine-precedent discipline) |
| **2 (license-clean fallback)** | **SmolVLM-256M-Instruct** (Apache 2.0) | Fully permissive; GGUF/ONNX; <1 GB single-image inference; proven phone deployments (llama.cpp, HuggingSnap) | **No demonstrated agentic/tool-calling capability** — the smolagents team itself flags weak VLMs of this class as unreliable for tool use; weaker doc scores (DocVQA 58.3, OCRBench 52.6/100 ≈ 526/1000 — scale-normalized it still trails LFM's 684); qualitative "works great" reports only, no hard latency/memory numbers found for the 256M variant |
| 3 (heavier tier, permissive) | **Phi-4-multimodal / Phi-4-mini** (MIT, 3.8B-class) | MIT-licensed with built-in function calling — the only *permissive + agentic* combination found, at ~8× the size | 3.8B ≈ 2.2–2.5 GB Q4 — flagship-phone territory, not the mid-range Android floor; no CPU tok/s numbers found anywhere (GAP); keep as the escape hatch if sub-1B agentic quality proves insufficient on the suite |
| — (rejected) | Moondream 3 (BSL 1.1 — commercial-competing use needs agreement), PaliGemma 2 (≥3B, not agentic), Gemma 3 270M/1B (**text-only** — no vision encoder below 4B), Florence-2 (MIT but not conversational/agentic), Qwen2.5-VL-3B (**license CONTESTED** in sources — Apache-2.0 vs restrictive Qwen Research License; unusable until verified from the actual repo license file), MiniCPM-V 4.6 (license unconfirmed — research GAP), MiniCPM-V 4.5 (8B — wrong tier), Gemma 3n (not deep-verified — no verdict, honestly) | | |

**Recommendation: LFM2.5-VL-450M as the primary pick — conditional on the §3.3 license
ruling — with SmolVLM-256M-Instruct as the standing license-clean fallback, and the entire
choice held swappable by construction (§3.4).** The honest core of the tradeoff: **the
permissive option cannot do the agentic half of the job on any evidence we have, and the
capable option is not open-source.** Pretending either half away would be the real mistake.

### 3.3 ⚠ THE LICENSE DECISION POINT — operator ruling required before any LFM weight ships

This is a business decision, not a footnote, and it needs to be made **now**, not discovered
at distribution time:

- **The term:** LFM Open License v1.0 is Apache-2.0-*based* but **not OSI-compliant**: free
  commercial use only while the using company's **annual revenue is under $10M USD**; above
  that, a commercial agreement with Liquid AI is required. Nonprofits exempt.
- **Who is "the using company" in a decentralized mesh?** Unresolved by the license text we
  have. If dowiz distributes the courier app with embedded weights, dowiz is plausibly the
  licensee-distributor; but hubs are *independent venue businesses* — a venue group above
  $10M revenue operating its own hub could independently trip the cap. The revenue trigger
  is a **field-of-use restriction that travels with the weights** into every node of a mesh
  whose whole thesis (D0: decentralized · local-first) is that any node can self-host and
  fork without asking anyone.
- **Repo-policy friction, stated exactly:** dowiz itself is AGPL-3.0-or-later, and
  `deny.toml` restricts *crate* licenses to OSI-approved/permissive. Model weights are not
  crates and `cargo-deny` will never see them — which is precisely why this needs an explicit
  recorded ruling: it would be the **first non-OSI-licensed artifact in the shipped stack**,
  entering through a gap in the automated gate.
- **Practical read, both directions:** for Albania/EU Wave-0, every realistic participant is
  far below $10M — the cap costs nothing *today*, and Liquid's terms are unusually clear as
  these things go. The real cost is strategic: embedding a revenue-gated dependency in the
  sovereignty story. The real cost of *refusing* it: shipping a fallback VLM with no
  demonstrated agentic ability, i.e. cutting scenario classes (b)/(c) until the permissive
  ecosystem catches up.
- **What this blueprint asks for:** a named operator ruling (§8 O-1) with three options —
  **(i)** adopt LFM under the license, recorded as a DECISIONS.md D-entry with the $10M term
  quoted and a standing swap trigger; **(ii)** permissive-only (SmolVLM now, Phi-4-class for
  agentic scenarios on capable devices); **(iii)** defer any VLM shipping until P52/P71 build
  actually needs it (the zero-cost option, since no call site exists yet). No LFM weight is
  pulled into any distributed artifact before this ruling exists.

### 3.4 Architecture guard: the pick must stay cheap to reverse

The mobile app necessarily runs its model **in-process** (there is no Ollama daemon on a
courier's phone). That does *not* touch the wiring blueprint's §1 lock — that lock scopes
`llm-adapters`/kernel on the hub, and P97 keeps it verbatim (§7). The mobile discipline is
the **`AsrModel` precedent** from `engine/src/voice.rs`, applied to vision: a small
`VlmModel`-style port (typed errors mirroring `LlmError`/`InferError`, offline fixture stub
satisfying the contract with no model present), with the concrete llama.cpp/GGUF-backed
implementation living at the app edge (`apps/courier` or a sibling edge crate) behind an
**off-by-default Cargo feature** with the standard what-it-pulls-in header. Model identity is
an asset + config value, never a type. Consequence: if the §3.3 ruling ever flips, the swap
is re-running §3.5's suite on the replacement — not an architecture change.

### 3.5 Validation — the model-agnostic courier scenario suite

The pick is validated the way this repo validates everything: a suite that can go RED.

- **Fixture set:** N ≥ 12 real courier scenarios across the §3.1 classes — PoD photo sanity
  (package visible / not visible / occluded), receipt line-item OCR, menu OCR, package-label
  read, and ≥ 3 **negative cases** (ambiguous photos where the required output is the refusal
  form, not a guess). Expected outputs are structured fields / expected-substring checks —
  mechanical scoring, **no LLM-as-judge** (wiring blueprint §3.2 ban applies).
- **Agentic half:** the same suite format as the wiring blueprint's golden tool-calling suite,
  with vision-conditioned tool selection cases (e.g. "photo + 'log this delivery'" must select
  the capture tool with the right arg keys, and must select `no_tool` on the ambiguous cases).
- **Hardware:** at least one real mid-range Android device representative of the Wave-0
  courier fleet. Latency + peak-RSS recorded per scenario; the sub-250 ms LFM claim marked
  **reproduced or not reproduced** (verbatim A4.4 Moonshine-precedent wording).
- **Comparative by construction:** the suite runs unchanged against primary and fallback;
  the scored delta is the recorded justification for whichever ships.

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

| Tier | Model | Serves | Budget note |
|---|---|---|---|
| **E — embedding, always hot** | `nomic-embed-text` (274 MB; or `qwen3-embedding:0.6b` after the wiring blueprint's §4 default-flip decision — one, not both, as default) | CS-3 (leak gate, retrieval) | Effectively free; never evicted |
| **S — small workhorse** | ONE small instruct model for classification-grade work — candidates in §4.3 | CS-4 intake assist, CS-5 comms drafting | Target ≤ 1 GB file; the "simple tasks" lane of the operator's idea |
| **G — strong generalist** | `llama3.1:8b` Q4_K_M (resident) | CS-1 agent loop; cascade target for CS-4 escalations | The "one strong reasoning model" |
| **C — code (dev lane)** | `qwen2.5-coder:7b` (Q5_K_M upgrade per wiring blueprint §4) | CS-2 harness only | Not product-path; tolerate cold-load when idle-evicted |
| (adjacent) | ASR model (whisper-family or Moonshine, CS-6) | voice lane | Counted in the RAM budget, designed elsewhere (A4 voice arc) |

Total resident weights ≈ 4.9 + 4.7 + ≤1 + 0.3–0.6 GB ≈ **11–12 GB** + KV/context + ASR —
comfortable in 28 GiB available. `OLLAMA_MAX_LOADED_MODELS` set to **4** (E+S+G+C; measured
in Phase B before being trusted — the default of 3 would evict one workhorse and cause
reload thrash, which is the *actual* multi-model failure mode on a residency-rich box, not
OOM). `OLLAMA_NUM_PARALLEL` stays **1**: combined with the dispatcher's locked `workers: 2`,
the whole system tops out at **two concurrent decode streams across different models** —
which §2 argues is already at or past this box's bandwidth comfort, and which Phase B
measures rather than assumes.

### 4.3 The Tier-S candidate question (small text model) — thinner evidence, so measure

The research's small-*text* shortlist is honestly thinner than its VLM one:
**LFM2.5-1.2B** (IFEval 86.23, MMLU-Pro 44.35; vendor-claimed 239 tok/s decode on an
*unspecified* "AMD CPU" — the one CPU number in class, but unverifiable as stated) and
**LFM2.5-350M/230M** carry the same §3.3 license question in server-side form (weaker there —
the hub operator, not an app-store artifact, does the "using" — but the same ruling should
cover it). **Phi-4-mini** (MIT, built-in function calling) is the license-clean candidate at
3.8B ≈ 2.2–2.5 GB Q4 — bigger than the ≤1 GB Tier-S target but viable on this RAM budget; no
CPU tok/s found (GAP). **Gemma 3 1B/270M** are text-only (fine here) under the Gemma license
(permits commercial use with a prohibited-use policy — verify against current terms at
adoption time). **LFM2.5-8B-A1B** (MoE, 1.5B active, <6 GB, strongest agentic story in the
lineup) is noted as a potential future *Tier-G replacement*, not Tier-S — same license gate.
Ruling: pick Tier-S **empirically** — pull the ≤2 candidates the O-1 ruling permits, run the
wiring blueprint's golden tool-calling suite plus the CS-4 fixture set against each, record
the scorecard, keep the winner (§8 O-2). No pick on paper.

### 4.4 Routing vs cascading — applied, not essayed

The research's distinction, applied to dowiz's actual lanes:

- **ROUTING (decide before generation — RouteLLM-class).** dowiz already has the degenerate
  form: `TaskClass` static routing in `ollama.rs::route_model`, becoming configurable via the
  wiring blueprint's Phase 3. P97 extends the *mapping*, not the machinery: CS-4/CS-5 route to
  Tier S; CS-1 routes to Tier G; CS-2 to Tier C. Mechanism: per-request `model_id` override
  (already passes through verbatim today) from the intake/comms call sites — **no new
  `TaskClass` variant, no kernel enum change** in v1; a `Classify` variant is only justified
  if classification call sites multiply (named trigger, not built). **Learned routing is
  explicitly rejected here**: RouteLLM trains its router on preference data dowiz does not
  have, and the HK-05/HK-09 arc already owns real-time model routing — the wiring blueprint's
  §7 non-duplication ruling stands verbatim. P97's routing is static-by-task, full stop.
- **CASCADING (escalate on insufficient quality — FrugalGPT/AutoMix-class).** Adopted on
  **exactly one lane: CS-4 intake assist**, because it is the only lane with a *deterministic*
  sufficiency oracle: the assisted output either re-parses through the pure `IntentParser`
  into a valid intent, or it is still `Ambiguous`. Cascade contract: Tier S attempt → if
  still `Ambiguous`, exactly one Tier-G attempt → if still `Ambiguous`, the existing
  human-handling path (which P48-INTAKE §8.7 already mandates as the backstop). Bounded, two
  rungs, no retry loop, no LLM-judge arbiter anywhere. **CS-1 (agent loop) deliberately does
  NOT cascade:** a wrong-but-well-formed agent answer has no mechanical detector, so any
  cascade trigger there would be judge-shaped — the exact technique the wiring blueprint's
  §3.2 evidence base rejects. The loop's existing bounded-iteration + tool-schema fail-closed
  design *is* its quality control.
- **Speculative decoding** — noted as distinct from routing (research), single-model-family
  draft-and-verify. Not designed here; Ollama-internal if it ever applies. Out of scope.
- Since P48-INTAKE is itself PLAN, the cascade lands **with** that build; P97's deliverable is
  this contract plus the fixture spec (§6 Phase C), so the intake build consumes a settled
  design instead of re-deriving one.

### 4.5 The measurement this whole section stands on (Phase B)

The research could not find a head-to-head "N small vs 1 big on shared cores" benchmark, and
this repo does not ship topology on third-party numbers anyway. Phase B is a scripted,
operator-run measurement matrix **on this exact box** (fixed prompts, temperature 0 where
honored, decode tok/s + prefill tok/s + P99 wall-clock per cell, ≥ 3 runs per cell,
`/api/generate` against the live daemon):

| Cell | What it answers |
|---|---|
| B-1: `llama3.1:8b` solo | the box's single-stream baseline |
| B-2: 2 × `llama3.1:8b` requests concurrent (2 dispatch workers) | does the locked 2-worker cap even make sense for two *same-model* streams |
| B-3: `llama3.1:8b` + Tier-S candidate concurrent | the actual production shape: strong + small decoding together |
| B-4: `llama3.1:8b` decode + `nomic-embed-text` embed burst | does the always-hot embed lane perturb the reasoning lane |
| B-5: 4-models-resident idle → cold-vs-warm first-token on each | residency plan's reload-thrash claim, quantified |

Results go in a committed scorecard (`docs/audits/` row or the blueprint's companion `.jsonl`,
same pattern as the wiring blueprint's regression scorecard) — **pass/fail probe, never
baseline-gated CI** (host-noisy LLM measurements stay out of the deterministic gate set, per
standing bench policy). Decision rule, falsifiable: if B-3 degrades the Tier-G stream's decode
by more than **25%** vs B-1, concurrent cross-tier decode is disallowed (dispatcher stays at 2
workers but the composition serializes G behind S completion for product lanes) and the
blueprint's residency-without-concurrency stance is recorded as *measured*; if under 25%, the
2-stream shape ships as designed. Either way the number replaces the assumption.

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

Ordering note: P97 consumes the wiring blueprint's Phase 1 (AiMode composition switch) and
Phase 3 (configurable model map) as its config substrate; nothing in P97 blocks them.

- **Phase A — mobile suite + license ruling.** RED: no vision scenario suite exists (trivially
  true — no VLM anything exists). GREEN: §3.5 suite exists with self-falsification proven
  (a scrambled expected-field must fail), run against the O-1-permitted candidates on ≥1 real
  device, scorecard committed, sub-250 ms claim marked reproduced/not. Gate: **O-1 ruling
  before any LFM weight enters any artifact.**
- **Phase B — the §4.5 measurement matrix.** RED by absence (no concurrency numbers exist for
  this box). GREEN: all 5 cells recorded, ≥3 runs each, decision rule applied and outcome
  written back into this document's §4.5.
- **Phase C — residency + routing enactment.** RED: `OLLAMA_MAX_LOADED_MODELS` unset (default
  3) and no Tier-S model resident. GREEN: env + model map configured per §4.2/§4.3 bake-off
  winner; one wiring-blueprint-golden-suite run recorded under the new map; the CS-4 cascade
  contract + fixture spec handed to the P48-INTAKE build (its RED→GREEN lands there, with a
  fixture `Ambiguous` message that must escalate exactly once and then stop).
- **Phase D — optional CPU-LoRA wall-clock probe (§5).** Operator-gated (O-3); acceptance
  criterion self-contained above.

## 7. Non-goals (explicit, closed list)

- No reopening of the wiring blueprint's §1 locks: `llm-adapters` stays a sync HTTP client;
  no in-process inference in the hub agent lane; 2-worker dispatch and single-backend-per-
  deployment stand. (Mobile in-process inference is a different binary and follows §3.4's
  port discipline — it never enters `llm-adapters` or the kernel.)
- No learned/real-time router — HK-05/HK-09 owns that arc; P97 is static task routing + one
  deterministic cascade, recorded as beneath that arc's threshold by design.
- No LLM-as-judge anywhere: not in the mobile suite, not as a cascade arbiter, not in CI.
- No vision output in settlement authority, ever: `DeliveryClaim` keeps no photo field; VLM
  assist is capture-UX only. (Any future evidence-attachment design belongs to P52 §3.4, and
  even there PoD settles on signatures, not pixels.)
- No new kernel types in v1 (no `TaskClass` variant; per-request `model_id` override suffices).
- No model weights in the git repo; no model-registry/auto-pull system (wiring §2.3 stance).
- No fine-tuning/training build of any kind (§5 — Phase D is a measurement, not infrastructure).
- No custom serving daemon / scheduler — Ollama env + existing config surface only.

## 8. Open decisions for the operator

1. **O-1 — LFM Open License adoption (the §3.3 ruling): adopt-with-D-entry / permissive-only /
   defer-VLM-entirely.** Blocks Phase A's LFM half and §4.3's LFM candidates; blocks nothing
   else. Recommendation: make the call now even if it is (iii) — the point is that it be a
   recorded decision, not a default that ships itself.
2. **O-2 — Tier-S bake-off pair.** Which ≤2 small text models enter the §4.3 bake-off
   (constrained by O-1). Recommendation: Phi-4-mini (license-clean) + one LFM small model if
   O-1 permits; otherwise Phi-4-mini + best-available permissive small model at build time.
3. **O-3 — run Phase D at all.** Costs ~one evening of machine time, closes research gaps
   #7/#8 permanently. Recommendation: yes, precisely because it is cheap and terminal — but it
   changes no near-term plan either way, so "no" is a legitimate answer.

---

*End of blueprint. Everything above is design + measurement plans; nothing is built. The load-
bearing sequence is O-1 → Phase B (pure local measurement, startable immediately) → Phase C;
Phase A rides the P52/P71 timeline; Phase D is optional and terminal.*
