/-
  Bebop.Group -- W-ES-P11 (R-EMBED-STORE §12 row P11): THE GROUP COMMIT UNDER TWO BARRIERS, and
  `Durability::Normal`'s checkpoints. Core Lean 4, no Mathlib, no `axiom`, no `sorry`.

  THE CODE IT MODELS (crates/bebop-io/src/writer.rs `Writer::flush_steps` + `flip`, plan.rs):
    N commits are STAGED in memory (`Store::stage_commit` N times, alternating superblock pages);
    one flush writes the arena of ALL of them, [mark_0, mark_0 + Σ alen_i), the PartTab of the LAST
    and, after ONE barrier, the superblock of the LAST, then the second barrier. When N is even the
    last superblock was staged on the durable page; the writer's FLIP moves it (and its PartTab) in
    memory to the other page and restores the durable page, so the device sees exactly `docOps` of
    ONE commit whose arena is the whole group's: `Group.whole`. The generations between never reach
    the device.

  PROVED:
    group_two_barriers      the group's op list holds exactly two syncs, whatever N
    group_commit_atomic     a crash after ANY prefix of the group's ops recovers the old generation
                            or the whole group's -- never a generation between (there is none on
                            the device: `group_no_between`)
    group_durable           once the second sync returned, every crash recovers the group
    checkpoints_never_lost  `Durability::Normal`: checkpoints g_1..g_m run whole, a crash anywhere
                            in checkpoint g_{m+1} recovers checkpoint m or m+1 -- a checkpointed
                            prefix with its exact view, never less than the last checkpoint

  The hypotheses are Disk.lean's (H_TORN, H_HOME, H_BEGIN) stated for the whole group's commit;
  nothing new is assumed.
-/
import Bebop.Disk

namespace Bebop.Disk

variable (crc : List Int → Int)

/-- N staged commits flushed once: the arena lengths of each (in order), the durable cursor the
  first one appended from, the page the LAST superblock lands on (after the flip: opposite the
  durable one), and the in-memory image after the last `stage_commit` (+ flip). -/
structure Group where
  sizes : List Nat
  mark : Nat
  sb : Nat
  S : Disk

namespace Group
variable (g : Group)

/-- How many commits the group holds. -/
def n : Nat := g.sizes.length

/-- The ONE commit the device sees: the whole arena, the last PartTab and superblock. -/
def whole : Commit := { sb := g.sb, mark := g.mark, alen := g.sizes.sum, S := g.S }

/-- What the writer issues: `plan`'s two-barrier order over `whole`. -/
def ops : List Op := g.whole.docOps

def after (D0 : Disk) : Disk := g.whole.after D0

/-- Syncs in an op list. -/
def syncs : List Op → Nat
  | [] => 0
  | .sync :: t => syncs t + 1
  | .write _ :: t => syncs t

/-- EXACTLY TWO BARRIERS, for every N (the sizes do not even appear). -/
theorem two_barriers : syncs g.ops = 2 := by
  simp [ops, Commit.docOps, syncs]

/-- The three-barrier order, for contrast: the mutant that would make `two_barriers` false. -/
theorem rust_three_barriers : syncs g.whole.rustOps = 3 := by
  simp [Commit.rustOps, syncs]

end Group

section
variable (n : Nat) (g : Group) (D0 : Disk)

/-- **Group atomicity.** A crash after ANY prefix of the group's ops recovers the generation before
  the group or the generation after the WHOLE group. -/
theorem group_commit_atomic (hsel : sel crc D0 = some g.sb) (hm : 1024 ≤ g.mark)
    (hpt : (blk D0 g.sb).getD 3 0 = 0 ∨ (blk D0 g.sb).getD 3 0 = (g.sb : Int) + 16)
    (hused : (blk D0 g.sb).getD 4 0 ≤ (g.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - g.sb + j) = D0 (512 - g.sb + j) ∨ d (512 - g.sb + j) = g.S (512 - g.sb + j)) →
      valid crc d (512 - g.sb) = true →
      blk d (512 - g.sb) = blk D0 (512 - g.sb) ∨ blk d (512 - g.sb) = blk g.S (512 - g.sb))
    (k : Nat) (d : Disk) (hd : Crash (run (g.ops.take k) ⟨D0, []⟩) d) :
    recover crc n d = recover crc n D0 ∨ recover crc n d = recover crc n (g.after D0) :=
  doc_commit_atomic crc n g.whole D0 hsel hm hpt hused htorn k d hd

/-- No generation between: EVERY cell of the device after the group (the superblock pages
  included) is the old disk's or the group's LAST image's -- the staged generations
  k+1..k+N-1 never reach it. -/
