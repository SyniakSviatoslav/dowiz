# BLUEPRINT P94 — Scope/Effect In-Memory Bitmask Representation (2026-07-19)

> **Standalone PROTOCOL blueprint (bebop2 `proto-cap`).** One coherent, independently buildable unit
> against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Scope source:
> `SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` §5 (the scope stub this pass expands to the full
> contract). Real evidence: `docs/research/OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN-2026-07-19.md` (the ONE
> genuine candidate the whole-layer scan found). Format precedent + direct sibling:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree: `/root/bebop-repo/bebop2` at
> HEAD, read live this pass (§0). Dependency sequence: **M1 → P93 → P92 → P94** (this is P94 — last,
> because its value is *created* by P92).
>
> **One sentence:** replace the `O(n·m)` linear `Vec` scans and per-frame/per-link heap clones on the
> hot authorization path with an **in-memory-only** `ScopeMask([u32; 18])` (`Copy`, branchless word-AND
> subset, zero allocation) derived at parse/verify time — while the **TLV ordered-pair list stays the
> byte-for-byte-unchanged canonical serialize/sign form**, so this is a **correctness-preserving refactor
> that changes nothing on the wire or in any signature**, and the entire verification plan is built to
> *prove* that equivalence.

---

## VERDICT (stated up front, per session research discipline)

**GO — but sequenced LAST (after P92), and small-on-its-own by honest admission.** This is not a bet
and not a hazard; it is a pure in-memory representation change with a machine-checkable equivalence
proof. Two disciplines make it safe and honest:

1. **Sequence after P92 — its value is created by the fast-path.** On the crypto-dominated full
   per-frame path, the bitmask is a *modest* allocation-elimination win (2× Ed25519 + 1× ML-DSA-65
   verify dominate; shaving `Vec` clones is secondary — S4 §5). It becomes a *genuine compounding*
   per-frame saving **only once P92 amortizes the crypto to once-per-session**, making `effect ⊆
   session_cap` "the dominant per-frame authorization operation." Building P94 before P92 would be
   optimizing a path the crypto already dominates — premature (SYNTHESIS §2.3). It is therefore
   **value-dependent on P92**, not functionally dependent (it compiles and runs standalone).

2. **In-memory ONLY — the wire/signing encoding is untouchable.** `Scope::to_tlv_bytes` is on the
   signed path (committed to by Ed25519 **and** ML-DSA-65). The **signature-stability KAT is the top
   invariant and the DoD spine**: a RED test proves the TLV signed bytes are byte-for-byte identical
   before/after. The bitmask is derived at parse time for *checks* and **never serialized**.

**Honesty constraint (carried forward, not softened):** this is **not** a hash-avoidance win. The
current structure is a `Vec`, not a `HashSet`, and typical scopes are single-pair, so the subset *scan*
is already ~1 compare. The gain is precisely **allocation elimination + `Copy` + branchless subset** —
stated as such, never as "O(1) `&` instead of O(hash)" (S4 §4c-2). A second honesty correction lands
here: `Scope`/`Effect` **cannot** themselves become `Copy` (they hold a `Vec` for signing) — so P94
introduces a *separate* `Copy` `ScopeMask` and routes the hot *check* path through it, deleting the
clones that existed *for checking*; the one clone that exists *for signing* (`roster.rs:129`) stays
(§4.2). The research's "make Scope/Effect Copy" shorthand is corrected to this accurate form.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every `file:line` below was read from source **this pass**
> (`/root/bebop-repo/bebop2/proto-cap`, HEAD, 2026-07-19), not inherited from the S4 research sketch.

### 0.1 The representation today: a heap `Vec` over a small closed domain

| Element | Cite | State |
|---|---|---|
| `struct Scope { grants: Vec<(Resource, Action)> }` | `scope.rs:128` | heap-allocated collection; `Serialize, Deserialize, Clone, PartialEq, Eq` — **not `Copy`**. |
| `struct Effect { grants: Vec<(Resource, Action)> }` (identical shape) | `roster.rs:61` | same. |
| `Scope::is_subset_of` | `scope.rs:167` | `self.grants.iter().all(|p| super.grants.contains(p))` — **O(n·m) linear `Vec` scan** (`Vec::contains` is linear). |
| `Effect::is_subset_of` | `roster.rs:84` | identical linear scan. |
| `Scope::to_tlv_bytes` | `scope.rs:153` | the **signed** serialize form — `len(u16 LE) ‖ (res_u8, act_u8)*` ordered-pair list. **Untouchable** (§0.2). |
| `enum Resource` | `scope.rs:18` | **17 variants** (counted live), pinned discriminants `0x01..0x11`. Headroom ≤ 32. |
| `enum Action` | `scope.rs:66` | **24 variants** (counted live), pinned `0x01..0x18`. Headroom ≤ 32. |
| pair space | — | **17 × 24 = 408 today; ≤ 32 × 32 = 1024 with headroom** → exceeds 64 bits, so **not** a single `u64` → a fixed `[u32; 18]` (or `[u64; 16]`) array. |

### 0.2 The signed path the refactor MUST NOT perturb

