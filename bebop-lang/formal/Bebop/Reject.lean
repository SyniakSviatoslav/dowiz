/-
  Bebop.Reject -- the STATIC rejection pass: the checks `bebop.bin` runs
  BEFORE it emits anything, and that `formal/` had none of.

  WHY THIS FILE EXISTS. Six `neg/*.bp` constructs used to PARSE, EVALUATE and
  report `VALUE_MISMATCH got ok N` -- c113_shadowclz `ok 60`, c114_shadowwait4
  `ok 5`, c120_oobstatic `ok 0`, c85_param15 `ok 15`, c93_unbound `ok 0`,
  read_before_assign, c112_kernelsys, c140_mapb_refused, c141_clone9. Every one
  of them is a program the compiler refuses at COMPILE time, so evaluating them
  at all was the wrong answer; the model had no notion of a static refusal.

  `c93_unbound` is the one worth naming twice: an evaluator that answers `0`
  for an unbound name is not merely failing one construct, it is silently wrong
  for every program with a typo. This pass is what makes a typo loud.

  WHAT IT MUST NOT BECOME. A pass that refuses programs the compiler accepts is
  worse than no pass at all: it would turn the 101 POSITIVE constructs red one
  at a time, and each rule below is therefore paired with the positive control
  that brackets it in the corpus --

    | rule                | negative (must refuse) | positive control (must NOT) |
    |---------------------|------------------------|-----------------------------|
    | builtin-named `fn`  | c113, c114             | every other `fn` in 121 files |
    | > 14 params         | c85_param15 (15)       | c92's `f0` has exactly 14   |
    | `sys_*` in kernel fn| c112_kernelsys         | c111_kernelfn               |
    | `sys_mapb`          | c140_mapb_refused      | -- no positive calls it     |
    | > 8 syms at clone   | c141_clone9 (9)        | c142_clone8 (exactly 8)     |
    | unbound symbol      | c93, read_before_assign| c86, c90, c95 (assign to a
    |                     |                        |  name that IS bound)        |
    | literal index >= len| c120_oobstatic (a[3]   | c13_array and the rest, all
    |                     |  on a 3-cell literal)  |  in range                   |

  Positions: the AST carries none, so every diagnosis reports 0:0 and names the
  FUNCTION instead. That is a real shortfall against `bebop.bin`, which prints
  the source position; it is recorded here rather than papered over.
-/

import Bebop.Basic

namespace Bebop.Reject

/-- One static refusal: the compiler exit code and a human diagnosis. -/
structure Diag where
  code : Nat
  msg  : String
  deriving Inhabited

-- ============================================================
-- Walking helpers
-- ============================================================

/-- Every `Expr.call` name appearing anywhere in an expression. -/
partial def callsInExpr : Expr → Array Name
  | .lit _ | .var _ | .strLit _ | .brkExpr => #[]
  | .paren e | .unop _ e | .retExpr e => callsInExpr e
  | .binop _ l r => callsInExpr l ++ callsInExpr r
  | .arrLit es => es.flatMap callsInExpr
  | .arrGet a i => callsInExpr a ++ callsInExpr i
  | .arrSet a i v => callsInExpr a ++ callsInExpr i ++ callsInExpr v
  | .call f as => (as.flatMap callsInExpr).push f
  | .builtin f as => (as.flatMap callsInExpr).push f
  | .ite c t f => callsInExpr c ++ callsInExpr t ++ callsInExpr f
  | .letIn _ e b => callsInExpr e ++ callsInExpr b
  | .assignIn _ e b => callsInExpr e ++ callsInExpr b
  | .matchExpr sc arms => callsInExpr sc ++ arms.flatMap (fun a => callsInExpr a.body)
  | .structLit _ fs => fs.flatMap (fun p => callsInExpr p.2)
  | .enumLit _ a => match a with | some e => callsInExpr e | none => #[]
  | .fieldAcc e _ => callsInExpr e

partial def callsInStmt : Stmt → Array Name
  | .let_ _ e | .assign _ e | .drop e | .ret e | .exprStmt e => callsInExpr e
  | .compound _ _ e => callsInExpr e
  | .arrStore a i v => callsInExpr a ++ callsInExpr i ++ callsInExpr v
  | .while_ c b => callsInExpr c ++ b.flatMap callsInStmt
  | .brk => #[]

