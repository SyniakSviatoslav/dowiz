# SPEC: the bebop DAG runtime (normative), 2026-09-28

**Status.** Normative specification, written by lane W-SPEC (docs only, tree `f334e5cb`) from the research
`docs/research/2026-09-28-bebop-dag.md` (cited below as **R §n**) and the operator's decisions of 2026-09-28
(one jump, no staged migration, no second runtime, Datalog mandatory, tensors/codec). Companion documents:
`docs/design/SPEC-DATALOG-AND-CODEC-2026-09-28.md` (the rule layer and the block codec) and
`docs/design/BLUEPRINT-BEBOP-DAG-2026-09-28.md` (lanes, rows, acceptance numbers, the switch checklist).
Roadmap rows: `bebop-lang/ROADMAP.md` Phase G (`DG*`) and `docs/design/ROADMAP-2026-09-22.md` Wave DAG (`DW*`).

**How to read it.** MUST / MUST NOT are binding on every coding lane; SHOULD is the default a lane deviates
from only with a measured reason in its verdict; MAY is permitted. **Every MUST names the test or gate that
checks it** (§11 is the matrix). Numbers carry the research's labels: **MEASURED** (a command run, its output
quoted, R §n), **DOC** (a vendor page), **EST** (arithmetic on measured inputs), **HYPOTHESIS** (not measured).
Line numbers are from `bebop-lang/bebop.bp` at 9,122 lines (`f334e5cb`); a lane re-derives them with `grep -n
'^fn <name>'` before editing, because they drift.

**Binding decisions this spec implements without re-deciding.**

| # | Decision (operator, 2026-09-28) | Where it binds |
|---|---|---|
| D-1 | bebop is rewritten onto a full DAG runtime: compile nodes, object nodes, projection nodes, effect nodes; never expression nodes (R §7.1, refuted 41x by `bebop-lang/bench/substrate_spike/RESULT.md`) | §1 |
| D-2 | ONE JUMP: the driver's memo, the store's read path and the worker pool are replaced in one commit; no staged migration, no parallel copy, no second runtime in the tree at any moment (R §8) | §10, BLUEPRINT Wave 1 |
| D-3 | Datalog is mandatory on the runtime's edges (R §12.1) | companion spec Part A |
| D-4 | prices/stock as integer tensors, everything computes as tensors via a universal codec (R §12.2, §13) | companion spec Part B; the dense-SIMD form is a GATED row (BLUEPRINT DG25, OPERATOR DECISION NEEDED) because R §12.3 MEASURED it 8x slower than the sparse scalar form |
| D-5 | the honesty floor stands: a memo hit MUST be provably equal to a recompute (`.becache`'s "exact bytes, no hash" rule, `bebop.bp:7940-7948`); the `dagfull` gate is the permanent proof (R §7.5) | §2.4, §8 |

Where the research and the operator disagree, both are recorded (§12) and nothing is silently picked.

**Path convention.** A backticked path is a file in the tree at `f334e5cb` (checkable with `ls`); a path written `+like/this.rs` is a file the named row CREATES and does not exist yet, so `tools/gates/paths.sh`'s rule (a cited path must exist) is not fooled by a plan.

---

## 1. Node kinds and the graph

### 1.1 Definitions

- **Node**: a pure function from its inputs to one output, identified by a **key** (§2). Four kinds:

| Kind | Input(s) | Output | Owner file after the jump |
|---|---|---|---|
| **compile node** | one top-level `fn`'s source span, the signature table, the literal table, the compiler's code version | position-independent words + relocation list + facts (§5) | `+bebop-lang/selfhost/prelude/dagc.bp` (new; the driver code that leaves `bebop.bp`, R §5 "Code size") + `bebop-lang/bebop.bp` driver |
| **object node** | bytes | a store object with `h0/h1` header (`bebop-lang/selfhost/prelude/store.bp:3-5`) | unchanged: the store IS a DAG of objects already (R §1.1) |
| **projection node** | input object nodes at stated generations | one output object | `bebop-lang/selfhost/prelude/store.bp` (`st_proj_*`, §6) + `bebop-lang/selfhost/std/kv.bp` + Rust twin `+crates/bebop-store/src/proj.rs` (new) |
| **effect node** | inputs + the clock as a parameter | exactly one appended record (outbox pattern) | dowiz side: `workers/api/src/outbox.rs` shape, unchanged by the jump |

- **Edge**: "node B reads node A's output". Edges are DECLARED, never inferred at run time: a compile node's
  edges are its call sites and literal uses (the compiler sees them); a projection node's edges are its
  parameter list; a Datalog rule's edges are its body atoms.
- **Node index**: the position of a node in the Kahn lowest-index-first topological order of the declared
  edges (`bebop-lang/selfhost/std/ordfsm.bp` `kahn`, line 208, already computes this order for the FSM).
  Compile nodes: source order of `fn` declarations (the order `collect_fns` at `bebop.bp:7081` produces) —
  this order is already the layout order, and MUST stay so (§5.3).
- **Generation**: the store's commit counter (superblock cell 2). A projection node's inputs are named with
  the generation they were read at.
- **Dirty set**: the nodes downstream of an input whose generation moved since the memo was written.

### 1.2 MUSTs

- **N-1** A node MUST be one of the four kinds. Expression-level nodes and "cell substrates" MUST NOT be
  built (MEASURED 41x, R §1.1 last row). Check: code review + `+bebop-lang/bench/vs_rust/dagfull.sh` has no arm for them.
- **N-2** Every node's output MUST be a function of its declared inputs and the code version only. Check:
  `dagfull` (§8) — warm output == cold output, byte for byte, per node.
- **N-3** Node granularity for the compiler MUST be the top-level `fn` (bebop has no nested fns:
  `bebop-lang/tools/arch_check.py` `check_no_nested_fn`). Check: `arch_check` + the compile arm of `dagfull`.
- **N-4** A chain-shaped computation (one ready node per level) SHOULD be ONE node (MEASURED 0.85x when
  scheduled as 24 nodes, R §3.3). Check: the scheduler's level report (§7.5) prints levels of width 1.

---

## 2. The node key (the 40-line key spec of R §8.2 — byte-exact)

### 2.1 Frame

Every key is a hash over ONE byte string, the **key frame**, built the same way for every node kind and by
every reader (bebop, Rust, Python, wasm32 — the four-reader discipline of `crates/bebop-wasm/gate.sh`):

```
frame      := tag(1 byte) ‖ field*                          (fields in the FIXED order of §2.3)
field      := len(8 bytes, u64 little-endian) ‖ bytes(len)  (a length-prefixed byte string; len MAY be 0)
```

There is no separator, no padding, no text. A number is encoded as an 8-byte little-endian i64 field
(`len = 8`). A 32-byte digest is a field with `len = 32`. A list is a field whose bytes are the
concatenation of its elements' fields (so a list of digests is `8‖32‖d0‖8‖32‖d1…` — lengths inside lengths,
which is what makes a frame unambiguous without a count).

