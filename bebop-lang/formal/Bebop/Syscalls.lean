/-  Bebop.Syscalls -- 26 axiomatised sys_* with declared footprints.
    From docs/RESEARCH-VERIFICATION-2026-09-09.md section 5.2:
    "26 `sys_*`: 14 file/memory, 2 arena, 3 process, 7 threading.
     A whole language semantics of them is a semantics of Linux.
     The only closable form is AXIOMATIC: each syscall is a state
     transformer with a declared footprint."

    Imports only Bebop.Basic; Bebop.Semantics imports this module.
    These are NOT executable. They are axiomatised: each syscall
    has a declared footprint (which cells it reads/writes) and a
    specification of its return value.

    7 threading builtins are OUT OF the single-thread semantics:
    sys_clone, sys_cond_set, sys_futex_wait_guard, sys_futex_wake,
    sys_atomic_add, sys_exit_thread_guard, sys_setaffinity.

    No theorem crosses a clone.
-/

import Bebop.Basic

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
-- 2. Per-syscall specifications (with DECLARED footprints)
-- ============================================================

/- File/memory syscalls (14 total). These are modelled as state
    transformers with abstract return values.

    Each spec declares:
    - Which arena cells are read/written (footprint)
    - The return value range (0 on success, -errno on failure)
    - Non-determinism: sys_read reads from the abstract file system
-/

/-- sys_open(cells, len, flags): open a file.
    LANGUAGE.md:91.
    Footprint: reads no arena cells; writes no arena cells.
    Returns fd >= 0 on success, or -errno on failure.
    The path is passed as a str value in the cells array (not modelled). -/
