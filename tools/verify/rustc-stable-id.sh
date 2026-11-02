#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# RUSTC_WRAPPER for tools/verify/build.sh: `-C metadata=<cargo's hash>` becomes `metadata=<crate name>`.
#
# WHY (W-MG-REPRO, 2026-11-01). The reader's four path dependencies live outside its one-crate
# workspace, so cargo's SourceId hash for them is taken over the ABSOLUTE path, and that hash is
# `-C metadata`. rustc makes the crate's StableCrateId from it, the id goes into every symbol hash,
# and symbol order decides what LLVM and the linker keep: same source, same flags, a different
# function count per checkout. `--remap-path-prefix` cannot reach this: no path survives in the wasm
# (its data section is byte-identical), only the hash of one. Nothing else is rewritten here.
#
# The crate name is a sound replacement: the reader's graph has one crate of each name and one
# feature set, which is all `-C metadata` is there to keep apart.
name=
prev=
n=$#
while [ "$n" -gt 0 ]; do
  a=$1; shift; n=$((n - 1))
  [ "$prev" = --crate-name ] && name=$a
  case "$prev:$a" in
    -C:metadata=*) [ -n "$name" ] || { echo "rustc-stable-id: -C metadata before --crate-name" >&2; exit 2; }; a="metadata=$name" ;;
  esac
  set -- "$@" "$a"; prev=$a
done
exec "$@"
