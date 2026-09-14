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
    /-- `return e` and `break` in EXPRESSION position. ROADMAP A18 step 1
        landed 2026-09-14 (`788b8dc`, promoted `c9d826c8`) and made both of
        these expressions, which `bench/parity_constructs/c124_condreturn.bp`
        exercises in an `if` ARM (`let _ = if n < 2 then return 1 else 0;`).
        docs/LANGUAGE.md:49 still lists `return` only as a statement; the
        compiler is the side that moved, so the doc is the stale one and this
        AST follows the compiler.

        Neither has a VALUE: evaluating one raises a control-flow signal
        (`State.pending`) and yields `none`, which is why they are not modelled
        as "an expression returning 0". -/
    | retExpr (e : Expr)
    | brkExpr
    /-- `let _ = NAME = e in body` -- an ASSIGNMENT to an existing binding,
        spelled through a throwaway `let` binder. It EVALUATES exactly like
        `letIn` (rebind, then body; `let` is fn-scoped with no shadowing), and
        it is a separate constructor only because the two differ STATICALLY:
        `let v0 = 0 in ...` introduces `v0`, while `let _ = v0 = 0 in ...`
        requires `v0` to exist already and is exit 101 when it does not. That
        is `bench/parity_constructs/neg/c93_unbound.bp` exactly, and collapsing
        the two into `letIn` is why it used to evaluate to `ok 0`. -/
    | assignIn (n : Name) (e body : Expr)
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
  /-- `let _ = NAME = e ;` -- assignment to an existing binding (c86_selfassign,
      c90_symalias, c95_symspan). Same evaluation as `let_`, different static
      rule: the target must already be bound. See `Expr.assignIn`. -/
  | assign (n : Name) (e : Expr)
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
  /-- `kernel fn f(...)`. Undocumented in LANGUAGE.md and real: c111_kernelfn
      compiles and neg/c112_kernelsys must be REFUSED with exit 102 for naming
      a `sys_*` inside one. The modifier used to be consumed and thrown away,
      which is why that refusal could not be modelled. -/
  isKernel : Bool := false
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

/-- The process exit code the runtime reports for each trap. Taken from the
    comments on the constructors above, which come from docs/TRAPS.md. It is a
    FUNCTION rather than a comment because `ParityRun` now ASSERTS a negative
    construct's `RUNFAIL:<code>` against it: a trap raised with the wrong code
    is a failure, not a pass. `frameOverflow` maps to 80 because A6 retired
    exit 81 and folded it into the arena trap. -/
def TrapCode.exitCode : TrapCode → Nat
  | .arenaExhausted => 80
  | .frameOverflow  => 80
  | .segfault       => 82
  | .codeBufferFull => 83
  | .boundsCheck    => 84
  | .unresolvedCall => 87
  | .stackOverflow  => 82
  | .tooManyFns     => 104
  | .tooManyParams  => 100
  | .unboundSymbol  => 101
  | .reservedWord   => 99
  | .sysInKernelFn  => 102
  | .noTailExpr     => 97
  | .stringConcat   => 96

def TrapCode.name : TrapCode → String
  | .arenaExhausted => "arenaExhausted"
  | .frameOverflow  => "frameOverflow"
  | .segfault       => "segfault"
  | .codeBufferFull => "codeBufferFull"
  | .boundsCheck    => "boundsCheck"
  | .unresolvedCall => "unresolvedCall"
  | .stackOverflow  => "stackOverflow"
  | .tooManyFns     => "tooManyFns"
  | .tooManyParams  => "tooManyParams"
  | .unboundSymbol  => "unboundSymbol"
  | .reservedWord   => "reservedWord"
  | .sysInKernelFn  => "sysInKernelFn"
  | .noTailExpr     => "noTailExpr"
  | .stringConcat   => "stringConcat"

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
-- 9a. The BUILTIN SURFACE, measured
--     `python3 tools/builtin_surface.py` in this tree, 2026-09-14: the
--     authority is bebop.bp's `emit_call_or_ctor` dispatch arms, recovered by
--     the compiler's own 131-rolling hash, and it reports
--         "compiler dispatches 41, resolved 41, unresolved 0"
--     Two consumers need it and they need it for OPPOSITE reasons, which is
--     why it lives here rather than in either:
--       * Bebop.Semantics: a call to one of these names that this model does
--         not implement is `stuck` -- a gap in formal/. A call to a name that
--         is NOT here and is not a declared `fn` is trap 87 -- a fact about
--         the program (neg/c52_undef).
--       * Bebop.Reject: a user `fn` named like one of these is exit 99 (T122).
--     The same run reports `sys_mapb` as the one name MISSING from bebop.bp's
--     T122 reserved table, so `fn sys_mapb` is shadowable where the other 40
--     are not; `reservedAgainstFn` below records that asymmetry rather than
--     smoothing it over.
-- ============================================================

