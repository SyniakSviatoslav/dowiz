# bebop inside wasm and the Worker: what stands between them, what it would cost, and what the product would gain

**Date:** 2026-09-23. **Tree read:** the working tree of `/root/dowiz` and `/root/dowiz/bebop-lang`
as of this date; this lane ran no git command, so no HEAD hash is quoted. Three other lanes were
editing `workers/api/src/command`, `hubdo.rs`, `crates/dowiz-hub/src/lib.rs`, `crates/dowiz-core`
tax/money, `services/ordering`, `services/customers` at the same time; line numbers in those files
are from the tree as read and may drift.

**Method.** Every "today" statement names a path and a line. `measured` means a command was run on this
box and its output is quoted; `hypothesis` means it was not. Costs in weeks are hypotheses unless a
number is cited. Where a document and the tree disagree, §11 names the document. Nothing here was
taken from a document about the code when the code itself was readable.

**The question.** The STACK blueprint (`BLUEPRINT-STACK-AND-DEPENDENCIES-2026-09-22.md:597-684`)
settled that no bebop code executes in the product. The operator's next question is the right one:
not "is it true" but **what would it TAKE** — what stands between bebop and running inside the
Worker, what it would cost, and what the product would actually gain. §0 is the answer; §1 re-checks
the premises first, because a plan built on a wrong premise is the expensive kind.

---

## 0. The answer, in ten lines

1. **The premises hold** (§1), with one correction: `docs/design/BEBOP-BACKEND-ROADMAP.md:13-16`
   claims a "v2 (live): WASM `codegen_wasm`" — no such code exists anywhere in the tree (measured,
   `find bebop-lang -iname '*wasm*'` → nothing). The claim is stale under a SUPERSEDED header.
2. **There is no backend seam.** `bebop.bp` is a single-pass parser-emitter that writes AArch64
   words as decimal constants from **595 `em(` sites** in **194 `emit_*`/`vs_*` functions** (measured,
   §2). wasm32 is not a target flag; it is a second emitter of the same size — T94, OPEN since
   2026-08 (`TASKS.md:103`).
3. **The subset the product would want is the store reader and the fold**, and its builtin surface
   is **twelve Linux syscalls plus `crc32x`** (measured, §3) — none of which exists in wasm32. The
   subset is exactly the code `crates/bebop-store` already is, in Rust, 2,222 lines, linked in the
   Worker today at **15,103 bytes** of code (`STACK:175`).
4. **What running bebop in the Worker would gain the product: nothing it does not already have**,
   except one thing — a **second, independent reader of the same bytes**, which is worth having and
   which does NOT need the language to run in wasm (§3.3).
5. **The honest boundary is the format.** bebop defines the bytes; Rust reads them where the request
   runs; bebop.bin reads them natively where it can. That is the tree's own design
   (`crates/bebop-store/Cargo.toml:5-11`, `kv.rs:9-10`) and this document confirms it (§4).
