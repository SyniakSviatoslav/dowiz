# SPEC: the Datalog rule layer and the universal block codec (normative), 2026-09-28

**Status.** Normative companion to `docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md` (cited as **RT §n**),
written by lane W-SPEC (docs only, tree `f334e5cb`) from `docs/research/2026-09-28-bebop-dag.md` (**R §n**) and
the operator's decisions D-3 (Datalog mandatory) and D-4 (tensors + universal codec). Rows and acceptance
numbers: `docs/design/BLUEPRINT-BEBOP-DAG-2026-09-28.md`. Labels as in RT: MEASURED / DOC / EST / HYPOTHESIS.
Every MUST names its check; §C is the matrix; §D records where research and operator disagree.

**Path convention.** A backticked path is a file in the tree at `f334e5cb` (checkable with `ls`); a path written `+like/this.rs` is a file the named row CREATES and does not exist yet, so `tools/gates/paths.sh`'s rule (a cited path must exist) is not fooled by a plan.

Two parts. **Part A** is the rule layer: rules as data, semi-naive evaluation over the dirty set, stratified
negation with its refusal code, the five first rules with exact relations, the gate and the Lean statements.
**Part B** is the block codec: one byte layout that is at once the CAS block, the wire format and the memory
format; per-type encodings; the round-trip gate; the money rules; and the dense-SIMD form as a gated row.

---

## Part A — the reactive Datalog layer

### A.1 Where it lives (R §12.1)

In bebop, as a LIBRARY over the store, compiled to the SAME node graph as everything else: a rule is a
projection node (RT §1.1) whose inputs are the relations its body reads and whose output is the relation it
derives. The seed is `bebop-lang/selfhost/std/qdsl.bp` (parser) and `bebop-lang/selfhost/std/qplan.bp`
(join order for k ≤ 4 tables, dense accumulators, CSR choice); what they lack is recursion, negation and an
incremental step. The surface stays imperative (Eve's lesson, `bebop-lang/docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md`
§7): rules are DATA, and a generator (`+bebop-lang/selfhost/std/gen_dl.bp`, new, in `gen_gb.bp`'s shape:
write bebop SOURCE into a buffer, compile it) turns each stratum into one `pure fn`.

**A-1** The Datalog core MUST NOT be a second language surface: no new keywords in `bebop.bp` beyond RT §3's
`pure`/`sched`/`out`; rules are cells. Check: `bebop-lang/bench/vs_rust/census.txt` (branch census) and
`bebop-lang/tools/builtin_surface.py` unchanged by the rule layer's commit.

### A.2 Relations

A relation is a **columnar block** (Part B) held as a store object node: `n` rows, one column per attribute,
attributes are i64 (ids are interned: a string id becomes its `K64` over the bytes, RT §2.2, and the intern
table is itself a relation `name(Id, Bytes)`). Relation kinds:

| Kind | Representation | Used for |
|---|---|---|
| EDB (source) | block of `k` i64 columns, rows sorted lexicographically | facts folded from the log / catalogue (`recipe`, `stock`, `state`, `assigned`, `holds_person`, `edge`) |
| binary relation, dense on the left | CSR: `row_ptr[n+1]`, `col[nnz]`, optional `val[nnz]` (`bebop-lang/selfhost/std/csr.bp` layout, integer `vv`) | `recipe(D,S,C)`: `row_ptr` over dishes, `col` = supply, `val` = grams |
| IDB (derived) | block, sorted, deduplicated | rule heads |
| Δ (delta) | block, the rows added in the current round | semi-naive state |

**A-2** Every relation MUST be sorted and deduplicated after each round (set semantics); the sort is the
counting sort `qplan.bp` already chooses for `|keys| ≤ 4096`, radix otherwise. Check: `dl_set_semantics`
gate — inserting the same fact twice yields `n` unchanged and the block's `K256` unchanged.

### A.3 Rules as data (byte-exact)

A **rule set** is one store object `RULES` and its atoms:

```
RULES {version=1, n_rules, n_preds, ref PREDS, ref RULEV, ref ATOMS}
PREDS  per predicate: {name_key64, arity, stratum, kind(0 EDB | 1 IDB)}                    4 cells
RULEV  per rule:      {head_pred, n_head_args, head_args[4] (var ids, -1 unused), atom_off, n_atoms}   8 cells
ATOMS  per body atom: {pred, neg(0|1), n_args, args[4]}  where an arg is
                      var id v ≥ 0, or a constant encoded as (-(c) - 2), or -1 = unused             7 cells
```

