# BLUEPRINT P67 — Hub provisioning & claim (2026-07-18)

> **Wave W3 automation blueprint.** One coherent, independently-buildable unit against the 20-point
> contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Scope source: `SYNTHESIS-LAUNCH-BLOCKERS-
> 2026-07-18.md` §5 (W3 table, row **P67**), cross-cut reasoning in **X8** (identity chain upstream
> of claim) and **X9** (the golden snapshot is the integration point for provisioning + update +
> backup — **P67 owns the image spec, P68 co-signs the slot/backup layout**). Technical grounding:
> `OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` §1 (CF Tunnel automation), §4 (Hetzner warm
> pool), §5 (swappable-port prior art + Synapse lesson), §6.1 (claim-mechanic), §7 (risks). Cert
> input: `BLUEPRINT-P59-capability-cert-chain.md` (root/child mint mechanism, cited precisely below).
> Operator rulings applied: **§4-C CLOSED** (abandoned hubs are *suspended-but-preserved*, not
> recycled). Format precedent: `BLUEPRINT-P51-open-map-routing.md`, `BLUEPRINT-P59-*`.
>
> **One sentence:** provision a warm pool of fixture-populated Hetzner hubs from a Packer golden
> snapshot, each injected at first-boot with a P59 self-signed hybrid root and a per-hub remotely-
> managed Cloudflare tunnel, so that a *claim* is an ownership-assignment-only hot path (zero infra
> work, zero boot wait) — with every external dependency behind a `TunnelProvider` / `VpsProvider`
> trait from day one, an append-only mutation log + scoped tokens + account-pool config as the
> compensating controls for Cloudflare's absent control-plane tenancy, and a CI dependency-graph
> fence enforcing the open/closed repo boundary as a testable gate.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**. The
> single most important correction: **provisioning is a green-field surface — there is NO existing
> `TunnelProvider`, `VpsProvider`, warm-pool, `cloudflared`, Packer, or `hcloud` code anywhere in
> the tree** (`grep -riE 'TunnelProvider|VpsProvider|warm.?pool|cfd_tunnel|cfargotunnel|cloudflared|
> packer|hcloud'` over `**/*.rs,*.toml,*.sh` excluding `docs/` and `target/` returns **zero hits**).
> P67 therefore *builds* the provisioning plane, but it does so on top of real, verified primitives
> the kernel already ships. Naming those primitives precisely is a prerequisite for a correct spec.

### 0.1 The real primitives P67 reuses (verified this pass — not green-field where it need not be)

| Primitive P67 needs | Real code (verified this pass) | How P67 uses it |
|---|---|---|
| append-only, content-hash-chained log (for the **tunnel-config mutation log**, §5.2) | `kernel/src/event_log.rs` — append-only event log, the kernel's canonical tamper-evident record shape | The closed provisioning service's mutation log **mirrors this shape** (write-ahead, hash-chained, append-only). P67 does not invent a log format; it reuses the kernel's. |
| liveness probe (for the **heartbeat**, §8) | `tools/native-spa-server/src/api.rs:367,512` — `GET /healthz` is an OPEN, capability-free liveness probe ("bypass all cap checks") | The hub-side heartbeat emitter reuses this exact "liveness is cap-free, data is not" split. The heartbeat carries **liveness only** (§16.53). |
| telemetry seam (for **claim latency + tunnel-cap gauge**, §11) | `kernel/src/metrics.rs` — `HostId` (`:25`), `ClaimLatencyRecord` (`:79`), `AnomalyFlag` (`:87`), `MetricSample` (`:68`) | `ClaimLatencyRecord` **already exists** and is already wired into the CI `claim-latency ledger` job (`ci.yml:64`). P67 emits assignment latency and the tunnel-count gauge through it — zero new telemetry infra. |
| declarative must-never CI fence (for the **open/closed dependency fence**, §7) | `tools/ops-alert/fences.toml` + `tools/ops-alert/src/fence_check.rs` — fence kinds `grep-absent`, `cargo-feature-absent`, `workflow-present`; `fence_count` tamper-guard; CI job `security fences` (`ci.yml:309`), S0-on-trip → RED | P67 **adds one fence** (`no-closed-import`, kind `grep-absent`) — it does not build a new CI mechanism. The `fence_count` bump discipline already exists. |
| external dead-man's-switch (precedent for the **1,000-tunnel-cap alert**, §5.4) | `.github/workflows/heartbeat-monitor.yml` — decoupled GitHub-hosted watcher, `PROBE_TARGETS` env matrix, Telegram send path, `workflow-present` fence `pager-alive` guards it | The tunnel-cap alert reuses this alerting lane + the `workflow-present` fence discipline; the probe is a tunnel-count gauge instead of a URL. |

### 0.2 The P59 cert primitives P67 consumes verbatim (the claim hands out a P59-shaped cert)

P67 does **not** implement any cryptography. It calls P59's surface (`kernel/src/pq/cert_chain.rs`,
extends `kernel/src/ports/agent/cap.rs`). The exact minting mechanism P67 invokes, cited from
`BLUEPRINT-P59-capability-cert-chain.md`:

- **Root mint (bake/provision time):** `SelfSignedRoot::mint(seed)` (P59 §4.4 / M4) — "derives a
  hybrid keypair (Ed25519 via seam + ML-DSA-65 via `pq::dsa::keygen`), sets
  `node_id = NodeId::from_keys(...)`, self-signs." `NodeId::from_keys(pq_pub, classical_pub)` is real
  code at `cap.rs:58` — "changing EITHER public key changes the id — no CA, no assignable owner."
  P59's `red_root_without_dowiz_is_valid` proves `verify_self()` returns `Ok` with **no dowiz
  co-sign present** — that is precisely the property P67 needs: *the pooled hub is trust-self-sufficient
  before anyone claims it, before any network round-trip to dowiz.*
- **Optional dowiz co-sign (claim-time convenience only):** `DowizCoSign` (P59 §3, §4.4) — "a
  *detached* dowiz signature over the root's public keys… its ABSENCE never invalidates the root."
  P59's `red_dowiz_cosign_absence_never_blocks` / `red_bad_dowiz_cosign_ignored` guarantee it is
  additive-only. P67 attaches it during claim as a second voucher, never as a gate (§17.7).
- **Owner→hub delegation (claim-time):** the owner root appends a child `Delegation` block per hub
  (P59 §4.5 / M5), `may_delegate = false`, single hop under `MAX_DELEGATION_DEPTH = 1`. P59's
  `red_owner_mints_child_offline` proves a hub verifies an owner-minted child cert "knowing ONLY the
  owner root's *public* key, no network." The hub trusts the owner root by enrolling its public key
  into the hub's `AnchorRoster` (`cap.rs:373-408`, `&mut`-gated `enroll`, "out-of-band operator/
  genesis only") at claim time.
- **Suite tag on every block:** `AlgSuite::MlDsa65Ed25519 = 0x0001` → OID `1.3.6.1.5.5.7.6.48`
  (P59 §3, M1). P67 never chooses or forks a suite; it carries whatever P59 stamps.

**Consequence for P67:** the claim service is a thin *assignment + handoff* orchestrator over P59's
identity primitives + the two provisioning traits. It contains **no crypto and no card data**
(the payment-account connect step is P60/P72, §2.2). This keeps the closed claim service's blast
radius bounded to routing/ownership, never keys (§12 isolation).

### 0.3 Where the code lives — the open/closed split is a ground-truth fact, not a wish (§16.54)

