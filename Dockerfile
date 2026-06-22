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

# Install dependencies and bundle apps
RUN pnpm install --frozen-lockfile
# Soft access gate (ADR-soft-access-gate): the frontend CTA render flag is baked at
# vite build time. Defaults false so prod stays DARK (STOP-1); staging passes
# --build-arg VITE_ACCESS_GATE_PUBLIC_ENABLED=true to verify the CTA before launch.
ARG VITE_ACCESS_GATE_PUBLIC_ENABLED=false
ENV VITE_ACCESS_GATE_PUBLIC_ENABLED=$VITE_ACCESS_GATE_PUBLIC_ENABLED
# Telegram notification categories (TG_CATEGORY_GATING): frontend preference-centre
# render flag, baked at vite build time. Defaults false so prod stays DARK.
ARG VITE_TG_CATEGORY_GATING=false
ENV VITE_TG_CATEGORY_GATING=$VITE_TG_CATEGORY_GATING
RUN pnpm -r build
RUN pnpm dlx tsx scripts/build-apps.ts

# Production Runtime Stage
FROM node:22-slim

WORKDIR /app

# Copy the bundled single-file ESM artifacts
COPY --from=builder /app/dist /app/dist

# Copy static assets (API public files and Web frontend build)
COPY --from=builder /app/apps/api/public /app/dist/public
COPY --from=builder /app/apps/web/dist /app/dist/public

# Install external dependencies that cannot be bundled (native modules or complex SDKs)
RUN npm install argon2 sharp @aws-sdk/client-s3 @aws-sdk/lib-storage

# The specific script will be provided by Fly [processes] definition
ENTRYPOINT ["node"]
