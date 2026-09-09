/-
  Bebop.Basic -- Core types for the Bebop definitional semantics.

  Every value in Bebop is a signed 64-bit integer (i64). Arrays are offsets
  into a single arena (A5 step 1b). Types are parsed and discarded by the
  compiler; they are checked by T48 outside it.

  References:
  - docs/LANGUAGE.md (status 2026-09-05, T119)
  - docs/RESEARCH-VERIFICATION-2026-09-09.md section 5.2
  - tools/bpref.py (727 lines, the executable reference semantics)
-/
import Lean

-- ============================================================
-- 1. Machine integers (Z/2^64 wrapping arithmetic)
-- ============================================================

/-- A Bebop value: a signed 64-bit integer, wrapping on overflow.
    LANGUAGE.md:73 "All arithmetic is 64-bit wrapping." -/
abbrev Val := Int64

namespace Val

/-- Signed truncating division. LANGUAGE.md:66: x/0 = 0, MIN/-1 = MIN. -/
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
  | arr      -- [i64], an array of i64
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
    | matchExpr (arms : Array MatchArm)
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
-- 7. Arena and memory model
-- ============================================================

/-- The arena: one 256 MB anonymous mapping. LANGUAGE.md:106-109.
    `zeros` bumps it; nothing is freed; crossing the end exits 80. -/
structure Arena where
  cells : Array Val
  capacity : Nat := 33554432  -- 256 MiB / 8

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
  env : Array (Name × Val) := #[]
  arena : Array Val := #[]
  arenaCapacity : Nat := 33554432
  frame : Array Val := #[]
  frameCapacity : Nat := 2048
  enumTags : Array (Name × Nat) := #[]
  enumArities : Array (Name × Nat) := #[]
  fns : Array FnDecl := #[]
  structs : Array (Name × Array Name) := #[]
  clockMs : Val := 0
  deriving Inhabited

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
