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
import Bebop.Reject
import Bebop.Sha256

open Bebop Bebop.Parser Bebop.Semantics

/-- Default evaluation fuel per program; override with argv[2].

    RAISED from 20000 to 2000000 on 2026-09-14, because the reason it had to be
    small is gone. It was small because the arena was an `Array` whose
    allocation was quadratic: at fuel 400000 the corpus did not finish in ten
    minutes. With the sparse arena (see `Arena` in Bebop/Basic.lean) the whole
    121-file corpus runs in 797 ms at fuel 20000 and 2310 ms at 2000000, so the
    fuel can be set where the CONSTRUCTS need it instead of where the arena
    tolerated it. Two constructs move from FUEL to PASS as a result:
    `c33_loopalloc` (100000 iterations, so it needs > 100000 fuel) and
    `c67_deeprec`.

    RAISED AGAIN, 2000000 -> 4000000, on 2026-09-14. The sentence that used to
    end this paragraph -- "`neg/c48_stackovf` stays FUEL at any budget and
    should" -- was WRONG, and it was wrong in the direction that matters: the
    construct's EXPECT is `RUNFAIL:82`, a stack overflow, and reporting
    exhaustion for it means the model has no notion of a bounded stack at all.
    `Bebop.Semantics.callDepthLimit` is that notion, and the budget has to be
    large enough for the evaluator to REACH depth 131072 before the fuel (which
    is a depth budget on the whole evaluation tree, ~20 units per activation)
    runs out. Measured after the change: c48 reports trap 82, and a program
    that merely loops a long time still reports FUEL.

    The fuel is printed with every run and `FUEL` is its own bucket, so a
    program that needs more is never confused with one that computes the wrong
    answer. -/
def defaultFuel : Nat := 4000000

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
    included once, dependencies first, deduplicated by path.

    `cas://sha256:<64 hex>` IS resolved now (it was left for the parser to
    refuse, as UNSUPPORTED, because "guessing would be exactly the silent-
    acceptance failure this harness is built to avoid" -- correct, and the
    answer was to stop guessing, not to stop resolving). The rule is
    `bebop.bp:8007` (`cas_prefix` / `cas_path` / `cas_verify`) and
    `tools/bpref.py:813`, which agree:

      1. the path is `.bcas/<hex>.bp`;
      2. after reading it, its SHA-256 must EQUAL `<hex>`, or the compile
         exits 88 -- "a module is named by what it IS".

    Step 2 is the whole content of the pair c50_cas / neg/c51_casbad: the two
    `.bcas` files in this tree are byte-identical, so only the digest tells
    them apart. A missing `.bcas/<hex>.bp` is also exit 88 (there is no module
    with that name).

    Returns the expanded text, the seen-set, and -- if a digest check failed --
    the refusal, which the caller turns into `Result.rejected 88`. -/
partial def expandUses (root : String) (depth : Nat) (seen : Array String)
    (src : String) : IO (String × Array String × Option String) := do
  if depth == 0 then return (src, seen, none)
  let mut out : Array String := #[]
  let mut seen := seen
  let mut bad : Option String := none
  for line in src.splitOn "\n" do
    let t := lstrip line
    if t.startsWith "use \"" then
      -- `String.drop` also returns a `String.Slice` in v4.33.1, so split on the
      -- quote instead of slicing: `use "p"` -> ["use ", "p", ...].
      let path := ((t.splitOn "\"")[1]?).getD ""
      if seen.contains path then
        pure ()                        -- already included (dedup by path)
      else if path.startsWith "cas://sha256:" then
        seen := seen.push path
        let want := String.intercalate ":" ((path.splitOn ":").drop 2)
        let full := root ++ "/.bcas/" ++ want ++ ".bp"
        if ← System.FilePath.pathExists full then
          let bytes ← IO.FS.readBinFile full
          let got := Bebop.Sha256.hex bytes.toList.toArray
          if got == want then
            let inner ← IO.FS.readFile full
            let (expanded, seen', bad') ← expandUses root (depth - 1) seen inner
            seen := seen'
            if bad'.isSome then bad := bad'
            out := out.push expanded
          else
            bad := some s!"`use \"{path}\"`: .bcas/{want}.bp hashes to {got}, not to the name it is filed under (bebop.bp cas_verify, exit 88)"
        else
          bad := some s!"`use \"{path}\"`: no .bcas/{want}.bp (exit 88)"
      else
        seen := seen.push path
        let full := root ++ "/" ++ path
        if ← System.FilePath.pathExists full then
          let inner ← IO.FS.readFile full
          let (expanded, seen', bad') ← expandUses root (depth - 1) seen inner
          seen := seen'
          if bad'.isSome then bad := bad'
          out := out.push expanded    -- dependencies FIRST
        else
          out := out.push line        -- missing file: let the parser complain
    else out := out.push line
  return (String.intercalate "\n" out.toList, seen, bad)

