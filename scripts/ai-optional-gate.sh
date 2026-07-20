#!/usr/bin/env bash
# Roadmap item 45 (SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19 §I) — ai-optional-gate.
# See docs/design/BLUEPRINT-ITEM-45-ai-optional-gate-2026-07-19.md.
#
# Enforces the AI-optional compile-time invariant ("the whole system must be able to
# run without AI") on TWO planes:
#
#   (A) BUILD plane — the default-features build (AI absent) MUST compile AND pass the
#       FULL kernel test suite (re-execute, never presence-check — CHECKLIST §10-P7; the
#       same discipline as the zero-dep-gate / hardening-gate jobs). This proves the
#       canonical order/money core is genuinely AI-free today and stays so.
#   (B) DEPENDENCY-DIRECTION plane — no core decision module may reference the AI module
#       paths (reserved prefix `crate::inference`, plus the explicit `inference`/`ai`
#       module-name set) OUTSIDE a `#[cfg(feature = "inference")]` block. AI depends on
#       core; core NEVER depends on AI. This is the INTRA-kernel firewall (distinct from
#       the P40 inter-crate agent-loop firewall) — the same "structurally cannot name X"
#       discipline. Today (AI absent) it is a pure "no core→AI reference" grep; when the
#       items-33–44 `inference` subsystem lands behind `#[cfg(feature = "inference")]`,
#       part (B) is additionally backed by name-resolution failure (Option B) — belt and
#       suspenders. The deterministic-math organs (`attention`, `micrograd`, `online`)
#       are EXPLICITLY excluded (non-AI per attention.rs:17–20); gating them would
#       false-positive the entire current tree.
#
#   (C) DEFAULT-GRAPH proof — `cargo tree -p dowiz-kernel -e no-dev` resolves AI-free
#       (the arc lands with the zero-dep allowlist still empty; same gate shape as items
#       1+13/§H build items).
#
# FAIL-CLOSED (P7 anti-forgery, same class as hardening-gate): every verdict is a live
# cargo exit code or a grep that would catch a planted reference. A zero-match "all
# clear" is GREEN only because the tree is genuinely AI-free — proven by the planted-
# import RED demonstration recorded in the item-45 PR body (§7: plant a
# `use crate::inference::…` line in markov.rs → part (B) RED; clean tree → gate green).
#
# Determinism (P6): every cargo invocation is --locked --offline; the kernel Cargo.lock
# hash is asserted byte-identical before vs after the run, exactly as zero-dep-gate.sh.
#
# SCOPE RULE: one crate's DEFAULT build (no arg => kernel, backward-compatible). The AI
# module path set is operator-decided (blueprint §10 OPERATOR-DECISION); the defaults
# below track the reserved prefix recorded in the blueprint.
set -euo pipefail

CRATE="${1:-kernel}"
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT" || exit 2
LOCK="$CRATE/Cargo.lock"

# ── (C/P6) lockfile hash BEFORE — the gate must not mutate resolution state ──
h0=$(sha256sum "$LOCK" | cut -d' ' -f1)

# The seven core decision modules named by item 45 (blueprint §I item 45 + §5 step 3b).
# A reference to the reserved AI module path in ANY of these (outside the feature gate)
# is a firewall breach.
CORE_MODS=(
  "kernel/src/order_machine.rs"
  "kernel/src/decision/"
  "kernel/src/hydra.rs"
  "kernel/src/event_log.rs"
  "kernel/src/markov.rs"
  "kernel/src/spectral.rs"
  "kernel/src/fdr/"
)

# The forbidden-in-core AI module path set (blueprint §10 OPERATOR-DECISION, provisional):
#   - the reserved single-root prefix `crate::inference` (mirrors the `pq` module shape)
#   - the bare `inference` module-name token in any path position (`::inference`,
#     `inference::Model`, `use … inference`)
# Explicitly EXCLUDED: attention / micrograd / online (deterministic-math organs, non-AI
# per attention.rs:17–20) and any identifier merely CONTAINING the token (e.g. the matrix
# variable `aik`, `aid`) — the non-word-char anchors below prevent those false positives.
# NOTE: word boundaries use the POSIX class `[^[:alnum:]_]` (and `^`/`$`), NOT `\b` — gawk
# turns a `\b` passed via `awk -v re=…` into a literal backspace, which would silently
# match NOTHING (the false-GREEN trap). This form is gawk-portable and exact.
AI_PATH_RE='(^|[^[:alnum:]_])inference([^[:alnum:]_]|$)'

