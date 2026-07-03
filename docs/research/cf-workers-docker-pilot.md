# Cloudflare Containers (Workers + Docker) pilot — out-of-tree edge-compute candidate

**STATUS: SCAFFOLDED — DO NOT USE. PARKED WITH TRIGGER.** Out-of-band. Not wired, not in CI, not a
dependency. Registered as a *future* candidate, dark.

## What it is
Cloudflare Containers — run a Docker image as part of a Worker, deployed to Cloudflare's edge without
managing infra ([cloudflare/containers](https://github.com/cloudflare/containers), the `Container` class
extends Durable Objects; images built/pushed via `wrangler deploy`). The "run a container next to edge
code" model. Related: `cloudflare/serverless-registry` (R2-backed registry).

## Why it's a candidate (trigger, not now)
The `workers-best-practices` + `wrangler` skills are already available, and R2 is already in the stack for
storefront assets. Cloudflare Containers is the hedge for **edge-proximate compute** — e.g. running the
`packages/voice` ASR or an image/menu-processing step close to users, or an edge cache/transform layer in
front of `/s/:slug` — without leaving the CF ecosystem we already touch for R2.

## Boundary
- **Trigger to revisit:** a concrete latency/edge requirement (voice inference at the edge, or a storefront
  edge-render/cache need) that Fly regions don't serve well. Until then: parked. The app plane stays Fly.
- Ops/infra plane only — never a product code dependency (**FORBIDDEN-DEP** as an import).
- Requires Docker locally for `wrangler deploy` image builds. Any edge service holds only its own config +
  the local-LLM / R2-scoped token — never a dowiz DB / RLS / tenant secret
  (`node scripts/skyvern-pilot/no-credential-attest.mjs <env>`).
- Do not split the deploy topology on spec: adding an edge lane means two deploy planes to keep coherent
  (`deploy-topology` memory). Justify with a measured need first.
