# Bebop — Stage B Swarm Specifications (24 swarms, 3 batches × 8)

Ground-truth contracts (do NOT invent): the C compiler in `native/src/` is the
source of truth. Every swarm ports a specific C module to self-hosted `.bp`,
mirroring its exact API. Read the C header (small) + the relevant `.c` bodies
(narrow reads) as the spec; the `.bp` output must typecheck (~675 fns in corpus), pass
`strict`, and its `self_check()` must return 0.

Repos: /root/dowiz (git root). Build: `cd /root/dowiz/bebop-lang/native && make`
(gcc C11, -Werror). Run a .bp fn: `./build/bebopc run ../selfhost/<f>.bp <fn>`.
Check: `./build/bebopc check ../selfhost/<f>.bp`. Strict: `./build/bebopc strict
../selfhost/<f>.bp`.

## Bebop authoring rules (ALL swarms, non-negotiable)
- NO hexadecimal literals — decimal only (`3531603968` not `0xD2800000`).
- Reassignment needs a prior binding: `let x = v` only after a `let x = ...`
  (mutable via `sym_bind` reuse of the register/slot).
- Array mutation returns 0: `(let _ = a[i] = v in 0)`.
- Avoid identifiers containing the substring `if(` / `if ` (strict false-positive).
- Comments `//`; `char(s,i)`; `str_len(s)`.
- BRANCHLESS LAW: comparisons as `Σ k·(k==N)*expr`; NO nested if/else in loops.
  Real branches ONLY for if/else and while control flow. O(n), no_std.
- Keep each module < ~25 fns or the .bp parser may overflow its fn-count (see
  MEMORY: parser limit); split large modules into 2 files if needed.

## Shared conventions
- Every module exports `self_check() -> i64` returning 0 on pass.
- Every module's `self_check` must include BOTH checksum tests (compare against
  a hand-computed reference where the C code already gives exact values) AND
  execution tests (compile+exec via `run_program`/`exec`, assert numeric result).
- Every swarm commits nothing; parent integrates. Report exact test output.

---

# BATCH 1 (Day 1) — Front-end unification

