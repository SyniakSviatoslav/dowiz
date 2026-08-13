# KTG-2: нативна 2-бітна тензорно-графова dataflow-архітектура

Статус: research proposal / hardware-software co-design blueprint  
Дата: 2026-08-13  
Ціль: замінити CPU-подібну модель виконання kernel на власну детерміновану тензорно-графову машину, де дані буквально течуть ребрами графа, а базове значення там, де це семантично можливо, займає 2 біти.

## 0. Короткий висновок

Так, така архітектура доцільна, але її не слід описувати як «2-bit ARM replacement» або як звичайний процесор із 2-бітними регістрами. Правильна форма — просторово-потокова машина з двома взаємодоповнювальними fabric:

1. статично розкладена systolic tensor fabric для регулярних матричних операцій;
2. невелика tagged-dataflow graph fabric для нерегулярних залежностей, умов, циклів і Unknown;
3. спільна локальна packed-trit пам’ять і credit-based transport;
4. конфігураційний graph ABI замість класичної load/store ISA;
5. Rust reference machine як канонічна семантика, незалежна від x86_64/aarch64.

Назва proposal: **KTG-2 — Kernel Tensor Graph, 2-bit**.

Найважливіше обмеження: «2-bit everywhere» застосовується до payload-елементів, масок, станів, ваг і локальних предикатів. Адреси, розміри, лічильники, accumulator-и, capability IDs, epoch-и та transport tags не можна безпечно стискати до 2 бітів. Архітектура має бути **2-bit dominant**, а не штучно 2-бітною в кожному полі.

## 1. Що вже існує у kernel

Поточний kernel має майже всі потрібні математичні органи, але вони ще не утворюють одну execution architecture.

### 1.1 Тристанова логіка

- `src/lib.rs:10-62` містить `TriState::{True, False, Unknown}` і strong-Kleene-подібні `and/or/not`.
- `src/trinary.rs:29-126` дублює тип як `Tri`, додає Kleene та Łukasiewicz implication.
- `Tri` має `#[repr(u8)]`, тобто фізично витрачає щонайменше байт на значення, а не 2 біти.
- `TriMatrix` у `src/trinary.rs:237-241` зберігає `Vec<Tri>`, тому також не packed.
- Поточний `TriMatrix::mul` є логічною композицією `AND + OR`, а не універсальним signed-ternary matrix multiply.

Наслідок: треба уніфікувати `TriState` і `Tri`, відділити фізичний 2-бітний код від семантичного типу та не змішувати логічний Unknown із числовим Zero.

### 1.2 Тензори

- `src/tensor.rs` уже є zero-BLAS органом, але `Tensor1/Tensor2` побудовані на `f64`.
- `src/inference/fixed.rs` має детерміновану integer-only MAC/rounding семантику.
- `src/inference/workspace.rs` має статичні offsets, fixed capacity і zero-mid-inference-allocation, але елемент workspace зараз `i8`.
- `src/csr.rs`, `src/cgraph.rs`, `src/hypergraph.rs`, `src/spectral_graph.rs` дають sparse/causal/hypergraph структури.

Наслідок: нова машина не починається з нуля. Вона має зробити ці органи backend-ами одного typed tensor-graph IR.

### 1.3 Залежність від host ISA

Canonical semantics переважно portable Rust, але швидкі шляхи в `simd.rs`, `householder.rs`, `inference/simd_i8.rs`, `fdr/pmu.rs` і частині `living_knowledge.rs` прив’язані до x86_64/AVX/FMA/raw syscall. На aarch64 працює scalar fallback. Це не означає, що kernel семантично є ARM64, але execution model досі CPU-centric: функція читає масиви, виконує цикли, записує масиви.

KTG-2 має змінити саме execution model. На першому етапі Rust-емулятор усе одно запускається на host CPU, але host ISA стає лише bootstrap substrate, а не архітектурним контрактом.

## 2. Наукова база

### 2.1 Ternary payload

BitNet b1.58 показує практичну цінність ternary weights `{-1, 0, +1}` та матричного виконання переважно через integer addition замість повноцінного floating multiply.[1] Bitnet.cpp додатково показує, що фізичне 2-бітне вирівняне представлення є практичним компромісом для ternary kernels, хоча інформаційна місткість трита дорівнює приблизно 1.58 біта.[2]

Це не доводить, що весь універсальний kernel повинен бути ternary. Це доводить, що weights, predicates, masks, sparse signs і логічні стани мають сильний case для 2-bit physical encoding.

