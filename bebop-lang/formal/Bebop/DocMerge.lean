/-
  Bebop.DocMerge -- W-MG-LEAN: the MERGE of two chains from one genesis, over the model of
  Bebop.DocLog. Core Lean 4, no Mathlib, no `axiom`, no `sorry`.

  THE CODE IT MODELS, BY READING: crates/einfile-log/src/doc/merge.rs `Doc::merge_into`. NOTHING HERE
  SAYS THE RUST IS PROVED. `union` is records ours-then-their-tail and seals likewise; the frontier
  rows must be equal (`CutOneSide`). The model does not write a MERGE: nor does `merge_into`.
  `Merges` is every refusal of `merge_into` the model can state, in the code's order; WHAT IT CANNOT
  STATE is in the section "Where this model and merge.rs differ" at the end of this file.

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
    last_verify_is_collision   past every other check, the union's last verify can fail on a collision
                        and on nothing else IN THE MODEL. In the code it fails on more: see the end.
    union_chain_seals, union_chain_seals'   per device the union keeps the longer seal chain
    union_nothing_new   nothing new on their side: the union IS ours (`wrote` false)
    frontOf_kept        a device with records stands at them, above its row
    union_front_max     the frontier of the union is the pointwise maximum, rows included
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

/-- A device's seals in file order: its chain, the model having no HEADS row. -/
def sealsOf (F : File A I P G) (a : A) : List (Seal A I G) := F.seals.filter (fun s => decide (s.actor = a))

/-- `SealFork`'s negation: per device, one side's seals are a prefix of the other's. -/
def NoSealFork (F₁ F₂ : File A I P G) : Prop := ∀ a, Compat (sealsOf F₁ a) (sealsOf F₂ a)

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

theorem mem_sealsOf {F : File A I P G} {a : A} {s : Seal A I G} :
    s ∈ sealsOf F a ↔ s ∈ F.seals ∧ s.actor = a := by
  simp [sealsOf, List.mem_filter]

/-- The device's seals in the union: ours, then what of theirs we lack. -/
theorem sealsOf_union (a : A) : sealsOf (union F₁ F₂) a =
    sealsOf F₁ a ++ (sealsOf F₂ a).filter (fun s => decide (s ∉ F₁.seals)) := by
  simp only [sealsOf, union, unionL, List.filter_append, List.filter_filter]
  congr 2
  funext s
  exact Bool.and_comm ..

/-- Their seal chain is the longer: the union's is theirs (`seal_chains`, a seal carried with no new
  record). -/
theorem union_chain_seals {a : A} (hn : (sealsOf F₂ a).Nodup) (h : sealsOf F₁ a <+: sealsOf F₂ a) :
    sealsOf (union F₁ F₂) a = sealsOf F₂ a := by
  obtain ⟨t, ht⟩ := h
  rw [sealsOf_union, ← ht, List.filter_append]
  have h1 : (sealsOf F₁ a).filter (fun s => decide (s ∉ F₁.seals)) = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro s hs
    simpa using (mem_sealsOf.mp hs).1
  have h2 : t.filter (fun s => decide (s ∉ F₁.seals)) = t := by
    apply List.filter_eq_self.mpr
    intro s hs
    have hb : s ∈ sealsOf F₂ a := ht ▸ List.mem_append_right _ hs
    have hact := (mem_sealsOf.mp hb).2
    simp only [decide_eq_true_eq]
    intro hin
    rw [← ht] at hn
    exact (List.nodup_append.mp hn).2.2 s (mem_sealsOf.mpr ⟨hin, hact⟩) s hs rfl
  rw [h1, h2, List.nil_append]

/-- Our seal chain is the longer: the union's is ours. -/
theorem union_chain_seals' {a : A} (h : sealsOf F₂ a <+: sealsOf F₁ a) :
    sealsOf (union F₁ F₂) a = sealsOf F₁ a := by
  rw [sealsOf_union]
  have : (sealsOf F₂ a).filter (fun s => decide (s ∉ F₁.seals)) = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro s hs
    have := h.subset hs
    simpa using (mem_sealsOf.mp this).1
  rw [this, List.append_nil]

theorem unionL_sub {R : Type} [DecidableEq R] {a b : List R} (h : ∀ r ∈ b, r ∈ a) : unionL a b = a := by
  have : b.filter (fun r => decide (r ∉ a)) = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro r hr
    simpa using h r hr
  simp only [unionL, this, List.append_nil]

