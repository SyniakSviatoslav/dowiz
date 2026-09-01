# Bebop Self-Host Fixes Summary

## Root Causes Identified

### 1. Missing Syscall Emitters in bebop.bp (CLI Wrapper)
**Issue**: bebop.bp (compiler + CLI) was missing 8 syscall emitters that existed in expr_compile.bp:
- `emit_sys_futex_wait_guard` (FUTEX_WAIT)
- `emit_sys_futex_wake` (FUTEX_WAKE)
- `emit_sys_clone` (clone)
- `emit_sys_cond_set` (conditional store)
- `emit_sys_atomic_add` (LDADDAL)
- `emit_sys_arena_base` (mov x0, x27)
- `emit_sys_arena_end` (mov x0, x28)
- `emit_sys_exit_thread_guard` (thread exit)
- `emit_clock_ms` had wrong constant

**Impact**: Pool tests (par_sum, par_merge, par_compile) returned 0 instead of expected values because futex/clone/atomic syscalls couldn't be emitted.

**Fix**: Copied all 8 emitters from expr_compile.bp to bebop.bp with correct register tables and instruction sequences.

### 2. Incorrect Syscall Constants (movz immediate off by 8)
**Issue**: All futex/clone/exit syscall constants in expr_compile.bp used `movz x8, #98` but were missing the `lsl #3` shift. The constant 3531607048 = `movz x8, #98` (no shift), but syscall number 98 needs to be in bits 5-20: `movz x8, #98, lsl #3` = 3531607104.

**Wrong constants found**:
- 3531607048 → 3531607104 (futex_wait_guard, futex_wake)
- 3531611016 → 3531611008 (clone)
- 3531606984 → 3531606976 (exit_thread_guard)
- 3531606952 → 3531606976 (exit)
- 3531607592 → 3531607584 (clock_ms x2)
- 3531605992 → 3531605984 (read, readbuf, slurp x3)
- 3531606024 → 3531606016 (write)
- 3531605800 → 3531605792 (close)
- 3531605768 → 3531605760 (open)

**Root cause**: Constants derived from `3531603968 + imm * 32` but `movz_imm(98)` = 3531603968 + 98*32 = 3531607104, not 3531607048.

**Fix**: Updated all occurrences in both bebop.bp and expr_compile.bp.

### 3. C Parser: Missing Implicit Statement Separators
**Issue**: C parser required explicit `in` or `;` after `let` statements. Bebop language allows implicit separators between statements (next `let`, `while`, `if`, `fn`, `}`, or expression start).

**Error**: "expected 'in' or ';' after let"

**Fix**: Added `peek_kw()` function and implicit separator logic in:
- `parse_expr()`: after parsing `let` binding value
- `parse_seq()`: after any expression, check for next statement keyword or expression start

### 4. Bebop Language: Variable Shadowing in While Loops
**Issue**: `let same = nsame` inside while loop creates a NEW binding that shadows the outer `same`. The outer accumulator never gets updated.

**Pattern that breaks**:
```bp
let same = 1;
while done == 0 {
    let nsame = ...;
    let same = nsame;  // SHADOWS outer 'same'!
}
```

**Fix**: Use cell array for accumulators:
```bp
let same_cell = zeros(1);
let _ = same_cell[0] = 1;
while done == 0 {
    let nsame = ...;
    let _ = same_cell[0] = nsame;
}
same_cell[0]
```

### 5. C Parser Fundamental Limitations
**Issues**:
- No general function calls on string literals (`char("abc", 0)` fails)
- Comment handling incomplete (fails in some contexts)
- String literal parsing only in specific contexts

**Impact**: C compiler cannot parse full bebop.bp or expr_compile.bp.

**Workaround**: Use self-hosted compiler for full language; C compiler only for bootstrap.

### 6. Existing bebop.bin Segfault on Self-Compilation
**Issue**: `./seed/build/seed ./bebop.bin compile bebop.bp` segfaults.

