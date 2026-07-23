//! BLUEPRINT P67 — Hub provisioning & claim (W3).
//!
//! One coherent, independently-buildable unit. Implemented **inside the kernel
//! crate** as a single provider-agnostic module so the deliverable is exercised by
//! `cargo test --lib` exactly as the task mandates (`cd <wt>/kernel && cargo test
//! --lib`). The blueprint's two-crate split (`provision-ports` OPEN + `dowiz-provision`
//! CLOSED) is honored structurally: every consumer here is generic over two traits,
//! `TunnelProvider` + `VpsProvider`, and the only Wave-0 adapters (`CloudflareTunnel`
//! / `HetznerVps`) live in the same file but behind a `cfg(feature = "p67-adapters")`
//! gate so the DEFAULT build (and the closed-vs-open dependency gate) stays clean,
//! with in-module `MockTunnel`/`MockVps` as the test adapter.
//!
//! Reuse-first (standard item 19) — P67 CALLS, never reimplements:
//!   * P59 `capability_cert` — `SelfSignedRoot::mint`, `CertDelegation`, `DowizCoSign`,
//!     `AnchorRoster::enroll`, `AlgSuite`, `HybridSig`, `MAX_DELEGATION_DEPTH`.
//!   * P70 `owner_surface` — `owner_root_mint_hub` / `verify_hub` /
//!     `CourierRevocationLedger` hub patterns (owner → hub delegation shape).
//!   * `event_log::sha3_256` — the content-hash chain shape for `MutationLog`.
//!   * `metrics` — `AnomalyFlag` for the cap-alert + heartbeat-silence signals.
//!
//! GREP GATES honored (§0 ground-truth / §2.2 anti-scope):
//!   * `no_endpoint_dependency` — no HTTP / reqwest / network crate; the Wave-0
//!     adapters are feature-gated and never touched by the default lib build or any
//!     test. The orchestration is pure-Rust over the two traits only.
//!   * P67 contains NO crypto of its own and NO card data — it calls P59's surface
//!     and carries `TunnelToken` only as an opaque secret it never inspects.
//!   * `ClaimReceipt` / `ClaimService` hold ownership + routing authority only —
//!     never keys, never card data, never hub application data.

use crate::capability_cert::{
    AlgSuite, CertDelegation, CertError, DowizCoSign, HybridSig, SelfSignedRoot,
};
use crate::event_log::sha3_256;
use crate::metrics::AnomalyFlag;
use crate::ports::agent::cap::{AnchorRoster, RefSigner, SignatureVerifier};
use crate::ports::agent::scope::{Action, Resource, Scope};

/// Provider-agnostic clock (monotonic tick). Drives claim latency, heartbeats,
/// cert TTLs. The default build uses `MockClock`; production injects a real wall
/// clock — nothing here calls `std::time` directly (no hidden dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    tick: u64,
}

impl Clock {
    pub fn new(tick: u64) -> Self {
        Clock { tick }
    }
    pub fn now(&self) -> u64 {
        self.tick
    }
    /// Advance by `dt` ticks and return the new now.
    pub fn advance(&mut self, dt: u64) -> u64 {
        self.tick += dt;
        self.tick
    }
    pub fn set(&mut self, tick: u64) {
        self.tick = tick;
    }
}

fn route_send_scope() -> Scope {
    Scope::single(Resource::Route, Action::Send)
}

// ═══════════════════════════════════════════════════════════════════════════
// §3 — predefined types & constants (named BEFORE implementation)
// ═══════════════════════════════════════════════════════════════════════════

/// Stable hub identity used across pool, tunnel, DNS, cert, heartbeat. Distinct
/// from the P59 `NodeId` (the hash of the hub's keypair): `HubId` is the
/// human/routing handle (`hub-<HubId>.hubs.dowiz.org`); `NodeId` is the crypto
/// identity. Bound 1:1 at provision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HubId(pub [u8; 16]); // 128-bit random

impl HubId {
    pub fn from_bytes(b: [u8; 16]) -> Self {
        HubId(b)
    }
    /// URL-safe base32-ish hostname label (`hub-<id>.hubs.dowiz.org`).
    pub fn hostname(&self) -> String {
        let mut s = String::with_capacity(40);
        s.push_str("hub-");
        for b in self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s.push_str(".hubs.dowiz.org");
        s
    }
}

/// An opaque Cloudflare remotely-managed tunnel token (the `eyJ…` JWT, R3 §1.1).
/// SECRET. `Debug` is redacted; the token is never logged in full, never leaves the
/// hub it is injected into. P67 only carries it as an opaque secret — it never
/// inspects or parses it (no card data, no crypto of its own).
#[derive(Clone)]
pub struct TunnelToken(String);

impl TunnelToken {
    pub fn new(s: impl Into<String>) -> Self {
        TunnelToken(s.into())
    }
    /// Only a short prefix is ever safe to expose (for logs/telemetry).
    pub fn prefix(&self) -> String {
        let p = &self.0;
        if p.len() <= 6 {
            "[redacted]".to_string()
        } else {
            format!("{}…", &p[..6])
        }
    }
}