def builtinSurface : Array String := #[
  "call_fn", "call_packed", "char", "clock_ms", "clz", "crc32", "crc32b",
  "crc32x", "hvham", "hvham2", "pack_fn", "scan", "str_len",
  "sys_arena_base", "sys_arena_end", "sys_atomic_add", "sys_clone",
  "sys_close", "sys_cond_set", "sys_exit", "sys_exit_thread_guard",
  "sys_export", "sys_fsync", "sys_ftruncate", "sys_futex_wait_guard",
  "sys_futex_wake", "sys_mapb", "sys_mmap", "sys_mprotect", "sys_msync",
  "sys_munmap", "sys_open", "sys_read", "sys_readbuf", "sys_rename",
  "sys_run", "sys_setaffinity", "sys_slurp", "sys_wait4", "sys_write",
  "zeros" ]

/-- The 40 of those 41 that bebop.bp's T122 table refuses as a `fn` name. -/
def reservedAgainstFn : Array String :=
  builtinSurface.filter (fun n => n != "sys_mapb")

#guard builtinSurface.size == 41
#guard reservedAgainstFn.size == 40

-- ============================================================
-- 9b. Control flow signals
--     Hoisted above `State` (2026-09-14) because `State` now carries a
--     PENDING signal: `return`/`break` in expression position have no value,
--     so `evalExpr` reports them by raising a signal in the state rather than
--     by inventing one.
-- ============================================================

inductive Signal where
  | cont
  | brk
  | ret (v : Val)
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
  /-- A control-flow signal raised by `return e` / `break` in EXPRESSION
      position (`Expr.retExpr` / `Expr.brkExpr`). While it is set, `evalExpr`
      evaluates NOTHING further -- that is the unwind, done without an
      exception monad -- and the first enclosing statement converts it into a
      `StmtResult.signal` and clears it (`Bebop.Semantics.takeSignal`). It is
      therefore never observable outside the statement that catches it; an
      unmatched one cannot leak into a value, because these expressions yield
      `none`, not 0. Mirrors bpref.py's `ReturnSignal`/`BreakSignal`. -/
  pending : Option Signal := none
  /-- A RUNTIME trap, raised and then sticky. Like `pending` it stops the
      evaluator dead (`evalExpr` evaluates nothing further while it is set),
      but unlike `pending` NOTHING catches it: it travels all the way to
      `evalProgram`, which reports `Result.trap`. Three raise it today --
      `zeros` past the arena capacity (80), a call to a name that is neither a
      builtin nor a declared fn (87), and the call-depth bound (82) -- and each
      is a construct's `RUNFAIL:<code>` expectation, compared by exit code.
      Kept apart from `stuck` on purpose: `stuck` is a gap in THIS model and a
      trap is a fact about the program. -/
  trapped : Option TrapCode := none
  /-- WHY the evaluator produced no value, recorded at the site that first
      produced `none`. Until 2026-09-14 `evalProgram` printed ONE message for
      four different causes --
        "main yielded no value (unbound symbol, unresolved call, arena fault,
         or no tail expression)"
      -- and 12 constructs carried it at once, which is why none of them was
      ever closed: the message named the whole class instead of the case.
      FIRST writer wins (`State.why`), so it is the site that actually stopped
      the run, not the outermost frame that noticed. -/
  stuckWhy : Option String := none
  /-- A MODEL GAP that the run actually touched: a string literal before the
      byte arena existed, or a call to a builtin in `Bebop.builtinSurface` that
      this model does not implement. It is reported even when the run went on
      to produce a value, and that is the point.

      MEASURED 2026-09-14, which is why it exists: `c142_clone8.bp` PASSED with
      `ok 201` while `sys_arena_base()` was entirely unmodelled. Its
      `let base = sys_arena_base();` evaluated to `none`, `Stmt.let_` answered
      `{ signal := .cont }` and simply did not bind, and the tail expression
      `a1 + ... + a6 + 138` never reads `base` -- so the construct scored a
      PASS for a program three of whose statements did nothing. A gap that can
      be hidden by an unused binding is a gap that can hide anywhere. -/
  gap : Option String := none
  /-- The BYTE arena. Separate from the cell arena because Bebop strings are
      byte-addressed: a `str` is the integer `(offset << 32) | length` into
      THIS buffer (tools/bpref.py `ev` for a `'str'` node, and `str_len`/`char`
      read it back). A string literal appends its UTF-8 bytes plus a NUL, as
      bpref does, so two occurrences of the same literal get different offsets
      -- which is what makes `t[0] == b` in c68_strval a test of handle
      identity rather than of content. -/
  bytes : Array UInt8 := #[]
  /-- Next file descriptor a modelled `sys_open` will hand out. 3 because 0/1/2
      are stdin/stdout/stderr. -/
  nextFd : Nat := 3
  /-- Bump pointer for modelled `sys_mmap` addresses, in bytes. -/
  mapCursor : Nat := 0
  /-- Number of user-function activations currently on the modelled stack.
      See `Bebop.Semantics.callDepthLimit`. -/
  depth : Nat := 0
  deriving Inhabited

