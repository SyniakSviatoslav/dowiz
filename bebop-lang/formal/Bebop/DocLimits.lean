/-
  Bebop.DocLimits -- W-MG-LEAN: what `holds` does NOT give, each limit a theorem or a counterexample
  in the model of Bebop.DocLog. Core Lean 4, no Mathlib, no `axiom`, no `sorry`. NOTHING HERE SAYS THE
  RUST IS PROVED, and a counterexample here is a file of the MODEL.

  rollback_holds IS THE LIMIT. A sealed prefix of a file that holds, holds. It is a true, shorter file:
  so one file cannot know a longer one exists. The hypotheses of `SealedPrefix` are what a commit
  boundary gives; `unsealed_prefix_fails` shows a bare record prefix does not hold, so they stay.

  PROVED, on small files over `Wz` (ids are numbers, signatures are trivially true, keys 1 and 2 are
  roots, body 101 = REVOKE key 1 after seq 1):
    fOne_holds, fTwo_holds, fRevoked_holds  the fixtures hold: `Holds` is inhabited, not vacuous
    rollback_holds            the limit, in general
    no_file_check_sees_rollback   any test true of every file that holds passes the rolled-back file
    rollback_hides_revoke     fRevoked holds and names a REVOKE of key 1; fOne, its prefix, holds; key
                              1 then writes on, fTwo holds: nothing in that file shows the REVOKE
    past_revoke_blocks_union  fRevoked and fTwo both hold, no fork, and their union does NOT hold:
                              two files see what one cannot, and "holds iff both hold and neither
                              forked" is false without NoPastRevoke
    unsealed_prefix_fails     a record prefix past the last seal does not hold
    union_holds_side_does_not a union can hold while a side does not: `merge_into` checks sides first
    ring_holds                a hash ring passes every field of Holds under NoCollision, and no walk
                              ends: the code's walk stops on `seen`, and the end at genesis is the
                              hash's, not a counter's
-/
import Bebop.DocMerge

set_option linter.unusedSectionVars false

namespace Bebop.DocLog

open Bebop.Sync

/-! ### Rollback, in general -/

section General

variable {A I P G : Type} [DecidableEq A]

/-- `P` is `F` as it stood at a commit boundary: per device a prefix, the seals it had, and what a
  commit gives: its own seals name records of its own, and vouch for, own and admit what it has. -/
structure SealedPrefix (W : World A I P G) (Q F : File A I P G) : Prop where
  rows : Q.rows = F.rows
  sub : Q.recs.Sublist F.recs
  chains : ∀ a, chainOf Q a <+: chainOf F a
  seals : ∀ s ∈ Q.seals, s ∈ F.seals
  sealed : Q.seals ≠ []
  names : ∀ s ∈ Q.seals, ∃ t ∈ Q.recs, t.actor = s.actor ∧ t.seq = s.seq ∧ t.id = s.tip
  admits : ∀ s ∈ Q.seals, Admitted W Q s.actor
  vouched : ∀ r ∈ Q.recs, ∃ s ∈ Q.seals, Reach Q.recs s.tip r
  ends : ∀ r ∈ Q.recs, EndsOk W Q r.prev
  owned : ∀ r ∈ Q.recs, ∃ s ∈ Q.seals, s.actor = r.actor ∧ r.seq ≤ s.seq

/-- A prefix of a dense chain is one. -/
theorem linked_of_prefix {c d : List (Rec A I P)} (h : c <+: d) (hd : Linked d) : Linked c := by
  obtain ⟨t, rfl⟩ := h
  induction c with
  | nil => trivial
  | cons x c ih =>
    cases c with
    | nil => trivial
    | cons y c => exact ⟨hd.1, hd.2.1, ih hd.2.2⟩

