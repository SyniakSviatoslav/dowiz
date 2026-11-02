#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# cell-usize: NO CELL REACHES usize BY `as` (W-MG-HOLES2, W-MG-ATK3).
#
# A cell is any i64. `as usize` keeps its low 32 bits on wasm32, so the reader there followed a ref,
# a count or a length every other reader called outside the image, and answered status=0. The one
# door is einfile_image::{idx, idx_end, idx_u}, over usize::try_from.
#
# Fails on a line of the production sources of einfile-image, einfile-log or einfile-kv where the value of
# a cell read, or a clamp of one, goes through `as usize`. Tests are outside it: they name offsets.
# `CELL_USIZE_ROOT` moves the root, for the proof.
set -u
ROOT="${CELL_USIZE_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
PAT='(\.get\(|get_in\(|cell_at\(|\.cell\(|obj_len(_in)?\()[^;]*\) as usize|cells\[[^;]*\] as usize|\.max\(0\) as usize|(cursor|mark)[^;]* as usize'
hits=$(
  for c in einfile-image einfile-log einfile-kv; do
    find "$ROOT/crates/$c/src" -name '*.rs' ! -name '*tests.rs' ! -path '*/tests/*'
  done | sort | while read -r f; do
    sed '/#\[cfg(test)\]/,$d' "$f" | grep -nE "$PAT" | grep -vE '^[0-9]+:\s*//' | sed "s|^|$f:|"
  done
)
if [ -n "$hits" ]; then
  echo "$hits"
  n=$(printf '%s\n' "$hits" | wc -l | tr -d ' ')
  echo "cell-usize: RED $n cell(s) reach usize by \`as\`: use einfile_image::idx"
  exit 1
fi
echo "cell-usize: ZERO. Every cell reaches usize through einfile_image::idx."
