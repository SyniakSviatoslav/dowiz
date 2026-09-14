/-
  Bebop.Parser -- a recursive-descent parser from `.bp` text to the AST in
  Bebop.Basic, so that Conformance can read `bench/parity_constructs/*.bp`
  instead of carrying transcribed copies of them.

  WHY THIS EXISTS. Until 2026-09-14 every program in Conformance.lean was a
  hand-typed string, and 88 of 97 of them were not the file they named. A
  parser removes the transcription step, so that whole class of defect becomes
  structurally impossible: there is nothing left to mistype.

  THE TWO FAILURE KINDS ARE KEPT APART, deliberately. `ParseError.unsupported`
  says "this parser does not handle this construct YET" -- a statement about
  this file. `ParseError.invalid` says "this text is not a legal program" -- a
  statement about the input. Collapsing them is how a harness ends up scoring
  its own gaps as the language's, and it is the same shape as `checkExpected`
  accepting any trap for a `none` expectation. Neither is ever silent: there is
  no branch in this file that skips input it does not understand.

  -- §0. PRECEDENCE, MEASURED 2026-09-14 ------------------------------------
  LANGUAGE.md:57-67 lists the ladder but does NOT mention `&&` or `||` at all,
  so their level had to be measured. Every row below was run through the
  promoted `bebop.bin` (sha256 3af32503b7f99a393a0fc76f1cdfd8ad34d32fd6cd81cfbd785fd7e238ac673d):

    loosest   ||        `1 || 0 && 0`      = 1     so `&&` binds tighter
              &&        `2 == 1 || 1`      = 1     so `||` is looser than `==`
                        `0 == 1 && 0`      = 0     so `&&` is looser than `==`
                        `1 | 0 && 0`       = 0     so `&&` is looser than `|`
                        `0 && 0 | 1`       = 0     (same, other side)
              == != < > <= >=              `1 < 2 < 3` = 1, chains left to right
              |         `1 | 0 == 0`       = 0     so `==` is LOOSER than `|`
              ^                                    -- NOT C: in C, `|` is looser
              &                                       than `==`. LANGUAGE.md:59-62
              << >> >>>                               is right and the "C precedence"
              + -       `1 >> 1 + 1`       = 0     remark in the doc is only
    tightest  * / %                                 partly true.
              unary - ! `!0 * 10 + !5`     = 10

  So `||` and `&&` sit ABOVE comparison exactly as in C, while comparison sits
  above `|`/`^`/`&` exactly as it does NOT in C. Both halves are encoded below.

  -- §1. THE TWO OPERATORS THAT ARE EASY TO MODEL BACKWARDS ------------------
  LANGUAGE.md:64: "`>>` is LOGICAL (lsrv), `>>>` is ARITHMETIC (asrv)". This is
  the opposite of Java/JavaScript. Measured, same binary:
      `(0 - 16) >> 4`  -> 1152921504606846975      (logical: zero-fill)
      `(0 - 16) >>> 4` -> -1                       (arithmetic: sign-fill)
  Hence `">>" => lsr` and `">>>" => asr` in `binTable`, and a
  #guard at the bottom of this file pins both so the mapping cannot be quietly
  swapped.

  -- §2. `&&` AND `||`: VALUE IS LOGICAL, EVALUATION IS NOT SHORT-CIRCUIT ----
  Measured the same day, and the two halves disagree with each other, so both
  must be modelled:
      `(2 && 1)*1000 + (2 || 1)*100 + (0 || 7)*10 + (0 && 7)` = 1110
        -- bitwise would give 370 (2&1=0, 2|1=3, 0|7=7). So the VALUE is
           logical: 0 or 1, not the bit pattern.
      `fn bump(a: i64) -> i64 { let _ = a[0] = 7; 1 }`
      `let a = zeros(1); let r = 0 && bump(a); a[0] * 10 + r`  = 70
        -- a[0] is 7, so `bump` RAN even though the left operand was 0.
           `1 || bump(a)` likewise gives 71. So evaluation is NOT
           short-circuit (T125), while the value IS logical.
  Bebop.Semantics models exactly that pair. This supersedes the older note in
  this tree that `&&` is a constant zero: that was measured on an earlier
  compiler and is false for the promoted binary.
-/

import Bebop.Basic
import Bebop.Lexer

namespace Bebop.Parser

open Bebop.Lexer

/-- A parse outcome that went wrong. The first two constructors are the whole
    point of this type and must never be merged. -/
inductive ParseError where
  /-- The lexer could not tokenize the text at all. -/
  | lex (e : LexError)
  /-- A construct this parser does not handle yet. NOT a claim that the program
      is wrong; a claim that this file is incomplete. `feature` is the name to
      count in the score table. -/
  | unsupported (feature : String) (line col : Nat)
  /-- The text is not a legal program under the grammar in LANGUAGE.md. This IS
      a claim about the input -- which for `neg/*.bp` is the expected outcome.
      No compiler exit code is attached: the refusal is a GRAMMAR refusal and
      this parser does not know which of `bebop.bin`'s codes it would carry. -/
  | invalid (msg : String) (line col : Nat)
  /-- The text is refused by a rule that DOES name a compiler exit code -- the
      tail-expression rule (97), and every rule in `Bebop.Reject`. Kept apart
      from `.invalid` so `ParityRun` can ASSERT the code against a negative
      construct's `COMPILEFAIL:<code>` header instead of accepting any refusal
      for any code. A refusal with the wrong code is a FAILURE, loudly. -/
  | refused (code : Nat) (msg : String) (line col : Nat)
  /-- The step budget ran out. Loud on purpose: it means the parser is looping
      or the input is pathological, and it must never be confused with either
      of the above. -/
  | outOfFuel (line col : Nat)
  deriving Inhabited

def ParseError.render : ParseError → String
  | .lex e => s!"LEX {e.render}"
  | .unsupported f l c => s!"UNSUPPORTED {l}:{c}: {f}"
  | .invalid m l c => s!"INVALID {l}:{c}: {m}"
  | .refused n m l c => s!"REJECTED {n} {l}:{c}: {m}"
  | .outOfFuel l c => s!"OUT-OF-FUEL near {l}:{c}"

/-- The bucket name used by the score table. Keeps "we cannot parse it" and
    "it is not a program" in separate columns. -/
def ParseError.kind : ParseError → String
  | .lex _ => "lex"
  | .unsupported _ _ _ => "unsupported"
  | .invalid _ _ _ => "invalid"
  | .refused _ _ _ _ => "rejected"
  | .outOfFuel _ _ => "out-of-fuel"

/-- Everything the expression parser needs to know that it cannot see locally.
    `structNames` and `enumCtors` come from a pre-scan of the token array, which
    is what lets `P { x: 1 }` be told from `while c { ... }` without a
    context flag, and `none` be told from an ordinary variable. -/