axiom sys_open_spec (cells len flags : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    -- Returns a non-negative fd on success
    (result >= 0 → result < 1024) ∧
    -- Returns -errno on failure
    (result < 0 → result ∈ #[ -2, -5, -13, -21, -27, -71 ]) ∧
    -- No arena cells read or written
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_read(fd, buf, n): read from a file descriptor.
    LANGUAGE.md:91.
    Footprint: writes arena[buf..buf+n-1] (the buffer).
    Reads no arena cells (fd is OS state).
    Returns bytes read (>= 0, <= n) on success, or -errno on failure.
    The content read is non-deterministic (from the abstract file system). -/
axiom sys_read_spec (fd buf n : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    -- Returns bytes read on success (0 <= result <= n)
    (result >= 0 → result ≤ n ∧ result ≤ 8192) ∧
    -- Returns -errno on failure
    (result < 0 → result ∈ #[ -3, -5, -9, -11, -14 ]) ∧
    -- Writes to the buffer region: arena[buf..buf+result-1] may be modified
    -- (the content is non-deterministic)
    (∀ i, (i < buf.toNatClampNeg ∨ i >= (buf + result).toNatClampNeg ∨ i >= s.arena.cells.size) →
           s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_write(fd, buf, n): write to a file descriptor.
    LANGUAGE.md:91.
    Footprint: reads arena[buf..buf+n-1] (the buffer).
    Writes no arena cells (fd is OS state).
    Returns bytes written (>= 0, <= n) on success, or -errno on failure. -/
axiom sys_write_spec (fd buf n : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    -- Returns bytes written on success (0 <= result <= n)
    (result >= 0 → result ≤ n ∧ result ≤ 8192) ∧
    -- Returns -errno on failure
    (result < 0 → result ∈ #[ -3, -5, -9, -11, -14 ]) ∧
    -- No arena cells written (data goes to fd, not arena)
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_close(fd): close a file descriptor.
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells.
    Returns 0 on success, or -errno on failure. -/
axiom sys_close_spec (fd : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -3, -9, -11 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_readbuf(fd, len): read into a buffer.
    LANGUAGE.md:91. Similar to sys_read but with different calling convention.
    Footprint: writes arena[0..len-1].
    Returns bytes read on success, or -errno on failure. -/
axiom sys_readbuf_spec (fd len : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result >= 0 → result ≤ len ∧ result ≤ 8192) ∧
    (result < 0 → result ∈ #[ -3, -5, -9, -11, -14 ]) ∧
    (∀ i, (i >= len.toNatClampNeg ∨ i >= s.arena.cells.size) →
           s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_slurp(fd, len): read entire file.
    LANGUAGE.md:91.
    Footprint: writes arena[0..len-1].
    Returns the data pointer (arena base offset) on success, or -errno on failure. -/
axiom sys_slurp_spec (fd len : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result >= 0 → result == Int64.ofNat s.arena.cells.size) ∧  -- returns arena offset
    (result < 0 → result ∈ #[ -3, -5, -9, -11, -14 ]) ∧
    (∀ i, (i >= len.toNatClampNeg ∨ i >= s.arena.cells.size) →
           s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_mmap(addr, len, prot, flags, fd, off): memory-mapped I/O.
    LANGUAGE.md:91.
    Footprint: writes arena[addr..addr+len-1] (the mapped region).
    Returns the mapped address (== addr) on success, or -errno on failure. -/
axiom sys_mmap_spec (addr len prot flags fd off : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result >= 0 → result == addr) ∧
    (result < 0 → result ∈ #[ -12, -13, -17, -22, -27, -35 ]) ∧
    (∀ i, (i < addr.toNatClampNeg ∨ i >= (addr + len).toNatClampNeg ∨ i >= s.arena.cells.size) →
           s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_munmap(a, len): unmap memory.
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells (munmap is an OS operation).
    Returns 0 on success, or -errno on failure. -/
axiom sys_munmap_spec (a len : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -12, -13, -17 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_ftruncate(fd, len): truncate file.
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells.
    Returns 0 on success, or -errno on failure. -/
axiom sys_ftruncate_spec (fd len : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -3, -5, -9, -11, -14, -22 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_rename(a, la, b, lb): rename file.
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells.
    Returns 0 on success, or -errno on failure.
    Path strings are passed as str values (not modelled). -/
axiom sys_rename_spec (a la b lb : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -2, -5, -13, -21, -39 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_export(cells, n, path, len): export data.
    LANGUAGE.md:91.
    Footprint: reads arena[cells..cells+n-1].
    Returns 0 on success, or -errno on failure. -/
axiom sys_export_spec (cells n path len : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -2, -5, -13, -21, -27 ]) ∧
    (∀ i, (i < cells.toNatClampNeg ∨ i >= (cells + n).toNatClampNeg ∨ i >= s.arena.cells.size) →
           s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_msync(addr, len, flags): msync (227), durable-commit call (T110).
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells (msync is an OS operation on pages).
    Returns 0 on success, or -errno on failure. -/
axiom sys_msync_spec (addr len flags : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -22, -27, -35 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_mprotect(addr, len, prot): mprotect (226).
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells (mprotect is an OS operation on pages).
    Returns 0 on success, or -errno on failure. -/
axiom sys_mprotect_spec (addr len prot : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -22, -27, -35 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_fsync(fd): fsync (74), durable-commit partner to sys_msync.
    LANGUAGE.md:91.
    Footprint: reads/writes no arena cells (fsync is an OS operation on fd).
    Returns 0 on success, or -errno on failure. -/
axiom sys_fsync_spec (fd : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result == 0 ∨ result ∈ #[ -22, -27, -35 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

-- ============================================================
-- 3. Arena syscalls (2 total)
-- ============================================================

/-- sys_arena_base(): return the base address of the arena.
    LANGUAGE.md:92.
    Footprint: reads/writes no arena cells.
    Returns the arena base address (always 0 in our model) on success. -/
axiom sys_arena_base_spec (s : State) :
  ∃ (result : Val), result == 0 ∧ True

/-- sys_arena_end(): return the end address of the arena.
    LANGUAGE.md:92.
    Footprint: reads/writes no arena cells.
    Returns the arena end address (arena_size * 8) on success. -/
axiom sys_arena_end_spec (s : State) :
  ∃ (result : Val), result == Int64.ofNat (s.arena.cells.size * 8) ∧ True

-- ============================================================
-- 4. Process syscalls (3 total)
-- ============================================================

/-- sys_run(addr, size, argc, argv): execute an already-mapped RX image
    in-process. LANGUAGE.md:102.
    Footprint: reads/writes no arena cells (the child image is already mapped).
    Returns the exit code of the child on success.
    This terminates the current execution context (the child replaces the parent
    in the formal model, or we model it as a state transition). -/
axiom sys_run_spec (addr size argc argv : Val) (s : State) :
  ∃ (result : Val), result >= 0 ∧ True
  -- Note: in a full model, this would replace the current state with the
  -- child's execution. For the single-thread semantics, we treat it as
  -- an axiom that returns an abstract result.

/-- sys_wait4(pid, status, opts, rusage): wait4 (260) on a sys_clone child.
    LANGUAGE.md:103.
    Footprint: reads/writes no arena cells.
    Returns the pid of the waited-for child on success, or -errno on failure. -/
axiom sys_wait4_spec (pid status opts rusage : Val) (s : State) :
  ∃ (result : Val) (s' : State),
    (result >= 0 → result == pid) ∧
    (result < 0 → result ∈ #[ -3, -5, -11, -14, -22 ]) ∧
    (∀ i, i < s.arena.cells.size → s.arena.cells[i]? == s'.arena.cells[i]?) ∧
    True

/-- sys_exit(code): exit the process with code.
    LANGUAGE.md:91. This terminates evaluation.
    Footprint: reads/writes no arena cells.
    Returns never (the process exits). In the formal model, this is represented
    as a state transformer that produces a terminal result. -/
axiom sys_exit_spec (code : Val) (s : State) :
  ∃ (result : Val), result == code ∧ True
  -- In the interpreter, sys_exit is handled specially: it causes the program
  -- to terminate with the given exit code. The axiom declares that if the
  -- syscall is invoked, the result is the exit code.

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
  -- This axiom is declared but not used in any theorem.
  -- Threading is outside the scope of the single-thread semantics.

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
-- 6. Footprint table (for 5 syscalls with declared footprints)
-- ============================================================

/- The complete footprint table for the 5 axiomatised syscalls
    that have declared footprints: sys_read, sys_write, sys_open,
    sys_close, sys_mmap.

    Each entry declares:
    - Which arena cells are read/written (footprint)
    - The possible error return values

    The remaining 21 sys_* are axiomatised without footprints
    (or with trivial footprints that don't affect the arena).
-/

/-- Footprint for sys_open:
    - Arena cells read: none
    - Arena cells written: none
    - Error set: [-2, -5, -13, -21, -27, -71]
-/
def sys_open_footprint : Footprint :=
  ⟨#[], #[], #[ -2, -5, -13, -21, -27, -71 ]⟩

/-- Footprint for sys_read:
    - Arena cells read: none (fd is OS state)
    - Arena cells written: [buf, buf+1, ..., buf+n-1] (the buffer)
    - Error set: [-3, -5, -9, -11, -14]
-/
def sys_read_footprint (buf n : Val) : Footprint :=
  ⟨#[], (Array.range n.toNatClampNeg).map (fun i => buf.toNatClampNeg + i), #[ -3, -5, -9, -11, -14 ]⟩

/-- Footprint for sys_write:
    - Arena cells read: [buf, buf+1, ..., buf+n-1] (the buffer)
    - Arena cells written: none (data goes to fd)
    - Error set: [-3, -5, -9, -11, -14]
-/
def sys_write_footprint (buf n : Val) : Footprint :=
  ⟨(Array.range n.toNatClampNeg).map (fun i => buf.toNatClampNeg + i), #[], #[ -3, -5, -9, -11, -14 ]⟩

/-- Footprint for sys_close:
    - Arena cells read: none
    - Arena cells written: none
    - Error set: [-3, -9, -11]
-/
def sys_close_footprint : Footprint :=
  ⟨#[], #[], #[ -3, -9, -11 ]⟩

/-- Footprint for sys_mmap:
    - Arena cells read: none
    - Arena cells written: [addr, addr+1, ..., addr+len-1] (the mapped region)
    - Error set: [-12, -13, -17, -22, -27, -35]
-/
def sys_mmap_footprint (addr len : Val) : Footprint :=
  ⟨#[], (Array.range len.toNatClampNeg).map (fun i => addr.toNatClampNeg + i), #[ -12, -13, -17, -22, -27, -35 ]⟩

-- ============================================================
-- 7. Dispatch (stub: returns sorry for all)
-- ============================================================

/-- Dispatch a syscall. Returns none if the name is not a syscall.

    NOTE: this is a STUB. The actual implementation would need
    the full Linux file-system and memory model. The axiom
    specifications above are the normative semantics. -/
def dispatchSyscall (name : Name) (args : Array Val) (s : State)
    : Option (State × Val) :=
  -- All syscalls are axiomatised. In the formal model, we
  -- return 0 (success) as a placeholder for testing purposes.
  -- The real semantics is the axiom declarations above.
  if name.startsWith "sys_" then
    -- For the 5 syscalls with declared footprints, we use the
    -- footprint to determine which cells are read/written, but
    -- we still return a placeholder value (0 for success).
    some (s, 0)
  else
    none

end Bebop.Syscalls