`Scope::to_tlv_bytes` (`scope.rs:153`) feeds the canonical, domain-separated, length-prefixed TLV signing
input (`capability.rs` header "fixed-layout encoding on the signed path; serde_json is
non-canonical and was the exact §4A defect"; the red-team confirms the TLV form is the **FIXED** canonical
signed representation — row 6). Both `verify_chain`'s per-link signature (`roster.rs:129`
`Scope::new(self.effect.grants.clone()).to_tlv_bytes()`) and the capability signature commit to these
bytes. **Changing them is a breaking wire + signature change** — categorically out of P94's scope
(D-94-A). The bitmask is a decision/compute-layer representation only.

### 0.3 The hot path — where the clones and scans actually are (the avoidable work)

`verify_chain` (`roster.rs:252-316`), reached on every recv, does per-link and per-frame scope work:

| Site | Cite | Cost |
|---|---|---|
| `link.effect.is_subset_of(&link.scope)` | `roster.rs:277` | linear scan, **per link** |
| `link.scope.is_subset_of(&ps)` (attenuation) | `roster.rs:289` | linear scan, **per link** |
| `parent_scope = Some(link.scope.clone())` | `roster.rs:295` | **heap clone per link** (for the next-link check) |
| `Effect::new(cap.scope.grants.clone())` | `roster.rs:305` | **heap clone per frame** (to build `requested`) |
| `requested.is_subset_of(&tail.scope)` | `roster.rs:306` | the `scope ⊆ cap` check — **the per-frame survivor P92 makes hot** |
| `Scope::new(self.effect.grants.clone()).to_tlv_bytes()` | `roster.rs:129` | clone + fresh `Vec` — **but this is on the SIGNING path (§4.2), the one clone that STAYS** |

`RedLineGate::check` (`redline.rs:97-127`) **allocates a fresh `Vec` (`red_pairs`) every call**
(`:99-104`) then nested-linear-scans; `is_red_line` (`redline.rs:70`) is the per-pair predicate the
precomputed mask replaces. Other linear scope call sites: `facade.rs:144` (`allowed_reads.contains(&scope)`
— whole-`Scope` equality over a `Vec<Scope>`), `port.rs:100` (`port_scope.is_subset_of(fs)`).

### 0.4 No bitmask packing exists in the layer today (this would be the first)

`grep -rE 'bitflags|#\[repr\(|bitvec|fixedbitset'` over both `src` trees returns nothing on the non-test
path (S4 §1); `proto-cap` deps are only `bebop2-core` + `serde`. P94 **adds no dependency** — `[u32; 18]`
+ word-AND is std-only, in-tree (§2.2, standard §2 item 19).

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P94 uses it — and what it does NOT take |
|---|---|---|
| **Fixed-domain permission bitmask (POSIX mode bits, Linux capability sets `kernel_cap_t`)** | a closed, small permission domain packed into fixed words; subset/test = bitwise AND | **Adopt the shape** — `Resource × Action` is a closed ≤1024-pair domain, exactly the precondition. **NOT taken:** a growable/dynamic capability model — the enums are closed by design (`scope.rs:16` "closed set so the gate is total"), so the domain is fixed, not open. |
| **Two-representation discipline (serde-transport vs hand-built-TLV-signing, already in this crate)** | one representation for the wire/signature, a different derived one for compute | **Adopt verbatim** — the TLV ordered-pair list stays the canonical signed form; the `ScopeMask` is the *derived compute* form, parsed once and never serialized. This is the same split `capability.rs`/`signed_frame.rs` already use (serde for framing vs TLV for signing). |
| **Precomputed constant mask (compile-time policy table)** | a policy expressed once as a `const` and checked by AND, not recomputed per call | **Adopt** for the red-line: `const RED_LINE: [u32; 18]` built from `is_red_line` over the full 17×24 matrix, replacing the per-call `red_pairs` `Vec` alloc (`redline.rs:99-104`). |
| **The session's settled "packed small-enum state" pattern (P86–P89 thunderdome/RGB fusion, 2-bit ping-pong masks)** | pack a small closed state into words; branchless transitions | **Compose with it** — same family, applied to the authz permission set. **NOT taken:** sub-byte packing of `bool`/`Result`/2–4-variant enums — S4 §6 proved those already at the 1-byte floor; P94 touches only the `Vec`-collection candidate, nothing else in the layer. |
| **`HashSet`/`bitvec`/`fixedbitset` crates** | general-purpose set/bitset containers | **REJECTED** — a `[u32; 18]` `Copy` array is leaner than any heap container, needs no dependency, and the domain is fixed so no dynamic sizing is needed. Adding a crate here would be over-engineering (ponytail). |

---

## 2. Scope — what P94 OWNS vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P94 OWNS

1. **`ScopeMask([u32; 18])`** (`Copy`) — a new in-memory type derived from `Scope`/`Effect` at parse/verify
   time (M1).
2. **Branchless subset + red-line + read-gate checks** routed through the mask (M2/M3/M4), deleting the
   *check-path* clones at `roster.rs:295,305` (M2).
3. **The precomputed `const RED_LINE: [u32; 18]`** replacing the per-call `red_pairs` allocation (M3).
4. **The equivalence + signature-stability verification plan** (§5) — the DoD spine of a
   correctness-preserving refactor.

