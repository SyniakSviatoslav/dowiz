#!/usr/bin/env bash
# Canonical staging deploy — carries the dark-launch VITE build-args that staging verifies with.
#
# WHY THIS EXISTS: the menu-characteristics features (compare, calorie/macro sort) are gated by
# BUILD-TIME flags (import.meta.env.VITE_*), default OFF in the Dockerfile so PROD stays dark. A plain
# `flyctl deploy -a dowiz-staging` omits the build-args → the SPA bakes them OFF → the features silently
# vanish from /s/demo. This happened twice on 2026-06-30 (a flag-less teammate deploy + a feature deploy
# both dropped them). Always deploy staging through THIS script so the flag set can't drift again.
#
# Usage:  scripts/deploy-staging.sh [extra flyctl args...]
#   e.g.  scripts/deploy-staging.sh                       # standard staging deploy
#         scripts/deploy-staging.sh --build-arg VITE_X=true   # add more args for a one-off
#
# Prod stays DARK: CI deploys prod from `main` with the Dockerfile defaults (all VITE_* dark). Enabling
# any of these on prod is a separate, explicit launch decision — never copy these args into the prod path.
set -euo pipefail

export PATH="$HOME/.fly/bin:$PATH"

# Staging dark-launch flag set (see memory: staging-deploy-flags-2026-06-30).
# VITE_MENU_ALLERGEN_FILTER is intentionally NOT set — ALLERGENS_ENABLED is hardcoded false (operator
# freeze), so the flag is a no-op until that freeze lifts.
STAGING_BUILD_ARGS=(
  --build-arg VITE_MENU_CHARACTERISTICS_ENABLED=true
  --build-arg VITE_MENU_CHARACTERISTICS_COMPARISON=true
  --build-arg VITE_MENU_CHARACTERISTICS_FILTER=true
)

echo "→ Deploying to dowiz-staging with menu-characteristics flags ON…"
exec flyctl deploy -a dowiz-staging --remote-only "${STAGING_BUILD_ARGS[@]}" "$@"