6. **Cost of the three routes to "bebop code in wasm"** (§5): an LLVM backend contradicts the
   language's zero-C law; a bebop→wasm emitter is ~5,600 lines of codegen to rewrite plus a host
   ABI for twelve syscalls (weeks: 8-16, hypothesis); an interpreter compiled to wasm is the cheapest
   (~2,000 lines, hypothesis; 60-120 KB, hypothesis, against wasm3's 64 KB) and buys code that runs
   an estimated 15-20x slower than bebop native. All three are **AGAINST** (§7).
7. **The router is not justified by the ETA error** (§6). Measured on three real Durrës pairs
   (OSRM): road/straight-line ratios **1.17, 1.56, 1.26**; but the kernel's `300 m/min` speed
   constant makes straight-line ETA already **over**-estimate travel (15 vs 9.8 min, 6 vs 3.6, 5 vs
   4.3). The error is the speed constant, not the geometry. A road graph for Durrës is **25,455
   nodes / 5,511 ways** (measured, Overpass), ~1.3 MB as a bebop image (arithmetic) — and no product
   surface would consume it.
8. **What was built (§9, item 1): `crates/bebop-wasm/`** — the format's reader as a standalone
   wasm32 module (**30,892 bytes, 12,650 gzipped**, measured) and a gate that makes **four readers of
   one image print one number**: bebop.bin natively, Rust natively, Rust-in-wasm32 under node, and
   Python. `n=5 root=-211109995167561145`, 4 of 4 (measured, §9). The no-SQL blueprint's F28
   four-way fold check was a document; it now runs.
9. **It found a divergence** on the first corrupted input: a KV image cut one cell short reads as
   its **previous generation** in Rust (`n=0`), is **refused** by the Python reader, and would be
   read at generation 2 with a zero cell by bebop's `st_open` (§3.3). Truncation semantics are not
   in the format; each reader chose. The Durable Object's meta-length check hides it in production
   (`hubdo.rs:404-409`); restores and imports do not have that check.
10. **Re-entry condition for the language in wasm:** a product surface that needs code a venue can
    EDIT and that must run at the edge (§7). None exists and the roadmap wants tables, not code.

---

## 1. Premises, re-verified

| Premise (STACK §9) | Verdict | Evidence |
|---|---|---|
| bebop is an integer-only AArch64 systems language | **HOLDS** | `bebop-lang/README.md:3-6` "for AArch64 … no C in the toolchain"; `docs/LANGUAGE.md:5-10` "compiled straight to AArch64 machine words … every value is a 64-bit integer"; `k1.bp` compiled and run here: `500000500000`, rc=0 (measured) |
| the compiler is written in bebop (`bebop.bp`) | **HOLDS** | `wc -l bebop.bp` = 9,122; 348 top-level `fn` (measured `grep -c '^fn '`) |
| the product uses the file format only; no bebop executes in the Worker | **HOLDS** | `workers/api/Cargo.toml:9` `crate-type = ["cdylib"]`, `wrangler.toml` `worker-build --release`; the only bebop dependency is `dowiz-hub → bebop-store` (`crates/dowiz-hub/Cargo.toml:12-13`) |
| a wasm32 target exists on this box | **HOLDS, off PATH** | `~/.rustup/toolchains/1.96.1-aarch64-unknown-linux-gnu/lib/rustlib/wasm32-unknown-unknown` present; `/usr/bin/cargo` is 1.93.1 without it — the first probe build failed with E0463 "target may not be installed" until `RUSTUP_TOOLCHAIN=1.96.1…` was set (measured) |
| the router has no product caller | **HOLDS** | `grep -rn "router::" kernel/src crates workers/api/src tools/native-spa-server/src` → `kernel/src/lib.rs:615` (`pub use`) and `span_metrics/instrument.rs:9-40` (a timing wrapper); nothing else (measured) |
| no road graph exists in any image | **HOLDS** | `find / -xdev -iname '*.osm*' -o -iname '*.pbf'` → nothing (measured; the `*durres*` hits are `web/sushi-durres`, a storefront) |
| "v2 (live): WASM codegen" | **FALSE** | `docs/design/BEBOP-BACKEND-ROADMAP.md:13-16` under a `SUPERSEDED (2026-09-04)` header; `find bebop-lang -iname '*wasm*'` → nothing; `LEGACY_BP_ANALYSIS.md:17` lists `wasm.bp+wasm_data.bp` among "70 dormant files … never loaded", since deleted |
| the four-way fold check exists | **DOCUMENT ONLY, until today** | `BLUEPRINT-NO-SQL-BEBOP-EVERYWHERE-2026-09-21.md:76-78, :325` names it (F28); the only code reference is a doc-comment `crates/dowiz-hub/src/table.rs:111`; `grep -rn "four-way\|fd11fc93f180ca47" bebop-lang/bench bebop-lang/tools crates tools/gates` → no script (measured). §9 item 1 makes it run |
| CI can run bebop.bin | **NO** | every job is `runs-on: ubuntu-latest` (`.github/workflows/ci.yml:6,19,115,124,138`) — x86_64; `bebop.bin` is AArch64. The bebop side of any parity check is box-only |

---

## 2. Q1 — What the compiler emits, and where the backend seam is

**There is no seam.** `bebop.bp` parses and emits in one pass; the emitter functions take the source
string and a position and write machine words into `insns` as they go:

- `emit_ident` (`bebop.bp:610`), `emit_bl` (`:1023`), `emit_apply_op` (`:4238`), `emit_array_get`
  (`:5003`), `emit_prologue_sized` (`:5590`), `emit_while_stmt` (`:6227`), `emit_body` (`:6697`),
  `emit_expr_words` (`:7706`), `emit_words` (`:7765`). Every one carries the signature
  `(s: str, pos: [i64], insns: [i64], n: [i64], stab, selfname, selfstart, fntab)` — the parser state
  and the instruction buffer are the same call.
- The words are AArch64 encodings as decimal constants at the emission site, e.g.
  `vs_array_get_imm` (`:5019-5030`): `ldr xd,[x17,xm,lsl #3]` is `4167072288 = 0xf8607a20`, the
  4-byte form `3093330464 = 0xb8607a20`, `add` is `2432696320 + idxc*1024 + base*32 + d`;
  `emit_half` (`:3109-3123`) assembles `movz/movk` from `4068474880 + sl*32 + hw*2097152 + d`.
- **Measured census:** 595 `em(insns` call sites; 194 functions named `emit_*` or `vs_*`;
  483 distinct 9-10-digit decimal literals (the opcode table, inlined); lines inside codegen-named
  functions 5,578 of 9,122 (`awk` over `^fn` names — approximate, since the split by name is not
  exact). `grep -c -i '\bwasm\b\|x86\|riscv' bebop.bp` → **0**. The word "target" occurs only as a
  branch target.
- **The register model is the ISA.** `bebop.bp:3006-3030`: values live in "the compile-time window
  x0..x7", callee-saved x19.., "an x15 frame slot", the arena base is x17 (`:5019`), the frame heap
  x14 and arena cursor x27/x28 (`LANGUAGE.md` "Memory model"). wasm32 has locals and an operand
  stack, no registers, no `csel`, no `svc`. The REGISTER-MODEL and IR-RUNG blueprints
  (`docs/IR-RUNG-BLUEPRINT.md:1-40`) chose an operand-TAG stack over an op-list IR precisely to
  avoid "the refactor every emit_*" cost (`DECISIONS-RESEARCH-2026-09-06.md:77`) — i.e. the project
  decided, on the record, NOT to build the intermediate form a second backend would need.
- **The project's own row for this is T94**: "WASM direct binary emitter (zero deps) + own
  interpreter", OPEN (`TASKS.md:103`; goal text `HISTORY.md:2309-2320`: "a second encoding table
  like T91 … this column exists for reach (browser hubs), not for the post-von-Neumann model").
  `ROADMAP-AUDIT-2026-09-04.md:158`: "not started".

**Verdict:** wasm32 is a rewrite of the emitter half of the compiler, on the bebop side, with a new
value model. It is not a backend parameter. This decides everything below.

---

## 3. Q2 — What subset would have to run, and what the product would gain

### 3.1 The subset, measured

The product stores two families of image: the chained event log (`crates/bebop-store/src/evlog.rs:12-33`)
and the sorted KV (`kv.rs:3-7`). On the bebop side the defining code is `selfhost/prelude/store.bp`
(superblock, PartTab, alloc, seal, commit, `st_pick :99-103`, `st_root :865-869`, `st_get :871`),
`selfhost/std/kv.bp` (117 lines, schema + `kv_snapshot :52`), and `selfhost/prelude/sha256.bp`
(layout digests). **The builtins that code calls, counted** (`grep -o` over the three files, measured):

| builtin | uses | wasm32 equivalent |
|---|---|---|
| `zeros(n)` | 15 | linear memory + a bump cursor — portable |
| `sys_exit` | 9 | host import |
| `sys_atomic_add` | 8 | wasm atomics behind a threads proposal; the Worker has one thread |
| `sys_open` / `sys_close` | 6 / 5 | none — a Worker has no file descriptors |
| `sys_mmap` / `sys_munmap` | 4 / 3 | none — the store IS an mmap (`LANG-DB-DESIGN.md` §4b); in a Worker the image is a `Vec<u8>` handed in by the object |
| `sys_msync` / `sys_fsync` / `sys_ftruncate` / `sys_rename` | 4 / 1 / 3 / 2 | none — durability is the Durable Object's PUT (`bebop-store/src/lib.rs:213-221` states the equivalence) |
| `sys_futex_wait` | 2 | none |
| `crc32x` / `crc32` | 4 / 1 | a loop (`bebop-store/src/lib.rs:36-46`) |

Twelve syscall builtins and one instruction-backed CRC. Every one of them is the reason
`bebop-store` exists: `lib.rs:99-105` "what lets the store live somewhere with no `open()`". A bebop
reader in wasm would need a host ABI that replaces all twelve with "here are the bytes", which is
what the Rust crate's `Store::from_bytes` (`lib.rs:131-176`) already is.

### 3.2 The fold, and where it lives

The order fold is not in bebop and not in `bebop-store`. `dowiz_hub::Hub` reads records
(`lib.rs:507` `events`, `:569` `orders`); the Worker folds deltas with a real JSON parser
(`workers/api/src/fold.rs:1-22`, `hubstore.rs:1268-1298` `orders_state`); the object memoises the
fold per generation (`hubdo.rs:203-210`, `:423-437`). A bebop fold would need JSON — `minijson`'s own
header says it is "never pointed at untrusted documents" (`fold.rs:17-22`), and bebop has no
runtime strings (`LANGUAGE.md:8-9`). **The fold is the wrong candidate.** The store walk is the only
candidate, and it is 200 lines of Rust already.

### 3.3 What the product would gain: a second reader — and it does not need the language in wasm

"The same bytes, read by a second implementation" is a real gain, and this lane can name what it
buys, because it built it and it paid on the first run:

- **The four-way check now runs** (§9 item 1): `bebop.bin` (kv.bp, native AArch64), Rust native,
  Rust in wasm32 under node, Python from the bytes alone — one fixture, `n=5`,
  `root=-211109995167561145`, 4 of 4 (measured). Before today the check was a sentence in two documents.
- **It exposed a divergence.** The fixture cut one cell below `arena_used` (measured,
  `crates/bebop-wasm/tests/parity.rs` "a_cut_below_the_arena_falls_back_to_the_previous_generation"):
  - Rust (`bebop-store`): `sb_fits` drops the superblock whose `arena_used` exceeds the image
    (`lib.rs:280-304`), `pick` takes the other one (`:306-325`) — **generation 1, the empty schema**
    that `kv.bp i` wrote. `Kv::load` succeeds with `n=0`. Every product KV image has this shape,
    because `compacted_bytes` (`kv.rs:272-283`) inits (gen 1, empty) then commits (gen 2, content).
  - Python (`crates/bebop-wasm/oracle.py`): picks the higher generation, sees it does not fit, **refuses**
    (status 1).
  - bebop (`store.bp:99-103` `st_pick`): validity by CRC and higher generation only; `st_open`
    `ftruncate`s the file up to size, so the missing cell reads as **zero at generation 2**.
    (Inferred from the code; not executed on the cut file because `kv.bin` extends its input in place.)
  Three readers, three answers. **The format does not define truncation; each reader chose.** In
  production the Durable Object refuses a short image before any reader sees it —
  `hubdo.rs:404-409` "image {id} is {} bytes, its meta says {}" — but `Table::load`
  (`crates/dowiz-hub/src/table.rs:94-101`) has no equivalent of the log's `chain_is_whole`
  (`lib.rs:766`, `logimage.rs:130-139`), so a restored or imported KV image that lost its tail
  reads as an empty roster with no error. **That is a finding about the seam, produced by the second
  reader, in one afternoon, at a cost of 30 KB of wasm — and none of it required the language to run.**

---

## 4. Q3 — The honest alternative: the format is the boundary

The tree already says it, in the crate that does the work:

- `crates/bebop-store/Cargo.toml:5-11`: "the format is POINTER-FREE by design … which is exactly what
  lets a second language read and write the same file. ZERO dependencies … It is the seam by which
  dowiz uses bebop as its database."
- `kv.rs:9-10`: "bebop creates the schema (the layout digests come from sha256, which stays on that
  side); this module reads and writes the data through the documented pointer-free format." The
  cross-language round trip was **exercised here** (measured): `kv.bin i` (bebop creates, 64 MiB) →
  `kvdemo` (Rust writes five entries, generation 2) → `kv.bin n` = `5`, `kv.bin h` =
  `-211109995167561145` = Rust's `snapshot_root_i64`. Both directions, one number.
