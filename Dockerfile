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
