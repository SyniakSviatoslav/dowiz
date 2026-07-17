# SYSTEM-BREAKER — Agent Exchange Plane safety stress-test (2026-07-17)

> Adversarial read of B1 (`AgentBridge`), B2 (`WorkReceipt`+`Settlement`), B3 (`ExposureLedger`)
> and B4 (crypto bench). Read-only against code. **This document names breaks and rates them; it
> proposes no fixes.** Where a blueprint already names and bounds an issue, that is stated and the
> would-be finding is withdrawn — see the "Already adequately addressed" section.
>
> Live code re-read to ground the mechanics: `kernel/src/event_log.rs:293-419`
> (`append`/`commit_after_decide`/`_drift_gate`), `kernel/src/token_bucket.rs:34-89`,
> `kernel/src/hydra.rs:253-348` (`boot_verify`, `ingest_peer_breach`).

Findings ranked most-severe first. Two of the six probed areas turned out partly-sound and are
credited, not padded into findings.

---

## F1 — [HIGH] B-CONSIST/B-FAIL · Cross-node DvP atomicity rests on *unsynchronized* local ticks; the safety margin is denominated in a quantity the substrate never bounds

**The within-log question is answered in the design's favour — credit first.** B2's claim/refund
cannot both succeed *inside one node's log*: `commit_after_decide` (`event_log.rs:339-361`) is
single-writer, dedups by content-id *before* re-running `decide` (`:350-351`), runs `decide` as a
pure fold to a terminal `{Settled, Refunded}` state, and persists nothing on rejection
(`:355-356`). A `SettlementClaimed` and a sweep-emitted `SettlementRefunded` arriving "both at tick
T" serialize through the one commit surface; whichever folds first reaches a terminal, the second
is a Law-reject. There is no intra-log double-resolution race. B2 §2.5's "safety never depends on
the sweep" is correct for the local tick-comparison.

**The break is the layer the design asserts but does not build: cross-node atomicity.** A
settlement has two legs in two logs on two nodes. B's receipt encumbrance voids at `lock_tick + Δ`
on **B's** tick; A's payment escrow refunds at `offer_tick + 2Δ` on **A's** tick. B2 §2.3 states
the margin "`2Δ − Δ = Δ` must absorb inter-node tick skew + RTT + verify + commit" — but names **no
bound on skew and no synchronization primitive.** `capability.rs:57-59` (cited by B2) defines the
tick as "a unix-ish monotonically-increasing counter … the caller supplies a comparable tick":
*comparable* is asserted, not enforced. Nothing in B1–B4 requires A's and B's tick **rates** or
**origins** to match; ticks are explicitly not wall-clock and there is no NTP/epoch anchor.

**Concrete break sequence (conforming B still loses):**
1. A (payer, secret-holder) offers; escrow refunds at A-tick `offer_tick + 2Δ`.
2. B locks its receipt; delivers output bytes; encumbrance voids at B-tick `lock_tick + Δ`.
3. A commits `SettlementClaimed{s}` (accepts the work, reveals `s`) just inside its window.
4. B, behaving optimally, pulls `s` and commits its payment-claim `(Ledger, SettlementRecorded)`.
5. **A's tick source runs at 2 ticks/s, B's at 1 tick/s** — both monotonic, both "valid" under the
   spec. A's `2Δ = 120` ticks elapses in 60 wall-seconds; B's `Δ = 60` ticks is 60 wall-seconds.
   B's supposed `Δ` margin has evaporated. B's payment-claim lands at A-tick `≥ 2Δ` → A's `decide`
   tick-rejects it (`now ≥ deadline`) → A's sweep-on-commit refunds the escrow to A.
6. **Terminal state: A holds accepted work *and* its money back; B did the work and is unpaid.**
   Each leg resolved by a locally-valid pure `decide`; no node did anything "illegal."

