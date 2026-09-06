#!/usr/bin/env bash
# journal_lint.sh (item 11, retro D13/P2): a docs/exp.journal line claiming
# transient/flaky/not reproducible without an errno=|rc= AND an EXPECT: is an
# unfalsifiable verdict (L10) -- refuse it. Run by tools/hooks/pre-push over the lines
# ADDED since origin/main (existing lines are grandfathered).
# Usage: tools/journal_lint.sh          # lint docs/exp.journal lines added vs origin/main
#        tools/journal_lint.sh FILE     # lint every line of FILE (scratch-copy testing)
cd "$(dirname "$0")/.." || exit 1
bad=0
check() {
  echo "$1" | grep -qiE 'transient|flaky|not reproducible' || return 0
  echo "$1" | grep -qE 'errno=|rc=' && echo "$1" | grep -q 'EXPECT:' && return 0
  echo "journal_lint: FAIL (needs errno=|rc= and EXPECT:): $1"; bad=1
}
if [ -n "${1:-}" ]; then
  while IFS= read -r line; do check "$line"; done < "$1"
else
  while IFS= read -r line; do check "${line#+}"; done < <(git diff origin/main..HEAD -- docs/exp.journal | grep -E '^\+[^+]')
fi
[ $bad = 0 ] && echo "journal_lint: clean"
exit $bad