structure Ctx where
  ts : Array Token
  structNames : Array String
  enumCtors : Array String

def eofTok : Token := ⟨.eof, 0, 0⟩

def Ctx.at (c : Ctx) (i : Nat) : Token := c.ts.getD i eofTok
def Ctx.tok (c : Ctx) (i : Nat) : Tok := (c.at i).tok
def Ctx.line (c : Ctx) (i : Nat) : Nat := (c.at i).line
def Ctx.col (c : Ctx) (i : Nat) : Nat := (c.at i).col

def Ctx.isP (c : Ctx) (i : Nat) (s : String) : Bool := c.tok i == Tok.punct s
def Ctx.isI (c : Ctx) (i : Nat) (s : String) : Bool := c.tok i == Tok.ident s

def Ctx.err (c : Ctx) (i : Nat) (msg : String) : ParseError :=
  .invalid msg (c.line i) (c.col i)
def Ctx.unsup (c : Ctx) (i : Nat) (f : String) : ParseError :=
  .unsupported f (c.line i) (c.col i)

def Ctx.expectP (c : Ctx) (i : Nat) (s : String) : Except ParseError Nat :=
  if c.isP i s then .ok (i + 1)
  else .error (c.err i s!"expected '{s}', found {(c.tok i).render}")

def Ctx.expectIdent (c : Ctx) (i : Nat) : Except ParseError (String × Nat) :=
  match c.tok i with
  | .ident s => .ok (s, i + 1)
  | t => .error (c.err i s!"expected an identifier, found {t.render}")

/-- Reserved words that may never be used as a plain identifier in expression
    position. Listed so that e.g. `then` appearing where a variable is expected
    is reported as a grammar error at the right place, not silently bound. -/
def keywords : List String :=
  ["fn", "enum", "struct", "module", "use", "let", "in", "while",
   "return", "break", "if", "then", "else", "match"]

/-- Binary operator table, LOOSEST LEVEL FIRST. See §0 and §1 above; every row
    is measured, none is inherited from another language. -/
def binTable : Array (Array (String × BinOp)) := #[
  #[("||", BinOp.lor)],
  #[("&&", BinOp.land)],
  #[("==", BinOp.eq), ("!=", BinOp.neq),
    ("<", BinOp.slt), (">", BinOp.sgt), ("<=", BinOp.sle), (">=", BinOp.sge)],
  #[("|", BinOp.bor)],
  #[("^", BinOp.bxor)],
  #[("&", BinOp.band)],
  -- `>>` is the LOGICAL shift and `>>>` the ARITHMETIC one (LANGUAGE.md:64,
  -- measured). Do not "fix" this to match Java.
  #[("<<", BinOp.lsl), (">>", BinOp.lsr), (">>>", BinOp.asr)],
  #[("+", BinOp.add), ("-", BinOp.sub)],
  #[("*", BinOp.mul), ("/", BinOp.sdiv), ("%", BinOp.srem)]
]

def maxLevel : Nat := binTable.size

private def opAt (lvl : Nat) (t : Tok) : Option BinOp :=
  match binTable[lvl]? with
  | none => none
  | some row =>
    match t with
    | .punct s => (row.find? (fun p => p.1 == s)).map (·.2)
    | _ => none

/-- Compound-assignment spellings (LANGUAGE.md:45). -/
def compoundOps : List (String × BinOp) :=
  [("+=", BinOp.add), ("-=", BinOp.sub), ("*=", BinOp.mul),
   ("/=", BinOp.sdiv), ("%=", BinOp.srem)]

-- ============================================================
-- Pre-scan: struct names and enum constructor names.
--
-- MEASURED 2026-09-14: the corpus declares `enum opt { none, some }` and then
-- writes `some(5)`, i.e. a payload constructor whose declaration carries NO
-- `(TYPE)`. So arity CANNOT be read off the declaration, and the parser
-- decides by use site: `ctor(arg)` is a payload constructor, bare `ctor` is a
-- nullary one. The `hasPayload` field of EnumCtor is filled from the
-- declaration where it is present (it only feeds `enumArities`, which the
-- match rule does not consult -- `enumTags` is what matters, and that is
-- built from ctor ORDER).
-- ============================================================

private def scanDecls (ts : Array Token) : Array String × Array String :=
  let rec go (n : Nat) (i : Nat) (ss cs : Array String) : Array String × Array String :=
   match n with
   | 0 => (ss, cs)
   | n + 1 =>
    if i + 1 < ts.size then
      match ts[i]!.tok, ts[i+1]!.tok with
      | .ident "struct", .ident nm => go n (i + 2) (ss.push nm) cs
      | .ident "enum", .ident _ =>
        -- collect ctor names until the matching '}'
        let rec ctors (m : Nat) (j : Nat) (acc : Array String) : Array String × Nat :=
          match m with
          | 0 => (acc, j)
          | m + 1 =>
            if j < ts.size then
              match ts[j]!.tok with
              | .punct "}" => (acc, j + 1)
              | .ident nm =>
                -- skip an optional (TYPE)
                if j + 1 < ts.size && ts[j+1]!.tok == Tok.punct "(" then
                  ctors m (j + 4) (acc.push nm)   -- ident ( TYPE )
                else ctors m (j + 1) (acc.push nm)
              | _ => ctors m (j + 1) acc
            else (acc, j)
        if i + 2 < ts.size && ts[i+2]!.tok == Tok.punct "{" then
          let (cn, k) := ctors ts.size (i + 3) #[]
          go n k ss (cs ++ cn)
        else go n (i + 2) ss cs
      | _, _ => go n (i + 1) ss cs
    else (ss, cs)
  go (ts.size + 1) 0 #[] #[]

-- ============================================================
-- Expressions
--
-- Every function below is structurally recursive on `fuel`, and `fuel` starts
-- at `tokens.size + 8`. That is why the parser cannot hang the harness on a
-- malformed file: it runs out of fuel and says so, in its own bucket. The
-- lexer can afford `partial` (each branch consumes a character); the parser
-- cannot, because a mis-written loop here would spin without consuming.
-- ============================================================

mutual

/-- An expression that may be an ARRAY STORE: `ARR[i] = v`.

    MEASURED 2026-09-14, because LANGUAGE.md:44 lists `let _ = ARR[expr] = expr`
    as a STATEMENT and says nothing about assignment in expression position --
    yet `selfhost/prelude/store.bp:188` (reached through `use` by four of the
    constructs) contains
        let _ = if bad > 0 then base[sb] = 0 else 0;
    i.e. a store inside an `if` ARM. Two probes on the promoted bebop.bin
    settled what is actually allowed:

      let a = zeros(2); let _ = if 1 then a[0] = 7 else 0; a[0]   ->  7
        so an ARRAY store IS an expression.

      let x = 0; let _ = if 1 then x = 9 else 0; x
        ->  COMPILE rc=101, "unbound symbol", at the `x` of `x = 9`
        so a VARIABLE assignment is NOT. The compiler refuses it.

    So the left-hand side is restricted to an array element here, and a
    variable on the left is reported `invalid` with that exit code cited --
    matching the compiler rather than generalising past it. The statement form
    `let _ = NAME = e` (c86_selfassign) is a different production and lives in
    `pLet`.

    Right-associative, and looser than every operator in `binTable`. -/
