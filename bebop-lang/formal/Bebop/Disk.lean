/-
  Bebop.Disk -- W-ES-P5 (docs/research/R-EMBED-STORE-2026-10-27.md §5 row P5, AC-ES-41, Lean L2):
  THE RUST COMMIT IS ATOMIC UNDER A CRASH AT ANY POINT, and a committed generation is never lost.
  Core Lean 4, no Mathlib, no `axiom`, no `sorry`. Every failure-model assumption is a HYPOTHESIS of
  the theorem that needs it, never an axiom.

  THE CODE IT MODELS (format: docs/FORMAT-BEBOP-IMAGE-v2.md, frozen 9f733d46):
    * `Store::commit` crates/bebop-store/src/lib.rs:868-878 --
        write_cells(arena [mark, cursor)) ; sync_data ; write_cells(PartTab, 21 cells at (512-sb)+16) ;
        sync_data ; write_cells(other superblock, 16 cells at 512-sb) ; sync_data
      (`rustOps` below; FORMAT §8 says the same three steps). Every value written is a cell of the
      in-memory store after `stage_commit` (lib.rs:809-865): `Commit.S`.
    * the doc's proposed two-barrier order, R-EMBED-STORE §3.5 (arena + PartTab, ONE barrier, then the
      superblock, barrier): `docOps`. Proved too -- §3.5's claim "one barrier between 2 and 4 suffices".
    * recovery = FORMAT §3.1 `pick_checked_in`: valid superblocks (magic and crc15) -> the higher
      generation (tie -> A) -> checks on the chosen one WITHOUT fallback (arena_used inside the image,
      PartTab inside / length 19 / crc, root inside). Fallback to the other generation happens ONLY
      through magic/crc15 (§3.1 item 3): `sel` decides, `chk` refuses, nothing falls back. The anchor
      (`anc`) is not consulted by pick (FORMAT §7), so the model does not consult it either.

  THE DISK MODEL (R-EMBED-STORE §4.3, made cell-granular):
    * a file is cells, `Nat → Int` (an i64 per cell);
    * a write appends (cell, value) pairs to the PENDING list; `sync` makes every pending write durable;
    * CRASH (`Crash`): every cell independently keeps its durable value or holds ANY value some pending
      write gave it. This covers reordering inside a window, any subset of sectors persisting, and a
      torn sector (the doc's "first half new, second half old" is a per-cell choice, since a half-sector
      is 32 whole cells). A crash at "any point" = a crash after any prefix of the op list.

  FAILURE-MODEL HYPOTHESES (each one an explicit theorem argument):
    H_BARRIER  `sync` = fdatasync returned 0 and every earlier write is durable (the `.sync` semantics;
               an EIO/fsyncgate return is the error path, which `commit` returns as Err -- not modelled);
    H_CELL     tearing granularity is a cell (8 bytes) or coarser -- built into `Crash`;
    H_TORN     (`htorn`) the crc15 of the superblock REJECTS every cell-mix of the old and the new
               superblock that is neither of them. Real CRC-32 only makes this overwhelmingly likely
               (one undetected mix in 2^32 per mix); it is the one probabilistic assumption here;
    H_HOME     (`hpt`) the live PartTab is 0 or on the live superblock's own page (sb + 16) -- what every
               writer produces (FORMAT §4) but what NO reader checks (finding F1 below);
    H_BEGIN    (`hmark`, `hused`) `begin`'s checks, lib.rs:741-746: the cursor is inside the arena and not
               behind the live superblock's arena_used.

  PROVED:
    rust_commit_atomic     crash after ANY prefix of `rustOps` -> recover = old generation's or new one's
    doc_commit_atomic      the same for §3.5's two-barrier order
    rust_commit_durable    after the last sync returns, every crash recovers the new generation
    rust_never_lost        a crash anywhere in the NEXT commit recovers generation k+1 or k+2, never k
    rust_commit_not_refused  with the staged generation valid (H_STAGED), recovery never refuses

  NOT COVERED (open, said plainly): the crc ARITHMETIC (`crc` is a parameter, `stage_commit`'s crc
  computation is not modelled -- the new generation's validity is hypothesis H_STAGED); file growth and
  `create` (fsync of file + directory); the anchor check at reopen (P2 recovery); bebop's mmap path
  (`st_commit_sync`, 2 msyncs); the DO path (atomicity from Cloudflare); errors from sync_data.

  FINDING F1 (no Rust edit; reported): `pick_checked_in` accepts a live superblock whose PartTab cell
  names ANY in-image PartTab, not only its home (sb + 16). If an image (hand-made, or a future writer)
  had live A naming a PartTab at 528, the next commit from A writes its new PartTab over that same
  live PartTab, and a crash in that window leaves A valid, naming a torn PartTab: pick refuses
  (PartTabBad) instead of returning generation k -- atomicity needs H_HOME, and the reader does not
  enforce it. No writer in the tree produces such an image.
-/

namespace Bebop.Disk

/-- A file, as cells: cell index ↦ the i64 it holds. -/
abbrev Disk := Nat → Int

/-! ## The device -/

/-- Durable cells, and the writes not yet behind a barrier (oldest first). -/
structure Dev where
  dur : Disk
  pend : List (Nat × Int)

inductive Op where
  | write (ws : List (Nat × Int))
  | sync

def upd (d : Disk) (p : Nat × Int) : Disk := fun j => if j = p.1 then p.2 else d j

/-- The pending writes applied in order (what the page cache shows). -/
def flush (pend : List (Nat × Int)) (d : Disk) : Disk := pend.foldl upd d

def step (s : Dev) : Op → Dev
  | .write ws => { s with pend := s.pend ++ ws }
  | .sync => { dur := flush s.pend s.dur, pend := [] }

def run (ops : List Op) (s : Dev) : Dev := ops.foldl step s

/-- CRASH: each cell keeps its durable value or holds any value a pending write gave it. -/
def Crash (s : Dev) (d : Disk) : Prop := ∀ i, d i = s.dur i ∨ (i, d i) ∈ s.pend

/-- `write_cells(off, n)` (lib.rs:881-889): n cells from the in-memory image `S`. -/
def cellsOf (S : Disk) (off n : Nat) : List (Nat × Int) :=
  (List.range n).map fun j => (off + j, S (off + j))

theorem mem_cellsOf {S : Disk} {off n i : Nat} {v : Int} :
    (i, v) ∈ cellsOf S off n ↔ off ≤ i ∧ i < off + n ∧ v = S i := by
  simp only [cellsOf, List.mem_map, List.mem_range, Prod.mk.injEq]
  constructor
  · rintro ⟨j, hj, rfl, rfl⟩
    exact ⟨by omega, by omega, rfl⟩
  · rintro ⟨h1, h2, rfl⟩
    exact ⟨i - off, by omega, by omega, by rw [Nat.add_sub_of_le h1]⟩

theorem cellsOf_S {S : Disk} {off n : Nat} : ∀ p ∈ cellsOf S off n, p.2 = S p.1 := by
  intro p hp
  have := (mem_cellsOf (S := S) (off := off) (n := n) (i := p.1) (v := p.2)).1 hp
  exact this.2.2

theorem flush_miss : ∀ (pend : List (Nat × Int)) (d : Disk) (j : Nat),
    (∀ v, (j, v) ∉ pend) → flush pend d j = d j
  | [], _, _, _ => rfl
  | p :: ps, d, j, h => by
    show flush ps (upd d p) j = d j
    rw [flush_miss ps (upd d p) j (fun v hv => h v (List.mem_cons_of_mem _ hv))]
    have hne : j ≠ p.1 := by
      intro e
      apply h p.2
      rw [e]
      exact List.mem_cons_self
    simp [upd, hne]

theorem flush_hit (S : Disk) : ∀ (pend : List (Nat × Int)) (d : Disk) (j : Nat),
    (∀ p ∈ pend, p.2 = S p.1) → (∃ v, (j, v) ∈ pend) → flush pend d j = S j
  | [], _, _, _, ⟨_, hv⟩ => absurd hv List.not_mem_nil
  | p :: ps, d, j, hS, ⟨v, hv⟩ => by
    show flush ps (upd d p) j = S j
    by_cases hin : ∃ w, (j, w) ∈ ps
    · exact flush_hit S ps (upd d p) j (fun q hq => hS q (List.mem_cons_of_mem _ hq)) hin
    · rw [flush_miss ps (upd d p) j (fun w hw => hin ⟨w, hw⟩)]
      rcases List.mem_cons.1 hv with e | e
      · rw [← e]
        have := hS (j, v) (by rw [e]; exact List.mem_cons_self)
        show (if j = j then v else d j) = S j
        rw [if_pos rfl]
        exact this
      · exact absurd ⟨v, e⟩ hin

theorem flush_cells (S d : Disk) (off n i : Nat) :
    flush (cellsOf S off n) d i = if off ≤ i ∧ i < off + n then S i else d i := by
  by_cases h : off ≤ i ∧ i < off + n
  · rw [if_pos h]
    exact flush_hit S _ d i cellsOf_S ⟨S i, mem_cellsOf.2 ⟨h.1, h.2, rfl⟩⟩
  · rw [if_neg h]
    apply flush_miss
    intro v hv
    have := mem_cellsOf.1 hv
    exact h ⟨this.1, this.2.1⟩

theorem flush_append (a b : List (Nat × Int)) (d : Disk) :
    flush (a ++ b) d = flush b (flush a d) := by
  simp [flush, List.foldl_append]

/-! ## The superblock, and recovery (FORMAT §3, §3.1) -/

def MAGIC : Int := 3554557610294396226

/-- The 16 cells of the superblock at `a`. -/
def blk (d : Disk) (a : Nat) : List Int := (List.range 16).map fun j => d (a + j)

theorem blk_get {d : Disk} {a j : Nat} (hj : j < 16) : (blk d a).getD j 0 = d (a + j) := by
  simp [blk, List.getD_eq_getElem?_getD, List.getElem?_range hj]

theorem blk_congr {d e : Disk} {a : Nat} (h : ∀ j, j < 16 → d (a + j) = e (a + j)) :
    blk d a = blk e a :=
  List.map_congr_left (fun j hj => h j (List.mem_range.1 hj))

theorem blk_cell {d e : Disk} {a j : Nat} (h : blk d a = blk e a) (hj : j < 16) :
    d (a + j) = e (a + j) := by
  have := congrArg (fun l => List.getD l j 0) h
  rwa [blk_get hj, blk_get hj] at this

variable (crc : List Int → Int)

/-- Valid superblock: magic, and cell 15 = crc of cells 0..14 (`sb_valid_in`, lib.rs:63-69). -/
def blkValid (b : List Int) : Bool := b.getD 0 0 == MAGIC && crc (b.take 15) == b.getD 15 0

def valid (d : Disk) (a : Nat) : Bool := blkValid crc (blk d a)

/-- FORMAT §3.1 item 1: both valid -> higher generation (tie -> A); one -> it; none -> none. -/
def selB (a b : List Int) : Option Nat :=
  if blkValid crc a && blkValid crc b then (if b.getD 2 0 > a.getD 2 0 then some 512 else some 0)
  else if blkValid crc a then some 0
  else if blkValid crc b then some 512
  else none

def sel (d : Disk) : Option Nat := selB crc (blk d 0) (blk d 512)

/-- Everything generation `a` names: its superblock, its PartTab (cell 3, 21 cells), and the arena
  up to its arena_used (cell 4) -- every object the generation can reach lives there. -/
def inFp (b : List Int) (a i : Nat) : Bool :=
  decide ((a ≤ i ∧ i < a + 16) ∨
    (b.getD 3 0 ≠ 0 ∧ b.getD 3 0 ≤ (i : Int) ∧ (i : Int) < b.getD 3 0 + 21) ∨
    (1024 ≤ i ∧ (i : Int) < b.getD 4 0))

/-- What a reader of generation `a` can see: the disk restricted to that generation's footprint. -/
def view (d : Disk) (a : Nat) : Disk := fun i => if inFp (blk d a) a i then d i else 0

/-- FORMAT §3.1 item 2 on the chosen superblock: refusals, never a fallback. `n` = image cells. -/
def chk (n : Nat) (v : Disk) (a : Nat) : Bool :=
  let used := v (a + 4)
  let pt := v (a + 3)
  decide (0 ≤ used ∧ used ≤ n) &&
  (pt == 0 ||
    (decide (0 < pt ∧ pt + 20 < n) &&
     v pt.toNat % 4294967296 == 19 &&
     crc ((List.range 19).map fun j => v (pt.toNat + 2 + j)) == v (pt.toNat + 1) / 4294967296 &&
     decide (0 ≤ v (pt.toNat + 18) ∧ (v (pt.toNat + 18) = 0 ∨ v (pt.toNat + 18) + 2 < n))))

/-- `open`: pick, check, and the generation's view -- or a refusal (`none`). -/
def recover (n : Nat) (d : Disk) : Option (Nat × Disk) :=
  match sel crc d with
  | none => none
  | some a => if chk crc n (view d a) a then some (a, view d a) else none

theorem selB_some {a b : List Int} {x : Nat} (h : selB crc a b = some x) :
    (x = 0 ∧ blkValid crc a = true) ∨ (x = 512 ∧ blkValid crc b = true) := by
  unfold selB at h
  cases ha : blkValid crc a <;> cases hb : blkValid crc b <;> simp [ha, hb] at h
  · exact Or.inr ⟨h.symm, rfl⟩
  · exact Or.inl ⟨h.symm, rfl⟩
  · split at h
    · exact Or.inr ⟨(Option.some.inj h).symm, rfl⟩
    · exact Or.inl ⟨(Option.some.inj h).symm, rfl⟩

theorem sel_congr {d e : Disk} (h0 : blk d 0 = blk e 0) (h1 : blk d 512 = blk e 512) :
    sel crc d = sel crc e := by
  simp only [sel, h0, h1]

/-- Both superblock blocks unchanged -> the same choice. -/
theorem sel_same {d e : Disk} {a : Nat} (ha : a = 0 ∨ a = 512)
    (hs : blk d a = blk e a) (ho : blk d (512 - a) = blk e (512 - a)) : sel crc d = sel crc e := by
  rcases ha with rfl | rfl
  · exact sel_congr crc hs ho
  · exact sel_congr crc ho hs

/-- The live block unchanged and the other one invalid -> still the live one. -/
theorem sel_other_invalid {d e : Disk} {a : Nat} (hsel : sel crc e = some a)
    (hs : blk d a = blk e a) (hinv : valid crc d (512 - a) = false) : sel crc d = some a := by
  rcases selB_some crc hsel with ⟨rfl, hv⟩ | ⟨rfl, hv⟩
  · have h0 : blkValid crc (blk d 0) = true := by rw [hs]; exact hv
    have h1 : blkValid crc (blk d 512) = false := hinv
    simp [sel, selB, h0, h1]
  · have h1 : blkValid crc (blk d 512) = true := by rw [hs]; exact hv
    have h0 : blkValid crc (blk d 0) = false := hinv
    simp [sel, selB, h0, h1]

/-- THE FRAME: same choice, and the disk unchanged on the chosen generation's footprint -> the same
  recovery (view and verdict both read only the footprint). -/
theorem recover_frame (n : Nat) {d e : Disk} {a : Nat} (he : sel crc e = some a)
    (hd : sel crc d = some a) (hfp : ∀ i, inFp (blk e a) a i = true → d i = e i) :
    recover crc n d = recover crc n e := by
  have hb : blk d a = blk e a :=
    blk_congr (fun j _ => hfp (a + j) (by simp only [inFp, decide_eq_true_eq]; left; omega))
  have hv : view d a = view e a := by
    funext i
    simp only [view, hb]
    by_cases h : inFp (blk e a) a i = true
    · simp [h, hfp i h]
    · simp [h]
  simp only [recover, he, hd, hv]

/-! ## One commit -/

/-- The commit `Store::commit` runs: `tx.sb`, `tx.mark`, `tx.cursor - tx.mark`, and the in-memory
  store after `stage_commit` (every value written is one of its cells). -/
structure Commit where
  sb : Nat
  mark : Nat
  alen : Nat
  S : Disk

namespace Commit
variable (c : Commit)

def inA (i : Nat) : Prop := c.mark ≤ i ∧ i < c.mark + c.alen
def inP (i : Nat) : Prop := 512 - c.sb + 16 ≤ i ∧ i < 512 - c.sb + 16 + 21
def inB (i : Nat) : Prop := 512 - c.sb ≤ i ∧ i < 512 - c.sb + 16

instance instDecA (i : Nat) : Decidable (c.inA i) := by unfold inA; infer_instance
instance instDecP (i : Nat) : Decidable (c.inP i) := by unfold inP; infer_instance
instance instDecB (i : Nat) : Decidable (c.inB i) := by unfold inB; infer_instance

/-- The arena cells, the PartTab (21 cells at (512 - sb) + 16), the other superblock (16 cells). -/
def wA : List (Nat × Int) := cellsOf c.S c.mark c.alen
def wP : List (Nat × Int) := cellsOf c.S (512 - c.sb + 16) 21
def wB : List (Nat × Int) := cellsOf c.S (512 - c.sb) 16

/-- lib.rs:868-878, in order. -/
def rustOps : List Op := [.write c.wA, .sync, .write c.wP, .sync, .write c.wB, .sync]
/-- R-EMBED-STORE §3.5: arena and PartTab, one barrier, superblock, barrier. -/
def docOps : List Op := [.write c.wA, .write c.wP, .sync, .write c.wB, .sync]

/-- The file once the commit is durable. -/
def after (D0 : Disk) : Disk := fun i => if c.inA i ∨ c.inP i ∨ c.inB i then c.S i else D0 i

/-- Before the superblock write: nothing outside the arena range and the new PartTab changed. -/
def Old (D0 d : Disk) : Prop := ∀ i, ¬ c.inA i → ¬ c.inP i → d i = D0 i
/-- During the superblock write: everything else is the committed file, the superblock a mix. -/
def Mid (D0 d : Disk) : Prop :=
  (∀ i, ¬ c.inB i → d i = c.after D0 i) ∧ (∀ i, c.inB i → d i = D0 i ∨ d i = c.S i)

theorem mem_wA {i : Nat} {v : Int} (h : (i, v) ∈ c.wA) : c.inA i := by
  have h' := mem_cellsOf.1 h; unfold inA; exact ⟨h'.1, h'.2.1⟩
theorem mem_wP {i : Nat} {v : Int} (h : (i, v) ∈ c.wP) : c.inP i := by
  have h' := mem_cellsOf.1 h; unfold inP; exact ⟨h'.1, h'.2.1⟩
theorem mem_wB {i : Nat} {v : Int} (h : (i, v) ∈ c.wB) : c.inB i ∧ v = c.S i := by
  have h' := mem_cellsOf.1 h; unfold inB; exact ⟨⟨h'.1, h'.2.1⟩, h'.2.2⟩

theorem fA (d : Disk) (i : Nat) : flush c.wA d i = if c.inA i then c.S i else d i := by
  rw [wA, flush_cells]; unfold inA; rfl
theorem fP (d : Disk) (i : Nat) : flush c.wP d i = if c.inP i then c.S i else d i := by
  rw [wP, flush_cells]; unfold inP; rfl
theorem fB (d : Disk) (i : Nat) : flush c.wB d i = if c.inB i then c.S i else d i := by
  rw [wB, flush_cells]; unfold inB; rfl

/-- The three-write file equals `after`. -/
theorem flush3 (D0 : Disk) : flush c.wB (flush c.wP (flush c.wA D0)) = c.after D0 := by
  funext i
  rw [fB, fP, fA]
  unfold after
  by_cases hB : c.inB i <;> by_cases hP : c.inP i <;> by_cases hA : c.inA i <;> simp [hA, hP, hB]

theorem B_disj (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark) {i : Nat} (h : c.inB i) :
    ¬ c.inA i ∧ ¬ c.inP i := by
  simp only [inA, inP, inB] at *
  omega

/-- After the arena and PartTab writes, at least durable: the file is `after` off the superblock. -/
theorem flush2_off_B (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark) (D0 : Disk) (i : Nat) :
    flush c.wP (flush c.wA D0) i = if c.inB i then D0 i else c.after D0 i := by
  rw [fP, fA]
  unfold after
  by_cases hB : c.inB i
  · have := c.B_disj hsb hm hB
    simp [hB, this.1, this.2]
  · by_cases hP : c.inP i <;> by_cases hA : c.inA i <;> simp [hA, hP, hB]

/-- Crash states before the superblock is written. -/
theorem old_of (D0 d D : Disk) (pend : List (Nat × Int))
    (hD : ∀ i, ¬ c.inA i → ¬ c.inP i → D i = D0 i)
    (hp : ∀ i v, (i, v) ∈ pend → c.inA i ∨ c.inP i) (hd : Crash ⟨D, pend⟩ d) : c.Old D0 d := by
  intro i hA hP
  rcases hd i with h | h
  · rw [h]; exact hD i hA hP
  · rcases hp i _ h with h' | h'
    · exact absurd h' hA
    · exact absurd h' hP

/-- Crash states while the superblock write is pending. -/
theorem mid_of (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark) (D0 d : Disk)
    (hd : Crash ⟨flush c.wP (flush c.wA D0), c.wB⟩ d) : c.Mid D0 d := by
  constructor
  · intro i hB
    rcases hd i with h | h
    · change d i = flush c.wP (flush c.wA D0) i at h
      rw [h, c.flush2_off_B hsb hm, if_neg hB]
    · exact absurd (c.mem_wB h).1 hB
  · intro i hB
    rcases hd i with h | h
    · change d i = flush c.wP (flush c.wA D0) i at h
      left; rw [h, c.flush2_off_B hsb hm, if_pos hB]
    · right; exact (c.mem_wB h).2

theorem rust_crash_cases (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark) (D0 d : Disk)
    (k : Nat) (hd : Crash (run (c.rustOps.take k) ⟨D0, []⟩) d) :
    c.Old D0 d ∨ c.Mid D0 d ∨ d = c.after D0 := by
  have nA : ∀ i, ¬ c.inA i → ¬ c.inP i → flush c.wA D0 i = D0 i := by
    intro i hA _; rw [c.fA, if_neg hA]
  have nP : ∀ i, ¬ c.inA i → ¬ c.inP i → flush c.wP (flush c.wA D0) i = D0 i := by
    intro i hA hP; rw [c.fP, if_neg hP, c.fA, if_neg hA]
  match k, hd with
  | 0, hd => exact Or.inl (c.old_of D0 d D0 [] (fun _ _ _ => rfl) (fun _ _ h => absurd h List.not_mem_nil) hd)
  | 1, hd => exact Or.inl (c.old_of D0 d D0 c.wA (fun _ _ _ => rfl)
      (fun _ _ h => Or.inl (c.mem_wA h)) (by simpa [rustOps, run, step] using hd))
  | 2, hd => exact Or.inl (c.old_of D0 d (flush c.wA D0) [] nA
      (fun _ _ h => absurd h List.not_mem_nil) (by simpa [rustOps, run, step] using hd))
  | 3, hd => exact Or.inl (c.old_of D0 d (flush c.wA D0) c.wP nA
      (fun _ _ h => Or.inr (c.mem_wP h)) (by simpa [rustOps, run, step] using hd))
  | 4, hd => exact Or.inl (c.old_of D0 d (flush c.wP (flush c.wA D0)) [] nP
      (fun _ _ h => absurd h List.not_mem_nil) (by simpa [rustOps, run, step] using hd))
  | 5, hd => exact Or.inr (Or.inl (c.mid_of hsb hm D0 d (by simpa [rustOps, run, step] using hd)))
  | k + 6, hd =>
    have e : c.rustOps.take (k + 6) = c.rustOps := List.take_of_length_le (by simp [rustOps])
    rw [e] at hd
    have hd' : Crash ⟨flush c.wB (flush c.wP (flush c.wA D0)), []⟩ d := by
      simpa [rustOps, run, step] using hd
    refine Or.inr (Or.inr (funext fun i => ?_))
    rcases hd' i with h | h
    · rw [h, c.flush3]
    · exact absurd h List.not_mem_nil

theorem doc_crash_cases (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark) (D0 d : Disk)
    (k : Nat) (hd : Crash (run (c.docOps.take k) ⟨D0, []⟩) d) :
    c.Old D0 d ∨ c.Mid D0 d ∨ d = c.after D0 := by
  have nP : ∀ i, ¬ c.inA i → ¬ c.inP i → flush c.wP (flush c.wA D0) i = D0 i := by
    intro i hA hP; rw [c.fP, if_neg hP, c.fA, if_neg hA]
  have hAP : ∀ i v, (i, v) ∈ c.wA ++ c.wP → c.inA i ∨ c.inP i := by
    intro i v h
    rcases List.mem_append.1 h with h | h
    · exact Or.inl (c.mem_wA h)
    · exact Or.inr (c.mem_wP h)
  match k, hd with
  | 0, hd => exact Or.inl (c.old_of D0 d D0 [] (fun _ _ _ => rfl) (fun _ _ h => absurd h List.not_mem_nil) hd)
  | 1, hd => exact Or.inl (c.old_of D0 d D0 c.wA (fun _ _ _ => rfl)
      (fun _ _ h => Or.inl (c.mem_wA h)) (by simpa [docOps, run, step] using hd))
  | 2, hd => exact Or.inl (c.old_of D0 d D0 (c.wA ++ c.wP) (fun _ _ _ => rfl) hAP
      (by simpa [docOps, run, step] using hd))
  | 3, hd =>
    have hd' : Crash ⟨flush (c.wA ++ c.wP) D0, []⟩ d := by simpa [docOps, run, step] using hd
    rw [flush_append] at hd'
    exact Or.inl (c.old_of D0 d _ [] nP (fun _ _ h => absurd h List.not_mem_nil) hd')
  | 4, hd =>
    have hd' : Crash ⟨flush (c.wA ++ c.wP) D0, c.wB⟩ d := by simpa [docOps, run, step] using hd
    rw [flush_append] at hd'
    exact Or.inr (Or.inl (c.mid_of hsb hm D0 d hd'))
  | k + 5, hd =>
    have e : c.docOps.take (k + 5) = c.docOps := List.take_of_length_le (by simp [docOps])
    rw [e] at hd
    have hd' : Crash ⟨flush c.wB (flush (c.wA ++ c.wP) D0), []⟩ d := by
      simpa [docOps, run, step] using hd
    rw [flush_append] at hd'
    refine Or.inr (Or.inr (funext fun i => ?_))
    rcases hd' i with h | h
    · rw [h, c.flush3]
    · exact absurd h List.not_mem_nil

end Commit

/-! ## Atomicity -/

section
variable (n : Nat) (c : Commit) (D0 : Disk)

/-- The live generation's footprint is disjoint from everything the commit writes (from H_HOME and
  H_BEGIN). -/
theorem fp_disjoint (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark)
    (hpt : (blk D0 c.sb).getD 3 0 = 0 ∨ (blk D0 c.sb).getD 3 0 = (c.sb : Int) + 16)
    (hused : (blk D0 c.sb).getD 4 0 ≤ (c.mark : Int)) :
    ∀ i, inFp (blk D0 c.sb) c.sb i = true → ¬ c.inA i ∧ ¬ c.inP i ∧ ¬ c.inB i := by
  intro i h
  simp only [inFp, decide_eq_true_eq] at h
  simp only [Commit.inA, Commit.inP, Commit.inB]
  omega

/-- Superblock cells (0..15, 512..527) are never in the arena range or the new PartTab. -/
theorem sbcells_not_AP (hsb : c.sb = 0 ∨ c.sb = 512) (hm : 1024 ≤ c.mark) {a j : Nat}
    (ha : a = 0 ∨ a = 512) (hj : j < 16) : ¬ c.inA (a + j) ∧ ¬ c.inP (a + j) := by
  simp only [Commit.inA, Commit.inP]
  omega

/-- The core: Old / Mid / done each recover the old or the new generation. -/
theorem atomic_of_cases (hsel : sel crc D0 = some c.sb) (hm : 1024 ≤ c.mark)
    (hpt : (blk D0 c.sb).getD 3 0 = 0 ∨ (blk D0 c.sb).getD 3 0 = (c.sb : Int) + 16)
    (hused : (blk D0 c.sb).getD 4 0 ≤ (c.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - c.sb + j) = D0 (512 - c.sb + j) ∨ d (512 - c.sb + j) = c.S (512 - c.sb + j)) →
      valid crc d (512 - c.sb) = true →
      blk d (512 - c.sb) = blk D0 (512 - c.sb) ∨ blk d (512 - c.sb) = blk c.S (512 - c.sb))
    (d : Disk) (hcase : c.Old D0 d ∨ c.Mid D0 d ∨ d = c.after D0) :
    recover crc n d = recover crc n D0 ∨ recover crc n d = recover crc n (c.after D0) := by
  have hsb : c.sb = 0 ∨ c.sb = 512 := by
    rcases selB_some crc hsel with ⟨h, _⟩ | ⟨h, _⟩
    · exact Or.inl h
    · exact Or.inr h
  have hosb : 512 - c.sb = 0 ∨ 512 - c.sb = 512 := by omega
  have hdis := fp_disjoint c D0 hsb hm hpt hused
  rcases hcase with hO | hM | hE
  · -- crash before the superblock write
    left
    have hs : blk d c.sb = blk D0 c.sb := blk_congr fun j hj =>
      let h := sbcells_not_AP c hsb hm hsb hj; hO _ h.1 h.2
    have ho : blk d (512 - c.sb) = blk D0 (512 - c.sb) := blk_congr fun j hj =>
      let h := sbcells_not_AP c hsb hm hosb hj; hO _ h.1 h.2
    have hseld : sel crc d = some c.sb := by rw [sel_same crc hsb hs ho]; exact hsel
    exact recover_frame crc n hsel hseld fun i hi =>
      let h := hdis i hi; hO i h.1 h.2.1
  · -- crash while the superblock write is pending
    have off : ∀ i, inFp (blk D0 c.sb) c.sb i = true → d i = D0 i := by
      intro i hi
      have h := hdis i hi
      rw [hM.1 i h.2.2]
      unfold Commit.after
      rw [if_neg (fun h' => h'.elim h.1 (fun h'' => h''.elim h.2.1 h.2.2))]
    have hs : blk d c.sb = blk D0 c.sb := blk_congr fun j _ =>
      off (c.sb + j) (by simp only [inFp, decide_eq_true_eq]; left; omega)
    cases hv : valid crc d (512 - c.sb)
    · left
      exact recover_frame crc n hsel (sel_other_invalid crc hsel hs hv) off
    · have hmix : ∀ j, j < 16 → d (512 - c.sb + j) = D0 (512 - c.sb + j) ∨
          d (512 - c.sb + j) = c.S (512 - c.sb + j) := fun j hj =>
        hM.2 _ (by simp only [Commit.inB]; omega)
      rcases htorn d hmix hv with ho | ho
      · left
        have hseld : sel crc d = some c.sb := by rw [sel_same crc hsb hs ho]; exact hsel
        exact recover_frame crc n hsel hseld off
      · right
        have : d = c.after D0 := by
          funext i
          by_cases hB : c.inB i
          · have hj : i - (512 - c.sb) < 16 := by simp only [Commit.inB] at hB; omega
            have e : 512 - c.sb + (i - (512 - c.sb)) = i := by simp only [Commit.inB] at hB; omega
            have := blk_cell ho hj
            rw [e] at this
            rw [this]
            unfold Commit.after
            rw [if_pos (Or.inr (Or.inr hB))]
          · exact hM.1 i hB
        rw [this]
  · right; rw [hE]

/-- **L2, the Rust order (lib.rs:868-878).** A crash after ANY prefix of the commit's writes and
  barriers recovers exactly the old generation or exactly the new one: never a mix. -/
theorem rust_commit_atomic (hsel : sel crc D0 = some c.sb) (hm : 1024 ≤ c.mark)
    (hpt : (blk D0 c.sb).getD 3 0 = 0 ∨ (blk D0 c.sb).getD 3 0 = (c.sb : Int) + 16)
    (hused : (blk D0 c.sb).getD 4 0 ≤ (c.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - c.sb + j) = D0 (512 - c.sb + j) ∨ d (512 - c.sb + j) = c.S (512 - c.sb + j)) →
      valid crc d (512 - c.sb) = true →
      blk d (512 - c.sb) = blk D0 (512 - c.sb) ∨ blk d (512 - c.sb) = blk c.S (512 - c.sb))
    (k : Nat) (d : Disk) (hd : Crash (run (c.rustOps.take k) ⟨D0, []⟩) d) :
    recover crc n d = recover crc n D0 ∨ recover crc n d = recover crc n (c.after D0) := by
  have hsb : c.sb = 0 ∨ c.sb = 512 := by
    rcases selB_some crc hsel with ⟨h, _⟩ | ⟨h, _⟩
    · exact Or.inl h
    · exact Or.inr h
  exact atomic_of_cases crc n c D0 hsel hm hpt hused htorn d (c.rust_crash_cases hsb hm D0 d k hd)

/-- **L2, the §3.5 order** (two barriers): atomic too. -/
theorem doc_commit_atomic (hsel : sel crc D0 = some c.sb) (hm : 1024 ≤ c.mark)
    (hpt : (blk D0 c.sb).getD 3 0 = 0 ∨ (blk D0 c.sb).getD 3 0 = (c.sb : Int) + 16)
    (hused : (blk D0 c.sb).getD 4 0 ≤ (c.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - c.sb + j) = D0 (512 - c.sb + j) ∨ d (512 - c.sb + j) = c.S (512 - c.sb + j)) →
      valid crc d (512 - c.sb) = true →
      blk d (512 - c.sb) = blk D0 (512 - c.sb) ∨ blk d (512 - c.sb) = blk c.S (512 - c.sb))
    (k : Nat) (d : Disk) (hd : Crash (run (c.docOps.take k) ⟨D0, []⟩) d) :
    recover crc n d = recover crc n D0 ∨ recover crc n d = recover crc n (c.after D0) := by
  have hsb : c.sb = 0 ∨ c.sb = 512 := by
    rcases selB_some crc hsel with ⟨h, _⟩ | ⟨h, _⟩
    · exact Or.inl h
    · exact Or.inr h
  exact atomic_of_cases crc n c D0 hsel hm hpt hused htorn d (c.doc_crash_cases hsb hm D0 d k hd)

