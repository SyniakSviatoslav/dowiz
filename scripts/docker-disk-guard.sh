#!/usr/bin/env bash
# docker-disk-guard.sh — keep Docker from filling the volume that also holds the local Postgres data dir.
#
# Why this exists: /var/lib/docker and /var/lib/postgresql share /dev/sda1. The pre-commit Docker build
# re-tags dowiz-check:latest each commit, orphaning the prior image + multi-stage layers; ~50 commits filled
# the disk and crashed Postgres into WAL-recovery (docs/incidents/2026-06-28-local-pg-disk-crash.md).
#
# What it does: reclaims Docker disk in ESCALATING, IDEMPOTENT tiers — cheapest + safest first — and STOPS as
# soon as <target_gb> is free on /. Normal commits only ever run tier 1 (remove the dangling layer the build
# just orphaned) and stop; a near-full disk escalates further. Running containers and in-use images always
# survive (prune never touches them; the aggressive tier additionally filters by age). Best-effort: every
# command is guarded, the script never fails a commit.
#
# Usage: bash scripts/docker-disk-guard.sh [target_free_gb]   (default 15)
set -u
TARGET_GB="${1:-15}"

command -v docker >/dev/null 2>&1 || exit 0

free_gb() { df -BG --output=avail / 2>/dev/null | tail -1 | tr -dc '0-9'; }
have_target() { local f; f="$(free_gb)"; [ -n "$f" ] && [ "$f" -ge "$TARGET_GB" ]; }

START="$(free_gb)"
have_target && exit 0   # already enough headroom — do nothing

# Tiers, cheapest/safest → most aggressive. Re-check after each; stop once the target is met.
#   1. dangling images   — the orphaned <none> layers from re-tagging :latest (the actual accumulator)
#   2. build cache       — BuildKit cache (no-op on the legacy builder; harmless)
#   3. stopped containers
#   4. unused volumes    — e.g. throwaway test-DB volumes (100% reclaimable in the incident)
#   5. unused images >24h — keeps the current dowiz-check + recently-pulled bases; clears stale ones
for tier in \
  "image prune -f" \
  "builder prune -f" \
  "container prune -f" \
  "volume prune -f" \
  "image prune -af --filter until=24h"; do
  have_target && break
  # shellcheck disable=SC2086
  docker $tier >/dev/null 2>&1 || true
done

END="$(free_gb)"
echo "[docker-disk-guard] / free: ${START}G → ${END}G (target ${TARGET_GB}G)"
exit 0
