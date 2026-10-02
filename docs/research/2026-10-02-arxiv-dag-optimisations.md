# arXiv on DAG architecture and deeper optimisation, read against what dowiz actually does

Research, 2026-10-02. Lane W-ARXIV (Fable), READ-ONLY: no code, build, test, deploy or git command; the one file
written is this report. Tree read at `/root/dowiz` (main `eb783904`); every dowiz claim carries a `file:line` read
today, every paper claim an arXiv id whose abstract was read (metadata corpus of 1,187 papers,
`scratchpad/arxiv/corpus.jsonl`; no full texts were fetched). Labels: **MEASURED** = a number quoted from a dated
measurement document or command in the tree (its file beside it); **CITED** = what an abstract says; **ESTIMATE** =
arithmetic on named inputs; **(not in corpus)** = a system named from general knowledge because the query set did
not pull it -- it is context, never evidence. Companions this report builds on and does not repeat:
`docs/research/2026-09-27-dag-architecture.md` (DA), `docs/research/2026-09-28-bebop-dag.md` (R),
`docs/design/BLUEPRINT-BEBOP-DAG-2026-09-28.md` (BP), `docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` (BN),
`docs/research/2026-10-01-cloudflare-free-cost-and-rust-web.md` (CM).

---

## 1. Summary

1. **dowiz already implements the core of the incremental-computation literature** -- generation-keyed memos
   (`workers/api/src/hubdo/menu.rs:36-61`), a stepped orders projection fed by the write itself
   (`workers/api/src/fold/projection.rs:1-30`), prefix-extendable log folds (`crates/bebop-store/src/proj.rs:100-112`),
   semi-naive Datalog on a dirty set (`bebop-lang/selfhost/std/dl_eval.bp:1-25`), a confluence + memo-soundness proof
   (`bebop-lang/formal/Bebop/Dag.lean:184-198`). What the literature adds is at the EDGES of that design.
2. **AX1 Deletion-aware Datalog** (Counting for non-recursive strata, DRed/B-F hybrid for recursive): a delete in a
   recursive stratum today recomputes the stratum whole (`dl_eval.bp:624`). Asymptotic gain O(stratum) -> O(affected);
   product gain small today (recipe edits are rare). Confidence high on the algorithm, low on the payoff.
3. **AX2 Always-a-delta `since=`**: the 256-entry ring answers `None` after any gap and the client refetches the whole
   list (`hubdo.rs:109,126,1098-1105`). A bounded generation-indexed delta store makes every poll O(changed). Cuts
   bytes across the hop and Worker CPU per poll. ESTIMATE 2-10x on poll bytes at a busy venue; confidence medium.
4. **AX3 Early cutoff on node keys**: a write that leaves a projection's `K64` unchanged must not invalidate its
   dependents, push `moved`, or (after BN2) rewrite R2 objects. Cuts R2 Class A writes and socket wakes. Confidence high
   that it is correct; gain ESTIMATE, depends on edit mix.
5. **AX4 Persist the orders projection as a PROJ node** so a cold object reads KB of memo, not the log: the bebop store
   has the mechanism (`proj.rs:1-20`), `hubdo.rs:219` keeps it in memory only. Gain = the cold-wake fold (MEASURED
   5.75 ms at 5,400 events, R §4; 120 ms at 32k, `projection.rs:7`) and the image read. Gate: measure read-only wakes first.
6. **AX5 Ring-valued analytics** (DBToaster/F-IVM): per-(day,dish) counters stepped +1/-1 by the same `Written` events,
   replacing the per-request fold over the order list (`hubdo/reads.rs:47-55`). O(1) read; small today, grows with window.
7. **AX6 Why-provenance for the two owner-facing rules** (`unavailable`, `allergen`): tractable for non-recursive rules
   (CITED 2303.12773), ~1.3x overhead in Souffle (CITED 1907.05045). Product value, not CPU.
8. **AX7 Shred before replicate**: once BN3 puts orders in IndexedDB, in-place redaction (`crates/dowiz-hub/src/forget.rs:1-27`)
   cannot reach the copies; only key deletion (DG10, `shred.rs`) can. Order DG10w before BN3. Compliance, not speed.
9. **Rejected with the number**: CRDTs (single writer per venue), differential-dataflow/DBSP engines in the object (5 rule
   sets x 165 rows), e-graph/superoptimiser rows (bebop is not on the request path), zero-copy IPC (no shared memory
   between a Worker and an object), worst-case-optimal IVM (165 rows), incremental topological ordering (12 static edges).
10. **Corpus**: 1,187 papers, 38 query tags; ~21 % off-topic by a keyword screen, up to 80 % in the `zerocopy` tag; 116
    abstracts read, 94 cited below, the rest discarded by rule (§6).

---

## 2. Map: research line -> where dowiz stands -> best papers -> gap

### 2.1 Incremental / self-adjusting / demand-driven computation

**dowiz today.** Two memo disciplines coexist. (a) *Key-by-input-generations*: the menu projection is folded once per
`(catalog, i18n, settings)` generation tuple and a hit is "three map lookups" with no storage call
(`workers/api/src/hubdo/menu.rs:19-61`). (b) *Step-by-the-write*: the orders projection is extended by exactly the events
the write landed -- `Written::Appended` applies one `fold_one`, `Written::Log` applies the k newest after a digest check
of the older history, `Written::Whole` drops the memo (`workers/api/src/fold/projection.rs:14-30, 165-198`); the
in-store twin extends a log memo by one `log_step` per newer record while the memo's tip is still in the chain
(`crates/bebop-store/src/proj.rs:15-20, 102-112, 212`). Both are checked against a from-scratch fold by `rebuild`
(`workers/api/src/rebuild.rs:96-160`, `stale`/`stranded`) and, in bebop, by the `dagfull` gate. The compiler's memo is
per-fn with a content key and a byte-exact hit test (`bebop-lang/selfhost/prelude/dagc.bp:1-20`: "K64 ... an INDEX,
never a proof; a hit is decided by the bytes"). Lean proves memo soundness and confluence over the node graph
(`bebop-lang/formal/Bebop/Dag.lean:184-198`).

**Literature.** Self-adjusting computation's consistency theorem -- "any two evaluations of the same program starting at
the same state yield the same result" and "self-adjusting programs are consistent with purely functional programming",
machine-checked in Twelf (CITED 1106.0478, 2011, c=7) -- is the statement `Dag.lean`'s `memo_sound`/`confluence` make for
dowiz's graph. Adapton-style demand-driven + incremental evaluation handles cyclic dependency structure (widening loops)
and proves demanded results equal batch results (CITED 2104.01270, 2021, c=27); miniAdapton is the 1-page core
(CITED 1609.05337). Parallel self-adjusting computation tracks series-parallel control dependencies to propagate changes
in parallel with work/span bounds (CITED 2105.06712, 2021, c=5). Incrementalisation as "the discrete counterpart of
differentiation" (CITED 2312.07946) and change actions / derivatives of fixpoints with "particular application to
generalised Datalog" (CITED 1811.06069, 2018, c=13; 1902.05465) are the algebra behind both DBSP and semi-naive. Shortcut
memoization learns repeated computation patterns across program versions (13x speedup, 20x memory -- CITED 2603.19560).
Adapton (Hammer 2014), Salsa and "Build systems à la carte" are **not in corpus**; DA §6.1 already covers Salsa's
durability idea.

**Gap.** Small. dowiz's memos are at the granularity the literature recommends for its sizes (function/projection, never
expression -- R §1.1's 41x refutation of the cell substrate). Two edges remain: *early cutoff* (a recompute whose output
bytes are unchanged still propagates -- §3 AX3) and *persistence of the memo across eviction* (§3 AX4). Demand-driven
evaluation is already the pattern (`menu_memo` builds on first read); nothing to add.