/-- What a `// EXPECT` header asks for. `neg/*.bp` headers are
    `COMPILEFAIL:<code>` or `RUNFAIL:<code>`; positives are a plain integer.
    The CODE is kept, not discarded: until 2026-09-14 the negative arm printed
    "code NOT compared" and any refusal was accepted for any code. -/
inductive Expect where
  | value (v : Int)
  | compileFail (code : Nat)
  | runFail (code : Nat)
  | unparsable (raw : String)

def parseExpect (want : String) (positive : Bool) : Expect :=
  if positive then
    match want.toInt? with
    | some v => .value v
    | none   => .unparsable want
  else
    match want.splitOn ":" with
    | ["COMPILEFAIL", c] => match c.toNat? with
                            | some n => .compileFail n
                            | none => .unparsable want
    | ["RUNFAIL", c]     => match c.toNat? with
                            | some n => .runFail n
                            | none => .unparsable want
    | _ => .unparsable want

inductive Outcome where
  /-- `diag` is empty for an ordinary value pass. For a NEGATIVE construct that
      this harness refuses STATICALLY, it carries the refusal's own diagnosis,
      which is printed beside the pass: the refusal may be for the right reason
      or the wrong one, and the score alone cannot tell them apart. -/
  | pass (diag : String)
  | valueMismatch (got want : String)
  /-- Refused, but with an exit code that is not the one the construct asks
      for, or refused at the wrong TIME (statically for a `RUNFAIL`, at runtime
      for a `COMPILEFAIL`). A failure. -/
  | codeMismatch (got want : String)
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
  | .pass _ => "PASS"
  | .valueMismatch _ _ => "VALUE_MISMATCH"
  | .codeMismatch _ _ => "CODE_MISMATCH"
  | .stuck _ => "STUCK"
  | .fuel => "FUEL"
  | .trapped _ => "TRAP/REJECTED"
  | .unsupported f => "UNSUPPORTED:" ++ f
  | .invalid _ => "INVALID"
  | .lexFail _ => "LEX"
  | .parserFuel => "PARSER_FUEL"
  | .noExpect => "NO_EXPECT"

def Outcome.detail : Outcome → String
  | .pass d => d
  | .valueMismatch g w => s!"got {g}, want {w}"
  | .codeMismatch g w => s!"got {g}, want {w}"
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
  | .trap c => s!"trap {c.name} (exit {c.exitCode})"
  | .rejected c _ m => s!"rejected {c} ({m})"
  | .stuck m => s!"stuck: {m}"
  | .fuelExhausted f => s!"fuelExhausted {f}"