/-- Every name a `let` (or a match arm, or a compound rebind) BINDS anywhere in
    an expression. `assignIn` is deliberately NOT here: it assigns, it does not
    bind, which is the whole point of the unbound rule. -/
partial def bindsInExpr : Expr → Array Name
  | .lit _ | .var _ | .strLit _ | .brkExpr => #[]
  | .paren e | .unop _ e | .retExpr e => bindsInExpr e
  | .binop _ l r => bindsInExpr l ++ bindsInExpr r
  | .arrLit es => es.flatMap bindsInExpr
  | .arrGet a i => bindsInExpr a ++ bindsInExpr i
  | .arrSet a i v => bindsInExpr a ++ bindsInExpr i ++ bindsInExpr v
  | .call _ as => as.flatMap bindsInExpr
  | .builtin _ as => as.flatMap bindsInExpr
  | .ite c t f => bindsInExpr c ++ bindsInExpr t ++ bindsInExpr f
  | .letIn n e b => (bindsInExpr e ++ bindsInExpr b).push n
  | .assignIn _ e b => bindsInExpr e ++ bindsInExpr b
  | .matchExpr sc arms => bindsInExpr sc ++ arms.flatMap (fun a =>
      match a.binder with
      | some bn => (bindsInExpr a.body).push bn
      | none => bindsInExpr a.body)
  | .structLit _ fs => fs.flatMap (fun p => bindsInExpr p.2)
  | .enumLit _ a => match a with | some e => bindsInExpr e | none => #[]
  | .fieldAcc e _ => bindsInExpr e

partial def bindsInStmt : Stmt → Array Name
  | .let_ n e => (bindsInExpr e).push n
  | .compound n _ e => (bindsInExpr e).push n
  | .assign _ e | .drop e | .ret e | .exprStmt e => bindsInExpr e
  | .arrStore a i v => bindsInExpr a ++ bindsInExpr i ++ bindsInExpr v
  | .while_ c b => bindsInExpr c ++ b.flatMap bindsInStmt
  | .brk => #[]

/-- Every name READ as a variable, and every name ASSIGNED (which is also a
    read of the register, and the case neg/c93_unbound turns on). -/
partial def readsInExpr : Expr → Array Name
  | .lit _ | .strLit _ | .brkExpr => #[]
  | .var n => #[n]
  | .paren e | .unop _ e | .retExpr e => readsInExpr e
  | .binop _ l r => readsInExpr l ++ readsInExpr r
  | .arrLit es => es.flatMap readsInExpr
  | .arrGet a i => readsInExpr a ++ readsInExpr i
  | .arrSet a i v => readsInExpr a ++ readsInExpr i ++ readsInExpr v
  | .call _ as => as.flatMap readsInExpr
  | .builtin _ as => as.flatMap readsInExpr
  | .ite c t f => readsInExpr c ++ readsInExpr t ++ readsInExpr f
  | .letIn _ e b => readsInExpr e ++ readsInExpr b
  | .assignIn n e b => (readsInExpr e ++ readsInExpr b).push n
  | .matchExpr sc arms => readsInExpr sc ++ arms.flatMap (fun a => readsInExpr a.body)
  | .structLit _ fs => fs.flatMap (fun p => readsInExpr p.2)
  | .enumLit _ a => match a with | some e => readsInExpr e | none => #[]
  | .fieldAcc e _ => readsInExpr e

partial def readsInStmt : Stmt → Array Name
  | .let_ _ e | .drop e | .ret e | .exprStmt e => readsInExpr e
  | .assign n e => (readsInExpr e).push n
  | .compound n _ e => (readsInExpr e).push n
  | .arrStore a i v => readsInExpr a ++ readsInExpr i ++ readsInExpr v
  | .while_ c b => readsInExpr c ++ b.flatMap readsInStmt
  | .brk => #[]

-- ============================================================
-- Rule: a user `fn` may not be named like a builtin (T122, exit 99)
-- ============================================================

