/-
  Bebop.Stock -- sheaf row S1 (docs/research/2026-10-05-bebop-db-gaps-strengths-tensor-graph.md
  G.4 "Lean", theorems 1-3; G.5 row S1). Core Lean 4, no Mathlib, no `axiom`, no `sorry`.

  THE MODEL IS THE STOCK FOLD's ARITHMETIC (`crates/dowiz-hub/src/stock/ledger.rs`), one level per item:
  a receipt adds, a sale and a write-off subtract, a count SETS the level. `glue base a b` is what
  `dowiz-hub` `glue::glue` checks: the common prefix, then the hub's tail, then the kitchen's.

  * `fold_glue`  -- theorem 2: with additive tails (no count in either), the glued level of every item
    is the base's plus each tail's delta. This is why `glue/delta.rs` may DERIVE the kitchen's level
    as `ancestor + dKitchen` instead of folding its image.
  * `glue_comm`  -- theorem 1: under the same premise the two glue orders give the same shelf, item by
    item. A count on either tail is NOT additive and is outside both theorems: S0 reports it as
    `Conflict::Recount` and a person decides (the report's G.2 row 3).
  * `escrow`     -- theorem 3 (Balegas et al. 2015, bounded counters): if the rights handed out sum to
    no more than the item's level at the sync (`StockLog::grant` refuses a share past that), and every
    context's tail draws no more than its rights (every sale past them is written `"prov":1` by
    `StockLog::sell` and set aside, as `StockLog::unflagged_oversold` sets it aside), then EVERY
    prefix of ANY merge order of the tails leaves the level >= 0 -- not only the end of the day.
    The hub is one more context: its rights are what no tablet holds (`Rights::may_sell(HUB, ..)`).

  WHAT IS NOT MODELLED, each a gap on purpose: reservations (`available = on_hand - reserved`; the
  theorem is stated over the level `StockLog::sell` checks against), grant expiry (`until`: an expired
  grant makes later sales provisional, which only shrinks the tails), uncounted shelves (they promise
  nothing and are never flagged), and real-world event order (count against sale: a person's call).

  `#print axioms` at the bottom: the build log is grepped for `sorryAx` (must be 0) and for any
  axiom outside Lean's own three (propext, Classical.choice, Quot.sound).
-/

namespace Bebop.Stock

/-- One stock record's movement, item by index. -/
inductive Ev where
  | received (item : Nat) (q : Int)
  | served   (item : Nat) (q : Int)
  | wasted   (item : Nat) (q : Int)
  | count    (item : Nat) (observed : Int)
  deriving Repr

/-- Every item's level. -/
abbrev Shelf := Nat → Int

def step (s : Shelf) : Ev → Shelf
  | .received i q => fun k => if k = i then s k + q else s k
  | .served i q   => fun k => if k = i then s k - q else s k
  | .wasted i q   => fun k => if k = i then s k - q else s k
  | .count i o    => fun k => if k = i then o else s k

/-- The fold, oldest record first. -/
def fold (s : Shelf) (es : List Ev) : Shelf := es.foldl step s

/-- A record that moves the shelf by a delta (everything but a count). -/
def Ev.additive : Ev → Prop
  | .count _ _ => False
  | _ => True

/-- The signed move of one record on item `k`. -/
def move (k : Nat) : Ev → Int
  | .received i q => if k = i then q else 0
  | .served i q   => if k = i then -q else 0
  | .wasted i q   => if k = i then -q else 0
  | .count _ _    => 0

/-- A tail's delta on item `k`. -/
def delta (k : Nat) : List Ev → Int
  | [] => 0
  | e :: es => move k e + delta k es

/-- The glued section: the common prefix, then the hub's tail, then the kitchen's. -/
def glue (base a b : List Ev) : List Ev := base ++ a ++ b

theorem fold_append (s : Shelf) (a b : List Ev) : fold s (a ++ b) = fold (fold s a) b := by
  simp [fold, List.foldl_append]

theorem delta_append (k : Nat) : ∀ (a b : List Ev), delta k (a ++ b) = delta k a + delta k b
  | [], b => by simp [delta]
  | e :: a, b => by
    show move k e + delta k (a ++ b) = (move k e + delta k a) + delta k b
    rw [delta_append k a b]
    omega

theorem step_additive (s : Shelf) (e : Ev) (h : e.additive) (k : Nat) :
    step s e k = s k + move k e := by
  cases e with
  | received i q => by_cases hk : k = i <;> simp [step, move, hk]
  | served i q => by_cases hk : k = i <;> simp [step, move, hk] <;> omega
  | wasted i q => by_cases hk : k = i <;> simp [step, move, hk] <;> omega
  | count i o => exact absurd h (by simp [Ev.additive])

theorem fold_additive (k : Nat) : ∀ (s : Shelf) (es : List Ev), (∀ e ∈ es, e.additive) →
    fold s es k = s k + delta k es
  | s, [], _ => by simp [fold, delta]
  | s, e :: es, h => by
    have he : e.additive := h e (by simp)
    have hs : ∀ x ∈ es, x.additive := fun x hx => h x (by simp [hx])
    show fold (step s e) es k = s k + (move k e + delta k es)
    rw [fold_additive k (step s e) es hs, step_additive s e he k]
    omega

theorem additive_append {a b : List Ev} (ha : ∀ e ∈ a, e.additive) (hb : ∀ e ∈ b, e.additive) :
    ∀ e ∈ a ++ b, e.additive := by
  intro e he
  rcases List.mem_append.mp he with h | h
  · exact ha e h
  · exact hb e h

/-- THEOREM 2: fold(glued) = fold(base) + ΔA + ΔB, item by item, for additive tails. -/
theorem fold_glue (s : Shelf) (base a b : List Ev)
    (ha : ∀ e ∈ a, e.additive) (hb : ∀ e ∈ b, e.additive) (k : Nat) :
    fold s (glue base a b) k = fold s base k + delta k a + delta k b := by
  rw [glue, List.append_assoc, fold_append, fold_additive k _ _ (additive_append ha hb), delta_append]
  omega

/-- THEOREM 1: glue(A, B) = glue(B, A) for additive (commuting) tails: the same shelf. -/
theorem glue_comm (s : Shelf) (base a b : List Ev)
    (ha : ∀ e ∈ a, e.additive) (hb : ∀ e ∈ b, e.additive) :
    fold s (glue base a b) = fold s (glue base b a) := by
  funext k
  rw [fold_glue s base a b ha hb k, fold_glue s base b a hb ha k]
  omega

/-! ### Theorem 3: escrow -/

/-- What one record draws from item `k`: a sale or a write-off. -/
def draw (k : Nat) : Ev → Int
  | .served i q => if k = i then q else 0
  | .wasted i q => if k = i then q else 0
  | _ => 0

def dsum (k : Nat) : List Ev → Int
  | [] => 0
  | e :: es => draw k e + dsum k es

/-- A record a tablet may write: a movement, never negative (`StockError::NotPositive` at the door),
    and never a count (a count on a tail is `Conflict::Recount`, outside the escrow). -/
def Ev.sane : Ev → Prop
  | .received _ q => 0 ≤ q
  | .served _ q   => 0 ≤ q
  | .wasted _ q   => 0 ≤ q
  | .count _ _    => False

theorem sane_additive {e : Ev} (h : e.sane) : e.additive := by
  cases e <;> simp_all [Ev.sane, Ev.additive]

theorem sane_draw (k : Nat) {e : Ev} (h : e.sane) : 0 ≤ draw k e ∧ 0 ≤ move k e + draw k e := by
  cases e with
  | received i q => by_cases hk : k = i <;> simp_all [Ev.sane, draw, move]
  | served i q => by_cases hk : k = i <;> simp_all [Ev.sane, draw, move] <;> omega
  | wasted i q => by_cases hk : k = i <;> simp_all [Ev.sane, draw, move] <;> omega
  | count i o => simp [Ev.sane] at h

theorem dsum_append (k : Nat) : ∀ (a b : List Ev), dsum k (a ++ b) = dsum k a + dsum k b
  | [], b => by simp [dsum]
  | e :: a, b => by
    show draw k e + dsum k (a ++ b) = (draw k e + dsum k a) + dsum k b
    rw [dsum_append k a b]
    omega

/-- A sane tail draws a non-negative total, and its delta is at least minus what it draws. -/
theorem sane_bounds (k : Nat) : ∀ (es : List Ev), (∀ e ∈ es, e.sane) →
    0 ≤ dsum k es ∧ 0 ≤ delta k es + dsum k es
  | [], _ => by simp [dsum, delta]
  | e :: es, h => by
    have he := sane_draw k (h e (by simp))
    have ih := sane_bounds k es (fun x hx => h x (by simp [hx]))
    show 0 ≤ draw k e + dsum k es ∧ 0 ≤ (move k e + delta k es) + (draw k e + dsum k es)
    omega

theorem dsum_perm (k : Nat) {l₁ l₂ : List Ev} (h : l₁.Perm l₂) : dsum k l₁ = dsum k l₂ := by
  induction h with
  | nil => rfl
  | cons x _ ih => simp [dsum, ih]
  | swap x y l => simp [dsum]; omega
  | trans _ _ ih₁ ih₂ => rw [ih₁, ih₂]

/-- Every context's (rights, tail), and their concatenation in context order. -/
def tails : List (Int × List Ev) → List Ev
  | [] => []
  | p :: ps => p.2 ++ tails ps

def rsum : List (Int × List Ev) → Int
  | [] => 0
  | p :: ps => p.1 + rsum ps

theorem tails_sane : ∀ (ps : List (Int × List Ev)), (∀ p ∈ ps, ∀ e ∈ p.2, e.sane) →
    ∀ e ∈ tails ps, e.sane
  | [], _, e, he => by simp [tails] at he
  | p :: ps, h, e, he => by
    rcases List.mem_append.mp he with h1 | h1
    · exact h p (by simp) e h1
    · exact tails_sane ps (fun q hq => h q (by simp [hq])) e h1

theorem tails_inside (k : Nat) : ∀ (ps : List (Int × List Ev)), (∀ p ∈ ps, dsum k p.2 ≤ p.1) →
    dsum k (tails ps) ≤ rsum ps
  | [], _ => by simp [tails, rsum, dsum]
  | p :: ps, h => by
    have hp := h p (by simp)
    have ih := tails_inside k ps (fun q hq => h q (by simp [hq]))
    show dsum k (p.2 ++ tails ps) ≤ p.1 + rsum ps
    rw [dsum_append]
    omega

/-- THEOREM 3 (ESCROW). The rights handed out at the sync sum to no more than the level; each
    context's tail is sane and draws inside its rights; `m` is the tails merged in ANY order
    (any permutation, so any interleaving). Then after EVERY prefix of `m` the level is >= 0. -/
theorem escrow (s : Shelf) (base : List Ev) (k : Nat) (ps : List (Int × List Ev)) (m : List Ev)
    (hsane : ∀ p ∈ ps, ∀ e ∈ p.2, e.sane)
    (hinside : ∀ p ∈ ps, dsum k p.2 ≤ p.1)
    (hrights : rsum ps ≤ fold s base k)
    (hmerge : m.Perm (tails ps))
    (pre rest : List Ev) (hsplit : m = pre ++ rest) :
    0 ≤ fold s (base ++ pre) k := by
  have hm : ∀ e ∈ m, e.sane := fun e he => tails_sane ps hsane e (hmerge.mem_iff.mp he)
  have hpre : ∀ e ∈ pre, e.sane := fun e he => hm e (by rw [hsplit]; exact List.mem_append_left _ he)
  have hrest : ∀ e ∈ rest, e.sane := fun e he => hm e (by rw [hsplit]; exact List.mem_append_right _ he)
  have hlevel : fold s (base ++ pre) k = fold s base k + delta k pre := by
    rw [fold_append, fold_additive k _ _ (fun e he => sane_additive (hpre e he))]
  have hp := sane_bounds k pre hpre
  have hr := sane_bounds k rest hrest
  have htotal : dsum k pre + dsum k rest ≤ rsum ps := by
    rw [← dsum_append, ← hsplit, dsum_perm k hmerge]
    exact tails_inside k ps hinside
  omega

/-- The end of the day: the whole merge. -/
theorem escrow_end (s : Shelf) (base : List Ev) (k : Nat) (ps : List (Int × List Ev)) (m : List Ev)
    (hsane : ∀ p ∈ ps, ∀ e ∈ p.2, e.sane)
    (hinside : ∀ p ∈ ps, dsum k p.2 ≤ p.1)
    (hrights : rsum ps ≤ fold s base k)
    (hmerge : m.Perm (tails ps)) :
    0 ≤ fold s (base ++ m) k :=
  escrow s base k ps m hsane hinside hrights hmerge m [] (by simp)

/-- The premise is load-bearing: X5's salmon (17 on the shelf, 16 and 2 sold on two tablets with
    no rights) glues to -1. `decide` evaluates the fold; no axiom. -/
theorem x5_without_rights :
    fold (fun _ => 0) [.received 0 17, .served 0 16, .served 0 2] 0 = -1 := by decide

end Bebop.Stock

#print axioms Bebop.Stock.fold_glue
#print axioms Bebop.Stock.glue_comm
#print axioms Bebop.Stock.escrow
#print axioms Bebop.Stock.escrow_end
#print axioms Bebop.Stock.x5_without_rights