/-- NOTHING NEW ON THEIR SIDE: the union IS ours, which is `wrote` false and no verify. -/
theorem union_nothing_new (hr : ∀ r ∈ F₂.recs, r ∈ F₁.recs) (hs : ∀ s ∈ F₂.seals, s ∈ F₁.seals) :
    union F₁ F₂ = F₁ := by
  show (⟨unionL F₁.recs F₂.recs, unionL F₁.seals F₂.seals, F₁.rows⟩ : File A I P G) = F₁
  rw [unionL_sub hr, unionL_sub hs]

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
      sealed := ?_, vouched := ?_, ends := ?_, owned := ?_, grounded := ?_ }
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
  · intro r h
    rcases union_keeps.mp h with h | h
    · exact (hA.grounded r h).mono sub₁ (fun _ hf => hf)
    · exact (hB.grounded r h).mono sub₂ (fun _ hf => by
        show _ ∈ F₁.rows
        rw [hrows]
        exact hf)

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

/-- What `merge_into` checks before it writes, in its order and as far as the model has the words:
  WHOLE on each side (`NotWhole`), one frontier (`CutOneSide`), ONE HISTORY (`Fork`), no record past a
  REVOKE (`RevokedTail`), one seal chain per device (`SealFork`). -/
structure Merges (F₁ F₂ : File A I P G) (W : World A I P G) : Prop where
  left : Holds W F₁
  right : Holds W F₂
  rows : F₁.rows = F₂.rows
  noFork : NoFork F₁ F₂
  noPast : NoPastRevoke W F₁ F₂
  noSealFork : NoSealFork F₁ F₂

/-- THE LAST VERIFY, IN THE MODEL. Past every check above, the union holds exactly when no id
  collides: `UnionNotWhole` is a collision and nothing else HERE. In the code it is more, because the
  model has no actor without a KEY row: see the end of this file. -/
theorem last_verify_is_collision (h : Merges F₁ F₂ W) :
    Holds W (union F₁ F₂) ↔ NoCollision (F₁.recs ++ F₂.recs) :=
  ⟨fun hU => ((merged_holds_iff h.left h.right h.rows).mp hU).2.2,
    fun hc => union_holds h.left h.right h.rows h.noFork h.noPast hc⟩

/-- The sentence the card asked for, every clause in the open: what `merge_into` accepts and ends on
  a file that holds is exactly: both sides hold, on one frontier, neither forked, no record past a
  REVOKE, one seal chain per device, no id collision. -/
theorem merges_iff : (Merges F₁ F₂ W ∧ Holds W (union F₁ F₂)) ↔
    (Holds W F₁ ∧ Holds W F₂ ∧ F₁.rows = F₂.rows ∧ NoFork F₁ F₂ ∧ NoPastRevoke W F₁ F₂ ∧
      NoSealFork F₁ F₂ ∧ NoCollision (F₁.recs ++ F₂.recs)) := by
  constructor
  · rintro ⟨h, hU⟩
    exact ⟨h.left, h.right, h.rows, h.noFork, h.noPast, h.noSealFork, (last_verify_is_collision h).mp hU⟩
  · rintro ⟨hA, hB, hr, hf, hp, hs, hc⟩
    exact ⟨⟨hA, hB, hr, hf, hp, hs⟩, union_holds hA hB hr hf hp hc⟩

/-! ### The frontier of the file -/

/-- The seq of a device's newest row, 0 with none. -/
def rowSeq (rows : List (Row A I)) (a : A) : Nat := ((rowOf rows a).map Row.seq).getD 0

/-- Per actor, the highest seq a file knows: its records, or the row a cut left when it kept none. -/
def frontOf (F : File A I P G) (a : A) : Nat :=
  max (rowSeq F.rows a) (front Rec.actor Rec.seq F.recs a)

/-- A device with records stands at them: a row is below every record of its device. -/
theorem frontOf_kept (H : Holds W F₁) {a : A} {r : Rec A I P} (hr : r ∈ F₁.recs) (ha : r.actor = a) :
    frontOf F₁ a = front Rec.actor Rec.seq F₁.recs a := by
  have hle : r.seq ≤ front Rec.actor Rec.seq F₁.recs a :=
    (front_le Rec.actor Rec.seq F₁.recs _ a).mp (Nat.le_refl _) r hr ha
  have hrow : rowSeq F₁.rows a ≤ r.seq := by
    unfold rowSeq
    cases h : rowOf F₁.rows a with
    | none => simp
    | some f => have := past_own_row H h hr ha; simp; omega
  exact Nat.max_eq_right (Nat.le_trans hrow hle)

/-- HEADS AS A POINTWISE MAXIMUM, on one frontier (`CutOneSide` refuses the rest). -/
theorem union_front_max (hrows : F₁.rows = F₂.rows) (a : A) :
    frontOf (union F₁ F₂) a = max (frontOf F₁ a) (frontOf F₂ a) := by
  show max (rowSeq F₁.rows a) (front Rec.actor Rec.seq (unionL F₁.recs F₂.recs) a) = _
  rw [front_union_max]
  simp only [frontOf, ← hrows]
  omega

