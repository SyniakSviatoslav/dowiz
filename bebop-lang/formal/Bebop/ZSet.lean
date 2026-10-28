/-
  Bebop.ZSet -- LV-INC-D (docs/research/R-BEBOP-LEVERS-2026-10-27.md §2.4, AC-LV-INC-08): Z-SETS ARE
  THE ONE DELTA CURRENCY (operator Q2, 2026-10-27), and "the incremental result = the full recompute"
  for every operator shape the levers use. Core Lean 4, no Mathlib, no `axiom`, no `sorry`.

  THE MODEL (DBSP, Budiu et al. PVLDB 16(7) 2023; formalisation tchajed/dbsp-theory):
    * a Z-set is a function `α → Int` (a weight per row); `add`/`neg`/`zero` make it a group;
    * `ofList` is the finite-support form the Rust side keeps (`crates/bebop-store/src/zset.rs`, a
      sorted `Vec<(row, i64)>`): a list of (row, weight) pairs, summed; `ofList_append` says
      concatenating two such lists adds the Z-sets, so sorting/merging them is an implementation detail;
    * `logTail` is `from_log_tail`: records after `tip`, every weight +1;
    * `integrate base Δs` is DBSP's I: the deltas added onto a base, oldest first. Compaction is
      integration (`compact_is_integral` below, from `KvDelta.replay_compact`).

  PROVED:
    memo_step_eq_full   any memo Q with a step obeying `step (Q z) d = Q (z + d)` (what the Rust gate
                        `step(full(a), Δ) == full(a + Δ)` checks on one Δ) gives the full recompute
                        after ANY chain -- PROJ kind 4. Q need not be linear (distinct via counts is).
    linear_incremental  a LINEAR Q (map/filter/project/sum_by): Q(I Δs) = fold (m + Q Δ) -- Q^Δ = Q.
    join_incremental    a BILINEAR J: J(a+Δa, b+Δb) = J a b + J Δa b + J a Δb + J Δa Δb; `join` (the
                        equi-join on a key, CSR join's semantics) is bilinear (`join_bilinear`).
    join_stream         the three-term step, folded over a stream of (Δa, Δb), ends at J (ΣA) (ΣB).
    distinct_not_linear the named counterexample: distinct must go through weights (counts), not Q^Δ = Q.
    step_is_zset / kv_chain_is_zset   the bridge to Bebop.KvDelta: a put is +(k,v_new) −(k,v_old), a
                        remove is −(k,v_old), `old` read from the image before the op (as
                        `from_kv_chain` reads it from the base); the image after a batch is the image
                        before plus the batch's delta.
    compact_is_integral the compacted chain has the same Z-set.

  NOT COVERED: bytes (K64 row layout, sorting, i64 overflow of a weight -- Lean's Int is unbounded),
  the refold-on-foreign-tip rule S-4 and redaction (decisions about WHEN to step, not whether a step
  is sound). The Rust twin that runs the same named chain is a hand-back to LV-INC-A.
-/
import Bebop.KvDelta

namespace Bebop.ZSet

variable {α β γ : Type}

/-- A weight per row. -/
abbrev ZSet (α : Type) := α → Int

def zero : ZSet α := fun _ => 0
def add (a b : ZSet α) : ZSet α := fun x => a x + b x
def neg (a : ZSet α) : ZSet α := fun x => - a x
def single [DecidableEq α] (x : α) (w : Int) : ZSet α := fun y => if y = x then w else 0

theorem add_assoc (a b c : ZSet α) : add (add a b) c = add a (add b c) := by
  funext x; simp only [add]; omega
theorem add_comm (a b : ZSet α) : add a b = add b a := by
  funext x; simp only [add]; omega
theorem zero_add (a : ZSet α) : add zero a = a := by
  funext x; simp only [add, zero]; omega
theorem add_zero (a : ZSet α) : add a zero = a := by
  funext x; simp only [add, zero]; omega
theorem add_neg (a : ZSet α) : add a (neg a) = zero := by
  funext x; simp only [add, neg, zero]; omega

/-! ### The finite-support form (Rust's sorted Vec) -/

def ofList [DecidableEq α] : List (α × Int) → ZSet α
  | [] => zero
  | (x, w) :: l => add (single x w) (ofList l)

theorem ofList_append [DecidableEq α] : ∀ (l₁ l₂ : List (α × Int)),
    ofList (l₁ ++ l₂) = add (ofList l₁) (ofList l₂)
  | [], l₂ => by show ofList l₂ = add zero (ofList l₂); rw [zero_add]
  | (x, w) :: l₁, l₂ => by
    show add (single x w) (ofList (l₁ ++ l₂)) = add (add (single x w) (ofList l₁)) (ofList l₂)
    rw [ofList_append l₁ l₂, add_assoc]

/-- `from_log_tail`: the records after `tip`, each with weight +1. -/
def logTail (recs : List α) : List (α × Int) := recs.map (fun r => (r, 1))

theorem logTail_append [DecidableEq α] (xs ys : List α) :
    ofList (logTail (xs ++ ys)) = add (ofList (logTail xs)) (ofList (logTail ys)) := by
  simp only [logTail, List.map_append]; exact ofList_append _ _

/-- DBSP's I: the deltas added onto a base, oldest first. -/
def integrate (base : ZSet α) (ds : List (ZSet α)) : ZSet α := ds.foldl add base

/-! ### Memo = full recompute (PROJ kind 4) -/

/-- THE kind-4 theorem: a step that is right for ONE delta is right for every chain. -/
theorem memo_step_eq_full {M : Type} (Q : ZSet α → M) (step : M → ZSet α → M)
    (hstep : ∀ z d, step (Q z) d = Q (add z d)) :
    ∀ (base : ZSet α) (chain : List (ZSet α)), chain.foldl step (Q base) = Q (integrate base chain)
  | _, [] => rfl
  | base, d :: ds => by
    show ds.foldl step (step (Q base) d) = Q (ds.foldl add (add base d))
    rw [hstep]; exact memo_step_eq_full Q step hstep (add base d) ds

/-! ### Linear operators: Q^Δ = Q -/

def Linear (Q : ZSet α → ZSet β) : Prop := ∀ a b, Q (add a b) = add (Q a) (Q b)

theorem linear_zero {Q : ZSet α → ZSet β} (h : Linear Q) : Q zero = zero := by
  have h0 := h zero zero
  rw [zero_add] at h0
  funext x
  have hx := congrFun h0 x
  simp only [add] at hx
  show Q zero x = 0
  omega

theorem linear_incremental {Q : ZSet α → ZSet β} (h : Linear Q) (base : ZSet α) (ds : List (ZSet α)) :
    Q (integrate base ds) = ds.foldl (fun m d => add m (Q d)) (Q base) :=
  (memo_step_eq_full Q (fun m d => add m (Q d)) (fun z d => (h z d).symm) base ds).symm

/-- From nothing: Q (I Δs) = the fold of Q over the deltas. -/
theorem linear_incremental_zero {Q : ZSet α → ZSet β} (h : Linear Q) (ds : List (ZSet α)) :
    Q (integrate zero ds) = ds.foldl (fun m d => add m (Q d)) zero := by
  rw [linear_incremental h, linear_zero h]

def filter (p : α → Bool) (a : ZSet α) : ZSet α := fun x => if p x then a x else 0

theorem filter_linear (p : α → Bool) : Linear (filter (α := α) p) := by
  intro a b; funext x; simp only [filter, add]
  cases p x <;> simp

/-- map/project on the finite-support form: one row per row, weights carried. -/
def mapL (f : α → β) (l : List (α × Int)) : List (β × Int) := l.map (fun p => (f p.1, p.2))

theorem mapL_linear [DecidableEq β] (f : α → β) (l₁ l₂ : List (α × Int)) :
    ofList (mapL f (l₁ ++ l₂)) = add (ofList (mapL f l₁)) (ofList (mapL f l₂)) := by
  simp only [mapL, List.map_append]; exact ofList_append _ _

/-! ### Bilinear operators: join -/

def Bilinear (J : ZSet α → ZSet β → ZSet γ) : Prop :=
  (∀ a a' b, J (add a a') b = add (J a b) (J a' b)) ∧
  (∀ a b b', J a (add b b') = add (J a b) (J a b'))

theorem join_incremental {J : ZSet α → ZSet β → ZSet γ} (h : Bilinear J)
    (a da : ZSet α) (b db : ZSet β) :
    J (add a da) (add b db) = add (add (add (J a b) (J da b)) (J a db)) (J da db) := by
  rw [h.1, h.2, h.2]
  funext x; simp only [add]; omega

/-- The equi-join on a key: weights multiply (CSR join's semantics). -/
def join [DecidableEq γ] (k₁ : α → γ) (k₂ : β → γ) (a : ZSet α) (b : ZSet β) : ZSet (α × β) :=
  fun p => if k₁ p.1 = k₂ p.2 then a p.1 * b p.2 else 0

theorem join_bilinear [DecidableEq γ] (k₁ : α → γ) (k₂ : β → γ) : Bilinear (join k₁ k₂) := by
  constructor
  · intro a a' b; funext p; simp only [join, add]
    split
    · exact Int.add_mul _ _ _
    · omega
  · intro a b b'; funext p; simp only [join, add]
    split
    · exact Int.mul_add _ _ _
    · omega

/-- One step of the incremental join: the state (A, B, A ⋈ B) and a pair of deltas. -/
def joinStep (J : ZSet α → ZSet β → ZSet γ) (s : ZSet α × ZSet β × ZSet γ) (d : ZSet α × ZSet β) :
    ZSet α × ZSet β × ZSet γ :=
  (add s.1 d.1, add s.2.1 d.2, add (add (add s.2.2 (J d.1 s.2.1)) (J s.1 d.2)) (J d.1 d.2))

theorem join_stream {J : ZSet α → ZSet β → ZSet γ} (h : Bilinear J) :
    ∀ (a : ZSet α) (b : ZSet β) (ds : List (ZSet α × ZSet β)),
    ds.foldl (joinStep J) (a, b, J a b) =
      (integrate a (ds.map Prod.fst), integrate b (ds.map Prod.snd),
       J (integrate a (ds.map Prod.fst)) (integrate b (ds.map Prod.snd)))
  | _, _, [] => rfl
  | a, b, d :: ds => by
    have hj : joinStep J (a, b, J a b) d = (add a d.1, add b d.2, J (add a d.1) (add b d.2)) := by
      show (add a d.1, add b d.2, add (add (add (J a b) (J d.1 b)) (J a d.2)) (J d.1 d.2)) = _
      rw [join_incremental h]
    show ds.foldl (joinStep J) (joinStep J (a, b, J a b) d) = _
    rw [hj]; exact join_stream h (add a d.1) (add b d.2) ds

/-! ### Distinct is not linear: it goes through weights -/

def distinct (a : ZSet α) : ZSet α := fun x => if 0 < a x then 1 else 0

theorem distinct_not_linear : ¬ Linear (distinct (α := Unit)) := by
  intro h
  have hx := congrFun (h (fun _ => 1) (fun _ => 1)) ()
  simp [distinct, add] at hx

/-! ### The bridge to Bebop.KvDelta: a KV chain IS a Z-set delta stream -/

section kv
open Bebop.KvDelta
variable {K V : Type} [DecidableEq K] [DecidableEq V]

/-- The rows a reader sees: (k, v) has weight 1 when the image maps k to v. -/
def zsetOf (m : Image K V) : ZSet (K × V) := fun p => if m p.1 = some p.2 then 1 else 0

/-- The row the image holds for `k` before an op, as a Z-set (empty when absent). -/
def oldOf (m : Image K V) (k : K) : ZSet (K × V) :=
  match m k with
  | some v => single (k, v) 1
  | none => zero

/-- One op as a Z-set delta, `old` read from the image before the op. -/
def deltaOf (m : Image K V) : Op K V → ZSet (K × V)
  | .put k v => add (single (k, v) 1) (neg (oldOf m k))
  | .del k => neg (oldOf m k)

/-- One batch: each op's delta against the running image. -/
def deltaOfBatch : Image K V → Delta K V → ZSet (K × V)
  | _, [] => zero
  | m, o :: os => add (deltaOf m o) (deltaOfBatch (KvDelta.step m o) os)

theorem step_is_zset (m : Image K V) (o : Op K V) :
    zsetOf (KvDelta.step m o) = add (zsetOf m) (deltaOf m o) := by
  funext ⟨j, w⟩
  cases o with
  | put k v =>
    by_cases hj : j = k
    · subst hj
      cases hm : m j with
      | none =>
        by_cases hw : w = v
        · subst hw; simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, zero, hm]
        · simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, zero, hm, hw, Ne.symm hw]
      | some u =>
        by_cases hw : w = v
        · subst hw
          by_cases hu : u = w
          · subst hu; simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, hm]
          · simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, hm, hu, Ne.symm hu]
        · by_cases hu : u = w
          · subst hu
            simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, hm, hw, Ne.symm hw]
          · simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, hm, hw, Ne.symm hw,
              hu, Ne.symm hu]
    · cases hm : m k <;> simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, zero, hj, hm]
  | del k =>
    by_cases hj : j = k
    · subst hj
      cases hm : m j with
      | none => simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, zero, hm]
      | some u =>
        by_cases hu : u = w
        · subst hu; simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, hm]
        · simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, hm, hu, Ne.symm hu]
    · cases hm : m k <;> simp [zsetOf, KvDelta.step, deltaOf, add, neg, oldOf, single, zero, hj, hm]

