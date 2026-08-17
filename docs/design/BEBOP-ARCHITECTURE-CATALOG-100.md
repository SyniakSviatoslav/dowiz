# Bebop Architecture Catalog — 100 Design Decisions (10 domains × 10)

> Status: living document. Each decision is immutable once ratified.
> Format: `D#N. Decision` — `Rationale` — `Tradeoffs`

---

## Domain 1: QTT Core Calculus

1. **Single calculus (QTT).** One typed calculus delivers ownership (Rust) + dependent types (Lean). Rationale: avoids two type systems, two compilers. Tradeoff: steep learning curve.

2. **Quantities 0/1/ω.** 1=move/linear, ω=shared, 0=erased/proof. Rationale: Idris 2 proved this is enough. Tradeoff: no fractional quantities (linear logic purism rejected).

3. **No η-conversion in kernel.** Only βδ-reduction. Rationale: keeps normalizer simple, proofs smaller. Tradeoff: some definitional equalities need explicit `refl`.

4. **δ-reduction for primitives.** Strings, Nat, Vec all compute definitionally. Rationale: `str_len("hello") ≡ 5` without induction. Tradeoff: normalizer grows with primitive count.

5. **Cumulative universes postponed.** Type₀ only for bootstrap. Rationale: don't need Type₁ for self-host. Tradeoff: can't encode `Type : Type` paradoxes (which is good).

6. **Inductive types via W-types.** All ADTs desugar to W. Rationale: single eliminator (`nat_rec`/`nat_ind`) pattern. Tradeoff: encoding is verbose.

7. **No quotient types yet.** Postponed to QTT-full phase. Rationale: self-host doesn't need them. Tradeoff: can't model `ℤ` from `ℕ × ℕ / ~`.

8. **Termination checker postponed.** Recursion guarded by `nat_rec` only. Rationale: structural recursion sufficient for bootstrap. Tradeoff: no general `fix`.

9. **No implicit arguments.** All args explicit. Rationale: simpler elaboration. Tradeoff: more typing in source.

10. **Unification: first-order only.** Pattern unification (Miller). Rationale: decidable, predictable. Tradeoff: can't infer higher-order patterns.

---

## Domain 2: Type System

11. **No subtyping.** All type equality is definitional. Rationale: simplifies kernel. Tradeoff: no implicit upcasting.

12. **Type reflection.** `type_size`/`type_align` at elaboration time. Rationale: layout computed once. Tradeoff: can't reflect at runtime.

13. **Nat as machine word (erased).** Peano in proofs, u64 at runtime. Rationale: zero overhead. Tradeoff: large Nat proofs may overflow u64.

14. **String as immutable byte sequence.** Borrowed pointer, never owned. Rationale: no allocation in kernel. Tradeoff: can't mutate strings.

15. **Array = Vec<T,n>** (fixed size). Rationale: predictable layout. Tradeoff: no growable arrays.

16. **Struct = named-product.** Fields accessed by name. Rationale: self-documenting. Tradeoff: no structural subtyping.

17. **Enum = tagged-union.** u8 tag + payload. Rationale: compact. Tradeoff: max 256 constructors.

18. **Proof terms erased (QTT 0).** `refl`, `nat_ind` compiled to no-ops. Rationale: zero runtime cost. Tradeoff: can't reflect on proofs.

19. **No `null` or `Option` built-in.** Use `enum {None, Some(T)}`. Rationale: explicit, safe. Tradeoff: extra indirection.

20. **I64 as default integer.** No implicit conversions. Rationale: predictable. Tradeoff: verbose for u8 arithmetic.

---

## Domain 3: Parser + Lexer

21. **Recursive-descent parser.** No parser generators. Rationale: debuggable, no dependencies. Tradeoff: manual precedence handling.

22. **Glyphs as UTF-8 identifiers.** Non-ASCII treated as `BP_TOK_GLYPH`. Rationale: source can be pure glyphs. Tradeoff: ASCII fallback always available.

23. **Keywords via `match_kw`.** Not lexer tokens — parser discriminates. Rationale: can add keywords without changing lexer. Tradeoff: slightly slower.

24. **Error recovery: fail-fast.** No error recovery in bootstrap. Rationale: simple. Tradeoff: one error per compilation.

