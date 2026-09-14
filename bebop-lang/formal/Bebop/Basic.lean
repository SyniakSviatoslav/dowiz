/-
  Bebop.Basic -- Core types for the Bebop definitional semantics.

  Every value in Bebop is a signed 64-bit integer (i64). Arrays are offsets
  into a single arena (A5 step 1b). Types are parsed and discarded by the
  compiler; they are checked by T48 outside it.

  References:
  - docs/LANGUAGE.md (status 2026-09-05, T119)
  - docs/RESEARCH-VERIFICATION-2026-09-09.md section 5.2
  - tools/bpref.py (727 lines, the executable reference semantics)

  This module imports nothing beyond Init (which already provides Int64):
  it is the leaf of the import DAG and every other Bebop module imports it.
-/

import Std.Data.TreeMap

-- ============================================================
-- 1. Machine integers (Z/2^64 wrapping arithmetic)
-- ============================================================

/-- A Bebop value: a signed 64-bit integer, wrapping on overflow.
    LANGUAGE.md:73 "All arithmetic is 64-bit wrapping." -/
abbrev Val := Int64

namespace Val

/-- Signed truncating division. LANGUAGE.md:66: x/0 = 0, MIN / -1 = MIN. -/
def sdiv (a b : Val) : Val :=
  if b == 0 then 0 else a / b

/-- Signed truncating remainder. LANGUAGE.md:66: x%0 = x. -/
def srem (a b : Val) : Val :=
  if b == 0 then a else a % b

end Val

-- ============================================================
-- 2. Names and identifiers
-- ============================================================

/-- A Bebop identifier (function names, parameter names, variable names). -/
abbrev Name := String

-- ============================================================
-- 3. Types (parsed and discarded; present only for static checks)
-- ============================================================

/-- Surface types in Bebop. Types are checked by T48 then discarded.
    LANGUAGE.md:23: TYPE := 'i64' | 'str' | '[' 'i64' ']' | NAME -/
inductive Ty where
  | i64
  | str
  | arr      -- [i64]
  | named    -- user-defined enum/struct name
  | ref_t    -- ref T
  deriving BEq, Inhabited

-- ============================================================
-- 4. Expressions
-- ============================================================

/-- Binary operators, matching the C-precedence tiers of LANGUAGE.md:60-68.
    Tier ordering (loosest first): cmp, |, ^, &, shifts, +-, */%. -/
inductive BinOp where
  | add | sub | mul | sdiv | srem
  | band | bor | bxor
  | lsl  | lsr  | asr
  | eq | neq | slt | sgt | sle | sge
  | lor | land
  deriving BEq, Inhabited

/-- Unary operators. LANGUAGE.md:65: -e (neg), !e = (e == 0). -/
inductive UnOp where
  | neg | lnot
  deriving BEq, Inhabited

