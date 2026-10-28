/-
  Bebop.Merkle -- LV-INC-D (docs/research/R-BEBOP-LEVERS-2026-10-27.md §4.4, AC-LV-PCD-06): the
  published root and its inclusion proof, and the certifying fold of a stock day close. Core Lean 4,
  no Mathlib, no `axiom`, no `sorry`.

  THE MODEL (Merkle 1987; Miller, Hicks, Katz, Shi, POPL 2014):
    * the hash `H : β → β → β` is a PARAMETER. Collision resistance is NOT an axiom here (the
      `lean.axioms` gate takes no new line for it); where a theorem needs it, it is a HYPOTHESIS
      (`hinj`: H is injective on pairs) and the theorem says exactly what it buys;
    * a tree is leaves and binary nodes; `root` hashes it bottom-up;
    * a path is a list of (side of the SIBLING, sibling hash), TOP-DOWN: the first pair is the one
      next to the root. Rust (`crates/bebop-store/src/merkle.rs`, LV-PCD-C) keeps the path
      bottom-up, leaf first; the model's path is that array REVERSED, nothing else;
    * `climb` recomputes the root from a leaf and a path; `verify` compares it with a root.

  PROVED:
    verify_path         verify H leaf p r = true ↔ r = climb H p leaf  (the verifier is the climb)
    prove_sound         every path `prove` emits for a leaf of t verifies against root t
    prove_length        that path is exactly as long as the position (one hash per level)
    verify_binding      WITH hinj and leaves kept apart from nodes (`LeavesSep`): a path that
                        verifies against root t climbs from the root of a real subtree of t at a
                        position as long as the path -- a forged path proves nothing
    verify_binding_leaf with a leaf-domain hash (H a b ≠ h for all a b, RFC 6962's 0x00/0x01 prefix):
                        that subtree is the leaf h itself
    witness_sound / witness_complete  the certifying day close: a witness of running levels is
                        accepted exactly when the close is the fold of the moves -- k additions, no
                        sha256 over the whole log

  NOT COVERED: SHA-256 itself, the byte encoding of a path (≤ 256 B is a Rust test), odd leaf counts
  (how the Rust tree pads), and the ML-DSA signature over the root (crates/dowiz-core/src/pq, KAT-gated).
-/

namespace Bebop.Merkle

variable {β : Type}

inductive Tree (β : Type) where
  | leaf (h : β)
  | node (l r : Tree β)

def root (H : β → β → β) : Tree β → β
  | .leaf h => h
  | .node l r => H (root H l) (root H r)

/-- Where the SIBLING sits. -/
inductive Side where
  | left
  | right

def combine (H : β → β → β) : Side → β → β → β
  | .left, sib, x => H sib x
  | .right, sib, x => H x sib

/-- Top-down: the head is the pair next to the root. -/
abbrev Path (β : Type) := List (Side × β)

def climb (H : β → β → β) : Path β → β → β
  | [], x => x
  | (s, sib) :: p, x => combine H s sib (climb H p x)

def verify [DecidableEq β] (H : β → β → β) (leaf : β) (p : Path β) (r : β) : Bool :=
  decide (climb H p leaf = r)

/-- The verifier is the climb: nothing else is checked, nothing is searched. -/
theorem verify_path [DecidableEq β] (H : β → β → β) (leaf : β) (p : Path β) (r : β) :
    verify H leaf p r = true ↔ r = climb H p leaf := by
  simp [verify, eq_comm]

/-- The leaf at a position (false = left, true = right, top-down) and its path. -/
def prove (H : β → β → β) : Tree β → List Bool → Option (β × Path β)
  | .leaf h, [] => some (h, [])
  | .leaf _, _ :: _ => none
  | .node _ _, [] => none
  | .node l r, false :: pos =>
    match prove H l pos with
    | some (h, p) => some (h, (.right, root H r) :: p)
    | none => none
  | .node l r, true :: pos =>
    match prove H r pos with
    | some (h, p) => some (h, (.left, root H l) :: p)
    | none => none

theorem prove_climb (H : β → β → β) : ∀ (t : Tree β) (pos : List Bool) (h : β) (p : Path β),
    prove H t pos = some (h, p) → climb H p h = root H t ∧ p.length = pos.length
  | .leaf x, [], h, p, e => by
    simp only [prove, Option.some.injEq, Prod.mk.injEq] at e
    obtain ⟨rfl, rfl⟩ := e
    exact ⟨rfl, rfl⟩
  | .leaf _, _ :: _, _, _, e => by simp [prove] at e
  | .node _ _, [], _, _, e => by simp [prove] at e
  | .node l r, false :: pos, h, p, e => by
    simp only [prove] at e
    cases e' : prove H l pos with
    | none => rw [e'] at e; simp at e
    | some q =>
      obtain ⟨h', p'⟩ := q
      rw [e'] at e
      simp only [Option.some.injEq, Prod.mk.injEq] at e
      obtain ⟨rfl, rfl⟩ := e
      obtain ⟨hc, hl⟩ := prove_climb H l pos _ p' e'
      refine ⟨?_, by simp [hl]⟩
      show combine H .right (root H r) (climb H p' _) = H (root H l) (root H r)
      rw [hc]; rfl
  | .node l r, true :: pos, h, p, e => by
    simp only [prove] at e
    cases e' : prove H r pos with
    | none => rw [e'] at e; simp at e
    | some q =>
      obtain ⟨h', p'⟩ := q
      rw [e'] at e
      simp only [Option.some.injEq, Prod.mk.injEq] at e
      obtain ⟨rfl, rfl⟩ := e
      obtain ⟨hc, hl⟩ := prove_climb H r pos _ p' e'
      refine ⟨?_, by simp [hl]⟩
      show combine H .left (root H l) (climb H p' _) = H (root H l) (root H r)
      rw [hc]; rfl

theorem prove_sound [DecidableEq β] (H : β → β → β) (t : Tree β) (pos : List Bool) (h : β)
    (p : Path β) (e : prove H t pos = some (h, p)) : verify H h p (root H t) = true :=
  (verify_path H h p _).2 (prove_climb H t pos h p e).1.symm

theorem prove_length (H : β → β → β) (t : Tree β) (pos : List Bool) (h : β) (p : Path β)
    (e : prove H t pos = some (h, p)) : p.length = pos.length :=
  (prove_climb H t pos h p e).2

/-! ### Binding: what a verifying path proves, under collision resistance as a hypothesis -/

def subtree : Tree β → List Bool → Option (Tree β)
  | t, [] => some t
  | .leaf _, _ :: _ => none
  | .node l _, false :: pos => subtree l pos
  | .node _ r, true :: pos => subtree r pos

/-- No leaf value is a node hash (the leaf/node domain separation). -/
def LeavesSep (H : β → β → β) : Tree β → Prop
  | .leaf x => ∀ a b, H a b ≠ x
  | .node l r => LeavesSep H l ∧ LeavesSep H r

theorem verify_binding (H : β → β → β) (hinj : ∀ a b c d, H a b = H c d → a = c ∧ b = d) :
    ∀ (p : Path β) (t : Tree β) (h : β), LeavesSep H t → climb H p h = root H t →
    ∃ pos s, subtree t pos = some s ∧ root H s = h ∧ pos.length = p.length
  | [], t, h, _, e => ⟨[], t, by cases t <;> rfl, e.symm, rfl⟩
  | (sd, sib) :: p, .leaf x, h, hs, e => absurd e (by cases sd <;> exact hs _ _)
  | (sd, sib) :: p, .node l r, h, hs, e => by
    cases sd with
    | right =>
      have hlr := hinj _ _ _ _ e
      obtain ⟨pos, s, h1, h2, h3⟩ := verify_binding H hinj p l h hs.1 hlr.1
      exact ⟨false :: pos, s, h1, h2, by simp [h3]⟩
    | left =>
      have hlr := hinj _ _ _ _ e
      obtain ⟨pos, s, h1, h2, h3⟩ := verify_binding H hinj p r h hs.2 hlr.2
      exact ⟨true :: pos, s, h1, h2, by simp [h3]⟩

/-- With a leaf-domain hash, the subtree a verifying path reaches is that leaf. -/
theorem verify_binding_leaf (H : β → β → β) (hinj : ∀ a b c d, H a b = H c d → a = c ∧ b = d)
    (p : Path β) (t : Tree β) (h : β) (hs : LeavesSep H t) (hh : ∀ a b, H a b ≠ h)
    (e : climb H p h = root H t) :
    ∃ pos, subtree t pos = some (.leaf h) ∧ pos.length = p.length := by
  obtain ⟨pos, s, h1, h2, h3⟩ := verify_binding H hinj p t h hs e
  cases s with
  | leaf x => exact ⟨pos, by simp only [root] at h2; rw [h1, h2], h3⟩
  | node a b => exact absurd h2 (hh _ _)

/-! ### The certifying fold: a stock day close checked by k additions -/

/-- The witness: the running level after each move. Accepted when every level is the previous one
    plus its move and the last one is the close. -/
def checkRun : Int → List Int → List Int → Int → Bool
  | o, [], [], c => decide (o = c)
  | o, m :: ms, l :: ls, c => decide (l = o + m) && checkRun l ms ls c
  | _, _, _, _ => false

/-- The witness an honest prover emits. -/
def runSums : Int → List Int → List Int
  | _, [] => []
  | o, m :: ms => (o + m) :: runSums (o + m) ms

theorem witness_sound : ∀ (o : Int) (ms ls : List Int) (c : Int),
    checkRun o ms ls c = true → c = ms.foldl (· + ·) o
  | o, [], [], c, h => by simp [checkRun] at h; simp [h]
  | _, [], _ :: _, _, h => by simp [checkRun] at h
  | _, _ :: _, [], _, h => by simp [checkRun] at h
  | o, m :: ms, l :: ls, c, h => by
    simp only [checkRun, Bool.and_eq_true, decide_eq_true_eq] at h
    obtain ⟨hl, hr⟩ := h
    have := witness_sound l ms ls c hr
    rw [hl] at this
    exact this

theorem witness_complete : ∀ (o : Int) (ms : List Int),
    checkRun o ms (runSums o ms) (ms.foldl (· + ·) o) = true
  | o, [] => by simp [checkRun, runSums]
  | o, m :: ms => by
    simp only [runSums, checkRun, Bool.and_eq_true, decide_eq_true_eq, List.foldl]
    exact ⟨trivial, witness_complete (o + m) ms⟩

/-! ### Evaluated examples (Nat with a toy H: the shape, not the security) -/

private def toyH (a b : Nat) : Nat := 2 * a + 3 * b + 1
private def t4 : Tree Nat := .node (.node (.leaf 2) (.leaf 4)) (.node (.leaf 6) (.leaf 8))

#guard (prove toyH t4 [true, false]).map (·.1) == some 6
#guard match prove toyH t4 [true, false] with
  | some (h, p) => verify toyH h p (root toyH t4) && p.length == 2
  | none => false
#guard match prove toyH t4 [true, false] with
  | some (h, p) => !verify toyH (h + 1) p (root toyH t4)
  | none => false
#guard checkRun 10 [5, -3, -2] [15, 12, 10] 10
#guard !checkRun 10 [5, -3, -2] [15, 12, 11] 11

end Bebop.Merkle

#print axioms Bebop.Merkle.verify_path
#print axioms Bebop.Merkle.prove_sound
#print axioms Bebop.Merkle.prove_length
#print axioms Bebop.Merkle.verify_binding
#print axioms Bebop.Merkle.verify_binding_leaf
#print axioms Bebop.Merkle.witness_sound
#print axioms Bebop.Merkle.witness_complete
