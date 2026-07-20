#!/usr/bin/env bash
# topics.sh — legacy topic-id shim.
#
# Historically this file declared the per-topic Telegram chat IDs that
# `telemetry` posted to. Those IDs are now hard-coded inside
# tools/telemetry/telemetry itself (e.g. TELEGRAM_TOPIC_ID=291 for the
# Planning topic), so this file is now a no-op kept only so the
# `source "$DIR/topics.sh"` line in `telemetry` (and shellcheck's
# `source=topics.sh` directive) keeps resolving. If you add a new
# topic, prefer hard-coding the ID in `telemetry` next to its use.
#
# Nothing to export.
