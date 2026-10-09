/-
  THE AXIOM CENSUS for tools/gates/lean.sh. Run from bebop-lang/formal after `lake build`:

    lake env lean ../../tools/gates/lean-axioms.lean

  For EVERY constant declared in a `Bebop.*` module (theorems, defs, instances, the `axiom`s
  themselves, and the `_native.bv_decide` auxiliaries) it asks the kernel environment which axioms
  that constant transitively rests on -- `Lean.collectAxioms`, the function `#print axioms` calls --
  and prints one line per axiom found:

    AXIOM <axiom> <number of Bebop constants resting on it> <up to 3 of them>

  Walking every constant, not a hand-kept list of theorem names, is the point: a new `axiom` that no
  theorem uses yet, a `sorry` inside a helper def, or a new native-decide auxiliary all show up
  without anyone remembering to add a `#print axioms` line. `sorry` appears here as `sorryAx`.

  The last line is `lean-axioms: <axioms> axiom(s) over <constants> constant(s) in <modules>
  module(s)`; the gate refuses a census that scanned no module, because an empty list is also what
  a broken import would print.
-/
import Lean
import Bebop
open Lean Elab Command

#eval show CommandElabM Unit from do
  let env ← getEnv
  let mods := env.header.moduleNames
  let mut users : Std.HashMap Lean.Name (Array Lean.Name) := {}
  let mut nConst := 0
  let mut nMod := 0
  for i in List.range mods.size do
    let m := mods[i]!
    if !(Lean.Name.isPrefixOf `Bebop m) then continue
    nMod := nMod + 1
    for c in env.header.moduleData[i]!.constNames do
      nConst := nConst + 1
      for ax in ← collectAxioms c do
        users := users.insert ax ((users.getD ax #[]).push c)
  let names : Array Lean.Name := (users.toArray.map (·.1)).qsort (fun a b => a.toString < b.toString)
  for ax in names do
    let us : Array Lean.Name := (users.getD ax #[]).qsort (fun a b => a.toString < b.toString)
    let shown := String.intercalate "," ((us.extract 0 3).toList.map toString)
    IO.println s!"AXIOM {ax} {us.size} {shown}"
  IO.println s!"lean-axioms: {names.size} axiom(s) over {nConst} constant(s) in {nMod} module(s)"
