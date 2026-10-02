# AX implementation plan: how to build every row of the arXiv report, including the ones it rejected

Research lane W-AXPLAN (Fable), 2026-10-02, READ-ONLY on code. Tree read at `/root/dowiz` (main `eb783904` plus the
uncommitted W-BN1A and W-PQ0 patches); every dowiz claim carries a `file:line` read today, every platform claim a
Cloudflare page with its "last updated" date, every paper claim an arXiv id. Labels: **MEASURED** = a number from a
dated measurement in the tree or a command run today; **CITED** = a document or a paper says it; **ESTIMATE** = arithmetic
on named inputs, not yet measured; **UNVERIFIED** = stated with the command that would settle it. No number below was
invented; where none exists the cell says so.

**Operator's instruction (verbatim, 2026-10-02):** "це потрібно добавити ... E-graphs, equality saturation,
супероптимізація; IPC без копіювання (zero-copy) між Worker і Durable Object; і це усе також — детальне fable
дослідження на те як це впровадити і зробити можливим, включно з усіма погодженими доповненнями з дослідження".
So this document does not re-argue whether; it says **how**, for all of: the eight accepted rows AX1–AX8 of
`docs/research/2026-10-02-arxiv-dag-optimisations.md` (§3), the five rows that report rejected in §4 (B1–B5 below,
now AX9–AX13), and the four "ideas worth lifting" (C, now AX14–AX17). Where a thing is literally impossible on this
platform the section says so with the citation, then designs the closest real mechanism and states how much of the
original benefit it captures.

Companions, not repeated: AX report (**AX**), `2026-09-27-dag-architecture.md` (**DA**), `2026-09-28-bebop-dag.md`
(**R**), `BLUEPRINT-BEBOP-DAG-2026-09-28.md` (**BP**), `SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md` (**RT**),
`2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` (**BN**), `2026-10-01-cloudflare-free-cost-and-rust-web.md`
(**CM**), `2026-10-02-system-integration-check.md` (**SI**), `/root/lanes/w-roadmap/docs/design/ROADMAP-2026-09-22.md`
(**RM**, the newer uncommitted roadmap).

---

## 0. One page

1. **Fourteen of the seventeen rows are buildable on Cloudflare Free as stated.** The three that are not, as stated,
   are: AX13 literal zero-copy Worker↔object (two isolates, the object pinned to one location — no shared memory;
   §B5 designs the zero-PARSE pass-through that captures the Worker-side 100 % of the copy/parse CPU and 0 % of the
   network hop); AX12a-b e-graph passes on the request path (bebop compiles to AArch64 only, nothing it emits runs in
   the Worker or the browser — §B4 designs the pass for the compiler on the box, where it pays today, and names the
   one operator decision — a wasm32 backend — that would put it on the request path); AX10 parallelism inside one
   Worker or one object (single-threaded by the platform — §B2 lists the five places where parallelism IS real).
2. **Instruments first, as the main session asked.** AX2 and AX4 are gated on four counters that do not exist today:
   the share of `?since=` polls answered `None`, read-only wakes, cold wakes per day, rows written by a projection
   write. §A0 is the row (AX0-COUNTERS); nothing in AX2/AX4 starts before its first nightly print.
