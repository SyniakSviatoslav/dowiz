# Testing

dowiz is tested in layers, from pure functions to real browsers against a real host. Each layer
exists because a defect once passed every layer below it; the reasons are recorded in the files'
own headers.

**The rule that runs through all of it: a check that has never failed has not been shown to
work.** A new gate ships with a `.prove.sh` that breaks the rule in a scratch copy and demands RED,
then restores and demands GREEN. A fix ships with a test that was RED on the unfixed tree.

## Before anything: there is no cargo workspace

Every crate is standalone. Enter it:

```sh
cd crates/dowiz-hub && cargo test
```

Never `cargo -p <crate>` or `--manifest-path` from the root: with no workspace that can resolve the
wrong target graph and pass as exit 0 (see `CLAUDE.md`, "Build model").

## 1. Rust suites

| Crate | Command | Covers |
|---|---|---|
| `crates/dowiz-core` | `cargo test` | order and booking FSMs, money, tax, VAT schedule, PQ primitives with their KAT vectors |
| `crates/dowiz-hub` | `cargo test` | order log, tables, capabilities, consent, forget, stock, room deciders, imports |
| `crates/bebop-store` | `cargo test` | the image format, truncation refused, event log |
| `crates/bebop-wasm` | `cargo test --features decide` | the reader and the room's amend/pay deciders built for wasm32 give the server's bytes |
| `workers/api` | `cargo test` | every handler's pure half, the outbox, idempotency, eBills mapping, fiscal documents |
| `tools/gen-vocab` | `cargo test` | the vocabulary generator agrees with the kernel's golden signature |
| `tools/native-spa-server` | `cargo test` | the native twin of the owner, courier and storefront routes |
| `kernel`, `engine`, `apps/courier` | `cargo test --lib` / `cargo test` | the std facade and the non-product crates |

`bash scripts/verify-hub.sh` runs the four product crates and the SQL and file-size ratchets in one
go.

### How a test is written here

- **Beside the code.** `foo.rs` has its tests in `foo/tests.rs` (see
  `workers/api/src/command/amend.rs` and `workers/api/src/command/amend/`).
- **Call the real function.** A test never re-implements the rule it checks.
- **Every refusal has a positive twin.** A test that proves a bad input is refused sits next to one
  that proves the nearest good input is accepted, so a check that refuses everything cannot pass.
- **Name the corruption.** Image-corruption defects are locked with one named, deterministic
  corrupted cell ("bit 33 of the first key's length cell"), never with a random generator. Bounded
  `proptest!` suites over typed inputs, with a stated case count, are allowed; unbounded fuzzing of
  image bytes is not (it once asked for an 8 GiB allocation and killed the session).

## 2. Browser unit tests

Pure browser modules are tested with Node's built-in runner, no dependencies:

```sh
node --test $(find workers/api/public -name '*.test.mjs' | sort)
```

This covers `lib/` (money, booking time, floor plan, replica fold, stamps, paid-with), `lib/ui/`
(the component library), `room/` (logic, tips, open, guest rounds) and `courier/screens.js`.

**A JS file that does not parse takes a whole surface down.** `node --check` does nothing useful on
a `.js` file that is an ES module outside a module package; copy it to `.mjs` and check that, and
confirm the check fails on a deliberately broken copy. Strings use ASCII quotes only.

## 3. Gates

```sh
sh tools/gates/run-all.sh            # every gate that needs no compiler, one table of exit codes
sh tools/gates/run-all.sh --cargo    # also vocab.sh and the bebop-wasm four-reader gate
```

`run-all.sh` runs each `*.prove.sh` before its gate, then every `tools/gates/*.sh`, `unreached.py`,
the design gate (`scripts/design_gate.py`) and every `e2e/gates/*.prove.mjs`. It prints each gate's
verdict line beside its exit code and exits 1 if any row failed. What each gate holds is in
[code-quality.md](code-quality.md).

## 4. The four readers

```sh
sh crates/bebop-wasm/gate.sh          # needs the pinned toolchain with the wasm32 target
sh crates/bebop-wasm/gate.sh --prove  # flips a bit and demands the number move or a refusal
```

## 5. Conservation laws

`e2e/gates/conservation.mjs` reads a live venue with an owner's credentials and checks thirteen laws:
folded status equals served status; nothing held by an ended order; an order's money is its lines
plus fees minus discount; delivered cash is accounted to a courier; image gauges under their
ceilings; nothing quarantined; the nightly witness not contradicted; projections rebuild from the
log; the tax block conserves money; the till adds up per currency; a transfer is two halves of one
move; every wallet payment has exactly one ledger leg; and (law N) fiscal codes where fiscalisation
is configured. It is read-only.

