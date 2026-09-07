# std_golden gate `store` — trap 82, pre-existing, compiler-independent

Status: 2026-09-08 SPEC (written by the main session; not a ROADMAP row — a battery blocker.
Every codegen row now lands with `battery: RED` and a "pre-existing" footnote until this closes.)

Read `docs/WORKER-CARD.md` first. This is a **non-codegen** task: never edit `bebop.bp`, never
run `tools/chain.sh`, never promote a binary.

## The observation (verified 2026-09-07)

`bench/vs_rust/std_golden.sh:155` runs

```
./seed/build/seed $BEBOP_BIN compile bench/vs_rust/std_tests/store.bp $BEBOP_TMP/store_test.bin
sleep 1 && run 60 $BEBOP_TMP/store_test.bin | tail -1
gate store 2245524994793680850 "$r"
```

The compile succeeds (rc 0) and the run prints

```
trap 82: SIGSEGV/SIGBUS (stack overflow or wild access)
```

so `$r` is empty and the gate reports `FAIL store: golden=2245524994793680850 got=`.

It reproduces **byte-identically on the OLD committed `bebop.bin`** (the pre-A14 binary at
commit f6da66d) as well as on the promoted A14 binary `7d8262a1` — therefore it is **not** a
compiler regression. The battery was GREEN with this gate in session 24 (`std_golden 111/111`,
docs/exp.journal), and the box was wiped and restored on 2026-09-07, so the most likely cause
is environmental, not a code change.

## What to find out, in this order

1. **Is it the store directory?** `store.bp` publishes through `sys_export` + `sys_rename`
   (renameat AT_FDCWD). Read `bench/vs_rust/std_tests/store.bp` and find every path it opens.
   Check that each directory exists and is writable under the current `BEBOP_TMP`, and that
   `renameat` works there — this box is a Termux **proot** with `--link2symlink`, where
   `rename()` across certain paths fails in ways ordinary filesystems do not (the same class of
   failure broke `npm`'s cacache on this box on 2026-09-07). A failing rename that the program
   does not check would leave a null/garbage handle and produce exactly this trap 82.
2. **Where exactly does it die?** Run the compiled `store_test.bin` under the seed with the
   arena/trap text visible, bisect the program by cutting `store.bp` down (copy it to
   `$OUT/store_min.bp`, never edit the original) until the trap disappears. The last removed
   statement is the site. Keep the shrink under 20 lines.
3. **Is `sleep 1` load-bearing?** The gate's own `sleep 1` between compile and run suggests a
   known publish/visibility race. If the trap is timing-dependent, say so with counts
   (e.g. "10 runs: 7 trap 82, 3 golden"), not adjectives.
4. **Does it depend on the path length?** Session 24 hit a `NAME_MAX` artefact in this same
   harness under a long `BEBOP_TMP`. Re-run with a short `BEBOP_TMP` (`/root/.cache/bebop/st`)
   and with a long one, and report both.

## Deliverable

- A root cause with evidence (which syscall, which path, which line of `store.bp`), or, if the
  cause is environmental, the exact environment fact plus the smallest change that makes the
  gate honest again — e.g. the harness creating the directory it assumes, or `store.bp`
  checking the rename's return value and exiting with a real diagnostic instead of walking into
  a wild access.
- The fix, if it is in the harness or in `store.bp`/the std library (both allowed: they are
  non-codegen). If the fix would need a change in `bebop.bp`, STOP and report — that is a
  codegen row and another worker owns the compiler.
- Verification per WORKER-CARD's non-codegen gate: `python3 tools/typecheck.py …` = 0 findings,
  `FORCE=1 J=1 bash bench/oracles/run_all.sh | tail -1` = `mismatch=0 missing=0`, and one
  pinned `BEBOP_TMP=$OUT nice -n 10 taskset -c 4 bash bench/vs_rust/std_golden.sh` showing
  `store` PASS (report the whole pass/fail line).
- ONE journal line in `docs/exp.journal`. Do not commit.

## Traps

- The golden `2245524994793680850` is the fold of a 220-byte "BT4R" stream that `bt.bp` also
  packs; if you find yourself editing the golden, you are solving the wrong problem.
- `run 60 …` is std_golden's own helper with a 60 s timeout; a hang and a trap look different in
  the log — quote the actual line.
- Never `pkill -f`; `tools/reap.sh` after each probe.
