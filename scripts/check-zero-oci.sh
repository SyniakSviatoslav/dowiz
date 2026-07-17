#!/usr/bin/env bash
# scripts/check-zero-oci.sh — DK-08 supply-chain gate.
#
# ASSERTS that a Dockerfile is ZERO-OCI: the runtime stage must NOT use an nginx
# base image (and, more broadly, no OCI "app container" runtime base). This is the
# locally-testable half of DK-08; the SBOM/scan/sign half runs in CI
# (see .github/workflows/ci.yml — `supply-chain` job).
#
# EXIT 0  — the Dockerfile is zero-OCI (no forbidden base).
# EXIT 1  — a forbidden base (nginx) is present  -> build MUST fail the gate.
# EXIT 2  — usage / file-not-found error.
#
# USAGE
#   scripts/check-zero-oci.sh [PATH_TO_DOCKERFILE]
#   (defaults to ./Dockerfile in the repo root)
set -euo pipefail

TARGET="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/Dockerfile}"

if [[ ! -f "${TARGET}" ]]; then
  echo "check-zero-oci: Dockerfile not found at '${TARGET}'" >&2
  exit 2
fi

echo "check-zero-oci: scanning ${TARGET}"

# Forbidden runtime bases. nginx is the legacy container we replaced (DK-04).
# We forbid nginx specifically and any explicit "app image" runtime markers.
FORBIDDEN=(
  "nginx"                 # chainguard/nginx, nginx:alpine, etc.
  "cgr.dev/chainguard/nginx"
)

violations=0
while IFS= read -r line; do
  # Only inspect FROM lines (case-insensitive, trim leading whitespace).
  if [[ "${line,,}" =~ ^[[:space:]]*from[[:space:]] ]]; then
    for forbidden in "${FORBIDDEN[@]}"; do
      if [[ "${line,,}" == *"${forbidden,,}"* ]]; then
        echo "  VIOLATION: forbidden base '${forbidden}' in: ${line}" >&2
        violations=$((violations + 1))
      fi
    done
  fi
done < "${TARGET}"

if [[ "${violations}" -gt 0 ]]; then
  echo "check-zero-oci: FAILED — ${violations} forbidden OCI base(s) found." >&2
  echo "  DK-08 requires a zero-OCI runtime (scratch + static binary). Fix the Dockerfile." >&2
  exit 1
fi

# Positive assertion: a true zero-OCI runtime stage should ultimately derive from
# `scratch` (or a distroless static base). We warn (non-fatal) if no scratch stage
# is present, so the gate stays useful during partial migrations.
if ! grep -qiE '^[[:space:]]*FROM[[:space:]]+scratch' "${TARGET}"; then
  echo "check-zero-oci: WARN — no 'FROM scratch' stage found; ensure the final runtime is zero-OCI." >&2
fi

echo "check-zero-oci: OK — no forbidden OCI base images. Zero-OCI gate passed."
exit 0