### 2.2 P94 does NOT own (anti-scope — the invariants that make this safe)

- **The wire / signing encoding.** `Scope::to_tlv_bytes` (`scope.rs:153`), the TLV field layout, and every
  signed byte are **untouched** (D-94-A). The mask is never serialized, never signed, never sent.
- **Canonicalization / pair-sorting.** The research noted an *ideal* canonical form would sort pairs before
  encoding (so equal sets have identical signed bytes). **P94 explicitly does NOT do this** — sorting would
  change the signed bytes and break every existing signature. If the current `to_tlv_bytes` is unsorted,
  P94 leaves it unsorted; the mask (order-insensitive) is used only for *checks*, and the order-sensitive
  `Vec` remains the signing form. Canonical-sorting is a **separate** change with its own P93-style
  versioned-signature migration cost — flagged, not bundled (§5.4).
- **Making `Scope`/`Effect` themselves `Copy`.** They hold a `Vec` for signing and cannot be `Copy`; P94
  adds the *separate* `Copy` `ScopeMask` and routes checks through it (VERDICT honesty correction, §4.2).
- **`bebop2-core`, the delegation lattice logic, revocation, the red-line *policy*** (which pairs are
  red-line — that stays `is_red_line`, `redline.rs:70`; P94 only precomputes its mask). No new dep, no core
  change.
- **P92 / P93.** P94 is value-dependent on P92 (sequence after) but edits disjoint files (`scope.rs`,
  `roster.rs`, `redline.rs`, a new `scope_mask.rs`), never `fastpath.rs` or the P93 signing-domain surface.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `Scope`/`Effect`/`Resource`/`Action` (`scope.rs`, `roster.rs`); their pinned
discriminants (`scope.rs:172-194`, `:224-251`); `is_red_line` (`redline.rs:70`); the call sites
`roster.rs:{277,289,306}`, `redline.rs:{99-127}`, `facade.rs:144`, `port.rs:100`.

**Value-dependency:** **P92** (SYNTHESIS §2.3) — worth building because the fast-path makes `effect ⊆
session_cap` the per-frame hot op. On the full path alone it is an opportunistic clean-up. **Sequence after
P92.**

**Consumers:** `verify_chain` (`roster.rs:252`), `RedLineGate::check` (`redline.rs:97`), the facade
read-gate (`facade.rs:144`), `check_port_scope` (`port.rs:100`), and — post-P92 — the fast-path's per-frame
`effect ⊆ session_cap` check, which precomputes the session scope as a `ScopeMask` once and word-ANDs per
frame.

### 2.4 Honest reconciliation (standard §2 item 6)

P94 changes **representation, not semantics**. The entire risk is a divergence between the mask and the
`Vec` — a money/auth authorization hazard if the mask ever accepts a subset the `Vec` would reject (or
vice-versa). The whole verification plan (§5) exists to make that divergence a machine-checkable
impossibility across the full 17×24 matrix. Nothing about *who may act* changes; only *how the same
decision is computed*.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

All new machinery lives in a **new module `proto-cap/src/scope_mask.rs`** (keeps `scope.rs`/`roster.rs`
from growing a new responsibility; imports `Resource`/`Action`/`Scope`/`Effect`). **Nothing here is ever
serialized or signed.**

```rust
// proto-cap/src/scope_mask.rs  (NEW) — IN-MEMORY ONLY. Never serialized, never signed, never sent.

/// A Copy, 72-byte in-memory representation of a Scope/Effect's (Resource, Action) set.
/// rows[res_disc - 1] holds a u32 whose bit (act_disc - 1) is set iff (resource, action) is granted.
/// Derived from a Scope/Effect at parse/verify time; the Vec + to_tlv_bytes remain the signed form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeMask {
    rows: [u32; 18],   // 17 resources used (0x01..0x11) + 1 slot headroom; 24 actions used per u32
}

impl ScopeMask {
    /// Derive from the canonical Vec. Total: unknown/out-of-range discriminants are impossible
    /// (closed enums) but the setter is written fail-closed (a would-be out-of-range pair panics in
    /// debug, is dropped in release — asserted never to happen by the closed-enum invariant test).
    pub fn from_scope(s: &Scope) -> Self { /* set bit per pair */ unimplemented!() }
    pub fn from_effect(e: &Effect) -> Self { /* identical */ unimplemented!() }

    /// Branchless subset: self ⊆ sup  iff  every set bit of self is set in sup.
    /// 18 word-ANDs, zero allocation, no Vec::contains inner loop.
    #[inline]
    pub fn is_subset_of(&self, sup: &ScopeMask) -> bool {
        self.rows.iter().zip(sup.rows.iter()).all(|(a, b)| a & !b == 0)
    }

    /// Red-line intersection against the precomputed constant mask — no per-call Vec alloc.
    #[inline]
    pub fn intersects_red_line(&self) -> bool {
        self.rows.iter().zip(RED_LINE.iter()).any(|(s, m)| s & m != 0)
    }

    /// Single-pair membership (for the facade read-gate / port check equivalence).
    #[inline]
    pub fn contains_pair(&self, r: Resource, a: Action) -> bool { /* rows[..] & (1<<..) */ unimplemented!() }
}

/// Precomputed at const-eval from `is_red_line` over the FULL 17×24 matrix. The single source of the
/// red-line set as a mask; a divergence between this and `is_red_line` is a money/auth hazard, so it
/// is proven identical by `prop_red_line_mask_equiv` (§5.2). Kept `const` so the compiler builds it
/// once and the check is a pure word-AND.
pub const RED_LINE: [u32; 18] = build_red_line_mask();   // const fn folding is_red_line over all pairs

/// Discriminant→index helpers (1-based pinned discriminants → 0-based array/bit index).
#[inline] const fn res_index(r: Resource) -> usize { (r.discriminant() as usize) - 1 }   // 0x01→0 .. 0x11→16
#[inline] const fn act_bit(a: Action) -> u32       { 1u32 << ((a.discriminant() as u32) - 1) } // 0x01→bit0
```