`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.54 (verbatim): "the hub software
(kernel/protocol/UI-rendering — whatever a self-host vendor actually installs) is open source;
`dowiz.org`'s own infrastructure (the claim mechanic, CF-tenant-isolation, the directory-of-nothing
landing site itself) stays closed." P67's build items are therefore split across two repos by
construction (§7 makes the boundary a *testable* gate, not a policy):

| Concern | Repo | Rationale |
|---|---|---|
| hub binary, `cloudflared` bundling, **shadow local ingress config**, **tunnel-target-switch escape hatch** (§17.3), **heartbeat emitter** (§8), self-signed root (P59, in kernel) | **OPEN** (`dowiz`, AGPLv3+TM+DCO) | A self-host vendor runs exactly this; the escape hatch must survive dowiz (§17.3). |
| claim service, warm-pool manager, CF-tenant-isolation, `TunnelProvider`/`VpsProvider` **adapters** (Cloudflare/Hetzner), append-only tunnel-config mutation log, tunnel-cap alerting, golden-image bake pipeline, heartbeat **collector** | **CLOSED** (`dowiz-provision`, dowiz operating infra) | dowiz's own operating infrastructure, not the product a vendor installs (§16.54). |
| `TunnelProvider` / `VpsProvider` **trait definitions** (the port contracts) | **OPEN** (`dowiz`, a small `provision-ports` crate) | The *traits* are AGPL so a self-host vendor can write their own adapter (§17.8/17.9). Only the *dowiz adapters* + orchestration are closed. |

---

## 1. Standards & prior-art map — adoption, not invention (standard §2 item 19; SYNTHESIS X9)

P67 adopts documented external mechanisms; it invents only the orchestration that stitches them.
Each row is a real, R3-cited artifact and the exact way P67 uses it — and what it does NOT take.

| Prior art | What it really is (cited from R3) | How P67 uses it — and what it does NOT take |
|---|---|---|
| **Cloudflare remotely-managed tunnels** | A remotely-managed tunnel "requires only a tunnel token to run — an opaque `eyJ…` JWT"; all ingress/routing config lives in CF's control plane, editable via API. (R3 §1.1) | **Adopt as the Wave-0 `TunnelProvider` default.** The hub ships `cloudflared`, is handed a token at provision time, dowiz configures routing centrally. **NOT taken:** locally-managed tunnels (YAML on host) — except as the **shadow ingress config** for the §17.3 escape hatch (§9.4). |
| **The concrete CF API flow** | `POST /accounts/{acct}/cfd_tunnel {name, config_src:"cloudflare"}` → `GET .../{tid}/token` → `PUT .../{tid}/configurations {ingress}` → `POST /zones/{zone}/dns_records {type:CNAME, content:<tid>.cfargotunnel.com, proxied:true}` → host runs `cloudflared service install <token>`. Fully unattended, no dashboard. (R3 §1.2) | **This is the exact body of `CloudflareTunnel`'s trait impl** (§3, §4.2). Each of the four API calls is one trait method + one write-ahead mutation-log entry (§5.2). |
| **CF account hard limits** | **1,000 tunnels / account**, 1,000 routes / account (co-capped), 25 replicas / tunnel. "One tunnel per hub → ≤1,000 hubs before a second account is mandatory. This is *the* scaling wall." (R3 §1.4) | **Adopt the 1,000-tunnel cliff as a first-class numeric constant** (`CF_TUNNELS_PER_ACCOUNT_CAP = 1000`, §3) with warn/critical watermarks + a second-account provisioning path that is a **config change, not a rewrite** (§5.3/§5.4). |
| **No CF control-plane tenancy** | "Control-plane isolation is NOT real: the *dowiz operator API token* … can see and modify **every** tunnel on the account. There is no per-tenant CF sub-account boundary… 'tenant isolation' means **dowiz's own provisioning service is the trust boundary**." (R3 §1.5) | **Adopt the finding honestly (§5).** The compensating controls (append-only mutation log, scoped/short-lived tokens, account-pool sharding to shrink blast radius) are P67's engineering answer — the trust boundary is the *service*, not Cloudflare. |
| **Hetzner golden-snapshot + warm pool** | (A) Packer golden snapshot → `hcloud_server` boots from it (far faster than install-from-scratch); (B) pre-create N running servers ahead of demand, claim = ownership-flip only, no Hetzner API call on the hot path. "**Recommended: (B) as the claim hot-path, (A) as the background refill.**" (R3 §4.1) | **Adopt B (warm pool) as the claim hot path, A (snapshot-provision) as the background refill.** §16.57 no-reclaim → the pool is net-consumed → refill is a **background pipeline, not a recycle loop** (§6). |
| **Synapse — the cautionary port lesson** | Synapse offloads media to S3 via a pluggable module, "no vendor lock-in" — **but hard-requires PostgreSQL; the DB is *not* swappable.** "A port is only a port if the *default* was built behind the interface from day one. Synapse's storage was; its DB wasn't — and the DB is now un-swappable." (R3 §5) | **The load-bearing design lesson.** `TunnelProvider` + `VpsProvider` are **real traits from day one** (§3, §4.1), with Cloudflare/Hetzner as the *only* Wave-0 adapters — precisely so the §17.3/17.8/17.9 escape hatch is a real switch, never a retrofit. A retrofitted port is not a port. |
| **Mastodon / SPIFFE UpstreamAuthority** | Mastodon: local vs S3/GCS/Swift selected purely by env var — "one interface, config-selected adapter, one default." SPIFFE: signer is a plugin, self-signed default + external-CA adapter optional. (R3 §5) | **Adopt the "config-selected adapter, one Wave-0 default" shape** for both traits; the `CertRoot` port is P59's, not P67's (anti-scope §2.2). |

**The blueprint's job therefore shrinks (SYNTHESIS X9) to: the two trait contracts + the warm-pool
lifecycle + the compensating controls + the golden-image spec + the open/closed fence + the
heartbeat — the tunnel/host/cert mechanisms themselves are solved upstream.**

---

## 2. Scope — what P67 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P67 OWNS

1. **The two provisioning ports as real traits** — `TunnelProvider`, `VpsProvider` (§3, §4.1), with
   `CloudflareTunnel` + `HetznerVps` as the *only* Wave-0 adapters (§17.8/17.9; Synapse lesson §1).
2. **The warm-pool lifecycle** — assignment-only claim hot path + Packer-golden-snapshot background
   refill; pool depth/refill cadence as a named engineering decision under §4-C (§6).
3. **The claim service** (closed repo) — ownership assignment + P59 root/child handoff + optional
   dowiz co-sign attach; the `dowiz.org` interest-form non-pool path (§4.6).
4. **The single-CF-token trust boundary + compensating controls** — append-only tunnel-config
   mutation log, scoped/short-lived API tokens, account-pool-ready config, 1,000-tunnel-cliff
   alerting (§5).
5. **The golden-image spec** (SYNTHESIS X9 — **P67 owns it, P68 co-signs the slot/backup layout**) —
   what is baked (shared) vs injected (per-hub), incl. the §17.3 shadow local ingress config (§9).
6. **Pre-minted self-signed root injection** at first-boot via P59's `SelfSignedRoot::mint` (§9.3).
7. **The public/closed repo split as a testable CI dependency-graph fence** (§7).
8. **The hub heartbeat** — liveness-only emitter (open) + collector (closed) (§16.53, §8).
9. **The §17.3 tunnel-target-switch escape hatch** (hub-side, open) so hubs survive dowiz (§9.4).

### 2.2 P67 does NOT own (anti-scope — prevents collision & scope-creep)

- **The capability-cert chain, root/child mint, suite versioning, revocation** → **P59**. P67 *calls*
  `SelfSignedRoot::mint` / `Delegation` append / `DowizCoSign` / `AnchorRoster::enroll`; it never
  implements crypto (§0.2). Any crypto bug is a P59 defect, gated by P59's §8 adversarial review.
- **The A/B update slots, the update supervisor, the age backup envelope + rclone transport** →
  **P68**. P67 owns the *image spec*; **P68 co-owns the slot/backup layout inside that spec** (X9).
  The supervisor's promote step and the age-snapshot-before-promote are P68's (SYNTHESIS X9,
  R5 risk #1) — P67 only reserves the slot layout in the golden image.
- **The owner UI that fans out N hub connections under one root** → **P70**. P67 provides the
  claim-time handoff API; P70 renders multi-hub management.
- **The `dowiz.org` landing page + signup UX** → **P73**. §16.56 rules it full-wgpu. P73's claim
  entry UX **hands off to P67's claim-service API** (SYNTHESIS §5 W3, P73 depends on "P67 service
  API"). P67 owns the service, not the landing surface.
- **The per-vendor payment-account connect step** → **P60/P72**. §0.2-1's "food-court vendor is
  payable only after connecting their own provider account" is a *named step in the claim/vendor-setup
  flow*, but the payment mechanics are P60's. P67 exposes the hook; P60 fills it. **No card data ever
  touches the claim service** (R2 §5.2 type-level no-card-data; §12).
- **Terraform as a hard dependency.** R3 §1.3/§4.2 cite Terraform+Packer as *reference*
  implementations. The **hot-path pool manager + claim service + both trait adapters are Rust-native**
  (CORE-ROADMAP-STANDARD §1: "kernel/Rust/WASM only… Node/TS/JS/Python are adapters at most") calling
  the CF/Hetzner REST APIs directly behind the traits. **Packer** is retained only as the image-bake
  tool (build-time infra, no mature pure-Rust VM-image baker; acceptable like any build tool);
  **Terraform is optional** and, where used, confined to the closed background refill pipeline —
  never the runtime claim path (§6.4). This is a deliberate honest reconciliation, not a silent gap.

### 2.3 Dependencies (standard §2 item 7 — named by artifact)

**Existing input (hard dependencies, must land first):**
- **P59** `BLUEPRINT-P59-capability-cert-chain.md` — the root/child/co-sign/suite primitives P67
  hands out (§0.2). P59 is W1; P67 is W3; the ordering is explicit in SYNTHESIS §3.2.6
  ("`TunnelProvider`, `VpsProvider` … are named in W1 blueprints so nothing hardcodes against them").
- `kernel/src/event_log.rs` (mutation-log shape), `tools/native-spa-server` `/healthz` (heartbeat
  shape), `kernel/src/metrics.rs` `ClaimLatencyRecord`/`HostId` (telemetry seam),
  `tools/ops-alert/fences.toml`+`fence_check.rs` (CI-fence mechanism), `heartbeat-monitor.yml`
  (alerting lane) — all §0.1, reused not rebuilt.

**Co-owner (shared contract — single-owner rule per SYNTHESIS "swarm dispatch summary"):**
- **P68** `BLUEPRINT-P68-hub-supervisor-update-backup.md` — **golden-image co-owner**. P67 writes the
  image spec (§9); P68 co-signs the A/B slot layout + age backup scheduler slots *within* it. Neither
  redefines the other's half. The image spec has a single authored home (this document, §9); P68
  cites it.

**Consumers (P67 is upstream of):**
- **P73** dowiz.org landing — calls the claim-service API (§2.2).
- **P70** owner surface — consumes the claim-time root/child handoff.
- **M3 gate** (first claimed hub) = P67 + P68 live (SYNTHESIS §3.1 W3).

### 2.4 Honest reconciliation: the pre-minted hub root vs the owner root (standard §2 item 6)

A careless design would conflate two roots. §17.7 says *each hub is its own self-signed root*; §16.48
says *the owner holds a root that delegates to child hubs*. P67 must hand out both coherently — P59
§2.4 already resolved the crypto shape (two anchor kinds, both `depth<=1`); P67 resolves the
*handoff*:

1. At **provision** (background refill), the pooled hub mints its **own** self-signed root
   (`SelfSignedRoot::mint`) — this is the hub's §17.7 trust-domain root, making it addressable and
   survivable-without-dowiz *before* any claim.
2. At **claim**, the owner's **separate** multi-hub root is enrolled into the hub's `AnchorRoster`
   (`&mut`-gated `enroll`, out-of-band = the claim handoff itself), and the owner root appends a child
   `Delegation` block (`may_delegate=false`, single hop) scoping the owner's authority over *this*
   hub. The hub now (a) has its own self-root identity and (b) trusts owner commands via the enrolled
   owner-root anchor.
3. The optional `DowizCoSign` is attached as a detached voucher over the hub's root — convenience for
   relying parties that trust dowiz, never load-bearing (P59 §4.4).

No operator ruling is overturned; the hub-root and owner-root are distinct anchors, exactly as P59
§2.4 designed. P67's job is only to *sequence the enroll + append + co-sign* correctly, which §4.5
tests adversarially (`red_claim_without_owner_root_still_self_sufficient`).

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

Two crates. **`provision-ports`** (OPEN, `dowiz` repo) holds the trait contracts + wire types so a
self-host vendor can write an adapter. **`dowiz-provision`** (CLOSED) holds the Cloudflare/Hetzner
adapters + the pool manager + claim service + mutation log. Constants are named, never magic.

```rust
// crate provision-ports  (OPEN, AGPLv3 — the port contracts only, no dowiz adapter)

