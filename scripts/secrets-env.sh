#!/usr/bin/env bash
# Load SOPS-encrypted local-dev secrets into the current shell. See secrets/README.md.
#
#   source scripts/secrets-env.sh [env]      # default env = staging
#
# Decrypts secrets/<env>.enc.env (age key from ~/.config/sops/age/keys.txt or $SOPS_AGE_KEY_FILE)
# and exports every key=value into the environment. Never writes plaintext to disk.
set -euo pipefail

env="${1:-staging}"
file="$(git rev-parse --show-toplevel 2>/dev/null || echo .)/secrets/${env}.enc.env"

if ! command -v sops >/dev/null 2>&1; then
  echo "sops not installed — see secrets/README.md (install sops + age)." >&2
  return 1 2>/dev/null || exit 1
fi
if [ ! -f "$file" ]; then
  echo "no encrypted secrets at $file (env='$env')." >&2
  return 1 2>/dev/null || exit 1
fi

# `set -a` so each line of the decrypted dotenv is exported; process-substitution keeps plaintext off disk.
set -a
# shellcheck disable=SC1090
source <(sops -d "$file")
set +a
echo "loaded $(env_keys=$(sops -d "$file" | grep -cE '^[A-Z]') ; echo "$env_keys") secrets from ${env}.enc.env into the shell."
