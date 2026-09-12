#!/bin/bash
# bebopc.sh — bebop compiler identity, versioning, and version-switching
# Manages compiler binaries, checks fixpoint, and prevents compiler drift

set -e
# Every path below is relative to the repo root, so resolve it once (same idiom as cc.sh:14).
cd "$(dirname "$0")/.." || exit 1

CACHE_DIR="/root/.cache/bebop/compilers"
MANIFEST_FILE="compilers/MANIFEST.tsv"
CURRENT_BIN="bebop.bin"
SCRATCH="${BEBOP_TMP:-/tmp/opencode}/bebopc"; mkdir -p "$SCRATCH"
# A self-compile of bebop.bp is ~90 s on this box and the chain measures 516 s for three of
# them plus a battery. 60 s was not a timeout, it was a coin flip -- and worse, a timeout read
# as "not a fixpoint", which is a failure wearing a verdict's clothes (AGENTS L1: failures are
# loud). 300 s, and a timeout is reported AS a timeout, never as a mismatch.
CC_TIMEOUT=${CC_TIMEOUT:-300}

# Helper: compute MD5 digest of a file (first 8 hex chars)
digest() {
  local file="$1"
  md5sum < "$file" | cut -c1-8
}

# Helper: compute digest of bebop.bp
source_md5() {
  digest "bebop.bp"
}

# Helper: compile bebop.bp WITH <compiler> and echo the digest of what comes out.
# Echoes the literal `TIMEOUT` or `COMPILEFAIL` instead when that is what happened, so no
# caller can mistake a broken run for a digest mismatch. One compile per call, never two.
compiles_to() {
  local compiler="$1"
  local out="$SCRATCH/out.$$.bin"
  rm -f "$out"
  local rc=0
  timeout "$CC_TIMEOUT" bash tools/cc.sh "$compiler" bebop.bp "$out" >/dev/null 2>&1 || rc=$?
  if [ "$rc" = 124 ]; then echo TIMEOUT; return 0; fi
  if [ "$rc" != 0 ] || [ ! -s "$out" ]; then echo COMPILEFAIL; return 0; fi
  digest "$out"
  rm -f "$out"
}

# Helper: is <candidate> its own fixpoint? Exit 0 only on a real, measured match.
is_fixpoint() {
  local candidate="$1"
  local out
  out=$(compiles_to "$candidate")
  [ "$out" = "$(digest "$candidate")" ]
}

# Command: id
cmd_id() {
  local promoted_digest
  promoted_digest=$(digest "$CURRENT_BIN")

  local source_md5_val
  source_md5_val=$(source_md5)

  local compiles_to_digest
  compiles_to_digest=$(compiles_to "$CURRENT_BIN")

  # A compile that timed out or failed is NOT a fixpoint verdict -- say which it was and exit
  # non-zero, so nothing downstream records `fixpoint=no` for a run that never finished.
  case "$compiles_to_digest" in
    TIMEOUT|COMPILEFAIL)
      echo "promoted=$promoted_digest"
      echo "source_md5=$source_md5_val"
      echo "compiles_to=$compiles_to_digest"
      echo "fixpoint=unknown"
      echo "commit=$(git rev-parse --short HEAD)"
      echo "ERROR: self-compile did not complete ($compiles_to_digest after ${CC_TIMEOUT}s) -- this is not a fixpoint verdict" >&2
      return 2
      ;;
  esac

  local is_fixpoint
  if [ "$compiles_to_digest" = "$promoted_digest" ]; then
    is_fixpoint="yes"
  else
    is_fixpoint="no"
  fi

  echo "promoted=$promoted_digest"
  echo "source_md5=$source_md5_val"
  echo "compiles_to=$compiles_to_digest"
  echo "fixpoint=$is_fixpoint"
  echo "commit=$(git rev-parse --short HEAD)"

  if [ "$is_fixpoint" != "yes" ]; then
    echo "ERROR: promoted ($promoted_digest) != compiles_to ($compiles_to_digest)" >&2
    return 1
  fi

  return 0
}

