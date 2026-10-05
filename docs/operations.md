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

### Live proof and alerting (W-LIVE, 2026-10-03)

**From GitHub, every probe sees only a challenge page today.** Measured with Cloudflare's
`firewallEventsAdaptive` for zone dowiz.org (token `/root/.cf_token`; the analytics token cannot read
zone events): every request from GitHub's runners (ASN 8075) gets `managed_challenge` from **Bot Fight
Mode**. That is why `health-cron` failed on every run from 2026-10-01T18:30Z to 2026-10-03T02:11Z
(8 of 8) while `health.sh` passes from the box. It is also why the old heartbeat was green: it read the
403 challenge as "up". A failing row in `health.sh` now says `CHALLENGED by Cloudflare (cf-mitigated: …)`
and quotes 80 bytes of the body.

**Bot Fight Mode stays on** (operator, 2026-10-03), so production is watched from inside Cloudflare:

| layer | what | how often | alert |
|---|---|---|---|
| 1 | **dowiz-watch** (`workers/watch/`, its own Worker, cron, SQLite Durable Object; no code shared with `workers/api`): platform `/healthz`, and per venue `/healthz`, the storefront with its CSP, the menu's dish count, the ETA quote -- the rows of `health.sh`. Up = expected status + body and no `cf-mitigated` | every 5 min | mail to the verified Email Routing address on every down/up transition |
| 2 | `heartbeat-monitor.yml` reads the watcher's `/healthz` (red when the last tick is older than 15 min) and `/status` (`ok:false` when any target is not up) at `https://dowiz-watch.sviatoslavsyniak.workers.dev`, outside the dowiz.org zone | every 10 min | Telegram if configured |
| 2b | `health-cron.yml` prints the watcher's full table into the run summary (decision: it reads `/status`; `health.sh` from GitHub only on a manual run with `direct: true`) | every 15 min | Telegram if configured |
| 3 | GitHub's own failure e-mail for a red run | per run | always |

```sh
curl -s https://dowiz-watch.sviatoslavsyniak.workers.dev/status     # per target: status, detail, since, last tick, watcher commit
curl -s https://dowiz-watch.sviatoslavsyniak.workers.dev/healthz    # "ok", or 503 "stale: ..."
```

Deploy (main): `cd workers/watch && . /root/.cf_deploy_token && npx wrangler@4 deploy --var WATCH_COMMIT:$(git rev-parse --short HEAD)`.
Free-plan cost: 288 cron invocations/day, 13 subrequests each (limit 50), about 290 Durable Object
requests and rows written per day (limits 100,000 each), and one mail per transition. A repo variable
`WATCH_URL` overrides the URL if the Worker ever moves.

How to see the challenges yourself:

```sh
. /root/.cf_token; curl -s -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" -H 'content-type: application/json' \
  https://api.cloudflare.com/client/v4/graphql --data '{"query":"{ viewer { zones(filter:{zoneTag:\"2929a252ceafe79f0bc52e9b9f0b92ee\"}) { firewallEventsAdaptive(limit: 20, filter:{datetime_gt:\"2026-10-03T00:00:00Z\", clientAsn:\"8075\"}, orderBy:[datetime_DESC]) { datetime action source clientRequestHTTPHost clientRequestPath userAgent } } } }"}'
```

Repository settings the alerting needs (Settings -> Secrets and variables -> Actions). None is set by
a lane:

| name | kind | used by | what |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | secret | health-cron, heartbeat-monitor, live-proof | the ops bot's token; without it the red run and GitHub's e-mail are the only alert |
| `OPS_TELEGRAM_CHAT_ID` | variable | health-cron, heartbeat-monitor (falls back to the old hard-coded chat), live-proof | the chat the alerts go to |

The per-link live proof (62 links, contracts in `tools/live-proof/contracts/`) is planned in
`docs/research/2026-10-03-live-proof-plan.md`. Its workflow secrets are listed there and get a row here
when the workflow lands.

### What the workflows read (owner step: GitHub, Settings, Secrets and variables, Actions)

`tools/gates/ci-refs.py` refuses a workflow that reads a secret or variable this table does not
name, or whose `run:` step names a file the repo does not have. Whether a run is GREEN on GitHub
is not readable from here (`gh` is not installed): that half is the Actions tab.

| Name | Kind | Read by | What |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | secret | health-cron, heartbeat-monitor | the ops bot that reports a failing probe |
| `OPS_TELEGRAM_CHAT_ID` | variable | health-cron | the ops chat |
| `WATCH_URL` | variable | health-cron, heartbeat-monitor | overrides the dowiz-watch URL (default its workers.dev host) |
| `HEALTH_VENUES` | variable | health-cron | venues the probes visit (default: all live) |
| `WALK_HOST` | secret | key-flows | the QA host the role walks run against (never production) |
| `WALK_LOC` | secret | key-flows | that host's venue id |
| `WALK_OWNER_EMAIL`, `WALK_OWNER_PASSWORD` | secret | key-flows | the QA venue's owner |
| `WALK_COURIER_PHONE`, `WALK_COURIER_PASSWORD` | secret | key-flows | the QA venue's courier |
| `EVALS_OWNER_EMAIL`, `EVALS_OWNER_PASSWORD` | secret | evals-nightly | an owner of the evaluated venue |
| `CF_ANALYTICS_ACCOUNT_ID`, `CF_ANALYTICS_TOKEN` | secret | evals-nightly | read-only Cloudflare analytics |

## Crons

From `workers/api/wrangler.toml` and `scheduled` in `workers/api/src/lib.rs`:

| Cron | Does |
|---|---|
| `17 3 * * *` | the nightly: each venue's off-site copy to its own bucket (with a manifest of each image's SHA-256), rotation (every copy for 7 days, weekly ones to 21 days), pruning, the witness of each log's tip, and the timers' safety net: every venue whose object has work due and no alarm is re-armed (log line `timers: N armed, N idle, N re-armed (lost alarms), N unreachable`; each re-arm is also a `timer.rearmed` line in that venue's error log) |

Each firing reads the clock once and passes the instant down.

There is no minute cron (removed 2026-09-28). Each venue's Durable Object sets its own ALARM
(`workers/api/src/hubdo/timer.rs`, rules in `workers/api/src/cron/timer.rs`) when a write makes work
due: an outbox entry (Telegram, WhatsApp, campaigns; due at once, retries on their backoff), a usable
eBills link (a minute on while open; while closed when the sales list is due or the venue opens), and
queued fiscal documents (never while `SEND_ENABLED = false`). The alarm asks the venue's runner object
(`cron~<venue>`) for one run -- at most 40 sends, the rest a minute later -- and sets the next alarm
only while work remains. A venue with nothing queued and no eBills link sets no alarm at all.

How to see it: `node tools/evals/run.mjs --suite nightly` (its cost group reads `tools/evals/collect/cf.mjs`) reports `cf.do_alarms_day` (alarm invocations) and
`cf.cron_runs_day` (scheduled invocations; 1 a day now, was 1,441). A day with no orders should show
`cf.do_requests_day` under 1,000.

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
design. Test orders are ended the same way, through the refund route (the old caretaker script that walked
them to DELIVERED was deleted on 2026-10-04).

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