theorem group_no_between (i : Nat) : g.after D0 i = D0 i ∨ g.after D0 i = g.S i := by
  unfold Group.after Commit.after
  by_cases h : g.whole.inA i ∨ g.whole.inP i ∨ g.whole.inB i
  · right; rw [if_pos h]; rfl
  · left; rw [if_neg h]

/-- The whole group's ops leave no pending write: the device is the committed file. -/
theorem group_run : run g.ops ⟨D0, []⟩ = ⟨g.after D0, []⟩ := by
  have : run g.ops ⟨D0, []⟩ = ⟨flush g.whole.wB (flush (g.whole.wA ++ g.whole.wP) D0), []⟩ := by
    simp [Group.ops, Commit.docOps, run, step]
  rw [this, flush_append, g.whole.flush3]
  rfl

/-- **Durability.** Once the second sync returned, every crash recovers the group. -/
theorem group_durable (d : Disk) (hd : Crash (run g.ops ⟨D0, []⟩) d) :
    recover crc n d = recover crc n (g.after D0) := by
  rw [group_run] at hd
  have : d = g.after D0 := funext fun i => by
    rcases hd i with h | h
    · exact h
    · exact absurd h List.not_mem_nil
  rw [this]

end

/-! ## `Durability::Normal`: a sequence of checkpoints -/

/-- Checkpoints g_1 .. g_m run whole, in order, from `D0`: the device after them. -/
def ckpt (gs : List Group) (D0 : Disk) : Disk := gs.foldl (fun D g => g.after D) D0

/-- Their ops, concatenated. -/
def ckptOps (gs : List Group) : List Op := gs.foldr (fun g acc => g.ops ++ acc) []

theorem ckpt_run (gs : List Group) (D0 : Disk) : run (ckptOps gs) ⟨D0, []⟩ = ⟨ckpt gs D0, []⟩ := by
  induction gs generalizing D0 with
  | nil => rfl
  | cons g gs ih =>
    show run (g.ops ++ ckptOps gs) ⟨D0, []⟩ = ⟨ckpt gs (g.after D0), []⟩
    have e : run (g.ops ++ ckptOps gs) ⟨D0, []⟩ = run (ckptOps gs) (run g.ops ⟨D0, []⟩) := by
      simp [run, List.foldl_append]
    rw [e, group_run, ih]

/-- **Checkpoints are never lost** (`Durability::Normal`). Checkpoints `gs` ran whole; the next
  checkpoint `g` crashes after ANY prefix: recovery is the last checkpoint's device or the new
  checkpoint's -- never less than the last checkpoint, never a mix. -/
theorem checkpoints_never_lost (n : Nat) (gs : List Group) (g : Group) (D0 : Disk)
    (hsel : sel crc (ckpt gs D0) = some g.sb) (hm : 1024 ≤ g.mark)
    (hpt : (blk (ckpt gs D0) g.sb).getD 3 0 = 0 ∨ (blk (ckpt gs D0) g.sb).getD 3 0 = (g.sb : Int) + 16)
    (hused : (blk (ckpt gs D0) g.sb).getD 4 0 ≤ (g.mark : Int))
    (htorn : ∀ d : Disk,
      (∀ j, j < 16 → d (512 - g.sb + j) = ckpt gs D0 (512 - g.sb + j) ∨
        d (512 - g.sb + j) = g.S (512 - g.sb + j)) →
      valid crc d (512 - g.sb) = true →
      blk d (512 - g.sb) = blk (ckpt gs D0) (512 - g.sb) ∨ blk d (512 - g.sb) = blk g.S (512 - g.sb))
    (k : Nat) (d : Disk) (hd : Crash (run (ckptOps gs ++ g.ops.take k) ⟨D0, []⟩) d) :
    recover crc n d = recover crc n (ckpt gs D0) ∨
      recover crc n d = recover crc n (g.after (ckpt gs D0)) := by
  have e : run (ckptOps gs ++ g.ops.take k) ⟨D0, []⟩ = run (g.ops.take k) (run (ckptOps gs) ⟨D0, []⟩) := by
    simp [run, List.foldl_append]
  rw [e, ckpt_run] at hd
  exact group_commit_atomic crc n g (ckpt gs D0) hsel hm hpt hused htorn k d hd

end Bebop.Disk

#print axioms Bebop.Disk.Group.two_barriers
#print axioms Bebop.Disk.group_commit_atomic
#print axioms Bebop.Disk.group_no_between
#print axioms Bebop.Disk.group_durable
#print axioms Bebop.Disk.checkpoints_never_lost