- `BLUEPRINT-NO-SQL-BEBOP-EVERYWHERE-2026-09-21.md:79`: "Rust and wasm carry the request. bebop
  carries the truth about the bytes."

**This document confirms that boundary and sharpens it in two places:**

1. **The bebop side of the truth cannot run in CI** (§1, `ubuntu-latest`). The format's defining
   reader runs on this box and on a phone. A parity gate must say NOT MEASURED there, never pass.
2. **The boundary is missing a rule** — truncation (§3.3). The cheapest fix is one line in Rust:
   refuse a KV image whose live superblock's `arena_used` exceeds the bytes, the way the log family
   already does. Where that line goes (`bebop-store` or `Table::load`) is the main session's call;
   the test that pins today's behaviour is in `crates/bebop-wasm/tests/parity.rs`.

**Re-entry condition** for moving the language across the boundary: a Worker-side consumer that needs
**code** the venue edits (a rule, not a table), executed at the edge, verified by the compiler as
T94/C1 intend — and the roadmap's data-driven rows (price per channel B3, floor plan, recipe,
`ROADMAP-2026-09-22.md:109-111`) are tables, not code. See Document 2 §2.7.

---

## 5. Q4 — Cost, measured where it could be

**The budget.** Worker wasm today **2,426,605 bytes** (measured `ls -la workers/api/build/index_bg.wasm`,
2026-09-22 build); 904,804 gzipped (STACK §1.5). Storage stack **86 KB** of the 1.82 MB code section,
of which `bebop_store` is **15,103 B** (`STACK:175-180`). Any second runtime is judged against that
86 KB, and against the 1 s startup budget (`workers/api/Cargo.toml:49-51`).

