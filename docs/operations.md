# Operations

Deploying, watching production, the crons, and what to do when something breaks, including on the
Android development box.

## Deploy

One Worker serves every venue and the platform; its configuration is `workers/api/wrangler.toml`.

```sh
cd workers/api
export PATH="$HOME/.cargo/bin:$PATH"   # wrangler runs `worker-build --release` through /bin/sh
set -a && . /path/to/deploy-token-env && set +a   # CLOUDFLARE_API_TOKEN with Workers Scripts write
npx --yes wrangler@4 deploy
bash scripts/smoke.sh https://sushi-durres.dowiz.org
```

- **Use the token that can deploy.** A token without Workers Scripts write builds the whole crate
  and only then fails with `Authentication error [code: 10000]`, which reads like a broken token.
- **On the development box** the wasm32 standard library lives in the rustup toolchain, not the
  distro compiler: `export RUSTUP_TOOLCHAIN=1.96.1-aarch64-unknown-linux-gnu` before deploying or
  running any `cargo check --target wasm32-unknown-unknown`.
- **Verify a deploy by reading it back**, not by the command's exit code: fetch the deployed script
  and look for a string unique to the change, and run `scripts/smoke.sh` and
  `tools/live-checks/health.sh`.
- `scripts/smoke.sh` is the once-per-deploy smoke. It also sends bodies at two deleted legacy write
  routes to prove they stay deleted, which is why it is not the thing that runs every 15 minutes.

### A new venue

1. `POST /api/platform/hubs` as the platform operator (creates the venue's object and owner).
2. `bash tools/platform/attach-host.sh <slug>` attaches `<slug>.dowiz.org` as a Workers custom
   domain; the host answers after a minute or two. This step goes away when the deploy token gains
   `Workers Routes:Edit` and the wildcard route returns to `wrangler.toml` (roadmap F19).

## Watch

| What | Where | How often |
|---|---|---|
| Production probes, read-only | `.github/workflows/health-cron.yml` running `tools/live-checks/health.sh` | every 15 minutes |
| Each role through the real UI | `.github/workflows/key-flows.yml` running `e2e/walk/` against `WALK_HOST` | daily, QA host only |
| A venue's own gauges | `GET /api/owner/health` (console: More, Health): image sizes against their ceilings, outbox depth and oldest entry, recent errors | on demand |
| The register | `node e2e/gates/conservation.mjs` with an owner's credentials | on demand, read-only |

`health.sh` checks, per venue: `/healthz` answers `ok`; the storefront is HTML with a
`content-security-policy` header; the menu has more than zero dishes; the kernel prices an ETA;
`GET /api/order/<made-up id>` is refused (401 or 404); `/admin/`, `/courier/`, `/room/` and the
manifest are served; and the platform landing and `/healthz`. Run it from anywhere:

```sh
bash tools/live-checks/health.sh
HEALTH_VENUES="sushi-durres" bash tools/live-checks/health.sh
```

When the workflow fails it writes the failing rows into the run summary and, if the repository has a
`TELEGRAM_BOT_TOKEN` secret and an `OPS_TELEGRAM_CHAT_ID` variable, sends them to the ops chat.

## Crons

From `workers/api/wrangler.toml` and `scheduled` in `workers/api/src/lib.rs`:

| Cron | Does |
|---|---|
| `17 3 * * *` | the nightly: each venue's off-site copy to its own bucket (with a manifest of each image's SHA-256), rotation (every copy for 7 days, weekly ones to 21 days), pruning, and the witness of each log's tip |
| `* * * * *` | drain every venue's outbox (Telegram, WhatsApp, campaigns), poll eBills for new till sales, and the fiscal sweep, which returns at once while `SEND_ENABLED = false` |

Each firing reads the clock once and passes the instant down.

## Runbooks

### A venue's kitchen is not being told about orders

1. Console, More, Health: look at the outbox depth and its oldest entry. A growing queue with an old
   head means the rail is failing; entries waiting with no failures mean the rail is not configured.
2. Console, More, Integrations: run the Telegram check. A venue-owned bot token that was rotated or
   removed shows here.
