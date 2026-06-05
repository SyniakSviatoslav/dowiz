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
RUN pnpm dlx tsx scripts/build-apps.ts

# Production Runtime Stage
FROM node:22-slim

WORKDIR /app

# Copy the bundled single-file ESM artifacts
COPY --from=builder /app/dist /app/dist

# The specific script will be provided by Fly [processes] definition
ENTRYPOINT ["node"]
