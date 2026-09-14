/-
  Bebop.Semantics -- Definitional interpreter with fuel for termination.

  This file implements the core operational semantics of Bebop as a
  terminating function (fuel-bounded evaluation). Every rule from
  LANGUAGE.md is one match arm.

  The interpreter mirrors bpref.py (tools/bpref.py, 727 lines) but
  is written in Lean 4 to serve as the normative semantics for the
  formal verification infrastructure (F4 gate: lean_conformance 86/86).

  Key design decisions:
  - Fuel-bounded: every recursive call decrements fuel. Every fuel-0 arm sets
    the sticky `State.fuelOut`, and `evalProgram` reports
    `Result.fuelExhausted` when it is set -- whatever value came out after.
    (Until 2026-09-13 the arms answered `.cont`/`none` and the run carried on:
    `while 1 { 0 }; 0` was `ok 0` at fuel 1000, probed; a diverging program
    could pass. Same rule as tools/kcheck.py `whnf`: exhaustion is loud.)
  - A function body's value is its tail expression: execBody (section 5).
  - All arithmetic is Int64 wrapping (Z/2^64).
  - Arena is a flat Array Val; frame heap is the arena itself (A6).
  - Function-scoped bindings; no block scoping, no shadowing.
  - While loops reset the frame heap per iteration (T43/A6).
  - Struct/enum literals allocate on arena; field access reads from arena.
  - 10 builtins + 5 sys_* dispatched via Builtins/Syscalls modules.
  - Import DAG: Basic <- {Builtins, Syscalls} <- Semantics. State.lookup,
    State.zeros, State.arenaRead/Write live in Basic so nothing below
    this module needs the evaluator.
-/
import Bebop.Basic
import Bebop.Builtins
import Bebop.Syscalls

namespace Bebop.Semantics

open Bebop.Builtins
open Bebop.Syscalls

-- ============================================================
-- 0. THE CALL-DEPTH BOUND -- a resource, and NOT the same one as fuel
-- ============================================================

/-- How many user-function activations the modelled stack holds. Crossing it is
    trap 82, `neg/c48_stackovf.bp`'s `RUNFAIL:82`.

    WHY A SECOND RESOURCE AT ALL. Before this, `c48_stackovf` reported FUEL and
    `ParityRun.defaultFuel`'s own comment said it "stays FUEL at any budget and
    should". It should not: the construct asserts that unbounded recursion is a
    LOUD trap 82 (docs/TRAPS.md:13 -- "SIGSEGV or SIGBUS in the program: stack
    overflow (deep recursion...)"; the entry stub's handler turns the fault into
    exit 82). Relabelling FUEL as trap 82 would have been the wrong fix twice
    over: `c33_loopalloc` and `c67_deeprec` were BOTH once FUEL and are both
    legal programs that simply needed more budget, and a genuinely slow program
    must keep reporting FUEL. So exhaustion and overflow stay two different
    verdicts with two different counters, and the control for that is a program
    that needs a lot of fuel and no depth (a long `while`): it must still say
    FUEL. It does.

    THE VALUE IS A MODEL PARAMETER, and here is the range it has to sit in.
    The real bound is per-FUNCTION, because A6 made the frame size depend on
    the function:
      * `bench/vs_rust/construct_parity.sh:6` runs the corpus under
        `ulimit -s 65536`, i.e. a 64 MiB stack.
      * docs/TRAPS.md:13 describes trap 82 as "deep recursion at 16 KiB per
        frame" -- the FLAT frame, which gives 64 MiB / 16 KiB = 4096
        activations.
      * `c67_deeprec.bp`'s own header computes A6's frame for its leaf fn `d`
        as F = 80 + 8*marks + 8 + 8*(S + tsp) = 88, rounded to 96 B, i.e.
        "9.6 MB for 10^5 activations, inside construct_parity.sh's own
        ulimit -s 65536" -- which gives 64 MiB / 96 B = 699050 activations.
    So the true bound is somewhere in [4096, 699050] and depends on which
    function is recursing. This model uses ONE number, which must be above the
    deepest LEGAL recursion in the corpus -- c67_deeprec's 100001 activations,
    a construct that must keep passing -- and inside that range. 131072 = 2^17
    is both. It is not measured, it is CHOSEN, and it is written here rather
    than buried so that the next person can see it is a parameter and not a
    fact. -/
def callDepthLimit : Nat := 131072

-- ============================================================
-- 1-2. Environment and arena operations
--      These live in Bebop.Basic (State.lookup/bind/zeros/arenaRead/
--      arenaWrite/...): Builtins and Syscalls need them, and importing
--      them from here was the import cycle that kept `lake build` from
--      ever succeeding.
-- ============================================================

