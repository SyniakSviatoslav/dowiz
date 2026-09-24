# Code quality: the rules, and what enforces each

A rule that lives only in a document gets bypassed at 23:00 on a Friday. Each rule below names the
gate or test that holds it, or says plainly that nothing does yet. Run them all with:

```sh
sh tools/gates/run-all.sh            # prints one row per gate: ok/FAIL, exit code, verdict line
sh tools/gates/run-all.sh --cargo    # plus vocab.sh and the bebop-wasm four-reader gate
```

Most gates are **ratchets**: a `.baseline` file records today's count, the gate refuses anything
worse, and the number may only fall, by hand, in the commit that earns the reduction.

## Mutation proofs: a gate must be heard to fire

A gate whose alarm has never sounded measures nothing. Three gates here once counted their own
epitaph (the comment recording a deletion) and passed while blind. So:

- every new gate ships with `tools/gates/<name>.prove.sh`, which breaks the rule in a scratch copy
  and demands RED, then restores it and demands GREEN; `run-all.sh` runs each proof before its gate;
- the live audits prove themselves against stubs: `e2e/gates/conservation.prove.mjs`,
  `e2e/gates/recipes.prove.mjs`;
- gates strip comments before counting;
- a rule broken on purpose to show a test goes red is restored in the very next edit. Before a
  change is handed over, this must print nothing for the files it touched:
  `grep -rn "false &&\|&& false\|if false\|if true\|-never\"" <files>`.

## The rules

| Rule | Why | Enforced by |
|---|---|---|
| **Files under 300 lines** | a file nobody can read whole is a file nobody reviews whole | `tools/gates/file-size.sh` (ratchet: count of files over 300 lines, and the worst file) |
| **Tests beside the code**: `foo.rs` with `foo/tests.rs` | the test moves with the code and is found by the next reader | convention, see `workers/api/src/command/amend/`; `tests.rs` files are exempt from `file-size.sh`. No gate. |
| **Every refusal test has a positive twin** | a check that refuses everything passes every refusal test | review. No gate. |
| **Tests call the real function** | a test that re-implements the rule tests the copy | review. No gate. |
| **Money is integer minor units** | a float cannot represent 0.10; a drawer that does not reconcile is a legal problem | `tools/gates/float-money.sh` (ratchet); currency-typed `i64`/`i128` in `crates/dowiz-core/src/money.rs`; conservation laws 3, 9, 10, 12 |
| **The clock is read at the entry point only** | a job whose parts each ask the wall clock has as many answers as parts; a pure core replays identically | `tools/gates/clock.sh` (zero); `clippy.toml` names `SystemTime::now` as disallowed (clippy is not yet a gate, see below) |
| **One venue per request** | 37 owner routes once authorised one venue and acted on another | `tools/gates/one-venue.sh` |
| **One image written per handler** | two writes in one handler can half-happen; a command runs in one object turn | `tools/gates/one-image.sh` |
| **No SQL** | the store is the venue's bebop images; D1 was removed on 2026-09-22 | `tools/gates/no-sql.sh` (zero) |
| **Replayable routes are idempotent on the server** | a phone's queued tap is replayed; a replay must get the first answer, not a second order | `tools/gates/idempotent.sh` |
| **Closed vocabularies** | an open set silently accepts a typo as a new kind | statuses and currencies: `tools/gates/vocab.sh` (the committed `lib/vocab.js` is exactly what `tools/gen-vocab` emits) and `tools/gates/vocabulary.sh` (every status has a word in three languages and a colour on every surface); order channel: `tools/gates/channel-closed.sh`; order event kinds in Rust and JS: `tools/gates/event-kinds.sh` |
| **Nobody is scored** | trust is a signed capability, never a score (`DECISIONS.md`) | `tools/gates/no-scoring.sh`; routing and capability enums deliberately have no `Ord` |
| **Marketing needs consent** | Law 124/2024 and GDPR Art. 7(1): consent must be demonstrable | `tools/gates/consent.sh` |
| **The customer card holds nothing a fold knows** | a stored copy of a derived number is a second number that disagrees | `tools/gates/record.sh` |
| **A control a thumb can hit** (44 px) | the person tapping is holding plates or riding in the rain | `tools/gates/tap-size.sh` |
| **Documents cite files that exist** | prose is never executed; `CLAUDE.md` once pointed every reader at a file that never existed | `tools/gates/paths.sh` (over `CLAUDE.md`, `AGENTS.md`, `DECISIONS.md`, `CONTEXT-INDEX.md`) |
| **Nothing built that nothing calls** | tested and unreachable is not switched on | `tools/gates/unreached.py` |
| **Shared design tokens and money formatting** | seventeen tokens drifted apart across three copies | `scripts/design_gate.py` |
| **CSP: no inline style or script, no unlisted origin** | on 2026-09-16 one header left every surface unstyled while every request answered 200 | the policy in `workers/api/public/_headers`; `e2e/kit-regression/render.mjs` fails on any CSP violation in a real browser (run by hand); `tools/live-checks/health.sh` checks the header is served |
| **ASCII quotes in JavaScript strings** | a typographic quote as a string delimiter is a syntax error that takes down a whole surface | no gate yet. Check a file by copying it to `.mjs` and running `node --check` on the copy, and confirm the check fails on a deliberately broken copy (`node --check` on a plain `.js` file proves nothing). |
| **Every UI string in sq, en and uk** | a missing key shows the key's name to a guest | partial: `tools/gates/vocabulary.sh` for status words; `e2e/walk/guest.mjs browse` reports untranslated keys live. No general gate yet. |
| **The four readers of an image agree** | a second reader found a truncation read as the previous generation on day one | `crates/bebop-wasm/gate.sh`, and the wasm module's size ratchet `crates/bebop-wasm/bytes.baseline` |
| **The register adds up** | the one question a restaurant asks | `e2e/gates/conservation.mjs` (thirteen laws, live and read-only; its proof runs in CI) |
| **Dependencies are allowed ones** | yanked crates, wildcards, disallowed licences | `deny.toml`, run by `cargo deny` in CI's `supply-chain` job |

