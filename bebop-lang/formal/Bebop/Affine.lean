/-
  Bebop.Affine -- W-CFVAL (R-LANG 2026-10-09 §2 item 2): translation validation of R-CF, the
  closed-form rewrite of affine `while` loops (compiler/affine.bp + compiler/affine_gen.bp).

  RESULT: A COUNTEREXAMPLE, NOT A PROOF. The lane card says: if the emitted form is not equal to the
  loop in some case the checker ACCEPTS, that is a compiler bug -- exhibit it and stop. This file
  exhibits it. The arithmetic core (affine forms over Z/2^64, M^N by binary powering, the fold's
  write order, the runtime helper's squaring) is NOT proved here.

  THE DEFECT (side condition, runtime path). R-CF's runtime path (`cf_emit_rt`, taken when the
  counter's start is not a literal) inserts, BEFORE the kept loop,
        let (W..) = cf__pK(W.., R..);
  which READS every symbol the loop touches. It is gated on `pre[11]` -- "every symbol is bound
  before the loop" -- but `cf_prefix` computes "bound" as: SOME identifier token with the same
  spelling occurs in the fn text before the loop and is not followed by `(`. A struct FIELD name
  (`P { x: 4 }`, `p.x`) is such a token. So a loop whose body-local `x` (first `let` inside the body)
  shares its spelling with a field gets a prefix that reads `x` before anything declares it.

  MEASURED on the lane compiler 8d9dec53 (.cfval/probes2/p11_fieldname.bp, W-CFVAL 2026-10-09):
        BEBOP_NO_CF=1  -> compiles rc 0, runs, prints 3998004
        R-CF on        -> compile REFUSED rc 84, "6:32: error[E84]: type error ..."
    and the rewritten text (.use) line is
        let (i, x, s) = cf__p0(i, x, s); while i < 2000 { let x = i * 2; ... };
    Control (.cfval/probes3/p11c_control.bp, the body-local renamed `z`, no field `z`): R-CF refuses
    the loop (z is not "bound"), CF and NO_CF binaries byte-identical, both print 3998004.
  So R-CF turns a VALID program into a compile refusal. It cannot produce a wrong VALUE this way: the
  helper's row for a body-local never reads the old value when it powers (e >= 1024, column zero),
  and when it does not power it returns the state unchanged and the kept loop overwrites x first.

  THE MODEL below is exactly the part of affine.bp that decides this: the cf_prefix "bound" scan
  (transliterated: a pending identifier is marked when the NEXT token is not `(`, the end-of-text
  token included), the runtime gate of cf_site (`rt`), and what cf_emit_rt's prefix reads. The
  compiler's E101 rule ("this name was not declared in this function", function-scoped: declared
  iff an earlier `let NAME` / tuple `let` / parameter) is modelled as `declared`.
  Tokens are cf_tok's, reduced: identifiers are numbered (legend at `p11Prefix`).
-/

namespace Bebop.Affine

/-- cf_tok's token kinds, reduced to what cf_prefix reads. `punct c` is the cell value cf_tok stores
    (c, or c1*256+c2 for a two-character operator); `eof` is kind 0. -/
inductive Tok where
  | ident (n : Nat)
  | num (v : Nat)
  | punct (c : Nat)
  | eof
  deriving DecidableEq, Repr

/-- cf_prefix's `pend`: the identifier token is a SITE symbol (cf_symat) -- else nothing pends. -/
def pendOf (sy : List Nat) : Tok → Option Nat
  | .ident n => if sy.contains n then some n else none
  | _ => none

/-- cf_prefix's marking step: the pending symbol is bound unless THIS token is `(` (a call). -/
def mark (pend : Option Nat) (t : Tok) (acc : List Nat) : List Nat :=
  match pend with
  | some n => if t = .punct 40 then acc else n :: acc
  | none => acc

/-- cf_prefix's scan over the fn text [fnstart, p), ending with the eof token that cf_tok returns
    at `p` (the bp loop marks the last pending identifier against that token too). -/