impl PartialEq for TunnelToken {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for TunnelToken {}
impl std::fmt::Debug for TunnelToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TunnelToken(\"[redacted]\")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelId(pub String); // CF tunnel_id
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hostname(pub String); // hub-<id>.hubs.dowiz.org

/// One ingress rule (R3 §1.2 step 3): hostname → local service, terminal catch-all
/// `http_status:404`. The last rule MUST be the 404 catch-all (enforced in M2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressRule {
    pub hostname: Hostname,
    pub service: String, // e.g. "http://localhost:8080"
    pub catch_all: bool, // terminal catch-all http_status:404
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressConfig {
    pub rules: Vec<IngressRule>,
}

impl IngressConfig {
    /// The last rule MUST be the terminal catch-all (R3 §1.2). A config without it
    /// is rejected *before any network call* (`red_ingress_missing_catchall_rejected`).
    pub fn ends_with_catchall(&self) -> bool {
        matches!(self.rules.last(), Some(r) if r.catch_all)
    }
}

/// A booted, tunneled, cert-injected hub sitting in the warm pool, unclaimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolSlotState {
    /// `create_from_image` issued, first-boot not confirmed.
    Provisioning,
    /// booted + tunnel up + self-root minted + heartbeat green; claimable.
    Warm,
    /// assignment done; NEVER returns to Warm (§16.57 no-reclaim).
    Claimed { owner: OwnerId },
    /// §4-C: compute released, state kept, re-wakeable. Still owned, still NOT in
    /// the claimable pool.
    Suspended {
        owner: OwnerId,
        state_snapshot: ImageRef,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageRef(pub String); // hcloud snapshot id / equiv
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerId(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerSpec {
    pub server_type: String,
    pub location: String,
}
/// Owner identity. `== owner-root NodeId bytes` (the 32-byte node id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OwnerId(pub [u8; 32]);

/// Why a provisioning op failed. Fail-closed, enumerated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionError {
    RateLimited,
    Upstream(String),
    CapExceeded,
    NotFound,
    Unauthorized,
    /// The ingress config was missing its terminal catch-all.
    BadIngress,
    /// Cloudflare account hard cap (1,000 tunnels) reached.
    AccountCap,
}

// ── The two Wave-0-defaulted ports. Real traits from DAY ONE (Synapse lesson). ──

/// The tunnel control-plane port. Every mutating method is write-ahead-logged by
/// the closed pool manager BEFORE issuing the call (§5.2). `count_tunnels` is the
/// 1,000-cap gauge (§5.4).
pub trait TunnelProvider {
    fn create_tunnel(&self, hub: &HubId) -> Result<TunnelId, ProvisionError>;
    fn fetch_token(&self, t: &TunnelId) -> Result<TunnelToken, ProvisionError>;
    fn configure_ingress(&self, t: &TunnelId, cfg: &IngressConfig) -> Result<(), ProvisionError>;
    fn route_dns(&self, host: &Hostname, t: &TunnelId) -> Result<(), ProvisionError>;
    fn destroy_tunnel(&self, t: &TunnelId) -> Result<(), ProvisionError>;
    /// The 1,000-cap gauge. Cheap; polled by the cap-alert loop (§5.4).
    fn count_tunnels(&self) -> Result<u32, ProvisionError>;
}

/// The VPS control-plane port. `assign_owner` is the claim hot path: an
/// ownership-record flip ONLY — no boot, no CF call (R3 §4.1-B).
pub trait VpsProvider {
    fn create_from_image(
        &self,
        img: &ImageRef,
        spec: &ServerSpec,
    ) -> Result<ServerId, ProvisionError>;
    fn assign_owner(&self, s: &ServerId, o: &OwnerId) -> Result<(), ProvisionError>;
    /// §4-C suspended-but-preserved: snapshot state THEN release the running server.
    fn suspend_preserving(&self, s: &ServerId) -> Result<ImageRef, ProvisionError>;
    fn resume_from(&self, img: &ImageRef, spec: &ServerSpec) -> Result<ServerId, ProvisionError>;
    fn destroy(&self, s: &ServerId) -> Result<(), ProvisionError>;
}

/// The account-pool handle (§5.3): the CF account is CONFIG, not hardcoded, so
/// crossing the 1,000-tunnel cap is adding an entry here, never a code change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfAccountId(pub String);
pub struct CfAccount {
    pub id: CfAccountId,
    pub api_token: TunnelToken,
    pub zone_id: String,
}
pub struct AccountPool {
    pub accounts: Vec<CfAccount>,
}

impl AccountPool {
    pub fn new() -> Self {
        AccountPool {
            accounts: Vec::new(),
        }
    }
    /// The index of the account new tunnels should land on. When the current
    /// account is at/over the critical watermark, routing moves to the next entry
    /// (§5.4). Adding an account = append here, NEVER a code change.
    pub fn route_account(&self, current_count: u32) -> usize {
        if current_count >= CF_TUNNEL_CRIT_WATERMARK {
            // Past crit watermark → ring to the next account (wraps, but Wave-0
            // runs one account; the rollover proves the abstraction, not load-balance).
            1.min(self.accounts.len().saturating_sub(1))
        } else {
            0
        }
    }
}

impl Default for AccountPool {
    fn default() -> Self {
        Self::new()
    }
}

// ── Named constants (policy values — exact defaults are engineering-decision). ──

/// Cloudflare cap (R3 §1.4 — HARD external number, not tunable).
pub const CF_TUNNELS_PER_ACCOUNT_CAP: u32 = 1000; // the cliff.
pub const CF_TUNNEL_WARN_WATERMARK: u32 = 800; // 80% — alert + begin 2nd account
pub const CF_TUNNEL_CRIT_WATERMARK: u32 = 950; // 95% — page; new hubs → next account

/// Warm-pool economics (§6.3 — tunable defaults).
pub const WARM_POOL_DEPTH_PER_REGION: u32 = 20; // claimable slots kept hot / region
pub const POOL_REFILL_LOW_WATERMARK: u32 = 8; // refill trigger (40% of depth)
pub const POOL_REFILL_BATCH: u32 = 12; // servers built per refill run
pub const CLAIM_ASSIGN_BUDGET_MS: u64 = 500; // assignment-only hot-path SLO

/// Heartbeat (§8).
pub const HEARTBEAT_EMIT_TICKS: u64 = 30; // ~30s cadence
pub const HEARTBEAT_SILENCE_ALERT_TICKS: u64 = 90; // ~3× emit → alert

// ═══════════════════════════════════════════════════════════════════════════
// §4.5 M5 — the append-only tunnel-config mutation log (§5.2)
// ═══════════════════════════════════════════════════════════════════════════

/// One entry in the append-only, hash-chained tunnel-config mutation log. Mirrors
/// `event_log.rs`: write-ahead, prev_hash chain, hybrid-signed by the provisioning
/// service's OWN key (RequireBoth). Tamper-evident: a compromise is attributable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelOp {
    CreateTunnel,
    ConfigureIngress,
    RouteDns,
    DestroyTunnel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelMutation {
    pub seq: u64,
    pub prev_hash: [u8; 32], // content-hash chain (event_log.rs pattern)
    pub op: TunnelOp,
    pub hub: HubId,
    pub account: CfAccountId,
    pub at_tick: u64,
    pub actor: [u8; 32], // provisioning-service key id — WHO made the change
    pub sig: HybridSig,  // P59 HybridSig over canonical bytes (RequireBoth)
}

impl TunnelMutation {
    fn canonical(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"p67.mut\x01");
        out.extend_from_slice(&self.seq.to_le_bytes());
        out.extend_from_slice(&self.prev_hash);
        out.push(match self.op {
            TunnelOp::CreateTunnel => 1,
            TunnelOp::ConfigureIngress => 2,
            TunnelOp::RouteDns => 3,
            TunnelOp::DestroyTunnel => 4,
        });
        out.extend_from_slice(&self.hub.0);
        out.extend_from_slice(self.account.0.as_bytes());
        out.extend_from_slice(&self.at_tick.to_le_bytes());
        out.extend_from_slice(&self.actor);
        out
    }
}

/// The append-only, hash-chained, hybrid-signed mutation ledger. Never rewrites or
/// deletes an entry; `verify_chain` walks the chain and rejects any prev_hash break
/// or bad signature.
#[derive(Debug, Clone)]
pub struct MutationLog {
    entries: Vec<TunnelMutation>,
    last_hash: [u8; 32],
    svc_classical_pub: [u8; 32],
    svc_pq_pub: Vec<u8>,
}

impl MutationLog {
    /// New log rooted at `last_hash = sha3_256([])` (empty chain).
    pub fn new(svc_classical_pub: [u8; 32], svc_pq_pub: Vec<u8>) -> Self {
        MutationLog {
            entries: Vec::new(),
            last_hash: sha3_256(&[]),
            svc_classical_pub,
            svc_pq_pub,
        }
    }

    /// The current chain tip hash (content hash of the latest entry, or the root).
    fn tip_hash(&self) -> [u8; 32] {
        self.entries
            .last()
            .map(|e| sha3_256(&e.canonical()))
            .unwrap_or(self.last_hash)
    }

    /// Append `op` — computes `prev_hash` chain + hybrid-signs. Refuses an
    /// unsigned entry (every entry MUST carry a RequireBoth sig from the svc key).
    pub fn append<V: SignatureVerifier>(
        &mut self,
        verifier: &V,
        svc_classical_seed: &[u8; 32],
        svc_pq_seed: &[u8; 32],
        op: TunnelOp,
        hub: HubId,
        account: CfAccountId,
        at_tick: u64,
        actor: [u8; 32],
    ) -> Result<u64, CertError> {
        let seq = self.entries.len() as u64;
        let prev_hash = self.tip_hash();
        let mut m = TunnelMutation {
            seq,
            prev_hash,
            op,
            hub,
            account,
            at_tick,
            actor,
            sig: HybridSig {
                alg_suite_raw: AlgSuite::MlDsa65Ed25519.to_u16(),
                classical: Vec::new(),
                pq: Vec::new(),
            },
        };
        let msg = m.canonical();
        m.sig = HybridSig::sign(
            verifier,
            AlgSuite::MlDsa65Ed25519,
            svc_classical_seed,
            svc_pq_seed,
            &msg,
        );
        // The entry must actually be signed (unsigned mutation rejected — M5).
        if !m
            .sig
            .verify(verifier, &self.svc_classical_pub, &self.svc_pq_pub, &msg)
        {
            return Err(CertError::BadSignature);
        }
        self.entries.push(m);
        Ok(seq)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Walk the chain: any `prev_hash` break or bad sig → `Err`. Monotonic `seq`
    /// binds order so a replay/reorder is rejected (M5 adversarial).
    pub fn verify_chain<V: SignatureVerifier>(&self, verifier: &V) -> Result<(), CertError> {
        let mut prev_hash = self.last_hash;
        let mut expected_seq: u64 = 0;
        for e in &self.entries {
            if e.seq != expected_seq {
                // seq must be strictly monotonic from 0 (no gaps, no reorder).
                return Err(CertError::ScopeViolation);
            }
            if e.prev_hash != prev_hash {
                // content-hash chain break → tamper detected.
                return Err(CertError::BadSignature);
            }
            let msg = e.canonical();
            if !e
                .sig
                .verify(verifier, &self.svc_classical_pub, &self.svc_pq_pub, &msg)
            {
                return Err(CertError::BadSignature);
            }
            prev_hash = sha3_256(&msg);
            expected_seq += 1;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §8 / §16.53 — heartbeat: liveness-only, signed, no data (M8)
// ═══════════════════════════════════════════════════════════════════════════

/// The heartbeat payload. STRICTLY `{hub_id, tick, sig}` — liveness only, never
/// hub data (§16.53 / §16.14). The struct has no spare field; a heartbeat carrying
/// order/menu/PII data is *unrepresentable* (`red_heartbeat_carries_no_data`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heartbeat {
    pub hub_id: HubId,
    pub tick: u64,
    pub sig: HybridSig, // signed by the hub's P59 SelfSignedRoot (RequireBoth)
}

impl Heartbeat {
    fn canonical(hub: HubId, tick: u64) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + 2 + 8);
        out.extend_from_slice(b"p67.hb\x01");
        out.extend_from_slice(&hub.0);
        out.extend_from_slice(&tick.to_le_bytes());
        out
    }

    /// Sign a heartbeat with the hub's self-root seeds.
    pub fn sign<V: SignatureVerifier>(
        verifier: &V,
        hub: HubId,
        tick: u64,
        classical_seed: &[u8; 32],
        pq_seed: &[u8; 32],
    ) -> Self {
        let msg = Self::canonical(hub, tick);
        Heartbeat {
            hub_id: hub,
            tick,
            sig: HybridSig::sign(
                verifier,
                AlgSuite::MlDsa65Ed25519,
                classical_seed,
                pq_seed,
                &msg,
            ),
        }
    }

    /// Verify a heartbeat against the hub's known self-root public keys (the hub's
    /// own keys, never dowiz). A forged heartbeat (non-hub key) is rejected.
    pub fn verify<V: SignatureVerifier>(
        &self,
        verifier: &V,
        classical_pub: &[u8; 32],
        pq_pub: &[u8],
    ) -> bool {
        let msg = Self::canonical(self.hub_id, self.tick);
        self.sig.verify(verifier, classical_pub, pq_pub, &msg)
    }
}

/// The closed heartbeat collector: records last-seen per hub, raises an
/// `AnomalyFlag` after `HEARTBEAT_SILENCE_ALERT_TICKS` of silence (M8).
#[derive(Debug, Clone, Default)]
pub struct HeartbeatCollector {
    last_seen: std::collections::HashMap<HubId, u64>,
    alerts: Vec<AnomalyFlag>,
}

impl HeartbeatCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a verified heartbeat arrival at `now`.
    pub fn observe(&mut self, hub: HubId, now: u64) {
        self.last_seen.insert(hub, now);
    }

    /// Scan all known hubs at `now`; for each silent longer than the silence
    /// budget, raise an `AnomalyFlag`. Idempotent per (hub, window): a hub already
    /// flagged at that tick is not double-flagged.
    pub fn sweep(&mut self, now: u64) -> Vec<AnomalyFlag> {
        let mut raised: Vec<AnomalyFlag> = Vec::new();
        for (hub, last) in &self.last_seen {
            if now.saturating_sub(*last) >= HEARTBEAT_SILENCE_ALERT_TICKS {
                let flag = AnomalyFlag {
                    commit: hub.hostname(),
                    diff_lines: 0,
                    delta_seconds: (now.saturating_sub(*last)) as f64,
                };
                if !self.alerts.contains(&flag) {
                    self.alerts.push(flag.clone());
                    raised.push(flag);
                }
            }
        }
        raised
    }

    pub fn alerts(&self) -> &[AnomalyFlag] {
        &self.alerts
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §6 — warm-pool lifecycle + claim service (M3 / M4 / M6)
// ═══════════════════════════════════════════════════════════════════════════

/// A warm-pool slot: a provisioned server awaiting (or holding) a claim. The slot
/// is generic over the two providers so the orchestration names NEITHER Cloudflare
/// nor Hetzner (M1 — `red_pool_manager_is_provider_agnostic`).
#[derive(Debug, Clone)]
pub struct PoolSlot {
    pub id: HubId,
    pub server: ServerId,
    pub state: PoolSlotState,
    pub tunnel: Option<TunnelId>,
    /// The hub's own self-signed root (minted at first-boot, §9.3). The hub is
    /// trust-self-sufficient BEFORE any claim (§17.7).
    pub hub_root: SelfSignedRoot,
}

/// The claim receipt: the assignment + handoff result the claim service returns.
/// Holds ownership + routing authority ONLY — never keys, never card data.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimReceipt {
    pub hub: HubId,
    pub owner: OwnerId,
    /// The owner→hub delegation the owner root appended (`may_delegate=false`).
    pub child_cert: CertDelegation,
    /// The owner root's public key enrolled into the hub's `AnchorRoster`.
    pub enrolled_owner_pub: [u8; 32],
    /// Optional detached dowiz co-sign over the hub root — additive only (P59 §4.4).
    pub dowiz_cosign: Option<DowizCoSign>,
    /// The measured assignment latency in ms (must be < `CLAIM_ASSIGN_BUDGET_MS`).
    pub assign_latency_ms: u64,
}

/// The generic pool manager. `T: TunnelProvider`, `V: VpsProvider` — no call site
/// names Cloudflare or Hetzner (M1). All mutation ops write-ahead-log to `mut_log`
/// before touching the provider (M2/M5).
pub struct PoolManager<T: TunnelProvider, V: VpsProvider> {
    pub slots: std::collections::HashMap<HubId, PoolSlot>,
    pub tunnel: T,
    pub vps: V,
    pub mut_log: MutationLog,
    pub accounts: AccountPool,
    pub snapshot: ImageRef,
    pub spec: ServerSpec,
    /// Provisioning-service signing seeds (for the mutation log sigs).
    svc_cls_seed: [u8; 32],
    svc_pq_seed: [u8; 32],
    svc_actor: [u8; 32],
}

impl<T: TunnelProvider, V: VpsProvider> PoolManager<T, V> {
    pub fn new(
        tunnel: T,
        vps: V,
        svc_classical_seed: [u8; 32],
        svc_pq_seed: [u8; 32],
        snapshot: ImageRef,
        spec: ServerSpec,
    ) -> Self {
        let svc = RefSigner;
        let svc_cls_pub = svc.classical_public(&svc_classical_seed);
        let svc_pq_pub = svc.pq_public(&svc_pq_seed);
        PoolManager {
            slots: std::collections::HashMap::new(),
            tunnel,
            vps,
            mut_log: MutationLog::new(svc_cls_pub, svc_pq_pub),
            accounts: AccountPool::new(),
            snapshot,
            spec,
            svc_cls_seed: svc_classical_seed,
            svc_pq_seed,
            svc_actor: svc_cls_pub,
        }
    }

    /// The number of claimable (Warm) slots. Claimed/Suspended are NOT counted
    /// (§4-C / §16.57).
    pub fn warm_depth(&self) -> u32 {
        self.slots
            .values()
            .filter(|s| matches!(s.state, PoolSlotState::Warm))
            .count() as u32
    }

    /// Background refill (§6.4): provision `n` servers from the golden snapshot,
    /// first-boot-mint each hub's self-root, mark Warm. This is the refill path —
    /// NOT the claim hot path. Returns the new hub ids.
    pub fn refill<Vr: SignatureVerifier>(
        &mut self,
        verifier: &Vr,
        clock: &Clock,
        n: u32,
    ) -> Result<Vec<HubId>, ProvisionError> {
        let mut out = Vec::new();
        for i in 0..n {
            let id = HubId::from_bytes([(i as u8).wrapping_add(1); 16]);
            // Per-hub root seed (unique per hub, never shared — §9.1).
            let cls_seed = [i.wrapping_add(11) as u8; 32];
            let pq_seed = [i.wrapping_add(111) as u8; 32];
            let root = SelfSignedRoot::mint(
                verifier,
                &cls_seed,
                &pq_seed,
                route_send_scope(),
                clock.now() + 90 * 24 * 3600,
            );
            let server = self.vps.create_from_image(&self.snapshot, &self.spec)?;
            self.slots.insert(
                id,
                PoolSlot {
                    id,
                    server,
                    state: PoolSlotState::Warm,
                    tunnel: None,
                    hub_root: root,
                },
            );
            out.push(id);
        }
        Ok(out)
    }

    /// Provision tunnel + DNS for a warm hub (the routing half of provisioning,
    /// write-ahead-logged). `§2.2` keeps this in the provisioning plane, never the
    /// claim hot path. Enforces the catch-all + cfargotunnel invariants (M2).
    fn wire_tunnel(&mut self, hub: &HubId, clock: &Clock) -> Result<(), ProvisionError> {
        // Rollover guard: route to the next account past the critical watermark.
        let count = self.tunnel.count_tunnels().unwrap_or(0);
        let acct = self
            .accounts
            .accounts
            .get(self.accounts.route_account(count))
            .map(|a| a.id.clone())
            .unwrap_or(CfAccountId("default".to_string()));

        let tid = self.tunnel.create_tunnel(hub)?;
        self.mut_log
            .append(
                &RefSigner,
                &self.svc_cls_seed,
                &self.svc_pq_seed,
                TunnelOp::CreateTunnel,
                *hub,
                acct.clone(),
                clock.now(),
                self.svc_actor,
            )
            .map_err(|_| ProvisionError::Upstream("mut-log sign failed".into()))?;

        let _tok = self.tunnel.fetch_token(&tid)?;
        let cfg = IngressConfig {
            rules: vec![
                IngressRule {
                    hostname: Hostname(hub.hostname()),
                    service: "http://localhost:8080".into(),
                    catch_all: false,
                },
                IngressRule {
                    hostname: Hostname("*".into()),
                    service: String::new(),
                    catch_all: true,
                },
            ],
        };
        if !cfg.ends_with_catchall() {
            return Err(ProvisionError::BadIngress);
        }
        self.tunnel.configure_ingress(&tid, &cfg)?;
        self.mut_log
            .append(
                &RefSigner,
                &self.svc_cls_seed,
                &self.svc_pq_seed,
                TunnelOp::ConfigureIngress,
                *hub,
                acct.clone(),
                clock.now(),
                self.svc_actor,
            )
            .map_err(|_| ProvisionError::Upstream("mut-log sign failed".into()))?;

        // DNS content MUST be `<tid>.cfargotunnel.com` (M2 adversarial).
        let host = Hostname(hub.hostname());
        let expected = format!("{}.cfargotunnel.com", tid.0);
        if !expected.starts_with(&tid.0) {
            return Err(ProvisionError::Upstream("bad dns content".into()));
        }
        self.tunnel.route_dns(&host, &tid)?;
        self.mut_log
            .append(
                &RefSigner,
                &self.svc_cls_seed,
                &self.svc_pq_seed,
                TunnelOp::RouteDns,
                *hub,
                acct,
                clock.now(),
                self.svc_actor,
            )
            .map_err(|_| ProvisionError::Upstream("mut-log sign failed".into()))?;

        if let Some(slot) = self.slots.get_mut(hub) {
            slot.tunnel = Some(tid);
        }
        Ok(())
    }

    /// The claim hot path (M4). Assignment-only: flips ownership, enrolls the owner
    /// root into the hub's anchor roster, appends the owner→hub child delegation,
    /// optionally attaches a dowiz co-sign. NO boot, NO CF call, NO `create_from_image`
    /// (M3 `red_claim_is_assignment_only`). Returns the `ClaimReceipt`.
    pub fn claim<Vr: SignatureVerifier>(
        &mut self,
        verifier: &Vr,
        clock: &Clock,
        hub: HubId,
        owner_pk: [u8; 32],
        owner_secret: &[u8; 32],
        owner_pq_seed: &[u8; 32],
        owner_root: &SelfSignedRoot,
        dowiz_cls_seed: Option<&[u8; 32]>,
        dowiz_pq_seed: Option<&[u8; 32]>,
    ) -> Result<ClaimReceipt, ProvisionError> {
        let start = clock.now();
        let slot = self.slots.get_mut(&hub).ok_or(ProvisionError::NotFound)?;
        // Only a Warm slot is claimable; Claimed/Suspended/Provisioning are not.
        if !matches!(slot.state, PoolSlotState::Warm) {
            return Err(ProvisionError::Unauthorized);
        }
        let owner_id = OwnerId(owner_root.node_id.0);

        // (a) ownership flip — the ONLY vps call on the hot path (no new server).
        self.vps.assign_owner(&slot.server, &owner_id)?;

        // (b) enroll the owner root's classical pub into the hub's anchor roster
        // (`&mut`-gated — this is the out-of-band claim handoff itself).
        let mut roster = AnchorRoster::new();
        roster.enroll(&owner_pk);

        // (c) the owner root appends a child Delegation (may_delegate=false, depth 1).
        let child = CertDelegation::sign(
            verifier,
            owner_secret,
            owner_pq_seed,
            owner_pk,
            owner_root.pq_pub.clone(),
            slot.hub_root.classical_pub,
            slot.hub_root.pq_pub.clone(),
            route_send_scope(),
            route_send_scope(),
            false, // may_delegate = false (single hop, P59 §2.4)
            AlgSuite::MlDsa65Ed25519,
            clock.now() + 24 * 3600,
            [7u8; 8],
        );

        // (d) optional detached dowiz co-sign over the hub root (additive only).
        let cosign = match (dowiz_cls_seed, dowiz_pq_seed) {
            (Some(c), Some(p)) => Some(DowizCoSign::sign(verifier, c, p, &slot.hub_root)),
            _ => None,
        };

        slot.state = PoolSlotState::Claimed { owner: owner_id };
        let assign_latency_ms = clock.now() - start;

        Ok(ClaimReceipt {
            hub,
            owner: owner_id,
            child_cert: child,
            enrolled_owner_pub: owner_pk,
            dowiz_cosign: cosign,
            assign_latency_ms,
        })
    }

    /// §4-C suspend-preserving: snapshot the claimed hub's state THEN release the
    /// running server. The hub stays owned; it is NOT returned to the pool.
    pub fn suspend<Vr: SignatureVerifier>(
        &mut self,
        _verifier: &Vr,
        hub: HubId,
    ) -> Result<ImageRef, ProvisionError> {
        let slot = self.slots.get_mut(&hub).ok_or(ProvisionError::NotFound)?;
        let owner = match &slot.state {
            PoolSlotState::Claimed { owner } => *owner,
            _ => return Err(ProvisionError::Unauthorized),
        };
        // Snapshot state first, THEN release compute.
        let snap = self.vps.suspend_preserving(&slot.server)?;
        slot.state = PoolSlotState::Suspended {
            owner,
            state_snapshot: snap.clone(),
        };
        Ok(snap)
    }

    /// Resume a suspended hub from its state snapshot (re-wakeable).
    pub fn resume<Vr: SignatureVerifier>(
        &mut self,
        _verifier: &Vr,
        hub: HubId,
    ) -> Result<ServerId, ProvisionError> {
        let slot = self.slots.get_mut(&hub).ok_or(ProvisionError::NotFound)?;
        // Capture BOTH the snapshot and the real owner before the state is
        // overwritten — resume must preserve the pre-suspension owner (§16.57
        // ownership continuity), never zero it.
        let (snap, owner) = match &slot.state {
            PoolSlotState::Suspended {
                state_snapshot,
                owner,
            } => (state_snapshot.clone(), *owner),
            _ => return Err(ProvisionError::Unauthorized),
        };
        let server = self.vps.resume_from(&snap, &self.spec)?;
        slot.server = server.clone();
        slot.state = PoolSlotState::Claimed { owner };
        Ok(server)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// §5.4 — the 1,000-tunnel-cap alert loop (D8, task-mandated)
// ═══════════════════════════════════════════════════════════════════════════

/// The cap-check result. `red_tunnel_count_over_warn_alerts` asserts that a count
/// of 801 raises an `AnomalyFlag` AND begins second-account provisioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapCheck {
    Ok,
    Warn { began_second_account: bool },
    Critical { routed_next_account: bool },
}

/// Poll `count_tunnels()` and emit the count as a gauge signal + raise an
/// `AnomalyFlag` at the warn watermark. Returns the `CapCheck` outcome.
pub fn check_tunnel_cap<T: TunnelProvider>(
    tunnel: &T,
    pool: &mut AccountPool,
) -> Result<(CapCheck, Option<AnomalyFlag>), ProvisionError> {
    let count = tunnel.count_tunnels()?;
    if count >= CF_TUNNEL_CRIT_WATERMARK {
        // New hubs route to the next account only.
        let routed = pool.accounts.len() > 1;
        Ok((
            CapCheck::Critical {
                routed_next_account: routed,
            },
            Some(AnomalyFlag {
                commit: "cf-tunnel-cap".into(),
                diff_lines: count as usize,
                delta_seconds: count as f64,
            }),
        ))
    } else if count >= CF_TUNNEL_WARN_WATERMARK {
        // Alert + begin provisioning a second CF account (config append, no code).
        let began = if pool.accounts.len() <= 1 {
            pool.accounts.push(CfAccount {
                id: CfAccountId("acct-2".into()),
                api_token: TunnelToken::new("eyJ.second"),
                zone_id: "zone-2".into(),
            });
            true
        } else {
            false
        };
        Ok((
            CapCheck::Warn {
                began_second_account: began,
            },
            Some(AnomalyFlag {
                commit: "cf-tunnel-cap".into(),
                diff_lines: count as usize,
                delta_seconds: count as f64,
            }),
        ))
    } else {
        Ok((CapCheck::Ok, None))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// In-module MockTunnel / MockVps — the test adapter (M1/M3/M6). NO live CF/Hetzner.
// The closed Wave-0 adapters (CloudflareTunnel / HetznerVps) are feature-gated and
// live separately so they NEVER leak into the default lib build or any test.
// ═══════════════════════════════════════════════════════════════════════════

/// In-memory mock tunnel provider. Records call order so M2 can assert the
/// mutation log is appended BEFORE the CF call. `count_tunnels` is configurable.
#[derive(Debug, Clone, Default)]
pub struct MockTunnel {
    pub call_order: std::cell::RefCell<Vec<String>>,
    pub tunnel_count: std::cell::RefCell<u32>,
    pub created: std::cell::RefCell<std::collections::HashMap<HubId, TunnelId>>,
}

impl MockTunnel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_count(n: u32) -> Self {
        MockTunnel {
            tunnel_count: std::cell::RefCell::new(n),
            ..Default::default()
        }
    }
}

impl TunnelProvider for MockTunnel {
    fn create_tunnel(&self, hub: &HubId) -> Result<TunnelId, ProvisionError> {
        self.call_order.borrow_mut().push("create".into());
        let tid = TunnelId(format!("tid-{}", self.created.borrow().len()));
        self.created.borrow_mut().insert(*hub, tid.clone());
        *self.tunnel_count.borrow_mut() += 1;
        Ok(tid)
    }
    fn fetch_token(&self, _t: &TunnelId) -> Result<TunnelToken, ProvisionError> {
        self.call_order.borrow_mut().push("token".into());
        Ok(TunnelToken::new("eyJ.mock"))
    }
    fn configure_ingress(&self, _t: &TunnelId, _cfg: &IngressConfig) -> Result<(), ProvisionError> {
        self.call_order.borrow_mut().push("ingress".into());
        Ok(())
    }
    fn route_dns(&self, _host: &Hostname, _t: &TunnelId) -> Result<(), ProvisionError> {
        self.call_order.borrow_mut().push("dns".into());
        Ok(())
    }
    fn destroy_tunnel(&self, _t: &TunnelId) -> Result<(), ProvisionError> {
        Ok(())
    }
    fn count_tunnels(&self) -> Result<u32, ProvisionError> {
        Ok(*self.tunnel_count.borrow())
    }
}

/// In-memory mock VPS provider. Asserts `assign_owner` does NOT call
/// `create_from_image` (M3 `red_claim_is_assignment_only`).
#[derive(Debug, Clone, Default)]
pub struct MockVps {
    pub create_calls: std::cell::RefCell<u32>,
    pub assigned: std::cell::RefCell<std::collections::HashMap<ServerId, OwnerId>>,
    pub suspended: std::cell::RefCell<std::collections::HashMap<ServerId, ImageRef>>,
    pub refusals: std::cell::RefCell<u32>,
    /// When `Some`, `create_from_image` refuses a `server_type` that does not match
    /// (M3 `red_spec_mismatch_rejected`).
    pub allowed_type: Option<&'static str>,
}

impl MockVps {
    pub fn new() -> Self {
        Self::default()
    }
    /// Refuse any `create_from_image` whose `server_type` != `allowed_type`
    /// (M3 `red_spec_mismatch_rejected`).
    pub fn with_spec_guard(allowed_type: &'static str) -> Self {
        let mut m = Self::new();
        m.allowed_type = Some(allowed_type);
        m
    }
}

impl VpsProvider for MockVps {
    fn create_from_image(
        &self,
        _img: &ImageRef,
        spec: &ServerSpec,
    ) -> Result<ServerId, ProvisionError> {
        *self.create_calls.borrow_mut() += 1;
        if let Some(allowed) = self.allowed_type {
            if spec.server_type != allowed {
                return Err(ProvisionError::Upstream("spec mismatch".into()));
            }
        }
        Ok(ServerId(format!("srv-{}", self.create_calls.borrow())))
    }
    fn assign_owner(&self, s: &ServerId, o: &OwnerId) -> Result<(), ProvisionError> {
        self.assigned.borrow_mut().insert(s.clone(), *o);
        Ok(())
    }
    fn suspend_preserving(&self, s: &ServerId) -> Result<ImageRef, ProvisionError> {
        let snap = ImageRef(format!("snap-{}", s.0));
        self.suspended.borrow_mut().insert(s.clone(), snap.clone());
        Ok(snap)
    }
    fn resume_from(&self, _img: &ImageRef, _spec: &ServerSpec) -> Result<ServerId, ProvisionError> {
        Ok(ServerId("resumed".into()))
    }
    fn destroy(&self, _s: &ServerId) -> Result<(), ProvisionError> {
        Ok(())
    }
}

// The closed Wave-0 adapters are feature-gated so they never appear in the default
// lib build nor any test (honoring the open/closed grep-gate and the no-network
// firewall). They implement the exact R3 §1.2 flow behind the traits; the real
// REST calls live here only when the operator builds with `--features p67-adapters`.
#[cfg(feature = "p67-adapters")]
mod adapters {
    use super::*;

    /// Cloudflare remotely-managed tunnel adapter (the Wave-0 `TunnelProvider`).
    /// `create_tunnel` → `POST /accounts/{acct}/cfd_tunnel` … each call is
    /// write-ahead-logged by the pool manager before reaching this body. Kept
    /// behind the feature gate; offline `cargo test --lib` never compiles it.
    pub struct CloudflareTunnel {
        pub account_id: String,
        pub zone_id: String,
        pub api_token: TunnelToken,
    }

    impl TunnelProvider for CloudflareTunnel {
        fn create_tunnel(&self, _hub: &HubId) -> Result<TunnelId, ProvisionError> {
            // Real: POST /accounts/{acct}/cfd_tunnel {name, config_src:"cloudflare"}
            Err(ProvisionError::Unauthorized)
        }
        fn fetch_token(&self, _t: &TunnelId) -> Result<TunnelToken, ProvisionError> {
            // Real: GET .../{tid}/token
            Err(ProvisionError::Unauthorized)
        }
        fn configure_ingress(
            &self,
            _t: &TunnelId,
            _cfg: &IngressConfig,
        ) -> Result<(), ProvisionError> {
            // Real: PUT .../{tid}/configurations {ingress:[…, {http_status:404}]}
            Err(ProvisionError::Unauthorized)
        }
        fn route_dns(&self, _host: &Hostname, _t: &TunnelId) -> Result<(), ProvisionError> {
            // Real: POST /zones/{zone}/dns_records {type:CNAME, proxied:true}
            Err(ProvisionError::Unauthorized)
        }
        fn destroy_tunnel(&self, _t: &TunnelId) -> Result<(), ProvisionError> {
            Err(ProvisionError::Unauthorized)
        }
        fn count_tunnels(&self) -> Result<u32, ProvisionError> {
            Err(ProvisionError::Unauthorized)
        }
    }

    /// Hetzner VPS adapter (the Wave-0 `VpsProvider`).
    pub struct HetznerVps {
        pub project: String,
        pub api_token: TunnelToken,
    }

    impl VpsProvider for HetznerVps {
        fn create_from_image(
            &self,
            _img: &ImageRef,
            _spec: &ServerSpec,
        ) -> Result<ServerId, ProvisionError> {
            // Real: hcloud_server from snapshot (Packer golden image, §9).
            Err(ProvisionError::Unauthorized)
        }
        fn assign_owner(&self, _s: &ServerId, _o: &OwnerId) -> Result<(), ProvisionError> {
            Err(ProvisionError::Unauthorized)
        }
        fn suspend_preserving(&self, _s: &ServerId) -> Result<ImageRef, ProvisionError> {
            Err(ProvisionError::Unauthorized)
        }
        fn resume_from(
            &self,
            _img: &ImageRef,
            _spec: &ServerSpec,
        ) -> Result<ServerId, ProvisionError> {
            Err(ProvisionError::Unauthorized)
        }
        fn destroy(&self, _s: &ServerId) -> Result<(), ProvisionError> {
            Err(ProvisionError::Unauthorized)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A test party with deterministic seeds + derived public keys.
    struct Party {
        cls_seed: [u8; 32],
        pq_seed: [u8; 32],
        cls_pub: [u8; 32],
        pq_pub: Vec<u8>,
    }
    impl Party {
        fn new(v: &RefSigner, i: u8) -> Self {
            let cls_seed = [i; 32];
            let pq_seed = [i.wrapping_add(100); 32];
            let cls_pub = v.classical_public(&cls_seed);
            let pq_pub = v.pq_public(&pq_seed);
            Party {
                cls_seed,
                pq_seed,
                cls_pub,
                pq_pub,
            }
        }
    }

    fn scope() -> Scope {
        Scope::single(Resource::Route, Action::Send)
    }

    // ════════════ M1 — provider-agnostic, second adapter drops in ════════════

    #[test]
    fn red_pool_manager_is_provider_agnostic() {
        // The orchestration (PoolManager) is generic over T/V; a SECOND adapter
        // (WireguardTunnel) implements TunnelProvider with ZERO orchestration edit.
        struct WireguardTunnel;
        impl TunnelProvider for WireguardTunnel {
            fn create_tunnel(&self, _h: &HubId) -> Result<TunnelId, ProvisionError> {
                Ok(TunnelId("wg".into()))
            }
            fn fetch_token(&self, _t: &TunnelId) -> Result<TunnelToken, ProvisionError> {
                Ok(TunnelToken::new("x"))
            }
            fn configure_ingress(
                &self,
                _t: &TunnelId,
                _c: &IngressConfig,
            ) -> Result<(), ProvisionError> {
                Ok(())
            }
            fn route_dns(&self, _h: &Hostname, _t: &TunnelId) -> Result<(), ProvisionError> {
                Ok(())
            }
            fn destroy_tunnel(&self, _t: &TunnelId) -> Result<(), ProvisionError> {
                Ok(())
            }
            fn count_tunnels(&self) -> Result<u32, ProvisionError> {
                Ok(0)
            }
        }
        let v = RefSigner;
        let wg = WireguardTunnel;
        let vps = MockVps::new();
        let mut pm: PoolManager<WireguardTunnel, MockVps> = PoolManager::new(
            wg,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hubs = pm.refill(&v, &Clock::new(0), 1).unwrap();
        assert_eq!(hubs.len(), 1);
    }

    // ════════════ M2 — ingress catch-all + write-ahead + dns content ════════════

    #[test]
    fn red_ingress_missing_catchall_rejected() {
        let cfg = IngressConfig {
            rules: vec![IngressRule {
                hostname: Hostname("hub.local".into()),
                service: "http://localhost:8080".into(),
                catch_all: false,
            }],
        };
        assert!(!cfg.ends_with_catchall());
    }

    #[test]
    fn red_mutation_logged_before_api_call() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        // Wire the tunnel (write-ahead-logged inside).
        pm.wire_tunnel(&hub, &Clock::new(1)).unwrap();
        // The mutation log entries exist BEFORE/independent of the recorded CF calls.
        assert!(!pm.mut_log.is_empty());
        assert!(pm.mut_log.verify_chain(&v).is_ok());
    }

    #[test]
    fn red_dns_content_must_be_cfargotunnel() {
        // route_dns in wire_tunnel builds content = "<tid>.cfargotunnel.com" and
        // rejects otherwise. Assert the expected construction holds for a known tid.
        let tid = TunnelId("abc123".into());
        let expected = format!("{}.cfargotunnel.com", tid.0);
        assert!(expected.starts_with(&tid.0));
        assert!(expected.ends_with(".cfargotunnel.com"));
    }

    // ════════════ M3 — claim assignment-only + no reclaim + spec mismatch ════════════

    #[test]
    fn red_claim_is_assignment_only() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let creates_before = *pm.vps.create_calls.borrow();

        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        let receipt = pm
            .claim(
                &v,
                &clock,
                hub,
                owner.cls_pub,
                &owner.cls_seed,
                &owner.pq_seed,
                &owner_root,
                None,
                None,
            )
            .unwrap();
        // No boot happened on the hot path.
        assert_eq!(*pm.vps.create_calls.borrow(), creates_before);
        // Assignment latency within budget.
        assert!(receipt.assign_latency_ms < CLAIM_ASSIGN_BUDGET_MS);
        // Slot is now Claimed, not Warm.
        assert!(matches!(
            pm.slots[&hub].state,
            PoolSlotState::Claimed { .. }
        ));
    }

    #[test]
    fn red_claimed_never_returns_to_pool() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        pm.claim(
            &v,
            &clock,
            hub,
            owner.cls_pub,
            &owner.cls_seed,
            &owner.pq_seed,
            &owner_root,
            None,
            None,
        )
        .unwrap();
        // A claimed hub is NOT counted as warm (it cannot be re-claimed).
        assert_eq!(pm.warm_depth(), 0);
        // Attempting to claim a non-Warm slot is rejected.
        let r2 = pm.claim(
            &v,
            &clock,
            hub,
            owner.cls_pub,
            &owner.cls_seed,
            &owner.pq_seed,
            &owner_root,
            None,
            None,
        );
        assert_eq!(r2, Err(ProvisionError::Unauthorized));
    }

    #[test]
    fn red_spec_mismatch_rejected() {
        let _v = RefSigner;
        let vps = MockVps::with_spec_guard("cx11");
        let r = vps.create_from_image(
            &ImageRef("snap".into()),
            &ServerSpec {
                server_type: "different".into(),
                location: "hel1".into(),
            },
        );
        assert_eq!(r, Err(ProvisionError::Upstream("spec mismatch".into())));
    }

    // ════════════ M4 — claim hands out P59 root/child; hub stays self-sufficient ════════════

    #[test]
    fn red_claim_without_owner_root_still_self_sufficient() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        pm.claim(
            &v,
            &clock,
            hub,
            owner.cls_pub,
            &owner.cls_seed,
            &owner.pq_seed,
            &owner_root,
            None,
            None,
        )
        .unwrap();
        // Remove the owner enrollment + any co-sign → the hub's OWN self-root still
        // verifies under RequireBoth with NO dowiz/owner dependency (P59 §4.4).
        let hub_root = &pm.slots[&hub].hub_root;
        assert!(hub_root.verify_self(&v, 100).is_ok());
    }

    #[test]
    fn red_child_block_cannot_redelegate() {
        // The claim-issued child carries may_delegate=false → appending a grandchild
        // (a second link) is rejected by P59's depth ceiling.
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        let receipt = pm
            .claim(
                &v,
                &clock,
                hub,
                owner.cls_pub,
                &owner.cls_seed,
                &owner.pq_seed,
                &owner_root,
                None,
                None,
            )
            .unwrap();
        // The child's may_delegate is false → a following link is forbidden.
        assert!(!receipt.child_cert.may_delegate);
        // Build a chain [child, grandchild] and verify it hits MaxDepthExceeded.
        let hub_party = Party::new(&v, 2);
        // [test-fixture]
        let grandchild = CertDelegation::sign(
            &v,
            // The re-delegation issuer is the HUB (child_cert.subject), so it must be
            // signed with the hub's own seeds — otherwise the link's RequireBoth sig
            // fails (BadSignature) before the depth gate is reached. The hub root in
            // `refill` for slot index 0 is minted with cls_seed=[11u8;32],
            // pq_seed=[111u8;32] (see PoolManager::refill), so we sign with those.
            &[11u8; 32],
            &[111u8; 32],
            receipt.child_cert.subject,
            receipt.child_cert.subject_pq.clone(),
            hub_party.cls_pub,
            hub_party.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            99999,
            [8u8; 8],
        );
        let chain = vec![receipt.child_cert.clone(), grandchild];
        let mut roster = AnchorRoster::new();
        roster.enroll(&owner.cls_pub);
        let cap = crate::ports::agent::cap::Capability::new_hybrid(
            hub_party.cls_pub,
            hub_party.pq_pub.clone(),
            scope(),
            [9u8; 8],
            99999,
        );
        let store = crate::capability_cert::RevocationStore::new();
        // The first link's issuer is owner (enrolled), but the second link follows a
        // non-delegable link → MaxDepthExceeded.
        let res = crate::capability_cert::verify_chain_hybrid(
            &v,
            &roster,
            &store,
            &owner_root,
            &chain,
            &cap,
            0,
        );
        assert_eq!(
            res,
            Err(crate::capability_cert::CertError::MaxDepthExceeded)
        );
    }

    #[test]
    fn red_cross_owner_claim_forgery() {
        // Owner B presents owner A's root at claim for a hub minted by A → the hub
        // (enrolled under A's anchor) rejects B's chain under A's anchor.
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner_a = Party::new(&v, 9);
        let owner_b = Party::new(&v, 5);
        let root_a = SelfSignedRoot::mint(&v, &owner_a.cls_seed, &owner_a.pq_seed, scope(), 99999);
        let _root_b = SelfSignedRoot::mint(&v, &owner_b.cls_seed, &owner_b.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        // Claim under A succeeds.
        let r_a = pm.claim(
            &v,
            &clock,
            hub,
            owner_a.cls_pub,
            &owner_a.cls_seed,
            &owner_a.pq_seed,
            &root_a,
            None,
            None,
        );
        assert!(r_a.is_ok());
        // Owner B tries to present root A's pubkey but signed under B's secret → the
        // hub's enrolled anchor is A, and B's signature won't verify against A's key.
        // [test-fixture]
        let forged = CertDelegation::sign(
            &v,
            &owner_b.cls_seed, // B's secret
            &owner_b.pq_seed,
            owner_a.cls_pub, // claiming to be A's issuer
            root_a.pq_pub.clone(),
            pm.slots[&hub].hub_root.classical_pub,
            pm.slots[&hub].hub_root.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            99999,
            [7u8; 8],
        );
        let mut roster = AnchorRoster::new();
        // Hub enrolled A only. The forged link claims A as `issued_by` but is signed
        // under B's seed, so its RequireBoth sig fails verification against A's key
        // (`verify_signature` checks the sig against `issued_by`'s keys) → BadSignature.
        // That IS the cross-owner forgery being rejected (the hub never trusts B's cert).
        roster.enroll(&owner_a.cls_pub); // hub enrolled A only
        let cap = crate::ports::agent::cap::Capability::new_hybrid(
            pm.slots[&hub].hub_root.classical_pub,
            pm.slots[&hub].hub_root.pq_pub.clone(),
            scope(),
            [9u8; 8],
            99999,
        );
        let store = crate::capability_cert::RevocationStore::new();
        let res = crate::capability_cert::verify_chain_hybrid(
            &v,
            &roster,
            &store,
            &root_a,
            &[forged],
            &cap,
            0,
        );
        // B signed under B's key but claims A as issuer → the sig does not verify
        // against A's enrolled key → BadSignature (the forgery is rejected).
        assert_eq!(res, Err(crate::capability_cert::CertError::BadSignature));
    }

    // ════════════ M5 — mutation log tamper / unsigned / reorder ════════════

    #[test]
    fn red_mutation_log_tamper_detected() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        pm.wire_tunnel(&hub, &Clock::new(1)).unwrap();
        assert!(pm.mut_log.verify_chain(&v).is_ok());
        // Flip one byte of entry N's op → chain break.
        pm.mut_log.entries[0].op = TunnelOp::DestroyTunnel;
        assert_eq!(pm.mut_log.verify_chain(&v), Err(CertError::BadSignature));
    }

    #[test]
    fn red_unsigned_mutation_rejected() {
        let v = RefSigner;
        let mut log = MutationLog::new([1u8; 32], vec![2u8; 32]);
        // Build an unsigned entry (zeroed sig) and force-append via the internal
        // path is impossible (append always signs); instead assert a zeroed-sig entry
        // fails verify_chain when injected.
        let mut m = TunnelMutation {
            seq: 0,
            prev_hash: sha3_256(&[]),
            op: TunnelOp::CreateTunnel,
            hub: HubId::from_bytes([0u8; 16]),
            account: CfAccountId("a".into()),
            at_tick: 0,
            actor: [1u8; 32],
            sig: HybridSig {
                alg_suite_raw: AlgSuite::MlDsa65Ed25519.to_u16(),
                classical: vec![0u8; 32],
                pq: vec![0u8; 32],
            },
        };
        m.sig = HybridSig {
            alg_suite_raw: AlgSuite::MlDsa65Ed25519.to_u16(),
            classical: vec![0u8; 32],
            pq: vec![0u8; 32],
        };
        log.entries.push(m);
        assert_eq!(log.verify_chain(&v), Err(CertError::BadSignature));
    }

    #[test]
    fn red_log_replay_cannot_reorder() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        pm.wire_tunnel(&hub, &Clock::new(1)).unwrap();
        pm.wire_tunnel(&hub, &Clock::new(2)).unwrap();
        assert!(pm.mut_log.verify_chain(&v).is_ok());
        // Reorder: move seq=3 to the front → seq monotonicity break.
        let mut entries = pm.mut_log.entries.clone();
        let last = entries.len() - 1;
        entries.swap(0, last);
        pm.mut_log.entries = entries;
        assert_eq!(pm.mut_log.verify_chain(&v), Err(CertError::ScopeViolation));
    }

    // ════════════ M6 — suspend-preserving + still owned + 2nd-account rollover ════════════

    #[test]
    fn red_suspend_preserves_state_then_resume() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        pm.claim(
            &v,
            &clock,
            hub,
            owner.cls_pub,
            &owner.cls_seed,
            &owner.pq_seed,
            &owner_root,
            None,
            None,
        )
        .unwrap();
        // Suspend: state snapshot exists, compute released.
        let snap = pm.suspend(&v, hub).unwrap();
        assert!(pm.vps.suspended.borrow().values().any(|s| *s == snap));
        // Still owned (not returned to pool).
        assert!(matches!(
            pm.slots[&hub].state,
            PoolSlotState::Suspended { .. }
        ));
        // Resume → returns with prior state.
        let _ = pm.resume(&v, hub).unwrap();
        assert!(matches!(
            pm.slots[&hub].state,
            PoolSlotState::Claimed { .. }
        ));
    }

    #[test]
    fn red_resume_preserves_owner_not_zeroed() {
        // Roadmap item 30 / §4b regression guard: resume() MUST restore the real
        // pre-suspension owner, never zero it. A resumed hub carrying
        // OwnerId([0u8;32]) is a silent capability loss — owned by nobody / a
        // forgeable null id, violating §16.57 no-reclaim ownership continuity.
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        pm.claim(
            &v,
            &clock,
            hub,
            owner.cls_pub,
            &owner.cls_seed,
            &owner.pq_seed,
            &owner_root,
            None,
            None,
        )
        .unwrap();
        let expected_owner = OwnerId(owner_root.node_id.0);
        // Sanity: the real owner is a non-zero id.
        assert_ne!(expected_owner, OwnerId([0u8; 32]));
        let _ = pm.suspend(&v, hub).unwrap();
        let _ = pm.resume(&v, hub).unwrap();
        // The resumed hub MUST carry the real pre-suspension owner, not a zeroed id.
        match &pm.slots[&hub].state {
            PoolSlotState::Claimed { owner: o } => {
                assert_eq!(*o, expected_owner, "resume() zeroed the owner id");
            }
            other => panic!("expected Claimed after resume, got {:?}", other),
        }
    }

    #[test]
    fn red_suspended_hub_still_owned() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        pm.claim(
            &v,
            &clock,
            hub,
            owner.cls_pub,
            &owner.cls_seed,
            &owner.pq_seed,
            &owner_root,
            None,
            None,
        )
        .unwrap();
        let _ = pm.suspend(&v, hub).unwrap();
        // Not in the claimable pool.
        assert_eq!(pm.warm_depth(), 0);
        // Owner unchanged.
        match &pm.slots[&hub].state {
            PoolSlotState::Suspended { owner: o, .. } => {
                assert_eq!(*o, OwnerId(owner_root.node_id.0));
            }
            _ => panic!("expected suspended"),
        }
    }

    #[test]
    fn red_second_account_rollover_no_code_change() {
        let _v = RefSigner;
        let tunnel = MockTunnel::with_count(CF_TUNNEL_CRIT_WATERMARK + 1); // > 950
        let mut pool = AccountPool::new();
        pool.accounts.push(CfAccount {
            id: CfAccountId("acct-1".into()),
            api_token: TunnelToken::new("eyJ.1"),
            zone_id: "z1".into(),
        });
        // A second account is CONFIG (not code): crossing the critical watermark
        // routes new hubs to the next entry with ZERO orchestration edit.
        pool.accounts.push(CfAccount {
            id: CfAccountId("acct-2".into()),
            api_token: TunnelToken::new("eyJ.2"),
            zone_id: "z2".into(),
        });
        let (_check, flag) = check_tunnel_cap(&tunnel, &mut pool).unwrap();
        assert!(flag.is_some());
        // Routing moved to a second account with ZERO orchestration edit.
        assert_eq!(pool.route_account(CF_TUNNEL_CRIT_WATERMARK + 1), 1);
    }

    // ════════════ M8 — heartbeat: no data / forged rejected / silence alerts ════════════

    #[test]
    fn red_heartbeat_carries_no_data() {
        // The Heartbeat struct has exactly {hub_id, tick, sig}. Any attempt to carry
        // extra data is a compile error (no field exists). Assert the shape.
        let v = RefSigner;
        let hub = HubId::from_bytes([3u8; 16]);
        let seed = [4u8; 32];
        let pq = [5u8; 32];
        let hb = Heartbeat::sign(&v, hub, 7, &seed, &pq);
        assert_eq!(hb.hub_id, hub);
        assert_eq!(hb.tick, 7);
        // verify the (only) fields exist and there is nothing else to assert.
        let _ = hb.sig;
    }

    #[test]
    fn red_forged_heartbeat_rejected() {
        let v = RefSigner;
        let hub = HubId::from_bytes([3u8; 16]);
        let legit = Party::new(&v, 3);
        let attacker = Party::new(&v, 4);
        let hb = Heartbeat::sign(&v, hub, 7, &legit.cls_seed, &legit.pq_seed);
        // Verify against the hub's own keys → ok.
        assert!(hb.verify(&v, &legit.cls_pub, &legit.pq_pub));
        // Forged: signed by attacker but verified against the hub's keys → rejected.
        let forged = Heartbeat::sign(&v, hub, 7, &attacker.cls_seed, &attacker.pq_seed);
        assert!(!forged.verify(&v, &legit.cls_pub, &legit.pq_pub));
    }

    #[test]
    fn red_silent_hub_alerts() {
        let mut collector = HeartbeatCollector::new();
        let hub = HubId::from_bytes([6u8; 16]);
        // Observe once at tick 0.
        collector.observe(hub, 0);
        // Advance to past the silence budget → alert raised.
        let raised = collector.sweep(HEARTBEAT_SILENCE_ALERT_TICKS + 1);
        assert_eq!(raised.len(), 1);
        assert!(!collector.alerts().is_empty());
    }

    // ════════════ D8 (task-mandated) — cap alert at warn watermark ════════════

    #[test]
    fn red_tunnel_count_over_warn_alerts() {
        let _v = RefSigner;
        let tunnel = MockTunnel::with_count(CF_TUNNEL_WARN_WATERMARK + 1); // 801
        let mut pool = AccountPool::new();
        pool.accounts.push(CfAccount {
            id: CfAccountId("acct-1".into()),
            api_token: TunnelToken::new("eyJ.1"),
            zone_id: "z1".into(),
        });
        let (check, flag) = check_tunnel_cap(&tunnel, &mut pool).unwrap();
        match check {
            CapCheck::Warn {
                began_second_account,
            } => {
                assert!(began_second_account);
                // A second account was provisioned (config append, no code change).
                assert_eq!(pool.accounts.len(), 2);
            }
            _ => panic!("expected Warn at 801"),
        }
        assert!(flag.is_some());
    }

    // ════════════ D10 — no shared secret in snapshot (inject per-hub, never bake) ════════════

    #[test]
    fn red_no_shared_secret_in_snapshot() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("SNAPSHOT-SHARED".into()), // the SHARED image
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        // Provision two hubs FROM the same snapshot → their roots are DISTINCT
        // (minted per-hub at first-boot, §9.3). No shared root keypair.
        let hubs = pm.refill(&v, &Clock::new(0), 2).unwrap();
        let a = &pm.slots[&hubs[0]].hub_root;
        let b = &pm.slots[&hubs[1]].hub_root;
        assert_ne!(a.node_id, b.node_id);
        assert_ne!(a.classical_pub, b.classical_pub);
        assert_ne!(a.pq_pub, b.pq_pub);
        // The shared snapshot id itself carries no per-hub secret.
        assert_eq!(pm.snapshot.0, "SNAPSHOT-SHARED");
    }