theorem applyDelta_is_zset : ∀ (m : Image K V) (d : Delta K V),
    zsetOf (applyDelta m d) = add (zsetOf m) (deltaOfBatch m d)
  | m, [] => by show zsetOf m = add (zsetOf m) zero; rw [add_zero]
  | m, o :: os => by
    show zsetOf (applyDelta (KvDelta.step m o) os) =
      add (zsetOf m) (add (deltaOf m o) (deltaOfBatch (KvDelta.step m o) os))
    rw [applyDelta_is_zset (KvDelta.step m o) os, step_is_zset, add_assoc]

/-- THE BRIDGE: appending a batch to a chain adds that batch's Z-set delta to what readers see. -/
theorem kv_chain_is_zset (c : Chain K V) (d : Delta K V) :
    zsetOf (replay (c ++ [d])) = add (zsetOf (replay c)) (deltaOfBatch (replay c) d) := by
  rw [replay_append]; exact applyDelta_is_zset _ _

/-- Compaction is integration: the compacted chain has the same Z-set. -/
theorem compact_is_integral (c : Chain K V) : zsetOf (replay (compact c)) = zsetOf (replay c) := by
  rw [replay_compact]

end kv

/-! ### Evaluated examples (the named chain a Rust twin runs through `from_kv_chain`) -/

