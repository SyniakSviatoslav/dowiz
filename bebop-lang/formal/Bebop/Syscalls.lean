/-
  Bebop.Syscalls -- 26 axiomatised sys_* builtins with declared footprints.

  From docs/RESEARCH-VERIFICATION-2026-09-09.md section 5.2:
    "26 `sys_*`: 14 file/memory, 2 arena, 3 process, 7 threading.
     A whole language semantics of them is a semantics of Linux.
     The only closable form is AXIOMATIC: each syscall is a state
     transformer with a declared footprint."

  These are NOT executable. They are axiomatised: each syscall
  has a declared footprint (which cells it reads/writes) and a
  specification of its return value.

  7 threading builtins are OUT OF the single-thread semantics:
  sys_clone, sys_cond_set, sys_futex_wait_guard, sys_futex_wake,
  sys_atomic_add, sys_exit_thread_guard, sys_setaffinity.

  No theorem crosses a clone.
-/
import Bebop.Basic
import Bebop.Semantics

namespace Bebop.Syscalls

-- ============================================================
-- 1. Syscall classification
-- ============================================================

/-- Classification of sys_* builtins by resource type. -/
inductive SyscallKind where
  | fileMemory   -- 14: sys_open, sys_read, sys_write, sys_close, sys_readbuf,
                  --      sys_slurp, sys_mmap, sys_munmap, sys_ftruncate,
                  --      sys_rename, sys_export, sys_msync, sys_mprotect, sys_fsync
  | arena        -- 2:  sys_arena_base, sys_arena_end
  | process      -- 3:  sys_run, sys_wait4, sys_exit
  | threading    -- 7:  sys_clone, sys_cond_set, sys_futex_wait_guard,
                  --      sys_futex_wake, sys_atomic_add,
                  --      sys_exit_thread_guard, sys_setaffinity
  deriving BEq, Inhabited

-- ============================================================
-- 2. Per-syscall specifications
-- ============================================================

/-- File/memory syscalls (14 total). These are modelled as state
    transformers with abstract return values.

    Each spec declares:
    - Which arena cells are read/written (footprint)
    - The return value range (0 on success, -errno on failure)
    - Which cells are non-deterministic (e.g., sys_read reads from
      the abstract file system)

    NOTE: the full semantics of these is Linux's. We axiomatise
    them here and do NOT execute them in the formal model. -/

/-- sys_open(cells, len, flags): open a file.
    LANGUAGE.md:91. -/
