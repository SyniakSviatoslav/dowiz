# BLUEPRINT P86 — SlotArena × GPU texture-channel lifecycle fusion (operator item A) (2026-07-19)

> **Standalone ENGINE/GPU-infrastructure blueprint (dowiz `kernel` + `engine`).** One coherent,
> independently buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md`
> §2. Scope source: `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §4.2 (operator direction A),
> §2 row 1/2 (divergence ledger). It designs the **first real consumer** of the already-landed
> `slot_arena.rs` — the exact "Future blueprint work" the module's own doc-comment parks (verified
> `kernel/src/slot_arena.rs:18-24` this pass). Grounding tree: `/root/dowiz` at HEAD, read live.
>
> **One sentence:** when the field engine's GPU state grows multi-channel (RGBA/RG32F packed physics
> fields + per-cell coefficient texels + complex `(re,im)`), give every *(texture, channel)* slot and
> every ping-pong *texture pair* a generational `Handle` from the existing `SlotArena`, so that a
> field freed and its channel re-leased to a different field can never be silently sampled through a
> stale handle (the ABA hazard) — GPU resource pools being the textbook home of generational indices.

---

## VERDICT (stated up front, per session discipline)

**GO AS A WRITTEN DESIGN NOW; BUILD IS GATED on the P38 §4.2 operator GPU decision.** Three honest
framings:

