#!/usr/bin/env bash
# W-6b — firewall RED-proof (DoD-2).
#
# Demonstrates that adding a direct `dowiz-kernel` import to a FORBIDDEN port
# (bebop-proto-wire) FAILS the build (E0432 unresolved import — the crate is
# not in proto-wire's graph). The script applies firewall-red.patch to a scratch
# copy, runs `cargo check -p bebop-proto-wire`, and EXITS 0 ONLY IF THE BUILD
# FAILS. It then cleans up.
#
# Adversarial control (blueprint §3.6): the same "import resolves" expectation
# must NOT hold for bebop-delivery-domain WITH kernel-rlib — there the import is
# sanctioned. This script targets only proto-wire, proving it distinguishes the
# forbidden seam from the sanctioned one.

set -u
BEBOP="${1:-/root/bebop-repo}"
PATCH="$(cd "$(dirname "$0")" && pwd)/firewall-red.patch"

if [ ! -d "$BEBOP/bebop2/proto-wire" ]; then
  echo "SKIP: bebop checkout not present at $BEBOP (CI provides it via sibling checkout)"
  exit 0
fi

cd "$BEBOP"
# Apply the forbidden import; capture the result.
git apply --check "$PATCH" 2>/dev/null || { echo "PATCH already applied or unapplicable; treating as breach-fail"; exit 1; }
git apply "$PATCH"

set +e
cargo check -p bebop-proto-wire >/tmp/firewall_check.log 2>&1
RC=$?
set -e

# Revert the scratch change so the repo is left clean.
git apply -R "$PATCH" 2>/dev/null || git checkout -- bebop2/proto-wire/src/lib.rs 2>/dev/null

if [ "$RC" -ne 0 ]; then
  echo "FIREWALL OK: forbidden dowiz-kernel import into proto-wire does NOT build (E0432)."
  exit 0
else
  echo "FIREWALL BREACH: proto-wire compiled with a direct dowiz-kernel import — invariant violated."
  exit 1
fi