/-- THE NEGATIVE RULE, and the one thing it must never become.

    A `neg/*.bp` whose header is `COMPILEFAIL:<code>` is a PASS when the
    harness refuses it STATICALLY -- LEX, INVALID, or `REJECTED <code>` from
    `Bebop.Reject`. Where the refusal names a code the codes are COMPARED and a
    mismatch is `CODE_MISMATCH`, a failure. Where the refusal is a grammar
    refusal that names no code, the pass is counted and the parser's own
    diagnosis is PRINTED beside it, so a refusal for the wrong reason is
    visible in the log instead of hidden by the score.

    What it must never become: a rule that passes anything the harness could
    not evaluate. `UNSUPPORTED` (a gap in this parser), `STUCK` (a gap in this
    evaluator) and `FUEL` are NOT passes, for a negative construct or any
    other, and a positive construct is unaffected by this rule entirely --
    corrupt one so it does not parse and it is still a failure. -/
def scoreNegative (want : String) (ex : Expect) (pe : Option Bebop.Parser.ParseError)
    (r : Result) : Outcome :=
  match ex with
  | .unparsable _ => .noExpect
  | .value _ => .noExpect
  | .compileFail code =>
    match pe with
    | some (.lex m) => .pass s!"refused statically: LEX {m.render} (want exit {code}; LEX names no code)"
    | some (.invalid m l c) =>
        .pass s!"refused statically: INVALID {l}:{c}: {m} (want exit {code}; this refusal names no code)"
    | some (.refused n m l c) =>
        if n == code then .pass s!"refused statically: REJECTED {n} at {l}:{c}: {m}"
        else .codeMismatch s!"REJECTED {n} ({m})" s!"COMPILEFAIL:{code}"
    | some (.unsupported f _ _) => .unsupported f
    | some (.outOfFuel _ _) => .parserFuel
    | none =>
      -- It parsed. A COMPILEFAIL construct must not reach evaluation at all,
      -- so only a `.rejected` (a static check run after the parse) can save it.
      match r with
      | .rejected c _ m =>
          if c == code then .pass s!"refused statically: rejected {c} ({m})"
          else .codeMismatch s!"rejected {c} ({m})" s!"COMPILEFAIL:{code}"
      | .trap tc => .codeMismatch s!"RUNTIME trap {tc.name} (exit {tc.exitCode})"
                                  s!"COMPILEFAIL:{code} (a COMPILE-time refusal)"
      | .stuck m => .stuck m
      | .fuelExhausted _ => .fuel
      | .ok v => .valueMismatch s!"ok {v.toInt}" s!"{want} (must be refused)"
  | .runFail code =>
    -- A RUNFAIL construct is a LEGAL program that fails when RUN. Refusing it
    -- statically is the wrong answer even though it is also a refusal.
    match pe with
    | some (.lex m) => .codeMismatch s!"LEX {m.render}" s!"RUNFAIL:{code} (must parse, then trap)"
    | some (.invalid m l c) => .codeMismatch s!"INVALID {l}:{c}: {m}" s!"RUNFAIL:{code} (must parse, then trap)"
    | some (.refused n m _ _) => .codeMismatch s!"REJECTED {n} ({m})" s!"RUNFAIL:{code} (must parse, then trap)"
    | some (.unsupported f _ _) => .unsupported f
    | some (.outOfFuel _ _) => .parserFuel
    | none =>
      match r with
      | .trap tc =>
          if tc.exitCode == code then .pass s!"trapped at runtime: {tc.name} (exit {tc.exitCode})"
          else .codeMismatch s!"trap {tc.name} (exit {tc.exitCode})" s!"RUNFAIL:{code}"
      | .rejected c _ m => .codeMismatch s!"rejected {c} ({m})" s!"RUNFAIL:{code} (a RUN-time trap)"
      | .stuck m => .stuck m
      | .fuelExhausted _ => .fuel
      | .ok v => .valueMismatch s!"ok {v.toInt}" s!"{want} (must be refused)"

/-- The `results.txt` token for one file: the blueprint's
    `<value|exit:N|unsupported:<why>>` (F3-lean-semantics.md §4.6). -/
