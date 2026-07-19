//! `landing/claim_client.rs` — M3 (BLUEPRINT-P73 §3 / §4.3).
//!
//! The claim-service CLIENT LEG. P73 is a **consumer** of P67's claim service (BLUEPRINT-P67,
//! greenfield this pass; contract source = `OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` §6.1
//! steps 1-5). P73 POSTs one of two requests and renders the outcome. P73 implements **NO claim
//! logic, mints NO certs, touches NO warm pool, calls NO Cloudflare/Hetzner API** — all of that is
//! P67. **HARD `RECONCILE-P67`:** these shapes are provisional until P67 lands its wire format; on
//! divergence, P73 adopts P67's. The `MockClaimService` is the contract stand-in so the whole
//! funnel is testable with ZERO network and BEFORE P67 lands.
//!
//! The client-side rate limit (X11) reuses `kernel/src/token_bucket.rs` (`CLAIM_BUCKET_CAPACITY`).

use crate::token_bucket::TokenBucket;
use super::{CLAIM_BUCKET_CAPACITY, CLAIM_BUCKET_REFILL};

/// An edge-verified anti-abuse token (X11 / R2 §7). Produced at the Cloudflare edge/challenge
/// layer, NOT embedded in the wgpu canvas (same DOM tension as X3). Opaque to P73; the claim
/// service RE-verifies it (defense in depth) before touching a pool slot.
#[derive(Clone, PartialEq, Debug)]
pub struct ChallengeToken(pub String);

/// The claim-service API — **OWNED by P67**. P73 is a CONSUMER (see module header).
pub trait ClaimServicePort {
    /// FAST PATH (R3 §6.1 steps 1-3: assignment-only, zero infra on the hot path). Returns a
    /// claimed, already-ONLINE, fixture-populated hub, OR `PoolEmpty` (NOT an error — the journey
    /// falls to `register_interest`).
    fn claim_warm_pool_hub(&self, req: ClaimRequest) -> Result<ClaimOutcome, ClaimError>;
    /// SLOW PATH (R3 §6.1 step 5 / §16.32 non-mandatory path): notify the operator for MANUAL
    /// follow-up. No automation, no infra — an ack id only.
    fn register_interest(&self, sub: InterestSubmission) -> Result<InterestAck, ClaimError>;
}

/// FAST PATH request body.
#[derive(Clone, PartialEq, Debug)]
pub struct ClaimRequest {
    pub contact: String,   // SignupForm.contact (resolved from P57 TextField::value())
    pub venue_name: String, // SignupForm.venue_name
    pub challenge: ChallengeToken, // edge-verified; the claim service re-verifies (§5.1)
}

/// SLOW PATH request body.
#[derive(Clone, PartialEq, Debug)]
pub struct InterestSubmission {
    pub contact: String,
    pub venue_name: String,
    pub notes: String,
    pub challenge: ChallengeToken,
}

/// Outcome of a warm-pool claim attempt. `PoolEmpty` ⇒ route to interest (§4.3) — NOT an error.
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimOutcome {
    Claimed(ClaimedHub),
    PoolEmpty,
}

/// What P67's service hands back on a successful claim (R3 §6.1 step 3 / §2.4).
#[derive(Clone, PartialEq, Debug)]
pub struct ClaimedHub {
    pub hub_id: HubId,       // opaque id
    pub hub_url: String,     // `hub-<id>.hubs.dowiz.org` (R3 §1.2) — where the vendor lands
    /// The owner root capability-cert — FORMAT is **P59's** (biscuit-style hybrid-signed chain),
    /// CUSTODY is **P66's** wallet / **P70's** owner surface. P73 receives it OPAQUE, forwards it
    /// ONCE (journey M6), never parses/stores/mints it (§5.1 — the unforgeable-cert argument).
    pub owner_root_cert: OwnerRootCert,
    pub fixtures_ready: bool, // §16.54 — hub is online & populated the instant it's claimed
}

/// Opaque hub id.
#[derive(Clone, PartialEq, Debug)]
pub struct HubId(pub String);

/// The operator's acknowledgement of an interest registration (slow path).
#[derive(Clone, PartialEq, Debug)]
pub struct InterestAck {
    pub ack_id: String,
}

