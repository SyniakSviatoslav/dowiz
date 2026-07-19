# Energy / compute-expenditure as an INTERNAL resource-allocation or anti-Sybil currency — scan (2026-07-18)

> RESEARCH-ONLY. No code written, no branch touched. Question under test (operator, 2026-07-18):
> *"what about energy used instead of currency?"* — explicitly **not** for customer/courier money
> (that stays `money.rs` exact `i64`, untouched — out of scope), but whether **energy / compute
> expenditure** could serve as an **internal** resource-allocation, rate-limiting, or anti-Sybil
> unit somewhere in dowiz/bebop2.
>
> Method: read the live code (`file:line` given for every load-bearing claim), then the design
> corpus that already adjudicated adjacent questions. **Epistemics honesty:** the session WebSearch
> budget was exhausted (200/200) before this scan's searches ran, so the external precedents below
> are stated from **established, textbook-stable CS knowledge** and flagged `[KNOWLEDGE]`, not from a
> fresh fetch this pass. They are not load-bearing for the verdict — the verdict rests on the
> in-repo code and the already-run Batch-7 adjudication, both `[VERIFIED-CODE]` / `[VERIFIED-DOC]`.

---

## §0 — Verdict (read first)

**The idea is real and has real precedent, but for dowiz's actual internal surfaces it is either
(a) already-realized in a better form, or (b) already-evaluated-and-rejected with citations.**
There is **no concrete near-term target** where "energy/compute as an internal currency" beats what
is already on disk. One genuinely open, non-redundant nuance survives — a hashcash-style *client
puzzle* as an anti-DoS layer on the **unauthenticated pre-crypto admission surface** under a
source-spoofing flood — and even that is likely dominated by the existing global rate ceiling and
imports a regressive cost. It is flagged, not recommended.

Three findings, each concrete:

1. **Compute-as-accounting-unit is ALREADY the design** for agent execution — the B1 agent budget is
   denominated in **Wasmtime fuel = CPU instructions** (`FUEL_PER_UNIT`, `admission.rs:50-59`),
   i.e. an Ethereum-gas-shaped compute meter, converted to token-bucket units at the edge. Nothing
   to add; it is pending a bench (B4) to *ground the constant*, not a missing concept.
2. **Compute-cost accounting for external GPU is ALREADY `budget.rs`** — a degrade-closed spend
   accountant (`ComputeBudget`, `budget.rs:86-121`) against a monthly ceiling. It already denominates
   real compute cost. It is not money and not a rate-limiter; reframing it as "energy currency" adds
   nothing it does not already do.
3. **As a Sybil-resistance mechanism, proof-of-work was EXPLICITLY evaluated and REJECTED** in
   `16-BATCH7-sybil-proof-capability-mechanism-findings.md §4` (candidate 4 of 4), superseded by
   asymmetric anchor-rooted issuance (`verify_chain`) + a per-anchor **counter** budget
   (`IssuanceBudget`, already built, `node_id.rs:187-375`). This is not a gap — it is a closed
   decision.

---

## §1 — Real-world precedent (grounding the operator's framing) `[KNOWLEDGE]`

The operator's instinct is historically sound; "compute cost as a currency for anti-abuse" is a real,
working lineage:

- **Hashcash (Adam Back, 1997)** — the original anti-spam proof-of-work: each email carries a partial
  SHA hash preimage costing measurable CPU to find, cheap to verify. The archetype of "impose a
  compute cost per request to deter bulk abuse." A *client puzzle*.
- **Bitcoin PoW (2009)** — security literally *backed by* measurable energy (hash power); the chain's
  Sybil-resistance is that rewriting history costs proportional real electricity.
- **Ethereum "gas"** — compute-metering as an internal accounting unit denominated in a resource
  cost, priced to real currency only at the edges. dowiz's fuel meter (§2.1) is this shape.
- **Proof-of-space / spacetime (Chia; Filecoin PoRep/PoSt)** — substitute *storage* for *energy* as
  the scarce resource, to escape PoW's energy waste while keeping asymmetric cost.
- **Proof-of-elapsed-time (Intel PoET)** — substitute *trusted-hardware-attested wait* for compute.