def prefixBound (sy : List Nat) : List Tok → Option Nat → List Nat → List Nat
  | [], pend, acc => mark pend .eof acc
  | t :: ts, pend, acc => prefixBound sy ts (pendOf sy t) (mark pend t acc)

/-- `pre[11]`: every site symbol is "bound". -/
def allBound (sy : List Nat) (toks : List Tok) : Bool :=
  let b := prefixBound sy toks none []
  sy.all (fun n => b.contains n)

/-- The runtime gate of cf_site, as written:
    rt = ok * pre[11] * (1 - pre[0]) * (1 - pre[2]) * cf_dirok * cf_adjok * (k <= 3) * (nsym <= 7).
    `ok` (the body parsed as affine, counter = itself + nonzero d) and the start-literal / kernel /
    direction / overflow flags are inputs; `k` = rows that are not the identity (written symbols). -/
def rtGate (ok : Bool) (sy : List Nat) (written : List Nat) (toks : List Tok)
    (litStart kernelFn dirok adjok : Bool) : Bool :=
  ok && allBound sy toks && !litStart && !kernelFn && dirok && adjok
    && decide (written.length ≤ 3) && decide (sy.length ≤ 7)

/-- The compiler's declare-before-use rule (E101), function-scoped: a name is declared at a point iff
    a parameter or an earlier `let NAME` / `let (.., NAME, ..)` declares it. `let` is identifier 16. -/
def declScan : Nat → List Tok → List Nat
  | _, [] => []
  | 0, t :: ts => if t = .ident 16 then declScan 1 ts else declScan 0 ts
  | 1, t :: ts => match t with
    | .ident n => n :: declScan 0 ts
    | .punct 40 => declScan 2 ts
    | _ => declScan 0 ts
  | 2, t :: ts => match t with
    | .ident n => n :: declScan 2 ts
    | .punct 44 => declScan 2 ts
    | _ => declScan 0 ts
  | _, _ :: ts => declScan 0 ts

/-- Mode 0 = scanning, 1 = just after `let`, 2 = inside a tuple `let (`. -/
def declLets (ts : List Tok) : List Nat := declScan 0 ts

/-- A loop body as R-CF accepts it: `let LHS = <affine form over READS>;` in order. -/
structure Stmt where
  lhs : Nat
  reads : List Nat
  deriving Repr

/-- Every read is declared when it runs: the declared set grows by each statement's LHS. -/
def bodyDeclOk : List Nat → List Stmt → Bool
  | _, [] => true
  | d, s :: ss => s.reads.all (fun r => d.contains r) && bodyDeclOk (s.lhs :: d) ss

/-- What cf_emit_rt's prefix `let (W..) = cf__pK(W.., R..);` reads: every site symbol (W then R),
    BEFORE the loop. -/
def rtPrefixReads (sy : List Nat) : List Nat := sy

-- ---------------------------------------------------------------------------------------------
-- The counterexample: .cfval/probes2/p11_fieldname.bp
--   struct P { x: i64, y: i64 }
--   fn h(n: i64) -> i64 {
--     let p = P { x: 4, y: n };
--     let s = p.x;
--     let i = n - n;
--     while i < 2000 { let x = i * 2; let s = s + x; let i = i + 1; 0 };
--     s
--   }
-- Identifiers: i 1, x 2, s 3, h 10, n 11, i64 12, p 13, P 14, y 15, let 16, fn 17.
-- Punctuation: ( 40, ) 41, : 58, -> 11582, { 123, } 125, = 61, , 44, ; 59, . 46, - 45.
-- ---------------------------------------------------------------------------------------------