**Sizing rationale (locked):** packing **(A) `[u32; 18]` per-resource** is chosen over **(B) flat `[u64;
16]`** (D-94-B): (A) is leaner (72 B vs 128 B) and maps 1:1 to the `Resource`/`Action` structure (S4 §4a).
`18` = 17 used resource rows + 1 headroom slot (keeps 1-based-discriminant indexing safe and leaves room
for the documented ≤32 resource headroom without a re-size until a 19th resource lands). Each `u32` holds
24 used action bits of 32 (≤32 action headroom).

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first, a test that goes RED before the change, code, then GREEN.**

### 4.1 M1 — `ScopeMask` type + derivation + the signature-stability KAT (the guardrail lands FIRST)

- **Spec:** implement `ScopeMask`, `from_scope`/`from_effect`, and the discriminant→index helpers. **Before
  any call site is rewired**, land the **signature-stability KAT**: a fixed corpus of `Scope`s whose
  `to_tlv_bytes()` output is asserted byte-for-byte against a golden vector — this test must be GREEN and
  must **stay** GREEN through every subsequent M (it is the tripwire that catches any accidental touch of
  the signed path).
- **RED `red_scope_mask_module_absent` → GREEN:** the module compiles and derives a mask from a known scope.
- **KAT `kat_tlv_bytes_unchanged` (the top invariant):** `Scope::to_tlv_bytes()` for a golden corpus equals
  a pinned byte vector captured from HEAD **before** P94. If any later M perturbs the signed path, this goes
  RED. This is the DoD spine (D1).
- **Adversarial `red_mask_ignores_pair_order`:** `from_scope` of `[(A,B),(C,D)]` == `from_scope` of
  `[(C,D),(A,B)]` (the mask is a *set*, order-insensitive) — while the corresponding `to_tlv_bytes` may
  differ by order (the Vec is order-sensitive on the signed path). Proves the mask is used for set-checks
  only, and that P94 does **not** conflate mask-equality with signing-equality (§5.4).

### 4.2 M2 — route `verify_chain` subset checks through the mask; delete the check-path clones

- **Spec:** in `verify_chain` (`roster.rs:252-316`), compute each link's `ScopeMask`/`EffectMask` once and
  replace the three linear subset checks (`:277`, `:289`, `:306`) with `ScopeMask::is_subset_of`. Replace
  the *check-path* clones `parent_scope = Some(link.scope.clone())` (`:295`) and `Effect::new(
  cap.scope.grants.clone())` (`:305`) with the `Copy` masks — **eliminating those two heap ops per
  link/frame.** The clone at `:129` (`...clone()).to_tlv_bytes()`) is on the **signing** path and
  **STAYS** (the honest correction: `Scope`/`Effect` cannot be `Copy`; only the checks move to masks).
- **RED `red_verify_chain_mask_equiv`:** for a battery of delegation chains (including the G4 attenuation
  fixtures at `roster.rs:557-630`), `verify_chain` returns the **identical** `Result` (Ok/Err and error
  variant) before and after the mask rewire. RED if any decision differs; GREEN when byte-identical.
- **RED `red_attenuation_still_narrows`:** the exact G4 scenario the code comment warns about
  (`roster.rs:51-55`: flat-equality once let attenuation narrow *nothing*) — a parent granting `{X,Y}`
  delegating a child `{X}` must still be a proper subset under the mask. Proves the mask preserves the real
  set-subset semantics, not a regression to flat equality.
- **Adversarial `red_superset_effect_rejected`:** an effect claiming a pair its scope lacks →
  `is_subset_of` false under the mask exactly as under the `Vec` (a would-be privilege escalation stays
  rejected). This is the money/auth-critical direction.

### 4.3 M3 — precomputed `RED_LINE` mask; delete the `red_pairs` per-call allocation

- **Spec:** replace `RedLineGate::check`'s per-call `red_pairs` `Vec` build (`redline.rs:99-104`) + nested
  scans with `ScopeMask::from_scope(scope).intersects_red_line()` against the `const RED_LINE`. The red-line
  *policy* (`is_red_line`, `redline.rs:70`) is unchanged — `RED_LINE` is *derived from it* at const-eval and
  proven identical (§5.2).
- **RED `red_red_line_mask_equiv`:** for **every** one of the 408 `(Resource, Action)` pairs (and random
  multi-pair scopes), `intersects_red_line()` == `RedLineGate::check(scope, DenyByDefault).is_err()`. RED if
  any pair diverges. This is the money/auth-hazard tripwire (D3).
