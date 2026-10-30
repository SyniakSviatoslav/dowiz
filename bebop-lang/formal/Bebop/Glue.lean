/-
  Bebop.Glue -- W-ES-P19 (operator decision 2026-10-30): GLUING IS THE SINGLE CONTRACT OF THE
  WRITE PATH. Core Lean 4, no Mathlib, no `axiom`, no `sorry`.

  THE CODE IT MODELS (crates/bebop-kv/src/glue.rs, the library operation; its instances in
  crates/bebop-io's commit section and the tree 3-way merge): two SECTIONS of one object over a
  common BASE -- each section is the base plus a delta; a section's RESTRICTION to the overlap is
  the set of keys its delta touches (`deltaKeys`); the sections GLUE when the restrictions do not
  disagree, i.e. no key is touched by both. Then the two deltas COMMUTE: the glued section is the
  base plus both, in either order. A shared key is an obstruction, and a named resolver answers it
  (refuse = `Refusal::Conflict`, the default; descend into the one child both rewrote, for a tree
  node; a person, for the kitchen). Nothing here resolves: gluing detects.

  PROVED:
    glue_commutes   THE GLUING THEOREM: deltas with disjoint key sets commute on any image.
    occ_glues       instance OCC: a transaction whose keys none of the commits since its base
                    touched replays to the same image after them as before them -- the rebase of
                    its path copy onto the current root is exact.
    group_glues     instance GROUP: two groups of commits (P11's flush groups, any size) with
                    disjoint key sets commute as wholes.
    merge_glues     instance TREE MERGE: a node is an image over slot positions; the two sides'
                    rewrites are deltas on it; disjoint replacement spans commute, so the merged
                    node is the old node plus both sides' replacements. A shared slot is the one
                    obstruction, resolved by descending into that child -- the same theorem one
                    level down (`merge_glues` again, on the child's slots), and on a leaf on the
                    entries' keys.

  WHAT THE THEOREM DOES NOT SAY (and the Rust docstring says the same): OCC also needs that the
  transaction's delta was COMPUTED from a read set no later commit touched -- that is the
  validation rule (ReadSet ∩ later WriteSets = ∅, the restriction check in glue.rs), which gives
  `t` as a fixed delta here. The commutation is then this theorem, with `deltaKeys t ⊆ ReadSet`.
-/
import Bebop.KvDelta

namespace Bebop.KvDelta

variable {K V : Type} [DecidableEq K]

omit [DecidableEq K] in
/-- The keys of `o :: d`, unfolded. -/
theorem deltaKeys_cons (o : Op K V) (d : Delta K V) : deltaKeys (o :: d) = o.key :: deltaKeys d := rfl

/-- A key one of the ops touched ends with the value its LAST op on it gave: the starting image is
  irrelevant for that key. -/
theorem applyDelta_in (d : Delta K V) : ∀ (m m' : Image K V) (j : K), j ∈ deltaKeys d →
    applyDelta m d j = applyDelta m' d j := by
  induction d with
  | nil =>
    intro m m' j h
    simp [deltaKeys] at h
  | cons o os ih =>
    intro m m' j h
    rw [applyDelta_cons, applyDelta_cons]
    by_cases hj : j ∈ deltaKeys os
    · exact ih _ _ j hj
    · have hk : j = o.key := by
        rw [deltaKeys_cons] at h
        exact (List.mem_cons.mp h).resolve_right hj
      rw [applyDelta_notin _ _ _ hj, applyDelta_notin _ _ _ hj]
      subst hk
      cases o <;> simp [step, Op.key]

/-- THE GLUING THEOREM. Two deltas whose key sets are disjoint commute on every image. -/
theorem glue_commutes (m : Image K V) (d₁ d₂ : Delta K V)
    (h : ∀ k, k ∈ deltaKeys d₁ → k ∉ deltaKeys d₂) :
    applyDelta (applyDelta m d₁) d₂ = applyDelta (applyDelta m d₂) d₁ := by
  funext j
  by_cases h1 : j ∈ deltaKeys d₁
  · have h2 : j ∉ deltaKeys d₂ := h j h1
    rw [applyDelta_notin _ _ _ h2]
    exact applyDelta_in d₁ m (applyDelta m d₂) j h1
  · by_cases h2 : j ∈ deltaKeys d₂
    · rw [applyDelta_notin _ _ _ h1]
      exact applyDelta_in d₂ (applyDelta m d₁) m j h2
    · rw [applyDelta_notin _ _ _ h2, applyDelta_notin _ _ _ h1, applyDelta_notin _ _ _ h1,
        applyDelta_notin _ _ _ h2]

/-- Instance OCC: the transaction `t` against the chain `cs` of commits that landed since its base. -/
theorem occ_glues (m : Image K V) (t : Delta K V) (cs : Chain K V)
    (h : ∀ k, k ∈ deltaKeys t → k ∉ keys cs) :
    applyDelta (replayFrom m cs) t = replayFrom (applyDelta m t) cs := by
  induction cs generalizing m with
  | nil => rfl
  | cons d cs ih =>
    have hd : ∀ k, k ∈ deltaKeys t → k ∉ deltaKeys d := by
      intro k hk hkd
      exact h k hk (List.mem_append.mpr (Or.inl hkd))
    have hcs : ∀ k, k ∈ deltaKeys t → k ∉ keys cs := by
      intro k hk hkc
      exact h k hk (List.mem_append.mpr (Or.inr hkc))
    rw [replayFrom_cons, replayFrom_cons, ih (applyDelta m d) hcs, glue_commutes m t d hd]

omit [DecidableEq K] in
/-- `deltaKeys` over a concatenation. -/
theorem deltaKeys_append (d₁ d₂ : Delta K V) : deltaKeys (d₁ ++ d₂) = deltaKeys d₁ ++ deltaKeys d₂ := by
  induction d₁ with
  | nil => rfl
  | cons o os ih => simp [deltaKeys_cons, ih]

omit [DecidableEq K] in
/-- A chain's keys are its flattening's keys. -/
theorem keys_flatten (c : Chain K V) : keys c = deltaKeys c.flatten := by
  induction c with
  | nil => rfl
  | cons d cs ih => simp [keys, List.flatten_cons, deltaKeys_append, ih]

/-- Instance GROUP: two flush groups with disjoint key sets commute as wholes. -/
theorem group_glues (m : Image K V) (g₁ g₂ : Chain K V)
    (h : ∀ k, k ∈ keys g₁ → k ∉ keys g₂) :
    replayFrom (replayFrom m g₁) g₂ = replayFrom (replayFrom m g₂) g₁ := by
  rw [replayFrom_eq_flatten, replayFrom_eq_flatten, replayFrom_eq_flatten, replayFrom_eq_flatten]
  apply glue_commutes
  intro k hk
  rw [← keys_flatten] at hk ⊢
  exact h k hk

/-- Instance TREE MERGE: a node as an image over slot positions (`Nat`), a side's rewrite as a
  delta on it (a replacement of slot `i` is `put i`, a dropped slot `del i`); disjoint replacement
  spans glue. The merged node of `glue.rs`'s tree instance is `applyDelta (applyDelta node a) b`.
  A shared slot is the obstruction the `Descend` resolver answers by the same theorem on the
  child's slots; on a leaf, with `K` the entry keys. -/
theorem merge_glues {S : Type} (node : Image Nat S) (a b : Delta Nat S)
    (h : ∀ i, i ∈ deltaKeys a → i ∉ deltaKeys b) :
    applyDelta (applyDelta node a) b = applyDelta (applyDelta node b) a :=
  glue_commutes node a b h

end Bebop.KvDelta
