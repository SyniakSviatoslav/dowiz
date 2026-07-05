# Incident — local Postgres crash into recovery (disk-full), 2026-06-28

> Dev-environment incident (NOT staging/prod). During the flow-simplification build the local Postgres
> (used for throwaway integration tests) crashed into permanent "recovery mode" mid-session. Root-caused to
> disk exhaustion driven by the pre-commit Docker build. Fixed at the source (pre-commit guardrail).

## Symptom
- `psql … FATAL: the database system is in recovery mode` on every connection, for minutes, not clearing.
- A `docker run postgres:16` to route around it failed `initdb: could not create directory "…/pg_wal":
  No space left on device`.

## Root cause (the failure chain)
1. `.husky/pre-commit` step **5/5** runs a full multi-stage `docker build -t dowiz-check .` on **every commit**
   (a Dockerfile pre-flight check; the real build runs in Fly's cloud).
2. Re-tagging `dowiz-check:latest` each build **orphans** the previous image (~593 MB) **plus the multi-stage
   intermediate layers** — and the hook did **no cleanup**. Net ~1 GB+ of dangling layers per commit.
3. This session committed ~50 times → **~50 GB** accumulated under `/var/lib/docker` on `/dev/sda1` (75 GB).
4. `df` hit **100%** (`74G/75G, 0 avail`). The local Postgres data dir (`/var/lib/postgresql/16/main`) is on
   the **same `/dev/sda1`**, so the server could not write WAL → it bounced into crash-recovery and could not
   complete WAL replay (no free space) → it stayed in "recovery mode".
5. `docker system prune -af` reclaimed **~40 GB** (51 images); Postgres then completed recovery and came back
   in ~2 s.

## Evidence
- `docker system df` before prune: Images 51, ~54.88 GB. After: disk 100% → 53%.
- `du -sh /var/lib/postgresql/16/main` = 168 MB (the DB itself was tiny — the disk was eaten by Docker, not PG).
- The container crash log: `initdb: … pg_wal: No space left on device`.

## Fix (durable, at the source) — `.husky/pre-commit` step 5/5
- **Disk guard:** skip the local Docker build when `< 10 GB` free (the cloud build still gates the actual
  deploy) — so a low-disk state can never be made worse by the pre-flight build.
- **Post-build prune:** `docker image prune -f` after every build reclaims the layers that build orphaned,
  capping the Docker footprint at ~one image instead of growing unbounded.

## Operability note
- The disk is shared by Docker + local Postgres + node_modules. If a future session commits heavily, run
  `docker image prune -f` (now automatic in pre-commit) or `docker system prune -af` to reclaim space.
- Considered but not done: stop rebuilding the Docker image on every commit (it's a per-commit cost of
  minutes). Left as-is to preserve the existing Dockerfile pre-flight signal; the prune makes it disk-safe.
