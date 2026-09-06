Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on A1-A3 landed and promoted (the md5 under test is the promoted ./bebop.bin after A3)

# A4 The 24 h fuzz freeze window (process, no code)

## 0. Goal

`fuzz_seeds_on_bin >= 10^5` on ONE promoted md5 with 0 CRASH / DIVERGE / COMPILEFAIL and TRAP-82 = 0 (TG-DONE 8, D12-C, D14 item 12). Nothing lands on bebop.bp during the window; docs, oracles, python tooling, twins (A9's Rust scan twin, B1's join twin) are parallel-safe.

## 1. Scope

In: the runbook below; the journal lines fuzzd writes; the ALERT handling; the PERF row. Out: any codegen change (a change resets the per-binary counter: docs/PERF.md `fuzz_seeds_on_bin` is keyed by md5 -- tools/perf.py:280, verified). Fixed points: tools/fuzzd.sh as deployed (Termux runit service `fuzzd`, own proot; stop = STOP FILE, `sv down fuzzd` also touches it; `tools/fuzzd.sh pause`/`resume` park the loop between batches -- tools/fuzzd.sh:8-30, verified); bench/fuzz/fuzz.sh classes OK DIVERGE COMPILEFAIL CRASH TIMEOUT BPREF-* GENFAIL STRAY TRAP-OK TRAP-UNPREDICTED TRAP-81 TRAP-82 (fuzz.sh:68-71, verified: TRAP-81 is a pass by design, TRAP-82 = SIGSEGV/SIGBUS is the alert class).

## 2. Preconditions

A3 promoted and pushed; `bench/vs_rust/invariants.sh --freeze` done; the box idle (no chains: fuzzd uses the LITTLE cores 0xd05 at nice 10, but a chain's proc-cap gate counts its ~5 processes -- fuzzd.sh:20-33, memory pitfall); session-18 state: fuzzd is PAUSED (`tools/fuzzd.sh pause`, next seed 145600) -- resume it as step 1.

## 3. Runbook

1. Confirm the target: `md5sum bebop.bin` == the promoted fixpoint of A3's commit; `git status` clean; `tools/reap.sh` clean.
2. `tools/fuzzd.sh resume` (removes the pause file; the daemon in its own proot continues at `$FUZZD/next`). If `tools/fuzzd.sh status` says not running: from Termux `sv up fuzzd`; never `tools/fuzzd.sh start` from a session proot (it dies or spins with the session -- fuzzd.sh:11-13).
3. Rate today: 1.5-3.5 programs/s on the little cores (TG-DONE 8 row) -> 10^5 seeds = 8-18 h. Batches of N = 2000 (default) write one journal line each (`H:fuzzd batch ...`), so progress = `grep -c 'H:fuzzd batch' docs/exp.journal` since the resume timestamp, or `tools/fuzzd.sh status`.
4. Every ~6 h: `python3 tools/perf.py run --bin bebop.bin ...` is NOT needed -- the row is refreshed by the next chain; read the raw count: `grep 'bin=<md5-prefix>' ~/.cache/bebop/fuzzd/log | awk '{s+=$3} END{print s}'` (the `fuzz: N=... bin=` summary lines; perf.py:271 parses the same fields).
5. ALERT (`$FUZZD/ALERT` exists; the pre-push hook refuses to push while it exists -- fuzzd.sh:5): read `$FUZZD/repros/`; classify: DIVERGE = compare bebop vs bpref output (bpref is the oracle; an undefined program per LANGUAGE.md loop-release rule is a generator bug, fix gen.py, keep the counter); CRASH = re-run 3x (fuzz.sh:14 already does), then the miscompile recipe (top-down cuts, <= 15-line probe, objdump gen2 vs gen3 if self-compile related); COMPILEFAIL with a new exit code = a capacity trap the generator should predict (TRAPS.md) -- add the prediction to gen.py/bpref, keep the counter; a real miscompile ENDS the freeze: fix as a normal chain commit (new md5, counter restarts), journal `VERDICT:refuted` on the window.
6. TRAP-82 > 0 is an ALERT even without DIVERGE (D12-C): same handling as CRASH.
7. Done when the summed seeds on the md5 >= 100000 with all counters 0: run one chain (no `--codegen`) so docs/PERF.md carries the `fuzz_seeds_on_bin` row for the md5; journal line `H:A4 24 h fuzz window on <md5> | DID:fuzzd resume ... | GOT:seeds=<n> DIVERGE=0 CRASH=0 COMPILEFAIL=0 TRAP-82=0 | VERDICT:confirmed`; HISTORY progress-log entry; ROADMAP TG-DONE 8 row updated.
8. Then `tools/fuzzd.sh pause` again only if the next task needs the cores/processes (A5 chains do); otherwise leave it running as the regression shield (its counter keeps growing on the same md5 until the next promotion).

## 4. Files touched

docs/exp.journal (by fuzzd and the closing line), docs/PERF.md (by the closing chain), HISTORY.md, ROADMAP.md TG-DONE 8 -- no source files.

## 5. Gates

`fuzz_seeds_on_bin >= 100000` on one md5, DIVERGE = CRASH = COMPILEFAIL = 0, TRAP-82 = 0, no ALERT file. RED = any ALERT that turns out to be a miscompile (then A4 restarts after the fix).

## 6. Risks

| risk | handling |
|---|---|
| the box needs the processes for a chain during the window | `tools/fuzzd.sh pause` (parks after the batch), chain, `resume`; the counter is per md5, pausing does not reset it |
| a docs-only commit changes nothing in bebop.bin | fine: the md5 is unchanged, the window continues |
| a session ends with fuzzd paused | memory note + `tools/fuzzd.sh status` at session start |
| generator exhaustion (same shapes) | gen.py widening is a docs/tooling change, parallel-safe; note the widened classes in the journal |

## 7. VERDICT format

```
VERDICT: confirmed|refuted
md5: <bebop.bin>
seeds: <n> (>= 100000)   DIVERGE=<0> CRASH=<0> COMPILEFAIL=<0> TRAP-82=<0> TRAP-81=<n, pass>
alerts handled: <list or none>
journal: <closing line>
```

## 8. Worker prompt skeleton

This task is operated by the main session (no codegen; the Sonnet worker is only needed if an ALERT requires the miscompile recipe). If delegated: <context> the runbook, the fuzzd facts, the classification rules </context> <constraints> no bebop.bp edits during the window; ALERT handling per §3.5; reap after chains </constraints> <output_format> §7 </output_format> <task> run the window to 10^5 seeds and report </task>.