3. Entries are retried with backoff (10 s, 30 s, 2 min, 5 min, then 10 min) and abandoned after six
   tries, which the health pane reports. An order is never lost because its message failed.

### A page renders without styles, or a screen is blank

1. Open the browser console. A Content Security Policy violation from our own origin is the known
   cause (2026-09-16); the policy lives in `workers/api/public/_headers`.
2. A blank app offline usually means its service worker shell does not list a module the page
   imports (`sw-shell.sh` guards this once committed).
3. `node e2e/kit-regression/render.mjs` against the host finds both in a real browser.

### A deploy "succeeded" but nothing changed

Read the deployed script back and grep it for a string unique to the change. Check that the deploy
used the token with Workers Scripts write and that `worker-build` was on `PATH`.

### Restore a venue

`GET /api/owner/backup` downloads the venue's images; `POST /api/owner/restore` reads such a file
back. The nightly copies in the venue's bucket carry a manifest with each image's SHA-256; check the
hashes before restoring. Copies are kept 21 days: every nightly copy for 7 days, then one a week
until day 21 (`KEEP_ALL_MS` / `KEEP_WEEKLY_MS` in `workers/api/src/cloud.rs`), so a copy made before
an erasure is gone within 21 days of it.

A restore through `/api/owner/restore` re-applies every erasure in the venue's erasure register by
itself before it answers; if any cannot be applied again it answers 500 instead of "restored".

### Point-in-time recovery (PITR) of a venue object

Cloudflare's point-in-time recovery of a Durable Object does not pass through the Worker, so the
erasure replay does not run by itself. After any Cloudflare point-in-time recovery of a venue
object, call `POST /api/owner/customers/reforget` as that venue's owner; expect
`{"reforgotten":{"entries":N,...,"failed":0}}`. A 500 names the erasures that failed; rerun after
fixing, it is idempotent.

### An order cannot be ended

An order past `PENDING` ends through a refund (`REFUNDING` to `COMPENSATED_REFUND`); a delivery
refused at the door ends the courier's run the same way. `DELIVERED` and `PICKED_UP` are final by
design. `e2e/kit-regression/drain-stuck-orders.mjs` is the caretaker's tool for test orders.

### The fiscal sender

It is switched off in code (`workers/api/src/fiscal/mod.rs`, `SEND_ENABLED = false`). Do not change
that without a new, explicit decision from the owner: every invoice at the tax authority is a legal
record, and a sale the venue's own till already fiscalised must never be sent again.

## The development box

The main development machine is an Android phone running Termux with a proot Linux. Android's
**phantom-process killer** ends the whole session (every shell, every agent, exit 137 or signal 9)
when the app holds more than 32 processes. It is not the OOM killer; memory is usually fine.

**Prevent it:**

- Every cargo command goes through the slot, in the foreground; it blocks until the slot is free:
  `bash bebop-lang/tools/slot.sh <label> cargo test --lib <filter> > out.txt 2>&1; echo rc=$?`
- `~/.cargo/config.toml` holds `jobs = 2` and `RUST_TEST_THREADS = "2"`. Cargo's default of one job
  per core alone crosses 32. Never pass `-j`, `--jobs` or `--test-threads` above 2.
- No background jobs, no wait loops (`until ...; sleep`), no `kill` or `pkill` (matching your own
  shell's arguments kills your own shell).
- Browser scripts in `e2e/walk/` refuse to launch Chromium above 26 processes and run it
  single-process.

**When it fires anyway:**

1. Reopen Termux and the session. Nothing on disk is lost; uncommitted work is still in the tree.
2. `git status --short` and read what the dead session left: half-written files, a gate disabled for a
   mutation proof (`grep -rn "false &&\|if false" <files>`), orphan shells.
3. Count processes before starting again: `ls /proc | grep -c '^[0-9]'`.
4. Re-run the last step that was in flight in the foreground, through the slot.

**The permanent fix is on the phone** (the owner's to make): Developer options, "Disable child
process restrictions" (Android 14 and later), and Termux battery usage set to Unrestricted. Run
`termux-wake-lock` at the start of a long session. Background and measurements:
`bebop-lang/docs/BOX.md`.
