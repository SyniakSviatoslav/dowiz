# Static SPA image — decentralized app shell.
#
# Root-cause note (2026-07-13, MANIFESTO/DECISIONS D1): the legacy
# centralized server (apps/api + apps/worker, deployed via attic/fly.toml with
# dist/api/server.cjs, dist/worker, and a release_command migrator) was DROPPED.
# There is no server process, no central DB, no Supabase, no Fly. This image
# serves ONLY the static SPA built by `pnpm -r build` (apps/web/dist), which
# the thin client / reference alt-client loads. It does NOT bundle or run any
# Node backend, and it does NOT auto-migrate a database.

# Build Stage
FROM node:22-slim AS builder

WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Copy monorepo config
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY packages ./packages
COPY apps ./apps
COPY scripts ./scripts

# Install dependencies and build all workspaces (produces apps/web/dist).
RUN pnpm install --frozen-lockfile
RUN pnpm -r build

# Assemble the static SPA artifact only (no backend, no migrator).
RUN pnpm dlx tsx scripts/build-apps.ts

# Production Runtime Stage — static file server, no server-side logic.
# Chainguard nginx: distroless (no shell/pkg-manager), SBOM, daily CVE rebuilds.
FROM cgr.dev/chainguard/nginx:latest AS runtime

# Static web root produced by build-apps.ts.
COPY --from=builder /app/dist/public /usr/share/nginx/html

# Hardened, minimal config: SPA fallback to index.html, no server runtime.
# Config lives in docker/nginx-default.conf (reviewable outside the Dockerfile;
# avoids the BuildKit-only heredoc COPY that the legacy builder rejects).
COPY docker/nginx-default.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080