This is exactly the outcome DvP exists to make impossible (R5 §5 / Herlihy PODC 2018). It is *not*
the deliberate free option of caveat 1 (A chose to walk) — here A *claimed* and a conforming B
still loses. The Herlihy argument in B2 §2.3 bullet 3 ("B presents `s` within the remaining `≥ Δ`
margin … claimable by construction") silently assumes a shared clock; the margin is real only if
`tick_rate_A ≈ tick_rate_B`, which no invariant guarantees.

**Violated invariant:** SYNTHESIS §2.4 "Settlement finality = both halves in both WORM logs" and
the Herlihy no-conforming-party-worse-off guarantee. **Harm:** direct fund/value loss, per-
settlement bounded by the settled amount. Conditional on effective skew exceeding the margin — but
because skew is *unbounded by design*, the only defense is an operator sizing Δ against an
unmeasured quantity. Highest harm ceiling in the arc; rated HIGH rather than CRITICAL only because
money-legged settlements additionally sit behind the armed red-line gate and per-settlement value
is capped.

---

## F2 — [HIGH] B-SCALE/B-FAIL · Admission-time verification is unmetered: cheap-to-forge, expensive-to-verify frames exhaust the node's verify budget *before* any TokenBucket exists

B1's rate-limit story (F2 anchor) is the **per-agent `TokenBucket` minted at admission** (§2.2 step
5, §2.4). That bucket does not exist for an *unadmitted* identity — it is created only *after* a
successful `admit()`. Every unsuccessful admission attempt therefore pays no toll. There is **no
rate limit on admission attempts anywhere in B1** (grep of §2.4/§3/§4: only post-admission dispatch
is bounded; no pre-admission limiter is named).