/-- The fn text before the `while`, as cf_tok tokenises it. -/
def p11Prefix : List Tok :=
  [ .ident 17, .ident 10, .punct 40, .ident 11, .punct 58, .ident 12, .punct 41, .punct 11582,
    .ident 12, .punct 123,
    .ident 16, .ident 13, .punct 61, .ident 14, .punct 123, .ident 2, .punct 58, .num 4, .punct 44,
    .ident 15, .punct 58, .ident 11, .punct 125, .punct 59,
    .ident 16, .ident 3, .punct 61, .ident 13, .punct 46, .ident 2, .punct 59,
    .ident 16, .ident 1, .punct 61, .ident 11, .punct 45, .ident 11, .punct 59 ]

/-- cf_site's symbols in registration order: the counter, then each `let` after its right side. -/
def p11Sy : List Nat := [1, 2, 3]
/-- All three rows change (i = i + 1, x = 2i, s = s + 2i): k = 3. -/
def p11Written : List Nat := [1, 2, 3]
/-- The parameter `n`. -/
def p11Params : List Nat := [11]
/-- The loop body. -/
def p11Body : List Stmt := [⟨2, [1]⟩, ⟨3, [3, 2]⟩, ⟨1, [1]⟩]

/-- What is declared when the `while` is reached. -/
def p11Declared : List Nat := p11Params ++ declLets p11Prefix

/-- R-CF's gate ACCEPTS the loop for the runtime path (`let i = n - n;` is not a literal start,
    `h` is not a kernel fn, `<` with d = +1 is the right direction, 2000 + 0 does not overflow). -/
theorem p11_rt_accepted :
    rtGate true p11Sy p11Written p11Prefix false false true true = true := by decide

/-- ... because the scan counts the FIELD name `x` (in `P { x: 4 }` and `p.x`) as a binding. -/
theorem p11_x_bound_by_field : (prefixBound p11Sy p11Prefix none []).contains 2 = true := by decide

/-- The ORIGINAL program declares before use: the body's first statement declares `x`. -/
theorem p11_original_declares_before_use : bodyDeclOk p11Declared p11Body = true := by decide

/-- The REWRITTEN program reads `x` before anything declares it -- the E84 refusal measured. -/
theorem p11_rewrite_reads_undeclared :
    (rtPrefixReads p11Sy).contains 2 = true ∧ p11Declared.contains 2 = false := by decide

/-- THE FIX's specification, checked on the same text: with "bound" = DECLARED (the E101 rule, not
    "mentioned"), the gate refuses this loop. -/
def rtGateDeclared (ok : Bool) (sy written declared : List Nat) (litStart kernelFn dirok adjok : Bool) :
    Bool :=
  ok && sy.all (fun n => declared.contains n) && !litStart && !kernelFn && dirok && adjok
    && decide (written.length ≤ 3) && decide (sy.length ≤ 7)

theorem p11_fixed_gate_refuses :
    rtGateDeclared true p11Sy p11Written p11Declared false false true true = false := by decide

/-- The control (.cfval/probes3/p11c_control.bp): the body-local spelled `z` (identifier 20), which
    no field shares -- the scan does not mark it and R-CF leaves the loop alone, as measured. -/
def p11cSy : List Nat := [1, 20, 3]
theorem p11c_control_refused :
    rtGate true p11cSy [1, 20, 3] p11Prefix false false true true = false := by decide

/-- The gate is sound for this rule whenever "bound" implies "declared": then every symbol the
    prefix reads is declared. (The general statement the fix must meet; trivially true, and it is
    exactly the hypothesis the shipped scan violates.) -/
theorem rt_prefix_reads_declared (sy declared : List Nat) (h : sy.all (fun n => declared.contains n) = true) :
    (rtPrefixReads sy).all (fun n => declared.contains n) = true := h

end Bebop.Affine

#print axioms Bebop.Affine.p11_rt_accepted
#print axioms Bebop.Affine.p11_x_bound_by_field
#print axioms Bebop.Affine.p11_original_declares_before_use
#print axioms Bebop.Affine.p11_rewrite_reads_undeclared
#print axioms Bebop.Affine.p11_fixed_gate_refuses
#print axioms Bebop.Affine.p11c_control_refused
#print axioms Bebop.Affine.rt_prefix_reads_declared
