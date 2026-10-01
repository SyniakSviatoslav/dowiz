/-
  Bebop.Dag -- row DG6 (docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md §9): the three statements the
  DAG runtime's scheduler and memo rest on, over the node GRAPH (not machine code, not `sys_clone`:
  formal/README.md names the sys_run / child-side gaps and they are not widened). Core Lean 4, no
  Mathlib. Lean is the CROSS-CHECK; the load-bearing checks are `dagfull sched` (sched_det: W = 0 and
  W = 3 folds equal, twice, on five shapes) and `dagfull compile` (memo hit == recompute).

  * `confluence` -- RT §7.1 T-1. Evaluating a DAG of pure nodes in ANY topological order and assembling
    the outputs BY NODE INDEX gives the same result. selfhost/prelude/sched.bp's sch_drain writes the
    node it CLAIMED into out[i]; the order nodes COMPLETE in is the order `o` here, and it does not
    matter. (The mutation proof -- dagfull.prove.sh step 5 -- assembles by completion order instead,
    which is NOT this `run`, and the gate goes red.)
  * `memo_sound` -- RT §4.1 / D-5. A memo whose every entry equals the recomputed value changes
    nothing: `runWithMemo` takes a memo hit verbatim and computes the rest.
  * `step_extends` -- RT §6.2 S-2's shape: folding the old records and then the new ones is folding
    the concatenation (List.foldl_append), which is what makes `fold_step` equal to the full fold.

  Deviations from the spec's sketch, each deliberate:
  * `IsTopo` states "every dependency of every node occurs EARLIER in o" as a prefix condition
    (o = a ++ i :: b -> deps i ⊆ a) instead of `o.indexOf j < o.indexOf i`; with `o.Nodup` (kept) the
    two say the same thing, and the prefix form needs no list-index lemmas.
  * `confluence` and `memo_sound` are proved THROUGH `val`, the node's value by strong recursion on its
    index (well-founded because deps point to smaller indices, `acyc`): every topological run equals
    `val` pointwise (`run_eq_val`). `memo_sound` therefore takes the run's topological order as a
    hypothesis, which the spec's statement leaves implicit.
  * `α` needs `Inhabited` (an unevaluated input reads `default`, never reached in a topological run).

  `#print axioms` at the bottom, MEASURED 2026-10-01 (lean 4.33.1): see the verdict of row DG6 -- the
  build log is grepped for `sorryAx` by bench/vs_rust/dagfull.sh's caller, and the count must be 0.
-/

namespace Bebop.Dag

/-- A finite DAG of total functions: node i reads only nodes with smaller index. -/
structure Dag (α : Type) where
  n    : Nat
  deps : Fin n → List (Fin n)
  acyc : ∀ i j, j ∈ deps i → j.val < i.val
  f    : Fin n → List α → α

variable {α : Type} [Inhabited α]

/-- The value of node i: its function over its inputs' values. -/
def val (g : Dag α) (i : Fin g.n) : α :=
  g.f i ((g.deps i).attach.map (fun x => val g x.1))
termination_by i.val
decreasing_by exact g.acyc i x.1 x.2

theorem attach_map_val {β γ : Type} (l : List β) (h : β → γ) :
    l.attach.map (fun x => h x.1) = l.map h := by
  simp

theorem val_eq (g : Dag α) (i : Fin g.n) : val g i = g.f i ((g.deps i).map (val g)) := by
  rw [val]
  exact congrArg (g.f i) (attach_map_val (g.deps i) (val g))

/-- One evaluation step: node i, from the memo when it has an entry, else from its inputs. -/
def stepM (g : Dag α) (memo : Fin g.n → Option α) (env : Fin g.n → Option α) (i : Fin g.n) :
    Fin g.n → Option α :=
  fun k => if k = i then some ((memo i).getD (g.f i ((g.deps i).map (fun j => (env j).getD default))))
           else env k

