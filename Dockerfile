# DK-04 / DK-08: Zero-OCI static SPA image.
#
# Root-cause note (2026-07-13, MANIFESTO/DECISIONS D1): the legacy centralized
# server (apps/api + apps/worker, Fly, Supabase) was DROPPED. This image serves
# ONLY the static SPA built by `pnpm -r build` + `scripts/build-apps.ts`.
#
# DK-04: the nginx CONTAINER is GONE. It is replaced by `native-spa-server`, a
# from-scratch native-Rust axum binary living in tools/native-spa-server/. The
# behaviour (SPA fallback, /assets immutable cache, the EXACT security headers
# from the old docker/nginx-default.conf) is ported 1:1 and locked by RED tests.
#
# DK-08: ZERO-OCI RUNTIME. The final stage is `scratch` and copies ONLY the
# compiled static binary + the SPA dist + CA certs. There is NO nginx base, NO
# chainguard/nginx, NO OCI "app image" runtime — the artifact is a single static
# binary. The microVM/host-rootfs path is built separately by docker/mkosi-rootfs.sh.
#
# The build still runs the pnpm SPA build for the dist; only the runtime swaps.

# ---------------------------------------------------------------------------
# Stage 1: build the SPA via pnpm (frontend only, no backend, no migrator).
# ---------------------------------------------------------------------------
FROM node:22-slim AS spa-builder

WORKDIR /app

RUN corepack enable && corepack prepare pnpm@latest --activate

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml tsconfig.base.json ./
COPY packages ./packages
COPY apps ./apps
COPY scripts ./scripts

RUN pnpm install --frozen-lockfile
RUN pnpm -r build
RUN pnpm dlx tsx scripts/build-apps.ts

# ---------------------------------------------------------------------------
# Stage 2: compile the native-Rust static server (DK-04). Single static binary.
# ---------------------------------------------------------------------------
FROM rust:1 AS server-builder

WORKDIR /build
COPY tools/native-spa-server ./tools/native-spa-server
WORKDIR /build/tools/native-spa-server

# Build only the native-spa-server crate (dowiz is NOT a cargo workspace).
RUN cargo build --release --bin native-spa-server

# ---------------------------------------------------------------------------
# Stage 3 (DK-08 final): scratch runtime. Binary + dist + CA certs ONLY.
# ---------------------------------------------------------------------------
FROM scratch

# The single Rust binary — this IS the server, no container runtime beneath.
COPY --from=server-builder \
     /build/tools/native-spa-server/target/release/native-spa-server \
     /native-spa-server

# Static SPA dist (mirrors legacy /usr/share/nginx/html).
COPY --from=spa-builder /app/dist/public /var/www

# CA certs for any outbound TLS the binary might open (minimal, optional).
# If the spa-builder stage has them, copy; otherwise omit (server is static).
COPY --from=spa-builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Serve on 8080 by default, root = the copied SPA dist.
EXPOSE 8080
ENTRYPOINT ["/native-spa-server", "--root", "/var/www", "--port", "8080"]
