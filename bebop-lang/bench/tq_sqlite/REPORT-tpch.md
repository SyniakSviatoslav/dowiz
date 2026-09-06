**PLACEHOLDER -- not a real timing run.** This file was produced by a single R=1 smoke
test of `bench/tq_sqlite/tpch_twin.sh` (contended box, other workers active on the same
cores) to verify the harness's mechanics -- process wiring, fold cross-checks, table/gate
formatting -- work end to end. Every number below is noise (do not cite it, do not treat
the gate FAIL as a real result). Fold checks ARE real and passed. Re-run for the real gate
on a quiet box, R=11: `BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/tq_sqlite/tpch_twin.sh`
(pins core 4 itself). See `bench/tq_sqlite/B7-PREP.md` for what this covers.

---

# REPORT-tpch -- B7 store-side prep twin, 600,000-row lineitem, pinned core 4 (A55), R=1 medians, bebop.bin f7a25d38

- load: bench/tq_sqlite/tpch_load.bp, 250 ms in-process (informational, not gated), 600000 rows, 7 columns as store `arr i64` objects + root, one commit.
- fold check Q6: bebop =={114672059591}==oracle (sqlite twin folds: [114672059591])
- fold check Q1: bebop =={6105941479581644684}==oracle (sqlite twin folds: [6105941479581644684])

| query | bebop (whole-process, us) | sqlite native (ctypes prepared, us, VM_STEP floor) | ratio | gate | first-query (tier 0) | repeat (pool) |
|---|---|---|---|---|---|---|
| Q6 | 108722.4 | 200597.0 (vm_steps 566744) | 1.8x | FAIL <10x | n/a (no planner/pool yet) | n/a (no planner/pool yet) |
| Q1 | 112752.0 | 1826785.1 (vm_steps 16003199) | 16.2x | PASS >=5x | n/a (no planner/pool yet) | n/a (no planner/pool yet) |

Note: bebop numbers above are whole-`seed`-process wall time (fork+exec+mmap+scan),
NOT an in-process kernel timer -- these hand-written kernels are functional-parity
templates for the B7 generator (docs/blueprints/B7-dsl-planner.md section 5 step 2),
not yet wired to the register-model / pool timing infra the real gate needs. The
ratio column is therefore a lower bound on bebop's advantage (process overhead is
shared by both sides only for sqlite's own python/ctypes startup, not for `seed`'s
own fixed costs), reported honestly rather than gated as a final number.
- VERDICT: GREEN if fold checks OK and gate6/gate1 both PASS, else RED. This run: RED