### 2.2 Differential / timely dataflow, DBSP, Z-sets

**dowiz today.** Events are deltas (`"_d": true`, DA §1.3) over a single totally ordered log per venue; `fold_one` is the
incremental operator for one order. No general operator graph exists and the companion reports argue none should
(BN §4.2: "over-engineered at this size; the semi-naive dirty-set loop is the same asymptotics with ~200 lines").

**Literature.** DBSP gives one algorithm that incrementalises "the full relational queries, grouping and aggregation,
monotonic and non-monotonic recursion, and streaming aggregation" (CITED 2203.16684, 2022, c=53); OpenIVM compiles views
to SQL "following the principles of DBSP" (CITED 2404.16486). Differential dataflow makes deletions as cheap as insertions
in Datalog materialisation (CITED 2308.04214) but can need "a prohibitively large amount of memory" of retained
differences, which 2208.00273 drops and recomputes on demand. Shared arrangements let concurrent queries reuse one
indexed state (CITED 1812.02639). Flo formalises the two properties every streaming system shares -- progress and eager
execution -- and models DBSP and Flink in it (CITED 2411.08274, 2024, c=8). The free-termination property says when a
node may answer without coordination (CITED 2502.00222); CALM + CRDT (CITED 2210.12605) says monotone queries over
replicated state are safe to read coordination-free. Workset ("incremental") iterations in dataflow gave up to two
orders of magnitude on sparse dependencies (CITED 1208.0088, 2012, c=168).

**Gap.** Two ideas worth lifting without the engines. (1) **Z-set weights** (a relation as a map to integer
multiplicities) make deletion an insertion with weight -1 and make `fold_step` for counted aggregates a sum -- this is
AX1's non-recursive half and AX5's data structure. (2) **Monotonicity as the safety argument for the browser replica**:
the replica folds a prefix of a single-writer log, which is monotone, so by CALM it needs no coordination to be
consistent with the server -- the design `replica.js:14-18` states informally ("the server is still the authority")
has a theorem behind it; DW1's "replica disagrees" check is then only a corruption detector, not a conflict resolver.

### 2.3 Incremental Datalog: semi-naive, DRed, Counting, provenance

**dowiz today.** Rules are data; relations are sorted deduplicated row blocks; strata = Kahn order over the SCC
condensation; negation inside a cycle is refused with exit 125 (`bebop-lang/selfhost/std/dl.bp:1-24, 394, 443`).
Event propagation: a non-recursive stratum computes CANDIDATE heads from one changed row and re-derives each
("inserted when it now holds, deleted when it no longer does", `dl_eval.bp:13-19, 547`); a recursive stratum with
inserts only continues semi-naively (`:592`); a recursive stratum with a delete, or any change under a negation, is
"recomputed from its complete lower strata and diffed against a snapshot (correct, not fast ...)" (`:19-22, 624`).
Lean: `seminaive_eq_naive`, `seminaive_lfp`, `neg_strictly_lower` (`bebop-lang/formal/Bebop/Datalog.lean:117, 138, 194`). Join
plans are generated per rule and baked into bebop fns (`gen_dl.bp:1-20`); the sushi rule set is generated from the
menu (`dl_sushi.bp:1-5`). MEASURED: one stock event -> its dishes in 82 ns (Rust) vs 3,316 ns naive re-derive,
independent of venue size (R §12.3); bebop target `dl_event_ns <= 1,000` (`bebop-lang/ROADMAP.md:233`).
Relations are in memory, not yet store projections (`dl.bp:30-31`; DGSW lane owns the binding).

**Literature.** The candidate-then-re-derive step dowiz uses is the *Backward/Forward* idea; Motik et al. show that
evaluating rules "backwards" (matching heads to facts, running partially instantiated bodies as queries) "can be a
considerable source of overhead even on very small updates", that *Counting* avoids it but handles only non-recursive
rules, and that DRed/B-F + Counting hybrids are "usually significantly faster ..., sometimes by orders of magnitude"
(CITED 1711.03987, 2017, c=18). Modular materialisation hands transitive closure (and symmetric-transitive closure) to a
specialised algorithm inside the semi-naive framework, "often by orders of magnitude" (CITED 1811.02304, c=12) -- dowiz's
`allergen` (transitive through `produced(S',S)`) and `personal` (closure over the edge table) are exactly closures.
Datalog over semirings: semi-naive is valid on a characterised class of ordered semirings (CITED 2105.14435, c=52);
grounding-based evaluation gives tight data-complexity bounds (CITED 2403.12436); linear programs converge in
O(p n^3) rounds on p-stable semirings (CITED 2311.17664). Provenance: why-provenance is intractable for recursive but
"highly tractable" for non-recursive Datalog (CITED 2303.12773, c=22); Souffle's provenance evaluation costs 1.27x
(CITED 1907.05045); semiring provenance for Datalog has several non-equivalent semantics (CITED 2202.10766, c=21) and
cannot be extended to difference for all semirings (CITED 1105.2255, c=47); PUG computes why/why-not for queries with
negation by rewriting to a Datalog program (CITED 1808.05752, c=32). Extended magic sets make stratified-negation
programs demand-driven with precise complexity (CITED 1909.08246). egglog unifies Datalog and equality saturation with
incremental execution (CITED 2304.04332, c=73). Column-oriented materialisation with pro-active subquery caching matched
state-of-the-art "under restricted resources" (CITED 1511.08915, c=64). FlowLog separates recursive control from
per-rule logical plans over Differential Dataflow (CITED 2511.00865). Monotonic aggregates in recursion avoid
stratification where PreM holds (CITED 1910.08888).

**Gap.** (1) Deletes in recursive strata and under negation: full recompute today -> Counting (non-recursive) +
DRed/B-F hybrid or specialised closure maintenance (recursive) -- AX1. (2) Provenance -- AX6. (3) Aggregates: `dl.bp`
has none (no `sum`/`count` in the rule language, grep today); stock totals and revenue live in Rust folds. A semiring
annotation (natural numbers for counts, i64 minor units for money) is the DBSP/F-IVM shape and the AX5 shape; it is
NOT recommended for the FSM or prices (R §12.3: kernels are microseconds). (4) Column layout: dowiz's blocks are
columnar already (DG7); `dl.bp` rows are 4-cell tuples with per-column perms (`dl.bp:22-28`) -- at the literature for
165 x 120.

### 2.4 Incremental view maintenance

**dowiz today.** Twelve projections over source images (DA §1.1 table); orders and menu memoised, analytics and the
relation graph recomputed per read (`hubdo/reads.rs:47-55` folds the owner's report from the memoised order list on
every call; `services/analytics/fold.rs:1-6` "NO ANALYTICS STORE"). `rebuild` is the view-consistency check.