### 2.2 Systolic execution

TPU-style systolic array передає проміжні значення між сусідніми PE та повторно використовує дані без постійного round-trip до register file/memory.[5] Це добре відповідає вимозі «data flow є буквальним».

Один статичний dataflow не оптимальний для кожного шару. Flex-TPU досліджує runtime-перемикання input-, output- і weight-stationary режимів.[3] Тому KTG-2 не повинен зашивати один режим у всі tensor nodes.

### 2.3 Hybrid graph execution

REVEL показує корисну композицію: регулярні inner regions виконуються на ефективній systolic fabric, а нерегулярні/індуктивні залежності — на меншій tagged-dataflow fabric; streams і predication стають first-class.[4]

Це безпосередньо підходить kernel: dense/packed tensor ops є регулярними, тоді як causal graph, hypergraph traversal, retries, Unknown propagation і control loops — нерегулярні.

## 3. Канонічний 2-бітний cell

### 3.1 Фізичний код

Вибір коду повинен робити Kleene AND/OR дешевими:

| Bits | LogicCell | SignedTrit | Стан transport payload |
|---|---|---|---|
| `00` | False | -1 | valid negative/false |
| `01` | Unknown | 0 | valid unknown/zero |
| `10` | True | +1 | valid positive/true |
| `11` | Poison | Poison | invalid/corrupt/reserved |

Перевага ordinal encoding `False < Unknown < True`:

- `AND_K3(a,b) = min(a,b)` для valid кодів;
- `OR_K3(a,b) = max(a,b)` для valid кодів;
- `NOT_K3`: `00 ↔ 10`, `01 → 01`;
- `11` завжди sticky Poison.

### 3.2 Чому четвертий код — Poison, а не ще один бізнес-стан

`11` потрібен як structural integrity state:

- неініціалізована або пошкоджена packed cell;
- type mismatch під час graph lowering;
- arithmetic overflow, якщо node contract вимагає exactness;
- shape/epoch violation;
- недозволене перетворення Unknown у bool.

Poison не дорівнює Unknown:

- Unknown — коректна інформація «ще не відомо»;
- Poison — порушення контракту або пошкодження.

Poison не слід використовувати як «немає token». Transport validity має бути окремим valid/credit protocol; інакше не можна відрізнити відсутній token від token-а з помилкою.

### 3.3 Typed semantics поверх однакового коду

Однакові 2 біти мають різні дозволені операції залежно від type tag у graph IR:

- `LogicCell`: False / Unknown / True;
- `SignedTrit`: -1 / 0 / +1;
- `MaskCell`: Drop / Maybe / Pass;
- `HealthCell`: Failed / Degraded / Healthy;
- `PermissionCell`: Deny / Pending / Allow.

Це важливо: `01` у `LogicCell` означає Unknown, а у `SignedTrit` — точний нуль. Physical encoding спільний, semantic algebra — типізована.

## 4. Truth tables

### 4.1 Strong Kleene AND

| AND | F | U | T | P |
|---|---:|---:|---:|---:|
| F | F | F | F | P |
| U | F | U | U | P |
| T | F | U | T | P |
| P | P | P | P | P |

### 4.2 Strong Kleene OR

| OR | F | U | T | P |
|---|---:|---:|---:|---:|
| F | F | U | T | P |
| U | U | U | T | P |
| T | T | T | T | P |
| P | P | P | P | P |

### 4.3 NOT

| x | NOT x |
|---|---|
| F | T |
| U | U |
| T | F |
| P | P |

### 4.4 Signed ternary multiply

| × | -1 | 0 | +1 | P |
|---|---:|---:|---:|---:|
| -1 | +1 | 0 | -1 | P |
| 0 | 0 | 0 | 0 | P |
| +1 | -1 | 0 | +1 | P |
| P | P | P | P | P |

Physical PE реалізує це без general multiplier: `+1 → add`, `0 → skip`, `-1 → subtract`.

## 5. Packed memory model

### 5.1 Layout

- 4 cells на byte.
- 32 cells на `u64` word.
- canonical order: cell 0 у bits `[1:0]`, cell 1 у `[3:2]`, ...
- row-major tiles; tensor shape та strides зберігаються в descriptor, не в кожній cell.
- tail cells завжди заповнюються Poison, щоб out-of-shape read не виглядав валідним.

### 5.2 Tensor descriptor

Payload не містить pointers. Descriptor використовує capability-like IDs:

```text
TensorDesc {
  tensor_id: u32,
  element_kind: LogicCell | SignedTrit | MaskCell | I8 | I16 | I32,
  rank: u8,
  shape: [u32; MAX_RANK],
  tile_shape: [u16; MAX_RANK],
  region_id: u16,
  base_cell: u64,
  epoch: u32,
  integrity: u32,
}
```

Ці metadata не є 2-bit і не повинні ними бути. Вони амортизуються на тисячі або мільйони cells.

### 5.3 Memory hierarchy

1. **PE registers** — кілька packed words + wide accumulator.
2. **Tile scratchpad** — banked SRAM model, explicit DMA/stream descriptors.
3. **Graph-local tensor store** — content/capability addressed regions.
4. **Host/shared memory bridge** — лише boundary adapter, не execution contract.

Немає general-purpose load/store з довільною адресою всередині fabric. Nodes працюють із `TensorId`, tiles та streams.

## 6. Execution fabric

### 6.1 Tensor Systolic Fabric (TSF)

Регулярна решітка PE, наприклад параметризована `8×8`, `16×16`, `32×32`. Розмір не є частиною семантики — graph compiler tiles операцію під доступну fabric.

Кожен PE має:

- два packed input latches;
- local signed-trit weight latch;
- integer accumulator (`i16/i32/i64`, визначений proof contract);
- truth LUT для LogicCell;
- add/sub/skip datapath для SignedTrit;
- neighbor links N/E/S/W;
- Poison detector;
- predicate/mask input;
- saturating або checked reduction mode.

Підтримувані stationary modes:

- weight-stationary;
- input-stationary;
- output-stationary;
- sparse-event mode, де zero/False lanes не створюють compute event.

### 6.2 Graph Dataflow Fabric (GDF)

Менша мережа temporal PE для:

- branch/merge із Unknown;
- graph traversal;
- causal/hypergraph operators;
- feedback loops;
- retries and bounded convergence;
- reductions зі змінним fan-in;
- stream/epoch control.

Node firing rule:

```text
READY(node, epoch) = усі required input ports мають token цього epoch
FIRE(node, epoch)  = READY && output credits sufficient && contract valid
```

Token:

```text
Token {
  tensor_or_scalar_ref,
  producer_node,
  epoch,
  logical_time,
  type_id,
  poison_summary,
}
```

Payload може бути inline packed 2-bit для малих значень або reference на immutable tile.

### 6.3 Stream Transfer Fabric (STF)

STF буквально переносить tiles між TSF, GDF і scratchpad:

- credit-based backpressure;
- deterministic route priority;
- explicit multicast;
- producer/consumer rates;
- inductive stream descriptors;
- no implicit cache coherence.

## 7. Graph ABI замість класичної ISA

KTG-2 не має ставити scalar instruction stream у центр. Primary executable — immutable `GraphImage`:

```text
GraphImage {
  version,
  type_table,
  tensor_table,
  node_table,
  edge_table,
  stream_table,
  placement_hints,
  proof_contracts,
  expected_hash,
}
```

### 7.1 Мінімальні node classes

- `TRI_AND`, `TRI_OR`, `TRI_NOT`, `TRI_MAJORITY`;
- `TRIT_DOT`, `TRIT_GEMM`, `TRIT_CONV`;
- `PACK`, `UNPACK`, `CAST_CHECKED`;
- `REDUCE_MIN/MAX/SUM/MAJORITY`;
- `CSR_GATHER`, `CSR_SCATTER`, `SEGMENT_REDUCE`;
- `HYPEREDGE_EXPAND`, `CAUSAL_FILTER`;
- `MERGE_EPOCH`, `DELAY`, `FIXPOINT_BOUNDED`;
- `ASSERT_SHAPE`, `ASSERT_KNOWN`, `ASSERT_NO_POISON`;
- `IO_SOURCE`, `IO_SINK` тільки на boundary.

### 7.2 Scalar escape hatch

Повністю прибирати scalar control недоцільно. Але він не повинен бути ARM/RISC clone. Потрібен невеликий deterministic control engine для:

- graph load/verify;
- stream setup;
- bounded loops;
- exceptions;
- telemetry snapshot;
- device boot.

Його bytecode не оперує arbitrary pointers і не виконує tensor arithmetic. Це control-plane VM, а не CPU data plane.

## 8. Compiler stack

### Layer A — Kernel Semantic Graph (KSG)

Високорівневі typed nodes: causal, retrieval, inference, telemetry, breaker, policy.

### Layer B — Typed Trit Tensor IR (T3IR)

Явні:

