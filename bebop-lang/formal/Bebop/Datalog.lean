/-
  Bebop.Datalog -- row DG8 (docs/design/SPEC-DATALOG-AND-CODEC-2026-09-28.md A.8): the two statements the
  Datalog layer rests on, proved in core Lean 4 (this project has no Mathlib: facts are predicates, not
  Finsets). Lean is the CROSS-CHECK, never load-bearing: the load-bearing check is `dagfull datalog`
  (A.7, incremental == from scratch over 10^4 events per rule set) and the dl_* gates.

  1. SEMI-NAIVE = NAIVE (selfhost/std/dl_eval.bp dl_semi / dl_round). Naive iteration is
     N0 = {}, N(k+1) = N k u T(N k). Semi-naive keeps (S, Delta): S0 = {}, Delta0 = T {},
     S(k+1) = S k u Delta k, Delta(k+1) = D(S(k+1), Delta k) \ S(k+1), where D is the DELTA OPERATOR --
     every rule evaluated with one body atom read from the last round's delta (dl_round_rule). The
     hypotheses are exactly what makes D a delta operator for T: COMPLETE (a derivation over S u Delta
     either uses no Delta fact, so it is in T S, or it uses one, so D finds it) and SOUND (D derives
     nothing T does not). Under them the two iterations agree at EVERY round (`seminaive_eq_naive`), so
     they stop at the same round with the same set; with T monotone that set is a fixpoint below every
     prefixed point -- the least fixpoint (`seminaive_lfp`). Finiteness of the domain (the spec's
     `hD : T s <= D`) is what makes the rounds stop; the engine does not trust it -- it traps 125 past
     1 + the IDB capacities (A-6, gate dl_bound) -- so it is not a hypothesis here.
     The spec states the theorem for a monotone T and leaves the semi-naive step implicit; here the step
     is an explicit operator D, and what it must satisfy (DeltaOp) is a hypothesis, not an assumption
     buried in a definition. The engine's D is dl_round: each rule once per same-stratum body atom read
     from the delta, every other atom from F -- complete and sound for rules whose bodies are joins.

  2. STRATIFICATION (selfhost/std/dl.bp dl_negcheck / dl_strata). A predicate graph with an edge p -> q
     for each body atom q of a rule with head p, some edges negative. dl.bp assigns each SCC one stratum
     (Kahn over the condensation), so every edge goes to an equal or lower stratum, and equal only inside
     an SCC. `neg_strictly_lower`: if no negative edge lies inside an SCC (the E125 check), every negated
     predicate is in a STRICTLY lower stratum -- complete before its stratum starts (A-5), so each
     stratum's operator is monotone in the stratum's own facts. `refusal_necessary`: conversely a
     negative edge inside an SCC defeats EVERY ranking that edges do not climb, so refusing it (E125) is
     not over-cautious. `stratum_model_unique`: a monotone stratum operator whose naive iteration
     stabilises has exactly one least model -- with the strata taken lowest first, the model of the
     whole program is unique (the perfect model).

  `#print axioms` at the bottom, MEASURED 2026-10-01 (`lake build` rc 0, 15 jobs; `lake env lean
  Bebop/Datalog.lean` rc 0, lean 4.33.1) -- standard axioms only, no `sorryAx`, no native/bv_decide axiom:
    seminaive_eq_naive      [propext, Classical.choice, Quot.sound]
    seminaive_lfp           [propext, Classical.choice, Quot.sound]
    stratum_model_unique    [propext, Quot.sound]
    neg_strictly_lower      does not depend on any axioms
    refusal_necessary       does not depend on any axioms
    stratified_well_defined [propext, Quot.sound]
  (Classical.choice is excluded middle in semi_invariant's last case: a fact either is already in S or
  is new.)
-/

namespace Bebop.Datalog

universe u

/-- A set of facts, as a predicate (no Mathlib `Set`/`Finset` in this project). -/
abbrev FSet (α : Type u) : Type u := α → Prop

variable {α : Type u}

abbrev empty : FSet α := fun _ => False
abbrev union (A B : FSet α) : FSet α := fun x => A x ∨ B x
abbrev diff (A B : FSet α) : FSet α := fun x => A x ∧ ¬ B x
abbrev SubF (A B : FSet α) : Prop := ∀ x, A x → B x
abbrev Mono (T : FSet α → FSet α) : Prop := ∀ A B, SubF A B → SubF (T A) (T B)

/-- Naive iteration: N 0 = {}, N (k+1) = N k u T (N k). -/
def naive (T : FSet α → FSet α) : Nat → FSet α
  | 0 => empty
  | k + 1 => union (naive T k) (T (naive T k))

/-- Semi-naive iteration: (S, Delta) with S 0 = {}, Delta 0 = T {},
    S (k+1) = S k u Delta k, Delta (k+1) = D (S (k+1)) (Delta k) \ S (k+1). -/
