Status: 2026-09-07 CURRENT (operator token-economy priority) -- the harness card every worker prompt points at instead of repeating it. Read once, in full.

# WORKER CARD (bebop-lang)

## Box (a Termux/proot phone; it has died from overload several times)
- the hard ceiling is ANDROID's, not ours: `max_phantom_processes` = 32, and the excess is SIGKILLed oldest-heaviest first (2026-09-08: five agents died together with slot.sh printing `procs 32/26`). ONE compile/run/chain/battery at a time; every heavy job through `tools/slot.sh <label> <cmd>` (ONE slot -- waiting on it is free); `bash tools/slot.sh --status` before starting one, and WAIT if procs is at or above 26/32. No `&` jobs, no background or sleep loops, no polling, no subagents.
- the chain runs in the FOREGROUND (Bash timeout 600000), never as a background job; a compiled program runs as `./seed/build/seed prog.bin [args]` (the .bin is not directly executable); a words-lane FAIL for new literals = `as` them + `objdump -d` into $BEBOP_TMP/words.objdump, then rerun `python3 tools/check_words.py`.
- prefix compiles/runs with `nice -n 10 taskset -c 0-3`; timing rows `taskset -c 4` alone.
- chain/battery ALWAYS `SERIAL=1 PROC_CAP=30`; exit 97 = cap: `tools/reap.sh`, then `tools/reap.sh kill`.
- every forked child is wait4'ed; `tools/reap.sh` after each probe and at the end. fuzz daemon and boxguard are REMOVED permanently (operator 2026-09-07); never start `tools/fuzzd.sh`, never install supervisor/daemon/wake-lock/cron on the box; fuzz batches run in FOREGROUND only.

## Gates
- codegen change: `SERIAL=1 PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT --codegen` (fixpoint = gen3 == gen4; FREEZE=1 implied; WORD_DELTA lines; census bcond/cbz/tbz never increase without a census_allow.txt line; bin_words growth needs a word_budget.txt line). Promote `cp $OUT/gen4.bin bebop.bin.tmp && mv bebop.bin.tmp bebop.bin` (never cp over bebop.bin), `bench/vs_rust/invariants.sh --freeze`, `SERIAL=1 BEBOP_TMP=$OUT FREEZE=1 SRC=bebop.bp tools/battery.sh ./bebop.bin $OUT/bat` -> `battery: GREEN`. New instruction words: `as` -> `objdump -d` -> python int into $OUT/words.objdump BEFORE editing (check_words.py).
- non-codegen change (libraries, std tests, oracles): no chain/battery; instead `python3 tools/typecheck.py bebop.bp bench/vs_rust/std_tests/*.bp bench/vs_rust/kernels/*.bp bench/parity_constructs/*.bp | tail -1` == `typecheck census: 0 findings`; `FORCE=1 J=1 bash bench/oracles/run_all.sh | tail -1` == mismatch=0 missing=0 (every `gate <name>` line needs bench/oracles/<name>.py whose LAST line is the golden); one full `BEBOP_TMP=$OUT nice -n 10 taskset -c 4 bash bench/vs_rust/std_golden.sh` at the end (lcjit passes pinned; if not, re-run it alone pinned and report both).
- oracles OUTSIDE /root/dowiz: `bench/oracles/rust/Cargo.toml` depends on `../../../crates/dowiz-core`, which lives outside bebop-lang at `/root/dowiz/crates`, so `money` and `ordfsm` come back ERR in ANY tree not rooted there -- and the failure is identical on the unpatched base, so it is never your change. `ln -s /root/dowiz/crates <your-tree-root>/crates` and re-run (2026-09-08, E1 lane).
- constructs: bench/parity_constructs/<name>.bp + `<name>) EXPECT=<v>;;` in bench/vs_rust/construct_parity.sh, EXPECT from `python3 tools/bpref.py <file>` (host-only builtins: derive by hand, show it in the comment).
- journal: ONE line appended to docs/exp.journal, `<epoch> H:... | DID:... | GOT:... | VERDICT:...`; a line containing flaky/transient/not reproducible must carry `rc=` and `EXPECT:`. Never edit old lines.

## Language / compiler traps
- exit 89 register pressure or > 8 live symbols across a clone (let-bind, split fns); exit 95 deeply nested parenthesised calls (write a loop); a nested `if` as a call argument must be let-bound; no `zeros` inside a while body (L8); `%` is the signed C remainder; array bounds are unchecked natively (an OOB store clobbers the next arena object; a bpref IndexError on a program the compiler runs is YOUR bug).
- trap 87 / "unresolved function": `objdump -D -b binary -m aarch64 <bin> | grep -c 'brk\s*#0x57'` must be 0; a missing `<out>.bin.use` means a `use` line was not seen. bebop.bp line 1 `use "selfhost/prelude/sha256.bp"` is never commented out. bebop.bp <= 511 fns.
- text scanners take `strn` from the caller (never str_len per char); scanners never advance the shared `pos`.
- store refs are typed (`ref GbArr`, `ref RP`, `ref <Struct>`), never carried as i64 (typecheck rung vii).
- fork = `sys_clone(17, sys_arena_base() + 16777216)` (never stack_top 0: the child rebinds x14/x15/x27/x28 to [stack_top, +12 MiB)); in-process run = mmap the .bin image RX and `sys_run(addr, size, argc, argv)`.
- before calling anything a compiler bug: run the repro through `git show <sha>:./bebop.bin` for the last 3 landed compilers -- identical outputs = your program's bug; a real one goes to $OUT/repro_<n>.bp (<= 15 lines), `open:`, stop.
- never `pkill -f <literal>`; never edit a bash script while it runs; never prefix a command with `S=`; `&&`-lists followed by `&` background the whole list; guard file variables before `sed -n ... $F` (an empty $F blocks on stdin forever).

## Modelling (added 2026-09-09 after the same error twice in one day)
- **Model the READ side, not only the write side.** Two rows were predicted correctly on writes and wrongly on reads in the same day. D4: packing L0 to u32 halved the promotion write exactly as predicted (-8,009,384 B, to the byte) and the log row went the WRONG WAY, +28 %, because `l0_load` reads L0 back ~3e6 times per run at ~53 ns per unpacked entry. C4: the view's break-even was predicted at 0.052 queries and measured 1.15, a 22x miss, because the recompute was sized against a full-graph traversal (110 ms) when the view's base is the DELTA (5 ms). Both times the write model was right. Before predicting, write down who reads the thing you changed, how often, and what the read costs -- and if you cannot, say the prediction covers writes only.
- A structure that assumes sparsity this workload does not have has now died FIVE times: step 2's blocks, 2''s row directory, 2'''s per-segment ranges, (b)'s lazy runs, C4's dense per-row views. Check the actual density first -- the binding promotion touches 0.632n distinct rows, not a small fraction.

## Token economy (operator priority 2026-09-07)
- hard cap: the prompt names N tool calls; at the cap write $OUT/STATE.md (files touched, md5s, goldens, what is RED, next step) and STOP with the VERDICT block -- no further reflection.
- no chatting, no progress narration; the VERDICT block is the whole report. Do not re-verify what the gate already proved.
- two honest attempts at a RED gate, then report the exact failing lines and stop.
- do not commit; the main session commits.
