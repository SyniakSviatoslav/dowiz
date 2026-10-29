/-
  Bebop.KvBytes -- W-ES-P5 (R-EMBED-STORE-2026-10-27 §4.5 AC-ES-40, Lean L1): KV v3 DELTA RECORDS,
  AS CELLS, DECODE AND REPLAY TO THE ABSTRACT KV OF Bebop.KvDelta. Core Lean 4, no Mathlib, no `axiom`,
  no `sorry`.

  THE CODE IT TRANSCRIBES (by hand -- no Aeneas on the box):
    * `packed` crates/bebop-store/src/kv/delta.rs:108-111: byte b of a field = (cell[at + b/8] >> 8(b%8))
      as u8. A cell is modelled by its unsigned 64-bit pattern (`Nat`); the shift is at most 56 and the
      cast keeps the low 8 bits, so arithmetic vs logical shift does not change the byte.
    * the writer packs eight bytes to a cell, little-endian, ceil(len/8) cells (FORMAT §9.1, `packCell`).
    * a record payload (FORMAT §9.1, delta.rs:22-24):
        {NEXT, OP (1 put, 2 remove), KLEN, VLEN, key ceil(KLEN/8) cells, value ceil(VLEN/8) cells}
      and `chain_in`'s per-record checks (delta.rs:139-154): length EXACTLY 4 + ceil(KLEN/8) +
      ceil(VLEN/8), OP in {1, 2}, a remove has VLEN 0. `decRec` refuses everything else.
    * the chain is NEWEST FIRST and holds exactly D records (delta.rs:155-161); `Kv::replay`
      (delta.rs:166-176) applies it reversed, oldest first, onto the base.

  PROVED:
    unpack_pack      packed (packBytes bs) bs.length = bs                    (bytes < 256)
    dec_enc          decRec (encRec next o) = some o                         (any NEXT, any op)
    readV3_encode    readV3 base D (newest-first records of `ops`) = some (applyDelta base ops)
    readV3_chain     for a chain of batches (append_delta calls): = some (KvDelta.replayFrom base c)
                     -- THE bridge: the bytes the writer lays down replay to KvDelta's model.

  NOT COVERED (open): the v2 BASE arrays (KIDX/KBLOB/VIDX/VBLOB, sorted keys) -- the base enters as an
  abstract `Image`; the arena walk itself (object headers, the NEXT ref as an object-relative offset,
  `follow`, the cell budget that ends a loop); every crc (root, record); Rust's `from_utf8_lossy` on
  keys (keys here are byte lists, so two byte strings that decode to one String stay two keys -- D6);
  i64 sign of KLEN/VLEN (`kl < 0` refusal) -- lengths are `Nat` here.
-/
import Bebop.KvDelta

namespace Bebop.KvBytes
open Bebop.KvDelta

abbrev Bytes := List Nat

/-! ## Eight bytes to a cell -/

/-- Byte k of a cell's 64-bit pattern: `(cell >> 8k) as u8`. -/
def byteOf (c k : Nat) : Nat := c / 256 ^ k % 256

/-- Little-endian: the first byte is the low byte. -/
def packCell : Bytes → Nat
  | [] => 0
  | b :: bs => b + 256 * packCell bs

/-- Eight bytes per cell, ceil(len/8) cells (`fuel` = the byte count bounds the cell count). -/
def packN : Nat → Bytes → List Nat
  | 0, _ => []
  | f + 1, bs => if bs = [] then [] else packCell (bs.take 8) :: packN f (bs.drop 8)

def packBytes (bs : Bytes) : List Nat := packN bs.length bs

/-- `packed`: `len` bytes out of the cells `cs`. -/
def packed (cs : List Nat) (len : Nat) : Bytes :=
  (List.range len).map fun b => byteOf (cs.getD (b / 8) 0) (b % 8)

def Small (bs : Bytes) : Prop := ∀ x ∈ bs, x < 256

theorem byteOf_packCell : ∀ (l : Bytes), Small l → ∀ k (hk : k < l.length),
    byteOf (packCell l) k = l[k]
  | [], _, k, hk => absurd hk (Nat.not_lt_zero k)
  | b :: bs, hs, k, hk => by
    have hb : b < 256 := hs b List.mem_cons_self
    have hbs : Small bs := fun x hx => hs x (List.mem_cons_of_mem _ hx)
    match k, hk with
    | 0, _ =>
      simp only [byteOf, packCell, Nat.pow_zero, Nat.div_one, Nat.add_mul_mod_self_left,
        List.getElem_cons_zero]
      exact Nat.mod_eq_of_lt hb
    | k + 1, hk =>
      have hk' : k < bs.length := by simp at hk; omega
      have e : (b + 256 * packCell bs) / 256 = packCell bs := by
        rw [Nat.add_mul_div_left _ _ (by decide : 0 < 256), Nat.div_eq_of_lt hb, Nat.zero_add]
      have := byteOf_packCell bs hbs k hk'
      simp only [byteOf] at this ⊢
      rw [Nat.pow_succ, Nat.mul_comm (256 ^ k) 256, ← Nat.div_div_eq_div_mul, packCell, e, this]
      simp