The through-line the literature also establishes (Douceur 2002; Cheng–Friedman 2005): a compute/energy
"entry fee" is one valid Sybil defense, but it is the **regressive** one — it excludes legitimate
low-resource newcomers as much as attackers (Friedman–Resnick), and a well-resourced attacker
out-computes honest small participants. The *asymmetric authorized-issuance* branch (a trusted root
gates who counts) is the other valid defense and does not have that cost. **This exact tradeoff is
what Batch 7 turned on (§4 below).**

---

## §2 — The concrete in-repo surfaces, each read fresh this pass

### §2.1 Agent execution budget — compute-as-accounting-unit ALREADY EXISTS `[VERIFIED-CODE]`

`kernel/src/ports/agent/admission.rs` (the B1 AgentBridge admission path):

- `FUEL_PER_UNIT: u64 = 100_000` (`:50-56`) — "converts billing units → **CPU-instruction fuel**",
  explicitly Wasmtime fuel; the comment flags it a **placeholder pending B4's criterion bench** that
  will ledger-ground it (`agent-adapters/src/fuel.rs`).
- `TRANCHE_UNITS: u64 = 8` (`:58-59`) — prepaid tranche size for the fuel loop.
- The agent manifest carries `cost_denomination: CostDenomination::TokenBucketUnits`
  (`admission.rs:611`), and the admitted agent is minted a per-agent `TokenBucket` budget envelope
  (`AdmissionRecord.bucket`, `:320`; F2 rate-limit).

**Reading:** dowiz's agent-execution accounting is *already* "compute expenditure as the internal
unit" — fuel (CPU instructions) is the denominator, token-bucket units are the wire form, real money
appears only if/when Modal GPU is billed at the edge (§2.2). This is the Ethereum-gas idea, already
chosen. What is open is **grounding the `FUEL_PER_UNIT` constant against a real bench (B4)** — an
empirical calibration task, not a conceptual "should we use energy?" question. The operator's idea is
not un-adopted here; it is adopted and awaiting measurement.

### §2.2 `budget.rs` — what it actually denominates `[VERIFIED-CODE]`

Read in full. It denominates **offline Modal GPU/compute spend**, not money, not a rate:

- Module doc (`:1-17`): "P11 §1 ComputeBudget + §4 Modal `JobPort`… GPU/Modal compute is OFFLINE,
  behind a port, never in-kernel, never in the request path."
- `Job.estimate: f64` (`:49-52`) = "expected **monthly-spend units** the `BudgetedJobPort` debits".
- `ComputeBudget { spent: f64, ceiling: f64 }` (`:86-121`) — a degrade-closed accumulator: `debit`
  advances `spent` iff `spent + amount <= ceiling`, else refuses and records nothing (`:113-120`).
- `BudgetedJobPort` (`:131-184`) wraps it in a `Mutex`, refuses over-ceiling submits with
  `BudgetExceeded` and records no spend; NaN/negative/inf estimates are refused before any debit
  (`:161-163`, the `aa70d7fa6` hardening).

**So it is a compute-COST accountant already** — a monthly $ ceiling on real external GPU, in `f64`
"spend units". It is the closest thing in the tree to "compute as a budget," and it confirms the
pattern is already native. It is **not** money (`money.rs` is exact `i64`, untouched) and **not**
a currency that circulates — it is a one-directional degrade-closed meter. Reframing it as an
"energy currency" would add a name, not a mechanism.

> Note on the task's premise: the brief said "R-Contention-Bench just touched `budget.rs` today with
> a Mutex→AtomicU64 CAS fix." **That does not match git.** `budget.rs`'s last commit is `aa70d7fa6`
> (NaN/negative + lock-poison hardening); it still holds `Mutex<ComputeBudget>` (`:133`). The
> AtomicU64/CAS work is in a *different* file — `kernel/src/ports/agent/admission.rs`
> (`AtomicUsize` gate `check_count`, `:152,:192`) — which is the agent-admission path, not
> `budget.rs`. Flagged for accuracy; it does not change any finding.

### §2.3 `token_bucket.rs` — a rate-limiter, not a currency `[VERIFIED-CODE]`

