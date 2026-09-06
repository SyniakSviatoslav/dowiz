Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175 with the A1 worker tree; depends on A1 (facts fw per fn), A2 step 0 (fntab relayout, fn cap 512, literal table 5000+); independent of A5-A9 (it memoises whatever the emitter produces)

# A10 Per-fn memo (Salsa-like): a one-fn edit recompiles one fn

## 0. Goal

The compiler keeps, next to the whole-output `.becache` (bebop.bp cache_hit/cache_write, verified: the cache is keyed by the compiler's own bytes + the full source bytes, and is all-or-nothing), a per-fn memo file `<out>.fnmemo` mapping `hash(fn text) -> (fw facts, word count, words with relocations)`; on a rebuild only fns whose text hash changed are re-emitted, the layout is recomputed from the memoised sizes, and `bl` targets / literal offsets are re-linked. Gate: **one-fn-edit self-compile <= 0.3 s** (today 1.5 s cold, 0.07 s whole-output hit -- docs/PERF.md selfcompile_wall, RESEARCH-DEPS §5.4), **the fixpoint md5 is unchanged** (memo hit and miss produce byte-identical output: gen3 == gen4 with the memo on AND off), zero dependencies (all in bebop.bp).

## 1. Scope

In: compile_program_offs (bebop.bp:~4449, verified: planning pass sizes -> prefix sums -> emission pass with the 3-zone fntab), cli_compile (the .becache path), a memo file format, the relocation list per fn (positions of `bl` words -- absolute-target words -- and of literal `adr`/handle constants), the invalidation key (fn text hash + the compiler's own digest + the facts of CALLEES? no: `bl` is re-linked by position, so a callee's change does not invalidate the caller; but a callee's PARAM COUNT change does change the caller's words (arg placement) -- the caller's text did not change... the caller's words depend on: its text, its callees' arities (parse_params of the callee at the call site: emit_call resolves via find_fn/ft_cache -- bebop.bp:emit_call verified in session context) and the enum/struct declarations (ctor tags, field indices), and the builtin table. Key = hash(fn text) XOR hash(global declaration text: every `fn NAME(params)` signature line, every enum/struct block) -- the "signature digest" -- so any signature or declaration change invalidates everything (rare), a body edit invalidates one fn). Out: memoising the planning pass separately (the memo stores the final words; the planning pass runs only for missed fns); cross-file memo; a memo for the seed stub.

## 2. Preconditions

A2 step 0 (fn cap 512, relayout); A1's facts word per fn (fw, bebop.bp:4242 verified) published by the planning pass -- the memo stores fw so the emission pass can size prologues without a planning pass for hit fns; `.becache` semantics understood (cache_hit bebop.bp:4447-4500 area verified: reads compiler bytes + source bytes and compares).

## 3. Design

**What a fn's words depend on** (audit, step 0): its own text; the layout (start offsets of callees -> `bl` imm26; its own start -> nothing else: branches are relative, literals are `adr` PC-relative (pre-A7) or handle constants (post-A7: absolute byte offsets in the literal region, which depend on the literal ORDER = source order of literals across ALL fns -> a literal inserted in an earlier fn shifts every later fn's handle constants: treat literal handles as relocations too: the memo stores (word index, literal ordinal) and re-links the movz/movk chain -- pre-A7 `adr` likewise (word index, literal ordinal -> imm21 from the new data offset)); callee signatures (arity, `use` expansion order for find_fn); ctor tags / struct field indices; the compiler's own bytes (any emitter change = full miss: include the compiler digest in the memo header, like .becache).

**Memo file** `<out>.fnmemo`: header {compiler digest (the same bytes hash .becache uses), signature digest, count}; per fn {text hash (the 131-rolling hash over the fn's source span, 64-bit, plus the span length), fw, nwords, nrelocs, words[nwords], relocs[nrelocs] as (word index, kind, target: callee fn-name hash | literal ordinal)}. Written after a successful compile (all fns), read at the start of the next.

**Compile with memo** (compile_program_offs): collect_fns as today; for each fn compute the text hash; if the header digests match and the fn's hash is present: size = memoised nwords, facts = memoised fw, skip BOTH passes for it; else run the planning pass (as today) to get size+facts. Prefix sums -> starts. Emission: hit fns are copied from the memo into insns at their start and their relocs patched (`bl`: imm26 = target_start - (start + idx); literal words: recomputed from the new literal table); missed fns are emitted as today and their words + relocs recorded (the emitter must REPORT relocations: `emit_bl` (bebop.bp:552) appends (n[0], callee hash) to a per-fn reloc list -- a fntab zone `fntab[3400 + 2k]`, k < 100 (verify free after A2 step 0; else a `zeros` list per fn), `emit_str` appends (n[0], literal ordinal)). Then the literal section and the stub as today. The output is byte-identical to a full compile by construction (same words, same layout); the gate proves it.

**Invalidation edge cases:** a fn ADDED or REMOVED changes the fn table but not other fns' words (calls resolve by name); a fn RENAMED = removed + added (callers now unresolved -> they miss because... their text did not change! -- their words change (brk #87 path vs bl). Include in the signature digest the SET of fn names -> any add/remove/rename invalidates all. Fine: signature-level edits are rare; body edits are the loop. `use` expansion: the source seen by the compiler is the expanded text (use_scan/use_expand bebop.bp), so the hash is over expanded text -- consistent.

**Timing target.** Today per fn: planning + emission passes over 250 fns = 1.5 s -> ~6 ms/fn; a one-fn edit = 2 passes of one fn (~6 ms) + hashing 230 KB of source (~2-5 ms: the read_ident-style rolling hash over the whole file; `char` at 3 words after A7 -- use `crc32b`/crc32x over the fn span: 8 B/cycle = 0.1 ms) + memo read (~1 MB file via sys_mmap: 1-2 ms) + copy + relink (~1 ms) + literal/stub/write (~5 ms) + .becache write (it also rewrites the whole cache: 0.1 s today? measure; skip the .becache write when the memo hit rate is > 0: the memo supersedes it for hit-ability -- decision: keep .becache (it is the gate memo's key) but write it only when the memo missed everything). Expected 0.05-0.15 s.

**Invariants.** Memo on/off produce identical bytes (gate); the memo never contains words of a fn that failed to compile; a corrupted memo (crc32x over the file, stored in the header) is ignored, never trusted; the fn cap and the reloc zone sizes are loud traps (exit 89).

## 4. Files and functions touched

| file:fn | change | anchor |
|---|---|---|
| bebop.bp:compile_program_offs | memo lookup, skip passes for hits, copy+relink | ~4449 (grep `^fn compile_program_offs`) |
| bebop.bp:emit_bl / emit_str / (post-A7 handle emit) | reloc recording | 552, 2933 |
| bebop.bp:cli_compile | read/write `<out>.fnmemo`, crc32x header, .becache interplay | cli_compile (grep) |
| bebop.bp: new fnmemo_read / fnmemo_write / fn_text_hash / sig_digest | ~5 fns | new |
| tools/chain.sh | a `--memo-off` env (BEBOP_FNMEMO=0) for the identity gate; perf row `selfcompile_edit_wall` (one-fn edit: append a comment inside one fn body, recompile, time) | chain.sh, tools/perf.py |
| docs/DEV-LOOP.md | the new loop | -- |

## 5. Steps

0. Dependency audit (which words of a fn depend on what) written into the journal; measure today's per-phase times (planning, emission, literal/stub, .becache write) with clock_ms prints in a scratch build.
1. Reloc recording + memo write (no reads yet): chain without `--codegen` byte-identical; `.fnmemo` produced; a python checker `tools/fnmemo_check.py` verifies that patching the memo's words with the recorded relocs reproduces the .bin exactly (pure python, no bebop).
2. Memo read + skip + relink: chain byte-identical with memo ON (default) and OFF (`BEBOP_FNMEMO=0`); gate `selfcompile_edit_wall`.
Leave uncommitted for the main session.

## 6. Constructs, oracles, twins

No constructs (no codegen change). Oracle = byte identity: `md5sum` of gen2 built with memo cold, warm (second run, all hits), and after a one-fn edit (one miss) must equal the memo-off build of the same source. tools/fnmemo_check.py as the structural oracle.

## 7. Gates

- `PROC_CAP=30 BEBOP_TMP=$OUT tools/chain.sh bebop.bp $OUT` (no codegen): gen3 == gen4; `BEBOP_FNMEMO=0` run gives the same md5.
- `selfcompile_edit_wall` <= 300 ms (docs/PERF.md new row; method: edit = add `// x` inside `fn em`'s body, recompile with a warm memo, median of 5); `selfcompile_wall` cold unchanged within 5 % (memo write cost).
- RED: any md5 difference between memo on/off = a missed dependency (probe: diff the .bins with objdump -> the fn whose words differ names the dependency).

## 8. Risks and probes

| risk | probe | symptom |
|---|---|---|
| a word depends on something not in the key (ctor tag, field index, builtin table, `use` order) | edit an enum in a scratch copy, rebuild, md5 vs memo-off | stale words |
| reloc zone overflow (a fn with > 100 calls) | `grep -c 'bl' per fn` in objdump; trap 89 | exit 89 |
| memo file torn by a crash | crc32x header; ignore on mismatch | full recompile (safe) |
| fn text hash collision | 64-bit + length; add the fn's start offset to the key? no -- offset changes on any edit above; length + hash is enough | -- |
| the whole-output .becache now rarely hits (it still keys on full source) | keep it for the gate memo; measure its write cost; skip write on partial-hit builds | K5 cold +5 % |

## 9. VERDICT format

```
VERDICT: GREEN|RED
phase times today: planning <ms> emission <ms> literals+stub <ms> becache <ms>
step1: byte-identical <md5>; fnmemo_check: ok
step2: memo on/off md5 equal: yes; cold <ms> warm <ms> one-fn-edit <ms> (gate 300)
journal: <lines>
open: <anything>
```

## 10. Worker prompt skeleton

<context> repo, this blueprint, compile_program_offs / cache_hit / emit_bl / emit_str read first, A2 step 0 layout, harness commands. </context>
<constraints> zero codegen change (byte identity is the gate); python checker before the read path; leave uncommitted. </constraints>
<output_format> §9. </output_format>
<task> A10 steps 0-2; report. </task>
