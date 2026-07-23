---
name: skill-armory
description: Scrapes + reverse-engineers + ingests free skill/prompt/plugin libraries. Use when: scrape, ingest, enrich, armory, harvest, collect prompts, reverse engineer skills. Use ONLY when explicitly asked to scrape or enrich the database.
---

# Skill Armory

The armory scrapes GitHub repositories, web APIs, and open prompt libraries,
reverse-engineers their skill/prompt/tool/plugin definitions, and produces
a JSONL feed consumable by `prompt_enrich::PromptEnrichEngine`.

## Sources (CC0/MIT-licensed)

| Source | License | Count | Format |
|--------|---------|-------|--------|
| fabric (danielmiessler/fabric) | MIT | 100+ | `data/patterns/*/system.md` |
| prompts.chat (f/awesome-chatgpt-prompts) | CC0 | 1000+ | CSV (`act,prompt`) |
| opencode built-in | MIT | 10+ | Markdown frontmatter |
| Any GitHub raw .md file | varies | N | Markdown |

## Commands

```sh
# Seed core prompts
skill_ingest --seed-core --seed-system --output prompt_enrich_db.jsonl

# Ingest from CSV
skill_ingest --file prompts.csv --output prompt_enrich_db.jsonl

# Scrape a URL
skill_ingest --url https://raw.githubusercontent.com/owner/repo/main/skill.md

# Pass through pre-formatted JSONL
skill_ingest --jsonl existing_db.jsonl --output merged.jsonl
```

## Reverse-engineering protocol

For any third-party skill/tool/plugin:
1. **Extract** the core abstraction (what problem does it solve?)
2. **Map** its interface to native primitives (no external deps)
3. **Reproduce** using only kernel std primitives
4. **Store** as PromptEntry with kind, triggers, source, license

## Database growth strategy

The engine supports recursive extraction (same pattern as `research.rs`):
- Batch N (100 prompts) → extract patterns → use patterns as search queries
- Batch N+1 (500 prompts) → repeat → 2,500 → 12,500 → 62,500 → 100,000
- ~6 iterations to reach 100k from a 100-prompt seed

## Cross-pattern detection

Co-occurring prompts across repos form cross-patterns with lift scores.
Example: `security_auditor × create_report_finding` (lift: 3.2)
Meaning: repos that have one almost always have the other → core security bundle.
