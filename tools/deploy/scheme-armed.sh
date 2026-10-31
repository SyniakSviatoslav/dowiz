#!/usr/bin/env bash
# EVERY LOG THE WORKER WRITES TO IS ARMED WITH ITS VENUE'S ID SCHEME (W-MG-MIG, 2026-10-31).
#
#   bash tools/deploy/scheme-armed.sh            # from the repo root; exit 0 = every file armed or named
#
# The hub crate's default is sha256 (M0). The expand release must WRITE LEGACY records until a venue is flipped
# (workers/api/src/scheme.rs), so every Worker file that loads or creates a `Hub`, `LogImage` or `StockLog` it
# may append to must call `scheme::arm` / `arm_scheme`. This is a file-level tripwire against a NEW file that
# constructs a log without arming it; a new unarmed site inside an already-armed file passes here and is
# caught live by the runbook's check (docs/design/MIGRATION-sha256-ids.md §3: `chained == 0` on an unflipped
# venue after the deploy). A file that only READS what it loads is named below with its reason.
set -u
cd "$(dirname "$0")/../.." || exit 2
SRC=workers/api/src
CTOR='Hub::load\|Hub::create_sized\|Hub::load_since\|LogImage::load\|LogImage::create\|LogImage::load_since\|StockLog::load\|StockLog::create_sized\|StockLog::load_since'
# read-only, or armed elsewhere: file<TAB>why
READ_ONLY='hubdo/archives.rs	folds an archive, never appends
hubdo/answers.rs	folds the log for a memo, never appends
hubdo/cube.rs	folds an archive, never appends
hubdo/journal/desk.rs	names LogImage::load in a comment only
hubdo/stock_turn.rs	stock_reset saves an EMPTY image: no record is written
platform_store.rs	armed by every caller: hubstore::load_log/with_log and errlog
quarantine.rs	the construction is in its test module
wallet.rs	the construction is in its test module
ebills/live.rs	an import dry-run that saves no image
command/amend/agrees.rs	golden tests over fixtures'
bad=0; armed=0; named=0
while IFS= read -r f; do
  rel=${f#"$SRC/"}
  case "$rel" in *tests.rs|*/tests/*) continue ;; esac
  # `set_sha256(` is a rebuild copying its source's scheme (`services/orders/chat/store.rs::rebuilt`).
  if grep -q 'scheme::arm\|arm_scheme\|set_sha256(' "$f"; then armed=$((armed + 1)); continue; fi
  why=$(printf '%s\n' "$READ_ONLY" | awk -F'\t' -v r="$rel" '$1 == r { print $2 }')
  if [ -n "$why" ]; then named=$((named + 1)); continue; fi
  bad=$((bad + 1)); echo "FAIL scheme-armed: $f constructs a log and never calls scheme::arm / arm_scheme:"
  grep -n "$CTOR" "$f" | head -5 | sed 's/^/   /'
done < <(grep -rl "$CTOR" "$SRC" --include='*.rs' | sort)
echo "scheme-armed: $armed files armed, $named read-only by name, $bad unarmed"
[ $bad = 0 ]