/-- The whole Rust commit leaves no pending write: the device is the committed file. -/
theorem rust_run (D0 : Disk) : run c.rustOps ⟨D0, []⟩ = ⟨c.after D0, []⟩ := by
  have : run c.rustOps ⟨D0, []⟩ = ⟨flush c.wB (flush c.wP (flush c.wA D0)), []⟩ := by
    simp [Commit.rustOps, run, step]
  rw [this, c.flush3]

/-- **Durability.** Once `commit` returns (its last sync_data returned), every crash recovers the new
  generation. -/
theorem rust_commit_durable (d : Disk) (hd : Crash (run c.rustOps ⟨D0, []⟩) d) :
    recover crc n d = recover crc n (c.after D0) := by
  rw [rust_run] at hd
  have : d = c.after D0 := funext fun i => by
    rcases hd i with h | h
    · exact h
    · exact absurd h List.not_mem_nil
  rw [this]

/-- **A committed generation is never lost by a later crash**: commit c1 returns, then commit c2
  (from the generation c1 made) crashes after ANY prefix -> generation k+1 or k+2, never k. -/
theorem rust_never_lost (c1 c2 : Commit)
    (hsel : sel crc (c1.after D0) = some c2.sb) (hm : 1024 ≤ c2.mark)
    (hpt : (blk (c1.after D0) c2.sb).getD 3 0 = 0 ∨
      (blk (c1.after D0) c2.sb).getD 3 0 = (c2.sb : Int) + 16)
    (hused : (blk (c1.after D0) c2.sb).getD 4 0 ≤ (c2.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - c2.sb + j) = c1.after D0 (512 - c2.sb + j) ∨
        d (512 - c2.sb + j) = c2.S (512 - c2.sb + j)) →
      valid crc d (512 - c2.sb) = true →
      blk d (512 - c2.sb) = blk (c1.after D0) (512 - c2.sb) ∨
        blk d (512 - c2.sb) = blk c2.S (512 - c2.sb))
    (k : Nat) (d : Disk) (hd : Crash (run (c1.rustOps ++ c2.rustOps.take k) ⟨D0, []⟩) d) :
    recover crc n d = recover crc n (c1.after D0) ∨
      recover crc n d = recover crc n (c2.after (c1.after D0)) := by
  have e : run (c1.rustOps ++ c2.rustOps.take k) ⟨D0, []⟩ =
      run (c2.rustOps.take k) (run c1.rustOps ⟨D0, []⟩) := by
    simp [run, List.foldl_append]
  rw [e, rust_run] at hd
  exact rust_commit_atomic crc n c2 (c1.after D0) hsel hm hpt hused htorn k d hd

