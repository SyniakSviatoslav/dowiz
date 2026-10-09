# docs/measurements/evals — what the evals harness writes here

`tools/evals/run.mjs` (BLUEPRINT-OPTIMIZATION-AND-EVALS-2026-09-24 §B.3) writes one pair per run:
`<date>-<commit>-<suite>.json` (one flat object, `_run` plus one entry per indicator:
value, unit, rule, baseline, status, source) and the `.md` beside it (breaches first, then every
UNVERIFIED indicator and why, then each group). `latest.html` is the static dashboard:
`node tools/evals/dashboard.mjs`. The live tick appends to `docs/measurements/live/<date>.jsonl`.

A commit ending `-dirty` was measured on a working tree with uncommitted changes.

## The GitHub nightly reads dowiz-watch (2026-10-08)

Bot Fight Mode on dowiz.org challenges every request from GitHub's runners, so `evals-nightly.yml`
runs `--suite nightly-cf`: no request to *.dowiz.org. The zone collectors (live, health, product)
run inside Cloudflare in dowiz-watch (`workers/watch/src/evals.js`, the SAME collector code) at
03:10 UTC; `collect/watch.mjs` reads `GET <WATCH_URL>/evals/latest` and FAILS the run
(`watch.evals_fresh` = 0) when the result is missing, unfinished or older than 12 h. The watcher's
live timings are `edge.*` (from inside Cloudflare), the box's stay `live.*`; each group has its own
baseline file. ux needs a browser on the zone and is one UNVERIFIED row (`ux.measured_here`) there;
the box's `--suite nightly` still measures it.

`tools/evals/baselines/edge.baseline` is written once from N >= 7 runs of the DEPLOYED watcher:

```sh
EVALS_READ_TOKEN=$(cat /root/.dowiz_evals_read_token) node tools/evals/edge-baseline.mjs --runs 7
```

## Running it on the box

Every suite goes through the slot (the nightly starts a headless Chromium):

```sh
cd /root/dowiz
bash bebop-lang/tools/slot.sh evals-ci      node tools/evals/run.mjs --suite ci
bash bebop-lang/tools/slot.sh evals-nightly node tools/evals/run.mjs --suite nightly
bash bebop-lang/tools/slot.sh evals-live    node tools/evals/run.mjs --suite live
node tools/evals/dashboard.mjs
```

For the box's own nightly, one crontab line (not installed by the harness; the operator adds it):

```
41 3 * * * cd /root/dowiz && bash bebop-lang/tools/slot.sh evals-nightly node tools/evals/run.mjs --suite nightly >> /root/dowiz/docs/measurements/evals/nightly.log 2>&1
```

Inputs, all optional (a missing one turns its indicators UNVERIFIED, never green): owner
credentials from `/root/.dowiz_owner` (or `OWNER_EMAIL`/`OWNER_PASSWORD`), the Cloudflare analytics
token from `/root/.cf_analytics_token` (or `CF_ANALYTICS_FILE`), a platform administrator's token as
`ctx.platformToken` for `/api/platform/health`, and a `traffic.mjs` report as `EVALS_TRAFFIC_JSON`
for the order suite (the harness never places an order itself).

Baselines live in `tools/evals/baselines/<group>.baseline`: written on the first run, lowered
(or, for `floor` rules, raised) by any run that improves them, and moved the other way only by
hand with a dated note. `--no-baseline-write` judges without touching them (CI does this).