1. **The fit is genuine, not forced.** GPU resource pools are the canonical application of
   generational indices (wgpu-core itself keys resources by index+epoch internally), and R13's
   multi-channel field-state future is exactly "a pool of live entries with per-element removal and
   cross-references" — the first concrete instance of the trigger the arena deep-dive parked
   (`slot_arena.rs:18-21`: "an incremental mesh/graph index that deletes nodes while other structures
   still hold references"). P86 is that consumer being designed, by name.
2. **It builds nothing until the operator takes P38 §4.2.** Until multi-channel GPU field state
   exists, there is no `(texture, channel)` to lease. P86 is a written design + the `slot-arena`
   feature staying dormant at **zero default-build cost** (`a857cd71a` is byte-neutral to the default
   build). The bet costs nothing to lose: if the GPU field-state decision never comes, the feature
   sleeps and this blueprint is a parked design (§2.5).
3. **Research honesty (the task's demand).** The **primary GPU-side sources are disk-lost.** R13
   (`OPUS-PERF-RGB-GPU-TEXTURE-PACKING`), R16 (`OPUS-PINGPONG-SHADOW-COPY-PROPAGATION`), and R14
   (`OPUS-PHYSICS-WAVE-ALGORITHMS…`) are **not on disk and not in the recovered scratchpad** (verified
   this pass). P86's GPU-side design is therefore grounded **second-hand** through
   `SYNTHESIS-PHYSICS-PERFORMANCE-VISION §4.2/§0`, whose author read them in full — *not* through the
   primaries. The **arena side is first-hand and strong**: `OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` is
   on disk and `kernel/src/slot_arena.rs` was read live. This asymmetry is stated wherever it bears.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." The arena facts are read from live source
> (`kernel/src/slot_arena.rs`, HEAD). The GPU-texture facts are attributed to the synthesis (their
> primaries are disk-lost, §0.5) and never asserted as first-hand.

### 0.1 `SlotArena` exists and is exactly the substrate this design needs (first-hand)

`kernel/src/slot_arena.rs` (12 589 bytes, landed `a857cd71a` on dowiz `main`, local-only/unpushed):

| Element | Cite | Fact |
|---|---|---|
| `pub struct Handle(Index)` | `:73` | **8-byte `Copy`** handle (`u32` slot + `NonZeroU32` generation); `Option<Handle>` is *also* 8 bytes (niche-packed) — asserted in tests, not assumed (`:45-47`) |
| `pub struct SlotArena<T>(Arena<T>)` | `:82` | thin wrapper over `thunderdome::Arena`; dense backing `Vec`, cache-friendly live iteration |
| `insert(value) -> Handle` | `:122` | O(1), reuses a recycled slot if free; handle carries the slot's **current** generation |
| `get / get_mut(handle) -> Option<&T>` | `:128/:133` | `Some` iff slot in range + occupied + **generation matches**; stale = safe `None` |
| `contains(handle) -> bool` | `:138` | cheap membership; `false` for stale |
| `remove(handle) -> Option<T>` | `:145` | O(1); **bumps the slot's generation**, invalidating every outstanding copy of the handle |
| ABA defeated by construction | `:39-44` | on removal the generation bumps; a recycled slot's new handle carries the higher generation; the old handle stays `None` (wrap horizon ≈ 2³² reuses/slot) |
| degrade-closed | `:35-38` | every fallible op returns `Option`, **never panics** on a stale/out-of-range/removed handle |
| no `unsafe` in the wrapper | `:48-49` | the only `unsafe` is upstream in thunderdome's audited packing |
| opaque handle | `:29-31,63-66` | `Index` payload is private — call sites cannot forge/unpack; **the backing crate stays swappable** |

### 0.2 The module's own doc-comment names P86 as its consumer (first-hand — the anchor)

`slot_arena.rs:18-24`, verbatim: *"The first named trigger the deep-dive parked (§5.2) is **an
incremental mesh/graph index that deletes nodes while other structures still hold references to
them**: a plain `Vec` index … silently reads a recycled slot after a removal (ABA); a generational
handle instead becomes a safe `None`. **Future blueprint work builds against *this* API — never
`thunderdome::*` directly** — so the backing crate can later be swapped … without touching a single
call site."* — P86 **is** that future blueprint work; the "delete an entry while others hold
references" shape is exactly the texture-channel lease lifecycle (§4).

### 0.3 The feature gate (first-hand)

`kernel/Cargo.toml:77` → `slot-arena = ["dep:thunderdome"]`; the module is *"OFF by default — compiled
ONLY under the `slot-arena` feature, so the canonical order/money core pulls zero extra crates (same
opt-in discipline as `pq` / `gpu` / `pgrust`)"* (`slot_arena.rs:6-8`). P86's registry is compiled only
under `slot-arena` **and** the engine's `gpu` feature — double-gated, zero default cost.

### 0.4 The operator override that put the arena in the tree (first-hand, via on-disk deep-dive)

`OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` §6 (on disk) + `slot_arena.rs:10-16`: the deep-dive swept the
tree six ways, found **no current** consumer, and its own verdict was "(c) no adoption now." The
**operator explicitly overrode** that after hearing it — land thunderdome now as forward-looking
infrastructure, behind a feature. The divergence is logged
(`SYNTHESIS-PHYSICS-PERFORMANCE-VISION §2 row 1`, §7 E3′): *the analysis (no current consumer) stands;
the verdict is reversed at the operator layer*. P86 designs the consumer that discharges the override's
named exit: "P86 names the first real consumer (GPU texture/channel lifecycle)."

### 0.5 The GPU-texture facts — attributed, because their primaries are disk-lost (HONEST)

**Verified this pass:** `docs/research/OPUS-PERF-RGB-GPU-TEXTURE-PACKING-2026-07-18.md` (R13),
`OPUS-PINGPONG-SHADOW-COPY-PROPAGATION-2026-07-18.md` (R16), and
`OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS-2026-07-18.md` (R14) are **absent from disk** and
**absent from the scratchpad `recovered/` dir** (`MASTER-STATUS-LEDGER §0`: 11 research docs recovered,
these three not among them). What survives on disk is `OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md` (R9,
the **CPU** angle, **REJECTED** as E1 — a *different* doc, not P86's grounding). Therefore the following
are **synthesis-attributed**, not first-hand:

| Claim used by P86 | Attributed source | First-hand? |
|---|---|---|
| Multi-channel field state — `(vx,vy,p,ρ)` in `RGBA32F`, coefficient texels `(Γ,c²,M,S)`, complex `(re,im)` in `RG32F` | R13 via `SYNTHESIS §4.2` / §0-R13 | **No** — synthesis-attributed (R13 disk-lost) |
| Free hardware bilinear for sub-cell sampling; RGBA/RG32F packing is the only compute mechanism on the WebGL2 floor | R13 via `SYNTHESIS §0-R13` | **No** — synthesis-attributed |
| One ping-pong pair per **independently-evolved** texture; correlated scalars packed into channels share that pair's swap (shared stencil + cadence + stability ⇒ shared pair) | R16 via `SYNTHESIS §0-R16` / §4.2 | **No** — synthesis-attributed |
| CPU-angle RGBA packing REJECTED; GPU-compute angle is a *different* question with a conditional-positive answer | R9/E1 (`RGB-PACKING-REUSE`, on disk) + R13 | R9 **yes** (on disk); R13 **no** |

**Consequence:** P86's *arena mechanics* (the safety property, the API, the ABA defeat) are solid and
first-hand; P86's *GPU consumer shape* (which channels, which formats, the pair-sharing rule) inherits
whatever fidelity `SYNTHESIS §4.2` preserved from the lost primaries. Any build worker MUST re-derive
the channel/format specifics against the **then-current P38 GPU field-state design**, not treat this
blueprint's channel list as authoritative — it is illustrative of a lost source (§9).

### 0.6 The build gate — P38 §4.2 (first-hand)

`BLUEPRINT-P38-webgpu-render-engine.md`: GPU compute / field-state on the GPU is *"a named future
decision (§4.2), not this phase"* (`:90`), and *"GPU is presentation until a separately-gated decision
(§4.2)"* (`:182`). The FE-16 fallback ladder is WebGPU → WebGL2 → CPU `compose_field`. **P86's build
legs do not start until the operator takes P38 §4.2** (OD-11). The design is writable now; the code is
not.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P86 uses it — and what it does NOT take |
|---|---|---|
| **Generational-index resource pools (wgpu-core `Arena`/`Id`, slotmap, thunderdome)** | resources keyed by `index + epoch`; a stale id resolves to a miss, never a wrong resource | **Adopt via the existing `SlotArena`** (`:73-147`) for the CPU-side channel/pair registry. **NOT taken:** re-implementing a pool — the arena is already in-tree and byte-neutral by default. |
| **The landed `kernel/src/slot_arena.rs`** | dowiz's opaque-`Handle` wrapper over thunderdome | **This IS the substrate.** P86 stores `ChannelAlloc`/`PingPongPair` in `SlotArena<T>` and hands out its `Handle`. Zero new arena code. |
| **Ping-pong / double-buffer (shadow-then-promote)** | evolve texture B from A, swap; the codebase's native idiom at 8 sites (R16) | **Adopt the pair as a pooled resource.** **NOT taken:** the swap *algorithm* — P86 pools the *pairs* and invalidates dependent bind groups on generation mismatch; it does not touch the stencil step. |
| **R16's pair-sharing rule** | one pair per independently-evolved texture; correlated channels share the pair's swap | **Encode as a type distinction** (§3): a shared channel gets a `Handle` to an existing pair; an independently-evolved field allocates a new pair. The rule becomes an API shape, not a comment. |
| **FE-07 "engine consumes, never re-derives" bridge contract** | the engine receives resolved data; math authority is CPU/kernel | **Honor it:** handles are **CPU-side bookkeeping only** and never cross into WGSL (§4.3); shaders get concrete bindings resolved at encode time. |
| **`BumpArena` (`kernel/src/arena.rs`)** | phase/region bump allocator, `O(1)` reset, no per-element free | **NOT the tool here** (the arena's own §51-59 orientation note): channels/pairs come and go **individually**, so `SlotArena` (per-element, stable handles across removal) is the sibling to reach for, not `BumpArena`. Stated so the choice is deliberate. |

P86 **adds no dependency** (thunderdome is already the `slot-arena` dep) and **invents no primitive** —
it composes the landed arena with the GPU resource lifecycle.

---

## 2. Scope — what P86 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P86 OWNS

1. **A CPU-side `ChannelLease` registry** in the engine: `SlotArena<ChannelAlloc>` mapping each logical
   scalar field to a `(texture, channel)` slot with a generational `Handle` (§3, §4.1).
2. **A ping-pong pair pool**: `SlotArena<PingPongPair>` recycling expensive texture pairs, where a
   generation bump on release makes use-after-release unrepresentable and signals bind-group
   invalidation (§4.2).
3. **The R16 shared-vs-separate-pair rule as a type distinction** in the pair-allocation API (§3, §4.2).
4. **The "handles never cross into WGSL" boundary** — CPU bookkeeping only; determinism-preserving
   (§4.3).
5. **The safety tests** (stale channel lease → `None`; recycled-pair generation-bump invalidates old
   handles) and a lease/free churn microbench that **documents cost, does not justify existence** (§4.4).

### 2.2 P86 does NOT own (anti-scope)

- **The GPU field-state decision itself** (P38 §4.2 / OD-11) — operator-owned; P86 is gated behind it.
- **The stencil `step()` / ping-pong swap algorithm** — untouched; P86 pools the pairs, it does not
  evolve them (that is the field engine's job, FE-07).
- **The channel/format catalogue as authority** — the specific `(vx,vy,p,ρ)`/`(Γ,c²,M,S)`/`(re,im)`
  packings are R13-attributed (§0.5, disk-lost) and MUST be re-derived against the live P38 design at
  build time; P86 owns the *registry mechanism*, not the field taxonomy.
- **Any money/order/oracle path** — the registry is presentation-side GPU bookkeeping, hard-walled from
  the deterministic authority (the P38 honesty split); no `slot-arena` code touches `money.rs`/`compose()`.
- **The 2-bit mask plane** — that is P87 (which *leases a channel through P86's registry*, §2.3); P86
  provides the lease, P87 defines the mask semantics.
- **The atomicity policy** — P88 (which P86's shaders inherit, §2.3); P86 does not set the WGSL
  write-discipline.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `kernel/src/slot_arena.rs` (`SlotArena`/`Handle`, feature `slot-arena`);
the engine's `gpu` feature + P38's WebGPU/WebGL2 render path; the FE-07 bridge contract.
**Build gate:** P38 §4.2 (OD-11) — multi-channel GPU field state must exist to lease.
**Design inputs (attributed, disk-lost — §0.5):** R13 (channel/format taxonomy), R16 (pair-sharing rule).
**Consumers:** **P87** (its 2-bit mask plane leases a channel slot through this registry); **P88** (its
atomicity rule governs the shaders that read the leased channels); the field engine's multi-channel GPU
state (the thing being registered).

### 2.4 Honest reconciliation (standard §2 item 6)

The arena deep-dive's "no current consumer" analysis **stands** — P86 does not overturn it; it *is* the
consumer, designed forward. The E3 "REJECTED" verdict is superseded **at the operator layer only**
(`SYNTHESIS §2 row 1`); its analysis is preserved. P86 does not claim the arena was needed before this —
it claims the GPU field-state future is the first place it *will* be, and designs for it without pulling
it into the default build.

### 2.5 The bet's falsifiable exit (records the cost of being wrong)

`SYNTHESIS §2 row 1` names it: *"If P86's trigger never materializes, the feature stays dormant at zero
default-build cost — the bet costs nothing to lose."* Concretely: if the operator never takes P38 §4.2,
or takes it in a form where field state stays single-channel/CPU, then P86 remains a written design and
the `slot-arena` feature sleeps. No sunk build cost, no default-build regression. That is the whole
downside, stated up front.

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

All new types live in a **new module `engine/src/gpu/channel_registry.rs`** (double-gated
`#[cfg(all(feature = "slot-arena", feature = "gpu"))]`), importing `kernel::slot_arena::{SlotArena,
Handle}`. Nothing here crosses into WGSL (§4.3).

```rust
// engine/src/gpu/channel_registry.rs  (NEW; cfg(all(slot-arena, gpu)))

use kernel::slot_arena::{SlotArena, Handle};

/// An opaque GPU texture id (P38's resource handle — NOT re-derived here; P86 references it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureId(pub u32);

/// One of the 4 (RGBA) or 2 (RG) channels of a packed float texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Channel { R = 0, G = 1, B = 2, A = 3 }

/// The packed float format a texture carries. Values are R13-attributed (§0.5) — the BUILD worker
/// re-derives the live set against the then-current P38 field-state design; these are the lost
/// source's illustrative set, pinned so the type exists, not so the taxonomy is frozen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PackedFormat {
    Rgba32F = 0x0001,   // e.g. (vx, vy, p, ρ) — 4 correlated physics scalars sharing one pair (R16)
    Rg32F   = 0x0002,   // e.g. complex (re, im) — 2 correlated scalars sharing one pair
    R32Uint = 0x0003,   // packed integer plane (P87's 2-bit mask rides here, 16 cells/word)
    // append-only; unknown formats REJECTED (fail-closed), never best-effort.
}

/// A leased (texture, channel) slot for ONE logical scalar field. Stored in a SlotArena; the
/// generational Handle is what a field holds. Freeing + re-leasing to a new field is the ABA
/// hazard slot_arena defeats: a stale holder gets None, never someone else's scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelAlloc {
    pub texture: TextureId,
    pub channel: Channel,
    pub pair: PairHandle,        // the ping-pong pair this channel's texture belongs to (§4.2)
    pub format: PackedFormat,
}

/// A ping-pong pair as a POOLED resource. `front`/`back` are the two textures; `current` is the
/// swap-parity tag (which is front this frame — the small "which buffer" scalar, P87 §4.3 note).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PingPongPair {
    pub front: TextureId,
    pub back: TextureId,
    pub format: PackedFormat,
    pub current: u8,             // 0 = front is read-source, 1 = back is — the per-pair parity scalar
    pub stencil_id: StencilId,   // the evolution stencil; R16: same stencil+cadence+stability ⇒ share this pair
    pub cadence: u32,            // steps between swaps; part of the shared-pair identity
}

/// Opaque generational handles (the whole safety point — 8-byte Copy, stale ⇒ None).
pub type ChannelHandle = Handle;   // into SlotArena<ChannelAlloc>
pub type PairHandle    = Handle;   // into SlotArena<PingPongPair>

/// The evolution-stencil identity that decides pair sharing (R16 rule, made a value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StencilId(pub u32);

/// The registry: two pools + the shared-pair index. The scaling axis (§ item 8) is live channels
/// and live pairs per node — both O(1) state per element, dense-Vec-backed.
pub struct ChannelRegistry {
    channels: SlotArena<ChannelAlloc>,
    pairs: SlotArena<PingPongPair>,
    // shared-pair lookup: (StencilId, cadence, PackedFormat) -> existing PairHandle (R16 sharing rule)
    shared_pairs: hashbrown::HashMap<(StencilId, u32, PackedFormat), PairHandle>,
}

/// Named, non-magic bounds.
pub const MAX_CHANNELS_PER_TEXTURE: u8 = 4;   // RGBA; RG uses 2
pub const CHANNEL_REGISTRY_INITIAL_CAP: usize = 64;  // SlotArena::with_capacity seed (sizing telemetry)
```

**The R16 rule as a TYPE distinction (the single-owner contract, §item 8):** the pair-allocation API
exposes exactly two entry points, so sharing-vs-separating is a *compile-time choice at the call site*,
not a runtime flag:

```rust
impl ChannelRegistry {
    /// Lease a channel that SHARES an existing pair (correlated scalar: same stencil+cadence+format).
    /// Returns None if no matching pair exists — caller must `allocate_independent_field` first.
    pub fn lease_shared_channel(
        &mut self, stencil: StencilId, cadence: u32, fmt: PackedFormat, channel: Channel,
    ) -> Option<ChannelHandle>;

    /// Allocate a NEW independently-evolved field: a fresh pair + its first channel. This is the ONLY
    /// path that creates a pair, so "independently-evolved ⇒ own pair" is enforced structurally (R16).
    pub fn allocate_independent_field(
        &mut self, front: TextureId, back: TextureId, fmt: PackedFormat,
        stencil: StencilId, cadence: u32, channel: Channel,
    ) -> (PairHandle, ChannelHandle);

    /// Free a channel; if it was the pair's last channel, free the pair too (generation bump ⇒ ABA-safe).
    pub fn free_channel(&mut self, h: ChannelHandle) -> Option<ChannelAlloc>;

    /// Resolve a handle to a concrete binding at ENCODE time (the only read path shaders see, §4.3).
    pub fn resolve(&self, h: ChannelHandle) -> Option<&ChannelAlloc>;   // None = stale ⇒ skip/rebuild
}
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first, a test that goes RED before the change, code, then GREEN.** Lifecycle is
modeled as events; tests assert on the sequence (item 3). All items are `#[cfg(all(slot-arena, gpu))]`
and do not start until P38 §4.2 (OD-11).

### 4.1 M1 — `ChannelLease` registry: lease, resolve, free with generational safety

- **Spec:** `allocate_independent_field` inserts a `PingPongPair` into `pairs` (→ `PairHandle`) and a
  `ChannelAlloc` into `channels` (→ `ChannelHandle`); `resolve(h)` returns `Some(&ChannelAlloc)` iff the
  handle's generation matches (delegates to `SlotArena::get`, `slot_arena.rs:128`), else `None`.
  `free_channel` `remove`s the channel (generation bump) and, if it was the pair's last channel, frees
  the pair.
- **RED `red_stale_channel_lease_resolves_none`:** lease a channel → free it → `resolve(old_handle)`
  returns **`None`**, never a stale `ChannelAlloc`. RED before the registry exists; GREEN after. This is
  the ABA defense at the channel grain — the exact property `slot_arena.rs:39-44` guarantees, exercised
  through P86's API.
- **RED `red_release_to_new_field_no_bleed`:** lease channel `c1` for field A → free it → lease `c2`
  (recycles the slot) for field B → `resolve(c1_handle)` is `None` (higher generation), `resolve(c2)`
  is B's alloc. Proves a stale holder of A's handle **cannot** silently sample B's scalar riding the
  recycled `(texture, channel)`.
- **Adversarial `red_forged_handle_rejected`:** a `Handle` is opaque (`slot_arena.rs:29-31,63-66`) — a
  call site cannot construct one from raw parts; assert the type does not expose a public constructor, so
  a "made-up" handle is not even expressible (compile-time defense).

### 4.2 M2 — ping-pong pair pool + generation-bump bind-group invalidation + R16 sharing rule

- **Spec:** pairs live in `SlotArena<PingPongPair>`; `allocate_independent_field` is the **only** creator
  (enforcing "independently-evolved ⇒ own pair"); `lease_shared_channel` looks up `shared_pairs` by
  `(StencilId, cadence, PackedFormat)` and reuses the existing `PairHandle` (R16: same stencil + cadence
  + stability ⇒ shared pair). Releasing a pair bumps its generation; a stale `PairHandle` held by a
  bind-group descriptor resolves to `None` — **that mismatch is the signal to rebuild the bind group**
  (stale-descriptor invalidation by construction).
- **RED `red_recycled_pair_invalidates_old_handle`:** allocate pair `p1` → free it → allocate `p2`
  (recycles the slot) → a bind group still holding `p1`'s handle resolves `None` and is flagged for
  rebuild; `p2`'s handle resolves fresh. RED before the pool; GREEN after.
- **RED `red_shared_channels_share_one_pair`:** lease four channels with identical
  `(stencil, cadence, Rgba32F)` → assert all four `ChannelAlloc.pair` handles are **equal** (one pair,
  four channels — the R16 correlated-scalar case). RED before `shared_pairs`; GREEN after.
- **RED `red_independent_fields_get_separate_pairs`:** two `allocate_independent_field` calls with
  **different** stencils → two **distinct** `PairHandle`s. Proves the type distinction actually separates.
- **Adversarial `red_shared_lease_without_pair_is_none`:** `lease_shared_channel` for a
  `(stencil, cadence, fmt)` with no existing pair → `None` (caller must `allocate_independent_field`
  first). Proves the shared path cannot silently mint a pair, keeping "independently-evolved ⇒ own pair"
  the ONLY pair-creation route.
- **Adversarial `red_swap_parity_survives_reload`:** flip `current` (swap), free/recycle unrelated
  channels, assert `current` on the live pair is unchanged — the per-pair parity scalar is pair state,
  not global (the P87 §4.3 "which buffer is current" tag, carried here as designed).

### 4.3 M3 — the WGSL boundary: handles are CPU-only, bindings resolved at encode time

- **Spec:** shaders **never** receive a `Handle`. At command-encode time the registry `resolve`s each
  handle to a concrete `(TextureId, Channel)` binding and writes that into the bind group; WGSL sees only
  the concrete binding. Allocation order is deterministic CPU code (the arena's insert order), preserving
  the FE-07 "engine consumes, never re-derives" contract and the CPU determinism the oracle depends on.
- **RED `red_no_handle_in_wgsl_surface`:** a call-graph/type assertion (CI, mirrors P92's
  `red_fastpath_never_reaches_ledger` pattern) that no `Handle` type flows into a WGSL uniform/storage
  binding struct — the registry's public surface reaching the encoder is `&ChannelAlloc`/`TextureId`
  only. RED if a future edit leaks a handle into a shader-facing struct.
- **Adversarial `red_registry_never_touches_authority`:** assert the `channel_registry` module's imports
  and call graph reach **no** `money`/`compose`/`event_log`/oracle symbol — the presentation/authority
  wall (P38 honesty split). A compile/CI failure if crossed.

### 4.4 M4 — lease/free churn microbench (documents cost; does NOT justify existence)

- **Spec:** a criterion bench (in P81's engine bench harness, the substrate) measuring `lease → resolve
  → free` churn at {64, 256, 1024} live channels and pair recycle rates. **Framing (item 10, stated
  honestly):** this is **safety infrastructure** — the bench documents that the generational registry's
  per-op cost is negligible vs a texture upload; it does **not** exist to prove a speedup, because the
  registry's justification is *correctness* (ABA-safety), not throughput. A "no measurable overhead"
  result is a PASS; a regression would flag an accidental O(n) path.
- **Falsifier:** the bench emits `lease_free_churn` numbers; a > O(1) trend fails the perf gate.

### 4.5 WebGL2 / CPU-floor fallback — the registry is WebGPU-compute-scoped (FE-16 floor)

P38's FE-16 fallback ladder is **WebGPU → WebGL2 → CPU `compose_field`** (§0.6;
`BLUEPRINT-P38-webgpu-render-engine.md` §3.6, §12.3) — a courier's mid-tier phone may reach only the
WebGL2 rung, which has **no compute shaders and no atomics** (core WebGL2 / GLES 3.0). The channel/pair
registry (§3) is CPU-side bookkeeping for **multi-channel GPU field state** — a structure that exists
**only** on the WebGPU compute path gated by P38 §4.2 (OD-11). Two consequences, stated plainly
(closing META-GAP-AUDIT-2026-07-19 §1 G2 for P86):

1. **On the WebGL2 and CPU floors there is no multi-channel GPU field state to register.** The field
   runs as today's single-plane CPU `FieldFrame` / `compose_field` (the P38 §3.1 authority path),
   presented by a WebGL2 fragment-raster textured quad (or canvas2d `putImageData` on the CPU floor).
   The whole registry is `#[cfg(all(slot-arena, gpu))]` and simply **does not participate** on these
   rungs — the feature **does not apply and falls back to the existing CPU path**; it is **not**
   re-implemented in a non-generational or non-atomic WebGL2 form. There is no ABA hazard to defend
   because there is no channel pool on WebGL2/CPU. This is **option (i)** of the audit's G2 choice: the
   compute consumer is WebGPU-only and the WebGL2 floor never runs it.
2. **`R32Uint` (and `Rgba32F`/`Rg32F`) renderability is a WebGPU-compute concern, not a WebGL2-floor
   one.** The audit flagged whether `R32Uint` is renderable on the WebGL2/mobile floor; since the
   registry never runs on WebGL2, the concern is resolved **by scope** — the packed formats are touched
   only by the WebGPU compute shaders (post-OD-11). The build-time re-derivation (§0.5, D-DERIVE) must
   still confirm each `PackedFormat` is a supported storage/render format **on the target WebGPU
   device** (fail-closed on an unsupported format, per §3's `PackedFormat` "unknown REJECTED" note),
   and confirm the WebGL2 fallback presents the field through the existing single-plane path — never
   requiring a packed integer texture to render the fallback.

**DoD addition — the FE-16 floor line (P38 §12.3 standing gate), previously absent (audit G2):**
**D-WEBGL2** — the field renders **CORRECT on the WebGL2 and CPU floors with the registry absent.**
Falsifier: with `navigator.gpu` forced undefined (P38 §3.6 degrade pattern) the field composes via the
CPU path and **no `channel_registry` symbol is reachable**; a build where the WebGL2/CPU fallback
depends on the registry, or where a `PackedFormat` is assumed renderable on WebGL2 without confirmation,
= **NOT done**.

---

## 5. The lifecycle in full (the design the operator asked for — item 2)

1. **Field is born (multi-channel GPU state, gated on P38 §4.2).** A logical scalar field either joins a
   correlated group (same stencil/cadence/format → `lease_shared_channel`, sharing an existing pair's
   swap per R16) or is independently evolved (`allocate_independent_field`, minting its own pair). It
   holds an 8-byte `ChannelHandle`.
2. **Field evolves.** The field engine runs its stencil `step()` on the pair's `front`/`back` and swaps
   `current` — **P86 does not touch this**; it only pooled the pair. Bind groups reference the pair via
   `PairHandle`; a live generation match means the descriptor is still valid.
3. **Field dies.** `free_channel` bumps the channel slot's generation (every outstanding copy of the
   handle is now `None`); if it was the pair's last channel, the pair is freed too (its generation bumps,
   invalidating any bind group still holding the pair handle → rebuild signal).
4. **Slot recycles.** A new field leases the recycled `(texture, channel)`; its handle carries the higher
   generation. **Any stale holder of the old field's handle gets `None`, never the new field's scalar** —
   the ABA hazard is unrepresentable, by construction (`slot_arena.rs:39-44`).
5. **Backing crate stays swappable.** Because call sites use P86's API + opaque `Handle` (never
   `thunderdome::*`), the arena deep-dive's hand-rolled `SlotArena` (§3 sketch) can later replace
   thunderdome without touching a single P86 call site (`slot_arena.rs:22-24`).

---

## 6. Definition of Done — falsifiable, RED→GREEN (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | a freed channel's stale handle resolves `None`, never a bled scalar | `red_stale_channel_lease_resolves_none`, `red_release_to_new_field_no_bleed` (M1) |
| D2 | a recycled pair's generation bump invalidates old bind-group handles (rebuild signal) | `red_recycled_pair_invalidates_old_handle` (M2) |
| D3 | R16 sharing is a type distinction: correlated channels share one pair; independent fields get separate pairs; shared-lease without a pair is `None` | `red_shared_channels_share_one_pair`, `red_independent_fields_get_separate_pairs`, `red_shared_lease_without_pair_is_none` (M2) |
| D4 | handles never reach WGSL; the registry never touches the authority path | `red_no_handle_in_wgsl_surface`, `red_registry_never_touches_authority` (M3) |
| D5 | a forged/hand-constructed handle is not expressible (opaque type) | `red_forged_handle_rejected` (M1) |
| D6 | lease/free churn is O(1); documented, not a throughput claim | `lease_free_churn` bench (M4) |
| D-GATE | the whole unit is `#[cfg(all(slot-arena, gpu))]`; default build byte-unchanged | `cargo build` (default) diff-clean; `cargo test --features slot-arena,gpu` green |
| D-DERIVE | the channel/format taxonomy was re-derived against the live P38 design, not copied from §3's illustrative set | a build-time note in the registry citing the then-current P38 field-state spec (§0.5, §9) |
| D-WEBGL2 | the field renders CORRECT on the WebGL2 + CPU floors with the registry absent (WebGPU-compute-only; those rungs fall back to the single-plane CPU path) — §4.5 | forced-`navigator.gpu=undefined` composes via the CPU path, no `channel_registry` symbol reachable, no packed format assumed renderable on WebGL2 (P38 §12.3 floor line) |

---

## 7. Cross-cutting obligations (standard §2 items 6–20)

- **Hazard-safety as structure (item 6):** the unsafe state — *a field silently sampling a recycled
  channel/pair that now belongs to a different field* — is **unrepresentable**: `SlotArena`'s generation
  bump makes every stale handle a typed `None` (`slot_arena.rs:39-44`), and the opaque `Handle` forbids
  forging one (`:29-31`). Reachability is argued from the type, not a runtime guard.
- **Schemas & scaling axis (item 8):** scaling axis = **live channels per node** and **live pairs per
  node**; both are O(1) dense-Vec-backed state (`slot_arena.rs:75-80`). Shape changes only if a node
  holds so many simultaneous live fields that the `shared_pairs` hashmap dominates — then a denser index;
  stated, not timeless. The `Handle` is fixed 8 bytes regardless of pool size.
- **Linux discipline (item 9):** **EXTENDS** the landed `slot_arena.rs` (its first named consumer, by
  the module's own doc); **REINFORCES** the opaque-handle/degrade-closed/fail-closed patterns;
  **ALREADY-EQUIVALENT** on the opt-in-feature discipline (`pq`/`gpu`/`pgrust` siblings);
  **DOES-NOT-TRANSFER** — no new allocator, no daemon.
- **Benchmarks + telemetry (item 10):** §4.4 (churn bench, honestly framed as documenting cost not
  justifying existence). Telemetry: `SlotArena::{len,capacity}` (`:104,:115`) already expose live-count
  and backing-capacity sizing telemetry — surface them per registry so pool growth is observable.
- **Isolation / bulkhead (item 11):** the registry is a **bulkhead** — its failure mode is a stale
  handle → `None` → the caller skips/rebuilds, never a crash and never a wrong sample. A registry bug
  cannot corrupt the money/oracle plane because there is **no code path** from it to those sinks
  (`red_registry_never_touches_authority`, M3).
- **Mesh awareness (item 12):** N/A — GPU resource bookkeeping is strictly node-local (one device),
  never gossiped/store-and-forwarded. Stated.
- **Rollback / self-heal as math (item 13):** **Self-termination** = a stale handle is a typed `None`
  (unrepresentable use-after-free, not a supervisor decision). **Snapshot re-entry** = a generation
  mismatch triggers a deterministic bind-group rebuild from live registry state. Self-healing is NOT
  claimed — a lost channel is simply re-leased.
- **Error-propagation / smart index (item 14):** the bug classes this introduces (a stale sample, a
  handle leaking into WGSL, the registry touching authority) are turned into **compile/CI-time**
  failures: the opaque type (M1), the WGSL call-graph assertion + authority-wall assertion (M3), the
  type-distinction pair API (M2). Not runtime surprises.
- **Living-memory awareness (item 15):** channel/pair leases are **deliberately ephemeral** (per-frame
  GPU resource lifecycle, no durable persistence) — the opposite of living memory; anything durable uses
  the full CPU-authority event log, never the registry. Stated.
- **Tensor/spectral (item 16):** the *contents* of the leased channels are spectral/physics field state
  (the field engine's domain), but the **registry itself is a resource pool, not a linear-algebra
  kernel** — forcing `spectral.rs` onto the bookkeeping would be over-engineering (ponytail). Honestly
  N/A for P86's own surface.
- **Regression tracking (item 17):** M1–M3's RED tests enter the suite permanently; a REGRESSION-LEDGER
  entry for "generational GPU resource registry" so the ABA-safety property is guarded forever.
- **Reuse-first (item 19):** §1 — P86 reuses the landed arena wholesale; extension (a hand-rolled pool)
  was considered and rejected because the arena is already in-tree, byte-neutral, and swappable.
- **Hermetic principles (item 20):** §7.1.

### 7.1 Hermetic principles honored (item 20 — load-bearing only)

- **Correspondence:** a handle *corresponds* to a live resource exactly while its generation matches —
  "as above (the generation counter), so below (the resource's liveness)"; the binding is
  self-describing, never asserted. A stale generation *is* a dead resource, by identity.
- **Cause & Effect:** a channel exists only because a field *leased* it; freeing the field is the cause
  whose effect is the generation bump — nothing is trusted by a bare index (correlation), only by a
  matching generation (a caused, verifiable link).
- **Polarity / no-middle:** `resolve` returns `Some(live)` or `None(stale)` — there is no "maybe valid"
  middle a stale index could smuggle through. The safety is binary by construction.

---

## 8. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (arena first-hand `slot_arena.rs:*`; GPU facts honestly synthesis-attributed §0.5) |
| 2 | Falsifiable DoD | §6 (D1–D-DERIVE) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; lifecycle events in §5) |
| 4 | Predefined types & constants | §3 (`ChannelAlloc`/`PingPongPair`/`ChannelRegistry`/… before impl) |
| 5 | Adversarial tests | §4 (each M has RED adversarial cases) |
| 6 | Hazard-safety from type structure | §7 (unrepresentable stale-sample), §0.1 |
| 7 | Links to docs & memory | §10 |
| 8 | Schemas with scaling axis | §7 (channels/pairs per node; fixed 8-byte handle) |
| 9 | Linux discipline | §7 (EXTENDS the landed arena, its first consumer) |
| 10 | Benchmarks + telemetry | §4.4 (honest: documents cost, `SlotArena::{len,capacity}` telemetry) |
| 11 | Isolation / bulkhead | §7 (stale→None→skip; no path to authority sinks) |
| 12 | Mesh awareness | §7 (node-local GPU resources, N/A stated) |
| 13 | Rollback/self-heal as math | §7 (self-termination = typed None; re-entry = deterministic rebuild) |
| 14 | Error-propagation / smart index | §7 (opaque type, WGSL + authority call-graph assertions) |
| 15 | Living-memory awareness | §7 (deliberately ephemeral; durable data uses the CPU authority log) |
| 16 | Tensor/spectral where applicable | §7 (contents are field state; the registry is a pool — N/A stated) |
| 17 | Regression tracking | §7 (RED tests permanent + REGRESSION-LEDGER entry) |
| 18 | Clear worker instructions | §9 |
| 19 | Reuse-first, upgrade-if-needed | §1, §2.4 (reuses the landed arena; extension rejected) |
| 20 | Hermetic principles | §7.1 |

---

## 9. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §4.2 (the P86 direction), §2 rows 1/2 (divergence
  ledger), §7 E3′ (override record), §0-R13/§0-R16 (the disk-lost GPU sources' summaries — P86's
  attributed grounding).
- `docs/research/OPUS-PERF-ARENA-DEEPDIVE-2026-07-18.md` §5.2/§6 (on disk — the parked trigger + operator
  override; the hand-rolled `SlotArena` fallback §3).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P86 row), §0 (which research docs are disk-lost), §5 OD-11.
- `BLUEPRINT-P38-webgpu-render-engine.md` §4.2 (the operator-owned GPU field-state gate, FE-16 ladder).
- **Disk-lost primaries (do NOT cite as first-hand — §0.5):** `OPUS-PERF-RGB-GPU-TEXTURE-PACKING`,
  `OPUS-PINGPONG-SHADOW-COPY-PROPAGATION`, `OPUS-PHYSICS-WAVE-ALGORITHMS-AND-TENSOR-SYNTHESIS`
  (2026-07-18). On disk instead: `OPUS-PERF-RGB-PACKING-REUSE-2026-07-18.md` (R9, the REJECTED CPU angle).
- Memory: `physics-ui-capture-quantum-math-arc-2026-07-14.md` (the field-UI engine; FE-07 bridge),
  `performance-priority-over-minimal-change-2026-07-17.md` (forward-looking infra is in-scope).
- **Standard:** `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. **Format precedent:** `BLUEPRINT-P92-*`.

**Existing code this blueprint extends (exact targets, dowiz — NOT bebop-repo):**
- **REUSE unchanged** `kernel/src/slot_arena.rs` (`SlotArena`/`Handle`; feature `slot-arena`,
  `kernel/Cargo.toml:77`) — never `thunderdome::*` directly (`slot_arena.rs:22-24`).
- **NEW** `engine/src/gpu/channel_registry.rs` — all §3 types; `#[cfg(all(feature="slot-arena",
  feature="gpu"))]`.
- **EDIT (build-time, post-P38 §4.2)** the engine's GPU field-state module to lease channels/pairs
  through the registry instead of holding raw `TextureId`s.
- **DO NOT TOUCH** the stencil `step()` / swap algorithm, any money/order/oracle path, or the default
  build (must stay byte-unchanged).

**For the worker with zero session context — exact acceptance path:**
1. **Do NOT start any build leg until P38 §4.2 (OD-11) is taken** — until multi-channel GPU field state
   exists there is nothing to lease. Confirm the operator decision first.
2. **Re-derive the channel/format taxonomy (§0.5, D-DERIVE)** against the *then-current* P38 field-state
   design — the §3 `PackedFormat`/`(vx,vy,p,ρ)` set is illustrative of a **disk-lost** source, not
   authoritative. Do not freeze it from this blueprint.
3. Write §3 types in `channel_registry.rs` first (types → tests → code — item 3); implement M1→M3 in
   order; each M's RED tests fail before its code and pass after.
4. Keep the whole unit `#[cfg(all(slot-arena, gpu))]`; verify the default build is byte-unchanged
   (D-GATE) exactly as `a857cd71a` was for the arena itself.
5. Add the "generational GPU resource registry" regression entry to `docs/regressions/REGRESSION-LEDGER.md`.
6. Anti-scope: never let a `Handle` reach WGSL; never touch money/oracle; never create a pair outside
   `allocate_independent_field`; never re-implement the arena — extend it.
7. **P87 leases through this registry** (its 2-bit mask plane is an `R32Uint` channel) and **P88's
   atomicity rule governs the shaders that read these channels** — coordinate, do not duplicate.

---

*This blueprint writes ZERO product code, touches no git branches, pushes nothing. It is a design +
acceptance contract; the build is gated on the P38 §4.2 operator decision (OD-11).*