def pAssign (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    match pBin c fuel 0 i with
    | .error e => .error e
    | .ok (lhs, j) =>
      if c.isP j "=" then
        match pAssign c fuel (j + 1) with
        | .error e => .error e
        | .ok (rhs, k) =>
          match lhs with
          | .arrGet arr idx => .ok (Expr.arrSet arr idx rhs, k)
          | .var _ =>
            .error (c.err j "a variable cannot be assigned in expression position (bebop.bin exits 101, `unbound symbol`); use `let _ = NAME = e` as a statement")
          | _ => .error (c.err j "left of `=` must be an array element")
      else .ok (lhs, j)

/-- Precedence-climbing over `binTable`; `lvl` counts from 0 = loosest. -/
def pBin (c : Ctx) (fuel lvl i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if lvl ≥ maxLevel then pUnary c fuel i
    else
      match pBin c fuel (lvl + 1) i with
      | .error e => .error e
      | .ok (lhs, j) => pBinLoop c fuel lvl lhs j

def pBinLoop (c : Ctx) (fuel lvl : Nat) (acc : Expr) (j : Nat)
    : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line j) (c.col j))
  | fuel + 1 =>
    match opAt lvl (c.tok j) with
    | none => .ok (acc, j)
    | some op =>
      match pBin c fuel (lvl + 1) (j + 1) with
      | .error e => .error e
      | .ok (rhs, k) => pBinLoop c fuel lvl (Expr.binop op acc rhs) k

def pUnary (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if c.isP i "-" then
      match pUnary c fuel (i + 1) with
      | .error e => .error e
      | .ok (e, j) => .ok (Expr.unop UnOp.neg e, j)
    else if c.isP i "!" then
      match pUnary c fuel (i + 1) with
      | .error e => .error e
      | .ok (e, j) => .ok (Expr.unop UnOp.lnot e, j)
    else pPostfix c fuel i

def pPostfix (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    match pPrimary c fuel i with
    | .error e => .error e
    | .ok (e, j) => pPostfixLoop c fuel e j

def pPostfixLoop (c : Ctx) (fuel : Nat) (acc : Expr) (j : Nat)
    : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line j) (c.col j))
  | fuel + 1 =>
    if c.isP j "[" then
      match pBin c fuel 0 (j + 1) with
      | .error e => .error e
      | .ok (idx, k) =>
        match c.expectP k "]" with
        | .error e => .error e
        | .ok k => pPostfixLoop c fuel (Expr.arrGet acc idx) k
    else if c.isP j "." then
      match c.expectIdent (j + 1) with
      | .error e => .error e
      | .ok (f, k) => pPostfixLoop c fuel (Expr.fieldAcc acc f) k
    else .ok (acc, j)

