# Lens 1 — bebop2 Protocol State (ground truth of /root/bebop-repo, 2026-07-11 evening)

> Program: transformation of dowiz's two half-hubs into ONE local-first decentralized hub
> (Rust/WASM kernel + bebop2 protocol + SQLite on participants' devices; no central server).
> This lens: what /root/bebop-repo actually contains TODAY, after the post-14:17 edits that
> made the morning audit (§2.2/§4.5) and gap blueprints G08/G09 partially stale.
>
> Method: read-only. `git log/status/diff` + full read of the new unified plan + source reads
> with `file:line` + **live `cargo test --workspace` executed this session**. Both repos left
> exactly as found. Labels: **VERIFIED** (checked against code/git/test-run this session),
> **CLAIMED** (asserted by a repo doc, not independently re-derived), **STALE-SINCE-AUDIT**
> (morning-audit/G08/G09 claim now contradicted by the current tree).

---

## 1. The delta since commit `8012b57` (2026-07-11 14:17)

> The repo was being **actively merged into while this lens ran** (~19:30–19:50): HEAD moved
> from `c977ea6` to `57b1c9a` mid-session. Everything below reflects the final observed state
> (`57b1c9a`); both states were live-verified.

**Branch:** `feat/wire-native-core` @ `57b1c9a` (VERIFIED). Working tree has
**no modified/staged tracked files** — only 5 untracked docs (VERIFIED, `git status`).

**Eight new commits** (VERIFIED, `git log 8012b57..HEAD`):

| Commit | Time | What |
|---|---|---|
| `ed2af6e` | 17:23 | WIP ML-DSA hint-debug iteration; **first commit of `kdf.rs` (Argon2id) and the full `pq_dsa.rs`** — the "uncommitted crown jewels" of G08-c / G09-D4 are now committed (STALE-SINCE-AUDIT) |
| `fb4e651` | 17:54 | ML-DSA-65 **sign/verify roundtrip green** — FIPS 204 Decompose (centered mod + q−1 special case), MakeHint fix |
| `0567003` | 18:09 | ML-DSA sign-retry gated on `z_ok` (out-of-bounds z must reject) |
| `c977ea6` | 19:22 | Ed25519 allocation-free 64-bit-limb GF(2^255−19) arithmetic + carry-chain fix; also updated `AGENTS.md`/`README.md` |
| `388f90b`+`0d3cd19` | ~19:45 | **G9 lands (merge of `feat/wasm32-hardening`):** f64 analytics gated behind a `host` feature; `std`/`host` features added to `bebop2/core/Cargo.toml`; wasm32 no_std empty-import build |
| `754c7d4`+`57b1c9a` | ~19:50 | **Merge of `feat/mldsa-kat`:** ML-DSA determinism + signature-drift KAT — commit message itself says "**NOT FIPS bit-exact**" (G10 still open) |

**Five untracked docs** (VERIFIED): `docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`
(19:23 — **the newest unified plan, see §2; it is NOT committed — bus-factor-1 exposure**),
`docs/design/plan-audit-bebop-2026-07-11.md` (17:34, the bebop-side audit feeding it),
`bebop-fable-research-2026-07-11.md`, `bebop-memory-optimisation-fable-research-2026-07-11.md`,
`fable-protocol-2026-07-11/` (F1–F4).

**Agent worktrees** (VERIFIED, `git worktree list`): the blueprint footer described 3 in-flight
parallel agents; **two of the three merged during this session** (`feat/wasm32-hardening` →
`0d3cd19`, `feat/mldsa-kat` → `57b1c9a`). Still in flight with uncommitted work:
`/root/bebop-arch` (`feat/arch-hardening` @ `0567003`, new `arch_hardening.rs` + 5 modified
files — H3/H4/H5 architecture hardening, unmerged).

**Test suite: 388/388 green — VERIFIED by live run this session** (`cargo test --workspace
--offline` at `57b1c9a`): 275 bebop + 19 rust-core + 94 bebop2-core, 0 failures. (Pre-merge
`c977ea6` was 385/385, also live-verified; morning audit said 384 — the count is climbing
through the session; STALE-SINCE-AUDIT in the harmless direction.)

---

## 2. The unified plan — faithful summary (the program's north star)

Source: `/root/bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`
(197 lines, read in full). It synthesizes `plan-audit-bebop-2026-07-11.md` + dowiz's
`plan-audit-memory-2026-07-11.md` + the dowiz MANIFESTO/DECISIONS doctrine.

**The one sentence (blueprint §0, quoted):** build an *"open-source, self-hosted owner hub
(local-first, Rust/WASM, pure deterministic event-sourced core, no AI in runtime) that funnels a
food-vendor's multi-channel / multi-device order entrypoints into one 0%-commission checkout and
dispatches the vendor's own couriers — with PQ signatures + mesh/P2P seams baked now so Phase-2+
can switch on without a rewrite, and the matcher decentralized (not a single dispatch server) so
the protocol never re-centralizes."*

**Hard constraints (§2, C1–C10):** no AI in runtime logic; pure core (no clock/RNG/env/floats/
network in `dowiz-core`/`bebop2-core`); immutable event-sourced state machine (`Intent → decide →
Event`, `state = fold(events)`); local-first + no central server as a reachable-free invariant
from the signed event log; integer-only money (`Lek(i64)`); AGPLv3 destination; Verified-by-Math
RED+GREEN on every change; **over-engineering is the #1 enemy — PQ/mesh/CRDT hard-gated behind
MVP (C8/D6)**; ethics charter; crypto from-scratch, zero-dep, RNG-free hot path.

**Layered stack (§3):** L0 event core (dowiz-core — "DONE": 10-status order machine, decide
composition, red-proof; CLAIMED, lives in dowiz, not this repo) → L1 identity/PoD (bebop2 PQ core,
self-cert `id = H(pq_pub‖classical_pub)`, no issuer) → L2 open replicable matcher (pure fn, any
node, identical fingerprints — NOT a dispatch server) → L3 settlement via ≥k-of-n device-sig
threshold verifier, NOT a single oracle; DLT only for final PoD settlement → L4 fail-closed
dispute machine OPEN→EVIDENCE→AUTO→ESCALATE→JURY→SETTLE, timeout ⇒ escrow HOLD + refund →
L5 thin client + reference alt-client. Kept visible: the four centralization dangers
(matcher/sequencer, SDK/bootstrap access, settlement oracle, identity root).

**MVP definition (§5, "Trojan horse" — shippable NOW):** the dowiz Sovereign Core owner hub —
multi-channel/multi-device entrypoints → ONE 0%-commission direct checkout; pure event machine,
integer money, idempotency; **seams baked free** (per-event content-hash + signature slot,
transport-agnostic sync port, WASM-pure core gate); aggregator order-intake **banned** from MVP;
couriers = honest **single-owner dispatch** (the vendor runs their own).

**Long-term strategy (§5, Phase-2+ destination):** open **competitive matcher market**
(permissionless matchers, force-inclusion timeout, attestation aggregation); per-actor PQ identity
(deferred choice, seams ready); mesh/P2P transport (libp2p vs Zenoh) + CRDT merge (deferred, C8);
vendor-owned courier marketplace (reassignment/auction). **Boundary rule:** the owner hub is the
thin replaceable L5 access layer — *one matcher among many, never the only one*.

**Build order (§8):** 1 MVP hub → 2 crypto re-point (vault/pod → bebop2, retire scrypt) →
3 wasm32 gate → 4 ML-DSA NIST-bit-exact (ACVP) → 5 open matcher + threshold settlement →
6 food-vendor gaps (storefront, courier marketplace, liveness, reroute) → 7 dispute +
PoD-contestability → 8 economics (1–3% fee, not 0%) + thin/alt client.

**The plan's own gap ledger (§6, G1–G12):** G1 no storefront/menu (HIGH, MVP-blocker for a food
vendor), G2 no courier marketplace/reassignment (HIGH), G3 no node liveness (MED), G4 no payout
contract (HIGH), G5 no economics model (MED), G6 mid-route failure (MED), G7 PoD physical handoff
has no trustless anchor — treat as contestable (HIGH), G8 dispute resolution unbuilt (MED-HIGH),
G9 wasm32 gate fails (HIGH), G10 ML-DSA not NIST-bit-exact (HIGH, "ACVP oracle before protocol
keys minted"), G11 two crypto cores (resolved by re-pointing at bebop2), G12 roadmap staleness
(resolved).

**Accuracy check of the plan's claims (VERIFIED this session):** "385 workspace tests pass" —
TRUE at the plan's writing; 388 after the mid-session merges (both live runs). "Ed25519 RFC 8032
§7.1 bit-exact" — consistent with `sign.rs` header + KAT tests in the green suite. "ML-DSA-65
roundtrip+tamper, not-yet-NIST-bit-exact" — TRUE and, per §6 below, understated: it is not merely
un-vectored, it deviates from FIPS 204 in sampling and challenge size. "G9 wasm32 in-flight" —
now LANDED (§3). "Roadmap ALL-STUBS claim is STALE" — TRUE (pq_dsa 738 / kdf 616 / aead 473 /
hash 361 / sign 887 lines, all tested).

---

## 3. Protocol inventory — what exists for the delivery-hub role (code+tests vs paper)

| Primitive | Status | Evidence (file:line) | Notes for the hub program |
|---|---|---|---|
| Deterministic matcher | **CODE + TESTED** (VERIFIED) | `crates/bebop/src/matcher.rs:74` `match_orders` pure fn; `:100` `fingerprint`; `:127` `MatcherClient` trait; `:149` `Transport` trait; tests `:274` replicable-no-hidden-server, `:306` remote==local | Real and well-shaped, but `Order = {id, src:usize, dst:usize}` (`:34-38`) — no items/price/menu/customer; costs are `f64` (money stays outside per C5); graph indices, not identities |
| Proof-of-delivery | **CODE + TESTED** (VERIFIED) | `pod.rs:73` `sign_delivery` (refuses misattribution), `:88` `verify_delivery`; claim bound to ts+loc; 4 RED+GREEN tests `:113-165` | Pseudonymous attribution over the vault hybrid sig. Physical-handoff binding remains contestable-by-design (`PROTOCOL-CENTRALIZATION-MAP.md:143-152`) |
| Reputation | **CODE + TESTED** (VERIFIED) | `reputation.rs:29` in-memory `HashMap` ledger; `:69` score (suspension ⇒ floor 0), `:85` risk premium, `:55` sticky-suspension decay | Purely local, volatile; no persistence, no cross-node merge/sync |
| Conservation ledger | **CODE + TESTED** (VERIFIED) | `ledger.rs:79-81` Σbalance==0 invariant; `:38` content-addressed `transfer_id` idempotency; 5 RED+GREEN tests | Header is honest: *"in-process only"* (`:13`) — an invariant kernel, not a datastore |
| zkVM boundary | **CODE + TESTED, honest mock** (VERIFIED) | `zkvm.rs:61` `cross`, `:94` `verify_with`; `Proof::Stark` **fails closed without an injected verifier** `:104-107` | *"NOT a full zero-knowledge proof system"* (`:10-13`) — a hash-commitment seam |
| Consensus kill-switch | **CODE + TESTED** (VERIFIED) | `guard.rs:107-113` ≥2/3 supermajority suspension | No central off-button |
| Transport "zenoh" | **STUB / stand-in** (VERIFIED) | `zenoh.rs:1-10` self-describes as *"local broker stand-in… A real Zenoh (`zenoh` crate) would implement the same `Mesh` trait"*; **`zenoh` absent from Cargo.lock** (grep = 0) | Process-local pub/sub only. There is **no real network transport anywhere in the workspace** (no HTTP/TCP/libp2p code in `crates/bebop` or `rust-core`; grep = 0) |
| PQ hybrid identity/vault | **CODE + TESTED — via audited host crates** (VERIFIED) | `vault.rs:3-7` ML-KEM-768⊕X25519, ML-DSA-65⊕Ed25519, Argon2id (scrypt already retired — `crates/bebop/Cargo.toml:31` "replaces scrypt"), XChaCha20-Poly1305; both sig halves required `:169`; self-cert id `:115`; deps = RustCrypto `ml-kem 0.3`/`ml-dsa 0.1`/`ed25519-dalek 3`/`argon2 0.5` (`Cargo.toml:29-40`) | The value path today runs on **published, audited crates, not bebop2's hand-rolled code** — the swap (blueprint build-order step 2) has NOT happened. MAP:118's "XChaCha20+scrypt" is stale in the doc, not the code |
| bebop2 zero-dep crypto core | **CODE + KAT-TESTED (91 tests), zero deps** (VERIFIED) | `bebop2/core/Cargo.toml` (no deps); `sign.rs` Ed25519 RFC 8032; `pq_kem.rs` ML-KEM-768 (coefficient-domain, `:446,:576`); `pq_dsa.rs` ML-DSA-65 (roundtrip green since `fb4e651`); `kdf.rs` Argon2id RFC 9106 §5.3 KAT; `aead.rs`/`hash.rs`/`rng.rs` | All committed now (STALE-SINCE-AUDIT vs "staged crown jewels"). PQ pair still non-FIPS-interoperable — §6 |
| wasm32 empty-import gate | **NOW PASSES** (VERIFIED live, post-merge `0d3cd19`): `cargo build -p bebop2-core --target wasm32-unknown-unknown --no-default-features` compiles, and the produced `bebop2_core.wasm` has **no import section at all** (byte-level check this session) | `bebop2/core/Cargo.toml` new `std`/`host` features (f64 analytics gated out of the no_std build) | STALE-SINCE-AUDIT (the "fails ~94/105 errors" state died mid-session). Remaining: bit-exact KAT execution under wasmtime + the `reloop` harness are still absent (CLAIMED next step, no code) |
| Delivery-protocol design docs | **DESIGN ONLY** (VERIFIED read) | `docs/design/delivery-protocol/`: PROTOCOL-CENTRALIZATION-MAP (5 dangers, weakest-link admission), MATCHER-API (JSON contract spec), DECOUPLED-MATCHER (actor graph, C1–C5 risk grades, economics), SYSTEM-ARCHITECTURE-AUDIT ("trust, not interface, is the blocker") | High-quality anti-re-centralization doctrine; matcher half is coded, settlement/DLT/access-layer halves are paper |
| fable-protocol review F1–F4 | **DESIGN ONLY** (untracked) | `docs/design/fable-protocol-2026-07-11/` | F1: 0%-fee is poetry, moat = local reputation graph; F2: dispute machine spec (zero code); F3: ~70% real / 30% poetry; F4: storefront/marketplace/liveness gaps |
| Dispute/arbitration code | **ABSENT** (VERIFIED: grep `dispute|arbitrat` in all Rust src = 0) | F2 spec only | Open since F2 |
| Runnable delivery-node binary | **ABSENT** (VERIFIED) | `bebop` binary = the coding-agent CLI (TUI/MCP/guard); `main.rs`/`cli.rs` contain **zero** references to matcher/pod/delivery | The delivery primitives are library modules with tests; nothing executes them as a node |
| SQLite / any DB | **ABSENT** (VERIFIED: grep `sqlite|rusqlite` across all .toml/.rs = 0) | — | The program's SQLite-on-device layer has no substrate in bebop-repo at all |

---

## 4. Gap table — what the unified plan + code still lack for real vendor/courier/customer nodes

Severity is judged against the plan's OWN definitions: MVP = single-vendor owner hub with
own couriers, seams only (C8/D6 gate); Long-term = open multi-node protocol.

| Gap | State today (VERIFIED unless noted) | MVP severity | Long-term severity |
|---|---|---|---|
| Transport / discovery | No network code at all; `zenoh.rs`/`InMemoryTransport` are in-process stand-ins; libp2p-vs-Zenoh explicitly deferred | **LOW** — plan bans it from MVP (C8); the transport-agnostic seam exists and is test-proven faithful (`matcher.rs:306`) | **CRITICAL** — nothing peers can actually talk over; discovery/bootstrap entirely undesigned beyond "DANGER #2" warnings |
| Node runtime | No runnable node binary for any role; primitives are libraries; bebop2's `kernel/ cli/ reloop/` still don't exist (dirs absent) | **HIGH** — someone must ship a process that runs on the vendor's device; plan assigns this to dowiz-core+hub UI (outside this repo), so bebop-side severity is MED but program-level HIGH | **CRITICAL** — vendor/courier/customer nodes are the product |
| Storage layer | Zero SQLite/DB; ledger+reputation in-memory; vault = one JSON blob on disk | **HIGH** — the program's stated substrate (SQLite on devices) exists in neither half today; event-log persistence is dowiz-side | **HIGH** — plus multi-node durability |
| Sync / replication | None; CRDT merge deferred (C8); no signed-event-log replication code; "decentralization reachable for FREE from the signed event log" is a design invariant (C4), not code | **MED** — single-device MVP tolerates it; per-event hash+sig seam must actually land | **CRITICAL** — partition-resilient reputation merge and settlement are named fail-closed-by-design but unimplemented (plan-audit (c)) |
| Settlement / payout | 0 lines (plan G4); threshold-sig verifier is design | **MED** — MVP checkout money flows through the existing hub, not the protocol | **CRITICAL** — "vendor gets paid on PoD" has no code; single-oracle trap (DANGER #3) if rushed |
| Dispute / arbitration (F2) | Zero code; fail-closed state machine specified; UMA/Kleros integration alternative | **MED-HIGH** (plan's own G8 rating) — even a single-owner MVP needs a refund/hold story | **HIGH** — protocol trust depends on it; PoD is contestable by design (G7) |
| Key recovery / backup for non-technical users | Vault = passphrase→Argon2id; **key-loss = identity-loss** (self-cert identity is "recoverable ❌", plan-audit (c)); social-recovery only mentioned in DECOUPLED-MATCHER C4 | **HIGH** — real vendors/couriers WILL lose passphrases; a value-bearing identity with no recovery is an MVP support disaster | **HIGH** — plus Sybil interaction (stake-bonded re-issuance is design-only) |
| Storefront / menu / hours (G1) | `Order` has no items/price | **HIGH (plan's own MVP-blocker rating)** — dowiz side owns this | MED |
| Courier marketplace / reassignment / liveness (G2/G3/G6) | Absent | **LOW-MED** — MVP is single-owner dispatch by definition | **HIGH** — 50%-courier-drop resilience |
| wasm32 empty-import proof (G9) | **CLOSED mid-session** — no_std build green, import section empty (VERIFIED); wasmtime bit-exact KAT run + `reloop` harness still missing | **LOW-MED** (residual: prove KATs bit-exact *inside* wasm) | MED |
| PQ interop + assurance (G10 + G09) | See §6 | **HIGH** — gates minting any protocol identity | **CRITICAL** for standard-interop peers |
| Economics (G5) | 1–3% + sinks decided in doc; nothing modeled | LOW | MED-HIGH |

---

## 5. Doc/plan risks worth flagging

- **The north-star blueprint is an untracked file** on one machine (as is `plan-audit-bebop`,
  the F1–F4 set). One `rm -rf` from oblivion. Same class of exposure G08 flagged for code —
  now migrated to the planning layer. (VERIFIED untracked.)
- The blueprint depends on dowiz-side claims (L0 "DONE", 0b-3/0b-5) it quotes from
  `plan-audit-memory-2026-07-11.md` — CLAIMED here, for other lenses to verify.
- Matcher money surface: matcher costs are `f64` while C5 mandates integer money — fine while
  the matcher only proposes routes, but the boundary must stay explicit when settlement lands.
- README/AGENTS.md were refreshed in `c977ea6` (19:13) but bebop-repo's `docs/ARCHITECTURE.md`/
  `CHANGELOG.md` TS-era drift (G08-d) was not re-checked this session — likely still stale.

---

## 6. Assurance constraints inherited from G09 — re-verified against the CURRENT tree

G09's core finding was: bebop2's PQ set is **not FIPS-interoperable and un-audited**, so no
bebop2 primitive may solely guard value. Status today:

1. **ML-KEM-768 still non-interoperable** (VERIFIED): `ek/dk` stored in the **coefficient
   domain**, not the NTT domain FIPS 203 mandates (`pq_kem.rs:446`, `:576`; header `:33`
   self-describes "schoolbook-coefficient-domain"). Bytes will never match ACVP or the
   `ml-kem` crate. Unchanged since G09.
2. **ML-DSA-65 still a bespoke scheme, not FIPS 204** (VERIFIED at final HEAD `57b1c9a`):
   despite the roundtrip-green fixes (`fb4e651`/`0567003`) and the just-merged drift-guard KAT
   (`754c7d4`, whose own commit message says "NOT FIPS bit-exact"), the matrix **A is still
   CBD-sampled** (`pq_dsa.rs:234` uses `sample_poly_cbd`; FIPS 204 requires uniform RejNTTPoly)
   and the **challenge is still 32 bytes** (`pq_dsa.rs:476 c_t: [u8; 32]`) vs the required
   λ/4 = 48 (`:38 LAMBDA=192`). The merged KAT freezes the *current bespoke* behavior against
   drift — useful, but it is a determinism anchor, not interop.
3. **Timing hotspots persist** (VERIFIED): Ed25519 `scalar_mul` is still double-and-add with a
   secret-dependent `if bit == 1` branch (`sign.rs:666-680`) even after the `c977ea6`
   limb-arithmetic rewrite (which addressed allocation/carry, not the ladder). KyberSlash-class
   division on secret-derived data in `pq_kem` compress/decompress was not re-audited but the
   module structure is unchanged.
4. **No external audit, no Wycheproof, no ACVP** — unchanged (grep Wycheproof = 0 at G09 time;
   nothing in the delta touches test-vector infrastructure).

**What this means for value-bearing identities in the MVP (the G09 posture, restated and still
binding):** any identity that can sign a PoD, hold reputation, or release money must keep an
**externally-audited half that alone suffices** — i.e. the hybrid PQ⊕classical design where the
classical/audited side is a published, scrutinized implementation. Today that posture is already
satisfied *by accident of sequencing*: `vault.rs` uses audited RustCrypto host crates for BOTH
halves (VERIFIED, `Cargo.toml:29-40`), and the bebop2 swap has not happened. The blueprint's
build-order step 2 ("re-point vault/pod at bebop2") **must not be executed as a wholesale swap**:
per G09's tier policy, bebop2 primitives are Tier-0 research today; the interoperable set
(SHA/AEAD/Ed25519/Argon2id) can earn Tier-2 (hybrid half) via the differential ladder, and the PQ
pair is capped at "PQ half of a hybrid whose audited half holds" until interop re-derivation
(G09-D1, ~15-25d) plus external audit. The unified plan's own G10 gate ("ACVP oracle before
protocol keys are minted") is consistent with this — the plan and G09 agree; neither is satisfied
by the current tree.

---

## 7. Verdict — what bebop2 can honestly carry TODAY vs in 3 months (≤10 lines)

TODAY (VERIFIED): a green 388-test library — pure replicable matcher, PoD attribution, reputation,
conservation ledger, consensus kill-switch, audited-crate hybrid PQ vault, and (as of tonight) a
**wasm32 no_std crypto core with a byte-verified empty import section** — plus an honest
anti-centralization design corpus and a coherent unified plan. It can carry the **crypto/identity
seam and matcher/PoD logic of the MVP hub** as embedded libraries. It CANNOT yet carry a single
real node: no network transport, no node binary, no database, no sync, no settlement, no dispute
code; the PQ pair stays non-interoperable and unaudited (hybrid-only). IN 3 MONTHS (plan steps
2-5 at tonight's velocity): ACVP-interoperable ML-DSA, vault re-pointed under the G09 hybrid tier
policy, an open matcher + threshold-settlement skeleton — the protocol *backbone*, still short of
vendor/courier/customer node runtimes, which no plan line staffs. Trust its invariants; don't schedule anything that needs a wire.