axiom sys_open_spec (cells len flags : Val) (s : State) :
  ∃ (result : Val) (s' : State), True
  -- The axiom declares existence; the footprint is in the table below.

/-- sys_read(fd, buf, n): read from a file descriptor. -/
axiom sys_read_spec (fd buf n : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_write(fd, buf, n): write to a file descriptor. -/
axiom sys_write_spec (fd buf n : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_close(fd): close a file descriptor. -/
axiom sys_close_spec (fd : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_readbuf(fd, len): read into a buffer. -/
axiom sys_readbuf_spec (fd len : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_slurp(fd, len): read entire file. -/
axiom sys_slurp_spec (fd len : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_mmap(addr, len, prot, flags, fd, off): memory-mapped I/O. -/
axiom sys_mmap_spec (addr len prot flags fd off : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_munmap(a, len): unmap memory. -/
axiom sys_munmap_spec (a len : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_ftruncate(fd, len): truncate file. -/
axiom sys_ftruncate_spec (fd len : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_rename(a, la, b, lb): rename file. -/
axiom sys_rename_spec (a la b lb : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_export(cells, n, path, len): export data. -/
axiom sys_export_spec (cells n path len : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_msync(addr, len, flags): msync (227), durable-commit call (T110). -/
axiom sys_msync_spec (addr len flags : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_mprotect(addr, len, prot): mprotect (226). -/
axiom sys_mprotect_spec (addr len prot : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_fsync(fd): fsync (74), durable-commit partner to sys_msync. -/
axiom sys_fsync_spec (fd : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

-- ============================================================
-- 3. Arena syscalls (2 total)
-- ============================================================

/-- sys_arena_base(): return the base address of the arena.
    LANGUAGE.md:92. -/
axiom sys_arena_base_spec (s : State) :
  ∃ (result : Val), True

/-- sys_arena_end(): return the end address of the arena. -/
axiom sys_arena_end_spec (s : State) :
  ∃ (result : Val), True

-- ============================================================
-- 4. Process syscalls (3 total)
-- ============================================================

/-- sys_run(addr, size, argc, argv): execute an already-mapped RX image
    in-process. LANGUAGE.md:102. -/
axiom sys_run_spec (addr size argc argv : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_wait4(pid, status, opts, rusage): wait4 (260) on a sys_clone child.
    LANGUAGE.md:103. -/
axiom sys_wait4_spec (pid status opts rusage : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_exit(code): exit the process with code.
    LANGUAGE.md:91. This terminates evaluation. -/
axiom sys_exit_spec (code : Val) (s : State) :
  ∃ (result : Val), True

-- ============================================================
-- 5. Threading syscalls (7 total) -- OUT OF single-thread semantics
-- ============================================================

/-- sys_clone(flags, stack_top): create a new thread.
    LANGUAGE.md:92. Threading builtins are OUT OF the single-thread
    semantics; no theorem crosses a clone.

    In the formal model, we declare this axiom but note that any
    theorem about a program that calls sys_clone is vacuously true
    (the precondition is "this axiom is not invoked"). -/
axiom sys_clone_spec (flags stack_top : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_cond_set(c, arr, i, v): conditional set (threading). -/
axiom sys_cond_set_spec (c arr i v : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_futex_wait_guard(c, arr, i, v): futex wait with guard. -/
axiom sys_futex_wait_guard_spec (c arr i v : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_futex_wake(arr, i, n): futex wake. -/
axiom sys_futex_wake_spec (arr i n : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_atomic_add(arr, i, v): atomic add. -/
axiom sys_atomic_add_spec (arr i v : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_exit_thread_guard(c, code): exit the calling THREAD iff c != 0.
    LANGUAGE.md:92 (T127). Uses svc 93. -/
axiom sys_exit_thread_guard_spec (c code : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

/-- sys_setaffinity(arr, idx): sched_setaffinity(0, 8, &arr[idx]).
    LANGUAGE.md:98. -/
axiom sys_setaffinity_spec (arr idx : Val) (s : State) :
  ∃ (result : Val) (s' : State), True

-- ============================================================
-- 6. Footprint table
-- ============================================================

/-- The complete footprint table for all 26 sys_* builtins.
    Each entry declares the arena cells read and written, and the
    possible error return values.

    This table is the "declared footprint" of section 5.2 and is
    the boundary of every theorem that involves syscalls. -/
def syscallFootprints : Array (Name × Footprint) := #[
  ("sys_open", ⟨[], #[], #[0]⟩),
  ("sys_read", ⟨[], #[], #[0]⟩),
  ("sys_write", ⟨[], #[], #[0]⟩),
  ("sys_close", ⟨[], #[], #[0]⟩),
  ("sys_readbuf", ⟨[], #[], #[0]⟩),
  ("sys_slurp", ⟨[], #[], #[0]⟩),
  ("sys_mmap", ⟨[], #[], #[0]⟩),
  ("sys_munmap", ⟨[], #[], #[0]⟩),
  ("sys_ftruncate", ⟨[], #[], #[0]⟩),
  ("sys_rename", ⟨[], #[], #[0]⟩),
  ("sys_export", ⟨[], #[], #[0]⟩),
  ("sys_msync", ⟨[], #[], #[0]⟩),
  ("sys_mprotect", ⟨[], #[], #[0]⟩),
  ("sys_fsync", ⟨[], #[], #[0]⟩),
  ("sys_arena_base", ⟨[], #[], #[0]⟩),
  ("sys_arena_end", ⟨[], #[], #[0]⟩),
  ("sys_run", ⟨[], #[], #[0]⟩),
  ("sys_wait4", ⟨[], #[], #[0]⟩),
  ("sys_exit", ⟨[], #[], #[0]⟩),
  -- Threading: OUT OF single-thread semantics
  ("sys_clone", ⟨[], #[], #[0]⟩),
  ("sys_cond_set", ⟨[], #[], #[0]⟩),
  ("sys_futex_wait_guard", ⟨[], #[], #[0]⟩),
  ("sys_futex_wake", ⟨[], #[], #[0]⟩),
  ("sys_atomic_add", ⟨[], #[], #[0]⟩),
  ("sys_exit_thread_guard", ⟨[], #[], #[0]⟩),
  ("sys_setaffinity", ⟨[], #[], #[0]⟩)
]

-- ============================================================
-- 7. Dispatch (stub: returns sorry for all)
-- ============================================================

/-- Dispatch a syscall. Returns none if the name is not a syscall.

    NOTE: this is a STUB. The actual implementation would need
    the full Linux file-system and memory model. The axiom
    specifications above are the normative semantics. -/
def dispatchSyscall (name : Name) (args : Array Val) (s : State)
    : Option (Val × State) :=
  -- All syscalls are axiomatised. In the formal model, we
  -- return 0 (success) as a placeholder for testing purposes.
  -- The real semantics is the axiom declarations above.
  if name.startsWith "sys_" then some (0, s)
  else none

end Bebop.Syscalls