/-- Evaluate in order o; outputs are assembled by INDEX (a function of the node, not a list). -/
def runWithMemo (g : Dag α) (o : List (Fin g.n)) (memo : Fin g.n → Option α) : Fin g.n → α :=
  fun i => ((o.foldl (stepM g memo) (fun _ => none)) i).getD default

def run (g : Dag α) (o : List (Fin g.n)) : Fin g.n → α := runWithMemo g o (fun _ => none)

def IsTopo (g : Dag α) (o : List (Fin g.n)) : Prop :=
  o.Nodup ∧ (∀ i, i ∈ o) ∧ ∀ a i b, o = a ++ i :: b → ∀ j, j ∈ g.deps i → j ∈ a

/-- Every value present is the node's value. -/
def Good (g : Dag α) (env : Fin g.n → Option α) : Prop := ∀ k v, env k = some v → v = val g k

/-- Every dependency of every node still to run is either earlier in the list or already present. -/
def Ready (g : Dag α) (env : Fin g.n → Option α) (l : List (Fin g.n)) : Prop :=
  ∀ a i b, l = a ++ i :: b → ∀ j, j ∈ g.deps i → j ∈ a ∨ (env j).isSome = true

def MemoOk (g : Dag α) (memo : Fin g.n → Option α) : Prop := ∀ i v, memo i = some v → v = val g i

theorem map_getD_eq (g : Dag α) (env : Fin g.n → Option α) (hg : Good g env) :
    ∀ (l : List (Fin g.n)), (∀ j, j ∈ l → (env j).isSome = true) →
      l.map (fun j => (env j).getD default) = l.map (val g) := by
  intro l
  induction l with
  | nil => intro _; rfl
  | cons a t ih =>
    intro hl
    have ha : (env a).isSome = true := hl a (by simp)
    have ht : ∀ j, j ∈ t → (env j).isSome = true := fun j hj => hl j (by simp [hj])
    have hv : (env a).getD default = val g a := by
      cases h : env a with
      | none => rw [h] at ha; simp at ha
      | some v => simp; exact hg a v h
    simp only [List.map_cons, hv, ih ht]

theorem stepM_self (g : Dag α) (memo env) (i : Fin g.n) : (stepM g memo env i i).isSome = true := by
  simp [stepM]

theorem stepM_keep (g : Dag α) (memo env) (i k : Fin g.n) (h : (env k).isSome = true) :
    (stepM g memo env i k).isSome = true := by
  unfold stepM
  by_cases hk : k = i
  · simp [hk]
  · simp [hk, h]

theorem stepM_good (g : Dag α) (memo : Fin g.n → Option α) (hm : MemoOk g memo)
    (env : Fin g.n → Option α) (hg : Good g env) (i : Fin g.n)
    (hd : ∀ j, j ∈ g.deps i → (env j).isSome = true) : Good g (stepM g memo env i) := by
  intro k v hk
  unfold stepM at hk
  by_cases hki : k = i
  · subst hki
    simp at hk
    cases hmi : memo k with
    | some w => rw [hmi] at hk; simp at hk; rw [← hk]; exact hm k w hmi
    | none =>
      rw [hmi] at hk; simp at hk
      rw [← hk, val_eq g k, map_getD_eq g env hg (g.deps k) hd]
  · simp [hki] at hk
    exact hg k v hk

