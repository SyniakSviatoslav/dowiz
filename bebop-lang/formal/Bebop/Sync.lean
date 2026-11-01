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

  THE FRONTIER EXTENSION (W-MG-LEAN, for M2; the file-level theorems are Bebop.DocMerge):
    glue / glue_comm / glue_idem / glue_assoc   one device's chain on two sides: of two lists, one a
                         prefix of the other (`Compat`, else a Fork), the longer. Commutative,
                         idempotent, and associative INCLUDING the refusal (`compat_assoc`)
    unionL_mem, no_record_lost   the union keeps every record of both sides, duplicates once
    front_perm           the frontier (per actor, the highest seq) as a fold: any arrival order, by
                         merge_comm_perm
    front_union_comm / _assoc / _idem   on frontiers the union is a semilattice, the file's own
                         order across devices playing no part (which nobody signs)
    conflict_named, conflict_stays, obstructed_iff, commutes_when_clean   GLUING DETECTS AND NEVER
                         RESOLVES: a clash on the overlap is named, both records stay, and with
                         nothing named the two tails commute
  THE MODEL'S CLASH IS PAIRWISE. dowiz-hub's `glue::obstructed` folds both sides and can name a
  conflict no single pair shows (a shelf two tails overdraw together). `named` is the shape of the
  guarantee (what is detected is named, and both records stay), not that detector.
-/
import Bebop.ZSet

namespace Bebop.Sync

set_option linter.unusedSectionVars false

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

/-! ### The frontier: two sides of one chain -/

section Frontier

variable {R : Type}

/-- One side's chain is a prefix of the other's. Anything else is a Fork (merge.rs `missing`). -/
def Compat (x y : List R) : Prop := x <+: y ∨ y <+: x

/-- The longer of the two: M2's "their tail". Meaningful under `Compat`. -/
def glue (x y : List R) : List R := if x.length ≤ y.length then y else x

theorem glue_of_prefix {x y : List R} (h : x <+: y) : glue x y = y := by
  simp [glue, h.length_le]

theorem glue_of_prefix' {x y : List R} (h : y <+: x) : glue x y = x := by
  unfold glue
  split
  · next hl => exact (h.eq_of_length (Nat.le_antisymm h.length_le hl)).symm ▸ rfl
  · rfl

theorem Compat.symm {x y : List R} (h : Compat x y) : Compat y x := Or.symm h

theorem compat_self (x : List R) : Compat x x := .inl (List.prefix_refl x)

/-- Two prefixes of one list are comparable: the fact behind the refusal's associativity. -/
theorem compat_of_prefixes {x y z : List R} (hx : x <+: z) (hy : y <+: z) : Compat x y := by
  rcases Nat.le_total x.length y.length with h | h
  · exact .inl (List.prefix_of_prefix_length_le hx hy h)
  · exact .inr (List.prefix_of_prefix_length_le hy hx h)