### Gates arriving with the 2026-09-24 audit (Wave G)

These are in the working tree on 2026-09-24, each with its `.prove.sh`, and join CI through
`run-all.sh` when they are committed: `sw-shell.sh` (every service-worker shell lists every module
its page imports, so an app cannot go blank offline), `venue-clock.sh` (a venue's day is read in the
venue's zone at the instant being built), `ui-adoption.sh` (hand-rolled UI that a `lib/ui` component
replaces, counted) and `idem-done.sh` (every refusal after an idempotency claim records its verdict).
See `docs/design/AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24.md`, section 3a.

## Formatting and lints: configured, not yet gates

- **`rustfmt.toml`** pins rustfmt's defaults (edition 2021, width 100). No product crate is
  fmt-clean yet (`cargo fmt --check` hunks on the 2026-09-24 working tree: `crates/bebop-store` 67,
  `crates/bebop-wasm` 26, `crates/dowiz-hub` 695, `crates/dowiz-core` 2,144, `workers/api` 2,383),
  so CI does not run
  `cargo fmt --check`. Turning it on is one mechanical `cargo fmt` commit per crate, made while no
  one else has that crate open, followed by that crate's tests.
- **`clippy.toml`** disallows `std::time::SystemTime::now` outside the entry point. `cargo clippy`
  is not in CI: `crates/dowiz-hub` stops on one deny-by-default lint in a test assertion
  (`src/promo.rs`, `d <= i64::MAX` is always true) and carries about 110 warnings. The plan: fix
  the one error, then add `cargo clippy -- -D clippy::disallowed_methods` per crate, then widen.
- **`.editorconfig`** records the conventions the tree already follows: UTF-8, LF, final newline,
  two-space indent for web files, four for Rust and Python, and fixtures left byte-exact.

## Engineering habits that are not rules but should be

- **A number is measured or it is a hypothesis.** Quote the command and the line it printed.
- **A 200 proves a route exists, not that it works.** Drive the real flow and read the result back.
- **Failures are loud.** Never let a failure look like success or like slowness; a gate prints what
  it counted, not only its exit code.
- **Capture exit codes directly**: `cmd > out 2>&1; echo rc=$?`, never `cmd | tail; echo $?`.
