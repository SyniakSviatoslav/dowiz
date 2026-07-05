# OpenRouter Orchestrator

Three-tier model dispatch: **Claude** researches and reviews; **free OpenRouter models** implement.

## Why

Claude's reasoning is expensive. Mechanical code-writing — "add this function", "apply this transformation", "translate this spec to code" — doesn't need frontier reasoning. By routing implementation tasks to free OpenRouter models, you spend Claude tokens only where they matter: understanding the problem, generating precise specs, and reviewing results.

## Model Tiers

| Task | Model | Cost |
|------|-------|------|
| Architecture, research, spec generation | Claude Sonnet (this session) | Paid |
| Spec compliance review | Claude subagent | Paid |
| Code quality review | Claude subagent | Paid |
| **Mechanical implementation** | `nvidia/nemotron-3-super-120b-a12b:free` | **Free** |
| Multi-file reasoning / complex integration | `qwen/qwen3-coder:free` | Free |

Switch models via `OPENROUTER_MODEL` env var.

## Setup

Add to your `.env` (never commit):
```
OPENROUTER_API_KEY=sk-or-v1-...
```

Get a key at https://openrouter.ai/keys (free tier available, no credit card required).

## The Workflow

This extends `subagent-driven-development`. In that skill, the **implementer** role uses `Agent` (Claude). Here, replace the implementer dispatch with the OpenRouter script:

```
[Claude orchestrator]
  ↓ reads files, generates spec
[scripts/openrouter-implement.ts via Bash]
  ↓ free model writes code
[Claude orchestrator]
  ↓ applies output with Edit/Write
  ↓ spec-compliance review (Claude subagent)
  ↓ code-quality review   (Claude subagent)
```

## How to Write a Good Spec

The free model has no filesystem access. Embed everything it needs:

```
You are implementing a task in a TypeScript/Node.js codebase.

## Context
[One paragraph: what the file does, invariants to preserve]

## Current file: path/to/file.ts
```typescript
[FULL CURRENT CONTENT — paste verbatim]
```

## Task
[Precise, mechanical description. No architecture decisions — those are already made.
 Just: "add this function", "change this field", "rename this handler".]

## Requirements
- [Bullet list of acceptance criteria]
- [Exact function signatures if relevant]
- [Type constraints]

## Output format
Output ONLY the complete modified file. No markdown fences, no explanations.
```

## Calling the Script

```bash
# Pipe spec to script, capture output
cat << 'SPEC_EOF' | npx tsx scripts/openrouter-implement.ts
[spec here]
SPEC_EOF

# Or via PowerShell
$spec = Get-Content scripts/my-spec.txt -Raw
$result = $spec | npx tsx scripts/openrouter-implement.ts
# then apply $result via Write tool
```

## Multi-File Tasks

Break into per-file calls. For each file:
1. Embed current file content in the spec
2. Call script → get modified file
3. Apply with Write/Edit
4. Move to next file

Never ask the model to produce multiple files in one call — parsing becomes fragile.

## When NOT to Use Free Models

- Task requires reading multiple files to form a plan (let Claude do the whole thing)
- Change touches > 3 tightly coupled files (risk of inconsistency)  
- Implementation requires understanding an ADR or architectural decision
- Debugging (hypothesis formation is reasoning, not writing)

## Environment Variables

| Var | Default | Description |
|-----|---------|-------------|
| `OPENROUTER_API_KEY` | *(required)* | Your OpenRouter key |
| `OPENROUTER_MODEL` | `nvidia/nemotron-3-super-120b-a12b:free` | Model to use |
| `OPENROUTER_MAX_TOKENS` | `8192` | Max output tokens |

## Switching Models

```bash
# Complex multi-file reasoning
OPENROUTER_MODEL=deepseek/deepseek-r1:free npx tsx scripts/openrouter-implement.ts

# Simple/fast edits
OPENROUTER_MODEL=meta-llama/llama-3.1-8b-instruct:free npx tsx scripts/openrouter-implement.ts
```