/// Stable hub identity used across pool, tunnel, DNS, cert, heartbeat. Distinct from the
/// P59 NodeId (which is the hash of the hub's keypair): HubId is the human/routing handle
/// (`hub-<HubId>.hubs.dowiz.org`); NodeId is the cryptographic identity. Bound 1:1 at provision.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HubId(pub [u8; 16]);              // 128-bit random, URL-safe base32 in hostnames

/// An opaque Cloudflare remotely-managed tunnel token (the `eyJ…` JWT, R3 §1.1). SECRET.
/// Never logged in full (only a prefix), never leaves the hub it is injected into.
#[derive(Clone)] pub struct TunnelToken(String);   // Debug is redacted (see impl)
#[derive(Debug, Clone, PartialEq, Eq)] pub struct TunnelId(pub String);   // CF tunnel_id
#[derive(Debug, Clone, PartialEq, Eq)] pub struct Hostname(pub String);   // hub-<id>.hubs.dowiz.org

/// One ingress rule (R3 §1.2 step 3): hostname → local service, terminal catch-all http_status:404.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressRule { pub hostname: Hostname, pub service: String }     // service = http://localhost:8080
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressConfig { pub rules: Vec<IngressRule> }                   // last rule MUST be the 404 catch-all

/// A booted, tunneled, cert-injected hub sitting in the warm pool, unclaimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolSlotState {
    Provisioning,                 // create_from_image issued, first-boot not confirmed
    Warm,                         // booted + tunnel up + self-root minted + heartbeat green; claimable
    Claimed { owner: OwnerId },   // assignment done; NEVER returns to Warm (§16.57 no-reclaim)
    Suspended { owner: OwnerId, state_snapshot: ImageRef },  // §4-C: compute released, state kept, re-wakeable
}

/// The two Wave-0-defaulted ports. Real traits from DAY ONE (Synapse lesson, R3 §5).
pub trait TunnelProvider {
    fn create_tunnel(&self, hub: &HubId) -> Result<TunnelId, ProvisionError>;   // POST /cfd_tunnel
    fn fetch_token(&self, t: &TunnelId) -> Result<TunnelToken, ProvisionError>; // GET  .../token
    fn configure_ingress(&self, t: &TunnelId, cfg: &IngressConfig) -> Result<(), ProvisionError>; // PUT .../configurations
    fn route_dns(&self, host: &Hostname, t: &TunnelId) -> Result<(), ProvisionError>;  // POST /zones/{z}/dns_records CNAME
    fn destroy_tunnel(&self, t: &TunnelId) -> Result<(), ProvisionError>;
    /// The 1,000-cap gauge (R3 §1.4). Cheap; polled by the cap-alert loop (§5.4).
    fn count_tunnels(&self) -> Result<u32, ProvisionError>;
}
pub trait VpsProvider {
    fn create_from_image(&self, img: &ImageRef, spec: &ServerSpec) -> Result<ServerId, ProvisionError>;
    fn assign_owner(&self, s: &ServerId, o: &OwnerId) -> Result<(), ProvisionError>;   // ownership flip only
    /// §4-C suspended-but-preserved: snapshot state THEN release the running server.
    fn suspend_preserving(&self, s: &ServerId) -> Result<ImageRef, ProvisionError>;
    fn resume_from(&self, img: &ImageRef, spec: &ServerSpec) -> Result<ServerId, ProvisionError>;
    fn destroy(&self, s: &ServerId) -> Result<(), ProvisionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)] pub struct ImageRef(pub String);   // hcloud snapshot id / equiv
#[derive(Debug, Clone, PartialEq, Eq)] pub struct ServerId(pub String);
#[derive(Debug, Clone, PartialEq, Eq)] pub struct OwnerId(pub [u8; 32]);  // == owner-root NodeId bytes
#[derive(Debug, Clone, PartialEq, Eq)] pub struct ServerSpec { pub server_type: String, pub location: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionError { RateLimited, Upstream(String), CapExceeded, NotFound, Unauthorized }
```

```rust
// crate dowiz-provision  (CLOSED — dowiz operating infra; the mutation log + pool + claim)

/// One entry in the append-only, hash-chained tunnel-config mutation log (§5.2). Mirrors the
/// kernel event_log.rs shape: write-ahead, prev_hash chain, hybrid-signed by the provisioning
/// service's OWN key. This is the tamper-evident record that a compromise is attributable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelMutation {
    pub seq: u64,
    pub prev_hash: [u8; 32],       // content-hash chain (event_log.rs pattern)
    pub op: TunnelOp,              // CreateTunnel | ConfigureIngress | RouteDns | DestroyTunnel
    pub hub: HubId,
    pub account: CfAccountId,      // which CF account (account-pool, §5.3)
    pub at_tick: u64,
    pub actor: [u8; 32],           // provisioning-service key id — WHO made the change
    pub sig: HybridSig,            // P59 HybridSig over canonical bytes (RequireBoth)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelOp { CreateTunnel, ConfigureIngress, RouteDns, DestroyTunnel }

/// The account-pool handle (§5.3): the CF account is CONFIG, not hardcoded, so crossing the
/// 1,000-tunnel cap is adding an entry here, never a code change (§17.8 port abstraction).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfAccountId(pub String);
pub struct CfAccount { pub id: CfAccountId, pub api_token: TunnelToken, pub zone_id: String }
pub struct AccountPool { pub accounts: Vec<CfAccount> }   // fill in order; new account = append
```

**Named constants (policy values — exact defaults are engineering-decision, §6.3 / §5.4):**

```rust
// ---- Cloudflare cap (R3 §1.4 — HARD external number, not tunable) ----
pub const CF_TUNNELS_PER_ACCOUNT_CAP: u32 = 1000;   // the cliff. Immutable external fact.
pub const CF_TUNNEL_WARN_WATERMARK:   u32 = 800;    // 80% — alert + begin second-account provision
pub const CF_TUNNEL_CRIT_WATERMARK:   u32 = 950;    // 95% — page; new hubs route to next account only

// ---- Warm-pool economics (§6.3 — tunable defaults; §4-C affects COST not supply) ----
pub const WARM_POOL_DEPTH_PER_REGION: u32 = 20;     // claimable slots kept hot per region
pub const POOL_REFILL_LOW_WATERMARK:  u32 = 8;      // refill trigger (40% of depth)
pub const POOL_REFILL_BATCH:          u32 = 12;     // servers built per refill run (restores to depth)
pub const CLAIM_ASSIGN_BUDGET_MS:     u64 = 500;    // assignment-only hot-path SLO (no boot, no CF call)

// ---- Heartbeat (§8) ----
pub const HEARTBEAT_EMIT_TICKS:       u64 = /* ~30s */; // hub → collector cadence
pub const HEARTBEAT_SILENCE_ALERT_TICKS: u64 = /* ~3× emit */; // collector alerts after N missed

