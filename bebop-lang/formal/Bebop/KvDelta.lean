/-
  Bebop.KvDelta -- W-DELTALEAN (R-LANG 2026-10-09 §2 item 3): a KV DELTA CHAIN REPLAYED EQUALS ITS
  COMPACTED IMAGE, for EVERY chain, not only the goldens the four readers agree on.

  THE CODE IT MODELS: crates/bebop-store/src/kv/delta.rs (+ delta/write.rs). A v3 KV root is a v2 base
  plus a chain of delta records; each record is ONE op (put key value | remove key); `append_delta`
  writes one batch of ops as records; a READ is the base, then the records replayed OLDEST FIRST (a put
  inserts or overwrites, a remove deletes, a remove of an absent key changes nothing: `Kv::replay` over
  `Kv::put`/`Kv::remove`); COMPACTION (`Kv::compacted_bytes_fit` of what was read) writes the folded
  entries as a new v2 base with no chain.

  THE MODEL (an ABSTRACTION -- read "what it does NOT cover" before citing it):
    * an image is the map a reader answers with: `K -> Option V` (`Kv::get` on every key);
    * a delta is a batch, a list of ops; a chain is a list of deltas, OLDEST FIRST;
    * a v2 base is the chain of one batch of puts -- exactly the shape `compact` produces, so "base +
      chain" is just a longer chain (`replay` starts from the empty map);
    * replay = the left fold of `step` over the batches and, inside each, over the ops;
    * compact = the folded image re-emitted as ONE batch: one put per live key, each key once, in the
      order of each key's LAST mention in the chain (`dedup` keeps the last occurrence). Rust writes
      the base SORTED instead; the image (the map) is the same, which is all the model compares.

  PROVED (core Lean 4 only, no Mathlib; `#print axioms` lines at the bottom, MEASURED 2026-10-09 on
  Lean 4.33.1: every one reads [propext, Quot.sound] -- kernel-only, not even Classical.choice):
    replay_compact        replay (compact c) = replay c                          (THE theorem)
    compact_idem          compact (compact c) = compact c                        (list equality)
    compact_append_comm   replay (compact (c ++ [d])) = replay (compact c ++ [d])
    compact_prefix        replay (compact c1 ++ c2) = replay (c1 ++ c2)          (compact ANY prefix,
                          then keep appending: what the writer does after `Appended::Compact`)
    replay_flatten        replay c = applyDelta empty c.flatten                  (batch boundaries do
                          not matter: the records are the unit, as on disk)
    newest_put_wins, remove_sticks, remove_absent_noop                           (the three RED targets
                          in delta/tests.rs, as statements)

  WHAT IT DOES NOT COVER (the differential test crates/bebop-store/src/kv/delta/lean_tests.rs runs the
  REAL writer + compactor over named chains and asserts these same equalities on the bytes):
    * byte layout: packing eight bytes to a cell, KIDX/KBLOB/VIDX/VBLOB, the sorted base, the v3 root's
      eight cells, the record header {NEXT, OP, KLEN, VLEN};
    * the crc of the root and every record (W-CRC), and every REFUSAL (`chain_in` returning None: a D that
      lies, a looping chain, a wrong record length) -- the model has no corrupt images;
    * `superseded_cells`, the compaction TRIGGER (`MAX_DELTAS`, `DEAD_DIV`, v1, arena full): the model
      proves compaction is SOUND whenever it happens, not when it happens;
    * key ENCODING: `Kv::replay` decodes a record's key with `from_utf8_lossy`; the model's keys are
      abstract, so two byte strings that decode to one String are one key in Rust and two here (the writer
      only ever writes `&str` keys, so the writer never produces such a pair);
    * the order of `Kv::entries` (sorted): the model compares maps, the Rust test compares sorted entries.
-/

namespace Bebop.KvDelta

/-- One change: what one delta record holds (OP 1 put, OP 2 remove). -/
inductive Op (K V : Type) where
  | put (k : K) (v : V)
  | del (k : K)

