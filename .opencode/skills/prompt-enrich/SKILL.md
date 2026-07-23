---
name: prompt-enrich
description: BATCH prompt/skill enrichment. Detects ALL intents from input, applies MULTIPLE prompts+skills simultaneously. SHOWS enrichment display. Use BEFORE ANY non-trivial cognitive work. Triggered by: any input with technical/cognitive content.
---

# Prompt Enrichment — BATCH Protocol

## MANDATORY: Before ANY cognitive work, run enrichment

**STEP 1: Detect all intents**
Execute `detect_all_intents(input)` — returns all intents with scores.

**STEP 2: Build enrichment batch**
Query the lattice for the best matching prompts AND skills across ALL detected intents. Apply them as a BATCH — not one at a time.

**STEP 3: Display enrichment**
Show what was detected and applied. Example output:

```
─── ENRICHMENT ───
  primary: code
  intents: code(5|0.42) test(3|0.25) refactor(2|0.17) analyze(2|0.17)
  prompts (3): [build_agent] [test_writer] [explain_code]
  skills (2): [prompt-enrich] [skill-armory]
───
```

**STEP 4: Apply silently**
Use the enrichment in your reasoning. Do NOT explain it unless asked.

## Batch skill execution

When enrichment returns skills, LOAD ALL OF THEM AS A BATCH. Not one at a time — all simultaneously. Example: if enrichment detects `[prompt-enrich, skill-armory, detect_patterns, detect_crosspatterns]`, load all 4 skills before beginning work.

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