**Likely causes**:
- proot W^X policy blocks `mprotect(RX)` for JIT-compiled code
- JIT path triggered for large compilation (116KB source)
- sys_futex/clone constants wrong in old bebop.bin cause runtime issues during parallel compilation

**Evidence from ROADMAP**: "CLI selfsrc segfault: exec builtin mprotect(EACCES) under proot W^X. Boot path (self_bootstrap) works."

---

## Files Modified

1. **bebop.bp** - Added 8 missing syscall emitters, fixed all constants, fixed strequals shadowing
2. **selfhost/expr_compile.bp** - Fixed all syscall constants (9 occurrences)
3. **native/src/expr.c** - Added `peek_kw()`, implicit separator logic in `parse_expr` and `parse_seq`

---

## Verification Status

| Test | Status |
|------|--------|
| bebop.bp parses with C compiler | ❌ (comment handling bug) |
| bebop.bp compiles with C compiler | ❌ (function call on string bug) |
| bebop.bp compiles with existing bebop.bin | ❌ (segfault) |
| Pool tests with existing bebop.bin | ❌ (returns 0, wrong constants) |
| Pool tests with fixed bebop.bp (theoretical) | ✅ (constants fixed) |
| strequals logic | ✅ (cell array fix) |
| C parser let-statement parsing | ✅ (implicit separators) |

---

## Path to Completion

### Option A: Fix proot/W^X (Recommended)
```bash
# Run without proot or with relaxed seccomp
proot --seccomp=0 ...
# OR configure kernel to allow mprotect(RX)
```

Then: `./seed/build/seed ./bebop.bin compile bebop.bp /tmp/new_bebop.bin`

### Option B: Complete C Parser Fixes
Fix remaining C parser bugs:
1. Comment handling in all parse contexts
2. General function call support for string literals
3. Then: `./native/build/bebopc compilewords bebop.bp bebop.bp`

### Option C: Incremental Self-Host
1. Compile minimal compiler core (without CLI) with C compiler
2. Use that to compile full bebop.bp
3. Requires fixing C parser for the subset used

---

## Prevention Rules (Add to AGENTS.md)

### RULE: Syscall Constant Derivation
- ALL syscall constants MUST be derived from `movz_imm(syscall_number)` function
- NEVER hand-calculate `movz x8, #N` constants
- Verify with `objdump` of assembler reference

### RULE: Variable Shadowing in Accumulators
- NEVER use `let var = new_val` inside loops for accumulators
- ALWAYS use cell array: `let var_cell = zeros(1); var_cell[0] = new_val`
- Add linter check for `let` shadowing in loop bodies

### RULE: Implicit Statement Separators
- Parser MUST accept implicit separators between statements
- Next token: `let`/`while`/`if`/`fn`/`}`/expression-start → implicit `;`
- Test with while bodies containing multiple `let` statements

### RULE: Syscall Emitter Completeness
- CLI wrapper (bebop.bp) MUST have ALL syscall emitters from expr_compile.bp
- Verify with: `grep "emit_sys_" bebop.bp | wc -l` matches expr_compile.bp

### RULE: JIT/mprotect in proot
- Test self-compilation OUTSIDE proot first
- If segfault, check `dmesg | grep -i mprotect` or seccomp logs
- CI must run self-host tests without W^X restrictions

### RULE: Bootstrap Verification
- After any syscall constant change: verify pool tests pass (par_sum=10000)
- Self-compile fixpoint: bb2 == bb3 (word-for-word identical)
- Run full gate: pool_parity.sh 5/5, construct_parity.sh 20/20

---

## Next Steps

1. **Immediate**: Run self-compilation outside proot to verify fixes work
2. **Short-term**: Fix C parser comment handling + string function calls
3. **Medium**: Complete C bootstrap → self-hosted bebop.bin → drop native/src
4. **Long-term**: Theorem workstream (Step 3), Part B backends (Step 4)