25. **Expression parser: Pratt.** Precedence via `parse_bin(min_prec)`. Rationale: handles all binary ops uniformly. Tradeoff: less flexible for mixfix.

26. **Let-in chains via recursive parsing.** `let x=e1 in e2` desugars to `(λx.e2) e1`. Rationale: no special AST node. Tradeoff: can't typecheck let-polymorphism.

27. **If-then-else via CSEL.** No branch divergence in JIT. Rationale: branchless for hot paths. Tradeoff: evaluates both branches always.

28. **While via actual branches.** cbz + b for loops. Rationale: can't unroll infinite loops. Tradeoff: pipeline bubbles on mispredict.

29. **Array indexing via CSEL-chain.** Immediate-offset loads + cmp+csel. Rationale: avoids broken register-offset ldr. Tradeoff: O(n) code size per access.

30. **No module system in bootstrap.** Flat namespace. Rationale: self-host doesn't need modules. Tradeoff: name collisions possible.

---

## Domain 4: Native Codegen (AArch64)

31. **Direct AArch64 encoding.** No assembler, hardcoded opcodes. Rationale: zero dependencies. Tradeoff: fragile, hand-verified.

32. **W^X memory.** Write → mprotect → execute. Rationale: no W+X pages ever. Tradeoff: can't JIT-patch running code.

33. **Register allocation: x19-x28.** 10 callee-saved for locals. Rationale: survives calls. Tradeoff: only 10 locals in registers.

34. **Heap: bump pointer in x14.** sp+256 as heap base, x14 tracks bump. Rationale: simple, fast. Tradeoff: no GC, no free.

35. **Eval stack: push/pop via sp.** sub sp,#16 / add sp,#16. Rationale: uses hardware stack. Tradeoff: stack and heap share 512B frame.

36. **Frame: 512 bytes.** sp-512 for JIT execution. Rationale: enough for small programs. Tradeoff: large programs overflow.

37. **CSEL for branchless if.** Both branches evaluated, result selected. Rationale: no branch mispredict. Tradeoff: wasted work for expensive branches.

38. **cbz/b for while.** Label-based backward branch. Rationale: minimal encoding. Tradeoff: 19-bit branch range.

39. **svc #0 for syscalls.** TERM_SYSCALL with ival=nr. Rationale: direct Linux syscall. Tradeoff: Linux-only.

40. **mov-imm for constants.** 16-bit immediate with mov/movk. Rationale: handles any u16. Tradeoff: multi-instruction for u64.

---

## Domain 5: Proof Kernel

41. **`refl` as sole equality constructor.** All proofs reduce to `refl`. Rationale: simple, trusted. Tradeoff: no `sym`/`trans` needed in kernel.

42. **`nat_ind` for induction.** Dependent eliminator. Rationale: one rule for all Nat proofs. Tradeoff: manual motive construction.

43. **No `cong`/`subst` axioms.** Derived from `nat_ind`. Rationale: minimal trusted base. Tradeoff: larger proof terms.

44. **Theorem surface via `theorem` keyword.** Parsed, elaborated, checked. Rationale: concrete syntax for proofs. Tradeoff: limited expressivity.

45. **`ty_eq` with convertibility check.** `a = b` true iff `qtt_conv(a,b)`. Rationale: definitional equality. Tradeoff: no propositional equality without proof.

46. **Type pool bounded.** 64 type allocations max. Rationale: predictable memory. Tradeoff: complex types exhaust pool.

47. **No tactics.** All proofs explicit terms. Rationale: bootstrap is small. Tradeoff: writing proofs by hand.

48. **`extern pure`/`extern io` effect tracking.** Call-graph analysis. Rationale: purity guarantees. Tradeoff: all externs must be annotated.

49. **Proof erasure.** QTT 0 terms emitted as no-ops. Rationale: zero runtime cost. Tradeoff: can't `printf`-debug proofs.

50. **Self-test macro.** All modules have `_test` functions. Rationale: continuous verification. Tradeoff: test code in production binary.

---

## Domain 6: Effect System

51. **Two effects: pure/io.** No effect lattice. Rationale: bootstrap needs only these. Tradeoff: no `log`/`network` tracking.

52. **Transitive effect propagation.** Caller inherits callee's effect. Rationale: sound. Tradeoff: false positives (unused callee).

