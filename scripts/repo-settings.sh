#!/usr/bin/env bash
# P18 §2.4 — dowiz repo-settings preparation script.
#
# ⚠ OWNER-RUN AT FLIP TIME ONLY. This script mutates GitHub-side settings
# (repo metadata, topics, branch protection). It does NOT change repo
# visibility and does NOT perform the public flip — those are separate, explicit
# operator actions. The agent authors it; only the operator runs it, with their
# own `repo:admin` token (the build sandbox token may not even resolve the repo).
#
# Idempotent: safe to re-run. Requires `gh` authenticated with a token that has
# `repo:admin` on SyniakSviatoslav/dowiz.
#
# Brand: {{BRAND}} placeholder — resolve O16 (dowiz vs DeliveryOS) BEFORE running,
# then replace the DESCRIPTION / TOPICS below accordingly.

set -euo pipefail

REPO="SyniakSviatoslav/dowiz"

# --- operator should confirm before any mutation -----------------------------
if [ "${1:-}" != "--yes" ]; then
  echo "DRY RUN (no changes). Re-run with --yes to apply against GitHub."
  echo "  gh auth status   # confirm a repo:admin token for $REPO is active"
  exit 0
fi

if ! command -v gh >/dev/null; then
  echo "error: gh CLI not found" >&2
  exit 1
fi

echo ">> applying repo settings to $REPO"

# --- repo metadata + issues on / wiki off -----------------------------
gh repo edit "$REPO" \
  --description "{{BRAND}} — sovereign, post-quantum delivery infrastructure: a deterministic Rust/WASM kernel and mesh protocol. AGPL-3.0." \
  --enable-issues --disable-wiki

# --- topics -------------------------------------------------------------
gh api "repos/$REPO/topics" -X PUT \
  -f names[]=delivery -f names[]=post-quantum -f names[]=mesh -f names[]=rust \
  -f names[]=wasm -f names[]=agpl -f names[]=self-hosted -f names[]=deterministic \
  -f names[]=event-sourcing -f names[]=offline-first

# --- branch protection on main -----------------------------------------
# Contexts must match the Phase 1 CI job names (gitleaks + cargo-test + DCO).
gh api "repos/$REPO/branches/main/protection" \
  -X PUT \
  -f required_status_checks='{"strict":true,"contexts":["gitleaks","cargo-test","dco"]}' \
  -f enforce_admins=true \
  -f required_pull_request_reviews='{"required_approving_review_count":1}' \
  -f require_signatures=true \
  -f dismiss_stale_reviews=true

# --- Discussions (activate AT flip, not before) ------------------------
gh api "repos/$REPO" -X PATCH -f has_discussions=true

echo ">> done. Verify: gh repo view $REPO"
echo ">> NOTE: the public flip (private -> public) is a SEPARATE operator action."
