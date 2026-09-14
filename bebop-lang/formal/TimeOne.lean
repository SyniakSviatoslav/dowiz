/-
  TimeOne -- an instrument, not a gate.

  `lake exe timeone <file.bp> <fuel>` parses ONE file, evaluates it, and prints
  the result together with the wall-clock milliseconds. It exists because the
  claim "the arena is quadratic" was an inference from reading
  `cells ++ Array.replicate n 0`, and an inference is not a measurement. Feed it
  a family of programs that differ only in iteration count and the scaling
  exponent is visible directly: a doubling of N that quadruples the time is
  quadratic, a doubling that doubles it is not.

  It prints ONE line per run, so a shell loop over N produces a table.
-/

import Bebop.Parser
import Bebop.Semantics

open Bebop Bebop.Parser Bebop.Semantics

def renderR : Result → String
  | .ok v => s!"ok {v.toInt}"
  | .trap _ => "trap"
  | .rejected c _ _ => s!"rejected {c}"
  | .stuck _ => "stuck"
  | .fuelExhausted f => s!"fuelExhausted {f}"

def main (args : List String) : IO UInt32 := do
  let path := args.headD ""
  let fuel := (match args[1]? with | some f => f.toNat? | none => none).getD 20000
  if path == "" then
    IO.eprintln "usage: timeone <file.bp> <fuel>"
    return 2
  let src ← IO.FS.readFile path
  match Bebop.Parser.parse src with
  | .error e =>
    IO.println s!"{path}\tfuel={fuel}\tPARSE-FAIL\t{e.render}"
    return 1
  | .ok prog =>
    let t0 ← IO.monoMsNow
    let r := evalProgram fuel prog
    -- Force the result before stopping the clock: Lean is lazy enough that
    -- `renderR` is what actually demands the evaluation.
    let rendered := renderR r
    let t1 ← IO.monoMsNow
    IO.println s!"{path}\tfuel={fuel}\t{rendered}\t{t1 - t0}ms"
    return 0
