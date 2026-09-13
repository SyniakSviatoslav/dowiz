-- Bebop.Traps -- Trap set and static rejections (F1 census).
-- v2 (F4 coherence patch): aligns trap table with F1 census
-- and adds the static rejection codes needed by conformance.
--
-- References:
--  - docs/RESEARCH-VERIFICATION-2026-09-09.md §3 (F1 census)
--  - ROADMAP.md F1 (LANDED 2026-09-09)

import Bebop.Basic
import Bebop.Semantics

namespace Bebop.Traps

open Bebop.Semantics

-- ============================================================
-- 1. Complete trap table (24 rows from the F1 census)
-- ============================================================

/-- A trap table entry, as defined by the F1 census.
Each row has: a trap code, the source location, a description,
the mechanism that makes it unrepresentable, the word cost,
and whether it is closed (has a landed mechanism + neg/ construct). -/
structure TrapRow where
  id : Nat               -- 1..24
  code : Nat             -- exit code or trap code
  source : String        -- LANGUAGE.md / WORKER-CARD reference
  description : String
  mechanism : String     -- how to make it unrepresentable
  words : Nat            -- machine words added by the mechanism
  closed : Bool          -- has landed mechanism + neg/ construct
  deriving Inhabited, BEq

/-- The complete trap table from the F1 census.
docs/RESEARCH-VERIFICATION-2026-09-09.md section 3. -/
def trapTableF1 : Array TrapRow := #[
  -- Row 1: read past end of array [T]
  ⟨1, 84, "LANGUAGE.md:68,119,129", "read past end of array [T]",
    "[T] carries length; every access checked unless PROVEN i < len",
    3, false⟩,
  -- Row 2: store past end [T]
  ⟨2, 84, "WORKER-CARD", "store past end clobbers next object",
    "as row 1", 3, false⟩,
  -- Row 3: read of unbound symbol
  ⟨3, 105, "LANGUAGE.md:53-55", "read of symbol no let has executed",
    "definite assignment bitmask in planning scan", 0, false⟩,
  -- Row 4: array literal in loop read after loop (c34)
  ⟨4, 106, "LANGUAGE.md:107-112", "array literal in loop read after loop",
    "loop_alloc_safe computes escape; exit 106", 0, false⟩,
  -- Row 5: zeros inside while body (L8)
  ⟨5, 107, "AGENTS.md L8", "zeros inside while body",
    "syntactic: exit 107 at zeros token inside loop", 0, false⟩,
  -- Row 6: register pressure (exit 89)
  ⟨6, 89, "TRAPS.md:16", "let while a call temp is live",
    "register collision check; exit 89 with position", 0, false⟩,
  -- Row 7: > 8 live symbols across sys_clone
  ⟨7, 108, "WORKER-CARD", "> 8 live symbols across sys_clone",
    "count live symbols at clone site; exit 108", 0, false⟩,
  -- Row 8: deeply nested parenthesised calls (exit 95)
  ⟨8, 95, "TRAPS.md:21", "expected ) (nesting cap 128)",
    "report real cause at position", 0, false⟩,
  -- Row 9: nested if as call argument must be let-bound
  ⟨9, 89, "WORKER-CARD", "nested if as call argument",
    "A14b reports empty; one neg/ construct proves it", 0, false⟩,
  -- Row 10: division semantics (DEFINED)
  ⟨10, 109, "LANGUAGE.md:66", "x/0 = 0, x%0 = x",
    "DEFINED; optional exit 109 if compiler proves divisor non-zero", 1, false⟩,
  -- Row 11: >> vs >>> (logical vs arithmetic)
  ⟨11, 110, "LANGUAGE.md:64", ">> is logical, >>> is arithmetic",
    "A8 type tags split u64/i64", 0, false⟩,
  -- Row 12: 64-bit wrap on every operator
  ⟨12, 111, "LANGUAGE.md:73", "64-bit wrap on every operator",
    "checked dialect bit makes +-* trap 111 on overflow", 2, false⟩,
  -- Row 13: &&/|| non-short-circuit
  ⟨13, 112, "bpref T125", "&&/|| bitwise, not short-circuit",
    "reject with exit 112 or implement short-circuit (+2 words)", 0, false⟩,
  -- Row 14: let rebinds, no shadowing
  ⟨14, 113, "LANGUAGE.md:41-43", "let rebinds the same register",
    "DEFINED; A8 tags make type mismatch exit 113", 0, false⟩,
  -- Row 15: s.f indexes first struct's field list
  ⟨15, 114, "bpref", "s.f indexes first struct's field list",
    "receiver type from A8 tag; exit 114", 0, false⟩,
  -- Row 16: match <var> emitted no code (CLOSED)
  ⟨16, 0, "ROADMAP A6", "match <var> emitted no code",
    "closed by c96_enumpay", 0, true⟩,
  -- Row 17: call to unresolved function
  ⟨17, 115, "TRAPS.md:14", "call to unresolved function",
    "fntab_lookup at end; exit 115 with name and position", 0, false⟩,
  -- Row 18: 15th parameter (CLOSED)
  ⟨18, 100, "TRAPS.md:26", "fn with > 14 parameters",
    "closed (A13)", 0, true⟩,
  -- Row 19: unbound symbol (CLOSED)
  ⟨19, 101, "TRAPS.md:27", "unbound symbol",
    "closed (A15)", 0, true⟩,
  -- Row 20: sys_ inside kernel fn (CLOSED)
  ⟨20, 102, "TRAPS.md:28", "sys_ inside kernel fn",
    "closed (C1)", 0, true⟩,
  -- Row 21: sys_clone with literal 0
  ⟨21, 116, "WORKER-CARD", "sys_clone(flags, 0)",
    "literal 0 is exit 116 at compile time; non-literal gets cbz", 1, false⟩,
  -- Row 22: char(s, i) on raw pointer
  ⟨22, 82, "LANGUAGE.md:89", "char(s, i) on raw pointer",
    "A7 str = (off<<32 | len) makes char a checked index", 0, false⟩,
  -- Row 23: use not line-initial
  ⟨23, 118, "LANGUAGE.md:17", "use not line-initial",
    "any use token not at column 0 is exit 118", 0, false⟩,
  -- Row 24: deep recursion -> trap 82
  ⟨24, 82, "LANGUAGE.md:117", "deep recursion -> trap 82 at 16 KiB/frame",
    "CAPACITY: cannot be static; loud with attribution is the floor", 0, false⟩
]

