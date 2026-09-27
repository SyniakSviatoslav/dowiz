# Extreme binary size (brief D) and the standard measures: what the Worker's 4.85 MB is made of and a ranked list of cuts

Research, 2026-09-27. Lane W-RESEARCH, READ-ONLY. Tree `/root/lanes/w-research` at `5b77de64`; the artefacts
measured are the main tree's `/root/dowiz/workers/api/build/index_bg.wasm` (built 2026-09-27 01:07) and
`/root/dowiz/workers/api/target/wasm32-unknown-unknown/release/dowiz_api_worker.wasm` (01:06). Labels:
**MEASURED** (a command here, or a dated measurement document quoted with its section), **CARD** (the number the
task card quotes from the CI `wasm.*` indicators), **DOC** (a page fetched 2026-09-27), **EST**, **HYPOTHESIS**.

Companions: `2026-09-27-dag-architecture.md`, `2026-09-27-sel4-wasm.md`, `2026-09-27-performance-paths.md`.

---

## 0. The answer in ten lines

1. **The bundle doubled in five days and nobody's gate caught it.** 2026-09-22: 2,426,605 B raw / 904,804 gzip,
   code 1,824,804, name 348,560 (MEASURED, `docs/design/BLUEPRINT-OPTIMIZATION-AND-EVALS-2026-09-24.md` §A.1).
   2026-09-27 (MEASURED, `tools/evals/collect/wasm.py` through `slot.sh`, §1.3): **4,852,622 B raw / 1,785,270
   gzip; code 3,787,055; name 640,684; data 401,412; 6,713 functions**. The largest function is no longer a
   `future_to_promise` closure: it is `<dowiz_api_worker::hubdo::HubImages as worker::durable::DurableObject>::fetch`
   at **409,879 B** — the object's dispatcher (`workers/api/src/hubdo.rs:1170`) with every `/fold/*` command's
   async state machine inlined into one `match`; second is the `storefront::place` closure at 48,280 B. (The
   card's "387 KB `future_to_promise` closure" is the same order of size under a different symbol; this build
   names the dispatcher.) The `wasm.raw` indicator is defined as a ratchet (`wasm.py:405`) but **no
   `wasm.baseline` exists** under `tools/evals/baselines/` (MEASURED listing: `cf`, `health`, `live`, `product`,
   `ux` only), so the ratchet has nothing to ratchet against.
2. **The optimiser runs at `-O`, for speed, not `-Oz`.** `workers/api/Cargo.toml` carries no `[package.metadata.wasm-pack]`
   table (MEASURED grep), so worker-build 0.8.5 uses its default: `wasm_opt_args()` → `["-O"]` (its
   `src/build/manifest.rs:299-303`), then appends `--all-features` and `--debuginfo` "Keep the Wasm names
   section" (`src/build/mod.rs:463-466`). The pre-opt file is 6,640,589 B and the bundle 4,852,622 B, so
   `wasm-opt` did run (MEASURED) — with the wrong goal for a bundle judged on size.
3. **The 630 KB name section is a build-tool decision, not a `strip = false` consequence.** `strip = false`
   (`Cargo.toml:60-71`) is forced because rustc's link-time strip removes what wasm-bindgen's catch wrappers need;
   but the name section that survives is kept by worker-build's `--debuginfo` flag to `wasm-opt`, which runs
   AFTER wasm-bindgen. A second `wasm-opt --strip-debug --strip-producers` pass on `build/index_bg.wasm` (the
   binaryen 130 binary is cached at `/root/.cache/worker-build/wasm-opt-aarch64-linux-130/bin/wasm-opt`,
   MEASURED) is the cheapest 13 % cut in this document. HYPOTHESIS until one build + smoke test: post-glue
   stripping cannot break the glue, which references exports by name, not the custom section.
4. **The bundle is the handlers, not the libraries** (OPT §A.1: `dowiz_api_worker` 52.7 % of code bytes; the
   twelve largest functions are `async fn` state machines holding `serde_json::json!` builders). Brief (D)'s
   "bytecode VM for business rules" would replace the SMALLEST part of the bundle (the pure deciders) and add an
   interpreter; it is the wrong target.