theorem union_comm_front (hrows : F₁.rows = F₂.rows) (a : A) :
    frontOf (union F₁ F₂) a = frontOf (union F₂ F₁) a := by
  rw [union_front_max hrows, union_front_max hrows.symm, Nat.max_comm]

theorem union_assoc_front (F₁ F₂ F₃ : File A I P G) (a : A) :
    frontOf (union (union F₁ F₂) F₃) a = frontOf (union F₁ (union F₂ F₃)) a := by
  show max (rowSeq F₁.rows a) (front Rec.actor Rec.seq (unionL (unionL F₁.recs F₂.recs) F₃.recs) a) =
    max (rowSeq F₁.rows a) (front Rec.actor Rec.seq (unionL F₁.recs (unionL F₂.recs F₃.recs)) a)
  rw [front_union_assoc]

theorem union_idem_front (F : File A I P G) (a : A) : frontOf (union F F) a = frontOf F a := by
  show max (rowSeq F.rows a) (front Rec.actor Rec.seq (unionL F.recs F.recs) a) = _
  rw [front_union_idem]
  rfl

/-! ### Where this model and merge.rs differ

  A theorem above says nothing past these lines. Each is a place the code does something the model
  cannot state, read against crates/einfile-log/src/doc on 2026-11-01:

  * NO ACTOR WITHOUT A KEY. `count.rs` judges only keyed actors, and `merge.rs` says `keyed` is not
    monotone: a side's KEY row judges the other side's stranger in the union. That is the one way
    `merge_into`'s last verify fails in the tests (`lean_tests.rs`, crates/dowiz-hub/tests/m2_union_not_whole.rs),
    and `last_verify_is_collision` does not cover it: in the model every actor is judged on each side.
  * THE SEAL CHAIN. Here `sealsOf` is a filter in file order and a seal has no link. The code walks
    `prev` from a HEADS row, and `SealFork` exists because HEADS keeps one chain per device. So
    `union_holds` needs no `NoSealFork`: two chains hold here and are refused there. `same_seal` also
    compares `claimed_ms` and `alg`, which a model seal does not carry. `prevTip` is a free field
    here; `seals.rs` derives it from the previous seal, or the device's own frontier row.
  * THE ROW OF A DEVICE WITH NO KEPT SEAL. `frontOf` reads the SEG rows, which the union carries, so
    `union_front_max` keeps such a device. HEADS is another table: `cut.rs` keeps its row with no seal,
    and `stage_union` writes a row only for a chain with a last seal, so the union's HEADS has none.
    Nothing here can state it: the model has no HEADS, and no writer that reads one.
  * ONE FRONTIER IS MORE THAN ONE LIST OF ROWS. `CutOneSide` compares whole segments (`plain`: range,
    root, counts, `seg_hash`); `Merges.rows` compares rows, so `Merges` accepts files the code refuses.
  * ONE WORLD. Both sides are read under one `trusted`. `ForeignRoot`, `KeyClash` and the two genesis
    tables have no statement here; the code compares the tables first and reads `theirs` under our roots.
  * FORK AND FORGETONESIDE are one `NoFork`. The code tells them apart by id; tombstones, FORGOTTEN and
    `LegacyTail` are outside the model.
  * EVERY REVOKE IS EFFECTIVE here. `revoked` in merge.rs takes the REVOKEs each side's `verify` found
    effective, in `settle.rs`'s fixpoint; a REVOKE by a keyless record stops nothing there
    (`dc_a_non_effective_revoke_does_not_block_the_merge`) and has no form here. `after_seq` is a
    Nat, so the negative `REVOKED_ALL` is `after` = 0 alone.
  * A WALK STOPS AT A TIP HERE, and only at a non-record there: `Grounded.tip` accepts a `prev` that is a
    frontier tip even if a record also has that id, where `unsealed` would go on through the record.
  * `merges_bad`, `declared`, `tombs_foreign`, `keys_disagree`, anchors and the MERGE record itself
    are not in `Holds`. `both_wrote` is the caller's, and a union here is never the file the caller seals.
  * `Doc::tail_into` and `Front` (`tail.rs`, `front.rs`) are not modelled: this is `merge_into` alone.
-/

end Bebop.DocLog

#print axioms Bebop.DocLog.union_holds
#print axioms Bebop.DocLog.fork_breaks_union
#print axioms Bebop.DocLog.merged_holds_iff
#print axioms Bebop.DocLog.merges_iff
#print axioms Bebop.DocLog.last_verify_is_collision
#print axioms Bebop.DocLog.union_chain_seals
#print axioms Bebop.DocLog.union_nothing_new
#print axioms Bebop.DocLog.union_front_max
#print axioms Bebop.DocLog.union_assoc_front
