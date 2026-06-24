# Agent-Skill Security Scanning (SkillSpector) — governance rule

> **Rule (sibling of `require-classification`): no third-party agent skill or MCP server enters
> `.claude/` / `.agents/skills/` until it passes a SkillSpector scan, judged by CATEGORY + explanation
> (not the bare score). DO-NOT-INSTALL is respected. This is a tripwire, not a sandbox — SAFE ≠ a
> guarantee; anything that can see secrets (Supabase / payment keys) still gets human review.**

## Why
The agent holds production secrets. A malicious skill = credential exfiltration (the canonical
SkillSpector example: collect `os.environ` → POST to an attacker). ~26% of public skills are
vulnerable, ~5% malicious. We install skills from arbitrary repos → this is a real, previously-open hole.

## The tool
NVIDIA SkillSpector — static (regex/AST/YARA) + optional LLM; 64 patterns / 16 categories
(prompt injection, credential exfiltration, MCP tool poisoning, excessive agency, supply-chain) +
OSV.dev CVEs → risk 0–100 → SAFE / CAUTION / DO-NOT-INSTALL. Installed (gitignored) at
`tools/skillspector/`. **Always run `--no-llm`** (pure static, ZERO content egress — a secret-scanning
tool must not ship skill contents to a cloud LLM; no local Ollama here). Exit code 1 when risk > 50.

```bash
cd tools/skillspector && source .venv/bin/activate
skillspector scan <dir-or-git-url> --no-llm                       # decide
skillspector scan <dir> --no-llm --format sarif -o report.sarif   # CI/IDE
```

## How to JUDGE a result (not by the number)
Risk score is a heuristic. Decide on the **categories that fired + their explanation/location**:
- A research/browser/scaffold skill scoring CRITICAL because it does **network fetch / code-exec /
  reads env BY DESIGN** is high-risk-by-CAPABILITY, not by intent — acceptable for a trusted source.
- The real red flag is capability with **no legitimate reason**: env/secret read → external POST,
  obfuscated payloads, supply-chain tampering, MCP tool poisoning. Those are DO-NOT-INSTALL.
- Markdown-only skills (instructions, no scripts) flagged "Memory Poisoning"/"Agent Snooping" on a
  `SKILL.md`/README are almost always false positives (a skill *is* instructions).

## Retro-scan (2026-06-24, all 65 installed skills, `--no-llm`)
Bands: 58 LOW · 3 CRITICAL · 1 HIGH · 3 MEDIUM. The non-LOW ones are high-risk-by-capability, judged
acceptable (trusted ecosystem skills; capabilities match function — no env→external-POST signature):
- `impeccable` (100) — its UI detector engine does fetches + references prompts (SSRF/prompt-extraction FPs).
- `last30days` (100) — research skill; fetches social/web data → many "Data Exfiltration" by design. **Watch:** it does egress; keep an eye on what it sends.
- `skill-creator` (100), `webapp-testing` (71) — code-exec / browser automation by design.
- MEDIUM: `systematic-debugging`, `supabase`. MCP servers (repowise/browser-use/playwright in `.mcp.json`)
  are external code — scanning them requires their source repos (follow-up).
- `emilkowalski/skills` (23, MEDIUM/CAUTION) — judged SAFE: 6 findings are all benign (README/LICENSE
  prose + design-instruction `SKILL.md` as "Memory Poisoning"); zero exec/exfil/credential/egress. Installed.

## CI gate (FF-style)
`.github/workflows/skill-security.yml` scans **only the skill dirs changed in a PR** (path-filtered to
`.agents/skills/**`) with `--no-llm`, uploads SARIF, and fails the build when a changed/new skill trips
the risk threshold. It gates ADDITIONS/CHANGES (the ratchet) — it does NOT re-litigate the existing
trusted-but-capable skills (a hard gate on all of them would false-fail on capability alone).
