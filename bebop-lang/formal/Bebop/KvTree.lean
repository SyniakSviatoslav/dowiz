import Bebop.KvDelta

/-
  Bebop.KvTree -- W-ES-P8F (P8f): THE COPY-ON-WRITE B+TREE IS THE ABSTRACT MAP, AND A WRITE NEVER
  CHANGES AN OLD ROOT.

  THE CODE IT MODELS: crates/bebop-kv/src/tree/{node,read,iter,write,build}.rs (W-ES-P8B, KV v4;
  design scratchpad es-p8.design.md §1-§3).

  THE MODEL:
    * the image is a HEAP: a list of objects, an object's ADDRESS is its index. Objects are a tree
      node `tn leaf its` (its = slots (key, address)) or a value `val v` -- values OUT OF LINE, one
      object per entry, exactly as v4's VAL objects;
    * a leaf's slot is (K_i, ref VAL_i); a node's slot is (K_i, ref child_i), K_i = child i's FIRST
      key (rule 3 of mod.rs); leaves sit at depth 0, the root at DEPTH (rule 2);
    * WF h d a: the object at a is the right kind for its level, is non-empty, EVERY REF IS STRICTLY
      BACKWARD (child/value address < a: rule 1), the entries under it are strictly ascending, and
      every separator is its child's first key;
    * the ABSTRACT MAP of a tree is its in-order entry list (`content`), strictly sorted;
    * a WRITE appends: values first, then the rewritten leaf, then each rewritten node bottom-up,
      root last (write.rs's post-order allocation). Untouched children keep their OLD address (shared,
      not copied). A node over the fanout splits by a SPLIT FUNCTION given as a parameter whose only
      hypothesis is `(split l).flatten = l`; `splitEven` is write.rs's `split_even` and satisfies it.
      The root grows while it has more than one slot (`finishUp`, write.rs `finish`).

  PROVED (core Lean only; `#print axioms` at the bottom):
    (1) get_eq            WFR h r -> get h r k = alookup k (rmap h r)
    (2) write_wf_map      WFR h r -> WFR h' r' ∧ rmap h' r' = applyOp (rmap h r) o
        write_get         ... and `get h' r'` = KvDelta.step (get h r) o (the v1-v3 semantics)
    (3) write_appends     h' = h ++ e (a write only APPENDS);
        old_root_unchanged  WFR h r0 -> rmap h' r0 = rmap h r0 ∧ get h' r0 = get h r0, for ANY old
                          root r0 (every node reachable from it sits below h.length: WF_lt)
    (4) scan_eq / scan_slice  the pruned in-order scan from `lo` until the first `stop` key equals
                          (map.filter (¬ k < lo)).takeWhile (¬ stop k); range_eq: for stop = (hi ≤ ·)
                          that is map.filter (lo ≤ k < hi). A PREFIX scan is stop = "not prefixed by p",
                          an instance of scan_slice (Rust: range(p..) + prefix stop).
    (5) migrate_get       writing a v1-v3 chain's compacted batch into the empty tree reads as
                          KvDelta.replay c; build_get: the bulk build (`migrate_to_tree`) of any
                          sorted entry list that reads as replay c reads as replay c.
  Fanout B and occupancy are NOT invariants of WF here (see "differs" below).

  WHERE THE MODEL DIFFERS FROM P8B's RUST (each is shape or encoding, none changes the map):
    * one op per write (Rust batches sorted ops: `stage_tree`); a batch = the fold `writes`, same map;
    * route and leaf search are linear scans (Rust binary-searches `search`/`child_for`; same answer
      on a WF node, not proved here);
    * no merge of under-full children (`rebalance` merges below max(B/4,2)) and no one-child root
      collapse: a one-child node is WF in the model; split bound <= B is not proved;
    * keys are an abstract strict linear order (LinOrd), not byte strings; no byte layout, no crc,
      no META/K-cell encoding, no refusal paths (`TreeError`), no `dead`/`n` counters;
    * build: Rust's stage_build takes `Kv::entries` (sorted); the model takes any sorted list.
-/

set_option linter.unusedSectionVars false
set_option linter.unusedSimpArgs false

namespace Bebop.KvTree
open Bebop.KvDelta (Op)

/-- A strict linear order on keys (Rust: lexicographic byte order). A hypothesis, not an axiom. -/
class LinOrd (K : Type) [LT K] : Prop where
  irrefl : ∀ a : K, ¬ a < a
  trans : ∀ a b c : K, a < b → b < c → a < c
  total : ∀ a b : K, a < b ∨ a = b ∨ b < a

instance : LinOrd Nat := ⟨Nat.lt_irrefl, fun _ _ _ => Nat.lt_trans, fun a b => by omega⟩

/-- An object: a tree node (leaf iff `leaf`) with its slots, or one value. -/
inductive Node (K V : Type) where
  | tn (leaf : Bool) (its : List (K × Nat))
  | val (v : V)

/-- The image: an object's address is its index. -/
abbrev Heap (K V : Type) := List (Node K V)
/-- A KV v4 root: none = the empty map (TREE = 0), some (top, DEPTH). -/
abbrev Root := Option (Nat × Nat)

variable {K V : Type} [LT K] [∀ a b : K, Decidable (a < b)] [DecidableEq K]

/-! ### Sorted association lists: the abstract map -/

def alookup {β : Type} (k : K) : List (K × β) → Option β
  | [] => none
  | p :: l => if p.1 = k then some p.2 else alookup k l

def upsert {β : Type} (k : K) (v : β) : List (K × β) → List (K × β)
  | [] => [(k, v)]
  | p :: l => if p.1 = k then (k, v) :: l else if k < p.1 then (k, v) :: p :: l else p :: upsert k v l

def eraseK {β : Type} (k : K) (l : List (K × β)) : List (K × β) := l.filter (fun p => !decide (p.1 = k))

/-- The abstract map updated by one op. -/
def applyOp : List (K × V) → Op K V → List (K × V)
  | l, .put k v => upsert k v l
  | l, .del k => eraseK k l

def SortedK {β : Type} (l : List (K × β)) : Prop := l.Pairwise (fun a b => a.1 < b.1)
def firstKey {β : Type} (l : List (K × β)) : Option K := l.head?.map Prod.fst

theorem alookup_upsert {β : Type} (k j : K) (v : β) (l : List (K × β)) :
    alookup j (upsert k v l) = if j = k then some v else alookup j l := by
  induction l with
  | nil =>
    by_cases hj : j = k
    · subst hj; simp [upsert, alookup]
    · have : ¬ k = j := fun h => hj h.symm
      simp [upsert, alookup, hj, this]
  | cons p l ih =>
    by_cases h1 : p.1 = k
    · by_cases hj : j = k
      · subst hj; simp [upsert, alookup, h1]
      · have : ¬ k = j := fun h => hj h.symm
        have h2 : ¬ p.1 = j := fun h => hj (h.symm.trans h1)
        simp [upsert, alookup, h1, hj, this, h2]
    · by_cases h2 : k < p.1
      · by_cases hj : j = k
        · subst hj; simp [upsert, alookup, h1, h2]
        · have : ¬ k = j := fun h => hj h.symm
          simp [upsert, alookup, h1, h2, hj, this]
      · simp only [upsert, h1, h2, if_false, alookup, ih]
        by_cases hj : j = k
        · subst hj; simp [h1]
        · simp [hj]

theorem alookup_eraseK {β : Type} (k j : K) (l : List (K × β)) :
    alookup j (eraseK k l) = if j = k then none else alookup j l := by
  induction l with
  | nil => simp [eraseK, alookup]
  | cons p l ih =>
    unfold eraseK at ih ⊢
    by_cases h1 : p.1 = k
    · rw [List.filter_cons_of_neg (by simp [h1]), ih]
      by_cases hj : j = k
      · simp [hj]
      · have : ¬ p.1 = j := fun h => hj (h.symm.trans h1)
        simp [hj, alookup, this]
    · rw [List.filter_cons_of_pos (by simp [h1])]
      simp only [alookup, ih]
      by_cases hj : j = k
      · subst hj; simp [h1]
      · simp [hj]

/-- One op on the abstract map IS one KvDelta.step on the image a reader sees. -/
theorem alookup_applyOp (l : List (K × V)) (o : Op K V) :
    (fun j => alookup j (applyOp l o)) = Bebop.KvDelta.step (fun j => alookup j l) o := by
  funext j
  cases o with
  | put k v => simp [applyOp, Bebop.KvDelta.step, alookup_upsert]
  | del k => simp [applyOp, Bebop.KvDelta.step, alookup_eraseK]