-- ============================================================
-- 3. Binary operation dispatch
-- ============================================================

/-- Evaluate a binary operation with wrapping semantics.
    LANGUAGE.md:60-68, bpref.py:132-148. -/
def evalBinOp (op : BinOp) (a b : Val) : Val :=
  match op with
  | .add  => a + b
  | .sub  => a - b
  | .mul  => a * b
  | .sdiv => Val.sdiv a b
  | .srem => Val.srem a b
  | .lsl  => a <<< Int64.ofNat (b.toNatClampNeg % 64)
  | .lsr  => (a.toUInt64 >>> UInt64.ofNat (b.toNatClampNeg % 64)).toInt64
  | .asr  => a >>> Int64.ofNat (b.toNatClampNeg % 64)
  | .band => a &&& b
  | .bor  => a ||| b
  | .bxor => a ^^^ b
  -- `||` / `&&` -- LOGICAL, and MEASURED 2026-09-14 on the promoted bebop.bin
  -- (sha256 3af3250...). This REPLACES the bitwise rules that stood here, and
  -- it also retires this tree's older note that `&&` is a constant zero: that
  -- was measured on an earlier compiler and is false for the promoted binary.
  --
  --   (2 && 1)*1000 + (2 || 1)*100 + (0 || 7)*10 + (0 && 7)   ->  1110
  --
  -- Bitwise would give 370 (2&1 = 0, 2|1 = 3, 0|7 = 7). So the VALUE is 0 or 1.
  --
  -- Evaluation, however, is NOT short-circuit -- both operands always run:
  --   fn bump(a: i64) -> i64 { let _ = a[0] = 7; 1 }
  --   let a = zeros(1); let r = 0 && bump(a); a[0] * 10 + r   ->  70
  -- a[0] is 7, so `bump` ran under a false left operand (and `1 || bump(a)`
  -- gives 71). That is T125's non-short-circuit rule, still true; it is the
  -- VALUE that changed. `evalBinOp` is applied AFTER `evalExpr` has already
  -- run both sides in `.binop`, so this function is exactly the right place
  -- for the value rule and the eager evaluation is inherited for free.
  --
  -- Precedence (`||` loosest, then `&&`, both ABOVE comparison) lives in
  -- Bebop/Parser.lean §0, with the probe for each row.
  --
  -- STILL A DOC DISAGREEMENT, NOT RESOLVED HERE: LANGUAGE.md:57-67 does not
  -- list `&&` or `||` at all. The compiler has them; the reference does not.
  -- Reported rather than picked.
  | .lor  => if a != 0 || b != 0 then 1 else 0
  | .land => if a != 0 && b != 0 then 1 else 0
  | .eq   => if a == b then 1 else 0
  | .neq  => if a != b then 1 else 0
  | .slt  => if a < b then 1 else 0
  | .sgt  => if a > b then 1 else 0
  | .sle  => if a ≤ b then 1 else 0
  | .sge  => if a ≥ b then 1 else 0

-- ============================================================
-- 3b. Helpers: enum tag/payload of an encoded value (used by match)
-- ============================================================

/-- Extract the tag from an encoded enum value (tag << 32). -/
def extractEnumTag (v : Val) : Nat :=
  (v.toUInt64 >>> 32).toNat

/-- Extract the payload offset from an encoded enum value. -/
def extractEnumPayload (v : Val) : Nat :=
  (v.toUInt64 &&& 0xFFFFFFFF).toNat

-- ============================================================
-- 4. Expression evaluation (fuel-bounded)
-- ============================================================

mutual