# Command: list
cmd_list() {
  if [ ! -f "$MANIFEST_FILE" ]; then
    echo "MANIFEST not found: $MANIFEST_FILE" >&2
    return 1
  fi

  # Newest first = by the DATE column (4), descending. The previous key was column 3
  # (source_md5), which is a hash: sorting on it produced an arbitrary order labelled "newest".
  grep -v '^#' "$MANIFEST_FILE" | head -1
  grep -v '^#' "$MANIFEST_FILE" | tail -n +2 | sort -r -t"$(printf '\t')" -k4,4
}

# Command: save
cmd_save() {
  local note="$*"

  mkdir -p "$CACHE_DIR"

  local current_digest
  current_digest=$(digest "$CURRENT_BIN")

  local source_md5_val
  source_md5_val=$(source_md5)

  local source_commit
  # The last commit that TOUCHED bebop.bp is not the provenance of this binary when the source
  # is dirty -- a save during an uncommitted edit would name a commit whose source produces a
  # DIFFERENT compiler, which is the exact confusion this file exists to end. Record HEAD, and
  # say `+dirty` when bebop.bp differs from it; the commit that lands the pair then supersedes it.
  source_commit=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
  git diff --quiet HEAD -- bebop.bp 2>/dev/null || source_commit="${source_commit}+dirty"

  local short_sha
  short_sha=$(git rev-parse --short HEAD)

  local date
  date=$(date +%Y-%m-%d)

  # Cache presence and MANIFEST presence are independent, and conflating them hid a defect:
  # cmd_promote copies the candidate into the cache BEFORE calling save, so save always found
  # it cached, returned early, and never wrote the manifest row. The tool's own main path
  # recorded no provenance -- found 2026-09-12 promoting f0747bfc. Cache first, then ALWAYS
  # fall through to the manifest.
  if [ -f "$CACHE_DIR/${current_digest}.bin" ] && [ "$(digest "$CACHE_DIR/${current_digest}.bin")" = "$current_digest" ]; then
    echo "cache: $current_digest already present"
  else
    cp "$CURRENT_BIN" "$CACHE_DIR/${current_digest}.bin"

    local copy_digest
    copy_digest=$(digest "$CACHE_DIR/${current_digest}.bin")
    if [ "$copy_digest" != "$current_digest" ]; then
      echo "ERROR: copy failed to match digest" >&2
      return 1
    fi
    echo "cache: $current_digest written"
  fi

  # Add to manifest if not present
  if [ ! -f "$MANIFEST_FILE" ]; then
    # Create manifest with header
    mkdir -p "$(dirname "$MANIFEST_FILE")"
    printf '# compilers/MANIFEST.tsv -- one row per bebop.bin that was ever promoted into this tree.\n# Columns: digest source_commit source_md5 date fixpoint note\n' > "$MANIFEST_FILE"
    printf 'digest\tsource_commit\tsource_md5\tdate\tfixpoint\tnote\n' >> "$MANIFEST_FILE"
  fi

  # Check if digest is already in manifest
  if grep -v '^#' "$MANIFEST_FILE" | cut -f1 | grep -qx "$current_digest"; then
    echo "digest $current_digest already in manifest (no update)"
  else
    # Determine if this is a fixpoint
    local is_fp
    if is_fixpoint "$CURRENT_BIN"; then
      is_fp="yes"
    else
      is_fp="no"
    fi

    # Append row to manifest
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$current_digest" "$source_commit" "$source_md5_val" "$date" "$is_fp" "$note" >> "$MANIFEST_FILE"
  fi

  echo "saved $current_digest"
}

