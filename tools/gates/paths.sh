#!/bin/sh
# F33 — A DOCUMENT MAY NOT CITE A FILE THAT DOES NOT EXIST.
#
# THE DEFECT, and it sat in the file every agent and every new reader is told
# to read first. `CLAUDE.md`'s "Kernel authority model" cited
# `kernel/src/order_machine.rs`, `kernel/src/domain.rs` and
# `kernel/src/ports/agent/scope.rs`. None of the three has ever existed: they
# live in `crates/dowiz-core/`, and `kernel/` is the FACADE that re-exports
# them. Anyone who trusted the document went looking in the wrong crate for the
# FSM, the money law's neighbours and the red-line policy -- the three things
# the same section calls load-bearing.
#
# It survived because prose is never executed. This executes it.
#
# WHAT COUNTS AS A CITATION, and the rule is deliberately narrow. A backticked
# token that contains a `/` and begins with a directory that exists at the repo
# root. That takes `workers/api/src/lib.rs` and `crates/dowiz-hub/` and leaves
# alone `decide → Event`, `i64`/`i128`, `--offline`, URLs and the shell
# one-liners the same documents are full of. A narrow rule that fires is worth
# more than a broad one somebody switches off.
#
# A TRAILING `:123` OR `:88` IS STRIPPED, because `file.rs:88` is how this
# repo's documents point at a line and the line number is not part of the path.
# So is trailing prose punctuation, because `kernel/engine,` and `tests/main).`
# are sentences, not citations. A glob (`*`) is skipped: it is a pattern.
#
# AND THE TOKEN MUST LOOK LIKE A FILE OR A DIRECTORY: it ends in an extension
# or in `/`. Without that rule the gate counts `engine/kernel` and `and/or`,
# and a gate with false positives is one somebody adds an exemption to rather
# than a finding. It does mean an extensionless citation is invisible here;
# that is the price of a rule nobody switches off.
set -eu
cd "$(dirname "$0")/../.."
BASELINE_FILE=tools/gates/paths.baseline
DOCS="CLAUDE.md AGENTS.md DECISIONS.md CONTEXT-INDEX.md"

missing=""
for doc in $DOCS; do
  [ -f "$doc" ] || continue
  # Every backticked token in the document, one per line.
  cited=$(sed 's/`/\n`/g' "$doc" | sed -n 's/^`\([^`]*\)`\{0,1\}$/\1/p' | sort -u)
  for tok in $cited; do
    case "$tok" in
      */*) ;;                       # only things shaped like a path
      *) continue ;;
    esac
    case "$tok" in
      *\**|http*|*://*) continue ;; # patterns and URLs are not citations
    esac
    path=$(printf '%s' "$tok" | sed 's/[,.)]*$//; s/:[0-9-]*$//')
    case "$path" in
      */) ;;                        # a directory
      *.*) ;;                       # something with an extension
      *) continue ;;
    esac
    root=${path%%/*}
    # Only paths under a real top-level directory. This is what keeps
    # `decide/fold`, `and/or` and `24/7` out of the count.
    [ -d "$root" ] || continue
    [ -e "$path" ] && continue
    missing="$missing$doc: $path\n"
  done
done

n=$(printf "$missing" | grep -c . || true)
echo "paths: $n cited path(s) that do not exist, across $DOCS"
if [ ! -f "$BASELINE_FILE" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "paths: baseline recorded at $n"
  exit 0
fi
baseline=$(cat "$BASELINE_FILE")
if [ "$n" -gt "$baseline" ]; then
  echo "paths: FAILED — $((n - baseline)) more than the baseline of $baseline."
  printf "$missing" | sed 's/^/  /'
  exit 1
fi
if [ "$n" -lt "$baseline" ]; then
  echo "$n" > "$BASELINE_FILE"
  echo "paths: ratchet lowered $baseline -> $n. Commit the baseline with the change."
fi
if [ "$n" -gt 0 ]; then
  printf "$missing" | sed 's/^/  /'
fi
exit 0