theorem mem_upsert {β : Type} (k : K) (v : β) (l : List (K × β)) (x : K × β) (hx : x ∈ upsert k v l) :
    x = (k, v) ∨ x ∈ l := by
  induction l with
  | nil => simp [upsert] at hx; exact Or.inl hx
  | cons p l ih =>
    simp only [upsert] at hx
    split at hx
    · simp at hx; rcases hx with h | h
      · exact Or.inl h
      · exact Or.inr (List.mem_cons_of_mem _ h)
    · split at hx
      · simp at hx; rcases hx with h | h | h
        · exact Or.inl h
        · exact Or.inr (by simp [h])
        · exact Or.inr (List.mem_cons_of_mem _ h)
      · simp at hx; rcases hx with h | h
        · exact Or.inr (by simp [h])
        · rcases ih h with h' | h'
          · exact Or.inl h'
          · exact Or.inr (List.mem_cons_of_mem _ h')

section Order
variable [LinOrd K]

theorem asymm {a b : K} (h : a < b) : ¬ b < a := fun h' => LinOrd.irrefl a (LinOrd.trans _ _ _ h h')
theorem ne_of_lt' {a b : K} (h : a < b) : a ≠ b := fun e => by subst e; exact LinOrd.irrefl a h
theorem lt_of_not_lt_ne {a b : K} (h1 : ¬ a < b) (h2 : a ≠ b) : b < a := by
  rcases LinOrd.total a b with h | h | h
  · exact absurd h h1
  · exact absurd h h2
  · exact h

theorem sorted_upsert {β : Type} (k : K) (v : β) (l : List (K × β)) (h : SortedK l) :
    SortedK (upsert k v l) := by
  induction l with
  | nil => simp [upsert, SortedK]
  | cons p l ih =>
    unfold SortedK at h ih ⊢
    rw [List.pairwise_cons] at h
    by_cases h1 : p.1 = k
    · simp only [upsert, h1, if_true, List.pairwise_cons]
      exact ⟨fun q hq => h1 ▸ h.1 q hq, h.2⟩
    · by_cases h2 : k < p.1
      · simp only [upsert, h1, h2, if_false, if_true, List.pairwise_cons]
        refine ⟨fun q hq => ?_, h.1, h.2⟩
        rcases List.mem_cons.1 hq with e | e
        · subst e; exact h2
        · exact LinOrd.trans _ _ _ h2 (h.1 q e)
      · simp only [upsert, h1, h2, if_false, List.pairwise_cons]
        refine ⟨fun q hq => ?_, ih h.2⟩
        rcases mem_upsert k v l q hq with e | e
        · subst e; exact lt_of_not_lt_ne h2 (fun e => h1 e.symm)
        · exact h.1 q e

theorem sorted_applyOp (l : List (K × V)) (o : Op K V) (h : SortedK l) : SortedK (applyOp l o) := by
  cases o with
  | put k v => exact sorted_upsert k v l h
  | del k => exact List.Pairwise.sublist (List.filter_sublist) h

