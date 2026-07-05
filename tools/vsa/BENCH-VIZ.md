# VSA-VIZ — visual token arbitrage (bench + verification, 2026-07-05)

Render a system STATE (logistics dispatch / VSA vectors) into a compact high-contrast **semantic
image** a vision model reads at a ~fixed image-token cost, instead of linear text-token burn on
dense JSON. Built pure-stdlib (zero deps): `src/raster.mjs` (RGBA canvas + zlib PNG encoder + 5×7
font) → `src/viz.mjs` (the shape/color/size dictionary + de-collision + `visionMessage()`) →
`viz-cli.mjs`. Render ≈ a few ms for 1024².

## The dictionary (why semantic, not pixels)
Vision models read clean geometry reliably and **hallucinate on pixel noise** — so we never pack
raw vectors as high-density pixels. Instead: SQUARES=couriers (size=load, color=free/busy/offline),
CIRCLES=orders (size=value, color=ok/soon/late), LINES=assignments (thickness=weight), position=
geography, a bottom HEATMAP strip=VSA vector intensities. The text `LEGEND` (the decoder ring) is
passed once in `system` (prompt-cacheable → paid once).

## Verification 1 — ACCURACY (a vision model actually reads it)
The demo state (4 couriers / 5 orders / 3 links / 5 VSA cells) was rendered and read back by a
Claude vision model (the production target). It correctly recovered **every** entity + attribute:
courier loads (C1:90% busy=amber biggest, C2:20% free=green, C3:0% offline=gray, C4:55% busy),
order urgency-colors + value-sizes (O14/O10 red-late, O12 amber-soon, O11/O13 green-ok), the
assignment lines, and all 5 heatmap cells (DEMAND 82…ETAVAR 60). One defect found + FIXED: two
entities at near-identical geo coords overlapped and clipped a label — added an iterative
**de-collision** relaxation pass; re-read = clean. → semantic encoding is vision-legible.

## Verification 2 — ARBITRAGE (real crossover, honest)
Image cost ≈ `(w·h)/750` = **1,399 tokens** fixed for 1024². JSON grows linearly:

| orders | json tokens | image tokens | winner |
|--:|--:|--:|:--|
| 3 | 170 | 1,399 | json |
| 15 | 827 | 1,399 | json |
| 20 | 1,123 | 1,399 | json |
| **30** | **1,664** | **1,399** | **image −16%** |
| 60 | 3,305 | 1,399 | image −58% |
| 110 | 6,062 | 1,399 | image −77% |
| 200 | 11,006 | 1,399 | image −87% |

Plus a scale state with 5 raw D=512 VSA vectors: **16,386 → 1,399 = 91.5%**. **Crossover ≈ 25-30
entities (~1,400 JSON tokens).** Below it, send JSON; above it, send the image. The win widens with
scale because the image cost is entity-count-independent. (Shrink the canvas — `width`/`height` —
to move the crossover earlier at the cost of legibility for very dense states; 1024² is the balance.)

## Use
```js
import { visionMessage } from './src/viz.mjs';
const { system, messages } = visionMessage(state, { style: 'anthropic' }); // or 'openai' for OpenRouter
// system = the LEGEND (cache it); messages[0] carries the base64 PNG. Send when state is large.
```
CLI: `node tools/vsa/viz-cli.mjs demo` (or `render <state.json>`) → PNG + token report.

## Honest limits
- A **scale play**, not a universal win — below the crossover, JSON is cheaper (the CLI prints which wins).
- Encodes what the dictionary models (couriers/orders/links/weights); arbitrary nested data still needs VSA frames or JSON.
- Legibility degrades if hundreds of nodes crowd one geo cluster; de-collision helps but a denser scene wants a bigger canvas (higher fixed cost) or filtering to the decision-relevant subset.
- The image is a DECISION-SUPPORT view (read at a glance), not a lossless store — keep the authoritative JSON server-side; send the image, act on the returned decision, verify against source.

---

# VSA-VIZ FRACTAL — hierarchical 3-level encoding (2026-07-05 extension)

Speaks Vision-Transformer's native language: attention resolves patches→local→global, so a NESTED
container image maps onto how the model already sees. Three levels, drawn at relative offsets (the
rasterizer analogue of nested SVG `<g transform>`, ~0 compute): **L1 macro** = zones/hubs (thick
outlined boxes; RED outline = alert/driver-deficit), **L2 meso** = vehicles inside a zone (SHAPE =
type: triangle=car, diamond=van), **L3 micro** = a 3×3 order-matrix inside each vehicle (filled cells
= load, color = deadline green/amber/red; empty = free capacity). `src/viz-fractal.mjs` +
`visionMessageFractal()`; CLI `node tools/vsa/viz-cli.mjs fractal`.

## Verification — ACCURACY + DECISION (the real test)
Rendered the operator's scenario (HUB-SE in alert with an all-red overloaded van V2; a spare EMPTY car
T2 in the calm HUB-SW) and read it back as a Claude vision model. It recovered ALL THREE levels
correctly — zones + the SE alert, every vehicle's type-by-shape, and every 3×3 matrix (T1 light-green,
V1 mixed, **T2 all-empty spare**, **V2 all-red critical**) — and reached the intended command:
**"dispatch the empty car T2 from HUB-SW to the alert zone HUB-SE to relieve V2's all-red overload."**
A 9-zone / 9-vehicle render was also read cleanly (H7 alert, H5 empty spare, all matrices distinct).

