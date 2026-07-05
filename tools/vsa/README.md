# tools/vsa — VSA Transmitter + local hypervector math (token economy)

Operator directive 2026-07-05: integrate VSA across the project, top priority, to reduce
token consumption; track usage before/after. Built and measured on Fable 5.

## What actually saves tokens (honest mechanics)

Raw hypervectors are **never sent to an LLM** — an 8192-dim vector serialized as text costs
*thousands* of tokens and the model has no learned semantics for it. The measured savings
come from three real mechanisms:

1. **Transmitter codec** (`src/codec.mjs`) — deterministic, **lossless** JSON→frame
   projection: columnar tables (keys paid once, not ×N rows), a savings-modeled string
   dictionary (UUIDs/keys/repeated values → `§n` refs), minification. `decode(encode(x))`
   deep-equals `x` — enforced by tests incl. the null-vs-absent class that broke wire
   parity elsewhere in this repo.
2. **h_t hidden state** — pass a compact structured state frame instead of chat history /
   log re-reads (`docs/ops/rebuild-cutover-h_t.vsa1` is the live example: the whole parked
   rebuild-cutover state in 844 tokens).
3. **True VSA math, locally** (`src/hv.mjs`) — bipolar D=8192 bind/bundle/cosine for
   matching/recall/dedup at **zero** token cost (the savings are whole LLM calls not made).
   `hvFor("SEARCH_LOGISTICS")` is a pure function of the string — same vector on every
   machine forever, no registry (fnv64 → splitmix64).

## Measured (2026-07-05, js-tiktoken cl100k_base, vs already-minified JSON)

| payload | min tok | frame tok | save |
|---|---|---|---|
| owner products list (93KB) | 33,928 | 21,068 | **37.9%** |
| public menu (55KB) | 20,063 | 14,347 | **28.5%** |
| cutover flags state | 370 | 213 | **42.4%** |
| location-info (small, low repetition) | 419 | 384 | 8.4% |
| **aggregate** | **54,780** | **36,012** | **34.3%** |

All lossless (byte-exact round-trip). Full table: `BENCH.md` (regenerate: `node cli.mjs bench`).
Prose-heavy states compress less (~13% on the h_t example); structural/tabular payloads
compress most. The spec's "40–60%" holds only for the most repetitive payloads — treat
34% aggregate as the honest project number against a minified baseline.

**Consumability proven**: a Fable 5 subagent answered 5/5 factual questions directly from
a 14k-token frame (prices, category counts, descriptions) given only the 90-token
`FRAME_SPEC` — no decoder, no script.

## Live A/B (real harness tokens, 2026-07-05)

Two identical Explore agents (Fable 5), same file content, same 5 analytical questions
(count/lookup/filter/distinct/max), answers **identical and correct** in both arms:

| arm | subagent tokens | tool uses |
|---|---|---|
| raw JSON (93KB) | 72,147 | 4 (paged reads) |
| VSA1 frame (43KB) | 46,725 | 2 |
| **saved** | **25,422 = 35.2% of the WHOLE dispatch** | half the round-trips |

This is the truest number: real Anthropic-tokenizer usage of a real dispatch, and the
frame also halves file-paging tool calls. `dispatch.mjs` composes such prompts in one
command (`--task "..." files…`); `cli.mjs report` shows the cumulative ledger.

## Usage

```
node tools/vsa/cli.mjs encode <file.json|->     # JSON → frame
node tools/vsa/cli.mjs decode <frame|->         # frame → exact JSON
node tools/vsa/cli.mjs tokens <file|->          # BPE count
node tools/vsa/cli.mjs bench                    # regenerate BENCH.md from bench/payloads/
node tools/vsa/cli.mjs match "<query>" corpus.jsonl   # zero-token similarity recall
node tools/vsa/cli.mjs pe pred.txt actual.txt   # prediction-error (1−cos), exit 1 if > VSA_PE_THRESHOLD
node tools/vsa/cli.mjs spec                     # the one-time frame decode spec for prompts
```

Every `encode`/`bench` appends `{ts,bytesIn,bytesOut,tokIn,tokOut}` to
`telemetry/usage.jsonl` (gitignored) — the running before/after ledger.

## Project convention (see AGENTS.md)

- Any JSON payload > ~1KB destined for an agent prompt goes through `encode`; the receiving
  agent gets `FRAME_SPEC` once per conversation (or relies on the AGENTS.md convention).
- Lane/session state is passed as an h_t frame (`*.vsa1` next to its `*.json` source),
  never as raw transcript/log dumps.
- Matching tasks→lessons/loops/memories uses `match` (local, zero tokens) before any
  LLM-based recall.
- `pe` is the Phase-I prediction-error signal: predicted-vs-observed state distance;
  > threshold ⇒ escalate per the doubt-escalation ladder — it never auto-mutates anything
  (advisory signal; deterministic gates stay the authority, per the repo's standing rule).

## Relation to the Active-Inference plan (Phases I–III)

- **Phase II (this)**: Transmitter + h_t + local HV math — built, measured, adopted.
- **Phase I (signal)**: `pe` + the usage ledger provide the per-node error/usage stream;
  the existing loop-harness telemetry already tags Success/Failure per loop node.
- **Phase III (evolution)**: the repo ALREADY has the machinery the doc asks about —
  the meta-loop gene-ledger (CERTIFIED, decorrelated-lens oracle) mutates harness
  parameters and re-runs tests red→green, and the L5 meta-controller gates
  self-modification behind an immutable core. Differential evolution of prompts/tools
  should extend THOSE, not add a parallel system. Auto-apply without a passing
  deterministic gate stays forbidden (Ethics Charter + advisory-vs-authority rule).

## Limits (measured, not hidden)

- `match` is bag-of-words+bigram HV — strong on keyword-bearing queries (top-1 on
  deploy-recipe recall), weak on semantic paraphrase (0.04 scores). It's a pre-filter,
  not an oracle.
- Frames are for DATA payloads. Instructions/prose to agents stay plain text — compressing
  instructions degrades compliance for pennies.
- cl100k_base is not Anthropic's tokenizer; before/after use the same ruler so ratios hold.

## VSA-VIZ — visual token arbitrage (2026-07-05)
A third token layer: render a large system STATE (dispatch / VSA vectors) as a compact **semantic
image** a vision model reads at a ~fixed image-token cost (≈1,399 for 1024²) instead of linear JSON
burn. `src/raster.mjs` (pure-stdlib RGBA+zlib-PNG, zero deps) → `src/viz.mjs` (shape/color/size
dictionary + `visionMessage()`) → `viz-cli.mjs`. Verified: a Claude vision model reads the state
accurately; **crossover ≈ 25-30 entities**, then up to −91.5% at scale. Below the crossover, send
JSON. Full bench + honest limits: [BENCH-VIZ.md](./BENCH-VIZ.md). Usage: `node tools/vsa/viz-cli.mjs demo`.
