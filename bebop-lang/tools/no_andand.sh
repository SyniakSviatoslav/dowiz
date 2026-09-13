#!/usr/bin/env bash
# tools/no_andand.sh (2026-09-13): gate that catches live && operators in .bp files
# The && operator in Bebop always evaluates to 0 and binds tighter than comparison,
# causing subtle logic bugs. This gate scans all .bp files for live && (outside comments),
# with an allowlist for the deliberate construct test.
#
# Exit codes: 0 if no live && found, non-zero if violations exist
# Usage: tools/no_andand.sh

cd "$(dirname "$0")/.." || exit 1

violations=0

# Scan all .bp files, stripping comments from // to EOL before checking
while IFS= read -r file; do
  # Normalize path (remove leading ./)
  normalized_file="${file#./}"

  # Skip the allowlisted file: bench/parity_constructs/c46_andor.bp
  # Reason: T125 deliberately pins && / || language-behaviour constructs
  if [ "$normalized_file" = "bench/parity_constructs/c46_andor.bp" ]; then
    continue
  fi

  # Blank string literals FIRST, then strip comments (// to EOL), then look for &&.
  # Order matters: a line like `let s = "a // b"; let z = x && y;` would otherwise have
  # its real `&&` hidden, because the `//` inside the STRING starts the comment strip.
  # Measured false negative 2026-09-13 before this was added.
  if sed 's/"[^"]*"/""/g; s|//.*||g' "$file" | grep -q '&&'; then
    echo "FAIL $normalized_file: found live &&"
    sed 's/"[^"]*"/""/g; s|//.*||g' "$file" | grep -n '&&' | sed 's/^/  /'
    violations=$((violations + 1))
  fi
done < <(find . -type f -name "*.bp" | sort)

if [ $violations -eq 0 ]; then
  echo "no_andand: PASS (0 live && found in non-allowlisted files)"
  exit 0
else
  echo "no_andand: FAIL ($violations files contain live &&)"
  exit 1
fi