# Command: use
cmd_use() {
  local target_digest="$1"

  if [ -z "$target_digest" ]; then
    echo "usage: tools/bebopc.sh use <digest8>" >&2
    return 1
  fi

  # Check if digest is in cache
  if [ ! -f "$CACHE_DIR/${target_digest}.bin" ]; then
    echo "ERROR: digest $target_digest not in cache" >&2
    return 1
  fi

  # Verify the cached binary has the expected digest
  local cached_digest
  cached_digest=$(digest "$CACHE_DIR/${target_digest}.bin")
  if [ "$cached_digest" != "$target_digest" ]; then
    echo "ERROR: cached binary digest $cached_digest does not match requested $target_digest" >&2
    return 1
  fi

  # Atomic swap: copy to temp, then move
  local tmp_file="${CURRENT_BIN}.tmp.$$"
  cp "$CACHE_DIR/${target_digest}.bin" "$tmp_file"

  # Verify temp file before moving
  local tmp_digest
  tmp_digest=$(digest "$tmp_file")
  if [ "$tmp_digest" != "$target_digest" ]; then
    rm -f "$tmp_file"
    echo "ERROR: temp file digest $tmp_digest does not match expected $target_digest" >&2
    return 1
  fi

  # Atomic move
  mv "$tmp_file" "$CURRENT_BIN"

  echo "switched to $target_digest"
}

# Command: promote
cmd_promote() {
  local candidate="$1"

  if [ -z "$candidate" ]; then
    echo "usage: tools/bebopc.sh promote <candidate.bin>" >&2
    return 1
  fi

  if [ ! -f "$candidate" ]; then
    echo "ERROR: candidate file not found: $candidate" >&2
    return 1
  fi

  # ONE self-compile decides this. The previous shape called is_fixpoint() and then compiled
  # a second time just to print what came out -- two ~90 s compiles to refuse one binary.
  local candidate_digest produced
  candidate_digest=$(digest "$candidate")
  produced=$(compiles_to "$candidate")

  case "$produced" in
    TIMEOUT|COMPILEFAIL)
      echo "ERROR: promotion refused: candidate $candidate_digest self-compile ended in $produced after ${CC_TIMEOUT}s -- refusing on an unfinished run, not on a verdict" >&2
      return 2
      ;;
  esac
  if [ "$produced" != "$candidate_digest" ]; then
    echo "ERROR: promotion refused: candidate $candidate_digest is not a fixpoint (compiles bebop.bp to $produced, not $candidate_digest)" >&2
    return 1
  fi

  # Copy to cache
  mkdir -p "$CACHE_DIR"
  cp "$candidate" "$CACHE_DIR/${candidate_digest}.bin"

  # Verify cache copy
  local cache_digest
  cache_digest=$(digest "$CACHE_DIR/${candidate_digest}.bin")
  if [ "$cache_digest" != "$candidate_digest" ]; then
    echo "ERROR: cache copy failed to match digest" >&2
    return 1
  fi

  # Atomic swap to bebop.bin
  local tmp_file="${CURRENT_BIN}.tmp.$$"
  cp "$CACHE_DIR/${candidate_digest}.bin" "$tmp_file"

  local tmp_digest
  tmp_digest=$(digest "$tmp_file")
  if [ "$tmp_digest" != "$candidate_digest" ]; then
    rm -f "$tmp_file"
    echo "ERROR: temp file digest mismatch" >&2
    return 1
  fi

  mv "$tmp_file" "$CURRENT_BIN"

  # Update manifest
  cmd_save "promoted from candidate"

  # Print new id
  echo ""
  cmd_id
}

# Main dispatcher
main() {
  local cmd="${1:-}"

  case "$cmd" in
    id)
      cmd_id
      ;;
    list)
      cmd_list
      ;;
    save)
      shift
      cmd_save "$@"
      ;;
    use)
      shift
      cmd_use "$@"
      ;;
    promote)
      shift
      cmd_promote "$@"
      ;;
    *)
      echo "usage: tools/bebopc.sh <id|list|save|use|promote>" >&2
      exit 1
      ;;
  esac
}

main "$@"