53. **`extern` declarations.** Must specify `pure` or `io`. Rationale: explicit, grepable. Tradeoff: verbose.

54. **No effect polymorphism.** Functions are pure OR io, not parametric. Rationale: simpler typechecking. Tradeoff: code duplication.

55. **No effect handlers.** Bootstrap doesn't need them. Rationale: less is more. Tradeoff: can't implement custom effects.

---

## Domain 7: Memory Model

56. **Arena-based allocation.** CoW log for time-travel. Rationale: predictable, no GC. Tradeoff: manual lifetime management.

57. **Append-only CoW log.** O(1) snapshot/rollback. Rationale: supervision tree needs it. Tradeoff: unbounded log growth.

58. **No heap in kernel.** All kernel terms are stack/static. Rationale: no allocation in typechecker. Tradeoff: limited term count.

59. **Borrowed pointers for strings.** Never owned. Rationale: no allocation. Tradeoff: lifetime tied to source.

60. **No `Box`/`Rc`/`Arc`.** Bootstrap doesn't need heap types. Rationale: simplicity. Tradeoff: can't write general data structures.

---

## Domain 8: Concurrency

61. **Green threads (stackful coroutines).** Yield via `gt_yield()`. Rationale: cooperative, no OS threads. Tradeoff: no preemption.

62. **Round-robin scheduler.** Fixed order, no priorities. Rationale: simple. Tradeoff: no fairness guarantees.

63. **No channels yet.** Postponed to post-bootstrap. Rationale: self-host doesn't need them. Tradeoff: threads can't communicate.

64. **Atomic intrinsics (CAS/LDADD).** Via machine-code emission. Rationale: lock-free data structures. Tradeoff: AArch64 only.

65. **No `async`/`await`.** Green threads simpler. Rationale: less syntax. Tradeoff: can't compose with OS I/O.

---

## Domain 9: Tooling

66. **No debugger.** GDB can attach to JIT code. Rationale: works today. Tradeoff: no source-level debugging.

67. **No profiler.** `perf` can sample JIT code. Rationale: works today. Tradeoff: no flame graphs from Bebop.

68. **No LSP (yet).** Postponed to Phase 2. Rationale: self-host first. Tradeoff: no IDE support.

69. **Error messages: single-line.** One error per compilation. Rationale: simple. Tradeoff: multiple errors need multiple passes.

70. **No formatter.** `bebopc fmt` planned but not built. Rationale: glyphs need special handling. Tradeoff: inconsistent style.

---

## Domain 10: Self-Host Philosophy

71. **C bootstrap → self-host in Bebop.** C compiler compiles Bebop compiler. Rationale: bootstrapping trust. Tradeoff: C code exists forever.

72. **No external compilers.** Own AArch64 backend, no LLVM/GCC. Rationale: full control. Tradeoff: no x86_64 yet.

73. **All optimizations mandatory.** No `-O0`. Rationale: `#[bit_identical]` requires consistent opts. Tradeoff: slow debug builds.

74. **Self-test on every commit.** All modules have `_test` in `make test`. Rationale: never regress. Tradeoff: longer CI.

75. **No unsafe blocks.** All code is safe by construction. Rationale: kernel is the escape hatch. Tradeoff: some expressivity lost.

76. **Line-count budgets.** Modules ≤500 lines each. Rationale: human-auditable. Tradeoff: more files.

77. **Zero dead code.** Every line reachable from `main`. Rationale: no cruft. Tradeoff: aggressive pruning.

78. **No dynamic dispatch.** All calls static. Rationale: predictable. Tradeoff: no trait objects.

79. **Source = truth.** No generated code committed. Rationale: auditable. Tradeoff: verbose.

80. **One binary.** `bebopc` does everything. Rationale: simple deployment. Tradeoff: large binary.

---

## Remaining 20 (abbreviated)

81-90: **Security**
- PAC (pointer auth), W^X, CoW rollback, fail-closed syscalls, no `unsafe`, bounded loops, stack canaries (via PAC), no format strings, deterministic memory

91-100: **Future**
- WASM backend, GPU backend, FPGA/ASIC HLS, x86_64 backend, LSP, formatter, module system, package manager, stdlib, dowiz rewrite