// ---- CF API token custody (§5.1) ----
pub const CF_TOKEN_MAX_TTL_TICKS:     u64 = /* short — rotate on cadence, R3 §1.5 */;
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first (types/invariant), a test that goes RED before the change, code, then GREEN.**
Lifecycle transitions are modeled as `PoolSlotState` events; tests assert on the *sequence*, not just
end-state (item 3). Adapters are tested against a **mock CF/Hetzner** (no live account in CI; the live
smoke test is a separate manual gate, §10 D-note).

### 4.1 M1 — the two ports as real traits + mock adapters (Synapse lesson, R3 §5)

- **Spec:** `TunnelProvider` + `VpsProvider` compile as traits in the OPEN `provision-ports` crate;
  the pool manager + claim service are generic over them (`<T: TunnelProvider, V: VpsProvider>`), so
  no call site names Cloudflare or Hetzner directly. A `MockTunnel`/`MockVps` (in-memory) is the test
  adapter; `CloudflareTunnel`/`HetznerVps` (closed) are the only Wave-0 real adapters.
- **RED test `red_pool_manager_is_provider_agnostic`:** a grep/compile assertion that
  `dowiz-provision`'s pool-manager module contains **zero** literal `"cloudflare"`/`"hetzner"`/
  `cfd_tunnel`/`hcloud` tokens outside the two adapter modules. RED if any leaks into orchestration;
  GREEN proves the port is real (the Synapse anti-pattern — a hardcoded default — is absent).
- **Adversarial `red_second_tunnel_adapter_drops_in`:** a `WireguardTunnel` stub implementing
  `TunnelProvider` is swapped into the pool manager with **no orchestration edit** → compiles + the
  full claim flow runs against it. Proves the §17.8 escape hatch is a real switch, not a retrofit.

### 4.2 M2 — `CloudflareTunnel` adapter = the exact R3 §1.2 API flow, each call write-ahead-logged

- **Spec:** `create_tunnel` → `POST /accounts/{acct}/cfd_tunnel {name:"hub-<id>", config_src:"cloudflare"}`;
  `fetch_token` → `GET .../{tid}/token`; `configure_ingress` → `PUT .../{tid}/configurations` with the
  `ingress` array (last rule = `http_status:404` catch-all); `route_dns` →
  `POST /zones/{zone}/dns_records {type:"CNAME", content:"<tid>.cfargotunnel.com", proxied:true}`.
  **Every mutating call writes its `TunnelMutation` to the append-only log BEFORE issuing the CF
  request** (write-ahead, §5.2).
- **RED test `red_ingress_missing_catchall_rejected`:** an `IngressConfig` whose last rule is not the
  `http_status:404` catch-all → `configure_ingress` returns `Err` before any network call. RED today
  (no validation), GREEN after — a malformed ingress can silently 5xx every hub route.
- **RED test `red_mutation_logged_before_api_call`:** the mock CF adapter records call order; assert
  the log append is observed **strictly before** the CF mutation. If the CF call is issued first and
  the log write fails, the change is unattributable — this test forbids that ordering.
- **Adversarial `red_dns_content_must_be_cfargotunnel`:** a `route_dns` with `content` not matching
  `<tid>.cfargotunnel.com` → rejected. Prevents a typo/injection re-pointing a hostname off-tunnel.

### 4.3 M3 — `HetznerVps` adapter + the warm-pool lifecycle (R3 §4.1-B hot path, §4.1-A refill)

- **Spec:** `create_from_image(golden_snapshot, spec)` boots a pooled server → first-boot injection
  (§9.3) → `PoolSlotState::Warm`. `assign_owner` is the **claim hot path**: an ownership-record flip +
  a `PoolSlotState::Warm → Claimed` transition, **no Hetzner API call, no boot** (R3 §4.1-B). The
  background refill (§6) calls `create_from_image` to top the pool up. `server_type`+`location` MUST
  match the snapshot's build params (R3 §4.2).
- **RED test `red_claim_is_assignment_only`:** the mock Hetzner adapter asserts `create_from_image` is
  **NOT called** during `assign_owner`; the claim latency is measured `< CLAIM_ASSIGN_BUDGET_MS`
  against a mock clock. RED if claim triggers a boot; GREEN proves the hot path is assignment-only —
  the entire §16.32 "assignment, not provisioning" property as an executable assertion.
- **RED test `red_claimed_never_returns_to_pool`:** attempt to transition `Claimed → Warm` →
  rejected at the type/state-machine boundary (§16.57 no-reclaim). Event sequence asserted:
  `Provisioning → Warm → Claimed` is legal; `Claimed → Warm` is unrepresentable.
- **Adversarial `red_spec_mismatch_rejected`:** `create_from_image` with a `server_type` differing
  from the snapshot's build param → `Err(Upstream)` (R3 §4.2 — boot-from-snapshot requires matching
  params). Prevents a silently-wrong pool refill.

### 4.4 M4 — the claim service: assignment + P59 root/child handoff (closed repo)

- **Spec:** `claim(hub: HubId, owner: OwnerRoot) -> Result<ClaimReceipt>` (a) `VpsProvider::assign_owner`
  (ownership flip), (b) enroll the owner root's public key into the hub's `AnchorRoster` (P59
  `&mut`-gated `enroll`), (c) the owner appends a child `Delegation` block (P59 M5, `may_delegate=false`,
  `MAX_DELEGATION_DEPTH=1`), (d) optionally attach a `DowizCoSign` over the hub root (P59 M4). The hub
  is already online (warm), so the vendor sees a fixture-populated hub instantly (§16.54).
- **RED test `red_claim_without_owner_root_still_self_sufficient`:** claim a hub, then *remove* the
  owner-root enrollment and the co-sign → the hub's `SelfSignedRoot::verify_self()` still returns
  `Ok` (P59 `red_root_without_dowiz_is_valid`). Proves the hub is trust-self-sufficient — the §17.7
  property survives the claim handoff. RED if the claim made the hub *depend* on dowiz/owner for its
  own identity.
- **RED test `red_child_block_cannot_redelegate`:** the claim-issued child `Delegation` tries to
  append a grandchild → rejected (`MaxDepthExceeded`, P59 §2.4). Owner→hub is a single hop.
- **Adversarial `red_cross_owner_claim_forgery`:** owner B presents owner A's root at claim for a hub
  → the enrolled anchor mismatch → the hub rejects A's chain under B's anchor (`UnknownIssuer`, P59
  M5). No cross-tenant claim.

### 4.5 M5 — the append-only tunnel-config mutation log (§5.2) as a hash-chained, signed ledger

- **Spec:** `MutationLog::append(op) -> Result<()>` computes `prev_hash` chain (event_log.rs shape),
  hybrid-signs (`HybridSig`, RequireBoth), appends; **never rewrites or deletes** an entry.
  `verify_chain()` walks the chain: any `prev_hash` break or bad sig → `Err`.
- **RED test `red_mutation_log_tamper_detected`:** flip one byte of entry N's `op` → `verify_chain`
  fails at N (hash-chain break). RED today (no log), GREEN after — the log is tamper-evident, so a
  provisioning-service compromise is *attributable* even though it is not *prevented* (R3 §1.5 — the
  service is the trust boundary; the log is the compensating control).
- **RED test `red_unsigned_mutation_rejected`:** an entry with an absent/bad `HybridSig` → `append`
  refuses (mirrors P59 `red_unsigned_revocation_blob_ignored` / `codesign::apply`). A rogue actor
  cannot inject an unsigned routing change.
- **Adversarial `red_log_replay_cannot_reorder`:** replay entry seq=3 after seq=5 → rejected (seq is
  monotonic, `prev_hash` binds order). Prevents a reorder attack un-doing a later config.

### 4.6 M6 — the non-pool path + suspend-preserving (§4-C) + second-account rollover (§5.3)

- **Spec:** (a) `dowiz.org` interest form (P73) → notifies the operator for manual follow-up, **no
  automation on the hot path** (R3 §6.1-5). (b) `suspend_preserving(server)` — §4-C: snapshot state
  THEN release the running server; `PoolSlotState::Claimed → Suspended{state_snapshot}`; `resume_from`
  re-wakes it. **Still the vendor's forever** (§16.57), just not burning a hot slot. (c) when
  `count_tunnels() >= CF_TUNNEL_CRIT_WATERMARK`, new tunnels route to `AccountPool.accounts[next]` —
  a config append, no code change (§5.3).
- **RED test `red_suspend_preserves_state_then_resume`:** suspend a claimed hub → assert compute
  released (mock: no running server) AND state snapshot exists → `resume_from` → the hub returns with
  its prior state (fixture/config intact). Event sequence: `Claimed → Suspended → Claimed`. Proves
  §4-C "state retained, compute released, re-wakeable."
- **RED test `red_suspended_hub_still_owned`:** a suspended hub's `OwnerId` is unchanged and it is
  **NOT** returned to the claimable pool. Proves §4-C qualifies but does not overturn §16.57.
- **Adversarial `red_second_account_rollover_no_code_change`:** drive `count_tunnels` past
  `CF_TUNNEL_CRIT_WATERMARK` with a two-entry `AccountPool` → new hubs land on account #2 with **zero**
  orchestration edit (only a config entry). Proves the §17.8 account-pool abstraction (§5.3).