def resultToken : Result → Outcome → String
  | _, .unsupported f => "unsupported:" ++ f
  | _, .lexFail _ => "exit:?"
  | _, .parserFuel => "unsupported:parser fuel exhausted"
  | _, .noExpect => "unsupported:no EXPECT header"
  | r, o =>
    match r with
    | .ok v => toString v.toInt
    | .trap tc => s!"exit:{tc.exitCode}"
    | .rejected c _ _ => s!"exit:{c}"
    | .fuelExhausted _ => "unsupported:evaluator fuel exhausted"
    | .stuck _ => "unsupported:" ++ o.detail

/-- Score ONE file. `positive` selects how the EXPECT header is read: a plain
    integer for `parity_constructs/*.bp`, a `COMPILEFAIL:<code>` /
    `RUNFAIL:<code>` tag for `neg/*.bp`. Returns the verdict AND the raw
    `Result`, which `results.txt` prints as its per-construct token. -/
def scoreOne (root : String) (fuel : Nat) (path : String) (positive : Bool) : IO (Outcome × Result) := do
  let raw ← IO.FS.readFile path
  match expectOf raw with
  | none => return (.noExpect, .stuck "no EXPECT header")
  | some want =>
    let ex := parseExpect want positive
    let (src, _, casBad) ← expandUses root 8 #[] raw
    match casBad with
    | some m =>
      -- The digest check ran and FAILED. That is a compile-time refusal with
      -- the compiler's own exit code, before the parser sees anything.
      let r := Result.rejected 88 ⟨0, 0⟩ m
      if positive then return (.trapped (renderResult r), r)
      else return (scoreNegative want ex none r, r)
    | none =>
    match Bebop.Parser.parse src with
    | .error e =>
      let pr : Result := match e with
                         | .refused n m _ _ => .rejected n ⟨0, 0⟩ m
                         | _ => .stuck "not evaluated (refused before evaluation)"
      if positive then
        match e with
        | .lex le => return (.lexFail le.render, pr)
        | .unsupported f _ _ => return (.unsupported f, pr)
        | .invalid m l c => return (.invalid s!"{l}:{c}: {m}", pr)
        | .refused n m l c => return (.invalid s!"{l}:{c}: REJECTED {n}: {m}", pr)
        | .outOfFuel _ _ => return (.parserFuel, pr)
      else
        return (scoreNegative want ex (some e) (.stuck "not evaluated"), pr)
    | .ok prog =>
      -- STATIC REJECTION runs BEFORE evaluation, as the compiler does: a
      -- program `bebop.bin` refuses never runs, so a refused program must not
      -- reach `evalProgram` here either. `Result.rejected` carries the exit
      -- code, which the negative rule then COMPARES.
      let r := match Bebop.Reject.check prog with
               | some d => Result.rejected d.code ⟨0, 0⟩ d.msg
               | none => evalProgram fuel prog
      if positive then
        match ex with
        | .value w =>
          match r with
          | .ok v => if v.toInt == w then return (.pass "", r)
                     else return (.valueMismatch (toString v.toInt) (toString w), r)
          | .stuck m => return (.stuck m, r)
          | .fuelExhausted _ => return (.fuel, r)
          | other => return (.trapped (renderResult other), r)
        | _ => return (.noExpect, r)
      else
        return (scoreNegative want ex none r, r)

structure Row where
  name : String
  outcome : Outcome
  token : String

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
      85  after the tree fixed `c46_andor`'s stale EXPECT header -- that one is
          the corpus catching up with this semantics, not a change here
      87  after the SPARSE ARENA let the default fuel rise to 2000000, which
          brings in `c33_loopalloc` and `c67_deeprec`
    No positive construct is in the INVALID bucket: all six remaining INVALIDs
    are `neg/*.bp`, i.e. programs that SHOULD be refused. -/
def passFloor : Nat := 120