- **RED `red_red_line_denies_settlement`:** the concrete cases the current tests pin (`redline.rs:145-149`:
  `Ledger/SettlementRecorded`, `Order/CreateOrder` are red-line) stay denied under the mask.
- **Adversarial `red_red_line_no_false_negative`:** an exhaustive assertion that **no** red-line pair is
  missing from `RED_LINE` (a false negative here would silently authorize a money/auth op). The const-build
  is folded from the *same* `is_red_line`, so the mask cannot omit a pair the predicate flags — the test
  proves it.

### 4.4 M4 — facade read-gate + port check equivalence (the remaining linear scope sites)

- **Spec:** route `facade.rs:144` (`allowed_reads.contains(&scope)`) and `port.rs:100`
  (`port_scope.is_subset_of(fs)`) through mask ops where the scopes are hot. The `facade` read-gate is
  whole-`Scope` equality over a `Vec<Scope>`; the mask makes each comparison a word-compare. (If a given
  site is not on a measured hot path, it may stay `Vec`-based — mask adoption there is opportunistic, not
  mandatory; §5.3 measures.)
- **RED `red_facade_read_gate_equiv`:** the facade read-projection decision (`facade.rs:144`, tested at
  `:491`) is identical before/after. GREEN when byte-identical.
- **RED `red_port_scope_equiv`:** `check_port_scope` (`port.rs:100`) returns the identical result under the
  mask.

### 4.5 M5 — the post-P92 fast-path enabler (why P94 is sequenced here)

- **Spec (activates once P92 lands):** at fast-path session establishment, precompute the session
  capability's `ScopeMask` once; per frame, `frame_effect_mask.is_subset_of(&session_scope_mask)` — a
  handful of word-ANDs, **zero allocation, branchless** — vs today's `Effect::new(cap.scope.grants.clone())`
  + linear subset (`roster.rs:305-306`). This is the compounding per-frame saving exactly where P92 wants
  throughput.
- **RED `red_fastpath_effect_subset_equiv`:** the per-frame `effect ⊆ session_cap` decision under the mask
  equals the `Vec`-based decision. (Landed with/after P92; if P92 is not yet built, this M is specified but
  dormant — P94 still delivers M2–M4 standalone.)
- **Note:** M5 is the *reason for the sequencing*, not a functional dependency — M1–M4 build and pass with
  or without P92.

---

## 5. The verification plan — proving representation-equivalence (the DoD spine, standard §2 items 2, 5)

Because this is a **correctness-preserving refactor**, the verification plan is not an afterthought — it
*is* the deliverable. It has four legs, each a property/KAT, each machine-checkable.

### 5.1 Subset equivalence across the full domain (the core proof)

A property test (proptest-style, or exhaustive over a bounded generator) over **arbitrary** `Scope`/`Effect`
pairs drawn from the 17×24 = 408-pair space:

```
prop_subset_equiv:  ∀ A, B ⊆ (Resource × Action) :
    ScopeMask::from(A).is_subset_of(&ScopeMask::from(B))  ==  A.is_subset_of(&B)   // existing Vec impl
```

- Includes the **boundary cases**: empty scope (⊆ everything), full 408-pair scope (superset of all),
  single-pair (the common case), and the G4 attenuation fixtures (`roster.rs:557-630`).
- Includes both directions explicitly: a proper subset must be accepted; a superset **must be rejected**
  (the escalation-critical direction, M2 `red_superset_effect_rejected`).
- **Exhaustive variant for confidence:** since the domain is only 408 pairs, the *single-pair-vs-single-pair*
  and *single-pair-vs-full* cases can be checked **exhaustively** (408² is trivial), not just sampled — the
  research's recommended "full 17×24 pair matrix" (S4 §7). Multi-pair combinations are proptest-sampled.

### 5.2 Red-line equivalence across every pair (the money/auth tripwire)

```
prop_red_line_mask_equiv:  ∀ scope :
    ScopeMask::from(scope).intersects_red_line()  ==  RedLineGate::check(scope, DenyByDefault).is_err()
```

Checked **exhaustively over all 408 single pairs** plus proptest-sampled multi-pair scopes. A divergence
here is a money/auth authorization hazard (a red-line op silently allowed), so this leg is exhaustive on the
pair matrix, not sampled. The `const RED_LINE` is folded from the **same** `is_red_line` predicate, so
identity is structural; the test proves the fold is faithful.

### 5.3 Benchmark before/after (proving the gain is real, and where — standard §2 item 10)

| Bench | Measures | Expectation (honest) |
|---|---|---|
| `bench_verify_chain_vec_vs_mask` | `verify_chain` cost per frame, `Vec` vs mask | on the full path: **modest** (crypto dominates — S4 §5); the win is allocation count, not wall-clock headline |
| `bench_redline_check_vec_vs_mask` | `RedLineGate::check` per call | eliminates the `red_pairs` `Vec` alloc — measurable allocator-pressure drop |
| `bench_fastpath_effect_subset` | per-frame `effect ⊆ session_cap`, `Vec` vs mask (post-P92) | **the real win** — word-AND vs linear scan + clone, where P92 makes it hot |
| allocator counter (dhat/heaptrack) | heap allocations per verified frame | **L+1 fewer** per L-length chain (the clones at `:295,:305`) + 1 per red-line check |

