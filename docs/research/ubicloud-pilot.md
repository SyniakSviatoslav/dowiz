# Ubicloud pilot — out-of-tree IaaS candidate (cloud-infra plane)

**STATUS: SCAFFOLDED — DO NOT USE. PARKED WITH TRIGGER.** Out-of-band. Not wired, not in CI, not a
dependency. Registered as a *future* candidate, dark.

## What it is
[ubicloud/ubicloud](https://github.com/ubicloud/ubicloud) (AGPL-3.0) — an open-source cloud (an "AWS
alternative") that runs on bare metal: VMs, managed Postgres, load balancers, K8s, private networking.
Can run on your own hardware or a provider's bare metal.

## Why it's a candidate (trigger, not now)
Current infra is Fly.io (app) + Supabase (Postgres) + R2 (assets) and it works. Ubicloud is the
**park-with-trigger** hedge for two futures: (a) a cost/scale inflection where managed Fly/Supabase pricing
stops making sense, or (b) a data-residency requirement (e.g. EU/Albania on-soil) that the current
providers can't satisfy. It could self-host Postgres + VMs on bare metal we control.

## Boundary
- **Trigger to revisit:** a costed scale-out plan OR a hard data-residency/sovereignty requirement.
  Until one lands: parked. Re-platforming a working stack on spec = wasted motion.
- AGPL-3.0 — infra/ops plane only; irrelevant as a product code dependency (**FORBIDDEN-DEP** as an import).
- If ever evaluated: stand up an isolated test project first; the current deploy topology
  (`deploy-topology` memory; prod vs staging Fly apps, Supabase DBs) is the migration surface to map, not
  touch. Prod migration would be a red-line, human-gated decision.