/- Verify F1 census counts. -/
#guard ((trapTableF1.filter (fun r => r.closed)).size == 4)
#guard ((trapTableF1.filter (fun r => !r.closed)).size == 20)
#guard (trapTableF1.size == 24)

-- ============================================================
-- 2. Static rejection checker
-- ============================================================

/-- A static check that can reject a program at compile time.
Each check corresponds to a trap row that costs 0 words. -/
inductive StaticRejection where
  /-- Row 3: read of symbol no let has executed. -/
  | definiteAssignment (pos : Position) (sym : Name)
  /-- Row 4: array literal in loop body read after loop. -/
  | loopLiteralEscape (pos : Position) (sym : Name)
  /-- Row 5: zeros inside while body. -/
  | zerosInLoop (pos : Position)
  /-- Row 6: register pressure / let while call temp is live. -/
  | registerPressure (pos : Position) (msg : String)
  /-- Row 7: > 8 live symbols across sys_clone. -/
  | tooManyLiveSymbols (pos : Position) (count : Nat)
  /-- Row 8: deeply nested parenthesised calls. -/
  | nestingDepth (pos : Position) (depth : Nat)
  /-- Row 9: nested if as call argument must be let-bound. -/
  | nestedIfInCall (pos : Position)
  /-- Row 11: >> on i64 (should use >>>). -/
  | logicalShiftOnSigned (pos : Position)
  /-- Row 13: &&/|| (non-short-circuit). -/
  | nonShortCircuit (pos : Position)
  /-- Row 14: let changes type of a variable. -/
  | typeMismatch (pos : Position) (var : Name) (old new : String)
  /-- Row 15: struct field not found. -/
  | fieldNotFound (pos : Position) (struct field : String)
  /-- Row 17: call to unresolved function. -/
  | unresolvedFunction (pos : Position) (name : Name)
  /-- Row 21: sys_clone with literal 0. -/
  | cloneZero (pos : Position)
  /-- Row 23: use not line-initial. -/
  | useNotLineInitial (pos : Position)
  deriving Inhabited

-- ============================================================
-- 3. Runtime trap detector
-- ============================================================

/-- Check if a Result is a trap that matches a known trap row. -/
def isKnownTrap (r : Result) : Option Nat :=
  match r with
  | .trap code =>
    match code with
    | .arenaExhausted => some 80
    | .segfault | .stackOverflow => some 82
    | .unresolvedCall => some 87
    | _ => none
  | _ => none

-- ============================================================
-- 4. Gate predicates
-- ============================================================

/-- trap_unrep: count of trap rows with a landed mechanism and neg/ construct.
From section 10 gate ladder. -/
def trapUnrepCount : Nat := 4  -- closedCount from F1 census

/-- Total word cost of all open trap mechanisms. -/
def totalWordCost : Nat :=
  trapTableF1.filter (fun r => !r.closed) |>.foldl (fun acc r => acc + r.words) 0

/-- Count of closed trap rows. -/
def closedCount : Nat :=
  trapTableF1.filter (fun r => r.closed) |>.size

/-- Count of open trap rows. -/
def openCount : Nat :=
  trapTableF1.filter (fun r => !r.closed) |>.size

/-- Total trap rows. -/
def totalTrapCount : Nat := trapTableF1.size

end Bebop.Traps
