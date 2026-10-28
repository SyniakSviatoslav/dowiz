/-
  Bebop.Sync -- LV-INC-D (docs/research/R-BEBOP-LEVERS-2026-10-27.md §3.4, AC-LV-LOC-06): the sync of
  a tablet's tail into the hub is DETERMINISTIC and LOSES NOTHING. Core Lean 4, no Mathlib, no
  `axiom`, no `sorry`. The sheaf words (section, gluing) are the sync's vocabulary only (operator Q6,
  2026-10-27); the algebra is the Z-set one (Q2), imported from Bebop.ZSet.

  SELF-CONTAINED ON PURPOSE: Bebop.Stock (SHEAF3's fold_glue / glue_comm / escrow) is not on main yet.
  `merge_comm_perm` below is the general form glue_comm is an instance of (stock moves commute); once
  SHEAF3 lands, glue_comm can be re-derived from it.

  THE MODEL: an event type E, a state S, a step `S → E → S`; `merge step s l` folds the events in
  list order.
    merge_comm_perm      if every two events commute, merge is the same for EVERY arrival order (any
                         permutation of the tails, so any interleaving of the tablets)
    merge_deterministic  for ANY step (commuting or not), merging in the CANONICAL order -- sorted by a
                         key (at, ctx) injective on the events -- gives the same state for every
                         arrival order: the hub and every tablet replaying the same set agree
    zset_tails_perm      Z-set deltas commute: the merged Z-set is order-free (Q2's currency)
    syncFold             the write door: each tail event is ACCEPTED (stepped) or HELD (set aside),
                         decided against the running state
    no_event_lost        accepted ++ held is a permutation of the tail
    held_subset          every held event came from the tail
    len_law              accepted + held = tail, the S2 law `len == events + held`
    state_is_merge       the state after sync is the merge of exactly the accepted events
-/
import Bebop.ZSet

namespace Bebop.Sync

variable {E S : Type}

/-- The events folded in list order. -/
def merge (step : S → E → S) (s : S) (l : List E) : S := l.foldl step s

/-- Commuting events: the arrival order does not matter. -/
theorem merge_comm_perm (step : S → E → S) (hc : ∀ s a b, step (step s a) b = step (step s b) a)
    (s : S) {l₁ l₂ : List E} (hp : l₁.Perm l₂) : merge step s l₁ = merge step s l₂ :=
  hp.foldl_eq' (fun x _ y _ z => hc z x y) s

/-! ### The canonical order: any step, one answer -/

/-- Lexicographic on (at, ctx). -/
def lexLe (a b : Nat × Nat) : Bool := a.1 < b.1 || (a.1 == b.1 && a.2 ≤ b.2)

def canon (key : E → Nat × Nat) (l : List E) : List E := l.mergeSort (fun a b => lexLe (key a) (key b))

def cmerge (step : S → E → S) (key : E → Nat × Nat) (s : S) (l : List E) : S :=
  merge step s (canon key l)

theorem lexLe_trans (a b c : Nat × Nat) : lexLe a b = true → lexLe b c = true → lexLe a c = true := by
  simp only [lexLe, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true, beq_iff_eq]
  omega

theorem lexLe_total (a b : Nat × Nat) : (lexLe a b || lexLe b a) = true := by
  simp only [lexLe, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true, beq_iff_eq]
  omega

theorem lexLe_antisymm (a b : Nat × Nat) : lexLe a b = true → lexLe b a = true → a = b := by
  obtain ⟨a1, a2⟩ := a
  obtain ⟨b1, b2⟩ := b
  simp only [lexLe, Bool.or_eq_true, decide_eq_true_eq, Bool.and_eq_true, beq_iff_eq, Prod.mk.injEq]
  omega

/-- Any step: the canonical merge is the same for every arrival order, when no two events share a key. -/
theorem merge_deterministic (step : S → E → S) (key : E → Nat × Nat)
    (hkey : ∀ a b, key a = key b → a = b) (s : S) {l₁ l₂ : List E} (hp : l₁.Perm l₂) :
    cmerge step key s l₁ = cmerge step key s l₂ := by
  have trans : ∀ a b c : E, lexLe (key a) (key b) = true → lexLe (key b) (key c) = true →
      lexLe (key a) (key c) = true := fun a b c => lexLe_trans _ _ _
  have total : ∀ a b : E, (lexLe (key a) (key b) || lexLe (key b) (key a)) = true :=
    fun a b => lexLe_total _ _
  have h : canon key l₁ = canon key l₂ :=
    List.Perm.eq_of_pairwise
      (fun a b _ _ h1 h2 => hkey a b (lexLe_antisymm _ _ h1 h2))
      (List.pairwise_mergeSort trans total l₁) (List.pairwise_mergeSort trans total l₂)
      ((List.mergeSort_perm l₁ _).trans (hp.trans (List.mergeSort_perm l₂ _).symm))
  simp only [cmerge, h]

/-- Z-set deltas commute, so tails of Z-sets merge to one Z-set whatever the arrival order. -/
theorem zset_tails_perm {α : Type} (base : ZSet.ZSet α) {l₁ l₂ : List (ZSet.ZSet α)}
    (hp : l₁.Perm l₂) : merge ZSet.add base l₁ = merge ZSet.add base l₂ :=
  merge_comm_perm ZSet.add
    (fun s a b => by rw [ZSet.add_assoc, ZSet.add_comm a b, ← ZSet.add_assoc]) base hp

/-! ### The write door: accept or hold, nothing lost -/

/-- Walk the tail with the running state: `ok` accepts (the event is stepped) or holds it. -/
def syncFold (step : S → E → S) (ok : S → E → Bool) : S → List E → List E × List E × S
  | s, [] => ([], [], s)
  | s, e :: es =>
    if ok s e then
      match syncFold step ok (step s e) es with
      | (a, h, s') => (e :: a, h, s')
    else
      match syncFold step ok s es with
      | (a, h, s') => (a, e :: h, s')

theorem syncFold_spec (step : S → E → S) (ok : S → E → Bool) : ∀ (s : S) (tail : List E),
    ((syncFold step ok s tail).1 ++ (syncFold step ok s tail).2.1).Perm tail ∧
    (syncFold step ok s tail).2.2 = merge step s (syncFold step ok s tail).1
  | s, [] => by simp [syncFold, merge]
  | s, e :: es => by
    by_cases h : ok s e
    · have ih := syncFold_spec step ok (step s e) es
      simp only [syncFold, h, if_true]
      refine ⟨List.Perm.cons e ih.1, ?_⟩
      simp only [merge, List.foldl] at ih ⊢
      exact ih.2
    · have ih := syncFold_spec step ok s es
      simp only [syncFold, h]
      exact ⟨List.perm_middle.trans (List.Perm.cons e ih.1), ih.2⟩

theorem no_event_lost (step : S → E → S) (ok : S → E → Bool) (s : S) (tail : List E) :
    ((syncFold step ok s tail).1 ++ (syncFold step ok s tail).2.1).Perm tail :=
  (syncFold_spec step ok s tail).1

theorem held_subset (step : S → E → S) (ok : S → E → Bool) (s : S) (tail : List E) :
    ∀ e ∈ (syncFold step ok s tail).2.1, e ∈ tail :=
  fun _ he => (no_event_lost step ok s tail).mem_iff.mp (List.mem_append_right _ he)

theorem len_law (step : S → E → S) (ok : S → E → Bool) (s : S) (tail : List E) :
    (syncFold step ok s tail).1.length + (syncFold step ok s tail).2.1.length = tail.length := by
  rw [← List.length_append]; exact (no_event_lost step ok s tail).length_eq

theorem state_is_merge (step : S → E → S) (ok : S → E → Bool) (s : S) (tail : List E) :
    (syncFold step ok s tail).2.2 = merge step s (syncFold step ok s tail).1 :=
  (syncFold_spec step ok s tail).2

/-! ### Evaluated example: a shelf of 3, sales of 2, 2, 1; a sale past the level is held -/

private def sell (lvl q : Int) : Int := lvl - q
private def fits (lvl q : Int) : Bool := decide (q ≤ lvl)

#guard syncFold sell fits 3 [2, 2, 1] == ([2, 1], [2], 0)
#guard cmerge (fun s (e : Nat × Nat) => s * 10 + e.2) id 0 [(2, 7), (1, 5)] ==
  cmerge (fun s (e : Nat × Nat) => s * 10 + e.2) id 0 [(1, 5), (2, 7)]

end Bebop.Sync

#print axioms Bebop.Sync.merge_comm_perm
#print axioms Bebop.Sync.merge_deterministic
#print axioms Bebop.Sync.zset_tails_perm
#print axioms Bebop.Sync.no_event_lost
#print axioms Bebop.Sync.held_subset
#print axioms Bebop.Sync.len_law
#print axioms Bebop.Sync.state_is_merge