def semi (T : FSet α → FSet α) (D : FSet α → FSet α → FSet α) : Nat → FSet α × FSet α
  | 0 => (empty, T empty)
  | k + 1 =>
    (union (semi T D k).1 (semi T D k).2,
     diff (D (union (semi T D k).1 (semi T D k).2) (semi T D k).2) (union (semi T D k).1 (semi T D k).2))

/-- D is a delta operator for T: complete and sound over S u Delta. -/
abbrev DeltaOp (T : FSet α → FSet α) (D : FSet α → FSet α → FSet α) : Prop :=
  (∀ S Δ, SubF (T (union S Δ)) (union (T S) (D (union S Δ) Δ))) ∧
  (∀ S Δ, SubF (D (union S Δ) Δ) (T (union S Δ)))

theorem semi_invariant (T : FSet α → FSet α) (D : FSet α → FSet α → FSet α) (hD : DeltaOp T D) :
    ∀ k, (semi T D k).1 = naive T k ∧ union (semi T D k).1 (semi T D k).2 = naive T (k + 1) := by
  intro k
  induction k with
  | zero => exact ⟨rfl, rfl⟩
  | succ k ih =>
    obtain ⟨h1, h2⟩ := ih
    refine ⟨h2, ?_⟩
    show union (union (semi T D k).1 (semi T D k).2)
        (diff (D (union (semi T D k).1 (semi T D k).2) (semi T D k).2)
          (union (semi T D k).1 (semi T D k).2))
      = union (naive T (k + 1)) (T (naive T (k + 1)))
    rw [← h2]
    funext x
    apply propext
    constructor
    · intro hx
      cases hx with
      | inl hU => exact Or.inl hU
      | inr hd => exact Or.inr (hD.2 _ _ x hd.1)
    · intro hx
      cases hx with
      | inl hU => exact Or.inl hU
      | inr hT =>
        cases hD.1 _ _ x hT with
        | inl hTs =>
          have hn : naive T (k + 1) x := by
            show union (naive T k) (T (naive T k)) x
            rw [← h1]
            exact Or.inr hTs
          rw [← h2] at hn
          exact Or.inl hn
        | inr hDx =>
          cases Classical.em (union (semi T D k).1 (semi T D k).2 x) with
          | inl hin => exact Or.inl hin
          | inr hout => exact Or.inr ⟨hDx, hout⟩

/-- Semi-naive and naive iteration agree at every round (SPEC A.8, first statement). -/
theorem seminaive_eq_naive (T : FSet α → FSet α) (D : FSet α → FSet α → FSet α) (hD : DeltaOp T D) :
    ∀ k, (semi T D k).1 = naive T k :=
  fun k => (semi_invariant T D hD k).1

/-- Every naive iterate of a monotone T lies below every prefixed point. -/
theorem naive_below (T : FSet α → FSet α) (hm : Mono T) (P : FSet α) (hP : SubF (T P) P) :
    ∀ k, SubF (naive T k) P := by
  intro k
  induction k with
  | zero => intro x hx; exact False.elim hx
  | succ k ih =>
    intro x hx
    cases hx with
    | inl h => exact ih x h
    | inr h => exact hP x (hm _ _ ih x h)

/-- The least model: a prefixed point below every prefixed point. -/
abbrev LeastPrefixed (T : FSet α → FSet α) (M : FSet α) : Prop :=
  SubF (T M) M ∧ ∀ P, SubF (T P) P → SubF M P

/-- When the naive iteration stops at round k, the semi-naive set of round k is the least fixpoint. -/
theorem seminaive_lfp (T : FSet α → FSet α) (D : FSet α → FSet α → FSet α) (hD : DeltaOp T D)
    (hm : Mono T) (k : Nat) (hstab : naive T (k + 1) = naive T k) :
    LeastPrefixed T (semi T D k).1 := by
  rw [seminaive_eq_naive T D hD k]
  refine ⟨?_, fun P hP => naive_below T hm P hP k⟩
  intro x hx
  have h : naive T (k + 1) x := Or.inr hx
  rw [hstab] at h
  exact h

/-- Two least models are one: SubF both ways, then extensionality. -/
theorem lfp_unique (T : FSet α → FSet α) (M N : FSet α)
    (hM : LeastPrefixed T M) (hN : LeastPrefixed T N) : M = N := by
  funext x
  apply propext
  exact ⟨hM.2 N hN.1 x, hN.2 M hM.1 x⟩