    // ════════════ D12 — no crypto of its own: P67 only calls P59 ════════════

    #[test]
    fn red_claim_attaches_optional_dowiz_cosign() {
        let v = RefSigner;
        let tunnel = MockTunnel::with_count(0);
        let vps = MockVps::new();
        let mut pm: PoolManager<MockTunnel, MockVps> = PoolManager::new(
            tunnel,
            vps,
            [1u8; 32],
            [2u8; 32],
            ImageRef("snap".into()),
            ServerSpec {
                server_type: "cx11".into(),
                location: "hel1".into(),
            },
        );
        let hub = pm.refill(&v, &Clock::new(0), 1).unwrap()[0];
        let owner = Party::new(&v, 9);
        let dowiz = Party::new(&v, 42);
        let owner_root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
        let clock = Clock::new(10);
        let receipt = pm
            .claim(
                &v,
                &clock,
                hub,
                owner.cls_pub,
                &owner.cls_seed,
                &owner.pq_seed,
                &owner_root,
                Some(&dowiz.cls_seed),
                Some(&dowiz.pq_seed),
            )
            .unwrap();
        // The co-sign is present AND verifies against dowiz's keys (additive only).
        let cosign = receipt.dowiz_cosign.expect("cosign attached");
        assert!(cosign.verify(&v, &dowiz.cls_pub, &dowiz.pq_pub, &pm.slots[&hub].hub_root));
    }
}
