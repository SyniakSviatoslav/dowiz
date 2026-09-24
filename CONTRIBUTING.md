# Contributing to dowiz

Thank you for helping. dowiz is licensed under the **GNU Affero General Public License v3.0**
(`LICENSE`) and contributions are accepted under the **Developer Certificate of Origin**
(`DCO`). Be kind and specific: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) applies everywhere.

## Sign off every commit (DCO)

```sh
git commit -s -m "courier: the app reopens underground"
git commit --amend -s     # if you forgot
```

A pull request whose commits lack a `Signed-off-by` line is not merged.

## Set up

Requirements: Rust through rustup (the repo pins 1.96.1 in `rust-toolchain.toml`; add the
`wasm32-unknown-unknown` target for the Worker), Node 22, Python 3.

**There is no cargo workspace.** Enter each crate:

```sh
cd crates/dowiz-hub && cargo test        # never `cargo -p dowiz-hub` from the root
bash scripts/verify-hub.sh               # the four product crates and two ratchets
node --test $(find workers/api/public -name '*.test.mjs' | sort)
sh tools/gates/run-all.sh                # every gate, one table of exit codes
```

The browser apps in `workers/api/public/` are plain ES modules with no build step. Serve them with
the Worker (`cd workers/api && npx --yes wrangler@4 dev`) or, for pure modules, test them with
`node --test`.

More: [`docs/testing.md`](docs/testing.md), [`docs/architecture.md`](docs/architecture.md).

## The rules a change must keep

The full list, each with the gate that enforces it, is [`docs/code-quality.md`](docs/code-quality.md).
The ones most often missed:

1. **Evidence, not claims.** A fix comes with a test that was RED on the unfixed tree. Quote the
   exact `test result:` line and exit code in the pull request.
2. **Tests beside the code** in `foo/tests.rs`, calling the real function. Every refusal test has a
   positive twin.
3. **Files under 300 lines** (`tools/gates/file-size.sh`, a ratchet that may only fall).
4. **Money is integer minor units.** No `f64` near money; rates are parts per million.
5. **The clock is read once**, at the entry point, and passed down as `now_ms`.
6. **One venue per request, one image per handler.**
7. **Closed vocabularies stay closed.** Statuses and currencies come from the kernel through
   `tools/gen-vocab`; an order's channel and fulfilment kind are closed sets.
8. **Every UI string in sq, en and uk**, with ASCII quotes in JavaScript strings.
9. **Nobody is scored.** No rating, ranking or tier of a courier, customer or staff member.
10. **A new gate proves it can fire** with a `.prove.sh` before it is trusted, and anything broken on
    purpose to prove it is restored in the same change.

Heavy or external dependencies go behind an off-by-default Cargo feature, with a short comparison of
alternatives in the change (see `CLAUDE.md`, "Feature discipline").

## Pull requests

- One concern per pull request; the template asks for the evidence and for what you could not verify.
- Keep commit subjects in the house style: `area: what changed, in plain words`
  (see `git log --oneline`).
- CI must be green: `.github/workflows/ci.yml` runs the product crates, every gate, the browser
  module tests, the wasm32 build and the offline-courier browser test.
- Documentation that cites a path must cite one that exists (`tools/gates/paths.sh` checks the
  core documents).

## Security

Never open a public issue for a vulnerability; see [`SECURITY.md`](SECURITY.md).

## Trademark

"dowiz" is a trademark of the project owner (see `NOTICE` and `TRADEMARK.md`). Contributing code
does not grant trademark rights.