**Literature.** DBToaster's viewlet transform materialises a query and its higher-order deltas so views "support each
other's incremental maintenance", tens of thousands of refreshes per second (CITED 1207.0137, 2012, c=249). F-IVM adds
factorised computation and a ring abstraction: views are maps from keys to ring payloads, so group-by aggregates,
covariance matrices and matrix chains share one maintenance scheme, "orders of magnitude" over first-order IVM with
less memory (CITED 2303.08583, c=13; survey 2404.17679). Worst-case-optimal update time needs a space-time trade-off
(CITED 1804.02780; heavy-light partitioning generalises it, CITED 2605.08397); constant-time maintenance under inserts is
characterised per semiring (CITED 2606.07795). Stale-view cleaning answers from a sample when maintenance is deferred
(CITED 1509.07454). Enzyme (Databricks) reports "billions of CPU seconds" saved daily from cost-based refresh planning
over pipelines of views (CITED 2603.27775). ActorDB proposes exactly dowiz's shape -- single-writer actors + IVM --
as an architecture paper with an MVP plan and no numbers (CITED 2509.25285, 2025, c=0).

**Gap.** The analytics report is the one owner-facing view that is a per-read fold; a ring-valued (day, dish) ->
(count, revenue) view stepped by the same `Written` events is first-order IVM with a trivial delta (AX5). Everything
heavier (higher-order deltas, factorised joins, worst-case-optimal structures) is sized for joins over millions of
rows; dowiz's largest relation is the 30-day hot log (~5,400 events, R §4). Rejected below.

### 2.5 Build systems and incremental compilation

**dowiz today.** The bebop compiler memoises per function in a store image with a content key, an exact-bytes hit test
and context bytes (`dagc.bp:1-30`); the `.becache` whole-program record stays as the outer key (R §1.1); the switch
(DGSW) is in flight with the 389/389 sweep and 418/418 compile arms green (`/root/lanes/w-roadmap/docs/design/ROADMAP-2026-09-22.md:89`).
On the dowiz side, the twelve projections form a static edge table that is not yet written down as data (DW2's
`edges.rs`, queued).

**Literature.** The corpus is thin here (11 `buildsys` hits, 2 `salsa`, both off-target). LaForge builds from a plain
script by tracing execution into TraceIR and replaying the trace to find what changed, "always-correct" incremental
builds without declared dependencies (CITED 2108.12469, 2021, c=4). Hybrid incremental compilers reuse a
non-incremental compiler by staging and an internal build system (CITED 2002.06183, c=8). Co-contextual typing removes
context dependence so type checking is compositional and incremental (CITED 1705.05828). Online topological ordering
under edge insertion is O(n^2 polylog n) average-case (CITED 0802.1059). Salsa, Shake, Pluto and Mokhov et al.'s
taxonomy (verifying/constructive traces, early cutoff) are **not in corpus**.

**Gap.** *Early cutoff* -- "if the rebuilt output equals the old output, do not rebuild dependents" -- is the one
build-system property dowiz's generation-keyed memos lack (a settings write that changes no menu byte still produces a
new catalogue generation, a new `Memo`, and after BN2 a new manifest). AX3. Verifying traces (store the hash of inputs
and output, re-verify cheaply) are what `K64 = (crc32 << 32) | len` of a frame already is (`nodekey.rs:11-14`).

### 2.6 DAG / task-graph scheduling

**dowiz today.** The L3 scheduler claims nodes from a per-level atomic counter, assembles outputs in node-index order,
wakes the parent once per level and runs a level serially unless its estimate is >= 10 ms per worker
(`bebop-lang/selfhost/prelude/sched.bp:4-20, 34, 119`). MEASURED 2.55-2.7x on three cores for >= 40 ms levels, 1.0x for
0.1 ms tasks, 0.85x on a chain (R §3.3). On the Worker side there is no scheduler: one request = one object turn.

**Literature.** "Even a completely random scheduler is surprisingly competitive" with Dask's hand-tuned one; "the main
bottleneck of Dask lies in its runtime overhead", fixed by a Rust server with a simpler algorithm (CITED 2010.11105,
2020, c=16). DAG scheduling in the BSP model is NP-hard already for in-trees or height-2 DAGs and APX-hard in general
(CITED 2303.05989). Decentralised list scheduling pays a bounded extra term for a distributed list (CITED 1107.3734).
Taskflow's in-graph control flow gave 29 % over oneTBB on 40 CPUs (CITED 2004.10908, c=166). Serverless DAG engines:
Wukong's decentralised scheduling on Lambda cut network I/O by orders of magnitude and cost by 93 % vs numpywren (CITED
2010.07268, c=149; 1910.05896); Pheromone's data-centric triggers cut function-interaction latency "by orders of
magnitude" (CITED 2109.13492, c=95); Triggerflow builds DAG/state-machine schedulers on event triggers (CITED
2006.08654); Fusionize fuses small functions to avoid invocation overhead and cold starts (CITED 2204.11533).

**Gap.** None to build. The literature's two robust findings -- runtime overhead beats scheduling cleverness, and
fuse small tasks -- are what T-4's 10 ms threshold and BN4's "one command, one object turn" already encode. Pheromone's
"follow the data" is DA's rule "an edge carries a derived node, never a source", now at `dataflow` = 23 call sites
(`tools/gates/dataflow.baseline`, grep `load_catalog(` = 23 today) from 38.

### 2.7 Event sourcing, log-structured storage, CRDTs

**dowiz today.** Append-only, content-chained log (`crates/bebop-store/src/evlog.rs:1-12`; v2 packs eight bytes per
cell, `:24-37`); readers walk the chain oldest-first (`:321`) unless a PROJ memo answers; KV images are rewritten whole
on put and `Kv::get` is still a linear scan over sorted entries in main (`crates/bebop-store/src/kv.rs:246-247`; BN7
in flight, not merged); forgetting is in-place redaction with the link kept and a declared count
(`crates/dowiz-hub/src/forget.rs:1-27`), crypto-shredding for new logs landed on the hub side (`shred.rs`, DW8 hub half);
no CRDT -- one object is the single writer per venue (arch-evo P6 AGAINST).

**Literature.** Event-sourced systems in industry name five challenges: "event system evolution, the steep learning
curve, lack of available technology, rebuilding projections, and data privacy", with versioned events, weak schema,
upcasting, in-place transformation and copy-and-transform as the evolution tactics (CITED 2104.01146, 2021, c=25).
LSM compaction is a four-primitive design space (trigger, layout, granularity, movement policy) (CITED 2202.04522, c=88;
survey 1812.07527, c=230); Lethe makes deletes first-class with latency guarantees "on the right-to-be-forgotten"
(CITED 2006.04777); log-structured cleaning order approximates an optimal policy (CITED 2005.00044). CRDTs: a JSON
CRDT for nested maps/lists (CITED 1608.03960, c=128), delta-state CRDTs for small incremental messages over unreliable
channels (CITED 1603.01529, c=118), pure op-based CRDTs with a partially ordered log (CITED 1710.04469), Merkle-DAGs as
logical clocks for CRDTs (CITED 2004.00107), just-right consistency with bounded counters (CITED 1801.06340), verified
strong eventual consistency (1707.01747, not read in full -- title only, excluded from the bibliography).

