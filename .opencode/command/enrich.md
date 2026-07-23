---
description: Detect intent and enrich prompt with best matching templates
agent: build
mode: subagent
---

# ENRICH — Universal prompt enrichment

The `enrich` binary reads any text from stdin or `--query` and enriches it with matched prompts, skills, and patterns from the 10,871-entry database.

## Self-enrichment protocol

For YOUR own prompts (this assistant): before executing any cognitive work:
1. Pipe the user's request through `enrich --verbose`
2. Read the output: primary intent, tree paths, matched prompts, inherited patterns
3. Apply the matched patterns to your reasoning
4. Display the enrichment block

## Usage

```sh
# Quick enrichment
echo "query" | kernel/target/debug/enrich --load kernel/prompt_enrich_db.jsonl

# Full report
echo "query" | enrich --load kernel/prompt_enrich_db.jsonl --verbose

# JSON for programmatic use
echo "query" | enrich --load kernel/prompt_enrich_db.jsonl --format json

# Intent detection only
enrich --load kernel/prompt_enrich_db.jsonl --detect "query"

# Pattern inheritance
enrich --load kernel/prompt_enrich_db.jsonl --patterns "query"

# Dashboard
enrich --load kernel/prompt_enrich_db.jsonl --dashboard
```

## Integration with any agent

The binary is **agent-agnostic** — reads stdin, outputs text/JSON. Works with:
- opencode: `/enrich "query"` command
- claude-code: `echo "query" | kernel/target/debug/enrich --load ...`
- codex: `cat prompt.txt | enrich -f json`
- shell scripts: `result=$(echo "$task" | enrich -f json)`
- CI pipelines: `echo "$COMMIT_MSG" | enrich --detect`

## Current state

- DB: 10,871 canonical entries (16 kinds, 17,533 keywords)
- Engine: 8D crystal lattice, tree-based intent (65 nodes), pattern taxonomy (33 nodes)
- Tests: 1971 green