def pPrimary (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    match c.tok i with
    | .num v => .ok (Expr.lit (Int64.ofInt v), i + 1)
    | .str str =>
        -- Parses; `Bebop.Semantics` then reports it as `stuck`. See the
        -- `Expr.strLit` docstring in Basic.lean for why those are kept apart.
        .ok (Expr.strLit str, i + 1)
    | .punct "(" =>
        match pAssign c fuel (i + 1) with
        | .error e => .error e
        | .ok (e, j) =>
          match c.expectP j ")" with
          | .error er => .error er
          | .ok j => .ok (Expr.paren e, j)
    | .punct "[" =>
        match pArgs c fuel (i + 1) "]" with
        | .error e => .error e
        | .ok (es, j) => .ok (Expr.arrLit es, j)
    | .ident "if" => pIf c fuel i
    | .ident "let" => pLetIn c fuel i
    | .ident "match" =>
        match pBin c fuel 0 (i + 1) with
        | .error e => .error e
        | .ok (scrut, j) =>
          match c.expectP j "{" with
          | .error er => .error er
          | .ok j =>
            match pArms c fuel j with
            | .error er => .error er
            | .ok (arms, j) => .ok (Expr.matchExpr scrut arms, j)
    -- `return e` / `break` IN EXPRESSION POSITION. Real, and undocumented:
    -- `let _ = if n < 2 then return 1 else 0;` and
    -- `let _ = if i * i > 50 then break else 0;` are both
    -- `bench/parity_constructs/c124_condreturn.bp`, and `c70_qdsl.bp` uses the
    -- `return` form 40 times. ROADMAP A18 step 1 (2026-09-14, `788b8dc`,
    -- promoted `c9d826c8`) is what made them expressions. docs/LANGUAGE.md in
    -- THIS tree still lists both under `stmt`, and ONLY there -- read
    -- 2026-09-14, lines 48-49 verbatim:
    --     return expr ;                 -- leave the function with expr (T99)
    --     break ;                       -- leave the innermost while (T99)
    -- The expression grammar at :57-82 does not mention either word. The
    -- compiler moved and the doc did not, so the doc is the stale side; this
    -- production follows the compiler and the gap is REPORTED, not resolved
    -- here (docs/ is not this lane's tree).
    --
    -- The operand is parsed at `pBin ... 0`, which is LOOSER than every
    -- operator and TIGHTER than `else`: `if c then return k else 0` therefore
    -- groups as `if c then (return k) else 0`, which is the only reading that
    -- makes c124's `first_div` return `k` rather than `k else 0`.
    | .ident "return" =>
        (match pBin c fuel 0 (i + 1) with
         | .error e => .error e
         | .ok (e, j) => .ok (Expr.retExpr e, j))
    | .ident "break" => .ok (Expr.brkExpr, i + 1)
    | .ident nm =>
        if keywords.contains nm then
          .error (c.err i s!"`{nm}` is a keyword and cannot start an expression")
        else if c.isP (i + 1) "(" then
          match pArgs c fuel (i + 2) ")" with
          | .error e => .error e
          | .ok (args, j) =>
            if c.enumCtors.contains nm then
              match args[0]? with
              | some a =>
                if args.size == 1 then .ok (Expr.enumLit nm (some a), j)
                else .error (c.err i s!"enum constructor `{nm}` takes one payload, got {args.size}")
              | none => .error (c.err i s!"enum constructor `{nm}` used with no payload")
            else .ok (Expr.call nm args, j)
        else if c.isP (i + 1) "{" && c.structNames.contains nm then
          match pFields c fuel (i + 2) with
          | .error e => .error e
          | .ok (fs, j) => .ok (Expr.structLit nm fs, j)
        else if c.enumCtors.contains nm then
          .ok (Expr.enumLit nm none, i + 1)
        else .ok (Expr.var nm, i + 1)
    | t => .error (c.err i s!"expected an expression, found {t.render}")

/-- `if c then a else b` -- LANGUAGE.md:75. There are NO braces on the arms;
    brace arms are what gave away the fabricated sources in Conformance.lean. -/
def pIf (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    match pBin c fuel 0 (i + 1) with
    | .error e => .error e
    | .ok (cnd, j) =>
      if !(c.isI j "then") then
        .error (c.err j "expected `then` (LANGUAGE.md:75 -- `if c then a else b`, no braces)")
      else
        -- ARMS use `pAssign`: `if bad > 0 then base[sb] = 0 else 0` is real
        -- (store.bp:188) and measured to work. See `pAssign`.
        match pAssign c fuel (j + 1) with
        | .error e => .error e
        | .ok (t, j) =>
          if !(c.isI j "else") then
            .error (c.err j "expected `else` (both arms are required)")
          else
            match pAssign c fuel (j + 1) with
            | .error e => .error e
            | .ok (f, j) => .ok (Expr.ite cnd t f, j)

/-- `let NAME = e in body`; `;` is a documented synonym for `in`
    (LANGUAGE.md:76, measured: `(let y = 3; y)` = 3). -/
def pLetIn (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    match c.expectIdent (i + 1) with
    | .error e => .error e
    | .ok (n, j) =>
      match c.expectP j "=" with
      | .error e => .error e
      | .ok j =>
        match pBin c fuel 0 j with
        | .error e => .error e
        | .ok (v, j) =>
          if c.isP j "=" then
            -- `let _ = <lhs> = <rhs> in body`. Real: c93_unbound is
            -- `let _ = v0 = 0 in 0`, c90_symalias and c95_symspan are the same
            -- shape. DESUGARED rather than refused, and the desugaring is
            -- exact, not approximate:
            --   `let _ = x = rhs in body`      ==>  Expr.letIn x rhs body
            --      both bind `x` to `rhs` and then evaluate `body`, and `let`
            --      is fn-scoped with no shadowing (LANGUAGE.md:41-43), which is
            --      exactly what `State.bind` does.
            --   `let _ = a[i] = rhs in body`   ==>  Expr.letIn "_" (arrSet a i rhs) body
            --      `Expr.arrSet` already exists and `evalExpr` has an arm for it.
            match pBin c fuel 0 (j + 1) with
            | .error e => .error e
            | .ok (rhs, k) =>
              if c.isI k "in" || c.isP k ";" then
                match pBin c fuel 0 (k + 1) with
                | .error e => .error e
                | .ok (b, m) =>
                  match v with
                  -- ASSIGNMENT, not a binding: `Expr.assignIn`. It evaluates
                  -- identically to `letIn`; the distinction is static
                  -- (neg/c93_unbound is exit 101 because `v0` is assigned
                  -- without ever having been bound).
                  | .var tgt => .ok (Expr.assignIn tgt rhs b, m)
                  | .arrGet arr idx => .ok (Expr.letIn "_" (Expr.arrSet arr idx rhs) b, m)
                  | _ => .error (c.err j "left of `=` is not a variable or an array element")
              else .error (c.err k "expected `in` or `;` after `let _ = <lhs> = <rhs>`")
          else if c.isI j "in" || c.isP j ";" then
            match pBin c fuel 0 (j + 1) with
            | .error e => .error e
            | .ok (b, j) => .ok (Expr.letIn n v b, j)
          else .error (c.err j "expected `in` or `;` after a let-in binding")

def pArgs (c : Ctx) (fuel i : Nat) (close : String) : Except ParseError (Array Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if c.isP i close then .ok (#[], i + 1)
    else pArgsLoop c fuel close #[] i

def pArgsLoop (c : Ctx) (fuel : Nat) (close : String) (acc : Array Expr) (j : Nat)
    : Except ParseError (Array Expr × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line j) (c.col j))
  | fuel + 1 =>
    match pBin c fuel 0 j with
    | .error e => .error e
    | .ok (e, j) =>
      let acc := acc.push e
      if c.isP j "," then pArgsLoop c fuel close acc (j + 1)
      else match c.expectP j close with
        | .error er => .error er
        | .ok j => .ok (acc, j)

def pFields (c : Ctx) (fuel i : Nat) : Except ParseError (Array (Name × Expr) × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if c.isP i "}" then .ok (#[], i + 1)
    else pFieldsLoop c fuel #[] i

def pFieldsLoop (c : Ctx) (fuel : Nat) (acc : Array (Name × Expr)) (j : Nat)
    : Except ParseError (Array (Name × Expr) × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line j) (c.col j))
  | fuel + 1 =>
    match c.expectIdent j with
    | .error e => .error e
    | .ok (f, j) =>
      match c.expectP j ":" with
      | .error e => .error e
      | .ok j =>
        match pBin c fuel 0 j with
        | .error e => .error e
        | .ok (v, j) =>
          let acc := acc.push (f, v)
          if c.isP j "," then pFieldsLoop c fuel acc (j + 1)
          else match c.expectP j "}" with
            | .error er => .error er
            | .ok j => .ok (acc, j)

def pArms (c : Ctx) (fuel i : Nat) : Except ParseError (Array MatchArm × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if c.isP i "}" then .ok (#[], i + 1)
    else pArmsLoop c fuel #[] i

def pArmsLoop (c : Ctx) (fuel : Nat) (acc : Array MatchArm) (j : Nat)
    : Except ParseError (Array MatchArm × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line j) (c.col j))
  | fuel + 1 =>
    match c.expectIdent j with
    | .error e => .error e
    | .ok (ctor, j) =>
      let bnd : Except ParseError (Option Name × Nat) :=
        if c.isP j "(" then
          match c.expectIdent (j + 1) with
          | .error e => Except.error e
          | .ok (b, k) =>
            match c.expectP k ")" with
            | .error e => Except.error e
            | .ok k => Except.ok (some b, k)
        else Except.ok (none, j)
      match bnd with
      | .error e => .error e
      | .ok (binder, j) =>
        match c.expectP j "=>" with
        | .error _ => .error (c.err j "expected `=>` in a match arm")
        | .ok j =>
          match pAssign c fuel j with
          | .error e => .error e
          | .ok (bdy, j) =>
            let acc := acc.push { ctor := ctor, binder := binder, body := bdy }
            if c.isP j "," then pArmsLoop c fuel acc (j + 1)
            else match c.expectP j "}" with
              | .error er => .error er
              | .ok j => .ok (acc, j)

-- ------------------------------------------------------------
-- Statements and bodies (LANGUAGE.md:38-56)
-- ------------------------------------------------------------

/-- One item of a `{ ... }` body: a statement, or an expression (which becomes
    `Stmt.exprStmt` -- and, if it is the LAST item, the body's tail
    expression, which is how `Bebop.Semantics.execBody` reads it). -/
def pItem (c : Ctx) (fuel i : Nat) : Except ParseError (Stmt × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if c.isI i "let" then pLet c fuel i
    else if c.isI i "while" then
      match pBin c fuel 0 (i + 1) with
      | .error e => .error e
      | .ok (cnd, j) =>
        match c.expectP j "{" with
        | .error er => .error er
        | .ok j =>
          match pBody c fuel j with
          | .error er => .error er
          | .ok (b, j) => .ok (Stmt.while_ cnd b, j)
    else if c.isI i "return" then
      match pBin c fuel 0 (i + 1) with
      | .error e => .error e
      | .ok (e, j) => .ok (Stmt.ret e, j)
    else if c.isI i "break" then .ok (Stmt.brk, i + 1)
    else
      -- `NAME += e` and friends, else a plain expression statement.
      match c.tok i, c.tok (i + 1) with
      | .ident n, .punct p =>
        match compoundOps.find? (fun q => q.1 == p) with
        | some (_, op) =>
          match pBin c fuel 0 (i + 2) with
          | .error e => .error e
          | .ok (e, j) => .ok (Stmt.compound n op e, j)
        | none =>
          match pAssign c fuel i with
          | .error e => .error e
          | .ok (e, j) => .ok (Stmt.exprStmt e, j)
      | _, _ =>
        -- `pAssign`, not `pBin`: a bare `ARR[i] = v` is a legal item and even a
        -- legal TAIL expression -- `selfhost/prelude/gb.bp` ends a function with
        -- `cache[0] = cnt + 1`. `pAssign` still refuses a VARIABLE on the left
        -- (bebop.bin exits 101 for that in expression position), so widening
        -- here does not widen to the bare `x = 5;` form.
        match pAssign c fuel i with
        | .error e => .error e
        | .ok (e, j) => .ok (Stmt.exprStmt e, j)

/-- `let NAME = e`, `let _ = e`, `let _ = ARR[i] = v`, `let _ = NAME = v`.

    The last two are real and both appear in the corpus:
    `c86_selfassign` is `let _ = x = x + 10;` (a REBIND through `let _`) and
    `c13_array`-style code is `let _ = a[2] = 99;` (an array store). The
    disambiguation is entirely local: parse the right-hand expression, then
    look for a following `=`. `==` is a different token, so there is no
    ambiguity with a comparison. -/
def pLet (c : Ctx) (fuel i : Nat) : Except ParseError (Stmt × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    match c.expectIdent (i + 1) with
    | .error e => .error e
    | .ok (n, j) =>
      match c.expectP j "=" with
      | .error e => .error e
      | .ok j =>
        match pBin c fuel 0 j with
        | .error e => .error e
        | .ok (v, j) =>
          if c.isP j "=" then
            -- `let <n> = <v> = <rhs>` : an assignment through a binder.
            match pBin c fuel 0 (j + 1) with
            | .error e => .error e
            | .ok (rhs, k) =>
              if c.isI k "in" then
                -- `let _ = x = rhs in body` in STATEMENT position: the whole
                -- thing is one expression (and, if last, the tail). Same exact
                -- desugaring as in `pLetIn`.
                match pBin c fuel 0 (k + 1) with
                | .error e => .error e
                | .ok (b, m) =>
                  match v with
                  | .var tgt => .ok (Stmt.exprStmt (Expr.assignIn tgt rhs b), m)
                  | .arrGet arr idx =>
                      .ok (Stmt.exprStmt (Expr.letIn "_" (Expr.arrSet arr idx rhs) b), m)
                  | _ => .error (c.err j "left of `=` is not a variable or an array element")
              else
                match v with
                | .var tgt => .ok (Stmt.assign tgt rhs, k)
                | .arrGet arr idx => .ok (Stmt.arrStore arr idx rhs, k)
                | _ => .error (c.err j "left of `=` is not a variable or an array element")
          else if c.isI j "in" || (c.isP j ";" && false) then
            -- a let-IN in statement position: the whole thing is the tail expr
            match pBin c fuel 0 (j + 1) with
            | .error e => .error e
            | .ok (b, k) => .ok (Stmt.exprStmt (Expr.letIn n v b), k)
          else if n == "_" then .ok (Stmt.drop v, j)
          else .ok (Stmt.let_ n v, j)

/-- A `{ ... }` body: items separated by `;`, with an optional trailing `;`.
    Returns the items; whether the LAST one must be an expression is the
    caller's rule (a `fn` body without a tail expression is exit 97, a `while`
    body's tail is discarded -- LANGUAGE.md:31-35, 48). -/
def pBody (c : Ctx) (fuel i : Nat) : Except ParseError (Array Stmt × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
    if c.isP i "}" then .ok (#[], i + 1)
    else pBodyLoop c fuel #[] i

def pBodyLoop (c : Ctx) (fuel : Nat) (acc : Array Stmt) (j : Nat)
    : Except ParseError (Array Stmt × Nat) :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line j) (c.col j))
  | fuel + 1 =>
    match pItem c fuel j with
    | .error e => .error e
    | .ok (st, j) =>
      let acc := acc.push st
      if c.isP j ";" then
        if c.isP (j + 1) "}" then .ok (acc, j + 2)
        else pBodyLoop c fuel acc (j + 1)
      else match c.expectP j "}" with
        | .error _ => .error (c.err j s!"expected `;` or `}}` in a body, found {(c.tok j).render}")
        | .ok j => .ok (acc, j)

end

def pExpr (c : Ctx) (fuel i : Nat) : Except ParseError (Expr × Nat) := pAssign c fuel i

-- ============================================================
-- Declarations and the whole program
-- ============================================================

/-- `i64 | str | [ i64 ] | NAME`. The parser records the type; NOTHING reads
    it. `Bebop.Semantics` never inspects `Ty` (`grep -n 'Ty\.' Semantics.lean`
    is empty), so `formal/` still has no type checker and every F1 static
    rejection that is a type error is unmodelled. Recorded here so the gap is
    visible at the place a reader would expect the check to be. -/
def pType (c : Ctx) (i : Nat) : Except ParseError (Ty × Nat) :=
  match c.tok i with
  | .ident "i64" => .ok (Ty.i64, i + 1)
  | .ident "str" => .ok (Ty.str, i + 1)
  | .ident "ref" =>
      (match c.expectIdent (i + 1) with
       | .error e => .error e
       | .ok (_, j) => .ok (Ty.ref_t, j))
  | .ident _ => .ok (Ty.named, i + 1)
  | .punct "[" =>
      (match c.tok (i + 1) with
       | .ident "i64" =>
         (match c.expectP (i + 2) "]" with
          | .error e => .error e
          | .ok j => .ok (Ty.arr, j))
       | t => .error (c.err (i + 1) s!"expected `i64` inside `[ ]`, found {t.render}"))
  | t => .error (c.err i s!"expected a type, found {t.render}")

/-- `fn NAME ( NAME : TYPE , ... ) -> TYPE { body }` -/
def pFn (c : Ctx) (fuel i : Nat) : Except ParseError (FnDecl × Nat) := do
  let (nm, i) ← c.expectIdent (i + 1)
  let i ← c.expectP i "("
  let rec params (n : Nat) (j : Nat) (ns : Array Name) (ts : Array Ty)
      : Except ParseError (Array Name × Array Ty × Nat) :=
    match n with
    | 0 => .error (.outOfFuel (c.line j) (c.col j))
    | n + 1 =>
      if c.isP j ")" then .ok (ns, ts, j + 1)
      else do
        let (pn, j) ← c.expectIdent j
        let j ← c.expectP j ":"
        let (pt, j) ← pType c j
        if c.isP j "," then params n (j + 1) (ns.push pn) (ts.push pt)
        else do
          let j ← c.expectP j ")"
          .ok (ns.push pn, ts.push pt, j)
  let (ns, tys, i) ← params fuel i #[] #[]
  let i ← c.expectP i "->"
  let (rt, i) ← pType c i
  -- CONTRACT CLAUSES. `fn add(x: i64, y: i64) -> i64 requires true ensures
  -- result > x { x + y }` is a real construct (c144_contract_ok,
  -- c145_fraction_ok) and LANGUAGE.md does NOT mention `requires` or `ensures`
  -- anywhere -- another doc/compiler disagreement, recorded not resolved.
  -- They are parsed and DISCARDED: nothing in `formal/` checks a contract, so
  -- keeping the term would imply a check that does not exist.
  let rec contracts (j : Nat) (n : Nat) : Except ParseError Nat :=
    match n with
    | 0 => .error (.outOfFuel (c.line j) (c.col j))
    | n + 1 =>
      if c.isI j "requires" || c.isI j "ensures" then
        match pExpr c fuel (j + 1) with
        | .error e => .error e
        | .ok (_, k) => contracts k n
      else .ok j
  let i ← contracts i 8
  let i ← c.expectP i "{"
  let (body, i) ← pBody c fuel i
  -- LANGUAGE.md:31-34: a fn body must end in ONE tail expression; a body that
  -- does not is a COMPILE-TIME error (exit 97). That is a property of the
  -- input, so it is `invalid`, not `unsupported`.
  match body.back? with
  | some (.exprStmt _) => .ok ({ name := nm, params := ns, paramTypes := tys,
                                 returnType := rt, body := body }, i)
  | some (.ret _) => .ok ({ name := nm, params := ns, paramTypes := tys,
                            returnType := rt, body := body }, i)
  | _ => .error (.refused 97 s!"fn `{nm}` has no tail expression (LANGUAGE.md:31-34, compiler exits 97)"
                   (c.line i) (c.col i))

/-- `enum NAME { CTOR ('(' TYPE ')')? , ... }` -/
def pEnum (c : Ctx) (fuel i : Nat) : Except ParseError (EnumDecl × Nat) := do
  let (nm, i) ← c.expectIdent (i + 1)
  let i ← c.expectP i "{"
  let rec ctors (n : Nat) (j : Nat) (acc : Array EnumCtor)
      : Except ParseError (Array EnumCtor × Nat) :=
    match n with
    | 0 => .error (.outOfFuel (c.line j) (c.col j))
    | n + 1 =>
      if c.isP j "}" then .ok (acc, j + 1)
      else do
        let (cn, j) ← c.expectIdent j
        let (hp, j) ←
          if c.isP j "(" then do
            let (_, j) ← pType c (j + 1)
            let j ← c.expectP j ")"
            pure (true, j)
          else pure (false, j)
        let acc := acc.push { name := cn, hasPayload := hp }
        if c.isP j "," then ctors n (j + 1) acc
        else do
          let j ← c.expectP j "}"
          .ok (acc, j)
  let (cs, i) ← ctors fuel i #[]
  .ok ({ name := nm, ctors := cs }, i)

/-- `struct NAME { NAME : TYPE , ... }` -/
def pStruct (c : Ctx) (fuel i : Nat) : Except ParseError (StructDecl × Nat) := do
  let (nm, i) ← c.expectIdent (i + 1)
  let i ← c.expectP i "{"
  let rec fields (n : Nat) (j : Nat) (acc : Array Name)
      : Except ParseError (Array Name × Nat) :=
    match n with
    | 0 => .error (.outOfFuel (c.line j) (c.col j))
    | n + 1 =>
      if c.isP j "}" then .ok (acc, j + 1)
      else do
        let (fn, j) ← c.expectIdent j
        let j ← c.expectP j ":"
        let (_, j) ← pType c j
        let acc := acc.push fn
        if c.isP j "," then fields n (j + 1) acc
        else do
          let j ← c.expectP j "}"
          .ok (acc, j)
  let (fs, i) ← fields fuel i #[]
  .ok ({ name := nm, fields := fs }, i)

/-- Skip a `{ ... }` block with balanced braces, starting at the `{`. Used for
    `test` blocks, whose contents the compiler does not compile. -/
def skipBalanced (c : Ctx) (fuel i : Nat) : Except ParseError Nat :=
  match c.expectP i "{" with
  | .error e => .error e
  | .ok j =>
    let rec go (k depth n : Nat) : Except ParseError Nat :=
      match n with
      | 0 => .error (.outOfFuel (c.line k) (c.col k))
      | n + 1 =>
        match c.tok k with
        | .eof => .error (c.err k "unterminated `{` block")
        | .punct "{" => go (k + 1) (depth + 1) n
        | .punct "}" => if depth == 0 then .ok (k + 1) else go (k + 1) (depth - 1) n
        | _ => go (k + 1) depth n
    go j 0 fuel

/-- Top level: `use | enum | struct | fn | module`, in any order. -/
def pProgramFrom (c : Ctx) (fuel i : Nat) (p : Program) : Except ParseError Program :=
  match fuel with
  | 0 => .error (.outOfFuel (c.line i) (c.col i))
  | fuel + 1 =>
  match c.tok i with
  | .eof => .ok p
  | .ident "fn" => do
      let (f, j) ← pFn c fuel i
      pProgramFrom c fuel j { p with fns := p.fns.push f }
  | .ident "kernel" =>
      -- `kernel fn f(...) -> i64 { ... }` (c111_kernelfn, and c112_kernelsys
      -- which must be REFUSED for calling a syscall from one). LANGUAGE.md does
      -- not document the modifier at all. It is consumed so the function body
      -- is modelled; the restriction it carries -- no `sys_*` inside -- is a
      -- STATIC CHECK that `formal/` does not implement, which is why
      -- c112_kernelsys shows up as not-refused in the score table instead of
      -- quietly passing.
      if c.isI (i + 1) "fn" then
        (do let (f, j) ← pFn c fuel (i + 1)
            pProgramFrom c fuel j { p with fns := p.fns.push { f with isKernel := true } })
      else .error (c.err i "`kernel` must be followed by `fn`")
  | .ident "test" =>
      -- `test NAME { ... }` is SKIPPED by the compiler (c133_testblock's own
      -- header: "test block with string literal must be skipped during
      -- compile"), so skipping it here is modelling it, not ducking it.
      -- Undocumented in LANGUAGE.md.
      (match c.expectIdent (i + 1) with
       | .error e => .error e
       | .ok (_, j) =>
         match skipBalanced c (fuel) j with
         | .error e => .error e
         | .ok j => pProgramFrom c fuel j p)
  | .ident "enum" => do
      let (e, j) ← pEnum c fuel i
      pProgramFrom c fuel j { p with enums := p.enums.push e }
  | .ident "struct" => do
      let (st, j) ← pStruct c fuel i
      pProgramFrom c fuel j { p with structs := p.structs.push st }
  | .ident "module" => do
      -- LANGUAGE.md:20: `module NAME { }` is INERT. It contributes nothing to
      -- the AST, so it is consumed and dropped -- which is modelling it, not
      -- skipping it: the doc says it means nothing.
      let (_, j) ← c.expectIdent (i + 1)
      let j ← c.expectP j "{"
      let j ← c.expectP j "}"
      pProgramFrom c fuel j p
  | .ident "use" =>
      -- `use` is textual inclusion done BEFORE parsing. The driver
      -- (ParityRun.lean) resolves plain paths and re-runs the parser on the
      -- expanded text; reaching here means an unresolved form, which is
      -- `cas://sha256:...` (LANGUAGE.md:28-31) since that needs `.bcas/`
      -- lookup and a digest check that this parser does not do.
      .error (c.unsup i "`use` that the driver could not expand (cas:// address)")
  | t => .error (c.err i s!"expected a top-level `fn`/`enum`/`struct`/`module`/`use`, found {t.render}")

/-- Parse a whole `.bp` source text.

    FUEL, and why it is `tokens.size * 32 + 256`. The first version used
    `tokens.size + 8` on the reasoning that every function consumes a token
    before recursing. That reasoning was WRONG and the corpus said so
    immediately: `c01_lit`, a one-line program, came back PARSER_FUEL. The
    precedence ladder is 9 levels deep, so `pBin` burns ~10 fuel DESCENDING to
    each atom without consuming anything, and `pPostfix`/`pUnary` add more.
    32 per token covers the ladder with room to spare. Running out now really
    does mean a loop in this file, which is why it keeps its own bucket. -/
def parse (source : String) : Except ParseError Program :=
  match Bebop.Lexer.tokenize source with
  | .error e => .error (.lex e)
  | .ok ts =>
    let (sn, ec) := scanDecls ts
    let c : Ctx := { ts := ts, structNames := sn, enumCtors := ec }
    pProgramFrom c (ts.size * 32 + 256) 0 { enums := #[], structs := #[], fns := #[] }

-- ============================================================
-- Grammar pins. These run at ELABORATION time, so `lake build` FAILS if any
-- measured fact below stops holding. That is the point: every one of them was
-- established by running the promoted `bebop.bin`, and every one of them is a
-- fact a future edit could plausibly get backwards.
--
-- This is possible only because nothing on the `parse` path is `partial`:
-- every function is structurally recursive on fuel. A `partial def` would be
-- opaque here and the pins would have to live in the executable instead.
-- ============================================================

/-- Operator names for the renderer. Written out rather than derived, because
    these strings appear in the pins below and a derived `Repr` could change
    shape under a compiler upgrade and break every pin at once. -/
def binOpName : BinOp → String
  | .add => "add" | .sub => "sub" | .mul => "mul" | .sdiv => "sdiv" | .srem => "srem"
  | .band => "band" | .bor => "bor" | .bxor => "bxor"
  | .lsl => "lsl" | .lsr => "lsr" | .asr => "asr"
  | .eq => "eq" | .neq => "neq" | .slt => "slt" | .sgt => "sgt"
  | .sle => "sle" | .sge => "sge"
  | .lor => "lor" | .land => "land"

def unOpName : UnOp → String
  | .neg => "neg" | .lnot => "lnot"

/-- A structural rendering, so a pin can state the exact tree it expects
    instead of a value that several different trees could produce. -/
partial def sexp : Expr → String
  | .lit v => toString v.toInt
  | .var n => n
  | .paren e => "(" ++ sexp e ++ ")"
  | .binop op l r => s!"({binOpName op} {sexp l} {sexp r})"
  | .unop op e => s!"({unOpName op} {sexp e})"
  | .arrLit es => "[" ++ String.intercalate " " (es.toList.map sexp) ++ "]"
  | .arrGet a i => s!"(get {sexp a} {sexp i})"
  | .arrSet a i v => s!"(set {sexp a} {sexp i} {sexp v})"
  | .call f as => s!"(call {f}" ++ String.join (as.toList.map (fun a => " " ++ sexp a)) ++ ")"
  | .ite c t f => s!"(if {sexp c} {sexp t} {sexp f})"
  | .letIn n e b => s!"(let {n} {sexp e} {sexp b})"
  | .matchExpr sc arms =>
      s!"(match {sexp sc}" ++ String.join (arms.toList.map (fun a =>
        " " ++ a.ctor ++ (match a.binder with | some b => "(" ++ b ++ ")" | none => "")
             ++ "=>" ++ sexp a.body)) ++ ")"
  | .structLit n fs => s!"(struct {n}" ++ String.join (fs.toList.map (fun p => s!" {p.1}={sexp p.2}")) ++ ")"
  | .enumLit n a => match a with
      | some e => s!"(ctor {n} {sexp e})"
      | none => s!"(ctor {n})"
  | .fieldAcc e f => s!"(field {sexp e} {f})"
  | .builtin n as => s!"(builtin {n}" ++ String.join (as.toList.map (fun a => " " ++ sexp a)) ++ ")"
  | .strLit str => "\"" ++ str ++ "\""
  | .retExpr e => s!"(return {sexp e})"
  | .brkExpr => "(break)"
  | .assignIn n e b => s!"(assign {n} {sexp e} {sexp b})"

/-- Parse `fn main() -> i64 { <e> }` and render main's tail expression, or the
    error kind. Used only by the pins below. -/
def pinOf (e : String) : String :=
  match parse ("fn main() -> i64 { " ++ e ++ " }") with
  | .error er => "ERR " ++ er.render
  | .ok p =>
    match p.fns[0]? with
    | none => "NO-MAIN"
    | some f =>
      match f.body.back? with
      | some (.exprStmt ex) => sexp ex
      | _ => "NO-TAIL"

-- §1: `>>` is the LOGICAL shift and `>>>` the ARITHMETIC one (LANGUAGE.md:64).
--     Measured: (0-16) >> 4 = 1152921504606846975, (0-16) >>> 4 = -1.
#guard pinOf "a >> b" == "(lsr a b)"
#guard pinOf "a >>> b" == "(asr a b)"
#guard pinOf "a << b" == "(lsl a b)"

-- §0: `&&` binds tighter than `||`. Measured: `1 || 1 && 0` = 1.
#guard pinOf "1 || 1 && 0" == "(lor 1 (land 1 0))"
-- `||` and `&&` are LOOSER than comparison. Measured: `2 == 1 || 1` = 1 and
-- `0 == 1 && 0` = 0 (the other grouping gives 0 and 1 respectively).
#guard pinOf "2 == 1 || 1" == "(lor (eq 2 1) 1)"
#guard pinOf "0 == 1 && 0" == "(land (eq 0 1) 0)"
-- ... and `&&` is looser than `|`. Measured: `1 | 0 && 0` = 0.
#guard pinOf "1 | 0 && 0" == "(land (bor 1 0) 0)"
-- Comparison is looser than `|` -- which is NOT C. Measured: `1 | 0 == 0` = 0.
#guard pinOf "1 | 0 == 0" == "(eq (bor 1 0) 0)"
-- Additive is tighter than the shifts. Measured: `1 >> 1 + 1` = 0.
#guard pinOf "1 >> 1 + 1" == "(lsr 1 (add 1 1))"
-- Comparison chains left to right. Measured: `1 < 2 < 3` = 1.
#guard pinOf "1 < 2 < 3" == "(slt (slt 1 2) 3)"
-- Multiplicative over additive, the ordinary way. (c02_arith depends on it.)
#guard pinOf "20 + 22 - 2 * 5 / 2 - 7 % 4" ==
  "(sub (sub (add 20 22) (sdiv (mul 2 5) 2)) (srem 7 4))"
-- Unary binds tighter than `*`. Measured: `!0 * 10 + !5` = 10.
#guard pinOf "!0 * 10 + !5" ==
  "(add (mul (lnot 0) 10) (lnot 5))"

-- `if c then a else b` is an EXPRESSION with no braces (LANGUAGE.md:75).
#guard pinOf "if 1 then 5 else 9" == "(if 1 5 9)"
-- An ARRAY STORE is legal in an `if` arm (store.bp:188; measured to give 7).
#guard pinOf "if 1 then a[0] = 7 else 0" == "(if 1 (set a 0 7) 0)"
-- A VARIABLE assignment in expression position is refused, as bebop.bin does
-- (exit 101). The pin checks it is `invalid`, i.e. a claim about the program.
#guard (pinOf "if 1 then x = 9 else 0").startsWith "ERR INVALID"
-- `;` is a synonym for `in` in a let-in (LANGUAGE.md:76; measured `(let y = 3; y)` = 3).
#guard pinOf "(let y = 3; y)" == "((let y 3 y))"
#guard pinOf "(let y = 3 in y)" == "((let y 3 y))"
-- Hex literals, and unary minus on a literal. Measured: `-5 + 0x1f` = 26.
#guard pinOf "-5 + 0x1f" == "(add (neg 5) 31)"
-- A string literal PARSES (it becomes `Expr.strLit`); the semantics then
-- reports it `stuck`. Keeping those apart is the whole design of ParseError.
#guard pinOf "str_len(\"hi\")" == "(call str_len \"hi\")"

/-- Enum constructors are resolved from a pre-scan, and arity comes from the USE
    site because the corpus declares `enum opt { none, some }` and then writes
    `some(5)` with no `(TYPE)` on the declaration. -/
def pinProg (src : String) : String :=
  match parse src with
  | .error er => "ERR " ++ er.render
  | .ok p =>
    match p.fns[0]? with
    | none => "NO-MAIN"
    | some f => match f.body.back? with
      | some (.exprStmt ex) => sexp ex
      | _ => "NO-TAIL"

/- c12_match.bp, verbatim: a payload ctor whose declaration has no `(TYPE)`. -/
#guard pinProg "fn main() -> i64 { match some(5) { none => 0, some(x) => x + 1 } } enum opt { none, some }"
  == "(match (ctor some 5) none=>0 some(x)=>(add x 1))"
/- c11_enum.bp: a bare ctor name is an enum literal, not a variable. -/
#guard pinProg "fn main() -> i64 { match none { none => 5, some(x) => x + 1 } } enum opt { none, some }"
  == "(match (ctor none) none=>5 some(x)=>(add x 1))"
/- A name that is NOT a declared ctor stays a variable. -/
#guard pinProg "fn main() -> i64 { none }" == "none"

/- `kernel fn` parses (the modifier is undocumented but real, c111_kernelfn). -/
#guard (match parse "kernel fn k(x: i64) -> i64 { x } fn main() -> i64 { k(3) }" with
        | .ok p => p.fns.size == 2
        | .error _ => false)
/- A `test` block is skipped, as the compiler skips it (c133_testblock). -/
#guard (match parse "fn main() -> i64 { 42 }\ntest t { let n = str_len(\"x\"); n }" with
        | .ok p => p.fns.size == 1
        | .error _ => false)
/- Contract clauses parse and are discarded (c144_contract_ok). -/
#guard (match parse "fn add(x: i64, y: i64) -> i64 requires true ensures y > x { x + y } fn main() -> i64 { add(1,2) }" with
        | .ok p => p.fns.size == 2
        | .error _ => false)
/- A body with no tail expression is REFUSED with the compiler's own exit code
   97, not merely `invalid`: `ParityRun` compares that 97 against
   `neg/c29_emptybody.bp`'s `COMPILEFAIL:97` header. -/
#guard (match parse "fn main() -> i64 { let x = 1; }" with
        | .error (.refused 97 _ _ _) => true
        | _ => false)
/- `return` and `break` in EXPRESSION position parse (ROADMAP A18 step 1).
   The grouping pin is the point: the operand stops at `else`, so the `then`
   arm is `(return 1)` and not `(return (1 else 0))` -- which would not parse
   at all -- nor `return` swallowing the whole conditional. -/
#guard pinOf "if 1 then return 1 else 0" == "(if 1 (return 1) 0)"
#guard pinOf "if 1 then break else 0" == "(if 1 (break) 0)"
#guard pinOf "if n % k == 0 then return k else 0" == "(if (eq (srem n k) 0) (return k) 0)"

end Bebop.Parser