Read in full. `TokenBucket { capacity, refill_rate, Mutex<Inner{tokens, last_refill}> }`
(`:26-30`). Monotonic-clock refill (`:48-60`); `try_acquire(n)` grants iff `tokens >= n` and
decrements (`:74-90`); verified-by-math over-grant invariant (`granted ≤ capacity + rate·elapsed`,
`:132-151`). Poison-recovery degrade-closed (`:65-73`).

**It has NO currency-like properties:** tokens do not persist as a transferable balance, do not
accrue value, and auto-refill to a cap (they are *not* spent-down-then-earned). It is purely a
rate/quota governor. It is the *substrate* for anti-flood (used two ways below) but is not itself a
resource-currency, and there is nothing currency-shaped to "unlock" in it.

### §2.4 B1 admission already has an anti-flood cost layer — rate-based, pre-crypto `[VERIFIED-CODE]`

`admission.rs` `AdmissionLimiter` (`:254-295`), "SH-1 Guard A": a **pre-cryptographic
admission-attempt limiter built entirely from `TokenBucket`** — a mandatory global ceiling
(`try_acquire(1.0)` per attempt) plus an optional fixed-size **sharded** array for bounded-memory
per-source fairness (`:285-294`), keyed by coarse `conn_id`. Test `crit11_flood_throttled_before_crypto`
(`:891-927`) proves 17 of 20 flood frames are dropped *before any signature verification*.

This is the exact anti-spam/anti-DoS role a hashcash puzzle would occupy — and it is **already built,
rate-based, and free to honest clients** (no puzzle to grind). See §3.4 for the one residual PoW
could theoretically address here.

### §2.5 `hybrid_gate.rs` + cert issuance — where a "cost of requesting a cert" would live `[VERIFIED-CODE]`

`bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs` `HybridGate::check` (`:124-209`) is admission-side
verification: freshness → anchor-rooted `verify_chain` → armed red-line deny → revocation → classical
Ed25519 → real ML-DSA-65 PQ → verify-then-record nonce. It imposes **no cost on the party requesting
a cert** — it is pure signature/authorization checking of an already-minted delegation.

The **issuance** side (the cost of *requesting* a new capability) lives in `node_id.rs` and is the
natural home the operator's question points at. It already has a scarcity gate — but a **counter**,
not a compute-cost. See §3.

---

## §3 — The anti-Sybil surface: already designed, and PoW already rejected

### §3.1 The standing mechanism `[VERIFIED-CODE]` `[VERIFIED-DOC]`

Sybil-resistance in the bebop2 mesh is **asymmetric anchor-rooted issuance**:
`verify_chain` (`roster.rs:252-316`) accepts authority only along a signed delegation chain rooted at
a genesis-frozen `AnchorRoster` anchor; N free keypairs are inert (`CapError::UnknownIssuer`).
Genesis is fail-closed (`load_genesis`, `node_id.rs:117-142`: missing/malformed/zero-anchor ⇒ error,
no authority captured). Root-delegation policy is an explicit operator enum
(`RootDelegationPolicy::{OperatorSigned, WebOfTrust, FirstContactQr, Unspecified}`, `:157-167`,
`Default = Unspecified`), resolved to **`OperatorSigned`** (MESH-12-RESOLVED, 2026-07-14; R-3 ruling
recorded, `BLUEPRINT-P-D-consensus-capability.md §11`).

### §3.2 The issuance budget IS a scarcity gate — but a counter, deliberately not a currency `[VERIFIED-CODE]`

`node_id.rs:187-375` (landed, `bebop-repo` commit `e08eb07`): `IssuanceBudget { anchor_id, epoch,
minted_count, max_per_epoch }` — a per-anchor, per-epoch **mint cap** enforced at delegation-sign
time by `sign_delegation_budgeted` (`:335`), with 10 RED→GREEN tests and a CI seam
(`scripts/ci-budgeted-issuance.sh`). Default `max_per_epoch = 1`, epoch = 86 400 ticks (`:199-203`).

The design **explicitly disambiguates this from a currency** (`BLUEPRINT-P-D §5`, R-2 note): *"it
counts signing ceremonies; it is **not** a currency, not transferable (`AnchorMismatch` pole is the
type-level enforcement), not a B2 money/compute budget"* — precisely to stay clear of COUNSEL's
"currency you call a budget" hazard and the money red-line. So the anti-Sybil scarcity unit was a
conscious choice of **counter, not cost**.