theorem length_packN : ∀ (f : Nat) (bs : Bytes), bs.length ≤ f →
    (packN f bs).length = (bs.length + 7) / 8
  | 0, bs, h => by
    have : bs.length = 0 := by omega
    simp [packN, this]
  | f + 1, bs, h => by
    by_cases he : bs = []
    · subst he; simp [packN]
    · have hl : 0 < bs.length := List.length_pos_iff.2 he
      simp only [packN, if_neg he, List.length_cons]
      rw [length_packN f (bs.drop 8) (by simp; omega)]
      simp only [List.length_drop]
      omega

theorem length_packBytes (bs : Bytes) : (packBytes bs).length = (bs.length + 7) / 8 :=
  length_packN _ _ (Nat.le_refl _)

/-- Cell q of the packing is the packing of bytes [8q, 8q+8). -/
theorem getD_packN : ∀ (q f : Nat) (bs : Bytes), bs.length ≤ f → 8 * q < bs.length →
    (packN f bs).getD q 0 = packCell ((bs.drop (8 * q)).take 8)
  | q, 0, bs, h1, h2 => by omega
  | 0, f + 1, bs, _, h2 => by
    have he : bs ≠ [] := List.ne_nil_of_length_pos (by omega)
    simp [packN, he]
  | q + 1, f + 1, bs, h1, h2 => by
    have he : bs ≠ [] := List.ne_nil_of_length_pos (by omega)
    simp only [packN, if_neg he, List.getD_cons_succ]
    rw [getD_packN q f (bs.drop 8) (by simp; omega) (by simp; omega), List.drop_drop]
    congr 3
    omega

theorem unpack_pack (bs : Bytes) (hs : Small bs) : packed (packBytes bs) bs.length = bs := by
  apply List.ext_getElem (by simp [packed])
  intro b h1 h2
  simp only [packed, List.getElem_map, List.getElem_range]
  have hq : 8 * (b / 8) < bs.length := by omega
  rw [packBytes, getD_packN (b / 8) _ bs (Nat.le_refl _) hq]
  have hsub : Small ((bs.drop (8 * (b / 8))).take 8) := fun x hx =>
    hs x (List.mem_of_mem_drop (List.mem_of_mem_take hx))
  have hk : b % 8 < ((bs.drop (8 * (b / 8))).take 8).length := by
    simp only [List.length_take, List.length_drop]; omega
  rw [byteOf_packCell _ hsub _ hk]
  simp only [List.getElem_take, List.getElem_drop]
  congr 1
  omega

/-! ## One record -/

def ceil8 (n : Nat) : Nat := (n + 7) / 8

/-- The writer's record payload (NEXT is whatever ref the arena gives it). -/
def encRec (next : Nat) : Op Bytes Bytes → List Nat
  | .put k v => [next, 1, k.length, v.length] ++ (packBytes k ++ packBytes v)
  | .del k => [next, 2, k.length, 0] ++ packBytes k

/-- `chain_in`'s checks on one record, then `Rec::key` / `Rec::value`. -/
def decRec : List Nat → Option (Op Bytes Bytes)
  | _ :: op :: kl :: vl :: rest =>
    if rest.length ≠ ceil8 kl + ceil8 vl then none
    else if op = 1 then some (.put (packed rest kl) (packed (rest.drop (ceil8 kl)) vl))
    else if op = 2 ∧ vl = 0 then some (.del (packed rest kl))
    else none
  | _ => none

/-- Reading the first `len` bytes never looks past the first ceil8 len cells. -/
theorem packed_append (l1 l2 : List Nat) (len : Nat) (h : ceil8 len ≤ l1.length) :
    packed (l1 ++ l2) len = packed l1 len := by
  simp only [packed]
  apply List.map_congr_left
  intro b hb
  have hb' : b < len := List.mem_range.1 hb
  have : b / 8 < l1.length := by unfold ceil8 at h; omega
  simp [List.getD_eq_getElem?_getD, List.getElem?_append_left this]

def OpSmall : Op Bytes Bytes → Prop
  | .put k v => Small k ∧ Small v
  | .del k => Small k