theorem upsert_append_right {β : Type} (k : K) (v : β) (A B : List (K × β)) (hB : ∀ x ∈ B, k < x.1) :
    upsert k v (A ++ B) = upsert k v A ++ B := by
  induction A with
  | nil =>
    cases B with
    | nil => rfl
    | cons x B =>
      have hx := hB x (by simp)
      simp [upsert, (ne_of_lt' hx).symm, hx]
  | cons p A ih =>
    simp only [List.cons_append, upsert]
    split
    · rfl
    · split
      · rfl
      · rw [ih]; rfl

theorem upsert_append_left {β : Type} (k : K) (v : β) (A B : List (K × β)) (hA : ∀ x ∈ A, x.1 < k) :
    upsert k v (A ++ B) = A ++ upsert k v B := by
  induction A with
  | nil => rfl
  | cons p A ih =>
    have hp := hA p (by simp)
    simp only [List.cons_append, upsert, ne_of_lt' hp, asymm hp, if_false]
    rw [ih (fun x hx => hA x (by simp [hx]))]

theorem applyOp_append_right (A B : List (K × V)) (o : Op K V) (hB : ∀ x ∈ B, o.key < x.1) :
    applyOp (A ++ B) o = applyOp A o ++ B := by
  cases o with
  | put k v => exact upsert_append_right k v A B hB
  | del k =>
    simp only [applyOp, eraseK, List.filter_append]
    congr 1
    apply List.filter_eq_self.2
    intro x hx
    have hx' : k < x.1 := hB x hx
    simp [(ne_of_lt' hx').symm]

theorem applyOp_append_left (A B : List (K × V)) (o : Op K V) (hA : ∀ x ∈ A, x.1 < o.key) :
    applyOp (A ++ B) o = A ++ applyOp B o := by
  cases o with
  | put k v => exact upsert_append_left k v A B hA
  | del k =>
    simp only [applyOp, eraseK, List.filter_append]
    congr 1
    apply List.filter_eq_self.2
    intro x hx
    have hx' : x.1 < k := hA x hx
    simp [ne_of_lt' hx']

theorem alookup_none {β : Type} (k : K) (B : List (K × β)) (hB : ∀ x ∈ B, x.1 ≠ k) : alookup k B = none := by
  induction B with
  | nil => rfl
  | cons p B ih => simp only [alookup, hB p (by simp), if_false]; exact ih (fun x hx => hB x (by simp [hx]))

theorem alookup_append_left {β : Type} (k : K) (A B : List (K × β)) (hB : ∀ x ∈ B, x.1 ≠ k) :
    alookup k (A ++ B) = alookup k A := by
  induction A with
  | nil => exact alookup_none k B hB
  | cons p A ih => simp only [List.cons_append, alookup, ih]

theorem alookup_append_skip {β : Type} (k : K) (A B : List (K × β)) (hA : ∀ x ∈ A, x.1 ≠ k) :
    alookup k (A ++ B) = alookup k B := by
  induction A with
  | nil => rfl
  | cons p A ih =>
    simp only [List.cons_append, alookup, hA p (by simp), if_false]
    exact ih (fun x hx => hA x (by simp [hx]))

/-- In a sorted list whose first key is s, every key is s or above. -/
theorem head_bound {β : Type} (C : List (K × β)) (s : K) (hs : SortedK C) (hf : firstKey C = some s) :
    ∀ x ∈ C, x.1 = s ∨ s < x.1 := by
  cases C with
  | nil => simp [firstKey] at hf
  | cons c C =>
    simp [firstKey] at hf
    unfold SortedK at hs; rw [List.pairwise_cons] at hs
    intro x hx
    rcases List.mem_cons.1 hx with e | e
    · subst e; exact Or.inl hf
    · exact Or.inr (hf ▸ hs.1 x e)

/-- In a sorted A ++ C where C starts at s, every key of A is below s. -/
theorem below_head {β : Type} (A C : List (K × β)) (s : K) (hs : SortedK (A ++ C)) (hf : firstKey C = some s) :
    ∀ x ∈ A, x.1 < s := by
  cases C with
  | nil => simp [firstKey] at hf
  | cons c C =>
    simp [firstKey] at hf
    unfold SortedK at hs; rw [List.pairwise_append] at hs
    intro x hx; exact hf ▸ hs.2.2 x hx c (by simp)

/-- Routing right: k below the next separator is below every key from there on. -/
theorem above_of_head {β : Type} (C : List (K × β)) (s k : K) (hs : SortedK C) (hf : firstKey C = some s)
    (hk : k < s) : ∀ x ∈ C, k < x.1 := by
  intro x hx
  rcases head_bound C s hs hf x hx with e | e
  · rw [e]; exact hk
  · exact LinOrd.trans _ _ _ hk e

/-- Routing left: k at or above the next separator is above every key before it. -/
theorem below_of_head {β : Type} (A C : List (K × β)) (s k : K) (hs : SortedK (A ++ C))
    (hf : firstKey C = some s) (hk : ¬ k < s) : ∀ x ∈ A, x.1 < k := by
  intro x hx
  have hx' := below_head A C s hs hf x hx
  by_cases e : k = s
  · rw [e]; exact hx'
  · exact LinOrd.trans _ _ _ hx' (lt_of_not_lt_ne hk e)

end Order

/-! ### The heap: objects, values, content -/

theorem lt_of_get {α : Type} {h : List α} {a : Nat} {x : α} (hx : h[a]? = some x) : a < h.length := by
  by_cases hl : a < h.length
  · exact hl
  · rw [List.getElem?_eq_none_iff.mpr (by omega)] at hx; cases hx

theorem get_ext {α : Type} {h : List α} (e : List α) {a : Nat} {x : α} (hx : h[a]? = some x) :
    (h ++ e)[a]? = some x := by
  rw [List.getElem?_append_left (lt_of_get hx)]; exact hx

def valAt (h : Heap K V) (a : Nat) : Option V :=
  match h[a]? with
  | some (.val v) => some v
  | _ => none

theorem valAt_ext (h e : Heap K V) (a : Nat) (v : V) (hv : valAt h a = some v) : valAt (h ++ e) a = some v := by
  unfold valAt at hv ⊢
  cases hh : h[a]? with
  | none => rw [hh] at hv; cases hv
  | some n => rw [hh] at hv; rw [get_ext e hh]; exact hv

theorem valAt_lt (h : Heap K V) (a : Nat) (v : V) (hv : valAt h a = some v) : a < h.length := by
  unfold valAt at hv
  cases hh : h[a]? with
  | none => rw [hh] at hv; cases hv
  | some n => exact lt_of_get hh

/-- A leaf's entries: each slot with the value its VAL object holds. -/
def leafC (h : Heap K V) : List (K × Nat) → List (K × V)
  | [] => []
  | p :: ps => match valAt h p.2 with
    | some v => (p.1, v) :: leafC h ps
    | none => leafC h ps

/-- The entries under the object at `a`, a tree of depth `d`, in order: the ABSTRACT MAP. -/
def content (h : Heap K V) : Nat → Nat → List (K × V)
  | 0, a => match h[a]? with
    | some (.tn true its) => leafC h its
    | _ => []
  | d + 1, a => match h[a]? with
    | some (.tn false cs) => cs.flatMap (fun p => content h d p.2)
    | _ => []

def slotsC (h : Heap K V) (d : Nat) (cs : List (K × Nat)) : List (K × V) := cs.flatMap (fun p => content h d p.2)

/-- The entries under a slot list at level `lv` (0 = a leaf's slots). -/
def cont (h : Heap K V) : Nat → List (K × Nat) → List (K × V)
  | 0, its => leafC h its
  | d + 1, cs => slotsC h d cs

/-- WELL-FORMED: right kind, non-empty, refs STRICTLY BACKWARD, sorted, separators = first keys. -/
def WF (h : Heap K V) : Nat → Nat → Prop
  | 0, a => ∃ its, h[a]? = some (.tn true its) ∧ its ≠ [] ∧ (∀ p ∈ its, p.2 < a) ∧
      SortedK (leafC h its) ∧ (∀ p ∈ its, ∃ v, valAt h p.2 = some v)
  | d + 1, a => ∃ cs, h[a]? = some (.tn false cs) ∧ cs ≠ [] ∧ (∀ p ∈ cs, p.2 < a) ∧
      SortedK (slotsC h d cs) ∧ (∀ p ∈ cs, WF h d p.2 ∧ firstKey (content h d p.2) = some p.1)

/-- Slots valid at level `lv`: values exist (lv = 0), or children are WF with their first key. -/
def OK (h : Heap K V) : Nat → List (K × Nat) → Prop
  | 0, its => ∀ p ∈ its, ∃ v, valAt h p.2 = some v
  | d + 1, cs => ∀ p ∈ cs, WF h d p.2 ∧ firstKey (content h d p.2) = some p.1

theorem WF_iff (h : Heap K V) (lv a : Nat) : WF h lv a ↔ ∃ its, h[a]? = some (.tn (lv == 0) its) ∧
    its ≠ [] ∧ (∀ p ∈ its, p.2 < a) ∧ SortedK (cont h lv its) ∧ OK h lv its := by
  cases lv <;> exact Iff.rfl

theorem content_of (h : Heap K V) (lv a : Nat) (its : List (K × Nat))
    (hA : h[a]? = some (.tn (lv == 0) its)) : content h lv a = cont h lv its := by
  cases lv with
  | zero => simp only [content, cont]; rw [hA]; rfl
  | succ d => simp only [content, cont, slotsC]; rw [hA]; rfl

theorem slotsC_cons (h : Heap K V) (d : Nat) (p : K × Nat) (cs : List (K × Nat)) :
    slotsC h d (p :: cs) = content h d p.2 ++ slotsC h d cs := by simp [slotsC, List.flatMap_cons]

theorem slotsC_append (h : Heap K V) (d : Nat) (A B : List (K × Nat)) :
    slotsC h d (A ++ B) = slotsC h d A ++ slotsC h d B := by simp [slotsC, List.flatMap_append]

theorem leafC_append (h : Heap K V) (A B : List (K × Nat)) : leafC h (A ++ B) = leafC h A ++ leafC h B := by
  induction A with
  | nil => rfl
  | cons p A ih => simp only [List.cons_append, leafC]; split <;> simp [ih]

theorem cont_append (h : Heap K V) (lv : Nat) (A B : List (K × Nat)) :
    cont h lv (A ++ B) = cont h lv A ++ cont h lv B := by
  cases lv with
  | zero => exact leafC_append h A B
  | succ d => exact slotsC_append h d A B

theorem cont_nil (h : Heap K V) (lv : Nat) : cont h lv [] = [] := by cases lv <;> rfl

theorem OK_sub (h : Heap K V) (lv : Nat) (A B : List (K × Nat)) (hs : ∀ x ∈ A, x ∈ B) (hB : OK h lv B) :
    OK h lv A := by
  cases lv with
  | zero => exact fun p hp => hB p (hs p hp)
  | succ d => exact fun p hp => hB p (hs p hp)

theorem WF_lt (h : Heap K V) (lv a : Nat) (hw : WF h lv a) : a < h.length := by
  obtain ⟨its, hA, -⟩ := (WF_iff h lv a).1 hw
  exact lt_of_get hA

/-- Every slot of a valid list points inside the heap. -/
theorem OK_lt (h : Heap K V) (lv : Nat) (its : List (K × Nat)) (hok : OK h lv its) :
    ∀ p ∈ its, p.2 < h.length := by
  cases lv with
  | zero => intro p hp; obtain ⟨v, hv⟩ := hok p hp; exact valAt_lt h _ v hv
  | succ d => intro p hp; exact WF_lt h d p.2 (hok p hp).1

theorem leafC_ext (h e : Heap K V) (its : List (K × Nat)) (hok : OK h 0 its) : leafC (h ++ e) its = leafC h its := by
  induction its with
  | nil => rfl
  | cons p ps ih =>
    obtain ⟨v, hv⟩ := hok p (by simp)
    simp only [leafC, valAt_ext h e _ v hv, hv]
    rw [ih (fun q hq => hok q (by simp [hq]))]

theorem slotsC_congr (h h' : Heap K V) (d : Nat) (cs : List (K × Nat))
    (hc : ∀ p ∈ cs, content h' d p.2 = content h d p.2) : slotsC h' d cs = slotsC h d cs := by
  induction cs with
  | nil => rfl
  | cons p cs ih =>
    rw [slotsC_cons, slotsC_cons, hc p (by simp), ih (fun q hq => hc q (by simp [hq]))]

/-- EXTENSION: appending objects changes neither the well-formedness nor the map of an old tree. -/
theorem WF_ext (h e : Heap K V) : ∀ lv a, WF h lv a → WF (h ++ e) lv a ∧ content (h ++ e) lv a = content h lv a
  | 0, a, hw => by
    obtain ⟨its, hA, hne, hlt, hs, hv⟩ := hw
    have hl := leafC_ext h e its hv
    refine ⟨⟨its, get_ext e hA, hne, hlt, hl ▸ hs, fun p hp => ?_⟩, ?_⟩
    · obtain ⟨v, hv'⟩ := hv p hp; exact ⟨v, valAt_ext h e _ v hv'⟩
    · rw [content_of (h ++ e) 0 a its (get_ext e hA), content_of h 0 a its hA]; exact hl
  | d + 1, a, hw => by
    obtain ⟨cs, hA, hne, hlt, hs, hc⟩ := hw
    have hsl : slotsC (h ++ e) d cs = slotsC h d cs := by
      exact slotsC_congr _ _ d cs (fun p hp => (WF_ext h e d p.2 (hc p hp).1).2)
    refine ⟨⟨cs, get_ext e hA, hne, hlt, hsl ▸ hs, fun p hp => ?_⟩, ?_⟩
    · have := WF_ext h e d p.2 (hc p hp).1
      exact ⟨this.1, this.2 ▸ (hc p hp).2⟩
    · rw [content_of (h ++ e) (d + 1) a cs (get_ext e hA), content_of h (d + 1) a cs hA]; exact hsl

theorem OK_ext (h e : Heap K V) (lv : Nat) (its : List (K × Nat)) (hok : OK h lv its) :
    OK (h ++ e) lv its ∧ cont (h ++ e) lv its = cont h lv its := by
  cases lv with
  | zero =>
    refine ⟨fun p hp => ?_, leafC_ext h e its hok⟩
    obtain ⟨v, hv⟩ := hok p hp; exact ⟨v, valAt_ext h e _ v hv⟩
  | succ d =>
    refine ⟨fun p hp => ?_, ?_⟩
    · have := WF_ext h e d p.2 (hok p hp).1
      exact ⟨this.1, this.2 ▸ (hok p hp).2⟩
    · show slotsC (h ++ e) d its = slotsC h d its
      exact slotsC_congr _ _ d its (fun p hp => (WF_ext h e d p.2 (hok p hp).1).2)

theorem WF_sorted (h : Heap K V) (lv a : Nat) (hw : WF h lv a) : SortedK (content h lv a) := by
  obtain ⟨its, hA, -, -, hs, -⟩ := (WF_iff h lv a).1 hw
  rw [content_of h lv a its hA]; exact hs

theorem firstKey_cont (h : Heap K V) (lv : Nat) (p : K × Nat) (ps : List (K × Nat)) (hok : OK h lv (p :: ps)) :
    firstKey (cont h lv (p :: ps)) = some p.1 := by
  cases lv with
  | zero =>
    obtain ⟨v, hv⟩ := hok p (by simp)
    simp [cont, leafC, hv, firstKey]
  | succ d =>
    have hf := (hok p (by simp)).2
    show firstKey (slotsC h d (p :: ps)) = some p.1
    rw [slotsC_cons]
    revert hf
    cases content h d p.2 with
    | nil => simp [firstKey]
    | cons c C => simp [firstKey]

/-! ### Lookup (theorem 1) -/

def lookItems (h : Heap K V) (k : K) : List (K × Nat) → Option V
  | [] => none
  | p :: ps => if p.1 = k then valAt h p.2 else lookItems h k ps

/-- `child_for`: the last separator <= k, or child 0. -/
def route (k : K) : List (K × Nat) → Option Nat
  | [] => none
  | [p] => some p.2
  | p :: q :: rest => if k < q.1 then some p.2 else route k (q :: rest)

def look (h : Heap K V) (k : K) : Nat → Nat → Option V
  | 0, a => match h[a]? with
    | some (.tn true its) => lookItems h k its
    | _ => none
  | d + 1, a => match h[a]? with
    | some (.tn false cs) => match route k cs with
      | some c => look h k d c
      | none => none
    | _ => none

/-- `TreeIn::get`. -/
def get (h : Heap K V) (r : Root) (k : K) : Option V :=
  match r with
  | none => none
  | some (a, d) => look h k d a

/-- The map a root names. -/
def rmap (h : Heap K V) : Root → List (K × V)
  | none => []
  | some (a, d) => content h d a

def WFR (h : Heap K V) : Root → Prop
  | none => True
  | some (a, d) => WF h d a

theorem lookItems_eq (h : Heap K V) (k : K) (its : List (K × Nat)) (hok : OK h 0 its) :
    lookItems h k its = alookup k (leafC h its) := by
  induction its with
  | nil => rfl
  | cons p ps ih =>
    obtain ⟨v, hv⟩ := hok p (by simp)
    simp only [lookItems, leafC, hv, alookup]
    rw [ih (fun q hq => hok q (by simp [hq]))]

section Order
variable [LinOrd K]

theorem route_eq (h : Heap K V) (k : K) (d : Nat) (ih : ∀ c, WF h d c → look h k d c = alookup k (content h d c)) :
    ∀ cs : List (K × Nat), OK h (d + 1) cs → SortedK (slotsC h d cs) →
      (match route k cs with | some c => look h k d c | none => none) = alookup k (slotsC h d cs)
  | [], _, _ => rfl
  | [p], hok, _ => by
    simp only [route, slotsC_cons]; rw [ih p.2 (hok p (by simp)).1]
    simp [slotsC]
  | p :: q :: rest, hok, hs => by
    have hokt : OK h (d + 1) (q :: rest) :=
      OK_sub h (d + 1) _ _ (fun x hx => List.mem_cons_of_mem _ hx) hok
    rw [slotsC_cons] at hs
    have hfC : firstKey (slotsC h d (q :: rest)) = some q.1 := firstKey_cont h (d + 1) q rest hokt
    have hsC : SortedK (slotsC h d (q :: rest)) := by
      unfold SortedK at hs ⊢; exact (List.pairwise_append.1 hs).2.1
    by_cases hk : k < q.1
    · simp only [route, hk, if_true]
      rw [ih p.2 (hok p (by simp)).1, slotsC_cons h d p (q :: rest), alookup_append_left]
      intro x hx
      exact (ne_of_lt' (above_of_head _ _ _ hsC hfC hk x hx)).symm
    · simp only [route, hk, if_false]
      rw [route_eq h k d ih (q :: rest) hokt hsC, slotsC_cons h d p (q :: rest), alookup_append_skip]
      intro x hx
      exact ne_of_lt' (below_of_head _ _ _ _ hs hfC hk x hx)

/-- (1) LOOKUP: a WF tree answers every key exactly as its abstract map. -/
theorem look_eq (h : Heap K V) (k : K) : ∀ lv a, WF h lv a → look h k lv a = alookup k (content h lv a)
  | 0, a, hw => by
    obtain ⟨its, hA, -, -, -, hv⟩ := hw
    simp only [look, content]; rw [hA]
    exact lookItems_eq h k its hv
  | d + 1, a, hw => by
    obtain ⟨cs, hA, -, -, hs, hc⟩ := hw
    simp only [look, content]; rw [hA]
    exact route_eq h k d (look_eq h k d) cs hc hs

theorem get_eq (h : Heap K V) (r : Root) (hr : WFR h r) (k : K) : get h r k = alookup k (rmap h r) := by
  cases r with
  | none => rfl
  | some p => obtain ⟨a, d⟩ := p; exact look_eq h k d a hr

end Order

/-! ### The writer: path copy by APPENDING -/

/-- Write each non-empty piece as one new node (an empty piece is dropped: write.rs `retain`). -/
def writeNodes (leaf : Bool) : Heap K V → List (List (K × Nat)) → Heap K V × List (K × Nat)
  | h, [] => (h, [])
  | h, [] :: ps => writeNodes leaf h ps
  | h, (p :: ps') :: ps =>
    ((writeNodes leaf (h ++ [.tn leaf (p :: ps')]) ps).1,
     (p.1, h.length) :: (writeNodes leaf (h ++ [.tn leaf (p :: ps')]) ps).2)

/-- `leaf_merge` for one op: a put writes its VAL object first, a remove drops the slot. -/
def leafOp (h : Heap K V) (its : List (K × Nat)) : Op K V → Heap K V × List (K × Nat)
  | .put k v => (h ++ [.val v], upsert k h.length its)
  | .del k => (h, eraseK k its)

/-- Rebuild a node's slot list: the routed child is rewritten (`rec`), every other child keeps its
    OLD address -- shared, not copied. -/
def wrList (rec : Heap K V → Nat → Heap K V × List (K × Nat)) (k : K) (h : Heap K V) :
    List (K × Nat) → Heap K V × List (K × Nat)
  | [] => (h, [])
  | [p] => rec h p.2
  | p :: q :: rest =>
    if k < q.1 then ((rec h p.2).1, (rec h p.2).2 ++ q :: rest)
    else ((wrList rec k h (q :: rest)).1, p :: (wrList rec k h (q :: rest)).2)

/-- `W::apply` for one op: the new slots replacing the node at `a` (depth `d`), objects appended. -/
def wr (split : List (K × Nat) → List (List (K × Nat))) (o : Op K V) :
    Nat → Heap K V → Nat → Heap K V × List (K × Nat)
  | 0, h, a => match h[a]? with
    | some (.tn true its) => writeNodes true (leafOp h its o).1 (split (leafOp h its o).2)
    | _ => (h, [])
  | d + 1, h, a => match h[a]? with
    | some (.tn false cs) =>
      writeNodes false (wrList (wr split o d) o.key h cs).1 (split (wrList (wr split o d) o.key h cs).2)
    | _ => (h, [])

/-- `W::finish`: no slot = the empty map; one slot = the root; more = grow a level (fuel bounds
    the loop; out of fuel, one node takes every slot). -/
def finishUp (split : List (K × Nat) → List (List (K × Nat))) :
    Nat → Heap K V → Nat → List (K × Nat) → Heap K V × Root
  | _, h, _, [] => (h, none)
  | _, h, lv, [s] => (h, some (s.2, lv))
  | 0, h, lv, s1 :: s2 :: rest => (h ++ [.tn false (s1 :: s2 :: rest)], some (h.length, lv + 1))
  | f + 1, h, lv, s1 :: s2 :: rest =>
    finishUp split f (writeNodes false h (split (s1 :: s2 :: rest))).1 (lv + 1)
      (writeNodes false h (split (s1 :: s2 :: rest))).2

/-- ONE COMMIT: `stage_tree` for one op. -/
def write (split : List (K × Nat) → List (List (K × Nat))) (h : Heap K V) : Root → Op K V → Heap K V × Root
  | none, .put k v => (h ++ [.val v, .tn true [(k, h.length)]], some (h.length + 1, 0))
  | none, .del _ => (h, none)
  | some (a, d), o =>
    finishUp split (wr split o d h a).2.length (wr split o d h a).1 d (wr split o d h a).2

section Order
variable [LinOrd K]

theorem leafC_upsert (h : Heap K V) (k : K) (n : Nat) (v : V) (hv : valAt h n = some v) (its : List (K × Nat))
    (hok : OK h 0 its) : leafC h (upsert k n its) = upsert k v (leafC h its) := by
  induction its with
  | nil => simp [upsert, leafC, hv]
  | cons p ps ih =>
    obtain ⟨vp, hvp⟩ := hok p (by simp)
    have ih' := ih (fun q hq => hok q (by simp [hq]))
    by_cases h1 : p.1 = k
    · simp [upsert, leafC, hvp, hv, h1]
    · by_cases h2 : k < p.1
      · simp [upsert, leafC, hvp, hv, h1, h2]
      · simp [upsert, leafC, hvp, h1, h2, ih']

theorem leafC_eraseK (h : Heap K V) (k : K) (its : List (K × Nat)) (hok : OK h 0 its) :
    leafC h (eraseK k its) = eraseK k (leafC h its) := by
  induction its with
  | nil => rfl
  | cons p ps ih =>
    obtain ⟨vp, hvp⟩ := hok p (by simp)
    have ih' := ih (fun q hq => hok q (by simp [hq]))
    unfold eraseK at ih' ⊢
    by_cases h1 : p.1 = k
    · simp [leafC, hvp, h1, ih']
    · simp [leafC, hvp, h1, ih']

theorem leafOp_spec (h : Heap K V) (its : List (K × Nat)) (o : Op K V) (hok : OK h 0 its) :
    (∃ e, (leafOp h its o).1 = h ++ e) ∧ OK (leafOp h its o).1 0 (leafOp h its o).2 ∧
      leafC (leafOp h its o).1 (leafOp h its o).2 = applyOp (leafC h its) o := by
  cases o with
  | put k v =>
    have hv : valAt (h ++ [.val v]) h.length = some v := by simp [valAt]
    have hok' := (OK_ext h [.val v] 0 its hok)
    refine ⟨⟨_, rfl⟩, fun p hp => ?_, ?_⟩
    · rcases mem_upsert k h.length its p hp with e | e
      · subst e; exact ⟨v, hv⟩
      · exact hok'.1 p e
    · show leafC (h ++ [.val v]) (upsert k h.length its) = upsert k v (leafC h its)
      rw [leafC_upsert _ k _ v hv its hok'.1]
      exact congrArg _ hok'.2
  | del k =>
    refine ⟨⟨[], by simp [leafOp]⟩, fun p hp => hok p (List.mem_filter.1 hp).1, ?_⟩
    exact leafC_eraseK h k its hok

/-- New nodes are WF, their map is the pieces' map, the heap only grew. -/
theorem writeNodes_spec (lv : Nat) (leaf : Bool) (hleaf : leaf = (lv == 0)) :
    ∀ (pieces : List (List (K × Nat))) (h : Heap K V),
    OK h lv pieces.flatten → SortedK (cont h lv pieces.flatten) →
    (∃ e, (writeNodes leaf h pieces).1 = h ++ e) ∧
      OK (writeNodes leaf h pieces).1 (lv + 1) (writeNodes leaf h pieces).2 ∧
      slotsC (writeNodes leaf h pieces).1 lv (writeNodes leaf h pieces).2 = cont h lv pieces.flatten
  | [], h, _, _ => ⟨⟨[], by simp [writeNodes]⟩, fun _ hp => by simp [writeNodes] at hp,
      by simp [writeNodes, slotsC, cont_nil]⟩
  | [] :: ps, h, hok, hs => by
    simp only [List.flatten_cons, List.nil_append] at hok hs
    simp only [writeNodes]
    exact writeNodes_spec lv leaf hleaf ps h hok hs
  | (p :: ps') :: ps, h, hok, hs => by
    simp only [List.flatten_cons] at hok hs
    rw [cont_append] at hs
    have hokP := OK_sub h lv (p :: ps') _ (fun x hx => List.mem_append_left _ hx) hok
    have hokF := OK_sub h lv ps.flatten _ (fun x hx => List.mem_append_right _ hx) hok
    have hsP : SortedK (cont h lv (p :: ps')) := by unfold SortedK at hs ⊢; exact (List.pairwise_append.1 hs).1
    have hsF : SortedK (cont h lv ps.flatten) := by unfold SortedK at hs ⊢; exact (List.pairwise_append.1 hs).2.1
    have hA : (h ++ [Node.tn leaf (p :: ps')])[h.length]? = some (.tn (lv == 0) (p :: ps')) := by
      rw [hleaf]; simp
    have eP := OK_ext h [Node.tn leaf (p :: ps')] lv (p :: ps') hokP
    have eF := OK_ext h [Node.tn leaf (p :: ps')] lv ps.flatten hokF
    have hw : WF (h ++ [Node.tn leaf (p :: ps')]) lv h.length := (WF_iff _ lv h.length).2
      ⟨p :: ps', hA, by simp, OK_lt h lv _ hokP, eP.2 ▸ hsP, eP.1⟩
    have hc : content (h ++ [Node.tn leaf (p :: ps')]) lv h.length = cont h lv (p :: ps') := by
      rw [content_of _ lv h.length _ hA]; exact eP.2
    obtain ⟨⟨e2, he2⟩, hok2, hm2⟩ := writeNodes_spec lv leaf hleaf ps _ eF.1 (eF.2 ▸ hsF)
    have hwn : writeNodes leaf h ((p :: ps') :: ps) =
        ((writeNodes leaf (h ++ [Node.tn leaf (p :: ps')]) ps).1,
         (p.1, h.length) :: (writeNodes leaf (h ++ [Node.tn leaf (p :: ps')]) ps).2) := rfl
    rw [hwn]
    have ex := WF_ext _ e2 lv h.length hw
    rw [← he2] at ex
    refine ⟨⟨[Node.tn leaf (p :: ps')] ++ e2, by rw [he2, List.append_assoc]⟩, fun s hs' => ?_, ?_⟩
    · rcases List.mem_cons.1 hs' with e | e
      · subst e
        exact ⟨ex.1, by rw [ex.2, hc]; exact firstKey_cont h lv p ps' hokP⟩
      · exact hok2 s e
    · rw [slotsC_cons, hm2, List.flatten_cons, cont_append, eF.2]
      exact congrArg (· ++ _) (ex.2.trans hc)

theorem wrList_spec (rec : Heap K V → Nat → Heap K V × List (K × Nat)) (h : Heap K V) (d : Nat) (o : Op K V)
    (hrec : ∀ c, WF h d c → (∃ e, (rec h c).1 = h ++ e) ∧ OK (rec h c).1 (d + 1) (rec h c).2 ∧
      slotsC (rec h c).1 d (rec h c).2 = applyOp (content h d c) o) :
    ∀ cs : List (K × Nat), cs ≠ [] → OK h (d + 1) cs → SortedK (slotsC h d cs) →
      (∃ e, (wrList rec o.key h cs).1 = h ++ e) ∧ OK (wrList rec o.key h cs).1 (d + 1) (wrList rec o.key h cs).2 ∧
        slotsC (wrList rec o.key h cs).1 d (wrList rec o.key h cs).2 = applyOp (slotsC h d cs) o
  | [], hne, _, _ => absurd rfl hne
  | [p], _, hok, _ => by
    simp only [wrList]
    have := hrec p.2 (hok p (by simp)).1
    refine ⟨this.1, this.2.1, ?_⟩
    rw [this.2.2, slotsC_cons]; simp [slotsC]
  | p :: q :: rest, _, hok, hs => by
    have hokt : OK h (d + 1) (q :: rest) :=
      OK_sub h (d + 1) _ _ (fun x hx => List.mem_cons_of_mem _ hx) hok
    rw [slotsC_cons] at hs
    have hfC : firstKey (slotsC h d (q :: rest)) = some q.1 := firstKey_cont h (d + 1) q rest hokt
    have hsC : SortedK (slotsC h d (q :: rest)) := by
      unfold SortedK at hs ⊢; exact (List.pairwise_append.1 hs).2.1
    by_cases hk : o.key < q.1
    · simp only [wrList, hk, if_true]
      obtain ⟨⟨e, he⟩, hok1, hm1⟩ := hrec p.2 (hok p (by simp)).1
      have et := OK_ext h e (d + 1) (q :: rest) hokt
      rw [← he] at et
      refine ⟨⟨e, he⟩, fun s hs' => ?_, ?_⟩
      · rcases List.mem_append.1 hs' with e' | e'
        · exact hok1 s e'
        · exact et.1 s e'
      · rw [slotsC_append, hm1, slotsC_cons h d p (q :: rest),
          applyOp_append_right _ _ o (above_of_head _ _ _ hsC hfC hk)]
        exact congrArg _ et.2
    · simp only [wrList, hk, if_false]
      obtain ⟨⟨e, he⟩, hok1, hm1⟩ := wrList_spec rec h d o hrec (q :: rest) (by simp) hokt hsC
      have ep := WF_ext h e d p.2 (hok p (by simp)).1
      rw [← he] at ep
      refine ⟨⟨e, he⟩, fun s hs' => ?_, ?_⟩
      · rcases List.mem_cons.1 hs' with e' | e'
        · subst e'; exact ⟨ep.1, by rw [ep.2]; exact (hok s (by simp)).2⟩
        · exact hok1 s e'
      · rw [slotsC_cons _ _ p, hm1, ep.2, slotsC_cons h d p (q :: rest),
          applyOp_append_left _ _ o (below_of_head _ _ _ _ hs hfC hk)]

variable (split : List (K × Nat) → List (List (K × Nat))) (hsplit : ∀ l, (split l).flatten = l)
include hsplit

theorem wr_spec (o : Op K V) : ∀ d h a, WF h d a →
    (∃ e, (wr split o d h a).1 = h ++ e) ∧ OK (wr split o d h a).1 (d + 1) (wr split o d h a).2 ∧
      slotsC (wr split o d h a).1 d (wr split o d h a).2 = applyOp (content h d a) o
  | 0, h, a, hw => by
    obtain ⟨its, hA, -, -, hs, hv⟩ := hw
    obtain ⟨⟨e1, he1⟩, hok1, hm1⟩ := leafOp_spec h its o hv
    have hs1 : SortedK (cont (leafOp h its o).1 0 (split (leafOp h its o).2).flatten) := by
      rw [hsplit]; show SortedK (leafC _ _); rw [hm1]; exact sorted_applyOp _ o hs
    obtain ⟨⟨e2, he2⟩, hok2, hm2⟩ := writeNodes_spec 0 true rfl (split (leafOp h its o).2) (leafOp h its o).1
      (by rw [hsplit]; exact hok1) hs1
    have hwr : wr split o 0 h a = writeNodes true (leafOp h its o).1 (split (leafOp h its o).2) := by
      simp only [wr, hA]
    have hc : content h 0 a = leafC h its := content_of h 0 a its hA
    rw [hwr, hc]
    refine ⟨⟨e1 ++ e2, by rw [he2, he1, List.append_assoc]⟩, hok2, ?_⟩
    rw [hm2, hsplit]; exact hm1
  | d + 1, h, a, hw => by
    obtain ⟨cs, hA, hne, -, hs, hc⟩ := hw
    obtain ⟨⟨e1, he1⟩, hok1, hm1⟩ := wrList_spec (wr split o d) h d o
      (fun c hc' => wr_spec o d h c hc') cs hne hc hs
    have hs1 : SortedK (cont (wrList (wr split o d) o.key h cs).1 (d + 1)
        (split (wrList (wr split o d) o.key h cs).2).flatten) := by
      rw [hsplit]; show SortedK (slotsC _ _ _); rw [hm1]; exact sorted_applyOp _ o hs
    obtain ⟨⟨e2, he2⟩, hok2, hm2⟩ := writeNodes_spec (d + 1) false rfl
      (split (wrList (wr split o d) o.key h cs).2)
      (wrList (wr split o d) o.key h cs).1 (by rw [hsplit]; exact hok1) hs1
    have hwr : wr split o (d + 1) h a = writeNodes false (wrList (wr split o d) o.key h cs).1
        (split (wrList (wr split o d) o.key h cs).2) := by
      simp only [wr, hA]
    have hc' : content h (d + 1) a = slotsC h d cs := content_of h (d + 1) a cs hA
    rw [hwr, hc']
    refine ⟨⟨e1 ++ e2, by rw [he2, he1, List.append_assoc]⟩, hok2, ?_⟩
    rw [hm2, hsplit]; exact hm1

theorem finishUp_spec : ∀ (f : Nat) (h : Heap K V) (lv : Nat) (sl : List (K × Nat)),
    OK h (lv + 1) sl → SortedK (slotsC h lv sl) →
    (∃ e, (finishUp split f h lv sl).1 = h ++ e) ∧ WFR (finishUp split f h lv sl).1 (finishUp split f h lv sl).2 ∧
      rmap (finishUp split f h lv sl).1 (finishUp split f h lv sl).2 = slotsC h lv sl
  | f, h, lv, [], _, _ => by
    have : finishUp split f h lv [] = (h, none) := by cases f <;> rfl
    rw [this]
    exact ⟨⟨[], by simp⟩, trivial, by simp [rmap, slotsC]⟩
  | f, h, lv, [s], hok, _ => by
    have : finishUp split f h lv [s] = (h, some (s.2, lv)) := by cases f <;> rfl
    rw [this]
    exact ⟨⟨[], by simp⟩, (hok s (by simp)).1, by simp [rmap, slotsC]⟩
  | 0, h, lv, s1 :: s2 :: rest, hok, hs => by
    have hf : finishUp split 0 h lv (s1 :: s2 :: rest) =
        (h ++ [Node.tn false (s1 :: s2 :: rest)], some (h.length, lv + 1)) := rfl
    rw [hf]
    have hA : (h ++ [Node.tn false (s1 :: s2 :: rest)])[h.length]? = some (.tn (lv + 1 == 0) (s1 :: s2 :: rest)) := by
      simp
    have eS := OK_ext h [Node.tn false (s1 :: s2 :: rest)] (lv + 1) _ hok
    have hw : WF (h ++ [Node.tn false (s1 :: s2 :: rest)]) (lv + 1) h.length := (WF_iff _ (lv + 1) h.length).2
      ⟨_, hA, by simp, OK_lt h (lv + 1) _ hok, eS.2 ▸ hs, eS.1⟩
    refine ⟨⟨_, rfl⟩, hw, ?_⟩
    show content _ (lv + 1) h.length = _
    rw [content_of _ (lv + 1) h.length _ hA]; exact eS.2
  | f + 1, h, lv, s1 :: s2 :: rest, hok, hs => by
    have hf : finishUp split (f + 1) h lv (s1 :: s2 :: rest) =
        finishUp split f (writeNodes false h (split (s1 :: s2 :: rest))).1 (lv + 1)
          (writeNodes false h (split (s1 :: s2 :: rest))).2 := rfl
    rw [hf]
    have hs' : SortedK (cont h (lv + 1) (split (s1 :: s2 :: rest)).flatten) := by rw [hsplit]; exact hs
    obtain ⟨⟨e1, he1⟩, hok1, hm1⟩ := writeNodes_spec (lv + 1) false rfl (split (s1 :: s2 :: rest)) h
      (by rw [hsplit]; exact hok) hs'
    rw [hsplit] at hm1
    have hs1 : SortedK (slotsC (writeNodes false h (split (s1 :: s2 :: rest))).1 (lv + 1)
        (writeNodes false h (split (s1 :: s2 :: rest))).2) := by rw [hm1]; exact hs
    obtain ⟨⟨e2, he2⟩, hw2, hm2⟩ := finishUp_spec f _ (lv + 1) _ hok1 hs1
    refine ⟨⟨e1 ++ e2, by rw [he2, he1, List.append_assoc]⟩, hw2, ?_⟩
    rw [hm2]; exact hm1

/-- (2)+(3) ONE WRITE: the heap only GROWS, the new root is WF, its map is the old map updated. -/
theorem write_spec (h : Heap K V) (r : Root) (o : Op K V) (hr : WFR h r) :
    (∃ e, (write split h r o).1 = h ++ e) ∧ WFR (write split h r o).1 (write split h r o).2 ∧
      rmap (write split h r o).1 (write split h r o).2 = applyOp (rmap h r) o := by
  cases r with
  | none =>
    cases o with
    | put k v =>
      have hv : valAt (h ++ [Node.val v, Node.tn true [(k, h.length)]]) h.length = some v := by simp [valAt]
      have hA : (h ++ [Node.val v, Node.tn true [(k, h.length)]])[h.length + 1]? =
          some (.tn true [(k, h.length)]) := by simp
      have hw : WF (h ++ [Node.val v, Node.tn true [(k, h.length)]]) 0 (h.length + 1) :=
        ⟨[(k, h.length)], hA, by simp, by simp, by simp [SortedK, leafC, hv],
          fun p hp => by simp at hp; subst hp; exact ⟨v, hv⟩⟩
      refine ⟨⟨_, rfl⟩, hw, ?_⟩
      show content (h ++ [Node.val v, Node.tn true [(k, h.length)]]) 0 (h.length + 1) = upsert k v []
      rw [content_of _ 0 _ _ hA]; simp [cont, leafC, hv, upsert]
    | del k => exact ⟨⟨[], by simp [write]⟩, trivial, by simp [write, rmap, applyOp, eraseK]⟩
  | some p =>
    obtain ⟨a, d⟩ := p
    obtain ⟨⟨e1, he1⟩, hok1, hm1⟩ := wr_spec split hsplit o d h a hr
    have hs1 : SortedK (slotsC (wr split o d h a).1 d (wr split o d h a).2) := by
      rw [hm1]; exact sorted_applyOp _ o (WF_sorted h d a hr)
    obtain ⟨⟨e2, he2⟩, hw2, hm2⟩ := finishUp_spec split hsplit (wr split o d h a).2.length _ d _ hok1 hs1
    refine ⟨⟨e1 ++ e2, by simp only [write]; rw [he2, he1, List.append_assoc]⟩, hw2, ?_⟩
    simp only [write]; rw [hm2, hm1]; rfl

theorem write_appends (h : Heap K V) (r : Root) (o : Op K V) (hr : WFR h r) :
    ∃ e, (write split h r o).1 = h ++ e := (write_spec split hsplit h r o hr).1

theorem write_wf_map (h : Heap K V) (r : Root) (o : Op K V) (hr : WFR h r) :
    WFR (write split h r o).1 (write split h r o).2 ∧
      rmap (write split h r o).1 (write split h r o).2 = applyOp (rmap h r) o := (write_spec split hsplit h r o hr).2

/-- (2) as the reader sees it: get after the write = KvDelta.step of get before. -/
theorem write_get (h : Heap K V) (r : Root) (o : Op K V) (hr : WFR h r) :
    (fun k => get (write split h r o).1 (write split h r o).2 k) = Bebop.KvDelta.step (fun k => get h r k) o := by
  obtain ⟨hw, hm⟩ := write_wf_map split hsplit h r o hr
  have hg : (fun k => get h r k) = fun k => alookup k (rmap h r) := funext (get_eq h r hr)
  funext j
  rw [get_eq _ _ hw, hm, hg]
  exact congrFun (alookup_applyOp (rmap h r) o) j

/-- (3) PERSISTENCE: every address the old image had holds the same object after the write. -/
theorem write_old_cells (h : Heap K V) (r : Root) (o : Op K V) (hr : WFR h r) :
    ∀ i < h.length, (write split h r o).1[i]? = h[i]? := by
  obtain ⟨e, he⟩ := write_appends split hsplit h r o hr
  intro i hi; rw [he, List.getElem?_append_left hi]

/-- (3) PERSISTENCE: ANY old root -- not only the one written from -- reads the same map and the
    same answers after the write (its nodes are all below h.length: WF_lt + refs backward). -/
theorem old_root_unchanged (h : Heap K V) (r : Root) (o : Op K V) (hr : WFR h r) (r0 : Root) (h0 : WFR h r0) :
    WFR (write split h r o).1 r0 ∧ rmap (write split h r o).1 r0 = rmap h r0 ∧
      ∀ k, get (write split h r o).1 r0 k = get h r0 k := by
  obtain ⟨e, he⟩ := write_appends split hsplit h r o hr
  rw [he]
  cases r0 with
  | none => exact ⟨trivial, rfl, fun _ => rfl⟩
  | some p =>
    obtain ⟨a, d⟩ := p
    have ex := WF_ext h e d a h0
    refine ⟨ex.1, ex.2, fun k => ?_⟩
    rw [get_eq (h ++ e) (some (a, d)) ex.1, get_eq h (some (a, d)) h0]; exact congrArg _ ex.2

end Order

/-! ### Range / prefix scan (theorem 4) -/

/-- The scan over an ordered entry list: skip keys below `lo`, stop at the first `stop` key.
    The Bool says whether it stopped (the iterator's `done`). -/
def scanL (lo : K) (stop : K → Bool) : List (K × V) → List (K × V) × Bool
  | [] => ([], false)
  | p :: l => if p.1 < lo then scanL lo stop l else if stop p.1 then ([], true)
      else (p :: (scanL lo stop l).1, (scanL lo stop l).2)

/-- A node's scan: a child whose NEXT separator is <= lo lies wholly below lo and is never opened
    (`seek`); a child that stopped ends the scan (the children after it are never opened). -/
def scanCs (lo : K) (sub : Nat → List (K × V) × Bool) : List (K × Nat) → List (K × V) × Bool
  | [] => ([], false)
  | [p] => sub p.2
  | p :: q :: rest => if ¬ lo < q.1 then scanCs lo sub (q :: rest)
      else if (sub p.2).2 then sub p.2
      else ((sub p.2).1 ++ (scanCs lo sub (q :: rest)).1, (scanCs lo sub (q :: rest)).2)

/-- `TreeIn::range` / `prefix`: the pruned in-order walk. -/
def scan (h : Heap K V) (lo : K) (stop : K → Bool) : Nat → Nat → List (K × V) × Bool
  | 0, a => match h[a]? with
    | some (.tn true its) => scanL lo stop (leafC h its)
    | _ => ([], false)
  | d + 1, a => match h[a]? with
    | some (.tn false cs) => scanCs lo (scan h lo stop d) cs
    | _ => ([], false)

def scanR (h : Heap K V) (r : Root) (lo : K) (stop : K → Bool) : List (K × V) :=
  match r with
  | none => []
  | some (a, d) => (scan h lo stop d a).1

theorem scanL_append (lo : K) (stop : K → Bool) (A B : List (K × V)) :
    scanL lo stop (A ++ B) = if (scanL lo stop A).2 then scanL lo stop A
      else ((scanL lo stop A).1 ++ (scanL lo stop B).1, (scanL lo stop B).2) := by
  induction A with
  | nil => simp [scanL]
  | cons p A ih =>
    simp only [List.cons_append, scanL]
    by_cases h1 : p.1 < lo
    · simp [h1, ih]
    · by_cases h2 : stop p.1 = true
      · simp [h1, h2]
      · by_cases hA : (scanL lo stop A).2 = true
        · simp [h1, h2, ih, hA]
        · simp [h1, h2, ih, hA]

theorem scanL_below (lo : K) (stop : K → Bool) (A : List (K × V)) (hA : ∀ x ∈ A, x.1 < lo) :
    scanL lo stop A = ([], false) := by
  induction A with
  | nil => rfl
  | cons p A ih =>
    simp only [scanL, hA p (by simp), if_true]
    exact ih (fun x hx => hA x (by simp [hx]))

/-- The ordered slice: the entries from `lo` on, up to (not including) the first `stop` key. -/
theorem scan_slice (lo : K) (stop : K → Bool) (l : List (K × V)) :
    (scanL lo stop l).1 = (l.filter (fun p => !decide (p.1 < lo))).takeWhile (fun p => !stop p.1) := by
  induction l with
  | nil => rfl
  | cons p l ih =>
    by_cases h1 : p.1 < lo
    · simp [scanL, h1, ih]
    · by_cases h2 : stop p.1 = true
      · simp [scanL, h1, h2, List.filter_cons, List.takeWhile_cons]
      · simp [scanL, h1, h2, ih, List.filter_cons, List.takeWhile_cons]

section Order
variable [LinOrd K]

theorem scanCs_eq (h : Heap K V) (lo : K) (stop : K → Bool) (d : Nat) (sub : Nat → List (K × V) × Bool)
    (ih : ∀ c, WF h d c → sub c = scanL lo stop (content h d c)) :
    ∀ cs : List (K × Nat), OK h (d + 1) cs → SortedK (slotsC h d cs) →
      scanCs lo sub cs = scanL lo stop (slotsC h d cs)
  | [], _, _ => by simp [scanCs, slotsC, scanL]
  | [p], hok, _ => by
    simp only [scanCs]; rw [ih p.2 (hok p (by simp)).1]; simp [slotsC]
  | p :: q :: rest, hok, hs => by
    have hokt : OK h (d + 1) (q :: rest) :=
      OK_sub h (d + 1) _ _ (fun x hx => List.mem_cons_of_mem _ hx) hok
    rw [slotsC_cons] at hs
    have hfC : firstKey (slotsC h d (q :: rest)) = some q.1 := firstKey_cont h (d + 1) q rest hokt
    have hsC : SortedK (slotsC h d (q :: rest)) := by
      unfold SortedK at hs ⊢; exact (List.pairwise_append.1 hs).2.1
    by_cases hk : lo < q.1
    · simp only [scanCs, hk, not_true_eq_false, if_false]
      rw [ih p.2 (hok p (by simp)).1, scanCs_eq h lo stop d sub ih (q :: rest) hokt hsC,
        slotsC_cons h d p (q :: rest), scanL_append]
    · simp only [scanCs, hk, not_false_eq_true, if_true]
      rw [scanCs_eq h lo stop d sub ih (q :: rest) hokt hsC, slotsC_cons h d p (q :: rest), scanL_append,
        scanL_below lo stop _ (below_of_head _ _ _ _ hs hfC hk)]
      simp

theorem scan_eq (h : Heap K V) (lo : K) (stop : K → Bool) :
    ∀ lv a, WF h lv a → scan h lo stop lv a = scanL lo stop (content h lv a)
  | 0, a, hw => by
    obtain ⟨its, hA, -⟩ := hw
    simp only [scan, content]; rw [hA]
  | d + 1, a, hw => by
    obtain ⟨cs, hA, -, -, hs, hc⟩ := hw
    simp only [scan, content]; rw [hA]
    exact scanCs_eq h lo stop d _ (scan_eq h lo stop d) cs hc hs

/-- (4) SCAN = THE MAP'S ORDERED SLICE, for any stop predicate (range: stop = hi <= k; prefix:
    stop = "k does not start with p"). -/
theorem scanR_eq (h : Heap K V) (r : Root) (hr : WFR h r) (lo : K) (stop : K → Bool) :
    scanR h r lo stop = ((rmap h r).filter (fun p => !decide (p.1 < lo))).takeWhile (fun p => !stop p.1) := by
  cases r with
  | none => rfl
  | some p =>
    obtain ⟨a, d⟩ := p
    show (scan h lo stop d a).1 = _
    rw [scan_eq h lo stop d a hr, scan_slice]; rfl

theorem takeWhile_sorted (hi : K) (m : List (K × V)) (hs : SortedK m) :
    m.takeWhile (fun p => decide (p.1 < hi)) = m.filter (fun p => decide (p.1 < hi)) := by
  induction m with
  | nil => rfl
  | cons p m ih =>
    unfold SortedK at hs; rw [List.pairwise_cons] at hs
    by_cases h1 : p.1 < hi
    · simp [List.takeWhile_cons, List.filter_cons, h1, ih hs.2]
    · simp only [List.takeWhile_cons, List.filter_cons, h1, decide_false, Bool.false_eq_true, if_false]
      symm; apply List.filter_eq_nil_iff.2
      intro x hx hx'
      exact h1 (LinOrd.trans _ _ _ (hs.1 x hx) (of_decide_eq_true hx'))

/-- (4) RANGE [lo, hi) of a WF tree = exactly the map's keys in [lo, hi), in order. -/
theorem range_eq (h : Heap K V) (r : Root) (hr : WFR h r) (lo hi : K) :
    scanR h r lo (fun x => !decide (x < hi)) =
      (rmap h r).filter (fun p => decide (p.1 < hi) && !decide (p.1 < lo)) := by
  rw [scanR_eq h r hr]
  simp only [Bool.not_not]
  have hs : SortedK (rmap h r) := by
    cases r with
    | none => exact List.Pairwise.nil
    | some p => obtain ⟨a, d⟩ := p; exact WF_sorted h d a hr
  rw [takeWhile_sorted hi _ (List.Pairwise.sublist List.filter_sublist hs), List.filter_filter]

end Order

/-! ### Migration from v1-v3 (theorem 5) -/

/-- A sequence of commits. -/
def writes (split : List (K × Nat) → List (List (K × Nat))) (h : Heap K V) (r : Root) :
    List (Op K V) → Heap K V × Root
  | [] => (h, r)
  | o :: os => writes split (write split h r o).1 (write split h r o).2 os

/-- Migration through the writer: the v1-v3 chain's compacted batch, written into the empty tree. -/
def migrate (split : List (K × Nat) → List (List (K × Nat))) (c : Bebop.KvDelta.Chain K V) : Heap K V × Root :=
  writes split [] none (Bebop.KvDelta.compact c).flatten

/-- One VAL object per entry, in order. -/
def putVals (h : Heap K V) : List (K × V) → Heap K V × List (K × Nat)
  | [] => (h, [])
  | p :: es => ((putVals (h ++ [.val p.2]) es).1, (p.1, h.length) :: (putVals (h ++ [.val p.2]) es).2)

/-- `stage_build` / `migrate_to_tree`: values, then leaves, then the levels above, root last. -/
def build (split : List (K × Nat) → List (List (K × Nat))) (h : Heap K V) (es : List (K × V)) : Heap K V × Root :=
  finishUp split (writeNodes true (putVals h es).1 (split (putVals h es).2)).2.length
    (writeNodes true (putVals h es).1 (split (putVals h es).2)).1 0
    (writeNodes true (putVals h es).1 (split (putVals h es).2)).2

theorem putVals_spec : ∀ (es : List (K × V)) (h : Heap K V),
    (∃ e, (putVals h es).1 = h ++ e) ∧ OK (putVals h es).1 0 (putVals h es).2 ∧
      leafC (putVals h es).1 (putVals h es).2 = es
  | [], h => ⟨⟨[], by simp [putVals]⟩, fun _ hp => by simp [putVals] at hp, rfl⟩
  | p :: es, h => by
    obtain ⟨⟨e, he⟩, hok, hm⟩ := putVals_spec es (h ++ [.val p.2])
    have hv : valAt (putVals (h ++ [.val p.2]) es).1 h.length = some p.2 := by
      rw [he]; exact valAt_ext _ e _ _ (by simp [valAt])
    refine ⟨⟨[.val p.2] ++ e, by simp only [putVals]; rw [he, List.append_assoc]⟩, fun q hq => ?_, ?_⟩
    · simp only [putVals] at hq
      rcases List.mem_cons.1 hq with e' | e'
      · subst e'; exact ⟨p.2, hv⟩
      · exact hok q e'
    · simp only [putVals, leafC, hv, hm]

section Order
variable [LinOrd K]
variable (split : List (K × Nat) → List (List (K × Nat))) (hsplit : ∀ l, (split l).flatten = l)
include hsplit

theorem writes_get : ∀ (os : List (Op K V)) (h : Heap K V) (r : Root), WFR h r →
    WFR (writes split h r os).1 (writes split h r os).2 ∧
      (fun k => get (writes split h r os).1 (writes split h r os).2 k) =
        Bebop.KvDelta.applyDelta (fun k => get h r k) os
  | [], h, r, hr => ⟨hr, rfl⟩
  | o :: os, h, r, hr => by
    have hw := (write_wf_map split hsplit h r o hr).1
    obtain ⟨h1, h2⟩ := writes_get os _ _ hw
    refine ⟨h1, ?_⟩
    simp only [writes]; rw [h2, Bebop.KvDelta.applyDelta_cons, write_get split hsplit h r o hr]

/-- (5) BRIDGE: migrating a v1-v3 chain through the writer reads exactly as the chain replayed. -/
theorem migrate_get (c : Bebop.KvDelta.Chain K V) :
    (fun k => get (migrate split c).1 (migrate split c).2 k) = Bebop.KvDelta.replay c := by
  rw [migrate, (writes_get split hsplit _ [] none trivial).2, ← Bebop.KvDelta.replay_compact c,
    Bebop.KvDelta.replay_flatten]
  rfl

theorem build_spec (h : Heap K V) (es : List (K × V)) (hs : SortedK es) :
    WFR (build split h es).1 (build split h es).2 ∧ rmap (build split h es).1 (build split h es).2 = es := by
  obtain ⟨-, hok, hm⟩ := putVals_spec es h
  have hs0 : SortedK (cont (putVals h es).1 0 (split (putVals h es).2).flatten) := by
    rw [hsplit]; show SortedK (leafC _ _); rw [hm]; exact hs
  obtain ⟨-, hok1, hm1⟩ := writeNodes_spec 0 true rfl (split (putVals h es).2) (putVals h es).1
    (by rw [hsplit]; exact hok) hs0
  rw [hsplit] at hm1
  have hs1 : SortedK (slotsC (writeNodes true (putVals h es).1 (split (putVals h es).2)).1 0
      (writeNodes true (putVals h es).1 (split (putVals h es).2)).2) := by
    rw [hm1]; show SortedK (leafC _ _); rw [hm]; exact hs
  obtain ⟨-, hw, hr⟩ := finishUp_spec split hsplit _ _ 0 _ hok1 hs1
  refine ⟨hw, ?_⟩
  unfold build; rw [hr, hm1]; show leafC _ _ = es; exact hm

/-- (5) BRIDGE, bulk: `migrate_to_tree` of the KV's sorted entries reads as the chain replayed. -/
theorem build_get (c : Bebop.KvDelta.Chain K V) (es : List (K × V)) (hs : SortedK es)
    (hes : ∀ k, alookup k es = Bebop.KvDelta.replay c k) :
    (fun k => get (build split [] es).1 (build split [] es).2 k) = Bebop.KvDelta.replay c := by
  obtain ⟨hw, hm⟩ := build_spec split hsplit [] es hs
  funext k; rw [get_eq _ _ hw, hm, hes]

end Order

/-! ### write.rs `split_even`, and evaluated examples -/

/-- `n` pieces from piece `p` on: piece size `base + (p < rem)`, the last takes the rest. -/
def evenGo {α : Type} (base rem : Nat) : Nat → Nat → List α → List (List α)
  | 0, _, l => [l]
  | 1, _, l => [l]
  | j + 2, p, l => l.take (base + if p < rem then 1 else 0) ::
      evenGo base rem (j + 1) (p + 1) (l.drop (base + if p < rem then 1 else 0))

/-- `split_even(v, b)`: ceil(len/b) pieces (at least one) whose sizes differ by at most one. -/
def splitEven {α : Type} (b : Nat) (l : List α) : List (List α) :=
  evenGo (l.length / max 1 ((l.length + b - 1) / b)) (l.length % max 1 ((l.length + b - 1) / b))
    (max 1 ((l.length + b - 1) / b)) 0 l

theorem evenGo_flatten {α : Type} (base rem : Nat) : ∀ (n p : Nat) (l : List α), (evenGo base rem n p l).flatten = l
  | 0, _, l => by simp [evenGo]
  | 1, _, l => by simp [evenGo]
  | j + 2, p, l => by
    simp only [evenGo, List.flatten_cons]; rw [evenGo_flatten base rem (j + 1)]; exact List.take_append_drop _ _

theorem splitEven_flatten {α : Type} (b : Nat) (l : List α) : (splitEven b l).flatten = l := evenGo_flatten _ _ _ _ _

#guard (splitEven 4 (List.range 9)).map List.length == [3, 3, 3]
#guard (splitEven 32 (List.range 33)).map List.length == [17, 16]
#guard (splitEven 32 (List.range 32)).map List.length == [32]
#guard (splitEven 4 ([] : List Nat)) == [[]]

private def sp : List (Nat × Nat) → List (List (Nat × Nat)) := splitEven 4
private def t20 : Heap Nat Nat × Root := writes sp [] none ((List.range 20).map fun i => Op.put (i * 7 % 20) i)
private def t21 : Heap Nat Nat × Root := write sp t20.1 t20.2 (.del 7)

#guard get t20.1 t20.2 7 == some 1
#guard (rmap t20.1 t20.2).map Prod.fst == List.range 20
#guard (t20.2.map Prod.snd).getD 0 ≥ 2
#guard get t21.1 t21.2 7 == none
#guard get t21.1 t20.2 7 == some 1
#guard (scanR t21.1 t21.2 5 (fun x => !decide (x < 10))).map Prod.fst == [5, 6, 8, 9]
#guard ((build sp [] ((List.range 50).map fun i => (i, i * i))).2.map Prod.snd).getD 0 ≥ 2
#guard get (build sp [] ((List.range 50).map fun i => (i, i * i))).1
  (build sp [] ((List.range 50).map fun i => (i, i * i))).2 31 == some 961

end Bebop.KvTree

#print axioms Bebop.KvTree.get_eq
#print axioms Bebop.KvTree.write_wf_map
#print axioms Bebop.KvTree.write_get
#print axioms Bebop.KvTree.write_appends
#print axioms Bebop.KvTree.write_old_cells
#print axioms Bebop.KvTree.old_root_unchanged
#print axioms Bebop.KvTree.scanR_eq
#print axioms Bebop.KvTree.range_eq
#print axioms Bebop.KvTree.migrate_get
#print axioms Bebop.KvTree.build_get
#print axioms Bebop.KvTree.splitEven_flatten