/// The owner root capability-cert — FORMAT is **P59's**, CUSTODY is **P66/P70's**. P73 holds it
/// as OPAQUE bytes and forwards it once. Never parsed/minted/long-term-stored here.
#[derive(Clone, PartialEq, Debug)]
pub struct OwnerRootCert(pub Vec<u8>);

/// Transport/challenge failure taxonomy. `Timeout`/`Transport` are DEgrade-closed → the journey
/// offers the interest path (§4.3). `Failed` is for transport/challenge ONLY, never for `PoolEmpty`.
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimError {
    Timeout,
    ChallengeRejected,
    Transport(String),
    RateLimited,
}

/// A claim client that (a) enforces the single-outstanding-claim rate limit via `TokenBucket` and
/// (b) delegates the actual transport to a `ClaimServicePort`. The real transport adapter swaps the
/// `MockClaimService` for an HTTP client once P67 lands (RECONCILE-P67). The bucket makes a
/// double-submit consume at most one pool slot (§16.57) — the second attempt is rate-limited.
pub struct ClaimClient<T: ClaimServicePort> {
    inner: T,
    bucket: TokenBucket,
}

impl<T: ClaimServicePort> ClaimClient<T> {
    /// Wrap a `ClaimServicePort` with the client-side claim limiter (X11).
    pub fn new(inner: T) -> Self {
        ClaimClient {
            inner,
            bucket: TokenBucket::new(CLAIM_BUCKET_CAPACITY, CLAIM_BUCKET_REFILL),
        }
    }

    /// FAST PATH with the single-outstanding-claim guard. A second call while one is "in flight"
    /// (the bucket has <1 token) is rate-limited WITHOUT touching the pool (§5.1).
    pub fn claim(&self, req: ClaimRequest) -> Result<ClaimOutcome, ClaimError> {
        if !self.bucket.try_acquire(CLAIM_BUCKET_CAPACITY) {
            return Err(ClaimError::RateLimited);
        }
        self.inner.claim_warm_pool_hub(req)
    }

    /// SLOW PATH (no pool slot consumed — interest is just an operator notify).
    pub fn register(&self, sub: InterestSubmission) -> Result<InterestAck, ClaimError> {
        self.inner.register_interest(sub)
    }
}

/// Wave-0 mock adapter — returns scripted outcomes so the funnel is testable with ZERO network and
/// BEFORE P67 lands (§2.3 Lane A). Behave identically to a real service from the journey's view.
#[derive(Clone, Default, Debug)]
pub struct MockClaimService {
    mode: MockMode,
}

#[derive(Clone, Default, Debug)]
enum MockMode {
    #[default]
    Claimed,
    PoolEmpty,
    Timeout,
    EmptyCert,
}

impl MockClaimService {
    pub fn claimed() -> Self {
        MockClaimService { mode: MockMode::Claimed }
    }
    pub fn pool_empty() -> Self {
        MockClaimService { mode: MockMode::PoolEmpty }
    }
    pub fn timeout() -> Self {
        MockClaimService { mode: MockMode::Timeout }
    }
    pub fn empty_cert() -> Self {
        MockClaimService {
            mode: MockMode::EmptyCert,
        }
    }
}

impl ClaimServicePort for MockClaimService {
    fn claim_warm_pool_hub(&self, _req: ClaimRequest) -> Result<ClaimOutcome, ClaimError> {
        match &self.mode {
            MockMode::Claimed => Ok(ClaimOutcome::Claimed(ClaimedHub {
                hub_id: HubId("hub_demo001".into()),
                hub_url: "https://hub-demo001.hubs.dowiz.org".into(),
                owner_root_cert: OwnerRootCert(vec![0xDE, 0xAD, 0xBE, 0xEF]),
                fixtures_ready: true,
            })),
            MockMode::PoolEmpty => Ok(ClaimOutcome::PoolEmpty),
            MockMode::Timeout => Err(ClaimError::Timeout),
            MockMode::EmptyCert => Ok(ClaimOutcome::Claimed(ClaimedHub {
                hub_id: HubId("hub_emptycert".into()),
                hub_url: "https://hub-emptycert.hubs.dowiz.org".into(),
                owner_root_cert: OwnerRootCert(Vec::new()), // malformed/useless hub
                fixtures_ready: true,
            })),
        }
    }
    fn register_interest(&self, _sub: InterestSubmission) -> Result<InterestAck, ClaimError> {
        Ok(InterestAck {
            ack_id: "ack_42".into(),
        })
    }
}
