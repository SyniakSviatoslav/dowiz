/-
  Bebop.DocMerge -- W-MG-LEAN: the MERGE of two chains from one genesis, over the model of
  Bebop.DocLog. Core Lean 4, no Mathlib, no `axiom`, no `sorry`.

  THE CODE IT MODELS, BY READING: crates/bebop-log/src/doc/merge.rs `Doc::merge_into`. NOTHING HERE
  SAYS THE RUST IS PROVED. `union` is records ours-then-their-tail and seals likewise; the frontier
  rows must be equal (`CutOneSide`). The model does not write a MERGE: nor does `merge_into`.

  WHAT THE CARD ASKED, AND WHAT IS TRUE. "The merged file holds iff both sides hold and neither
  forked" is FALSE as a sentence about the union, in both directions, and each failure is a theorem:
    * a union can hold while a side does not (DocLimits.union_holds_side_does_not): that is why
      `merge_into` checks each side FIRST, and `Merges` below is that check, not a consequence.
    * two sides can hold, unforked, and their union not hold (DocLimits.past_revoke_blocks_union): a
      REVOKE on one side and the device's tail past it on the other, `RevokedTail`.
  The exact statement is `merged_holds_iff`: GIVEN both sides hold on one frontier, the union holds
  iff neither forked AND no record sits past a REVOKE of the union AND no id collides.

  PROVED:
    union_chain, union_chain'   the union's chain of a device is the longer of the two
    union_holds         the forward direction
    fork_breaks_union   the converse for forks: a union that holds was no fork
    merged_holds_iff    the iff, for sides that hold
    merges_iff          `Merges` (what `merge_into` checks) with a union that holds
    union_comm_front / union_assoc_front / union_idem_front   the frontier of the file is a semilattice
    union_keeps         no record lost, in either direction
-/
import Bebop.DocLog
import Bebop.Sync

set_option linter.unusedSectionVars false

namespace Bebop.DocLog

open Bebop.Sync

variable {A I P G : Type} [DecidableEq A] [DecidableEq I] [DecidableEq P] [DecidableEq G]

/-- Ours as they stand, then their tail; the same for seals. Rows are ours: `merges_iff` asks that
  they are theirs too. -/
def union (F₁ F₂ : File A I P G) : File A I P G :=
  ⟨unionL F₁.recs F₂.recs, unionL F₁.seals F₂.seals, F₁.rows⟩

/-- Neither forked: per device, one side's chain is a prefix of the other's (`missing`). -/
def NoFork (F₁ F₂ : File A I P G) : Prop := ∀ a, Compat (chainOf F₁ a) (chainOf F₂ a)

/-- `RevokedTail`'s negation: every record of the union is valid under every REVOKE of the union. -/
def NoPastRevoke (W : World A I P G) (F₁ F₂ : File A I P G) : Prop :=
  ∀ r ∈ (union F₁ F₂).recs, Valid W (union F₁ F₂) r.actor r.seq

variable {W : World A I P G} {F₁ F₂ : File A I P G}

theorem union_keeps {r : Rec A I P} : r ∈ (union F₁ F₂).recs ↔ r ∈ F₁.recs ∨ r ∈ F₂.recs :=
  unionL_mem

theorem union_keeps_seal {s : Seal A I G} : s ∈ (union F₁ F₂).seals ↔ s ∈ F₁.seals ∨ s ∈ F₂.seals :=
  unionL_mem

/-- The device's part of the union: ours, then what of theirs we lack. -/
theorem chainOf_union (a : A) : chainOf (union F₁ F₂) a =
    chainOf F₁ a ++ (chainOf F₂ a).filter (fun r => decide (r ∉ F₁.recs)) := by
  simp only [chainOf, union, unionL, List.filter_append, List.filter_filter]
  congr 2
  funext r
  exact Bool.and_comm ..

