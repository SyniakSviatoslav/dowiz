# Operator Runbook — Running a Live dowiz Hub Node

> For a human operating a real Hetzner node without an AI agent's help. Every command below is
> verified against the actual scripts/units in `deploy/` — nothing aspirational. Written 2026-07-20
> in response to an audit finding that no such runbook existed (see
> `docs/design/DOWIZ-STRATEGIC-REGRET-MINIMIZATION-SYNTHESIS-2026-07-20.md` §2 item 7).

## What's actually running on a node today

Two systemd-managed native binaries (no Docker — see `deploy/check-no-docker.sh`):

- **`native-spa-server`** — the zero-OCI static+order HTTP server (`deploy/native-spa-server.service`).
  This is the one product-facing process. Binds `0.0.0.0:8080` by default.
- **`pgrust`** — the native Postgres-in-Rust process, if this node runs the database tier
  (`deploy/pgrust.service`). Not every node needs this.

Plus two maintenance timers (`deploy/deep-clean.timer`, `deploy/deep-clean-audit.timer`) that run
periodic housekeeping — check `systemctl list-timers` to confirm they're active.

## Checking a node's health

```sh
systemctl status native-spa-server        # is the process up?
curl -fsS http://127.0.0.1:8080/healthz   # does it answer? (static string today, no build-sha)
journalctl -u native-spa-server -n 200    # recent logs
```

**Known limitation (see the strategic audit, report F):** `/healthz` returns a static string with
no build identifier. To find out what's actually running, don't trust the service — check the
release symlink directly:

```sh
readlink -f /var/lib/deliveryos/releases/current   # the git sha actually deployed
```

There is currently no live alerting beyond the Telegram heartbeat (`.github/workflows/heartbeat-monitor.yml`,
polls `https://webhook.dowiz.org/` every 10 min) — that answers "is it up," not "is it correct."
Don't assume silence means healthy; it means the process is responding, nothing more.

## Deploying a new version

```sh
./deploy/deploy-symlink.sh <git-sha>
```

This builds, runs a local health-check on a throwaway port (refuses the swap if the new build is
broken — production stays untouched on failure), does an atomic `ln -sfn` symlink swap (one
`rename()`, no half-deployed state), then watches for 30s and **auto-rolls-back** if ≥3 health
checks fail in that window. There is deliberately **no CI/CD auto-deploy** — every deploy is a
human running this command. (`D5-F2` lesson, recorded in the script's own header: never
reintroduce auto-deploy touching live state without an explicit human invocation.)

## Rolling back

```sh
./deploy/rollback.sh <prev-git-sha>          # e.g. the sha shown by `readlink -f .../current` before your last deploy
```

One command, runnable over SSH from a phone. Re-points the symlink, restarts the service, verifies
`/healthz` responds. Refuses if the target release directory doesn't exist locally (you can't roll
back to something never deployed to this box) — check `ls /var/lib/deliveryos/releases/` for what's
actually available.

**What this does NOT roll back:** any data/schema change. There is no down-migration engine and
no clean undo for a bad data mutation (see the strategic audit, report G) — binary rollback and
data rollback are two different problems, and only the first one has real tooling today.

## Enrolling this node to actually accept orders

**Important:** the production server boots with an **empty capability roster** by design — it
rejects every `/api/*` request (401/403) until real anchor certificates are enrolled. A freshly
deployed node that returns 401 on every order request is working correctly, not broken — it has
simply never been told which capability certs to trust. Real anchor enrollment tooling doesn't
exist yet (see task backlog); until it does, don't expect a fresh node to admit orders.

## If something's wrong and you're not sure why

1. `journalctl -u native-spa-server -n 500` — start here.
2. Check the FDR ring if the kernel/telemetry feature is enabled: it's currently **write-only**
   (nothing reads it live) — recovering it requires the `fdr_recorder recover` forensic tool
   run manually, not a dashboard. This is a known gap, not a missing step you forgot.
3. `./deploy/rollback.sh <last-known-good-sha>` is almost always safer than debugging in place on
   a production box — roll back first, investigate the old build's logs after.
4. Check `docs/ops/` for incident-specific runbooks (`P1-PAUSE-SECRET-PUSH-RUNBOOK.md`,
   `P7-DECIDE-*`, `P8-*`) if the issue matches one of those named scenarios.

## What this runbook does NOT cover (known gaps, not oversights)

- Staging environment — none currently exists (Fly.io, the prior staging host, was fully retired
  2026-07-18; there is no replacement staging tier yet).
- Cross-mesh backup — every node's durability is currently its own disk plus one vendor-key-scoped
  offsite envelope; there's no second node holding a live copy of this one's data.
- Multi-node version-skew — no wire-format version negotiation exists outside the crypto-suite
  layer; don't assume two nodes on different kernel versions are compatible without checking.