**Gap.** (1) The industry study's "rebuilding projections" and "data privacy" are dowiz's `rebuild` law 8 and
DG10 -- at the literature; its evolution tactics match `_d` deltas + the versioned payload byte (BN6). (2) Lethe's
point -- a deletion SLO -- becomes real the day BN3 puts copies on devices: AX7. (3) Compaction: the store's Cheney
compaction and `HOT_KEEP_MS` rotation (DA §2 row 8) are a single fixed policy; the LSM literature's lesson is only that
the policy should be named and measured, which the G7 row does (R §11.1: 747 ms per 85 MB). No change.
(4) CRDTs stay rejected: one writer per venue, and delta-CRDT's one useful idea (ship small deltas joined to state) is
AX2 without the merge function.

### 2.8 Zero-copy formats, arenas, cache-conscious and columnar layout

**dowiz today.** The catalogue block "is at once the CAS unit, the wire format and the in-memory format: a reader
views a field with one bounds check and one `from_le_bytes`" (`crates/dowiz-hub/src/block/mod.rs:3-6`; `view.rs:1-6,
31-40`); MEASURED decode 8,066 ns in the tree vs JSON parse 6.33 ms (BN §3.1); the `Store::from_bytes` copy is 70 µs per
538 KB, 1 % of the parse (BN §3.1). The bebop store is a pointer-free arena (`store.bp`), objects carry layout digest +
crc (R §1.1).

**Literature.** Serialisation is the cost, not the copy: Roadrunner's shim maps function memory to give
"serialization-free data transfer between WebAssembly-based serverless functions", 44-89 % latency cut, 69x throughput
(CITED 2511.01888, 2025, c=3); interpreted Wasm pays "up to 10x I/O-serialization overhead" (CITED 2510.05118); TZC's
partial serialisation keeps IPC cost constant in message size (CITED 1810.00556, c=36). Columnar layouts for schemaless
LSM document stores (Dremel-style, piggy-backed on LSM events) improved query time "by orders of magnitude" with
minimal ingestion impact (CITED 2111.11517, c=11); Relational Memory transforms rows to columns in hardware (CITED
2109.14349) -- the FPGA half is irrelevant, the finding that projectivity decides row vs column is not.

**Gap.** None on the format: DG7 is the literature's answer and BN1 is finishing its adoption (23 sites left). Zero-copy
ACROSS the Worker-object hop is impossible on this platform (no shared memory between isolates; the hop is a network
call, BN §1.1), so Roadrunner/Faasm-style sharing is rejected, not deferred. The one open item is the BN7 micro-fix
set (`Kv::get` binary search, borrow instead of `from_bytes`), already a row.

### 2.9 WebAssembly performance, serverless cold start, stateful edge / actors

**dowiz today.** A 4.75 MB wasm Worker (BN §5.1), 92 ms compile+instantiate on an A78 (MEASURED there); one Durable
Object per venue with hibernatable sockets (`hubdo.rs:437-458`); one alarm per object (`hubdo/timer.rs:1-14`); one
cron left (`workers/api/wrangler.toml:157`); clients still poll at 12-15 s (`public/store/track.js:38`,
`public/admin/core.js:28`); MEASURED 14.7 object requests per Worker request, CPU p50 6.8 ms (BN §1.3).

**Literature.** Wasm runs SPEC 45-55 % slower than native, peaks 2.08-2.5x (CITED 1901.09056, c=186) -- the tax DA/BN
already price in. Faasm shares memory between co-located functions via SFI and restores from snapshots (CITED
2002.09344, c=382); functions restored from snapshots run 95 % slower on average because state is faulted in one page
at a time (CITED 2101.09355, c=259); Faa$T pre-warms a per-application cache with "objects likely to be accessed"
(CITED 2104.13869, c=159); FaaSLight loads only indispensable code via the call graph (CITED 2207.08175, c=98); Azure's
production trace shows an 8-order-of-magnitude range of invocation frequencies and most functions invoked rarely (CITED
2003.03423, c=944); Huawei's trace: cold starts dominated by dependency deployment and scheduling (CITED 2410.06145).
Stateful serverless: Cloudburst's lattice-encapsulated caches co-located with executors (CITED 2001.04592, c=166);
Beldi's log-based fault-tolerant transactional functions (CITED 2010.06706); Durable Functions / Netherite's recovery
logs and asynchronous snapshots (CITED 2103.00033); GoldFish's short-term stateful Wasm actors at the edge (CITED
2412.02867); Histrio's exactly-once actor model over FaaS (CITED 2410.21793). A fast in-place Wasm interpreter trades
peak speed for startup (CITED 2205.01183). SQL compiled to Wasm in under a millisecond on V8 (CITED 2104.15098).

**Gap.** The Durable Object IS the stateful-serverless actor these papers build (single-threaded, co-located state,
hibernation), so the platform already gives dowiz Cloudburst/GoldFish for free. Two lessons transfer: (1) snapshot
cost is proportional to the WORKING SET faulted in -- keep the cold-wake read small, i.e. AX4 (read the projection,
not the log); (2) code-loading latency is the cold-start term an application can cut -- BN5's diet, already queued.
Faa$T's "pre-warm likely objects" is what the alarm already does implicitly when an outbox entry is due (DA §4.3 P6).

### 2.10 E-graphs, equality saturation, superoptimisation

**dowiz today.** No rewrite engine. bebop's emitter has hand-written strength reductions (e.g. multiply-by-constant
without a multiplier, `bebop-lang/docs/exp.journal` T104b); the Worker is compiled by rustc/LLVM with `wasm-opt`.

**Literature.** egg's rebuilding + e-class analyses (CITED 2004.03082); relational e-matching with worst-case-optimal
joins, "orders of magnitude faster" (CITED 2108.02290, c=25); egglog = Datalog + EqSat (CITED 2304.04332, c=73);
certifying equality saturation with small proofs (CITED 2209.03398); e-graphs + abstract interpretation for conditional
rewrites (CITED 2205.14989); Caviar's early-stop heuristics for compiler TRSs (CITED 2111.12116). Superoptimisers:
STOKE (CITED 1211.0557, c=394), Souper found optimisations shipped in LLVM/MSVC and shrank clang 4.4 % (CITED 1711.04422,
c=98), Minotaur +7.3 % on GMP (CITED 2306.00229), Souper over LLVM shrank the code section of 8 of 12 Wasm programs
(CITED 2002.10213, c=14).

**Gap.** Deliberately none. Nothing on a venue's request path is compiled by bebop (BN §4.2: bebop kernels 2.6-10.6x
slower than Rust, nowhere in the Worker), and the Worker's binary is sized by crates, not by missed peepholes (BN §5.2).
Only egglog is conceptually adjacent -- a Datalog with equivalence classes would model supply aliasing
(`produced(S',S)`) as congruence -- but `allergen` is a 2-step closure over ~120 supplies; a union-find in `dl.bp` would
do and is not needed today. Rejected in §5.

---

## 3. Proposals (roadmap-ready rows)

Every acceptance is a number in an existing gate or eval. "Free-plan effect" names which of the three walls
(Worker requests, object requests, rows written) or which CPU/bytes term the row moves. Labels: MEASURED basis cited;
ESTIMATE where no number exists yet -- none of the gains below has been measured by this lane.

| ID | Build | Files | Measure (gate, number to beat) | Expected gain | Risk | Papers |
|---|---|---|---|---|---|---|
| **AX1** | **Deletion-aware incremental Datalog.** Non-recursive strata: Counting -- a derivation count per IDB row, delete = decrement, drop at 0 (no candidate re-derivation, no backward evaluation). Recursive strata: DRed over-delete/re-derive or the B-F + Counting hybrid; closure rules (`allergen` via `produced`, `personal`) as a specialised transitive-closure module. Z-set weights (DBSP) are the same count with sign. | `bebop-lang/selfhost/std/dl_eval.bp` (`dl_inc_nonrec :547`, `dl_recompute_s :624`), `dl.bp` DESC layout (`:22-28`, one more cell per row), `gen_dl.bp` (count maintenance baked into generated walks), `bebop-lang/bench/oracles/dl_*.py`, `bebop-lang/formal/Bebop/Datalog.lean` (Counting correct for non-recursive strata) | `dl_event_ns` for a DELETE in a recursive stratum on the 165/120/352 fixture, today = the whole stratum (`dl_recompute_s`), target <= 10x the insert path; `dagfull datalog` incremental == scratch after 10^4 mixed insert/delete events; `arch_check` file-size 800 | Asymptotic O(stratum) -> O(affected). Product: ESTIMATE small -- recipe/supply edits are rare, stock events are inserts; the row exists so the engine is complete before DW2 puts `personal` (a closure, deletions on schema change) on it | Counting + negation needs care (counts under a negated atom are not derivation counts) -- keep DRed there; Lean statement grows | 1711.03987, 1811.02304, 2203.16684, 2308.04214, 1811.06069 |
| **AX2** | **Every `since=` answers a delta.** Replace the 256-entry in-memory ring with a bounded generation-indexed delta store (last N generations' `Change`s, N by bytes, kept in the object AND persisted beside the log so a cold wake still answers deltas); `changes_since` returns `None` only beyond the bound; clients apply `fold_one` (the replica already does). | `workers/api/src/hubdo.rs` (`RECENT_KEEP :109`, `changes_since :126`, the clear at `:1098-1105`, route `:1246`), `workers/api/public/lib/replica.js`, a new `+workers/api/src/hubdo/deltas.rs` (<= 300 lines) | New collector: fraction of `?since=` answered `None` (today unmeasured) -> <= 1 %; bytes per poll answered as delta vs the full list on qa-durres (`cf.do_response_bytes_day / cf.do_requests_day` already exists); `rebuild.stale = []` | Free-plan: bytes across the hop and Worker CPU per poll (the pass-through handler then forwards ~100 B instead of the list, DA §4 "double serialisation"). ESTIMATE 2-10x on poll bytes at a busy venue; counted requests unchanged (sockets, A2/A6, are the request lever) | A whole-image write (rotation, import, forget) still has no per-event delta: keep `None` there; the persisted ring is one more row written per turn only if written outside the existing `put_image` transaction -- write it inside | 2203.16684 (deltas as the unit), 1603.01529 (small deltas joined to state), 2210.12605 (monotone reads safe without coordination) |
| **AX3** | **Early cutoff on node keys.** Each projection records the `K64` of its output; a recompute whose `K64` is unchanged does not bump the projection's generation, does not send `moved`, and (after BN2) does not rewrite the R2 object or the manifest. Make it a rule of DW2's edge table: `(inputs, projection, step, out_key)`. | `workers/api/src/hubdo/menu.rs` (`Memo` gets `out_key`), `workers/api/src/fold/menu.rs`, the planned `+workers/api/src/hubdo/edges.rs` (DW2), `+workers/api/src/hubdo/publish.rs` (BN2: compare before write), `crates/bebop-store/src/nodekey.rs:87` (`k64` of the body) | A test: a settings write that changes no menu byte leaves the menu `K64` and generation unchanged and sends no `moved`; after BN2: `publish::tests::only_changed_blocks_are_written` extended to "no object written when no block changed"; `cf.do_response_bytes_day` and R2 Class A writes per edit on qa-durres | Free-plan: R2 Class A writes per edit (BN2 target <= 3 -> often 0-1), socket wakes, dependent recomputes. ESTIMATE; depends on the owner's edit mix (a price edit changes `menu_prices` but not `names`) | A key collision would hide a real change: `K64` is crc32-based (`nodekey.rs:11-14`); compare BYTES on equal keys (the `dagc.bp` rule) or use `K256` for the cutoff | 2108.12469 (trace-based change detection), 2002.06183, 1106.0478 (memo reuse consistency); Mokhov et al. "early cutoff" is the name (not in corpus) |
| **AX4** | **Persist the orders projection as a PROJ node** in the log image (DG5's `PROJTAB`, `proj.rs`), written in the same `put_image` commit as the append, so a cold object reads the memo (KB) instead of `Hub::load` + fold over the whole hot log. Gate the row on the §3.4 measurement DA asked for: the share of read-only wakes. | `workers/api/src/hubdo.rs` (`folded :219`, `orders_view :339-356`, `put_image` at `:1086`), `workers/api/src/fold/projection.rs` (serialise `Orders` as the PROJ OUT), `crates/bebop-store/src/proj.rs` (a kind for the dowiz orders view beside kinds 1-2), `workers/api/src/rebuild.rs` (law 8 checks the persisted copy) | FIRST a number: `do_reads_per_wake` and cold-wake count per day (BN8); THEN `/fold/orders` wall on a cold object before/after (DG19's probe); rows written per order unchanged (`cf.do_rows_written_day`); `rebuild.stale = []` nightly | MEASURED basis: full fold 5.75 ms at 5,400 events vs memo read ~µs (R §4); 120,776 µs natively at 32k events (`projection.rs:7`); plus the 2.9 MB image read a cold wake avoids. Free-plan: object CPU and wall on the first request after idle (DA P6: cold 260 ms vs warm 105 ms) | A memo is a second account of the truth -- bounded only by `rebuild` (DA §3.4); `forget` rewrites records in place, so the memo must drop on `Written::Whole` exactly as today (`projection.rs:24-27`) | 2101.09355 (snapshot cost = pages faulted), 2104.13869 (pre-warm likely objects), 1207.0137 (materialise what reads ask) |
| **AX5** | **Ring-valued analytics view.** Maintain `(local_day, dish) -> (orders, revenue_minor, refunds)` as a first-order IVM view stepped by the same `Written` events the orders projection applies (+1 on a terminal `Placed`-reaching order, -1/-amount on `COMPENSATED_REFUND`); the report becomes a read of the window's rows. Keep `services::analytics::fold` as the oracle (`rebuild`-style equality). | `workers/api/src/fold/projection.rs` (a second stepped memo beside `Orders`), `workers/api/src/hubdo/reads.rs:47-55`, `workers/api/src/services/analytics/fold.rs` (unchanged, becomes the check) | Native test: stepped view == `fold::fold` after 10^4 random events incl. refunds and timezone boundaries; `/fold/analytics` object CPU per call in DW6's `cost_us` (today unmeasured; the fold is over the 30-day order list) | O(window) -> O(1) per read. ESTIMATE small today (a few hundred envelopes, `fold.rs:4-5`); grows linearly with the window and with the owner dashboard's poll rate | Timezone: a day bucket depends on `Zone` and `now` (`fold.rs:98`); the view must be keyed by venue-local day and re-derived when the zone changes (DA §2 "venue timezone was a summer constant") | 1207.0137, 2303.08583, 2404.17679, 2606.07795 |
| **AX6** | **Why-provenance for `unavailable` and `allergen`.** Each derived row carries the ids of the EDB rows of ONE derivation (why-provenance, minimal proof height), so the console can say "unavailable: supply S at Q < C x qmin" and "allergen A via supply S (produced from S')". Non-recursive = tractable; for the transitive `allergen`, record the witness chain the semi-naive step used. | `bebop-lang/selfhost/std/dl.bp` (DESC: a witness column per IDB row), `dl_eval.bp` (write the witness when a head is derived), `gen_dl.bp`, `+workers/api/src/hubdo/provenance.rs` (DW5, planned) reads it, `workers/api/public/admin/more.js` ("why" link) | `dl_event_ns` not above 1.3x its pre-row value (CITED overhead band 1907.05045); golden: witness of every `unavailable(D)` on the sushi fixture names a supply whose stock the oracle agrees is short; `dagfull datalog` unchanged | Product: owner trust and support load; no CPU gain. Replaces DW5's projection-granularity walk with fact-granularity answers for the two rules owners ask about | Recursive why-provenance is intractable in general (CITED 2303.12773): keep ONE witness, never the set; negation (`courier_may`) gets why-not only via PUG-style rewriting -- out of scope | 2303.12773, 1907.05045, 2202.10766, 1808.05752, 1105.2255 |
| **AX7** | **Shred before replicate (ordering constraint, not code).** Land DG10w (seal personal fields at placement under the per-person key; forget = drop key + `Forgotten`) BEFORE BN3 puts orders into IndexedDB; the device replica stores sealed fields and the key is fetched per session, never stored; a deletion SLO gate: after `forget`, no device, R2 copy or archive can read the field. | `workers/api/src/hubdo/forget.rs`, `crates/dowiz-hub/src/shred.rs` (landed, default off), `workers/api/public/lib/replica.js` (BN3), `e2e/gates/conservation.mjs` (law `chain.redacted == declared`, missing today -- `docs/design/ROADMAP-2026-09-22.md` P2) | The P2 CHECK: 0 hits for the phone's three spellings in every image AND archive AND a Playwright-dumped IndexedDB after `forget`; `personal-data` gate 0; the law RED on one undeclared tombstone | Compliance: the one deletion mechanism that reaches copies dowiz does not control. No CPU effect; AEAD per personal field ESTIMATE µs | Key table is a mutable image (R §11.1); its loss = unreadable history, so it joins the nightly S3 copy under the restore drill | 2006.04777 (delete latency guarantees), 2104.01146 (data privacy as a top-5 event-sourcing challenge), 2004.00107 (content addresses survive because the bytes do not change) |
| **AX8** | **Named-policy compaction measurement** (no code change): write the store's compaction/rotation as the four LSM primitives (trigger, layout, granularity, movement) in `bebop-lang/docs/PERF.md` beside the G7 row, and add the compaction wall to the battery's printed numbers so the policy is a measured row, not a constant. | `bebop-lang/docs/PERF.md`, `bebop-lang/tools/battery.sh` (print only), `workers/api/src/hubstore.rs` (`HOT_KEEP_MS`, rotation) | `compaction_ms` per MB printed with a baseline; rotation keeps per ORDER (DA §2 row 8) stated as the data-movement policy | None in speed; it closes a documentation gap the LSM literature names as the usual silent defect | -- | 2202.04522, 1812.07527, 2005.00044 |

Order by payoff per lane-day for the Free plan: **AX3 (with BN2) > AX2 > AX4 (measure first) > AX5 > AX7 (ordering) >
AX1 > AX6 > AX8.** AX1 is the only asymptotic improvement to the Datalog layer and ranks low only because its trigger
is rare in the product today.

---

## 4. Where dowiz is at or past the literature, and what is rejected

**At or past (honest).**
- *Memo soundness and confluence are machine-checked* (`Dag.lean:184-198`, `Datalog.lean:117-194`), the property
  1106.0478 proves for self-adjusting computation in Twelf. The gate that makes the proof load-bearing (`dagfull`,
  warm == cold byte for byte) is stronger than most papers' "equal to batch" claims because it is a differential test on
  every commit, not a theorem about a model.
- *Granularity*: the 41x refutation of expression-level dataflow (R §1.1) is the empirical version of the field's
  consensus that incrementality pays only when the change is small relative to the recomputed region; dowiz measured its
  crossover (0.39 % change) rather than assuming it.
- *Semi-naive on a dirty set at O(fan-out)* (82 ns/event, R §12.3) is what DBSP, Differential Dataflow and Souffle-class
  engines deliver after far more machinery; at 165 x 120 the hand-rolled loop is the right size.
- *Content-addressed, four-reader-reproducible node hashes* (`nodekey.rs`, `crates/bebop-wasm/gate.sh`) and *"effects as
  data" in an outbox image with idempotent `(order, channel)` ids* (DA §1.3) are the event-sourcing study's five tactics
  in practice (2104.01146).
- *The stateful actor*: the Durable Object is the Cloudburst/GoldFish/Histrio design implemented by the platform;
  dowiz's contribution is keeping the WHOLE venue state in one object so a transaction is a DAG inside one turn (DA §2).
- *Scheduling*: T-4's "serial under 10 ms per worker" is the Dask finding (2010.11105) applied before it was needed.
- *Zero-copy where it is possible*: DG7's block is the only format that can be zero-copy on this platform (one copy
  across the isolate boundary is unavoidable); BN §3.1 measured that the copy is 1 % of the parse.

**Rejected for dowiz, with the reason.**
| Idea | Papers | Why not |
|---|---|---|
| CRDTs for orders/stock | 1608.03960, 1603.01529, 1710.04469, 2004.00107 | one writer per venue (the object); merge semantics would make stock a bounded counter with coordination (1801.06340) -- the FSM already refuses the conflicts a CRDT would merge |
| A DBSP / Differential Dataflow engine in the object or browser | 2203.16684, 2308.04214, 2511.00865 | 5 rule sets x 165 rows; retained differences cost memory (2208.00273); the semi-naive loop is the same asymptotics in ~200 lines (BN §4.2); a bebop VM in the browser is a second runtime (operator D-2) |
| Higher-order / factorised / worst-case-optimal IVM | 1207.0137 (as engine), 2303.08583, 1804.02780, 2605.08397 | sized for joins over millions of rows; dowiz's largest relation is ~5,400 events; AX5 takes only the first-order ring |
| E-graphs, equality saturation, superoptimisation | 2004.03082, 2108.02290, 2304.04332, 1711.04422, 2002.10213 | nothing on the request path is compiled by bebop; the Worker's size is crates (BN §5.2), not peepholes; Souper-on-Wasm shrank code size only |
| Zero-copy IPC between Worker and object | 2511.01888, 2002.09344, 1810.00556 | two isolates on two machines; no shared memory; the hop is a network call (BN §1.1) |
| Parallel self-adjusting computation / DAG parallelism on the request path | 2105.06712, 2004.10908 | every kernel is microseconds (R §12.3); a Worker is single-threaded |
| Incremental topological ordering of the edge table | 0802.1059 | twelve static edges; Kahn once at compile time (`ordfsm.bp`) |
| Stale-view sampling | 1509.07454 | an owner's revenue is exact money; a sampled estimate is a float by another name |
| Dense tensors / SIMD | (operator C-1; R §12.3) | 8x slower than sparse scalar at 1x; withdrawn 2026-09-28 |

---

## 5. What was NOT verified, and the command that settles each

- No full paper text was fetched; every CITED claim is from an abstract in the corpus. Numbers inside abstracts
  (e.g. 1907.05045's 1.27x, 2101.09355's 95 %) are quoted, not reproduced.
- The share of `?since=` polls answered `None` (AX2's premise) and the share of read-only wakes (AX4's gate) are
  unmeasured: both need a counter in `hubdo.rs` surfaced through `cf.mjs` (BN8).
- Whether a PROJ write inside the same `put_image` transaction counts as an extra row written (AX4): `cf.do_rows_written_day`
  on qa-durres over 20 placements (U9 in BN §10).
- `dl_event_ns` for a delete in a recursive stratum (AX1's baseline): no fixture exercises it; `dl_recompute_s` is the
  path by reading `dl_eval.bp:19-22, 624`, not by timing.
- DGSW's state is read from the lane roadmap (`/root/lanes/w-roadmap/.../ROADMAP-2026-09-22.md:89`), not from git.
- Salsa, Adapton (2014), Naiad, "Build systems à la carte", Gupta-Mumick Counting and Noria are not in the corpus; they
  are named as context only.

---

## 6. Corpus method and what was discarded

1,187 unique papers from 38 query tags (`scratchpad/arxiv/corpus.jsonl`; `topics` per paper). Primary categories:
cs.DC 320, cs.DB 187, cs.PL 139, cs.SE 74, cs.AI 65. Ranking by Semantic Scholar citations alone put off-topic
papers first in six of nine groups ("Adversarial Attacks on Neural Networks for Graph Data", c=1,305, under
`incremental`; "Epistemic Neural Networks" under `datalog`), so each line was re-filtered by a keyword regex over
title+abstract and then read by hand. A screen for ML/physics/robotics vocabulary flags 255 papers (21 %) as off-topic;
by tag: `zerocopy` 80 %, `dagsched` 50 %, `eventsrc` 43 %, `actors` 37 %, `serverless` 31 %, `edgestate` 28 %,
`incremental` 22 %; the clean tags are `ivm` 4 %, `incview` 3 %, `selfadj` 0 %, `dbsp` 0 %. Discarded by rule: LLM
serving and agent frameworks under `edgestate`/`serverless`/`taskgraph` (not our problem), graph-neural and continual
learning under `incremental`, ontology/OBDA query rewriting under `dl_eval` (Datalog as a target language, not an
engine), blockchain CRDTs, trusted-execution Wasm (security, not performance), HLS/FPGA dataflow, physics papers that
matched "e-graph". Abstracts read by hand: 116 (`scratchpad/arxiv/shortlist_abstracts.md`); cited: 94 (§7); read and
set aside: 22 (listed at the end of §7).

---

## 7. Bibliography (abstract read; citations = Semantic Scholar count in the corpus)

| arXiv | Title | Year | Cites | Used in |
|---|---|---|---|---|
| 1106.0478 | A Consistent Semantics of Self-Adjusting Computation | 2011 | 7 | 2.1, AX3, §4 |
| 2104.01270 | Demanded Abstract Interpretation | 2021 | 27 | 2.1 |
| 1609.05337 | miniAdapton | 2016 | 0 | 2.1 |
| 2105.06712 | Efficient Parallel Self-Adjusting Computation | 2021 | 5 | 2.1, §4 |
| 2312.07946 | Incremental Computation: What Is the Essence? | 2023 | 10 | 2.1 |
| 1811.06069 | Fixing Incremental Computation: Derivatives of Fixpoints, and the Recursive Semantics of Datalog | 2018 | 13 | 2.1, AX1 |
| 1902.05465 | Change Actions: Models of Generalised Differentiation | 2019 | 16 | 2.1 |
| 2603.19560 | Incremental Live Programming via Shortcut Memoization | 2026 | 0 | 2.1 |
| 1104.2293 | Reactive Imperative Programming with Dataflow Constraints | 2011 | 34 | 2.1 (read; not relied on) |
| 2203.16684 | DBSP: Automatic Incremental View Maintenance for Rich Query Languages | 2022 | 53 | 2.2, AX1, AX2, §4 |
| 2404.16486 | OpenIVM: a SQL-to-SQL Compiler for Incremental Computations | 2024 | 2 | 2.2 |
| 2308.04214 | On The Suitability of Differential Dataflow For Datalog Interpretation In Highly Dynamic Settings | 2023 | -- | 2.2, AX1, §4 |
| 2208.00273 | Optimizing Differentially-Maintained Recursive Queries on Dynamic Graphs | 2022 | 6 | 2.2, §4 |
| 1812.02639 | Shared Arrangements | 2018 | 2 | 2.2 |
| 2411.08274 | Flo: a Semantic Foundation for Progressive Stream Processing | 2024 | 8 | 2.2 |
| 2502.00222 | The Free Termination Property of Queries Over Time | 2025 | 3 | 2.2 |
| 2210.12605 | Keep CALM and CRDT On | 2022 | 17 | 2.2, AX2 |
| 1208.0088 | Spinning Fast Iterative Data Flows | 2012 | 168 | 2.2 |
| 2511.00865 | FlowLog: Efficient and Extensible Datalog via Incrementality | 2025 | 1 | 2.3, §4 |
| 1711.03987 | Optimised Maintenance of Datalog Materialisations | 2017 | 18 | 2.3, AX1 |
| 1811.02304 | Modular Materialisation of Datalog Programs | 2018 | 12 | 2.3, AX1 |
| 2105.14435 | Convergence of Datalog over (Pre-) Semirings | 2021 | 52 | 2.3 |
| 2403.12436 | Evaluating Datalog over Semirings: A Grounding-based Approach | 2024 | 12 | 2.3 |
| 2311.17664 | On the Convergence Rate of Linear Datalogo over Stable Semirings | 2023 | 4 | 2.3 |
| 2303.12773 | The Complexity of Why-Provenance for Datalog Queries | 2023 | 22 | 2.3, AX6 |
| 1907.05045 | Provenance for Large-scale Datalog | 2019 | 3 | 2.3, AX6 |
| 2202.10766 | Revisiting Semiring Provenance for Datalog | 2022 | 21 | 2.3, AX6 |
| 1105.2255 | On the Limitations of Provenance for Queries With Difference | 2011 | 47 | 2.3, AX6 |
| 1808.05752 | PUG: Why & Why-Not Provenance | 2018 | 32 | 2.3, AX6 |
| 1909.08246 | Extended Magic for Negation | 2019 | 6 | 2.3 |
| 2304.04332 | Better Together: Unifying Datalog and Equality Saturation (egglog) | 2023 | 73 | 2.3, 2.10, §4 |
| 1511.08915 | Column-Oriented Datalog Materialization for Large Knowledge Graphs | 2015 | 64 | 2.3 |
| 1910.08888 | Monotonic Properties of Completed Aggregates in Recursive Queries | 2019 | 9 | 2.3 |
| 1207.0137 | DBToaster: Higher-order Delta Processing | 2012 | 249 | 2.4, AX4, AX5, §4 |
| 2303.08583 | F-IVM: Analytics over Relational Databases under Updates | 2023 | 13 | 2.4, AX5, §4 |
| 2404.17679 | Recent Increments in Incremental View Maintenance | 2024 | 7 | 2.4, AX5 |
| 1804.02780 | Counting Triangles under Updates in Worst-Case Optimal Time | 2018 | 37 | 2.4, §4 |
| 2605.08397 | Maintaining Queries under Updates Using Heavy-Light Partitioning | 2026 | 1 | 2.4, §4 |
| 2606.07795 | The Role of Semirings in Incremental View Maintenance | 2026 | 1 | 2.4, AX5 |
| 1509.07454 | Stale View Cleaning | 2015 | 28 | 2.4, §4 |
| 2603.27775 | Enzyme: Incremental View Maintenance for Data Engineering | 2026 | 3 | 2.4 |
| 2509.25285 | ActorDB: Single-Writer Actors, IVM, Zero-Trust Messaging | 2025 | 0 | 2.4 |
| 2108.12469 | LaForge: Always-Correct and Fast Incremental Builds | 2021 | 4 | 2.5, AX3 |
| 2002.06183 | Constructing Hybrid Incremental Compilers ... Internal Build System | 2020 | 8 | 2.5, AX3 |
| 1705.05828 | A Co-contextual Type Checker for Featherweight Java | 2017 | 12 | 2.5 |
| 0802.1059 | Average-Case Analysis of Online Topological Ordering | 2008 | 23 | 2.5, §4 |
| 2010.11105 | Runtime vs Scheduler: Analyzing Dask's Overheads | 2020 | 16 | 2.6, §4 |
| 2303.05989 | DAG Scheduling in the BSP Model | 2023 | 7 | 2.6 |
| 1107.3734 | Decentralized List Scheduling | 2011 | 19 | 2.6 |
| 2004.10908 | Taskflow | 2020 | 166 | 2.6, §4 |
| 2010.07268 | Wukong | 2020 | 149 | 2.6 |
| 1910.05896 | In Search of a Fast and Efficient Serverless DAG Engine | 2019 | 38 | 2.6 |
| 2109.13492 | Following the Data, Not the Function (Pheromone) | 2021 | 95 | 2.6 |
| 2006.08654 | Triggerflow | 2020 | 79 | 2.6 |
| 2204.11533 | Fusionize | 2022 | 21 | 2.6 |
| 2104.01146 | An Empirical Characterization of Event Sourced Systems | 2021 | 25 | 2.7, AX7, §4 |
| 2202.04522 | Constructing and Analyzing the LSM Compaction Design Space | 2022 | 88 | 2.7, AX8 |
| 1812.07527 | LSM-based Storage Techniques: A Survey | 2018 | 230 | 2.7, AX8 |
| 2006.04777 | Lethe: A Tunable Delete-Aware LSM Engine | 2020 | 2 | 2.7, AX7 |
| 2005.00044 | Efficiently Reclaiming Space in a Log Structured Store | 2020 | 4 | 2.7, AX8 |
| 1608.03960 | A Conflict-Free Replicated JSON Datatype | 2016 | 128 | 2.7, §4 |
| 1603.01529 | Delta State Replicated Data Types | 2016 | 118 | 2.7, AX2, §4 |
| 1710.04469 | Pure Operation-Based Replicated Data Types | 2017 | 40 | 2.7, §4 |
| 2004.00107 | Merkle-CRDTs | 2020 | 17 | 2.7, AX7, §4 |
| 1801.06340 | Just-Right Consistency | 2018 | 16 | 2.7, §4 |
| 2511.01888 | Roadrunner: Accelerating Data Delivery to WebAssembly-Based Serverless Functions | 2025 | 3 | 2.8, §4 |
| 2510.05118 | Lumos: WebAssembly as a Serverless Runtime in the Edge-Cloud Continuum | 2025 | 3 | 2.8 |
| 1810.00556 | TZC: Towards Zero-Copy IPC with Partial Serialization | 2018 | 36 | 2.8, §4 |
| 2111.11517 | Columnar Formats for Schemaless LSM-based Document Stores | 2021 | 11 | 2.8 |
| 2109.14349 | Relational Memory | 2021 | 21 | 2.8 |
| 1901.09056 | Not So Fast: WebAssembly vs. Native Code | 2019 | 186 | 2.9 |
| 2002.09344 | Faasm | 2020 | 382 | 2.9, §4 |
| 2101.09355 | Benchmarking, Analysis, and Optimization of Serverless Function Snapshots | 2021 | 259 | 2.9, AX4 |
| 2104.13869 | Faa$T | 2021 | 159 | 2.9, AX4 |
| 2207.08175 | FaaSLight | 2022 | 98 | 2.9 |
| 2003.03423 | Serverless in the Wild | 2020 | 944 | 2.9 |
| 2410.06145 | Serverless Cold Starts and Where to Find Them | 2024 | 49 | 2.9 |
| 2001.04592 | Cloudburst | 2020 | 166 | 2.9, §4 |
| 2010.06706 | Beldi | 2020 | 20 | 2.9 |
| 2103.00033 | Durable Functions and Netherite | 2021 | 17 | 2.9 |
| 2412.02867 | GoldFish | 2024 | 10 | 2.9, §4 |
| 2410.21793 | Histrio | 2024 | 1 | 2.9, §4 |
| 2205.01183 | A fast in-place interpreter for WebAssembly | 2022 | 34 | 2.9 |
| 2104.15098 | Fast Compilation and Execution of SQL Queries with WebAssembly | 2021 | 4 | 2.9 |
| 2004.03082 | egg | 2020 | 5 | 2.10, §4 |
| 2108.02290 | Relational E-Matching | 2021 | 25 | 2.10, §4 |
| 2209.03398 | Small Proofs from Congruence Closure | 2022 | 19 | 2.10 |
| 2205.14989 | Combining E-Graphs with Abstract Interpretation | 2022 | 11 | 2.10 |
| 2111.12116 | Caviar | 2021 | 14 | 2.10 |
| 1211.0557 | Stochastic Superoptimization (STOKE) | 2012 | 394 | 2.10 |
| 1711.04422 | Souper | 2017 | 98 | 2.10, §4 |
| 2306.00229 | Minotaur | 2023 | 22 | 2.10 |
| 2002.10213 | Superoptimization of WebAssembly Bytecode | 2020 | 14 | 2.10, §4 |

Not cited although read (no bearing on a dowiz decision): 0812.0564 Provenance Traces; 1108.3265 Self-Adjusting
Stack Machines; 1912.09747 ST2 on Timely; 2004.05297 Graphsurge; 2104.04512 dependency-guided synchronisation;
1505.00212 rewriting + maintenance with equality; 1804.10565 certified view maintenance in Coq; 1812.03975 RecStep;
2112.01132 DP provenance; 2104.01241 TreeToaster; 2009.13631 Tempura; 1403.6968 LINVIEW; 1203.2704 reliable build
systems; 2308.16517 BeeFlow; 1207.0140 LogBase; 1902.05870 formal serverless; 2206.12888 Wasm continuum; 2208.07100
seminaive DatalogMTL; 2504.08914, 1910.07910, 2605.07584 (semiring circuits, fixed-point provenance, lifted planning);
2308.03615 Dirigo.