Telemetry: emit per-verify `{alloc_count, subset_check_ns}` through the existing metrics seam so an
accidental reintroduction of a clone/scan surfaces automatically (item 14). **No measure-first NO-GO gate**
— unlike P92, P94 is not a bet; the benches quantify a known-direction win and confirm the full-path gain is
modest-not-negative (a refactor that *slowed* the path would be RED).

### 5.4 Signature-stability KAT (the top invariant — restated as its own leg)

```
kat_tlv_bytes_unchanged:  Scope::to_tlv_bytes()  ==  <golden bytes captured from HEAD before P94>
```

Any perturbation of `to_tlv_bytes`, the TLV field layout, or pair ordering flips this RED. Because P94 must
touch **zero** bytes on the signed path, this KAT is the single most important test and is asserted after
**every** M (D1). **Corollary (the canonicalization non-goal):** P94 does **not** introduce pair-sorting —
sorting would change these golden bytes and break every existing signature. Canonical-sorting is a separate,
P93-style versioned-signature migration, explicitly out of scope (§2.2).

---

## 6. Adversarial self-check — real effort to break the design (item 3 — the heart)

- **Can the mask accept a subset the `Vec` rejects (privilege escalation)?** No — `prop_subset_equiv`
  (exhaustive on single pairs, sampled on multi) proves identity in both directions; the escalation
  direction (`red_superset_effect_rejected`) is called out separately.
- **Can the mask miss a red-line pair (silent money/auth authorization)?** No — `RED_LINE` is folded from
  the *same* `is_red_line`; `prop_red_line_mask_equiv` is exhaustive over all 408 pairs.
- **Can a discriminant exceed the array/bit range (memory unsafety / silent drop)?** No — the enums are
  closed (17/24 variants, pinned) and the derivation asserts range in debug; a `red_discriminant_in_range`
  test pins that no `Resource`/`Action` discriminant exceeds 18/32. If a 19th resource or 33rd action is
  ever added, this test goes RED and forces a resize (the smart index, item 14).
- **Can the refactor silently change a signature?** No — `kat_tlv_bytes_unchanged` is the tripwire; the mask
  is never serialized (grep-assert: `ScopeMask` appears in no `Serialize`/`to_tlv`/signing path).
- **Can two equal *sets* in different pair *order* be treated as signing-equal by mistake?** No —
  `red_mask_ignores_pair_order` proves the mask is set-equal while the signed `Vec` stays order-sensitive;
  P94 never uses mask-equality to decide signing (§5.4).
- **Honest residual:** the multi-pair subset space (2^408) cannot be checked exhaustively — it is
  proptest-sampled. Mitigation: the *single-pair* matrix (the ~99% real case, S4 §3 — almost every scope is
  single-pair) **is** exhaustive, and the structural fold (mask derived from the same enums/predicate) makes
  a multi-pair divergence require a per-pair divergence, which the exhaustive single-pair test already
  excludes. Stated, not hidden.

---

## 7. Independent review gate — LIGHTER than P92/P93 (standard §2 items 5, 6)

P94 introduces **no new crypto, no new wire format, no new authority surface** — so it does **not** require
the full B4/SSR-2020 forgery-building gate P92/P93 mandate. It requires a **focused correctness review**:

- **Reviewer independence:** an actor not the implementer confirms the two equivalence properties (§5.1,
  §5.2) are (a) actually exhaustive where claimed, and (b) test the escalation/false-negative directions,
  not just the happy path — the exact failure mode of a "green but wrong" equivalence test.
- **The one hard check:** the reviewer independently re-derives `RED_LINE` from `is_red_line` and diffs it
  against the `const`, by hand or by a second implementation — because a wrong red-line mask is the only
  money/auth hazard here, and a bug in the `const fn` fold would pass a self-referential test.
- **Gate outcome:** PASS = the equivalence proofs are genuinely exhaustive on the pair matrix and the
  red-line mask is independently confirmed; FAIL = any equivalence leg is sampled where it claims
  exhaustive, or the red-line mask diverges → RED.

---