Variables are numbered per rule from 0; a variable in a NEGATED atom MUST also occur in a positive atom of the
same rule (safety). Arithmetic guards are atoms over the built-in predicates `lt(A,B)`, `le`, `eq`, `mul(A,B,C)`
(C = A·B, checked i64: overflow is a refusal, Part B §B.5), which are stratum 0 and never materialised.

**A-3** `gen_dl` MUST refuse an unsafe rule (a head or negated-atom variable that no positive atom binds) with
**exit 125** and `error[E125]: rule <i> unsafe -- variable <v> not bound by a positive atom`. Check:
`+bebop-lang/bench/vs_rust/std_tests/dl_neg_unsafe.bp` (`gate dl_neg_unsafe` `EXPECT exit:125`) and its positive twin.

### A.4 Stratification and the refusal (R §12.1 "Stratified negation")

Build the predicate dependency graph: an edge `p → q` for each body atom `q` of a rule with head `p`, marked
NEGATIVE when the atom is negated. Strata = the Kahn order over the CONDENSATION (SCCs) of that graph — the
same `kahn` shape as `bebop-lang/selfhost/std/ordfsm.bp:208`; each SCC is one stratum; recursion lives inside
an SCC.

**A-4** A NEGATIVE edge inside an SCC (negation through recursion) MUST be refused by `gen_dl` with **exit
125** and `error[E125]: negation inside a cycle -- <p> depends negatively on <q> in the same stratum`. A
negative edge between strata requires the negated predicate to be in a LOWER stratum, which the condensation
order gives by construction. Check: `dl_neg_cycle.bp` (`p :- q, not p` shape, `EXPECT exit:125`); positive
twin `dl_strata.bp` prints the stratum vector and its golden equals `+bebop-lang/bench/oracles/dl_strata.py`'s.
**A-5** Evaluation MUST run stratum by stratum, lowest first; a stratum's negated inputs are complete before
it starts (they belong to lower strata). Check: `dagfull datalog` arm (A.7) on the `courier_may` rule set.

### A.5 Semi-naive evaluation over the dirty set (R §12.1, §12.2 `delta_avail`)

Per stratum `s`, with full relations `F_p` and deltas `Δ_p` for its predicates:

```
round 0:  Δ_p := rows of F_p that changed since the memoised generation (the DIRTY SET, RT §1.1) for EDB p;
          Δ_p := F_p for a cold start
repeat:
    for each rule r in s, for each body atom position j that is a stratum-s predicate:
        new := eval(r) with atom j read from Δ and every other stratum-s atom from F (positive atoms of
               LOWER strata always from F; negated atoms from F of the lower stratum -- complete)
        Δ'_head ∪= new \ F_head
    F_p := F_p ∪ Δ'_p ; Δ_p := Δ'_p           for every p in s
until every Δ_p is empty
```