### 4.7 M7 — the CI dependency-graph fence (open must never import closed) — §7

- **Spec:** add a `no-closed-import` fence to `tools/ops-alert/fences.toml` (kind `grep-absent`),
  pattern = the closed crate name(s) `dowiz-provision`/`dowiz_provision`, glob = the OPEN repo's
  `**/Cargo.toml` + `**/*.rs` (excluding the closed crate's own tree if vendored). Bump `fence_count`.
  The existing `security fences` CI job (`ci.yml:309`) runs it, S0-on-trip → RED.
- **RED test `red_open_importing_closed_trips_fence`:** a fixture OPEN-repo file containing
  `use dowiz_provision::claim::…` (or a `dowiz-provision` Cargo dependency) → `fence-check` exits
  non-zero. Remove it → exit 0. This is the **falsifiable test the task mandates** for the fence.
- **Adversarial `red_fence_count_tamper_detected`:** deleting the `no-closed-import` `[[fence]]` block
  without decrementing `fence_count` → the existing `fence_count` mismatch guard trips (fences.toml
  discipline, already enforced). Proves the fence itself cannot be silently removed.

### 4.8 M8 — the heartbeat: liveness-only emitter (open) + collector (closed) — §8, §16.53

- **Spec:** the hub emits `Heartbeat { hub_id, tick, sig: HybridSig }` over its tunnel every
  `HEARTBEAT_EMIT_TICKS`; **payload carries NO order/menu/PII data** (§16.14). The collector records
  last-seen per `hub_id` and raises an `AnomalyFlag` (metrics.rs) after `HEARTBEAT_SILENCE_ALERT_TICKS`
  of silence. The heartbeat is **signed by the hub's self-root** so a spoof cannot mask a dead hub.
- **RED test `red_heartbeat_carries_no_data`:** a `Heartbeat` struct with any field beyond
  `{hub_id, tick, sig}` fails to compile / the serializer rejects extra bytes. The "liveness only,
  never hub data" exception (§16.53) is type-enforced, not prose.
- **RED test `red_forged_heartbeat_rejected`:** a heartbeat signed by a non-hub key → collector
  rejects (verify against the hub's known self-root). A spoofed heartbeat cannot keep a dead hub
  "green" nor forge liveness for a hub the attacker doesn't own.
- **Adversarial `red_silent_hub_alerts`:** stop emitting for `> HEARTBEAT_SILENCE_ALERT_TICKS` (mock
  clock) → `AnomalyFlag` raised. Proves the §16.53 "alert if one silently drops" property.

---

## 5. The single-CF-token trust boundary + compensating controls (standard §2 items 6, 11, 14; R3 §1.5, §7 risk #1)

**The finding, stated without softening (R3 §7 risk #1 — the highest-severity open item):** there
is **no control-plane tenant boundary in Cloudflare's own model.** dowiz's one operator API token can
create, read, and rewrite the routing of **every** hub's tunnel on the account. A compromise of that
token or the service holding it lets an attacker re-point every hub's public hostname. Cloudflare
provides **no per-tenant sub-account boundary** in the §16.45 one-account model. Therefore: **the
provisioning service itself is the trust boundary, not Cloudflare's tenancy.** P67 cannot delegate
this to the platform; it must build compensating controls. (What is *never* at risk via CF: the hubs'
application data and their P59 capability-cert keys — those never touch Cloudflare, §12.)

### 5.1 CF API-token custody (engineering-decision, named default)

- The API token is a **top-tier secret** held only by the closed `dowiz-provision` service — never on
  a hub, never in the OPEN repo (the §7 fence enforces the code boundary; the secret boundary is
  operational: the token lives in the closed service's secret store, gitleaks-scanned CI already
  guards against accidental commit, `ci.yml:175`).
- **Scoped as narrowly as Cloudflare permits:** a scoped API token (not a global key) with only
  `Zero Trust: Edit` (tunnel CRUD) + `DNS: Edit` **on the `hubs.dowiz.org` zone only** — not
  account-wide DNS, not billing, not Workers. This shrinks a leaked token's reach to tunnel+hub-zone
  routing, nothing else.
- **Short-lived + rotated:** `CF_TOKEN_MAX_TTL_TICKS` bounds validity; rotation is on a cadence
  (R3 §1.5 recommends tunnel-token rotation; the same discipline applies to the API token). Rotation
  is graceful — the pool manager reloads the token from the secret store without a restart.

### 5.2 Append-only tunnel-config mutation log (the primary compensating control — §4.5)

Every tunnel-config mutation (`create_tunnel`/`configure_ingress`/`route_dns`/`destroy_tunnel`) is
**write-ahead-logged** to an append-only, hash-chained, hybrid-signed ledger (`TunnelMutation`, §3)
**before** the CF API call. Modeled on `kernel/src/event_log.rs` (§0.1) — dowiz's canonical
tamper-evident record. This does not *prevent* a compromise (nothing on one account can), but makes
every routing change **attributable and tamper-evident**: `who` (`actor` key id), `what` (`op`),
`when` (`at_tick`), `which account` (`account`), in an order no replay can forge (M5). A silent
malicious re-point is impossible to hide — the log is the forensic and the alert substrate.

### 5.3 Account-pool-ready config — the 1,000-cap is a config change, not a rewrite (§17.8)

The CF account is a **runtime-config list** (`AccountPool`, §3), never a hardcoded handle. Wave-0
runs one account (§16.45); crossing the cap (§5.4) means **appending a second `CfAccount`** and
routing new hubs to it — the pool manager fills accounts in order. Because `TunnelProvider` is a real
trait (M1) and the account is config, this is a config append, **not** a re-architecture (R3 §1.4
planning takeaway: "the CF-account handle [is] itself a swappable/shardable parameter … so hitting the
cliff is a config change"). It also **shrinks blast radius**: sharding hubs across N accounts caps a
single-token compromise at that account's slice, not the whole fleet (R3 §7 risk #1's own suggested
mitigation).

### 5.4 The 1,000-tunnel-cliff alerting (falsifiable — the task's second mandated test)

A background loop polls `TunnelProvider::count_tunnels()` and emits the count as a gauge through the
metrics seam (`metrics.rs`, §11). Thresholds (§3):
- `>= CF_TUNNEL_WARN_WATERMARK` (800) → alert + **begin provisioning a second CF account** (§5.3), so
  headroom exists *before* the cliff, not at it.
- `>= CF_TUNNEL_CRIT_WATERMARK` (950) → page (via the `heartbeat-monitor.yml` alerting lane, §0.1) +
  new hubs route to the next account only.

**Falsifiable DoD test `red_tunnel_count_over_warn_alerts`:** a mock `TunnelProvider::count_tunnels()`
returning 801 → the cap-check loop raises an `AnomalyFlag` and begins second-account provisioning
(mock). RED before the alerting code exists, GREEN after. Plus a `workflow-present` fence
(`tunnel-cap-alive`, fences.toml) asserting the cap-alert workflow exists — mirroring the `pager-alive`
fence that guards the heartbeat monitor. Both are machine-checked in CI.

---

## 6. Warm-pool economics under §4-C (standard §2 items 8, 13; §4-C CLOSED)

**Binding ruling applied (do not re-ask):** operator has ruled §4-C **closed** — a long-inactive
claimed hub may be **suspended-but-preserved** (state retained, compute released, still theirs,
re-wakeable), *not* recycled. Critically (SYNTHESIS §4-C verbatim): "the pool is net-consumed with no
recycling either way; this only affects the *cost* of consumed slots." **The pool-sizing math must
NOT assume recycling** (§16.57 + §4-C both forbid returning a claimed hub to the claimable pool).

### 6.1 The consumption model (stated as math, not metaphor — item 13)

Let `C(t)` = cumulative claims, `P` = warm-pool depth, `R(t)` = cumulative refills. The claimable
pool at time `t` is `P + R(t) − C(t)`, and it stays `≥ 0` only if **refill throughput ≥ gross claim
rate** — because every claim permanently consumes one slot (no `Claimed → Warm`, M3). §4-C changes
**nothing** about supply: a suspended hub is still `Claimed`/`Suspended`, never back in the pool. §4-C
changes only the **ongoing cost** of a consumed slot: a suspended hub costs one cheap Hetzner snapshot
(the `state_snapshot: ImageRef`) instead of a running server. So:

- **Supply invariant (unchanged by §4-C):** refill cadence must keep `P + R(t) − C(t) ≥
  POOL_REFILL_LOW_WATERMARK`. This is a background pipeline, never a recycle loop (R3 §6.1-4).
- **Cost function (improved by §4-C):** `cost ≈ (warm_slots × hot_server_price) +
  (active_claimed × hot_server_price) + (suspended × snapshot_price)`, with `snapshot_price ≪
  hot_server_price`. §4-C bounds the tail cost of abandoned hubs to snapshot storage, not idle
  compute — the reason the ruling matters economically.

### 6.2 The hard ceiling the pool math must respect (§5 ↔ §6)

Every warm, claimed, AND suspended hub holds one CF tunnel (a suspended hub keeps its hostname so it
is re-wakeable at the same URL). Therefore per CF account:

```
warm_pool_depth + active_claimed + suspended_hubs  ≤  CF_TUNNELS_PER_ACCOUNT_CAP (1000)
```

This couples §6 to §5.4: the pool cannot be sized so large that pool+claimed+suspended approaches
1,000 before the second-account rollover (§5.3) has headroom. The `CF_TUNNEL_WARN_WATERMARK = 800`
exists precisely so the second account is provisioning while ~200 tunnels of headroom remain.

### 6.3 Named engineering decision — pool depth & refill cadence (proposed defaults, §3 constants)

| Knob | Proposed default | Rationale (tunable, one-line const change) |
|---|---|---|
| `WARM_POOL_DEPTH_PER_REGION` | **20** | Absorbs a claim burst without any hub hitting cold-boot; small enough that pool+claimed+suspended stays far under 800 for a long ramp. |
| `POOL_REFILL_LOW_WATERMARK` | **8** (40%) | Refill triggers with comfortable margin; background Packer+provision has time to restore depth before the pool drains. |
| `POOL_REFILL_BATCH` | **12** | Restores 8 → 20 in one background run; batched to amortize the Packer bake + provision cost. |
| `CLAIM_ASSIGN_BUDGET_MS` | **500** | Assignment-only hot-path SLO (no boot, no CF call) — measured against `ClaimLatencyRecord` (§11), M3's `red_claim_is_assignment_only`. |

These are `dowiz-provision` constants with a one-line change surface; the blueprint sets them, the
operator need not (they are engineering-decision E per SYNTHESIS §4-E: "warm-pool depth/refill cadence
(P67, downstream of ruling C)"). Multi-region = multiple pools, each with its own depth (R3 §4.2:
"one server_type + one primary location per pool; multi-region = multiple pools").

### 6.4 The refill pipeline (background, closed repo, Rust-native hot path)

The refill loop: when claimable depth `< POOL_REFILL_LOW_WATERMARK`, build/refresh the golden snapshot
(Packer, §9) if stale, then `VpsProvider::create_from_image` × `POOL_REFILL_BATCH`, first-boot-inject
each (§9.3), await `Warm`. **The hot-path claim (§4.4) never touches this** — it is assignment-only.
Terraform (R3 §1.3) is an optional declarative alternative for the refill *only*; the Rust-native
default calls `hcloud`/CF REST directly behind the traits (§2.2 reconciliation).

---

## 7. Public/closed repo split as a testable CI dependency-graph fence (standard §2 items 9, 14; §16.54; R5 §6.4 Option A)

§16.54's open/closed boundary is a **policy** until it is a **gate**. P67 makes it a gate — a
build-time check that the open hub-software repo never imports from the closed claim-service repo,
modeled on the existing kernel-fence guards (§0.1).

### 7.1 The fence (concrete, not prose)

- **Mechanism:** one new `[[fence]]` in `tools/ops-alert/fences.toml`, kind `grep-absent`:
  ```toml
  [[fence]]
  id = "no-closed-import"
  kind = "grep-absent"
  pattern = "dowiz.provision"        # matches `dowiz-provision` / `dowiz_provision`
  glob = "**/Cargo.toml,**/*.rs"     # the OPEN repo tree (closed crate is a separate repo, not vendored here)
  severity = "S0"
  note = "§16.54 open/closed boundary: the open hub-software repo must NEVER import the closed claim service"
  ```
  `fence_count` bumped 3 → 4 (the existing tamper-guard forces this, fences.toml discipline).
- **Where it runs:** the existing `security fences` CI job (`ci.yml:309`,
  `cargo run … --bin fence-check`), S0-on-trip → RED CI. **No new CI mechanism** — P67 adds a
  declarative row, not a workflow.
- **Direction:** the load-bearing direction is *open must not depend on closed* — a leak would both
  break the AGPL boundary (§16.57) and couple the survivable hub software to dowiz's private infra
  (undermining §17.3). The closed repo *may* depend on the open `provision-ports` traits (that is the
  intended direction, §3).

### 7.2 Falsifiable test (the task mandate)

`red_open_importing_closed_trips_fence` (M7): a fixture OPEN-repo file with `use dowiz_provision::…`
(or a `dowiz-provision` Cargo dependency) → `fence-check` exits non-zero; removing it → exit 0.
Plus `red_fence_count_tamper_detected` (M7): deleting the fence without decrementing `fence_count`
trips the existing count-mismatch guard. The fence cannot be bypassed by a leaked import *or* by
silently deleting the fence.

---

## 8. The heartbeat — liveness-only, signed, no data (standard §2 item 12; §16.53)

§16.53 (verbatim): "dowiz receives a heartbeat/liveness signal from every hub via the CF Tunnel layer
and can alert if one silently drops — a deliberate, narrow exception to §16.14's data isolation
(liveness only, never hub data)." P67's mechanism:

- **Emitter (OPEN, hub-side):** every `HEARTBEAT_EMIT_TICKS`, the hub sends
  `Heartbeat { hub_id, tick, sig }` over its own tunnel to the dowiz collector. The payload is
  **only** those three fields — no order counts, no menu, no PII (`red_heartbeat_carries_no_data`
  type-enforces this, M8). It reuses the `/healthz`-style "liveness is cap-free, data is not" split
  already in `native-spa-server` (§0.1).
- **Signature:** signed by the hub's P59 `SelfSignedRoot` (`HybridSig`, RequireBoth) so a spoofed
  heartbeat cannot mask a dead hub or forge liveness for a hub the attacker doesn't control
  (`red_forged_heartbeat_rejected`, M8).
- **Collector (CLOSED, dowiz-side):** records last-seen per `hub_id`; after
  `HEARTBEAT_SILENCE_ALERT_TICKS` of silence raises an `AnomalyFlag` (metrics.rs) → the
  `heartbeat-monitor.yml` alerting lane (§0.1). This is the operator alert for §16.14's *silent,
  unexplained* hub-down variant, distinct from the client-visible honest-offline UX (§16.14).
- **Payload budget (item 12):** `hub_id` (16B) + `tick` (8B) + `HybridSig` (~3.4KB, ML-DSA-65
  dominates) per emit — trivially within one frame; cadence bounded by `HEARTBEAT_EMIT_TICKS`. The
  heartbeat is **not** gossip — it is a direct hub→collector liveness ping (the single deliberate
  narrow exception to node-locality, §16.53).

---

## 9. The golden-image spec (standard §2 items 8, 13; SYNTHESIS X9 — P67 OWNS, P68 CO-SIGNS)

One Packer-built image is the shared contract for provisioning (P67), update (P68), and backup (P68).
**P67 authors this spec; P68 co-owns the A/B slot + backup-scheduler layout inside it** (X9). The
image spec has a single home — this section — which P68 cites, never redefines.

### 9.1 Two layers — baked (shared) vs injected (per-hub). This split is a CORRECTNESS invariant.

A subtle but load-bearing point an adversarial reviewer must catch: **the per-hub tunnel token and the
per-hub root keypair MUST NOT be baked into the shared snapshot** — if they were, every hub cloned from
that snapshot would share one tunnel credential and one identity, destroying the per-tunnel data-plane
isolation that is the *only* real isolation CF provides (R3 §1.5). Therefore:

| Layer | Contents | Why here |
|---|---|---|
| **Baked (shared, in the Packer snapshot)** | hub binary in the **A/B slot layout** (P68 co-owns); `cloudflared` binary; the `dowiz-hub-supervisor` (P68); the age backup scheduler (P68); demo fixtures — test menu + test couriers (§16.54); the **shadow local ingress config *template*** (§9.4); the AGPL open hub-software | Identical for every hub; cloneable; refreshed only when the image version bumps (R3 §4.1-A). |
| **Injected at pool-provision (per-hub, first-boot)** | `SelfSignedRoot::mint(seed)` → **unique** hybrid root keypair (§9.3); the **per-hub** tunnel token (from `TunnelProvider::create_tunnel` + `fetch_token`); the per-hub hostname `hub-<HubId>.hubs.dowiz.org`; the shadow-ingress config *materialized* with this hub's values | Must be unique per hub or isolation breaks. Injected via first-boot cloud-init when `create_from_image` runs. |

R3 §6.1 ("pre-minted its own self-signed root … at **snapshot/provision** time") is read precisely as
*provision* time (per-server first-boot), not shared-snapshot time — this section states that
explicitly so no implementer bakes a shared secret.

### 9.2 Rollback story for state (item 13 — P68 co-owns, P67 reserves the slot)

Per SYNTHESIS X9 + R5 risk #1: "The supervisor's promote step **must** take an age state snapshot
first … restoring the pre-promote snapshot is the rollback story for state" (code rollback must never
outrun forward-only schema migrations). P67's image spec **reserves** the age-snapshot slot in the
baked layout; **P68 owns the promote/snapshot logic**. P67 does not implement it — it guarantees the
image has room for it (anti-scope §2.2). This is snapshot-re-entry as math (item 13): recovery is
regeneration from the last valid epoch (the pre-promote age snapshot), not an error-correcting heal.

### 9.3 Pre-minted root injection (P59, cited precisely — §0.2)

At first-boot, the injection step calls P59 `SelfSignedRoot::mint(seed)` (P59 §4.4): derive the
hybrid keypair (Ed25519 via the seam + ML-DSA-65 via `pq::dsa::keygen`), set
`node_id = NodeId::from_keys(pq_pub, classical_pub)` (`cap.rs:58`), self-sign under
`HybridPolicy::RequireBoth`, stamp `AlgSuite::MlDsa65Ed25519`. The hub now has a verifiable identity
**before any network round-trip to dowiz** — P59's `red_root_without_dowiz_is_valid` is the executable
proof. The seed is per-hub (from the host CSPRNG at first-boot), never shared, never baked.

### 9.4 The §17.3 escape hatch — shadow local ingress config (R3 §7 risk #5)

Remotely-managed tunnels keep ingress config in Cloudflare's control plane; switching a hub to the
vendor's own CF account or a WireGuard relay (§17.3/§17.8) would otherwise require re-materializing the
routing config the hub never held locally. R3 risk #5 asks whether Wave-0 hubs should store a **shadow
local ingress config from day one** so the escape hatch is a real switch, not a rebuild. **P67's
answer: yes.** The baked image carries a shadow-ingress *template*; first-boot materializes it with the
hub's values. The hub-side (OPEN) `switch_tunnel_target(new_provider)` reads the shadow config and
re-points to a `config_src: local` cloudflared (or a `WireguardTunnel` adapter) with **no dowiz round
trip** — making §17.3 "hubs survive dowiz" a real mechanism, not an aspiration. This lives in the OPEN
repo precisely because it must work when dowiz is gone.

---

## 10. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

Each is a test that is **RED before the change, GREEN after**, or an artifact that exists/doesn't.

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | both ports are real traits; pool manager is provider-agnostic; a 2nd adapter drops in with no orchestration edit | `red_pool_manager_is_provider_agnostic`, `red_second_tunnel_adapter_drops_in` (M1) |
| D2 | the CF adapter is the exact R3 §1.2 flow; ingress catch-all enforced; every mutation write-ahead-logged | `red_ingress_missing_catchall_rejected`, `red_mutation_logged_before_api_call`, `red_dns_content_must_be_cfargotunnel` (M2) |
| D3 | claim is assignment-only (no boot, `< CLAIM_ASSIGN_BUDGET_MS`); claimed never returns to pool | `red_claim_is_assignment_only`, `red_claimed_never_returns_to_pool`, `red_spec_mismatch_rejected` (M3) |
| D4 | claim hands out a P59 root/child; hub stays self-sufficient without dowiz/owner; no re-delegation; no cross-owner claim | `red_claim_without_owner_root_still_self_sufficient`, `red_child_block_cannot_redelegate`, `red_cross_owner_claim_forgery` (M4) |
| D5 | the mutation log is append-only, hash-chained, hybrid-signed, tamper-evident, replay-resistant | `red_mutation_log_tamper_detected`, `red_unsigned_mutation_rejected`, `red_log_replay_cannot_reorder` (M5) |
| D6 | §4-C: suspend preserves state + releases compute + re-wakeable + still owned; 2nd-account rollover needs no code change | `red_suspend_preserves_state_then_resume`, `red_suspended_hub_still_owned`, `red_second_account_rollover_no_code_change` (M6) |
| D7 | **the open/closed CI dependency-graph fence trips on a closed import and cannot be silently removed** | `red_open_importing_closed_trips_fence`, `red_fence_count_tamper_detected` (M7) — **task-mandated** |
| D8 | **the 1,000-tunnel-cap alert fires at the warn watermark and the cap-alert workflow is fence-guarded** | `red_tunnel_count_over_warn_alerts` + `tunnel-cap-alive` `workflow-present` fence (§5.4) — **task-mandated** |
| D9 | the heartbeat carries liveness only (type-enforced), rejects forgeries, and alerts on silence | `red_heartbeat_carries_no_data`, `red_forged_heartbeat_rejected`, `red_silent_hub_alerts` (M8) |
| D10 | the golden image bakes shared / injects per-hub; no shared tunnel token or shared root keypair in the snapshot | `red_no_shared_secret_in_snapshot` (asserts the injection step, not the bake, produces the token+root); §9.1 |
| D11 | the §17.3 shadow-ingress escape hatch switches tunnel target with no dowiz round-trip | `red_switch_tunnel_target_offline` (hub reads shadow config, re-points, no network to dowiz) (§9.4) |
| D12 | no card data / no crypto in the claim service | grep: zero card-data type + zero direct crypto impl in `dowiz-provision` (P59 is the only crypto); §12 |
| D13 | both crates build; full test suite green incl. all new REDs now GREEN | `cargo test` (provision-ports + dowiz-provision, offline, mock adapters) |
| D14 | **live smoke (manual gate, not CI):** one real hub provisioned end-to-end on the real dowiz CF account + Hetzner project, claimed, heartbeat green | operator-run smoke against the live target (no live creds in CI); logged, not automated |

---

## 11. Benchmark plan + telemetry (standard §2 item 10) — existing harness, zero new infra

| Bench / gauge | Measures | Harness |
|---|---|---|
| `bench_claim_assign_latency` | assignment-only hot path vs `CLAIM_ASSIGN_BUDGET_MS` (500ms) — proves no boot on the hot path | `ClaimLatencyRecord` (metrics.rs, already CI-wired at `ci.yml:64`) |
| `gauge_tunnel_count` | live `count_tunnels()` per account vs the 800/950 watermarks | `MetricSample` gauge (metrics.rs) → cap-alert loop (§5.4) |
| `bench_refill_batch_throughput` | wall-clock to restore `POOL_REFILL_LOW_WATERMARK → WARM_POOL_DEPTH` (must beat claim rate, §6.1) | background-loop timing, logged |
| `gauge_pool_depth` | claimable depth per region; alerts if it approaches 0 before refill | `MetricSample` gauge |

Telemetry hook: claim latency + tunnel count + pool depth emit through the existing `metrics.rs` seam
so a refill-starvation or cap-approach surfaces automatically, not at review time (item 14). The
`ClaimLatencyRecord` type and its CI ledger **already exist** — P67 populates them, builds nothing new.

---

## 12. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe states are (a) *two hubs share one tunnel credential
  or one root* → made unrepresentable by §9.1's inject-per-hub-never-bake invariant + M3's
  spec-match; (b) *a claimed hub silently returns to the pool* → `Claimed → Warm` is an
  unrepresentable `PoolSlotState` transition (M3); (c) *an unsigned/unattributable routing change* →
  `TunnelMutation` requires a `HybridSig` and a write-ahead log entry (M2/M5). Reachability is argued
  from the state-machine + type structure, not policy.
- **Schemas & scaling axis (item 8):** scaling axis = **tunnels per CF account** (hard cap 1,000,
  §5/§6.2) and **warm-pool depth per region**. The shape changes at the cap: at ~800 tunnels the
  design shards to a second account (§5.3) — stated, not timeless. The mutation log grows unbounded
  (append-only); it is periodically archived (offline), never pruned in place (living memory, item 15).
- **Isolation / bulkhead (item 11):** the claim service holds **routing + ownership authority only —
  never keys, never card data, never hub application data** (§0.2, D12). A compromise of the
  provisioning plane can re-point routing (bounded, attributable via §5.2, alertable via §5.4) but
  **cannot forge a hub's identity** (keys are P59, never touch CF/Hetzner, R3 §1.5) nor read hub data
  (never centralized, §16.14). The infra plane's failure does not propagate to the identity or data
  plane — the same bulkhead P59 §11 states from its side.
- **Mesh awareness (item 12):** hubs are **node-local**; the **heartbeat is the one deliberate
  direct-to-collector exception** (liveness only, §8/§16.53), NOT gossip. The tunnel-config mutation
  log is service-local (closed), not mesh-propagated. Payload budgets: heartbeat ~3.4KB/emit (§8);
  no mesh frame carries provisioning data.
- **Living memory (item 15):** the mutation log is **append-only + temporally ordered** (seq,
  `at_tick`) — demote/archive-never-delete (the living-memory arc pattern). Pool slots are
  **topology-scoped** (region → pool → slot) and **time-scoped** (`Warm` freshness via heartbeat).
- **Rollback / self-healing vocabulary as math (item 13):** **Snapshot re-entry** = §4-C
  `suspend_preserving` + `resume_from` (regeneration from the last valid state snapshot) AND P68's
  age-snapshot-before-promote (§9.2) — cheap regenerative recovery, not error-correction.
  **Self-termination** = the unrepresentable `Claimed → Warm` transition + the shared-secret-is-
  unbakeable invariant (§9.1) — hard boundaries, not a supervisor's choice. **Self-healing is NOT
  claimed** for a lost owner root — P59 §5.2's honest stranding tradeoff governs; P67 does not invent a
  recovery backdoor (§4-B closed, consistent with P59).
- **Error-propagation / smart index (item 14):** the bug classes this introduces — a leaked
  closed-import, a shared baked secret, an unlogged routing change, a cap-cliff surprise — are each
  turned into a **compile-time/CI-time/test-time** failure: the §7 fence (CI), §9.1's inject-not-bake
  test (D10), §5.2's write-ahead-before-call test (D2), §5.4's cap alert + fence (D8). Not runtime
  surprises.
- **Tensor/spectral (item 16):** **N/A, honestly** — provisioning is orchestration + a state machine +
  API calls, not a linear-algebra kernel. Forcing `spectral.rs` here would be over-engineering
  (ponytail). Stated rather than shoehorned (same posture as P59 §11).
- **Linux discipline (item 9):** verdict framework — **EXTENDS** the ops-alert fence mechanism (one
  new declarative fence, §7) and the metrics seam (claim latency + cap gauge, §11); **REUSES** the
  event_log append-only shape (§5.2), the `/healthz` liveness split (§8), the `heartbeat-monitor.yml`
  alerting lane (§5.4); **REINFORCES** P59's key-isolation bulkhead from the infra side (§11 isolation);
  **DOES-NOT-TRANSFER**: no new daemon sprawl (Synapse lesson §1 — no heavyweight K8s/SPIRE-style
  provisioner); Packer is a build tool, not a runtime service.

---

## 13. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** the hub's routing identity (`hub-<HubId>.hubs.dowiz.org`)
  corresponds 1:1 to its cryptographic identity (`NodeId::from_keys`) bound at provision (§2.4) — the
  addressable handle and the self-describing key-hash are two faces of one hub, no external CA between
  them.
- **Cause & Effect:** every routing change has a signed, logged cause (`TunnelMutation.actor` +
  `HybridSig`, §5.2) — nothing in the fleet re-points by correlation or unattributed action. A
  compromise is always a *traceable* effect of a *named* actor.
- **Polarity / no-middle:** a hub is `Warm` (claimable) xor `Claimed`/`Suspended` (owned forever,
  §16.57) — there is no half-claimed middle state; `Claimed → Warm` is unrepresentable (§12). Ownership
  is binary and monotone, like P59's `RequireBoth`.

---

## 14. Standard-compliance map (all 20 points, checkable — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth with live `file:line` | §0 (green-field confirmation; the reused primitives cited; the P59 mint mechanism cited precisely) |
| 2 | Falsifiable DoD | §10 (D1–D14, each a RED→GREEN test or artifact check; incl. the two task-mandated tests D7/D8) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; `PoolSlotState` sequence asserts in M3/M6) |
| 4 | Predefined types & constants | §3 (`TunnelProvider`/`VpsProvider`, `PoolSlotState`, `TunnelMutation`, `AccountPool`, named consts) |
| 5 | Adversarial/breaking tests | §4 (every M has RED adversarial cases: reorder, forgery, cross-owner, spec-mismatch, cap-rollover) |
| 6 | Hazard-safety from type/state structure | §12 (unrepresentable `Claimed→Warm` + unbakeable-shared-secret + unsigned-mutation), §2.4 |
| 7 | Links to docs & memory | §15 |
| 8 | Schemas with scaling axis | §6.2/§12 (tunnels/account cap 1,000; pool depth/region; log append-only archive point) |
| 9 | Linux engineering discipline | §12 (EXTENDS/REUSES/REINFORCES/DOES-NOT-TRANSFER verdict) |
| 10 | Benchmarks + telemetry | §11 (claim latency via existing `ClaimLatencyRecord`; tunnel-count + pool-depth gauges) |
| 11 | Isolation / bulkhead | §12 (routing-authority-only claim service; keys/data/card never in the provisioning plane) |
| 12 | Mesh awareness | §8/§12 (node-local hubs; heartbeat the one direct-to-collector exception; payload budgets) |
| 13 | Rollback/self-heal as math | §12/§9.2/§6.1 (suspend-resume + age-snapshot = snapshot re-entry; no self-heal for lost root, honestly) |
| 14 | Error-propagation / smart index | §12 (CI fence, inject-not-bake test, write-ahead test, cap alert+fence — all compile/CI/test-time) |
| 15 | Living-memory awareness | §12 (append-only temporally-ordered mutation log; topology+time-scoped pool slots) |
| 16 | Tensor/spectral where applicable | §12 (N/A, stated honestly — orchestration, not linear algebra) |
| 17 | Regression tracking | §15 (D2 mutation-log + D7 fence + D8 cap-alert added to `REGRESSION-LEDGER.md`) |
| 18 | Clear worker instructions | §15 |
| 19 | Reuse-first, upgrade-if-needed | §0.1 (reuse event_log/healthz/metrics/fences), §1 (adopt CF/Hetzner/SPIFFE-model not invent), §2.2 (anti-scope) |
| 20 | Hermetic principles | §13 |

---

## 15. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / co-owns / cites:**
- `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (P67 row, W3), **X8** (identity upstream of claim),
  **X9** (golden snapshot integration point — **P67 owns image spec, P68 co-signs slot/backup**),
  §4-C (abandoned-hub suspend-but-preserve, CLOSED), §4-E (warm-pool depth + CF-token custody +
  mutation log + account-pool + CI fence = engineering decisions).
- `OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` §1 (CF Tunnel API flow + 1,000-cap + no-tenancy
  finding), §4 (Hetzner warm pool + snapshot refill), §5 (port prior art + **Synapse lesson**), §6.1
  (claim-mechanic), §7 risks #1/#4/#5 (CF blast radius, pool economics, shadow-ingress).
- **`BLUEPRINT-P59-capability-cert-chain.md`** — the cert primitives P67 hands out (`SelfSignedRoot::
  mint`, `NodeId::from_keys`, `DowizCoSign`, child `Delegation` `may_delegate=false`,
  `AnchorRoster::enroll`, `AlgSuite`). **Hard input dependency (§2.3).**
- **`BLUEPRINT-P68-hub-supervisor-update-backup.md`** — **golden-image co-owner** (§9): A/B slots +
  age-snapshot-before-promote + backup scheduler live in P68; P67 reserves their slots in the spec.
- `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.1/§16.2 (hosting topology, CF Tunnel),
  §16.12 (self-serve automated onboarding), §16.32 (claim mechanic), §16.45 (one CF account Wave-0),
  §16.53 (heartbeat), §16.54 (demo fixtures + open/closed split), §16.57 (claimed hub forever, now
  qualified by §4-C), §17.3 (tunnel-target-switch survival), §17.7 (self-signed root), §17.8/§17.9
  (Cloudflare/Hetzner swappable ports).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract this document is measured against).
