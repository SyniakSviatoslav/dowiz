# BLUEPRINT: bebop onto the DAG runtime — lanes, rows, numbers, the switch (2026-09-28)

**Status.** Implementation blueprint, lane W-SPEC (docs only, tree `f334e5cb`). It turns
`docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md` (**RT**) and `docs/design/SPEC-DATALOG-AND-CODEC-2026-09-28.md`
(**DC**) into rows a coding lane (Opus or Haiku, ≤ ~200-600 lines each) executes without re-deciding
anything. Measured numbers are quoted with their section of `docs/research/2026-09-28-bebop-dag.md` (**R §n**).
Row ids: `DG1-DG10`, `DG17-DG26` (bebop-lang side, `bebop-lang/ROADMAP.md` Phase G; DG11-DG16 are not used so the numbering of R §14's measurement rows stays stable) and `DW1-DW9` (dowiz side,
`docs/design/ROADMAP-2026-09-22.md` Wave DAG). Labels: MEASURED / DOC / EST / HYPOTHESIS as in R.

**Box rules that shape every wave** (`/root/dowiz/.claude/lanes/DOWIZ-COMMON-RULES.md` (box-local, untracked)): MAX 3 lanes at once (memory
`one-agent-one-worker`), every `cargo`/compile through `bebop-lang/tools/slot.sh` in the foreground, no git
writes by lanes, files a lane owns are disjoint within a wave, new `.rs` files ≤ 300 lines
(`tools/gates/file-size.sh`), new `.bp` files ≤ 800 (`bebop-lang/tools/arch_check.py`), tests beside the
code, every refusal test has a positive twin, mutation proofs undone in the next edit, 100 % coverage rule
(memory `dowiz-100-percent-test-coverage`), Haiku first and a failed Haiku lane relaunched on Opus (memory
`model-routing-opus-plans-haiku-codes`) — the "model" column below is the FIRST attempt.

**Path convention.** A backticked path is a file in the tree at `f334e5cb` (checkable with `ls`); a path written `+like/this.rs` is a file the named row CREATES and does not exist yet, so `tools/gates/paths.sh`'s rule (a cited path must exist) is not fooled by a plan.

---

## 0. The plan in ten lines

1. **Wave 0 (3 independent lanes, land before the jump, no second runtime):** DG1 O(1) fn-name table
   (EST 3-6x cold compile, R §2.2), DG2 one node key × 4 readers (RT §2), DG3 KV v2 packing × 4 readers
   (MEASURED 8x image, R §4).
2. **Wave 1 — THE ONE JUMP (3 lanes landing in ONE commit):** DG4 L1 compiler (per-fn memo + link), DG5 L2
   store (projection nodes + anchor verify + Rust twin), DG6 L3 scheduler + `dagfull` + `pure` + `Dag.lean`.
   Main merges, runs the switch checklist (§5) as commands, re-promotes `bebop.bin`.
3. **Wave 2 (rules and blocks):** DG7 columnar catalogue block in `dowiz-hub` (MEASURED 2,600x, R §13.2),
   DG8 Datalog rule layer (MEASURED 82 ns/event, R §12.3), DG9 codec round-trip gate + block twins.
4. **Wave 3 (synergy, dowiz rows DW1-DW6):** browser replica as subscriber, personal labels as the rule,
   gates as graph queries, invariants per edge, provenance, per-node accounting; DG10 crypto-shredding.
5. **Wave M (measurement rows DG17-DG24):** R §14's eight not-verified items, each a command.
6. **Gated rows:** DG25 dense-SIMD tensors (OPERATOR DECISION NEEDED, DC §B.8), DG26 v128 emitter (not in
   the jump).
7. Every row: owned files, deps, RED tests first, acceptance number with its measured basis, mutation proof,
   gates, lines EST, model.
8. The switch (§5) is shown correct against the PRE-jump commit's frozen binaries — no parallel copy in the
   tree (R §8.4).
9. Conflicts (§7) are numbered, both numbers quoted; nothing is silently picked.
10. Line estimates come from the sizes of the code each row replaces (R §5, §8.2); they are EST.

---

## 1. Wave 0 — foundations (independent lanes; can also land after the jump; none is a second runtime)

### DG1 — O(1) fn-name table in the compiler (R §6 row 1)

- **Owns:** `bebop-lang/bebop.bp` (`find_fn` at `:6819`, `emit_bl_call` at `:1388`, the `fntab` name zone
  writers in `compile_program_offs` `:7490-7560`); `bebop-lang/tools/check_abi.py` (`--fntab` zone map, if a
  zone moves).
- **Depends on:** nothing. Codegen-NEUTRAL: `bebop-lang/tools/chain.sh bebop.bp <out>` without `--codegen`
  MUST give gen2 == gen3 == gen4.
