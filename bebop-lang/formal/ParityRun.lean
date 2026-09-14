/-
  ParityRun -- the conformance harness that READS `bench/parity_constructs/`.

  This is the point of the parser. Until now `Conformance.lean` carried
  hand-typed copies of the programs, and 88 of 97 of them were not the file
  they named. Nothing here is transcribed: the `.bp` text and its
  `// EXPECT <v> from:` header are the only inputs, so that defect class is
  gone by construction rather than by care.

  WHAT IT REFUSES TO DO. It never scores a program it did not understand as a
  pass, and it never reports its own gaps as the language's. Every file lands
  in exactly one bucket and the bucket names keep the two apart:

    PASS              parsed, evaluated, value == EXPECT
    VALUE_MISMATCH    parsed and evaluated, value != EXPECT  <- a real defect,
                      in the semantics or in the parser. Never silent.
    STUCK             parsed, evaluator produced no value (unmodelled builtin,
                      string literal, unbound name, ...). A SEMANTICS gap.
    FUEL              evaluator ran out of fuel. Not a value.
    TRAP / REJECTED   evaluator reported a trap or a static rejection.
    UNSUPPORTED:<f>   the PARSER does not handle construct <f> yet.
                      A statement about formal/, not about the program.
    INVALID:<m>       the text is not a legal program. For neg/*.bp that is
                      the CORRECT outcome; for a positive construct it means
                      either the program is illegal (unlikely -- they all
                      compile) or this parser's grammar is wrong.
    LEX / PARSER_FUEL loud, and separate from everything above.

  Run it:  lake exe parityrun [<repo-root>]
-/

import Bebop.Basic
import Bebop.Lexer
import Bebop.Parser
import Bebop.Semantics

open Bebop Bebop.Parser Bebop.Semantics

/-- Default evaluation fuel per program; override with argv[2].

    MEASURED 2026-09-14, and the reason this is a knob rather than a constant:
    the evaluator's arena is a Lean `Array` written through `Array.set!`, so a
    construct that does `zeros(10000)` and then loops over it costs O(n) per
    store when the array is not held linearly. At fuel 400000 the corpus did
    not finish in 10 minutes. The fuel is printed with every run and `FUEL` is
    its own bucket, so a program that needs more is never confused with one
    that computes the wrong answer. -/
def defaultFuel : Nat := 20000

/-- The `// EXPECT <v> from:` header, which `bench/vs_rust/construct_parity.sh`
    also reads (it greps `// EXPECT [^ ]+ from:`). Same pattern, so the two
    harnesses cannot drift apart on what a construct expects. -/
def expectOf (src : String) : Option String :=
  let lines := src.splitOn "\n"
  let hit := lines.find? (fun l => (l.splitOn "// EXPECT ").length > 1 && (l.splitOn " from:").length > 1)
  match hit with
  | none => none
  | some l =>
    match (l.splitOn "// EXPECT ") with
    | _ :: rest :: _ => (rest.splitOn " ").head?
    | _ => none

/-- Drop leading spaces/tabs. `String.trim` returns a `String.Slice` in Lean
    v4.33.1 and a Slice does not carry `splitOn`, so this stays on `String`. -/
def lstrip (s : String) : String :=
  (s.toList.dropWhile (fun c => c == ' ' || c == '\t')).foldl (fun acc c => acc.push c) ""

/-- Textual inclusion for `use "path"` (LANGUAGE.md:25-31): the file is
    included once, dependencies first, deduplicated by path. `cas://sha256:...`
    is NOT resolved here -- it needs a `.bcas/` lookup and a digest check, and
    guessing would be exactly the silent-acceptance failure this harness is
    built to avoid; those reach the parser and come back as UNSUPPORTED. -/
partial def expandUses (root : String) (depth : Nat) (seen : Array String)
    (src : String) : IO (String × Array String) := do
  if depth == 0 then return (src, seen)
  let mut out : Array String := #[]
  let mut seen := seen
  for line in src.splitOn "\n" do
    let t := lstrip line
    if t.startsWith "use \"" then
      -- `String.drop` also returns a `String.Slice` in v4.33.1, so split on the
      -- quote instead of slicing: `use "p"` -> ["use ", "p", ...].
      let path := ((t.splitOn "\"")[1]?).getD ""
      if path.startsWith "cas://" then
        out := out.push line          -- leave it for the parser to refuse loudly
      else if seen.contains path then
        pure ()                        -- already included (dedup by path)
      else
        seen := seen.push path
        let full := root ++ "/" ++ path
        if ← System.FilePath.pathExists full then
          let inner ← IO.FS.readFile full
          let (expanded, seen') ← expandUses root (depth - 1) seen inner
          seen := seen'
          out := out.push expanded    -- dependencies FIRST
        else
          out := out.push line        -- missing file: let the parser complain
    else out := out.push line
  return (String.intercalate "\n" out.toList, seen)

inductive Outcome where
  | pass
  | valueMismatch (got want : String)
  | stuck (msg : String)
  | fuel
  | trapped (what : String)
  | unsupported (feature : String)
  | invalid (msg : String)
  | lexFail (msg : String)
  | parserFuel
  | noExpect
  deriving Inhabited

def Outcome.bucket : Outcome → String
  | .pass => "PASS"
  | .valueMismatch _ _ => "VALUE_MISMATCH"
  | .stuck _ => "STUCK"
  | .fuel => "FUEL"
  | .trapped _ => "TRAP/REJECTED"
  | .unsupported f => "UNSUPPORTED:" ++ f
  | .invalid _ => "INVALID"
  | .lexFail _ => "LEX"
  | .parserFuel => "PARSER_FUEL"
  | .noExpect => "NO_EXPECT"

def Outcome.detail : Outcome → String
  | .pass => ""
  | .valueMismatch g w => s!"got {g}, want {w}"
  | .stuck m => m
  | .fuel => "evaluator fuel exhausted"
  | .trapped w => w
  | .unsupported f => f
  | .invalid m => m
  | .lexFail m => m
  | .parserFuel => "parser fuel exhausted"
  | .noExpect => "no `// EXPECT <v> from:` header"

def renderResult : Result → String
  | .ok v => s!"ok {v}"
  | .trap _ => "trap"
  | .rejected c _ m => s!"rejected {c} ({m})"
  | .stuck m => s!"stuck: {m}"
  | .fuelExhausted f => s!"fuelExhausted {f}"

/-- Score ONE file. `positive` selects how the EXPECT header is read: a plain
    integer for `parity_constructs/*.bp`, a `COMPILEFAIL:<code>` /
    `RUNFAIL:<code>` tag for `neg/*.bp`. -/
def scoreOne (root : String) (fuel : Nat) (path : String) (positive : Bool) : IO Outcome := do
  let raw ← IO.FS.readFile path
  match expectOf raw with
  | none => return .noExpect
  | some want =>
    let (src, _) ← expandUses root 8 #[] raw
    match Bebop.Parser.parse src with
    | .error e =>
      match e with
      | .lex le => return .lexFail le.render
      | .unsupported f _ _ => return .unsupported f
      | .invalid m l c => return .invalid s!"{l}:{c}: {m}"
      | .outOfFuel _ _ => return .parserFuel
    | .ok prog =>
      let r := evalProgram fuel prog
      if positive then
        match want.toInt? with
        | none => return .noExpect
        | some w =>
          match r with
          | .ok v => if v.toInt == w then return .pass
                     else return .valueMismatch (toString v.toInt) (toString w)
          | .stuck m => return .stuck m
          | .fuelExhausted _ => return .fuel
          | other => return .trapped (renderResult other)
      else
        -- A negative construct must be REFUSED. This parser can refuse it for
        -- a grammar reason (INVALID) but it cannot check the compiler's exit
        -- CODE, and most of these are rejected by checks `formal/` does not
        -- model at all (type errors, reserved words, arity). Anything that
        -- parses and then evaluates to a value is reported as a MISS, loudly.
        match r with
        | .rejected c _ _ => return .trapped s!"rejected {c} (want {want}; code NOT compared)"
        | .trap _ => return .trapped s!"trap (want {want}; code NOT compared)"
        | .stuck m => return .stuck m
        | .fuelExhausted _ => return .fuel
        | .ok v => return .valueMismatch s!"ok {v.toInt}" s!"{want} (must be refused)"

structure Row where
  name : String
  outcome : Outcome

def listBp (dir : String) : IO (Array String) := do
  if !(← System.FilePath.pathExists dir) then return #[]
  let entries ← System.FilePath.readDir dir
  let mut out : Array String := #[]
  for e in entries do
    let p := e.path.toString
    if p.endsWith ".bp" then out := out.push p
  return out.qsort (· < ·)

def baseName (p : String) : String :=
  let parts := p.splitOn "/"
  let f := parts.getLast!
  (f.splitOn ".bp").headD f

/-- The score floor. `lake exe parityrun` exits 1 if the pass count drops below
    this, so a regression fails a command rather than being noticed later.
    RAISE it when the parser improves; never lower it to make a run green.

    83, MEASURED 2026-09-14 at the default fuel. The progression this round,
    each number from a full run of all 121 files:
      62  first run (parser fuel was `tokens.size + 8`, far too tight -- 17
          files, `c01_lit` among them, came back PARSER_FUEL)
      80  after fuel -> `tokens.size * 32 + 256`, `let _ = x = e in body`
          desugared, `kernel fn` / `test { }` / `requires`-`ensures` handled,
          and `return` in expression position moved to UNSUPPORTED
      83  after `ARR[i] = v` was allowed as an expression (store.bp:188)
      84  after a BARE `ARR[i] = v` was allowed as a body item and a tail
          expression (gb.bp ends a function with `cache[0] = cnt + 1`)
    At 84, NO positive construct is in the INVALID bucket any more: all six
    remaining INVALIDs are `neg/*.bp`, i.e. programs that SHOULD be refused. -/
def passFloor : Nat := 84

def main (args : List String) : IO UInt32 := do
  let root := args.headD "."
  let fuel := match args[1]? with
              | some f => (f.toNat?).getD defaultFuel
              | none => defaultFuel
  let posDir := root ++ "/bench/parity_constructs"
  let negDir := posDir ++ "/neg"
  let posFiles ← listBp posDir
  let negFiles ← listBp negDir
  if posFiles.isEmpty then
    IO.eprintln s!"parityrun: no .bp under {posDir} (pass the repo root as argv[1])"
    return 2
  let mut rows : Array (String × Row) := #[]
  -- Per-file progress on stderr, flushed. A run that stalls must name the file
  -- it stalled on; a silent ten-minute hang is not a measurement.
  for p in posFiles do
    IO.eprint s!"[pos] {baseName p} ... "
    (← IO.getStderr).flush
    let o ← scoreOne root fuel p true
    IO.eprintln o.bucket
    rows := rows.push ("positive", { name := baseName p, outcome := o })
  for p in negFiles do
    IO.eprint s!"[neg] {baseName p} ... "
    (← IO.getStderr).flush
    let o ← scoreOne root fuel p false
    IO.eprintln o.bucket
    rows := rows.push ("negative", { name := baseName p, outcome := o })

  -- bucket tally
  let mut buckets : Array (String × Nat) := #[]
  for (_, r) in rows do
    let b := r.outcome.bucket
    match buckets.findIdx? (fun q => q.1 == b) with
    | some i => buckets := buckets.setIfInBounds i (b, buckets[i]!.2 + 1)
    | none => buckets := buckets.push (b, 1)
  let tally := buckets.qsort (fun a b => a.2 > b.2)

  let total := rows.size
  let parsedOk := rows.foldl (fun n (_, r) =>
    match r.outcome with
    | .unsupported _ | .invalid _ | .lexFail _ | .parserFuel | .noExpect => n
    | _ => n + 1) 0
  let passes := rows.foldl (fun n (_, r) =>
    match r.outcome with | .pass => n + 1 | _ => n) 0

  IO.println s!"parityrun: {posFiles.size} positive + {negFiles.size} negative = {total} .bp files read from disk"
  IO.println s!"eval fuel per program: {fuel}"
  IO.println ""
  IO.println s!"PARSE   : {parsedOk} of {total} parsed into an AST"
  IO.println s!"EVALUATE: {passes} of {total} evaluated to their // EXPECT"
  IO.println ""
  IO.println "failures by reason (every file is in exactly one bucket):"
  for (b, n) in tally do
    IO.println s!"  {n}\t{b}"
  IO.println ""
  IO.println "detail (non-PASS only, first 120):"
  let mut shown := 0
  for (kind, r) in rows do
    match r.outcome with
    | .pass => pure ()
    | o =>
      if shown < 120 then
        shown := shown + 1
        IO.println s!"  [{kind}] {r.name}: {o.bucket} {o.detail}"
  IO.println ""
  if passes < passFloor then
    IO.eprintln s!"parityrun: REGRESSION -- {passes} passes is below the floor {passFloor}"
    return 1
  IO.println s!"parityrun: {passes} passes (floor {passFloor})"
  return 0