/-- THE LIMIT. Drop any tail at a commit boundary, REVOKEs and seals with it: what is left holds. -/
theorem rollback_holds {W : World A I P G} {Q F : File A I P G} (H : Holds W F)
    (hp : SealedPrefix W Q F) : Holds W Q := by
  have hmem : ∀ r ∈ Q.recs, r ∈ F.recs := fun _ h => hp.sub.subset h
  refine
    { ids := fun r h => H.ids r (hmem r h), nodup := H.nodup.sublist hp.sub,
      idsOnce := fun x hx y hy he => H.idsOnce x (hmem x hx) y (hmem y hy) he,
      noFork := fun x hx y hy ha hs => H.noFork x (hmem x hx) y (hmem y hy) ha hs,
      device := ?_, sealsOk := ?_, sealed := hp.sealed, vouched := hp.vouched, ends := hp.ends,
      owned := hp.owned }
  · intro a
    have hd := H.device a
    refine ⟨linked_of_prefix (hp.chains a) hd.1, fun x hx => ?_⟩
    rw [hp.rows]
    obtain ⟨t, ht⟩ := hp.chains a
    refine hd.2 x ?_
    rw [← ht]
    cases hq : chainOf Q a with
    | nil => rw [hq] at hx; cases hx
    | cons y l => rw [hq] at hx; exact hx
  · intro s hs
    obtain ⟨⟨_, hv, hsig⟩, _⟩ := H.sealsOk s (hp.seals s hs)
    exact ⟨⟨hp.names s hs, fun r hr after hrev => hv r (hmem r hr) after hrev, hsig⟩, hp.admits s hs⟩

/-- Any test of one file that passes every file that holds passes the rolled-back file. There is no
  test to write: the comparison needs a second file, or an anchor. -/
theorem no_file_check_sees_rollback {W : World A I P G} {Q F : File A I P G}
    (check : File A I P G → Bool) (hsound : ∀ X, Holds W X → check X = true) (H : Holds W F)
    (hp : SealedPrefix W Q F) : check Q = true :=
  hsound Q (rollback_holds H hp)

end General

/-! ### Small files -/

abbrev ZRec := Rec Nat Nat Nat
abbrev ZFile := File Nat Nat Nat Unit
abbrev ZSeal := Seal Nat Nat Unit

/-- Ids are `10a + s`, so an id says nothing of `prev`: a free hash, enough for a ring. -/
def Wz : World Nat Nat Nat Unit where
  zero := 0
  idOf a s _ _ := 10 * a + s
  ver _ _ _ := true
  trusted a := a = 1 ∨ a = 2
  admitOf _ := none
  revokeOf b := if b = 101 then some (1, 1) else none

def r1 : ZRec := ⟨11, 1, 1, 0, 0⟩
def r2 : ZRec := ⟨12, 1, 2, 11, 0⟩
/-- Key 2 writes a REVOKE of key 1 after seq 1. -/
def q1 : ZRec := ⟨21, 2, 1, 0, 101⟩
def s1 : ZSeal := ⟨1, 1, 11, 0, ()⟩
def s2 : ZSeal := ⟨1, 2, 12, 11, ()⟩
def t1 : ZSeal := ⟨2, 1, 21, 0, ()⟩

def fOne : ZFile := ⟨[r1], [s1], []⟩
def fTwo : ZFile := ⟨[r1, r2], [s1, s2], []⟩
def fRevoked : ZFile := ⟨[r1, q1], [s1, t1], []⟩

theorem chain_cases (F : ZFile) (a : Nat) (hrec : ∀ r ∈ F.recs, r.actor ≠ a) : chainOf F a = [] := by
  apply List.filter_eq_nil_iff.mpr
  intro r hr
  simpa using hrec r hr

theorem start_of_head {l : List ZRec} {r x : ZRec} {a : Nat} (hs : r.seq = 1)
    (hl : (r :: l).head? = some x) : Starts ([] : List (Row Nat Nat)) a x := by
  have e : r = x := by simpa using hl
  subst e
  exact hs

theorem fOne_holds : Holds Wz fOne where
  ids := by intro r hr; simp only [fOne, List.mem_singleton] at hr; subst hr; rfl
  nodup := by simp [fOne]
  idsOnce := by intro x hx y hy _; simp only [fOne, List.mem_singleton] at hx hy; rw [hx, hy]
  noFork := by intro x hx y hy _ _; simp only [fOne, List.mem_singleton] at hx hy; rw [hx, hy]
  device := by
    intro a
    by_cases h : a = 1
    · subst h
      rw [show chainOf fOne 1 = [r1] from rfl]
      exact ⟨trivial, fun x hx => start_of_head rfl hx⟩
    · rw [chain_cases _ a (by simp [fOne, r1]; omega)]; exact ⟨trivial, by simp⟩
  sealsOk := by
    intro s hs
    simp only [fOne, List.mem_singleton] at hs
    subst hs
    refine ⟨⟨⟨r1, by simp [fOne], rfl, rfl, rfl⟩, ?_, rfl⟩, .root (.inl rfl)⟩
    intro r hr after h
    simp only [fOne, List.mem_singleton] at hr
    subst hr
    simp [Wz, r1] at h
  sealed := by simp [fOne]
  vouched := by
    intro r hr
    simp only [fOne, List.mem_singleton] at hr
    subst hr
    exact ⟨s1, by simp [fOne], .tip (by simp [fOne]) rfl⟩
  ends := by intro r hr; simp only [fOne, List.mem_singleton] at hr; subst hr; exact .inl rfl
  owned := by
    intro r hr
    simp only [fOne, List.mem_singleton] at hr
    subst hr
    exact ⟨s1, by simp [fOne], rfl, Nat.le_refl _⟩

