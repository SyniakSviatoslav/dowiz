#!/usr/bin/env bash
# ITEM 53 (space-grade roadmap §J / BLUEPRINT-ITEM-53-lint-gate-2026-07-19.md):
# `lint-gate` — per-crate clippy + rustfmt contribution gate.
#
# NO workspace in dowiz (CLAUDE.md "Build model — CRITICAL"): we MUST `cd` into each
# crate dir and run `cargo fmt` / `cargo clippy` there. We NEVER invoke cargo at the
# repo root (the documented FALSE-GREEN trap — ci.yml `cargo-test` header: with no
# workspace, `--manifest-path` from root resolves the manifest but pulls the wrong
# target graph and can mask failures as exit-0).
#
# Crate roster is enumerated EXPLICITLY below (blueprint §3.1 — do not glob; the
# Repowise index is pre-"drop js" and the live tree is the source of truth). The
# roster mirrors `scripts/zero-dep-crates.txt` (the repo's authoritative per-crate
# list) and applies the SAME two exclusions (item 31 §2.5) — recorded, not silent:
#   - agent-governance-wasm : depends on bebop2-core by ABSOLUTE path
#     (/root/bebop-repo/bebop2/core), unresolvable on any CI runner layout.
#   - mesh-adapter          : relative ../../bebop-repo/... path deps resolve only in
#     the dual-checkout `mesh-adapter` CI job; linting here would RED every run on an
#     environmental resolution failure, not a lint.
#
# Gate logic (blueprint §3.2): for each crate, `( cd "$crate" && cargo fmt --check &&
# cargo clippy --all-targets -- --deny warnings )`. `--deny warnings` makes any clippy
# warning a hard failure; `fmt --check` makes any format divergence a hard failure.
# Aggregate non-zero exits => job RED.
#
# ADVISORY until required (item 53 §3.5, inherits item-14): this job is advisory until
# marked a required status check in branch protection (server-side, G5-owed). A merged
# workflow alone does NOT enforce it.
#
# ESCALATION TRIGGER (item 53 §7.1): the moment the operator authorizes public-flip
# *preparation* (ADR-0020's gate), item 53 jumps the queue to a pre-flip BLOCKER
# alongside the all-origin-refs gitleaks sweep. Until that trigger fires, LOW is the
# grounded priority.

set -uo pipefail

CRATES=(
  kernel
  engine
  wasm
  apps/courier
  agent-adapters
  agent-facade
  agent-loop
  llm-adapters
  tools/async-spool
  tools/ci-truth
  tools/deep-clean
  tools/eqc-rs
  tools/native-spa-server
  tools/nfc-pod-codec
  tools/nfc-pod-flipper
  tools/ops-alert
  tools/shell-spike
  tools/skillspector-rs
  tools/telemetry/hetzner-exporter
  tools/telemetry/native-ser
  tools/telemetry/native-trackers
  tools/telemetry/rust-spool
  tools/telemetry/swarm-proof
  tools/telemetry/topics
  # EXCLUDED: agent-governance-wasm (absolute-path bebop2 dep, CI-unresolvable)
  # EXCLUDED: mesh-adapter          (relative ../../bebop-repo deps, dual-checkout only)
)

rc=0
for crate in "${CRATES[@]}"; do
  if [ ! -f "$crate/Cargo.toml" ]; then
    echo "::error::lint-gate: crate dir '$crate' has no Cargo.toml (stale roster?)"
    rc=1
    continue
  fi
  echo "== lint-gate: $crate =="
  (
    cd "$crate"
    cargo fmt --check
    cargo clippy --all-targets -- --deny warnings
  ) || {
    echo "::error::lint-gate: $crate FAILED (clippy warning or fmt divergence)"
    rc=1
  }
done

if [ "$rc" -ne 0 ]; then
  echo "lint-gate: RED (one or more crates had clippy warnings or unformatted lines)"
  exit 1
fi
echo "lint-gate: GREEN (all crates fmt-clean + clippy-clean under --deny warnings)"