private def c0 : KvDelta.Chain String Nat := [[.put "a" 1, .put "b" 2]]
private def d1 : KvDelta.Delta String Nat := [.put "a" 3, .del "b", .del "zz", .put "c" 4]

#guard deltaOfBatch (KvDelta.replay c0) d1 ("a", 1) == -1
#guard deltaOfBatch (KvDelta.replay c0) d1 ("a", 3) == 1
#guard deltaOfBatch (KvDelta.replay c0) d1 ("b", 2) == -1
#guard deltaOfBatch (KvDelta.replay c0) d1 ("zz", 0) == 0
#guard deltaOfBatch (KvDelta.replay c0) d1 ("c", 4) == 1
#guard zsetOf (KvDelta.replay (c0 ++ [d1])) ("a", 3) == 1
#guard zsetOf (KvDelta.replay (c0 ++ [d1])) ("b", 2) == 0
#guard ofList (logTail ["x", "y", "x"]) "x" == 2

end Bebop.ZSet

#print axioms Bebop.ZSet.memo_step_eq_full
#print axioms Bebop.ZSet.linear_incremental
#print axioms Bebop.ZSet.join_incremental
#print axioms Bebop.ZSet.join_bilinear
#print axioms Bebop.ZSet.join_stream
#print axioms Bebop.ZSet.distinct_not_linear
#print axioms Bebop.ZSet.step_is_zset
#print axioms Bebop.ZSet.kv_chain_is_zset
#print axioms Bebop.ZSet.compact_is_integral