theorem fTwo_holds : Holds Wz fTwo where
  ids := by
    intro r hr
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl <;> rfl
  nodup := by simp [fTwo, r1, r2]
  idsOnce := by
    intro x hx y hy he
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hx hy
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;> simp_all [r1, r2]
  noFork := by
    intro x hx y hy _ hs
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hx hy
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;> simp_all [r1, r2]
  device := by
    intro a
    by_cases h : a = 1
    · subst h
      rw [show chainOf fTwo 1 = [r1, r2] from rfl]
      exact ⟨⟨rfl, rfl, trivial⟩, fun x hx => start_of_head rfl hx⟩
    · rw [chain_cases _ a (by simp [fTwo, r1, r2]; omega)]; exact ⟨trivial, by simp⟩
  sealsOk := by
    have hv : ∀ n, Valid Wz fTwo 1 n := by
      intro n r hr after h
      simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl <;> simp [Wz, r1, r2] at h
    intro s hs
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hs
    rcases hs with rfl | rfl
    · exact ⟨⟨⟨r1, by simp [fTwo], rfl, rfl, rfl⟩, hv _, rfl⟩, .root (.inl rfl)⟩
    · exact ⟨⟨⟨r2, by simp [fTwo], rfl, rfl, rfl⟩, hv _, rfl⟩, .root (.inl rfl)⟩
  sealed := by simp [fTwo]
  vouched := by
    intro r hr
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact ⟨s1, by simp [fTwo], .tip (by simp [fTwo]) rfl⟩
    · exact ⟨s2, by simp [fTwo], .tip (by simp [fTwo]) rfl⟩
  ends := by
    intro r hr
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact .inl rfl
    · exact .inr (.inr ⟨r1, by simp [fTwo], rfl⟩)
  owned := by
    intro r hr
    simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact ⟨s1, by simp [fTwo], rfl, Nat.le_refl _⟩
    · exact ⟨s2, by simp [fTwo], rfl, Nat.le_refl _⟩

