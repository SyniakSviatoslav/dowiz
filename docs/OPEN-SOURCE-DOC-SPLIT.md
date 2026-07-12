# Open-Source Documentation Split

**Date:** 2026-07-02
**Purpose:** decide which docs are safe to ship in the **public** AGPL repo vs
which must stay **private** (internal strategy, financial model, confidential
business analysis) before the repository is made public.

> This is an advisory classification for a human to action. Nothing here is
> removed automatically. Pair it with the
> [`pre-opensource-secrets-audit.md`](./security/pre-opensource-secrets-audit.md)
> (secrets are a hard gate) before flipping the repo public.

---

## Rule of thumb

- **PUBLIC** — anything an external contributor needs to build, run, understand,
  and extend the software: code, ADRs, architecture, contributor/security/
  runbook docs, design specs. Transparency here is the whole point of open
  source.
- **PRIVATE** — anything that is **business strategy, financials, competitive
  positioning, acquisition/exit planning, or internal-only summaries**. These
  give away commercial advantage without helping anyone run the software, and
  several are already marked "Конфіденційно".

## Must stay PRIVATE (do NOT publish)

| Path | Why |
| ---- | --- |
| `DeliveryOS-Business-Value-Sort.md` (root) | Commercial strategy: pricing wedge vs aggregators, ROI framing, per-system business ranking. Marked *Конфіденційно*. |
| `DeliveryOS-Product-Goal-Alignment-Audit.md` (root) | Internal strategic audit. Marked *Конфіденційно*. |
| `DeliveryOS-As-Built-Summary-v1.md` (root) | Internal "as-built" summary. Marked *Конфіденційно*. |
| `DeliveryOS-Full-Summary-v2.docx` (root) | Internal full summary (binary). Not a contributor artifact. |
| `docs/DeliveryOS-Architecture-Update-v3.1.md` | Internal handoff/summary. Marked *Конфіденційно*. |
| `docs/DeliveryOS-Context-Handoff-v4_5.md` | Internal context handoff. Marked *Конфіденційно*. |
| `docs/DeliveryOS-Service-Build-Plan-v4_4.md` | Internal build/roadmap plan. Marked *Конфіденційно*. |
| `docs/audit/execution-plan.md` | Internal execution plan. Marked *Конфіденційно*. |
| `docs/finance/**` (e.g. `settlements.md`) | Financial model / settlement economics. |
| `docs/acquisition/**` (INVENTORY + provisioning notes) | Acquisition/go-to-market strategy. *(Note: any `migration-*.ts` here are code — decide per-file; the strategy INVENTORY is private.)* |
| `docs/exits/**` | Exit-strategy framing (even where filenames overlap with tech topics — review content). |

**Also review before publishing (may contain internal strategy / prospect PII):**
`docs/acquisition/` prospect data, `docs/design-review/` competitive audits, and
any `docs/research/*-pilot.md` that names commercial terms. Prospect/venue data
scraped for demos (real business names, addresses) should be scrubbed or kept
private.

## Safe to publish PUBLIC

| Path | Why |
| ---- | --- |
| `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `TRADEMARK.md`, `LICENSE` | The open-source front door. |
| `docs/adr/**` | Architecture Decision Records — the "why", exactly what contributors need. |
| `docs/architecture/**` | System architecture. |
| `docs/runbooks/**`, `docs/backup/**` (redact any host/secret specifics) | Operational docs — publishable once ops-specific endpoints/IDs are scrubbed. |
| `docs/operating-model/**`, `docs/agent-rules/**`, `docs/governance/**` | How the project is built and governed. |
| `docs/frontend/**`, `docs/branding/**`, `docs/seo/**`, `design.md`, `PRODUCT.md` | Product & design specs (contain no secrets). |
| `docs/regressions/**`, `docs/lessons/**`, `docs/reflections/**`, `docs/harness/**` | Engineering/quality process — fine to be transparent. |
| `docs/migration/**`, `docs/phase4/**`, `docs/phase5/**`, `docs/incidents/**` | Technical history (redact any live infra identifiers). |
| `AGENTS.md`, `CONVENTIONS.md`, `CONTEXT-INDEX.md`, `MEMORY-MAP.md`, `TOOLING-REGISTRY.md`, `DEPLOYMENT-FIXES.md` | Dev-facing conventions/tooling (skim for embedded secrets/host IDs first). |

## "Конфіденційно" markers to remove

These files carry an explicit *Конфіденційно* (Confidential) marker. Because
they are classified **PRIVATE above**, the correct action is to **keep them out
of the public repo entirely** — do **not** merely strip the marker to make them
look publishable. Files with the marker:

```
DeliveryOS-As-Built-Summary-v1.md
DeliveryOS-Business-Value-Sort.md
DeliveryOS-Product-Goal-Alignment-Audit.md
docs/DeliveryOS-Architecture-Update-v3.1.md
docs/DeliveryOS-Context-Handoff-v4_5.md
docs/DeliveryOS-Service-Build-Plan-v4_4.md
docs/audit/execution-plan.md
```

If any *content* from these ever needs to appear publicly, extract the neutral
technical parts into a new doc and drop the confidential business framing —
never publish the marked file as-is or with only the marker deleted.

## Recommended mechanism

- Move private docs to a **separate private repo** (or a git-ignored
  `private/` tree), rather than deleting — the team still needs them.
- Because several of these are already in **git history**, moving them now does
  **not** remove them from history. If they must never have been public, they
  need the same history-rewrite treatment as the secrets audit (filter-repo/BFG)
  **before** the repo goes public. Coordinate the two history rewrites into one
  operation.