- **RED first:** a probe program with 701 wide fns (R §2.2's `wide body` generator) timed under
  `bebop-lang/tools/slot.sh`; record `wide_701_ms` before the change (MEASURED 2,061-2,063 ms on 2026-09-28
  at 0.69-0.96 GHz).
- **Change:** a hash-indexed name table (open addressing over `fnames`, `bebop-lang/selfhost/std/csheaf.bp`'s
  16-slot probe shape generalised to 1,024 slots for the 768-fn cap) replacing the linear scan per call site.
- **Acceptance (MEASURED basis R §2.2, exponent 1.85 in fn count):** `wide_701_ms` falls to ≤ 1.3x the
  NARROW 701 line (676/478 ms) — i.e. the fns × calls term is gone; cold `selfcompile_wall` ≤ 0.5x its
  same-session control (the EST 3-6x's conservative end); digest of `bebop.bp`'s output UNCHANGED
  (`d1dedc62` at `f334e5cb`).
- **Mutation proof:** re-enable the linear scan behind a flag in a scratch copy → the number returns to the
  control. Restore, quote `grep -n`.
- **Gates:** `bebop-lang/tools/chain.sh` (no `--codegen`), `bebop-lang/bench/vs_rust/invariants.sh`,
  `bebop-lang/docs/PERF.md` rows. **Lines EST:** ~50. **Model:** Opus (compiler internals).

### DG2 — one node key, four readers (RT §2; R §10 item 1)

- **Owns:** `+bebop-lang/selfhost/prelude/nodekey.bp` (new: `key_frame`, `key64`, `key256`);
  `+crates/bebop-store/src/nodekey.rs` (new); `+crates/bebop-wasm/src/nodekey.rs` (new) + `oracle.py`
  (`--key`); `+crates/bebop-wasm/fixtures/key.expected` (new).
- **Depends on:** nothing (the frame is bytes; `crc32x`, `sha256_words`, `sha2` exist).
- **RED first:** `crates/bebop-wasm/gate.sh` gains a `key` step that fails until four readers print the same
  `K64` and `K256` for three fixture frames (a compile-node frame, a projection frame with two inputs, an
  empty-field frame).
- **Acceptance:** 4/4 readers agree on all three fixtures; the frame length of the compile-node fixture equals
  the derived expression of RT K-3 in every reader's test.
- **Mutation proof:** flip the field order in one reader → the gate names the reader.
- **Gates:** `crates/bebop-wasm/gate.sh`; `bebop-lang/bench/vs_rust/std_golden.sh` `gate nodekey`.
  **Lines EST:** ~60 × 4. **Model:** Haiku (mechanical, spec-exact).

### DG3 — KV v2 byte packing across four readers (R §6 row 4; memory `dowiz-hub-seven-phases`, phase 4b)

- **Owns:** `bebop-lang/selfhost/std/kv.bp` (`kv_snapshot :52`, a `VERSION` cell in the root as evlog v2 does
  at `crates/bebop-store/src/evlog.rs:24-33`); `crates/bebop-store/src/kv.rs` (`load :87`, `put :164`,
  `compacted_bytes :279`); `crates/bebop-wasm/src/lib.rs` + `oracle.py`; `+crates/bebop-wasm/fixtures/kv2.store`
  + `kv2.expected` (new; `kv.store` UNCHANGED — v1 images stay v1, exactly evlog's rule).
- **Depends on:** nothing. Rule 10: every reader in the SAME commit.
- **RED first:** `oracle.py` asserts on a v2 fixture; `crates/bebop-store` test `kv::tests::v2_bytes_per_entry`.
- **Acceptance (MEASURED basis R §4):** the 165 × 3.2 KB fixture image falls from **4,256,224 B to ≤ 600,000 B**
  (8x amplification removed; EST 560,000); `snapshot_root` of a v2 image equals the v1 image's for the same
  entries (the fold is over BYTES, not cells); four readers 4/4 on `kv.store` (v1) AND `kv2.store`.
- **Mutation proof:** pack 7 bytes per cell in one reader → 4/4 becomes 3/4 naming it.
- **Gates:** `crates/bebop-wasm/gate.sh`; `.github/workflows/ci.yml` four-reader job; `cd crates/dowiz-hub &&
  cargo test` golden-image tests untouched (v1). **Lines EST:** ~200 across four readers. **Model:** Opus
  (format change; a silent reader drift here cost five gates once — `CLAUDE.md` "three laws").

---

## 2. Wave 1 — THE ONE JUMP (three lanes, one commit; R §8.2)

Preconditions (RT §10.2) are established by MAIN before the lanes start: the control battery GREEN, the
frozen sweep binaries committed under `bebop-lang/bench/golden/dag-pre-<rev>/` (L13), `bebop.bin ==
compile(bebop.bp)`. Both L1 and L2 read RT §2 (the key) and RT §4 (the memo image) — the "40-line spec main
writes first" of R §8.2 is those two sections.

### DG4 — L1 compiler: per-fn nodes, PI emission, relocations, link, `<out>.dag` (RT §2.3, §4, §5)

- **Owns:** `bebop-lang/bebop.bp` — `cli_compile :8545`, `cli_check :8676`, `compile_program_offs :7490`,
  `emit_bl :1023`, `emit_bl_call :1388`, the `FNVAL` pair emitter (A16, `:2106` `emit_call_fn` area) and the
  literal handle emitter; `+bebop-lang/selfhost/prelude/dagc.bp` (new: `dag_open`, `dag_lookup`, `dag_append`,
  `dag_commit`, `dag_link`, `dag_key`); `bebop-lang/tools/cc.sh` (a `DAG_CAP` passthrough);
  `bebop-lang/docs/TRAPS.md` (E124 is L3's; E126 is this row's).
- **Depends on:** DG2's `nodekey.bp` if landed (else inline the frame, ~40 lines, same bytes); RT §4 layout.
- **RED first (in `bebop-lang/bench/vs_rust/std_tests/`):** `dagc_hit_exact.bp`, `dagc_miss_on_one_byte.bp`,
  `dagc_key_collision.bp`, `dagc_key_fields.bp`, `dagc_append_is_o1.bp`, `dagc_hit_gen.bp`, `dagc_torn_image.bp`
  with `gate` lines and `bebop-lang/bench/oracles/dagc_*.py` (Python reads the `.dag` image from bytes: RT §4.2);
  `+bebop-lang/bench/parity_constructs/neg/c1xx_dag_full.bp` (`EXPECT exit:126`).
- **Acceptance (MEASURED basis R §2.1, §2.3):** `dagfull --sweep` = every program byte-identical to the
  frozen pre-jump binary (RT L-1); `selfcompile_edit_wall` ≤ **200 ms** (hit path 156-174 ms MEASURED; EST
  120-150 ms for one fn + link + floor; ~100x on today's 14.5 s, ~12x on the committed 1.6 s row); cold
  `selfcompile_wall` ≤ 1.05x its same-session control (crc32x keys 0.3 ms MEASURED, R §2.4); `check` writes
  nothing beside the source (RT L-4).
- **Mutation proof:** RT §8.3 steps 1-2.
- **Gates:** `bebop-lang/tools/chain.sh bebop.bp <out> --codegen` (relocation changes codegen: FREEZE, constructs
  re-frozen), `bebop-lang/bench/vs_rust/becache_gate.sh` (still green: the whole-program record is unchanged),
  `+bebop-lang/bench/vs_rust/dagfull.sh compile --sweep`, `bebop-lang/bench/vs_rust/invariants.sh`.
  **Lines EST:** ~600 (replaces the ~250-line becache/driver block, touches three emitters; R §8.2).
  **Model:** Opus.

### DG5 — L2 store: projection nodes, `fold_step`, anchor verify, Rust twin, wasm reader (RT §6)

- **Owns:** `bebop-lang/selfhost/prelude/store.bp` (`st_proj_eval`, `st_proj_put`, `PROJTAB` at superblock
  cell 12, `anc` at cell 9, `st_open_verify_all`; `st_reopen_verify :176`); `bebop-lang/selfhost/std/kv.bp`
  (kind-2 memo of `kv_snapshot`); `+crates/bebop-store/src/proj.rs` (new) + `lib.rs` (`from_bytes :144` gains a
  borrowing `view` twin — R §11.2's "borrow `&[u8]` instead of copying", MEASURED 31 ms copy vs 5.75 ms fold);
  `crates/bebop-wasm/src/lib.rs` + `oracle.py` (cells 13-14 == 0; PROJ read); `+crates/bebop-wasm/fixtures/proj.store`
  + `proj.expected` (new).
- **Depends on:** RT §2 key; DG2 if landed.
- **RED first:** Rust `proj::tests::{step_equals_full, redaction_drops_memo, hit_is_o1}`; bebop
  `std_golden` gates `sproj`, `sproj_neg` (trap 124), `sanchor`; `oracle.py` refold of `proj.store`.
- **Acceptance (MEASURED basis R §4):** one append + `fold_step` ≤ **0.05 ms** at 5,400 events vs full fold
  5.75 ms (254x; 1,274x at 54,000); `hi == hf` asserted on every fixture; four readers 4/4 on `proj.store`;
  `bebop-lang/bench/vs_rust/scrash_torn.sh` TRIALS=50 → 0 invalid reopens with the anchor path; a non-fresh open of the
  G7 image verifies in ≤ 5 ms (was a 44.7 ms whole-arena scan per ROADMAP C6's text — re-measure the control
  first); existing `*.store` goldens byte-unchanged (format unchanged, RT §10.1).
- **Mutation proof:** RT §8.3 step 3.
- **Gates:** `bebop-lang/bench/vs_rust/std_golden.sh` (G1-G8 rows not degraded), `crates/bebop-wasm/gate.sh`,
  `+bebop-lang/bench/vs_rust/dagfull.sh store`, `cd crates/bebop-store && cargo test`, `cd crates/dowiz-hub && cargo test`.
  **Lines EST:** ~500 bebop + ~300 Rust. **Model:** Opus.

### DG6 — L3 scheduler + `dagfull` + `pure` refusal + `Dag.lean` (RT §3, §7, §8, §9)

- **Owns:** `+bebop-lang/selfhost/prelude/sched.bp` (new); `bebop-lang/selfhost/std/pool.bp` (becomes a client);
  `+bebop-lang/bench/vs_rust/dagfull.sh` + `dagfull.prove.sh` (new); `bebop-lang/tools/arch_check.py` (a
  `pure-class` check: every `st_proj_*`-registered fn is `pure fn`); the `pure`/`sched`/`out` keywords and E124
  in `bebop-lang/bebop.bp` (`collect_fns :7081` class bit, `diag_exit :86` arm, the check at call/builtin
  emission — DISJOINT from DG4's regions by agreement: DG6 edits `collect_fns`, `emit_ident`/builtin dispatch
  and `diag_exit`; DG4 edits `cli_*`, `compile_program_offs`, `emit_bl*`; main merges);
  `+bebop-lang/formal/Bebop/Dag.lean` (new), `Reject.lean` (rule 124), `bebop-lang/formal/Bebop.lean` (import);
  `bebop-lang/bench/parity_constructs/neg/c1xx_pure_*.bp` + positive twins; `bebop-lang/docs/TRAPS.md` (E124).
- **Depends on:** RT §3, §7; DG4/DG5 land together (dagfull's arms need them; the arms are written against
  the spec and go red until then — that IS the RED).
- **RED first:** `sched_det`, `sched_wakes`, `sched_threshold` gates (the R §3 probe's shape -- its source, p6/dag.bp, lives in the research lane's scratch, not in the tree:
  `N × L × K`, fold + wall); the four `c1xx_pure_*` constructs; `dagfull.prove.sh` steps 0-3.
- **Acceptance (MEASURED basis R §3.3):** W=0 == W=3 folds on all shapes, twice; ≥ **2.3x** at W=3 on the
  8 × 4 × 5,000,000 shape (MEASURED 2.55x; 2.7x on 3 × 4 × 20 M) and ≤ 1.2x serial wall on the 64 × 8 × 100,000
  shape (T-4 threshold routes it serial); wakes == levels; `pool_parity.sh` green; `lake build` rc 0,
  `sorryAx` 0 for `Dag.lean`; `lean_conformance` ≥ 120/122; exit 124 on the three negatives, 0 on the twins.
- **Mutation proof:** assemble in completion order in a scratch copy → `sched_det` red (fold differs at
  W=3 on the 24-independent shape at least once in 4 runs; if it does not, the proof reports "not
  demonstrated" and the row is not done — a proof that cannot fail proves nothing).
- **Gates:** `bebop-lang/tools/battery.sh` (dagfull added), `bebop-lang/bench/vs_rust/pool_parity.sh`,
  `bebop-lang/bench/vs_rust/construct_parity.sh`, `lake build` via `bebop-lang/tools/slot.sh`.
  **Lines EST:** ~400 bebop + ~120 Lean + ~150 sh + ~60 py. **Model:** Opus (Lean + scheduler); the
  constructs and `.prove.sh` MAY be split to a Haiku sub-lane after the spec is frozen.

---

## 3. Wave 2 — rules and blocks (after the switch; three lanes)

### DG7 — columnar catalogue block in `dowiz-hub` (R §6 row 5; DC §B.4) — dowiz row DW7 mirrors it

- **Owns:** `+crates/dowiz-hub/src/block/{mod,schema,encode,decode,view,tests}.rs` (new, each ≤ 300 lines);
  `crates/dowiz-hub/src/stock.rs` (`bom_of :1393` reads the `bom` block when present; JSON path kept for
  writers); `workers/api/src/hubdo/menu.rs` (`/fold/menu`, `/fold/products` answer `menu_prices` + `names`
  blocks alongside the JSON body — the JSON stays for the browser until DW1); `workers/api/src/fold/menu.rs`;
  `+crates/dowiz-hub/fixtures/blocks/*.dwb` (new).
- **Depends on:** DC §B.2-B.4 (the byte layout); DG9's twins for the four-reader check (may land after).
- **RED first:** `block::tests::{roundtrip_menu_prices, roundtrip_bom, refuse_bad_magic, refuse_bad_crc,
  refuse_bad_offsets, price_overflow_refused, mask_vec_equals_scalar, schema_table_agrees}`; a `bom_of` test
  that reads the block and the JSON for the same product and asserts equal lines.
- **Acceptance (MEASURED basis R §13.2):** decode of the 165-dish `menu_prices` block ≤ **10 µs** (MEASURED
  2.4 µs) vs JSON parse 6.33 ms; `bom_of` for a 5-line placement ≤ **1 µs** from the block (MEASURED 5 × 82 ns)
  vs 41.2 µs; block bytes ≤ 10 KB for 165 dishes (MEASURED 7,464); `cf.cpu_kills_day` (`tools/evals/collect/cf.mjs`)
  does not rise; `one-image` gate stays 0; `file-size` baseline does not rise.
- **Mutation proof:** corrupt one `row_ptr` in a scratch fixture → `refuse_bad_offsets` names the column.
- **Gates:** `cd crates/dowiz-hub && cargo test`, `cd workers/api && cargo test`, `sh tools/gates/run-all.sh`,
  `tools/gates/float-money.sh`, `tools/gates/dataflow.sh` (baseline 38 may only fall). **Lines EST:** ~250 Rust
  + tests. **Model:** Haiku (spec-exact codec) — Opus if the `bom_of` seam fights back.

### DG8 — the Datalog rule layer (DC Part A; R §12.1, §12.3)

- **Owns:** `+bebop-lang/selfhost/std/dl.bp` (new: relations, semi-naive engine, strata), `+bebop-lang/selfhost/std/gen_dl.bp`
  (new, generator in `gen_gb.bp`'s shape), `bebop-lang/bench/vs_rust/std_tests/dl_*.bp`, `bebop-lang/bench/oracles/dl_*.py`,
  `+bebop-lang/bench/vs_rust/dl_bench.sh` (new), `+bebop-lang/formal/Bebop/Datalog.lean` (new), `bebop-lang/docs/TRAPS.md`
  (E125/trap 125).
- **Depends on:** the switch (projection nodes exist); DC §A.3 layout; `bebop-lang/selfhost/std/qplan.bp`.
- **RED first:** `dl_neg_unsafe`, `dl_neg_cycle` (exit 125), `dl_bound` (trap 125), `dl_strata` vs
  `dl_strata.py`, `dl_set_semantics`, the five rule gates + `_neg` twins (DC A-9), `dagfull datalog`.
- **Acceptance (MEASURED basis R §12.3):** `dl_event_ns` ≤ **1,000** on the 165/120/352-nnz fixture (MEASURED
  82 ns Rust native; the bebop bound is 10x); incremental == scratch on every fixture after 10^4 random
  events; `allergen` derives the declared list of the sushi fixture with a printed diff (a finding, not a
  fix); `lake build` rc 0, `sorryAx` 0 for `Datalog.lean`.
- **Mutation proof:** `dagfull.prove.sh` step 4 (drop head replacement → stale `unavailable`).
- **Gates:** `std_golden`, `dagfull datalog`, `arch_check` (`gate-oracle`, file-size 800),
  `bebop-lang/tools/builtin_surface.py` unchanged (DC A-1). **Lines EST:** ~200 engine + ~150 generator + ~120
  Lean + oracles. **Model:** Opus.

### DG9 — codec round-trip gate and the block twins (DC §B.6)

- **Owns:** `+bebop-lang/selfhost/std/block.bp` (new: reader/writer twin, `gate block_rt`, `block_neg`,
  `block_money_neg`, `block_schema`); `+crates/bebop-wasm/src/block.rs` (new) + `oracle.py` (`--block`,
  `--schemas`) + `gate.sh` (blocks step); `+crates/dowiz-hub/fixtures/blocks/` shared with DG7 (DG7 creates,
  DG9 reads — sequence DG7 first or seed the fixtures from the spec's schema strings).
- **Depends on:** DC §B; DG7's fixtures.
- **RED first:** `gate.sh` blocks step 4/4 red until all readers agree on `n`, `nnz`, `K256` for each fixture
  and 1,000 seeded random blocks per type; the empty block.
- **Acceptance:** 4/4 on every fixture and type; `encode(decode(b)) == b` byte-exact in every reader; the
  bebop half box-only and SAID SO (never counted as agreement).
- **Mutation proof:** `gate.sh --prove` flips one byte per header region → a named refusal in every reader.
- **Gates:** `crates/bebop-wasm/gate.sh`, `.github/workflows/ci.yml` four-reader job (Rust + Python halves),
  `std_golden`. **Lines EST:** ~200 bebop + ~120 Rust + ~80 py. **Model:** Haiku.

---

## 4. Wave 3 — synergy rows (dowiz side unless said) and crypto-shredding

### DG10 — crypto-shredding for new logs + AEAD KAT (R §11.1; RT §12 C-4)

- **Owns:** `+crates/dowiz-hub/src/shred.rs` (new: per-person key table object, `seal_field`/`open_field`,
  `forget = drop key + append Forgotten`); `crates/dowiz-core/src/pq/aes_gcm.rs` (REUSED, unchanged: AES-256-GCM,
  KAT-gated against the McGrew–Viega vector — the research's "no AEAD in the tree" is corrected: none in
  `crates/dowiz-hub/src/crypto.rs`, one in `dowiz-core`); `workers/api/src/hubdo/forget.rs` (new logs take the
  shred path; history keeps in-place redaction, `crates/dowiz-hub/src/forget.rs:36`); a `dowiz-hub` feature
  `shred` that pulls `dowiz-core/pq` (header comment: what it pulls, how to verify the default graph stays
  clean — `CLAUDE.md` "Feature discipline").
- **Depends on:** the switch (blocks keep their `K256` under shredding, which is the point); P2/P3 of Wave P.
- **RED first:** `shred::tests::{kat_aes256gcm_vector_from_core, forget_makes_field_unreadable_everywhere,
  block_hash_unchanged_by_forget, key_table_holds_no_personal_bytes}`; a conservation law test that
  `chain.redacted == declared` still holds (`e2e/gates/conservation.mjs`).
- **Acceptance:** after `forget`, a byte search of every image AND every archived block for the phone's
  three spellings finds 0 (the P2 CHECK, `docs/design/ROADMAP-2026-09-22.md` Wave P), and every block's
  `K256` is unchanged (`cmp` before/after); KAT vector byte-exact; `personal-data` gate 0.
- **Mutation proof:** skip the key deletion → `forget_makes_field_unreadable_everywhere` red.
- **Gates:** `cd crates/dowiz-hub && cargo test --features shred`, `tools/gates/personal-data.sh`,
  `tools/gates/float-money.sh`, `cargo tree -e no-dev | grep -c aes` == 0 on the default graph.
  **Lines EST:** ~200 Rust. **Model:** Opus (crypto seam; never fake a primitive — `CLAUDE.md`).

### DW1 — browser replica as a graph subscriber (R §10 item 2)

- **Owns:** `workers/api/public/lib/replica.js`, `workers/api/src/hubdo.rs` (`changes_since :120` answers
  changed NODE keys + blocks), `workers/api/src/hubdo/menu.rs` (block bodies), `crates/bebop-wasm/src/lib.rs`
  (the browser recomputes the node and checks `K64`).
- **Depends on:** DG2, DG7, DG9. **RED first:** `e2e/walk` step: a replica whose recomputed fold differs from
  the server's `K64` MUST show "replica disagrees" loudly (today it silently trusts the server,
  `replica.js:14-18`). **Acceptance:** poll payload per change ≤ the changed blocks' bytes (EST from the block
  sizes); two folds can no longer silently disagree (the loud line fires in a scratch build that corrupts one
  block). **Mutation proof:** the scratch corruption. **Gates:** `design` gate, `sw-shell`, `ui-reach`.
  **Lines EST:** ~200 Worker + ~120 JS. **Model:** Haiku.

### DW2 — personal-data labels as the `personal(N)` rule (R §10 item 3; DC A.6 row 5)

- **Owns:** `workers/api/src/privacy/registry.rs` (emits the `holds_person` EDB), `tools/gates/personal-data.sh`
  (+ `.prove.sh`, `.baseline`: the count becomes `personal(N) ∧ ¬registered(N)` over the declared-edge table),
  the edge table `+workers/api/src/hubdo/edges.rs` (new: `(inputs, projection, step)` — the companion's Phase 4,
  `docs/research/2026-09-27-dag-architecture.md` §7).
- **Depends on:** DG8 (the rule engine) for the bebop evaluation; until then the gate evaluates the closure in
  Python over the same table (same answer, checked by the `dl_personal` golden). **RED first:**
  `personal-data.prove.sh` adds an unregistered projection of a labelled input → RED naming it. **Acceptance:**
  `personal-data 0` on HEAD; the prove fires; the edge table's row count equals the `/fold/*` route count
  (companion Phase 4 gate). **Mutation proof:** the prove. **Gates:** `personal-data`, `unreached`.
  **Lines EST:** ~150. **Model:** Haiku.

### DW3 — gates as graph queries (R §10 item 4)

- **Owns:** `tools/gates/dataflow.sh`, `tools/gates/one-image.sh`, `tools/gates/ui-reach.sh`,
  `tools/gates/unreached.py`, `tools/gates/learn.sh` — each rewritten to query `+workers/api/src/hubdo/edges.rs`'s table (via a
  generated `+tools/gates/edges.json`, emitted by a test so it cannot drift) instead of grepping; comments never
  counted (the epitaph trap, `docs/design/ROADMAP-2026-09-22.md` §5).
- **Depends on:** DW2's table. **RED first:** each gate's `.prove.sh` still fires in both directions.
  **Acceptance:** every rewritten gate prints the SAME number as before on HEAD (a rewrite changes no
  baseline), and its `.prove.sh` fires; total gate shell falls (3,809 lines today, `wc -l tools/gates/*.sh`).
  **Mutation proof:** the proves. **Lines EST:** ~80 per gate. **Model:** Haiku.

### DW4 — money conservation and stock-vs-orders as per-edge invariants (R §10 item 7)

- **Owns:** `workers/api/src/rebuild.rs` (`stale`, `stranded`, `unheld` as projection nodes whose output must
  be the empty set, evaluated on the dirty set, not nightly), `e2e/gates/conservation.mjs` (the nine laws
  read the projections). **Depends on:** the switch (persisted projections readable by `rebuild`).
  **RED first:** a test that appends a stock event without its order → the `stranded` node is non-empty
  within the same object turn. **Acceptance:** `rebuild.stale = []`, `stranded = []` per commit on the QA venue;
  nightly run unchanged as the cross-check. **Mutation proof:** skip one law → red. **Gates:** `conservation`.
  **Lines EST:** ~60 per law. **Model:** Haiku.

### DW5 — provenance "why is this number X" (R §10 item 8)

- **Owns:** `+workers/api/src/hubdo/provenance.rs` (new: walk the edge table backwards from a node, answer input
  node keys + generations), `workers/api/src/mcp.rs` (one read-only tool), `workers/api/public/admin/more.js`
  (one "why" link). **Depends on:** DW2. **Acceptance:** for a fixture, the answer lists exactly the input
  nodes (test); the MCP tool is in the catalogue (P16's `mcp-coverage` when it lands). **Lines EST:** ~80.
  **Model:** Haiku.

### DW6 — per-node CPU/bytes accounting (R §10 item 9)

- **Owns:** `tools/evals/collect/cf.mjs` (+ `cf.test.mjs`): `cf.node_cpu_us` per `/fold/*` route from the
  analytics API where it splits; `workers/api/src/hubdo.rs` (a per-node `cost_us` counter in the health
  gauges). **Acceptance:** the FREE-TIER budget (10 ms, 50 subrequests) is reported per node in the nightly
  eval with a baseline (`tools/evals/baselines/cf.baseline`). **Lines EST:** ~60. **Model:** Haiku.

---

## 5. The switch checklist (RT §10; R §8.3-8.4) — commands, in order, on the merged Wave-1 tree

All heavy steps through the slot; `$PRE` = the pre-jump commit; `$OUT` = a scratch dir under the session
scratchpad. Every step prints a NUMBER that goes in the merge commit message.

```sh
cd /root/dowiz/bebop-lang
# 0. control FIRST, at $PRE (memory head-may-not-reproduce-its-own-artifacts)
git -C /root/dowiz rev-parse HEAD                                  # = $PRE, quote it
bash tools/slot.sh switch python3 tools/arch_check.py > $OUT/arch.txt 2>&1; echo rc=$?; grep -n "binary" $OUT/arch.txt   # bebop.bin == compile(bebop.bp)
bash tools/slot.sh switch tools/battery.sh ./bebop.bin $OUT/pre > $OUT/battery-pre.txt 2>&1; echo rc=$?; tail -3 $OUT/battery-pre.txt
# 1. freeze the sweep: every .bp under the four trees, compiled by the PRE binary, hashed, committed (L13)
bash tools/slot.sh switch tools/cc.sh --raw bash bench/vs_rust/dagfull.sh --freeze bench/golden/dag-pre-$PRE > $OUT/freeze.txt 2>&1; echo rc=$?; tail -1 $OUT/freeze.txt   # prints "frozen <n> programs"
# 2. merge the three lanes into ONE commit (main only; lanes never git-write)
# 3. the compiler half
bash tools/slot.sh switch tools/chain.sh bebop.bp $OUT/chain --codegen > $OUT/chain.txt 2>&1; echo rc=$?; grep -n "gen3\|gen4\|md5" $OUT/chain.txt | tail -3   # gen3 == gen4
bash tools/slot.sh switch bash bench/vs_rust/dagfull.sh --sweep bench/golden/dag-pre-$PRE > $OUT/sweep.txt 2>&1; echo rc=$?; tail -1 $OUT/sweep.txt            # "dagfull sweep <n>/<n> pre=$PRE"
bash tools/slot.sh switch bash bench/vs_rust/dagfull.sh compile > $OUT/dfc.txt 2>&1; echo rc=$?; tail -1 $OUT/dfc.txt                                            # edit_ms_med <= 200
bash tools/slot.sh switch bash bench/vs_rust/becache_gate.sh > $OUT/bec.txt 2>&1; echo rc=$?; tail -1 $OUT/bec.txt                                              # PASS, warm bins identical
# 4. the store half
bash tools/slot.sh switch bash bench/vs_rust/dagfull.sh store > $OUT/dfs.txt 2>&1; echo rc=$?; tail -1 $OUT/dfs.txt                                             # "<m>/<m> readers 4/4"
(cd /root/dowiz/crates/bebop-wasm && bash ../../bebop-lang/tools/slot.sh switch sh gate.sh > $OUT/four.txt 2>&1; echo rc=$?; tail -2 $OUT/four.txt)              # 4 of 4 on kv.store AND proj.store
bash tools/slot.sh switch bash bench/vs_rust/scrash_torn.sh > $OUT/torn.txt 2>&1; echo rc=$?; tail -1 $OUT/torn.txt                                             # 0 invalid reopens, TRIALS=50
# 5. the scheduler and Lean
bash tools/slot.sh switch bash bench/vs_rust/dagfull.sh sched > $OUT/dfsched.txt 2>&1; echo rc=$?; tail -1 $OUT/dfsched.txt                                     # "<k>/<k>" twice
(cd formal && bash ../tools/slot.sh switch lake build > $OUT/lean.txt 2>&1; echo rc=$?; grep -c sorryAx $OUT/lean.txt; grep lean_conformance results.txt)      # rc 0, 0, >= 120/122
# 6. numbers must not degrade (gates outrank rows)
bash tools/slot.sh switch bash bench/vs_rust/honest.sh > $OUT/honest.txt 2>&1; echo rc=$?; grep -n "K1H\|K2H\|K3H\|K4" $OUT/honest.txt | tail -4               # each <= its pre row
grep -n "selfcompile_wall\|selfcompile_edit_wall\|chain_wall" docs/PERF.md | tail -3                                                                            # L-5, L-6
# 7. the whole battery on the candidate, then promote from the fixpoint
bash tools/slot.sh switch tools/battery.sh $OUT/chain/gen4.bin $OUT/post > $OUT/battery-post.txt 2>&1; echo rc=$?; tail -3 $OUT/battery-post.txt
# promotion: main copies gen4.bin over bebop.bin ONLY when every line above carried its number; then
bash tools/slot.sh switch python3 tools/arch_check.py > $OUT/arch2.txt 2>&1; echo rc=$?; grep -n "binary" $OUT/arch2.txt
git -C /root/dowiz status --short                                   # lanes write outside their tree: must be only the intended files
```

`dagfull.sh --freeze` and `--sweep` are DG6's; `dagfull.sh` MUST refuse with a named line if the frozen dir
is absent or its count differs from the live `.bp` count (RT G-1; L24). The sweep count is printed and goes in
the verdict — R §8.3's "360" is re-derived, not copied (RT §12 C-5).

---

## 6. Wave M — measurement rows (R §14: not verified, and the command that settles each)

| Row | Not verified (R §14) | Command that settles it | Acceptance = a number written into the row |
|---|---|---|---|
| **DG17** | the per-fn memo's real gain on `bebop.bp` | DG4's `+bebop-lang/bench/vs_rust/dagfull.sh compile` on `bebop.bp`: cold, one-line edit, cold again; `md5sum` equal; through `bebop-lang/tools/slot.sh` | `edit_ms_med` ≤ 200 and both outputs' md5 equal (this IS DG4's acceptance; the row exists so the number is recorded in `bebop-lang/docs/PERF.md` `selfcompile_edit_wall`) |
| **DG18** | the cause of the superlinear compile (exponent 1.85 MEASURED, loop HYPOTHESIS) | instrument `compile_fn_at` with `clock_ms()` per fn in a scratch copy of `bebop.bp`; plot ms vs fn index and vs call count | a fitted exponent per phase quoted; DG1's acceptance confirms or refutes the `fntab` scan hypothesis |
| **DG19** | the dowiz `orders_state` fold time on the live 5,830-cell log | `/fold/rebuild` wall on the QA hub (`qa-durres.dowiz.org`, Wave F F7) before and after R1 — R1 is landed; measure now | ms quoted, both arms |
| **DG20** | wasm SIMD inside a real Worker (measured under node 22 only) | deploy the R §12.3 probe crate (the research lane's scratch crate bdag-kern, not in the tree) as an owner-gated `/fold/probe-simd` route on the QA hub; time with `performance.now()`; remove after | ns per kernel in the Worker vs node; the DG25 decision reads this |
| **DG21** | AEAD for crypto-shredding | DG10: `cd crates/dowiz-core && cargo test --features pq aes_gcm` (KAT), then DG10's tests | KAT byte-exact; corrects R §14's "no AEAD" |
| **DG22** | the Lean confluence theorem (designed, not written) | DG6: `cd bebop-lang/formal && lake build` through the slot; `grep -c sorryAx` | rc 0, count 0 |
| **DG23** | B6's 1.00x scan | `bebop-lang/ROADMAP.md` B6 step 3 (a) as written; the DAG scheduler measured compute-bound tasks only | the number, whatever it is |
| **DG24** | any number on the Box (Raspberry Pi class) | the same probes pinned `-C target-cpu=cortex-a76` when a board exists | none until a board exists (recorded so it is not forgotten) |

---

## 7. Gated and refused rows

### DG25 — dense integer tensors + wasm SIMD for prices/stock (operator D-4; **OPERATOR DECISION NEEDED**)

- **What the operator decided:** dense/columnar integer tensors recomputed via v128 in the Worker and the hub
  object, the same kernels natively on the Box, a scalar oracle, byte-identical results.
- **What was MEASURED (R §12.3, 165/120/70 fixture; every kernel byte-identical scalar vs vector):** dense
  mask simd128 **9,749 ns** vs sparse scalar **1,089 ns** (8x slower; 580x at 100x); availability dense 14,886
  vs CSR 3,355 ns; price 2,068 vs 2,219 ns (0 %); all kernels 8.3 µs = 0.08 % of the 10 ms wall.
- **Row as written:** a MEASUREMENT row that can land only on its number. Owns `+crates/dowiz-hub/src/block/dense.rs`
  (new, behind feature `dense-simd`, `-C target-feature=+simd128` for wasm) + the DG20 Worker probe.
  **Acceptance:** dense simd128 mask ≤ 1,089 ns at 1x AND ≤ 137,533 ns at 100x on the real fixture, in the
  Worker (DG20), byte-identical to the scalar oracle; else the feature is deleted in the same lane and the
  row closes REFUTED with the numbers. **Lines EST:** ~150 Rust. **Model:** Haiku.
- **Decision the operator owns:** run DG25 as this measurement row, or withdraw the dense form and keep the
  Arrow-sense tensors (DC Part B) as the whole of D-4.

### DG26 — a bebop v128 emitter (R §8.5, §12.2) — NOT in the jump

No v128 emitter exists (`bebop.bp` has NEON only inside `hvham`/`hvham2`, `:1521-1600`); a vector register
tier is T64/T94-class work with its own register model and its own construct set; R §12.3 found no gain to
collect for prices/stock. Re-entry: DG25 lands GREEN (a dense kernel that beats sparse scalar in the Worker)
AND a Box exists where bebop, not Rust, must run it. Until then: refused, with the numbers above.

### Refused with the number that refuses them (R §6 "Do NOT")

| Do not | Number |
|---|---|
| expression-level dataflow / cell substrate as the execution model | 41x slower on dense work, crossover 0.39 % change (`bebop-lang/bench/substrate_spike/RESULT.md`) |
| in-place redaction under a pure CAS | breaks the block's address; crypto-shredding for new logs (DG10), declared redaction kept for history |
| a Rust `Store::from_bytes` copy on the hot path once projections persist | 31 ms copy vs 5.75 ms fold (R §4) — DG5's `view` twin |
| a second runtime beside the old, a staged migration, a parallel copy | operator D-2; the switch (§5) is the only path |

---

## 8. Conflicts needing an operator decision (numbered; both numbers)

| # | Operator | Research / tree | What this blueprint does meanwhile |
|---|---|---|---|
| **C-1** | prices/stock as DENSE tensors on wasm SIMD; "everything computes as tensors" | MEASURED dense simd128 mask 9,749 ns vs sparse scalar 1,089 ns (8x), 580x at 100x; SIMD 0 % on price/portions; codec 2,600x is where the time is (R §12.3, §13.2) | DC Part B (Arrow-sense tensors + scalar oracle) normative; DG25 as a gated measurement row |
| **C-2** | bebop v128 codegen so the Box runs the same kernels natively | no emitter exists; T64/T94-class; no gain to collect (R §12.2) | DG26 refused until DG25 is green and a Box exists |
| **C-3** | R §7.3's wording "the memo IS a `kv.bp` KV image" | `kv.bp` layout: 8x amplification, 42.8 ms per put (R §4) | RT §4: KV semantics, packed append-only layout; no decision needed unless the operator wants the literal `kv.bp` layout |
| **C-4** | R §11.1/§14: "no AEAD in the tree; one new primitive" | `crates/dowiz-core/src/pq/aes_gcm.rs` exists, KAT-gated | DG10 reuses it; a bebop-side AEAD twin is not scheduled |
| **C-5** | R §8.3: sweep = 360 programs (122 + 144 + 102 + neg) | `f334e5cb`: 102 + 20 + 138 + 127 `.bp` under the four trees | the freeze prints its count; the verdict quotes it |
| **C-6** | R: cold self-compile 14.5 s today | committed row 623-667 ms (`bebop-lang/docs/PERF.md`); the box is clocked at 0.3-0.4x (R §2.2) | every acceptance is a RATIO against a same-session control |
| **C-7** | "one jump" = the runtime replaced in one commit (D-2) | R §6: rows 1, 4, 5, 7, 8 are independent lanes that add no second runtime | Waves 0/2/3 are separate commits around the one-commit Wave 1. If "one jump" means one commit for EVERYTHING, Waves 0-2 fold into Wave 1 (3 lanes × ~2,600 lines) — decision needed only in that reading |
| **C-8** | Datalog "as vectorised joins over columns" | the 82 ns is the dirty set (O(fan-out)), not vectorisation; a whole-relation join IS the naive 3,316 ns (R §12.3) | DC §A.5: joins over the delta; whole-relation join only on a cold start |

## 9. OPEN (honest)

- The three Wave-1 lanes edit `bebop.bp` in disjoint regions by agreement (DG4: `cli_*`,
  `compile_program_offs`, `emit_bl*`; DG6: `collect_fns`, `diag_exit`, builtin dispatch). `bebop.bp` is ONE
  9,122-line file and the ROADMAP's rule is "one writer owns bebop.bp" (Open decisions, 2026-09-08). If main
  prefers one writer, DG6's compiler part (~80 lines: keywords + E124) moves into DG4 and DG6 keeps
  `sched.bp`/`dagfull`/Lean/constructs.
- Line estimates are EST from the code replaced; the box's clock (0.3-0.4x) makes every wall-time acceptance
  a ratio, so a lane MUST run its control in the same session.
- `KEEP_GENS = 16` (RT §4.3) is a HYPOTHESIS until a lane measures compiles-per-battery in `bebop-lang/docs/exp.journal`.
- No row here has run; every number is the research's or the tree's as of `f334e5cb`.
