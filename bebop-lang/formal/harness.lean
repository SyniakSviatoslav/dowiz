-- harness.lean — F4 conformance driver (OFF-BOX)
-- #eval-driven: runs 5 sample constructs through the Lean interpreter
-- and prints PASS/FAIL against construct_parity.sh EXPECT rows.
--
-- Usage (off-box, on a machine with lean 4 + mathlib):
--   cd formal && lake build && lean --run harness.lean
--
-- The output is captured to formal/results.json, whose hash is
-- bound to the .lean sources and checked by the chain-side Python step.
--
-- References:
--  - bench/vs_rust/construct_parity.sh (EXPECT rows)
--  - docs/RESEARCH-VERIFICATION-2026-09-09.md §5.3 (F3 blueprint)

import Bebop.Basic
import Bebop.Semantics
import Bebop.Builtins
import Bebop.Syscalls
import Bebop.Traps
import Bebop.Conformance

open Bebop.Basic
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
  | .trap c => "trap(" ++ toString c ++ ")"
  | .rejected code pos msg => "rejected(" ++ toString code ++ "," ++ msg ++ ")"

-- ============================================================
-- Helper: run a sample and print PASS/FAIL
-- ============================================================

def runSample (name : String) (got : Result) (exp : Expected) : IO Unit :=
  let pass := checkExpected got exp
  if pass then
    IO.println s!"✓ {name}: {printResult got}  (expected {exp.verdict})"
  else
    IO.println s!"✗ {name}: {printResult got}  (expected {exp.verdict})"

-- ============================================================
-- Main: run 5 sample constructs
-- ============================================================

def main : IO Unit := do
  IO.println "=== F4 Conformance Harness (sample run) ==="
  IO.println ""

  -- Sample 1: c01_lit — integer literals and wrapping
  let (got1, exp1) := sample_c01
  runSample "c01_lit" got1 exp1

  -- Sample 2: c02_arith — arithmetic operators
  let (got2, exp2) := sample_c02
  runSample "c02_arith" got2 exp2

  -- Sample 3: c07_while — while loop
  let (got3, exp3) := sample_c07
  runSample "c07_while" got3 exp3

  -- Sample 4: c08_call — function call
  let (got4, exp4) := sample_c08
  runSample "c08_call" got4 exp4

  -- Sample 5: c13_array — array literal + index + set
  let (got5, exp5) := sample_c13
  runSample "c13_array" got5 exp5

  IO.println ""
  IO.println s!"--- Summary ---"
  IO.println s!"Positive constructs: {positiveConstructCount}"
  IO.println s!"Negative constructs: {negativeConstructCount}"
  IO.println s!"Total constructs: {totalConstructCount}"
  IO.println s!"Oracle programs: {oracleProgramCount}"
  IO.println s!"Executable builtins: {executableBuiltinCount}"
  IO.println s!"Axiomatised builtins: {axiomatisedBuiltinCount}"
  IO.println s!"Total builtins: {builtinCount}"
  IO.println s!"Trap rows (F1): {trapCount} (closed {closedTrapCount}, open {openTrapCount})"
  IO.println ""
  IO.println "=== Done ==="
