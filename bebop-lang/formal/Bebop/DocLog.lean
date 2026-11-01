/-
  Bebop.DocLog -- W-MG-LEAN: what `Report::holds` means, as a MODEL. Core Lean 4, no Mathlib, no
  `axiom`, no `sorry`.

  THE CODE IT MODELS, BY READING: crates/bebop-log/src/doc/{verify,count,seals,settle,declare}.rs and
  docs/FORMAT-BEBOP-IMAGE-v2.en.md §9.4. NOTHING HERE SAYS THE RUST IS PROVED. A theorem below is
  about the structure `Holds`; that the Rust counters compute `Holds` is a reading, and
  W-MG-LEAN's verdict lists each place where it does not.

  WHAT THE MODEL IS: a log of records (id, actor, seq, prev, body), seals (signed tips), frontier
  rows (what a cut left), and `Holds`: id formula, no fork, dense seq per device, every record
  reachable from a sealed tip, every chain ends at genesis, a frontier tip or a record, every seal ok
  (names a record, key admitted, valid at its seq, signature verifies), and every record at or below
  its own device's seals.

  WHAT THE MODEL IS NOT:
    * every actor has a KEY row. A pre-seal actor (no KEY row, any seq) is outside it; so are legacy
      ids and tombstones (M2 refuses both in a tail) and FORGOTTEN / TRUNCATE accounting.
    * an actor IS its key: `actor_of` always binds, and `KeyClash` cannot occur.
    * `Valid` is the fold of every REVOKE in the file. settle.rs's alternating fixpoint and
      `revokes_unsettled` are not modelled; in the model's scope every REVOKE's writer is covered by
      an ok seal, so every one is effective. A REVOKE naming seq 0 revokes all: seqs here are >= 1.
    * `Admitted` takes ANY ADMIT in the file; the code takes the one the KEY row's `ref ADMIT_REC`
      names. The model admits at least what the code admits: its conclusion is the weaker one.
    * no bytes, no crc, no HEADS rows, no budget, no `claimed_ms`, no TREE, u64 is Nat.

  HYPOTHESES, NEVER AXIOMS. Signature unforgeability is `Unforgeable` and only for a key whose holder
  is `honest`: a STOLEN key signs, and nothing here can tell. Collision resistance is `NoCollision`,
  over the finite list of records at issue, because a global "the hash is injective" is false of
  SHA-256; a list with no collision is what a real run has. Neither excludes a hash RING
  (DocLimits.ring_holds), so `chain_ends` takes a writing order as one more hypothesis.

  PROVED:
    signed_for          Holds: every record has an ok seal of ITS OWN device at or above it, whose tip
                        reaches it by prev; the key is admitted and valid at the RECORD's seq.
    reach_own_chain     dense + linked: a tip reaches every lower record of its device.
    signed_for_honest   with Unforgeable: the key's holder really signed that tip.
    signer_saw          with NoCollision: a record the signed tip reaches IS a record of the history
                        the signer held, bytes and all.
    holds_gives         the three together: the theorem the file promises.
    admission_signed    with Unforgeable: an admitted key is a trusted root, or an admitted holder
                        signed over its ADMIT. A root is what `trusted` says; under Trust::Genesis
                        that is the unsigned table: this theorem then says nothing about a forger.
    chain_ends          with a writing order: every walk ends at genesis or at a frontier tip.
-/

set_option linter.unusedSectionVars false

namespace Bebop.DocLog

structure Rec (A I P : Type) where
  id : I
  actor : A
  seq : Nat
  prev : I
  body : P
  deriving DecidableEq

structure Seal (A I G : Type) where
  actor : A
  seq : Nat
  tip : I
  prevTip : I
  sig : G
  deriving DecidableEq

/-- A frontier row: a device's newest cut record, as a segment recorded it. -/
structure Row (A I : Type) where
  actor : A
  seq : Nat
  tip : I
  deriving DecidableEq

structure File (A I P G : Type) where
  recs : List (Rec A I P)
  seals : List (Seal A I G)
  /-- Newest segment first, as `Frontiers::of` indexes them. -/
  rows : List (Row A I)

/-- The fixed outside: the id formula (§9.4), the signature check, the roots, the body parsers. -/
structure World (A I P G : Type) where
  zero : I
  idOf : A → Nat → I → P → I
  ver : A → (A × Nat × I × I) → G → Bool
  trusted : A → Prop
  admitOf : P → Option A
  revokeOf : P → Option (A × Nat)