/-- A monotone stratum operator whose naive iteration stabilises has exactly one least model. -/
theorem stratum_model_unique (T : FSet α → FSet α) (hm : Mono T) (k : Nat)
    (hstab : naive T (k + 1) = naive T k) :
    ∃ M, LeastPrefixed T M ∧ ∀ M', LeastPrefixed T M' → M' = M := by
  have hl : LeastPrefixed T (naive T k) := by
    refine ⟨?_, fun P hP => naive_below T hm P hP k⟩
    intro x hx
    have h : naive T (k + 1) x := Or.inr hx
    rw [hstab] at h
    exact h
  exact ⟨naive T k, hl, fun M' hM' => lfp_unique T M' (naive T k) hM' hl⟩

-- ---- stratification ----------------------------------------------------------------------------------

/-- The predicate graph: `dep p q` when a rule with head p has body atom q; `neg p q` when negated. -/
structure PredGraph where
  dep : Nat → Nat → Prop
  neg : Nat → Nat → Prop
  neg_dep : ∀ p q, neg p q → dep p q

/-- One or more dependency steps (dl.bp dl_closure's transitive closure). -/
inductive Reach (G : PredGraph) : Nat → Nat → Prop where
  | step {p q : Nat} : G.dep p q → Reach G p q
  | trans {p q r : Nat} : Reach G p q → Reach G q r → Reach G p r

/-- dl.bp dl_same_scc: the same predicate, or each reaches the other. -/
abbrev SameSCC (G : PredGraph) (p q : Nat) : Prop := p = q ∨ (Reach G p q ∧ Reach G q p)

/-- The E125 condition of A-4: no negative edge inside an SCC. -/
abbrev NoNegativeEdgeInSCC (G : PredGraph) : Prop := ∀ p q, G.neg p q → ¬ SameSCC G p q

/-- What dl_strata's Kahn order over the condensation gives: edges never climb, and an edge stays in
    one stratum only inside one SCC. -/
structure Stratification (G : PredGraph) where
  r : Nat → Nat
  down : ∀ p q, G.dep p q → r q ≤ r p
  flat : ∀ p q, G.dep p q → r q = r p → SameSCC G p q

/-- With no negative edge inside an SCC, every negated predicate sits in a STRICTLY lower stratum. -/
theorem neg_strictly_lower (G : PredGraph) (h : NoNegativeEdgeInSCC G) (S : Stratification G) :
    ∀ p q, G.neg p q → S.r q < S.r p := by
  intro p q hn
  have hd := G.neg_dep p q hn
  have hle := S.down p q hd
  cases Nat.lt_or_ge (S.r q) (S.r p) with
  | inl hlt => exact hlt
  | inr hge =>
    exact False.elim (h p q hn (S.flat p q hd (Nat.le_antisymm hle hge)))

theorem reach_le (G : PredGraph) (r : Nat → Nat) (down : ∀ p q, G.dep p q → r q ≤ r p) :
    ∀ p q, Reach G p q → r q ≤ r p := by
  intro p q h
  induction h with
  | step hd => exact down _ _ hd
  | trans _ _ ih1 ih2 => exact Nat.le_trans ih2 ih1

/-- The refusal is necessary: a negative edge inside an SCC defeats every ranking edges do not climb,
    so no evaluation order can make the negated predicate complete first. -/
theorem refusal_necessary (G : PredGraph) (r : Nat → Nat) (down : ∀ p q, G.dep p q → r q ≤ r p)
    (p q : Nat) (hs : SameSCC G p q) : ¬ r q < r p := by
  intro hlt
  cases hs with
  | inl heq => rw [heq] at hlt; exact Nat.lt_irrefl _ hlt
  | inr hr =>
    have hqp := reach_le G r down q p hr.2
    exact Nat.lt_irrefl _ (Nat.lt_of_lt_of_le hlt hqp)

/-- A stratification makes each stratum's model well-defined and unique (SPEC A.8, second statement,
    in the form this layer uses): negated inputs are strictly lower (complete first), and a monotone
    stratum operator that stabilises has one least model. -/
theorem stratified_well_defined (G : PredGraph) (h : NoNegativeEdgeInSCC G) (S : Stratification G)
    (T : FSet α → FSet α) (hm : Mono T) (k : Nat) (hstab : naive T (k + 1) = naive T k) :
    (∀ p q, G.neg p q → S.r q < S.r p) ∧
    ∃ M, LeastPrefixed T M ∧ ∀ M', LeastPrefixed T M' → M' = M :=
  ⟨neg_strictly_lower G h S, stratum_model_unique T hm k hstab⟩

end Bebop.Datalog

#print axioms Bebop.Datalog.seminaive_eq_naive
#print axioms Bebop.Datalog.seminaive_lfp
#print axioms Bebop.Datalog.stratum_model_unique
#print axioms Bebop.Datalog.neg_strictly_lower
#print axioms Bebop.Datalog.refusal_necessary
#print axioms Bebop.Datalog.stratified_well_defined
