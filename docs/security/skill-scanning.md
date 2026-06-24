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
- MEDIUM: `systematic-debugging`, `supabase`.

## MCP servers (`.mcp.json`, scanned 2026-06-24, `--no-llm`)
All three are HIGH-CAPABILITY by design; SkillSpector (static, never executes) cannot CLEAR them — they
are accepted on a TRUST + CONFIG basis, not because they're "clean". Judge by category, not the number.
- `playwright-test` → **Microsoft `@playwright/test@1.60.0`** — trusted anchor (same tier as NVIDIA);
  NOT full-scanned (huge package, trusted, widely audited). Standard browser automation.
- `repowise` (uv tool, `~/.local/share/uv/tools/repowise`) → 100/CRITICAL/DO_NOT_INSTALL, 225 issues
  (115 HIGH): Data Exfiltration 73 + Dangerous Code Execution 56 — i.e. it reads the repo and SENDS
  embeddings, BY DESIGN. **Mitigation: our `.mcp.json` pins a LOCAL embedder (Ollama @127.0.0.1:11434)
  → no cloud egress of code. KEEP IT LOCAL** (a remote embedder would actually exfiltrate source).
- `browser-use` (uvx, `github.com/browser-use/browser-use`) → 100/CRITICAL/DO_NOT_INSTALL, 342 issues
  (5 CRITICAL, 132 HIGH): Dangerous Code Execution 85 + Tool Misuse 61 + Data Exfiltration 47 +
  Supply Chain 19. Widest blast radius (agentic browser: code-exec + LLM + arbitrary web + PyPI pull).
  Highest-caution of the three: run only when needed; telemetry off (set); BYOK LLM not default.
None showed a clean unambiguous malicious signature (e.g. hardcoded attacker URL receiving os.environ)
in the category tallies — but capability ≠ cleared. Anything secret-touching still gets human review.
- `emilkowalski/skills` (23, MEDIUM/CAUTION) — judged SAFE: 6 findings are all benign (README/LICENSE
  prose + design-instruction `SKILL.md` as "Memory Poisoning"); zero exec/exfil/credential/egress. Installed.

## CI gate (FF-style)
`.github/workflows/skill-security.yml` scans **only the skill dirs changed in a PR** (path-filtered to
`.agents/skills/**`) with `--no-llm`, uploads SARIF, and fails the build when a changed/new skill trips
the risk threshold. It gates ADDITIONS/CHANGES (the ratchet) — it does NOT re-litigate the existing
trusted-but-capable skills (a hard gate on all of them would false-fail on capability alone).