/-- With both generations recoverable (H_STAGED: the staged image is itself a valid generation),
  a crash anywhere in the commit is NEVER a refusal. -/
theorem rust_commit_not_refused (hsel : sel crc D0 = some c.sb) (hm : 1024 ≤ c.mark)
    (hpt : (blk D0 c.sb).getD 3 0 = 0 ∨ (blk D0 c.sb).getD 3 0 = (c.sb : Int) + 16)
    (hused : (blk D0 c.sb).getD 4 0 ≤ (c.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - c.sb + j) = D0 (512 - c.sb + j) ∨ d (512 - c.sb + j) = c.S (512 - c.sb + j)) →
      valid crc d (512 - c.sb) = true →
      blk d (512 - c.sb) = blk D0 (512 - c.sb) ∨ blk d (512 - c.sb) = blk c.S (512 - c.sb))
    (h0 : recover crc n D0 ≠ none) (h1 : recover crc n (c.after D0) ≠ none)
    (k : Nat) (d : Disk) (hd : Crash (run (c.rustOps.take k) ⟨D0, []⟩) d) :
    recover crc n d ≠ none := by
  rcases rust_commit_atomic crc n c D0 hsel hm hpt hused htorn k d hd with h | h <;> rw [h]
  · exact h0
  · exact h1

end

/-! ### Non-vacuity: a fresh image (create, lib.rs:283-301) satisfies `sel` = A -/

private def toyCrc (l : List Int) : Int := l.foldl (· + ·) 0
private def fresh : Disk := fun i =>
  if i = 0 then MAGIC else if i = 1 then 2 else if i = 4 then 1024 else if i = 12 then 64
  else if i = 15 then MAGIC + 2 + 1024 + 64 else 0

example : sel toyCrc fresh = some 0 := by decide
example : (recover toyCrc 1088 fresh).isSome = true := by decide

end Bebop.Disk

#print axioms Bebop.Disk.rust_commit_atomic
#print axioms Bebop.Disk.doc_commit_atomic
#print axioms Bebop.Disk.rust_commit_durable
#print axioms Bebop.Disk.rust_never_lost
#print axioms Bebop.Disk.rust_commit_not_refused