theorem fRevoked_holds : Holds Wz fRevoked where
  ids := by
    intro r hr
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl <;> rfl
  nodup := by simp [fRevoked, r1, q1]
  idsOnce := by
    intro x hx y hy he
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hx hy
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;> simp_all [r1, q1]
  noFork := by
    intro x hx y hy ha _
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hx hy
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;> simp_all [r1, q1]
  device := by
    intro a
    by_cases h : a = 1
    · subst h
      rw [show chainOf fRevoked 1 = [r1] from rfl]
      exact ⟨trivial, fun x hx => start_of_head rfl hx⟩
    · by_cases h2 : a = 2
      · subst h2
        rw [show chainOf fRevoked 2 = [q1] from rfl]
        exact ⟨trivial, fun x hx => start_of_head rfl hx⟩
      · rw [chain_cases _ a (by simp [fRevoked, r1, q1]; omega)]; exact ⟨trivial, by simp⟩
  sealsOk := by
    have hv : ∀ a n, n ≤ 1 → Valid Wz fRevoked a n := by
      intro a n hn r hr after h
      simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl <;> simp [Wz, r1, q1] at h
      omega
    intro s hs
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hs
    rcases hs with rfl | rfl
    · exact ⟨⟨⟨r1, by simp [fRevoked], rfl, rfl, rfl⟩, hv _ _ (Nat.le_refl _), rfl⟩, .root (.inl rfl)⟩
    · exact ⟨⟨⟨q1, by simp [fRevoked], rfl, rfl, rfl⟩, hv _ _ (Nat.le_refl _), rfl⟩, .root (.inr rfl)⟩
  sealed := by simp [fRevoked]
  vouched := by
    intro r hr
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact ⟨s1, by simp [fRevoked], .tip (by simp [fRevoked]) rfl⟩
    · exact ⟨t1, by simp [fRevoked], .tip (by simp [fRevoked]) rfl⟩
  ends := by
    intro r hr
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl <;> exact .inl rfl
  owned := by
    intro r hr
    simp only [fRevoked, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact ⟨s1, by simp [fRevoked], rfl, Nat.le_refl _⟩
    · exact ⟨t1, by simp [fRevoked], rfl, Nat.le_refl _⟩

/-- The rollback is a `SealedPrefix`. -/
theorem fOne_prefix_of_fRevoked : SealedPrefix Wz fOne fRevoked where
  rows := rfl
  sub := by simp [fOne, fRevoked]
  chains := by
    intro a
    by_cases h : a = 1
    · subst h; exact List.prefix_refl _
    · rw [chain_cases fOne a (by simp [fOne, r1]; omega)]; exact List.nil_prefix
  seals := by intro s hs; simp only [fOne, List.mem_singleton] at hs; subst hs; simp [fRevoked]
  sealed := by simp [fOne]
  names := fun s hs => (fOne_holds.sealsOk s hs).1.1
  admits := fun s hs => (fOne_holds.sealsOk s hs).2
  vouched := fOne_holds.vouched
  ends := fOne_holds.ends
  owned := fOne_holds.owned

/-- A REVOKE stands in one file, its prefix holds without it, and the revoked key writes on, past the
  REVOKE, in a file that holds. Nothing in `fTwo` shows it. -/
theorem rollback_hides_revoke :
    Holds Wz fRevoked ∧ (∃ r ∈ fRevoked.recs, Wz.revokeOf r.body = some (1, 1)) ∧
      Holds Wz fOne ∧ SealedPrefix Wz fOne fRevoked ∧
      Holds Wz fTwo ∧ (∀ r ∈ fTwo.recs, Wz.revokeOf r.body = none) ∧ (∃ r ∈ fTwo.recs, r.actor = 1 ∧ 1 < r.seq) := by
  refine ⟨fRevoked_holds, ⟨q1, by simp [fRevoked], rfl⟩, fOne_holds, fOne_prefix_of_fRevoked, fTwo_holds, ?_,
    ⟨r2, by simp [fTwo], rfl, by decide⟩⟩
  intro r hr
  simp only [fTwo, List.mem_cons, List.not_mem_nil, or_false] at hr
  rcases hr with rfl | rfl <;> simp [Wz, r1, r2]

/-- Two files see what one cannot: both hold, one history per device, and the union does not hold. -/
theorem past_revoke_blocks_union :
    Holds Wz fRevoked ∧ Holds Wz fTwo ∧ NoFork fRevoked fTwo ∧ ¬ Holds Wz (union fRevoked fTwo) := by
  refine ⟨fRevoked_holds, fTwo_holds, ?_, ?_⟩
  · intro a
    by_cases h : a = 1
    · subst h; exact .inl ⟨[r2], rfl⟩
    · by_cases h2 : a = 2
      · subst h2; exact .inr ⟨[q1], rfl⟩
      · have e₁ : chainOf fRevoked a = [] := chain_cases _ a (by simp [fRevoked, r1, q1]; omega)
        rw [e₁]; exact .inl (List.nil_prefix)
  · intro hU
    have hr2 : r2 ∈ (union fRevoked fTwo).recs := union_keeps.mpr (.inr (by simp [fTwo]))
    have hq1 : q1 ∈ (union fRevoked fTwo).recs := union_keeps.mpr (.inl (by simp [fRevoked]))
    obtain ⟨_, _, _, _, _, hv, _⟩ := signed_for hU hr2
    have := hv q1 hq1 1 rfl
    simp [r2] at this

/-- A record prefix past the last seal does not hold: the boundary is a hypothesis of the limit. -/
theorem unsealed_prefix_fails : ¬ Holds Wz ⟨[r1, r2], [s1], []⟩ := by
  intro H
  obtain ⟨s, hs, _, hle⟩ := H.owned r2 (by simp)
  simp only [List.mem_singleton] at hs
  subst hs
  simp [s1, r2] at hle

/-- A union can hold while a side does not: nothing in the union says a side was whole. -/
theorem union_holds_side_does_not :
    ¬ Holds Wz ⟨[r1], [], []⟩ ∧ Holds Wz (union ⟨[r1], [], []⟩ fOne) := by
  refine ⟨fun H => H.sealed rfl, ?_⟩
  have : union ⟨[r1], [], []⟩ fOne = fOne := by
    simp [union, unionL, fOne]
  rw [this]; exact fOne_holds

/-! ### A ring passes -/

def x1 : ZRec := ⟨11, 1, 1, 21, 0⟩
def y1 : ZRec := ⟨21, 2, 1, 11, 0⟩
def fRing : ZFile := ⟨[x1, y1], [⟨1, 1, 11, 0, ()⟩, ⟨2, 1, 21, 0, ()⟩], []⟩

theorem ring_green : Holds Wz fRing where
  ids := by
    intro r hr
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl <;> rfl
  nodup := by simp [fRing, x1, y1]
  idsOnce := by
    intro x hx y hy he
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hx hy
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;> simp_all [x1, y1]
  noFork := by
    intro x hx y hy ha _
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hx hy
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;> simp_all [x1, y1]
  device := by
    intro a
    by_cases h : a = 1
    · subst h
      rw [show chainOf fRing 1 = [x1] from rfl]
      exact ⟨trivial, fun x hx => start_of_head rfl hx⟩
    · by_cases h2 : a = 2
      · subst h2
        rw [show chainOf fRing 2 = [y1] from rfl]
        exact ⟨trivial, fun x hx => start_of_head rfl hx⟩
      · rw [chain_cases _ a (by simp [fRing, x1, y1]; omega)]; exact ⟨trivial, by simp⟩
  sealsOk := by
    have hv : ∀ a n, Valid Wz fRing a n := by
      intro a n r hr after h
      simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hr
      rcases hr with rfl | rfl <;> simp [Wz, x1, y1] at h
    intro s hs
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hs
    rcases hs with rfl | rfl
    · exact ⟨⟨⟨x1, by simp [fRing], rfl, rfl, rfl⟩, hv _ _, rfl⟩, .root (.inl rfl)⟩
    · exact ⟨⟨⟨y1, by simp [fRing], rfl, rfl, rfl⟩, hv _ _, rfl⟩, .root (.inr rfl)⟩
  sealed := by simp [fRing]
  vouched := by
    intro r hr
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact ⟨⟨1, 1, 11, 0, ()⟩, by simp [fRing], .tip (by simp [fRing]) rfl⟩
    · exact ⟨⟨2, 1, 21, 0, ()⟩, by simp [fRing], .tip (by simp [fRing]) rfl⟩
  ends := by
    intro r hr
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact .inr (.inr ⟨y1, by simp [fRing], rfl⟩)
    · exact .inr (.inr ⟨x1, by simp [fRing], rfl⟩)
  owned := by
    intro r hr
    simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with rfl | rfl
    · exact ⟨⟨1, 1, 11, 0, ()⟩, by simp [fRing], rfl, Nat.le_refl _⟩
    · exact ⟨⟨2, 1, 21, 0, ()⟩, by simp [fRing], rfl, Nat.le_refl _⟩

/-- A file with no collision, every field green, and no record whose walk ends. "Chains end at genesis
  or a frontier tip" is not a counter's result: it is the hash, which is `chain_ends`'s hypothesis. -/
theorem ring_holds : Holds Wz fRing ∧ NoCollision fRing.recs ∧
    ∀ r ∈ fRing.recs, r.prev ≠ Wz.zero ∧ ∀ f ∈ fRing.rows, f.tip ≠ r.prev := by
  refine ⟨ring_green, ring_green.idsOnce, ?_⟩
  intro r hr
  simp only [fRing, List.mem_cons, List.not_mem_nil, or_false] at hr
  rcases hr with rfl | rfl <;> simp [Wz, fRing, x1, y1]

end Bebop.DocLog

#print axioms Bebop.DocLog.rollback_holds
#print axioms Bebop.DocLog.rollback_hides_revoke
#print axioms Bebop.DocLog.past_revoke_blocks_union
#print axioms Bebop.DocLog.unsealed_prefix_fails
#print axioms Bebop.DocLog.union_holds_side_does_not
#print axioms Bebop.DocLog.ring_holds
