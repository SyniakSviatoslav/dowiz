#!/usr/bin/env bash
# EVERY LOG THE WORKER WRITES TO IS ARMED WITH ITS VENUE'S ID SCHEME (W-MG-MIG, 2026-10-31).
#
#   bash tools/deploy/scheme-armed.sh            # from the repo root; exit 0 = every file armed or named
#   bash tools/deploy/scheme-armed.sh --behaviour # the whole Worker suite, every log on legacy: no door may store sha256
#
# The hub crate's default is sha256 (M0). The expand release must WRITE LEGACY records until a venue is flipped
# (workers/api/src/scheme.rs), so every Worker file that loads or creates a `Hub`, `LogImage` or `StockLog` it
# may append to must call `scheme::arm` / `arm_scheme`. This is a file-level tripwire against a NEW file that
# constructs a log without arming it; a new unarmed site inside an already-armed file passes here and is
# caught live by the runbook's check (docs/design/MIGRATION-sha256-ids.md §3: `chained == 0` on an unflipped
# venue after the deploy). A file that only READS what it loads is named below with its reason.
#
# --behaviour IS THE CHECK; the rest is a fast tripwire (W-MG-TRANS). The file grep printed `0 unarmed` over a
# consent reload, a journal compaction, the witness's census, the chat prune and a venue's first order. So: run
# `cargo test --lib` in workers/api with SCHEME_BEHAVIOUR set: `scheme::choice` answers legacy for every log, and
# `hubdo/store.rs::put_chunks`, the one door every image takes, writes a line for any image that newly holds a sha256
# record. The only lines allowed are the tests below, each of which hands the object a fixture it built itself.
set -u
cd "$(dirname "$0")/../.." || exit 2
SRC=workers/api/src
FIXTURES='hubdo::counters::tests::one_price_edit_on_a_dubin_size_menu_is_timed_by_its_counters
hubdo::journal::tests::a_resident_journal_writes_exactly_the_bytes_the_reloaded_one_wrote
hubdo::journal::tests::one_price_edit_on_a_dubin_size_menu_writes_two_chunks_per_image_and_no_journal
hubdo::logmark::tests::a_cold_read_drops_the_entry_log_mark_and_storage_is_scanned_whole
hubdo::glue_turn::orders::tests::a_tail_with_a_foreign_key_is_refused_whole_and_writes_nothing
hubdo::glue_turn::orders::tests::offline_taps_are_decided_by_the_objects_own_deciders_and_none_is_lost
hubdo::glue_turn::tests::a_refused_tail_writes_nothing_and_names_why
hubdo::glue_turn::tests::a_tablet_syncs_its_offline_sale_and_the_oversale_waits_for_a_person
hubdo::glue_turn::tests::the_night_writes_the_stock_checkpoint_a_glue_deferred'
if [ "${1:-}" = --behaviour ]; then
  OUT=$(mktemp "${TMPDIR:-/tmp}/scheme-behaviour.XXXXXX"); LOG=$OUT.log; rm -f "$OUT"
  ( cd workers/api && SCHEME_BEHAVIOUR=$OUT ${SLOT:-bash /root/dowiz/bebop-lang/tools/slot.sh scheme} cargo test --offline --lib ) > "$LOG" 2>&1; rc=$?
  # A red test is not this check's verdict (one journal test compares bytes across its own sha256 fixture); no test run is.
  grep -q '^test result:' "$LOG" || { echo "FAIL scheme-armed --behaviour: no test run (rc=$rc, $LOG)"; exit 2; }
  doors=$(sed -n 's/^UNARMED image=\([^ ]*\) .* test=\(.*\)$/\1 \2/p' "$OUT" 2>/dev/null | sort -u | grep -v -F -f <(printf '%s\n' "$FIXTURES" | cut -c1-90))
  rm -f "$OUT"
  if [ -n "$doors" ]; then printf 'FAIL scheme-armed --behaviour: unarmed door: %s\n' "$doors"; exit 1; fi
  echo "scheme-armed --behaviour: $(grep '^test result:' "$LOG" | cut -d. -f2 | cut -c2-): no door stored sha256 at \"\" but the named fixtures"
  rm -f "$LOG"
  exit 0
fi
CTOR='Hub::load\|Hub::create_sized\|Hub::load_since\|LogImage::load\|LogImage::create\|LogImage::load_since\|StockLog::load\|StockLog::create_sized\|StockLog::load_since'
# read-only, or armed elsewhere: file<TAB>why
READ_ONLY='hubdo/archives.rs	folds an archive, never appends
hubdo/answers.rs	folds the log for a memo, never appends
hubdo/cube.rs	folds an archive, never appends
hubdo/journal/desk.rs	names LogImage::load in a comment only
hubdo/stock_turn.rs	stock_reset saves an EMPTY image: no record is written
platform_store.rs	arms inside load_log_at, whose venue is a required argument
quarantine.rs	the construction is in its test module
wallet.rs	the construction is in its test module
services/orders/chat/store.rs	the construction is in its test module; the rebuild is LogImage::fresh_sized
ebills/live.rs	an import dry-run that saves no image
ebills/live_tab.rs	a #[cfg(test)] #[ignore] harness: an in-memory hub that is never saved
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