- Format precedent: `BLUEPRINT-P51-open-map-routing.md`, `BLUEPRINT-P59-capability-cert-chain.md`.

**Existing code this blueprint reuses/extends (exact targets):**
- **NEW crate** `provision-ports` (OPEN) — `TunnelProvider`/`VpsProvider` traits + §3 wire types.
- **NEW crate** `dowiz-provision` (CLOSED) — `CloudflareTunnel`/`HetznerVps` adapters, pool manager,
  claim service, `TunnelMutation` log, cap-alert loop, heartbeat collector.
- **NEW (OPEN, hub-side)** heartbeat emitter + `switch_tunnel_target` escape hatch + shadow-ingress.
- **EDIT** `tools/ops-alert/fences.toml` — add `no-closed-import` + `tunnel-cap-alive` fences, bump
  `fence_count` 3→5.
- **REUSE unchanged** `kernel/src/event_log.rs` (log shape), `kernel/src/metrics.rs`
  (`ClaimLatencyRecord`/gauges), `tools/native-spa-server` `/healthz` (heartbeat shape),
  `.github/workflows/heartbeat-monitor.yml` (alert lane), `tools/ops-alert/src/fence_check.rs`.
- **CALL, never reimplement** `kernel/src/pq/cert_chain.rs` + `kernel/src/ports/agent/cap.rs` (P59).