### §3.3 Proof-of-work was evaluated head-to-head and rejected `[VERIFIED-DOC]`

`16-BATCH7-sybil-proof-capability-mechanism-findings.md §4` scored four candidate cost sources
against three hard constraints (no courier-scoring; structural not watchdog; deployable on an
unprivileged Firecracker microVM). The operator's "energy instead of currency" maps directly onto
candidates 3 and 4:

| Candidate | Batch-7 verdict (verbatim thrust) |
|---|---|
| Anchor-rooted delegation (rate-limited trusted quorum) — EXISTING | **ADOPT (winner)** — asymmetric, structural, zero hardware |
| Hardware-attestation (TPM/device-bound) | **REJECT (physics)** — Firecracker is unprivileged, no TPM passthrough |
| **Real-world stake (refundable bond)** | **VIABLE-AS-OPTIONAL-HARDENING** but *drags in the money red-line* + excludes low-resource newcomers |
| **Proof-of-work** | **REJECT (weak + regressive)** — "taxes honest low-resource couriers (a phone should not grind hashes) as much as attackers… a moderately-resourced attacker out-computes honest newcomers — the exact Friedman-Resnick 'entry fee excludes legitimate newcomers' cost, on a shared 4-vCPU host. **Strictly dominated by the anchor mechanism.**" |

The §0 verdict of that doc names it directly: *"Proof-of-work is rejected (punishes honest
low-resource couriers, weak Sybil defense)."* This is a **closed, cited decision**, not an open gap —
the operator's question, applied to mesh admission, has already been answered *no*, and *why*.

### §3.4 The one residual where a client-puzzle is non-redundant (flag, not recommend)

The Batch-7 rejection is specifically about PoW as a **Sybil / identity-issuance** cost, where the
regressive "entry fee excludes newcomers" property bites the honest courier. There is a narrower,
distinct surface it did not fully close: **pure anti-DoS on the unauthenticated pre-crypto admission
path** (§2.4). There, a client cannot yet be rate-limited by *identity* (identity is the thing being
established), and the sharded `AdmissionLimiter` fairness degrades if an attacker **spoofs many
`conn_id`s** — the global ceiling still holds, but per-source fairness is diluted. This is the exact
niche the client-puzzle literature (hashcash, TLS/TCP SYN puzzles) targets: make the *attacker* pay
CPU per attempt before you spend a signature verification, with no standing identity.

Honest cost/benefit if anyone ever revisits it:
- **For:** attaches cost to a spoofable, identity-less flood surface where rate-limiting alone is
  weakest; it is a puzzle on *attempts*, not on *courier identity*, so the Friedman-Resnick
  newcomer-exclusion critique is softer (a legitimate one-time admission grinds one puzzle, not an
  ongoing tax).
- **Against:** (1) the mandatory global `TokenBucket` ceiling already bounds total pre-crypto work
  node-wide *regardless of source cardinality/spoofing* (`admission.rs:257-259` states exactly this
  as its design intent) — so the marginal benefit is small; (2) it still imports *some* regressive
  cost onto low-end courier phones at admission; (3) it adds a new primitive + tunable difficulty +
  a verification path, against dowiz's ponytail/YAGNI and "no new machinery" bias. **Net: dominated
  by the existing ceiling in the common case; only worth reconsidering if a real spoofed-source
  pre-crypto flood is ever observed in telemetry. Not a near-term target.**

---

## §4 — Documented spam/abuse/Sybil concern: where it actually lives `[VERIFIED-CODE]`