variable {A I P G : Type} [DecidableEq A]

/-- The bytes a seal signs, less `claimed_ms`: `seal::message`. -/
def msgOf (s : Seal A I G) : A × Nat × I × I := (s.actor, s.seq, s.tip, s.prevTip)

/-- A device's records in file order: `Chains::by_actor`. -/
def chainOf (F : File A I P G) (a : A) : List (Rec A I P) := F.recs.filter (fun r => decide (r.actor = a))

/-- Consecutive records name each other and their seqs climb by one (count.rs `prev_ok`, `dense`). -/
def Linked : List (Rec A I P) → Prop
  | [] => True
  | [_] => True
  | x :: y :: l => y.prev = x.id ∧ y.seq = x.seq + 1 ∧ Linked (y :: l)

/-- The device's newest row: `Frontiers::of_actor`. -/
def rowOf (rows : List (Row A I)) (a : A) : Option (Row A I) := rows.find? (fun f => decide (f.actor = a))

/-- A device's oldest kept record: at seq 1 with no row, else it continues the row (`first_ok`). -/
def Starts (rows : List (Row A I)) (a : A) (r : Rec A I P) : Prop :=
  match rowOf rows a with
  | none => r.seq = 1
  | some f => r.seq = f.seq + 1 ∧ r.prev = f.tip

/-- One device's chain. -/
def DeviceOk (rows : List (Row A I)) (a : A) (c : List (Rec A I P)) : Prop :=
  Linked c ∧ ∀ x, c.head? = some x → Starts rows a x

/-- Reachable by `prev` from the id `t`, over ids and not over positions (verify.rs `unsealed`). -/
inductive Reach (L : List (Rec A I P)) (t : I) : Rec A I P → Prop
  | tip {r} : r ∈ L → r.id = t → Reach L t r
  | step {q r} : Reach L t q → r ∈ L → r.id = q.prev → Reach L t r

/-- `s` may be read at `(a, n)`: every REVOKE in the file names a later or equal seq. -/
def Valid (W : World A I P G) (F : File A I P G) (a : A) (n : Nat) : Prop :=
  ∀ r ∈ F.recs, ∀ after, W.revokeOf r.body = some (a, after) → n ≤ after

/-- A seal's facts, admission apart: it names a record at (actor, seq), the key is valid there, and
  the signature verifies. -/
def SealFacts (W : World A I P G) (F : File A I P G) (s : Seal A I G) : Prop :=
  (∃ t ∈ F.recs, t.actor = s.actor ∧ t.seq = s.seq ∧ t.id = s.tip) ∧
    Valid W F s.actor s.seq ∧ W.ver s.actor (msgOf s) s.sig = true

/-- Admission (settle.rs `admit_more`): a root, or the body of a record whose writer is admitted and
  has a good seal at or above that record. Validity at the record's seq follows from the seal's. -/
inductive Admitted (W : World A I P G) (F : File A I P G) : A → Prop
  | root {a} : W.trusted a → Admitted W F a
  | byAdmit {a} (r : Rec A I P) (s : Seal A I G) : r ∈ F.recs → s ∈ F.seals →
      W.admitOf r.body = some a → s.actor = r.actor → r.seq ≤ s.seq → SealFacts W F s →
      Admitted W F s.actor → Admitted W F a

/-- A walk may stop here: genesis, a frontier tip of ANY segment, or it goes on at a record. -/
def EndsOk (W : World A I P G) (F : File A I P G) (p : I) : Prop :=
  p = W.zero ∨ (∃ f ∈ F.rows, f.tip = p) ∨ ∃ r ∈ F.recs, r.id = p