Its alarm is proved in CI against a stub: `node e2e/gates/conservation.prove.mjs` checks each law
goes red when it should and stays green when it should not. `e2e/gates/recipes.prove.mjs` does the
same for the recipe audit.

## 6. Real browsers

`e2e/kit-regression/` drives Chromium through Playwright (the `playwright` library only; there is no
`@playwright/test` dependency).

| Script | Runs where | Proves |
|---|---|---|
| `outbox.mjs` | CI (`offline-writes` job), served from disk with a counting stub | a courier's tap made offline is queued, replayed once, and a refusal is not retried for ever; the RED half serves the courier app from before the outbox and clicks the real button |
| `courier-cold.mjs` | CI (`offline-writes` job) | the courier app reopened with no network still shows the job |
| `render.mjs`, `mobile.mjs`, others | by hand, against a host | CSP violations, unstyled pages, horizontal scroll, broken icons, tap sizes |

The CI pair needs git history (`fetch-depth: 0`) because the RED half checks out the courier app as
it was before the outbox.

## 7. Live walks, one per role

`e2e/walk/` walks each role through the real UI and reads every step back through the API. Each
script prints one `PASS`, `FAIL` or `INFO` line per step and exits with the number of FAILs.

| Script | Phases |
|---|---|
| `guest.mjs` | `browse`, `orders`, `book`, `bookcancel`, `qr` |
| `owner.mjs` | `setup`, `bookings`, `orders`, `refunds`, `panes`, `cleanup` |
| `waiter.mjs` | `claim`, `confirm`, `pay` |
| `kitchen.mjs` | `claim`, `cook` |
| `courier.mjs` | one run: offer, pick up, delivered screen up to its last tap, refused at the door |

**They write.** A walk places TEST orders, books tables, invites TEST staff and refunds what it
placed. Never point one at a real venue without the owner's agreement; the scheduled workflow
`key-flows.yml` runs them only against a QA host named in the `WALK_HOST` secret. Settings:

- `HOST` (the venue's origin), `LOC` (its slug), `OUT` (where logs and state go);
- a credentials file with `export OWNER_EMAIL=...`, `export OWNER_PASSWORD=...` and, for the
  courier walk, `export QA_COURIER_PHONE=...` and `export QA_COURIER_PASSWORD=...`. The scripts read
  it from `/root/.dowiz_owner`.

The courier walk stops short of "Delivered" on purpose: `DELIVERED` has no exit in the order
machine, so a TEST order delivered would stand as a real sale.

## 8. What CI runs

`.github/workflows/ci.yml`, on every push and pull request:

| Job | Runs |
|---|---|
| `product` | `cargo test` in `bebop-store`, `dowiz-core`, `dowiz-hub`, `workers/api`, `native-spa-server`, `gen-vocab`; `bebop-wasm --features decide` |
| `gates` | `tools/gates/run-all.sh --cargo` (every gate, prove scripts, design gate, conservation and recipes proofs, vocab, the four readers) |
| `browser-units` | `node --test` over every `*.test.mjs` under `workers/api/public` |
| `wasm32` | the Worker compiled for `wasm32-unknown-unknown` with the pinned toolchain |
| `offline-writes` | the two real-browser courier scripts |
| `supply-chain` | `cargo deny check` with `deny.toml` |
| `legacy-crates` | `kernel`, `engine`, `apps/courier` |

Scheduled: `mutations.yml` (`cargo mutants` over two kernel files, weekly and on demand), `health-cron.yml` (production probes every 15 minutes) and `key-flows.yml` (the walks,
daily, QA host only). See [operations.md](operations.md).

Other workflows in `.github/workflows/`, as of this release:

| Workflow | Status |
|---|---|
| `safety-floor.yml` | kept: runs `.claude/hooks/verify-safety-floor.sh`, which exits 0 on the tree |
| `skill-security.yml` | kept: scans agent-skill directories a pull request adds or changes |
| `visual.yml` | obsolete, to be deleted: its path filter watches `apps/web/` and `packages/ui/`, which were removed on 2026-07-15, so it never runs |
| `heartbeat-monitor.yml` | obsolete, to be deleted: a dead-man's switch for a Hetzner box that no longer serves production; `health-cron.yml` replaces it |
| `academia-extract.yml`, `academia_full_cron.yml` | obsolete, to be deleted: unrelated scraping jobs, and neither file parses as YAML |

## On the development box

The box is Android under Termux and proot; its phantom-process killer ends the whole session above
32 processes. Every cargo command goes through the slot, in the foreground:

```sh
bash bebop-lang/tools/slot.sh <label> cargo test --lib <filter> > out.txt 2>&1; echo rc=$?
```

Capture the exit code of the command itself, never of a pipe's last stage. Details in
[operations.md](operations.md#the-development-box).
