# Local-dev secrets — SOPS + age

Git-versioned **encrypted** secrets you decrypt locally with your own key. No raw `.env` passed around,
no external vendor, no infra to run. **Prod/staging runtime still read from Fly Secrets** — this is the
local-dev source of truth (and an optional upstream you can push to Fly).

## One-time setup (per dev)

1. Install tooling:
   - `age` + `age-keygen` — `apt install age` (Debian/Ubuntu) or `brew install age`.
   - `sops` — `brew install sops`, or the binary from https://github.com/getsops/sops/releases.
2. Generate your key (kept OUT of the repo):
   ```bash
   mkdir -p ~/.config/sops/age
   age-keygen -o ~/.config/sops/age/keys.txt      # prints your PUBLIC key (age1…)
   ```
3. Add your **public** key to `.sops.yaml` (`age:` list), then a current key-holder re-encrypts so you
   can decrypt:
   ```bash
   sops updatekeys secrets/staging.enc.env
   ```

## Daily use

```bash
# Load decrypted secrets into the current shell (never written to disk):
source scripts/secrets-env.sh staging

# Edit secrets (opens $EDITOR with plaintext, re-encrypts on save):
sops secrets/staging.enc.env
```

## Creating / updating the encrypted bundle

```bash
cp secrets/staging.env.example secrets/staging.env   # fill REAL values (gitignored)
sops -e --input-type dotenv --output-type dotenv secrets/staging.env > secrets/staging.enc.env
rm secrets/staging.env                               # only the .enc.env is committed
```

## Optional: make this the single source, push to Fly

```bash
sops -d secrets/staging.enc.env | flyctl secrets import -a dowiz-staging
```

## Rules

- **Commit only** `*.enc.env` (ciphertext values) + `*.env.example` (no values). The repo `.gitignore`
  blocks plaintext `secrets/*.env` and age private keys — never override that.
- Your age **private** key (`~/.config/sops/age/keys.txt`) never leaves your machine.
- Rotating a secret = `sops secrets/<env>.enc.env`, change the value, commit. Removing a dev = drop their
  public key from `.sops.yaml` + `sops updatekeys` + rotate anything they could have read.
- These are **internal**-classed (G5 env-classification) — SOPS+age add no external subprocessor.
