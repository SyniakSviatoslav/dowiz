/-
  Bebop.DatalogZ -- LV-INC-D2 (AX14, docs/research/2026-10-02-ax-implementation-plan.md §C; LV-INC-F's
  selfhost/std/dl_del.bp): the Datalog relation store's weights ARE Z-sets, and presence = weight > 0.
  Core Lean 4, no Mathlib, no `axiom`, no `sorry`. Ties Bebop.Datalog's `cnt`/`counting_sound` to
  Bebop.ZSet's `ofList`/`logTail`/`add`/`single`.

  THE MODEL. A non-recursive stratum's derivations over the current inputs are a list `ds` (a multiset:
  one entry per rule instance that fires); `head d` is the IDB row it derives. The fact set F is
  `{h | ∃ d ∈ ds, head d = h}`. The store (dl_del.bp's CNT region) keeps a weight per row; its
  denotation is the Z-set `weight head ds = ofList (logTail (ds.map head))` -- every derivation +1.

  PROVED:
    weight_apply          weight head ds h = cnt head ds h (the Z-set and the count are one cell).
    zset_present_iff_pos  AX14: h ∈ F ↔ 0 < weight head ds h.
    weight_nonneg         a weight built from derivations is never negative.
    weight_cons / weight_erase  an insertion is `+ single (head d) 1`, a deletion `+ single (head d) (-1)`
                          (only for a derivation that is there: dl_cnt_leaf's -1 walk reads DELETED rows).
    store_invariant       folding the signed updates over ANY valid event stream from `weight ds` gives
                          `weight` of the resulting derivation list -- so `store_present_iff_pos`: after
                          the stream, presence in F is exactly a positive stored weight.
    delete_absent_negative the store's trap 125 is the right refusal: removing a derivation of a row that
                          has none drives the weight to -1, which no derivation list denotes.
    dred_sound            DRed (recursive strata): if over-deletion KEEPS only rows that are still in the
                          new least model M (K ⊆ M), then re-deriving -- iterating T from K -- stops at
                          exactly M. Over-deleting too much is harmless; keeping a lost row is not
                          (`dred_needs_sound_keep` shows the hypothesis cannot be dropped).

  OPEN (not proved here): that dl_del.bp's over-delete rule ("every row derivable through a deleted row")
  establishes K ⊆ M -- it needs derivation trees (well-founded proofs of membership), which this predicate
  model of facts does not carry. The bytes (5-cell rows, sorted regions, i64 overflow) are not modelled.
-/
import Bebop.Datalog
import Bebop.ZSet

namespace Bebop.DatalogZ

open Bebop.Datalog (cnt counting_sound FSet SubF Mono LeastPrefixed union empty naive)

section weights
variable {δ β : Type} [DecidableEq β]

/-- The store's Z-set: one +1 per derivation, at the row it derives. -/
def weight (head : δ → β) (ds : List δ) : ZSet.ZSet β := ZSet.ofList (ZSet.logTail (ds.map head))

theorem weight_apply (head : δ → β) : ∀ (ds : List δ) (h : β), weight head ds h = (cnt head ds h : Int)
  | [], h => by simp [weight, ZSet.logTail, ZSet.ofList, ZSet.zero, cnt]
  | d :: ds, h => by
    have ih := weight_apply head ds h
    simp only [weight, ZSet.logTail, List.map_cons, ZSet.ofList, ZSet.add, ZSet.single] at ih ⊢
    rw [ih]
    simp only [cnt, List.countP_cons]
    by_cases hd : head d = h
    · simp [hd]; omega
    · simp [hd, Ne.symm hd]

/-- AX14: a row is present in F iff its Z-set weight is positive. -/
theorem zset_present_iff_pos (head : δ → β) (ds : List δ) (h : β) :
    (∃ d ∈ ds, head d = h) ↔ 0 < weight head ds h := by
  rw [weight_apply, counting_sound]; omega

theorem weight_nonneg (head : δ → β) (ds : List δ) (h : β) : 0 ≤ weight head ds h := by
  rw [weight_apply]; omega

/-- An insertion adds `single (head d) 1`. -/
theorem weight_cons (head : δ → β) (d : δ) (ds : List δ) :
    weight head (d :: ds) = ZSet.add (weight head ds) (ZSet.single (head d) 1) := by
  rw [ZSet.add_comm]; rfl

/-- A deletion of a derivation that is there adds `single (head d) (-1)`. -/
theorem weight_erase [DecidableEq δ] (head : δ → β) (ds : List δ) (d : δ) (hd : d ∈ ds) :
    weight head (ds.erase d) = ZSet.add (weight head ds) (ZSet.single (head d) (-1)) := by
  funext h
  have hp := (List.perm_cons_erase hd).countP_eq (fun x => decide (head x = h))
  simp only [ZSet.add, ZSet.single, weight_apply, cnt] at hp ⊢
  rw [hp, List.countP_cons]
  by_cases hh : head d = h
  · simp [hh]; omega
  · simp [hh, Ne.symm hh]

/-- The trap-125 case: deleting a derivation of a row with none leaves a negative weight. -/
theorem delete_absent_negative (head : δ → β) (ds : List δ) (d : δ)
    (habs : ¬ ∃ d' ∈ ds, head d' = head d) :
    ZSet.add (weight head ds) (ZSet.single (head d) (-1)) (head d) < 0 := by
  rw [zset_present_iff_pos] at habs
  show weight head ds (head d) + (if head d = head d then (-1 : Int) else 0) < 0
  rw [if_pos rfl]
  omega

/-! ### The store over a stream of signed events (dl_cnt_leaf's +1 / -1) -/

variable [DecidableEq δ]

/-- An event: `(d, true)` a new derivation, `(d, false)` a lost one. -/
def applyEv (ds : List δ) : δ × Bool → List δ
  | (d, true) => d :: ds
  | (d, false) => ds.erase d

/-- What the store does with an event: a signed unit at the derived row. -/
def zstep (head : δ → β) (z : ZSet.ZSet β) : δ × Bool → ZSet.ZSet β
  | (d, b) => ZSet.add z (ZSet.single (head d) (if b then 1 else -1))

/-- A valid stream: every lost derivation was there when it was lost. -/
def Valid : List δ → List (δ × Bool) → Prop
  | _, [] => True
  | ds, (d, true) :: es => Valid (d :: ds) es
  | ds, (d, false) :: es => d ∈ ds ∧ Valid (ds.erase d) es

theorem store_invariant (head : δ → β) :
    ∀ (ds : List δ) (es : List (δ × Bool)), Valid ds es →
      es.foldl (zstep head) (weight head ds) = weight head (es.foldl applyEv ds)
  | _, [], _ => rfl
  | ds, (d, true) :: es, hv => by
    show es.foldl (zstep head) (zstep head (weight head ds) (d, true)) = _
    have : zstep head (weight head ds) (d, true) = weight head (d :: ds) := by
      simp only [zstep, if_true]; rw [weight_cons]
    rw [this]; exact store_invariant head (d :: ds) es hv
  | ds, (d, false) :: es, hv => by
    show es.foldl (zstep head) (zstep head (weight head ds) (d, false)) = _
    have : zstep head (weight head ds) (d, false) = weight head (ds.erase d) := by
      simp only [zstep]; rw [weight_erase head ds d hv.1]; rfl
    rw [this]; exact store_invariant head (ds.erase d) es hv.2

/-- After any valid stream, presence in F is exactly a positive STORED weight. -/
theorem store_present_iff_pos (head : δ → β) (ds : List δ) (es : List (δ × Bool)) (hv : Valid ds es)
    (h : β) :
    (∃ d ∈ es.foldl applyEv ds, head d = h) ↔ 0 < es.foldl (zstep head) (weight head ds) h := by
  rw [store_invariant head ds es hv]; exact zset_present_iff_pos head _ h

end weights

/-! ### DRed for a recursive stratum: over-delete, then re-derive from what was kept -/

section dred
universe u
variable {α : Type u}

/-- Iteration seeded with the kept rows K: I 0 = K, I (k+1) = I k ∪ T (I k). -/
def iterFrom (T : FSet α → FSet α) (K : FSet α) : Nat → FSet α
  | 0 => K
  | k + 1 => union (iterFrom T K k) (T (iterFrom T K k))

theorem iterFrom_below (T : FSet α → FSet α) (hm : Mono T) (M : FSet α) (hpre : SubF (T M) M)
    (K : FSet α) (hK : SubF K M) : ∀ k, SubF (iterFrom T K k) M := by
  intro k
  induction k with
  | zero => exact hK
  | succ k ih =>
    intro x hx
    cases hx with
    | inl h => exact ih x h
    | inr h => exact hpre x (hm _ _ ih x h)

/-- DRed IS SOUND: re-deriving from a kept set that the over-delete made sound (K ⊆ M, M the new
    least model) stops at exactly M -- no lost row survives, no surviving row is lost. -/
theorem dred_sound (T : FSet α → FSet α) (hm : Mono T) (M : FSet α) (hM : LeastPrefixed T M)
    (K : FSet α) (hK : SubF K M) (k : Nat) (hstab : iterFrom T K (k + 1) = iterFrom T K k) :
    iterFrom T K k = M := by
  have hbelow := iterFrom_below T hm M hM.1 K hK k
  have hpre : SubF (T (iterFrom T K k)) (iterFrom T K k) := by
    intro x hx
    have h : iterFrom T K (k + 1) x := Or.inr hx
    rw [hstab] at h; exact h
  have habove := hM.2 _ hpre
  funext x; apply propext; exact ⟨hbelow x, habove x⟩

/-- From scratch (K = {}) is the special case: DRed with everything over-deleted is a recompute. -/
theorem dred_from_scratch (T : FSet α → FSet α) (k : Nat) : iterFrom T empty k = naive T k := by
  induction k with
  | zero => rfl
  | succ k ih => show union _ _ = union _ _; rw [ih]

/-- The keep hypothesis cannot be dropped: T = identity, M = {} is its least model, but keeping a
    lost row (K = everything) re-derives a fixpoint that is not M. -/
theorem dred_needs_sound_keep :
    ∃ (T : FSet Unit → FSet Unit) (M K : FSet Unit), Mono T ∧ LeastPrefixed T M ∧
      iterFrom T K 1 = iterFrom T K 0 ∧ iterFrom T K 0 ≠ M := by
  refine ⟨fun X => X, empty, fun _ => True, fun _ _ h => h, ⟨fun _ h => h, fun _ _ _ h => h.elim⟩,
    ?_, ?_⟩
  · funext x; apply propext; exact ⟨fun _ => trivial, fun h => Or.inl h⟩
  · intro h; have := congrFun h (); exact (cast this trivial : False)

end dred

/-! ### Evaluated example (allergen-shaped: two derivations of one row, one deleted, then the other) -/

private def hd : Nat → String := fun n => if n < 2 then "a" else "b"
private def evs : List (Nat × Bool) := [(2, true), (0, false), (1, false)]

#guard weight hd [0, 1] "a" == 2
#guard evs.foldl (zstep hd) (weight hd [0, 1]) "a" == 0
#guard evs.foldl (zstep hd) (weight hd [0, 1]) "b" == 1
#guard (evs.take 2).foldl (zstep hd) (weight hd [0, 1]) "a" == 1

end Bebop.DatalogZ

#print axioms Bebop.DatalogZ.zset_present_iff_pos
#print axioms Bebop.DatalogZ.weight_erase
#print axioms Bebop.DatalogZ.store_invariant
#print axioms Bebop.DatalogZ.store_present_iff_pos
#print axioms Bebop.DatalogZ.delete_absent_negative
#print axioms Bebop.DatalogZ.dred_sound
#print axioms Bebop.DatalogZ.dred_needs_sound_keep