- shapes;
- element algebra;
- Unknown/Poison propagation;
- reduction laws;
- exactness/overflow contracts;
- sparse structure;
- production/consumption rates.

### Layer C — Dataflow Schedule IR (DFIR)

- node firing dependencies;
- epochs;
- static versus tagged regions;
- stream descriptors;
- buffer/credit bounds;
- deadlock proof inputs.

### Layer D — Fabric Placement IR (FPIR)

- TSF/GDF placement;
- tile size;
- stationary mode;
- routes;
- scratchpad allocation;
- schedule hash.

### Layer E — GraphImage

Canonical deterministic bytes, hashable and replayable.

## 9. Determinism і safety

### 9.1 Deterministic execution

Для однакового `GraphImage`, input tensor bytes і fabric profile результат повинен бути byte-identical:

- fixed arbitration order;
- epoch barriers at declared cut points;
- integer reductions із canonical tree/order;
- no unordered floating reductions;
- bounded feedback;
- explicit clock/logical-time tokens;
- replay log only at nondeterministic boundaries.

### 9.2 Unknown policy

Unknown не має автоматично перетворюватись на False. Усі resolution nodes явні:

- `ASSERT_KNOWN`: Poison/error, якщо U;
- `RESOLVE_DEFAULT(F|T)`: boundary policy;
- `WAIT_UNTIL_KNOWN(bound)`: feedback із deadline;
- `VOTE_MAJORITY`: tie → Unknown;
- `FAIL_CLOSED`: U → F лише в явно security-typed node.

### 9.3 Poison policy

- sticky across normal ops;
- може бути очищений лише `RECOVER_POISON` node з audit reason;
- tensor descriptor містить poison summary bitmap/count;
- sink за замовчуванням відмовляється commit-ити Poison.

### 9.4 Deadlock and backpressure

Compiler має довести або runtime має перевірити:

- bounded FIFO occupancy;
- credit conservation;
- feedback edge має delay/initial token;
- multi-input joins не змішують epochs;
- finite graph region або declared persistent service graph.

## 10. Що не варто робити

1. Не замінювати всі `f64` механічно на `Tri`: spectral decomposition, timestamps, money та cryptography мають інші algebra/range requirements.
2. Не використовувати `Unknown` як numeric zero без type distinction.
3. Не робити `11` fourth business truth value; це знищить універсальний corruption sentinel.
4. Не використовувати host SIMD intrinsics як canonical semantics.
5. Не будувати одразу ASIC. Спочатку executable Rust spec, differential tests, cycle model, FPGA prototype.
6. Не обіцяти «ARM більше не потрібен» до появи boot, memory, interrupt, IO та toolchain stack. На ранніх фазах ARM/x86 лише хостять reference machine.
7. Не переносити tagged-dataflow на кожен PE: це дорого. Tagged fabric має бути малою, systolic fabric — основною.[4]

## 11. Migration plan для dowiz kernel

### Phase 0 — Formal semantics

Новий орган `src/ktg2/cell.rs`:

- `#[repr(transparent)] PackedCell(u8)` лише для single-cell API;
- `LogicCell`, `SignedTrit`, `Poison` conversions;
- exhaustive truth tables для всіх 16 input pairs;
- compile-time encoding assertions.

Уніфікувати `TriState` і `trinary::Tri` через один canonical type або compatibility alias.

Acceptance:

- exhaustive cell laws;
- `AND=min`, `OR=max` для valid logic cells;
- Poison sticky;
- жодного implicit `Unknown -> bool`.

### Phase 1 — Packed tensor substrate

Нові органи:

- `packed2.rs` — pack/unpack/get/set, slices, iterators;
- `trit_tensor.rs` — shape/stride/tile descriptors;
- `trit_kernel.rs` — logic and signed ternary kernels;
- `trit_csr.rs` — packed sparse signs/masks.

Замінити `TriMatrix.data: Vec<Tri>` на `Packed2Vec` через compatibility facade.

Acceptance:

- 4× менше payload memory проти `repr(u8)`;
- differential parity зі старим `TriMatrix` на exhaustive small matrices;
- no external dependencies;
- scalar Rust oracle first.

### Phase 2 — Graph IR and interpreter

Нові органи:

- `graph_ir.rs`;
- `graph_verify.rs`;
- `dataflow_exec.rs`;
- `stream.rs`;
- `epoch.rs`.

Спочатку single-thread deterministic interpreter. Він є source of truth для майбутнього hardware.

Acceptance:

