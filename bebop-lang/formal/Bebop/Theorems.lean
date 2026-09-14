/-
  Bebop.Theorems -- F9 first theorems. 6 PROVED, 1 still an axiom.

  MEASURED 2026-09-14 (`grep -c '^theorem'` / `grep -c '^axiom'`): 6 `theorem`,
  1 `axiom`, 0 `sorry`. `#print axioms` is run on every one of them at the
  bottom of this file, so `lake build`'s own output states what each depends
  on. Quoting that output, because two of the lines are NOT the plain answer:

    isqrt_correct       [propext, Classical.choice, Quot.sound]
    addov_correct       [propext, Classical.choice, Quot.sound,
                         addov_correct._native.bv_decide.ax_1_6]
    mulov_total         [propext]
    cursor_monotone     [propext, Quot.sound]
    st_len_invariant    [propext, Quot.sound]
    st_len_masks_digest [propext, Classical.choice, Quot.sound,
                         st_len_masks_digest._native.bv_decide.ax_1_5]
    crc_consistent      does not depend on any axioms

  The four plain lines are ordinary Lean proofs: kernel-checked terms over the
  three standard axioms. The two `._native.bv_decide.ax_*` lines are NOT.
  `bv_decide` in Lean v4.33.1 discharges its LRAT check with `nativeEqTrue`
  (Lean/Meta/Tactic/BVDecide/Prover/Bitblast.lean:39), i.e. by COMPILED
  evaluation, and records that step as a generated axiom. So `addov_correct`
  and `st_len_masks_digest` are machine-checked with the Lean COMPILER and
  cadical in the trust root, not by the kernel alone. Against the F9 gate
  wording ("a kernel-checked term AND an LRAT certificate") they supply the
  certificate but not yet the kernel-only term; closing that is F7 work.
  Do not report them as kernel-checked.

  WHAT CHANGED, AND WHY IT MATTERED (measured 2026-09-14, lane l4lean).
  Two of the seven axioms were FALSE as stated, which made the whole axiom set
  INCONSISTENT -- from a false axiom every goal in this file, and in any file
  importing it, is derivable. The counterexamples, evaluated (they are kept as
  `#guard`s at the end of each section so they cannot silently come back):
    * `isqrt_correct` claimed `r*r <= s` with no sign hypothesis. At s = -1,
      isqrt_spec (-1) = 0 and 0*0 = 0 <= -1 is FALSE.
      It also claimed `s < (r+1)*(r+1)` over WRAPPING Int64. At s = i64::MAX,
      r = 3037000499 and (r+1)*(r+1) wraps to -9223372036709301616, so
      s < (r+1)^2 is FALSE.
    * `cursor_monotone` claimed `old + 2 + len >= old` over wrapping Int64 with
      no hypothesis on `len`. At old = 1024, len = -10 the new cursor is 1016;
      at old = i64::MAX, len = 0 it is -9223372036854775807. Both FALSE.
  Both are now theorems with the hypotheses that make them true, stated over
  `Int64.toInt` (unbounded Z) where the wrap was the bug.

  This file proves, about Bebop programs:
  1. fp_mul(a,b) = sign * floor(|a|*|b|/2^32) -- Q32 fixed-point multiply
     -- STILL AN AXIOM, the only one. See `fp_mul_correct`.
  2. isqrt(s)^2 <= s < (isqrt(s)+1)^2 -- integer square root      [THEOREM]
  3. Money laws (B8) -- addov exactness, mulov totality           [THEOREM x2]
  4. Store invariants -- st_len, cursor monotone, crc consistency [THEOREM x3]

  `addov_correct` is proved by `bv_decide`, i.e. by bit-blasting to SAT and
  checking the refutation with an LRAT certificate inside the Lean kernel --
  the pipeline ROADMAP F4 names. `st_len_invariant` and `crc_consistent` are
  proved by `rfl`: they are TRUE BUT VACUOUS, restating a definition, and
  `crc_consistent` in particular says nothing because `crc32x_impl` is still a
  `0` stub. They are marked as such below; do not cite them as content.

  F9 gate: theorems >= 3, each with a kernel-checked term and an LRAT certificate.
  Status against that gate: 6 of 7 proved; 1 (`addov_correct`) via LRAT.
  Dependencies: F7 (kernel) for certified proofs; specifications stated here.

  References:
  - ROADMAP.md F9 row (line 193)
  - selfhost/prelude/fp.bp (fp_mul, isqrt implementations)
  - selfhost/std/money.bp (money laws)
  - selfhost/prelude/store.bp (store invariants)
  - bench/oracles/rust/src/bin/money.rs (Rust oracle)
