# oh-my-mermaid pilot — G3 (tooling-integration-eval, Phase 3)

**Verdict: PILOT complete — MEDIUM value, low stakes. Keep as an opt-in local doc tool; do not wire into CI.**

## What was run (pinned, no cloud login)

```
npx -y oh-my-mermaid@0.2.0 init
npx -y oh-my-mermaid@0.2.0 write api-routes diagram  -   < api-routes.mmd
npx -y oh-my-mermaid@0.2.0 write api-routes description -
npx -y oh-my-mermaid@0.2.0 validate      # → api-routes: ✓ valid (6 warnings)
npx -y oh-my-mermaid@0.2.0 read  api-routes diagram > docs/architecture/api-routes.mmd
```

No `omm login` / `omm link` / `omm push` — nothing left the box (G4-style egress check N/A; the
tool's analysis is performed by the in-repo AI agent, omm is only the store + validator + viewer).

## Artifact

- `docs/architecture/api-routes.mmd` — the `apps/api/src/routes` layer (61 files / 7 audience groups
  over the shared order+dispatch core). Renders natively on GitHub.
- `.omm/api-routes/{diagram.mmd,description.md,meta.yaml}` — the tool's native, regenerable store.

## Tool evaluation

- **Useful:** `omm validate` gave real, actionable feedback (missing edge labels; a node-count
  warning suggesting the 22-node diagram be split into sub-perspectives — a genuine readability rule,
  not noise). The `.omm/` tree is a persisted, versioned, multi-perspective, regenerable-from-source
  diagram store — closer to living docs than an ad-hoc Mermaid block.
- **Thin:** producing a single diagram is a one-prompt task for an agent already in the repo; the
  tool's real value is the *structure + validation + persistence*, not the diagram authoring (which
  the agent does either way).
- **Cost:** MIT, TypeScript, local, no required keys/infra. `npx` pinned (do **not** `-g` install).

## vs repowise (overlap)

repowise `get_overview` already ships architecture maps + a guided tour as a **served index**
(out-of-repo, refreshed on `repowise update`). oh-my-mermaid's differentiator is **in-repo,
versioned `.mmd` artifacts** that render on GitHub and can be hand-curated + validated + diffed
(`omm diff`) per change. They are complementary, not redundant: repowise = always-fresh served map;
oh-my-mermaid = durable curated diagrams checked into `docs/`. Recommendation: adopt opt-in for
ADR/`docs/` diagrams when a durable picture is wanted; keep repowise as the live map. Not in CI.

LAST-REVIEWED: 2026-06-29