# (B) DEPENDENCY-DIRECTION firewall — per core module.
# A reference is permitted ONLY when it sits inside a `#[cfg(feature = "inference")]`
# block. We handle this with a stateful scan: a `#[cfg(feature = "inference")]` attribute
# arms the skip; once the attribute closes (its `]`), the IMMEDIATELY FOLLOWING line (the
# gated item) is also exempt, then the skip disarms. This matches Rust's attribute grammar
# (the attribute precedes the item it gates) and covers both single-line and multi-line
# cfg attributes. The blueprint's Option-A shape is the lexical check; this stateful filter
# is the precise, still-grep-class form that exempts legitimately gated references (for when
# the subsystem lands, Option B — compile-backed name-resolution failure).
echo "=== ai-optional-gate: dependency-direction firewall (part B) ==="
rc=0
for m in "${CORE_MODS[@]}"; do
  if [ ! -e "$m" ]; then
    echo "  SKIP (absent): $m"
    continue
  fi
  # Expand directories to their .rs files (awk cannot read a directory).
  if [ -d "$m" ]; then
    mapfile -t files < <(find "$m" -name '*.rs' | sort)
  else
    files=( "$m" )
  fi
  # Collect offending lines (file:line:content). State machine over the module source so
  # references inside `#[cfg(feature = "inference")]` are exempt (the future Option-B
  # compile-backed subsystem is allowed to be named there).
  breaches=$(awk -v re="$AI_PATH_RE" '
    {
      line = $0
      # arm the skip when an inference feature-gate attribute opens
      if (line ~ /#\[cfg\(feature[ ]*=[ ]*"inference"/) { skip = 1 }
      if (skip) {
        # attribute closes on this line -> the NEXT line is the gated item (still exempt)
        if (line ~ /\]/) { skip = 0; gated_next = 1 }
        next
      }
      if (gated_next) { gated_next = 0; next }   # the gated item line itself is exempt
      if (line ~ re) { printf "%s:%d:%s\n", FILENAME, FNR, line }
    }
  ' "${files[@]}" || true)
  if [ -n "$breaches" ]; then
    while IFS= read -r b; do
      echo "::error::AI-optional breach: core module references reserved AI path — $b"
    done <<< "$breaches"
    echo "  RED: $m references the AI module outside the feature gate" >&2
    rc=1
  else
    echo "  OK: $m references no AI module path outside the feature gate"
  fi
done

# (A) BUILD plane — default-features (AI absent) compile + FULL kernel suite.
# Re-execute, never presence-check: a live `cargo test` exit code. The default build has
# the `inference` feature OFF (it does not exist yet — it is items 33–44's deliverable),
# so this proves the canonical core is AI-free and fully green without it.
echo "=== ai-optional-gate: default-features full kernel suite (part A) ==="
set +e
cargo_test_out=$(cd "$CRATE" && cargo test --locked --offline 2>&1)
cargo_rc=$?
set -e
if [ "$cargo_rc" -ne 0 ]; then
  echo "::error::ai-optional-gate BUILD plane RED: default-features kernel suite FAILED (exit $cargo_rc)"
  printf '%s\n' "$cargo_test_out" | grep -E 'error\[|error:|FAILED|panicked|test result: FAILED' | head -20 | sed 's/^/    /'
  rc=1
else
  passed=$(printf '%s\n' "$cargo_test_out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END{print s+0}')
  echo "  OK: default-features kernel suite passed ($passed passed, AI absent)"
fi

# (C) DEFAULT-GRAPH proof — cargo tree -e no-dev resolves AI-free.
# Matches the items-1+13/§H build-plane law: the inference subsystem, when it lands,
# rides a non-default feature, so the DEFAULT no-dev tree carries zero AI/crate deps.
echo "=== ai-optional-gate: default-graph AI-free proof (part C) ==="
cargo tree --manifest-path "$CRATE/Cargo.toml" -e no-dev --locked --offline --prefix none 2>/dev/null \
  | { grep -iE '(^|/)inference|/ai-' || true; } > /tmp/aiog-tree.txt
if [ -s /tmp/aiog-tree.txt ]; then
  echo "::error::ai-optional-gate DEFAULT GRAPH RED: 'inference'/'ai' present in default no-dev tree:"
  sed 's/^/    /' /tmp/aiog-tree.txt
  rc=1
else
  echo "  OK: default no-dev dependency tree is AI-free (inference absent)"
fi

# (P6) lockfile hash AFTER must be unchanged (verdict is a function of the repo only).
h1=$(sha256sum "$LOCK" | cut -d' ' -f1)
if [ "$h0" != "$h1" ]; then
  echo "::error::ai-optional-gate P6 violation: Cargo.lock changed during the gate run ($h0 -> $h1)"
  rc=1
fi

if [ "$rc" -eq 0 ]; then
  echo "=== ai-optional-gate: GREEN (build + dependency-direction firewall + default-graph all pass; lockfile stable $h0) ==="
else
  echo "=== ai-optional-gate: RED ==="
fi
exit "$rc"