-- ============================================================
-- THE RESULTS FILE -- `formal/results.txt`, the F4 gate's deliverable
--
-- ROADMAP F4's gate is `lean_conformance: c/121` AND
-- `lean_results_hash == sha256(formal/*.lean)`. It has been specified and
-- unenforced: `grep -c lean tools/battery.sh` is 0.
--
-- The SHAPE is docs/blueprints/F3-lean-semantics.md §4.6 verbatim -- "one line
-- per program `name <value|exit:N|unsupported:<why>>`, a header with `sha256`
-- over `formal/**/*.lean` in sorted order and the toolchain's `lean --version`
-- string" -- and the file is written BY THIS PROGRAM, on every run, so it
-- cannot be hand-edited into agreement with a tree it was not produced from.
--
-- THE COMBINED HASH IS REPRODUCIBLE WITH STANDARD TOOLS, which is the only
-- thing that makes it a gate rather than a number. From the repo root:
--
--   find formal -name '*.lean' | LC_ALL=C sort | xargs sha256sum | sha256sum
--
-- must print `lean_sources_sha256`. That is exactly what is computed below:
-- each file's own digest, formatted as `sha256sum` formats it
-- (64 hex digits, two spaces, the path, a newline), concatenated in sorted
-- path order, and hashed again.
-- ============================================================

/-- Every `.lean` file under `<root>/formal`, as repo-root-relative paths,
    sorted. -/
partial def leanSources (root : String) (rel : String) : IO (Array String) := do
  let dir := root ++ "/" ++ rel
  if !(← System.FilePath.pathExists dir) then return #[]
  let entries ← System.FilePath.readDir dir
  let mut out : Array String := #[]
  for e in entries do
    let name := e.fileName
    let path := rel ++ "/" ++ name
    if (← (System.FilePath.mk (root ++ "/" ++ path)).isDir) then
      out := out ++ (← leanSources root path)
    else if name.endsWith ".lean" then
      out := out.push path
  return out

/-- The toolchain string. `lean --version` when `lean` is reachable (the
    blueprint asks for exactly that string); otherwise the pin in
    `formal/lean-toolchain`, which is what `lake` itself resolves. -/