-/

import Std.Tactic.BVDecide
import Bebop.Basic
import Bebop.Semantics
import Bebop.Builtins
import Bebop.Syscalls
import Bebop.Traps
import Bebop.Conformance

namespace Bebop.Theorems

open Bebop.Semantics
open Bebop.Builtins
open Bebop.Syscalls
open Bebop.Traps
open Bebop.Conformance

-- ============================================================
-- 0. Bridging lemmas (Int64 <-> Nat/Int). Needed so the arithmetic
--    facts can be stated over UNBOUNDED integers instead of over the
--    wrapping Int64 ring, where two of the old axioms were FALSE.
-- ============================================================

/-- `Int64.ofNat` round-trips through `toInt` for any `n < 2^63`. -/
theorem toInt_ofNat_of_lt {n : Nat} (h : n < 2 ^ 63) :
    (Int64.ofNat n).toInt = (n : Int) := by
  rw [Int64.toInt_ofNat']
  exact Int.bmod_eq_of_le (by simp [Int64.size]) (by simp [Int64.size]; omega)

/-- `Nat.sqrt n <= n`. Lean core proves `Nat.sqrt_le` (`sqrt n * sqrt n <= n`)
    but not this; it follows in two cases. -/
theorem sqrt_le_self (n : Nat) : Nat.sqrt n ≤ n := by
  rcases Nat.eq_zero_or_pos (Nat.sqrt n) with h | h
  · omega
  · have hle := Nat.sqrt_le n
    calc Nat.sqrt n = Nat.sqrt n * 1 := by omega
      _ ≤ Nat.sqrt n * Nat.sqrt n := Nat.mul_le_mul_left _ h
      _ ≤ n := hle

-- ============================================================
-- 1. fp_mul theorem: Q32 fixed-point multiply
-- ============================================================

/-- Specification: fp_mul(a,b) = sign * floor(|a|*|b|/2^32).

    For Q32 fixed-point numbers, the product is computed by:
    1. Taking absolute values and tracking sign
    2. Computing |a|*|b| as a full 128-bit product
    3. Dividing by 2^32 (right shift by 32)
    4. Applying the sign

    The result wraps in i64 arithmetic (Z/2^64). -/
def fp_mul_spec (a b : Val) : Val :=
  let na := if a < 0 then 1 else 0
  let aa := if na == 1 then 0 - a else a
  let nb := if b < 0 then 1 else 0
  let ab := if nb == 1 then 0 - b else b
  -- Compute |a|*|b| in arbitrary precision, then shift right by 32.
  -- NOTE: use toInt.natAbs, NOT toNat: Int64.toNat clips negatives to 0,
  -- which would map |i64::MIN| to 0 (MIN wraps to itself on negation).
  -- Verified against the limb impl on 20k cases incl. MIN/MIN (2026-09-11).
  let prod := aa.toInt.natAbs * ab.toInt.natAbs
  let shifted := prod >>> 32
  -- Determine sign: odd number of negative operands => negative result
  let flip := na + nb
  let neg := (flip - (flip / 2) * 2) == 1
  let signed := if neg then
    Int64.ofNat shifted  -- Will wrap in Z/2^64
  else
    Int64.ofNat shifted
  -- Apply sign via wrapping subtraction
  if neg then 0 - signed else signed

/-- Bebop `>>` is the LOGICAL shift (LANGUAGE.md:64; `evalBinOp .lsr` in
    Semantics.lean). Lean's `>>>` on `Int64` is `Int64.shiftRight`, which is
    ARITHMETIC, so a faithful transcription of fp.bp's `>>` must go through
    UInt64. Measured 2026-09-13: with `>>>` on Int64 the case (i64::MIN, 1)
    gave impl = 2147483648 vs spec = -2147483648 and `#guard f9_fp_mul` failed;
    fp.bp itself (logical shift) agrees with the spec there. -/
def lsr (x : Val) (k : UInt64) : Val := (x.toUInt64 >>> k).toInt64

/-- fp_mul implementation matching selfhost/prelude/fp.bp:6-30 line for line
    (`>>` transcribed as `lsr`, `<<` as `<<<`).
    Uses 32/16-bit limb decomposition to avoid i128 and >> on negatives. -/
def fp_mul_impl (a b : Val) : Val :=
  let na := if a < 0 then 1 else 0
  let aa := if na == 1 then 0 - a else a
  let nb := if b < 0 then 1 else 0
  let ab := if nb == 1 then 0 - b else b
  let a1 := lsr aa 32
  let a0 := aa - (a1 <<< 32)
  let b1 := lsr ab 32
  let b0 := ab - (b1 <<< 32)
  let hi := a1 * b1
  let mid := a1 * b0 + a0 * b1
  let ah := lsr a0 16
  let al := a0 - (ah <<< 16)
  let bh := lsr b0 16
  let bl := b0 - (bh <<< 16)
  let ll_h := ah * bh
  let ll_m := ah * bl + al * bh
  let ll_l := al * bl
  let low := ll_h + (lsr (ll_m + (lsr ll_l 16)) 16)
  let p := (hi <<< 32) + mid + low
  let flip := na + nb
  let flip := flip - (flip / 2) * 2
  let p := if flip == 1 then 0 - p else p
  p

/-- AXIOM (NOT PROVED -- the only unproved statement left in this file).
    `fp_mul_impl a b = fp_mul_spec a b` for all a, b : Val.

    Why it is still an axiom, measured 2026-09-14:
    * `bv_decide` cannot take it. The spec side multiplies two 64-bit
      magnitudes in UNBOUNDED `Nat` (`aa.toInt.natAbs * ab.toInt.natAbs`) and
      then shifts right by 32, i.e. it names the high half of a 128-bit
      product. Bit-blasting a 64x64 multiplier is the classic hard case for
      SAT, and the `Nat`-to-`BitVec 128` bridge is not in the goal language.
    * The honest proof is a limb-decomposition argument
      (a = a1*2^32 + a0, b = b1*2^32 + b0, and likewise 16-bit inside a0*b0)
      plus a carry bound on `ll_m + (ll_l >>> 16)`. That is real work, not a
      tactic call, and is deliberately NOT faked here with `sorry`.

    Evidence that the statement is at least not false: `f9_fp_mul` below
    checks impl == spec on 13 hand-picked pairs including i64::MIN, and
    `fp_mul_sweep` checks 1024 more from a deterministic LCG. A passing
    differential is not a proof; it only rules out the cheap refutation. -/
axiom fp_mul_correct (a b : Val) :
  fp_mul_impl a b = fp_mul_spec a b

/-- Deterministic differential sweep for the ONE remaining axiom. An LCG
    (the 64-bit Knuth/MMIX constants, reduced into Int64) generates 1024
    operand pairs and checks `fp_mul_impl == fp_mul_spec` on each. This is a
    TEST, not a proof: it can only refute `fp_mul_correct`, never establish
    it. It exists so that the axiom is not merely unexamined. -/
def fp_mul_sweep : Bool := Id.run do
  let mut x : UInt64 := 0x853c49e6748fea9b
  let mut ok : Bool := true
  for _ in [0:1024] do
    x := x * 6364136223846793005 + 1442695040888963407
    let a : Val := x.toInt64
    x := x * 6364136223846793005 + 1442695040888963407
    let b : Val := x.toInt64
    if fp_mul_impl a b != fp_mul_spec a b then
      ok := false
  return ok

/- 1024 LCG operand pairs: impl agrees with spec (a test, not a proof) -/
#guard fp_mul_sweep

/-- f9_fp_mul gate: verifies fp_mul theorem is stated and well-formed.
    Evaluates to true when:
    - The specification is well-defined for all test cases
    - The implementation matches the spec on sample values
    - The theorem statement is correctly formed

    Note: In Q32 fixed-point, 1 unit = 1/2^32. To represent integer N, use N * 2^32. -/
def f9_fp_mul : Bool :=
  -- Test cases covering: zero, positive, negative, mixed signs, boundary
  let test_cases : List (Val × Val) := [
    (0, 0),                                    -- zero
    (4294967296, 4294967296),                  -- 1.0 * 1.0 = 1.0 (Q32 identity)
    (4294967296, 1),                           -- 1.0 * epsilon = epsilon
    (0 - 4294967296, 1),                       -- -1.0 * epsilon = -epsilon
    (0 - 4294967296, 4294967296),              -- -1.0 * 1.0 = -1.0
    (4294967296, 0 - 4294967296),              -- 1.0 * -1.0 = -1.0
    (8589934592, 2),                           -- 2.0 * epsilon = 2*epsilon
    (9223372036854775807, 1),                  -- i64::MAX * epsilon
    (0 - 9223372036854775807 - 1, 1),          -- i64::MIN * epsilon
    (123456789, 987654321),                    -- arbitrary
    (0 - 123456789, 987654321),                -- negative * positive
    (123456789, 0 - 987654321),                -- positive * negative
    (0 - 123456789, 0 - 987654321)             -- negative * negative
  ]
  let check (a b : Val) : Bool :=
    fp_mul_impl a b == fp_mul_spec a b
  -- Verify impl matches spec on ALL test cases (incl. i64::MIN edge).
  let all_match := test_cases.all (fun (a, b) => check a b)
  -- Verify spec is well-formed on a few key cases (Q32 arithmetic)
  let spec_zero := fp_mul_spec 0 0 == 0                    -- 0 * 0 = 0
  let spec_identity := fp_mul_spec 4294967296 4294967296 == 4294967296  -- 1.0 * 1.0 = 1.0
  let spec_neg := fp_mul_spec (0 - 4294967296 : Val) (4294967296 : Val) == (0 - 4294967296 : Val)  -- -1.0 * 1.0 = -1.0
  all_match && spec_zero && spec_identity && spec_neg

/- f9_fp_mul gate passes -/
#guard f9_fp_mul

-- ============================================================
-- 2. isqrt theorem: integer square root
-- ============================================================

/-- Specification: isqrt(s) = floor(sqrt(s)) for s >= 0.

    Uses Nat.sqrt which is the integer square root in Lean 4.
    For s <= 0, returns 0 (matching the Bebop implementation). -/
def isqrt_spec (s : Val) : Val :=
  if s <= 0 then 0
  else
    let n := s.toInt.natAbs
    let r := Nat.sqrt n
    Int64.ofNat r

/-- THEOREM (proved): isqrt satisfies isqrt(s)^2 <= s < (isqrt(s)+1)^2.

    Stated over `Int64.toInt`, i.e. in unbounded Z, and under `0 < s`.
    BOTH of those are load-bearing, and their absence is why the previous
    `axiom` of this name was false (measured 2026-09-14):
    * without `0 < s`: at s = -1 the spec returns 0 and `0*0 <= -1` is false;
    * over Int64 rather than Z: at s = i64::MAX, r = 3037000499 and
      `(r+1)*(r+1)` WRAPS to -9223372036709301616, so `s < (r+1)^2` is false.
    See `cex_isqrt_neg` / `cex_isqrt_wrap` below, which evaluate those two
    refutations.

    Proof: `isqrt_spec` is `Int64.ofNat (Nat.sqrt |s|)` on the positive
    branch; `toInt_ofNat_of_lt` discharges the round-trip (Nat.sqrt n <= n <
    2^63), and then it is exactly `Nat.sqrt_le` and `Nat.lt_succ_sqrt` from
    Lean core, cast to Z. -/
theorem isqrt_correct (s : Val) (hs : 0 < s) :
    (isqrt_spec s).toInt * (isqrt_spec s).toInt ≤ s.toInt ∧
      s.toInt < ((isqrt_spec s).toInt + 1) * ((isqrt_spec s).toInt + 1) := by
  have hpos : (0 : Int) < s.toInt := by
    have := Int64.lt_iff_toInt_lt.mp hs
    simpa using this
  have hlt : s.toInt < 2 ^ 63 := Int64.toInt_lt s
  have hspec : isqrt_spec s = Int64.ofNat (Nat.sqrt s.toInt.natAbs) := by
    unfold isqrt_spec
    rw [if_neg]
    intro hle
    have := Int64.le_iff_toInt_le.mp hle
    simp at this
    omega
  have hn : s.toInt = (s.toInt.natAbs : Int) := by omega
  have hrle : Nat.sqrt s.toInt.natAbs ≤ s.toInt.natAbs := sqrt_le_self _
  have hrlt : Nat.sqrt s.toInt.natAbs < 2 ^ 63 := by omega
  rw [hspec, toInt_ofNat_of_lt hrlt, hn]
  exact ⟨by exact_mod_cast Nat.sqrt_le s.toInt.natAbs,
         by exact_mod_cast Nat.lt_succ_sqrt s.toInt.natAbs⟩

/-- The two counterexamples that refuted the OLD `axiom isqrt_correct`, kept
    executable so the false form cannot silently return. `cex_isqrt_neg` is the
    lower bound failing at s = -1; `cex_isqrt_wrap` is the upper bound failing
    at s = i64::MAX because `(r+1)*(r+1)` wraps negative in Int64. Each is
    `true` exactly when the OLD statement is refuted at that point. -/
def cex_isqrt_neg : Bool :=
  let s : Val := 0 - 1
  let r := isqrt_spec s
  !(decide (r * r ≤ s))

def cex_isqrt_wrap : Bool :=
  let s : Val := 9223372036854775807
  let r := isqrt_spec s
  decide ((r + 1) * (r + 1) < 0) && !(decide (s < (r + 1) * (r + 1)))

/- The old axiom was FALSE at s = -1 (lower bound) -/
#guard cex_isqrt_neg
/- ... and FALSE at s = i64::MAX (upper bound wraps) -/
#guard cex_isqrt_wrap

/-- f9_isqrt gate: verifies isqrt theorem is stated correctly.
    Evaluates to true when:
    - The specification satisfies the bound on all test cases
    - The theorem statement is correctly formed -/
def f9_isqrt : Bool :=
  -- Test cases covering: zero, perfect squares, non-squares, boundary
  let test_cases : List Val := [
    0,                                          -- zero
    1,                                          -- 1 = 1^2
    2,                                          -- non-square
    3,                                          -- non-square
    4,                                          -- 4 = 2^2
    8,                                          -- non-square
    9,                                          -- 9 = 3^2
    15,                                         -- non-square
    16,                                         -- 16 = 4^2
    100,                                        -- 100 = 10^2
    1000000,                                    -- 1000000 = 1000^2
    1000001,                                    -- non-square
    4611686018427387903,                        -- 2^62 - 1 (max valid input)
    4611686018427387904                         -- 2^62
  ]
  let check (s : Val) : Bool :=
    let r := isqrt_spec s
    let r_nat := r.toInt.natAbs
    let s_nat := s.toInt.natAbs
    -- Check: r^2 <= s < (r+1)^2
    let lower := decide (r_nat * r_nat ≤ s_nat)
    let upper := decide (s_nat < (r_nat + 1) * (r_nat + 1))
    lower && upper
  test_cases.all check

/- f9_isqrt gate passes -/
#guard f9_isqrt

-- ============================================================
-- 3. Money laws (B8): overflow detection matches Rust oracle
-- ============================================================

/-- money_addov implementation (mirrors selfhost/std/money.bp) -/
def money_addov (a b : Val) : Val :=
  if (((a ^^^ (a + b)) &&& (b ^^^ (a + b))) < 0) then 1 else 0

/-- money_mulov implementation (mirrors selfhost/std/money.bp) -/
def money_mulov (a b : Val) : Val :=
  let lo := 0 - 9223372036854775807 - 1
  let bz := if b == 0 then 1 else 0
  let bm := if b == (0 - 1) then 1 else 0
  let bs := if (bz + bm) != 0 then 1 else b
  let p := a * b
  let q := Val.sdiv p bs
  let gen := if q != a then 1 else 0
  let ovm := if a == lo then 1 else 0
  bm * ovm + (1 - bz) * (1 - bm) * gen

set_option maxHeartbeats 1000000 in
/-- THEOREM (proved, by `bv_decide`): addov detects i64 addition overflow
    EXACTLY -- the iff holds for all 2^128 operand pairs, not just on samples.

    `money_addov a b = 1` iff a+b overflows i64, i.e. iff both operands are
    strictly positive and the wrapped sum is negative, or both are strictly
    negative and the wrapped sum is non-negative. The implementation tests
    `((a ^ (a+b)) & (b ^ (a+b))) < 0`, which is the sign-bit-disagreement
    trick; the theorem is that that trick is the overflow predicate.

    Proof: `bv_decide` bit-blasts the goal to CNF, refutes it with cadical and
    checks the LRAT certificate. This is the first use in this tree of the
    "native LRAT pipeline" the F4 row cites as the reason Lean was chosen over
    Coq -- and note the word NATIVE. Measured 2026-09-14, `#print axioms
    addov_correct` reports a dependency on `addov_correct._native.bv_decide.
    ax_1_6`: the certificate check runs as compiled code, so the Lean compiler
    is in the trust root here, the kernel alone is not. True and checked, but
    not kernel-checked; see the file header.

    Matches the Rust oracle in bench/oracles/rust/src/bin/money.rs. -/
theorem addov_correct (a b : Val) :
    (money_addov a b = 1) ↔
      ((0 < a ∧ 0 < b ∧ a + b < 0) ∨ (a < 0 ∧ b < 0 ∧ 0 ≤ a + b)) := by
  simp only [money_addov]
  bv_decide

/-- THEOREM (proved): mulov is TOTAL and BOOLEAN -- it returns 0 or 1 for
    every operand pair, never some other value.

    Read the statement for exactly what it does and does NOT say. It does NOT
    say that 1 means overflow; that is the stronger `mulov_exact` claim, which
    is NOT proved here (the argument needs `Val.sdiv` at i64::MIN / -1 and a
    division-inverse lemma). What it does say is that the arithmetic
    combination `bm*ovm + (1-bz)*(1-bm)*gen` cannot produce a third value --
    which is not obvious from the expression, since it is a sum of products of
    four independently-computed indicators.

    Proof: case-split all four indicator `if`s (`b == 0`, `b == -1`,
    `a == i64::MIN`, `(a*b)/bs != a`); each of the 16 leaves is a closed Int64
    expression and falls to `decide`.

    The value agreement with bench/oracles/rust/src/bin/money.rs is checked by
    `f9_money` below on samples, which is a test, not this theorem. -/
theorem mulov_total (a b : Val) :
    money_mulov a b = 0 ∨ money_mulov a b = 1 := by
  simp only [money_mulov]
  repeat' split
  all_goals decide

/-- Money theorem count (addov + mulov laws) -/
def f9_money_theorem_count : Nat := 2

/-- f9_money gate: verifies money laws are stated correctly. -/
def f9_money : Bool :=
  -- Test addov on key cases
  let addov_zero := money_addov 0 0 == 0
  let addov_pos := money_addov 1 1 == 0
  let addov_neg := money_addov (-1 : Val) (-1 : Val) == 0
  let addov_max := money_addov 9223372036854775807 1 == 1  -- overflow
  let addov_min := money_addov (0 - 9223372036854775807 - 1) (0 - 1) == 1  -- overflow
  -- Test mulov on key cases
  let mulov_zero := money_mulov 0 0 == 0
  let mulov_one := money_mulov 1 1 == 0
  let mulov_max := money_mulov 9223372036854775807 2 == 1  -- overflow
  let mulov_min := money_mulov (0 - 9223372036854775807 - 1) (0 - 1) == 1  -- overflow
  addov_zero && addov_pos && addov_neg && addov_max && addov_min &&
  mulov_zero && mulov_one && mulov_max && mulov_min

/- f9_money gate passes -/
#guard f9_money

-- ============================================================
-- 4. Store invariants: st_len, cursor monotone, crc
-- ============================================================

/-- st_len implementation (mirrors selfhost/prelude/store.bp) -/
def st_len (base : Array Val) (off : Val) : Val :=
  (base.getD off.toNatClampNeg (0 : Val)) &&& 4294967295

/-- crc32x implementation (placeholder for formal model).

    In the full formal model, this would compute CRC32 over the raw
    little-endian bytes of the cells. For now, it's a placeholder
    since the actual CRC32 computation is complex and the invariant
    is stated as an axiom. -/
def crc32x_impl (base : Array Val) (off n : Val) : Val :=
  -- Placeholder: the actual implementation would:
  -- 1. Read n cells starting at off
  -- 2. Extract 8 LE bytes from each cell
  -- 3. Compute CRC32 over all bytes
  -- For the formal model, we use 0 as a placeholder
  0

/-- st_crc implementation (mirrors selfhost/prelude/store.bp) -/
def st_crc (base : Array Val) (off n : Val) : Val :=
  crc32x_impl base off n

/-- THEOREM (proved by `rfl`) -- but VACUOUS, and labelled so on purpose.

    For an object at offset off with header h0 = (digest << 32) | length,
    st_len returns h0 & 0xFFFFFFFF = length; that is the invariant that lets
    the store traverse objects in the append-only arena.

    The statement below, however, is the DEFINITION of `st_len` written twice,
    so `rfl` closes it and it carries no information. It is kept because
    deleting it would silently drop a row from the F9 list; it is relabelled
    because calling it a proved store invariant would be a false claim.
    The real content would be: given a header cell whose low 32 bits are `len`
    and whose high 32 are a digest, `st_len` recovers `len` and NOT any digest
    bit -- that needs the header-construction function from store.bp, which is
    not modelled in `formal/` yet. That gap is listed in this lane's verdict. -/
theorem st_len_invariant (base : Array Val) (off : Val) :
    st_len base off = (base.getD off.toNatClampNeg (0 : Val)) &&& 4294967295 := rfl

set_option maxHeartbeats 1000000 in
/-- THEOREM (proved): `st_len` really does ignore the high 32 bits of the
    header cell -- the non-vacuous half of the invariant above, for the one
    header shape the model can express. For any digest d and length l < 2^32,
    `st_len #[(d <<< 32) ||| l] 0 = l`, checked over the whole 32-bit range by
    `bv_decide` rather than on samples. -/
theorem st_len_masks_digest (d l : Val) (hl : 0 ≤ l) (hl' : l < 4294967296) :
    ((d <<< 32) ||| l) &&& 4294967295 = l := by
  bv_decide

/-- THEOREM (proved): cursor monotone, stated so that it is TRUE.

    After st_alloc of `len` cells the cursor (tx[2]) advances by exactly
    2 + len (header cells h0, h1 plus payload), so the arena is append-only.

    The hypothesis `0 <= len` and the statement over `Int64.toInt` (unbounded
    Z) are both load-bearing. The previous `axiom` of this name had neither and
    was FALSE twice over, measured 2026-09-14:
    * len = -10, old = 1024 -> new cursor 1016, which is LESS than old;
    * len = 0, old = i64::MAX -> new cursor -9223372036854775807, wrapped.
    See `cex_cursor_neglen` / `cex_cursor_wrap` below.

    So what is proved is the ALGEBRAIC advance, in Z, for a non-negative
    allocation length. The remaining obligation -- that the Int64 cursor does
    not wrap, i.e. that the arena never allocates past 2^63 cells -- is a
    capacity fact about st_alloc's guard, not about this arithmetic, and is
    not modelled here. Listed as a gap in this lane's verdict. -/
theorem cursor_monotone (tx : Array Val) (len : Val) (hlen : 0 ≤ len) :
    (tx.getD (2 : Nat) (0 : Val)).toInt ≤
      (tx.getD (2 : Nat) (0 : Val)).toInt + 2 + len.toInt := by
  have h : (0 : Int) ≤ len.toInt := by
    have := Int64.le_iff_toInt_le.mp hlen
    simpa using this
  omega

/-- The two counterexamples that refuted the OLD `axiom cursor_monotone`, kept
    executable. Each is `true` exactly when the OLD statement is refuted. -/
def cex_cursor_neglen : Bool :=
  let old_cursor : Val := 1024
  let len : Val := 0 - 10
  !(decide (old_cursor + 2 + len ≥ old_cursor))

def cex_cursor_wrap : Bool :=
  let old_cursor : Val := 9223372036854775807
  let len : Val := 0
  !(decide (old_cursor + 2 + len ≥ old_cursor))

/- The old axiom was FALSE for a negative allocation length -/
#guard cex_cursor_neglen
/- ... and FALSE when the Int64 cursor wraps -/
#guard cex_cursor_wrap

/-- THEOREM (proved by `rfl`) -- but VACUOUS TWICE OVER, and labelled so.

    The intent is: st_crc computes crc32x over the raw little-endian bytes of
    n cells starting at off, the crc is stored in the object header and
    verified on every read, so reads detect corruption.

    What the statement actually says is `st_crc = crc32x_impl`, which is the
    definition of `st_crc`; and `crc32x_impl` is itself a `0` STUB (see its
    definition above -- it ignores all three arguments). So this closes by
    `rfl` and asserts nothing whatsoever about CRC32. Proving the real thing
    needs crc32x modelled over the LE byte decomposition of the cells, which
    `formal/` does not have. Biggest single gap in the store family. -/
theorem crc_consistent (base : Array Val) (off n : Val) :
    st_crc base off n = crc32x_impl base off n := rfl

/-- Honest witness that `crc32x_impl` is a stub: it returns 0 for inputs whose
    real CRC32 differs. Kept so nobody reads `crc_consistent` as content. -/
def crc_is_stub : Bool :=
  crc32x_impl #[1, 2, 3, 4] 0 4 == 0 && crc32x_impl #[9, 9, 9, 9] 0 4 == 0

/- crc32x_impl is a constant-0 stub, so crc_consistent is vacuous -/
#guard crc_is_stub

/-- Store theorem count (st_len + cursor_monotone + crc_consistent) -/
def f9_store_theorem_count : Nat := 3

/-- f9_store gate: verifies store invariants are stated correctly. -/
def f9_store : Bool :=
  -- Test st_len on a sample header
  let test_base : Array Val := #[3544952156018063160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
  let len_result := st_len test_base 0
  -- The header 0x3132333435363738 ("87654321" LE) is 3544952156018063160 in decimal;
  -- the scaffold wrote 3554557610294396226, which is NOT that number (its low 32 bits are
  -- 1329743170), so this #guard had never held. Low 32 bits of the
  -- real header: 0x35363738 = 892745528 (recomputed 2026-09-13).
  let st_len_ok := len_result == 892745528
  -- Test cursor monotone content: old=1024, new=1024+2+10 >= old.
  -- (The axiom itself is opaque, so the gate decides the arithmetic content.)
  let cursor_ok := decide ((1024 : Val) + 2 + 10 ≥ 1024)
  st_len_ok && cursor_ok

/- f9_store gate passes -/
#guard f9_store

-- ============================================================
-- 5. F9 summary and combined gate
-- ============================================================

/-- How many of the F9 statements are PROVED (`theorem`, kernel-checked term,
    no `sorry`). Counted 2026-09-14, one per declaration:
    - fp_mul: 0 of 1. `fp_mul_correct` is still an `axiom`.
    - isqrt:  1 of 1. `isqrt_correct`.
    - money:  2 of 2. `addov_correct` (bv_decide/LRAT), `mulov_total`.
    - store:  3 of 3. `st_len_invariant` (vacuous, rfl), `cursor_monotone`,
              `crc_consistent` (vacuous, rfl), plus the non-vacuous
              `st_len_masks_digest` which is extra, not one of the seven.
    Total: 6 of 7 proved. Keep this number equal to `grep -c '^theorem'` over
    the seven F9 names; it is not derived, so it can rot. -/
def f9_proved_count : Nat := 6

/-- How many of the F9 statements remain unproved assumptions. -/
def f9_axiom_count : Nat := 1

/-- Total F9 statements (proved + still-axiom). -/
def f9_theorem_count : Nat :=
  f9_proved_count + f9_axiom_count

/-- F9 combined gate: all sub-gates pass and theorem count >= 3.

    Per ROADMAP F9 gate: `theorems: >= 3` then growing, each with a
    kernel-checked term and an LRAT certificate. -/
def f9_gate_pass : Bool :=
  f9_fp_mul && f9_isqrt && f9_money && f9_store && decide (f9_proved_count ≥ 3)

/- F9 combined gate passes -/
#guard f9_gate_pass

/-- F9 verdict: reports the status of the first theorems. -/
def f9_verdict : String :=
  if f9_gate_pass then
    "PASS: F9 first theorems — "
    ++ toString f9_proved_count ++ " of " ++ toString f9_theorem_count
    ++ " PROVED (0 sorry; 4 of them kernel-only, 2 via bv_decide's NATIVE "
    ++ "LRAT check so the Lean compiler is in their trust root), "
    ++ toString f9_axiom_count ++ " still axiom. "
    ++ "PROVED: isqrt_correct (integer sqrt, in Z, for 0 < s); "
    ++ "addov_correct (exact i64 add-overflow iff, via bv_decide/LRAT); "
    ++ "mulov_total (mulov is 0-or-1); "
    ++ "cursor_monotone (arena advance in Z, for 0 <= len); "
    ++ "st_len_invariant and crc_consistent (rfl — VACUOUS, see their docs); "
    ++ "st_len_masks_digest (non-vacuous, via bv_decide). "
    ++ "AXIOM: fp_mul_correct (Q32 fixed-point; 64x64 product is out of "
    ++ "bv_decide's reach, the limb argument is unwritten). "
    ++ "Two former axioms (isqrt_correct, cursor_monotone) were FALSE as "
    ++ "stated and are now theorems with the hypotheses that make them true; "
    ++ "their counterexamples are kept as #guards."
  else
    "FAIL: F9 gate check failed"

-- ============================================================
-- 6. Audit trail: what each proved statement actually depends on.
--    `#print axioms` is the only honest answer to "is this proved?".
--    It prints during `lake build`, so the build log carries the evidence.
--    A `sorryAx` in any of these lines would mean the proof is fake;
--    measured 2026-09-14, none of them mention it.
-- ============================================================

#print axioms isqrt_correct
#print axioms addov_correct
#print axioms mulov_total
#print axioms cursor_monotone
#print axioms st_len_invariant
#print axioms st_len_masks_digest
#print axioms crc_consistent
#print axioms toInt_ofNat_of_lt
#print axioms sqrt_le_self

end Bebop.Theorems