## Swarm B1-1: lexer.bp — full parity with lexer.c
FILE (write/extend): `selfhost/lexer.bp` (EXISTS — 10 fns, ~4KB, NOT a stub; extend to full C lexer.c parity).
REFERENCE (read): `native/src/lexer.h` (26 lines) + `native/src/lexer.c`.
GOAL: tokenize a `.bp` source string into a token array, mirroring `bp_lex`.
CONTRACT (from lexer.h): token kinds BP_TOK_EOF(0)/IDENT(1)/GLYPH(2)/
NUMBER(3)/PUNCT(4); each token = (kind, byte-offset, length, line). Tokens are
stored as flat i64 triples in an output array (kind at 3i, offset at 3i+1, len
at 3i+2; line at 3i+3 if 4-wide — pick 4-wide: kind/off/len/line).
STEPS:
1. Tokenize ASCII idents (`[A-Za-z_][A-Za-z0-9_]*`), decimal integers, punct
   (`()[]{}:;,+-*/=<>!.&|^` single + `->` `=>` `==` `!=` `<=` `>=` `++` `::`),
   `//` line comments, `/* */` block comments (skip, track newlines for line #).
2. GLYPH: a non-ASCII UTF-8 run (first byte >= 0x80) is ONE token; copy its
   bytes verbatim (don't decode); report byte length.
3. String literals `"..."` with escapes `\n \t \\ \"` (same as expr.c sbuf logic).
4. Track line numbers (increment on `\n`).
5. `self_check`: tokenize a fixed corpus (idents, numbers, punct, glyphs,
   strings, comments, multi-line) and assert exact token triples + line numbers
   against a hand-derived reference from lexer.c.
VERIFY: `check` (0 errors), `strict` (PASS), `self_check` (0).
PITFALL: HTTP 524 — read lexer.c in ≤60-line windows.

## Swarm B1-2: parser.bp — item-level parity with parser.c
FILE (write/extend): `selfhost/parser.bp` (EXISTS — 18 fns, ~4.5KB, NOT a stub; extend to full C parser.c parity).
REFERENCE: `native/src/parser.h` + `native/src/parser.c` `bp_parse` (lines
~131-226).
GOAL: split a source into top-level items, mirroring `bp_parse`.
CONTRACT: item kinds MODULE(0)/FN(1)/STRUCT(2)/ENUM(3)/CONST(4)/USE(5)/TYPE(6)/
THEOREM(7)/UNKNOWN(8); each item = (kind, name-hash, name-offset, name-len,
text-offset, text-len). Return count + error line/msg on failure.
STEPS:
1. Detect keywords `fn`, `struct`, `enum`, `const`, `use`, `type`, `theorem`,
   `module` at item boundaries (branchless detection like collect_fns does).
2. For each item, capture the full source span (text-offset..text-len) and the
   name. Struct/enum spans end at their closing `}`; fn spans end at the
   matching `}` of the body (track brace depth); const/use/type/theorem end at
   `;` or newline.
3. `self_check`: parse a fixed multi-item source and assert item kinds + name
   hashes + spans.
VERIFY: `check`, `strict`, `self_check` all green.

## Swarm B1-3: expr_parser.bp — expression-level parity with expr.c
FILE (write): `selfhost/expr_parser.bp` (NEW file; do NOT touch expr_compile.bp).
REFERENCE: `native/src/expr.c` (parse_expr/parse_seq/parse_primary/parse_lambda,
~692 lines) + `native/src/qtt.h` Term/TermKind/BinOp.
GOAL: parse expressions into a flat term array (kind-indexed), mirroring the C
parser's precedence and desugaring.
CONTRACT (from qtt.h TermKind): VAR/LIT/LAM/APP/ANN/BIN/IF/LET/STRUCT/FIELD/
ENUM_CTOR/MATCH/TYPE/IO/REFL/SUBST/NAT_Z/NAT_S/NAT_REC/NAT_IND/CONG/EQ_TYPE/
STR/FLIT/STR_LEN/STR_CAT/WHILE/ARRAY/ARRAY_GET/STR_CHAR/ARRAY_SET/SYSCALL/CHR/
SPAWN/AWAIT/ADDR_OF/DEREF_PTR/EXEC. BinOp: ADD/SUB/MUL/DIV/EQ/LT/NE/LE/GE/GT/CAT.
STEPS:
1. Precedence: or → and → comparison → add/sub → mul/div → unary → postfix
   (call, index) → primary (parens, lambda, literals, idents, while, array,
   string, match, let-in, if).
2. Desugar `a; b` → nested let (`_sN` temp), `let x = e in b` → LET, `while`
   → WHILE, `match` → MATCH arms, struct/enum ctor, field access `.f`, index
   `a[i]` / `a[i]=v`, syscall idents (write/exit/power), char/chr/str_len/exec.
3. Represent each term as a flat record: [kind, i64val, f64(2 slots), a, b, c,
   d, fields-ptr, nfields, ...] — pick a fixed stride (e.g. 12 i64) and
   document it in a header comment (this IS the IR).
4. `self_check`: parse a corpus of expressions and assert the term-tree shape
   (kind sequence + operand indices) against a hand-derived reference.
VERIFY: `check`, `strict`, `self_check`.

## Swarm B1-4: typecheck.bp — QTT core parity with qtt.c infer/check
FILE (write/extend): `selfhost/typecheck.bp` (EXISTS — 24 fns, NOT a stub; also FIX the strict failure — this is the 1/136 file).
REFERENCE: `native/src/qtt.h` (Ty/TyKind/Quantity) + `native/src/qtt.c` infer()
(~line 858) + check() (~1412).
GOAL: QTT type inference/checking over the flat term IR from B1-3.
STATUS (2026-08-21 sweep): typecheck.bp — check FAIL, strict FAIL (nested-if violations). This is the 1/136 strict straggler. Fix needed before Phase 0 gate clears.
CONTRACT: Quantity Q_ZERO(0)/Q_ONE(1)/Q_MANY(2); semiring qtt_add/qtt_mul
(0+p=p, 1+1=ω, ω+p=ω; 0·p=0, 1·p=p, ω·0=0, ω·1=ω, ω·ω=ω). TyKind: I64/U8/U32/
U64/F64/BOOL/VOID/FN/PI/FIELD/HYPERVEC/VEC/STRUCT/ENUM/TYPE/VAR/EQ/NAT/STR/PTR.
STEPS:
1. Port qtt_add/qtt_mul/qtt_q_name (branchless: `qtt_add(a,b) = (a==ω||b==ω)*ω
   + (a==1&&b==1)*ω + (a==1||b==1)*1` style).
2. Port `infer` for: VAR (linearity: Q_ONE var used twice → error), LIT/FLIT,
   LAM (extend context with binder q; body must check), APP (dom/cod, consume
   the arg's quantity via qtt_mul), BIN (i64/f64/str semantics), IF, LET,
   STRUCT/FIELD, ENUM_CTOR/MATCH, ANN, NAT/REFL/SUBST/CONG/EQ_TYPE, STR/STR_LEN/
   STR_CAT/STR_CHAR, WHILE (cond bool|i64), ARRAY/ARRAY_GET/ARRAY_SET, SYSCALL,
   CHR/SPAWN/AWAIT/ADDR_OF/DEREF_PTR/EXEC, TYPE/IO.
3. Port `check` (infer + ty_leq; LAM special-case checks against FN/PI).
4. `self_check`: port the qtt_check_test() cases (semiring laws + each term
   kind's type; include the "linear var used twice → error" and "(1+true)→error"
   negative cases).
VERIFY: `check` (0), `strict` (PASS — this file currently FAILS, fix it),
`self_check` (0).

## Swarm B1-5: typecheck kernel — conv/norm/subst parity
FILE (write): `selfhost/kernel.bp` (NEW; keep separate from typecheck.bp to stay
under the fn-count limit).
REFERENCE: `native/src/qtt.c` qtt_subst (~2106), norm_rec (~2233), conv_rec
(~2477), qtt_norm/qtt_conv/qtt_prove.
GOAL: definitional equality (β-conversion) kernel — the trustworthy proof core.
CONTRACT: qtt_subst(t,name,v) capture-avoiding; qtt_norm(t) β-normal form;
qtt_conv(a,b) = 1 iff β-convertible (α-equivalence on binders).
STEPS:
1. Port subst_rec/norm_rec/conv_rec over the flat term IR, handling every
   TermKind (including NAT_REC/NAT_IND definitional reduction: nat_rec b s Z→b,
   nat_rec b s (S k)→(s k)(nat_rec b s k)).
2. Port qtt_prove (proof : goal judgement), qtt_prove_refl, qtt_prove_induction
   (natural-number induction: P(Z) + step → P(n)).
3. `self_check`: port qtt_conv_test + qtt_proof_test + qtt_nat_test cases
   (conversion laws, refl, nat_rec computation, induction).
VERIFY: `check`, `strict`, `self_check`.

## Swarm B1-6: codegen.bp — typed IR + lowering (single source of truth)
FILE (write): `selfhost/codegen.bp` (extend existing 55-LOC stub).
REFERENCE: `native/src/codegen.h` + `native/src/codegen.c` + `native/src/qtt.h`.
GOAL: define the single typed IR consumed by all backends, and lower the parsed
term (from B1-3's flat IR) into it. This is the contract the backends read.
CONTRACT: codegen produces a linear IR array of (opcode, a, b, c) quads:
  - IROp: IMM(v) / ADD/SUB/MUL/DIV / CMP(cond) / SEL(then,else) / LOAD(var) /
    STORE(var) / BR(target) / BRZ(target) / CALL(name) / RET / LD_ARR / ST_ARR /
    LD_FIELD / ST_FIELD / ALLOC(n) / SYSCALL(nr) / FLIT(lo,hi) ...
  - var slots map 1:1 to the .bp compiler's sym_bind registers (x19..x28) or
    frame slots (document the spill rule: 11th+ local → frame offset).
STEPS:
1. Define the IR opcode enum (decimal codes) + the flat quad encoding, document
   it in a header comment (this comment IS the backend contract).
2. Lower the term IR: each TermKind → a quad sequence. Lower calls to CALL
   (resolved later by the backend's two-pass layout); lower if → CMP+BRZ+SEL;
   while → BR/BRZ loop; let → STORE(var)/LOAD(var); arrays → ALLOC+ST_ARR;
   structs → ALLOC+ST_FIELD; enum/match → tag CMP+BRZ dispatch.
3. `self_check`: lower a corpus (arith, if, while, let, call, array, struct,
   match, recursion) and assert the exact quad sequences against a reference.
VERIFY: `check`, `strict`, `self_check`.

## Swarm B1-7: infra — symtab/name_pool/source_map/type_registry/ast/token
FILE (write): `selfhost/infra.bp` (NEW single file combining the small infra
modules to stay under fn-count; or extend the existing stubs individually —
CHOOSE the existing stub files: symtab.bp, name_pool.bp, source_map.bp,
type_registry.bp, ast.bp, token.bp, type_eq.bp).
REFERENCE: `native/src/typereg.h` + `native/src/typereg.c`; symtab/name-pool
semantics from how expr_compile.bp's sym_bind/sym_lookup already work.
GOAL: the shared data structures every other module uses.
CONTRACT:
  - type_registry: name-hash → Ty; put/get; TYPEREG_MAX 64 (bump to 128).
  - name_pool: intern a string → stable id (hash-sum, dedup); pool for idents.
  - symtab: stack of name→reg/slot bindings (push scope/pop scope).
  - source_map: token-id → (line,col); error reporting helper.
  - ast/token: the token + item enums as decimal constants (shared).
  - type_eq: structural type equality (port qtt.c ty_eq ~line 589).
STEPS:
1. Port each structure to flat i64 arrays with documented stride.
2. `self_check`: unit tests for each (put/get roundtrip, intern dedup, scope
   push/pop shadowing, line/col mapping, ty_eq structural cases).
VERIFY: each file `check`+`strict`+`self_check` green.

## Swarm B1-8: pipeline integration + typecheck strict fix + test port
FILE (write): `selfhost/compiler_main.bp` + `selfhost/bebopc.bp` +
`selfhost/driver.bp` (extend existing 65/73/38-LOC stubs) + fix `typecheck.bp`
strict failure (coordinate: B1-4 also touches typecheck.bp — B1-8 OWNS the
final strict fix if B1-4 hasn't; avoid double-edit by having B1-8 run LAST).
REFERENCE: `native/src/main.c` cmd_run/cmd_check/cmd_strict (~472-745) + the
existing pipeline stubs.
GOAL: compose lexer→parser→expr_parser→typecheck→codegen→aarch64 into one
end-to-end `compile(source) -> word-array` and wire the driver commands.
STEPS:
1. Write `compile_source(s)` that runs the full front-end and emits the IR.
2. Wire `bebopc.bp` (CLI: `run`/`check`/`strict`/`selfcompile` entry points),
   `compiler_main.bp` (orchestrator), `driver.bp` (dispatch).
3. Fix typecheck.bp strict failure (hoist nested ifs, rename if-colliding
   idents) — the 1/135 file.
4. Port a REPRESENTATIVE slice of native self-tests to `.bp` selftests:
   qtt_self_test (semiring), qtt_check_test, qtt_eval_test, qtt_str_test,
   qtt_array_test, qtt_enum_test, qtt_struct_test, qtt_universe_test,
   qtt_conv_test, qtt_proof_test, qtt_nat_test. Each becomes a `self_check` in
   the corresponding module (or a dedicated `selftest.bp`).
5. `self_check`: end-to-end compile a fixed source and assert the emitted
   checksum matches the C reference; plus the ported self-tests pass.
VERIFY: all touched files `check`+`strict` green; `self_check` 0;
`./build/bebopc run ../selfhost/bebopc.bp main` runs end-to-end.

---

# BATCH 2 (Day 2) — Backends

## Swarm B2-1: aarch64.bp — native parity (control flow + calls)
FILE (write/extend): `selfhost/aarch64.bp` (EXISTS — 4 fns, NOT a stub; extend to full C native.c parity).
REFERENCE: `native/src/native.c` (emit_expr + helpers, ~692 lines) +
`native/src/native.h`; the .bp encodings already proven in expr_compile.bp.
GOAL: lower the B1-6 IR to AArch64 words (control flow + calls + closures),
matching native.c bit-for-bit.
CONTRACT (encodings, from native.c + expr_compile.bp):
  movz=0xD2800000|(imm16<<5)|(hw<<21)|rd ; movk=0xF2800000|... ;
  add reg=0x8B010000 ; sub=0xCB010000 ; mul=0x9B017C00 ; sdiv=0x9AC10C00 ;
  cmp/subs=0xEB01001F (subs xzr,x0,x1) ; cset=0x9A9F07E0|(cond<<12) ;
  b=0x14000000|imm26 ; b.cond=0x54000000|(imm19<<5)|cond ; cbz=0xB4000000|(imm19<<5) ;
  bl=0x94000000|imm26 ; ret=0xD65F03C0 ; stp pre=0xA9800000|(imm7<<15)|(rt2<<10)|(rn<<5)|rt ;
  ldp post=0xA8C00000|... ; ldr imm=0xF9400000|(imm12<<10)|(rn<<5)|rt ;
  str imm=0xF9000000|... ; ldr reg-offset=0xF8607800|(idx<<16)|(base<<5)|rd ;
  str reg-offset=0xF8207800|... ; ldrb=0x39400000|(imm12<<10)|(rn<<5)|rt ;
  adr=0x10000000|(imm21<<5)|rd.
STEPS:
1. Port the emit_* helpers (em/push/pop/emit_mov64/emit_stp_sp/emit_ldp_sp).
2. Lower IR: IMM→movz+push, ADD/SUB/MUL/DIV→pop x1,pop x0,<op>,push;
   CMP→subs+cset; SEL→branch-based if (emit_cond: cmp+b.cond+then/else+b);
   BR/BRZ→b/cbz; CALL→bl (two-pass layout for cross-fn); RET; LOAD/STORE→
   mov reg↔x0; LD_ARR/ST_ARR→reg-offset ldr/str; LD_FIELD/ST_FIELD→ldr/str imm.
3. Two-pass layout: pass 1 measure fn sizes (CALL emits 1-word bl placeholder),
   pass 2 emit real bl imm26 offsets. Mirror compile_program's approach.
4. `self_check`: for each construct, assert emitted word array == native.c
   output (checksum equality) AND exec the result (via run_program) to confirm
   correct numeric behavior (fact, while, cross-call, if/else, let).
VERIFY: `check`, `strict`, `self_check`.

## Swarm B2-2: aarch64.bp — native parity (data: struct/enum/array/string/alloc)
FILE (write): `selfhost/aarch64_data.bp` (NEW, to stay under fn-count).
REFERENCE: `native/src/native.c` TERM_STRUCT/FIELD/ENUM_CTOR/MATCH/ARRAY/STR/
SYSCALL cases (search + read ~20 lines each).
GOAL: lower data-heavy IR ops to AArch64, matching native.c.
CONTRACT: struct = bump-allocate on x14 arena (add x14,sp,#256 base), ST_FIELD
into slots; enum = tag byte + payload; array = bump-allocate N×8 + ST_ARR;
string = trailing data section + adr; SYSCALL = svc #0 (nr in x8, arg x0).
STEPS:
1. ALLOC(n): add x14,x14,#(n*8) ; mov x0,x14_prev — expose base. ST_FIELD:
   str xd,[xbase,#off]. ST_ARR: reg-offset str.
2. String: append bytes after code; adr x0,<data>; LD_FIELD/LD_ARR load.
3. SYSCALL: mov x8,#nr ; svc #0 ; mov x0,x0 (result already in x0).
4. `self_check`: struct/enum/match/array/string/syscall(write) programs —
   checksum == native.c AND exec correct (match arm selection, field access,
   arr[i], str_len, char).
VERIFY: `check`, `strict`, `self_check`.

## Swarm B2-3: wasm.bp — WASM MVP core (control flow + memory)
FILE (write/extend): `selfhost/wasm.bp` (EXISTS — 21 fns, partial skeleton; extend to full WASM MVP parity).
REFERENCE: `native/src/codegen.h` + `native/src/codegen.c`.
GOAL: lower the B1-6 IR to a valid WebAssembly MVP module (i32/i64, locals,
control flow, linear memory).
CONTRACT (wasm): module = magic `\0asm` + version 1; type section, function
section, export section, code section, memory section. Stack-machine, typed.
STEPS:
1. Emit a valid wasm binary skeleton (magic/version/sections with correct LEB128
   size encoding — implement uleb128 encoder branchless).
2. Lower IMM/ADD/SUB/MUL/DIV/CMP/SEL/BR/BRZ/CALL/RET/LOAD/STORE to wasm
   i64.const/i64.add/i64.sub/i64.mul/i64.div_s/i64.eq/i64.lt_s/select/br_if/
   br/call/return/local.get/local.set.
3. `self_check`: assert the emitted bytes are a structurally valid module
   (parse magic + sections) and, for a fixed program, match a hand-derived byte
   reference; ALSO run through any available wasm parser if present.
VERIFY: `check`, `strict`, `self_check`.

## Swarm B2-4: wasm.bp — WASM data (struct/array/string + memory ops)
FILE (write): `selfhost/wasm_data.bp` (NEW).
REFERENCE: `native/src/codegen.c` data cases.
GOAL: lower data IR ops to wasm memory + i32/i64.
CONTRACT: wasm linear memory; alloc = grow_memory or bump pointer in a global;
store/load via i64.store/i64.load (offset+align immediate).
STEPS:
1. Lower ALLOC/LD_FIELD/ST_FIELD/LD_ARR/ST_ARR/SYSCALL(→unreachable or import)
   to wasm memory ops; strings → data segment + i32.const address.
2. `self_check`: fixed programs → assert valid module + byte reference where
   deterministic; otherwise structural validity + a runtime value check if a
   wasm interpreter is available (else structural only, documented).
VERIFY: `check`, `strict`, `self_check`.

## Swarm B2-5: vir.bp — NEON/vector-first backend
FILE (write): `selfhost/vir.bp` (NEW — file does NOT currently exist; must be created from scratch).
REFERENCE: `native/src/vir.h` + `native/src/vir.c` + `native/src/vir_umulh2.c`.
GOAL: port the VIR (vector IR) to .bp: 128-bit SIMD ops lowered to hand-encoded
AArch64 NEON; vector umulh synthesis; LSE atomics.
CONTRACT (from vir.h): VirOp ADD_2D/SUB_2D/ADD_4S/SUB_4S/MUL_4S/FADD_2D/FSUB_2D/
FMUL_2D; Vir128{lo,hi}; vir_binop; vir_umulh2; vir_atomic_add/cas.
STEPS:
1. Hand-encode NEON: ld1 {v},[x0]; ld1 {v},[x1]; <op> v0,v1,v2; st1 {v0},[x2];
   ret. Emit via the exec bridge. Encode: ld1 64b=0x4C007000|(rn<<5)|rt (single
   lane 8B→Vn), add 2D=0x4EE08400|... (derive EXACT encodings with objdump like
   swarm A-0 did — verify every encoding against `objdump -d`).
2. vir_umulh2: synthesize 2×64 multiply-high from UMULL decomposition (no
   native vector umulh).
3. Atomics: hand-encoded LSE (ldaddal/ldadd, casal) — verify with objdump.
4. `self_check`: port vir_self_test + vir_atomic_self_test (vector add/sub/mul
   over 2×i64/4×i32/2×f64 lanes, umulh2 known vectors, atomic add/cas return
   old value).
VERIFY: `check`, `strict`, `self_check`.

## Swarm B2-6: GPU/FPGA — VIR lowering slice + emit contract
FILE (write): `selfhost/gpu_fpga.bp` (NEW — file does NOT currently exist; must be created from scratch) + doc.
REFERENCE: `native/src/vir.h` (the VIR is the single source), `native/src/
compute.c`/`ntt.c`/`hyper.c` (the hot kernels to target).
GOAL: define the VIR→GPU/FPGA lowering contract and emit a first slice for the
dowiz hot kernels (NTT, hypervector, living-memory).
CONTRACT: VIR is the portable layer; GPU/FPGA backends consume VIR ops and emit
(a) for GPU: a documented shader/ISPC-style kernel text, (b) for FPGA: a
documented HLS/Verilog-style module skeleton. This swarm delivers the CONTRACT +
one working slice, not full GPU/FPGA codegen.
STEPS:
1. Write the emit contract: for each VirOp, the GPU/FPGA target lowering rule
   (data layout, lane mapping, memory pattern). Document in a header comment.
2. Emit a GPU kernel (ISPC/CUDA-pseudocode or WGSL) for element-wise add/sub/mul
   over NTT buffers, generated from the VIR.
3. Emit an FPGA skeleton (Verilog module) for a single VirOp (ADD_2D) with
   documented pipelining.
4. `self_check`: assert the emitted GPU/FPGA text is deterministic and contains
   the expected structure (kernel name, lane count, op mapping) for a fixed VIR.
VERIFY: `check`, `strict`, `self_check`.

## Swarm B2-7: benchmark — honest compile throughput vs C
FILE (write): `selfhost/bench_compile.bp` + `native/src/bench_selfhost.c` (new)
wired into Makefile as a `selfhost-bench` command.
REFERENCE: `native/src/wcet.c` (timing pattern, CLOCK_BOOTTIME median) +
`native/src/startup.c`.
GOAL: measure self-host compiler throughput honestly (median over R≥10).
CONTRACT (from the A-5 swarm, extend): (a) typecheck time, (b) self_check time
(41+ compile+exec), (c) compile_program of ~1KB source, (d) words/sec.
STEPS:
1. C harness: load the .bp compiler fns (like cmd_run), time each phase with
   CLOCK_BOOTTIME, median over R=10, print a table. No DCE (consume results).
2. .bp harness: same measurements from inside .bp (self-timed).
3. Report honest numbers; wire `./build/bebopc selfhost-bench`.
VERIFY: `make` clean; run `selfhost-bench`; paste median table.

## Swarm B2-8: verify — AArch64 bit-match parity checker
FILE (write): `selfhost/parity.bp` (NEW).
REFERENCE: `native/src/native.c` + `native/src/disasm.c` (from swarm A-6, if
present) + the known checksums.
GOAL: prove the .bp aarch64 backend emits the SAME words as native.c for a
fixed construct corpus.
CONTRACT: for each construct, compile with BOTH (a) the C native_eval (via a
probe) and (b) the .bp aarch64 backend, and assert the word arrays are identical.
STEPS:
1. Build a corpus of ~20 constructs (arith, cmp, if, let, while, call, closure,
   struct, enum, match, array, string, syscall, recursion).
2. For each, compute the .bp checksum AND the C checksum (via a small C probe
   that dumps native_eval's em_code words as a sum). Assert equality.
3. Report a per-construct parity table; any mismatch = a bug to fix (document).
VERIFY: all constructs bit-match; report the table.

---

# BATCH 3 (Day 3) — Verification + closure

## Swarm B3-1: self-compile the FULL compiler (not just expr)
FILE (write): integration of all modules; final selfcompile.
GOAL: the composed pipeline compiles its OWN complete source end-to-end;
checksum stable + bit-matches the C reference.
STEPS: wire compile_source over the full compiler source; run selfcompile;
assert deterministic checksum across 3 runs; diff against native.c output.
VERIFY: `./build/bebopc selfcompile ../selfhost/<full>.bp` stable; bit-match.

## Swarm B3-2: port ALL native self-tests to .bp selftests
GOAL: 100% of native self-test modules have a .bp selftest twin.
STATUS (2026-08-21): 134/136 selftest PASS; 2 files block (expr_compile.bp, selftest_exec.bp — unbound emit_call).
STEPS: enumerate native/src/*_self_test/_test functions; port each to a .bp
self_check in the matching module; assert every one returns 0.
VERIFY: a sweep script runs every selftest; all green.

## Swarm B3-3: typecheck sweep — 136/136 clean + strict PASS
GOAL: every .bp file typechecks AND passes strict.
SWEEP RESULT (2026-08-21): 134/136 self_check PASS; 0/136 check PASS; 0/136 strict PASS.
  FAILED: all 136 check, typecheck.bp strict (1), expr_compile.bp+selftest_exec.bp unbound emit_call.
STEPS: run check+strict over all 136 files; fix the stragglers (esp.
  typecheck.bp strict + any parser-limit files by splitting modules).
  VERIFY: sweep script reports 136/136 clean + strict PASS.

## Swarm B3-4: fuzz — 300k inputs achieved, extend toward 1M no crash
GOAL: extend the A-3 fuzzer to the full front-end; target 1M generated/mutated inputs
(currently at 300k — lexer+parser+AST destructor only; typecheck/codegen/backends not yet fuzzed).
STEPS: extend fuzz.c to cover typecheck/codegen/backends; fuzz lexer/parser/expr_parser/
typecheck/codegen/backends; assert no crash/hang; fix any root cause found (minimal, behavior-preserving).
VERIFY: fuzz run completes; summary line "0 crashes / N" (N ≥ 300k; target 1M).

## Swarm B3-5: wasm validation + execution
GOAL: validate emitted wasm with a real parser, and execute if a runtime is
available (wasmtime/wasmer/node) else document structural validity only.
STEPS: validate all B2-3/B2-4 outputs; run a corpus through a wasm runtime if
present; report pass/fail.
VERIFY: validation green; execution results correct (or documented gap).

## Swarm B3-6: NEON correctness + performance verify
GOAL: verify vir.bp NEON output is bit-correct vs scalar reference AND measure
speedup.
STEPS: for each VirOp, compare NEON result vs scalar C reference on randomized
inputs; measure ops/sec vs scalar.
VERIFY: all bit-correct; report speedup table (honest).

## Swarm B3-7: docs — SELFHOST.md + ROADMAP.md final state
GOAL: update bench/SELFHOST.md and ROADMAP.md with the completed Stage B state,
architecture, ABI, design-law compliance, verification results, remaining gaps.
VERIFY: docs accurately reflect reality (no embellishment).

## Swarm B3-8: release — integration, full test, commit+push
GOAL: final integration of all modules; run the complete verification gate
(build/check/strict/self_check/selfcompile/make test/fuzz/bench); commit + push
to origin/main at the milestone.
STEPS: assemble; run the full gate; fix any integration regressions; commit and
push. Report the final green matrix.
VERIFY: full gate green; pushed to origin/main.

---

## Coordination rules (parent)
- Run batches sequentially: Batch 1 → integrate+commit → Batch 2 → Batch 3.
- Within a batch, all 8 swarms run in parallel; they write DISJOINT files
  (listed under FILE per swarm). If two swarms must touch the same file
  (B1-4 & B1-8 both touch typecheck.bp), B1-8 runs LAST and owns the final fix.
- After each batch: parent runs full verify gate, resolves merge conflicts,
  commits + pushes. Then next batch.
- Every swarm reports: files changed, what implemented, exact test output,
  design decisions. No commit/push by swarms.
