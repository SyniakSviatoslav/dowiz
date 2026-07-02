#!/usr/bin/env bash
# harness-curation-local.sh — LOCAL weekly self-improvement run (meta-loop P1, 2026-07-02).
# Fallback twin of the claude.ai cloud routine `harness-librarian-health`
# (trig_01LPSCXnRhhH68YoijEH6rsx), installed because the cloud environment's GitHub access
# check was failing (github_repo_access_denied). Runs headless Claude on THIS checkout.
# When the cloud routine's auth heals, keep ONE of the two (this one runs Mondays 09:37,
# the cloud one 09:07 UTC — remove the crontab entry or disable the routine).
set -uo pipefail
cd "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
LOG=".claude/logs/curation-cron.log"
mkdir -p "$(dirname "$LOG")"
echo "=== harness-curation-local $(date -Iseconds) ===" >>"$LOG"

claude -p --permission-mode acceptEdits "You are the weekly harness curation run for this repo (local twin of the harness-librarian-health routine; meta-loop P1 2026-07-02). Work on the CURRENT checkout. HARD LIMITS: writes confined to docs/lessons/**, docs/reflections/**, docs/governance/**; zero product code; never weaken a gate/test; never edit CLAUDE.md or .claude/**.

STEP 1 — LIBRARIAN: follow .claude/agents/librarian.md (distill → challenge → promote → prune) over docs/reflections/INBOX/: qualifying reflections → ONE atomic trigger-keyed lesson each in docs/lessons/ + INDEX row (keep INDEX.md machine-parseable '| TRIGGER | file |'), then archive the reflection; unfilled-WHY placeholders get back-filled from ledger/git evidence (CONFIDENCE noted) or rejected with a reason. PRUNE lessons fully covered by Tier-1 guardrails; use pre-edit-lessons/inject counts from .claude/logs/harness-events.jsonl — zero-hit lessons >30d old are prune candidates. The store must not grow net.

STEP 2 — RETRO (only if INBOX held 3+ reflections): run cause-critic, pattern-critic, ratchet-critic per their .claude/agents/ definitions; write docs/reflections/RETRO/<date>-weekly-retro.md; enact doc/lesson-level outputs; list guardrail-level outputs as proposals in the summary.

STEP 3 — HEALTH: run node scripts/agent-health-pass.mjs and read the report it writes.

DELIVERY: if anything changed, make ONE contextual docs-only commit on the current branch (message: 'chore(harness): weekly curation + health pass <date>' + summary of curated/warnings/proposals). If nothing to curate and zero warnings, commit nothing and just print a one-line no-op summary." >>"$LOG" 2>&1

echo "=== exit=$? $(date -Iseconds) ===" >>"$LOG"