mutual
  inductive Expr where
    | lit (v : Val)
    | var (n : Name)
    | paren (e : Expr)
    | binop (op : BinOp) (l r : Expr)
    | unop (op : UnOp) (e : Expr)
    | arrLit (es : Array Expr)
    | arrGet (arr idx : Expr)
    | arrSet (arr idx val : Expr)
    | call (fn : Name) (args : Array Expr)
    | ite (c t f : Expr)
    | letIn (n : Name) (e body : Expr)
    | matchExpr (scrut : Expr) (arms : Array MatchArm)     -- match scrut { arms }
    | structLit (name : Name) (fields : Array (Name × Expr))  -- struct literal
    | enumLit (name : Name) (arg : Option Expr)                -- enum ctor
    | fieldAcc (structExpr : Expr) (field : Name)              -- s.f
    | builtin (name : Name) (args : Array Expr)                -- builtin call
    /-- A string literal. LANGUAGE.md:80 allows `"..."` ONLY as a call
        argument (`str_len(s)`, `char(s, i)`). It is an AST node rather than a
        parse error so that a program containing one PARSES -- the syntax is
        understood -- and then fails at EVALUATION with a named reason. Those
        are different defects and the conformance score must not merge them:
        no byte arena is modelled, so `Bebop.Semantics` reports `stuck` here
        (see `evalExpr`'s `.strLit` arm) rather than inventing a handle. -/
    | strLit (s : String)
  deriving Inhabited

  structure MatchArm where
    ctor : Name
    binder : Option Name
    body : Expr
  deriving Inhabited
end

-- ============================================================
-- 5. Statements
-- ============================================================

inductive Stmt where
  | let_ (n : Name) (e : Expr)
  | drop (e : Expr)
  | arrStore (arr idx val : Expr)
  | compound (n : Name) (op : BinOp) (e : Expr)
  | while_ (cond : Expr) (body : Array Stmt)
  | ret (e : Expr)
  | brk
  | exprStmt (e : Expr)
  deriving Inhabited

-- ============================================================
-- 6. Top-level declarations
-- ============================================================

structure EnumCtor where
  name : Name
  hasPayload : Bool
  deriving BEq, Inhabited

structure EnumDecl where
  name : Name
  ctors : Array EnumCtor
  deriving Inhabited

structure StructDecl where
  name : Name
  fields : Array Name
  deriving Inhabited

structure FnDecl where
  name : Name
  params : Array Name
  paramTypes : Array Ty
  returnType : Ty
  body : Array Stmt
  deriving Inhabited

structure Program where
  enums  : Array EnumDecl
  structs : Array StructDecl
  fns    : Array FnDecl
  deriving Inhabited

-- ============================================================
-- 7. Arena and memory model (single one-cell-array, 64-bit cells)
-- ============================================================

/-- The arena: one 256 MB anonymous mapping. LANGUAGE.md:106-109.
    `zeros` bumps it; nothing is freed; crossing the end exits 80.
    Frame heap is the arena itself (A6: no separate frame heap).

    SPARSE, since 2026-09-14. `cells` was `Array Val` and the arena was
    physically materialised, which made allocation QUADRATIC. Measured before
    the change with `lake exe timeone`, on a loop doing two 3-cell array
    literals per iteration (`c33_loopalloc`'s exact shape), wall clock taken
    from OUTSIDE the process, of which ~155 ms is startup:

        iterations   2000     4000      8000      16000
        wall         346 ms   1195 ms   4436 ms   21703 ms
        ratio                 3.45x     3.71x     4.89x

    Doubling the iteration count multiplied the time by 3.5-4.9x, i.e.
    Theta(n^2). The identical loop with NO allocation was FLAT at 150-160 ms
    from 2000 to 16000 iterations, so the interpreter is linear and the arena
    was the whole cost. Two independent factors produced it:

      1. `zeros` was `cells ++ Array.replicate n 0`, which MATERIALISES n
         zeros, and `Array.append` copies its left argument whenever that array
         is not uniquely referenced.
      2. Uniqueness was routinely lost. `evalExpr`'s `.arrLit` arm read
         `s1.arena.cells.size` AFTER deriving `s2` from `s1`, so both states
         were live and every append copied the whole arena.

    This representation removes both -- and removes the second BY CONSTRUCTION
    rather than by care, since there is no array left to copy:

    THE STRUCTURE IS A PERSISTENT TREE, NOT A HASH MAP, AND THAT IS THE POINT.
    `Std.HashMap` was tried FIRST and measured WORSE than the array -- same
    quadratic shape, roughly twice the constant:

        iterations    2000     4000      8000      16000     32000
        Array         346 ms   1195 ms   4436 ms   21703 ms   (not run)
        Std.HashMap   457 ms   1800 ms   8065 ms   47295 ms   246918 ms
        Std.TreeMap   see the table in formal/README.md

    The reason is that `Array.set!` and `Std.HashMap.insert` are both
    COPY-ON-WRITE-WHEN-SHARED: they mutate in place only while the reference is
    unique, and copy the whole structure otherwise. This evaluator threads
    `State` functionally and routinely keeps two versions live at once -- the
    `.while_` arm alone holds `result` while building `s3` from
    `result.state` -- so uniqueness is lost on essentially every iteration and
    the "amortised O(1)" never applies. Making it apply would mean auditing
    reference uniqueness across all 680 lines of the evaluator and preserving
    it under every future edit, which is not a property a reader can check.

    `Std.TreeMap` is a PERSISTENT balanced tree: `insert` allocates O(log n)
    new nodes and SHARES every untouched subtree, so its cost does not depend
    on whether anything else holds a reference. That turns the per-operation
    cost from "O(1) if unique, O(n) if not" into "O(log n), always" -- which is
    worse than the best case and enormously better than the actual case.

        operation    was                                    now
        zeros(n)     Theta(n), materialises n zeros          O(1): `cursor += n`
        write        O(1) if unique, O(n) when shared        O(log n) ALWAYS
        read         O(1) array index                        O(log n) ALWAYS

    The read path is the deliberate cost: reads are the common case in an
    evaluator and O(log n) is not O(1). It is paid knowingly, because the
    alternative measured quadratic. log2(600000) is about 20 comparisons on
    `Nat` keys, and the corpus wall-clock before and after is in
    formal/README.md so the trade is visible rather than asserted.

    `cursor` is the bump pointer: cells `[0, cursor)` are ALLOCATED. `cells`
    holds only the cells that have been WRITTEN; an allocated cell that was
    never written reads as 0, which is not an approximation -- it is what
    `zeros` MEANS (LANGUAGE.md:85, "allocate n zeroed i64 cells"). So no zero is
    ever stored and `zeros(n)` does no work proportional to n at all.

    Bounds behaviour is UNCHANGED, which is what keeps the trap set intact:
    `off < cursor` is in bounds and reads 0 if unwritten; `off >= cursor` is out
    of bounds and `get?` is `none`, exactly as the old `off < cells.size` test
    behaved. `capacity` is still checked against `cursor + n`, so
    `zeros(40000000)` still traps `arenaExhausted` without touching memory. -/
structure Arena where
  /-- Only the WRITTEN cells. Absent means "allocated but never written" = 0. -/
  cells : Std.TreeMap Nat Val := ∅
  /-- Bump pointer: cells `[0, cursor)` are allocated. -/
  cursor : Nat := 0
  capacity : Nat := 33554432  -- 256 MiB / 8

/-- Number of ALLOCATED cells. This was spelled `arena.cells.size`; that now
    means the number of WRITTEN cells, a different and much smaller number, so
    every caller must use this instead. -/
def Arena.size (a : Arena) : Nat := a.cursor

/-- Read one cell. `none` iff the offset is not allocated -- the same
    out-of-bounds condition the `Array` version had. An allocated cell that was
    never written reads as 0. -/
def Arena.get? (a : Arena) (off : Nat) : Option Val :=
  if off < a.cursor then some (a.cells.getD off 0) else none

/-- Write one cell, or `none` if the offset is not allocated. -/
def Arena.set? (a : Arena) (off : Nat) (v : Val) : Option Arena :=
  if off < a.cursor then some { a with cells := a.cells.insert off v } else none

/-- A frame-allocated array is just an offset (start index) into the arena.
    The length is tracked separately via the struct field list or enum arity. -/
abbrev FrameArray := Nat  -- offset into arena

-- ============================================================
-- 8. Trap codes and runtime state
-- ============================================================

inductive TrapCode where
  | arenaExhausted    -- exit 80
  | frameOverflow     -- exit 81 (retired in A6, now exit 80)
  | segfault          -- exit 82
  | codeBufferFull    -- exit 83
  | boundsCheck       -- trap 84
  | unresolvedCall    -- trap 87
  | stackOverflow     -- trap 82 (deep recursion)
  | tooManyFns        -- exit 104
  | tooManyParams     -- exit 100
  | unboundSymbol     -- exit 101
  | reservedWord      -- exit 99
  | sysInKernelFn     -- exit 102
  | noTailExpr        -- exit 97
  | stringConcat      -- exit 96
  deriving Inhabited, BEq, Repr

structure Position where
  line : Nat
  col : Nat
  deriving Inhabited, BEq, Repr

inductive Result where
  | ok (v : Val)
  | trap (code : TrapCode)
  | rejected (code : Nat) (pos : Position) (msg : String)
  /-- The evaluator produced no value: the function body's tail expression
      (or a `ret`) evaluated to `none` -- unbound symbol, unresolved call,
      arena fault, fuel exhausted, or a body with no tail expression. Kept
      distinct from `ok` so a failed evaluation can never print as `ok 0`
      (which is what every harness sample printed until 2026-09-13). -/
  | stuck (msg : String)
  /-- The evaluator ran out of fuel somewhere in the run. Reported INSTEAD of
      whatever value the run produced afterwards: until 2026-09-13 every
      fuel-0 arm answered `.cont`/`none` and the run carried on, so
      `while 1 { 0 }; 0` was `ok 0` at fuel 1000 where `bebop.bin` never
      terminates -- a diverging program could pass. Mirrors tools/kcheck.py
      `whnf`: exhaustion RAISES, "a checker that timed out into 'yes' would be
      unsound". Not a `.trap`, so `checkExpected`'s any-trap arm cannot accept it. -/
  | fuelExhausted (fuel : Nat)
  deriving Inhabited

-- ============================================================
-- 9. Footprint for axiomatised syscalls
-- ============================================================

/-- A footprint describes which cells a syscall reads and writes.
    26 sys_* are axiomatised; 7 threading builtins are OUT OF
    the single-thread semantics. -/
structure Footprint where
  reads : Array Nat
  writes : Array Nat
  errorSet : Array Val
  deriving Inhabited

-- ============================================================
-- 10. Runtime state
-- ============================================================

/-- The complete runtime state. -/
structure State where
  env : Array (Name × Val) := #[]            -- fn-scoped bindings
  arena : Arena := {}
  frameArrays : Array (Name × FrameArray × Nat) := #[]  -- name -> (base_offset, length)
  enumTags : Array (Name × Nat) := #[]      -- ctor name -> tag
  enumArities : Array (Name × Nat) := #[]   -- ctor name -> arity (0 or 1)
  fns : Array FnDecl := #[]
  structs : Array (Name × Array Name) := #[] -- struct name -> field list
  clockMs : Val := 0
  /-- Sticky: set by every fuel-0 arm of the evaluator, never cleared, read by
      `evalProgram`. A run that touched fuel 0 is reported as
      `Result.fuelExhausted`, whatever value it went on to produce. -/
  fuelOut : Bool := false
  deriving Inhabited


-- ============================================================
-- 10b. State operations (environment and arena)
--      Hoisted from Semantics.lean so that Builtins/Syscalls can use
--      them without importing the evaluator (this breaks the
--      Builtins <-> Semantics <-> Syscalls import cycles). They live
--      in the root namespace because `State` does: dot-notation
--      `s.lookup` only resolves `State.lookup`, not
--      `Bebop.Semantics.State.lookup`.
-- ============================================================

/-- Look up a binding (most recent wins; fn-scoped rebind).
    LANGUAGE.md:41-43: let rebinds the same register, no shadowing. -/
def State.lookup (s : State) (n : Name) : Option Val :=
  s.env.find? (fun p => p.1 == n) |>.map (·.2)

/-- Bind or REBIND a name (fn-scoped, no shadowing). LANGUAGE.md:41-43: "a later
    `let x` updates the same register". An existing entry is updated in place;
    only a new name is pushed. (Until 2026-09-13 this always pushed while
    `State.lookup` returned the FIRST match, so every rebind -- `let x = x + 1`,
    `x += 1` -- was invisible: c07_while looped on `i = 0` to fuel exhaustion
    and printed `ok 0`. Measured, then fixed.) -/
def State.bind (s : State) (n : Name) (v : Val) : State :=
  match s.env.findIdx? (fun p => p.1 == n) with
  | some i => { s with env := s.env.setIfInBounds i (n, v) }
  | none   => { s with env := s.env.push (n, v) }

/-- Look up a frame-allocated array by name. -/
def State.lookupFrameArray (s : State) (n : Name) : Option (FrameArray × Nat) :=
  s.frameArrays.find? (fun p => p.1 == n) |>.map (·.2)

/-- Look up a frame-allocated array by base offset. -/
def State.lookupFrameArrayAny (s : State) (base : Nat) : Option (FrameArray × Nat) :=
  s.frameArrays.find? (fun p => p.2.1 == base) |>.map (·.2)

/-- Register a frame-allocated array (base offset + length). -/
def State.registerFrameArray (s : State) (n : Name) (base : FrameArray) (len : Nat) : State :=
  { s with frameArrays := s.frameArrays.push (n, base, len) }

/-- zeros(n): allocate n zeroed i64 cells. Exit 80 if exhausted.

    O(1) IN n: it advances `cursor` and stores nothing. The zeros are not
    materialised because an unwritten allocated cell already READS as 0 (see
    `Arena.get?`). The capacity test is on the cursor, so an allocation larger
    than the arena still traps without touching memory. -/
def State.zeros (s : State) (n : Nat) : State × Option TrapCode :=
  let newCursor := s.arena.cursor + n
  if newCursor ≤ s.arena.capacity then
    ({ s with arena := { s.arena with cursor := newCursor } }, none)
  else
    (s, some TrapCode.arenaExhausted)

/-- Read a cell from the arena. Returns none if OOB (off >= cursor). -/
def State.arenaRead (s : State) (off : Nat) : Option Val :=
  s.arena.get? off

/-- Write a cell to the arena. Returns none if OOB (off >= cursor). -/
def State.arenaWrite (s : State) (off : Nat) (v : Val) : Option State :=
  match s.arena.set? off v with
  | some a => some { s with arena := a }
  | none => none

-- ============================================================
-- 11. Control flow signals
-- ============================================================

inductive Signal where
  | cont
  | brk
  | ret (v : Val)
  deriving Inhabited

structure StmtResult where
  signal : Signal
  state : State
  deriving Inhabited

-- ============================================================
-- 12. Fuel
-- ============================================================

/-- Fuel for the definitional interpreter. Guarantees termination. -/
abbrev Fuel := Nat