/-- One `append_delta` call: a batch of ops, applied in order. -/
abbrev Delta (K V : Type) := List (Op K V)
/-- A chain, OLDEST FIRST (the reverse of `chain_in`'s newest-first walk; `Kv::replay` reverses it). -/
abbrev Chain (K V : Type) := List (Delta K V)
/-- What a reader answers for each key. -/
abbrev Image (K V : Type) := K → Option V

variable {K V : Type} [DecidableEq K]

def Op.key : Op K V → K
  | .put k _ => k
  | .del k => k

def empty : Image K V := fun _ => none

/-- `Kv::put` / `Kv::remove` on the map a reader sees. -/
def step (m : Image K V) : Op K V → Image K V
  | .put k v => fun j => if j = k then some v else m j
  | .del k => fun j => if j = k then none else m j

def applyDelta (m : Image K V) (d : Delta K V) : Image K V := d.foldl step m
def replayFrom (m : Image K V) (c : Chain K V) : Image K V := c.foldl applyDelta m
/-- READ: the chain replayed oldest first from the empty map (the base is the chain's first batch). -/
def replay (c : Chain K V) : Image K V := replayFrom empty c

def deltaKeys : Delta K V → List K
  | [] => []
  | o :: d => o.key :: deltaKeys d

/-- Every key the chain mentions, with repeats. -/
def keys : Chain K V → List K
  | [] => []
  | d :: c => deltaKeys d ++ keys c

def dedup : List K → List K
  | [] => []
  | k :: ks => if k ∈ ks then dedup ks else k :: dedup ks

/-- The keys of `l` the image holds. -/
def live (m : Image K V) : List K → List K
  | [] => []
  | k :: ks => match m k with
    | some _ => k :: live m ks
    | none => live m ks

/-- One put per key of `l` the image holds, with the image's value. -/
def emit (m : Image K V) : List K → Delta K V
  | [] => []
  | k :: ks => match m k with
    | some v => .put k v :: emit m ks
    | none => emit m ks

/-- COMPACTION: the folded image re-emitted as ONE batch of puts, each live key once. -/
def compact (c : Chain K V) : Chain K V :=
  [emit (replay c) (live (replay c) (dedup (keys c)))]

def NoDup : List K → Prop
  | [] => True
  | k :: ks => k ∉ ks ∧ NoDup ks

/-! ### Folding -/

theorem applyDelta_cons (m : Image K V) (o : Op K V) (d : Delta K V) :
    applyDelta m (o :: d) = applyDelta (step m o) d := rfl

theorem replayFrom_cons (m : Image K V) (d : Delta K V) (c : Chain K V) :
    replayFrom m (d :: c) = replayFrom (applyDelta m d) c := rfl

theorem applyDelta_append (m : Image K V) (d1 d2 : Delta K V) :
    applyDelta m (d1 ++ d2) = applyDelta (applyDelta m d1) d2 := by
  simp [applyDelta, List.foldl_append]

theorem replayFrom_append (m : Image K V) (c1 c2 : Chain K V) :
    replayFrom m (c1 ++ c2) = replayFrom (replayFrom m c1) c2 := by
  simp [replayFrom, List.foldl_append]

theorem replay_append (c1 c2 : Chain K V) :
    replay (c1 ++ c2) = replayFrom (replay c1) c2 := replayFrom_append _ _ _

theorem replayFrom_eq_flatten (m : Image K V) (c : Chain K V) :
    replayFrom m c = applyDelta m c.flatten := by
  induction c generalizing m with
  | nil => rfl
  | cons d c ih => rw [replayFrom_cons, ih, List.flatten_cons, applyDelta_append]

/-- Batch boundaries do not matter: the records are the unit. -/
theorem replay_flatten (c : Chain K V) : replay c = applyDelta empty c.flatten :=
  replayFrom_eq_flatten _ _

/-! ### Frame: a key no op mentions keeps its value -/

theorem step_ne (m : Image K V) (o : Op K V) (j : K) (h : j ≠ o.key) : step m o j = m j := by
  cases o with
  | put k v => simp [step, Op.key] at h ⊢; simp [h]
  | del k => simp [step, Op.key] at h ⊢; simp [h]

theorem applyDelta_notin (m : Image K V) (d : Delta K V) (j : K) (h : j ∉ deltaKeys d) :
    applyDelta m d j = m j := by
  induction d generalizing m with
  | nil => rfl
  | cons o d ih =>
    simp only [deltaKeys, List.mem_cons, not_or] at h
    rw [applyDelta_cons, ih _ h.2, step_ne _ _ _ h.1]

theorem replayFrom_notin (m : Image K V) (c : Chain K V) (j : K) (h : j ∉ keys c) :
    replayFrom m c j = m j := by
  induction c generalizing m with
  | nil => rfl
  | cons d c ih =>
    simp only [keys, List.mem_append, not_or] at h
    rw [replayFrom_cons, ih _ h.2, applyDelta_notin _ _ _ h.1]

/-- Support: the image holds only keys the chain mentions. -/
theorem replay_support (c : Chain K V) (j : K) (h : replay c j ≠ none) : j ∈ keys c := by
  by_cases hn : j ∈ keys c
  · exact hn
  · exact absurd (by rw [replay, replayFrom_notin _ _ _ hn]; rfl) h

/-! ### The lists compaction builds -/

theorem mem_dedup (l : List K) (j : K) : j ∈ dedup l ↔ j ∈ l := by
  induction l with
  | nil => simp [dedup]
  | cons k ks ih =>
    by_cases hk : k ∈ ks
    · simp only [dedup, hk, if_true, ih, List.mem_cons]
      constructor
      · exact Or.inr
      · rintro (rfl | h) <;> assumption
    · simp only [dedup, hk, if_false, List.mem_cons, ih]

omit [DecidableEq K] in
theorem mem_live (m : Image K V) (l : List K) (j : K) : j ∈ live m l ↔ j ∈ l ∧ m j ≠ none := by
  induction l with
  | nil => simp [live]
  | cons k ks ih =>
    cases hk : m k with
    | some v =>
      simp only [live, hk, List.mem_cons, ih]
      constructor
      · rintro (rfl | ⟨h1, h2⟩)
        · exact ⟨Or.inl rfl, by simp [hk]⟩
        · exact ⟨Or.inr h1, h2⟩
      · rintro ⟨rfl | h1, h2⟩
        · exact Or.inl rfl
        · exact Or.inr ⟨h1, h2⟩
    | none =>
      simp only [live, hk, List.mem_cons, ih]
      constructor
      · rintro ⟨h1, h2⟩; exact ⟨Or.inr h1, h2⟩
      · rintro ⟨rfl | h1, h2⟩
        · exact absurd hk h2
        · exact ⟨h1, h2⟩

theorem dedup_nodup (l : List K) : NoDup (dedup l) := by
  induction l with
  | nil => trivial
  | cons k ks ih =>
    by_cases hk : k ∈ ks
    · simp only [dedup, hk, if_true]; exact ih
    · simp only [dedup, hk, if_false]; exact ⟨by rw [mem_dedup]; exact hk, ih⟩

omit [DecidableEq K] in
theorem live_nodup (m : Image K V) (l : List K) (h : NoDup l) : NoDup (live m l) := by
  induction l with
  | nil => trivial
  | cons k ks ih =>
    cases hk : m k with
    | some v =>
      simp only [live, hk]
      exact ⟨fun hm => h.1 ((mem_live m ks k).1 hm).1, ih h.2⟩
    | none => simp only [live, hk]; exact ih h.2

theorem dedup_of_nodup (l : List K) (h : NoDup l) : dedup l = l := by
  induction l with
  | nil => rfl
  | cons k ks ih => simp only [dedup, h.1, if_false, ih h.2]

omit [DecidableEq K] in
theorem live_live (m : Image K V) (l : List K) : live m (live m l) = live m l := by
  induction l with
  | nil => rfl
  | cons k ks ih =>
    cases hk : m k with
    | some v => simp only [live, hk, ih]
    | none => simp only [live, hk, ih]

omit [DecidableEq K] in
theorem deltaKeys_emit (m : Image K V) (l : List K) : deltaKeys (emit m l) = live m l := by
  induction l with
  | nil => rfl
  | cons k ks ih =>
    cases hk : m k with
    | some v => simp only [emit, live, hk, deltaKeys, Op.key, ih]
    | none => simp only [emit, live, hk, ih]

/-- Replaying the emitted batch onto any start: the image on the listed live keys, the start elsewhere. -/
theorem applyDelta_emit (m0 m : Image K V) (l : List K) (j : K) :
    applyDelta m0 (emit m l) j = if j ∈ l ∧ m j ≠ none then m j else m0 j := by
  induction l generalizing m0 with
  | nil => simp [emit, applyDelta]
  | cons k ks ih =>
    cases hk : m k with
    | some v =>
      simp only [emit, hk]
      rw [applyDelta_cons, ih]
      by_cases hj : j = k
      · subst hj; simp [step, hk]
      · simp [step, hj, List.mem_cons]
    | none =>
      simp only [emit, hk]
      rw [ih]
      by_cases hj : j = k
      · subst hj; simp [hk]
      · simp [hj, List.mem_cons]

/-! ### THE THEOREMS -/

/-- REPLAY = COMPACTION: the compacted image reads exactly as the chain it was compacted from. -/
theorem replay_compact (c : Chain K V) : replay (compact c) = replay c := by
  funext j
  show applyDelta empty (emit (replay c) (live (replay c) (dedup (keys c)))) j = replay c j
  rw [applyDelta_emit]
  by_cases hn : replay c j = none
  · simp [hn, empty]
  · have hm : j ∈ live (replay c) (dedup (keys c)) :=
      (mem_live _ _ _).2 ⟨(mem_dedup _ _).2 (replay_support c j hn), hn⟩
    simp [hm, hn]

/-- COMPACTION IS IDEMPOTENT, as lists: compacting a compacted chain writes the same batch. -/
theorem compact_idem (c : Chain K V) : compact (compact c) = compact c := by
  have hk : keys (compact c) = live (replay c) (dedup (keys c)) := by
    simp [compact, keys, deltaKeys_emit, live_live]
  have hnd : NoDup (live (replay c) (dedup (keys c))) := live_nodup _ _ (dedup_nodup _)
  show [emit (replay (compact c)) (live (replay (compact c)) (dedup (keys (compact c))))] = compact c
  rw [replay_compact, hk, dedup_of_nodup _ hnd, live_live]
  rfl

/-- Compact ANY prefix, then keep appending: the chain reads as if nothing had been compacted. -/
theorem compact_prefix (c1 c2 : Chain K V) : replay (compact c1 ++ c2) = replay (c1 ++ c2) := by
  rw [replay_append, replay_compact, ← replay_append]

/-- APPENDING A DELTA COMMUTES WITH COMPACTION. -/
theorem compact_append_comm (c : Chain K V) (d : Delta K V) :
    replay (compact (c ++ [d])) = replay (compact c ++ [d]) := by
  rw [replay_compact, compact_prefix]

/-! ### The RED targets of delta/tests.rs, as statements -/

/-- "replay skips the newest delta": the newest put to a key is what the key reads. -/
theorem newest_put_wins (c : Chain K V) (k : K) (v : V) : replay (c ++ [[.put k v]]) k = some v := by
  rw [replay_append]; simp [replayFrom, applyDelta, step]

/-- "compaction drops a remove": a removed key stays removed, through compaction too. -/
theorem remove_sticks (c : Chain K V) (k : K) :
    replay (c ++ [[.del k]]) k = none ∧ replay (compact (c ++ [[.del k]])) k = none := by
  have h : replay (c ++ [[.del k]]) k = none := by
    rw [replay_append]; simp [replayFrom, applyDelta, step]
  exact ⟨h, by rw [replay_compact]; exact h⟩

/-- A remove of an absent key changes nothing. -/
theorem remove_absent_noop (m : Image K V) (k : K) (h : m k = none) : step m (.del k) = m := by
  funext j
  by_cases hj : j = k
  · subst hj; simp [step, h]
  · simp [step, hj]

/-! ### Evaluated examples (the named chains the Rust differential test also runs) -/

private def ex : Chain String Nat :=
  [[.put "a" 1, .put "b" 2], [.put "a" 3], [.del "b"], [.del "zz"], [.put "c" 4, .put "c" 5]]

#guard replay ex "a" == some 3
#guard replay ex "b" == none
#guard replay ex "c" == some 5
#guard (compact ex).length == 1
#guard ((compact ex).flatten.map Op.key) == ["a", "c"]
#guard replay (compact ex) "a" == some 3 && replay (compact ex) "c" == some 5

end Bebop.KvDelta

#print axioms Bebop.KvDelta.replay_compact
#print axioms Bebop.KvDelta.compact_idem
#print axioms Bebop.KvDelta.compact_prefix
#print axioms Bebop.KvDelta.compact_append_comm
#print axioms Bebop.KvDelta.replay_flatten
#print axioms Bebop.KvDelta.newest_put_wins
#print axioms Bebop.KvDelta.remove_sticks
#print axioms Bebop.KvDelta.remove_absent_noop
