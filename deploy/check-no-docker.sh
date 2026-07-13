#!/usr/bin/env bash
# check-no-docker.sh — DK-05 RED gate.
#
# Asserts that the pgrust systemd unit does NOT invoke a container runtime
# (docker / podman / nerdctl) in its ExecStart. pgrust MUST run as a native
# systemd process, never as a container. Exits non-zero (failing CI) if any
# forbidden runtime name appears in ExecStart.
#
# Usage: ./deploy/check-no-docker.sh [path-to-pgrust.service]
set -euo pipefail

UNIT="${1:-$(dirname "$0")/pgrust.service}"

if [[ ! -f "$UNIT" ]]; then
  echo "check-no-docker: unit file not found: $UNIT" >&2
  exit 2
fi

# Extract the ExecStart line(s) and scan for container-runtime names.
# tolerate spaces around '='; only inspect lines beginning with ExecStart.
forbidden=("docker" "podman" "nerdctl" "containerd" "ctr")
violation=0

while IFS= read -r line; do
  # Strip leading whitespace; only look at ExecStart*= lines.
  trimmed="${line#"${line%%[![:space:]]*}"}"
  case "$trimmed" in
    ExecStart*)
      for name in "${forbidden[@]}"; do
        if echo "$trimmed" | grep -q -E "(^|[^[:alnum:]])${name}([^[:alnum:]]|$)"; then
          echo "check-no-docker: FORBIDDEN container runtime '$name' in: $trimmed" >&2
          violation=1
        fi
      done
      ;;
  esac
done < "$UNIT"

if [[ "$violation" -ne 0 ]]; then
  echo "check-no-docker: FAIL — pgrust unit references a container runtime. DK-05 requires native execution." >&2
  exit 1
fi

echo "check-no-docker: PASS — pgrust.service ExecStart references no container runtime."
exit 0