/-- neg/c113_shadowclz (`fn clz`) and neg/c114_shadowwait4 (`fn sys_wait4`).
    The list is `Bebop.reservedAgainstFn`, derived from the compiler's own
    dispatch table by `tools/builtin_surface.py`, NOT hand-written -- c114's
    header is about exactly the kind of drift a hand-written table produces
    (bebop.bp spells one hash signed and the other unsigned, and the audit
    reported 16 shadowable builtins instead of 7). -/
def checkReservedFnName (p : Program) : Option Diag :=
  match p.fns.find? (fun f => reservedAgainstFn.contains f.name) with
  | some f => some ⟨99, s!"fn `{f.name}` is named like a builtin (T122: bebop.bin's reserved table refuses it, exit 99)"⟩
  | none => none

-- ============================================================
-- Rule: at most 14 parameters (exit 100)
-- ============================================================

/-- neg/c85_param15 declares 15. The positive side of the boundary is in the
    corpus too: neg/c92_letlive2's `f0` has exactly 14 and is NOT refused for
    arity (it is refused, for having no tail expression, by the parser). -/
def checkParamCount (p : Program) : Option Diag :=
  match p.fns.find? (fun f => f.params.size > 14) with
  | some f => some ⟨100, s!"fn `{f.name}` declares {f.params.size} parameters; the limit is 14 (exit 100)"⟩
  | none => none

-- ============================================================
-- Rule: no `sys_*` inside a `kernel fn` (exit 102)
-- ============================================================

/-- neg/c112_kernelsys: `kernel fn kbad(fd) { sys_close(fd) }`. The construct's
    own header says the reject is on the NAME's `sys_` PREFIX in emit_ident,
    not on a list of known builtins -- "a guard that only knew about a list of
    builtins would pass this and fail a fn-local `sys_foo`" -- so the test here
    is the prefix, deliberately, and not membership of `builtinSurface`. -/
def checkKernelSyscall (p : Program) : Option Diag :=
  let bad := p.fns.filterMap (fun f =>
    if !f.isKernel then none
    else match (f.body.flatMap callsInStmt).find? (fun n => n.startsWith "sys_") with
         | some n => some (f.name, n)
         | none => none)
  match bad[0]? with
  | some (fn, call) => some ⟨102, s!"`kernel fn {fn}` calls `{call}`; no `sys_` name may appear in a kernel fn (exit 102)"⟩
  | none => none

-- ============================================================
-- Rule: `sys_mapb` is refused outright (exit 108)
-- ============================================================

/-- neg/c140_mapb_refused. The builtin's emitter passed the path's LENGTH as
    openat's FLAGS and wrote files named by raw addresses; ROADMAP A7 refused
    it rather than fixing it, because it has no call site in the tree. -/
def checkMapb (p : Program) : Option Diag :=
  if p.fns.any (fun f => (f.body.flatMap callsInStmt).contains "sys_mapb") then
    some ⟨108, "`sys_mapb` is refused at compile time (ROADMAP A7, exit 108): its emitter passed the path LENGTH as openat's FLAGS"⟩
  else none

-- ============================================================
-- Rule: at most 8 symbols bound at a `sys_clone` (exit 109)
-- ============================================================

/-- ROADMAP A19. neg/c141_clone9 sits at 9 and must be refused; c142_clone8
    sits at exactly 8 and must COMPILE -- the two bracket the boundary, so
    neither "refuse every clone" nor "accept every clone" passes both.

    The count is the number of DISTINCT names bound at the spawn point:
    parameters, plus every `let` earlier in the function, plus the pending
    binder of the `let r = sys_clone(...)` the call is the right-hand side of.
    c142 counts `base a1 a2 a3 a4 a5 a6` = 7 and `r` = 8; c141 counts
    `h1..h7 base` = 8 and `r` = 9. That the `a*` in c142 are literal constants
    and the `h*` in c141 are array handles makes no difference to the count --
    both constructs' own headers count them the same way. -/