**The cost asymmetry is the weapon.** Anyone can *construct* a manifest frame — the bytes are free.
The **verifier** pays `HybridGate::check` to discover it is invalid: `verify_chain` does one real
Ed25519 verify **per delegation-chain link** (B4 §1 item 2: "cost scales with chain depth"), plus a
real Ed25519 (`:171`) and, if reached, a real ML-DSA-65 verify (`:180-186`) at B4's own estimate of
~0.1–1 ms and unbenchmarked (B4's entire reason to exist). Chain length at verify time is **not
bounded** (B1's depth cap of 3 governs `InvokeAgent` dispatch depth, a different quantity).

**Concrete break:**
- Attacker streams garbage manifest frames with long anchor-shaped delegation chains. At B4/R4's
  ~10³ verify/s/core, **~1,000 frames/s saturates one core**; a multi-link chain multiplies the
  per-frame cost, so a few hundred fabricated frames/s can pin a core. Frames are free to mint.
- Worse than pure CPU burn: `HybridGate::check`'s nonce insert is under a **shared `Mutex`** (B4 §1
  item 2b, `hybrid_gate.rs:193-206`). The flood contends the same mutex **legitimate** hybrid
  verification needs, degrading *all* verification node-wide, not just the attacker's own lane.
- The mesh is sparse (R2 §2, few peers), so a single attacker can target and starve one node's
  admission path with no admitted identity and no valid signatures.

**Violated invariant:** F2 "deny-by-default **+ rate-limit**" — the rate-limit half is unmet for
the pre-admission path. **Harm:** unauthenticated liveness DoS on the node's core trust primitive;
no fund loss, self-limited only by whatever MESH-09 transport does (out of scope, unspecified here).
Genuine blind spot — not named in B1 or in the consolidation's §5 honest-gap audit.

---

## F3 — [HIGH] B-SEC · Per-peer exposure cap is defeated by identity fragmentation; no Sybil resistance exists and both candidate defenses were deliberately rejected

B3 keys exposure on `PeerId = NodeId = SHA3-256(pq_pub ‖ classical_pub)` (§2.1). One operator/
sub-hub can mint arbitrarily many keypairs → arbitrarily many distinct `NodeId`s → arbitrarily many
distinct `PeerId`s. B1 admission checks a signature and an **anchor-rooted chain**; it does **not**
limit how many identities root under one anchor, and delegation lets a sub-hub mint *breadth*
(the depth cap of 3 constrains chain length, never fan-out). Nothing in B1's `admit()` or B3's
ledger provides Sybil resistance for this fragmentation.

**Concrete break:** `default_per_peer_cap` = P, `aggregate_cap` = A, with A ≫ P by construction
(the aggregate must accommodate many honest peers simultaneously). An attacker mints
`K = A / P` identities, opens up to P outstanding on each, and reaches the **whole aggregate cap
alone**. The property R5 §3 / Knight Capital demanded — "one bad peer must not consume the node's
whole headroom" — is exactly defeated: the per-peer cap becomes fiction, the only surviving bound
is the aggregate, and one actor holds all of it, **starving every honest peer** (`try_commit`
returns `AggregateCapExceeded` / regime → `Paused` for everyone; see F4).

**Why nothing catches it:** B3 §5 explicitly bans reputation ("never history-scored") and the arc
bans stake/tokenomics wholesale (SYNTHESIS §3.5). Sybil resistance without cost-to-identity or
reputation is unsolved; the design has closed both doors and named no third. Burnt-peer zeroing
(§2.5, via `ingest_peer_breach` → `append_raw`, `hydra.rs:346`) never fires here — Sybil identities
commit no *breach*, they behave "conformingly" up to the cap. The only real throttle would be
operator-gated per-`NodeId` enrollment, which B1 does not require (admission is signature +
anchor-rooted chain, not per-identity operator approval).

**Violated invariant:** R5 §3 per-counterparty isolation ("must be per-counterparty … in the order
path"). **Harm:** total financial exposure stays aggregate-bounded (good — no theft), but the
per-peer isolation guarantee is broken and honest peers are deniable-of-service. Genuine open gap.

---

## F4 — [HIGH] B-SCALE/B-FAIL · Concurrent-settlement grief: only *value* is capped, never *count*, and the 85% aggregate LimitState converts one griefer into a node-wide transaction freeze

B2 caveat 2 honestly bounds the *per-settlement* and *per-peer* grief: capital lock ≤ `2Δ`, total
griefable ≤ per-peer cap × `2Δ`. It does **not** bound concurrency or third-party collateral. B3
stores open commitments in `open: BTreeMap<[u8;32], Commitment>` (§2.1) with the only limits being
`peer.outstanding ≤ cap` and `aggregate_outstanding ≤ aggregate_cap` — **value sums, never a count.
There is no cap on how many concurrent settlements one counterparty may open.**

**Concrete break (single actor, no Sybil needed):**
1. Attacker opens many small settlements against victim V, each within per-peer value cap, driving
   V's `aggregate_outstanding` toward 85% of `aggregate_cap`.
2. At `aggregate_outstanding × 20 ≥ aggregate_cap × 17` (B3 §2.3, `HIGH_WATER = 17/20`), regime
   flips to `LimitState` and `try_commit` refuses **every new commitment from every peer** with
   `Paused` — "in-flight settles freely," but *no honest peer can open anything new*.
3. The attacker controls exit: by stalling each settlement to `2Δ` it controls how long exposure
   stays elevated, and reopens as refunds land. Auto-reopen needs both `≤ 70%` **and**
   `LIMIT_DWELL_TICKS` dwell — the attacker simply keeps topping back above 70%, so the node never
   reopens. **V is frozen out of transacting with anyone for as long as the attacker chooses.**
4. Amplified by F3: with Sybil fragmentation a single actor reaches the 85% trip *by itself*,
   spread across identities each below the per-peer cap.

Secondary amplifier: K tiny concurrent settlements bloat the `SettlementBook` min-heap; near a
common `expiry_tick` they expire together, and B2's sweep-on-commit must commit K
`SettlementRefunded` events — each a full `decide` fold — in a burst on the commit path.

**Violated invariant:** the same R5 §3 per-counterparty containment as F3, plus R5 §1's LULD
LimitState (designed as "friction before wall") weaponized into a node-wide amplifier. **Harm:**
renewable operational DoS on the victim's ability to transact; self-heals only when the attacker
stops and dwell elapses. No theft. HIGH.

---

## F5 — [MEDIUM] B-CONSIST/B-FAIL · Crash mid-propagation: local recovery is clean, but the "both halves in both logs" finality bar has no automatic cross-node reconciliation

**Credit the local half.** `boot_verify` (`hydra.rs:253-265`) is **not** a settlement replay — it is
a spectral covert-persistence check (`ρ < 1` or hard-stop "re-seed from golden"). Settlement state
is reconstructed by folding the durable WORM log (`fold_exposure` B3 §2.4, `settlement_machine`
B2 §2.4), which is deterministic. The commit path's durability barrier is `insert`'s `?`
(`event_log.rs:309`) *before* `set_tip` (`:310`); a crash between them leaves the event present but
tip lagging, which self-heals because the next `append`/`commit_after_decide` dedups on
`contains(id)` (`:303`, `:350`). So a node that crashes after `decide` accepts but before durable
`insert` simply never recorded the event; after durable `insert` it replays cleanly. **There is no
undescribed *local* corruption window** — the task's worry about `boot_verify` leaving an
ill-defined local state does not materialize.

**The undescribed window is cross-node.** If V crashes *after* durably committing its half (e.g.
`SettlementClaimed`, `s` revealed) but the counterparty P never receives it (send not yet made, or
in flight), V recovers into `Settled` locally while P sits in `Locked`. Reconciliation depends
entirely on best-effort Sync·Pull anti-entropy (MESH-07) racing the `2Δ` tick deadline. If the
outage outlasts that race — V down past P's ability to pull `s` before A's escrow refunds — the two
ledgers **permanently disagree** (one leg `Settled`, the other refundable), collapsing to the same
value-loss as F1. The only described resolution is the **F44 dispute machine**, which is *manual
operator arbitration* ("arbiter per operator ruling O3"), not an automatic path. B2's Herlihy
argument assumes synchrony; it does not analyze crash-during-propagation.

**Violated invariant:** SYNTHESIS §2.4 finality bar has no liveness guarantee under crash+partition.
**Harm:** per-settlement bounded value loss, with a stated (manual, slow) F44 backstop and both
half-logs as evidence. Shares F1's root (no cross-node atomicity); rated MEDIUM because local
recovery is genuinely clean and a named — if manual — reconciliation exists.

---

## F6 — [MEDIUM] B-SEC · Shared-process WASM admits mutually-distrusting agents into a shared cache and shared CPU; key-theft is closed, cross-agent confidentiality is not

**Verdict, no hedge: the side channel is real and currently unaddressed for the multi-tenant case;
the design's isolation model closes key-theft but not cross-agent inference.**

B1 §2.2 step 4 states WASM is "an integrity boundary, not confidentiality," and closes the *specific*
confidentiality concern it names — signing keys ("node signing keys never enter the guest address
space — the host signs, the guest only computes"). That is genuine and worth crediting: an admitted
agent cannot exfiltrate signing material. **But B1's whole premise is admitting third-party agent
code from potentially mutually-distrusting operators, and WASM components run in-process in
Wasmtime by default** (`SandboxTier::WasmComponent`, the default tier; the alternative
`NativeProcessRequiresKvm` is the one that gets a separate address space). The design never states
per-operator process separation for the default tier, nor per-agent cache partitioning.

**Concrete leak — shared `CachingBackend` timing oracle.** B1 §2.3 reuses `CachingBackend<B, S>`
"verbatim," keyed by `sha3_256` of the canonical request (`cache.rs`). If two bridged agents share a
cache store, agent X submits request R and measures latency: a **hit (fast)** reveals that some
co-resident agent (or X earlier) issued the identical R — a cross-tenant request-content existence
oracle, purely via timing, no memory read required. The blueprint neither partitions the cache per
agent nor forbids adversarial co-residence.

**Secondary, lower-value:** each agent's fuel tranche is fixed instruction count; measuring the
*wall-time* a tranche takes (fuel/actual-throughput) leaks co-tenancy CPU contention — a coarse
"someone else is busy" signal, not their data. And the shared nonce `Mutex` (F2) leaks node-wide
verification volume via contention timing.

**Violated invariant:** none stated — the design *concedes* "not confidentiality." The finding is
that it concedes the boundary without analyzing that its own use case (multi-operator agent
admission into a shared process/cache) *is* the multi-tenant confidentiality scenario, and applies
no scoping (no per-tenant cache, no co-residence rule). **Harm:** inference/timing leakage between
tenants; requires adversarial co-residence; no key or integrity compromise. MEDIUM.

---

## Already adequately addressed (not manufactured into findings)

- **Intra-log settlement double-resolution (the literal Area-1 question).** Decide-before-persist
  *does* prevent both claim and refund succeeding within one log: single-writer commit surface,
  dedup-before-decide (`event_log.rs:350`), pure `decide` to terminal state, nothing persisted on
  reject (`:355-356`). No same-log race exists; F1 attacks the *cross-node* clock assumption, a
  different and genuine gap.
- **Local crash recovery (the Area-6 local half).** `boot_verify` + deterministic log-fold +
  `insert`-before-`set_tip` durability barrier + `contains`-dedup give a well-defined local state
  after any crash; F5 is scoped to the cross-node divergence only.
- **WASM signing-key confidentiality (the closed part of Area-5).** Host-held keys with guest-only
  compute genuinely closes key exfiltration; F6 is scoped to cross-agent cache/timing inference.

---

*System-breaker pass, 2026-07-17. Read-only against `feat/agentic-mesh-protocol-2026-07-17`; code
mechanics verified live in `event_log.rs`, `token_bucket.rs`, `hydra.rs`. No fixes proposed, no
product code or sibling document edited.*