### 2.2 Two hashes, one rule for which

| Purpose | Hash | Why (MEASURED) |
|---|---|---|
| **In-process memo INDEX** (the `<out>.dag` memo, a store's projection table, the scheduler's ready set) | **`K64 = (crc32x(frame) << 32) \| (frame_len & 0xffffffff)`**, one i64 cell; `crc32x` is the hardware builtin over the little-endian bytes of cells (`bebop-lang/docs/LANGUAGE.md`, builtins table) | 4 MiB in ≈ 1 ms ⇒ ≥ 4 GB/s; the whole `bebop.bp` keys in 0.3 ms (R §2.4). Software sha256 is 19-21 MB/s ⇒ 23 ms for 348 fns, affordable against a 14.5 s miss and NOT against a 160 ms hit |
| **Cross-machine / CAS name** (a block sent to a Box or a browser, a `cas://` module, an archived projection) | **`K256 = sha256(frame)`**, 4 cells / 32 bytes; `sha256_words` (`bebop-lang/selfhost/prelude/sha256.bp:67`) on the bebop side, `sha2` on the Rust side (`crates/dowiz-hub/src/lib.rs:864` `content_id_chained` already is sha256) | 1.55 µs per 330 B event, 30 µs per 7.5 KB block (R §13.2); the store's own rule bans FNV for content addressing (T80, `bebop-lang/docs/LANG-DB-DESIGN.md:163`) |