5. **`panic_immediate_abort` / `no_std` / u8 error codes are largely moot here**: the target already defaults
   to `-Cpanic=abort` (DOC, rustc wasm32 page); `immediate-abort` removes panic-message formatting only
   (DOC, codegen options) and needs `-Zbuild-std` on nightly (DOC, min-sized-rust); `dowiz-core` is already
   `#![no_std]`. The 252 KB of `core` (OPT §A.1) is `core::fmt` reached from `serde_json` and `format!`, not from
   panics. EST saving from immediate-abort: tens of KB; risk: a nightly toolchain against the repo's pinned
   `1.96.1` (`rust-toolchain.toml`) and the toolchain-bump gate.
6. **The worst offender is Cloudflare's own glue plus dowiz's async bodies**: two `future_to_promise` closures
   own every exported async future; the card's 387 KB closure is the sum of the handler futures inlined into the
   export. The cure is structural — thin `async` wrappers around sync bodies (OPT §A.1's own diagnosis) — and it
   is the same move as the DAG's "nodes are pure functions" (companion §3).
7. **The size limit is not the problem; startup is.** DOC (limits page, Sep 5 2026): "Worker size (uncompressed)
   64 MiB" on both plans, "There is no compressed size limit"; "A Worker must parse and execute its global scope
   ... within 1 second", "Larger bundles and expensive initialization code in global scope increase startup
   time"; `wrangler check startup` and `startup_time_ms` in the deploy output report it. Startup is UNMEASURED
   from this box (no token reads the dashboard); it is the first number the ranked list should be judged by.
8. **Ranked list (§4)**: the first four rows cost one line each and are worth ≈ 1.3 MB raw (EST); the next
   three are lane-sized refactors worth more; the brief's own four ideas are at the bottom with the reason.
9. **Every row needs a baseline first**: write a `wasm.baseline` file beside the five that exist under
   `tools/evals/baselines/`, from today's build (`wasm.py` already prints the indicators; `run.mjs` reads
   baselines per group, `tools/evals/rules.mjs:86`). Without it a 2× regression is invisible, which is what just
   happened.
10. **Risk register (§5)**: the catch-wrapper trap (`Cargo.toml:60-69`, measured once), the panic-recovery
    trade-off (`--no-panic-recovery` vs `--panic-unwind`), nightly, and the name section's value in a stack
    trace when a 500 has to be chased through Workers Logs at 10 % sampling.

---

## 1. What is in the bundle

### 1.1 Numbers, with their dates

| Artefact | Bytes | Date | Source |
|---|---|---|---|
| `build/index_bg.wasm` (what wrangler uploads) | **4,852,622** | 2026-09-27 01:07 | MEASURED `ls -la` |
| `target/.../release/dowiz_api_worker.wasm` (pre-`wasm-opt`, pre-bindgen) | 6,640,589 | 2026-09-27 01:06 | MEASURED |
| `build/index.js` (esbuild shim, panic-recovery proxy) | 33,154 | 2026-09-27 01:07 | MEASURED |
| Same bundle by section (probe, §1.3): gzip -9 / code / data / name / other / functions | 1,785,270 / 3,787,055 / 401,412 / 640,684 / 23,471 / 6,713 | 2026-09-27 01:07 | MEASURED |
| Largest functions: `HubImages::fetch` / `storefront::place` closure | 409,879 / 48,280 | 2026-09-27 | MEASURED (`wasm.functions_over_40k` = 2) |
| CI indicators (card): raw / gzip / code / name / largest closure | 4.78 MB / 1.76 MB / 3.7 MB / 630 KB / 387 KB | 2026-09-27 | CARD |
| Same bundle, five days earlier: raw / gzip / code / name / data / largest closure | 2,426,605 / 904,804 / 1,824,804 / 348,560 / 234,831 / 49,634 | 2026-09-22 22:20 build | OPT §A.1 |
| `Cargo.toml:70` comment | "~447 KB wasm + 22 KB JS" | 2026-09-15 (`cb95cdb2`) | stale by 10× |
| Limit | 64 MiB uncompressed, both plans; 1 s startup | Sep 5 2026 | DOC limits page |

