#!/usr/bin/env bash
# secrets-history-scrub.sh — purge leaked prod Supabase credentials from the ENTIRE git history.
# Operator-run (the agent is hook-blocked from protect-path removal + force-push-main, by design).
#
# ⚠️ READ FIRST — order matters:
#   1. ROTATE the Supabase credentials in the dashboard BEFORE running this. History-scrubbing does
#      NOT secure the creds — anyone who ever cloned already has them. Rotation is the ONLY real fix.
#      Rotate: the `postgres` SUPERUSER password AND the `deliveryos_api_user` password
#      (project elxukhxvuycnftqwaghg). Update Fly secrets + local .env after rotating.
#   2. Coordinate: this rewrites ALL history and requires a force-push. Make sure no PR/branch is
#      mid-merge; tell any collaborator to re-clone after. The cloud plane-maintainer + worktrees
#      will need fresh clones.
#   3. This script works on a MIRROR CLONE in a temp dir — it does NOT touch your working repo until
#      the final force-push, which it PROMPTS for. Safe to run and inspect first.
set -euo pipefail

REMOTE="git@github.com:SyniakSviatoslav/dowiz.git"
WORK="$(mktemp -d)/dowiz-scrub.git"
IDENT='elxukhxvuycnftqwaghg\|aws-1-eu-central-1.pooler.supabase'   # host/project identifier (not the pw)

# The 13 throwaway files that hold the live cred (per docs/security/pre-opensource-secrets-audit.md).
PATHS=(
  ".agents/tmp/check-jobs.mjs" ".agents/tmp/check-session.mjs" ".agents/tmp/test-connections.cjs"
  "apps/api/fix-db.js" "fix-db.js"
  "packages/db/scripts/check-job-details.ts" "packages/db/scripts/check-job-schema.ts"
  "packages/db/scripts/check-notify-jobs2.ts" "packages/db/scripts/check-schemas.ts"
  "packages/db/scripts/test-connections.js"
  "packages/platform/test-notify2.cjs" "packages/platform/test-notify3.cjs"
  "docs/design/telegram-notifications-actions/staging-probe-bypassrls.md"
)

echo "== 0. Confirm rotation done =="
read -r -p "Have you ALREADY rotated the Supabase superuser + api-user passwords? (yes/no) " ans
[ "$ans" = "yes" ] || { echo "ABORT — rotate the credentials first (that is the real fix). See runbook."; exit 1; }

echo "== 1. git-filter-repo =="
command -v git-filter-repo >/dev/null 2>&1 || pip3 install --user git-filter-repo
export PATH="$HOME/.local/bin:$PATH"

echo "== 2. mirror clone (safe — separate from your working repo) =="
git clone --mirror "$REMOTE" "$WORK"
cd "$WORK"
echo "backup of pre-scrub refs → $WORK/../refs-backup.txt"
git show-ref > "$WORK/../refs-backup.txt"

echo "== 3. purge the files from ALL history =="
PATHARGS=(); for p in "${PATHS[@]}"; do PATHARGS+=(--path "$p"); done
git filter-repo --force --invert-paths "${PATHARGS[@]}"
# Belt-and-suspenders: if any secret string lingers inline elsewhere, add a replacements.txt
# (one `literal-old==>REMOVED` per line, populated by YOU from the rotated-away old values) and:
#   git filter-repo --force --replace-text ../replacements.txt

echo "== 4. VERIFY 100% clean — must print NOTHING =="
if git grep -I -n -E "$IDENT" $(git rev-list --all) 2>/dev/null | head; then
  echo "❌ STILL FOUND above — do NOT push. Investigate (inline occurrence needs --replace-text)."; exit 1
fi
echo "✅ history clean: zero occurrences of the Supabase host/project across all commits."
echo "   files gone from history: $(for p in "${PATHS[@]}"; do git log --all --oneline -- "$p" | head -1; done | wc -l) (expect 0 with matches)"

echo "== 5. FORCE-PUSH (irreversible) — review, then run manually =="
cat <<EOF
Reviewed and rotation confirmed? Then push the rewritten history:
    cd $WORK && git push --mirror --force
After: everyone re-clones; re-add the CI secrets gate (docs/proposals/ci-security-wiring.md);
delete this temp dir. This script intentionally does NOT auto-force-push.
EOF