/-- Record why evaluation produced no value, if nothing has yet. -/
def State.why (s : State) (msg : String) : State :=
  if s.stuckWhy.isSome then s else { s with stuckWhy := some msg }

/-- Record a MODEL GAP the run touched, and the reason, if nothing has yet.
    Also records the reason as the stuck reason. -/
def State.noteGap (s : State) (msg : String) : State :=
  let s := s.why msg
  if s.gap.isSome then s else { s with gap := some msg }

-- ============================================================
-- 10c. Strings: the byte arena
--      A Bebop `str` VALUE is the integer `(offset << 32) | length` into
--      `State.bytes`. Mirrors tools/bpref.py exactly:
--          off = len(self.bytes); self.bytes.extend(content); self.bytes.append(0)
--          return ((off << 32) | len(content))
--      and `str_len` = `s & 0xffffffff`, `char(s,i)` = `self.bytes[off+i]`.
-- ============================================================

/-- Intern a string literal: append its UTF-8 bytes plus the NUL terminator and
    return the handle. Two evaluations of the same literal get DIFFERENT
    offsets, exactly as bpref does -- the buffer is append-only. -/
def State.internStr (s : State) (str : String) : State × Val :=
  let bs := str.toUTF8.toList.toArray
  let off := s.bytes.size
  let s' := { s with bytes := (s.bytes ++ bs).push 0 }
  (s', Int64.ofNat (off * 4294967296 + bs.size))

/-- The byte offset encoded in a `str` handle (the high 32 bits). -/
def strOff (h : Val) : Nat := (h.toUInt64 >>> 32).toNat

/-- The length encoded in a `str` handle (the low 32 bits). -/
def strLen (h : Val) : Nat := (h.toUInt64 &&& 0xFFFFFFFF).toNat

/-- Read one byte of the byte arena; 0 past the end, which is what bpref's
    `char` does (`... if off + i < len(self.bytes) else 0`). -/
def State.byteAt (s : State) (i : Nat) : UInt8 := s.bytes.getD i 0


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
-- 11. Statement results
-- ============================================================

structure StmtResult where
  signal : Signal
  state : State
  deriving Inhabited

-- ============================================================
-- 12. Fuel
-- ============================================================

/-- Fuel for the definitional interpreter. Guarantees termination. -/
abbrev Fuel := Nat
