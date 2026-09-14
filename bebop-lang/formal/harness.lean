-- harness.lean — F4 conformance driver (runs ON-BOX, measured 2026-09-13)
-- Runs 5 sample constructs (ASTs transcribed from bench/parity_constructs/*.bp)
-- through the Lean interpreter and prints PASS/FAIL against the EXPECT rows,
-- then 5 PROBE fixtures (Conformance.lean §5b), one per fake closed on
-- 2026-09-13: syscalls answering `some 0`, silent fuel exhaustion, caller env
-- lost after a call. Probes are counted on their own line and are NOT
-- conformance: a probe PASS means the fake is absent, not that bebop.bin agrees.
--
-- Prints two grep-able lines and exits 1 if either count is short:
--   lean_conformance: N/5      (samples whose value matches the EXPECT row)
--   lean_probes: M/5           (fakes shown absent)
--
-- Usage (no mathlib needed; LEAN_PATH is required because `lean --run` does
-- not read lakefile.lean). Both steps are heavy jobs and go through the slot:
--   cd formal && PERF=0 ../tools/slot.sh lean-build /root/s30/outC4/lean/bin/lake build
--   PERF=0 ../tools/slot.sh lean-harness env LEAN_PATH=$PWD/.lake/build/lib/lean \
--     /root/s30/outC4/lean/bin/lean --run harness.lean
--
-- References:
--  - bench/vs_rust/construct_parity.sh (EXPECT rows)
--  - docs/blueprints/F3-lean-semantics.md §4.4 (syscall placeholder must be `none`)
--  - tools/kcheck.py whnf (fuel exhaustion raises; the pattern mirrored here)

import Bebop.Basic
import Bebop.Semantics
import Bebop.Builtins
import Bebop.Syscalls
import Bebop.Traps
import Bebop.Conformance

open Bebop.Semantics
open Bebop.Builtins
open Bebop.Syscalls
open Bebop.Traps
open Bebop.Conformance

-- ============================================================
-- Helper: print a Result as a string
-- ============================================================

def printResult (r : Result) : String :=
  match r with
  | .ok v => "ok " ++ toString v
  | .trap c => "trap(" ++ reprStr c ++ ")"
  | .rejected code _ msg => "rejected(" ++ toString code ++ "," ++ msg ++ ")"
  | .stuck msg => "stuck(" ++ msg ++ ")"
  | .fuelExhausted f => "FUEL EXHAUSTED(" ++ toString f ++ ")"

-- ============================================================
-- Helper: run a sample and print PASS/FAIL
-- ============================================================

def runSample (name : String) (got : Result) (exp : Expected) : IO Bool := do
  let pass := checkExpected got exp
  if pass then
    IO.println s!"✓ {name}: {printResult got}  (expected {exp.verdict})"
  else
    IO.println s!"✗ {name}: {printResult got}  (expected {exp.verdict})"
  return pass

/-- Run a list of (name, got, expected) rows; return the pass count. -/
def runRows (rows : List (String × Result × Expected)) : IO Nat := do
  let mut n := 0
  for (name, got, exp) in rows do
    if (← runSample name got exp) then n := n + 1
  return n

-- ============================================================
-- Main: run 5 sample constructs
-- ============================================================

def main : IO Unit := do
  IO.println "=== F4 Conformance Harness (sample run) ==="
  IO.println ""
  IO.println "--- Samples (EXPECT rows from bench/parity_constructs) ---"
  let samples : List (String × Result × Expected) := [
    ("c01_lit",   sample_c01.1, sample_c01.2),
    ("c02_arith", sample_c02.1, sample_c02.2),
    ("c07_while", sample_c07.1, sample_c07.2),
    ("c08_call",  sample_c08.1, sample_c08.2),
    ("c13_array", sample_c13.1, sample_c13.2) ]
  let nSamples ← runRows samples

  IO.println ""
  IO.println "--- Probes (a PASS means the named fake is absent; not conformance) ---"
  let probes : List (String × Result × Expected) := [
    ("p01a_syscall_unmodelled",  probe_p01a_syscall_unmodelled.1,  probe_p01a_syscall_unmodelled.2),
    ("p01b_sys_exit_unmodelled", probe_p01b_sys_exit_unmodelled.1, probe_p01b_sys_exit_unmodelled.2),
    ("p01c_user_fn_sys_prefix",  probe_p01c_user_fn_sys_prefix.1,  probe_p01c_user_fn_sys_prefix.2),
    ("p02_fuel_exhaustion",      probe_p02_fuel_exhaustion.1,      probe_p02_fuel_exhaustion.2),
    ("p03_caller_env",           probe_p03_caller_env.1,           probe_p03_caller_env.2) ]
  let nProbes ← runRows probes

  IO.println ""
  IO.println s!"--- Summary ---"
  IO.println s!"lean_conformance: {nSamples}/{samples.length}"
  IO.println s!"lean_probes: {nProbes}/{probes.length}"
  IO.println s!"Positive constructs: {positiveConstructCount}"
  IO.println s!"Negative constructs: {negativeConstructCount}"
  IO.println s!"Total constructs: {totalConstructCount}"
  IO.println s!"Program entries in Conformance.lean: {oracleCount} (ROADMAP F4 target {oracleProgramTarget})"
  IO.println s!"Constructs actually RUN by the interpreter: {executedConstructCount} of {suiteConstructCount} on disk"
  IO.println s!"Executable builtins: {executableBuiltinCount}"
  IO.println s!"Axiomatised builtins: {axiomatisedBuiltinCount}"
  IO.println s!"Total builtins: {builtinCount}"
  IO.println s!"Trap rows (F1): {trapCount} (closed {closedTrapCount}, open {openTrapCount})"
  IO.println ""
  IO.println "=== Done ==="
  -- Fail loud: a short count is a non-zero exit, never a green rc with a red line.
  if nSamples != samples.length || nProbes != probes.length then
    IO.Process.exit 1