theorem dec_enc (next : Nat) (o : Op Bytes Bytes) (hs : OpSmall o) : decRec (encRec next o) = some o := by
  cases o with
  | put k v =>
    have hk := length_packBytes k
    have hv := length_packBytes v
    simp only [encRec, List.cons_append, List.nil_append, decRec]
    have hlen : (packBytes k ++ packBytes v).length = ceil8 k.length + ceil8 v.length := by
      simp [hk, hv, ceil8]
    rw [if_neg (by rw [hlen]; exact fun h => h rfl), if_pos (by simp),
      packed_append _ _ _ (by rw [hk]; exact Nat.le_refl _),
      List.drop_left' (by rw [hk]; rfl), unpack_pack k hs.1, unpack_pack v hs.2]
  | del k =>
    have hk := length_packBytes k
    simp only [encRec, List.cons_append, List.nil_append, decRec]
    have hlen : (packBytes k).length = ceil8 k.length + ceil8 0 := by simp [hk, ceil8]
    rw [if_neg (by rw [hlen]; exact fun h => h rfl), if_neg (by simp), if_pos (by simp),
      unpack_pack k hs]

/-! ## The chain -/

/-- Every record decodes, in order (`None` as soon as one does not). -/
def decAll : List (List Nat) → Option (List (Op Bytes Bytes))
  | [] => some []
  | p :: ps => match decRec p, decAll ps with
    | some o, some os => some (o :: os)
    | _, _ => none

/-- READ of a v3 root: exactly D records (newest first), each decoded, replayed oldest first onto
  the base. -/
def readV3 (base : Image Bytes Bytes) (D : Nat) (recs : List (List Nat)) : Option (Image Bytes Bytes) :=
  if recs.length ≠ D then none
  else match decAll recs with
    | none => none
    | some ops => some (applyDelta base ops.reverse)

/-- What `append_delta` lays down for a list of ops, oldest first: the records NEWEST first, D. -/
def encodeV3 (ops : List (Op Bytes Bytes)) : Nat × List (List Nat) :=
  (ops.length, (ops.map (encRec 0)).reverse)

theorem decAll_map (l : List (Op Bytes Bytes)) (hs : ∀ o ∈ l, OpSmall o) :
    decAll (l.map (encRec 0)) = some l := by
  induction l with
  | nil => rfl
  | cons o os ih =>
    simp only [List.map_cons, decAll,
      dec_enc 0 o (hs o List.mem_cons_self), ih (fun x hx => hs x (List.mem_cons_of_mem _ hx))]

theorem readV3_encode (base : Image Bytes Bytes) (ops : List (Op Bytes Bytes))
    (hs : ∀ o ∈ ops, OpSmall o) :
    readV3 base (encodeV3 ops).1 (encodeV3 ops).2 = some (applyDelta base ops) := by
  simp only [readV3, encodeV3, List.length_reverse, List.length_map, ne_eq, not_true_eq_false,
    if_false]
  rw [← List.map_reverse, decAll_map _ (fun o ho => hs o (List.mem_reverse.1 ho))]
  simp

theorem replayFrom_flatten (m : Image Bytes Bytes) (c : Chain Bytes Bytes) :
    replayFrom m c = applyDelta m c.flatten := by
  induction c generalizing m with
  | nil => rfl
  | cons d c ih =>
    simp only [replayFrom, List.foldl_cons, List.flatten_cons, applyDelta, List.foldl_append] at ih ⊢
    exact ih _

/-- **L1.** A chain of `append_delta` batches, laid down as records, reads back as KvDelta's replay. -/
theorem readV3_chain (base : Image Bytes Bytes) (c : Chain Bytes Bytes)
    (hs : ∀ o ∈ c.flatten, OpSmall o) :
    readV3 base (encodeV3 c.flatten).1 (encodeV3 c.flatten).2 = some (replayFrom base c) := by
  rw [readV3_encode base _ hs, replayFrom_flatten]

/-! ### Evaluated: a 9-byte key crosses a cell; a remove; the refusals -/

#guard packed (packBytes [1, 2, 3, 4, 5, 6, 7, 8, 9]) 9 == [1, 2, 3, 4, 5, 6, 7, 8, 9]
#guard (packBytes [1, 2, 3, 4, 5, 6, 7, 8, 9]).length == 2
#guard (decRec (encRec 0 (.del [7, 7]))).isSome
#guard (decRec [0, 3, 0, 0]).isNone          -- OP 3
#guard (decRec [0, 2, 0, 1, 0]).isNone       -- a remove with VLEN 1
#guard (decRec [0, 1, 9, 0, 0]).isNone       -- KLEN 9 needs 2 cells, has 1
#guard (readV3 (fun _ => none) 2 [encRec 0 (.del [1])]).isNone   -- D says 2, the walk gives 1

end Bebop.KvBytes

#print axioms Bebop.KvBytes.unpack_pack
#print axioms Bebop.KvBytes.dec_enc
#print axioms Bebop.KvBytes.readV3_encode
#print axioms Bebop.KvBytes.readV3_chain