theorem glue_comm {x y : List R} (h : Compat x y) : glue x y = glue y x := by
  rcases h with h | h
  · rw [glue_of_prefix h, glue_of_prefix' h]
  · rw [glue_of_prefix' h, glue_of_prefix h]

theorem glue_idem (x : List R) : glue x x = x := glue_of_prefix (List.prefix_refl x)

/-- Associativity on the values: the two neighbours compatible is enough. -/
theorem glue_assoc {x y z : List R} (hxy : Compat x y) (hyz : Compat y z) :
    glue (glue x y) z = glue x (glue y z) := by
  rcases hxy with a | a <;> rcases hyz with b | b
  · rw [glue_of_prefix a, glue_of_prefix b, glue_of_prefix (a.trans b)]
  · rw [glue_of_prefix a, glue_of_prefix' b, glue_of_prefix a]
  · rw [glue_of_prefix' a, glue_of_prefix b]
  · rw [glue_of_prefix' a, glue_of_prefix' b, glue_of_prefix' a, glue_of_prefix' (b.trans a)]

/-- THE REFUSAL IS ASSOCIATIVE TOO: gluing left-first is refused exactly when gluing right-first is,
  because the compatibility of three is one fact: pairwise. -/
theorem compat_assoc {x y z : List R} :
    (Compat x y ∧ Compat (glue x y) z) ↔ (Compat y z ∧ Compat x (glue y z)) := by
  have left : (Compat x y ∧ Compat (glue x y) z) → Compat y z ∧ Compat x z := by
    rintro ⟨hxy | hxy, h⟩
    · rw [glue_of_prefix hxy] at h
      refine ⟨h, ?_⟩
      rcases h with h | h
      · exact .inl (hxy.trans h)
      · exact compat_of_prefixes hxy h
    · rw [glue_of_prefix' hxy] at h
      refine ⟨?_, h⟩
      rcases h with h | h
      · exact .inl (hxy.trans h)
      · exact compat_of_prefixes hxy h
  have right : (Compat y z ∧ Compat x (glue y z)) → Compat x y ∧ Compat x z := by
    rintro ⟨hyz | hyz, h⟩
    · rw [glue_of_prefix hyz] at h
      refine ⟨?_, h⟩
      rcases h with h | h
      · exact compat_of_prefixes h hyz
      · exact .inr (hyz.trans h)
    · rw [glue_of_prefix' hyz] at h
      refine ⟨h, ?_⟩
      rcases h with h | h
      · exact compat_of_prefixes h hyz
      · exact .inr (hyz.trans h)
  constructor
  · intro h
    obtain ⟨hyz, hxz⟩ := left h
    refine ⟨hyz, ?_⟩
    rcases hyz with b | b
    · rw [glue_of_prefix b]; exact hxz
    · rw [glue_of_prefix' b]; exact h.1
  · intro h
    obtain ⟨hxy, hxz⟩ := right h
    refine ⟨hxy, ?_⟩
    rcases hxy with a | a
    · rw [glue_of_prefix a]; exact h.1
    · rw [glue_of_prefix' a]; exact hxz

end Frontier

/-! ### The union: no record lost, a conflict named -/

section Union

variable {R : Type} [DecidableEq R]

/-- Ours as they stand, then what they have and we lack, in their order (merge.rs `all`). -/
def unionL (a b : List R) : List R := a ++ b.filter (fun r => decide (r ∉ a))

theorem unionL_mem {a b : List R} {r : R} : r ∈ unionL a b ↔ r ∈ a ∨ r ∈ b := by
  simp only [unionL, List.mem_append, List.mem_filter, decide_eq_true_eq]
  constructor
  · rintro (h | h)
    · exact .inl h
    · exact .inr h.1
  · rintro (h | h)
    · exact .inl h
    · by_cases ha : r ∈ a
      · exact .inl ha
      · exact .inr ⟨h, ha⟩

/-- NO RECORD LOST: whatever the two sides say, conflict or none, both stay. -/
theorem no_record_lost (a b : List R) : (∀ r ∈ a, r ∈ unionL a b) ∧ (∀ r ∈ b, r ∈ unionL a b) :=
  ⟨fun _ h => unionL_mem.mpr (.inl h), fun _ h => unionL_mem.mpr (.inr h)⟩

/-- The conflicts of two tails under a clash test: `n_conf`, as a function of the tails. -/
def named (clash : R → R → Bool) (xs ys : List R) : List (R × R) :=
  xs.flatMap fun x => (ys.filter (clash x)).map (fun y => (x, y))

theorem named_mem {clash : R → R → Bool} {xs ys : List R} {x y : R} :
    (x, y) ∈ named clash xs ys ↔ x ∈ xs ∧ y ∈ ys ∧ clash x y = true := by
  simp only [named, List.mem_flatMap, List.mem_map, List.mem_filter, Prod.mk.injEq]
  constructor
  · rintro ⟨x', hx, y', ⟨hy, hc⟩, rfl, rfl⟩
    exact ⟨hx, hy, hc⟩
  · rintro ⟨hx, hy, hc⟩
    exact ⟨x, hx, y, ⟨hy, hc⟩, rfl, rfl⟩

/-- A clash on the overlap is named, never dropped. -/
theorem conflict_named {clash : R → R → Bool} {xs ys : List R} {x y : R} (hx : x ∈ xs) (hy : y ∈ ys)
    (hc : clash x y = true) : (x, y) ∈ named clash xs ys := named_mem.mpr ⟨hx, hy, hc⟩

/-- A named conflict resolves nothing: both of its records stay in the union. -/
theorem conflict_stays {clash : R → R → Bool} {xs ys a b : List R} {x y : R}
    (hxa : ∀ r ∈ xs, r ∈ a) (hyb : ∀ r ∈ ys, r ∈ b) (h : (x, y) ∈ named clash xs ys) :
    x ∈ unionL a b ∧ y ∈ unionL a b :=
  have m := named_mem.mp h
  ⟨unionL_mem.mpr (.inl (hxa _ m.1)), unionL_mem.mpr (.inr (hyb _ m.2.1))⟩

/-- merge_body.rs's state: 3 exactly when a conflict is named. -/
def gstate (clash : R → R → Bool) (xs ys : List R) : Nat := if named clash xs ys = [] then 2 else 3

theorem obstructed_iff {clash : R → R → Bool} {xs ys : List R} :
    gstate clash xs ys = 3 ↔ ∃ x y, (x, y) ∈ named clash xs ys := by
  unfold gstate
  cases h : named clash xs ys with
  | nil => simp
  | cons p l =>
    simp only [reduceCtorEq, if_false, true_iff]
    exact ⟨p.1, p.2, List.mem_cons_self⟩

/-- With nothing named, the tails commute, for any step in which unclashing events do. -/
theorem commutes_when_clean {S : Type} (step : S → R → S) (clash : R → R → Bool)
    (hstep : ∀ s x y, clash x y = false → step (step s x) y = step (step s y) x) {xs ys : List R}
    (hn : named clash xs ys = []) (s : S) : merge step s (xs ++ ys) = merge step s (ys ++ xs) := by
  have one : ∀ (x : R) (l : List R) (s : S), (∀ y ∈ l, clash x y = false) →
      merge step (step s x) l = step (merge step s l) x := by
    intro x l
    induction l with
    | nil => intro _ _; rfl
    | cons y l ih =>
      intro s h
      simp only [merge, List.foldl_cons]
      rw [hstep s x y (h y List.mem_cons_self)]
      exact ih (step s y) (fun z hz => h z (List.mem_cons_of_mem _ hz))
  have clean : ∀ x ∈ xs, ∀ y ∈ ys, clash x y = false := by
    intro x hx y hy
    cases hc : clash x y with
    | false => rfl
    | true => have := conflict_named hx hy hc; rw [hn] at this; cases this
  induction xs generalizing s with
  | nil => simp
  | cons x xs ih =>
    have hn' : named clash xs ys = [] := by
      apply List.eq_nil_iff_forall_not_mem.mpr
      rintro ⟨u, v⟩ hm
      have m := named_mem.mp hm
      have := clean u (List.mem_cons_of_mem _ m.1) v m.2.1
      rw [this] at m
      cases m.2.2
    have ih' := ih hn' (step s x) (fun u hu v hv => clean u (List.mem_cons_of_mem _ hu) v hv)
    have h1 : merge step s (x :: xs ++ ys) = merge step (step s x) (xs ++ ys) := rfl
    rw [h1, ih']
    simp only [merge, List.foldl_append, List.foldl_cons]
    congr 1
    exact one x ys s (clean x List.mem_cons_self)

end Union

/-! ### The frontier as a fold -/

section Top

variable {A R : Type} [DecidableEq A]

/-- One record moves its actor's seq up. -/
def fstep (actor : R → A) (seq : R → Nat) (f : A → Nat) (r : R) : A → Nat :=
  fun a => if a = actor r then max (f a) (seq r) else f a

/-- Per actor, the highest seq: the `(seq, tip)` a HEADS row carries, the tip being unique once there
  is no fork. -/
def front (actor : R → A) (seq : R → Nat) (l : List R) : A → Nat :=
  merge (fstep actor seq) (fun _ => 0) l

theorem fstep_comm (actor : R → A) (seq : R → Nat) (f : A → Nat) (x y : R) :
    fstep actor seq (fstep actor seq f x) y = fstep actor seq (fstep actor seq f y) x := by
  funext a
  simp only [fstep]
  by_cases h1 : a = actor y
  · by_cases h2 : a = actor x
    · simp only [if_pos h1, if_pos h2]; omega
    · simp only [if_pos h1, if_neg h2]
  · by_cases h2 : a = actor x
    · simp only [if_neg h1, if_pos h2]
    · simp only [if_neg h1, if_neg h2]

theorem front_perm (actor : R → A) (seq : R → Nat) {l₁ l₂ : List R} (hp : l₁.Perm l₂) :
    front actor seq l₁ = front actor seq l₂ :=
  merge_comm_perm _ (fstep_comm actor seq) _ hp

theorem front_le (actor : R → A) (seq : R → Nat) (l : List R) (n : Nat) (a : A) :
    front actor seq l a ≤ n ↔ ∀ r ∈ l, actor r = a → seq r ≤ n := by
  have gen : ∀ (l : List R) (f : A → Nat), merge (fstep actor seq) f l a ≤ n ↔
      f a ≤ n ∧ ∀ r ∈ l, actor r = a → seq r ≤ n := by
    intro l
    induction l with
    | nil => intro f; simp [merge]
    | cons x l ih =>
      intro f
      have hm : merge (fstep actor seq) f (x :: l) = merge (fstep actor seq) (fstep actor seq f x) l := rfl
      rw [hm, ih]
      simp only [fstep, List.mem_cons, forall_eq_or_imp]
      by_cases h : a = actor x
      · rw [if_pos h]
        constructor
        · rintro ⟨h1, h2⟩
          exact ⟨by omega, fun _ => by omega, h2⟩
        · rintro ⟨h1, h2, h3⟩
          have := h2 h.symm
          exact ⟨by omega, h3⟩
      · rw [if_neg h]
        have hne : ¬ actor x = a := fun e => h e.symm
        simp [hne]
  simpa [front] using gen l (fun _ => 0)

/-- The frontier sees only which records there are. -/
theorem front_mem_eq (actor : R → A) (seq : R → Nat) {l₁ l₂ : List R} (h : ∀ r, r ∈ l₁ ↔ r ∈ l₂) :
    front actor seq l₁ = front actor seq l₂ := by
  funext a
  apply Nat.le_antisymm
  · exact (front_le actor seq l₁ _ a).mpr fun r hr ha =>
      (front_le actor seq l₂ _ a).mp (Nat.le_refl _) r ((h r).mp hr) ha
  · exact (front_le actor seq l₂ _ a).mpr fun r hr ha =>
      (front_le actor seq l₁ _ a).mp (Nat.le_refl _) r ((h r).mpr hr) ha

variable [DecidableEq R]

theorem front_union_comm (actor : R → A) (seq : R → Nat) (a b : List R) :
    front actor seq (unionL a b) = front actor seq (unionL b a) :=
  front_mem_eq actor seq fun r => by simp only [unionL_mem, or_comm]

theorem front_union_assoc (actor : R → A) (seq : R → Nat) (a b c : List R) :
    front actor seq (unionL (unionL a b) c) = front actor seq (unionL a (unionL b c)) :=
  front_mem_eq actor seq fun r => by simp only [unionL_mem, or_assoc]

theorem front_union_idem (actor : R → A) (seq : R → Nat) (a : List R) :
    front actor seq (unionL a a) = front actor seq a :=
  front_mem_eq actor seq fun r => by simp only [unionL_mem, or_self]

/-- The union's frontier is the pointwise maximum: HEADS as a pointwise maximum. -/
theorem front_union_max (actor : R → A) (seq : R → Nat) (a b : List R) (x : A) :
    front actor seq (unionL a b) x = max (front actor seq a x) (front actor seq b x) := by
  apply Nat.le_antisymm
  · refine (front_le actor seq _ _ x).mpr fun r hr ha => ?_
    rcases unionL_mem.mp hr with h | h
    · have := (front_le actor seq a _ x).mp (Nat.le_refl _) r h ha
      exact Nat.le_trans this (Nat.le_max_left _ _)
    · have := (front_le actor seq b _ x).mp (Nat.le_refl _) r h ha
      exact Nat.le_trans this (Nat.le_max_right _ _)
  · have h1 : front actor seq a x ≤ front actor seq (unionL a b) x :=
      (front_le actor seq a _ x).mpr fun r hr ha =>
        (front_le actor seq _ _ x).mp (Nat.le_refl _) r (unionL_mem.mpr (.inl hr)) ha
    have h2 : front actor seq b x ≤ front actor seq (unionL a b) x :=
      (front_le actor seq b _ x).mpr fun r hr ha =>
        (front_le actor seq _ _ x).mp (Nat.le_refl _) r (unionL_mem.mpr (.inr hr)) ha
    exact Nat.max_le.mpr ⟨h1, h2⟩

end Top

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
#print axioms Bebop.Sync.glue_assoc
#print axioms Bebop.Sync.compat_assoc
#print axioms Bebop.Sync.commutes_when_clean
#print axioms Bebop.Sync.front_union_assoc