| Route | What it is | Lines (measured / hypothesis) | Bundle bytes | Weeks (hypothesis) | What breaks |
|---|---|---|---|---|---|
| **(a) LLVM backend** | replace `bebop.bp`'s emitter with LLVM IR emission | needs the frontend re-hosted in a language that can link LLVM: the parser half of bebop.bp (~3,500 lines) rewritten | n/a — output is native or wasm | 8-12 | **the language's own law**: "no C in the toolchain" (`README.md:3`), zero deps, a 1.5 KB seed as trust root (`docs/TRUST-CHAIN.md`); `RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md:9` already refuses "a second compiler-sized, safety-critical artifact" |
| **(b) bebop→wasm emitter** (T94) | a second `em` table and a new value model in `bebop.bp` or a sibling file | codegen today 5,578 lines / 194 fns / 595 sites (measured); a wasm emitter for the same surface: 3,000-5,000 (hypothesis: no register model, but a control-flow-structured output — wasm has no `b.cond` to arbitrary words, so `emit_cond_branch :4708` / `emit_while_stmt :6227` must be re-derived as `block/loop/br_if`) | the emitted module for `store.bp+kv.bp`: ~20-40 KB (hypothesis from this crate's 30 KB doing the same job in Rust); plus the host ABI shim | 8-16 to a `std_golden`-equivalent subset; the tree's own tiering history (T96 register tier, `TASKS.md:105` PARTIAL after weeks) is the calibration | 800-line file cap (`AGENTS.md:348`) means 4-7 new files; every `.bp` change needs the three-generation fixpoint and the whole battery (`README.md:19-21`); nothing in the Worker consumes the output |
| **(c) interpreter compiled to wasm** | a Rust (or bebop-in-wasm, circular) interpreter of the bebop subset | `tools/bpref.py` is 886 lines of Python for the differential subset (measured); a Rust port 1,500-2,500 (hypothesis) | 60-120 KB (hypothesis; `wasm3`, the smallest serious wasm interpreter, is 64 KB — `RESEARCH-BYOK:70`); i.e. 0.7-1.4x the whole storage stack | 3-6 | speed: interpreters sit ~10x below JIT (`RESEARCH-BYOK:71`, OOPSLA'22) and wasm itself 45-55 % below native (`RESEARCH-LITERATURE-2026-09-08.md:9`) → bebop-in-interpreter-in-wasm ≈ 15-20x slower than bebop native (hypothesis, product of the two); `bpref.py` disagreed with the compiler for four days with nothing running it (`LANGUAGE.md:1`) — a second semantics is a second thing to keep honest |
| **(d) the format's reader in wasm** — BUILT | `crates/bebop-wasm` | 110 lines `lib.rs` + 109 `abi.rs` (measured `wc -l`) | **30,892 B raw / 12,650 B gzip** standalone (measured); predicted Worker delta ≤ 16 KB raw (hypothesis: `bebop_store`'s 15,103 B of bodies are already linked; command to settle it in §10) | done in one lane-day | nothing; zero new dependencies |

**What the wasm probe measured on the way** (`scratchpad/probe_wasm`, not in the tree): a cdylib that
exports only `EvLog::len` over `bebop-store` builds to **21,710 B** with `opt-level="z"`, `lto`,
`panic="abort"` (measured). That is the floor for "bebop-store in a module"; the 30,892 B above adds
the KV fold, the log fold, the allocator pair and the status surface.

---

## 6. Q5 — The router

**What exists.** `crates/dowiz-core/src/router.rs:1-14`: "CSR-native Dijkstra / A* … Contraction-
Hierarchy shortcuts + OSM road-graph ingestion", ported from the bebop repo's `cost_estimate.rs`;
`route(g, src, dst, heuristic, shortcuts)` at `:91`, `road_graph_from_ways(nodes, ways)` at
`:224`. 340 lines, five tests, **f64 weights and `haversine_meters`** (`:21-22, :55`) — it is on the
float side of MANIFESTO C2 (`MANIFESTO.md:15`), which the product's own ETA code avoids on purpose
(`crates/dowiz-hub/src/zone.rs:9-20` "INTEGER ARITHMETIC THROUGHOUT … THE EARTH IS FLAT HERE";
`crates/dowiz-core/src/eta.rs:285-318` `straight_line_m` in micro-degrees with a fixed-point cosine).

**What it would replace.** `eta.rs:219-224` `estimate(items, ahead, distance_m, k)`; `:203-211`
`travel_minutes = ceil(distance_m / courier_speed_m_per_min)`; the default profile
`courier_speed_m_per_min: 300` (`:85`), `pickup_min: 5`, `handover_min: 3`, `spread_pct: 50` (`:83-86`).
The docstring already says: "a caller with a routing service should pass that instead" (`:291-293`).

**Is straight-line wrong enough? Measured, on three real Durrës pairs.** Origin 41.3231 N 19.4410 E
(the coordinates `eta.rs:515` tests with); OSRM public `route/v1/driving` (2026-09-23);
straight line by a Python replica of `straight_line_m` (`eta.rs:294-318`, integer, same tables):

| destination | straight (kernel) | road (OSRM) | road/straight | OSRM time | kernel travel @300 m/min | note |
|---|---|---|---|---|---|---|
| 41.3050 N 19.4890 E (beach road, SE) | 4,409 m | 5,172 m | **1.17** | 9.8 min | 15 min | kernel over by 5 min |
| 41.3320 N 19.4550 E (NE) | 1,512 m | 2,356 m | **1.56** | 3.6 min | 6 min | kernel over by 2.4 min |
| 41.3150 N 19.4300 E (old town, SW) | 1,272 m | 1,606 m | **1.26** | 4.3 min | 5 min | kernel over by 0.7 min |

Two things the numbers say. (1) **Road is 17-56 % longer than the line** — the detour factor is real
and varies with direction (the NE pair crosses a rail line and the port road). (2) **The kernel's
travel estimate is already ABOVE the routed time in all three cases**, because 300 m/min is 18 km/h
and OSRM's car speeds came out at 22-39 km/h. The straight-line error and the speed error have
opposite signs and the second is larger. A router would fix the smaller one and leave the estimate
still wrong unless the speed constant is calibrated too — and **calibrating the constant needs no
graph**: the log holds `Placed` and `Delivered` timestamps per order (`dowiz_hub::EventKind`,
`lib.rs:102-130`) and the storefront has both coordinates; a per-venue `m/min` and detour factor
folded from delivered orders is a table row, not an algorithm. (Hypothesis until folded: the tree
has no such fold today; §9 item 4 names it.)

**Where would a road graph come from, and what would it cost?** Overpass, bbox 41.28-41.36 N ×
19.40-19.52 E (≈9 × 10 km around Durrës), measured 2026-09-23: **5,511 `highway` ways, 25,455
nodes**. As CSR in a bebop image (arithmetic on the measured counts; edge count is a hypothesis of
~2 × (nodes − ways) ≈ 40 k directed): `row_ptr` 25 k cells + `col_idx` 40 k + `val` 40 k + coords
51 k ≈ **156 k cells ≈ 1.25 MB per venue region**, before any CH shortcuts. Ingestion is an OSM→CSR
pipeline the tree does not have (`router.rs:239-240` "OSM parsing itself is a downstream (Phase 13)
concern"). A Dijkstra over 25 k nodes is sub-millisecond — the cost is the pipeline, the image, and a
float router on the decision path, not the query.

**Verdict:** AGAINST giving the router a caller now (§7). The measured ETA error is dominated by a
constant that the log can calibrate for free; the road-graph pipeline is real work that would ship
a float algorithm onto a path the kernel keeps integer. Re-entry: after calibration, if the residual
per-direction error on delivered orders exceeds the spread the venue quotes (`spread_pct`), a
per-zone detour table (Document 2 §2.7) is the next cheapest step, and a graph the one after.

---

## 7. Recommended AGAINST, with re-entry conditions

| # | Against | Why | What would change my mind |
|---|---|---|---|
| A1 | An LLVM backend for bebop | contradicts the language's zero-C, seed-as-trust-root law (`README.md:3`, `TRUST-CHAIN.md`); re-hosts the frontend; `RESEARCH-BYOK:9,83` refuses the class | the operator changes D0 for bebop-lang; not a dowiz decision |
| A2 | A bebop→wasm emitter (T94) built FOR the Worker | 5,578 lines of AArch64-shaped codegen to re-derive (§2, §5b), a host ABI for twelve syscalls (§3.1), and no consumer: the code that would run (store walk) is 200 lines of Rust already in the bundle | a Worker-side consumer of venue-EDITED code (a rule engine), or T94 landing on bebop-lang's own roadmap for its own reasons ("browser hubs", `HISTORY.md:2316`) — then the Worker only loads a `.wasm` it did not build |
| A3 | A bebop interpreter in the Worker | 60-120 KB (hyp.) for code 15-20x slower (hyp.) than native, a second semantics to keep honest (`LANGUAGE.md:1` records bpref drifting for four days) | same as A2, plus a measured turn budget that has room for interpretation |
| A4 | Rewriting the fold in bebop | the fold needs JSON over untrusted text (`fold.rs:17-22`); bebop has no runtime strings (`LANGUAGE.md:8-9`; A7/A8 rows pending) | A7 step 2 + A8 tag 7 land and a bebop JSON reader passes the Worker's fold tests byte-for-byte |
| A5 | Giving `router.rs` a product caller | §6: the ETA error is the speed constant; the router is f64 on a C2 path; the graph pipeline does not exist | calibrated per-venue speed + detour still misses the quoted spread on delivered orders; then a per-zone detour table first, a graph second |
| A6 | Making the bebop side of the parity gate a CI requirement | CI is x86_64 (§1); a gate that skips on the runner it runs on measures nothing there | an `ubuntu-24.04-arm` (or self-hosted) runner in `ci.yml` — then the gate's "NOT MEASURED" becomes a fourth column |

---

## 8. Gates (ratchets that may only fall, proved both ways)

| Gate | Value | How it fails | Proved |
|---|---|---|---|
| G1 `crates/bebop-wasm/gate.sh` — **four readers, one number** | `readers agreeing with kv.expected: 4 of 4` on aarch64; `3 of 4` + `bebop.bin: NOT MEASURED` elsewhere; RED below 3 | any reader prints a different `n`/`root`, or fewer than three run | `gate.sh --prove` flips one payload bit in a copy → `kv status=2 n=0 root=0`, rc=1 (measured) |
| G2 `crates/bebop-wasm/bytes.baseline` — **the reader's wasm cost** | `30892` | the module grows; a fall is announced for the baseline to be lowered in the same commit | first run wrote the baseline; a growth run would print `REFUSED -- the module grew` (the branch exists; not exercised — see §10) |
| G3 (proposed, main session) `Table::load` refuses a short KV image | `Err(NotAHub)` on the fixture cut by one cell | reads as generation 1 with `n=0` | the test `a_cut_below_the_arena_falls_back_to_the_previous_generation` pins today's behaviour and must be INVERTED in the commit that adds the check |
| G4 (proposed) ETA calibration fold | per venue: `m_per_min`, `detour_e3` from delivered orders; the gate is `|quoted − actual| ≤ spread` on the last N deliveries | the miss rate rises | needs the fold (§9 item 4); not built |

---

## 9. Order of work, each with its CHECK

### Item 1 — BUILT: the format's reader in wasm32, and the four-way gate (`crates/bebop-wasm/`)

- **Files (all new, this lane's):** `Cargo.toml` (cdylib+rlib, dep `bebop-store` only, Worker's
  profile with `panic="abort"`, `strip=true`), `src/lib.rs` (110 lines: `kv_view`, `log_view`,
  `Refusal`), `src/abi.rs` (109 lines: `bw_alloc/bw_free/bw_kv/bw_log/bw_abi_version`, status codes),
  `tests/parity.rs` (six tests), `fixtures/kv.store` (9,360 B: bebop-created schema, Rust-written
  entries, trimmed at `arena_used`=1170 cells), `fixtures/kv.expected` (what `kv.bin n`/`h` printed),
  `harness.mjs` (node, no imports), `oracle.py` (Python reader from the bytes), `gate.sh`,
  `bytes.baseline`.
- **RED before GREEN, measured:** the test file against a stubbed reader (a scratch copy with
  `kv_view`/`log_view` returning `Err`): `cargo test` rc=101, **5 failed, 1 passed** (the one that
  passes on a stub is the refusal test, which is the honest shape of that test). Against the real
  reader: rc=0, **6 passed**.
- **The four readers (measured, `sh gate.sh`, rc=0):**
  `native: cargo test rc=0` · `wasm32: 30892 bytes (12650 gzip)` · `node -> 'kv status=0 n=5
  root=-211109995167561145' rc=0` · `python: 'kv status=0 n=5 root=-211109995167561145' rc=0` ·
  `bebop.bin: kv.bin n -> '5', kv.bin h -> '-211109995167561145'` · `GREEN (4 readers, 30892 bytes)`.
- **Bytes:** 30,892 raw, 12,650 gzip, 145 functions, exports `memory, bw_abi_version, bw_alloc,
  bw_free, bw_kv, bw_log` (measured `wasm2wat`).
- **CHECK:** `cd crates/bebop-wasm && sh gate.sh` prints `GREEN (4 readers, …)` on this box and
  `GREEN (3 readers, …)` with `bebop.bin: NOT MEASURED` on x86; `sh gate.sh --prove` prints
  `the number moved`.

### Item 2 — wire the reader into the Worker's rebuild (main session's call; the change as TEXT)

The object already refolds the log from bytes and diffs it against the memo (`hubdo.rs:873-911`,
`rebuild.rs:1-22`). Adding the second reader's number to that report makes F28 run in production on
every rebuild, not only on a fixture:

- `workers/api/Cargo.toml`, under `[dependencies]`:
  `bebop-wasm = { path = "../../crates/bebop-wasm" }`
- `workers/api/src/rebuild.rs`, in `Report`: `pub kv_root: Option<i64>,` — the `bebop_wasm::kv_view`
  root of the catalogue image, so an operator can compare it with `kv.bin h` on a downloaded copy.
- `workers/api/src/hubdo.rs`, in `rebuild()` after the stock image match: 
  `let kv_root = self.image(CATALOG_IMAGE).await?.and_then(|(_, b)| bebop_wasm::kv_view(&b).ok()).map(|v| v.root);`
- **Bundle delta:** hypothesis ≤ 16 KB raw (`bebop_store` already linked; this crate adds the fold
  and the status surface, and the `abi` module is dead code the linker drops when only `kv_view` is
  called). **Measure with:** `cd workers/api && worker-build --release && ls -l build/index_bg.wasm`
  before and after the two lines; report both numbers, as `STACK` §1.5 did.
- **CHECK:** `/fold/rebuild` on the live venue reports `kv_root`; `kv.bin h` on the same image
  downloaded from the object prints the same integer.

### Item 3 — the truncation rule (main session's call; one line, one inverted test)

Refuse a KV image whose live superblock's `arena_used` exceeds the bytes, in `Table::load` or in
`bebop-store::pick` (the latter changes the log family too and must be checked against
`chain_is_whole`). **CHECK:** invert `a_cut_below_the_arena_falls_back_to_the_previous_generation`
to expect `Err`, and `oracle.py` and Rust then agree on the cut image (G1 extended with a second
fixture, `fixtures/kv.cut.store`).

### Item 4 — ETA calibration from the log, before any router

A fold over `Placed`→`Delivered` pairs with both coordinates: per venue `m_per_min` and a detour
factor, written as settings keys the venue can override. **CHECK:** on the three Durrës pairs of §6
the calibrated estimate lands within `spread_pct` of the OSRM time; and G4 runs.

### Item 5 — an ARM runner for the bebop side of G1 (infrastructure; owner: whoever owns `ci.yml`)

**CHECK:** `gate.sh` prints `4 of 4` in CI.

### Items deferred, with the row that owns them

T94 (bebop→wasm emitter) stays on bebop-lang's roadmap for bebop-lang's reasons; A2 is its re-entry
here. The router waits on item 4's numbers.

---

## 10. What was NOT determined, and the command that would settle it

| Claim | Status | Command |
|---|---|---|
| Worker bundle delta of wiring `bebop-wasm` (item 2) | **hypothesis** ≤ 16 KB | add the two lines of item 2; `cd workers/api && worker-build --release; ls -l build/index_bg.wasm`; diff against 2,426,605 |
| G2's growth branch refuses | branch written, not exercised | add ten lines to `abi.rs`, run `sh gate.sh`, expect `REFUSED -- the module grew`, revert |
| bebop's own reading of the one-cell-cut image | inferred from `st_pick`/`st_open`; not run, because `kv.bin` `ftruncate`s its input to 64 MiB and would read zeros for the missing cell | copy the cut fixture to a scratch dir as `kv.store`, run `kv.bin n` and `kv.bin h`, and — the real test — `st_verify` on it |
| The emitter-route line count (§5b) | hypothesis from the AArch64 codegen census | not settleable without building it; the calibration is T96's history in `HISTORY.md` |
| Interpreter route bytes and speed (§5c) | hypotheses from `wasm3` and two cited papers | port `bpref.py`'s subset to Rust behind a feature in a scratch copy of this crate; build; time `k1.bp` |
| OSRM's car profile vs a scooter courier | OSRM `driving` used; a courier on a scooter is faster in Durrës traffic (hypothesis) | fold the log (item 4) — the venue's own deliveries are the only oracle that matters |
| Edge count of the Durrës graph | hypothesis (~40 k directed) from way/node counts | Overpass `way["highway"](bbox);out geom;` and count segments |
| Whether any restore/import path hands `Table::load` bytes without a meta length | not traced (the S3 nightly copy and `import.rs` were not read for this) | `grep -n "Table::load\|LogImage::load" workers/api/src/*.rs` and read each caller's source of bytes |

---

## 11. Corrections to the framing this lane was given

1. `docs/design/BEBOP-BACKEND-ROADMAP.md:13-16` "v2 (live): WASM" describes code that is not in the
   tree; it should say so under its SUPERSEDED header or be deleted.
2. The four-way fold check (`NO-SQL:76-78, :325`; `table.rs:111`) was not a check; it is now
   `crates/bebop-wasm/gate.sh`, and its bebop column can only be measured on aarch64.
3. "The Worker's storage stack is 86 KB" is right, and the reader alone is 30 KB standalone; the
   two are not additive (§5d).
4. The card asked whether straight-line ETA is "wrong enough to justify" the router. Measured: it is
   wrong, in the direction a router does not fix (§6).
5. The `wasm32-unknown-unknown` target is present but only in the rustup toolchain the repo pins;
   any script that uses `cargo` from PATH will conclude wasm is impossible, as the first probe did.
   `gate.sh` takes `$HOME/.cargo/bin/cargo` explicitly for that reason.
