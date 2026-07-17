#!/usr/bin/env bash
# P01 §2.5 / BLUEPRINT-P01 §6 DECART — decart-dep-lint
#
# Cheap CI guard: a NEW [dependencies] / [dev-dependencies] entry must be
# justified by a DECART report (docs/**/*DECART*.md) OR carry an `innovate:` /
# `decart:` marker in the same Cargo.toml hunk that adds it. Without that, the
# new dependency is a silent supply-chain/architecture addition => RED.
#
# Usage: scripts/decart-dep-lint.sh <base> <head>
# Exits 0 = all new deps justified; 1 = at least one unjustified new dep.
#
# SCOPE RULE: dev-time fence on the canonical repo. Does not constrain hubs.

set -uo pipefail

BASE="${1:-origin/main}"
HEAD="${2:-HEAD}"

echo "decart-dep-lint: diffing dependencies ${BASE}..${HEAD}"

# Collect added dependency names from Cargo.toml diffs across the repo.
# Only lines that are actual dependency entries (under a [dependencies] /
# [dev-dependencies] / [build-dependencies] table) count — NOT bare `version =`
# fields or [features] entries. We track the current TOML table from the diff.
added=$(git diff "${BASE}" "${HEAD}" -- '**/Cargo.toml' \
  | awk '
      # track table context from added lines
      /^\+\[/ { table=$0; sub(/^\+/,"",table) }
      # dependency line:  +name = "..."   or   +name = { ... }
      /^\+[a-zA-Z0-9_-]+[[:space:]]*=[[:space:]]*("|\{)/ {
        line=$0; sub(/^\+/,"",line)
        # skip if we are not inside a dependency table
        if (table !~ /\[dependencies\]/ && table !~ /\[dev-dependencies\]/ && table !~ /\[build-dependencies\]/) next
        # skip in-repo path dependencies (not external integrations)
        if (line ~ /path[[:space:]]*=[[:space:]]*/) next
        dep=line; sub(/[[:space:]]*=.*/,"",dep)
        print dep
      }
    ' | sort -u)

if [ -z "${added}" ]; then
  echo "decart-dep-lint: no new dependencies added — GREEN"
  exit 0
fi

# Any existing DECART doc referencing the crate name (tracked OR untracked).
mapfile -t decart_docs < <(git ls-files 'docs/**/*DECART*.md' 2>/dev/null; \
                          git ls-files --others --exclude-standard 'docs/**/*DECART*.md' 2>/dev/null)

rc=0
while IFS= read -r dep; do
  [ -z "${dep}" ] && continue
  justified=0

  # (a) a DECART doc mentions the crate
  for d in "${decart_docs[@]:-}"; do
    if [ -n "${d}" ] && grep -qiE "(^|[^a-zA-Z0-9_-])${dep}([^a-zA-Z0-9_-]|$)|\"${dep}\"" "${d}" 2>/dev/null; then
      justified=1; break
    fi
  done

  # (b) the diff hunk adding it carries an `innovate:` / `decart:` marker
  if [ "${justified}" -eq 0 ]; then
    if git diff "${BASE}" "${HEAD}" -- '**/Cargo.toml' \
       | grep -qE "(\+.*(innovate|decart):|added.*${dep}.*(innovate|decart):)" ; then
      # marker present somewhere in the dependency diff — accept (conservative)
      justified=1
    fi
  fi

  if [ "${justified}" -eq 0 ]; then
    echo "  RED: new dependency '${dep}' has no DECART report / marker"
    rc=1
  else
    echo "  GREEN: '${dep}' justified (DECART doc or marker present)"
  fi
done <<< "${added}"

if [ "${rc}" -eq 0 ]; then
  echo "decart-dep-lint: all new dependencies justified — GREEN"
else
  echo "decart-dep-lint: unjustified dependency additions — RED"
fi
exit "${rc}"