Growth 09-22 → 09-27: **+2.0×** raw, name section **+1.8×**, largest function **+7.8×**. The five days include
the room commands moving into `crates/dowiz-hub` with `serde_json` (`crates/dowiz-hub/src/room/delta.rs:1-4`
"MOVED FROM `workers/api/src/fold.rs` (D7 phase 1)"), the ebills and fiscal modules (7,588 lines across
`workers/api/src/ebills` and `workers/api/src/fiscal`, MEASURED `wc -l`), the room, the till, tips, transfers
(`workers/api/src/command/` listing). Which of these grew the closure is UNMEASURED until the per-crate probe
lands (§1.3).

### 1.2 Code bytes by crate, 2026-09-22 vs 2026-09-27

| Crate | 09-22 (OPT §A.1, 1,819,648 code B) | 09-27 (probe, 3,787,055 code B) | Growth |
|---|---:|---:|---:|
| `dowiz_api_worker` | 958,184 (52.7 %) | **2,333,474 (61.8 %)** | 2.4× |
| `core` | 252,318 | **724,549 (19.1 %)** | 2.9× |
| `dowiz_hub` | 45,712 | 123,805 | 2.7× (the room moved in with `serde_json`) |
| `alloc` | 24,253 | 91,212 | 3.8× |
| `serde_json` (+ `serde_core`) | 33,227 | 86,706 + 46,958 | 4.0× |
| `worker` | 48,017 | 82,733 | 1.7× |
| `js_sys` | 104,201 | 52,663 | 0.5× |
| `dowiz_core` | 25,090 | 40,605 | 1.6× |
| `url` | 30,355 | 30,656 | 1.0× |
| unnamed | 185,772 | 19,956 | 0.1× (better name coverage this build) |
| `bebop_store` | 15,103 | 15,070 | 1.0× |

Functions: 3,703 → 6,713. "The kernel (`dowiz_kernel`) does not appear by name: LTO inlined it" (OPT). The
growth is dowiz's own handlers plus the `core`/`alloc`/`serde` machinery they instantiate (`Debug`/`Display`
impls, `Vec`/`String` monomorphisations, `serde_json::Value` walks) — not new dependencies: `url`,
`bebop_store` and `js_sys` are flat or down.

Dependency graph today: 106 crates in `workers/api/Cargo.lock` (MEASURED), among them `chrono`, `tokio`,
`url`/`idna`/`icu_*` (pulled by `worker`), `wasm-streams`, `serde-wasm-bindgen`, `strum`. `serde_json` use
sites under `workers/api/src`: 2,529 (MEASURED grep).

### 1.3 The probe this lane ran

`tools/evals/collect/wasm.py` over the 2026-09-27 bundle, through `slot.sh` (the box rule for every probe).
The slot was held by other lanes (`audit`, then `kaccess`) for most of the session; the probe acquired slot 1 at
10:45:38, ran for 1 s, `rc=0`. Its indicators are the MEASURED rows of §1.1 and the right-hand column of §1.2.
The command is:

```
bash /root/dowiz/bebop-lang/tools/slot.sh wasmeval python3 tools/evals/collect/wasm.py \
  /root/dowiz/workers/api/build/index_bg.wasm | grep '^\[' > wasm-ind.json
```

(`grep '^\['` because `slot.sh` prints its own "slot: acquired" line into stdout before the JSON, which is why
the previous run of this lane read an empty file with `rc=0`.)

---

## 2. What each tool actually does today (so the flags below are changes, not guesses)