## Verification — ARBITRAGE + the resolution ceiling (honest)
Each vehicle carries 9 order-cells, so the fractal packs ~9× the entities of the flat viz per image.
JSON vs image (fixed 1,399 @1024²):

| zones×veh | orders | json tok | image | winner |
|--:|--:|--:|--:|:--|
| 9×2 | 72 | 477 | 1,399 | json |
| 16×3 | 233 | 1,245 | 1,399 | json (edge) |
| 25×4 | 439 | 2,394 | 1,399 | **image −42%** |
| 36×4 | 681 | 3,539 | 1,399 | **image −60%** |

**Crossover ≈ 230-400 orders.** BUT: Claude downscales images to ~1568px, and legible micro-cells
need ≥~20px — so the practical legible ceiling is ~9-16 vehicles (~80-144 order-cells) at max
effective resolution. That ceiling sits a little BELOW the pure-token crossover, so the fractal's
first-order win is **comprehension / decision-support** (read the whole zone→vehicle→order hierarchy
AND get the reallocation answer at a glance — verified) more than raw token savings; it turns
token-positive at dense scale (~150-230+ orders) right around that ceiling. Use the **flat** viz
(crossover ~25-30) for simple scatter; use the **fractal** when the HIERARCHY (zone→vehicle→order) is
the decision — its structure is what a ViT reads best.

---

# VSA-VIZ MACRO — the token floor (256² + minified legend + delta frames, 2026-07-05)

Three levers drive the per-request cost from the 1024²-fractal's 1,399 tok down toward ~88 (operator
directive). `src/viz-macro.mjs`: `renderMacro` (low-res zone-heat + per-driver status grid),
`LEGEND_MIN` (telegraphic), `diffDrivers`/`renderDelta` (patch strip), `visionMessageMacro`.

## Measured (image tok ≈ w·h/750)
| artifact | tokens | note |
|---|--:|---|
| macro 256² | **88** | verified decision-legible (HUB3 alert reads) |
| macro 384² | 197 | per-driver differentiation clear |
| macro 512² | 350 | |
| minified legend | 113 | vs 235 prose (cached once) |
| delta frame (4/50 changed) | 11 | only the movers |

## Verified legibility (read back as a vision model)
At **256² / 88 tok** the dispatch DECISION survives: the red-bordered alert zone (HUB3) with its
all-red drivers is unmistakable, other zones differentiated (green/amber/red per driver, hollow=spare).
What does NOT survive: the per-order 3×3 matrix — that's the deliberate drop. Fixed a semantics bug
found in testing: driver color must be SHARE-of-late (not worst-single-order, which saturates every
driver to red the moment one order slips).

## The honest ceiling
Claude tokenizes 256² at ~88 perceptual tokens — that, not the 65,536 pixels, is the information
ceiling. So 256² carries a MACRO decision (~5-25 readable elements: which zone is hot + where the
spares are), NOT 450 legible micro-states. Use it TIERED: delta/macro heartbeat for the routine tick →
1024² fractal to drill a hot zone → JSON to audit orders. ROI (86,400 calls/mo, Sonnet rates): naive
rich JSON $12,283/mo → macro 256² **$78/mo (−99.4%)** → delta steady-state **$58/mo (−99.5%)**, a ~160×
cut. It's paying macro prices for macro questions, not one image replacing the source of truth.

---

# BLIND ORCHESTRATION — inversion of control (the physical floor, 2026-07-05)

The cheapest token is the one never sent. `tools/vsa/orchestrate.mjs` inverts control: deterministic
code (spatial cull + ETA + VSA cosine via `hv.mjs`) does ALL the math at $0 and auto-assigns every
clear match; the LLM wakes ONLY for a genuine judgment call (soft-constraint tradeoff / scarcity) as
a ~50-token micro-prompt with a cached ~65-token judge. `node tools/vsa/orchestrate.mjs sim`.

## Measured (Fable-5-scale sim: 50 drivers, 5 zones)
| scenario | orders | auto-resolved ($0) | escalated | LLM tokens | vs full JSON |
|---|--:|--:|--:|--:|--:|
| cold start | 120 | 82 (68%) | 38 tradeoffs | 1,654 (1 batched call) + 65 cached | −82% |
| steady state (4/tick) | 4/tick | ~all | 0 | 0 (0/100 ticks called LLM) | −100% |

The escalations are ALL real tradeoffs (overtime, VIP-vs-low-rating, scarcity) — exactly what a model
should decide and code should not. In steady state with slack, the model is never called → ~$0/mo.

## Honest limits
Calibration-dependent: a busier system (demand > slots) raises genuine scarcity → more LLM calls; the
risk thresholds + cull radius need tuning against real data, and the deterministic resolver must be
CORRECT (a wrong silent auto-assign is worse than an LLM call — when truly unsure, escalate). It's the
architecture that's proven here (deterministic-first, escalate-the-residue), not a production-tuned
dispatcher. This is the capstone ABOVE the compression tiers: don't-send-state (this) → compress-state
(frame/viz/macro). AGENTS.md rule −1.