/-- THE PREDICATE. Each field names the counter it stands for. -/
structure Holds (W : World A I P G) (F : File A I P G) : Prop where
  /-- chained: every id is the formula's. -/
  ids : ∀ r ∈ F.recs, r.id = W.idOf r.actor r.seq r.prev r.body
  /-- `unsealed`'s first term: records minus distinct ids. -/
  nodup : F.recs.Nodup
  idsOnce : ∀ x ∈ F.recs, ∀ y ∈ F.recs, x.id = y.id → x = y
  /-- forks = 0. -/
  noFork : ∀ x ∈ F.recs, ∀ y ∈ F.recs, x.actor = y.actor → x.seq = y.seq → x = y
  /-- broken = 0: link and dense seq. -/
  device : ∀ a, DeviceOk F.rows a (chainOf F a)
  /-- seals_bad = 0. -/
  sealsOk : ∀ s ∈ F.seals, SealFacts W F s ∧ Admitted W F s.actor
  /-- seals_ok > 0. -/
  sealed : F.seals ≠ []
  /-- `unsealed`: each record reached from an ok tip. -/
  vouched : ∀ r ∈ F.recs, ∃ s ∈ F.seals, Reach F.recs s.tip r
  /-- `unsealed`: no walk leaves the file elsewhere. -/
  ends : ∀ r ∈ F.recs, EndsOk W F r.prev
  /-- `unsealed`'s third term, `unowned`. -/
  owned : ∀ r ∈ F.recs, ∃ s ∈ F.seals, s.actor = r.actor ∧ r.seq ≤ s.seq

/-! ### The own chain -/

theorem Linked.tail {x : Rec A I P} {l : List (Rec A I P)} (h : Linked (x :: l)) : Linked l := by
  cases l with
  | nil => trivial
  | cons y l => exact h.2.2

theorem Linked.seq_lt {x : Rec A I P} {l : List (Rec A I P)} (h : Linked (x :: l)) :
    ∀ y ∈ l, x.seq < y.seq := by
  induction l generalizing x with
  | nil => intro y hy; cases hy
  | cons m l ih =>
    intro y hy
    have hs : m.seq = x.seq + 1 := h.2.1
    cases hy with
    | head => omega
    | tail _ hy' => have := ih h.2.2 y hy'; omega

theorem Reach.mono {L M : List (Rec A I P)} {t : I} {r : Rec A I P} (hLM : ∀ x ∈ L, x ∈ M)
    (h : Reach L t r) : Reach M t r := by
  induction h with
  | tip hm hi => exact .tip (hLM _ hm) hi
  | step _ hm hi ih => exact .step ih (hLM _ hm) hi

theorem Reach.trans {L : List (Rec A I P)} {t : I} {q e : Rec A I P} (h₁ : Reach L t q)
    (h₂ : Reach L q.id e) : Reach L t e := by
  induction h₂ with
  | tip hm hi => have := h₁; cases this with
    | tip hq hqi => exact .tip hm (hi.trans hqi)
    | step hp hq hqi =>
      exact .step hp hm (hi.trans hqi)
  | step _ hm hi ih => exact .step ih hm hi