Searched `apps/api/src` for `rate.?limit|abuse|spam|throttle|too.?many` and the customer OTP path:
**no in-code rate-limiting primitive surfaced on the customer-facing API surface** (the product-side
OTP/order paths). The *documented, engineered* spam/abuse/Sybil concern lives entirely in the **mesh
/ agent-mesh layer** (Batch 7, P-D audit, B1 SH-1 guards) — i.e. node/agent admission, not customer
traffic. That is consistent with the operator's own framing: the internal resource-allocation /
anti-Sybil question is a *mesh* question, and the mesh already has both the flood guard (§2.4) and
the Sybil guard (§3.1–§3.2). The absence of a customer-API rate-limiter is a separate observation
(possibly a real product gap, but a *rate-limit* gap, not an *energy-currency* one — and out of this
scan's scope).

---

## §5 — Honest bottom line

- **Is "energy/compute as an internal unit" a real, precedented idea?** Yes (§1) — hashcash,
  gas, PoW/PoSpace are genuine working examples.
- **Does dowiz have a concrete internal surface where adding it would win?** **No.**
  - Agent-execution accounting *already is* a compute meter (fuel/gas, §2.1) — adopted, awaiting a
    bench, not a concept.
  - External-GPU cost *already is* a degrade-closed compute-spend accountant (`budget.rs`, §2.2).
  - Anti-flood on admission *already exists* rate-based and free-to-honest-clients (§2.4).
  - Anti-Sybil *already rejected PoW with citations* in favor of anchor-rooted issuance + a counter
    budget, deliberately kept non-currency (§3.2–§3.3).
- **The single non-redundant nuance** — a client-puzzle purely as anti-DoS on the spoofable
  pre-crypto admission surface (§3.4) — is real in the literature but likely dominated by the
  existing global ceiling and imports a regressive cost. Flagged for telemetry-triggered
  reconsideration only; **not** a recommendation to build.

This is, honestly, mostly a **settled / already-realized** design space rather than an open one —
which is itself the finding, and is consistent with this session's research discipline: the operator's
question is good, and the reason it does not open new work is that the codebase already walked this
path and chose (or already occupies) the better-dominated option at each surface.

---

## §6 — Citation index (read fresh this pass)

- `kernel/src/budget.rs:1-319` — `ComputeBudget`/`BudgetedJobPort`; monthly GPU spend accountant,
  degrade-closed; NaN/neg refusal (`aa70d7fa6`). `[VERIFIED-CODE]`
- `kernel/src/token_bucket.rs:1-152` — monotonic rate-limiter; over-grant invariant; no
  currency semantics. `[VERIFIED-CODE]`
- `kernel/src/ports/agent/admission.rs:50-59,254-295,300-321,611,891-927` — `FUEL_PER_UNIT` (CPU-
  instruction fuel meter, pending B4); `AdmissionLimiter` pre-crypto flood guard; per-agent
  `TokenBucket` envelope; `crit11` flood-throttle test. `[VERIFIED-CODE]`
- `bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs:124-209` — admission verify sequence; no cost on
  cert request. `[VERIFIED-CODE]`
- `bebop-repo/bebop2/proto-cap/src/node_id.rs:117-142,157-185,187-375` — fail-closed genesis;
  `RootDelegationPolicy`; `IssuanceBudget`/`sign_delegation_budgeted` counter cap. `[VERIFIED-CODE]`
- `bebop-repo/bebop2/proto-cap/src/roster.rs:13-34,252-316` — anchor-rooted `verify_chain`
  asymmetric Sybil gate. `[VERIFIED-CODE]`
- `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/16-BATCH7-sybil-proof-capability-mechanism-findings.md
  §0,§4` — four-candidate evaluation; **PoW REJECT (weak+regressive)**; stake viable-but-money-gated;
  anchor mechanism wins. `[VERIFIED-DOC]`
- `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-D-consensus-capability.md §5,§11` +
  `P-D-audit-root-delegation-policy.md §2,§4` — IssuanceBudget "not a currency" (R-2); no
  scarcity/PoW/stake primitive in `proto-cap`; Options A/B/C. `[VERIFIED-DOC]`
- `docs/design/mesh-real/MESH-12-RESOLVED-2026-07-14.md` — operator-signed-root genesis decision.
  `[VERIFIED-DOC]`
- External precedent (hashcash, Bitcoin PoW, Ethereum gas, Chia/Filecoin PoSpace/PoSt, Intel PoET;
  Douceur 2002, Cheng–Friedman 2005) — `[KNOWLEDGE]`, WebSearch budget exhausted this session; not
  load-bearing for the verdict.