def toolchainString (root : String) : IO String := do
  let viaLean ← try
      let o ← IO.Process.output { cmd := "lean", args := #["--version"] }
      pure (if o.exitCode == 0 then some o.stdout else none)
    catch _ => pure none
  match viaLean with
  | some v => return v.replace "\n" " " |>.trimAscii.toString
  | none =>
    let pin := root ++ "/formal/lean-toolchain"
    if ← System.FilePath.pathExists pin then
      let t ← IO.FS.readFile pin
      return "toolchain-pin " ++ (t.replace "\n" " ").trimAscii.toString
    else return "unknown"

def writeResults (root : String) (fuel passes total : Nat)
    (rows : Array (String × Row)) : IO String := do
  let srcs := (← leanSources root "formal").qsort (· < ·)
  let mut perFile := ""
  for f in srcs do
    let bytes ← IO.FS.readBinFile (root ++ "/" ++ f)
    perFile := perFile ++ Bebop.Sha256.hex bytes.toList.toArray ++ "  " ++ f ++ "\n"
  -- LOUD when there is nothing to hash. sha256 of the empty string is a
  -- perfectly good-looking 64-hex digest, and a checker comparing two runs
  -- that both found no sources would see them AGREE. Measured: running
  -- parityrun against a root with no `formal/*.lean` printed
  -- e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855, the
  -- empty hash, with no complaint.
  let combined := if srcs.isEmpty then "NONE-NO-LEAN-SOURCES-FOUND"
                  else Bebop.Sha256.hex perFile.toUTF8.toList.toArray
  if srcs.isEmpty then
    IO.eprintln s!"parityrun: WARNING -- no .lean sources under {root}/formal; the results hash is not a hash of anything"
  let tc ← toolchainString root
  let mut body := ""
  body := body ++ "# formal/results.txt -- EMITTED by `lake exe parityrun <repo-root>`.\n"
  body := body ++ "# Do not hand-edit: the next run overwrites it, and the hash below is\n"
  body := body ++ "# over the .lean SOURCES, so an edited file cannot agree with a tree it\n"
  body := body ++ "# was not produced from. Shape: docs/blueprints/F3-lean-semantics.md 4.6.\n"
  body := body ++ "#\n"
  body := body ++ "# Recompute lean_sources_sha256 from the repo root with:\n"
  body := body ++ "#   find formal -name '*.lean' | LC_ALL=C sort | xargs sha256sum | sha256sum\n"
  body := body ++ "#\n"
  body := body ++ "# Per-construct token is <value|exit:N|unsupported:<why>>; the verdict\n"
  body := body ++ "# column says whether that token matched the construct's `// EXPECT`.\n"
  body := body ++ s!"toolchain {tc}\n"
  body := body ++ s!"eval_fuel {fuel}\n"
  body := body ++ s!"call_depth_limit {Bebop.Semantics.callDepthLimit}\n"
  body := body ++ s!"lean_conformance {passes}/{total}\n"
  body := body ++ s!"lean_sources_sha256 {combined}\n"
  body := body ++ s!"lean_source_count {srcs.size}\n"
  body := body ++ perFile
  for (kind, r) in rows do
    let v := match r.outcome with | .pass _ => "PASS" | _ => "FAIL"
    body := body ++ s!"{kind}/{r.name} {v} {r.token}\n"
  IO.FS.writeFile (root ++ "/formal/results.txt") body
  return combined

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
    let (o, r) ← scoreOne root fuel p true
    IO.eprintln o.bucket
    rows := rows.push ("positive", { name := baseName p, outcome := o, token := resultToken r o })
  for p in negFiles do
    IO.eprint s!"[neg] {baseName p} ... "
    (← IO.getStderr).flush
    let (o, r) ← scoreOne root fuel p false
    IO.eprintln o.bucket
    rows := rows.push ("negative", { name := baseName p, outcome := o, token := resultToken r o })

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
    match r.outcome with | .pass _ => n + 1 | _ => n) 0

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
    | .pass _ => pure ()
    | o =>
      if shown < 120 then
        shown := shown + 1
        IO.println s!"  [{kind}] {r.name}: {o.bucket} {o.detail}"
  -- STATIC REFUSALS THAT SCORED AS PASSES, each with the diagnosis that
  -- produced it. A `neg/` construct passes here because the harness REFUSED
  -- it, and a refusal for the wrong reason scores the same as one for the
  -- right reason. Printing every diagnosis is what keeps that visible: the
  -- score cannot show it, so the log must.
  let refusals := rows.filter (fun (_, r) => match r.outcome with
                                             | .pass d => d != ""
                                             | _ => false)
  if refusals.size > 0 then
    IO.println s!"refusal diagnoses ({refusals.size} passes that are refusals, not values):"
    for (kind, r) in refusals do
      IO.println s!"  [{kind}] {r.name}: {r.outcome.detail}"
    IO.println ""
  let digest ← writeResults root fuel passes total rows
  IO.println s!"results file: formal/results.txt written"
  -- The two GATE LINES, in the shape tools/battery.sh's `line` helper asserts
  -- on. ROADMAP F4's gate is `lean_conformance: c/121` AND
  -- `lean_results_hash == sha256(formal/*.lean)`; the hash half is only a gate
  -- if it can be recomputed WITHOUT this program, which is why the digest is
  -- the double-sha256sum of the sorted source list and the recipe is printed
  -- into results.txt.
  IO.println s!"lean_conformance: {passes}/{total}"
  IO.println s!"lean_results_hash: {digest}"
  IO.println ""
  if passes < passFloor then
    IO.eprintln s!"parityrun: REGRESSION -- {passes} passes is below the floor {passFloor}"
    return 1
  IO.println s!"parityrun: {passes} passes (floor {passFloor})"
  return 0