**K-1** A `K64` is an INDEX, never a proof. A memo hit under `K64` MUST be confirmed by **byte-equality of
the memoised input bytes** against the live input bytes (the `.becache` honesty floor generalised: exact
bytes, so a hit cannot collide). A `K64` collision with unequal bytes is a MISS and the entry is replaced.
Check: `dagfull` compile arm + unit test `dagc_key_collision` (two fn spans with a forced equal `K64` — the
test constructs them by appending bytes until `crc32x` repeats within the test's budget, or, if that is
impractical, by patching the memo image's key cell; the second compile MUST re-emit, not reuse).
**K-2** A `K256` name MUST be verified on receipt (`cas_verify`, `bebop.bp:8205`, exit 88 when the sha256
differs from the name). Check: construct `c51_casbad` (exists) + the block twin's refusal test (companion §B.6).

### 2.3 Fields per node kind (fixed order; nothing else goes in)

**Compile node** (tag `0x43` `'C'`):

| # | Field | Bytes | Source |
|---|---|---|---|
| 1 | `compiler_digest` | 8: `(crc32x(bebop.bin bytes) << 32) \| (len(bebop.bin) & 0xffffffff)` | the compiler reads its own bytes (argv[1], as `.becache` does at `bebop.bp:7958`) |
| 2 | `fn_source` | the fn's source span, from the byte after the previous fn's closing `}` (or the start of the expanded source) to and including this fn's closing `}` — comments between fns belong to the FOLLOWING fn | `collect_fns` positions (`bebop.bp:7081`) |
| 3 | `sig_digest` | 8: `crc32x`-pair over the frame `[ for each fn in index order: name_hash(8) ‖ arity(8) ‖ class(8) ] ‖ [ for each enum ctor: name_hash(8) ‖ tag(8) ‖ payload_arity(8) ]` | the name zone of `fntab`, `collect_ctors` (`bebop.bp:1262`); `class` is §3's purity class (0 pure, 1 io, 2 sched) |
| 4 | `lit_digest` | 8: `crc32x`-pair over the literal pool bytes in pool order | `scan_literals` (`bebop.bp:7411`) |
| 5 | `use_digest` | 8: `crc32x`-pair over the `seen` table of `use_scan` (`bebop.bp:8238`) in expansion order | the module set; a changed dependency invalidates every fn (conservative, like A10's rule for adds/removes) |

A fn's words depend on nothing else (audit in R §2.3: `use_expand`, tables, pass 1 per fn, prefix sums,
pass 2 per fn, literals, entry stub). The prefix-sum `starts` are deliberately NOT a field: §5 removes the
dependency by relocation. **K-3** Adding a field to this table is a format change: the memo image version
(§4) MUST bump and `dagfull` MUST be re-run cold. Check: `dagc_key_fields` unit test asserts the frame length
for a fixture fn equals `1 + 5*8 + 8 + len(fn_source) + 8 + 8 + 8` (derived in the test, never a constant).

**Object node** (tag `0x4f` `'O'`): `layout_digest(8) ‖ payload(LE bytes of the payload cells)`. Its `K64`
IS already in the object header: `h0 = layout_digest_lo32 << 32 | length`, `h1 = crc32(payload) << 32 |
generation` (`store.bp:3-5`); the store's crc is the index and the payload bytes are the proof. `K256` is
computed only when the object crosses a machine.

**Projection node** (tag `0x50` `'P'`):

| # | Field | Bytes |
|---|---|---|
| 1 | `code_version` | 8: the running program's `compiler_digest`-shaped pair over its own `.bin` bytes (`sys_arena_base()`-independent: the image bytes as loaded) |
| 2 | `fn_key` | 8: the projection fn's own compile-node `K64` (so a changed projection fn invalidates only its projections) |
| 3 | `inputs` | list of `(input_node_key(8 or 32) ‖ generation(8))` in parameter order |
| 4 | `params` | list of the scalar parameters as 8-byte fields, in order (the clock, a locale id, a window) |

**Effect node** (tag `0x45` `'E'`): as a projection node; the output is the record's content id
(`content_id_chained`, already a `K256`).

### 2.4 What the key kills, structurally

`compiler_digest` / `code_version` in every key ends "memo survived a deploy" and "bebop.bin != source"
(two real incidents: memory `bebop-binary-must-match-source`; R §7.3). **K-4** A reader MUST refuse a memo
entry whose `compiler_digest`/`code_version` differs from its own, never fall back to it. Check: `dagfull`
mutation proof step 2 (§8.3) — a memo written by a patched compiler MUST produce 0 hits.

---

## 3. Purity model and the `pure` refusal

### 3.1 Classes

Every top-level fn has exactly one class, computed in the collect pass (like `kernel_marked`, `bebop.bp:172`,
which sets `fntab[5543]`), so forward references resolve before emission:

| Class | Keyword | May call | May use builtins |
|---|---|---|---|
| **pure** | `pure fn f(...)` | pure fns only | `zeros`, `zeros32`, `str_len`, `char`, `clz`, `smulh`, `umulh`, `crc32`, `crc32x`, `crc32b`, `hvham`, `hvham2`, arithmetic, `[i64]`/`[u32]` reads and writes to DECLARED outputs (§3.2) |
| **sched** | `sched fn f(...)` | pure and sched fns | pure builtins + `sys_clone`, `sys_cond_set`, `sys_futex_wait_guard`, `sys_futex_wake`, `sys_atomic_add`, `sys_exit_thread_guard`, `sys_setaffinity`, `clock_ms` (for the level report only) |
| **io** | `fn f(...)` (no keyword; today's default) | anything | anything |

`kernel fn` (C1 dialect, exit 102) is unchanged and orthogonal: it forbids `sys_` names inside a kernel; a
`kernel fn` MAY also be `pure`.

**P-1** The compiler MUST refuse, at compile time, with **exit 124** and the diagnostic
`<line>:<col>: error[E124]: <name> is not pure -- <reason>` (the `diag_exit` shape at `bebop.bp:86`, one
`diag_str` arm per code as A17 requires), any of: a `pure` fn naming a `sys_*` builtin or `clock_ms`; a
`pure` fn calling a fn whose class is io or sched; a `sched` fn calling an io fn; a `pure` fn writing an array
parameter not declared as an output (§3.2). The reason text names the offending call or builtin. Check:
`+bebop-lang/bench/parity_constructs/neg/c1xx_pure_sys.bp` (`// EXPECT exit:124`), `+bebop-lang/bench/parity_constructs/neg/c1xx_pure_calls_io.bp`,
`+bebop-lang/bench/parity_constructs/neg/c1xx_pure_writes_input.bp`, and their positive twins `c1xx_pure_ok.bp` (numbers above the highest
construct in use, `c146` as of A16 — grep first); `bebop-lang/formal/Bebop/Reject.lean` gains rule 124 (§9).
**P-2** The class MUST be part of `sig_digest` (§2.3 field 3), so reclassifying a callee invalidates its
callers. Check: `dagc_key_fields`.

### 3.2 Inputs, outputs, allocation, the clock (R §7.2)

- A `[i64]`/`[u32]` parameter a `pure` fn only reads is an INPUT. One it writes is an OUTPUT and MUST be
  declared with the `out` marker in the signature: `pure fn fold_step(acc: out [i64], rec: [i64]) -> i64`.
  The A8 index-typing walk already distinguishes reads from writes per array symbol; the check is: a store
  through a parameter symbol without `out` inside a `pure` fn → E124. Check: `+bebop-lang/bench/parity_constructs/neg/c1xx_pure_writes_input.bp`.
- **Allocation is node-local.** A `pure` fn MUST leave the arena cursor where it found it: the prologue saves
  `x27/x28` (T126 does this in 9-symbol prologues) and the epilogue restores them; anything the node returns
  MUST be copied into a caller-provided `out` buffer or returned as a scalar. Check: construct
  `c1xx_pure_arena.bp` — `sys_arena_base()`-relative cursor read before and after a `pure` call is equal
  (the test is an `io fn main` calling the pure fn; `// EXPECT 0`).
- **The clock is an input.** `clock_ms()` is `io`/`sched`; a projection needing time takes `now_ms: i64` as
  a parameter (the dowiz `Req { now_ms }` rule, `docs/research/2026-09-27-dag-architecture.md` §1.3). Check:
  `+bebop-lang/bench/parity_constructs/neg/c1xx_pure_sys.bp` covers `clock_ms`.
- One-cell mutables (`let x = x + 1`) are local rebinding and never escape (`bebop-lang/docs/LANGUAGE.md`
  "if is an expression", memory `bebop-if-is-an-expression`); they need no rule.

**P-3** Every node fn the runtime memoises — every compile-node emitter path is exempt (it is the compiler
itself, `io`), but every PROJECTION fn and every Datalog rule fn — MUST be `pure`. The store's `st_proj_eval`
(§6) MUST refuse a non-pure fn value at registration with **trap 124** at run time (the same number, the
run-time twin, printed as `trap 124: projection fn is not pure` on stderr). Check: `std_golden` gate
`sproj_neg` (`EXPECT exit:124`) and its positive twin `sproj`.

---

## 4. The memo image `<out>.dag`

### 4.1 Semantics

`<out>.dag` is the compile-node memo, written beside the OUTPUT (never beside the source — the `cli_check`
incident of R "Incident, 04:32": `<src>.use` written beside the source rewrote a tracked file). It is a bebop
store image (`store.bp` superblocks + arena, so `st_open`/`st_pick`/`st_reopen_verify` and the Rust and Python
readers open it unchanged) with a **KV meaning** — sorted keys, one value per key, a root fold — and a
**packed layout**, not `kv.bp`'s byte-per-cell one. R §7.3 says "the memo store IS a bebop KV image
(`kv.bp`)"; this spec keeps the KV semantics and rejects the `kv.bp` layout on R's own numbers: one byte per
cell is an 8x amplification and `kv.bp`'s whole-image rewrite costs 42.8 ms per put on a 165-entry image
(R §4), against a hit path of 156-174 ms (R §2.1). Recorded in §12 as conflict C-3.

### 4.2 Layout (all cells i64 LE; every object carries the standard `h0/h1` header)

```
root  DAGROOT  {magic, version, compiler_hi, compiler_lo, n, ref IDX, gen, cap_cells}      8 cells
      magic       = 0x314f4d4741440000 ('\0\0DAGMO1' little-endian; derived in source as a sum, never a literal)
      version     = 1
      compiler_hi = crc32x(bebop.bin bytes)      compiler_lo = len(bebop.bin)
      n           = number of live entries       gen = this image's own commit generation (superblock cell 2)
      cap_cells   = arena capacity the image was created with (derived: see §4.4)
IDX   {3 cells per entry, SORTED by key64}: key64, ref ENTRY, last_hit_gen
ENTRY {key64, kind, src_len, nwords, nrelocs, facts, size_words, ref SRC, ref WORDS, ref RELOCS, cost_us}   11 cells
      kind = 1 compile node (the only kind in v1); facts = the fn's `factbuf` word (vc*2+has_alloc, bebop.bp:7541 comment)
      size_words = pass-1 size (so a hit skips BOTH passes, A10 §3)
SRC   ceil(src_len/8) cells: the fn_source bytes PACKED 8 per cell LE, last cell zero-filled (evlog v2's pack_payload, crates/bebop-store/src/evlog.rs:57)
WORDS ceil(nwords/2) cells: two 32-bit words per cell, word 2k in the low half
RELOCS 2 cells per relocation: (word_index, (kind << 56) | target)     kinds in §5.2
```

**M-1** A hit MUST satisfy: `IDX.key64 == K64` AND `ENTRY.src_len == len(fn_source)` AND `SRC` bytes ==
`fn_source` bytes AND `DAGROOT.compiler_* == compiler_digest`. Any inequality is a miss. Check: unit tests
`dagc_hit_exact`, `dagc_miss_on_one_byte` (flip one byte inside the fn: miss; flip one byte in a comment
BETWEEN two fns: the following fn misses, nothing else), `dagc_key_collision` (K-1).
**M-2** Appends are O(1): a miss appends `ENTRY/SRC/WORDS/RELOCS` objects and the run ends with ONE new
`IDX` and ONE new root, ONE commit (`st_commit`). The image MUST NOT be rewritten whole on a put. Check:
`dagc_append_is_o1` — commit count per compile == 1; arena growth per miss ≈ entry size, asserted within 2x.
**M-3** `last_hit_gen` is updated on every hit (in the new `IDX`, not in place — nothing is overwritten).
Check: `dagc_hit_gen`.

### 4.3 Eviction and capacity

- Eviction is by generation at compaction: when `arena_used > cap_cells * 3 / 4`, `st_compact` runs with the
  live set = entries with `last_hit_gen >= gen - KEEP_GENS` (`KEEP_GENS = 16`, derived as "the number of
  compiles a lane runs between two battery runs, measured 2026-09-28 as ≤ 12 in `bebop-lang/docs/exp.journal`'s day"
  — HYPOTHESIS, a lane re-measures and writes the number beside the constant).
- **M-4** If, after compaction, one compile's new entries do not fit, the compiler MUST refuse with **exit
  126** and `0:0: error[E126]: memo image full (<need> cells over <cap_cells>) -- delete <out>.dag or raise
  DAG_CAP`, never write a partial image, never silently compile without a memo (LAW: failures are loud).
  Check: `+bebop-lang/bench/parity_constructs/neg/c1xx_dag_full.bp` driven by `+bebop-lang/bench/vs_rust/dagfull.sh` with `DAG_CAP=4096` (`EXPECT exit:126`).
- **M-5** A torn or corrupt image (superblock crc, object crc, `SRC` length past the object) MUST be
  refused by `st_reopen_verify`'s existing checks and reported as `dag: image refused (<reason>), cold
  compile` on stderr, THEN the compile proceeds cold and rewrites the image. This is the one place a
  failure may degrade to slow, and it MUST print. Check: `dagc_torn_image` (truncate the image by one cell;
  expect the stderr line and a correct binary).

### 4.4 Sizing (derived, not literal)

`cap_cells = 4 * (len(expanded_source) + 4 * nwords_estimate) / 8 + 2 * 1024`, rounded up to a 1 MiB
multiple; `nwords_estimate = len(expanded_source) / 2` (MEASURED ratio for `bebop.bp`: 206,880 B of output
for 457,622 B of source ⇒ 0.45 B/B, R §2.1). Override: `DAG_CAP=<cells>` env. The constant is written in the
source as that expression (LAW 3, "derive constants in the source").

---

## 5. Position-independent emission, relocations, link

### 5.1 What is position-dependent today (audit)

- `bl` to another fn: `emit_bl` (`bebop.bp:1023`) emits `2483027968 + offset + is_back * 67108864` with
  `offset = target - n[0]`, `target` = the callee's absolute start from `fntab[1 + cnt + k]` (the prefix sum,
  `bebop.bp:7541-7548`). A one-word change in fn 3 moves every later start (R §1.2).
- Self-recursive `bl` and every branch inside a fn: relative to the fn's own words — already PI.
- Fn values `&f` (A16 step 1): ONE-BASED fn start (`bebop.bp:2061` comment) — absolute.
- Literal handles (post-A7): absolute byte offsets in the literal region, whose ORDER is source order across
  all fns (`scan_literals`, `bebop.bp:7411`; A10 blueprint §3) — absolute.
- The entry stub (172 words) and `idx_of_main` (`bebop.bp:8043`): whole-program, cheap, recomputed always.
- Unresolved calls (`brk #87` path): PI.

### 5.2 Relocation kinds

A compile node's `WORDS` are emitted as if the fn started at word 0 with every absolute reference replaced by
a placeholder; the `RELOCS` list says how to patch them at link:

| kind | placeholder word | target | link patch |
|---|---|---|---|
| 1 `BL` | `2483027968` (imm26 = 0) | callee fn index `k` | `word = 2483027968 + ((start[k] - (start[i] + widx)) & 67108863)` — the two's-complement imm26, identical to today's `offset + is_back * 2^26` |
| 2 `FNVAL` | the `movz/movk` pair for value 0 (`emit_half`, `bebop.bp:3109-3123` shape) | fn index `k` | re-emit the pair for `start[k] + 1` (one-based, A16) |
| 3 `LIT` | the handle constant pair for offset 0 | literal ordinal within THIS fn (`o`) | re-emit the pair for `lit_base + lit_offset[ordinal_of(i, o)]`, where `ordinal_of` maps (fn, local ordinal) to the global pool ordinal the whole-program `scan_literals` assigns |

**L-1** The link step MUST produce, for every program in the sweep (§10.2), bytes identical to the pre-jump
compiler's output. This is the acceptance test of the whole compiler half: `cmp` against `git show
<pre>:<frozen .bin>` for all ~360 programs (count re-derived at switch time: `bebop-lang/bench/parity_constructs` 102 +
`neg` 20 + `bebop-lang/bench/vs_rust/std_tests` 138 + `selfhost` 127 `.bp` files as of `f334e5cb`; R §8.3 says 360 with
a different partition — the lane counts, prints the count, and the sweep MUST cover every `.bp` under those
four trees). Check: `+bebop-lang/bench/vs_rust/dagfull.sh --sweep` (§8.2).
**L-2** `emit_bl` MUST record a relocation and emit the placeholder whenever the target is another fn; it
MUST keep today's relative encoding for a self-call. Check: `dagfull --sweep` (a wrong choice breaks byte
identity on `c1xx`-class recursion constructs).
**L-3** The link MUST lay fns out in node-index order (= source order), so `starts` is the same prefix sum as
today and `offs[]` (`compile_program_offs`'s output for the harness) is unchanged. Check: `bebop-lang/tools/check_abi.py`
(footer/entry identity, runs in `bebop-lang/bench/vs_rust/invariants.sh`).

### 5.3 The driver after the jump (`cli_compile`, `bebop.bp:8545`)

```
expanded = use_expand(src)                                  -- unchanged, pure by content (R §2.3)
tables   = collect_fns + collect_ctors + scan_literals        -- unchanged, whole-program, cheap
dag      = dag_open(<out>.dag) or cold                         -- §4; refused image => stderr line, cold
for each fn i in index order:
    K = key(fn i)                                             -- §2.3
    hit = dag_lookup(K) and M-1 holds
    if hit: sizes[i], facts[i], words[i], relocs[i] from ENTRY  -- skip BOTH passes
    else:   pass 1 (compile_fn_at is_emission=0) -> sizes[i], facts[i]
            pass 2 at base 0 with placeholders    -> words[i], relocs[i]; dag_append(K, ...)
starts   = prefix sum of sizes                               -- unchanged
link     = copy words[i] to starts[i]; patch relocs (§5.2); entry stub; literal pool; export   -- new
dag_commit()                                                  -- one commit
.becache: unchanged (whole-program exact-bytes record)        -- it still short-circuits the no-change case
```

**L-4** `check` (`cli_check`, `bebop.bp:8676`) MUST run the same per-fn path against the same memo (a check
after a compile is then a hit path) and MUST NOT write `<src>.use` beside the source: it writes its scratch
beside `<out>` or under `BEBOP_TMP`. Check: `+bebop-lang/bench/vs_rust/dagfull.sh` step "check leaves the source dir
unchanged" (`git status --short` of the source dir before/after is empty).
**L-5** The cold path MUST cost within 1.05x of the pre-jump cold compile on `bebop.bp` (MEASURED basis:
crc32x keys 0.3 ms on 14,500 ms, R §9 "cold full compile" CONFIRMED ≈ 1.0x). Check: `bebop-lang/docs/PERF.md` row
`selfcompile_wall` (existing) — the chain records it.
**L-6** The one-line-edit path MUST cost ≤ 200 ms on this box at today's clock for `bebop.bp` (MEASURED
basis: hit 156-174 ms; EST 120-150 ms for one fn + link + floor, R §2.3; the acceptance number is the EST's
upper bound plus the floor's spread). Check: new `bebop-lang/docs/PERF.md` row `selfcompile_edit_wall` (A10 blueprint §7
named it; method: insert `// x` inside `fn em`'s body, recompile warm, median of 5).

---

## 6. Projection nodes in the store: `fold_step`, dirty set, anchor verify

### 6.1 Objects

```
PROJ {key64, kind, code_hi, code_lo, input_gen, input_tip_hi, input_tip_lo, ref OUT, fn_key}   9 cells
     kind: 1 = log fold (incremental), 2 = kv snapshot (memoised per generation), 3 = Datalog relation (companion spec)
     input_gen  = the input image's generation the OUT is current at
     input_tip  = the first 16 bytes of the input log's tip content id (the chain position; a generation alone
                  is not enough because `forget` rewrites in place at the same generation — the R1 lesson,
                  `workers/api/src/fold/projection.rs` header)
OUT  the projection's output object (a fold value is a 1-cell object; a relation is a CSR block, companion §B)
```

`PROJ` objects live in the SAME image as their inputs (the store already keeps generation g and g-1 and a
commit chain, C3: `store.bp` header cells 10/11), reached from a `PROJTAB` object named by superblock cell 12
(today "12..14 zero"; the PartTab snapshot copies it, so the crc covers it). **S-1** Superblock cells 13-14
stay zero; using them is a format change requiring all four readers in one commit (rule 10). Check:
`crates/bebop-wasm/oracle.py` asserts cells 13-14 == 0 on every fixture.

### 6.2 Evaluation rule

```
st_proj_eval(base, key, fn, inputs):
    p = PROJTAB lookup(key)                       -- sorted, 3 cells per entry like IDX
    if p exists and p.code == code_version and p.input_gen == gen(input) and p.input_tip == tip(input):
        return p.OUT                                -- HIT: O(1)
    if p exists and kind == 1 and p.input_gen < gen(input) and tip(input) chains back to p.input_tip:
        dirty = records after p.input_tip, oldest first   -- walk from the tip back to the memoised tip: O(k)
        acc = OUT; for r in dirty: acc = fold_step(acc, r)  -- the FNV chain h' = fnv(h, frame) is prefix-extendable
        write PROJ' (new object, new PROJTAB, one commit); return acc
    else:
        full = fold from genesis (today's walk); write PROJ'; return full
```

**S-2** `fold_step` for the LOG fold MUST be exactly one frame of the four-reader fold: FNV-1a 64 over
`seq(8) ‖ payload_len(8) ‖ payload` oldest first (`crates/bebop-wasm/src/lib.rs:11-14`), so that the
incremental value equals the full value (`hi == hf`, MEASURED equal at 5,400 and 54,000 events, R §4).
Check: `dagfull` store arm (§8.2) + Rust test `proj::tests::step_equals_full` + `oracle.py` refold.
**S-3** The KV snapshot (`kv_snapshot`, `kv.bp:52`) is MEMOISED per generation, NOT incremental: FNV over
sorted key frames is not splice-able without changing the fold, and the fold is a four-reader contract that
the jump does not change (R §8.5). Check: the store arm asserts kind 2 entries are re-derived whole on a put.
**S-4** A whole-image write that is not an append (rotation, import, `forget`'s in-place redaction —
`crates/dowiz-hub/src/forget.rs:36` `REDACTED_BIT`) MUST invalidate every PROJ of that input: the tip check
does it for redaction (the tip content id changes because ids chain over content) and the generation check
for the rest. Check: Rust test `proj::tests::redaction_drops_memo` (redact record 3 of 10 at the same
generation; the next read refolds and differs from the stale OUT).
**S-5** The Rust twin (`+crates/bebop-store/src/proj.rs`, new) and `crates/bebop-wasm` MUST land in the same
commit as `store.bp`'s `st_proj_*` (rule 10). Check: `crates/bebop-wasm/gate.sh` extended with a `proj`
fixture (`+crates/bebop-wasm/fixtures/proj.store`, `proj.expected`), four readers, one number.

### 6.3 Anchor-cell verify (B1's designed-not-landed follow-up)

`st_reopen_verify` (`store.bp:176`) crc-scans the whole arena on every non-fresh open (R §1.2; 44.7 ms on the
G7 image per ROADMAP C6's text). After the jump: superblock cell 9 `anc` = `(arena_used_before_commit << 32)
| crc32x(cells[arena_used_before_commit .. arena_used])` — the range the LAST commit wrote. **S-6** A
non-fresh open MUST verify (a) both superblocks' crc, (b) the PartTab, (c) the `anc` range; and MUST run the
whole-arena scan only under `st_open_verify_all` (used by `dagfull`, `scrash.sh`, `scrash_torn.sh`). Check:
`std_golden` gates `sanchor` (a torn last commit is refused via `anc`; `EXPECT` the refusal code) and the
existing `scrash_torn` TRIALS=50 stays 0 invalid reopens (ROADMAP B1). Every open of an image whose cell 9 is
zero (pre-jump images) MUST take the whole-arena path (compatibility: no image is rewritten).

### 6.4 The dowiz side (not changed by the jump; named so the rows line up)

`workers/api/src/fold/projection.rs` (R1, landed) already is a projection node with a dirty set in Rust;
`workers/api/src/hubdo/menu.rs` (R2, landed) memoises the catalogue per generation; `tools/gates/dataflow.sh`
(R3, landed, baseline 38) is the edge rule. The jump gives them a bebop reader (`st_proj_*`) of the SAME
persisted projection bytes, so `rebuild` (`workers/api/src/rebuild.rs`) and the four readers can check a
persisted projection (`docs/research/2026-09-27-dag-architecture.md` §3.4 R4). Rows: BLUEPRINT Wave 2.

---

## 7. The scheduler (`+bebop-lang/selfhost/prelude/sched.bp`, new)

### 7.1 Rules (R §7.4, §3)

- **T-1** Nodes are pure (§3), so any topological order gives the same outputs; determinism therefore needs
  ONE rule: **outputs are assembled in node-index order, never completion order.** Check: `sched_det` gate —
  the R §3 probe shape (`levelled DAG N × L × K`) at W = 0 and W = 3, run TWICE each, folds equal (memory
  `bebop-clone-kept-symbol-limit`: threaded measurements run twice).
- **T-2** The ready set of a level = nodes whose inputs are all complete; workers claim by `sys_atomic_add`
  on a per-level counter; a level ends when its done counter reaches N; the parent is woken ONCE per level
  (`sys_futex_wake` when done == N, compared at claim time), never once per task (MEASURED: per-task wakes
  made W = 1 2.8x SLOWER than serial on 0.1 ms tasks, R §3.3). Check: `sched_wakes` gate counts wakes ==
  levels via a counter cell.
- **T-3** W ≤ 3 (three A78 cores in a slot; `bebop-lang/tools/slot.sh` header), persistent for the run, parked on a
  futex between levels; each worker's env is ONE `[i64]` block (`bebop-lang/selfhost/std/sconc.bp` idiom); every fn that
  spans `sys_clone` keeps ≤ 8 symbols (exit 109 enforces it). Check: compile of `sched.bp` (exit 109 is the
  gate) + `sched_det`.
- **T-4** A level is scheduled on workers only when its estimated work ≥ **10 ms per worker** (MEASURED:
  barrier ≈ 0.4 ms idle / 1.5-2.5 ms loaded, spawn floor 1/4/5 ms at W = 1/2/3; speedup 2.5-2.7x at ≥ 40 ms
  per level, 1.0x at 0.1 ms tasks, R §3.2-3.3). Below it the level runs serially in the parent — the W = 0
  path, which is the same code. The estimate is the sum of the nodes' last `cost_us` (§4.2 ENTRY, or 0 on a
  cold node → serial). Check: `sched_threshold` gate — a 512 × 0.1 ms level MUST report `serial` and cost ≤
  1.2x the W = 0 wall.
- **T-5** `pool.bp` becomes a client of `sched.bp` (its worker loop is the scheduler's); `pool_parity.sh`
  MUST stay green. Check: `bebop-lang/bench/vs_rust/pool_parity.sh`.

### 7.2 Where the scheduler runs inside the jump

- Compile nodes: one level of up to 768 independent nodes (given the tables); cold `bebop.bp` = 348 × ≈ 41 ms
  ⇒ EST 2.5x on the cold compile at W = 3 IF per-worker `fntab` scratch (8,192 cells each) and per-worker
  `insns` buffers are allocated once (HYPOTHESIS: `compile_fn_at` shares scratch through `fntab`; a lane
  measures first). This is a MAY in the jump; the gate is the cold `selfcompile_wall` NOT degrading (L-5).
- Store projections: below T-4's threshold at a venue's size (every kernel is microseconds, R §12.3) ⇒ serial
  by rule. The scheduler pays only on the store's whole-image work (folds of tens of ms, chain verify 8.5-85
  ms, compaction 747 ms, R §3.3).

### 7.3 Level report

**T-6** The scheduler MUST print, when `BEBOP_SCHED_REPORT=1`, one line per level: `level <i> width <n>
est_us <e> mode serial|W=<w> wall_us <t>`. A width-1 level is the N-4 signal. Check: `sched_det` reads it.

---

## 8. The `dagfull` gate (`+bebop-lang/bench/vs_rust/dagfull.sh`, new; permanent, every commit)

### 8.1 Statement

For every fixture and every node: (a) evaluate with the memo WARM after a random sequence of appends/edits,
(b) evaluate from the sources with the memo EMPTY, (c) the two outputs are byte-identical and their
four-reader folds equal. `rebuild`'s `stale = []` on the dowiz side is the same statement; here it runs on
every commit, not nightly (R §7.5). The gate's VALUE is the deliverable: it prints counts, not only rc.

### 8.2 Arms and their printed line

| Arm | What it does | Prints |
|---|---|---|
| `compile` | for each program P in the sweep: cold compile → `c.bin`; insert `// dagfull` inside the LAST fn's body → warm compile → `w.bin`; remove it → warm compile → `w2.bin`; assert `w2 == c` byte for byte and `w` == cold compile of the edited source | `dagfull compile <identical>/<programs> edit_ms_med <ms>` |
| `--sweep` | every program compiled by the candidate vs the frozen pre-jump `.bin` from `git show <pre>:<path>` (§10.2) | `dagfull sweep <identical>/<programs> pre=<sha>` |
| `store` | for each store fixture: `SEED` random appends (`bebop-lang/bench/vs_rust/scrash.sh`'s generator shape), evaluate the log fold warm (via `st_proj_eval`) and cold (`st_open_verify_all` + full walk), compare; four readers on the final image | `dagfull store <equal>/<fixtures> readers 4/4` |
| `datalog` | companion spec §A.7 | `dagfull datalog <equal>/<rulesets>` |
| `sched` | T-1's twice-run W = 0 vs W = 3 folds | `dagfull sched <equal>/<shapes>` |

`dagfull` runs inside `bebop-lang/tools/battery.sh` (serial, one slot) and its numbers land in `bebop-lang/docs/PERF.md` through
`bebop-lang/tools/perf.py record`. **G-1** The gate MUST refuse to run when its prerequisite artefacts are missing and
name them (L24) and MUST NOT count a skip as agreement (memory `gates-that-count-skips-as-passes`;
`crates/bebop-wasm/gate.sh` shows the shape). Check: `dagfull.prove.sh` step 0 runs it with the frozen
binaries removed and expects a named refusal.

### 8.3 Mutation proof (`+bebop-lang/bench/vs_rust/dagfull.prove.sh`; rule 4 of `/root/dowiz/.claude/lanes/DOWIZ-COMMON-RULES.md` (box-local, untracked))

1. Skip one node's invalidation in a scratch copy of the driver (make `M-1`'s `SRC` comparison always true)
   → the `compile` arm MUST go red naming the program whose `w2 != c`.
2. Patch one byte of the compiler binary used for the WARM run → `K-4`: 0 hits, the arm stays green but the
   printed `hits` count MUST be 0 (the proof reads the number, not rc).
3. Skip the tip check in `st_proj_eval` → redact a record in a fixture → the `store` arm MUST go red.
Each step restores the check in the next edit and the verdict quotes `grep -n` of the restored line.

---

## 9. Lean: `+bebop-lang/formal/Bebop/Dag.lean` (new), the statements

The development is over the node GRAPH, not over machine code or `sys_clone` (`bebop-lang/formal/README.md` names the
`sys_run` / child-side gaps and they are not widened). It joins `lean_lib Bebop` (`bebop-lang/formal/lakefile.lean`).

```lean
/-- A finite DAG of total functions: node i reads only nodes with smaller index. -/
structure Dag (α : Type) where
  n     : Nat
  deps  : Fin n → List (Fin n)
  acyc  : ∀ i j, j ∈ deps i → j < i
  f     : Fin n → List α → α          -- the node's pure function over its inputs' values

def IsTopo (g : Dag α) (o : List (Fin n)) : Prop :=  o.Nodup ∧ (∀ i, i ∈ o) ∧ ∀ i j, j ∈ g.deps i → o.indexOf j < o.indexOf i
def run (g : Dag α) (o : List (Fin n)) : Fin n → α    -- evaluate in order o, assemble by index

theorem confluence (g : Dag α) (o₁ o₂) (h₁ : IsTopo g o₁) (h₂ : IsTopo g o₂) : run g o₁ = run g o₂
theorem memo_sound (g : Dag α) (memo : Fin n → Option α) (hm : ∀ i v, memo i = some v → v = run g o i) :
  runWithMemo g o memo = run g o
theorem step_extends (step : β → α → β) (init : β) (xs ys : List α) :
  List.foldl step (List.foldl step init xs) ys = List.foldl step init (xs ++ ys)   -- S-2's shape; List.foldl_append
```

**V-1** `lake build` rc = 0 and `grep -c sorryAx` over the build log = 0 for `Dag.lean` (the F9 wording:
`#print axioms` on every theorem at the bottom of the file, quoted in the file header as `Theorems.lean`
does). Check: `+bebop-lang/bench/vs_rust/dagfull.sh` step `lean` runs `lake build` through `bebop-lang/tools/slot.sh` and greps.
**V-2** `bebop-lang/formal/Bebop/Reject.lean` gains static rule 124 (P-1) and the conformance count stays ≥ 120/122
(`bebop-lang/formal/results.txt`). Check: `lake exe parityrun` line `lean_conformance`.
**V-3** The Datalog theorems are in the companion spec §A.8 (`+bebop-lang/formal/Bebop/Datalog.lean`).

---

## 10. The one jump: what the spec fixes about it

### 10.1 What is replaced (R §8.1), and what is not

Replaced in ONE commit: the driver's memo (`.becache` stays as the whole-program short-circuit; the per-fn
memo is added and the two-pass loop becomes hit-or-compile), `emit_bl`/`FNVAL`/`LIT` emission (placeholders
+ relocations), a link step, the store's read path (`st_proj_*`, anchor verify), `pool.bp` → `sched.bp`
client, `dagfull` + `arch_check` purity, `Dag.lean`. NOT changed: the store FORMAT (no image is rewritten;
superblock cells 12 and 9 gain meanings that read as zero in old images), the loader, the language surface
beyond `pure`/`sched`/`out` keywords, the emitted words for any fn (only placement — a codegen change to a
fn's words is a separate commit, AGENTS L14).

### 10.2 Preconditions the switch checks (each a number; the commands are in the BLUEPRINT §5)

1. `bebop.bin == compile(bebop.bp)` at the pre-jump commit (`arch_check` artifact identity; MEASURED TRUE
   `d1dedc62` on 2026-09-28, R §2.1).
2. Frozen pre-jump `.bin` for EVERY program of the sweep, produced by the pre-jump binary, hashed, committed
   under `bebop-lang/bench/golden/` per L13 (`bebop-lang/bench/golden/<name>-<rev>.bin`), before the jump lands.
3. Four-reader fixtures unchanged by construction (`crates/bebop-wasm/fixtures/kv.store`, `kv.expected`).
4. Battery GREEN at the pre-jump commit, run as the control FIRST (memory `head-may-not-reproduce-its-own-artifacts`).

### 10.3 Correctness at the switch, without a parallel copy (R §8.4)

Compiler: `dagfull --sweep` identical for all programs; `bebop-lang/tools/chain.sh bebop.bp <out> --codegen` gen3 == gen4;
`dagfull compile` on the sweep. Store: `dagfull store`, `crates/bebop-wasm/gate.sh` 4/4. Scheduler: `dagfull
sched`. Lean: V-1, V-2. Numbers: K1H-K4 (`bebop-lang/bench/vs_rust/honest.sh`) and G7/G8 rows MUST NOT degrade (gates
outrank rows, memory `gates-must-improve-not-degrade`); `selfcompile_wall` within L-5; `selfcompile_edit_wall`
within L-6. Then re-promote `bebop.bin` from the fixpoint.

---

## 11. Traceability: every MUST and its check

| MUST | Check (test / gate / construct) |
|---|---|
| N-1..N-4 | code review; `dagfull` arms; scheduler level report |
| K-1 | `dagc_key_collision`, `dagfull compile` |
| K-2 | `c51_casbad`; block twin refusal (companion §B.6) |
| K-3 | `dagc_key_fields` |
| K-4 | `dagfull.prove.sh` step 2 (hits == 0) |
| P-1 | `+bebop-lang/bench/parity_constructs/neg/c1xx_pure_sys.bp`, `+bebop-lang/bench/parity_constructs/neg/c1xx_pure_calls_io.bp`, `+bebop-lang/bench/parity_constructs/neg/c1xx_pure_writes_input.bp` + positive twins; `Reject.lean` rule 124 |
| P-2 | `dagc_key_fields` |
| P-3 | `std_golden` `sproj_neg` / `sproj` |
| M-1..M-3 | `dagc_hit_exact`, `dagc_miss_on_one_byte`, `dagc_append_is_o1`, `dagc_hit_gen` |
| M-4 | `+bebop-lang/bench/parity_constructs/neg/c1xx_dag_full.bp` under `DAG_CAP=4096` (exit 126) |
| M-5 | `dagc_torn_image` |
| L-1, L-2 | `dagfull --sweep` |
| L-3 | `bebop-lang/tools/check_abi.py` in `bebop-lang/bench/vs_rust/invariants.sh` |
| L-4 | `dagfull` "check leaves the source dir unchanged" |
| L-5, L-6 | `bebop-lang/docs/PERF.md` `selfcompile_wall`, `selfcompile_edit_wall` |
| S-1 | `crates/bebop-wasm/oracle.py` cells 13-14 == 0 |
| S-2 | `dagfull store`; `proj::tests::step_equals_full`; `oracle.py` refold |
| S-3 | `dagfull store` (kind 2 re-derived whole) |
| S-4 | `proj::tests::redaction_drops_memo` |
| S-5 | `crates/bebop-wasm/gate.sh` `proj` fixture 4/4 |
| S-6 | `std_golden` `sanchor`; `bebop-lang/bench/vs_rust/scrash_torn.sh` TRIALS=50 → 0 |
| T-1..T-6 | `sched_det`, `sched_wakes`, exit 109, `sched_threshold`, `bebop-lang/bench/vs_rust/pool_parity.sh`, level report |
| G-1 | `dagfull.prove.sh` step 0 |
| V-1..V-3 | `lake build` + `grep -c sorryAx` = 0; `lean_conformance ≥ 120/122`; companion §A.8 |

The `dagc_*` unit tests are bebop programs under `bebop-lang/bench/vs_rust/std_tests/dagc_*.bp` with `gate` lines in
`bebop-lang/bench/vs_rust/std_golden.sh` and Python oracles under `bebop-lang/bench/oracles/` (`arch_check` `gate-oracle`: every
gate has an oracle; `producer-memo`: gate producers are run directly, never through `run`'s memo).

---

## 12. Where the research and the operator (or this spec) disagree — recorded, not picked

| # | Operator / research says | The other side, with its number | Status |
|---|---|---|---|
| C-1 | Operator: prices/stock as dense integer tensors recomputed on wasm SIMD; "everything computes as tensors" | R §12.3 MEASURED: sparse scalar mask 1,089 ns vs best simd128 dense 9,749 ns at 165/120/70 (8x), 580x at 100x; price/portions 0 % from SIMD (no integer divide / i64 mulhi in v128); all kernels 8.3 µs = 0.08 % of the 10 ms wall. R §13.2 MEASURED the codec (columnar block) 2,600x faster than JSON parse — the Arrow-sense "tensor" wins, the dense-SIMD sense loses | Companion spec specifies the CSR/columnar block + scalar oracle (satisfies "tensor" in the Arrow sense); the dense-SIMD form is BLUEPRINT row DG25, gated on ITS measured number, **OPERATOR DECISION NEEDED** |
| C-2 | Operator: bebop codegen for v128 as part of the runtime | R §12.2, §8.5: no v128 emitter exists (NEON only inside `hvham`/`hvham2`), T64/T94-class work with its own register model; no gain to collect (C-1) | BLUEPRINT DG26 "do not, in the jump"; **OPERATOR DECISION NEEDED** if C-1 is decided for dense SIMD |
| C-3 | R §7.3: "the memo store IS a bebop KV image (`kv.bp`)" | R §4 MEASURED `kv.bp`'s layout: one byte per cell (8x), 42.8 ms whole-image rewrite per put | This spec §4: KV semantics, packed append-only layout, own `layout_digest`; the four readers open it unchanged |
| C-4 | R §11.1 / §14: "the tree has HMAC/PBKDF2 ... and no AEAD — that is the one new primitive" | The tree HAS one: `crates/dowiz-core/src/pq/aes_gcm.rs` (AES-256-GCM, KAT-gated against the McGrew–Viega vector, used by `backup_seal.rs`); `crates/dowiz-hub/src/crypto.rs` has none | BLUEPRINT DG10 reuses `dowiz-core`'s AEAD behind the `pq` feature seam rather than adding ChaCha20-Poly1305; a bebop-side AEAD twin (for a Box that shreds without Rust) is a separate gated row |
| C-5 | R §8.3: the sweep is "360 programs (selfhost 122 + std_tests 144 + parity_constructs 102 + neg)" | `f334e5cb` counts: parity 102, neg 20, std_tests 138, selfhost 127 `.bp` — a different partition and total | L-1: the lane re-derives the count at switch time and covers every `.bp` under the four trees; the number in the verdict is the one that ran |
| C-6 | R §2.1/§2.2: cold self-compile 14.5 s on this box today | `bebop-lang/ROADMAP.md` Measured row / `bebop-lang/docs/PERF.md` `selfcompile_wall` 623-667 ms (2026-09-08/14, 6,909-line source, cores at full clock) | Both stand: L-5 and L-6 are RATIOS against the pre-jump binary measured in the same session (R §2.2: "every ratio holds at any clock") |
| C-7 | Operator: one jump, no staged migration | R §6: rows 1, 4, 5, 7, 8 (fn-name table, KV v2, columnar block, Datalog, node key) "can land before or after it without a second runtime existing at any moment" | This spec treats them as independent lanes (BLUEPRINT Waves 0 and 2); none introduces a second runtime, so the one-jump rule is not violated. If the operator reads "one jump" as "everything in one commit", the BLUEPRINT's wave order collapses into one merge and the box cap (3 lanes) makes it three lanes of ~1,900 + ~700 lines — **OPERATOR DECISION NEEDED** only in that reading |

## 13. Not in this spec (deliberately)

The Datalog rule format, stratification and its refusal code E125, the block codec byte layout, per-type
encodings, the round-trip gate and money rules: `docs/design/SPEC-DATALOG-AND-CODEC-2026-09-28.md`. Lane
rows, owned files, acceptance numbers, models, the switch checklist as commands, the §10/§11/§14 rows of the
research: `docs/design/BLUEPRINT-BEBOP-DAG-2026-09-28.md`.
