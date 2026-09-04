Status: 2026-09-04 CURRENT (read-only research by the session-8 analyst agent: prior art, code inventory, measured mmap/sqlite physics, the persistence design + gates G1-G8, Eve, the four-taxes decomposition, the tensor-graph indexing analysis §9; the design is a PROPOSAL pending operator decisions)

# Language + database at once: research and a proposal for Bebop

Status: 2026-09-04, read-only research (session-8 analyst). Repo = /root/dowiz/bebop-lang at the
working tree of today (bebop.bp 3822 lines, seed.S 159 lines, 94 gate lines in std_golden.sh).
Every repo claim carries file:line; every web claim carries a URL; every number on "this box"
was measured today with `taskset -c 4` (Cortex-A78) by
`scratchpad/mmap_phys.py` (python only — no compiler, no gate battery was run).
Web sources were read through a fetcher; three PDFs (Twizzler ATC'20, Crotty CIDR'22,
Dearle/Kirby/Morrison 2009) were text-extracted locally and quoted from the extraction.

## 0. Executive summary (10 lines)

1. "Language AND database at once" has a 50-year proof (MUMPS globals, AS/400 single-level store) and a 30-year failure record (PS-algol/Napier88/PJama): the survivors kept ONE address model for RAM and disk; the losers were closed worlds with no schema-evolution story and a store GC nobody trusted.
2. The mechanism that every survivor shares is: data on disk is position-independent (offsets, never absolute addresses), every object carries its own length, and the root is swapped atomically (LMDB's two meta pages, EROS/KeyKOS checkpoint headers, Datomic's immutable segments).
3. Bebop today has zero of the four walls closed: no struct layout (types are discarded, bebop.bp:127; struct literals killed, :186), arrays are headerless absolute pointers into an anonymous 256 MB arena (seed.S:55-68, emit_zeros bebop.bp:3440), no versioning, no free, no file-backed data mapping — but it has every PART: `sys_mmap` with all 6 args (:3711), `sys_export` MAP_SHARED publish (:3665), tmp+rename atomic publish (cli_compile :3538-3596), relative-link blocks (pieblock.bp), CoW-append MVCC with exact freed accounting (mvcc.bp:48-95), a CSR bucket index that beats sqlite 13.8x (nnidx.bp, T100), sha256.bp, threads+LSE+futex (pool.bp).
4. Physics on this box: page-cache read fault 0.3 us/page, MAP_PRIVATE CoW fault 3.5 us/page, first write to a sparse f2fs file 7 us/page, msync 0.6 GB/s, one-page msync/fdatasync ~90-100 us, rename ~270 us under proot; f2fs is mounted `fsync_mode=nobarrier`, so power-loss durability is NOT provable here — only process-crash consistency is.
5. "Zero-copy" is worth 10-100x against serde/JSON and 1.0x against sqlite's page cache on bytes moved; the real wins over sqlite are (a) no per-row VDBE/record decode (M1: 8.6x measured on scans against sqlite's native 158 ms MIN scan) and (b) fewer cache misses per point query (M2) — and the T100 "13.8x" shrinks to ~9x once the ctypes call floor (1.17 us per C call, 16 calls = ~19 us of the 55 us) is subtracted; §8 decomposes sqlite's native ~35 us into B-tree seeks (~70%), VDBE (~9%), setup/sort (~20%), parse (0%). 100x is reachable on the scan class only (Rust-quality codegen: 112x measured), never on DRAM-resident random point lookups (both engines sit on the same ~100 ns miss; bebop just needs 3-4x fewer of them).
5c. The operator's tensor-graph proposals (§9): "dimensional descent" is the T100 grid index under another name (a 2-level arithmetic router = grid file; more levels = a B-tree with shifts; wins on uniform keys, loses on skew); its real mechanisms are clustering (range scan at 2 ns/row), block zone maps and a presence bitmap (the only root summary that answers "does k exist" in one L2 hit — Stokes sums and tensor cores cannot); graph = tensor is exact (GraphBLAS), SpMSpV beats pointer chasing 5-10x only for frontiers (miss batching + push/pull switch) and is the same machine words for a single row; "row_ptr amortized O(1)" is false for packed CSR — Bebop takes edge log + tiered CSR rebuilt by generation (GraphOne/LSM class); delete = tombstone bitmap at word granularity but block-CoW'd, update under MVCC = 4 KB block CoW, not "one store". The 100x row that survives is BFS vs sqlite recursive CTE (~15-40 ns vs ~1.5 us per edge), gate G8.
5b. Eve (2014-2018, §7) is the closest prior attempt at "everything is a record + insert-only logs + views + unified I/O": its mechanisms (event log, derived state, dependency-graph recomputation) map onto mvcc.bp/csheaf.bp/M3 and §4a; its death teaches what NOT to import — a pure Datalog surface (users said it "didn't even feel like programming") and whole-graph incremental recomputation without a k/N regime (M3's crossover at k/N < 3%).
6. Proposal: one file = two superblocks (LMDB pattern) + an append-only bump arena of self-describing objects (2-cell header: layout digest+length, crc32+generation); refs are OBJECT-relative i64 cell offsets (deref = `ldr; add x0,x0,x1,lsl #3`, 2 words, no reserved register, no seed change); commit = append + superblock toggle (no rename, no CoW pages); rollback = old root; readers never lock; the mapping IS the reader token (munmap = release; the kernel's inode refcount is the "nilpotent product to zero").
7. Schema evolution: field append is free (length in header + zero default, Cap'n Proto rule); removal = tombstone in the layout table (FlatBuffers rule); anything else = a migration fn named by sha256 in the store's migration table and executed only from the local `.bcas` (T80) at compaction time — code is never loaded from the data file.
8. Reclamation = Cheney copy of the live graph into a fresh file (the arena is a bump allocator, so compaction IS the GC), forwarding cells kept in a MAP_PRIVATE scratch view of the old file, published by tmp+rename; trigger = superseded/live ratio from mvcc's exact accounting; no free lists ever.
9. Compiler order: T47 `use` -> T43 struct literals + field access -> T48 checked types with `ref T` as a distinct type (the ONLY thing that stops absolute arena pointers from leaking into the file) -> `crc32` builtin (hardware, 8 B/cycle) -> `sys_msync`. Seed stays frozen.
10. Seven falsifiable gates with python oracles (layout, roundtrip-at-another-base, evolution, compaction, kill -9, 4-thread lost-update=0, T100-style rows vs sqlite); the honest expectation is: bebop wins PK lookup 3-5x (sqlite native seek is 0.6 us), window queries ~9x, scans 8.6x today / 30-50x after T101-T105 / 112x at Rust quality; loses file size (~2.2x: i64 cells vs sqlite varints); ties updates and reopen.

---

## 1. Prior art matrix

| system | pointer model | schema evolution | reclamation | concurrency / isolation | crash consistency | why it won / lost | what Bebop takes |
|---|---|---|---|---|---|---|---|
| MUMPS / M globals (1966-today, Epic, VistA) [w1] | no pointers at all: persistent sparse arrays `^A(k1,k2)`; keys ARE the address; B-tree underneath | none needed: schemaless keys; evolution = write new subscripts | B-tree page reuse in the global directory | per-global locks (`LOCK`), TP in later M's | journaling + before-image (GT.M) | WON in hospitals for 50 years: language = database, one namespace, no ORM; lost mindshare because the syntax is 1966 and the model is untyped | the principle "the variable name IS the storage address"; a typed twin = struct field -> fixed offset |
| IBM System/38 -> AS/400 -> IBM i (1978-today) [w2] | single-level store: one 64-bit virtual address space over RAM+disk; 128-bit tagged pointers, hardware tag bit protects them; TIMI re-translates programs when hardware changes | objects carry their type; TIMI abstraction let CISC->PowerPC migration re-translate binaries from the saved intermediate form | object-level GC ("built-in persistence and garbage collection") | object locks; journaling for DB files | journaling + object checkpoints | WON commercially for 45 years: the only mass-market proof that language+OS+DB in one address space scales; needed custom hardware (tag bits) to be safe | virtual address == persistent address is what Bebop's arena already IS in RAM; the lesson is the pointers must be PROTECTED (typed) — Bebop's substitute for tag bits is T48 `ref` type |
| PS-algol / Napier88 / PJama (orthogonal persistence, 1980-2000) [w3][w4] | object identity by store address, pointers swizzled at load (PJama: object cache) | Napier88 "hyper-code": source bound to every code object so evolution can reflect over data+code; still "invariably problematic" [w4 text extract] | store-level GC (compacting; Napier88 stable store) | one process per store; transactions bolted on later | shadow paging ("stable store", Brown) | LOST: "To invest in significant use of any closed storage system requires a very high level of trust in the long-term viability of the technology ... Other obvious limiting factors are the relatively limited scalability" (Dearle/Kirby/Morrison 2009, extracted text); PJama died with Sun's JVM changes | evolution-aware store: layout digest in every object; hyper-code = Bebop's T80 content-addressed code, so a migration is a digest |
| Grasshopper OS (1994) [w5] | containers + loci + capabilities; persistent address spaces on conventional hardware | none stated | container-level | capabilities | checkpoint | LOST: research OS; the paper's own admission — building persistence above a conventional OS "invariably constructed a complete abstract machine ... with resulting loss of efficiency" | do NOT build an abstract machine; use mmap directly (Bebop already does) |
| KeyKOS / EROS (1980s-2002) [w6][w7] | single-level store of pages+nodes; capabilities | n/a | checkpoint log migrates to home pages | capability isolation | system-wide checkpoint every ~30 s to alternating checkpoint areas; header written last | LOST as products, but the checkpoint design is the canonical "two areas + header last" protocol | superblock A/B + write header last |
| Phantom OS (2010s) [w8] | managed bytecode, object-level protection, no pointer arithmetic | n/a | VM GC | object-level | asynchronous whole-memory snapshot, synchronous in "personal time" | research; proves the model needs a managed language (no raw pointers) | Bebop has raw i64 pointers today; the store needs the managed subset (`ref`) |
| Smalltalk image / GemStone/S [w9] | image = object memory dumped; GemStone = shared multi-user repository ("extents") with OOPs | class change => instance migration hooks (GemStone), image = no history | GemStone repository GC (mark/sweep epochs) | GemStone transactions, optimistic | GemStone tranlogs | image persistence lost to files; GemStone survived in niche (Seaside) — schema migration was manual pain | the image idea = Bebop's .bin + store; migration must be a first-class table, not a hook |
| Twizzler (ATC'20, UCSC) [w10] | invariant 64-bit pointers `FOT_idx:offset` (e.g. 24-bit index + 40-bit offset); per-object Foreign Object Table maps idx -> 128-bit object ID + flags; intra-object = idx 0; cost measured: intra-object 0.4 ns, cached FOT 3.2 ns, uncached 27.9 ns, mapping an object 49.4 ns (Table 1, extracted) | late binding: FOT entry can hold a NAME resolved by a user function | object-level | views per security context; kernel out of the I/O path | cache-line writeback + fences; TXSTART/TXEND logging library; persistent views restart threads at `_resume` | research; most relevant because it names the exact reason an FOT exists: "allow a large ID space without increasing pointer size. PMDK, by contrast, increases pointer size to 128 bits" and objects stay self-contained ("easier to share") | intra-object/store-relative offsets for v1 (0.4 ns class); an FOT ONLY when cross-store refs appear (YAGNI now) |
| PMDK libpmemobj [w11] | PMEMoid = 128-bit (pool uuid, offset); deref = base + offset; avoids swizzling and ASLR | type numbers per allocation; app-level | transactional allocator with redo/undo logs; free lists per size class | per-pool locks, TX per thread | undo log + flush/fence | maintained but niche (Optane died 2022); 128-bit pointers hurt cache (Twizzler 5.2 shows up to 2 hash lookups per translation) | offset-based refs, but 64-bit not 128-bit |
| Metall (LLNL) [w12] | mmap-backed allocator, boost offset_ptr (self-relative), Supermalloc-style bins | app-level | segregated bins + chunk directory | none (single process) | snapshot by copying/reflink | 11.7x over Boost.Interprocess on graph build; proves offset_ptr + mmap is fast for pointer-heavy data | self-relative offsets on mmap = the design |
| Cap'n Proto [w13] | struct pointer = 30-bit signed word offset (relative to the pointer's own end) + 16-bit data-section words + 16-bit pointer-section words; far pointers cross segments | reading beyond the data section returns default; fields XOR'd with defaults so zero-fill = default; new fields go in padding or at the end; cannot reorder/remove | n/a (message) | n/a | n/a | WON as an RPC/message format; the segment machinery exists because "it can be difficult to predict how large a message might be" | length-in-header + zero default = free field append; XOR-with-default if nonzero defaults are wanted |
| FlatBuffers [w14] | vtable of field offsets per table; offsets relative | add only at the end, never remove: mark `deprecated`; vtable says "absent" -> default | n/a | n/a | n/a | WON in games/mobile; the vtable is the price of optional fields | the deprecate-never-delete rule for tombstones |
| rkyv [w15] | RelPtr: offset relative to the pointer's own address; mmap + cast = data ready; validation by bytecheck | none built in (archived type = layout) | n/a | n/a | n/a | WON in Rust for zero-copy; schema evolution is the user's problem | self-relative pointer semantics; the validation step (crc/bytecheck) before trusting a mapped file |
| LMDB [w16][w17] | mmap of the whole file at a fixed max size; B+tree pages; page numbers not addresses | none (KV) | free-page B+tree keyed by txnid; pages reused once no reader needs them; "stale reader transactions ... cause further writes to grow the database quickly" (lmdb.h) | single writer (mutex), readers lock-free in a shared-memory reader table; writers scan it for the oldest txn | copy-on-write pages, two meta pages, root written last, "no recovery needed"; MDB_WRITEMAP "loses protection from ... wild pointer writes" | WON (OpenLDAP, Monero, many); Crotty et al. name it the "most prominent proponent" of shadow paging over mmap | the whole commit protocol: A/B meta pages, CoW, single writer, reader table only if pages are REUSED (the proposal below avoids reuse, so no table) |
| SQLite (page store + WAL) [w18][w19] | page numbers; records are varint-encoded (decode per row = the 180 ns/row VDBE cost in SPEEDUP §3 M1) | ALTER TABLE ADD COLUMN cheap (default fill), DROP rewrites | freelist pages, VACUUM rewrites | WAL: readers don't block the writer; one writer | rollback journal (before-image) or WAL (after-image frames with chained checksums); main-file pages have NO checksum by default (cksumvfs adds 8 B/page) | WON everywhere; the T100 oracle; its point lookups are at the microsecond floor (55 us C-API for a 9-bucket window) | checksums on frames/generations, not on every page; the WAL insight: after-image append = exactly the append-only arena |
| Datomic [w20] | immutable datoms in immutable segments; indexes = persistent trees with fat nodes (thousands of items) | attributes are data; add attributes anytime | old index segments become garbage after re-index; GC by unreferenced-segment deletion | one transactor, any number of peers reading immutable segments from any cache | segments never overwritten, log appended | WON in Clojure world; proves "readers read immutable segments from anywhere" | immutable generations; re-index = rebuild the CSR arrays as a new object |
| Noms / Dolt prolly trees [w21] | content-addressed chunks (hash = address); chunk boundaries by rolling hash on keys, ~4 KB target | schema per table version | unreferenced chunks GC'd | git-like branches/merge | chunks immutable | WON a niche (versioned SQL); cost "(1+k/w)*log_k(n)" 4 KB chunks rewritten per edit | content address = csheaf.bp/ptrless.bp discipline; use it for CODE and for compaction dedup, NOT for every record (hashing per record is 100x the cost of an offset) |
| Unison [w22] | code by 512-bit hash of AST+deps; names are metadata; codebase = a database | rename free; "perfect compilation cache" | n/a | n/a | n/a | shipped 1.0; the homoiconic-store proof for CODE | T80 `cas://sha256` is this; migrations and query kernels are stored by hash |
| Automerge (CRDT store) [w23] | columnar op log; ops by (actor, seq); change hashes chain | JSON-like, schemaless | none (history kept) | merge = CRDT | append-only chunks | WON for local-first sync; ~1.1 B/op | out of scope for a single-writer store; the change-hash chain is the same as a generation crc chain |
| Zig `extern struct` / packed [w24] | layout = C ABI, `@offsetOf` comptime for well-defined layouts only | none | n/a | n/a | n/a | shows the rule: only DECLARED-layout types get comptime offsets; plain structs have none | Bebop: every persisted struct has a declared, digested layout |
| LINQ (expression trees -> provider) | query = AST compiled by the provider | n/a | n/a | n/a | n/a | proves "queries are native language constructs compiled by the compiler"; the store must expose the AST, Bebop's compiler is the provider (T32 qjit) | the query IS a .bp fn; compile-once by digest (T108) |
| Crotty/Leis/Pavlo CIDR'22 "Are you sure you want to use mmap" [w25] | (critique) | | | | | "using mmap in database systems is almost always a bad idea": (1) transactions need out-of-place writes because the OS may flush any dirty page anytime, (2) I/O stalls, no async, (3) SIGBUS error handling, (4) page-table contention, single-threaded kswapd eviction, TLB shootdowns; measured 100 threads / 10 SSDs / dataset > page cache: mmap collapsed to ~half of fio after the cache filled | the proposal is squarely in the regime where the paper concedes mmap is fine: small store (<= RAM), single writer, out-of-place (append) writes, <= 4 threads; NEVER truncate a mapped file (SIGBUS), read-only mappings for readers |

Sources: [w1] https://en.wikipedia.org/wiki/MUMPS ; [w2] https://en.wikipedia.org/wiki/IBM_AS/400 ; [w3] https://en.wikipedia.org/wiki/Napier88 , https://www.vldb.org/conf/1999/P55.pdf ; [w4] https://archive.cs.st-andrews.ac.uk/papers/download/DKM09a.pdf (also https://arxiv.org/abs/1006.3448) ; [w5] https://archive.cs.st-andrews.ac.uk/gh/pub/gh-03.pdf ; [w6] https://zoo.cs.yale.edu/classes/cs422/2013/bib/bomberger92keykos.pdf ; [w7] https://www.usenix.org/conference/2002-usenix-annual-technical-conference/design-evolution-eros-single-level-store ; [w8] https://en.wikipedia.org/wiki/Phantom_OS ; [w9] https://docs.gemtalksystems.com/GBS/8.x/GBS-VW-UsersGuide-8.4/1-Overview.htm ; [w10] https://www.usenix.org/conference/atc20/presentation/bittman (pdf https://dbittman.github.io/pubs/atc20-twizzler.pdf) ; [w11] https://pmem.io/blog/2016/01/c-bindings-for-libpmemobj-part-2-persistent-smart-pointer/ ; [w12] https://arxiv.org/abs/2108.07223 ; [w13] https://capnproto.org/encoding.html ; [w14] https://flatbuffers.dev/evolution/ ; [w15] https://rkyv.org/architecture/relative-pointers.html ; [w16] https://raw.githubusercontent.com/LMDB/lmdb/mdb.master/libraries/liblmdb/lmdb.h ; [w17] https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database ; [w18] https://www.sqlite.org/wal.html ; [w19] https://www.sqlite.org/cksumvfs.html ; [w20] https://tonsky.me/blog/unofficial-guide-to-datomic-internals/ , https://docs.datomic.com/indexes/index-model.html ; [w21] https://www.dolthub.com/docs/architecture/storage-engine/prolly-tree ; [w22] https://www.unison-lang.org/docs/the-big-idea/ ; [w23] https://automerge.org/automerge-binary-format-spec/ ; [w24] https://zig.guide/working-with-c/extern-structs/ , https://github.com/ziglang/zig/issues/8642 ; [w25] https://db.cs.cmu.edu/papers/2022/cidr2022-p13-crotty.pdf .

The pattern across the winners (MUMPS, AS/400, LMDB, SQLite WAL, Datomic): (i) never store a
RAM address, (ii) never overwrite what a reader may be reading, (iii) commit = one atomic root
swap, (iv) length lives with the data. The losers (Napier88, PJama, Grasshopper, image
persistence) had (i)-(iii) too; what they lacked was (v) an evolution story that a business
could trust and (vi) a store GC with predictable cost. So the design below spends its
engineering on (v) and (vi), and copies (i)-(iv) from LMDB verbatim.

---

## 2. What Bebop has today (honest inventory)

| mechanism | where | what it really is | gap for a store |
|---|---|---|---|
| process memory | seed.S:31-46 code mmap PROT_READ|EXEC MAP_PRIVATE of the .bin; seed.S:55-68 arena = mmap(256 MB, RW, PRIVATE|ANON) into x27 (cursor) / x28 (end) | the arena is ANONYMOUS: nothing in it survives the process; addresses differ per run | needs a second, file-backed mapping; the arena stays as the volatile heap |
| allocation | emit_zeros bebop.bp:3440-3468: `mov x0,x27; add x27,x27,x1` + zero loop; check_abi.py:15-16 allows only that bump | bump allocator, no header, no length, no free; `zeros(n)` returns an ABSOLUTE address | store objects need a header and offsets; `zeros` addresses must never be written into a store cell (T48 `ref` type is the guard) |
| frame heap | emit_prologue bebop.bp:2272 (16 KiB frame; x14 heap bump from sp+1024); emit_array_lit :2017-2054 and emit_struct_lit :285-310 allocate on x14 | array literals and struct literals live in the CALLER'S FRAME and die at return | a THIRD allocator world; the compiler must know which world a value is in (it discards types: skip_to_delim :127) |
| structs | `struct_kill = 1` bebop.bp:186 (literal branch disabled: "a bare `ident {` would eat a while/if body"); emit_field_access :312-323 emits `ldr x0,[x0,#idx*8]` (word 4181721088 + idx*1024); field_index scans the first `struct` decl text | field access = fixed-offset load ALREADY (the M-language principle at word level); no layout digest, no header, only ONE struct decl is found (find_struct) | re-enable with a context flag (T43), one layout per struct name, header offset +2 cells |
| enums | emit_enum_ctor bebop.bp:371-384: [tag, payload] two cells on x14, pointer pushed; nullary :386 | already an object shape (tag cell + payload cell) | same header rule as structs |
| arrays | `[i64]` = raw pointer (F-F, ROADMAP.md:1360-1374); emit_array_get :2056 `ldr x0,[x2,x1,lsl 3]`; no length, no bounds | headerless | store arrays get [len] header; bounds check optional (loud trap) |
| strings | scan_literals bebop.bp:2978 appends literal cells after the code, `adr x0` at use | the ONLY data that persists today = string literals inside the .bin (SPEEDUP §3 M10) | fine; strings in the store = byte arrays |
| file I/O | emit_sys_open/read/write/close/slurp/readbuf (:1087 slurp), ftruncate :3627, munmap :3645, export :3665 (ftruncate + mmap MAP_SHARED + byte stores + munmap, "NO sys_write"), mmap :3711 (all 6 args), rename :3755; missing: msync (227), fsync, unlink, flock | a .bp program can already `sys_mmap(addr,len,prot,flags,fd,off)` a file MAP_SHARED at any base — the store needs NO seed change | add `sys_msync` (one word table, L1/L2 discipline) for the durability row; the crash gate works without it (page cache survives kill -9) |
| atomic publish | cli_compile bebop.bp:3538-3596: export to tmp, close, `sys_rename(tmp,out)`; store.bp:131 `bt_store` = same for .bt | the compaction publish path exists and is gated (store gate, morph gate T11) | reuse as-is for compaction output; NOT for every commit (rename costs ~270 us under proot, measured) |
| .bt codec | bt.bp:1-12 format (magic BT4R, u32 version, rank 4, dims, i64 LE data); bt_pack :22, bt_unpack :84, bt_fnv :69; copied verbatim into store.bp (F-D duplication) | a self-describing dense tensor with a VERSION field; byte-per-cell buffers (8x memory) | the store's array object = a .bt payload without the byte-per-cell detour (map the i64s directly) |
| position independence | pieblock.bp:1-13: block = [magic,count,{payload,rel_next}]; loaded at base 0 and 1000, identical walk, FNV move-invariant; SS-13 ROADMAP.md:519 says "zero-copy mmap save/load ... deferred" | the relative-link discipline is gate-proven at toy scale | this IS the ref model; make the compiler emit it |
| content addressing | ptrless.bp:1-8 (digest = the only pointer; corrupt key must not resolve); csheaf.bp:44-57 probe/resolve (16-slot open addressing, `slot = d & 15`), :60-83 check() O(degree), :85 insert rejects inconsistent stalks; sha256.bp (125 lines, software); T80 bans FNV for source addressing | O(1) digest -> slot; validation-by-neighbours | use digests for CODE (migrations, query kernels) and layouts; not per record |
| MVCC | mvcc.bp:1-20 header; upd :48 (update = NEW record + prev edge, never in place), acq :67 / rel :79 (Grassmann reader tokens, collapse when superseded AND no token; exact freed-cell accounting st[0]); single-threaded LCG interleaver | the append-only semantics + exact live accounting the compaction trigger needs | the token = an integer-free liveness device; in the file design the kernel's inode refcount replaces it (see 4c) |
| STM | stm.bp:90 begin / :113 commit (odd-sector context, conflict = nilpotent product 0, abort leaves the store bit-identical, Stokes check inline) | conflict detection for MULTIPLE writers | not needed under single-writer; keep for in-arena multi-thread state |
| reversible journal | rev.bp:58 rev_round / :72 rev_undo (XOR deltas); T73 ROADMAP.md:1913 makes it THE rollback mechanism and rejects a second CoW log ("two mechanisms = two truths") | O(delta) undo for the volatile arena | for the STORE, rollback = the previous root (free). Decision needed: T73's "one mechanism" law must say "XOR journal for the arena, root swap for the store" or it contradicts LMDB-style commit |
| generation arena | genarena.bp:1-12: mark/reset, zero fragmentation, "mmap MAP_NORESERVE/mprotect half deferred" | the write-transaction abort primitive (reset cursor to mark) | exactly the store's txn abort |
| GC replacement | entcol.bp:1-10 (collapse by diversity proxy, "no scans, no refcounts") | a demo; not a reclamation mechanism for arbitrary graphs | compaction = Cheney copy; entcol stays a demo |
| index | nnidx.bp:7 cellof, :25-46 build (counting sort -> rp[]/ci[] CSR), :52-66 scan_cell, :68 query (3x3 window); T100 (ROADMAP.md:2508): 4.0 us/query vs sqlite C-API 55 us | the bucket index that made the 13.8x; lives in the anonymous arena, rebuilt every run (~15 ms for 1M) | make rp/ci two store arrays with a header naming the key fn's digest |
| threads | pool.bp:1-24 (clone 68864, 64 KiB stacks in the arena, futex park, LSE atomics); T45 landed 4 real threads under proot | single process, shared arena | single-writer lock = one atomic cell in the store header via `sys_atomic_add` + `sys_futex_wait_guard` (bebop.bp:982/:920) |
| hashing hw | /proc/cpuinfo Features: crc32, sha1, sha2, atomics (F-I); crc.bp is a bit-by-bit software CRC (32 iterations per byte); no crc32 emitter | hardware CRC32X = 8 bytes/cycle exists and is unused | one builtin `crc32(cells, n)` (L1: asm -> objdump -> words) |
| persistence today | = the compiled .bin + literal cells (M10) + `.bt` files written by `sys_export` (raw cells, no header beyond .bt's) | "arrays are NOT in the artifact" (SPEEDUP §3 M10, §6.1 last bullet) | the whole proposal |
| versioning / free | none: no headers, no generations, no free, no msync, no lock file | | |

Bluntly: today a Bebop value is an absolute address into memory that evaporates at exit, of
unknown length and unknown type. The compiler cannot tell an arena pointer from a frame-heap
pointer from an integer. That is the root gap; every wall (pointer stability, evolution,
reclamation) is downstream of "types are parsed and discarded" (F-F).

---

## 3. The physics on this box

Measured today (`scratchpad/mmap_phys.py`, `taskset -c 4`, python loop floor ~0.25 us per
iteration already subtracted where noted; kernel 6.17 under proot; f2fs on /dev/block/dm-64
mounted `fsync_mode=nobarrier, lazytime, noatime`; 4 KB pages; 7.7 GB RAM, ~2.7 GB available;
no DAX/PMEM device):

| operation | measured | consequence for the design |
|---|---|---|
| anonymous first-touch fault (MAP_PRIVATE|ANON) | ~2.5 us/page (2.78 raw) | the 256 MB arena costs 2.5 us per NEW page touched; `zeros(1M)` = 8 MB = 2048 pages = ~5 ms of faults + the 8 MB zero pass (§6.1 of the analysis) |
| file MAP_SHARED, first WRITE to a sparse f2fs page | ~7.0 us/page | allocating file blocks on write is 3x an anonymous fault; pre-extend + pre-touch the store file in 64 MB steps (ftruncate + MAP_POPULATE) |
| file MAP_SHARED READ fault, page in page cache | ~0.3 us/page | a 40 MB store (1M records) cold-mapped costs ~10k faults = ~3 ms; with MAP_POPULATE 64 MB = 5.2 ms total, then 0 faults |
| MAP_PRIVATE CoW write fault on a file page | ~3.5 us/page (3.80 raw) | CoW is 10x a read fault; fine for the compaction scratch view (forwarding cells), wrong for the commit path -> commit must NOT rely on page CoW |
| msync(MS_SYNC), 64 MB dirty | 110 ms = 0.61 GB/s | a full-store flush is 10-20x slower than DRAM copy; flush only dirty ranges (append region + superblock) |
| msync one dirty page / write+fdatasync 4 KB / write+fsync 4 KB | ~100 us / ~90 us / ~130 us | a durable commit floor of ~0.1-0.2 ms (two flushes: data then superblock); 5-10k durable commits/s max; batch commits |
| rename(2) | ~270 us median | under proot every path syscall is ptrace-translated; rename-per-commit would cost more than the flush -> rename only at compaction |
| durability caveat | f2fs `fsync_mode=nobarrier` | fdatasync returns without a device cache flush; power-loss durability CANNOT be proven on this box; kill -9 consistency can (page cache survives the process) |
| zlib.crc32 / hashlib.sha256 / blake2b | 2.42 / 1.24 / 0.46 GB/s | sha256 here is OpenSSL's ARMv8 SHA2 path (~1.2 GB/s per A78); hardware CRC32X is 1 instruction per 8 bytes, throughput 1/cycle on ARMv8 big cores (https://dougallj.wordpress.com/2022/05/22/faster-crc32-on-the-apple-m1/ , A57 optimization guide https://documentation-service.arm.com/static/5ed75ee1ca06a95ce53f93c0) = ~19 GB/s at 2.4 GHz: a 64-byte object costs ~8 cycles (~3 ns); sha256 of the same object ~50 ns |
| per-object digest via python | crc32 360 ns, sha256 870 ns (dominated by the interpreter) | irrelevant for the design; native numbers above are what count |
| DRAM (SPEEDUP §3) | ~12 GB/s, one A78 saturates it; 4 cores 1.0-1.4x on streams | scans are DRAM-bound at ~1.4 ns/row (Rust); bebop's scan is codegen-bound (18.4 ms / 1M = 18 ns/row) |

What "zero-copy" buys, precisely:

| against | cost being removed | gain | when it holds |
|---|---|---|---|
| serde/JSON/text | parse at 0.1-0.5 GB/s vs mmap+use at DRAM speed | 10-100x (SPEEDUP §3 M10 says the same) | always, if the record is used in place |
| sqlite page cache, bytes moved | none: sqlite pages are also memory-resident and never "parsed" as text | 1.0x (M10) | — |
| sqlite per-row record decode + VDBE | varint header decode + ~30 VDBE opcodes = ~180 ns/row (M1) vs `ldr` in place | 9.9x measured on a 1M scan (T100: 183 ms vs 18.4 ms); ~128x at Rust codegen quality | scan-class queries |
| sqlite point lookup through a B-tree | prepared statement + 9 index seeks + 8.5 rowid seeks + decodes = ~35 us NATIVE (the T100 55 us includes ~19 us of python-ctypes call overhead, measured §8) | ~9x native (4.0 us) — the win is 3-4x fewer DRAM misses per query (M2), not zero-copy | index-class queries |
| relative offset vs absolute pointer | `ldr x1,[x0,#f]; add x0,x0,x1,lsl #3` (2 words, +1 cycle dependent latency) vs `ldr x0,[x0,#f]` | -1 cycle per ref hop (Twizzler intra-object 0.4 ns); a fixed-offset scalar field costs NOTHING extra: `ldr x0,[x0,#(2+idx)*8]` | pointer-chasing loses ~20% per hop; field access loses 0 |
| digest per object | crc32 hardware 8 B/cycle: 3 ns per 64 B record; sha256 ~50 ns | crc32 on the write path is free relative to the 90 us flush; sha256 per record is NOT free (1M records = 50 ms) | crc32 for integrity, sha256 only for code/layout identity |

The honest summary of §3: on this box the store cannot beat sqlite by "zero-copy"; it beats
sqlite by (1) compiled queries (no VDBE) and (2) the bucket index, both of which the repo
already measured. The persistence layer's job is to not lose those two while adding
durability, versioning and evolution — its own overheads (2-cell header, 2-word deref, 3 ns
crc) are below the noise of a 90 us flush.

---

## 4. The design (minimal, zero C, integer-only, gated)

### 4a. Object model

Every persisted value is a run of i64 cells with a 2-cell header:

| cell | content | why |
|---|---|---|
| h0 | `layout_digest_lo32 << 32 | length_in_cells (32 bits)` | length with the data (Cap'n Proto/FlatBuffers rule); the digest names the layout in the store's layout table |
| h1 | `crc32(payload) << 32 | generation (32 bits)` | integrity (crc32 hardware, ~3 ns/64 B); generation = the commit that wrote it (MVCC "as of", mvcc.bp `prev` edge is the previous version's offset, kept as a normal `ref` field when the type declares history) |
| p0..p(len-1) | payload cells | struct fields in declaration order; enum = [tag, payload...] as emit_enum_ctor does today; array = [n, e0..e(n-1)] |

- Scalars: i64 only (doctrine). `fp` (Q32) is an i64 with a type tag in the layout, zero cost.
- Refs: `ref T` = OBJECT-relative signed cell offset: `target = this_object_start + off*8`;
  0 = null (an object never points at its own header through a field). Deref = 2 words. Not
  self-relative (Cap'n Proto/rkyv) because Bebop never moves a field without its object; not
  store-relative (PMDK/Twizzler intra) because that needs a base register or a reload.
  Cross-store refs: not in v1 (Twizzler's FOT is the answer if ever needed; today YAGNI).
- Layout digest: sha256 (sha256.bp, compile time only) of the canonical layout string
  `T{i64,i64,ref U,arr i64,...}` — TYPES ONLY, positional, no field names (renaming a field is
  free, as in Cap'n Proto; names live in the layout table for tooling). Low 64 bits kept in the
  layout table, low 32 in h0. FNV is banned for this (T80's rule, ROADMAP.md:1986-1996).
- Alignment: 8 bytes (cells). 64-byte alignment is an OPTION per table (hot CSR arrays), not a
  rule — it would waste up to 56 B per small record.

### 4b. The file

```
page 0 : superblock A   page 1 : superblock B      (LMDB pattern)
page 2.. : bump arena of objects, append-only within a generation, never overwritten
```
Superblock (16 cells, crc32 last): magic, format version, generation, root (store-relative
cell offset), arena_used (cells), layout_table (offset), migration_table (offset),
live_cells (exact, mvcc.bp accounting), superseded_cells, crc32 of cells 0..14. A reader
picks the superblock with the higher generation whose crc is valid.

- Mapping: readers `sys_mmap(0, size, PROT_READ, MAP_SHARED, fd, 0)`; the writer maps
  PROT_READ|WRITE MAP_SHARED. Any base — nothing in the file is an address. Under proot both
  are proven to work (store.bp gate uses MAP_SHARED writes through sys_export).
- The seed is NOT changed (T57 doctrine: seed frozen). The volatile arena x27/x28 stays; the
  store is a second mapping whose bump cursor is `arena_used` in the superblock. A tiny
  library `store.bp` (with T47 `use`) owns: open/map, alloc(cells), commit, abort, deref
  helpers until the compiler emits them.
- Growth: pre-extend by `sys_ftruncate` in 64 MB steps and remap (a remap changes the base —
  which is exactly why nothing may hold an absolute pointer across a call that can grow the
  store; the compiler rule: `ref` values are offsets, materialized to addresses only inside the
  expression that uses them).

### 4c. Persistence unit, publish, readers

- Persistence unit = the file + the root offset in the live superblock. A generation = the set
  of objects reachable from that root.
- Write transaction (single writer): mark = arena_used; append objects (new versions, mvcc.bp
  :48 semantics: never in place); commit = (optional `sys_msync` of the appended range) ->
  write the OTHER superblock with generation+1 and the new root -> (optional msync of that
  page). Abort = arena_used := mark (genarena.bp reset). No rename, no page CoW, no journal.
  This is LMDB's protocol with append instead of page CoW; SQLite's WAL is the same idea
  (after-image append) without the checkpoint-back step.
- Readers: map, read the superblock, hold the root. Because objects reachable from any older
  root are never overwritten (append-only, no page reuse), a reader is consistent for as long
  as its mapping lives — no reader table, no lock, no token integers. The "nilpotent reader
  token" of T33 becomes physical: the mapping is the token, `sys_munmap` is the release, and
  when compaction replaces the file (rename), the old inode stays alive exactly while a mapping
  exists — the kernel's inode refcount is the product that reaches zero. (LMDB needs its reader
  table only because it REUSES pages; this design never does.)
- Cost of never reusing: the file grows by every update until compaction (4e). That is the
  trade the operator asked for ("CoW trees so the file neither fragments nor bloats") — it does
  not fragment (bump), it bloats between compactions by exactly `superseded_cells`, which the
  superblock reports.
- In-process multi-thread readers/writers (pool.bp): the writer lock = one cell in the
  superblock page via `sys_atomic_add` (LSE, bebop.bp:982) + `sys_futex_wait_guard` (:920);
  cross-process writers: `sys_open` with O_CREAT|O_EXCL (flags 64|128) on `<store>.lock` —
  no new builtin.

### 4d. Schema evolution

| change | mechanism | cost |
|---|---|---|
| append a field | new layout digest; old objects are shorter (h0 length) -> reader returns 0 (or XOR-default) for missing cells; write path always writes the new length | free (Cap'n Proto rule) |
| rename a field | digest is types-only -> unchanged | free |
| remove a field | layout table marks it tombstoned (FlatBuffers `deprecated`); the cell stays until compaction | free until compaction |
| change a type / split / merge | a migration fn `fn m(old: ref T@d1) -> ref T@d2` compiled to a .bin, addressed by sha256, registered in the store's migration table as (d1, d2, sha256); applied at compaction time while the Cheney copy visits every live object (that is the one moment every object is read anyway); until then readers compiled for d2 that meet d1 objects trap loudly (ptrless discipline) unless a read-side shim exists | one extra pass per object at compaction; zero on the read path |
| history | if `T` declares `prev: ref T`, old versions stay reachable (Datomic "as of"); if not, superseded objects are garbage at the next compaction | user's choice per type |
| code from the store | the store holds ONLY digests; the bytes come from `.bcas/<hex>.bin` (T80) produced by the operator's compiler and verified by sha256 before `seed` runs them | a store file cannot inject code |

Why layout digests beat source-text digests: the same struct written with different
whitespace, comments or field names must map to the same on-disk layout; the same TYPES in the
same order ARE the same layout. (The operator's worry — "a field size change must not break
history" — is answered structurally: every cell is 8 bytes, a "size change" is a type change,
and a type change is a migration, never an in-place reinterpretation.)

### 4e. Reclamation: compaction = Cheney

The arena is a bump allocator, so "GC" is a copying collection of the live graph from the
root into a fresh file, exactly Cheney 1980 (https://yorotoo.medium.com/paper-1-2-3-semispace-copying-by-fenichel-and-yochelson-1969-and-cheney-1980-f6fdc2384f16):

1. map the old file MAP_PRIVATE RW (scratch: forwarding offsets are written into old headers'
   h1 cells; the CoW pages are anonymous and vanish at munmap — the old file is never
   modified; cost 3.5 us per touched page, measured);
2. create `<store>.tmp`, copy the root object, then BFS: for each copied object rewrite each
   `ref` field to (forwarded target - new object start), copying targets on demand; apply
   pending migrations per layout digest during the copy; recompute crc32;
3. write superblock A of the new file (generation+1, live_cells = superseded_cells = 0 ...
   wait: superseded = 0, live = copied), msync if durability is on;
4. `sys_rename(tmp, store)` (cli_compile bebop.bp:3538-3596 pattern, ~270 us) — readers of the
   old inode keep their snapshot; new openers see the compacted file.

Trigger: `superseded_cells > k * live_cells` with k frozen by the compaction gate (start
k = 1: the file is at most 2x the live set). No free lists, no fragmentation metric needed —
bump allocation has none. Objects that fit in one cache line stay in one cache line after
copy (BFS preserves locality by reference order, better than the insertion order it replaces).

### 4f. Queries

- Field access = `ldr x0,[x0,#(2+idx)*8]` — emit_field_access bebop.bp:312 already emits
  the shape (word 4181721088 + idx*1024); the +2 is the header.
- `ref` deref = `ldr x1,[x0,#(2+idx)*8]; add x0,x0,x1,lsl #3` (2 words; the shifted-register
  ADD encodes as one word; L1 discipline: asm -> objdump -> decimal).
- Predicates and joins are `.bp` fns over `ref T` — compiled by bebop.bin, published by the
  morph path (T11), memoized by digest (T108). No planner: the "plan" is the index chosen by
  the programmer, exactly as in MUMPS.
- Index: the T100 bucket index generalised — `idx = csr_build(table: ref [T], key: fn(ref T)->i64, K)`
  produces two store arrays rp[K+1], ci[N] plus a header object {K, N, key fn sha256, table
  generation}; `csr_scan(idx, k)` yields `ci[rp[k]..rp[k+1])`. Rebuild is O(N) counting sort
  (nnidx.bp:25-46); the index is stale when `table generation != idx.generation` (loud trap
  or rebuild). Range scans over an integer key = consecutive buckets; the 3x3 window is
  `k-1..k+1` in two dimensions. That is the entire "query engine" needed to keep the 13.8x.

### 4g. Crash consistency

- Nothing is ever overwritten except the alternate superblock; its crc32 covers its 15 cells
  and is written last; a torn superblock is rejected and the other one used (LMDB/EROS).
- Objects appended after the last valid superblock are unreachable garbage (the next writer
  restarts the cursor from `arena_used` of the valid superblock; the crc in h1 catches a
  torn object if a debugging reader walks past the cursor).
- kill -9 anywhere: consistent by construction (page cache is coherent, MAP_SHARED writes are
  visible to the file at the instant of the store instruction).
- Power loss: requires `sys_msync` before the superblock write AND a device barrier that this
  box's `fsync_mode=nobarrier` does not give — gate rows must say "process-crash proven,
  power-loss forward-port".

### 4h. Concurrency

Single writer + N readers (LMDB). Readers never take a lock and never see a torn object
(append + root swap). Writer lock = one atomic cell (in-process) or an O_EXCL lock file
(cross-process). Threads from pool.bp shard READ work (T106 nn4 pattern); write batching
through one thread is what every winner in §1 does (LMDB, SQLite, Datomic's transactor).

### 4i. What the compiler must gain, in order

| step | task | why here | words / risk |
|---|---|---|---|
| 1 | T47 `use` (ROADMAP.md:1646) | store.bp/bt.bp/csr.bp are copy-pasted 5-14x (F-D); the store library must be `use`d, not pasted | parser only, fixpoint |
| 2 | T43 struct literals + field access (:1620), with the context flag that `struct_kill` (bebop.bp:186) lacks; one `find_struct` per NAME | layouts need declared structs | emitter, fixpoint, c32-c35 |
| 3 | T48 checked types (:1654) extended with `ref T`, `[T]`, `fp` | THE guard: an arena/frame address can never be stored into a `ref` cell; a `ref` is never used as an address outside a deref | compile-time traps, zero runtime words |
| 4 | `crc32(cells, n)` builtin (crc32x in a 5-word loop) | integrity at 8 B/cycle; crc.bp's bit loop is ~300 ops/byte | L1/L2 register table, check_abi allowlist |
| 5 | `sys_msync(addr,len,flags)` (syscall 227) | the durability row | one word table |
| 6 | store.bp library over 1-5: open/alloc/commit/abort/compact/csr_build/csr_scan | the gates below | pure .bp |
| 7 | emitter: `.f` on `ref T` = 2-word deref; struct literal into the store when the target type is persisted (`store T{...}` form, mirrors `zeros` vs frame) | field access at codegen | one emitter fn |
| later | T32 qjit over store tables; T108 digest memo; T26 regrec = the mapped .bt (already re-scoped that way, D9(2)) | | |

Not needed: any seed change (sys_mmap exists), a second allocator strategy, free lists, a
reader table, a WAL, a planner, an FOT, 128-bit pointers, per-record sha256.

---

## 5. The gates (each = one `.bp` file + `bench/oracles/<gate>.py`, L17)

| # | gate | what the .bp does | oracle (python, stdlib only) | falsifier |
|---|---|---|---|---|
| G1 | `slayout` | declare 3 structs (scalars, a ref, an array) + 1 enum; write one instance of each into a store file; fold = crc32 of the file bytes | `struct.pack('<QQ...')` builds the expected bytes from the layout rules (header, types-only sha256, object-relative offsets); crc32 must match; also asserts `@offset(field)` == python's | any header/offset/digest rule mismatch |
| G2 | `sround` | write N=10^5 objects (LCG) into a store, `sys_munmap`, `sys_mmap` the file TWICE (two different bases in one process) and ALSO reopen in a second run; fold over the graph from the root through the second mapping | parse the file with `struct.unpack` following offsets from the superblock; fold equal; the two bases must differ (asserted) | an absolute address anywhere in the file |
| G3 | `sevolve` | program v1 (layout L1) writes; program v2 (L1 + appended field) reads v1's store and writes new objects; program v1 reads v2's store (ignores the extra cell); program v3 (a field split) registers a migration by sha256 and compacts; folds | oracle computes all four folds from the layout rules and the migration function re-implemented in python | any read of a stale layout that does not either default or trap; a migration that changes a value the oracle did not |
| G4 | `scompact` | build 10^6 objects, supersede 60% (new versions), compact; fold of the live graph before == after; file size after <= live_cells*8 + 3 pages; `superseded_cells == 0` after | reachable-set fold and the exact byte bound; also checks that every ref in the new file targets a header whose digest exists in the layout table | a lost live object, a dangling ref, or bloat |
| G5 | `scrash` | writer publishes 10^4 generations (each: 100 appends + superblock toggle); the harness kills it with SIGKILL at a random microsecond (100 trials); reopen; fold of the reachable graph | for the observed generation g the oracle recomputes the expected fold (generation content is a pure function of g via the LCG); asserts g in {last written per the writer's stdout log, that -1} and superblock crc valid | any reopen that traps, any fold != oracle(g) |
| G6 | `sconc` | pool.bp: 4 threads x 10^4 increments of counters through the single-writer lock (append new version + root swap under the lock); 4 reader threads concurrently fold the visible generation 10^4 times | final counters == 4*10^4 each; every reader fold must equal oracle(g) for SOME g <= final (mvcc.bp invariant at scale); lost updates == 0 | a lost update or a torn read |
| G7 | `sbench` (T100 pattern, `bench/tq_sqlite/run.sh` style) | 1M records (id,u,v,cell): insert (append), point lookup by id (direct offset), range scan 10^4 by cell (csr_scan), update 10^5 (new versions), reopen (map + superblock), file size, compaction time; pinned A78, R=5 medians; plus the durable variant with msync per commit | sqlite 3.46.1 same rows through the C API (ctypes) as in `bench/tq_sqlite/sqlite_capi.py`: `INSERT` in one transaction, `SELECT ... WHERE id=?`, `WHERE cell BETWEEN`, `UPDATE`, open + first query, file size. RULE (new, from §8): report sqlite NATIVE time = measured minus the ctypes floor (calls x 1.17 us, floor measured in the same run), and record `sqlite3_stmt_status(VM_STEP)` per query; otherwise the ratio is inflated ~1.5x | numbers, whatever they are (D8(5)); pass = point lookup >= 3x sqlite native and scan >= 5x; file size reported even if 2x worse |

Expected G7 shape (derivations, not promises): PK point lookup ~0.1-0.2 us (one line miss on
the record + header check) vs sqlite ~0.6 us NATIVE for a primary-key seek (measured §8; 3
B-tree levels with hot upper levels) = 3-5x, and ~9x on the 9-bucket window query; range scan by cell:
bucket + window at ~10-20 ns/row vs sqlite ~200 ns/row; insert 1M: ~40 MB append at DRAM
speed ~10-30 ms + one flush vs sqlite executemany 2.46 s (python) / ~0.3-0.5 s (C API);
reopen: 5 ms (MAP_POPULATE 64 MB) vs sqlite open ~1 ms + first-query page-ins — roughly a
tie; file size: 40 MB (5 cells x 8 B per record) vs sqlite 18 MB — a 2.2x LOSS that must be
reported as measured. Durable commit: ~0.1-0.2 ms per commit (two flushes), sqlite WAL
`synchronous=NORMAL` is in the same class.

Gate order: G1 -> G2 -> G4 -> G5 -> G3 -> G6 -> G7 (G3 needs T80's `.bcas`; G6 needs the
lock builtins that exist; G7 last because it is the only one that can fail for speed
reasons rather than correctness reasons).

---

## 6. Risks and what kills it

| risk | mechanism | mitigation / verdict |
|---|---|---|
| mmap under proot | ptrace intercepts syscalls, not page faults; MAP_SHARED file writes proven by the store gate; measured faults are 2.5-7 us (slow but functional); rename 270 us | works; keep syscalls per commit at 0-2; MAP_POPULATE on open |
| i64 offsets double the record vs i32 | integer-only doctrine: every cell is 8 B; a 3-field record = 40 B with header vs sqlite ~18 B | accept in v1 and REPORT it (G7 file-size row); a packed layout (`i32`/`i16` cells) is a later type-level feature, not a store change |
| page-granular write amplification | a 40 B update dirties a 4 KB page; msync writes the page (100x); per-commit msync ~100 us | append-only commits batch naturally (dirty pages are contiguous); durable mode is per-batch, not per record |
| absolute pointers creeping back | `zeros()` returns addresses; the frame heap (x14) returns addresses; a store cell that receives one is silently wrong after remap | T48 `ref` is a distinct type; G2 maps at two bases every run; the emitter materialises addresses only inside a deref expression; check_abi-style scan for `str` of an x27/x14-derived register into the store mapping is a tooling guard |
| store growth between compactions | never reusing space means every update costs its size until compaction | exact accounting in the superblock; k=1 trigger; compaction is O(live) at DRAM speed (40 MB ~ 10-30 ms) |
| compaction while readers hold the old inode | disk holds two copies until munmap | readers that live forever pin disk, never correctness; document, and add a `store_stat` row |
| schema digest vs source text | a comment or rename must not change the layout; a type change must | types-only positional digest; G1 asserts a renamed field gives the same digest and a retyped field a different one |
| executing migration code named in the store | a hostile store file could name arbitrary digests | code is loaded ONLY from the local `.bcas` by sha256 match; unknown digest = loud trap; the store carries no code bytes |
| power-loss durability | f2fs `fsync_mode=nobarrier`; no PMU; no way to cut power | G5 proves process-crash consistency only; label durability forward-port (T15 class) |
| Crotty's mmap failure modes | I/O stalls, SIGBUS on truncation, TLB shootdowns at 100 threads, single-threaded eviction when the dataset exceeds the page cache | regime: store <= available RAM (~2.7 GB), <= 8 threads, never truncate a mapped file (compaction writes a NEW file), readers PROT_READ; outside that regime the design is wrong and must say so |
| two rollback truths (T73) | XOR journal (rev.bp) for the arena vs root swap for the store | decide explicitly: T73 governs the volatile arena; the store's rollback is the previous root; write it into T73's text |
| three allocator worlds | frame heap (x14), arena (x27), store | the compiler must type them; until T48 lands, the store library is the only writer and every gate maps at two bases |
| self-hosting fixpoint churn | every emitter change re-freezes constructs (L16, T96 precedent) | steps 2, 3, 7 of 4i are one-commit-each with fixpoint; the store library (step 6) touches no emitter |
| "1.0x vs sqlite" on bytes | the thesis "language IS the database" gives no speed by itself | the speed is M1 x M2 (already measured); the store's job is to keep them while adding durability; if G7's point lookup is not >= 3x sqlite's C API, the persistence layer has eaten the win and must be fixed before anything else |

What kills it outright: (1) letting an absolute address into a store cell (G2 catches it
only if T48 exists to prevent the next one); (2) reintroducing a second commit mechanism
(journal + root) — the losers in §1 all had two truths; (3) hashing per record with sha256
"for purity" (50 ns x 1M = the whole scan budget); (4) per-commit rename under proot.

---

## 7. Eve (2014-2018): the nearest prior attempt, mechanism by mechanism

Facts (sources: https://witheve.com/deepdives/whateveis.html ; https://github.com/witheve/eve-experiments/blob/master/design/language.md ; https://chris-granger.com/2016/07/21/two-years-of-eve/ ; https://groups.google.com/g/eve-talk/c/YFguOGkNrBo (24 Jan 2018 wind-down) ; https://news.ycombinator.com/item?id=16227130 ; https://github.com/futureofcoding/futureofcoding.org/blob/6a53eefa8ed041051cd1645b14270daaae001b53/catalog/eve.html ; https://observablehq.com/@jashkenas/against-the-current-what-we-learned-from-eve-transcript (LIVE 2018 talk; fetch rate-limited today, cited from the search abstract)):

- Founded late 2013 by Chris Granger and Rob Attorri after Light Table; $2.3M from a16z in 2014; "30+ prototypes" in two years, v0.2 public 2016, v0.3, then a Rust runtime v0.4 released open-source with the shutdown on 24 Jan 2018 ("we weren't able to find a good home for Eve"; the team chose acquisition over another round because "it's hard to pitch a greenfield project like Eve, because there's no real great way to quantify the benefits of a language before it's been fully realized"). Granger at LIVE 2018: "34 environments and 24 compilers and 9 interpreters" by "four guys in a tiny little office".
- Model: "a variant of Datalog, based heavily on Dedalus and Functional-Relational Programming ... Picture a relational spreadsheet with I/O" (design/language.md). Everything is a record = an id with a set of attribute/value pairs; a program is a set of blocks, each `search` (pattern over all records) + `bind` (derived records that exist exactly while the search matches) or `commit` (records that persist until removed). Blocks have no order; the runtime finds a fixpoint; "a language without inherent order", "programs with no loops" (whateveis). Unified I/O: "Want to add something to the screen? Send a message to a Slack channel? Add data to the database? All of these would involve either `commit` or `bind`" (catalog). Events (#click, #keydown, HTTP responses) arrive as records; state is a view: a counter is a `count` over click records, never a mutable cell (Dedalus's temporal logic: facts are indexed by time, "now" is derived by rules). Literate: "programs look more like Word documents than code files".
- Stated limits, in their own words: "Eve is still very early. It can't handle large amounts of data or hundreds of blocks" (whateveis); HN (2018): "Eve was very different from any language in widespread adoption, to the point where using it didn't even feel like programming at times", which made it impossible "to quantify the benefits to others". The v0.2/0.3 runtimes were JavaScript in the browser running a full relational fixpoint per input event; v0.4 moved to Rust precisely because of that cost.

Successors that solved the engine half (not the surface half):

| engine | what it fixed | cost model (mechanism) | URL |
|---|---|---|---|
| Differential Dataflow / Materialize | incremental joins/aggregates over arbitrary lattice times; shared "arrangements" (indexed batches) | work per update ∝ |delta| x size of the matched arrangement slice; memory = all arrangements resident; user assembles incremental operators | https://materializedview.io/p/everything-to-know-incremental-view-maintenance |
| DBSP / Feldera (VLDB'23) | syntax-directed translation of ANY relational query (incl. recursion) into its incremental circuit over Z-sets; time is a single sequence | linear operators cost O(|delta|); joins O(|delta| x |indexed other side|); the paper's theorem gives the incremental version mechanically | https://docs.feldera.com/vldb23.pdf , https://arxiv.org/pdf/2203.16684 |
| Rel / RelationalAI | Datalog with a semantic optimizer (rewrites proven equivalent using schema/ontology) | query-time planning, not incremental per se | https://arxiv.org/pdf/2504.10323 |
| Datomic / DataScript | EAV facts, immutable, Datalog queries, "as of" time; no incremental views | full query per call over indexes; the log is the truth, indexes are derived | https://github.com/tonsky/datascript , https://docs.datomic.com/indexes/index-model.html |

What Bebop takes (mechanisms already in the tree):

| Eve mechanism | Bebop twin | how it maps, exactly |
|---|---|---|
| insert-only input log; state = views over the log | mvcc.bp:48 `upd` (new record + prev edge, never in place) + the §4b append-only arena; csheaf.bp:85 `insert` validates a record against its neighbours at insert time | every external event (token stream T78, mesh message T67, clock) becomes an appended record with a generation; a "counter" is a fold over the records of one type — the CSR bucket (§4f) IS Eve's index over a record attribute |
| everything is a record (DOM node, DB row, API response = same type) | §4a object model: header (layout digest, length, crc, generation) + cells; enums = [tag,payload] (bebop.bp:371) | one representation for persisted data, tokens, and messages; the layout digest plays the role of Eve's `#tag` |
| dependency graph rebuilt by the compiler, recompute what changed | M3 (SPEEDUP §3): activity bits (substrate.bp/spike.bp tzcnt drain), csheaf.bp:60 `check()` O(degree) revalidation, T107 incremental curve | a view's dirty set = the records whose generation > the view's generation; recompute only their dependents; crossover measured, not assumed (M3: only wins when k/N < 3% at Rust-quality constants, < 0.15% today) |
| `commit` vs `bind` | store commit (root swap, §4c) vs a derived array rebuilt from a table generation (§4f index with `generation` in its header) | `bind` = "stale when table.generation != view.generation" (rebuild or incremental); `commit` = append + superblock |
| unified I/O as records | T78 `.bt` token streams, T67 mesh, sys_export publish | writes to the world = appending a record of an I/O type that a driver block consumes; the driver is ordinary .bp |
| time-travel / "as of" | superblock generations + `prev` refs; T73 rollback for the volatile arena | free with the append-only design; Eve had to keep every fact indexed by time in RAM |

What Bebop must NOT take, with the mechanism-level reason:

1. The surface. Eve made the LOGICAL model the only way to write anything; a click handler was a Datalog block with fixpoint semantics, and users reported it "didn't even feel like programming". Bebop keeps `fn`, `let`, `while`, direct field access and compiled queries (T32) — the record model lives in the TYPES and the STORE, the control flow stays imperative. Datalog-style rules are one library (a fold over a bucket), not the language.
2. Whole-graph incremental recomputation as the default engine. Eve's v0.2/0.3 re-ran the relational fixpoint over all blocks per event in a browser; the cost is ∝ (number of blocks) x (size of touched relations) with an interpreter constant, and no k/N regime was enforced. Bebop's numbers say the same thing in cycles: the sweep engine is 41x slower than linear code on straight-line work (T55 spike) and pays off only when the changed fraction is below ~3% (M3). So: incremental only for sparse deltas (T106/T107), full recompute (a compiled scan at 18 ns/row today, ~1.4 ns/row at Rust quality) otherwise — the compiler decides per site with a measured crossover, never a global fixpoint.
3. "No order" as a semantic. Fixpoint over unordered blocks needs stratified negation, aggregation semantics and termination proofs (Dedalus's whole point); rewrite.bp already restricts itself to confluent+terminating rule sets for that reason (T31). Bebop's programs stay ordered; only the STORE is order-free (a set of records with generations).
4. Everything resident and indexed by time. Eve kept the entire temporal database in browser memory; Bebop keeps one mapped file with a bump arena, and Cheney compaction drops history unless a type asks for `prev`.
5. Building the environment before the engine. 34 environments, 24 compilers; the runtime that could carry the model (Rust v0.4) arrived the month the company closed. Bebop's order is the opposite by law (gates + oracles first, T36/L17; surface tasks T43/T47/T48 before speed tasks T101-T108).

---

## 8. The "four taxes" thesis for 100x over SQLite — measured on this box

Method: `scratchpad/sqlite_decomp.py` (python ctypes over libsqlite3 3.46.1, in-memory DB, the
T100 data: 1M LCG points, 1000 queries, `taskset -c 4`), reading `sqlite3_stmt_status(..,
VM_STEP)` per statement and `EXPLAIN` opcode lists. Raw results:

| statement | us/query (incl. ctypes) | ctypes calls | native us (minus 1.17 us/call) | VDBE steps | rows |
|---|---|---|---|---|---|
| ctypes floor: 13 `bind_int64` | 15.2 | 13 | — (= 1.17 us per C call) | — | — |
| `SELECT 1` | 2.05 | 2 | ~0 | 5 | 1 |
| PK point lookup `WHERE id=?` (random id) | 5.31 | 4 | ~0.6 | 9 | 0.95 |
| same, cache-hot id | 4.10 | 4 | ~0 | 9 | 1 |
| one cell bucket `WHERE cell=?` (covering index) | 5.77 | 4 | ~1.1 | 13 | 0.93 |
| 9-cell `IN`, id only (covering index, no ORDER BY) | 34.8 | ~20 | ~12 | 117 | 8.53 |
| 9-cell `count(*)` | 29.4 | 12 | ~15 | 113 | 1 |
| 9-cell `IN`, 4 columns decoded (rowid re-seek per row) | 50.2 | ~20 | ~27 | 152 | 8.53 |
| T100 window query (`IN` + `ORDER BY d, id LIMIT 1`) | 53.4 | 16 | **~35** | 262 (76 opcodes, 1 sort, ephemeral table for the IN list) | 1 |
| full scan `MIN(d)` over 1M rows | 157,700 | 5 | 157,700 | 14,000,013 = 14 steps/row | 1 |

Derived constants: **11 ns per VDBE step** (157.7 ms / 14.0M steps); **158 ns per scanned
row**; a covering-index seek ≈ **1.0 us**; a rowid seek into the table plus a 4-column
record decode ≈ **1.8 us** (27 - 12 = 15 us for 8.5 rows); statement setup for the IN-list
(9 x MakeRecord/IdxInsert into an ephemeral b-tree) + the top-1 sort ≈ 4-7 us. The T100
"55 us" is therefore ~35 us of sqlite and ~19 us of python calling into C; bebop's 4.0 us
is **~9x native sqlite**, not 13.8x. (SPEEDUP §4.2's "~20-50 us C API" guess was right; the
gate's ctypes shim over-counts.)

Where bebop's 4.0 us goes (nnidx.bp:52-66, per query): 9 buckets x (rp[c], rp[c+1] = 1 cache
line) + ~8.5 points x (ci[j] line, us[i] line, vs[i] line) ≈ 9 + 26 = ~35 random DRAM lines
across four 8 MB arrays (32 MB working set >> 1 MB L2) at ~100 ns each ≈ 3.5 us. **bebop's
point query is already at the DRAM-latency floor of its own layout**; codegen (P1-P5) cannot
move it, only fewer misses can (records as {u,v} pairs: -8 lines; a Morton-ordered layout
that puts the 3x3 window in one line run: -20 lines).

The four taxes, one row each:

| tax | sqlite cost here (measured / bounded) | bebop cost after P1-P5 | mechanism | evidence | verdict on "100x" |
|---|---|---|---|---|---|
| (1) parse/plan | 0 per query with a prepared statement (setup opcodes Init/Transaction/OpenRead ≈ 5 steps ≈ 55 ns); the python wrapper's statement cache also hides it; a cold `prepare` is ~20-50 us | 0 (compiled) | sqlite compiles SQL to VDBE once; Bebop compiles the query fn once (T108 digest memo) | `SELECT 1` = 5 VDBE steps; SPEEDUP §4.2 | overclaimed: it is already zero in the oracle; counting it inflates the ratio |
| (2) impedance / serialization | record decode = varint header walk per `Column` opcode: ~2 of the 14 steps/row on the scan (~25-40 ns/row); on the point query the decode is bundled with the rowid re-seek (1.8 us per row, of which decode ≈ 0.1 us) | 0: `ldr x0,[base,#off]` (§4f); `ref` deref +1 word | deterministic layout, offsets not pointers, no per-row header | scan: 14 steps/row x 11 ns; window: 4-column vs id-only delta | real but small on this data (3 i64 columns); the 10-100x number is for JSON/serde, not sqlite (§3) |
| (3) B-tree pointer chasing / cache misses | ~1.0 us per covering-index seek, ~1.8 us per table seek+decode: a 3-level B-tree over 8318 pages x 4 KB = 34 MB (> L2), each level a binary search touching 3-6 lines ⇒ ~10-15 misses per seek; the window query does 9 + 8.5 seeks ≈ 25 us of its 35 | CSR: 2 lines per bucket + 3 lines per point ≈ 35 misses ≈ 3.5 us; a {u,v} record layout ≈ 27 lines ≈ 2.7 us; hardware prefetch helps ONLY the sequential ci/us walk inside a bucket (8-9 adjacent points), not the 9 random bucket entries; NEON (2 x i64 lanes, no SVE) buys nothing on 8 points | same 100 ns DRAM miss on both sides; the win is the miss COUNT (3-4x), not the miss cost | nnidx.bp:25-46 layout; SPEEDUP §3 DRAM 12 GB/s, L2 ~1 MB | the true source of today's ~9x; capped at ~10-15x on random point queries by the ~25-35 line floor; 100x only if the working set fits L2 (≤ ~30k records) or queries have spatial locality |
| (4) VDBE interpretation | 11 ns/step: scan = 14 steps/row = 158 ns/row (100% of the scan cost); window query = 262 steps ≈ 2.9 us = 8% of its 35 us | scan: 18 ns/row today (18.4 ms/1M, T96 step 1), floor 1.4 ns/row (Rust, DRAM-bound at 16 MB); after P1-P5 (T101-T105) ~3-5 ns/row is the honest estimate; point query: ~50-100 ns of straight-line code, invisible under the 3.5 us of misses | register codegen vs a stack VM | SPEEDUP §1.3-1.4, §4.1; T100 rows | scan class: 8.6x today (158/18.4), ~30-50x after P1-P5, **112x measured at Rust quality (157.7 ms / 1.41 ms)** — this is the ONLY place 100x exists; point class: VDBE is 8% of sqlite's cost, so removing it is worth 1.09x |

Where the thesis holds and where it overclaims:

- Holds, with a measured number: scan-class queries. sqlite pays 14 VDBE steps + 2 record
  decodes per row (158 ns); a compiled scan pays one `ldp` + 3 ALU ops per row (1.4 ns at the
  DRAM limit). 112x is on the table once codegen reaches Rust quality; 30-50x after
  T101-T105; 8.6x today. Taxes (2)+(4) are the whole story here, (1) and (3) do not apply.
- Overclaims: point/index-class queries. sqlite's 35 us is 70% B-tree seeks (DRAM misses),
  8% VDBE, ~20% statement setup + sort, 0% parse. Bebop removes the setup/sort/VDBE (~10 us)
  and cuts the misses 3-4x (25 us -> 3.5 us). Ceiling ≈ 10-15x. "A few hundred ns" per point
  query (100-200x) requires the bucket AND the records in L1/L2: true for a 30k-record store
  or a hot working set, false for 1M random rows — the same physics that caps sqlite.
- Overclaims: "CSR + prefetch + NEON". Prefetch is a sequential-stream mechanism (the 8-9
  adjacent points of one bucket, already prefetched today); NEON on this CPU is 2 x i64
  lanes with no SVE (F-I), and pays only on 1-2-bit data (M6, hvham) — irrelevant to i64
  coordinates.
- Overclaims: updates. The append-only design (§4c) writes a new 40-byte record per update
  plus, in durable mode, an msync of the 4 KB page it lands on (measured ~100 us) — the
  same page-write amplification sqlite's WAL has (one 4 KB frame per changed page). No
  100x on the update path; parity at best, and the file-size row LOSES 2.2x (40 B vs 18 B
  per record) until packed cells exist.
- Overclaims: "zero-copy vs sqlite". On bytes moved both engines are at the page-cache/DRAM
  limit (M10: 1.0x). Zero-copy is a 10-100x claim only against text/serde formats.

Net: one honest sentence for the roadmap — "Bebop beats sqlite by ~10x on indexed point
queries (fewer cache misses, no VDBE/setup) and by 10-100x on scans (no interpreter; the
top of the range needs Rust-quality codegen), ties on updates and reopen, and loses ~2x on
file size." The "four taxes" are two taxes ((3) for point queries, (4) for scans); (1) is
zero in any prepared-statement oracle and (2) is worth tens of ns per row on integer data.

---

## 9. The operator's proposals (A) dimensional descent, (B) graph = tensor, (C) CRUD on it — mechanism by mechanism

Constants used throughout (this box, measured earlier): DRAM miss ~100 ns (SPEEDUP §3: 12 GB/s,
one A78 saturates it; a random 64 B line at ~10-20 outstanding misses ≈ 10-20 ns effective in a
pipelined loop, ~100 ns dependent); L2 ~1 MB per core; L1D 64 KB; no SVE, NEON = 2 x i64
lanes, no gather instruction (ARMv8-A NEON has no indexed load; gathers are SVE-only); 1M
records = 8 MB per i64 column (> L2). sqlite native costs from §8: covering-index seek ~1.0 us,
rowid seek + decode ~1.8 us, PK seek ~0.6 us, 11 ns/VDBE step, 158 ns/row scanned; sqlite
native insert of 1M rows ≈ 7.44 s - 6 ctypes calls x 1.17 us x 10^6 ≈ 0.42 s + 0.72 s index.

### 9.1 (A) "Hierarchical tensorization / dimensional descent" — is it an index?

Yes, and it has a name: a fixed-fanout arithmetic partition of the key space is a **grid file /
k-ary trie / radix tree**, and its 2-level instance ("vector = routing coefficients, matrix =
local cluster") is exactly the T100 bucket grid (nnidx.bp:7 `cellof` computes the router
coordinate with a shift; :25-46 builds rp/ci). It differs from a B-tree in ONE respect: the
router is computed by arithmetic on the key (shift/multiply), not by comparisons against
stored separators — the same idea as learned indexes ("index = a function of the key",
Kraska et al. 2018, https://arxiv.org/abs/1712.01208) and radix trees (ART,
https://db.in.tum.de/~leis/papers/ART.pdf). Arithmetic routing wins when the key
distribution is known and near-uniform (the LCG points are), and LOSES on skew (one cell
holding 10^5 points is a 10^5-point scan; a B-tree rebalances, a grid does not — that is why
Terrace stores high-degree vertices differently, https://people.eecs.berkeley.edu/~aydin/terrace.pdf).

Miss counts per operation, 1M records, i64 cells (the numbers that decide, not the names):

| operation | B-tree (sqlite, 4 KB pages, 3 levels, interior hot) | hash table (open addressing, 16 B slots, load 0.5) | dense array by id | 2-level router (grid + window), records in insertion order | same, records CLUSTERED by cell |
|---|---|---|---|---|---|
| exact key lookup ("does k exist / fetch k") | ~6-8 line misses (binary search inside the cold leaf page) + 9 VDBE steps ≈ 0.6 us measured | 1 slot miss + 1 record miss ≈ 0.2 us | 1 miss ≈ 0.1 us | grid is NOT an exact index unless the key is the cell: 1 line (rp) + 1 line (ci) + per candidate 2 lines (u,v) — a spatial hash | 1 + 1 + 1 line (record adjacent) |
| 3x3 window / small range | 9 seeks + 8.5 rowid seeks ≈ 25 us of 35 (measured) | not possible (no order) | not possible on (u,v) | ~35 lines ≈ 3.5-4 us measured (4.0) | ~9 rp lines + ~4 contiguous record lines ≈ 1.3 us |
| 1-D range scan over 10^4 keys | seek + leaf walk, 158 ns/row (VDBE) ≈ 1.6 ms | full scan only | contiguous if id-range | ci slice contiguous but us/vs gathers random: 2 misses/row ≈ 0.2-2 ms | pure stream: 24 B/row at 12 GB/s = 2 ns/row ≈ 20 us — this is the "kills random access" mechanism, and its name is CLUSTERING (sqlite has it too: WITHOUT ROWID / clustered index) |
| insert 1M | ~0.42 s native + 0.72 s index (page splits, memmove inside pages) | ~2 misses/row ≈ 0.2 s | append 24 MB ≈ 5-10 ms | append + counting-sort rebuild: 3 passes over 8-24 MB ≈ 5-10 ms at DRAM speed; bebop today ~50-100 ms (18-50 ns/op), after T101-T105 ~10-20 ms | + one permutation pass (24 MB) |
| delete 1 | seek + memmove within page ≈ 1 us | tombstone 1 miss | tombstone bit | physical: shift O(nnz) = 8 MB memmove ≈ 1 ms; tombstone: 1 bit (1 miss) | same |

So the 2-level router is a mechanism, already measured, and its honest ceiling per point query
is the ~25-35 lines of the window (§8) — 3.5 us — falling to ~1.3 us with clustering. Adding
MORE levels does not reduce misses: a 3rd level adds a line per level unless that level's table
fits in L1/L2; 1M leaves with fanout 1024 need exactly 2 levels and the 8 KB root is L1-hot —
which is also precisely why sqlite's 3-level B-tree keeps its interior pages hot and pays only
for the leaf. "Descent" beyond two levels is the B-tree, spelled with shifts.

The "scalar at the top" — when a root summary skips work in 1 cycle and when it is a metaphor:

| root summary | question it answers | cost | verdict |
|---|---|---|---|
| dense presence bitmap over the id space (1M bits = 128 KB, L2-resident) | "does id k exist" (and the tombstone mask of 9.3) | 1 L2 hit ≈ 4 ns; "is this 64-id block entirely dead" = 1 cycle on the word | mechanism — this IS the nilpotent-product-zero test scaled up: grass.bp's `I & J != 0` is a 5-bit version of it |
| per-block zone map (min/max of u and v per 4 KB block, 4 cells per block = 1 line per 8 blocks) | "can this block contain a key in [a,b]" | 1 line per 8 blocks; skips a block without touching it | mechanism (Parquet/ORC row-group statistics; sqlite has none) — add to the CSR object (§4f) |
| Bloom filter (10 bits/key, 1% FP; 1.2 MB for 1M keys) | "is key k definitely absent" | 1-2 misses | mechanism when the store is larger than L2 and misses dominate; not needed at 1M with a bitmap |
| crc32 / generation in the object header (§4a) | "did anything under this root change since g" | 1 word | mechanism (the incremental-view trigger of §7) |
| "Stokes energy sum" / any linear aggregate (tdgstokes.bp:25-40 boundary vs :43-57 interior) | "is the total conserved" — an AUDIT of a region in O(perimeter) | O(√R) | mechanism for audit only; a sum is not injective: it cannot answer "does k exist" or "which subtree holds k" — as a lookup router it is a metaphor |
| eigenvalue / spectral gap of a subtree | "has the structure changed a lot" | O(N) iterations (spectral.bp:86 topk is a power method) | metaphor for lookup; robustness tool at best |
| tensor-train / hierarchical-Tucker cores | approximate low-rank reconstruction of a SMOOTH dense tensor | TT-SVD is global O(N r^2); incremental TT-ICE / HT-RISE update by rank growth, i.e. approximation drift per update (https://arxiv.org/pdf/2211.12487 , https://arxiv.org/abs/2412.16544) | metaphor for a database: a key set is a sparse 0/1 tensor with no low-rank structure (1M random points in a 1024x1024 grid have rank up to 1024), and an index must be EXACT; the operator's own trap is real and their fix ("block-sparse tree of routers") is the k-ary trie above |

Verdict on (A): true-with-conditions. Mechanism = arithmetic routing + clustering + block
summaries (bitmap, zone map); metaphor = "search becomes arithmetic instead of branching" (a
router lookup is still a dependent load; the branch that vanishes is the comparison ladder
inside a B-tree page, worth ~5 misses per seek, not the miss itself), "boundary operators /
rank reduction" as lookup, and any claim that levels beyond two reduce memory traffic.

### 9.2 (B) "Graph and tensor are the same structure" — SpMV vs pointer chasing

The identity is exact and old: a property graph IS the sparse (src, dst, relation) tensor,
CSR is its storage, and BFS over a Boolean semiring IS iterated sparse-matrix x sparse-vector
(SpMSpV) — this is the GraphBLAS thesis (https://www.researchgate.net/publication/326910948_Implementing_Push-Pull_Efficiently_in_GraphBLAS).
The physics of one nonzero on this box, CSR with i64 ci/vv (16 B/nnz) and a dense x of 8 MB:
`ldr ci[k]` (sequential, prefetched, ~0), `ldr x[ci[k]]` (random gather: 1 line miss, ~10-20
ns pipelined because the loop has no dependence between iterations, ~100 ns if the address
chain is dependent), multiply-add, `str out[j]` (push: random scatter, another line) — i.e.
**10-40 ns per nonzero, memory-bound; NEON adds nothing** (no gather, 2 lanes). Per-edge cost
on CPUs in the literature is the same class: Ligra/GAP-class single-machine BFS runs at
~10-50 ns per edge per core on CSR (Beamer/Asanović/Patterson direction-optimizing BFS,
https://scottbeamer.net/pubs/beamer-sc2012.pdf ; Ligra, https://jshun.csail.mit.edu/ligra.pdf).
spectral.bp:26 `spmv_fp` is exactly this loop (pull order: rows outer, `out[j] +=` scatter
inner — a PUSH formulation per row; the `fp_mul` schoolbook costs ~30 ops per nnz today,
M12).

Where the tensor formulation beats pointer chasing, and where it is the same code:

| workload | pointer traversal (queue + adjacency walk) | SpMV / frontier formulation | who wins and why |
|---|---|---|---|
| single point query = one row (neighbours of v, or the T100 window) | `ldr rp[v]; ldr rp[v+1]; loop ci[k]` — one contiguous slice | e_v^T A = the SAME slice; there is nothing to batch | identical machine words; "contraction" is a metaphor for one row |
| BFS/traversal with a frontier of thousands of vertices | one dependent miss chain per vertex, ~100 ns/edge | frontier sorted by vertex id -> rows visited in address order, misses independent -> 10-20 ns/edge; switch to PULL (unvisited x A^T) when the frontier exceeds ~1/14 of the edges (Beamer's alpha=14, beta=24; Ligra's dense threshold m/20) | SpMV wins 5-10x by batching misses and by direction switching; Gunrock does the same on GPUs (https://arxiv.org/pdf/1701.01170) |
| PageRank / power iteration / sheaf Laplacian solve (sheafh0.bp) | n/a | streaming SpMV, 16 B/nnz -> 12 GB/s / 16 B ≈ 1.3 ns/nnz floor if x fits L2, 10-20 ns if not | memory-bound; 4 cores 1.0-1.4x (M5) |
| "message passing / GNN on the metal" | n/a | SpMM (matrix x dense feature block): the only place NEON pays (dense inner block, 2 x i64 or 4 x i32 lanes) | mechanism only when features are dense vectors per node |
| sqlite recursive CTE for the same BFS | 1 index seek per edge ≈ 1-2 us/edge + VDBE | — | this is where the 100x lives: ~1.5 us vs ~15 ns per edge |

The language-graph isomorphism (§4a): a `ref T` field IS an edge (src = the object's offset,
relation = the field index in the layout, dst = the target offset); a struct IS a row of the
(src, relation) fibre; an array of refs IS one CSR row. A compiled traversal of one hop is
`ldr x1,[x0,#(2+f)*8]; add x0,x0,x1,lsl #3` — 2 words, one dependent miss per hop — and that is
optimal for ONE path; SpMV is what you emit when the hop applies to a SET of objects (the
frontier): sort the set by offset, walk it in address order, let the OoO core overlap the
misses. "Business logic and storage merge into one instruction stream with no cache misses" is
half right: no serialization, no VDBE, yes; no cache misses, no — a 1M-node graph is 8+ MB per
column and every random hop is a line miss; register promotion (P1-P5) removes the stack-machine
words (51 -> 14 in K1), not the DRAM latency.

Verdict on (B): mechanism for bulk traversal (frontier-batched, direction-switching SpMSpV over
CSR — add it), identity for single queries (same words either way), metaphor for "no cache
misses" and for NEON on index gathers.

### 9.3 CSR mutation: the "row_ptr amortized O(1)" claim is false as stated

Inserting an edge into row r of a packed CSR shifts every element after `rp[r+1]` in ci/vv
(for 10M edges, 160 MB of memmove per insert ≈ 15 ms) and increments n - r row pointers; only
an append to the LAST row is O(1). The real mechanisms, with their measured trade-offs:

| mechanism | insert cost | traversal cost | snapshot / MVCC | source |
|---|---|---|---|---|
| PCSR (packed memory array with gaps) | O(log^2 N) amortized element moves; "orders of magnitude faster than CSR" | ~2x slower than CSR (gaps) | none | Wheatman & Xu, HPEC'18, https://itshelenxu.github.io/files/papers/pcsr.pdf |
| VCSR (vertex-centric PMA, slack per vertex proportional to degree) | better locality than PCSR for skew | ~CSR | none | https://webpages.charlotte.edu/ddai/data/dong-ccgrid-22.pdf |
| Aspen (C-trees: chunked purely-functional trees) | batch inserts, O(log) per batch element; lightweight snapshots by construction | slower than CSR (tree chunks), faster than Stinger/LLAMA | yes (functional) | Dhulipala/Blelloch/Shun PLDI'19, https://arxiv.org/abs/1904.08380 |
| Terrace (per-vertex structure by degree: in-place array / PMA / B-tree) | faster batch inserts than Aspen up to 1M | near CSR for low degree | partial | SIGMOD'21, https://people.eecs.berkeley.edu/~aydin/terrace.pdf |
| LSM levels of CSR (LSMGraph, BACH) | append to L0, merge down; 50x LiveGraph throughput | reads merge k levels | per level | https://arxiv.org/pdf/2411.06392 , https://www.vldb.org/pvldb/vol18/p1509-miao.pdf |
| edge log + periodic adjacency snapshot (GraphOne) | O(1) append | reads snapshot + log tail; "lower data freshness" | snapshot per rebuild | https://www.researchgate.net/publication/340434324_G_raph_O_ne_A_Data_Store_for_Real-time_Analytics_on_Evolving_Graphs |
| in-place edge vectors with resizing (LiveGraph TEL) | in-place, lowest throughput in the LSMGraph comparison | sequential per vertex | per-edge versions | same |

Pick for Bebop (the §4 arena is append-only, so the choice is forced): **edge log + tiered CSR
rebuilt by generation** = GraphOne with an LSM twist, all of it objects in the store:
- L_log: appended edge records (src, dst, rel), at most 4096 since the last L0 rebuild — a
  query scans this tail contiguously (32-64 KB, prefetched, ~2 ns/edge ≈ ≤ 10 us worst case).
- L0: a small CSR rebuilt from the tail every 4096 edges (counting sort 3 passes ≈ 12k ops ≈
  50-200 us today, 20 us after T101-T105), growing to 2^18 edges.
- L1: the big CSR; L0 is merged into it every 2^18 edges (rebuild over E: 10M edges = 3 passes x
  160 MB ≈ 40 ms at DRAM speed; today's codegen ~0.5 s, ~0.1 s after T101-T105).
- Amortized per edge: ~3 ops (L0) + 3 x (E / 2^18) ops (L1 merges) ≈ 10-120 ops for E = 1-10M
  ≈ 0.2-2 us/edge today, 20-200 ns after codegen; PCSR's log^2 N ≈ 400 element moves is the
  same class, with worse traversal. Staleness window: zero (the log is always read); rebuild
  latency is the only stall, and it is a background job publishing a new generation (§4c).
- A point query reads ≤ 3 row slices (L1, L0, tail) + the tombstone mask; all contiguous.
- Row_ptr "amortized O(1)" becomes literally true only for the log; everything else is
  rebuild-by-generation, which the roadmap's own CSR (csr.bp:19 `csr_from_edges`, capped at
  4096 edges by its 4096-cell working buffer) already does at toy scale.

### 9.4 (C) CRUD: delete via bitmask tombstones, and what "update = one store" hides

Delete: a presence/tombstone bitmap of N bits is N/8 bytes (128 KB per 1M records, L2-resident;
1.25 MB per 10M, DRAM). Delete = clear one bit = 1 line write. Scan-skip = `tzcnt` over live
words: AArch64 has no scalar popcount or tzcnt — `cnt` exists only on NEON (the hvham emitters
bebop.bp:584-663 already use `cnt` + `uaddlv`) and tzcnt = `rbit` + `clz` (2 words). spike.bp
uses SWAR popcount and a de Bruijn multiply (~12 and ~4 ops per word); bitset.bp:12-27 computes
`pow2(bit)` with a loop of up to 63 multiplications and then DIVIDES — ~100x slower than
`(w >> bit) & 1`, which the language now has (T42). A 64-record block that is all dead costs one
word test (1 cycle when the word is in L1); a scan over 1M records with 10% tombstones costs
15.6k word loads + the live records — the skip is real at WORD granularity, not at record
granularity (a live word still walks its 64 records with a mask). Compact the mask (rebuild CSR
without tombstones) when dead > ~25% (PCSR/PMA density thresholds are the same idea), which is
the same generation rebuild as 9.3.

Interaction with §4: the bitmap is a store object ([len] + words, layout `arr:i64`, crc32,
generation). Flipping a bit IN PLACE breaks snapshot isolation (a reader holding root g would see
a deletion committed at g+1) and breaks the object's crc. So a delete is a **block-CoW**: copy the
4 KB block holding the word into the append region, flip the bit there, publish a new block
table (one ref per 512-cell block: 32 refs for 1M bits) with the new root. Cost per delete batch
≈ 4 KB + 256 B append ≈ 0.5 us; per single delete the same (batch them). The same rule applies to
"Update of fixed-size values = one store into the contiguous array": true only WITHOUT snapshot
isolation (in-place, the mvcc.bp:48 rule "never in place" violated); with §4c it is a block CoW
of 4 KB (≈ 0.3-0.5 us, 100x write amplification for an 8-byte change, exactly sqlite's WAL frame
cost) — so the design gains one rule: **arrays larger than one block are chunked (block table +
refs), and an update CoWs one block**; small structs are re-appended whole (mvcc semantics).
Read: "offsets + vector instructions (max speed)" — offsets yes; vector instructions only on the
dense inner blocks (SpMM, distance on a clustered window), never on the index gather.
Create = append: solved (§4c), 24 B/record at DRAM speed.

### 9.5 Verdicts and what to add

| claim | verdict | add to §4 / §5 |
|---|---|---|
| (A) dimensional descent replaces B-tree/hash | true-with-conditions: it is a 2-level arithmetic router (grid file) = the T100 index; more levels = a B-tree with shifts; wins on uniform keys, loses on skew | §4f: clustering of records by index order (the range-scan mechanism), per-block zone maps (4 cells/block), a presence bitmap per table; a skew guard (cell count > threshold -> loud trap or sub-grid) |
| (A) scalar root skips subtrees in 1 cycle | mechanism for bitmaps/zone maps/crc; metaphor for Stokes sums, eigenvalues, tensor cores as lookup | zone map + bitmap in the CSR header object |
| (A) "search is arithmetic not branching" | half: the comparison ladder disappears (~5 misses/seek), the dependent load does not | — |
| (B) graph = tensor, CSR storage | mechanism (GraphBLAS identity) | — |
| (B) SpMV traversal beats pointer chasing | true for frontiers (5-10x by miss batching + push/pull switch at alpha=14), identity for single rows, metaphor for "no cache misses" and NEON on gathers | a `frontier` library: sorted frontier, push/pull switch, over the tiered CSR |
| (B) language-graph isomorphism | mechanism at the object level: ref = edge, struct = fibre row, array of refs = CSR row; a hop = 2 words | already §4a/§4f; state it in the type rules |
| (B) row_ptr update amortized O(1) | false as stated; true for the log only | §4f: edge log + L0/L1 tiered CSR rebuilt by generation (9.3) |
| (C) create = append | mechanism | — |
| (C) update = one store | true only without snapshot isolation; with MVCC = 4 KB block CoW | §4: chunked arrays + block table; batch updates |
| (C) delete = tombstone bitmap, O(1) | mechanism at word granularity; compaction at ~25% dead; bitmap must be block-CoW'd, not flipped in place | §4: tombstone bitmap object; rebuild trigger; fix bitset.bp to shifts (100x) |
| "no serialization, no VDBE, no cache misses" | 2 of 3 | — |

New gate G8 `sgraph` (python oracle: networkx-free BFS with dicts, levels fold + counts):
1M nodes, 10M edges from the LCG (uniform; plus a skewed variant with 1% hubs), stored as the
edge log + tiered CSR of 9.3 in the §4 store. Rows: (a) BFS from 100 sources — pointer/queue
traversal vs frontier SpMSpV with push/pull switch vs sqlite `WITH RECURSIVE` over an indexed
edge table (expected ~15-40 ns/edge vs ~1.5 us/edge: the 100x row of this whole exercise);
(b) insert 1M edges through the log with L0/L1 rebuilds, amortized ns/edge and max stall ms;
(c) delete 10% via the tombstone bitmap, then BFS again (fold == oracle on the pruned graph),
then compaction (file size, time); (d) point query "neighbours of v" before and after
compaction (≤ 3 slices). Pass = folds == oracle; numbers reported whatever they are (D8(5)).

---

Appendix — measurement script: `scratchpad/mmap_phys.py` (python 3, stdlib mmap/os/zlib/
hashlib; 64 MB working set; taskset -c 4). Raw lines: anon first-touch 2.779 us/page;
file MAP_SHARED first-touch write 7.27 us/page; msync 64 MB 109.6 ms; msync 1 page 101.5 us;
write+fsync 4 KB 130.7 us; write+fdatasync 4 KB 89.5 us; warm read fault 0.53 us/page (0.25
loop floor); MAP_PRIVATE CoW 3.80 us/page; MAP_POPULATE 64 MB 5.16 ms; post-populate 0.254
us/page (= loop floor); zlib.crc32 2.42 GB/s; sha256 1.24 GB/s; blake2b 0.46 GB/s; rename
267 us.