/-- Evaluate an array literal, returning values and updated state. -/
partial def evalArrLitAux (es : Array Expr) (fuel : Fuel) (s : State)
    : Array Val × State :=
  match fuel with
  | 0 => (#[], { s with fuelOut := true })
  | fuel + 1 =>
    let rec go (es : Array Expr) (idx : Nat) (acc : Array Val) (s : State)
        : Array Val × State :=
      if h : idx < es.size then
        let (s', v) := evalExpr fuel es[idx] s
        match v with
        | none => (acc, s')
        | some v' => go es (idx + 1) (acc.push v') s'
      else
        (acc, s)
    go es 0 #[] s

/-- Evaluate an expression, returning a value and updated state.
    LANGUAGE.md:57-82. The evaluator is fuel-bounded for termination. -/
partial def evalExpr (fuel : Fuel) (e : Expr) (s : State) : State × Option Val :=
  match fuel with
  | 0 => ({ s with fuelOut := true }, none)
  | fuel + 1 =>
    -- THE UNWIND. `return e` / `break` in expression position set
    -- `State.pending`; while it is set NOTHING further evaluates, which is
    -- what an exception would do in bpref.py (`ReturnSignal`/`BreakSignal`)
    -- and what this evaluator has to do by hand because it is a pure
    -- state-threading function. Without this line the right operand of a
    -- `binop`, the remaining arguments of a call and the rest of an array
    -- literal would all still run after a `return` fired.
    -- ... and the same for a RUNTIME TRAP. Nothing catches `trapped`, so once
    -- it is set the whole run is over and no further expression may evaluate:
    -- `let a = zeros(40000000); a[0]` must report trap 80, not read a[0].
    if s.pending.isSome || s.trapped.isSome then (s, none) else
    match e with
    -- Literals
    | .lit v => (s, some v)

    -- `return e` in expression position: evaluate `e`, then RAISE. The
    -- expression itself has NO value (`none`), so nothing downstream can read
    -- a `return` as a number; the value travels in the signal.
    | .retExpr e' =>
      let (s1, v) := evalExpr fuel e' s
      match v with
      | none => (s1, none)      -- the operand itself got stuck or unwound
      | some v' => ({ s1 with pending := some (.ret v') }, none)

    -- `break` in expression position: raise, no operand, no value.
    | .brkExpr => ({ s with pending := some .brk }, none)

    -- String literal: NOT modelled, and loudly so. `formal/` has no byte
    -- arena, `builtinStrLen`/`builtinChar` are known-wrong (README §defects),
    -- and returning any number here would make a wrong evaluator look right on
    -- the 22 corpus programs that contain a string. `none` propagates to
    -- `Result.stuck`, which `checkExpected` cannot mistake for a value.
    -- STRING LITERAL. There IS a byte arena now (`State.bytes`), so this
    -- returns a real `(offset << 32) | length` handle instead of the `none`
    -- that stood here while "no byte arena is modelled" was true. It is the
    -- `State.internStr` rule, which is bpref.py's `'str'` node verbatim,
    -- terminator included.
    | .strLit str =>
      let (s1, h) := s.internStr str
      (s1, some h)

    -- Variable reference
    | .var n =>
      match s.lookup n with
      | some v => (s, some v)
      | none => (s.why s!"unbound symbol `{n}`", none)

    -- Parenthesized expression
    | .paren e' => evalExpr fuel e' s

    -- Unary operators
    | .unop op e' =>
      let (s1, v) := evalExpr fuel e' s
      match v with
      | none => (s1, none)
      | some v' =>
        match op with
        | .neg  => (s1, some (-v'))
        | .lnot => (s1, some (if v' == 0 then 1 else 0))

    -- Binary operators
    | .binop op l r =>
      let (s1, vl) := evalExpr fuel l s
      match vl with
      | none => (s1, none)
      | some vl =>
        let (s2, vr) := evalExpr fuel r s1
        match vr with
        | none => (s2, none)
        | some vr => (s2, some (evalBinOp op vl vr))

    -- Array literal: [e0, e1, ...]
    | .arrLit es =>
      let (vals, s1) := evalArrLitAux es fuel s
      -- Allocate on arena; frame heap is the arena (A6).
      -- `base` is read BEFORE `zeros`, on purpose. Reading it afterwards (from
      -- `s1`, while `s2` also existed) kept two states live and was one of the
      -- two causes of the quadratic arena -- see the `Arena` docstring. It is a
      -- `Nat` now, so nothing of `s1` survives the call.
      let base := s1.arena.cursor
      let (s2, err) := s1.zeros vals.size
      match err with
      | some e => (s2, none)  -- arena exhausted
      | none =>
        -- Write values into arena at base
        let s3 := Id.run do
          let mut st := s2
          for i in [:vals.size] do
            let idx := base + i
            let v := vals[i]!
            let st' := st.arenaWrite idx v
            match st' with
            | some s'' => st := s''
            | none => pure ()
          return st
        (s3, some (Int64.ofNat base))

    -- Array get: a[i]
    | .arrGet arr idx =>
      let (s1, va) := evalExpr fuel arr s
      match va with
      | none => (s1, none)
      | some va =>
        let (s2, vi) := evalExpr fuel idx s1
        match vi with
        | none => (s2, none)
        | some vi =>
          let base := va.toNatClampNeg
          let index := vi.toNatClampNeg
          -- Check if base is a frame-allocated array; if so use its length
          let result := s2.lookupFrameArrayAny base
          match result with
          | some (fbase, flen) =>
            if h : base == fbase && index < flen then
              let off := fbase + index
              match s2.arenaRead off with
              | some v => (s2, some v)
              | none => (s2, some 0)
            else
              let off := base + index
              match s2.arenaRead off with
              | some v => (s2, some v)
              | none => (s2, some 0)
          | none =>
            let off := base + index
            match s2.arenaRead off with
            | some v => (s2, some v)
            | none => (s2, some 0)

    -- Array set: a[i] = v
    | .arrSet arr idx val =>
      let (s1, va) := evalExpr fuel arr s
      match va with
      | none => (s1, none)
      | some va =>
        let (s2, vi) := evalExpr fuel idx s1
        match vi with
        | none => (s2, none)
        | some vi =>
          let (s3, vv) := evalExpr fuel val s2
          match vv with
          | none => (s3, none)
          | some vv =>
            let base := va.toNatClampNeg
            let index := vi.toNatClampNeg
            let off := base + index
            match s3.arenaWrite off vv with
            | some s4 => (s4, some 0)
            | none => (s3, some 0)

    -- Function call
    | .call fnName args =>
      -- Evaluate arguments left to right
      let rec evalArgs (args : Array Expr) (idx : Nat) (acc : Array Val) (s : State)
          : Array Val × State :=
        if h : idx < args.size then
          let (s', v) := evalExpr fuel args[idx] s
          match v with
          | none => (acc, s')
          | some v' => evalArgs args (idx + 1) (acc.push v') s'
        else
          (acc, s)
      let (argVals, s1) := evalArgs args 0 #[] s
      -- Check for builtin first
      match dispatchBuiltin fnName argVals s1 with
      | some (s', v) => (s', some v)
      | none =>
        -- Check for syscall
        match dispatchSyscall fnName argVals s1 with
        | some (s', v) => (s', some v)
        | none =>
          -- User function call
          let fnOpt := s1.fns.find? (fun f => f.name == fnName)
          match fnOpt with
          | none =>
            -- UNRESOLVED CALL. The two reasons are kept apart, because one is
            -- a fact about the program and the other is a gap in this model:
            --   * the name is not a declared `fn` and not in the language's
            --     BUILTIN surface at all -- `nowhere(3)` in neg/c52_undef.
            --     That is trap 87 (T130: "a call to a function that never
            --     resolves is a runtime trap (exit 87), not a silent 0").
            --   * the name IS a builtin (`Bebop.builtinSurface`, measured from
            --     bebop.bp) that `dispatchBuiltin`/`dispatchSyscall` does not
            --     implement. Calling that a trap would report OUR gap as the
            --     program's defect, so it stays `stuck` -- loud, and in the
            --     bucket that names formal/ as the incomplete side.
            if builtinSurface.contains fnName then
              (s1.noteGap s!"unmodelled builtin `{fnName}` (it IS in the language's builtin surface; `formal/` does not implement it -- see Bebop/Syscalls.lean's default arm for why)", none)
            else ({ s1 with trapped := some TrapCode.unresolvedCall }, none)
          | some fn =>
            if argVals.size != fn.params.size then
              (s1.why s!"call to `{fnName}` with {argVals.size} args, but it declares {fn.params.size}", none)
            else
              -- Create new activation: bind params
              let newEnv := Id.run do
                let mut env : Array (Name × Val) := #[]
                for i in [:fn.params.size] do
                  env := env.push (fn.params[i]!, argVals[i]!)
                return env
              if s1.depth ≥ callDepthLimit then
                -- STACK OVERFLOW, trap 82. Loud, and distinct from fuel.
                ({ s1 with trapped := some TrapCode.stackOverflow }, none)
              else
              let s2 := { s1 with env := newEnv, depth := s1.depth + 1 }
              -- Execute the body: its value is the tail expression (or a `ret`).
              -- A body that yields no value propagates `none`; it was `some 0`
              -- until 2026-09-13, which hid every failed call as a 0.
              let (s3, v) := execBody fuel fn.body s2
              -- Leave the activation: the CALLER's bindings come back (bpref.py
              -- `call` builds a fresh env dict and the caller's is untouched);
              -- arena and the sticky fuel flag are the callee's. Until
              -- 2026-09-13 the callee's env was returned, so
              -- `let x = 5; let y = f(1); x + y` was `stuck` (`ok 0` before
              -- `stuck` existed) where bpref gives 7.
              ({ s3 with env := s1.env, depth := s1.depth }, v)

    -- If-then-else
    | .ite c t f =>
      let (s1, vc) := evalExpr fuel c s
      match vc with
      | none => (s1, none)
      | some vc =>
        if vc != 0 then evalExpr fuel t s1
        else evalExpr fuel f s1

    -- Let-in: let x = e in body
    | .letIn n e' body =>
      let (s1, ve) := evalExpr fuel e' s
      match ve with
      | none => (s1, none)
      | some ve => evalExpr fuel body (s1.bind n ve)

    -- `let _ = NAME = e in body`. EVALUATES exactly like `letIn` -- `let` is
    -- fn-scoped with no shadowing (LANGUAGE.md:41-43), so binding and
    -- rebinding are the same operation on the same register. The two are
    -- separate AST nodes because the STATIC rule differs; see Bebop.Reject.
    | .assignIn n e' body =>
      let (s1, ve) := evalExpr fuel e' s
      match ve with
      | none => (s1, none)
      | some ve => evalExpr fuel body (s1.bind n ve)

    -- Struct literal: struct Name { f1: e1, f2: e2, ... }
    | .structLit name fields =>
      -- Evaluate all field values
      let rec evalFields (fields : Array (Name × Expr)) (idx : Nat)
          (acc : Array (Name × Val)) (s : State)
          : Array (Name × Val) × State :=
        if h : idx < fields.size then
          let (fname, fexpr) := fields[idx]!
          let (s', fval) := evalExpr fuel fexpr s
          match fval with
          | none => (acc, s')
          | some fval => evalFields fields (idx + 1) (acc.push (fname, fval)) s'
        else
          (acc, s)
      let (fieldVals, s1) := evalFields fields 0 #[] s
      -- Look up struct definition to get field list
      match s1.structs.find? (fun p => p.1 == name) with
      | none => (s1, none)  -- undefined struct
      | some (_, structFields) =>
        -- Allocate on arena: store field values in order
        let base := s1.arena.cursor
        let (s2, err) := s1.zeros structFields.size
        match err with
        | some _ => (s2, none)
        | none =>
          -- Write field values
          let s3 := Id.run do
            let mut st := s2
            for i in [:structFields.size] do
              let fname := structFields[i]!
              -- Find the value for this field name
              match fieldVals.find? (fun p => p.1 == fname) with
              | some (_, v) =>
                let st' := st.arenaWrite (base + i) v
                match st' with
                | some s'' => st := s''
                | none => pure ()
              | none => pure ()  -- missing field: leave as 0
            return st
          -- Register as frame array so field access can find it
          let s4 := s3.registerFrameArray name base structFields.size
          (s4, some (Int64.ofNat base))

    -- Enum literal: enumName(ctor, arg?) or enumName(arg) for unary ctors
    | .enumLit name arg =>
      -- Look up enum definition to find ctor list
      match s.enumTags.find? (fun p => p.1 == name) with
      | some (_, tag) =>
        -- Evaluate arg if present
        match arg with
        | some argExpr =>
          let (s2, argVal) := evalExpr fuel argExpr s
          match argVal with
          | none => (s2, none)
          | some argVal =>
            -- Allocate payload cell on arena
            let (s3, err) := s2.zeros 1
            match err with
            | some _ => (s3, none)
            | none =>
              let base := s3.arena.cursor - 1
              let s4 := s3.arenaWrite base argVal |>.getD s3
              -- Return encoded enum value: (tag << 32) | payload_offset
              (s4, some ((Int64.ofNat tag <<< 32) ||| Int64.ofNat base))
        | none =>
          -- Nullary ctor: just return the tag
          (s, some (Int64.ofNat tag <<< 32))
      | none =>
        -- Unknown enum; try to find ctor by name
        match s.enumArities.find? (fun p => p.1 == name) with
        | some (_, arity) =>
          -- Enumeration ctor name lookup: find tag by position
          -- Simplified: tag = index in enumArities
          (s, none)
        | none => (s, none)

    -- Field access: s.f
    | .fieldAcc structExpr field =>
      let (s1, sval) := evalExpr fuel structExpr s
      match sval with
      | none => (s1, none)
      | some sval =>
        let base := sval.toNatClampNeg
        -- Look up struct definition to find field index
        match s1.structs.find? (fun p => p.1 == field) with
        -- Try to find struct that has this field
        | none =>
          -- Search all structs for the field
          let rec findField (structs : Array (Name × Array Name)) (idx : Nat)
              : Option Nat :=
            if h : idx < structs.size then
              let (_, fields) := structs[idx]!
              match fields.findIdx? (fun f => f == field) with
              | some fi => some fi
              | none => findField structs (idx + 1)
            else none
          match findField s.structs 0 with
          | some fi =>
            let off := base + fi
            match s1.arenaRead off with
            | some v => (s1, some v)
            | none => (s1, some 0)
          | none => (s1, none)  -- field not found
        | some _ =>
          -- Field name matches a struct name; treat as struct access
          -- This shouldn't happen in well-formed programs
          (s1, none)

    -- Builtin call (explicit form, e.g. char(s, i))
    | .builtin name args =>
      -- Evaluate arguments left to right (same helper the array literal uses),
      -- then dispatch on the VALUES; the scaffold passed the unevaluated Exprs.
      let (argVals, s1) := evalArrLitAux args fuel s
      match dispatchBuiltin name argVals s1 with
      | some (s', v) => (s', some v)
      | none =>
        -- Try syscall
        match dispatchSyscall name argVals s1 with
        | some (s', v) => (s', some v)
        | none => (s1, none)

    -- Match expression: match scrut { ctor1 => body1, ctor2 => body2, ... }
    -- 1. Evaluate the scrutinee to an enum-encoded value (tag << 32 | offset).
    -- 2. Find the arm whose ctor's tag equals the scrutinee's tag.
    -- 3. If the arm has a binder, bind the payload VALUE -- i.e. READ the
    --    arena cell that the low 32 bits point at (see `.enumLit` above:
    --    a payload ctor does `zeros 1`, writes the argument to that cell and
    --    returns `(tag <<< 32) ||| base`). Binding `base` itself is wrong.
    -- 4. Evaluate that arm's body.
    --
    -- TWO defects have been fixed here, both measured before the fix:
    -- (a) the original scaffold read the scrutinee from an env key
    --     "_scrutinee" that nothing bound, so every match evaluated to
    --     `none`. The scrutinee is now an AST field, `Expr.matchExpr
    --     (scrut : Expr) (arms ...)` in Basic.lean.
    -- (b) MEASURED 2026-09-14: the binder was then bound to
    --     `extractEnumPayload scrutVal`, which is the arena OFFSET, not the
    --     value stored there. bench/parity_constructs/c12_match.bp
    --     (`match some(5) { none => 0, some(x) => x + 1 }`, EXPECT 6)
    --     evaluated to `ok 1`, because `some(5)` allocated arena cell 0 and
    --     `x` was bound to 0 rather than to 5. Only a payload-binding arm was
    --     affected, so c11_enum (nullary) was `ok 5` and looked fine. Fixed
    --     by `arenaRead`; a payload cell that is out of range now yields
    --     `none` (stuck) rather than a silently wrong number.
    --
    -- Note on scope: LANGUAGE.md:78-79 says `match` is COMPILE-TIME and the
    -- scrutinee must be a literal constructor. This rule is deliberately more
    -- general -- it evaluates an arbitrary scrutinee expression, which is what
    -- c96_enumpay.bp (`let a = some(7); match a { ... }`) needs. That is a
    -- SUPERSET of the documented language, so a program this rule accepts may
    -- be rejected by bebop.bin; the F1 static-rejection side, not this rule,
    -- is where that restriction belongs.
    | .matchExpr scrut arms =>
      let (s0, scrutOpt) := evalExpr fuel scrut s
      match scrutOpt with
      | none => (s0, none)
      | some scrutVal =>
        let tag := extractEnumTag scrutVal
        -- Find matching arm by tag index
        let rec findArm (idx : Nat) : Option MatchArm :=
          if h : idx < arms.size then
            let arm := arms[idx]
            -- The arm's ctor name maps to a tag via enumTags
            match s0.enumTags.find? (fun p => p.1 == arm.ctor) with
            | some (_, armTag) =>
              if armTag == tag then some arm
              else findArm (idx + 1)
            | none => findArm (idx + 1)
          else none
        match findArm 0 with
        | some arm =>
          match arm.binder with
          | some bName =>
            -- The low 32 bits are an arena OFFSET; the payload is the cell.
            let payloadOff := extractEnumPayload scrutVal
            match s0.arenaRead payloadOff with
            | none => (s0, none)   -- payload cell out of range: stuck, not 0
            | some payloadVal =>
              evalExpr fuel arm.body (s0.bind bName payloadVal)
          | none => evalExpr fuel arm.body s0
        | none => (s0, none)  -- no matching arm

-- ============================================================
-- 5. Statement execution (fuel-bounded)
-- ============================================================

/-- Execute a single statement, and CATCH any `return`/`break` raised from
    expression position inside it.

    `execStmtRaw` runs the statement; `State.pending`, if set, is then turned
    into this statement's control-flow signal and CLEARED. That is the whole
    unwind: `evalExpr` refuses to evaluate anything while `pending` is set, so
    the signal travels up the expression tree untouched, and the first
    enclosing statement is where it becomes an ordinary `Signal` that
    `execStmts` and the `while` arm already know how to handle. Clearing it
    here is what keeps it from leaking past its catcher. -/
partial def execStmt (fuel : Fuel) (stmt : Stmt) (s : State) : StmtResult :=
  let r := execStmtRaw fuel stmt s
  match r.state.pending with
  | some sg => { signal := sg, state := { r.state with pending := none } }
  | none => r

partial def execStmtRaw (fuel : Fuel) (stmt : Stmt) (s : State) : StmtResult :=
  match fuel with
  | 0 => { signal := .cont, state := { s with fuelOut := true } }
  | fuel + 1 =>
    match stmt with
    | .let_ n e =>
      let (s', v) := evalExpr fuel e s
      match v with
      | none => { signal := .cont, state := s' }
      | some v' => { signal := .cont, state := s'.bind n v' }
    -- `let _ = NAME = e ;` -- same rebind as `let_`, different static rule.
    | .assign n e =>
      let (s', v) := evalExpr fuel e s
      match v with
      | none => { signal := .cont, state := s' }
      | some v' => { signal := .cont, state := s'.bind n v' }
    | .drop e =>
      let (s', _) := evalExpr fuel e s
      { signal := .cont, state := s' }
    | .arrStore arr idx val =>
      let (s1, va) := evalExpr fuel arr s
      match va with
      | none => { signal := .cont, state := s1 }
      | some va =>
        let (s2, vi) := evalExpr fuel idx s1
        match vi with
        | none => { signal := .cont, state := s2 }
        | some vi =>
          let (s3, vv) := evalExpr fuel val s2
          match vv with
          | none => { signal := .cont, state := s3 }
          | some vv =>
            let base := va.toNatClampNeg
            let index := vi.toNatClampNeg
            let off := base + index
            match s3.arenaWrite off vv with
            | some s4 => { signal := .cont, state := s4 }
            | none => { signal := .cont, state := s3 }
    | .compound n op e =>
      let cur := s.lookup n |>.getD 0
      let (s', v) := evalExpr fuel e s
      match v with
      | none => { signal := .cont, state := s' }
      | some v' =>
        { signal := .cont, state := s'.bind n (evalBinOp op cur v') }
    | .while_ cond body =>
      let rec loop (fuel : Fuel) (s : State) : StmtResult :=
        match fuel with
        | 0 => { signal := .cont, state := { s with fuelOut := true } }
        | fuel + 1 =>
          let (s1, vc) := evalExpr fuel cond s
          match vc with
          | none => { signal := .cont, state := s1 }
          | some vc =>
            if vc == 0 then { signal := .cont, state := s1 }
            else
              -- Reset frame at loop entry (T43)
              let s2 := { s1 with frameArrays := #[], arena := s1.arena }
              let result := execStmts fuel body s2
              match result.signal with
              | .brk => { signal := .cont, state := result.state }
              | .ret v => { signal := .ret v, state := result.state }
              | .cont =>
                -- Reset frame at back-edge (T43)
                let s3 := { result.state with frameArrays := #[], arena := result.state.arena }
                loop fuel s3
      loop fuel s
    | .ret e =>
      let (s', v) := evalExpr fuel e s
      match v with
      | none => { signal := .ret 0, state := s' }
      | some v' => { signal := .ret v', state := s' }
    | .brk => { signal := .brk, state := s }
    | .exprStmt e =>
      let (s', _) := evalExpr fuel e s
      { signal := .cont, state := s' }

/-- Execute a sequence of statements, propagating control flow signals. -/
partial def execStmts (fuel : Fuel) (stmts : Array Stmt) (s : State) : StmtResult :=
  match fuel with
  | 0 => { signal := .cont, state := { s with fuelOut := true } }
  | fuel + 1 =>
    let rec go (fuel : Fuel) (idx : Nat) (s : State) : StmtResult :=
      if h : idx < stmts.size then
        let result := execStmt fuel stmts[idx] s
        match result.signal with
        | .cont => go fuel (idx + 1) result.state
        | .brk | .ret _ => result
      else
        { signal := .cont, state := s }
    go fuel 0 s

/-- Execute a FUNCTION BODY: statements followed by ONE tail expression
    (LANGUAGE.md:27-28). The body's value is the tail expression's value; a
    `ret` anywhere overrides it. This is the ONLY place a statement sequence
    yields a value: `execStmts` never does, because a `while` body's tail is
    discarded (LANGUAGE.md:47) and every non-final `exprStmt` really is
    dropped. Mirrors bpref.py `run_body` (tools/bpref.py:528-560): `val` is
    the last executed statement's value when that statement is an
    expression; `return` overrides; the parser refuses a body whose last
    item is not an expression (exit 97), so that case yields `none` here
    rather than bpref's unreachable 0.
    Returns `none` when the tail (or the `ret`) fails to evaluate -- unbound
    symbol, unresolved call, arena fault, fuel exhausted -- so no caller can
    read a failed body as 0. (Until 2026-09-13 there was no tail rule at all:
    `execStmt (.exprStmt e)` dropped the value and both callers answered 0.) -/
partial def execBody (fuel : Fuel) (body : Array Stmt) (s : State) : State × Option Val :=
  match fuel with
  | 0 => ({ s with fuelOut := true }, none)
  | fuel + 1 =>
    -- A body starts with no pending signal and must LEAVE with none: a
    -- callee's `return` is caught by the callee's own body rule, and letting
    -- one escape would freeze the caller's evaluator (nothing evaluates while
    -- `pending` is set). The clear is on the way IN; every exit path below
    -- either never set it or cleared it.
    let s := { s with pending := none }
    if body.size == 0 then (s.why "empty body: no tail expression", none)
    else
      let stmts := body.pop        -- every statement but the last
      let last := body.back!       -- the tail
      let r := execStmts fuel stmts s
      match r.signal with
      | .ret v => (r.state, some v)
      | .brk   => (r.state, some 0)  -- `break` outside a loop: LANGUAGE.md calls
                                     -- the shape undefined; pre-2026-09-13 answer kept
      | .cont  =>
        match last with
        | .exprStmt e =>
          -- The TAIL expression can itself raise: `fn f() { if c then return 1
          -- else 2 }` is legal. `evalExpr` yields `none` in that case and puts
          -- the value in `pending`, so the tail rule has to catch it here as
          -- well -- otherwise a body whose tail returns would read as `stuck`.
          let (s', v) := evalExpr fuel e r.state
          (match s'.pending with
           | some (.ret rv) => ({ s' with pending := none }, some rv)
           | some .brk      => ({ s' with pending := none }, some 0)
           | some .cont     => ({ s' with pending := none }, v)
           | none           => (s', v))
        | stmt =>
          -- Last item is not an expression (the parser rejects this, exit 97).
          -- Run it for its `ret`, otherwise there is no value.
          let r2 := execStmt fuel stmt r.state
          match r2.signal with
          | .ret v => (r2.state, some v)
          | _      => (r2.state.why "body's last item is not an expression and did not `return`", none)

end

-- ============================================================
-- 6. Program evaluation
-- ============================================================

/-- Evaluate a Bebop program: build initial state, call main.
    This is the top-level entry point for the definitional semantics. -/
def evalProgram (fuel : Fuel) (prog : Program) (clockMs : Val := 0) : Result :=
  let s : State := {
    env := #[]
    arena := {}
    frameArrays := #[]
    enumTags := Id.run do
      let mut tags : Array (Name × Nat) := #[]
      for enum in prog.enums do
        for i in [:enum.ctors.size] do
          tags := tags.push (enum.ctors[i]!.name, i)
      return tags
    enumArities := Id.run do
      let mut arities : Array (Name × Nat) := #[]
      for enum in prog.enums do
        for ctor in enum.ctors do
          arities := arities.push (ctor.name, if ctor.hasPayload then 1 else 0)
      return arities
    fns := prog.fns
    structs := Id.run do
      let mut result : Array (Name × Array Name) := #[]
      for st in prog.structs do
        result := result.push (st.name, st.fields)
      return result
    clockMs := clockMs
  }
  let mainOpt := prog.fns.find? (fun f => f.name == "main")
  match mainOpt with
  | none => .rejected 97 ⟨0, 0⟩ "no main function"
  | some main =>
    if main.params.size != 0 && main.params.size != 2 then
      .rejected 100 ⟨0, 0⟩ "main has wrong param count"
    else
      -- main's value is its tail expression (or a `ret`), via execBody.
      -- A body that yields no value is reported as `stuck`, never as `ok 0`.
      let (s', v) := execBody fuel main.body s
      -- Fuel first: a run that hit fuel 0 anywhere is NOT answered with the
      -- value it produced afterwards (kcheck.py `whnf`: exhaustion raises).
      if s'.fuelOut then .fuelExhausted fuel
      else
        match s'.trapped with
        | some tc => .trap tc
        | none =>
        -- A MODEL GAP the run TOUCHED is reported even when a value came out.
        -- Measured 2026-09-14: c142_clone8 scored `ok 201` while
        -- `sys_arena_base()` was unmodelled, because its `let base = ...`
        -- quietly failed to bind and the tail never read `base`. A gap that an
        -- unused binding can hide is a gap that can hide anywhere.
        match s'.gap with
        | some g => .stuck s!"{g} [the run produced a value anyway; it is NOT reported, because the value was computed with this gap in it]"
        | none =>
        match v with
        | some v => .ok v
        | none   => .stuck (s'.stuckWhy.getD "no value, and no site recorded a reason (report this: every `none` site should call State.why)")

end Bebop.Semantics