3. **The format law decides the shape of four rows.** AX1 (a count/weight cell per Datalog row), AX4 (a PROJ kind 3 in
   the log image), AX6 (a witness column), AX14 (Z-set weights) each change a written format: every reader, oracle,
   golden and the harness model move in the SAME commit (`CLAUDE.md`, bebop law 4; B5 step 1's five-gate lesson).
4. **The 300-line Worker gate and the 800-line `.bp` cap shape every file list.** `tools/gates/file-size.baseline`
   holds `over=20 worst=1602` (MEASURED today) and `hubdo.rs` IS the 1,602-line file: no row below adds a line to it net;
   each adds a module ≤ 300 lines and one hook line, paid for by moving a line out. `dl.bp` 787 / `dl_eval.bp` 791 /
   `gen_dl.bp` 556 lines (MEASURED `wc -l`) leave 13, 9 and 244 lines of headroom: AX1 and AX6 go into new files.
5. **The order** (§6): Wave 0 instruments + measurement rows + the dynamic edge table; Wave 1 the three cheapest
   wins (AX3 early cutoff, AX2 deltas, AX1 Counting); Wave 2 AX4 PROJ (if the counters say so), AX13 pass-through,
   AX12c congruence closure; Wave 3 AX5 ring analytics, AX7+AX16 shred-before-replicate with the deletion SLO,
   AX14 Z-sets; Wave 4 AX9 factorised analytics, AX6 provenance, AX10 parallel fan-outs; Wave 5 AX12a e-graph pass,
   AX12d wasm-opt measurement, AX15 CALM test; Wave 6 (operator) AX12e wasm32 backend design, AX12b plan saturation.
   ≈ 46 lane-days in total (ESTIMATE, per-row sizing in §7), three file-disjoint lanes per wave.
6. **What needs the operator** (§8): one re-decision (D-2's "no second runtime" vs a COMPILED bebop wasm32 module),
   two irreversible format additions (PROJ kind 3 in dowiz log images; the DG10 key table), OA-1 (CI deploy token)
   for anything Souper/LLVM-sized, and nothing else — no plan change, no secret, no paid feature.

---

## 1. Platform facts every row is designed against (CITED, with dates)

| Fact | Source (developers.cloudflare.com, read 2026-10-02) | Bears on |
|---|---|---|
| A Durable Object is "inherently single-threaded"; soft limit 1,000 requests/s per object; KV-backed value ≤ 128 KiB (131,072 B); SQLite row/BLOB ≤ 2 MB | durable-objects/platform/limits, Jun 1 2026 | AX10 (no parallelism inside the object), AX4/AX13 (the 96 KiB `CHUNK`, `hubdo.rs:43`) |
| RPC: "Nearly all types that are Structured Cloneable" pass; `ReadableStream`/`WritableStream` pass "with automatic streaming flow control" and "ownership of the stream is transferred to the recipient"; `Request` and `Response` pass; "The maximum serialized RPC limit is 32 MiB"; "Smart Placement is currently ignored when making RPC calls ... Worker A will run locally, on the same machine" (Worker↔Worker only) | workers/runtime-apis/rpc, Sep 10 2026 | AX13: the hop serialises; streams are TRANSFERRED, not copied twice |
| RPC to a Durable Object is the preferred call path for compatibility date ≥ 2024-04-03 (ours is `2026-09-15`, `workers/api/wrangler.toml:5`); `stub.fetch()` is the legacy path | durable-objects/best-practices/create-durable-object-stubs-and-send-requests, Apr 21 2026 | AX13 |
| "Every RPC method call on a Durable Objects stub is its own RPC session and therefore a single billed request"; Free: 100,000 requests/day, 100,000 rows written/day, 5 M rows read/day, 13,000 GB-s/day; "Each setAlarm() is billed as a single row written"; "There is no charge for outgoing WebSocket messages"; incoming messages at 20:1 | durable-objects/platform/pricing, Sep 30 2026 | AX2 (push is free, pull is counted), AX4 (rows written), AX13 (RPC is not cheaper on the count) |
| Service bindings: "both Workers run on the same thread of the same Cloudflare server", "without incurring additional costs" | workers/runtime-apis/bindings/service-bindings, Aug 18 2026 | AX13: this is the ONE co-located IPC on the platform, and it is Worker↔Worker, not Worker↔object |
| A Response body stream "will be streamed to the client as it becomes available ... faster than buffering the entire payload into memory" | workers/runtime-apis/streams, Jun 15 2026 | AX13: returning the object's `Response` unread is the zero-parse path |
| KV-backed storage: `get`/`put` up to 128 keys per call; puts without an intervening `await` are "automatically combined and submitted atomically" | durable-objects/api/storage-api, Sep 21 2026 | AX4: a PROJ written in the same coalesced batch as the chunks is one commit |
| The object runs where its first `get()` was made and does not move (BN §1.1, data-location page Jun 26 2026); the Worker runs at the customer's edge | BN §1.1 | AX13: the hop is a network call; no shared memory is possible |
| Free Worker: 10 ms CPU per invocation, 50 subrequests, 6 simultaneous connections, 100,000 requests/day | CM §1.2 (limits page Sep 5 2026) | AX10: fan-outs bounded at 6; AX13: the CPU term |

The object class is SQLite-backed (`new_sqlite_classes = ["HubImages"]`, `wrangler.toml:146`) and uses the KV-style
storage API over it (`store.get`, `get_chunks`, `put_bytes`, `hubdo.rs:287-345, 1006-1060`), so "rows written" is the
binding write metric and a 96 KiB chunk put is one row (ESTIMATE from the pricing page's SQLite definition; U9 of BN
§10 measures it).

---

## A. The accepted rows

### A0. AX0-COUNTERS — the instruments the main session asked for FIRST

**Goal.** Four numbers no row below may be built without: (a) `since_total` / `since_none` — polls at
`?since=` and how many `changes_since` answered `None` (AX2's premise, AX §5 "unmeasured"); (b) `wakes_total` /
`wakes_readonly` — object constructor runs per day and how many served only reads before the next hibernation
(AX4's gate, DA §3.4 "do_reads_per_wake"); (c) `cold_fold_us` — the wall of the first `orders_view` after a wake
(AX4's number to beat on the live object); (d) `proj_rows` — chunks written per `put_image_as` (U9: does a PROJ in
the image add a row). Free-plan term moved: none directly; it decides whether AX2/AX4 move anything.

**Feasibility.** Yes. The object already keeps in-memory state across requests (`mem`, `recent`, `folded`,
`hubdo.rs:208-219`) and health already prints object fields (`/api/owner/health`, SI §5). `cf.mjs` prints eight
`cf.*` indicators from the GraphQL analytics API (`tools/evals/collect/cf.mjs:82-92`, MEASURED by reading);
`cf.do_response_bytes_day` and `cf.do_requests_day` exist. Hibernation resets in-memory counters, so the counters
must be flushed to health on every read and summed by the collector, never kept only in memory.

**Design.** New `+workers/api/src/hubdo/counters.rs` (≤ 120 lines): a `Counters` struct of `u32` cells in a
`RefCell`, `bump(kind)`, `snapshot() -> serde_json::Value`, and a `wake_mark` set in the constructor and cleared by
the first write. Hooks (one line each): `changes_since` call site (`hubdo.rs:1246` route), `put_image_as` after
`changed` is computed (`hubdo.rs:1026-1033`: `changed.len()` IS `proj_rows` per write), `orders_view` cold branch
(`hubdo.rs:346-356`, time with `Date::now` around the `load`), the constructor (`hubdo.rs:270-275`). Health adds one
object `counters`. `cf.mjs` gains `cf.since_none_share` (= none/total), `cf.wakes_readonly_share`,
`cf.cold_fold_us_p50`, `cf.proj_rows_per_write` read from the three venues' health (the owner token is in
`/root/.dowiz_owner`, memory `dowiz-hub-owner-credentials`). `hubdo.rs` net growth 0: the four hook lines are paid
by moving `Change` + `changes_since` (`hubdo.rs:109-141`, 33 lines) into `+hubdo/deltas.rs`, which AX2 owns next.

**RED first + gate.** `counters::tests::wake_then_read_counts_readonly` (plain-Rust `wire::Call` path, W-COV C2):
a fresh object answering two `/fold/orders` and no write → `wakes_readonly = 1`; one `append` → 0. `cf.test.mjs`:
the four indicators print with a baseline line or fail loud (`WHY_NO_TOKEN` shape, `cf.mjs:66`). Number to beat: none
— this row PRODUCES numbers. Acceptance: four indicators printed nightly for 7 days before AX2/AX4 start.

**Dependencies.** TELEM/DW6 merge (RM row 4) for the nightly runner; F10 (`hubdo.rs` split) is NOT required because
the row is net-zero on `hubdo.rs`. 1 lane-day. **Risk.** Counters that reset on hibernation under-count: the health
read flushes; the collector sums the day's health reads (at least the nightly's) and prints `samples=` beside the
share so a single sample is visibly weak. **Papers.** none (instrumentation).

### A1. AX1 — Deletion-aware incremental Datalog (Counting / DRed / B-F + closure module)

**Goal.** A delete in a recursive stratum, or any change under a negated atom, today recomputes the stratum whole and
diffs against a snapshot: `dl_recompute_s` (`bebop-lang/selfhost/std/dl_eval.bp:624-639`, "correct, not fast",
header `:19-22`). Non-recursive strata re-derive every candidate head backwards (`dl_inc_nonrec :547`,
`dl_exists_r :531`). Target: O(affected) instead of O(stratum) on deletes, and no backward evaluation in
non-recursive strata. Free-plan term: none today (Datalog is not on the request path, SI row 49); the row completes
the engine before DW2's `personal` closure (deletions on schema change) and AX12c/AX14 build on the same cell.

**Feasibility.** Yes; pure bebop on the box. Counting handles "only nonrecursive rules" and DRed/B-F "evaluate rules
'backwards'", and the hybrid is "usually significantly faster ..., sometimes by orders of magnitude" (CITED
1711.03987, Hu, Motik, Horrocks); transitive closure gets a specialised module inside semi-naive, "often by orders of
magnitude" (CITED 1811.02304). Both of dowiz's recursive rules ARE closures: `allergen` through `produced(S2,S)`
(`dl_sushi.bp:4`) and `personal` over `edge` (`dl_fix.bp:13`).

**Design.**
- *Data.* DESC is 32 cells per predicate, all used (`dl.bp:22-28`; `dl_d(db,p) = db[1] + 32*p`, `dl.bp:503`). Add a
  parallel **CNT region** of `cap` cells per IDB predicate holding the derivation count (AX14 turns it into a signed
  Z-set weight): DESC grows to 36 cells (`32` becomes a derived constant `dl_desc_cells()`; cells 32/33 = CNT (offset,
  n), 34 = module kind 0 generic | 1 transitive-closure | 2 symmetric-TC, 35 reserved). `dl_db_cells` sizes it.
  Written-format change: `bench/oracles/dl_common.py`'s DESC model, every `dl_*.py`, `dl_set_semantics.py` and the
  `dagfull datalog` arm move in the same commit.
- *Counting (non-recursive strata).* A derivation that lands a head `+1`s CNT; `dl_inc_nonrec`'s delete pass `-1`s
  per lost derivation instead of calling `dl_exists_r`; the row leaves F at CNT = 0. The candidate walk stays (it is
  the only way to find which heads a deleted body row supported) but the RE-DERIVATION goes: the `exists` variant of
  `gen_dl.bp` (`:9-13`) is no longer generated for non-recursive strata. Negated atoms: counts under `¬q` are not
  derivation counts (AX §3 risk) — a rule with a negated atom keeps today's re-derive path, selected per rule by the
  `simple` bit `dl_db_strata` already computes (`dl_eval.bp:713, 767-791`).
- *Recursive strata.* Two paths by DESC cell 34: **module 1/2** for a stratum whose rules are exactly `p(X,Y) :-
  e(X,Y); p(X,Y) :- e(X,Z), p(Z,Y)` (detected syntactically in `dl_check`, `dl.bp:443`): maintain reachability
  per deleted edge by the standard "mark suspects reachable only through the deleted edge, re-derive from unaffected
  frontier" (DRed restricted to the closure, the 1811.02304 shape); **generic DRed** otherwise: over-delete
  everything derivable from the deleted rows through the stratum's rules (a semi-naive pass over `del`), then
  re-derive from the surviving F (`dl_semi :363` seeded with the over-deleted set as candidates), then `dl_diff`. B-F
  is not added: at 120 supplies the backward step DRed avoids is the whole cost and B-F's extra pass has nothing to
  save (ESTIMATE; re-open if a stratum exceeds 10^4 rows).
- *Files.* `+bebop-lang/selfhost/std/dl_del.bp` (new, ≤ 800: counting, over-delete, closure module), `dl.bp`
  (DESC derive, `dl_check` module detection — within its 13 spare lines, else move the strata section `:277-443`
  into `+dl_strata.bp`), `dl_eval.bp` (route `dl_inc_s :700` to the new paths), `gen_dl.bp` (CNT maintenance in the
  generated `cand` walk; stop generating `exists` for simple strata), `bench/oracles/dl_common.py` + `dl_*.py`,
  `bench/vs_rust/dagfull.sh` (datalog arm: 10^4 MIXED insert/delete events incl. recursive deletes — today's arm
  exercises inserts, AX §5), `formal/Bebop/Datalog.lean` (new theorem `counting_sound`: for a non-recursive
  stratum, `CNT(h) = |derivations of h|` is preserved by +1/−1, and `h ∈ F ↔ CNT(h) > 0` — sibling of
  `seminaive_eq_naive :117`).

**RED first + gate.** (1) New fixture `dl_allergen_del`: delete one `produced(S2,S)` row that was the only path of
one allergen → exactly that `allergen(D,A)` leaves; oracle `dl_allergen_del.py`. RED today because the fixture does
not exist and `dl_recompute_s` is untimed (AX §5). (2) `dl_bench.sh` gains `dl_event_del_ns` on the 165/120/352
fixture. **Numbers to beat:** `dl_event_ns` ≤ 1,000 stays (MEASURED 970 generated, `ROADMAP.md:233`); the new
`dl_event_del_ns` ≤ 10× `dl_event_ns` (AX §3 target), baseline = `dl_recompute_s` timed FIRST (unmeasured today);
`dagfull datalog` incremental == scratch byte for byte after 10^4 mixed events; `arch_check` file-size ≤ 800 per
file; `lake build` rc 0, `sorryAx` 0.

**Dependencies.** DGSW merged (it owns `bebop.bp`; the std files are free once it lands, RM row 1). Feeds DW2
(closure deletions), AX12c, AX14. **Size:** 4 lane-days (1 fixture+baseline, 2 counting+DRed, 1 Lean+oracles).
**Risks.** A wrong count hides a row forever: the `dagfull datalog` arm compares to a from-scratch `dl_cold_s` after
every batch, and `dl_set_semantics.py` holds `present ↔ CNT > 0`; a count that drifts is caught at the next event.
Lean statement grows (one theorem). **Papers.** 1711.03987, 1811.02304, 2203.16684 (weights as counts), 2308.04214,
1811.06069.

### A2. AX2 — Every `since=` answers a delta

**Goal.** `changes_since` answers `None` beyond a 256-entry ring (`RECENT_KEEP`, `hubdo.rs:109`; `:126-141`) and the
ring is CLEARED on every whole-image write and every non-order event (`hubdo.rs:383, 1102`); the client then
refetches the whole list. Target: `None` only beyond a byte-bounded window, including on a COLD object. Free-plan
terms: bytes across the hop (MEASURED 250 MB/day object→Worker, BN §1.3) and Worker CPU per poll; counted requests
unchanged (sockets A2/A6 are that lever).

**Feasibility.** Yes, without a second structure. Every generation step appends ≥ 1 records to the log and the
records ARE the deltas (`Change.payload` is the event payload, `hubdo.rs:112-118`); what is missing is only the map
generation → "how many records existed then". Keep that map in the image, and a cold wake answers deltas.

**Design.**
- *Data.* A **GENTAB** of the last N generations: `(generation, record_count_after, kind)` triples, N = 256 (the same
  window as today, now surviving hibernation), ≈ 6 KB. Written INSIDE the log image at the tail by the same
  `put_image_as` call (the tail chunk already moves on every append: "ONLY THE CHUNKS THAT MOVED", `hubdo.rs:1026`) —
  +0 rows (ESTIMATE; AX0's `proj_rows` confirms). Whole-image writes that are not appends (`Written::Whole`:
  rotation, import, forget — `projection.rs:24-27`) write a `kind = GAP` row: `changes_since(g)` for `g` before a GAP
  still answers `None` (a redaction in place has no delta — AX §3 risk, kept). Non-order events (`Revealed`) are
  deltas of kind "moved": the client bumps its generation and does not refetch (today it is told to, `hubdo.rs:382-392`).
- *Answer.* `changes_since(g)`: find `g` in GENTAB → `count_after(g)`; the delta is the records with index ≥ that
  count, newest `k` from `hub.events()` (newest-first, `lib.rs:521-525` per DA §3.2) — O(k). `None` iff `g` is older
  than the table or a GAP lies between `g` and now, or `g > newest` (today's two refusals, `hubdo.rs:131-139`, kept).
- *Files.* `+workers/api/src/hubdo/deltas.rs` (≤ 300: `Change`, `changes_since`, `GenTab` encode/decode, the three
  refusals), `workers/api/src/hubdo.rs` (hooks replace `recent` + `RECENT_KEEP`; net ≤ 0 lines), `crates/dowiz-hub/src`
  (GENTAB as a trailing object the `Hub::load` reader skips — the hub image is a bebop-store image, so it is one
  more object kind beside PROJTAB with its own layout digest; bebop twin `selfhost/std/wlog.bp` family + `oracle.py`
  + `bebop-wasm/src/proj.rs` in the same commit — format law), `workers/api/public/lib/replica.js:14-18` ("the next
  successful read replaces the lot" → applies deltas, refetches only on `None`; the `moved` kind no longer triggers
  a list read), `workers/api/src/owner.rs:309-340` (the `changes_since` route).

**RED first + gate.** Native: `deltas::tests::cold_object_answers_delta` — build an image with 300 appends, drop
every in-memory field, `changes_since(298)` returns 2 changes (RED today: the ring is memory-only). Live: AX0's
`cf.since_none_share` — **number to beat: today's measured share (unknown until AX0 prints; AX §3 target ≤ 1 %)**;
`cf.do_response_bytes_day / cf.do_requests_day` on qa-durres falls (bytes per answered poll ≈ 100 B vs the list);
`rebuild.stale = []` nightly unchanged. **Dependencies.** AX0 first (7 nightly prints). **Size:** 3 lane-days.
**Risks.** A delta applied to a replica that missed a GAP would resurrect a forgotten customer — the GAP row makes
`None` mandatory across any whole-image write, and `replica.js` already refetches on `None`. **Papers.** 2203.16684
(deltas as the unit), 1603.01529 (small deltas joined to state), 2210.12605 (CALM: a prefix of a single-writer log is
monotone — see AX15).

### A3. AX3 — Early cutoff on node keys (and AX17: the edge-table column)

**Goal.** A settings write that changes no menu byte still produces a new catalogue generation, drops the `Memo`
(`put_image_as`, `hubdo.rs:1018-1021`), rebuilds it on the next read, and after BN2 would rewrite R2 objects and the
manifest. Target: a recompute whose output bytes are unchanged bumps nothing downstream. Free-plan terms: R2 Class A
writes per edit (BN2 target ≤ 3 → often 0–1), socket wakes, dependent recomputes.

**Feasibility.** Yes. `K64 = (crc32(frame) << 32) | len` and `K256 = sha256(frame)` exist in
`crates/bebop-store/src/nodekey.rs:77-88` with four-reader parity (`crates/bebop-wasm/gate.sh`). The compiler's rule
already is "K64 is an INDEX, never a proof; a hit is decided by the bytes" (`selfhost/prelude/dagc.bp:14-15`). The
main session's addition — never trust crc32 alone — is that rule applied here.

**Design.**
- `fold::menu::Memo` (`workers/api/src/fold/menu.rs:86-99`) gains `out: OutKey { k64: i64, k256: [u8;32] }` per
  output (per locale JSON body, per block). On a rebuild, the new output's K64 is compared to the old; **on equal K64
  the bytes are compared** (cheap: the old body is in the memo), and only then is the output "unchanged". K256 is
  what crosses machines (the manifest, BN2) — RT §2.2's rule.
- *Cutoff effects.* Unchanged output ⇒ `generation_out` is not bumped, no `moved` is sent on the storefront socket
  (BN2's "server push of the storefront ROOT"), `publish.rs` (BN2) compares K256 before any `PutObject`, and
  dependents keyed on `generation_out` (the edge cache key in `storefront.rs:186-194` uses the URL today — it would
  carry the out generation).
- *AX17: the edge table column.* DW2's `+workers/api/src/hubdo/edges.rs` row becomes `(inputs, projection, step,
  out_key)`: `out_key` names which key the cutoff compares (`K64+bytes` in-process, `K256` across machines) — the
  table is data (AX11 makes it dynamic) and `+tools/gates/edges.json` (DW3) prints it, so a projection without a
  cutoff rule is visible.
- *Files.* `fold/menu.rs`, `hubdo/menu.rs` (`menu_memo :44-61` keeps the old memo until the new one is compared),
  `+hubdo/edges.rs` (with AX11), `+hubdo/publish.rs` (BN2; compare-before-write), `crates/bebop-store/src/nodekey.rs`
  (no change; `k256` is there).

**RED first + gate.** `menu::tests::settings_write_without_menu_effect_keeps_out_key`: write a settings field the
menu does not read → `Memo.out.k64` and `k256` unchanged, no `moved` on a test socket (RED today: the memo is
dropped and rebuilt with a new generation tuple; nothing records an output key). After BN2:
`publish::tests::no_object_written_when_no_block_changed`. **Numbers to beat:** R2 Class A writes per one-dish price
edit ≤ 3 → the cutoff case 0 (BN2's number sharpened); `cf.do_response_bytes_day` not up; live: `moved` frames per
edit on qa-durres counted by the Playwright socket probe (BN2's `storefront-r2.mjs`). **Dependencies.** BN2 for the
R2 half; DW2/AX11 for the column; independent of AX0. **Size:** 2 lane-days (+1 inside BN2). **Risks.** A K64
collision hiding a real change is closed by the byte compare; a K256 collision is cryptographic. The memo compare
costs one extra body in memory per rebuild (≤ 10 KB block, ≈ 70 KB JSON — ESTIMATE from BN §8.2). **Papers.**
2108.12469, 2002.06183, 1106.0478; Mokhov et al. "early cutoff" (not in corpus).

### A4. AX4 — Persist the orders projection as a PROJ node in the log image

**Goal.** A cold object folds the whole hot log on its first read (`orders_view`, `hubdo.rs:346-356` →
`projection::read` refolds when the memo is absent, `projection.rs:184-196`): MEASURED 5.75 ms at 5,400 events,
120,776 µs at 32k (R §4; `projection.rs:7`). Target: read the memo (KB) from the image, fold only the records after
its tip. Free-plan terms: object CPU and wall on the first request after idle (DA P6: cold 260 ms vs warm 105 ms,
ESTIMATE-carried), and the 2.9 MB image read is unchanged (the PROJ lives inside it).

**Feasibility.** Yes, and the mechanism exists: `crates/bebop-store/src/proj.rs` PROJTAB/PROJ/OUT in the SAME image,
"EXTENDED one `log_step` per newer record while the memo's tip is still in the chain" (`proj.rs:1-20`), kinds 1 (log
fold) and 2 (KV snapshot). What is missing is a kind whose OUT is a BLOB (the serialised `Orders`), not `[value,
count]`. Rows written: the PROJ object is appended at the image's used-end, i.e. in the tail chunk the append already
rewrites; PROJTAB lives in superblock cell 5, in the front chunk the append already rewrites (the superblock moves
on every commit) — **+0 rows per write when the OUT fits the tail chunk** (ESTIMATE; AX0 `proj_rows` and U9
`cf.do_rows_written_day` measure it). A separate `proj:orders` image (DA §3.4's "+1 row per order event") is
rejected for exactly that reason.

**Design.**
- `proj.rs`: `KIND_BLOB = 3`, `LAYOUT_OUTB = "OUTB{i64,[i64]}"` (nbytes, then 8 bytes per cell LE, last cell
  zero-filled — `dagc.bp`'s SRC packing, `:12`), digest pinned by `tests::digests_are_st_digest_of_their_layouts`;
  bebop twin `selfhost/prelude/proj.bp` + `crates/bebop-wasm/src/proj.rs` (`bw_proj`) + `oracle.py` in the same
  commit; `gate.sh` step 6's `fixtures/proj.store` gains a kind-3 fixture.
- `fold/projection.rs`: `Orders::to_bytes()` / `from_bytes()` — `by_id`, `newest_first`, `count`, `digest`,
  `generation` as a versioned frame (`nodekey::Frame`, so the four readers can key it); `after_log_write`
  (`:165-180`) also returns the bytes to write; `read` (`:184-196`) tries the PROJ first (`input_gen` == the root's
  generation, tip in chain) and extends by `fold_one` over the records after the tip — `proj.rs`'s S-2 rule.
- `hubdo.rs:1002` (`put_image` → `put_image_as(LOG_IMAGE, …, hub.to_bytes_trimmed())`): the hub's bytes are
  produced with the PROJ appended in the same buffer, so the write is ONE `put_image_as` — no second commit, the
  coalescing rule of the storage API makes chunks+meta one atomic batch (§1).
- `rebuild.rs:96-160`: law 8 compares `hubstore::orders_state` from the BYTES against the PERSISTED PROJ, not the
  memo (DA §3.4). `forget.rs` writes `Written::Whole` → PROJ dropped (`Hub::redact` "carries no memo", `proj.rs:11-13`).
- Files: `crates/bebop-store/src/proj.rs`, `bebop-lang/selfhost/prelude/proj.bp`, `crates/bebop-wasm/src/proj.rs`
  + `oracle.py` + `gate.sh` + fixture, `workers/api/src/fold/projection.rs` (+ `+fold/projection/bytes.rs` ≤ 300),
  `workers/api/src/hubdo.rs` (hook, net 0), `workers/api/src/rebuild.rs`, `crates/dowiz-hub/src/logimage.rs` (the
  trimmed writer keeps the PROJ).

**RED first + gate.** FIRST AX0's `wakes_readonly_share` and `cold_fold_us_p50` for 7 days — the row proceeds only if
read-only wakes are a material share (DA §3.4's condition; "material" = the operator reads the number, this lane
proposes ≥ 30 %). Then: `projection::tests::cold_read_from_persisted_proj_equals_refold` (RED today: no persisted
form); `rebuild` law 8 against a scratch image whose PROJ was hand-corrupted → `stale` names it (mutation proof);
`dagfull store` warm == cold on the kind-3 fixture, four readers 4/4. **Numbers to beat:** `cold_fold_us_p50` from
AX0 (MEASURED basis 5.75 ms at 5,400 events native; the live number is AX0's) → ≤ 0.5 ms (ESTIMATE: memo read + ≤ 10
`fold_one`); `cf.do_rows_written_day` per placement unchanged (U9: ≤ 21 over 20 placements); `rebuild.stale = []`
nightly. **Dependencies.** AX0; DGSW (the store twins); BN4 shares `hubdo.rs`'s write path — AX4 lands before or
inside BN4's lane, not beside it. **Size:** 4 lane-days. **Risks.** A second account of the truth bounded only by
law 8 (DA §3.4) — law 8 runs nightly on both venues already; the kind-3 OUT must drop on `Written::Whole` exactly as
the memo does (`projection.rs:24-27`). An OUT larger than the tail's free space spills into a new chunk: +1 row — BN6
(120 B/order) keeps a 30-order OUT ≈ 4 KB. **Papers.** 2101.09355, 2104.13869, 1207.0137.

### A5. AX5 — Ring-valued analytics view (exact integer money)

**Goal.** `/fold/analytics` folds the owner's report from the whole memoised order list on every call
(`hubdo/reads.rs:47-55` → `services::analytics::handler::answer` → `fold::fold`, `services/analytics/fold.rs:98`).
Target: `(local_day, dish) -> (orders, revenue_minor, refunds_minor, rejected)` stepped by the same `Written` events,
report = read of the window's rows. O(window) → O(1) per read. Free-plan term: object CPU per owner-dashboard poll
(unmeasured; DW6's `cost_us`). The main session's addition: **exact integer money, no sampling** — the ring is
`(Z, +)` on `i64` minor units, overflow-checked as `kernel/src/money.rs` requires; stale-view sampling (1509.07454)
stays rejected.

**Feasibility.** Yes; it is first-order IVM with a trivial delta — "views are maps from keys to ring payloads"
(CITED 2303.08583); constant-time maintenance under inserts is characterised per semiring (CITED 2606.07795).

**Design.** `+workers/api/src/fold/analytics_view.rs` (≤ 300): `View { zone: Zone, zone_gen: i64, days: BTreeMap<i64,
DayCell>, dishes: BTreeMap<(i64, String), DishCell> }`, `step(ev: &Event, now_tz)`: a terminal `Placed` (the fold's
rule for counting an order, `fold.rs:98-…`) adds `+1, +total`; `COMPENSATED_REFUND` adds `-amount` to `refunds`;
`Rejected` to `rejected`; keyed by `start_of_local_day_ms(zone, placed_at)` (`fold.rs:72-79`). The report for a
window is the sum of the day cells in `day_starts` (`fold.rs:72`) — same `Report` struct (`fold.rs:36-46`).
Hook: `after_log_write` passes the same `Written` to the second memo (one line in `hubdo.rs:1085`; the memo lives
beside `folded`, `hubdo.rs:219`, as `analytics: RefCell<Option<View>>`). Timezone: the view stores `zone` and is
dropped when the venue's zone setting changes (settings write → `MENU_INPUTS` drop path, `hubdo/menu.rs:20`; the
view adds itself to that drop) — DA's "venue timezone was a summer constant" risk. `services/analytics/fold.rs` is
UNCHANGED and becomes the oracle.

**RED first + gate.** Native `analytics_view::tests::stepped_equals_fold_after_10k_events`: 10^4 random events incl.
refunds, rejections and placements at local-midnight boundaries in `Europe/Tirane` and `Europe/Kyiv`, compared to
`fold::fold` on every 100th step (RED today: no view). `rebuild` gains law 10: `view == fold` (read-only, named
diff). **Numbers to beat:** `/fold/analytics` object `cost_us` (DW6) — baseline unmeasured, printed first; `float-money`
gate 0 unchanged; `clock` gate 0 (the view takes `now` from the request as `fold` does). **Dependencies.** AX4 (same
`Written` plumbing — sequence after it); DW6 for the number. **Size:** 2 lane-days. **Risks.** Day bucket drift at
DST changes: the oracle test covers the 2026-10-25 boundary explicitly. **Papers.** 1207.0137, 2303.08583, 2404.17679,
2606.07795.

### A6. AX6 — Why-provenance for `unavailable` and `allergen`

**Goal.** The console can say "unavailable: supply S at Q < C × qmin" and "allergen A via supply S (produced from
S')". Product value (owner trust, support load), no CPU gain. Non-recursive why-provenance is "highly tractable",
recursive is intractable in general (CITED 2303.12773) → ONE witness per derived row, never the set; Souffle's
provenance costs 1.27× (CITED 1907.05045) — the overhead band to stay under.

**Design.** The CNT/weight cell of AX1 is joined by a **WIT region** per IDB predicate: 4 cells per row = the
`(pred, row_index)` of up to two body rows of ONE derivation (a rule has ≤ 4 atoms; two EDB witnesses suffice for the
two product rules; a closure row's witness is the `(edge, p)` pair the semi-naive step used — minimal proof height is
the first derivation found, which semi-naive reaches first by round). Written when `dl_put`/`dl_replace`
(`dl_eval.bp:408, 420`) lands a head; `gen_dl.bp`'s generated walks write it from the bound variables (the plan
knows which atoms bound the head). Reader: `+workers/api/src/hubdo/provenance.rs` (DW5) walks WIT from a derived row
to EDB rows and names them in the console's "why" link (`admin/more.js`). Files: `+dl_wit.bp` (≤ 800, beside
`dl_del.bp`), `gen_dl.bp`, `dl.bp` DESC (two more cells: WIT offset/n — the AX1 DESC change carries the room), oracles
`dl_unavail.py`/`dl_allergen.py` (+ `_wit` arms), `provenance.rs`, `admin/more.js`. **RED first:** golden — the
witness of every `unavailable(D)` on the sushi fixture names a supply the oracle agrees is short (RED: no witness
column). **Numbers to beat:** `dl_event_ns` ≤ 1.3 × its pre-row value (CITED band); `dagfull datalog` unchanged;
`arch_check` 800. **Dependencies.** AX1 (DESC layout); DW2/DW5 for the Worker reader. **Size:** 3 lane-days.
**Risks.** Negation (`courier_may`) gets no why-not (PUG-style rewriting out of scope, 1808.05752); a witness that
names a redacted record after `forget` — witnesses point at supply/recipe rows, never at a person. **Papers.**
2303.12773, 1907.05045, 2202.10766, 1808.05752, 1105.2255.

### A7. AX7 — Shred before replicate (ordering constraint), with AX16 the deletion SLO

**Goal.** Once BN3 puts orders into IndexedDB, in-place redaction (`crates/dowiz-hub/src/forget.rs:1-27`) cannot
reach the copies; only key deletion can (`shred.rs:1-30`: "every copy of every block becomes unreadable at the same
instant"). Constraint: **DG10w lands before BN3**. AX16 (Lethe's point, CITED 2006.04777: deletes as first-class with
a latency guarantee) makes it a measured gate: after `forget`, within one nightly, NO copy dowiz controls or caused
can read the field.

**Feasibility.** Yes; the hub half is landed (`shred.rs`, feature `shred`, default off, `crates/dowiz-hub/Cargo.toml:42`)
and the Worker never compiles it (SI row 25: "shred never compiled anywhere", CI tests the hub without the feature).

**Design (the Worker half, DG10w, as RM row 20 states, plus the SLO).** Seal personal fields at placement under the
per-person key (`KeyTable` in a small mutable image `keys`, registered in `privacy/registry.rs`); `forget` = drop the
key + `Forgotten`; history in clear keeps declared redaction (law 9). **The SLO's copies list, each with its purge:**
(1) the venue's images — key dropped; (2) nightly S3/R2 copies — sealed under the key table's state at copy time;
the key table itself joins the nightly copy under the restore drill (R5), so an OLD copy restored still lacks the
dropped key only if the key table restored is the NEW one — the drill asserts it; (3) IndexedDB replicas (BN3) — store
sealed fields, fetch the key per session, never store it; (4) the edge cache — `/api/public/.../menu` has no personal
data; order views are `private, no-store` (`lib.rs:206`, SI row 48) — assert, do not purge; (5) R2 published blocks
(BN2) — the block schema has no personal column (`block/schema.rs`); a test asserts the `personal-data` registry has
no block row. Files: `workers/api/src/hubdo/forget.rs`, `+hubdo/keys.rs` (≤ 300), `privacy/registry.rs`,
`crates/dowiz-hub/Cargo.toml` (`shred` on for the Worker), `workers/api/Cargo.toml`, `e2e/gates/conservation.mjs`
(law `chain.redacted == declared`, missing today — RM P2), `workers/api/public/lib/replica.js` (BN3: sealed fields),
`+tools/gates/deletion-slo.sh` (the P2 CHECK over images + archive + a Playwright-dumped IndexedDB).

**RED first + gate.** `conservation.mjs` law RED on a scratch image with one undeclared tombstone;
`deletion-slo.sh` RED today (the IndexedDB arm has no replica to dump — it prints `NOT MEASURED: no replica` and
exits non-zero, never a skip counted as pass). **Numbers:** 0 hits for the phone's three spellings in every image AND
archive AND the dumped IndexedDB after `forget`; `personal-data` 0; AEAD per field ESTIMATE µs (KAT-gated
`aes_gcm`). **Dependencies.** DG10 hub side (landed), R5 restore drill for the key-table clause; BLOCKS BN3.
**Size:** 3 lane-days (+0.5 for the SLO script). **Risks.** Key table loss = unreadable history (R §11.1) — the
nightly copy + drill; the irreversible step is the first sealed placement on a live venue (§8). **Papers.**
2006.04777, 2104.01146, 2004.00107.

### A8. AX8 — Named compaction policy, measured

**Goal.** The store's compaction and `HOT_KEEP_MS` rotation (`workers/api/src/hubstore.rs`) are one fixed policy;
write it as the four LSM primitives (trigger, layout, granularity, movement — CITED 2202.04522) and print the
compaction wall per MB in the battery so it is a row with a baseline. No code change in the store. **Design.**
`bebop-lang/docs/PERF.md` (a table beside the G7 row: MEASURED 747 ms per 85 MB, R §11.1), `bebop-lang/tools/battery.sh`
(print `compaction_ms_per_mb` from the existing G7 run; no new run), `hubstore.rs` header (the rotation stated as
"movement = per ORDER, trigger = `HOT_KEEP_MS`"). **Gate:** the printed line with a baseline; a run that prints no
number is RED (the "instrument that measures nothing" rule, memory `bebop-instruments-that-measure-nothing`).
**Size:** 0.5 lane-day. **Papers.** 2202.04522, 1812.07527, 2005.00044.

---

## B. The previously rejected rows, now approved: how

### B1. AX9 — Higher-order / factorised IVM where a join actually exists

**Goal.** AX5 maintains a FIRST-order view over one relation (orders). The joins dowiz has, each small: (i) analytics
**order × dish × supply** — food cost and revenue per supply/allergen: `Σ_lines qty(line) × recipe(dish, supply, c) ×
price(supply)`; a supply PRICE change today would touch every order line that used it; (ii) **ПФ recipe expansion**
(semi-finished products: recipe of recipe) — a matrix chain `recipe_dish×pf · recipe_pf×supply`; (iii) courier × zone —
too small to matter (one courier pool per venue). Free-plan term: object CPU on owner reports as windows grow (30 →
365 days) and as the kitchen role (memory `dowiz-kitchen-and-stock-research-2026-09-26`) asks for cost-per-dish daily.

**Feasibility.** Yes, exact: F-IVM's view tree with ring payloads covers "group-by queries with joins and sum/count
aggregates" and "matrix chain multiplication" and beats first-order IVM "by orders of magnitude while using less
memory" (CITED 2303.08583); DBToaster's point is that the delta VIEWS "support each other's incremental maintenance"
(CITED 1207.0137). The ring is `Z^k` on i64 — commutative, exact; no floats enter (the `float-money` gate holds it).
Worst-case-optimal structures and heavy-light partitioning (1804.02780, 2605.08397) stay out until a relation exceeds
10^5 rows (ESTIMATE threshold; the largest relation today is ≈ 5,400 events, R §4) — the row names the trigger.

**Design — the view tree (two levels above AX5):**
```
V_day_dish(day, dish)            = (orders, qty, revenue)            -- AX5's view
V_day_supply(day, supply)        = Σ_dish recipe(dish, supply, c) · qty(day, dish)        (factorised: never per order)
V_day_cost(day)                  = Σ_supply V_day_supply · price(supply, catalogue_gen)
V_pf(dish, supply)               = recipe_dish×pf · recipe_pf×supply  (the ПФ chain, re-derived per catalogue generation)
```
Deltas: an order event steps `V_day_dish` (+1 row) and `V_day_supply` for that dish's ≤ 10 supplies; a recipe edit
re-derives `V_pf` and the affected `V_day_supply` columns for the window (catalogue-generation keyed: a recipe edit
does not rewrite history — last March's cost uses last March's recipe, B4's rule for tax); a supply price edit steps
`V_day_cost` only. Implemented in `+workers/api/src/fold/analytics_tree.rs` (≤ 300) over AX5's `View`, inputs =
`Written` events + the catalogue block `bom` (`block/view.rs` `bom_of`, 634 ns MEASURED) + `menu_prices`. Oracle: a
from-scratch fold in `services/analytics/cost.rs` (new, pure, ≤ 200) the way `fold.rs` is AX5's.

**RED first + gate.** `analytics_tree::tests::tree_equals_scratch_after_10k_events_and_100_recipe_edits` (RED: no
tree); `rebuild` law 11 (`tree == scratch`, named diff). **Numbers to beat:** object `cost_us` of the new
`/fold/analytics?cost=1` ≤ the AX5 report's (ESTIMATE: both O(window)); scratch fold wall on 365 days × 30
orders/day ≈ 11k orders printed as the row's baseline FIRST. **Dependencies.** AX5, BN1 (blocks in the object), DW6.
**Size:** 4 lane-days. **Risks.** A third account of the truth: law 11 nightly; recipe generations must be stamped
on the view cells (a cell stepped under recipe gen g is re-derived when g moves). **Papers.** 1207.0137, 2303.08583,
2404.17679, 1804.02780, 2605.08397 (the thresholds), 2606.07795.

### B2. AX10 — Parallelism where it is real

**Where it is NOT.** Inside one Worker invocation and inside one object: single-threaded by the platform (§1).
Kernels are microseconds (R §12.3). Nothing changes there, and the row says so in `docs/design` so it is not retried.

**Where it IS (five places), and what to parallelise:**
1. **Independent object/R2 calls in one Worker request** — `futures_util::future::try_join` is already used at
   eight sites (MEASURED grep: `cloud.rs:648`, `courier.rs:785`, `hubstore.rs:353` `join_all`, `owner.rs:635`,
   `preview.rs:45`, `customers/handlers.rs:66,158`). Not yet: BN2's publish (N `PutObject` per generation →
   `join_all` in slices of 6, the Free "6 simultaneous connections" bound, CM §1.2), the nightly's per-venue loop
   (`cloud.rs:594` `for r in rows` serial → 6-wide, under the 50-subrequest limit per invocation; today each venue is
   its own object so the objects already run in parallel once asked).
2. **`wait_until` after the response** — used once (`lib.rs:178`, trace export). Broadcasting, outbox enqueue
   acknowledgement and the edge `cache_put` (`storefront.rs:219-223`) can leave the response path; CPU is still
   charged, wall is not on the customer's RTT.
3. **Across objects** — the venue object is the unit; cross-venue work (nightly, platform waitlist) is parallel by
   construction; one alarm per venue (FT2) already fans out.
4. **The browser** — a Web Worker for `bebop-wasm` folds/decodes (BN3) keeps the UI thread free; the replica fold
   runs there; message passing is a structured-clone copy of ≤ 10 KB blocks (no SharedArrayBuffer: needs COOP/COEP
   headers, a CSP-class risk — memory `dowiz-csp-blocked-every-stylesheet`; not proposed).
5. **The box / native hub — the only real cores.** bebop's L3 scheduler (`selfhost/prelude/sched.bp`, T-1..T-6:
   index-order assembly, W ≤ 3, ≥ 10 ms per worker per level) MEASURED 2.55–2.7× on three cores for ≥ 40 ms levels,
   1.0× for 0.1 ms tasks (R §3.3). The parallel self-adjusting result — SP-tree-tracked dependencies let change
   propagation run in parallel with work/span bounds (CITED 2105.06712) — maps onto the level structure: a dirty
   set's affected nodes at one level are independent by construction (RT §7). Targets: `dagfull --sweep` (389
   programs, each an independent compile node → W=3 on the sweep; the per-program compile is 14.4–14.7 s cold /
   156–174 ms hit, R §2.1, far above the 10 ms threshold), `battery.sh` arms, and the dagc per-fn compile nodes of
   one program (DG4) where a level's fns sum ≥ 30 ms.

**Design/files.** `workers/api/src/cloud.rs` (bounded `join_all` over venues), `+workers/api/src/hubdo/publish.rs`
(BN2: slices of 6), `bebop-lang/bench/vs_rust/dagfull.sh` (`BEBOP_SCHED_W=3` on the sweep; the `sched.bp` client is
`pool.bp`'s pattern), `bebop-lang/compiler/dagdrv.bp` (level the fn nodes; T-4 decides serial vs W). **RED first:**
`dagfull.sh --sweep` prints `wall_s` with W=0 and W=3 and the SAME byte-identical result (T-1); a nightly test that
two venues' backups finish in < 1.5× one venue's wall. **Numbers to beat:** sweep wall at W=0 (MEASURED by the DGSW
lane: 389/389 — its wall is the baseline to print); nightly wall per venue ≈ 105 ms (BN §1.3, ESTIMATE-carried) ×
venues → ÷ min(6, venues); BN2 publish wall per generation (unmeasured; print first). **Dependencies.** DGSW merged
(sweep); BN2 (publish). **Size:** 3 lane-days. **Risks.** The box's 32-process cap (memory `box-sigkill-phantom`):
`sys_clone` workers are threads not processes (memory `agents-are-not-processes`) but cargo's `-j` is capped at 2 —
W=3 inside bebop is fine, parallel CARGO is not. Subrequest and connection limits on the Worker fan-outs: bound at 6,
count per invocation ≤ 50. **Papers.** 2105.06712, 2004.10908, 2010.11105 (overhead beats cleverness — keep T-4).

### B3. AX11 — Incremental topological ordering of a DYNAMIC edge table

**Goal.** DW2's edge table (`+workers/api/src/hubdo/edges.rs`, `(inputs, projection, step[, out_key])`, RM row 21 / BP
§4 DW2) is planned as a static compile-time table ordered by Kahn once, like `ordfsm.bp`'s FSM signature and
`dl_strata`'s SCC+Kahn at `dl_check` time (`dl.bp:277-443`). Approved target: rules and projections that change at
run time — per-venue rule sets (`dl_sushi.bp` is GENERATED from the menu, `:3`), plug-in projections, a venue
adding a derived view — with the order maintained online and a cycle REFUSED at the insertion that closes it.

**Feasibility.** Yes; the data is tiny (12 projections, ≈ 30 edges; 5–10 predicates per rule set). Online topological
ordering under edge insertion is O(n² polylog n) average-case for the Alpern/Katriel-Bodlaender/Pearce-Kelly
algorithms (CITED 0802.1059, Ajwani & Friedrich); the Pearce-Kelly algorithm's per-insertion cost is bounded by the
affected region between `ord[y]` and `ord[x]` (the PK paper is named by that abstract, not in corpus). Deletions
need no reordering (a valid order stays valid when an edge leaves — true by definition, no citation needed). At this
size the gain is not time (µs either way) but the SEMANTICS: insertion-time refusal naming the edge, and strata that
do not need a full `dl_check` per menu edit.

**Design.** `edges.rs` (≤ 300): `Table { nodes, edges, ord: Vec<u32> }`, `insert(x→y)`: if `ord[x] < ord[y]` done;
else forward DFS from `y` bounded to nodes with `ord ≤ ord[x]`, backward DFS from `x` bounded to `ord ≥ ord[y]`; if
the forward set reaches `x` → `Err(Cycle { path })` (loud, names the edge and the path; the table is unchanged);
else reassign the two sets' positions in sorted order (PK's "reorder the affected region"). `remove(x→y)`: drop the
edge, order unchanged. The Datalog twin: `+bebop-lang/selfhost/std/dl_strata.bp` (moving `dl.bp:277-443` out — also
AX1's headroom) gains `dl_strata_insert(R, p→q, neg)`: the same PK step over the predicate graph; a negative edge
inside the affected region's SCC → the existing E125 refusal (`Datalog.lean` `neg_strictly_lower :194`,
`refusal_necessary :213` already state the property for a given stratification; a new lemma `pk_step_preserves_topo`
is a 20-line finite-graph statement). `+tools/gates/edges.json` (DW3) is emitted from `edges.rs`'s table by a test so
gates read the dynamic table's current state.

**RED first + gate.** `edges::tests::thousand_random_insertions_match_kahn` (RED: no module) and
`edges::tests::closing_edge_is_refused_with_path`; bebop: `dl_strata.py` oracle gains an insert arm. **Numbers:**
order equals a Kahn recompute after each of 10^3 random insertions on 50 nodes; refusal names the edge; `personal-data`
gate unchanged; `dl_event_ns` unchanged (strata are precomputed into `db[32..63]`, `dl_eval.bp:713`, so the online
step is off the event path). **Dependencies.** None (it IS DW2's table, done once); AX3/AX17 add the column; AX12b
reads it. **Size:** 2 lane-days. **Risks.** A table that is data at run time must be in an image (per venue) with a
layout digest; mutation proof: a hand-edited cycle in the image → the loader refuses. **Papers.** 0802.1059.

### B4. AX12 — E-graphs, equality saturation, superoptimisation (five concrete targets)

**The one fact that sizes this section.** bebop's compiler emits AArch64 for the box only; "there is no wasm32
backend in the tree" (BN §4.2); nothing bebop compiles runs in the Worker (Rust/LLVM + `wasm-opt`) or the browser
(0 `.wasm` under `public/`, SI row 46). So an optimiser on bebop IR pays TODAY on: the compiler's own self-compile
(gen2/gen3/gen4 fixpoint), the battery's kernels (K3–K7), the dagc compile nodes, and the store/Datalog libraries
measured on the box. It pays on the REQUEST path only after AX12e. Said plainly so no row claims a Worker CPU gain.

**What exists to build on (MEASURED by reading).** (i) Instruction-level peepholes in `emit_binop_regs`: `madd_try`
(T104, `exp.journal:629`), `shl_try`, `addshift_try`, `mulc_try` (T104b steps 1–2, `:657, :692`), each
objdump-verified; T104b's remaining items were REFUTED as unable to move the K4 kernel (gate ≤ 3.0 ms, measured
5.26 ms; `:704`) — the honest baseline for any optimiser row. (ii) A term-rewriting engine with canonisation and a
Newman local-confluence check of a rule set (`selfhost/std/rewrite.bp`, T31: "a rule set is accepted iff every pair
of reducts joins"; mirror `bench/oracles/rewrite.py`). (iii) The Datalog engine (DG8) — egglog is "Datalog +
congruence closure + extraction" (CITED 2304.04332). (iv) The content-keyed compile memo (`dagc.bp`) and the
`dagfull` byte-identity gate. (v) Lean semantics of the i64 operators incl. the logical `>>` (memory
`bebop-shift-right-is-logical`).

**AX12a — an e-graph rewrite pass on bebop's expression IR (the compiler, on the box).**
*Design.* `+bebop-lang/selfhost/prelude/egraph.bp` (≤ 800): e-nodes `(op, child_class_ids…)` in a hashcons table,
e-classes in a union-find with path compression, the worklist **rebuilding** invariant restoration (CITED 2004.03082:
amortised, "asymptotic speedups ... in practice"), an e-class **analysis** carrying the constant value when every
e-node in the class is a literal (constant folding, egg's example), and extraction by a cost function = emitted word
count per op (the census already counts words: `bebop-lang/tools/census.py`). Rules (data, in `+prelude/egraph_rules.bp`):
`x*1→x`, `x+0→x`, `x*c1*c2→x*(c1*c2)` (T104b item 1), `(x<<a)<<b→x<<(a+b)` only when `a+b<64`, commutativity of
`+`/`*`/`&`/`|`, `x-x→0`, `x&x→x`, `(x+c1)+c2→x+(c1+c2)`; NOTHING on `/`, `%`, `>>` (logical) or comparisons in the
first set — every rule is a theorem over `i64` with bebop's wrap/trap semantics in `+formal/Bebop/Rewrite.lean`
(`Semantics.lean` holds the operators), and `rewrite.bp`'s Newman check runs on the rule set at battery time (a
non-confluent set is refused before it compiles anything). The pass runs per fn between parsing and emission, under
`BEBOP_EGRAPH=1` first (opt-in), with iteration bound 8 and node bound 4,096 per fn (Caviar's early-stop lesson,
CITED 2111.12116); over the bound → the fn is emitted unoptimised and a stderr line says so (loud, never silent).
**Certifying rewrites:** each extracted term carries the rewrite chain (proof-producing union-find, the O(n log n)
greedy of CITED 2209.03398) into the dagc ENTRY's `facts` cell as a digest, so `dagfull` can replay "same proof,
same words".
*RED first + gate.* The rule set itself: `rewrite.bp`-style confluence RED on a deliberately conflicting pair (the
T31 GOOD/BAD shape, `rewrite.bp:12-15`). Then: `bpref.py` oracle on 17 eval probes (the T104b set) unchanged;
`std_golden` 99/99, `construct_parity` 52/52, `pool_parity`, `run_all ok=99`; gen3 == gen4 under `--codegen` (the
self-compile fixpoint — the optimiser must be a fixpoint of itself); census word count printed. NOTE on `dagfull
--sweep`: its arm "byte-identical to the frozen pre-jump binaries" (RT §8) goes RED by construction when codegen
changes — the row re-freezes with the operator's knowledge and the behavioural gates above are the proof, as T104b
did ("constructs re-frozen", `exp.journal:657`). **Numbers to beat:** K4 ≤ 3.0 ms (now 5.26 ms, T104b) — the
measured expectation from T104b is that the TWO open algebraic items cannot move K4; what the e-graph adds over the
peepholes is cross-`let` CSE and commutativity-aware matching, UNMEASURED — the row's first deliverable is therefore
the number: `K3..K7` word counts and ms before/after, printed in `PERF.md`; `selfcompile_wall` ≤ 1.05× control
(RT's DG4 bound) with the pass on. **Dependencies.** DGSW merged (owns `bebop.bp`); N3-C rows share the compiler
files — one writer. **Size:** 5 lane-days. **Risks.** A rule true over Z and false over wrapping i64 (e.g. `(x+c1)+c2`
when the inner add traps — bebop traps on overflow? `dl_mulck` traps on mul overflow, `dl.bp:14`; the Lean file decides
per operator); the oracles catch a wrong fold on the 17 probes; the fixpoint catches self-miscompilation.
**Papers.** 2004.03082, 2108.02290, 2209.03398, 2205.14989, 2111.12116.

**AX12b — Datalog plan selection by saturation.** `dl_plan` (`dl_eval.bp:89`) orders a rule's atoms once; `qplan.bp`
has a join-order model for k ≤ 4. Design: enumerate atom orders as an e-graph of plans (join is associative and
commutative — two rules), cost by the dense-index bounds DESC already carries (`dl.bp:28-31` cells 28..31) and the
relation sizes, extract the cheapest; equality saturation here is the textbook "query plan e-graph" and the plan
space is ≤ 4! = 24 per rule, so saturation is exhaustive. **Gate:** `dl_event_ns` not worse on the 165/120/352
fixture; a constructed 4-atom rule where the default order is 10× worse than the best is found. ESTIMATE value:
small today; it is what makes per-venue generated rule sets (AX11) safe to grow. 2 lane-days, after AX12a's e-graph
file exists (shared engine). **Papers.** 2108.02290 (relational e-matching is the same join problem), 2304.04332.

**AX12c — Congruence closure for supply aliasing (union-find in Datalog).** The product fact: eBills import (B7,
SI row 13) and recipe entry create supplies by NAME; the same physical supply under two spellings is two rows, so
stock, allergen and cost split. `produced(S2,S)` is a directed derivation (closure, not equivalence) and stays a
rule; `same_supply(S1,S2)` IS an equivalence. Design: an EDB relation `alias(S1,S2)` and a union-find in
`+dl_cc.bp` (≤ 300) that canonicalises supply ids at load (`dl_load_*` in `dl_fix.bp:230-251`) and on each `alias`
event (merge two classes, re-emit the merged supply's `stock` rows as a delta — AX1's delete + insert, no stratum
recompute). This is egglog's "union" primitive in dowiz's engine, without the full egglog. Owner UI: "merge supplies"
in `admin/ingredients*.js` writes `alias`. **Gate:** `dl_allergen` golden unchanged; new `dl_alias.py`: two names →
one class; stock totals sum; `dagfull datalog` incremental == scratch after 10^3 alias events. 2 lane-days, after
AX1 (shares DESC/CNT). **Papers.** 2304.04332, 2209.03398.

**AX12d — Superoptimisation of the Worker wasm, measured honestly.** Souper over LLVM shrank the code section of
8 of 12 programs (CITED 2002.10213; a 4-page workshop paper, no speedups reported) and found optimisations shipped
in LLVM (CITED 1711.04422); Minotaur +7.3 % on GMP (CITED 2306.00229). dowiz's binary is sized by crates, not
peepholes (BN §5.2: 4,747,270 B raw / 1,840,961 B gzip MEASURED). Design: (1) a one-afternoon measurement row on the
box: `wasm-opt` passes beyond the current config (`-O4`, `-Oz`, `--converge`, `--flatten --rereloop`, `--dce
--vacuum --merge-blocks`) on `index_bg.wasm`, each printed as raw/gzip bytes and the Node `compile+instantiate` ms
(the cost map's `measure.mjs`), appended to `docs/research/2026-09-27-binary-size.md`; (2) Souper/Minotaur cannot
run on the box (LLVM build; 1.6 GB RAM, memory `claude-install-and-ecc`) — a `workflow_dispatch` CI job (R3, needs
OA-1) that runs `souper` over the Worker's LLVM IR (`-C save-temps`/`--emit=llvm-ir`) and reports candidate count
and bytes saved; it never deploys. **Gate:** the printed table; `file-size`/`bytes.baseline`-style ratchet on
`index_bg.wasm` is BN5's — AX12d only feeds it numbers. **Numbers to beat:** 4,747,270 B raw; 92 ms compile
(BN §5.1). ESTIMATE 1–5 % from wasm-opt variants; Souper's share UNMEASURED until the job runs. 1 lane-day (+ CI).
**Papers.** 1711.04422, 2002.10213, 2306.00229, 1211.0557.

**AX12e — Making bebop code eligible for the request path (so AX12a-c matter there).** Three steps, the last an
operator decision:
1. **Serve `bebop-wasm` to the browser** (BN3/DW1; SI row 46 "0 `.wasm` in public"): the reader module is 31,626 B
   (`crates/bebop-wasm/bytes.baseline`, MEASURED); compiling the DG9 block module in moved it +143 B (`lib.rs:27-31`);
   with `bw_block` + `bw_proj` exported it is the device's verifier of blocks, projections and node keys. This is
   Rust-compiled wasm reading bebop FORMATS — no bebop CODE runs yet.
2. **Wire DG8 Datalog into the hub** (SI row 49: 0 refs from Worker/hub): the five rules as a Rust twin module
   `crates/dowiz-hub/src/rules/` (the R §12.3 Rust probe measured 82 ns/event — that probe is the seed) evaluated in
   the object on the dirty set, with the bebop `dl.bp` as the oracle through `dagfull datalog`'s cross-check; rules
   stay DATA with a `K64` contract (BN §4.2 I1 verdict: "rules yes, bytecode no").
3. **A bebop wasm32 backend** (design-first row; the ONLY way bebop-COMPILED code reaches the Worker/browser):
   position-independent emission with relocations exists for AArch64 (RT §5); a wasm32 emitter is a second target of
   the same IR — the e-graph pass of AX12a then optimises both. Operator D-2 refused "a bebop VM in the browser" (a
   second RUNTIME); a compiled wasm MODULE is a Rust-wasm peer, not a runtime — but the decision is the operator's
   (§8). Size L (ESTIMATE ≥ 10 lane-days; the AArch64 emitter is the precedent). Gate: the 4-reader gate becomes
   5-reader (bebop-compiled wasm32 reads the same fixtures); `bytes.baseline` for the module.
Until step 3 lands, every AX12a-c gain is a box-side gain and the plan says so in its row text.

### B5. AX13 — "Zero-copy" between Worker and Durable Object: what is impossible, what is built instead

**What is literally impossible, with the citation.** A Worker and its object are two isolates: the object "is
inherently single-threaded" and runs where its first `get()` happened and does not move (§1; BN §1.1), while the
Worker runs at the customer's edge; the only co-located IPC the platform offers is a service binding between two
WORKERS ("same thread of the same Cloudflare server", §1) — a Durable Object cannot be a service-binding target. RPC
to an object is serialised ("maximum serialized RPC limit is 32 MiB") and billed per method call. Faasm/Roadrunner
style sharing (memory-mapping the function's linear memory into a data hose, 44–89 % latency cut, 69× throughput —
CITED 2511.01888; 2002.09344) needs one host process; Cloudflare does not expose one. So: **no shared memory, no
mmap, the hop is a network call; zero-copy of the BYTES across it is not a thing on this platform.** What IS on the
table is every copy and every parse on BOTH sides of the hop that is not the hop itself — and today there are
several.

**What happens today on one storefront menu read (MEASURED by reading).** Worker: `menu_body` →
`stub.fetch_with_request` → object renders/serves the memo bytes → `edge::Stub::fetch_with_request` wraps the
`worker::Response` into `wire::Reply` by BUFFERING the body into a `Vec<u8>` (`edge.rs:130-133` → `wire.rs:216`,
`Reply.body: Option<Vec<u8>>` `:156`) — copy 1 (JS→wasm linear memory); `menu_body` then does `res.text().await?`
(`fold/menu_edge.rs:24-26`) — UTF-8 validation into a `String`; `Response::ok(body)` (`storefront.rs:213`) — copy 2
(wasm→JS); `res.cloned()` for `cache_put` (`:219-223`) — a tee. The object side: `Response::ok(body)` from the memo's
`String` — copy 0 (wasm→JS inside the object). The ONE handler that already does it right is the socket upgrade:
`live.rs:127` returns `stub.fetch_with_request(out).await` UNTOUCHED — the object's `Response` object is handed back
to the runtime, body stream and all. That is the pattern.

**Design — the zero-PARSE pass-through, DG7 end to end.**
1. **`Stub::fetch_through(req) -> worker::Response`** in `edge.rs` (Live arm: return the inner `worker::Response`
   as-is; Mem arm for tests: wrap the `Reply`). Every `/fold/*` reader that forwards bytes (menu, products,
   analytics, kitchen, stock, basket, exceptions, orders list) uses it: the body is a `ReadableStream` the runtime
   streams "as it becomes available" (§1 streams page) — JS→JS, never entering wasm linear memory. Headers the
   Worker must set (cache-control, content-type) are set on the forwarded response (`live.rs`'s lesson at `:117-121`:
   headers on a response that crossed a fetch are immutable — so the OBJECT sets them, or the Worker builds
   `Response::from_body(res.body())` with the same stream, which is a header copy, not a body copy).
2. **The object answers blocks, not JSON, on block routes:** `fold_block` (`hubdo/menu.rs:65`,
   `application/vnd.dowiz.block`) already exists with no reader (SI row 45); BN3's browser `bw_block` view is the
   reader. The storefront then carries DG7 bytes DO → Worker (streamed) → browser (viewed in place): **one
   serialisation (the block encode, once per generation in the object) instead of three (object JSON build, Worker
   parse/rebuild, browser `JSON.parse`).**
3. **Writes pass through too:** a placement body today is parsed in the Worker (`storefront.rs:411`
   `serde_json::from_str`, SI §2 row 2) and re-serialised for the object; BN4's `POST /fold/command` forwards the
   raw body after the HMAC/principal check (the Worker parses NOTHING: `req.body()` stream → `fetch_through`), and
   the object parses once with `crate::body` strict (memory `worker-deny-unknown-fields-inert`). This also fixes the
   `scheduled_for_ms` drop at ingress as a side effect (one parser, in the object).
4. **Inside the object, the wasm↔JS boundary:** `put_bytes`/`get_chunks` (`hubdo.rs:1038`, `:316`) — verify they
   move `Uint8Array` views, not `Vec<u8>` through serde (the D1 BLOB lesson, memory `d1-blob-is-a-js-number-array`:
   one JS value per byte); `worker 0.8.5` has `put_raw`/`put_multiple_raw` (`durable.rs:379, 408`) for exactly this.
   UNVERIFIED today — command: `grep -n "put_bytes\|get_chunks" -A6 workers/api/src/hubdo.rs | grep -n "Uint8Array\|serde"`.
5. **RPC instead of `fetch`:** eligible (compat date, §1) and the Rust crate exposes `Stub::into_rpc<T: JsCast>()`
   (`durable.rs:61`); the gain is HTTP framing, not the hop (BN §1.2: 1.2–2× on a tiny call's serialisation share,
   ESTIMATE; 0× on the count — each method call is a billed request). Blocked today by the object being a Rust
   `#[durable_object]` class that exposes `fetch`/`alarm`/websocket handlers — RPC methods need a JS wrapper class or
   crate support: UNVERIFIED, command: `grep -rn "rpc\|Rpc" ~/.cargo/registry/src/*/worker-macros-0.8*/src`. Kept as
   an optional step 6 after measurement, not a dependency.

**How much of the original benefit it captures.** Of the hop (network, 1–25 ms ESTIMATE, BN §1.3): 0 % — physics.
Of the Worker-side CPU per forwarded body: 100 % of copies 1 and 2 and the UTF-8 pass (ESTIMATE 50–300 µs per
request on small bodies, BN §5.1; proportionally more on the 70 KB JSON list bodies) and all of the allocation; of the
object-side JSON build on block routes: 100 % (the block is encoded once per generation). Of the 250 MB/day the
object sends the Worker (MEASURED): the bytes still cross, but are no longer copied into wasm or parsed. Honest
expectation on `cf.cpu_p50_us` (MEASURED 6,800 µs): the pass-through removes a term of the order of 0.1–0.3 ms per
remaining request (ESTIMATE, BN §5.3) — a 3–5 % term today, a 30–50 % term of the ≈ 0.3–1 ms floor BN §1.3 projects
after BN1/BN4. It is the row that makes the floor reachable, not a 10× on its own.

**Files.** `workers/api/src/edge.rs` (`fetch_through`), `wire.rs` (no change; `Reply` stays for tests and for the
handlers that must read), `fold/menu_edge.rs`, `storefront.rs:177-225` (menu handler returns the stream; `cache_put`
takes the live response), `hubstore.rs` (`/fold/*` readers that only forward), `+hubdo/command.rs` (BN4: raw body
in), `+workers/api/public/lib/blocks.js` + `crates/bebop-wasm` `bw_block` export (BN3), `hubdo/menu.rs` (object sets
cache headers). Each ≤ 300; `storefront.rs` is 1,123 lines (MEASURED) and over the gate already — the menu handler
moves to `+storefront/menu.rs` in this lane (net −60).

**RED first + gate.** Native: `edge::tests::fetch_through_does_not_read_body` — a Mem object answering a 1 MB body;
the Worker-side handler's `Reply` is never materialised (a counting body type). Live: AX0/BN8 per-route CPU on
`/api/public/locations/:slug/menu` MISS and HIT and on `/api/owner/orders` before/after — **numbers to beat:** the
per-route p50 printed FIRST by BN8 (unmeasured today); `cf.cpu_p50_us` 6,800 not up; `cf.do_response_bytes_day`
unchanged (the bytes still cross — if it FALLS the block route took over, which is BN3's number); a `wasm` memory
probe: peak linear memory on the orders-list route ≤ the body size (today ≥ 2× body). Byte-identity gate: the body
the browser receives is byte-equal to the object's (Playwright compares the DO's `/fold/menu` bytes fetched by an
owner token with the storefront's — no re-serialisation). **Dependencies.** BN8 (the number), BN1 (the `/fold/*`
readers exist — 23 sites left, `dataflow.baseline` MEASURED `sites=23`), BN3 (the browser block reader), BN4 (raw
command bodies). **Size:** 3 lane-days. **Risks.** A streamed body cannot be inspected for the 404/other status
mapping `menu_body` does (`menu_edge.rs:27-31`) — status and headers are available without the body, the mapping
stays; `cache_put` with a stream needs the tee (`res.cloned()` exists); immutable headers after a fetch (`live.rs`'s
500) — the object sets them. **Papers.** 2511.01888, 2002.09344, 1810.00556 (TZC: partial serialisation = our
"encode once, view everywhere").

---

## C. The "ideas worth lifting", as rows

**AX14 — Z-set weights in the Datalog relation store.** AX1's CNT cell becomes a signed `i64` weight: a relation is a
map row → weight, insert = `+1`, delete = `−1`, presence = weight > 0 for set semantics (DBSP's Z-sets, CITED
2203.16684; "Datalog over semirings" for the semi-naive validity on the ordered semiring, CITED 2105.14435). Why a
row of its own: it unifies AX1 (counts), AX5/AX9 (ring payloads: `(count, revenue)` is a Z² weight) and AX12c (a
merge re-emits weights) under one cell and one oracle (`dl_set_semantics.py` → `dl_zset.py`). Files: `dl_del.bp`
(AX1's), `dl.bp` DESC (the same cells), oracles, `Datalog.lean` (`zset_present_iff_pos`). Gate: `dagfull datalog`
mixed events; a weight that goes negative is a trap 125 (loud). 1 lane-day after AX1.

**AX15 — CALM as the browser replica's safety argument, made a test.** The replica folds a PREFIX of a single-writer
log: monotone, so coordination-free reads are consistent (CITED 2210.12605; free termination 2502.00222). Today
`replica.js:14-18` says it informally ("the server is still the authority"). Row: (1) `docs/design` paragraph in
BN3's blueprint stating the theorem's premise and the ONE place it fails — a whole-image write (GAP, AX2) — which is
why `None` forces a refetch; (2) the test: BN3's "a delta applied == a full refetch (golden)" extended to a random
interleaving of deltas and GAPs, and the `replica disagrees` path (DW1) classified as CORRUPTION DETECTOR (K64 of the
list differs after a full refetch) never as conflict resolution; (3) `e2e/kit-regression` Playwright: kill the
network mid-delta, resume, state equals server. 1 lane-day inside BN3/DW1. No Lean: the finite statement is the
test.

**AX16 — Deletion SLO (Lethe)** — written into AX7 above as `+tools/gates/deletion-slo.sh`. 0.5 lane-day there.

**AX17 — Early cutoff as a DW2 edge-table column** — written into AX3 above (`out_key`). 0 extra days.

---

## 6. Implementation sequence (waves of ≤ 3 file-disjoint lanes)

| Wave | Lane 1 | Lane 2 | Lane 3 | Gate to leave the wave |
|---|---|---|---|---|
| **0** | **AX0-COUNTERS** (`+hubdo/counters.rs`, `+hubdo/deltas.rs` move, `cf.mjs`) | **AX8** (`bebop-lang/docs/PERF.md`, `battery.sh` print, `hubstore.rs` header) | **AX11** (`+hubdo/edges.rs`, `+std/dl_strata.bp`, `dl_strata.py`) | four indicators print nightly; `compaction_ms_per_mb` printed; `edges` tests green, `personal-data` 0 |
| **1** | **AX3+AX17** (`fold/menu.rs`, `hubdo/menu.rs`, `edges.rs` column) | **AX2** (`hubdo/deltas.rs`, hub GENTAB + twins, `replica.js`, `owner.rs`) — starts after 7 AX0 prints | **AX1** (`+std/dl_del.bp`, `dl.bp`, `dl_eval.bp`, `gen_dl.bp`, oracles, `Datalog.lean`) — after DGSW | settings-write test green; `cf.since_none_share` ≤ 1 %; `dl_event_del_ns` ≤ 10× insert, `dagfull datalog` mixed green |
| **2** | **AX4** (`proj.rs` kind 3 + 3 twins, `fold/projection.rs`, `rebuild.rs`) — only if `wakes_readonly_share` ≥ 30 %; else the lane becomes AX5 | **AX13** (`edge.rs`, `fold/menu_edge.rs`, `+storefront/menu.rs`, `hubstore.rs` readers) | **AX12c** (`+std/dl_cc.bp`, `dl_fix.bp`, `dl_alias.py`, `admin/ingredients*.js`) | `cold_fold_us_p50` ≤ 0.5 ms, U9 rows ≤ 21/20; per-route CPU p50 down on menu/orders, body byte-equal; alias fixture 2→1 |
| **3** | **AX5** (`+fold/analytics_view.rs`, `reads.rs`, law 10) | **AX7+AX16** (`hubdo/forget.rs`, `+hubdo/keys.rs`, `registry.rs`, `conservation.mjs`, `+gates/deletion-slo.sh`) — BEFORE BN3 | **AX14** (`dl_del.bp` weights, `dl_zset.py`, `Datalog.lean`) | view == fold over 10^4; law `redacted == declared` + SLO 0 hits; `dagfull datalog` with negative-weight trap |
| **4** | **AX9** (`+fold/analytics_tree.rs`, `+services/analytics/cost.rs`, law 11) | **AX6** (`+std/dl_wit.bp`, `gen_dl.bp`, `+hubdo/provenance.rs`, `admin/more.js`) | **AX10** (`cloud.rs` fan-out, `publish.rs` slices, `dagfull.sh` W=3, `dagdrv.bp` levels) | tree == scratch; witness golden + `dl_event_ns` ≤ 1.3×; sweep W=3 byte-identical and faster |
| **5** | **AX12a** (`+prelude/egraph.bp`, `+egraph_rules.bp`, `+formal/Bebop/Rewrite.lean`, compiler hook) — one writer on the compiler | **AX12d** (`binary-size.md` table; CI `workflow_dispatch` job — needs OA-1) | **AX15** (BN3/DW1 blueprint paragraph + the interleaving test + Playwright) | K3–K7 numbers printed, gen3 == gen4, oracles 17/17; wasm-opt table; replica test green |
| **6** (operator) | **AX12e step 3** design row: bebop wasm32 backend (one page, gate numbers) | **AX12b** plan saturation (after AX12a) | **AX12e steps 1–2** ride on BN3 and DW2 | design accepted or refused in writing |

Total ≈ 46 lane-days (ESTIMATE: AX0 1, AX1 4, AX2 3, AX3 2, AX4 4, AX5 2, AX6 3, AX7 3.5, AX8 0.5, AX9 4, AX10 3,
AX11 2, AX12a 5, AX12b 2, AX12c 2, AX12d 1, AX13 3, AX14 1, AX15 1), excluding AX12e step 3. Slots AFTER the approved
queue (RM "the one ordered list" rows 1–36): the BN wave owns `hubdo.rs`'s write path (BN4) and `replica.js` (BN3) —
AX2/AX4 land inside or after those lanes, never beside them. Every lane: RED first, `run-all` after merge (memory
`run-all-after-every-merge`), bebop rows `battery.sh` full.

---

## 7. Roadmap-ready rows

| ID | Row | Owns (disjoint within its wave) | Acceptance (a number in a gate) | Deps | Size |
|---|---|---|---|---|---|
| **AX0** | Counters: since/None share, read-only wakes, cold fold µs, rows per write | `+hubdo/counters.rs`, `+hubdo/deltas.rs` (moved), `cf.mjs`, health | 4 indicators nightly with baselines, `samples=` printed; `hubdo.rs` net ≤ 0 lines | TELEM merge | 1 |
| **AX1** | Deletion-aware Datalog: Counting + DRed + closure module | `+std/dl_del.bp`, `dl.bp`, `dl_eval.bp`, `gen_dl.bp`, oracles, `Datalog.lean` | `dl_event_del_ns` ≤ 10 × `dl_event_ns` (≤ 1,000); `dagfull datalog` 10^4 mixed == scratch; `arch_check` 800; `sorryAx` 0 | DGSW | 4 |
| **AX2** | Every `since=` a delta: GENTAB in the log image, cold wake answers | `hubdo/deltas.rs`, hub GENTAB + 3 twins, `replica.js`, `owner.rs` | `cf.since_none_share` ≤ 1 %; `proj_rows` per append unchanged; `rebuild.stale = []` | AX0 (7 prints) | 3 |
| **AX3** | Early cutoff: `out_key` (K64 + bytes; K256 across machines) | `fold/menu.rs`, `hubdo/menu.rs`, `edges.rs` column, `publish.rs` (BN2) | settings write w/o menu effect → no gen bump, no `moved`, 0 R2 writes; Class A per price edit ≤ 3 | DW2/AX11, BN2 | 2 |
| **AX4** | Orders projection as PROJ kind 3 in the log image | `proj.rs` + 3 twins, `fold/projection.rs`, `rebuild.rs`, `logimage.rs` | `cold_fold_us_p50` ≤ 500 µs (from AX0's baseline); U9 ≤ 21 rows / 20 placements; 4/4 readers; law 8 on the persisted copy | AX0 ≥ 30 % read-only wakes; DGSW; before/inside BN4 | 4 |
| **AX5** | Ring analytics view, exact i64 money | `+fold/analytics_view.rs`, `reads.rs`, law 10 | view == `fold::fold` over 10^4 events incl. DST; `/fold/analytics` `cost_us` ≤ baseline; `float-money` 0 | AX4, DW6 | 2 |
| **AX6** | Why-provenance: one witness per derived row | `+std/dl_wit.bp`, `gen_dl.bp`, oracles, `+hubdo/provenance.rs`, `admin/more.js` | witness golden on sushi fixture; `dl_event_ns` ≤ 1.3 × pre-row; `dagfull datalog` unchanged | AX1, DW2/DW5 | 3 |
| **AX7** | Shred before replicate (DG10w) + **AX16** deletion SLO gate | `hubdo/forget.rs`, `+hubdo/keys.rs`, `registry.rs`, `conservation.mjs`, `+gates/deletion-slo.sh`, Cargo features | law `redacted == declared` RED on one tombstone; SLO: 0 hits in images + archive + IndexedDB; `personal-data` 0 | DG10 hub; R5; BLOCKS BN3 | 3.5 |
| **AX8** | Compaction policy named and printed | `PERF.md`, `battery.sh`, `hubstore.rs` header | `compaction_ms_per_mb` printed with baseline (basis 747 ms / 85 MB) | — | 0.5 |
| **AX9** | Factorised analytics tree (order × dish × supply, ПФ chain) | `+fold/analytics_tree.rs`, `+services/analytics/cost.rs`, law 11 | tree == scratch after 10^4 events + 100 recipe edits; `cost_us` ≤ AX5's; scratch wall at 11k orders printed | AX5, BN1, DW6 | 4 |
| **AX10** | Parallel fan-outs (6-wide) + L3 scheduler on sweep/compile | `cloud.rs`, `publish.rs`, `dagfull.sh`, `dagdrv.bp` | sweep W=3 byte-identical, wall printed vs W=0; nightly 2 venues < 1.5 × one | DGSW, BN2 | 3 |
| **AX11** | Dynamic edge table with online topo order (PK), cycle refused at insert | `+hubdo/edges.rs`, `+std/dl_strata.bp`, `dl_strata.py`, `edges.json` emitter | 10^3 insertions == Kahn; refusal names the path; `dl_event_ns` unchanged | — | 2 |
| **AX12a** | E-graph rewrite pass on bebop IR (box) | `+prelude/egraph.bp`, `+egraph_rules.bp`, `+formal/Bebop/Rewrite.lean`, compiler hook | rule set confluent (Newman) or refused; oracles 17/17; `std_golden`, constructs, gen3 == gen4; K3–K7 ms/words printed; `selfcompile_wall` ≤ 1.05× | DGSW, N3-C writer | 5 |
| **AX12b** | Datalog plan saturation | `dl_eval.bp` `dl_plan`, `qplan.bp`, `egraph.bp` | `dl_event_ns` not worse; constructed 10× case found | AX12a, AX11 | 2 |
| **AX12c** | Supply aliasing by congruence closure | `+std/dl_cc.bp`, `dl_fix.bp`, `dl_alias.py`, `admin/ingredients*.js` | 2 names → 1 class; `dl_allergen` unchanged; `dagfull datalog` 10^3 alias events | AX1 | 2 |
| **AX12d** | Worker wasm superoptimisation, measured (wasm-opt on the box; Souper in CI) | `binary-size.md`, `.github/workflows` (dispatch job) | table of raw/gzip/compile-ms per pass set vs 4,747,270 B / 92 ms; Souper candidates counted | OA-1/R3 for CI | 1 |
| **AX12e** | bebop on the request path: wasm served (BN3), DG8 wired (hub twin), wasm32 backend DESIGN | `crates/bebop-wasm` exports, `+crates/dowiz-hub/src/rules/`, `+docs/design/BEBOP-WASM32-<date>.md` | 5-reader gate when the backend lands; until then the design page with gate numbers | BN3, DW2; **operator** | 2 + L |
| **AX13** | Zero-parse pass-through DO → Worker → browser, DG7 end to end | `edge.rs`, `fold/menu_edge.rs`, `+storefront/menu.rs`, `hubstore.rs` readers, `+hubdo/command.rs` (BN4), `blocks.js` (BN3) | body byte-equal DO ↔ browser; per-route CPU p50 down vs BN8 baseline; linear-memory peak ≤ body; `cf.do_response_bytes_day` unchanged | BN8, BN1, BN3, BN4 | 3 |
| **AX14** | Z-set weights in the relation store | `dl_del.bp`, `dl.bp` DESC, `dl_zset.py`, `Datalog.lean` | `dagfull datalog` mixed; negative weight = trap 125 | AX1 | 1 |
| **AX15** | CALM: replica monotonicity as a test, "disagrees" = corruption detector | BN3 blueprint, `replica.js` test, Playwright | random delta/GAP interleaving == full refetch; network-kill resume equal | BN3/DW1 | 1 |

---

## 8. What needs the operator

| | Decision / action | Why only the operator | Rows blocked |
|---|---|---|---|
| **OD-1** | Re-read D-2 ("no second runtime in the browser") against a COMPILED bebop wasm32 module: refuse, or approve the design row | D-2 is an operator decision; this plan does not reinterpret it | AX12e step 3; without it AX12a-c stay box-side (still useful, said so) |
| **OD-2** | Approve two irreversible format additions in LIVE images: PROJ kind 3 (AX4) and GENTAB (AX2) in venue log images; the DG10 key-table image (AX7). Older readers read cell 5 = 0 and skip unknown trailing objects (`proj.rs:5`), so forward-compatible — but once written on a live venue, the image carries them | written-format law: every reader/oracle/golden in the same commit; a live image cannot be un-written | AX2, AX4, AX7 |
| **OA-1** (standing) | CI deploy token as a GitHub secret | Souper/LLVM cannot run on the box (RAM, process cap); R3 is the only place | AX12d's second half |
| **OD-3** | Confirm the AX4 gate value: build the persisted PROJ only if `wakes_readonly_share` ≥ 30 % (this lane's proposal) | a threshold is a product judgement | AX4 |
| none | No secret, no plan change, no paid feature, no compat flag is required by any row above | — | — |

---

## 9. Not verified here, and the command that settles each

| # | Claim used | Command |
|---|---|---|
| V1 | `put_bytes`/`get_chunks` move `Uint8Array` views, not serde `Vec<u8>` (AX13 step 4) | `grep -n "put_bytes\|get_chunks" -A6 workers/api/src/hubdo.rs \| grep -n "Uint8Array\|serde\|JsValue"` |
| V2 | `worker-macros 0.8` can expose RPC methods on a Rust `#[durable_object]` (AX13 step 5) | `grep -rn "rpc\|Rpc" ~/.cargo/registry/src/*/worker-macros-0.8*/src ~/.cargo/registry/src/*/worker-0.8.5/src/durable.rs` |
| V3 | A PROJ/GENTAB appended at the image tail lands in the chunk the append rewrites (+0 rows) | AX0 `proj_rows`; U9 `cf.do_rows_written_day` over 20 placements ≤ 21 |
| V4 | `dl_recompute_s` baseline time for a recursive delete (AX1's number to beat) | `bash bebop-lang/tools/slot.sh axplan sh -c 'cd bebop-lang && sh bench/vs_rust/dl_bench.sh dl_allergen_del'` after the fixture exists |
| V5 | The sweep wall at W=0 (AX10's baseline) | the DGSW lane's `dagfull --sweep` log line (`/root/lanes/w-dgswitch`) |
| V6 | Per-route Worker CPU (AX13's number) | BN8 collector, not started by W-TELEM (RM row 14) |
| V7 | Whether an e-graph pass moves K4 at all (T104b says the two algebraic items cannot) | AX12a's first deliverable IS this number |
| V8 | Souper's effect on the Worker wasm | the CI job (OA-1) |

No build, benchmark or sub-agent was run by this lane: every number above is quoted from a dated document or from a
file read today; the three `slot.sh` runs allowed were not needed to settle a feasibility question.

---

## 10. Papers used (arXiv ids; abstract read; full text where the row says so)

1711.03987 (Counting/DRed/B-F hybrid) · 1811.02304 (modular materialisation, TC module) · 2203.16684 (DBSP, Z-sets)
· 2308.04214 · 1811.06069 · 2105.14435 (semi-naive over semirings) · 2303.12773 (why-provenance complexity) ·
1907.05045 (1.27× provenance) · 2202.10766 · 1808.05752 · 1105.2255 · 1207.0137 (DBToaster viewlets) · 2303.08583
(F-IVM rings, factorised views) · 2404.17679 · 2606.07795 · 1804.02780 · 2605.08397 (thresholds only) · 1509.07454
(rejected: sampling) · 2108.12469 · 2002.06183 · 1106.0478 · 2101.09355 · 2104.13869 · 2105.06712 (parallel
self-adjusting, SP-trees) · 2004.10908 · 2010.11105 · 0802.1059 (online topological ordering) · 2004.03082 (egg:
rebuilding, analyses) · 2108.02290 · 2304.04332 (egglog) · 2209.03398 (small proofs from congruence closure) ·
2205.14989 · 2111.12116 (Caviar early stop) · 1711.04422 (Souper) · 2002.10213 (Souper on Wasm: 8/12 smaller) ·
2306.00229 · 1211.0557 · 2511.01888 (Roadrunner) · 2002.09344 (Faasm) · 1810.00556 (TZC) · 2006.04777 (Lethe) ·
2104.01146 · 2004.00107 · 1603.01529 · 2210.12605 (CALM) · 2502.00222 · 2202.04522 · 1812.07527 · 2005.00044.
