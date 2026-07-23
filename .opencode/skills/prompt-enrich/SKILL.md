---
name: prompt-enrich
description: Dynamic prompt enrichment — detects intent and injects best matching prompt templates. Use BEFORE executing ANY task. Triggered by: code, write, analyze, summarize, plan, review, security, test, debug, refactor, enrich, improve prompt, reverse engineer, scrape, skill, tool, plugin. Use ONLY when performing non-trivial cognitive work.
---

# Prompt Enrichment Skill

This skill wraps the `prompt_enrich` kernel module and `skill_ingest` armory.

## Enrichment protocol

Before processing ANY user request, silently:
1. **Detect intent** from the user's text (keyword + domain classification)
2. **Query the lattice** — find the top-3 matching prompt templates by kind + keyword overlap
3. **Inject** the best template's reasoning pattern into your execution
4. **NEVER reveal** enrichment to the user — it is invisible augmentation

## Intent → Template Mapping

| Intent | Best Template | Core Pattern |
|--------|--------------|--------------|
| code/implement/build | build_agent + explain_code | Full dev mode, explain code |
| analyze/evaluate/audit | analyze_claims + security_auditor | Truth-claim evaluation |
| summarize/tldr | summarize | Markdown summary with MAIN POINTS |
| extract/parse/scrape | extract_wisdom + reverse_engineer | Ideas, insights, quotes extraction |
| plan/design/blueprint | plan_agent + agility_story | Dependency analysis, spec-first |
| review/audit/check | code_reviewer | Security, perf, maintainability |
| security/harden | security_auditor + create_report_finding | Threat model, vuln report |
| test/spec/assert | test_writer | RED→GREEN, edge cases |
| debug/fix | debug_agent | Root cause, not symptom |
| refactor/clean/dedup | refactor_cleanup + detect_patterns | Extract helpers, eliminate dup |
| meta/improve prompt | improve_prompt + self_enrich | 6-strategy OpenAI PE |
| scrape/ingest | scrape_github + skill_armory | Harvest + reverse-engineer |

## Available seeds

```
fabric patterns:  analyze_claims, summarize, extract_wisdom, explain_code,
                  improve_prompt, rate_content, label_and_rate, write_essay,
                  create_report_finding, agility_story, clean_text, capture_thinkers_work

opencode agents:  code_reviewer, security_auditor, docs_writer, plan_agent,
                  build_agent, explore_agent, test_writer, debug_agent, refactor_cleanup

system prompts:   self_enrich, reverse_engineer, scrape_github, enrich_all_prompts,
                  skill_armory, detect_idempotency, detect_invariants,
                  detect_patterns, detect_crosspatterns
```

## Armory usage

```sh
# Seed built-in prompts
skill_ingest --seed-core --seed-system

# Ingest from file
skill_ingest --file prompts.tsv --output prompt_enrich_db.jsonl

# Scrape a URL
skill_ingest --url https://raw.githubusercontent.com/.../system.md
```

## Kernel API

```rust
use dowiz_kernel::prompt_enrich::{PromptEnrichEngine, PromptKind};

let mut engine = PromptEnrichEngine::new();
engine.ingest(seed_fabric_prompts());
let result = engine.enrich("summarize this document");
// result.matches has top-5 matching templates
```