/-- The head of a linked chain is below every member. -/
theorem head_below {h : Rec A I P} {l : List (Rec A I P)} (hl : Linked (h :: l)) :
    ∀ t ∈ h :: l, Reach (h :: l) t.id h := by
  induction l generalizing h with
  | nil =>
    intro t ht
    have : t = h := by simpa using ht
    subst this; exact .tip (List.mem_singleton.mpr rfl) rfl
  | cons m l ih =>
    intro t ht
    cases ht with
    | head => exact .tip (List.mem_cons_self) rfl
    | tail _ ht' =>
      have hsub : ∀ x ∈ m :: l, x ∈ h :: m :: l := fun x hx => List.mem_cons_of_mem _ hx
      have r := Reach.mono hsub (ih hl.2.2 t ht')
      exact .step r List.mem_cons_self hl.1.symm

/-- A tip reaches every member of its chain at or below its seq. -/
theorem reach_own_chain {c : List (Rec A I P)} (hc : Linked c) :
    ∀ t ∈ c, ∀ x ∈ c, x.seq ≤ t.seq → Reach c t.id x := by
  induction c with
  | nil => intro t ht; cases ht
  | cons h l ih =>
    intro t ht x hx hle
    cases hx with
    | head => exact head_below hc t ht
    | tail _ hx' =>
      cases ht with
      | head => have := hc.seq_lt x hx'; omega
      | tail _ ht' =>
        exact Reach.mono (fun y hy => List.mem_cons_of_mem _ hy) (ih hc.tail t ht' x hx' hle)

theorem mem_chainOf {F : File A I P G} {a : A} {r : Rec A I P} :
    r ∈ chainOf F a ↔ r ∈ F.recs ∧ r.actor = a := by
  simp [chainOf, List.mem_filter]

/-! ### What `Holds` gives -/

variable {W : World A I P G} {F : File A I P G}

/-- THE STRUCTURAL HALF. Every record: an ok seal of its own device at or above it reaches it, and the
  key is admitted and valid at the record's own seq. -/
theorem signed_for (H : Holds W F) {r : Rec A I P} (hr : r ∈ F.recs) :
    ∃ s ∈ F.seals, s.actor = r.actor ∧ r.seq ≤ s.seq ∧ Admitted W F r.actor ∧
      Valid W F r.actor r.seq ∧ W.ver r.actor (msgOf s) s.sig = true ∧ Reach F.recs s.tip r := by
  obtain ⟨s, hs, ha, hle⟩ := H.owned r hr
  obtain ⟨⟨⟨t, ht, hta, hts, hti⟩, hv, hsig⟩, had⟩ := H.sealsOk s hs
  refine ⟨s, hs, ha, hle, ha ▸ had, ?_, ha ▸ hsig, ?_⟩
  · intro q hq after hrev
    have := hv q hq after (by rw [ha]; exact hrev)
    omega
  · have hc := (H.device r.actor).1
    have htm : t ∈ chainOf F r.actor := mem_chainOf.mpr ⟨ht, hta.trans ha⟩
    have hrm : r ∈ chainOf F r.actor := mem_chainOf.mpr ⟨hr, rfl⟩
    have hre := reach_own_chain hc t htm r hrm (by omega)
    rw [← hti]
    exact Reach.mono (fun x hx => (mem_chainOf.mp hx).1) hre

/-- Unforgeability, as a hypothesis, and only for an `honest` key. `Signed a m`: the holder of `a`'s
  key really signed `m`. It is abstract: nothing in the file can observe it. -/
def Unforgeable (W : World A I P G) (Signed : A → (A × Nat × I × I) → Prop) (honest : A → Prop) : Prop :=
  ∀ a m g, honest a → W.ver a m g = true → Signed a m

theorem signed_for_honest {Signed : A → (A × Nat × I × I) → Prop} {honest : A → Prop}
    (hU : Unforgeable W Signed honest) (H : Holds W F) {r : Rec A I P} (hr : r ∈ F.recs)
    (hh : honest r.actor) :
    ∃ s ∈ F.seals, s.actor = r.actor ∧ Admitted W F r.actor ∧ Valid W F r.actor r.seq ∧
      Signed r.actor (msgOf s) ∧ Reach F.recs s.tip r := by
  obtain ⟨s, hs, ha, _, had, hv, hsig, hre⟩ := signed_for H hr
  exact ⟨s, hs, ha, had, hv, hU _ _ _ hh hsig, hre⟩

/-- Collision resistance on a finite list: the same id is the same record. -/
def NoCollision (L : List (Rec A I P)) : Prop := ∀ x ∈ L, ∀ y ∈ L, x.id = y.id → x = y

/-- A history a signer held: whole down to genesis (what was later cut is still in it). -/
def Whole (W : World A I P G) (V : List (Rec A I P)) : Prop :=
  ∀ v ∈ V, v.prev = W.zero ∨ ∃ u ∈ V, u.id = v.prev

/-- With no collision between the file and the signer's history, what the tip reaches in the file IS
  in that history: `prev` hashes the history into the tip. -/
theorem signer_saw {V : List (Rec A I P)} {t : I} (hW : Whole W V) (hc : NoCollision (F.recs ++ V))
    (hz : ∀ r ∈ F.recs, r.id ≠ W.zero) (ht : ∃ v ∈ V, v.id = t) {r : Rec A I P}
    (h : Reach F.recs t r) : r ∈ V := by
  induction h with
  | @tip r hm hi =>
    obtain ⟨v, hv, hvi⟩ := ht
    have := hc r (List.mem_append_left _ hm) v (List.mem_append_right _ hv) (hi.trans hvi.symm)
    exact this ▸ hv
  | @step q r _ hm hi ih =>
    rcases hW q ih with h0 | ⟨u, hu, hui⟩
    · exact absurd (hi.trans h0) (hz r hm)
    · have := hc r (List.mem_append_left _ hm) u (List.mem_append_right _ hu) (hi.trans hui.symm)
      exact this ▸ hu

/-- THE THEOREM THE FILE PROMISES. Given: the file holds; signatures of honest keys cannot be forged;
  an honest signer signs only tips of a history it holds whole; no collision against that history.
  Then every record of an honest device is IN a history its own admitted key signed, the key valid at
  that record's seq. A theorem about `Holds`, not about `verify`. -/
theorem holds_gives {Signed : A → (A × Nat × I × I) → Prop} {honest : A → Prop}
    (hU : Unforgeable W Signed honest)
    (hOwn : ∀ a m, honest a → Signed a m → ∃ V, Whole W V ∧ ∃ v ∈ V, v.id = m.2.2.1)
    (hc : ∀ V, Whole W V → NoCollision (F.recs ++ V))
    (hz : ∀ r ∈ F.recs, r.id ≠ W.zero)
    (H : Holds W F) {r : Rec A I P} (hr : r ∈ F.recs) (hh : honest r.actor) :
    ∃ s ∈ F.seals, ∃ V, s.actor = r.actor ∧ Admitted W F r.actor ∧ Valid W F r.actor r.seq ∧
      Signed r.actor (msgOf s) ∧ Whole W V ∧ r ∈ V := by
  obtain ⟨s, hs, ha, had, hv, hsg, hre⟩ := signed_for_honest hU H hr hh
  obtain ⟨V, hV, ht⟩ := hOwn _ _ hh hsg
  exact ⟨s, hs, V, ha, had, hv, hsg, hV, signer_saw hV (hc V hV) hz ht hre⟩

/-- Admission rests on signatures: an admitted key is a root, or an ADMIT of it lies under a tip an
  admitted, honest writer signed. Induction needs every admitted key honest: one stolen key admits
  anybody, which is the limit. -/
theorem admission_signed {Signed : A → (A × Nat × I × I) → Prop} {honest : A → Prop}
    (hU : Unforgeable W Signed honest) (hall : ∀ a, Admitted W F a → honest a) {a : A}
    (h : Admitted W F a) :
    W.trusted a ∨ ∃ r ∈ F.recs, ∃ s ∈ F.seals, W.admitOf r.body = some a ∧ s.actor = r.actor ∧
      r.seq ≤ s.seq ∧ Signed s.actor (msgOf s) ∧ Admitted W F s.actor := by
  cases h with
  | root ht => exact .inl ht
  | byAdmit r s hr hs hb ha hle hf had =>
    exact .inr ⟨r, hr, s, hs, hb, ha, hle, hU _ _ _ (hall _ had) hf.2.2, had⟩

/-- Every walk ends at genesis or at a frontier tip, GIVEN a writing order: `rank` drops along `prev`.
  `Holds` alone does not give this: DocLimits.ring_holds is a ring that passes every field. -/
theorem chain_ends (H : Holds W F) (rank : I → Nat)
    (hrank : ∀ x ∈ F.recs, ∀ q ∈ F.recs, q.id = x.prev → rank q.id < rank x.id) :
    ∀ r ∈ F.recs, ∃ e, Reach F.recs r.id e ∧ (e.prev = W.zero ∨ ∃ f ∈ F.rows, f.tip = e.prev) := by
  have key : ∀ n, ∀ r ∈ F.recs, rank r.id = n →
      ∃ e, Reach F.recs r.id e ∧ (e.prev = W.zero ∨ ∃ f ∈ F.rows, f.tip = e.prev) := by
    intro n
    induction n using Nat.strongRecOn with
    | ind n ih =>
      intro r hr hn
      rcases H.ends r hr with h0 | hf | ⟨q, hq, hqi⟩
      · exact ⟨r, .tip hr rfl, .inl h0⟩
      · exact ⟨r, .tip hr rfl, .inr hf⟩
      · have hlt := hrank r hr q hq hqi
        obtain ⟨e, hre, he⟩ := ih (rank q.id) (hn ▸ hlt) q hq rfl
        exact ⟨e, Reach.trans (.step (.tip hr rfl) hq hqi) hre, he⟩
  exact fun r hr => key _ r hr rfl

end Bebop.DocLog

#print axioms Bebop.DocLog.signed_for
#print axioms Bebop.DocLog.holds_gives
#print axioms Bebop.DocLog.admission_signed
#print axioms Bebop.DocLog.chain_ends