- topological/static graphs;
- bounded cyclic graphs;
- deterministic replay;
- deadlock detection;
- Poison/Unknown tests.

### Phase 3 — Hybrid software fabric

- TSF emulator із tile-by-tile systolic timing;
- GDF tagged executor;
- bounded FIFOs/credits;
- scheduler вибирає static or tagged region;
- adapters для `tensor`, `csr`, `hypergraph`, `cgraph`, `inference`.

Acceptance:

- graph results bit-identical reference interpreter;
- no architecture-specific intrinsics in canonical path;
- performance counters count cell moves, fires, stalls, poison, unknown.

### Phase 4 — Kernel migration

Порядок consumers:

1. policy/breaker/workflow gates → `LogicCell`;
2. inference masks and ternary weights → `SignedTritTensor`;
3. retrieval adjacency/sign masks → packed CSR;
4. hypergraph incidence → graph nodes;
5. telemetry health matrix → packed tensor;
6. only then selected dense tensor operators.

### Phase 5 — RTL/FPGA

- Rust executable spec freezes Graph ABI;
- cycle model establishes FIFO/PE sizing;
- generate or hand-author small RTL tile;
- FPGA co-simulation against Rust traces;
- exhaustive 2-bit PE verification;
- scale tile only after parity.

## 12. Proposed Rust module tree

```text
src/ktg2/
  mod.rs
  cell.rs             # 2-bit physical code + typed wrappers
  packed.rs           # 4 cells/byte, 32/u64
  tensor.rs           # descriptors, views, tiling
  algebra.rs          # Kleene, signed-trit, mask algebras
  graph.rs            # nodes, edges, GraphImage
  verify.rs           # shape/type/deadlock/credit checks
  stream.rs           # rates, epochs, backpressure
  interpreter.rs      # canonical semantic executor
  systolic.rs         # TSF software model
  tagged.rs           # GDF software model
  schedule.rs         # partition/place/route
  workspace.rs        # static region planner
  trace.rs            # deterministic event trace
  adapters/
    trinary.rs
    tensor.rs
    csr.rs
    cgraph.rs
    hypergraph.rs
    inference.rs
```

## 13. Evaluation matrix

### Correctness

- exhaustive truth-table tests;
- exhaustive all 256 packed-byte values;
- pack/unpack round-trip;
- differential old/new `TriMatrix`;
- graph interpreter versus TSF/GDF;
- Poison injection;
- Unknown convergence;
- epoch mismatch rejection;
- FIFO deadlock tests.

### Architecture independence

- identical golden GraphImage traces on x86_64 and aarch64 host;
- no `std::arch` in `ktg2` canonical modules;
- no host pointer in serialized GraphImage;
- deterministic endian-defined encoding.

### Performance

Measure, do not assume:

- payload bytes moved;
- packed/unpacked overhead;
- PE utilization;
- static/tagged region ratio;
- stall cycles by cause;
- sparse zero-skip rate;
- energy proxy = cell movement + accumulator activity;
- latency versus current scalar Rust oracle.

## 14. Architectural decision record

Recommended decision:

- **Adopt** a 2-bit-dominant typed trinary substrate.
- **Adopt** hybrid systolic + tagged dataflow execution.
- **Adopt** GraphImage as primary executable contract.
- **Keep** wide integer accumulators and metadata.
- **Keep** host CPU only as bootstrap/reference target during migration.
- **Reject** universal conversion of all data to 2-bit.
- **Reject** direct ASIC-first development.

This is a genuine new architecture, not an ARM64 extension: computation is declared as typed tensor/graph nodes, firing is operand-driven, memory is tensor/stream addressed, and regular operations execute spatially. ARM64/x86_64 remain temporary implementation hosts until the Rust reference machine, compiler, simulator, and RTL agree bit-for-bit.

## Sources

[1] https://arxiv.org/html/2402.17764 — BitNet b1.58: ternary weights and low-bit matrix computation.  
[2] https://arxiv.org/html/2502.11880v1 — Bitnet.cpp: aligned 2-bit ternary kernels and edge inference.  
[3] https://arxiv.org/html/2407.08700v1 — Flex-TPU: runtime-reconfigurable systolic dataflows.  
[4] https://polyarch.cs.ucla.edu/papers/hpca2020-revel.pdf — hybrid systolic/dataflow execution and inductive streams.  
[5] https://cloud.google.com/blog/products/ai-machine-learning/an-in-depth-look-at-googles-first-tensor-processing-unit-tpu — systolic array data movement and reuse.