## 8. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | **the TLV signed bytes are byte-for-byte unchanged** (the top invariant) | `kat_tlv_bytes_unchanged` (M1, asserted after every M) — **the signature-stability spine** |
| D2 | `ScopeMask` subset is provably equivalent to the `Vec` subset across the full matrix | `prop_subset_equiv` (exhaustive single-pair, sampled multi), `red_verify_chain_mask_equiv`, `red_attenuation_still_narrows`, `red_superset_effect_rejected` (M1/M2) |
| D3 | the precomputed `RED_LINE` mask is provably identical to `is_red_line` over every pair | `prop_red_line_mask_equiv` (exhaustive over 408 pairs), `red_red_line_denies_settlement`, `red_red_line_no_false_negative` (M3) — **money/auth tripwire** |
| D4 | the check-path clones at `roster.rs:295,305` and the `red_pairs` alloc are deleted; the signing clone at `:129` stays | allocator-counter bench (§5.3) shows L+1 fewer heap ops per L-chain + 1 per red-line check; `kat_tlv_bytes_unchanged` proves `:129` untouched |
| D5 | facade read-gate + port check decisions unchanged under the mask | `red_facade_read_gate_equiv`, `red_port_scope_equiv` (M4) |
| D6 | discriminants provably fit the `[u32; 18]` domain; a future overflow forces a resize | `red_discriminant_in_range` (M1) |
| D7 | (post-P92) the fast-path per-frame `effect ⊆ session_cap` decision is mask-equivalent | `red_fastpath_effect_subset_equiv` (M5, dormant until P92) |
| D-REVIEW | the focused correctness review (§7) confirms exhaustiveness + the red-line mask | §7 attestation; FAIL ⇒ RED |
| D-BUILD | `proto-cap` builds & full `cargo test` green incl. all new REDs now GREEN, **no dep added** | `cargo test -p bebop-proto-cap`; `grep` proves no new Cargo dep |
| D-NOREG | every existing scope/roster/redline/facade test stays green; no wire/serde test changes | existing `scope.rs`/`roster.rs`/`redline.rs`/`facade.rs` suites |

---

## 9. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe state — *the mask authorizes something the `Vec` would
  deny* (or misses a red-line) — is made **unrepresentable-by-proof**: the equivalence properties (§5.1/§5.2)
  are exhaustive on the single-pair matrix and the red-line mask is a structural fold of the same predicate.
  The refactor cannot *reach* a divergent decision without a per-pair divergence the exhaustive test
  excludes. Argued from the closed-enum structure + the fold, not a prose assurance.
- **Schemas & scaling axis (item 8):** the mask is **fixed-size** (`[u32; 18]`, 72 B, `Copy`) — it does not
  scale with scope cardinality (a 1-pair and a 408-pair scope are the same 72 B). It changes shape only if
  `Resource` exceeds 32 variants (→ widen rows) or `Action` exceeds 32 (→ `[u64; 18]`) — pinned by
  `red_discriminant_in_range`, not timeless.
- **Linux discipline (item 9):** **EXTENDS** the settled packed-small-enum pattern (P86–P89) to the authz
  set; **REINFORCES** the two-representation discipline (signed TLV vs compute form); **DOES-NOT-TRANSFER**
  — no daemon, no dynamic container, no new dep. **ALREADY-EQUIVALENT** on the closed-domain precondition
  (the enums are already closed and pinned).
- **Isolation / bulkhead (item 11):** the mask is a **pure derived value** — it owns no state, touches no
  I/O, and cannot fail at runtime (derivation is total over closed enums). Its "failure mode" is a
  compile/test failure (a divergence caught by §5), never a runtime authorization surprise. It cannot
  corrupt the signed path because it never touches it (D1).
- **Mesh awareness (item 12):** **N/A for the wire** — the mask is node-local compute state, never gossiped,
  never sent (the `Vec`/TLV form is what travels). Explicitly stated so no worker serializes a mask.
- **Rollback / self-healing as math (item 13):** **Snapshot re-entry** = the mask is *always* re-derivable
  from the `Vec` in O(pairs) — there is no persistent mask state to lose or corrupt; drop it and rebuild.
  **Self-termination / self-healing are NOT claimed** (a pure function has no lifecycle) — stated, not
  loosely borrowed.
- **Error-propagation / smart index (item 14):** the bug classes P94 could introduce — a mask/`Vec`
  divergence, a red-line false negative, a discriminant overflow, an accidental touch of the signed path —
  are each turned into a **CI-time failure**: `prop_subset_equiv`, `prop_red_line_mask_equiv`,
  `red_discriminant_in_range`, `kat_tlv_bytes_unchanged`. Not runtime surprises.
- **Living-memory awareness (item 15):** **N/A** — the mask is stateless derived compute with no temporal or
  topological access pattern (contrast P93's window, which is living memory). Stated honestly.
- **Tensor/spectral (item 16):** **N/A, honestly** — a fixed permission bitmask is a bitwise operation, not a
  linear-algebra kernel; forcing `spectral.rs` here would be over-engineering (ponytail).

---

## 10. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** the mask *corresponds exactly* to the `Vec` set — proven, not
  asserted (§5). Two representations of one truth; the compute form mirrors the signed form.
- **Polarity / no-middle:** a pair is either in the set (bit set) or not (bit clear) — there is no ambiguous
  partial-membership; the subset check is a total word-AND with no degraded/partial accept.
- **Cause & Effect:** the mask is *caused by* the `Vec` (derived at parse time), never an independent source
  of authority — the signed `Vec` remains the sole cause of what is authorized; the mask only computes the
  same effect faster.

---

## 11. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (Scope/Effect/is_subset/to_tlv/discriminant cites re-verified; 17×24=408) |
| 2 | Falsifiable DoD | §8 (D1–D-NOREG); §5 (the equivalence/KAT spine) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; the KAT lands before any rewire) |
| 4 | Predefined types & constants | §3 (`ScopeMask`/`RED_LINE`/index helpers named before impl) |
| 5 | Adversarial/breaking tests | §4 (per-M adversarial cases), §5 (equivalence proofs), §6 (self-attack) |
| 6 | Hazard-safety from type structure | §9 (divergence unrepresentable-by-proof), §6 |
| 7 | Links to docs & memory | §12 |
| 8 | Schemas with scaling axis | §9 (fixed 72 B; resize only past 32 variants) |
| 9 | Linux engineering discipline | §9 (EXTENDS/REINFORCES/DOES-NOT-TRANSFER/ALREADY-EQUIVALENT) |
| 10 | Benchmarks + telemetry | §5.3 (before/after + allocator counter + alloc telemetry; honest "modest on full path") |
| 11 | Isolation / bulkhead | §9 (pure derived value; failure = compile/test, never runtime) |
| 12 | Mesh awareness | §9 (N/A for wire — mask never sent; stated to prevent serialization) |
| 13 | Rollback/self-heal as math | §9 (snapshot re-entry = always re-derivable; others NOT claimed) |
| 14 | Error-propagation / smart index | §9 (equivalence + red-line + discriminant + signature-stability CI gates) |
| 15 | Living-memory awareness | §9 (N/A — stateless; stated honestly) |
| 16 | Tensor/spectral where applicable | §9 (N/A, stated honestly) |
| 17 | Regression tracking | §8 D1/D3 (signature-stability + red-line equivalence enter the REGRESSION-LEDGER) |
| 18 | Clear worker instructions | §12 |
| 19 | Reuse-first, upgrade-if-needed | §0.4 (no dep), §1 (adopt not invent), §2.2 (anti-scope) |
| 20 | Hermetic principles | §10 |