/-- Their chain is the longer: the union's is theirs. -/
theorem union_chain {a : A} (hn : (chainOf F₂ a).Nodup) (h : chainOf F₁ a <+: chainOf F₂ a) :
    chainOf (union F₁ F₂) a = chainOf F₂ a := by
  obtain ⟨t, ht⟩ := h
  rw [chainOf_union, ← ht, List.filter_append]
  have h1 : (chainOf F₁ a).filter (fun r => decide (r ∉ F₁.recs)) = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro r hr
    simpa using (mem_chainOf.mp hr).1
  have h2 : t.filter (fun r => decide (r ∉ F₁.recs)) = t := by
    apply List.filter_eq_self.mpr
    intro r hr
    have hb : r ∈ chainOf F₂ a := ht ▸ List.mem_append_right _ hr
    have hact := (mem_chainOf.mp hb).2
    simp only [decide_eq_true_eq]
    intro hin
    rw [← ht] at hn
    exact (List.nodup_append.mp hn).2.2 r (mem_chainOf.mpr ⟨hin, hact⟩) r hr rfl
  rw [h1, h2, List.nil_append]

/-- Our chain is the longer: the union's is ours. -/
theorem union_chain' {a : A} (h : chainOf F₂ a <+: chainOf F₁ a) :
    chainOf (union F₁ F₂) a = chainOf F₁ a := by
  rw [chainOf_union]
  have : (chainOf F₂ a).filter (fun r => decide (r ∉ F₁.recs)) = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro r hr
    have := h.subset hr
    simpa using (mem_chainOf.mp this).1
  rw [this, List.append_nil]

/-- A seal's facts stand in the union, given validity there. -/
theorem facts_lift {F : File A I P G} {s : Seal A I G} (hsub : ∀ r ∈ F.recs, r ∈ (union F₁ F₂).recs)
    (h : SealFacts W F s) (hv : Valid W (union F₁ F₂) s.actor s.seq) :
    SealFacts W (union F₁ F₂) s := by
  obtain ⟨⟨t, ht, h1, h2, h3⟩, _, hsig⟩ := h
  exact ⟨⟨t, hsub t ht, h1, h2, h3⟩, hv, hsig⟩

/-- Admission lifts: nothing a side admitted falls in the union, past-REVOKE tails excluded. -/
theorem admitted_lift {F : File A I P G} (hsub : ∀ r ∈ F.recs, r ∈ (union F₁ F₂).recs)
    (hsubs : ∀ s ∈ F.seals, s ∈ (union F₁ F₂).seals) (hp : NoPastRevoke W F₁ F₂) {a : A}
    (h : Admitted W F a) : Admitted W (union F₁ F₂) a := by
  induction h with
  | root ht => exact .root ht
  | byAdmit r s hr hs hb ha hle hf _ ih =>
    obtain ⟨⟨t, ht, h1, h2, _⟩, _, _⟩ := id hf
    have hv : Valid W (union F₁ F₂) s.actor s.seq := by
      have := hp t (hsub t ht); rwa [h1, h2] at this
    exact .byAdmit r s (hsub r hr) (hsubs s hs) hb ha hle (facts_lift hsub hf hv) ih

/-- THE FORWARD DIRECTION. Both sides hold on one frontier, neither forked, no record past a REVOKE,
  no collision: the union holds. -/