**For the worker with zero session context — exact acceptance path:**
1. Land P59 first (hard dependency). Write §3 types in `provision-ports` (OPEN) before any adapter.
2. Implement M1→M8 in order; each M's RED tests fail before its code and pass after. Adapters are
   tested against `MockTunnel`/`MockVps` — **no live CF/Hetzner account in CI**.
3. Add the D2 (mutation-log tamper), D7 (open/closed fence), D8 (cap-alert) regression tests to
   `docs/regressions/REGRESSION-LEDGER.md`; bump `fence_count` with the two new fences.
4. `cargo test` fully green for both crates (D13). Run the `security fences` CI job locally to confirm
   the `no-closed-import` fence is live (D7).
5. **Do NOT mark P67 done until the live smoke (D14) passes** — one real hub provisioned end-to-end on
   the real dowiz CF account + Hetzner project, claimed, heartbeat green. This is a manual operator
   gate (no live creds in CI), logged not automated.
6. **Anti-scope (do NOT build):** the cert chain / any crypto (P59); the A/B update slots + supervisor
   + age backup logic (P68 — you only *reserve* their slots in the image spec §9); the owner/dowiz.org
   UI (P70/P73 — you expose the claim API, not the surface); the payment-account connect mechanics
   (P60/P72 — you expose the hook only); NO card-data type and NO recovery/break-glass path anywhere
   (§4-B closed, consistent with P59 §5.2).