partial def cloneDepth (params : Array Name) (body : Array Stmt) : Option Nat :=
  let rec go (i : Nat) (bound : Array Name) : Option Nat :=
    if h : i < body.size then
      let st := body[i]
      -- names bound BEFORE this statement
      let pend : Nat := match st with
        | .let_ n _ => if bound.contains n then 0 else 1
        | _ => 0
      let hasClone := (callsInStmt st).contains "sys_clone"
      if hasClone then some (bound.size + pend)
      else
        let bound := bound ++ (bindsInStmt st).filter (fun n => !bound.contains n)
        -- dedup
        let bound := bound.foldl (fun acc n => if acc.contains n then acc else acc.push n) #[]
        go (i + 1) bound
    else none
  go 0 (params.foldl (fun acc n => if acc.contains n then acc else acc.push n) #[])

def checkCloneSymbols (p : Program) : Option Diag :=
  let bad := p.fns.filterMap (fun f =>
    match cloneDepth f.params f.body with
    | some n => if n > 8 then some (f.name, n) else none
    | none => none)
  match bad[0]? with
  | some (fn, n) => some ⟨109, s!"`sys_clone` in fn `{fn}` has {n} symbols bound at the spawn point; the limit is 8 (ROADMAP A19, exit 109)"⟩
  | none => none

-- ============================================================
-- Rule: unbound symbol (exit 101)
-- ============================================================

/-- neg/c93_unbound (`let _ = v0 = 0 in 0` -- `v0` is ASSIGNED and never bound)
    and neg/read_before_assign (`let abcdefg = 5; let _ = abcdefh in abcdefh`
    -- a different spelling with a colliding hash, read without ever being
    bound). Both are exit 101.

    The scope is the whole FUNCTION and it is flow-INSENSITIVE, on purpose:
    LANGUAGE.md:53-55 says reading a symbol whose `let` has not executed yet is
    UNDEFINED, not an error, so a name bound anywhere in the fn -- including
    inside a `while` body that may run zero times -- counts as bound. Only a
    name bound NOWHERE is refused. That is the weaker of the two possible rules
    and it is the one that keeps every positive construct green. -/
def checkUnbound (p : Program) : Option Diag :=
  let globals : Array Name :=
    p.fns.map (·.name) ++ p.structs.map (·.name)
      ++ p.enums.flatMap (fun e => e.ctors.map (·.name))
      ++ builtinSurface
  let bad := p.fns.filterMap (fun f =>
    let bound := f.params ++ f.body.flatMap bindsInStmt
    match (f.body.flatMap readsInStmt).find? (fun n =>
        n != "_" && !bound.contains n && !globals.contains n) with
    | some n => some (f.name, n)
    | none => none)
  match bad[0]? with
  | some (fn, n) => some ⟨101, s!"`{n}` is read in fn `{fn}` but no `let` in that fn ever binds it (exit 101, unbound symbol)"⟩
  | none => none

-- ============================================================
-- Rule: a LITERAL index at or past a statically declared length (exit 65)
-- ============================================================

/-- ROADMAP F3, neg/c120_oobstatic: `let a = [1, 2, 3]; a[3]`.

    MIRRORS `bebop.bp:5071 emit_array_index`'s F3 block, which is three lines:
        let bidx = lit_upto(s, pos[0], strn, 93);            -- a LITERAL index
        let blen = if bkey >= 0 then fntab[5410 + bkey] else 0;
        let bbad = (bidx >= 0) * (blen > 0) * (bidx >= blen - 1);
    `fntab[5410 + k]` is "length PLUS ONE (0 = unknown)" (bebop.bp:4994) and it
    is written on every `let` from `rhs_len_p1`, which recognises exactly two
    right-hand sides: `zeros(<literal>)` and an `[...]` array literal. So the
    rule is: a length is known only from those two shapes, a later `let` to
    anything else makes it unknown again, and the refusal fires only when BOTH
    operands are literal. Everything dynamic -- a computed index, a handle
    passed as a parameter, `a[i]` in a loop -- is untouched, which is what
    LANGUAGE.md:129 means when it says the language has no bounds checks. -/
def staticLen : Expr → Option Nat
  | .arrLit es => some es.size
  | .paren e => staticLen e
  | .call "zeros" args =>
      match (args[0]? : Option Expr) with
      | some (Expr.lit v) => if v.toInt < 0 then some 0 else some v.toNatClampNeg
      | _ => none
  | _ => none

/-- A literal index, if the index expression is one. `-3` is `unop neg (lit 3)`,
    which is negative and therefore also out of range. -/
def litIndex : Expr → Option Int
  | .lit v => some v.toInt
  | .paren e => litIndex e
  | .unop .neg e => (litIndex e).map (fun k => -k)
  | _ => none

abbrev LenEnv := Array (Name × Nat)

def lenGet (env : LenEnv) (n : Name) : Option Nat :=
  (env.find? (fun p => p.1 == n)).map (·.2)

def lenSet (env : LenEnv) (n : Name) (l : Option Nat) : LenEnv :=
  let env := env.filter (fun p => p.1 != n)
  match l with
  | some v => env.push (n, v)
  | none => env

/-- Check one expression for an out-of-range literal index, under the lengths
    known at this point. -/
partial def oobInExpr (env : LenEnv) : Expr → Option Diag
  | .lit _ | .var _ | .strLit _ | .brkExpr => none
  | .paren e | .unop _ e | .retExpr e => oobInExpr env e
  | .binop _ l r => (oobInExpr env l).orElse (fun _ => oobInExpr env r)
  | .arrLit es => es.findSome? (oobInExpr env)
  | .arrGet a i =>
      match (oobInExpr env a).orElse (fun _ => oobInExpr env i) with
      | some d => some d
      | none =>
        match a, litIndex i with
        | .var n, some k =>
          match lenGet env n with
          | some len => if k ≥ (len : Int) || k < 0 then
                          some ⟨65, s!"`{n}[{k}]` indexes a statically {len}-cell allocation (ROADMAP F3, exit 65: index past the end)"⟩
                        else none
          | none => none
        | _, _ => none
  | .arrSet a i v =>
      (oobInExpr env (.arrGet a i)).orElse (fun _ => oobInExpr env v)
  | .call _ as => as.findSome? (oobInExpr env)
  | .builtin _ as => as.findSome? (oobInExpr env)
  | .ite c t f => (oobInExpr env c).orElse (fun _ =>
                   (oobInExpr env t).orElse (fun _ => oobInExpr env f))
  | .letIn n e b =>
      match oobInExpr env e with
      | some d => some d
      | none => oobInExpr (lenSet env n (staticLen e)) b
  | .assignIn n e b =>
      match oobInExpr env e with
      | some d => some d
      | none => oobInExpr (lenSet env n (staticLen e)) b
  | .matchExpr sc arms =>
      (oobInExpr env sc).orElse (fun _ => arms.findSome? (fun a => oobInExpr env a.body))
  | .structLit _ fs => fs.findSome? (fun p => oobInExpr env p.2)
  | .enumLit _ a => match a with | some e => oobInExpr env e | none => none
  | .fieldAcc e _ => oobInExpr env e

/-- Walk a statement sequence in TEXTUAL order, threading the length table, as
    the compiler's single pass does. -/
partial def oobInStmts (env0 : LenEnv) (stmts : Array Stmt) : Option Diag × LenEnv :=
  stmts.foldl (fun (acc : Option Diag × LenEnv) st =>
    let (d, env) := acc
    if d.isSome then acc
    else
      match st with
      | .let_ n e | .assign n e =>
          (oobInExpr env e, lenSet env n (staticLen e))
      | .drop e | .ret e | .exprStmt e => (oobInExpr env e, env)
      | .compound n _ e => (oobInExpr env e, lenSet env n none)
      | .arrStore a i v =>
          ((oobInExpr env (.arrGet a i)).orElse (fun _ => oobInExpr env v), env)
      | .while_ c b =>
          match oobInExpr env c with
          | some d => (some d, env)
          | none => let (d, env') := oobInStmts env b; (d, env')
      | .brk => (none, env))
    (none, env0)

def checkStaticOob (p : Program) : Option Diag :=
  p.fns.findSome? (fun f => (oobInStmts #[] f.body).1)

-- ============================================================
-- The pass
-- ============================================================

/-- Run every static check, in the order the compiler would reach them: name
    and signature first, then the body rules. The FIRST refusal wins, and the
    order is fixed so that a construct's exit code is reproducible. -/
def check (p : Program) : Option Diag :=
  (checkReservedFnName p).orElse (fun _ =>
  (checkParamCount p).orElse (fun _ =>
  (checkKernelSyscall p).orElse (fun _ =>
  (checkMapb p).orElse (fun _ =>
  (checkCloneSymbols p).orElse (fun _ =>
  (checkUnbound p).orElse (fun _ =>
   checkStaticOob p))))))

end Bebop.Reject
