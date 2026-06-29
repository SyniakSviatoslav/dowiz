# STORM + Scrapling pilot — out-of-tree research/scrape harness

**STATUS: SCAFFOLDED — PENDING OPERATOR RUN.** Out-of-band. Not wired, not in CI, not a dependency.

## What they are
- [Stanford STORM](https://github.com/stanford-oval/storm) (Python/dspy; PyPI `knowledge-storm`) — LLM
  knowledge-curation → long-form **cited** reports (perspective-guided question asking + simulated expert
  conversation). The "researches + writes" layer.
- [Scrapling](https://github.com/D4Vinci/Scrapling) (BSD-3-Clause, Python) — adaptive web-scraping
  framework (single request → full crawl). The "fetch public sources" layer.

## 🔶 Conflict / coherence eval (the user's "don't conflict, utilize" rule)
STORM, DeerFlow, and the in-tree **`deep-research` skill** (+ ODR) are ALL research-report generators —
**overlapping**. Do NOT run three research lanes. Coherence decision:
- **Research synthesis:** keep ONE lane. The `deep-research` skill is already in-tree/available — make it
  the default; STORM + DeerFlow are out-of-tree pilots to **benchmark against it**, adopt the winner (one),
  REJECT the rest. (Avoids redundant tooling + token waste.)
- **Scraping:** Scrapling is the **complementary fetch layer** (no conflict) — feeds whichever synthesis lane wins.

## Boundary & controls (G5 + scraping ethics)
- Python, OUT OF TREE. `knowledge-storm`/`scrapling`/`dspy` are FORBIDDEN-DEP (never product deps/imports).
- **Scraping is respectful public-source research only** — robots.txt honored, rate-limited (≥500ms),
  public sources only, NO PII harvest, no auth-walled/ToS-violating targets. Mechanical gate:
  `node scripts/scrape-pilot/scraping-conduct-attest.mjs <config.json>` (fail-closed; red→green proven).
- No dowiz credential in the sidecar env — `node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>`.
- Local LLM + telemetry off; egress allowlisted.

## What it's for
A deeper, continuously-refreshed **food-delivery-market** intel sweep (public discussions + reports) than a
one-shot WebSearch. The first in-house pass (no external scraper needed) is captured in
`docs/research/food-delivery-market-brief-2026.md`; STORM/Scrapling are the out-of-band path for a heavier,
repeatable sweep — pending operator + the conduct/credential gates.