| Stage | Tool and flags | Evidence |
|---|---|---|
| rustc | profile.release: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = false`; `panic` unset → target default `abort` | `Cargo.toml:57-71`; DOC wasm32 page: "By default the `wasm32-unknown-unknown` target is compiled with `-Cpanic=abort`" |
| wasm-bindgen | `--target bundler`-style module output, `--no-typescript`; with panic recovery on (default): `--experimental-reset-state-function --force-enable-abort-handler`; `--keep-debug` only for the profiling profile | worker-build `src/main.rs:88-92`, `:348-351`; `src/build/mod.rs:532-539` |
| wasm-opt (binaryen 130) | `-O --all-features --debuginfo` | `src/build/manifest.rs:302`; `src/build/mod.rs:463-466`; binary at `/root/.cache/worker-build/wasm-opt-aarch64-linux-130/bin/wasm-opt` |
| esbuild | `--format=esm --bundle --minify` on the JS shim; wasm external | `src/main.rs:370-384` |
| panic recovery | default on (the `Reflect.construct` proxy in `build/index.js`); `--no-panic-recovery` drops it; `--panic-unwind` (0.8.0+) keeps instance state across a panic but needs nightly + `-Zbuild-std` | `src/main.rs:69`, `:109`; DOC https://blog.cloudflare.com/making-rust-workers-reliable/ |
| toolchain | `1.96.1`, pinned, bump gated by a spot-check document | `rust-toolchain.toml`; `docs/design/BLUEPRINT-ITEM-14-toolchain-pin-2026-07-19.md` |
| worker-build version | 0.8.5 installed; 0.8.7 released 2026-09-25 | MEASURED `worker-build --version`; DOC https://lib.rs/crates/worker-build |

---

## 3. The panic strategy, because it is the one place size and reliability trade

- `strip = true` at the rustc stage breaks wasm-bindgen: "error: failed to generate catch wrappers ... externref
  table required for catch wrappers" — measured once, with and without link args, and recorded in
  `Cargo.toml:60-69`. That is a fact about **rustc's** strip removing sections wasm-bindgen reads. It says nothing
  about stripping **after** wasm-bindgen has run.
- `--no-panic-recovery` "drops automatic reinitialisation after a WebAssembly.RuntimeError; keeping the symbols
  is cheaper than losing that" (`Cargo.toml:67-69`). Agreed, and irrelevant to the name section (§0.3).
- `--panic-unwind` (DOC, Cloudflare post): "panics can be fully recovered ... The WebAssembly instance remains
  valid and reusable"; for Durable Objects "reinitialization means losing that state entirely" — dowiz's
  `mem` + `folded` (`workers/api/src/hubdo.rs:196-208`). Cost: `RUSTFLAGS='-Cpanic=unwind' cargo +nightly build
  -Zbuild-std`, legacy EH instructions by default until LLVM flips (`-Cllvm-args=-wasm-use-legacy-eh=false`,
  DOC rustc wasm32 page), and unwind tables in the code section (size UP, EST +5–10 %). Worth measuring after
  the DAG's incremental memo exists (companion §3.2), because that is when in-memory state becomes valuable.
- `panic=immediate-abort` (DOC codegen options: "terminate the process upon panic, and do not call any panic
  hooks"; "every crate in the graph must use `immediate-abort`", i.e. build-std): removes the panic formatting
  machinery. min-sized-rust (DOC https://github.com/johnthagen/min-sized-rust) pairs it with
  `-Zlocation-detail=none -Zfmt-debug=none` and `build-std-features="optimize_for_size"`. All nightly. The
  repo's toolchain pin is the gate that decides this, not size.

---

## 4. The ranked list

Estimates are EST unless a measured number is cited; "raw" is the uploaded `.wasm`; effects on gzip are
smaller for the name section (names compress well) and larger for code.

| # | Change | Where | Bytes (raw) | Risk | Cost |
|---|---|---|---|---|---|
| 1 | **Write the baseline**: a `wasm.baseline` under `tools/evals/baselines/` from today's `wasm.py` output; make `wasm.raw`/`wasm.gzip`/`wasm.section.name` ratchets red in `.github/workflows/evals-nightly.yml` | `tools/evals/run.mjs:48-49` already collects; `rules.mjs:86` maps `wasm.*` to the file | 0 now; prevents the next 2× | none | one file |
| 2 | **Strip the name and producers sections after the glue**: append `&& /root/.cache/worker-build/wasm-opt-aarch64-linux-130/bin/wasm-opt --strip-debug --strip-producers build/index_bg.wasm -o build/index_bg.wasm` to `[build] command` (or set `WASM_OPT_BIN` and a wrapper) | `wrangler.toml` `[build]`; worker-build `binary.rs:284` honours `WASM_OPT_BIN` | **−640,684 B** (MEASURED name section, 13.2 % of raw; −348,560 on the 09-22 build) | LOW: post-glue; the recovery proxy names exports, not functions. Loses function names in Workers Logs stack traces (10 % sampled, dashboard-only on Free). HYPOTHESIS until one build + `workers/api/scripts/smoke.sh` | one line + a smoke run |
| 3 | **`wasm-opt -Oz` instead of `-O`**: `[package.metadata.wasm-pack.profile.release] wasm-opt = ["-Oz"]` in `Cargo.toml` | worker-build `manifest.rs:303` `ExplicitArgs` | −10 to −20 % of code (DOC rustwasm book: "another 15-20% savings"; on 3.7 MB code, EST −400–700 KB) | LOW; measure CPU too (companion performance doc P5: `s`/`z`/`O` is one probe) | one table |
| 4 | **Try `opt-level = "s"`** against `"z"` | `Cargo.toml:57` | ±5 % (DOC: "`s` can sometimes result in smaller binaries than `z`. Always measure!") | none | one probe |
| 5 | **Split the object's dispatcher and thin the async wrappers**: `HubImages::fetch` (`hubdo.rs:1170-1512`) is ONE 409,879 B function because every `/fold/*` arm's command future is inlined into its `match`; each arm should call a separate `#[inline(never)] async fn` (the child modules `hubdo/room.rs`, `hubdo/ebills.rs` etc. already exist — the parent still inlines them under LTO + `codegen-units = 1`). Then the biggest Worker handlers (`storefront::place` 48,280 B, and OPT §A.1's twelve) become `async fn` that await inputs, call a sync `fn body(...) -> Result<Out, Refused>`, and await the write. The future then holds inputs and outputs, not every `json!` builder's live set | `hubdo.rs`; `workers/api/src/storefront.rs`, `owner.rs`, `platform.rs`, `bootstrap.rs`, `cloud.rs`, `auth.rs`, `services/catalogue` | splitting alone mostly moves bytes; the saving is the smaller live sets across awaits: EST −200–400 KB; the sync bodies become `cargo test`-able (the DAG's "nodes are pure"); and the 410 KB function stops being the object's hot path's instruction-cache footprint | MEDIUM: a refactor per handler; the `one-image`/`clock` gates hold | one lane for the dispatcher, then one per 3–4 handlers |
| 6 | **Byte pass-through instead of parse-then-reserialise** on the poll and menu paths (companion §3.2, §4): `Response` from the object's bytes | `hubstore::orders` → `owner::orders`; `/api/menu` | removes `serde_json` `Deserialize` impls for `OrderView` and the menu shapes on those paths; EST −50–150 KB, and CPU | LOW | R1/R2 of the DAG plan |
| 7 | **Drop `url`/`idna`/`icu_*`** where `worker`'s `Url` is used only to read query pairs (`hubdo.rs` `/fold/*` routes parse `?id=`, `?since=`, `?venue=`) | `worker` pulls `url` transitively; a hand `split('&')` for the object's own routes avoids `icu_normalizer_data` | −30 KB code + `icu_normalizer_data` data bytes (OPT: "`icu_normalizer` 4.6 KB it drags in"; data section 234 KB in 09-22 includes tables) | LOW–MEDIUM: only if `worker`'s own `Request::url()` is avoidable on those paths; else 0 | measure with `twiggy dominators` first |
| 8 | **`wasm-snip` the panic formatting and `core::fmt::float`** paths, then `wasm-opt --dce` | DOC rustwasm book "wasm-snip replaces a WebAssembly function's body with an `unreachable`"; float formatting is unreachable by the money law (integer minor units, `DOWIZ-COMMON-RULES` rule 6) | EST −20–60 KB | MEDIUM: a snipped function that IS reached is a trap in production; needs the 360-program sweep's equivalent — the 221 Worker tests plus smoke | a lane, with a RED test that a snipped path is unreachable |
| 9 | `--panic-unwind` | build command; nightly + build-std | size UP (EST +5–10 %); reliability of object memory up | HIGH (toolchain pin) | defer to after the incremental memo |
| 10 | `panic=immediate-abort` + `-Zlocation-detail=none` + `optimize_for_size` std | nightly + build-std | EST −30–80 KB (panic strings, location tables) | HIGH (toolchain pin; every crate in the graph) | defer; row 8 gets most of it on stable |
| 11 | Brief (D): string dictionary packing | the catalogue's 538 KB is DATA in an image, not code; its Kv packing is the un-done phase 4b (memory `dowiz-hub-seven-phases`), a bebop-lang change on a different toolchain | 0 bytes of wasm; a store change | — | not a binary-size item |
| 12 | Brief (D): BSS-zero-initialised state | wasm linear memory is zero-initialised by construction; the `data` section holds only initialised statics (234 KB on 09-22: JSON keys, i18n strings, ICU tables) | 0 | — | already true |
| 13 | Brief (D): `u8` error codes | `Refused` (`crates/dowiz-hub/src/room`) carries the customer's own words ("salmon: 80 wanted, 0 available", `command/mod.rs:226-228`) and the 409/404 status; replacing them with a byte saves the strings and loses the product | EST −10–30 KB | product regression | AGAINST |
| 14 | Brief (D): a bytecode VM for business rules | the rules are the smallest part: `dowiz_hub` 45,712 + `dowiz_core` 25,090 + `bebop_store` 15,103 B (OPT, 4.7 % of code); an interpreter is +60–120 KB (BEBOP-IN-WASM §0.6, EST) and 15–20× slower | negative | AGAINST | — |

EST total for rows 2–4 (one line each): **≈ 1.0–1.4 MB raw** off 4.85 MB. Rows 5–6 are the ones that keep
falling as the DAG lands.

---

## 5. Risks, each with the check that retires it

| Risk | Check |
|---|---|
| Row 2 breaks the recovery proxy or a `catch` wrapper | build, `bash workers/api/scripts/smoke.sh https://<qa-hub>`, then force a panic on the QA hub (a route that `unwrap`s `None` behind a secret flag) and watch the next request succeed (the reinitialisation path) |
| Row 3's `-Oz` slows the fold | the three-profile probe through `slot.sh` (companion performance doc P5) on `/fold/rebuild` wall time |
| Row 5 changes behaviour | the 221 Worker tests + `one-image`, `clock`, `one-venue` gates; each handler moved gets its sync body's tests first (rule 7, tests beside the code) |
| Row 8 snips a reached function | a RED test per snipped symbol proving it is unreachable from every route; `smoke.sh` after deploy |
| Rows 9–10 need nightly | the toolchain-bump gate requires a spot-check document under docs/audits/toolchain for the new version (`docs/design/BLUEPRINT-ITEM-14-toolchain-pin-2026-07-19.md`); do not do this for size alone |
| Startup regresses while raw falls | read `startup_time_ms` from `wrangler deploy` output and record it beside `wasm.raw` in the baseline (the number is in the deploy log, not in the analytics API) |
| The name section's absence hides a 500's origin | keep a non-stripped copy of each deployed build under `build/` for symbolisation; Workers Logs on Free are 3-day, 10 % sampled anyway (FREE-TIER §1.2) |

---

## 6. What was NOT verified

- Which handlers inside `HubImages::fetch` account for the 409,879 B: `wasm.py` reports whole-function sizes;
  a `twiggy dominators` run (DOC https://github.com/rustwasm/twiggy) on the same bundle would split it by callee.
- That post-glue `--strip-debug` is safe (row 2): one build was not run here (heavy job; slot busy).
- `startup_time_ms` for any build: no deploy log exists on this box (`ls` of `workers/api/*.log` empty) and the
  dashboard is unreadable from here.
- Whether worker-build 0.8.7 (2026-09-25) changed the `--debuginfo` default; 0.8.5's source was read.