---

## 12. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `SYNTHESIS-MESH-MAJOR-REFACTOR-PLAN-2026-07-19.md` §5 (the scope stub), §2.3 (P94-after-P92 value
  dependency), §2.4 (three concerns / three units).
- `docs/research/OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN-2026-07-19.md` — the full inventory (§1), the hot-path
  (§2), the small-closed-domain precondition (§3), the two honest caveats (§4c), the P92 fast-path locus
  (§5), the blueprint stub (§7).
- `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` — the fast-path whose per-frame `effect ⊆
  session_cap` op P94 accelerates (M5, §2.3).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- MEMORY: `crypto-safe-first-pass-2026-07-14.md` (why the red-line direction gets independent confirmation,
  §7), `verified-by-math-2026-07-07.md` (the equivalence-proof-as-DoD discipline),
  `test-integrity-rules-2026-06-27.md` (money/RLS/PII red-lines — the red-line mask is exactly this class).

**Existing code this blueprint edits/extends (exact targets, bebop-repo — NOT dowiz):**
- **NEW** `bebop2/proto-cap/src/scope_mask.rs` — `ScopeMask`, `from_scope`/`from_effect`, `is_subset_of`,
  `intersects_red_line`, `contains_pair`, `const RED_LINE`, index helpers (§3). In-memory only.
- **EDIT** `bebop2/proto-cap/src/roster.rs` — `verify_chain` subset checks (`:277,:289,:306`) → mask;
  delete the check-path clones (`:295,:305`); **keep** the signing clone (`:129`).
- **EDIT** `bebop2/proto-cap/src/redline.rs` — `RedLineGate::check` (`:97-127`) → `intersects_red_line`
  against `const RED_LINE`; **keep `is_red_line` (`:70`) as the policy source** the mask is folded from.
- **EDIT** `bebop2/proto-cap/src/facade.rs` (`:144`) + `port.rs` (`:100`) — mask where hot (opportunistic).
- **DO NOT TOUCH** `scope.rs::to_tlv_bytes` (`:153`), the TLV field layout, any `Serialize`/signing path, or
  `bebop2-core`. `Scope`/`Effect` keep their `Vec` (they cannot be `Copy`).

**For the worker with zero session context — exact acceptance path:**
1. **Sequence AFTER P92** — P94's value is the fast-path per-frame subset (M5); building it earlier
   optimizes a crypto-dominated path (premature). M1–M4 still build standalone if P92 is delayed.
2. **Land the signature-stability KAT (`kat_tlv_bytes_unchanged`) FIRST** (M1), capture the golden bytes
   from HEAD, and keep it GREEN through every subsequent M — it is the tripwire for any accidental signed-path
   touch.
3. Write §3 types first (types → tests → code — item 3); implement M1→M4 (M5 with/after P92); each M's RED
   tests fail before its code and pass after.
4. Make the two equivalence proofs (§5.1 subset, §5.2 red-line) **exhaustive on the 408-pair matrix** where
   claimed — sampled-only equivalence is a FAIL (§7).
5. Add D1 (signature-stability) + D3 (red-line equivalence) to `docs/regressions/REGRESSION-LEDGER.md`.
6. `cargo test -p bebop-proto-cap` fully green, **no new Cargo dep**; all existing scope/roster/redline/facade
   tests (D-NOREG) stay green.
7. **Do NOT mark P94 done until §7's focused correctness review confirms** the equivalence proofs are
   genuinely exhaustive and the `RED_LINE` mask is independently re-derived. A green-but-sampled equivalence
   test is the one way this refactor ships a silent authorization bug.
8. Anti-scope: never serialize/sign a mask; never sort pairs (breaks signatures — §5.4); never make
   `Scope`/`Effect` `Copy`; keep `is_red_line` as the red-line policy source.
