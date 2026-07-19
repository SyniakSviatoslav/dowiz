# Packed-flags / bitmask-representation scan of the mesh auth/authz decision layer

> Research-only. Zero code written, no branches touched, no files modified except this doc.
> Every claim is grounded in a `file:line` read of the live tree at HEAD
> (`/root/bebop-repo/bebop2/proto-cap`, `/root/bebop-repo/bebop2/mesh-node`) on 2026-07-19.
> Scope per operator directive: **DECISION-LAYER / STATE-LAYER** representations only — the
> outputs and intermediate flags of the auth/authz pipeline. Crypto material (sig/MAC/nonce/key
> low-bit) and money/ledger amount low-bit are **closed and NOT revisited here**. Companion to
> `OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md`. (The P92 hotstream blueprint
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` did **not** exist in
> `docs/design/CORE-ROADMAP-2026-07-17/` at read time — this scan proceeds from the handshake
> research doc's §5b fast-path definition instead, and should be cross-checked against P92 once written.)

---

## 0. TL;DR verdict

- **~90% of the decision/state representations in this layer are already appropriately minimal** and
  need no change: every gate decision is `Result<(), CapError>` (~1 byte), every policy/status is a
  C-like enum with a pinned `u8` discriminant (`HybridPolicy`, `ClaimStatus`, `RedLineCategory`,
  `KillState`, …), and the `Option<Vec<u8>>` "present?" flags on crypto fields carry their flag in a
  **free niche discriminant**. There is **no sub-byte "make it 1 bit" win** anywhere — matching the
  session's settled position exactly. A `bool`/`Result` is already the smallest addressable unit.
- **There is exactly ONE genuine, valuable candidate**, and it is the real thing the task is looking
  for: **`Scope` and `Effect` are `struct { grants: Vec<(Resource, Action)> }`** — a heap-allocated
  collection over a **small, closed, fixed** domain (`Resource` ≤32 variants, `Action` ≤32). A
  fixed in-memory **bitmask** representation would make them `Copy`, kill the per-frame heap
  clone/alloc churn on the hottest authz path, and turn subset / red-line / read-gate checks into
  branchless word-ANDs.
- **Two honest caveats keep this from being oversold** (details in §4): (1) it is **not** a
  "hash-lookup → `&`" win because the current structure is a **`Vec`, not a `HashSet`** — for the
  typical single-pair scope the *scan* is already ~1 compare; the real win is **allocation
  elimination + `Copy` + branchless**; (2) the pair space is **408 today (≤1024 headroom), which
  exceeds 64 bits**, so it is **not** a single `u64` — it needs a small fixed array
  (`[u32; N]` or `[u64; 16]`), and the **on-wire/signed TLV encoding must stay the ordered-pair
  list** (signatures commit to it), so the bitmask is an **in-memory decision-layer representation
  only**. The payoff concentrates on the **P92 hotstream fast-path** (§5), where crypto is amortized
  away and `effect ⊆ cap.scope` becomes the dominant per-frame operation.

---

## 1. Full inventory — every decision output / state flag / scope representation in the layer

| # | Thing | Rust type | File:line | Already minimal? |
|---|-------|-----------|-----------|------------------|
| 1 | Gate decision (per frame) | `CapResult<()>` = `Result<(), CapError>` | `hybrid_gate.rs:124`, `signed_frame.rs:258` | **Yes** — `Ok(())` zero-size; niche-packed |
| 2 | Auth fault enum | `CapError` (16 fieldless-ish variants) | `error.rs:14-68` | **Yes** — ~1 byte discriminant |
| 3 | Gate policy | `HybridPolicy` (2 variants, `Copy`) | `hybrid_gate.rs:24-34` | **Yes** — the "2-state" pattern already; 1 byte |
| 4 | Classical/PQ verify result | `CapResult<()>` (internal `ok: bool`) | `signed_frame.rs:208-246` | **Yes** — `bool` is 1 byte, floor |
| 5 | Frame sig-presence flags | `Option<Vec<u8>>` (`classical_sig`, `pq_sig`), `Option<[u8;32]>` (`channel_binding`) | `signed_frame.rs:87-97` | **Yes** — flag is a free niche; payload is crypto (out of scope) |
| 6 | **Scope (granted/requested authz set)** | **`struct Scope { grants: Vec<(Resource, Action)> }`** | **`scope.rs:127-131`** | **NO — the one candidate (§3/§4)** |
| 7 | **Effect (delegation grant set)** | **`struct Effect { grants: Vec<(Resource, Action)> }`** (identical) | **`roster.rs:60-64`** | **NO — same candidate** |
| 8 | Resource / Action vocab | closed enums, pinned `u8` disc via method | `scope.rs:18-62`, `66-118`, `172-284` | **Yes per-variant** — the *collection* (#6/#7) is the issue, not the enums |
| 9 | Red-line category / policy | `RedLineCategory` (4), `RedLinePolicy{DenyByDefault, AllowList(Vec<Scope>)}` | `redline.rs:26-51` | **Category yes**; `AllowList` inherits #6's `Vec<Scope>` |
| 10 | Claim lifecycle state | `ClaimStatus` (4 variants, `Copy`, pinned disc) | `claim_machine.rs:19-51` | **Yes** — 1 byte C-like enum; not per-frame hot |
| 11 | Kill state / reject | `KillState` (2), `KillReject` (5) | `kill_switch.rs:145-187` | **Yes** — control-plane, 1 byte, not per-frame |
| 12 | Node lifecycle enums | `BootError`, `DodFault`, `ApplyOutcome`, `PolicyError`, `ListenerAction`, `DeliveryStatus` | `boot.rs:36`, `dod.rs:27`, `hub_policy.rs:279`, `260`, `listener_reconcile.rs:18`, `event_dict.rs:30` | **Yes** — control-plane enums, minimal |
| 13 | Anchor roster | `HashSet<[u8;32]>` | `roster.rs:193` | **Yes** — 32-byte keys, unbounded domain; a set is correct, NOT a small fixed domain |
| 14 | Revocation set | `HashSet<[u8;32]>` × 2 | `revocation.rs:51-53` | **Yes** — same as #13 |
| 15 | Replay/nonce ledger | `Mutex<HashSet<[u8;8]>>` | `hybrid_gate.rs:67` | **Yes** — 8-byte nonces, unbounded; set is correct |
| 16 | Facade read-gate | `allowed_reads: Vec<Scope>` | `facade.rs:79`, checked `:144` | **Inherits #6** — `Vec<Scope>` `.contains(&scope)` whole-struct scan |
| 17 | `enabled` toggles | `bool` | `entropy.rs:91`, `hub_policy.rs:36` | **Yes** — 1 byte, floor; sub-byte packing yields nothing |

**No `bitflags`, no `#[repr(...)]`, no manual mask exists anywhere in either crate today**
(`grep -rE 'bitflags|#\[repr\(|bitvec|fixedbitset'` over both `src` trees returns nothing on the
non-test path; `proto-cap/Cargo.toml` deps are only `bebop2-core` + `serde`). So the layer today
carries **zero** bitmask packing — the candidate below would be the first.

---

## 2. Where the decision/state values are actually computed (the hot path)

Per-frame authorization is `HybridGate::check` (`hybrid_gate.rs:124-209`), reached from both carriers
on **every recv** (companion doc §1b: `iroh_transport.rs:395-401`, `wss_transport.rs:612-621`) and
from `KernelFacade::submit_intent` (`facade.rs:123-136`). Inside `check`, the scope/authz work is:

- `verify_chain(roster, chain, cap, now)` — `hybrid_gate.rs:142` → `roster.rs:252-316`.
- `RedLineGate::check(&frame.capability.scope, rl)` — `hybrid_gate.rs:151` → `redline.rs:97-127`.

`verify_chain` performs **per-link** and **per-frame** scope work (`roster.rs`):

- `:277` `link.effect.is_subset_of(&link.scope)` — per link.
- `:289` `link.scope.is_subset_of(&ps)` — per link (attenuation).
- `:295` `parent_scope = Some(link.scope.clone())` — **heap clone per link**.
- `:305` `Effect::new(cap.scope.grants.clone())` — **heap clone per frame**.
- `:306` `requested.is_subset_of(&tail.scope)` — **the `scope ⊆ cap` check** the P92 / handshake
  research names as the cheap per-frame survivor (companion §2 row C2, §5b).
- `:129` `Scope::new(self.effect.grants.clone()).to_tlv_bytes()` — clone + fresh `Vec` alloc inside
  `Delegation::canonical_bytes`, which runs in **every** `verify_signature` (`:172-184`).

`is_subset_of` itself (`scope.rs:167-169`, `roster.rs:84-86`) is
`self.grants.iter().all(|p| super_scope.grants.contains(p))` — an **O(n·m) linear `Vec` scan**
(`Vec::contains` is linear), not a hash lookup.

`RedLineGate::check` (`redline.rs:97-127`) **allocates a fresh `Vec` (`red_pairs`) every call**
(`:99-104`) then does nested linear scans (`:118`
`allowed.iter().any(|allow| allow.grants.contains(&(r,a)))`).

Other scope-subset call sites, all `Vec`-linear: `port.rs:100` (`check_port_scope`),
`facade.rs:144` (`read_projection` — whole-`Scope`-struct `Vec` equality), `breach.rs:104`
(direct `scope.grants != &[...]`).

---

## 3. The domain is small, closed, and fixed — the precondition for bitmask packing

- `Resource` (`scope.rs:18-62`): **17 variants**, pinned discriminants `0x01..0x11` (`:172-194`).
  Headroom ≤ 32 without changing byte width.
- `Action` (`scope.rs:66-118`): **24 variants**, pinned `0x01..0x18` (`:224-251`). Headroom ≤ 32.
- Pair space (`Resource` × `Action`): **17 × 24 = 408 possible pairs today; ≤ 32 × 32 = 1024 with
  headroom.** This is the exact "small fixed set of possible permissions" precondition the task
  describes — the closed enums are literally documented as "closed set so the gate is total"
  (`scope.rs:16`, `:64`).
- **In practice the sets are tiny** — almost every constructed scope is single-pair: `Scope::single`
  / `Capability::new` / `Effect::single` dominate every non-test call site; multi-pair `Scope::new`
  appears only in the G4 attenuation tests (`roster.rs:557-630`). So the *cardinality* per scope is
  ~1; the domain *breadth* is ≤1024.

---

## 4. The candidate: `Scope`/`Effect` as an in-memory bitmask (honest design + benefit)

### 4a. Two viable packings (neither is a single `u64` — 408/1024 > 64)

**(A) Per-resource action bitmask** — `[u32; N_RES]`, one `u32` action-set per `Resource`:
```
// in-memory ONLY; NOT the wire encoding (see 4c)
struct ScopeMask { rows: [u32; 18] }   // 18 u32 = 72 bytes, Copy
// pair present iff  rows[res_disc-1] & (1 << (act_disc-1)) != 0
```
- subset:  `self.rows.iter().zip(sup.rows).all(|(a,b)| a & !b == 0)` — 18 word-ANDs, branch-light,
  **zero allocation, no per-pair loop**.
- red-line: one precomputed `const RED_LINE: [u32; 18]` built from `is_red_line` (`redline.rs:70-85`);
  check is `self.rows.iter().zip(RED_LINE).any(|(s,m)| s & m != 0)` — **no per-call `Vec` alloc**.
- equality (read-gate `:144`): word compare.

**(B) Flat 1024-bit set** — `[u64; 16]` (128 bytes, `Copy`), index `= (res_disc<<5) | act_disc`.
Same properties; slightly wider but a single flat index. (A) is leaner and maps 1:1 to the
resource/action structure; recommend (A).

### 4b. What the bitmask actually buys (measured against §2)

1. **`Scope`/`Effect` become `Copy`** → deletes **every** per-frame/per-link `.clone()` heap op on the
   hot path: `roster.rs:129, :295, :305`. On a delegation chain of length L this is L+1 fewer heap
   allocations per verified frame, plus the `red_pairs` `Vec` alloc per red-line check
   (`redline.rs:99`). This is the **largest, most defensible** win — pure allocator-pressure and
   cache reduction, no behavioural change.
2. **Subset/red-line/read-gate checks become branchless word-ANDs** — O(#words)=const, no
   `Vec::contains` inner loop, no pointer chase into heap-backed `Vec`s.
3. It composes with the settled "packed small-enum state" pattern (P86-P89 thunderdome/RGB fusion,
   2-bit ping-pong masks) — same family, applied to the authz permission set.

### 4c. The two hard constraints (why this is scoped, not a free win)

1. **Wire/signing encoding MUST NOT change.** `Scope::to_tlv_bytes` (`scope.rs:153-161`) is on the
   **signed path**: its bytes are committed to by Ed25519 **and** ML-DSA-65 signatures
   (`capability.rs:110-124`, `roster.rs:126-143`, `signed_frame.rs:144-162`). The encoding is
   `len(u16 LE) || (res_u8, act_u8)*` — an **ordered pair list**. Changing it is a **breaking wire +
   signature change**. Therefore the bitmask must be an **in-memory representation only**: keep the
   TLV list as the canonical serialize/sign form (canonicalized — sort pairs before encoding so two
   equal sets have identical bytes), derive the bitmask at parse time for checks, and re-serialize
   deterministically. This is exactly the "two representations" discipline already used in this crate
   (serde for transport framing vs hand-built TLV for signing — `capability.rs:11-17`,
   `signed_frame.rs:27-32`). The bitmask is a **decision/compute-layer** representation; the TLV list
   stays the wire/signing representation.
2. **Magnitude honesty — this is NOT a hash-avoidance win.** The current structure is a `Vec`, not a
   `HashSet`, and typical scopes are single-pair (§3), so the subset **scan** is already ~1 compare.
   The gain is **allocation elimination + `Copy` + branchless**, not "O(1) `&` instead of O(hash)".
   Framing it as hash-avoidance (as the generic task prompt hypothesizes) would overstate it — there
   is no hash here. State it as: *remove per-frame heap clone/alloc churn from `verify_chain` +
   `RedLineGate::check` and make the authz-set checks branchless.*

---

## 5. Where the win is real and worth doing: the P92 / handshake fast-path

On the full per-frame path, the bitmask is a **modest but real** allocation-elimination win (the
crypto — 2× Ed25519 + 1× ML-DSA-65 verify, companion §1b — dominates cost, so shaving `Vec` clones is
secondary). **The bitmask's value concentrates on the live-session hot-stream fast-path** that the
handshake research (`OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md` §5b) prescribes for
both-endpoints-online, high-frequency, same-scope streams (e.g. courier `Presence` updates):

> "*Per subsequent frame: symmetric AEAD/MAC + a monotonic per-session sequence counter (cheap) — but
> keep the two cheap non-crypto per-frame checks: `effect ⊆ C.scope` (else re-open C2) and freshness
> ≤ `C.expiry`.*" (§5b, and §2 row C2: "*the per-frame **scope⊆cap** check … must survive into any
> session model*").

On that fast-path the **crypto is amortized to once-per-session**, so `effect ⊆ C.scope`
(`roster.rs:306` in spirit) becomes **the dominant per-frame authorization operation**. Concretely:

- **Once at session establishment:** compute the session capability's `ScopeMask` from `C` (one parse).
- **Per frame:** `frame_effect_mask & !session_scope_mask == 0` — a handful of word-ANDs, **zero
  allocation, zero heap touch, branchless** — vs today's `Effect::new(cap.scope.grants.clone())` +
  linear `Vec` subset (`roster.rs:305-306`). This is a genuine, compounding per-frame saving exactly
  where P92 wants throughput, and it is squarely a **decision-layer** optimization (the crypto
  primitives are untouched).

**Recommendation:** flag the bitmask as a **fast-path enabler for P92**, not a blanket rewrite. If
P92 lands the verify-once session model, precomputing the session scope as a `ScopeMask` and running
the per-frame subset as a word-AND is the natural, high-value shape. On the full per-frame path,
adopt the same in-memory `ScopeMask` opportunistically to delete the `verify_chain` clone churn — but
sequence it **after** P92, and keep the TLV list as the signed wire form (§4c-1).

---

## 6. Explicitly NOT candidates (so the report is falsifiable, not vague)

- **Any `bool` / `Result<(), _>` / 2-4-variant enum** (#1-5, #10-12, #17): already at the 1-byte
  floor. "Make it 1 bit" is meaningless — no sub-byte addressable storage — and matches the settled
  position. Leave them.
- **`AnchorRoster`, `RevocationSet`, nonce `seen` ledger** (#13-15): `HashSet` over 32-/8-byte keys
  with an **unbounded** domain. A set is the correct structure; there is no small fixed domain to
  pack. Leave them.
- **Crypto material** (`classical_sig`, `pq_sig`, `subject_key_pq`, channel-binding hash): out of
  scope by directive (closed as unsound to low-bit) — and correctly full-width today.
- **`ClaimStatus` / `KillState` state machines**: already the settled "small-enum state" pattern
  (`Copy`, pinned `u8` disc). Not per-frame hot. No change.

---

## 7. One-paragraph blueprint stub (for whoever picks this up)

Introduce an **in-memory** `ScopeMask([u32; 18])` (`Copy`) derived from `Scope`/`Effect` at
parse/verify time; keep `Scope`/`Effect`'s `Vec<(Resource,Action)>` + `to_tlv_bytes` **unchanged** as
the canonical signed wire form (canonicalize by sorting pairs before encoding). Replace the linear
`is_subset_of` (`scope.rs:167`, `roster.rs:84`), the `RedLineGate::check` `red_pairs` allocation
(`redline.rs:99-104`) with a precomputed `const RED_LINE: [u32;18]` word-AND, and the
`verify_chain`/`facade` scope comparisons (`roster.rs:277,289,306`, `facade.rs:144`, `port.rs:100`)
with `ScopeMask` word-ops — deleting the `.clone()`s at `roster.rs:129,295,305`. **Gate the priority
on P92**: the win is small on the crypto-dominated full path, large on the P92 verify-once hot-stream
fast-path where `effect ⊆ session_cap` is the per-frame hot operation. RED test: prove
`ScopeMask`-subset is byte-for-byte equivalent to the current `Vec` subset across the full
17×24 pair matrix (mirror `scope.rs:300-355` roundtrip), and that the TLV signed bytes are unchanged
(signature-stability KAT).

---

*Sources — live tree (`file:line`) cited inline: `scope.rs`, `capability.rs`, `roster.rs`,
`redline.rs`, `hybrid_gate.rs`, `signed_frame.rs`, `error.rs`, `facade.rs`, `port.rs`,
`claim_machine.rs`, `node_id.rs`, `kill_switch.rs`, plus grep sweeps of `proto-cap/src` +
`mesh-node/src`. Context: `OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md` §2/§5b. P92 blueprint
absent at read time — reconcile when written.*