theorem union_holds (hA : Holds W F₁) (hB : Holds W F₂) (hrows : F₁.rows = F₂.rows)
    (hf : NoFork F₁ F₂) (hp : NoPastRevoke W F₁ F₂) (hc : NoCollision (F₁.recs ++ F₂.recs)) :
    Holds W (union F₁ F₂) := by
  have sub₁ : ∀ r ∈ F₁.recs, r ∈ (union F₁ F₂).recs := fun _ h => union_keeps.mpr (.inl h)
  have sub₂ : ∀ r ∈ F₂.recs, r ∈ (union F₁ F₂).recs := fun _ h => union_keeps.mpr (.inr h)
  have sub₁s : ∀ s ∈ F₁.seals, s ∈ (union F₁ F₂).seals := fun _ h => union_keeps_seal.mpr (.inl h)
  have sub₂s : ∀ s ∈ F₂.seals, s ∈ (union F₁ F₂).seals := fun _ h => union_keeps_seal.mpr (.inr h)
  have mem_both : ∀ r ∈ (union F₁ F₂).recs, r ∈ F₁.recs ++ F₂.recs := fun r h =>
    List.mem_append.mpr (union_keeps.mp h)
  have hub : ∀ s ∈ (union F₁ F₂).seals, SealFacts W (union F₁ F₂) s ∧ Admitted W (union F₁ F₂) s.actor := by
    intro s hs
    rcases union_keeps_seal.mp hs with h | h
    · obtain ⟨hff, had⟩ := hA.sealsOk s h
      obtain ⟨⟨t, ht, h1, h2, _⟩, _, _⟩ := id hff
      have hv : Valid W (union F₁ F₂) s.actor s.seq := by
        have := hp t (sub₁ t ht); rwa [h1, h2] at this
      exact ⟨facts_lift sub₁ hff hv, admitted_lift sub₁ sub₁s hp had⟩
    · obtain ⟨hff, had⟩ := hB.sealsOk s h
      obtain ⟨⟨t, ht, h1, h2, _⟩, _, _⟩ := id hff
      have hv : Valid W (union F₁ F₂) s.actor s.seq := by
        have := hp t (sub₂ t ht); rwa [h1, h2] at this
      exact ⟨facts_lift sub₂ hff hv, admitted_lift sub₂ sub₂s hp had⟩
  have nodupB : ∀ a, (chainOf F₂ a).Nodup := fun a => hB.nodup.filter _
  refine
    { ids := ?_, nodup := ?_, idsOnce := ?_, noFork := ?_, device := ?_, sealsOk := hub,
      sealed := ?_, vouched := ?_, ends := ?_, owned := ?_ }
  · intro r h
    rcases union_keeps.mp h with h | h
    · exact hA.ids r h
    · exact hB.ids r h
  · refine List.nodup_append.mpr ⟨hA.nodup, hB.nodup.filter _, ?_⟩
    intro x hx y hy he
    have := (List.mem_filter.mp hy).2
    simp only [decide_eq_true_eq] at this
    exact this (he ▸ hx)
  · intro x hx y hy he
    exact hc x (mem_both x hx) y (mem_both y hy) he
  · intro x hx y hy ha hs
    have cross : ∀ {u v : Rec A I P}, u ∈ F₁.recs → v ∈ F₂.recs → u.actor = v.actor →
        u.seq = v.seq → u = v := by
      intro u v hu hv hact hseq
      have m₁ : u ∈ chainOf F₁ v.actor := mem_chainOf.mpr ⟨hu, hact⟩
      have m₂ : v ∈ chainOf F₂ v.actor := mem_chainOf.mpr ⟨hv, rfl⟩
      rcases hf v.actor with h | h
      · exact hB.noFork u (mem_chainOf.mp (h.subset m₁)).1 v hv hact hseq
      · exact hA.noFork u hu v (mem_chainOf.mp (h.subset m₂)).1 hact hseq
    rcases union_keeps.mp hx with h | h <;> rcases union_keeps.mp hy with h' | h'
    · exact hA.noFork x h y h' ha hs
    · exact cross h h' ha hs
    · exact (cross h' h ha.symm hs.symm).symm
    · exact hB.noFork x h y h' ha hs
  · intro a
    show DeviceOk F₁.rows a (chainOf (union F₁ F₂) a)
    rcases hf a with h | h
    · rw [union_chain (nodupB a) h, hrows]; exact hB.device a
    · rw [union_chain' h]; exact hA.device a
  · intro h
    have hnil : F₁.seals = [] := by
      have : (union F₁ F₂).seals = F₁.seals ++ _ := rfl
      rw [this] at h
      exact (List.append_eq_nil_iff.mp h).1
    exact hA.sealed hnil
  · intro r h
    rcases union_keeps.mp h with h | h
    · obtain ⟨s, hs, hre⟩ := hA.vouched r h
      exact ⟨s, sub₁s s hs, Reach.mono sub₁ hre⟩
    · obtain ⟨s, hs, hre⟩ := hB.vouched r h
      exact ⟨s, sub₂s s hs, Reach.mono sub₂ hre⟩
  · intro r h
    have lift : ∀ {F : File A I P G}, F.rows = F₁.rows → (∀ q ∈ F.recs, q ∈ (union F₁ F₂).recs) →
        ∀ p, EndsOk W F p → EndsOk W (union F₁ F₂) p := by
      intro F hr hs p he
      rcases he with h0 | ⟨f, hf', hft⟩ | ⟨q, hq, hqi⟩
      · exact .inl h0
      · exact .inr (.inl ⟨f, hr ▸ hf', hft⟩)
      · exact .inr (.inr ⟨q, hs q hq, hqi⟩)
    rcases union_keeps.mp h with h | h
    · exact lift rfl sub₁ _ (hA.ends r h)
    · exact lift hrows.symm sub₂ _ (hB.ends r h)
  · intro r h
    rcases union_keeps.mp h with h | h
    · obtain ⟨s, hs, ha, hle⟩ := hA.owned r h
      exact ⟨s, sub₁s s hs, ha, hle⟩
    · obtain ⟨s, hs, ha, hle⟩ := hB.owned r h
      exact ⟨s, sub₂s s hs, ha, hle⟩

/-- Two dense chains from one start, with no pair at one seq but equals, are comparable. -/
theorem dense_compat {a b : List (Rec A I P)} (la : Linked a) (lb : Linked b)
    (hs : ∀ x y, a.head? = some x → b.head? = some y → x.seq = y.seq)
    (hf : ∀ x ∈ a, ∀ y ∈ b, x.seq = y.seq → x = y) : Compat a b := by
  induction a generalizing b with
  | nil => exact .inl (List.nil_prefix)
  | cons x a' ih =>
    cases b with
    | nil => exact .inr (List.nil_prefix)
    | cons y b' =>
      have hxy := hf x List.mem_cons_self y List.mem_cons_self (hs x y rfl rfl)
      subst hxy
      have hc : Compat a' b' := by
        refine ih la.tail lb.tail ?_ ?_
        · intro u v hu hv
          cases a' with
          | nil => cases hu
          | cons m t =>
            cases b' with
            | nil => cases hv
            | cons n t' =>
              have hu' : m = u := by simpa using hu
              have hv' : n = v := by simpa using hv
              subst hu' hv'
              have h1 : m.seq = x.seq + 1 := la.2.1
              have h2 : n.seq = x.seq + 1 := lb.2.1
              omega
        · intro u hu v hv
          exact hf u (List.mem_cons_of_mem _ hu) v (List.mem_cons_of_mem _ hv)
      rcases hc with h | h
      · exact .inl ((List.prefix_cons_inj x).mpr h)
      · exact .inr ((List.prefix_cons_inj x).mpr h)

/-- FORKS DO NOT SURVIVE: a union that holds, of sides that hold on one frontier, was no fork. A
  conflicting pair at one (actor, seq) stays in the union by `union_keeps` and is its `noFork`. -/
theorem fork_breaks_union (hA : Holds W F₁) (hB : Holds W F₂) (hrows : F₁.rows = F₂.rows)
    (hU : Holds W (union F₁ F₂)) : NoFork F₁ F₂ := by
  intro a
  have start : ∀ {c d : List (Rec A I P)} {x y : Rec A I P}, DeviceOk F₁.rows a c →
      DeviceOk F₁.rows a d → c.head? = some x → d.head? = some y → x.seq = y.seq := by
    intro c d x y hc hd hx hy
    have s₁ := hc.2 x hx
    have s₂ := hd.2 y hy
    unfold Starts at s₁ s₂
    cases h : rowOf F₁.rows a with
    | none => rw [h] at s₁ s₂; omega
    | some f => rw [h] at s₁ s₂; omega
  have dB : DeviceOk F₁.rows a (chainOf F₂ a) := hrows ▸ hB.device a
  refine dense_compat (hA.device a).1 (hB.device a).1 (fun x y hx hy => start (hA.device a) dB hx hy) ?_
  intro x hx y hy hs
  have mx := mem_chainOf.mp hx
  have my := mem_chainOf.mp hy
  exact hU.noFork x (union_keeps.mpr (.inl mx.1)) y (union_keeps.mpr (.inr my.1))
    (mx.2.trans my.2.symm) hs

/-- THE IFF. For two sides that hold on one frontier: the union holds exactly when neither forked, no
  record sits past a REVOKE, and no id collides. -/
theorem merged_holds_iff (hA : Holds W F₁) (hB : Holds W F₂) (hrows : F₁.rows = F₂.rows) :
    Holds W (union F₁ F₂) ↔
      NoFork F₁ F₂ ∧ NoPastRevoke W F₁ F₂ ∧ NoCollision (F₁.recs ++ F₂.recs) := by
  constructor
  · intro hU
    refine ⟨fork_breaks_union hA hB hrows hU, ?_, ?_⟩
    · intro r hr
      obtain ⟨_, _, _, _, _, hv, _⟩ := signed_for hU hr
      exact hv
    · intro x hx y hy he
      have mem : ∀ {r : Rec A I P}, r ∈ F₁.recs ++ F₂.recs → r ∈ (union F₁ F₂).recs := fun h =>
        union_keeps.mpr (List.mem_append.mp h)
      exact hU.idsOnce x (mem hx) y (mem hy) he
  · rintro ⟨hf, hp, hc⟩
    exact union_holds hA hB hrows hf hp hc

/-- What `merge_into` checks before it writes: WHOLE on each side, one frontier, ONE HISTORY. -/
structure Merges (F₁ F₂ : File A I P G) (W : World A I P G) : Prop where
  left : Holds W F₁
  right : Holds W F₂
  rows : F₁.rows = F₂.rows
  noFork : NoFork F₁ F₂

/-- The sentence the card asked for, with its one missing clause in the open: what `merge_into`
  accepts and ends on a file that holds is exactly: both sides hold, on one frontier, neither forked,
  no record past a REVOKE, no id collision. -/
theorem merges_iff : (Merges F₁ F₂ W ∧ Holds W (union F₁ F₂)) ↔
    (Holds W F₁ ∧ Holds W F₂ ∧ F₁.rows = F₂.rows ∧ NoFork F₁ F₂ ∧ NoPastRevoke W F₁ F₂ ∧
      NoCollision (F₁.recs ++ F₂.recs)) := by
  constructor
  · rintro ⟨⟨hA, hB, hr, hf⟩, hU⟩
    obtain ⟨_, hp, hc⟩ := (merged_holds_iff hA hB hr).mp hU
    exact ⟨hA, hB, hr, hf, hp, hc⟩
  · rintro ⟨hA, hB, hr, hf, hp, hc⟩
    exact ⟨⟨hA, hB, hr, hf⟩, union_holds hA hB hr hf hp hc⟩

/-! ### The frontier of the file -/

/-- Per actor, the highest seq of a file. -/
def frontOf (F : File A I P G) : A → Nat := front Rec.actor Rec.seq F.recs

theorem union_comm_front (F₁ F₂ : File A I P G) :
    frontOf (union F₁ F₂) = frontOf (union F₂ F₁) :=
  front_union_comm _ _ _ _

theorem union_assoc_front (F₁ F₂ F₃ : File A I P G) :
    frontOf (union (union F₁ F₂) F₃) = frontOf (union F₁ (union F₂ F₃)) :=
  front_union_assoc _ _ _ _ _

theorem union_idem_front (F : File A I P G) : frontOf (union F F) = frontOf F :=
  front_union_idem _ _ _

end Bebop.DocLog

#print axioms Bebop.DocLog.union_holds
#print axioms Bebop.DocLog.fork_breaks_union
#print axioms Bebop.DocLog.merged_holds_iff
#print axioms Bebop.DocLog.merges_iff
#print axioms Bebop.DocLog.union_assoc_front
