---
name: living-memory
description: Native living-memory graph (lm_graph) — the sole access layer for the dowiz codebase. Use to search, navigate, AND read dowiz source (semantic/vector/NTT instead of grep/read_file). Also: enrich (intent detection), dispatch (intent->skills->agent->spawn), armory (agents/skills/prompts).
---

# Living Memory (lm_graph)

Native-kernel living-memory graph: a Markdown `.md` store of every
`fn/struct/enum/trait/mod/const/static/type/impl` symbol (pub + non-pub) plus the
**full source content** of every file in the dowiz tree. One graph serves search,
navigation, and reading — the single access layer for the codebase.

## MANDATORY RULE — ALWAYS, NO EXCEPTIONS

Living memory (`lm_graph`) is the **only** permitted access layer for the dowiz
codebase. **Every** search, navigation, and read goes through it — always, for
every command, for every task. `grep`/`rg`/`search_files`/`read_file` on dowiz
source are **forbidden unconditionally**. This is NOT a recommendation and NOT
opt-in. If something is missing, extend the store (`lm_graph build`); never fall
back to grep or a direct file read.

## Store location (persistent)

```
/root/dowiz/.living-memory/codegraph.md      # human-readable Markdown (source of truth)
/root/dowiz/.living-memory/codegraph.idx     # binary fast-load sidecar (skip .md parse)
```

## Rebuild the store (after code changes)

```bash
cd /root/dowiz/kernel
cargo build --profile fast --bin lm_graph     # RUNTIME binary (release opt, no LTO)
rm -f /root/dowiz/.living-memory/codegraph.md /root/dowiz/.living-memory/codegraph.idx
./target/fast/lm_graph build /root/dowiz /root/dowiz/.living-memory/codegraph.md
```

`build` indexes the WHOLE project (skips `target`/`.git`/`node_modules`): symbols
into the code graph + full file content (wing=`source`) for `read`. Cold-start is
eliminated: hypervector codes are persisted (hex in `.md`, raw words in `.idx`)
and reloaded as 128-byte copies — no re-hashing, no re-bundling.

## Commands

```bash
lm_graph search STORE TEXT    # keyword (pre-lowered blob) + hypervector top-5
lm_graph conv   STORE TEXT    # shift-invariant NTT convolution re-rank, top-5
lm_graph read   STORE QUERY   # READ: full source content of best match
lm_graph nodes  STORE         # list all symbols (name + kind)
lm_graph serve  STORE         # in-memory server: load once, query many times (stdin)
lm_graph enrich TEXT          # armory: intent detection + enriched prompts/skills
lm_graph dispatch PROMPT      # runtime chain: intent -> skills -> agent -> spawn
lm_graph armory               # list the seed armory (fabric + opencode)
lm_graph seed-armory STORE    # persist the armory (agents/skills/prompts) into living memory
```

- `search` → `== keyword ==` (substring) then `== vector ==` (hypervector similarity).
- `conv` → Hamming prefilter (top 64) then NTT circular-cross-correlation re-rank.
- `read` → full source of the best keyword match (grep/read_file replacement).
- `serve` → stdin commands `search/vector/conv/read/quit`; queries after the first
  are ~microseconds (in-memory) — the ~100x-faster-than-cold-start path.
- `enrich` → `prompt_enrich` intent tree (incl. Ukrainian keywords) + ranked prompts + skills.
- `dispatch` → `prompt_enrich` intent → skill pick → `agent_orchestrator::TaskOracle`
  (Light/Heavy) → `dynamic_spawner` batch. Agents DO pick skills; specialized agents ARE selected.
- `seed-armory` → ingest agents/skills/enriched-prompts as Procedural records
  (wing=`armory`, room=skills/plugins/tools/dynamic-skills/prompts), persistent.

## Performance

~65ms cold-start (read 24MB `.idx` + materialize ~16.5k records + search + print)
on the whole-project store, vs grep ~200-800ms on the same files. In-memory
(`serve`) queries are sub-millisecond. Full search after load is O(n) over records.

## Pitfalls

- `build` APPENDS — delete `.md`/`.idx` first for a clean rebuild, or symbols accumulate.
- The `.idx` sidecar is only fresh after `persist()` (final write of `build`).
  Incremental `remember` appends to `.md` without touching `.idx` → sidecar goes
  stale → store falls back to the (correct, slower) `.md` hex-code load.
- Store files are generated artifacts under `.living-memory/` (keep gitignored).
