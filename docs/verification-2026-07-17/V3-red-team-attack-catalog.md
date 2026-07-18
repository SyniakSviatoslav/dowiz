# V3 — dowiz/DeliveryOS Kernel Red-Team Attack Catalog

> **Scope**: defensive security research on the operator's own delivery kernel. Read-only against
> product code (`/root/dowiz-verify-redteam/kernel/src/`, `/root/dowiz-verify-redteam/web/`),
> write-only into this isolated worktree (`research/dowiz-verify-redteam-2026-07-17`). Every entry is
> an *executable* adversarial test design: name · exact malicious-input construction · expected-safe
> behavior (+ which existing doctrine it should match) · priority. Methodology mirrors the sibling
> bebop2 pass (`/root/bebop2-verify-redteam/docs/verification-2026-07-17/V3-red-team-attack-catalog.md`,
> which found 11 HIGH — unauthenticated revocation + a dormant safety gate). This pass looks for the
> dowiz-side analogs: a **defeated fail-closed guard**, **browser-reachable money forgery**, and an
> **unbounded/uncapped allocation** despite the degrade-closed doctrine.
>
> **Priority key**
> - **HIGH** — code evidence that the *current* implementation actually fails or has a real,
>   reachable weakness (missing bound, reachable panic, fail-open branch, unenforced trust flag,
>   defeated guard).
> - **MEDIUM** — untested, plausibly exploitable, or documented-but-unenforced.
> - **LOW** — untested-but-probably-fine; write as a regression assertion to pin current safe behavior.
>
> **Doctrines referenced** (from the kernel's own headers/comments):
> - *degrade-closed* — every unhappy branch refuses (`Err`/`false`/`None`/`Locked`), never proceeds
>   (`budget.rs:16`, `token_bucket.rs:5`, `bounded_drainer.rs:11`, `hydra.rs:184`).
> - *fail-closed integrity* — a baseline that cannot be verified locks the organism (`hydra.rs:183-192`).
> - *no float on money* — money is integer minor units; checked arithmetic → `Err`, never wrap
>   (`wasm.rs:27`, `money.rs:7-8` "BP-17 overflow-safe", `domain.rs:69-79`).
> - *fail-closed trust reconstruction* — a JS-boundary order is `price_trusted:false` (`wasm.rs:156-159`);
>   "downstream MUST refuse to charge an untrusted order" (`domain.rs:56-59`).
> - *idempotent local-first append* — a replayed content-id is a `Duplicate` no-op (`event_log.rs:300-301`).

---

## Evidence map (files read, live tree — `/root/dowiz-verify-redteam/kernel/src/`)

| Layer | Files | Key structures / entry points |
|-------|-------|-------------------------------|
| WASM boundary (browser-facing, untrusted) | `wasm.rs`, `web/src/lib/kernel/kernel_client.mjs` | `place_order_js`, `apply_event_js`, `channel_ledger_js`, `reduce_anomalies_js`, `estimate_order_total_js`, `harmonic_centrality_js`, `spectral_*_js`, `geo_*_js` |
| Event-sourcing core | `event_log.rs` | `append` (306), `append_raw` (330), `commit_after_decide` (366), `commit_after_decide_drift_gate` (419), drift-gate (431-445), `verify_chain` (475), `MemEventStore` (210) |
| Order/money FSM | `order_machine.rs`, `domain.rs`, `money.rs` | `allowed_next` (78-94), `assert_transition` (139), `place_order` (156), `place_order_priced` (215), `apply_event` (256), `compensate` (367), `apply_tax` (269), `estimate_order_total` (394), `ledger_sum` (243) |
| Integrity / "organism" | `hydra.rs`, `spectral.rs` | `integrity_check` (180), `boot_verify` (253), `OrganismState` (Live/Locked), `spectral_radius` (217), `eigenvalues` (195), `classify_drift` (342), `roots` (Durand-Kerner, 141) |
| Degrade-closed primitives | `budget.rs`, `bounded_drainer.rs`, `token_bucket.rs` | `ComputeBudget::debit` (113), `BudgetedJobPort::submit` (152), `BoundedDrainer::tick` (70), `TokenBucket::try_acquire` (74) |

**Reachability note.** Anything ending `_js` is called from JS with **fully attacker-controlled JSON**
(`kernel_client.mjs` passes strings straight into the wasm exports). `parse_matrix` (`wasm.rs:625-658`)
rejects non-square/jagged matrices, so the jagged-matrix panic (§4.2) is reachable only through the
*native* `pub` spectral/drift-gate API, not the browser. JSON has no `NaN`/`Infinity` literal
(serde_json rejects them), so `NaN` enters the spectral path only when the solver *generates* it from
huge finite inputs (§4.1) — never as a directly injected matrix cell.

---

## Category 1 — Malformed / adversarial WASM-boundary inputs

### 1.1 `channel_ledger_js` leaks memory permanently, per event — `channel_ledger_box_leak_ooms` — HIGH
- **Weakness**: `channel_ledger_logic` calls `Box::leak(...)` for **every event's** `order_id` and
  `channel` on **every call** (`wasm.rs:235-236`):
  ```rust
  let oid: &'static str = Box::leak(e.order_id.clone().into_boxed_str());
  let ch:  &'static str = Box::leak(e.channel.clone().into_boxed_str());
  ```
  The leaked `'static` strings are **never reclaimed** — not when the call returns, not ever. Two
  amplifiers: (a) one call with a huge events array leaks O(Σ len) bytes; (b) repeated calls
  monotonically accumulate — the wasm linear memory only grows.
- **Input**: from JS, call `channel_ledger_js(JSON.stringify(Array.from({length:1e6}, (_,i)=>({order_id:'x'.repeat(64)+i, channel:'c'.repeat(64), status:'PENDING', at_ms:i}))))`; or the same modest array in a `for(;;)` loop.
- **Expect-safe**: the aggregation must not leak — reuse an owned map keyed by `String`, or intern with
  a bounded arena, or (minimum) cap the event count and free the boxes. Degrade-closed = bounded,
  reclaimable memory; a per-call permanent leak is degrade-**open**.
- **Test**: call `channel_ledger_logic` N times on the native host; assert leaked bytes do not grow
  with N (measure via an allocator counter / `jemalloc` stats), and/or add a `MAX_EVENTS` guard and
  assert an over-cap array returns `Err`. **Why HIGH**: trivially browser-reachable, unbounded,
  unreclaimable — a clean OOM/DoS on the client kernel, contradicting the resource-limiting doctrine.

### 1.2 `apply_event_js` trusts client-supplied `subtotal`/`total` verbatim — `apply_event_trusts_forged_totals` — HIGH
- **Weakness**: `order_from_in` (`wasm.rs:142-161`) reconstructs the `Order` taking `subtotal: o.subtotal`
  and `total: o.total` **directly from the input JSON** — it never recomputes them from `items`. The
  fail-closed `price_trusted:false` is set (good), but §5.6 shows that flag is read by **no** kernel
  money path, so nothing downstream rejects the forged numbers.
- **Input**: `apply_event_js('{"id":"o","status":"CONFIRMED","items":[{"product_id":"p","quantity":1,"unit_price":100}],"subtotal":1,"total":1,"created_at_ms":0}', "PREPARING")`
  — items imply 100 minor units, but `subtotal`/`total` claim `1`. The returned order carries
  `total:1` and advances through the FSM unchallenged.
- **Expect-safe**: at the JS boundary, either recompute `subtotal`/`total` from `items` (and reject on
  mismatch), or refuse to act on an order whose totals are not derivable — matching *no float on money*
  + *fail-closed trust reconstruction*.
- **Test**: feed `apply_event_logic` an order whose `subtotal`/`total` disagree with `Σ quantity·unit_price`;
  assert it is rejected (or the totals are recomputed), not passed through. **Why HIGH**: browser-reachable
  money forgery — the single most concerning finding (see report).

### 1.3 `place_order_js` accepts negative quantity / unit_price → negative money — `place_order_negative_amounts` — HIGH
- **Weakness**: `place_order` computes `subtotal = compute_subtotal(items)` then sets `total: subtotal`
  (`domain.rs:171-179`). `compute_subtotal` uses `checked_mul`/`checked_add` (overflow-safe) but has
  **no sign check**; `place_order` never calls `assert_non_negative` (imported at `domain.rs:22`, used
  only inside `compute_order_total`, which the creation path skips). The doc even concedes it
  (`domain.rs:154-155`: "kept … for future validation (non-negative subtotal, etc.)").
- **Input**: `place_order_js(null, '[{"product_id":"p","modifier_ids":[],"quantity":-5,"unit_price":1000}]', null)`
  → `subtotal = -5000`, `total = -5000`, `Ok`. Same with `unit_price:-1000`.
- **Expect-safe**: reject negative quantity/unit_price and negative subtotal at creation (`Err(Invalid)`),
  matching *no float on money* (a negative earn leg is a refund the customer never authorized).
- **Test**: `place_order_logic(None, r#"[{"product_id":"p","modifier_ids":[],"quantity":-5,"unit_price":1000}]"#, None)`
  → assert `Err` (currently `Ok` with `subtotal:-5000`). **Why HIGH**: browser-reachable, produces a
  negative money total that `post_earn` will post verbatim (`domain.rs:86-98`).

### 1.4 `estimate_order_total_js` div-by-zero panic at `tax_rate ≈ -1.0` — `apply_tax_div_by_zero_panic` — HIGH
- **Weakness**: `apply_tax`'s zero-guard rejects only `tax_rate == 0.0` (`money.rs:270`), then in the
  inclusive branch computes `denom = SCALE + rate_micro` and divides by it (`money.rs:279-280`).
  `SCALE = 1_000_000`; `tax_rate = -1.0 → rate_micro = -1_000_000 → denom = 0` → **integer division by
  zero**, which panics in **every** build profile (not just debug). `.ok()` at `money.rs:399` cannot
  catch a panic — the wasm instance traps.
- **Input**: `estimate_order_total_js(100, '{"is_pickup":false,"delivery_fee_flat":200,"has_distance_tiers":false,"tax_rate":-1.0,"price_includes_tax":true,"min_order_value":null}')`.
- **Expect-safe**: validate `tax_rate` (finite, `> -1.0`, sane bound) and return `Err`/degrade to `None`,
  never trap — *degrade-closed*.
- **Test**: `apply_tax(100, -1.0, true)` → assert it returns `Err` (currently panics "attempt to divide
  by zero" at `money.rs:280`). **Why HIGH**: browser-reachable guaranteed panic (release included).

### 1.5 `estimate_order_total_js` unchecked i64 total overflow → fabricated total — `estimate_total_i64_overflow` — HIGH
- **Weakness**: after the `Option`-degraded fee/tax, the final combine is a **raw** i64 add
  (`money.rs:405-407`): `(Some(fee), Some(tax)) => Some(subtotal + fee + tax)`. `subtotal` is a direct
  `i64` parameter of `estimate_order_total_js` (`wasm.rs:408`).
- **Input**: `estimate_order_total_js(9223372036854775807 /* i64::MAX */, '{"is_pickup":false,"delivery_fee_flat":200,"has_distance_tiers":false,"tax_rate":0.0,"price_includes_tax":false,"min_order_value":null}')`
  → `fee=Some(200)`, `tax=Some(0)` → `i64::MAX + 200` → debug panic / **release: silent two's-complement
  wrap → a fabricated (negative) total shown to the client**.
- **Expect-safe**: `checked_add` → `None` on overflow (the module already degrades fee/tax to `None`;
  this last add must too), matching *no float on money* / fail-closed.
- **Test**: `estimate_order_total(i64::MAX, &cfg_flat_fee_no_tax)` → assert `total == None` (currently
  panics/wraps). **Why HIGH**: browser-reachable, fabricates a money number in release wasm.

### 1.6 `apply_tax` unchecked i128 overflow (huge finite `tax_rate`) → fabricated tax — `apply_tax_i128_overflow` — HIGH
- **Weakness**: `money.rs` header claims "all arithmetic OVERFLOW-SAFE" (`money.rs:7-8`), but only the
  **final** `i64::try_from` is checked (`money.rs:286`). The i128 intermediates are raw: `sub * rate_micro`
  and `+ SCALE/2` (`money.rs:284`), and `SCALE + rate_micro` (`money.rs:279`). The float→i128 cast
  saturates (`+Inf→i128::MAX`), feeding pathological magnitudes into unchecked i128 math. The existing
  test `red_tax_overflow_degrades_estimate_to_none` uses `subtotal=1000`, so `sub*rate_micro` stays in
  range and only `try_from` fires — **the large-subtotal i128-overflow path is untested**, which is why
  the false "overflow-safe" claim survives.
- **Input**: `apply_tax(i64::MAX, 1e17, false)` → `sub·rate_micro ≈ 9.2e18 · 1e23 ≈ 9.2e41 > i128::MAX`
  → overflow (debug panic / release wrap → `try_from` may accept a fabricated tax). Also
  `apply_tax(1, f64::INFINITY, true)` → `SCALE + i128::MAX` overflows at `money.rs:279`.
- **Expect-safe**: `checked_mul`/`checked_add` on the i128 intermediates (or clamp `rate_micro` to a sane
  bound) → `Err`, never wrap.
- **Test**: `apply_tax(i64::MAX, 1e17, false)` and `apply_tax(1, f64::INFINITY, true)` → assert `Err`
  (currently panics/wraps). **Why HIGH**: contradicts the module's own overflow-safety contract on a
  reachable input.

### 1.7 `harmonic_centrality_js` unbounded `n` → capacity-overflow panic / OOM / O(n²) — `harmonic_unbounded_n_dos` — HIGH
- **Weakness**: `harmonic_centrality(n, edges)` allocates `vec![Vec::new(); n]` (`harmonic.rs:31`),
  `vec![0.0f64; n]` (`harmonic.rs:39`), `vec![-1; n]` (`harmonic.rs:40`), then an O(n) reset inside an
  O(n) source loop (`harmonic.rs:43-47`) = O(n²) even with zero edges. `n: usize` is passed straight from
  JS (`wasm.rs:723`). **No cap anywhere.**
- **Input**: `harmonic_centrality_js(18446744073709551615 /* usize::MAX */, "[]")` → `vec![_; n]`
  capacity computation overflows → panic "capacity overflow". Or `n = 1e9` → tens of GB → OOM. Or
  `n = 100000, edges = []` → ~1e10 reset iterations → hang.
- **Expect-safe**: a `MAX_NODES` guard returning `Err` before allocation — *degrade-closed*.
- **Test**: `harmonic_centrality(usize::MAX, &[])` and `harmonic_centrality(1<<30, &[])` → assert `Err`
  (currently panics/OOMs). **Why HIGH**: browser-reachable unbounded alloc/compute, no cap.

### 1.8 `spectral_*_js` no matrix-dimension cap → O(n⁴) compute DoS — `spectral_matrix_no_dim_cap` — MEDIUM
- **Weakness**: `parse_matrix` (`wasm.rs:625-658`) caps nothing but shape (square). For `n > 32`,
  `eigenvalues` runs `charpoly` with `matmul` inside a `for k in 2..=n` loop = **O(n⁴)** (`spectral.rs:206`,
  `spectral.rs:113-137`). A browser posting a 400×400 matrix drives ≈2.5·10¹⁰ ops per call.
- **Input**: `spectral_radius_js(JSON.stringify(Array.from({length:400},()=>Array(400).fill(1))))` in a loop.
- **Expect-safe**: a `MAX_MATRIX_DIM` guard in `parse_matrix` → `Err`.
- **Test**: assert `parse_matrix` rejects an n > cap matrix; or time `eigenvalues` at n=200 vs n=400 to
  document the quartic blow-up. **Why MEDIUM**: bounded (no infinite loop) but a real wall-clock DoS,
  untested, no cap.

### 1.9 Deeply-nested / oversized JSON — `nested_json_bounded_flat_array_unbounded` — LOW/MEDIUM
- **Input (a)**: `place_order_js`/`channel_ledger_js` with `[[[[...128+ deep...]]]]`. **Expect**:
  serde_json's default 128-depth recursion limit returns `Err`, no stack overflow — pin as LOW.
- **Input (b)**: a giant *flat* items/events array. **Expect (current)**: unbounded `Vec` allocation
  (`wasm.rs:189,223,266`), plus the §1.1 leak for events — no length cap. Recommend a `MAX_ITEMS`/
  `MAX_EVENTS` guard. MEDIUM (overlaps §1.1).

### 1.10 Unicode / injection strings in text fields — `unicode_text_fields_no_kernel_sink` — LOW
- **Input**: `product_id`/`customer_id`/`order_id`/`channel` = `"'; DROP TABLE…"`, NUL bytes, RTL
  overrides, 4-byte emoji, 10 MB string.
- **Expect**: the kernel has no SQL/eval/format sink for these — they are stored/echoed as opaque
  `String`s. Pin that they round-trip unchanged and cause no panic (the only real risk is the §1.1
  leak amplifier and any *downstream* consumer, out of kernel scope). LOW.

---

## Category 2 — Replay / reorder on the event log

### 2.1 `append` replay of a zero-`prev` event double-commits — `append_zero_prev_replay_double_commits` — HIGH
- **Weakness**: `append` rebinds `prev` to the current tip **before** computing the content-id
  (`event_log.rs:306-311`), then dedups on that id (`event_log.rs:312`):
  ```rust
  if ev.prev == [0u8;32] { if let Some(tip) = self.store.tip() { ev.prev = tip; } }
  let id = ev.event_id();
  if self.store.contains(&id) { return Ok(Duplicate(id)); }
  ```
  First `append(E{prev:0})` on an empty log stores under `id_E` (prev stays 0). Replaying the
  **identical** `E{prev:0}` now sees `tip = Some(id_E)`, rebinds `prev → id_E`, computes a **different**
  id, `contains` misses → `Committed` a second time. The idempotency key includes chain position, so
  the same logical event replayed at a different tip is not a duplicate.
- **Input**: build a fresh `EventLog`; `append(e_zero_prev)` twice (same event).
- **Expect-safe**: the second `append` returns `Duplicate` and `len() == 1` — matching *idempotent
  local-first append* (`event_log.rs:300-301`). Note `append_raw` (`event_log.rs:330-338`, verbatim
  `prev`) and `commit_after_decide` are correctly idempotent — this gap is specific to `append`'s
  tip-rebind.
- **Test**: `let a = log.append(e)?; let b = log.append(e.clone())?; assert!(matches!(b, Duplicate(_))); assert_eq!(log.len(), 1);`
  (currently the 2nd is `Committed`, `len()==2`). **Why HIGH**: shipped code, no regression test (the
  only chaining test appends two *distinct* events), chain-integrity double-write.

### 2.2 Multi-writer TOCTOU across `contains → decide → insert` re-runs `decide` — `commit_toctou_double_decide` — HIGH
- **Weakness**: `commit_after_decide` checks `contains`, then runs the side-effecting `decide` closure,
  then `insert`s (`event_log.rs:378-389`):
  ```rust
  if self.store.contains(&id) { return Ok((Duplicate(id), None)); }
  let decision = decide(&ev)?;              // side effect (money law)
  let outcome = self.append_raw(ev)?;
  ```
  `&mut self` serialises one in-process `EventLog`, but the module explicitly targets a shared
  **multi-writer pgrust** store (`event_log.rs:19`). Two writers each round-trip
  `contains`→miss→`decide`→`insert` with no transaction/row-lock → `decide` runs **twice** for one
  content-id (the "a replayed `SettlementClaimed` must never re-run its hashlock" invariant, doc
  `event_log.rs:363-365`, breaks under concurrency). Also any store whose `contains` lies (the test
  `FaultyStore::contains` returns `false`, `event_log.rs:547-549`) defeats all dedup — idempotency is
  only as honest as the store.
- **Input (design)**: two `EventLog`s over one shared store; interleave two `commit_after_decide` of the
  same event so both pass `contains` before either `insert`s.
- **Expect-safe**: dedup+decide+insert must be one atomic transaction (or `decide` must be a pure
  function whose effect is itself keyed on the content-id) — *idempotent local-first append*.
- **Test**: a `SharedStore` behind a barrier that releases both `contains` calls before either
  `insert`; assert `decide` fires once. **Why HIGH**: the exactly-once money guarantee does not hold on
  the stated deployment target.

### 2.3 `commit_after_decide` / `append_raw` single-writer idempotency — `commit_replay_is_idempotent` — LOW (pin)
- **Input**: `commit_after_decide(e, decide)` twice on the same log (single writer).
- **Expect**: 2nd → `Duplicate`, `decide` does **not** re-run, root unchanged (`event_log.rs:330-338,
  366-391`; covered by `dup_event_is_idempotent_no_state_change` / `commit_after_decide_replay…`). Pin
  as the correct baseline that §2.1/§2.2 deviate from.

### 2.4 `MemEventStore` dedup is non-durable — `restart_replays_all_events` — MEDIUM
- **Weakness**: `by_id`/`by_event` are in-memory `HashSet`/`HashMap` (`event_log.rs:210-234`); after any
  restart they are empty → every prior event replays and re-runs `decide`. The `EventStore` trait makes
  no durability guarantee (module doc line 7 disclaims TTL/persistence).
- **Test**: build a store, commit E, drop+recreate the store, commit E again → assert it is treated as a
  duplicate by a *durable* store (documents that `MemEventStore` is not, so exactly-once depends on the
  unshipped pgrust impl).

### 2.5 No per-actor sequence monotonicity — `actor_seq_reorder_undetected` — MEDIUM
- **Weakness**: `actor_seq` is documented "per-actor monotonic" but nothing checks it
  (`event_log.rs:133-155`); a replay with a decremented/fresh seq and identical payload hashes to a new
  id → fresh commit + `decide` re-run. Dedup is content-address, not semantic ordering.
- **Test**: append two events for one actor with `actor_seq` 5 then 3 → assert both accepted (documents
  the missing monotonicity gate).

---

## Category 3 — Resource exhaustion / degrade-closed bypass

### 3.1 `channel_ledger_js` permanent leak — see §1.1 — HIGH
### 3.2 `harmonic_centrality_js` unbounded `n` — see §1.7 — HIGH

### 3.3 Uncapped event `payload: Vec<u8>` → unbounded alloc + hash — `event_payload_uncapped` — HIGH
- **Weakness**: `MeshEvent.payload` is an uncapped `Vec<u8>` (`event_log.rs:142`), copied whole in
  `event_id` (`Vec::with_capacity(…+payload.len())`, `event_log.rs:149`) and in `sha3_256`
  (`input.to_vec()`, `event_log.rs:105`). One event with a multi-GB payload → multi-GB alloc + hash. No
  length guard.
- **Test**: assert `append`/`commit_after_decide` reject (or cap) an oversized payload — currently no
  cap exists to assert against; add `MAX_PAYLOAD_BYTES`. **Why HIGH**: unbounded per-event work, no
  degrade-closed bound.

### 3.4 Event log grows without bound — `event_log_unbounded_growth` — MEDIUM/HIGH
- **Weakness**: `insert`+`set_tip` with no cap (`event_log.rs:318,335`); `MemEventStore` keeps every
  body forever (`event_log.rs:213,234`), no eviction/TTL/max-count. N distinct events → O(N) memory;
  `verify_chain` cost grows O(len) full-payload re-hashes (`event_log.rs:481-501`), so an attacker-grown
  log makes every integrity walk arbitrarily expensive.
- **Test**: pin that a `max_len`/epoch bound exists or document the growth DoS.

### 3.5 `ComputeBudget`/`BudgetedJobPort` NaN defeats the ceiling — `budget_nan_poisons_ceiling` — MEDIUM
- **Weakness**: the degrade-closed gate is a raw float compare: `if self.spent + amount > self.ceiling`
  (`budget.rs:114`) and `if b.spent() + job.estimate > b.ceiling()` (`budget.rs:157`). A `NaN` amount
  makes the comparison `false` → the code takes the *allow* branch and does `spent += NaN`
  (`budget.rs:117`) → `spent` becomes `NaN` → **every future** debit compares `NaN + x > ceiling`
  = `false` → always allowed. The budget is permanently disabled — degrade-**open**.
- **Input**: `ComputeBudget::new(10.0).debit(f64::NAN)` then `debit(1e9)` → returns `true`.
- **Expect-safe**: reject non-finite `amount`/`estimate` (`Err`/`false`) before touching `spent` —
  *degrade-closed*. **Reachability caveat**: `Job.estimate` is set by internal callers, not the wasm
  boundary, so this is MEDIUM (doctrine violation, low direct reachability) — but it is the same
  defeated-guard shape as the hydra NaN issue (§4.1).
- **Test**: `let mut b = ComputeBudget::new(10.0); assert!(!b.debit(f64::NAN)); assert_eq!(b.spent(), 0.0);`
  (currently `debit(NaN)` returns `true` and poisons `spent`).

### 3.6 `TokenBucket::try_acquire` with negative `n` mints tokens — `token_bucket_negative_acquire_mints` — LOW/MEDIUM
- **Weakness**: `try_acquire(n)` does `if inner.tokens >= n { inner.tokens -= n; true }`
  (`token_bucket.rs:87-92`). For `n < 0`, `tokens >= n` is true and `tokens -= n` **adds** tokens — a
  negative acquire refills the bucket. (NaN `n` is safe: `tokens >= NaN` is `false` → refused.)
- **Expect-safe**: reject `n <= 0` / non-finite. **Reachability**: `cost_per_unit`/`n` are internal, not
  wasm-supplied → LOW; pin as a regression so a future caller can't feed a signed delta.
- **Test**: `let b = TokenBucket::new(1.0, 0.0); b.try_acquire(-100.0); assert!(b.available() <= 1.0);`
  (currently `available()` jumps to ~101).

### 3.7 `BoundedDrainer` / `TokenBucket` degrade-closed under exhaustion — `drainer_stops_when_bucket_empty` — LOW (pin)
- **Input**: `BoundedDrainer::new(100, 10, 1.0)` with a `TokenBucket::new(3.0, 0.0)`; `tick`.
- **Expect**: runs exactly 3 units then stops early, 97 stay queued (`bounded_drainer.rs:70-82`;
  covered by `degrade_closed_when_budget_exhausted`). Also pin `TokenBucket`'s monotonic-`Instant`
  refill (`token_bucket.rs:48-59`) and poison-recovery `unwrap_or_else(into_inner)`
  (`token_bucket.rs:75-78`). These are the doctrine done right — pin them as the reference.

---

## Category 4 — Integrity (`hydra.rs`) edge cases: NaN / infinity / degenerate graph

### 4.1 NaN eigenvalue is masked to 0.0 → integrity/drift FAIL OPEN — `spectral_nan_mask_fails_open` — HIGH
- **Weakness**: `spectral_radius` folds eigenvalue moduli with `fold(0.0, f64::max)` (`spectral.rs:218`).
  Rust's `f64::max` **drops NaN** (returns the other argument), and the accumulator seeds at `0.0`, so a
  solver that produces NaN eigenvalues yields `spectral_radius == 0.0` — the *healthiest possible* value,
  indistinguishable from a perfectly-damped ρ=0 graph. This **defeats** the guard written to catch it:
  ```rust
  // hydra.rs:186 — is_finite() was meant to catch NaN, but rho is already 0.0
  if rho < 1.0 && rho.is_finite() { /* Locked -> Live */ } else { self.state = Locked; }
  ```
  `0.0 < 1.0 && 0.0.is_finite()` → the organism stays/returns **Live**. Same in `classify_drift`
  (`spectral.rs:342-352`): `rho = 0.0` → `Damped` → the event-log drift-gate passes the mutation. Because
  the mask happens *upstream*, it is worse than the textbook "NaN comparison is false → else-branch": it
  lands on the *most-healthy* verdict, not the marginal one.
- **Input**: an adjacency the solver cannot evaluate to a finite spectrum. `topology_adjacency` filters
  non-finite *weights* (`hydra.rs:44`), so NaN must be *generated internally* — e.g. huge finite weights
  on an `n > 32` graph that overflow `charpoly` (§4.9), or a near-coincident Durand-Kerner denominator
  that underflows (§4.9).
- **Expect-safe**: `spectral_radius` must **propagate** a non-finite spectrum (return `NaN`/`f64::INFINITY`,
  e.g. seed the fold with `f64::NAN` awareness or check `is_nan` per element) so the `is_finite()` guard
  in `integrity_check`/`boot_verify`/`classify_drift` actually fires → `Locked`/`Unstable`. *Fail-closed
  integrity*.
- **Test**: unit-test `spectral_radius` on a matrix whose eigensolve yields a NaN component; assert it
  does **not** return `0.0`. Then `integrity_check` on the same baseline → assert `Locked` (currently
  `Live`). **Why HIGH / most concerning safety-gate**: a fail-closed guard that the code specifically
  wrote (`is_finite()`) is silently defeated — the direct analog of bebop2's "dormant safety gate."

### 4.2 Jagged / non-square matrix → index-OOB panic (native gate) — `spectral_jagged_matrix_panics` — HIGH
- **Weakness**: `eigenvalues` (n≤32 path) fills `buf[i*n+j] = a[i][j]` with `n = a.len()`, assuming every
  row has ≥ n cols (`spectral.rs:199-202`). A row shorter than `n` panics (index OOB). Same in
  `laplacian` (`spectral.rs:291-293`, feeds `algebraic_connectivity`). `spectral_radius`,
  `classify_drift`, `graph_energy` all funnel through `eigenvalues`.
- **Reachability**: the wasm `parse_matrix` rejects jagged input (`wasm.rs:642-644`), so this is reachable
  through the **native** `pub` API — notably the event-log drift-gate
  `commit_after_decide_drift_gate(ev, adjacency: &[Vec<f64>], …)` (`event_log.rs:419-421`), whose own
  comment (`event_log.rs:443`) shows `row.len()` can differ from `adjacency.len()`.
- **Input**: `classify_drift(&vec![vec![1.0,2.0], vec![3.0]])` (2 rows, row 1 has 1 col).
- **Expect-safe**: validate rectangularity → return a neutral/`Err` result, never panic — *degrade-closed*.
- **Test**: `classify_drift(&[vec![1.0,2.0], vec![3.0]])` → assert no panic (currently OOB-panics at
  `spectral.rs:201`). **Why HIGH**: reachable kernel panic (DoS) via a `pub` gate.

### 4.3 `boot_verify` `assert!` panics on a tampered baseline + fail-opens on NaN — `boot_verify_assert_panic_and_nan` — MEDIUM
- **Weakness**: `boot_verify` uses `assert!(rho < 1.0 && rho.is_finite(), …)` (`hydra.rs:257-263`), which
  is **not** stripped in release. (a) An attacker who shifts the baseline to ρ≥1 turns boot into a hard
  panic/abort (self-DoS), where the sibling `integrity_check` handles the same condition gracefully →
  `Locked`. (b) A NaN-producing baseline (§4.1) makes the assert pass (`0.0 < 1.0`) and `boot_verify`
  returns `0.0` (reported healthy).
- **Test**: `boot_verify` on a ρ=2 self-loop baseline → currently panics (decide if graceful `Locked` is
  wanted); on a NaN-generating baseline → currently returns `0.0` (should fault).

### 4.4 `integrity_check` silently auto-heals Locked→Live — `integrity_auto_heal_no_reseed` — MEDIUM
- **Weakness**: on any later check where the *current* spectrum is healthy, `integrity_check` flips
  `Locked → Live` (`hydra.rs:186-190`) with no owner action, no re-seed, no M9 gate — contradicting its
  own contract ("owner re-seed / M9 required", `hydra.rs:208,229`). A tamperer who shifts `base_edges`
  to ρ≥1 (Locked) then restores it silently reopens the gate on the next check.
- **Test**: Live → tamper (ρ=2) → `integrity_check == Locked` → restore `base_edges` → `integrity_check`
  → assert it should stay `Locked` until an explicit re-seed API (currently returns `Live`).

### 4.5 `Hydra::new` defaults to `Live` with no check — `hydra_new_default_live_unchecked` — MEDIUM
- **Weakness**: `Hydra::new` sets `state: OrganismState::Live` unconditionally (`hydra.rs:163-168`); the
  integrity check only runs lazily in `commit`/`integrity_check`. A caller reading `state()`
  (`hydra.rs:198`) right after `new()` with an already-tampered `base_edges` (ρ≥1) sees a fail-open
  default.
- **Test**: `Hydra::new(store, n, tampered_edges_rho2).state()` → assert not silently `Live`.

### 4.6 Empty adjacency → ρ=0 → drift-gate is a no-op — `empty_adjacency_gate_noop` — MEDIUM
- **Weakness**: `spectral_radius(&[]) == 0.0` (`spectral.rs:217-218`) → `classify_drift → Damped`
  (`spectral.rs:342`). The event-log drift-gate reads only the adjacency spectral radius, never the
  `payload` (`event_log.rs:431-445`); an empty/benign topology with an arbitrary money/order payload
  riding along always passes.
- **Test**: `commit_after_decide_drift_gate(ev, &[], false, …)` → assert `Committed` (documents that
  drift provides no protection against a malicious payload on benign topology).

### 4.7 `+Infinity` correctly fail-closes to Locked — `inf_rho_locks` — LOW (pin)
- **Input**: a baseline whose ρ overflows to `+Inf`. **Expect**: `Inf < 1.0` is `false` →
  `integrity_check` → `Locked` (`hydra.rs:186`). The `&&` ordering is load-bearing — pin a regression so
  a refactor to `!(rho >= 1.0)` (which would fail-open on NaN) is caught.

### 4.8 `classify_drift` would mis-class NaN as `Resonant` if the mask were removed — `classify_drift_nan_latent` — LOW
- Today NaN can't reach the compare (masked by §4.1). If `spectral_radius` is fixed to propagate NaN
  without also fixing `classify_drift`, NaN falls through both `if`s to `else => Resonant`
  (`spectral.rs:345-351`) — marginal/allowed, a residual fail-open. Pin: when fixing §4.1, add an
  explicit `rho.is_nan() → Unstable` guard.

### 4.9 Durand-Kerner emits NaN/Inf roots and hides non-convergence — `dk_nan_and_silent_nonconvergence` — MEDIUM
- **Weakness**: `roots` guards only *exact*-zero denominators (`if denom.is_zero() { continue }`,
  `spectral.rs:171`, where `is_zero` is `re==0.0 && im==0.0`); a near-zero denom `(1e-300,0)` passes,
  then `Complex::div` computes `d = re²+im²` which underflows to `0.0` → Inf/NaN root (`spectral.rs:76-82`).
  On non-convergence the loop just exits after the 200-iter cap and returns the **un-converged** estimates
  with no error signal (`spectral.rs:181-183`). These NaN/garbage roots feed the §4.1 mask.
  (Pin the positive: the 200-iteration cap `DK_ITERS` at `spectral.rs:160` means there is **no
  infinite-loop DoS** in `roots` — regression-pin the constant.)
- **Test**: a matrix engineered so DK yields a NaN component; assert `spectral_radius` surfaces it rather
  than returning `0.0`.

### 4.10 Drift-gate `intervention == true` bypasses all drift safety — `drift_gate_intervention_bypass` — HIGH
- **Weakness**: `commit_after_decide_drift_gate` runs the `Unstable` reject **only** `if !intervention`
  (`event_log.rs:431`); `intervention` is a plain caller-supplied `bool`, threaded verbatim from
  `hydra.rs:214,218,244` with zero authorization. Anyone in the call path who sets `intervention=true`
  lifts the entire drift gate — a fail-open on an unauthenticated flag (the existing test
  `drift_gate_lifts_on_intervention` *asserts this as intended*).
- **Input**: `commit_after_decide_drift_gate(ev, &adj_rho_2, /*intervention=*/true, decide)`.
- **Expect-safe**: `intervention` must be an authenticated/operator-gated capability, not a bare bool —
  matching the "kill-switch / M9 is operator-gated" doctrine. **Test**: assert an unstable-topology
  commit is rejected when `intervention=false` and pin the `true` path as the known fail-open pole
  pending an authorization gate. **Why HIGH**: a single caller bool disables the safety spectrum gate.

> **Kill-switch note**: there is **no** hard kill-switch in `event_log.rs` — the comment "The only hard
> stop remains kill-switch" (`event_log.rs:413`) has no corresponding code; the only hard stop is hydra's
> `Locked`, which is non-latching (auto-heals, §4.4) and defeated by the NaN mask (§4.1). No `fn kill`/
> `fn stop` exists (`hydra.rs` "M9 kill-switch" is comment-only). This is itself a MEDIUM finding: the
> advertised hard stop is not implemented.

---

## Category 5 — Money / order state-machine absurd scenarios

### 5.1 Transition allowlist is airtight — `every_illegal_edge_rejected` — LOW (pin)
- **Input**: table-drive **every** `(from, to)` not in `allowed_next` (`order_machine.rs:78-94`): e.g.
  `Pending→Delivered`, `Delivered→anything`, `Rejected→Confirmed`, `Cancelled→Preparing`,
  `CompensatedRefund→anything`, plus every `from==to`.
- **Expect**: `assert_transition` returns `Err(Illegal)` for each (`order_machine.rs:139-151`); terminals
  have empty edge sets so no terminal can be re-exited. Pin this as the correct baseline — the allowlist
  is a strict allowlist, no silently-accepted illegal edge exists.

### 5.2 `CompensatedRefund` reachable via `apply_event` without reversing the ledger — `compensated_terminal_money_not_reversed` — HIGH
- **Weakness**: the "compensated terminal nets to zero by construction" claim (`money.rs:126`,
  `domain.rs:22-24`) is convention, not FSM-enforced. `apply_event` is a pure status move that never
  touches the ledger (`domain.rs:256-267`); the reversal lives only in `compensate` (`domain.rs:378`).
  Nothing gates entry into `CompensatedRefund` on `ledger_balance() == 0`.
- **Input**: `apply_event(order, Refunding)` then `apply_event(refunding, CompensatedRefund)` (both legal
  edges), skipping `compensate` entirely → terminal `CompensatedRefund` with the earn leg intact
  (`ledger_balance() == total`, not 0).
- **Expect-safe**: entering `CompensatedRefund` must require `ledger_balance() == 0` (or route only
  through `compensate`) — a compensated order that still owes/holds money is a money-integrity break.
- **Test**: drive Refunding→CompensatedRefund via `apply_event` only; assert `ledger_balance() != 0` at
  a "compensated" terminal (proves the gap). **Why HIGH**: reachable money-inconsistent terminal state.

### 5.3 `compensate` reverses money on ANY legal edge, not only a refund — `compensate_reverses_on_any_edge` — MEDIUM/HIGH
- **Weakness**: `compensate(order, next, earn_id, reversal_id)` does `apply_event(order, next)?` then
  **unconditionally** reverses the earn leg (`domain.rs:374-378`); `next` is caller-supplied and never
  constrained to `Refunding`.
- **Input**: `compensate(confirmed_order, OrderStatus::Preparing, 1, 2)` — legal `Confirmed→Preparing` —
  refunds the money while the order advances toward fulfillment (goods delivered **and** money returned).
- **Expect-safe**: `compensate` must reject `next` unless it is a refund/cancel edge.
- **Test**: `compensate(confirmed, Preparing, 1, 2)` → assert `Err` (currently nets the ledger to zero
  while advancing — that's the bug).

### 5.4 No version/CAS guard → concurrent Accept + Cancel both succeed — `no_optimistic_concurrency_lost_update` — MEDIUM
- **Weakness**: `apply_event(&Order, next)` is pure over an immutable borrow (`domain.rs:256-264`);
  `Order` has no `version`/`seq`/`updated_at` field (`domain.rs:42-64`). From `Pending`, both `Confirmed`
  (courier accept) and `Cancelled` (customer cancel) are legal (`order_machine.rs:81`), so applying both
  to the same base returns `Ok` for each — lost-update lives entirely in the (absent-here) caller.
- **Test**: apply two different legal transitions to one base `Order`; assert both are `Ok` (documents the
  absence of any stale-state/CAS guard).

### 5.5 Negative quantity/price → negative total — see §1.3 — HIGH
- Same root as §1.3 but framed at the FSM/money layer: `place_order_priced` (the *trusted* path,
  `domain.rs:215-220`) re-derives only `unit_price` and leaves `quantity` caller-controlled, so a negative
  `quantity` yields a negative subtotal **even for `price_trusted=true`** orders; `post_earn(o.total, …)`
  then posts a negative earn leg. Test both `place_order` and `place_order_priced` with `quantity:-5`.

### 5.6 `price_trusted` is set but never enforced — `price_trusted_never_checked` — HIGH
- **Weakness**: `price_trusted` is assigned `false` (`domain.rs:184`) / `true` (`domain.rs:237`) with the
  mandate "Downstream (charge/settlement) MUST refuse to charge an untrusted order" (`domain.rs:56-59`),
  but within the kernel it is **read by no code path** — only by tests (`domain.rs:547,571`). The
  money-moving `post_earn` (`domain.rs:86-98`), `apply_event`, `compensate`, `recompute_total` all ignore
  it. An attacker-priced legacy order (`place_order`, `price_trusted=false`) is driven through the whole
  lifecycle and earn-posted with zero trust check.
- **Input**: `place_order(...)` (untrusted) → `order.post_earn(order.total, 1, EUR)` → succeeds.
- **Expect-safe**: `post_earn`/settlement must refuse when `price_trusted == false` — *fail-closed trust
  reconstruction*.
- **Test**: untrusted order → `post_earn(total,…)` → assert `Err` (currently `Ok`). **Why HIGH**: the one
  control designed to stop price tampering gates nothing; it is the server-side twin of §1.2.

### 5.7 `ledger_sum` / `estimate_order_total` unchecked additions — `ledger_and_estimate_unchecked_add` — MEDIUM
- **Weakness (a)**: `ledger_sum` uses `.map(|e| e.amount.minor).sum()` (`money.rs:243`); two large earn
  legs (two `post_earn` with distinct ids) overflow the i64 `.sum()` → panic(debug)/wrap(release).
  `ledger_balance()` calls it (`domain.rs:104`). **(b)**: `estimate_order_total`'s final add (§1.5).
- **Test**: two `post_earn` legs near `i64::MAX`; call `ledger_balance()` → assert no panic/wrap.

---

## Category 6 — "Absurd" / semantically-valid-but-nonsensical

### 6.1 Device with a wildly wrong clock skews anomaly detection — `wrong_clock_reduce_anomalies_evasion` — MEDIUM
- **Weakness**: `reduce_anomalies` keys per-order events in a `BTreeMap<i64, OrderStatus>` by `at_ms`
  (`analytics.rs:143-146`): a **duplicate** `at_ms` overwrites the earlier status (silently dropping an
  event), and the fold **seeds from `statuses[0]`** (`analytics.rs:158-160`), not `Pending` as the doc
  claims (`analytics.rs:130`). So a courier phone with a wrong/duplicate clock reorders or hides the
  status sequence and evades the illegal-transition counter.
- **Input (a)**: `[(o,Pending,1),(o,Delivered,1)]` → BTreeMap collapses to `{1:Delivered}` → `steps=[]` →
  the illegal `Pending→Delivered` jump is **not** counted. **(b)**: `[(o,Delivered,1)]` → `start=Delivered`,
  no steps → an order that appears already-delivered with no history is not flagged.
- **Expect-safe**: seed the fold from `Pending`, and treat duplicate/out-of-order `at_ms` as an anomaly
  (or key on a monotonic sequence, not the caller clock).
- **Test**: `reduce_anomalies(&[(o,Pending,1),(o,Delivered,1)])` → assert count `== 1` (currently `0`).

### 6.2 Token bucket is NTP-jump-proof (monotonic clock) — `token_bucket_monotonic_clock` — LOW (pin)
- **Input (design)**: refill uses `Instant::now()` (`token_bucket.rs:49`), not wall-clock; a device NTP
  jump cannot mint tokens. **Expect**: pin that refill is driven by `saturating_duration_since` on a
  monotonic `Instant` (`token_bucket.rs:50-52`) so a backward wall-clock jump can never bypass the
  throttle. The doctrine done right — pin as reference. (Contrast the caller-supplied `at_ms` in §6.1,
  which *is* clock-manipulable.)

### 6.3 Tampered JS context can't corrupt kernel state, but forges money numbers — `tampered_context_forges_money` — HIGH
- **Weakness**: the wasm exports are **free functions** (no `this`); the only mutable static is
  `ORDER_SEQ: AtomicU64` (`wasm.rs:51`), which a tampered caller cannot corrupt (atomic, monotonic). So a
  tampered `this`/prototype cannot poison kernel state — *but* the JSON is fully attacker-controlled, and
  `apply_event_js` trusts `subtotal`/`total` from it (§1.2). The "context" an attacker actually controls
  is the money payload, not the runtime object.
- **Test**: assert `ORDER_SEQ`-driven `id`/`created_at_ms` are monotonic regardless of caller (pin), and
  cross-reference §1.2 for the forged-total gap. **Why HIGH**: rolls up to the §1.2 money-forgery vector.

### 6.4 Rapid repeated calls — reentrancy is safe except the leak — `rapid_repeat_calls_atomic_but_leaky` — LOW (pin) / see §1.1
- **Input**: hammer `place_order_js` from many concurrent JS calls (in wasm, single-threaded; the concern
  is the atomic). **Expect**: `ORDER_SEQ.fetch_add(Ordering::SeqCst)` (`wasm.rs:192`) yields unique
  monotonic ids — reentrant-safe; pin it. The real rapid-call hazard is the §1.1 `Box::leak` amplifier,
  not a data race.

### 6.5 Semantically nonsensical but structurally valid order — `valid_shape_nonsense_semantics` — LOW/MEDIUM
- **Input**: `apply_event_js` on an order with `items: []` (subtotal 0) but `total: i64::MIN`, or a valid
  status move on a self-contradictory order.
- **Expect**: the FSM is payload-agnostic — it authorizes the *transition*, not the money semantics. Pin
  that semantic validation is the money layer's job (§1.2/§5.2/§5.6 show that layer is currently
  under-enforced), so a structurally valid but nonsensical order rides through today.

---

## Priority tally

**HIGH (13)** — suspected real current-code weakness:
- 1.1 `channel_ledger_js` permanent per-event `Box::leak` (browser OOM)
- 1.2 `apply_event_js` trusts client-forged `subtotal`/`total`
- 1.3 `place_order_js` accepts negative quantity/unit_price → negative money
- 1.4 `estimate_order_total_js` div-by-zero panic at `tax_rate ≈ -1.0` (all builds)
- 1.5 `estimate_order_total` unchecked i64 total overflow → fabricated total
- 1.6 `apply_tax` unchecked i128 overflow → fabricated tax (contradicts "overflow-safe")
- 1.7 `harmonic_centrality_js` unbounded `n` → capacity-panic / OOM / O(n²)
- 2.1 `append` zero-`prev` replay double-commits
- 2.2 multi-writer TOCTOU `contains→decide→insert` re-runs `decide` (pgrust target)
- 3.3 uncapped event `payload: Vec<u8>` unbounded alloc + hash
- 4.1 spectral NaN mask → integrity/drift **fail open** (defeats the `is_finite()` guard)
- 4.2 jagged matrix → index-OOB panic via the `pub` drift-gate
- 4.10 drift-gate `intervention=true` disables the spectrum gate (unauthenticated bool)
- 5.2 `CompensatedRefund` reachable via `apply_event` without ledger reversal
- 5.6 `price_trusted` set but never enforced (server twin of 1.2)

*(15 line items; 1.5/1.6 are two faces of the money-arithmetic weakness and 5.5 shares 1.3's root — count
them as ~13 distinct HIGH weaknesses.)*

**MEDIUM (12)**: 1.8, 1.9(b), 2.4, 2.5, 3.4, 3.5, 4.3, 4.4, 4.5, 4.6, 4.9, 5.3, 5.4, 5.7, 6.1
(+ the "no real kill-switch" note).

**LOW (regression pins)**: 1.9(a), 1.10, 2.3, 3.6, 3.7, 4.7, 4.8, 5.1, 6.2, 6.4, 6.5.

---

## Cross-cutting recommendations (for the fix agent, not applied here)
1. **Recompute money at the JS boundary** — in `order_from_in`, recompute `subtotal`/`total` from
   `items` and reject on mismatch; make `post_earn`/settlement **refuse** when `price_trusted == false`
   (closes 1.2, 5.6, and half of 6.3/6.5 in one move).
2. **Sign + bound money at creation** — `place_order`/`place_order_priced` reject negative
   quantity/unit_price/subtotal and cap item count (closes 1.3/5.5).
3. **Make money arithmetic honestly overflow-safe** — validate `tax_rate` (finite, `> -1.0`, bounded);
   `checked_*` the i128 intermediates in `apply_tax` and the final `subtotal+fee+tax`/`ledger_sum` adds
   (closes 1.4, 1.5, 1.6, 5.7).
4. **Propagate non-finite spectra** — `spectral_radius` must not mask NaN to 0.0; add `rho.is_nan() →
   Unstable/Locked` so the `is_finite()` guards actually fire (closes 4.1, hardens 4.3/4.8).
5. **Rectangularity-check matrices** before `eigenvalues`/`laplacian` (closes 4.2).
6. **Cap unbounded work** — `MAX_EVENTS`/`MAX_ITEMS` at the wasm boundary + stop the `Box::leak`
   (own the strings), `MAX_NODES` in `harmonic_centrality`, `MAX_MATRIX_DIM` in `parse_matrix`,
   `MAX_PAYLOAD_BYTES` on `MeshEvent`, `MAX_LEN`/epoch on the event log (closes 1.1, 1.7, 1.8, 3.3, 3.4).
7. **Fix the append replay + commit atomicity** — either always chain via `append_raw`'s verbatim `prev`,
   or dedup on a chain-independent id; wrap `contains→decide→insert` in one transaction (closes 2.1, 2.2).
8. **Authorize `intervention`** and make `Locked` latch (require an explicit owner re-seed API instead of
   auto-heal); implement a real `fn kill` if the "hard stop" is to exist (closes 4.10, 4.4, kill-switch note).
9. **Gate money-consistent terminals** — require `ledger_balance()==0` to enter `CompensatedRefund`, and
   constrain `compensate`'s `next` to refund/cancel edges (closes 5.2, 5.3).
10. **Reject non-finite in the degrade-closed primitives** — `ComputeBudget::debit`/`TokenBucket::try_acquire`
    reject `NaN`/`≤0` before mutating state (closes 3.5, 3.6).