Bounds: **A-6** the number of rounds MUST be bounded by `1 + Σ_p |domain(p)|` (finite relations, monotone);
`gen_dl`'s emitted loop carries that bound as its `while` guard and traps **trap 125** (`trap 125: datalog
fixpoint bound exceeded`) if reached — a loud impossibility, never a hang (LAW 1: a wait is bounded and says
what it waited for). Check: `dl_bound.bp` — a rule set with a forged non-monotone builtin reaches the bound
(`EXPECT exit:125`).

**A-7** Retraction. Facts are never retracted in place (the log is append-only): a stock DEcrease is a new
`stock(S, Q')` fact replacing `stock(S, Q)` by key `S`, so `Δ_stock` carries the new row and the rule for
`unavailable` is re-evaluated for the affected column. For IDB predicates whose truth can turn false
(`unavailable(D)` when stock rises), the delta step re-evaluates the AFFECTED heads (the rows whose body
touched the dirty column) from F and REPLACES them — the "column fan-out" evaluation R §12.3 MEASURED at 82 ns
per stock event (2 dishes on average) vs 3,316 ns for the naive re-derive (40x at 1x, 2,260x at 100x).
Check: `dagfull datalog` arm — after a random event sequence with increases and decreases, the incremental
relation equals the relation re-derived from scratch, byte for byte.

**A-8** Cost per event on the 165-dish fixture MUST stay ≤ 1 µs (MEASURED basis 82 ns native, 89 ns wasm,
R §12.3 row "Datalog delta"; the acceptance is 10x the measurement to leave room for the bebop form, whose
kernels run 1.8-3.4x Rust on honest twins, `bebop-lang/ROADMAP.md` TG-DONE row 1). Check: `bebop-lang/docs/PERF.md` row
`dl_event_ns` from `+bebop-lang/bench/vs_rust/dl_bench.sh` (median of 5 runs × 10^5 events).

### A.6 The five first rules from dowiz — exact relations

Units: grams / ml / unit are the supply's base unit (`crates/dowiz-hub/src/stock.rs:1380` `BomLine.qty`, "how
much ONE portion uses, in the supply's base unit"; `Qty = i64`, `stock.rs:27`). Ids are interned `K64`s.

| Rule | Relations (columns, types) | Body | Δ trigger | Oracle |
|---|---|---|---|---|
| `unavailable(D)` | `recipe(D, S, C:i64 base units per portion)` CSR over dishes; `stock(S, Q:i64)`; `qmin(D, M:i64)` default 1 | `recipe(D,S,C) ∧ stock(S,Q) ∧ mul(C, M, CM) ∧ qmin(D,M) ∧ lt(Q, CM)` | a `StockEvent` on S (`Received/Reserved/Consumed/Released/Wasted/Stocktake/Served`, `stock.rs:86`) → column S | `crates/dowiz-hub/src/stock.rs` availability: the R §12.3 "CSR scalar ref" kernel, portions = min ⌊Q/C⌋ |
| `allergen(D, A)` | `recipe(D,S,_)`; `supply_allergen(S, A)` (A ∈ the fourteen declarable allergens, `crates/dowiz-hub/src/allergens.rs:78`); `produced(S2, S)` (S2 made from S) | `recipe(D,S,_) ∧ carries(S,A)`; `carries(S,A) :- supply_allergen(S,A)`; `carries(S2,A) :- produced(S2,S) ∧ carries(S,A)` (recursion inside one stratum) | a recipe, supply or `produced` edit | `allergens.rs` list per product (today's declared list) — the rule DERIVES; a derived allergen absent from the declared list is a finding, not a fix (the gate prints the diff) |
| `fsm_ok(O, T)` | `state(O, F)`; `allowed(F, T)` = the 14 edges of `FSM_ADJ` (`crates/dowiz-core/src/order_machine.rs:216-222`, 12 states in `OrderStatus` order `:14-30`) | `state(O,F) ∧ allowed(F,T)` | an order event on O | `bebop-lang/bench/oracles/ordfsm.py` (PRODUCTION Rust `order_machine.rs`) |
| `courier_may(C, O)` | `assigned(C, O)`; `ended(O)` = `state(O, F) ∧ terminal(F)` with `terminal` = {Delivered, Rejected, Cancelled, CompensatedRefund} | `assigned(C,O) ∧ ¬ended(O)` — `ended` is stratum 0, `courier_may` stratum 1 | an order or assignment event | the same `ordfsm.py` `is_terminal` |
| `personal(N)` | `holds_person(N0)` (the registry rows with a subject, `workers/api/src/privacy/registry.rs`); `edge(N0, N)` = the declared-edge table (RT §1.1) | `personal(N) :- holds_person(N)`; `personal(N) :- edge(N0,N) ∧ personal(N0)` (transitive closure) | a schema change (a new node kind or edge) | `tools/gates/personal-data.sh`'s row check — the rule is R §10 item 3 as data, and the gate's grep becomes `personal(N) ∧ ¬registered(N)` = ∅ |

**A-9** Each rule set MUST ship with (i) a `std_golden` gate line whose golden comes from its oracle (never
from the program: `arch_check` `gate-oracle`, TG-DONE row 3), (ii) a negative twin, (iii) a `dagfull datalog`
fixture. Check: `bebop-lang/bench/vs_rust/std_golden.sh` lines `gate dl_unavail`, `dl_allergen`, `dl_fsm`, `dl_courier`,
`dl_personal` + `_neg` twins; `bebop-lang/bench/oracles/dl_*.py`.

### A.7 The gate: `dagfull datalog` (RT §8.2 row)

For each rule set and fixture: seed random EDB events (`SEED` printed), evaluate incrementally through the
dirty set after every event, then re-derive from scratch with an empty memo; the two IDB blocks MUST be
byte-identical (`cmp`) and their `K256` equal. Prints `dagfull datalog <equal>/<rulesets> events <n>`.
Mutation proof (`dagfull.prove.sh` step 4): drop the replacement of affected heads (A-7) → after a stock
increase the incremental `unavailable` keeps a stale row → red, naming the dish.

### A.8 Lean: `+bebop-lang/formal/Bebop/Datalog.lean` (new)

```lean
/-- A monotone operator on finite sets of facts over a finite domain reaches the same least fixpoint whether
    iterated naively or semi-naively (only the last round's delta feeds the next). -/
theorem seminaive_eq_naive (T : Finset α → Finset α) (mono : Monotone T) (D : Finset α) (hD : ∀ s, T s ⊆ D) :
  lfp_naive T = lfp_seminaive T
/-- A stratification is well-defined when the predicate graph restricted to negative edges has no edge inside
    a strongly connected component; then the stratum-by-stratum evaluation is the unique perfect model. -/
theorem stratified_well_defined (G : PredGraph) (h : NoNegativeEdgeInSCC G) : ∃! M, PerfectModel G M
```

**A-10** `lake build` rc = 0, `sorryAx` = 0 for `Datalog.lean` (`#print axioms` quoted in the header as
`bebop-lang/formal/Bebop/Theorems.lean` does; `bv_decide`-generated axioms MUST be named if any appear). Check: the
`lean` step of `+bebop-lang/bench/vs_rust/dagfull.sh`. Lean is the cross-check oracle, never load-bearing (ROADMAP
thesis): the load-bearing check is A.7.

---

## Part B — the universal block codec

### B.1 One block, three roles (R §13.1)

A **block** is a byte string that is at once (i) the CAS unit (named by `K256` = sha256 of its bytes, RT §2.2),
(ii) the wire format (a Worker `Response` body, a Box sync message, a browser fetch) and (iii) the in-memory
format (a reader views a field with one bounds check and one `from_le_bytes`: MEASURED 2 ns per view,
independent of size, R §13.2). The block replaces JSON on the hot paths (MEASURED: JSON parse of a 165-dish
catalogue 6.33 ms vs block decode 2.4 µs = 2,600x; `bom_of` × 5 lines 41 µs vs 5 × 82 ns; block 7,464 B vs
385,456 B JSON = 52x smaller; sha256 30 µs vs 1.54 ms, R §13.2). JSON stays at the browser edge.

### B.2 Header layout (byte-exact, little-endian)

```
offset  size  field
0       4     magic      "DWB1" (0x44 0x57 0x42 0x31)
4       2     version    u16 = 1
6       2     ncols      u16 (≤ 4096)
8       4     n          u32  rows
12      4     nnz        u32  non-zeros (0 unless a CSR column is present)
16      8     schema     u64  = K64 of the schema string (§B.4) -- a reader refuses a block whose schema it does not know
24      16*k  coldesc[k] per column: type u8, flags u8, unit u8, reserved u8, len u32, offset u32, reserved u32
24+16k  ..    column data, each column 8-byte aligned (offset % 8 == 0), zero padding between
end-4   4     crc32      u32 over bytes [0, end-4)  (zlib crc32, = `crc32x`/`bebop_store::crc32`)
```

`type`: 1 i64, 2 i32, 3 u8-bytes, 4 offsets-u32 (n+1 entries; pairs with the previous bytes column), 5 CSR
row_ptr u32 (n+1), 6 CSR col u32 (nnz), 7 CSR val i64 (nnz), 8 validity bitmask (ceil(n/64) u64), 9 u16 mask
table (n entries). `flags` bit 0: a validity column follows this column. `unit`: 0 none, 1 minor units
(money), 2 grams, 3 ml, 4 unit, 5 ppm, 6 ms since epoch, 7 generation. The header size is derived in every
reader as `24 + 16 * ncols` (never a literal). Total ≤ 96 KiB per block so one block is one Durable Object
chunk (`workers/api/src/hubdo.rs:44` `CHUNK = 96 * 1024`; a bigger relation is split by rows into a chain of
blocks whose `K256`s form a manifest block).

**B-1** A reader MUST refuse (never pad, never guess) a block whose magic, version, crc, schema, column
offsets/lengths or alignment are wrong, and MUST name which (`crates/bebop-wasm/src/lib.rs` "EVERYTHING
REFUSES, NOTHING PANICS" shape: a typed refusal, one status code each). Check: `block_refuse_*` tests in Rust
(`+crates/dowiz-hub/src/block/tests.rs`), bebop (`+bebop-lang/bench/vs_rust/std_tests/block_neg.bp`) and Python
(`crates/bebop-wasm/oracle.py` extended) — one flipped byte in each region → the named refusal.

### B.3 Per-type encodings and the ops they admit

| Logical type | Columns | Encoding | Ops that stay in the column form |
|---|---|---|---|
| number / money / weight | 1 × i64 (or i32 with `unit` grams when the bound is provable, §B.5) | LE two's complement | add, sub, cmp, min, max, checked mul via `smulh`/`umulh` high words |
| text | u8 bytes + u32 offsets (n+1; string i = bytes[off[i]..off[i+1]]) | UTF-8 bytes, no NUL | validate, search, compare, sort keys — the simdutf shape on the Rust side; scalar on bebop (no v128 emitter, R §12.2) |
| log / ragged records | offsets + values (the evlog's `payload_len ‖ payload` frames are this already, `crates/bebop-store/src/evlog.rs:24-33`) | as text | scan, filter |
| relation / DAG / FSM | CSR (`row_ptr`, `col`, `val`) + a u16 mask table for a transition table (`allowed[from] >> to & 1`, = `FSM_ADJ`) | csr.bp layout with integer `val` | gather; a branch becomes `mask ← cmp; select` ONLY when the column is long (§B.7) |
| optional field | values + validity bitmask | bit i set = present | and/or of masks |
| JSON at the edge | a parser/serialiser to and from the block; Rust `serde_json` today, `minijson`'s shape on the bebop side (`crates/dowiz-hub/src/minijson.rs`) | — | boundary only |
| hash / CAS id | `K256` over the block bytes | sha256 | — |

### B.4 Schemas of the first blocks (the catalogue projection, R §6 row 5)

Schema strings are ASCII, `name:v1(col:type:unit,...)`; the `schema` header field is their `K64`.

| Block | Schema string | Source JSON today | Consumer |
|---|---|---|---|
| `menu_prices` | `menu_prices:v1(dish:i64:0,price:i64:1,tax_ppm:i64:5,mods_ptr:u32rp:0,mods_col:u32:0,mods_val:i64:1)` | `"price"` (minor units, `crates/dowiz-hub/src/stock.rs:1472` shape), the modifiers array (`crates/dowiz-hub/src/modifiers.rs`), the tax rate as `RatePpm` (`crates/dowiz-core/src/domain.rs:740`; Phase B B1 of `docs/design/ROADMAP-2026-09-22.md`) | `workers/api/src/hubdo/menu.rs` (`/fold/menu`, `/fold/products`), placement pricing |
| `bom` | `bom:v1(dish_ptr:u32rp:0,supply:u32:0,qty:i64:<unit of the supply>)` | `"bom":[{"supply","qty"}]` (`stock.rs:1395`) | `bom_of` (`stock.rs:1393`) → reads the block row instead of parsing JSON per line |
| `stock_levels` | `stock_levels:v1(supply:i64:0,qty:i64:<unit>,gen:i64:7)` | the `StockLedger` fold (`stock.rs`) | `unavailable(D)` (A.6) |
| `names` | `names:v1(id:i64:0,bytes:u8:0,off:u32off:0)` | the intern table | every block with interned ids |

**B-2** The schema string of every shipped block MUST appear in ONE table, `+crates/dowiz-hub/src/block/schema.rs`
(Rust), mirrored in `+bebop-lang/selfhost/std/block.bp` and `crates/bebop-wasm/oracle.py`; a test in each
asserts the three tables' `K64`s are equal (the vocabulary-gate shape: `tools/gates/vocab.sh` keeps hand
copies from drifting). Check: `block::tests::schema_table_agrees`, `gate block_schema` (bebop), `oracle.py`
`--schemas`.

### B.5 Money and overflow (rule 6 of `/root/dowiz/.claude/lanes/DOWIZ-COMMON-RULES.md` (box-local, untracked); `kernel/src/money.rs`)

- **B-3** Money is i64 minor units; rates are `RatePpm` (i64 parts per million); every product or sum is
  CHECKED (overflow → refusal, never wrap): `price × ppm` needs a 64-bit intermediate (21,000,000 × 1.2e6
  overflows i32, R §12.2) and is computed as `umulh/smulh` + low word with the high word checked against the
  bound. Division by 1,000,000 with the stated rounding (half-up, truncating div: `bebop-lang/selfhost/std/money.bp`
  op 5/6, twin of `crates/dowiz-core/src/money.rs`). No f64 anywhere near a block (`tools/gates/float-money.sh`).
  Check: `float-money` gate = 0; `block::tests::price_overflow_refused`; the bebop twin `block_money_neg`.
- **B-4** Grams/ml MAY be i32 columns only when the bound is provable from the schema (`qty ≤ 2^27` per
  portion and portions ≤ 10 ⇒ product < 2^31, R §12.2); otherwise i64. The scalar reference kernel is the
  ORACLE for any vectorised form: both run in the test and their outputs MUST be byte-identical (R §12.3
  asserted this across native / wasm / simd128). Check: `block::tests::mask_vec_equals_scalar`.
- **B-5** Integer DIVISION MUST NOT be written in a vector-lane form (there is none in simd128 or NEON, R
  §12.2): availability is asked as `C·M ≤ Q` (a mask); "how many portions" stays scalar. Check: code review +
  the `mask_vec_equals_scalar` test's shape.

### B.6 The round-trip gate (per type, per block)

**B-6** For every type in §B.3 and every schema in §B.4, `decode(encode(x)) == x` (structural equality) AND
`encode(decode(b)) == b` (byte equality) over: the fixtures (`+crates/dowiz-hub/fixtures/blocks/*.dwb`, new:
the 165-dish sushi catalogue as `menu_prices`, `bom`, `names`; a 120-supply `stock_levels`), 1,000 random
blocks per type from a seeded generator (the seed printed), and the empty block (`n = 0`). Four readers agree
on `n`, `nnz` and `K256`: Rust native, Rust-in-wasm32 (`crates/bebop-wasm`), bebop (`block.bp`), Python
(`oracle.py`). Check: `crates/bebop-wasm/gate.sh` gains `+crates/dowiz-hub/fixtures/blocks/`; `bebop-lang/bench/vs_rust/std_golden.sh`
`gate block_rt`; the CI job `.github/workflows/ci.yml` runs the Rust and Python halves (the bebop half is
box-only, said so, never counted as agreement).
**B-7** Money columns round-trip through the JSON edge with `Intl.NumberFormat`-free rendering (memory
`kit-had-a-fourth-money-copy`: grep every new surface for `/ 100` and `NumberFormat`). Check: the existing
money-copy grep in the design gate + `float-money`.

### B.7 How branch-heavy logic is tensorised, and when it is not (R §13.1)

Both arms computed over the whole column and a mask selects (`bad |= (coef != 0) & (coef·q > stock)`). This
costs both arms always, so it is a WIN only when arms are cheap and the column is long (a 165 × 120 CSR, 352
nnz) and a LOSS for the order FSM's decide (one order, 14 edges: the u16 bitmask lookup `allowed[from] >> to
& 1` that `FSM_ADJ` already is beats any column form; 12 × 12 = 144 cells is not a workload). **B-8** A lane
MUST NOT tensorise a per-order decision; it MAY tensorise a per-catalogue or per-log pass; every such pass
ships with its scalar oracle (B-4). Check: code review against this line + `mask_vec_equals_scalar`.

### B.8 The dense-SIMD form — a gated row, not a rule (operator D-4 vs R §12.3)

The operator decided prices/stock as DENSE tensors recomputed with wasm SIMD (v128 i32x4/i64x2) in the Worker
and the hub object, and the same kernels natively on the Box. R §12.3 MEASURED, on the 165/120/70 fixture:

| Kernel | sparse scalar (CSR) | dense simd128 | dense native NEON | verdict |
|---|---|---|---|---|
| mask "q = 2 servable" | **1,089 ns** | 9,749 ns | 8,643 ns | dense 8x SLOWER; 580x at 100x (200 M cells) |
| availability portions | **3,355 ns** | 14,886 ns | 15,476 ns | division scalarises; dense 4.4x slower |
| price + modifiers + tax (i64 ÷ 1e6) | 2,219 ns | 2,068 ns | 2,221 ns | 0 % (no i64 mulhi / divide in v128) |
| all kernels, one recompute | **≈ 8.3 µs** | — | — | 0.08 % of the 10 ms Free-plan wall |

The block codec (§B.1) satisfies "tensor" in the Arrow sense — columns, CSR, masks, one layout everywhere —
and carries the measured 2,600x; the dense-SIMD sense has no measured gain to collect. **This spec therefore
specifies the CSR/columnar forms with the scalar oracle as normative, and writes the dense-SIMD form as
BLUEPRINT row DG25**, whose acceptance number is the MEASURED one: it lands only if a probe on the real
fixture shows the dense simd128 mask ≤ the sparse scalar mask (1,089 ns at 1x AND ≤ 137,533 ns at 100x, the
CSR row's 100x number), asserted byte-identical to the scalar oracle. **OPERATOR DECISION NEEDED** (§D C-1):
build DG25 as a measurement row that can only land on that number, or withdraw the dense form. A bebop v128
emitter (T64/T94-class) is DG26 and is not in the jump in either case (R §8.5).

---

## C. Traceability

| MUST | Check |
|---|---|
| A-1 | `bebop-lang/bench/vs_rust/census.txt`, `bebop-lang/tools/builtin_surface.py` unchanged |
| A-2 | `gate dl_set_semantics` |
| A-3 | `dl_neg_unsafe.bp` (exit 125) + twin |
| A-4 | `dl_neg_cycle.bp` (exit 125); `dl_strata.bp` vs `+bebop-lang/bench/oracles/dl_strata.py` |
| A-5, A-7 | `dagfull datalog` arm; `dagfull.prove.sh` step 4 |
| A-6 | `dl_bound.bp` (trap 125) |
| A-8 | `bebop-lang/docs/PERF.md` `dl_event_ns` ≤ 1,000 |
| A-9 | `gate dl_unavail/dl_allergen/dl_fsm/dl_courier/dl_personal` + `_neg`; `bebop-lang/bench/oracles/dl_*.py` |
| A-10 | `lake build` rc 0, `sorryAx` 0 for `Datalog.lean`; `lean_conformance` unchanged |
| B-1 | `block_refuse_*` (Rust, bebop, Python) |
| B-2 | `schema_table_agrees`, `gate block_schema`, `oracle.py --schemas` |
| B-3 | `float-money` = 0; `price_overflow_refused`; `block_money_neg` |
| B-4, B-5, B-8 | `mask_vec_equals_scalar`; code review |
| B-6 | `crates/bebop-wasm/gate.sh` blocks 4/4; `gate block_rt`; CI Rust+Python halves |
| B-7 | design gate money grep; `float-money` |

## D. Where the research and the operator disagree (this document's share; RT §12 has the rest)

| # | Operator | Research (MEASURED) | Status |
|---|---|---|---|
| C-1 | dense integer tensors + wasm SIMD for prices/stock; everything computes as tensors | R §12.3: dense mask 8x slower than sparse scalar at 1x, 580x at 100x; prices/portions 0 % from SIMD; kernels 8.3 µs total. R §13.2: the codec is where the time is (2,600x) | §B.1-B.7 normative (Arrow-sense tensors + scalar oracle); §B.8 dense-SIMD as gated row DG25. **OPERATOR DECISION NEEDED** |
| C-2 | the same kernels natively on the Box via bebop codegen for v128 | R §12.2: no v128 emitter; T64/T94-class; nothing to collect | DG26 "not in the jump". Decision follows C-1 |
| C-8 | Datalog "runs as vectorised joins over these columns" (D-4 text) | R §12.3: the semi-naive delta step is 82 ns because it is O(fan-out) — the win is the DIRTY SET, not vectorisation; a vectorised join over the whole relation IS the naive re-derive (3,316 ns) | §A.5 normative: joins over the delta; a whole-relation vectorised join is allowed only for a cold start |
| C-9 | JSON at the edges as a SIMD parser/serialiser (simdjson-style) | R §13.2: with the block as the wire format the Worker parses no JSON on the hot path; the remaining JSON is the browser render (398 µs per full menu today), which R §10 item 2 makes a client cost | not specified here; a `simd-json` crate in the Worker is a DECART question for the stack blueprint (`docs/design/BLUEPRINT-STACK-AND-DEPENDENCIES-2026-09-22.md`), not a runtime rule |
