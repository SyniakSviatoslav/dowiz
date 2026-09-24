## What and why

<!-- One paragraph: the defect or need, and what this change does about it. -->

## Evidence

<!-- Paste the exact lines, with exit codes. "It works" is not evidence. -->

- [ ] Tests: `cd <crate> && cargo test` -> `test result: ...` (per crate touched; no `cargo -p` from the root)
- [ ] Browser modules: `node --test ...` (if `workers/api/public/` changed)
- [ ] Gates: `sh tools/gates/run-all.sh` -> `run-all: N gate(s), all exit 0`
- [ ] A fix has a test that was RED before the change; a refusal test has a positive twin
- [ ] A new gate has a `.prove.sh`, and any rule broken on purpose to prove it is restored

## Rules this change keeps (docs/code-quality.md)

- [ ] Files under 300 lines, tests beside the code (`foo/tests.rs`)
- [ ] Money in integer minor units; no clock read outside the entry point
- [ ] One venue per request, one image per handler; closed vocabularies stay closed
- [ ] JS strings use ASCII quotes; every new UI string exists in sq, en and uk
- [ ] Nobody is rated, ranked or tiered
- [ ] Every commit is signed off (`git commit -s`, see `DCO`)

## Not verified

<!-- What you could not run or check, said plainly. -->