theorem fold_good (g : Dag α) (memo : Fin g.n → Option α) (hm : MemoOk g memo) :
    ∀ (l : List (Fin g.n)) (env : Fin g.n → Option α), Good g env → Ready g env l →
      Good g (l.foldl (stepM g memo) env) ∧
      ∀ k, ((env k).isSome = true ∨ k ∈ l) → ((l.foldl (stepM g memo) env) k).isSome = true := by
  intro l
  induction l with
  | nil =>
    intro env hg _
    refine ⟨hg, ?_⟩
    intro k hk
    cases hk with
    | inl h => exact h
    | inr h => simp at h
  | cons i t ih =>
    intro env hg hr
    have hd : ∀ j, j ∈ g.deps i → (env j).isSome = true := by
      intro j hj
      cases hr [] i t (by simp) j hj with
      | inl h => simp at h
      | inr h => exact h
    have hg' := stepM_good g memo hm env hg i hd
    have hr' : Ready g (stepM g memo env i) t := by
      intro a i' b ht j hj
      cases hr (i :: a) i' b (by simp [ht]) j hj with
      | inl h =>
        cases List.mem_cons.mp h with
        | inl he => right; rw [he]; exact stepM_self g memo env i
        | inr ha => left; exact ha
      | inr h => right; exact stepM_keep g memo env i j h
    obtain ⟨hgood, hcov⟩ := ih (stepM g memo env i) hg' hr'
    refine ⟨by simpa using hgood, ?_⟩
    intro k hk
    simp only [List.foldl_cons]
    apply hcov k
    cases hk with
    | inl h => left; exact stepM_keep g memo env i k h
    | inr h =>
      cases List.mem_cons.mp h with
      | inl he => left; rw [he]; exact stepM_self g memo env i
      | inr ht => right; exact ht

/-- Every topological run, with a sound memo, is `val` pointwise. -/
theorem runWithMemo_eq_val (g : Dag α) (o : List (Fin g.n)) (h : IsTopo g o)
    (memo : Fin g.n → Option α) (hm : MemoOk g memo) (i : Fin g.n) :
    runWithMemo g o memo i = val g i := by
  have hg0 : Good g (fun _ => none) := by intro k v hk; simp at hk
  have hr0 : Ready g (fun _ => none) o := by
    intro a i' b ho j hj
    left; exact h.2.2 a i' b ho j hj
  obtain ⟨hgood, hcov⟩ := fold_good g memo hm o (fun _ => none) hg0 hr0
  have hs := hcov i (Or.inr (h.2.1 i))
  unfold runWithMemo
  cases he : (o.foldl (stepM g memo) (fun _ => none)) i with
  | none => rw [he] at hs; simp at hs
  | some v => simp; exact hgood i v he

theorem run_eq_val (g : Dag α) (o : List (Fin g.n)) (h : IsTopo g o) (i : Fin g.n) :
    run g o i = val g i :=
  runWithMemo_eq_val g o h (fun _ => none) (by intro k v hk; simp at hk) i

/-- T-1: any two topological orders give the same index-assembled outputs. -/
theorem confluence (g : Dag α) (o₁ o₂ : List (Fin g.n)) (h₁ : IsTopo g o₁) (h₂ : IsTopo g o₂) :
    run g o₁ = run g o₂ := by
  funext i
  rw [run_eq_val g o₁ h₁ i, run_eq_val g o₂ h₂ i]

/-- D-5: a memo whose every entry is the recomputed value changes nothing. -/
theorem memo_sound (g : Dag α) (o : List (Fin g.n)) (h : IsTopo g o) (memo : Fin g.n → Option α)
    (hm : ∀ i v, memo i = some v → v = run g o i) : runWithMemo g o memo = run g o := by
  have hm' : MemoOk g memo := by
    intro i v hv; rw [hm i v hv, run_eq_val g o h i]
  funext i
  rw [runWithMemo_eq_val g o h memo hm' i, run_eq_val g o h i]

/-- S-2's shape: the old fold extended by the new records is the fold of the concatenation. -/
theorem step_extends {β γ : Type} (step : β → γ → β) (init : β) (xs ys : List γ) :
    List.foldl step (List.foldl step init xs) ys = List.foldl step init (xs ++ ys) := by
  rw [List.foldl_append]

end Bebop.Dag

#print axioms Bebop.Dag.confluence
#print axioms Bebop.Dag.memo_sound
#print axioms Bebop.Dag.step_extends
#print axioms Bebop.Dag.run_eq_val
