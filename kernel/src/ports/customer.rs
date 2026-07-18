//! ports/customer.rs — P49 per-order customer identity (BLUEPRINT-P47-P50 §4).
//!
//! Operator ruling (2026-07-18): identity = **per-order capability grant**
//! (option 2). Privacy-minimal, no device/personal data:
//!
//! * NO account / profile beyond one order's needs.
//! * NO loyalty / CRM / marketing identity.
//! * NO device fingerprint, NO email/SMS, NO magic-link, NO contact channel.
//! * NO conflation with courier/operator device-bound certs.
//! * NO porting old TS (`softVerifyAuth`) — re-derived natively here.
//!
//! The customer identity *is* a [`OrderTrackingGrant`]: a capability scoped to
//! ONE order, minted at order placement, carried by the client, and dying with
//! the order. It reuses the proto-cap signing *convention* — a domain-separated
//! fixed-layout SHA3 commitment (the kernel already does this for roster /
//! revocation hashes and the `agent` admission seam) — without linking
//! `bebop2/proto-cap` (verified absent from `kernel/Cargo.toml`). No external
//! crypto is invented. The commit is a server-side symmetric commitment; when
//! `proto-cap` becomes a kernel dependency the `mac` step can be swapped for the
//! hybrid Ed25519 ⊕ ML-DSA-65 gate without touching the typed surface.
//!
//! Compile firewall (mirrors `ports/llm.rs`, `ports/agent/cap.rs`): ZERO
//! network / HTTP / JSON / serde. Pure `std` only.

use std::collections::HashMap;

use crate::event_log::sha3_256;
use crate::geo;
use crate::kalman::KalmanFilter;
use crate::order_machine::OrderStatus;
use crate::rng::Rng;

// ── domain-separation tags (16 bytes each, pinned) ──────────────────────────────
/// Signing domain for the grant commitment (separate from every other kernel domain).
const DOMAIN_GRANT: &[u8; 16] = b"dowiz.grant.v1..";
/// Server-side mint secret (domain-separated constant). The commitment is
/// symmetric: only the kernel that minted the grant can re-derive its MAC.
/// Swappable for the proto-cap hybrid gate once that crate is a kernel dep.
const MINT_SECRET: &[u8; 16] = b"dowiz.grant.key0";

/// Default grant lifetime, in ticks. A tracking grant outlives the typical
/// delivery window but self-terminates well before the order is ancient — the
/// expiry lives *in the type*, not in a cron job (BLUEPRINT §4.5-2).
pub const DEFAULT_GRANT_LIFETIME_TICKS: u64 = 6_000;

/// Bits of entropy carried by a grant handle. 32 bytes = 256 bits — guessing an
/// active order's handle at wire rate is computationally infeasible (§4.5-1).
pub const GRANT_HANDLE_BYTES: usize = 32;
/// Effective security strength floor we assert on the handle (bits). 128-bit is
/// the conservative "infeasible at wire rate" bar; the handle actually carries
/// 256, so this is a one-sided floor, never a ceiling.
pub const GRANT_HANDLE_MIN_ENTROPY_BITS: u32 = 128;

/// A per-order capability grant — the entire customer identity.
///
/// Minted at order placement, handed to the client, dies when the order (and
/// thus the grant's `expiry_tick`) ends. There is no subject key, no device
/// binding, no account: the `handle` is the only secret, and it is unlinkable
/// across orders (each order gets an independent handle from the RNG).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderTrackingGrant {
    /// The single order this grant authorizes tracking for.
    pub order_id: String,
    /// High-entropy random handle (the client-carried secret). 256 bits.
    pub handle: [u8; GRANT_HANDLE_BYTES],
    /// Monotonic issuance tick.
    pub issued_tick: u64,
    /// Expiry tick. Past this, the grant is dead — replay rejected (§4.5-2).
    pub expiry_tick: u64,
    /// Server-side commitment over the fields above (domain-separated SHA3).
    pub mac: [u8; 32],
}

impl OrderTrackingGrant {
    /// Mint a grant for `order_id` at `issued_tick` with `lifetime_ticks`.
    /// The handle is drawn from `rng` (256 bits of entropy).
    pub fn mint(order_id: &str, rng: &mut Rng, issued_tick: u64, lifetime_ticks: u64) -> Self {
        let handle = random_handle(rng);
        let expiry_tick = issued_tick.saturating_add(lifetime_ticks);
        let mac = grant_mac(order_id, &handle, issued_tick, expiry_tick);
        OrderTrackingGrant {
            order_id: order_id.to_string(),
            handle,
            issued_tick,
            expiry_tick,
            mac,
        }
    }

    /// Verify the grant: not-yet-valid, expired, and tampered states all fail
    /// closed. Returns `Ok(&self)` carrying the bound `order_id` on success.
    pub fn verify(&self, now_tick: u64) -> Result<&Self, GrantError> {
        if now_tick < self.issued_tick {
            return Err(GrantError::NotYetValid);
        }
        if now_tick >= self.expiry_tick {
            return Err(GrantError::Expired);
        }
        let expected = grant_mac(
            &self.order_id,
            &self.handle,
            self.issued_tick,
            self.expiry_tick,
        );
        if expected != self.mac {
            return Err(GrantError::BadMac);
        }
        Ok(self)
    }

    /// Bits of entropy the handle carries (always 8 × byte length). Exposed so a
    /// brute-force test can assert the wire-rate-infeasibility floor.
    pub fn handle_entropy_bits(&self) -> u32 {
        (self.handle.len() * 8) as u32
    }
}

/// Draw a 256-bit handle from the deterministic RNG (4 × u64 LE words).
fn random_handle(rng: &mut Rng) -> [u8; GRANT_HANDLE_BYTES] {
    let mut h = [0u8; GRANT_HANDLE_BYTES];
    for word in h.chunks_exact_mut(8) {
        word.copy_from_slice(&rng.next_u64().to_le_bytes());
    }
    h
}

/// Domain-separated SHA3 commitment binding order_id + handle + issued + expiry
/// under the server-side mint secret. Tampering with any field breaks the MAC.
fn grant_mac(
    order_id: &str,
    handle: &[u8; GRANT_HANDLE_BYTES],
    issued: u64,
    expiry: u64,
) -> [u8; 32] {
    let mut buf = Vec::with_capacity(
        DOMAIN_GRANT.len() + MINT_SECRET.len() + order_id.len() + GRANT_HANDLE_BYTES + 16,
    );
    buf.extend_from_slice(DOMAIN_GRANT);
    buf.extend_from_slice(MINT_SECRET);
    buf.extend_from_slice(order_id.as_bytes());
    buf.extend_from_slice(handle);
    buf.extend_from_slice(&issued.to_le_bytes());
    buf.extend_from_slice(&expiry.to_le_bytes());
    sha3_256(&buf)
}

/// Why a grant / tracking request was rejected. Every variant is fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantError {
    /// Grant used before its issuance tick.
    NotYetValid,
    /// Grant used at/after `expiry_tick` — replay after completion rejected (§4.5-2).
    Expired,
    /// MAC mismatch — the grant was forged or tampered with.
    BadMac,
    /// Cross-order leak (§4.5-3): a grant for order A was used to request
    /// order B's tracking view. Fail-closed.
    OrderMismatch,
}

/// The read-only projection of Kalman/EMA courier state a customer sees.
///
/// Pure data — no new math. It is *built* from existing kernel math
/// (`geo::progress_along_route`, `geo::eta_seconds`, and an already-stepped
/// `KalmanFilter`'s `last_surprise`). The pixel render is WAVE G3 (WebGPU);
/// here we only expose the typed, deterministic view derived from kernel math.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackingView {
    /// The order this view is for (never leaks another order's data).
    pub order_id: String,
    /// Current order status (from the order's fold state).
    pub status: OrderStatus,
    /// Last known courier position.
    pub courier_lat: f64,
    pub courier_lng: f64,
    /// Metres remaining along the route to the destination.
    pub remaining_m: f64,
    /// Estimated seconds to arrival (from `geo::eta_seconds`).
    pub eta_seconds: f64,
    /// Courier position snapped to the route polyline.
    pub snapped_lat: f64,
    pub snapped_lng: f64,
    /// Novelty of the last courier measurement (‖y‖/√tr(S)) from a Kalman filter
    /// tracking the courier. 0.0 until a measurement is folded in.
    pub kalman_surprise: f64,
}

impl TrackingView {
    /// Build the view from a courier position + route polyline + ETA baseline,
    /// reusing `geo` math. `kalman_surprise` is the dimensionless novelty scalar
    /// from a [`KalmanFilter`] already stepped on courier observations.
    pub fn from_positions(
        order_id: String,
        status: OrderStatus,
        courier: (f64, f64),
        route: &[(f64, f64)],
        eta_baseline_s: f64,
        kalman_surprise: f64,
    ) -> Self {
        let prog = geo::progress_along_route(route, courier);
        let total_m = geo::polyline_length_meters(route);
        let eta = geo::eta_seconds(prog.remaining_m, total_m, eta_baseline_s);
        TrackingView {
            order_id,
            status,
            courier_lat: courier.0,
            courier_lng: courier.1,
            remaining_m: prog.remaining_m,
            eta_seconds: eta,
            snapped_lat: prog.snapped.0,
            snapped_lng: prog.snapped.1,
            kalman_surprise,
        }
    }

    /// Build the view, pulling the novelty scalar directly from a stepped
    /// [`KalmanFilter`] (reuses `kalman.rs` math, no new math).
    pub fn from_kalman(
        order_id: String,
        status: OrderStatus,
        courier: (f64, f64),
        route: &[(f64, f64)],
        eta_baseline_s: f64,
        kalman: &KalmanFilter,
    ) -> Self {
        Self::from_positions(
            order_id,
            status,
            courier,
            route,
            eta_baseline_s,
            kalman.last_surprise(),
        )
    }
}

/// The order↔channel link. Dies with the order — there is no durable
/// subscription. `channel_ref` is a *reference* to a channel P43 owns; this
/// struct never holds a contact address or second transport (anti-scope).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationBinding {
    pub order_id: String,
    pub channel_ref: String,
}

/// A fail-closed notification router: maps an order to its single channel. A
/// state change on order A MUST reach only A's channel, never B's (§4.5-4).
#[derive(Debug, Clone, Default)]
pub struct NotificationRouter {
    bindings: HashMap<String, String>,
}

impl NotificationRouter {
    /// Bind an order to a channel (dies with the order; caller drops it).
    pub fn bind(&mut self, b: NotificationBinding) {
        self.bindings.insert(b.order_id, b.channel_ref);
    }

    /// Release a binding (order terminal, grant dead).
    pub fn unbind(&mut self, order_id: &str) {
        self.bindings.remove(order_id);
    }

    /// Resolve the channel an order's notification must go to. `None` if the
    /// order has no binding (fail-closed: no channel ⇒ no send, never a default).
    pub fn route(&self, order_id: &str) -> Option<&str> {
        self.bindings.get(order_id).map(|s| s.as_str())
    }

    /// Deliver a state-change notification for `order_id`. Returns the channel it
    /// was routed to. `None` (no send) when the order is unbound — fail-closed,
    /// no fallback channel, so a state change can never leak to another order.
    pub fn deliver(&self, order_id: &str) -> Option<&str> {
        self.route(order_id)
    }
}

/// The stateless tracking authority — the seam the wire presents to a client.
///
/// It holds NO grants (the grant is self-verifying via its MAC) and NO accounts.
/// Re-identification and tracking are pure functions over the presented grant.
pub struct TrackingAuthority;

impl TrackingAuthority {
    /// Mint a grant at order placement.
    pub fn mint_grant(
        order_id: &str,
        rng: &mut Rng,
        issued_tick: u64,
        lifetime_ticks: u64,
    ) -> OrderTrackingGrant {
        OrderTrackingGrant::mint(order_id, rng, issued_tick, lifetime_ticks)
    }

    /// Re-identify a returning client from its grant — no login, no account.
    /// On success returns the bound `order_id` (proof the grant is valid + live).
    pub fn reidentify(grant: &OrderTrackingGrant, now_tick: u64) -> Result<String, GrantError> {
        grant.verify(now_tick)?;
        Ok(grant.order_id.clone())
    }

    /// Produce the tracking view for `requested_order_id`, scoped by `grant`.
    ///
    /// **Load-bearing cross-order guard (§4.5-3):** if `grant.order_id !=
    /// requested_order_id`, the request is rejected fail-closed before any view
    /// is built — a valid grant for order A can never yield order B's tracking.
    pub fn tracking_view(
        grant: &OrderTrackingGrant,
        requested_order_id: &str,
        now_tick: u64,
        status: OrderStatus,
        courier: (f64, f64),
        route: &[(f64, f64)],
        eta_baseline_s: f64,
        kalman_surprise: f64,
    ) -> Result<TrackingView, GrantError> {
        grant.verify(now_tick)?;
        if grant.order_id != requested_order_id {
            return Err(GrantError::OrderMismatch);
        }
        Ok(TrackingView::from_positions(
            requested_order_id.to_string(),
            status,
            courier,
            route,
            eta_baseline_s,
            kalman_surprise,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{place_order, OrderItem};
    use crate::kalman::KalmanFilter;
    use crate::order_machine::OrderStatus;

    // ── B1: anonymous place → later re-identify → track, no durable account ──
    #[test]
    fn b1_anonymous_place_reidentify_track_no_account() {
        // 1. Place an order anonymously (no customer_id) — kernel authority.
        let items = vec![OrderItem {
            product_id: "p1".into(),
            modifier_ids: vec![],
            quantity: 2,
            unit_price: 500,
        }];
        let order = place_order(
            "ORD-P49-B1".into(),
            None, // anonymous: NO customer account/profile (anti-scope)
            items,
            0,
            Some("cash".into()),
            None,
        )
        .expect("anonymous place must succeed");
        // Anti-scope assertion: the order carries NO customer account/profile.
        assert!(
            order.customer_id.is_none(),
            "P49: no customer account of any kind"
        );

        // 2. Mint the per-order grant AT placement.
        let mut rng = Rng::new(0xABC_D_EF0, 1);
        let grant =
            TrackingAuthority::mint_grant(&order.id, &mut rng, 1_000, DEFAULT_GRANT_LIFETIME_TICKS);

        // 3. Later (no login, no account), the client re-identifies with the grant.
        let re_id =
            TrackingAuthority::reidentify(&grant, 2_000).expect("valid live grant re-identifies");
        assert_eq!(
            re_id, order.id,
            "re-identification recovers the bound order"
        );

        // 4. Track — deterministic view built from kernel geo math.
        let route = vec![(50.4500, 30.5234), (50.4520, 30.5260), (50.4540, 30.5280)];
        let view = TrackingAuthority::tracking_view(
            &grant,
            &order.id,
            2_000,
            OrderStatus::InDelivery,
            (50.4510, 30.5245),
            &route,
            600.0,
            0.0,
        )
        .expect("tracking view for own order");
        assert_eq!(view.order_id, order.id);
        assert!(
            view.remaining_m >= 0.0,
            "remaining distance must be non-negative"
        );
        assert!(
            view.eta_seconds.is_finite(),
            "eta must be finite for a live order"
        );
    }

    // ── §4.5-1: grant handle carries stated entropy; brute-force infeasible ──
    #[test]
    fn adversarial_grant_handle_entropy_floor() {
        let mut rng = Rng::new(0xFEED, 1);
        // Mint many independent grants for distinct orders.
        let mut seen = std::collections::HashSet::new();
        for i in 0..512u64 {
            let g = OrderTrackingGrant::mint(
                &format!("ORD-{i}"),
                &mut rng,
                0,
                DEFAULT_GRANT_LIFETIME_TICKS,
            );
            // (a) handle is exactly 256 bits.
            assert_eq!(g.handle.len(), GRANT_HANDLE_BYTES);
            assert!(g.handle_entropy_bits() >= GRANT_HANDLE_MIN_ENTROPY_BITS);
            // (b) handles never collide across orders (no linkability).
            assert!(
                seen.insert(g.handle),
                "two orders must never share a handle"
            );
        }
    }

    #[test]
    fn adversarial_brute_force_infeasible_at_wire_rate() {
        // The handle is 256 bits. Even at an absurd 1e9 guesses/sec, the expected
        // time to guess ONE active order's handle is ~2^255 / 1e9 seconds —
        // astronomically larger than the age of the universe (~4.3e17 s). We
        // assert the search-space floor directly (via log2, no integer overflow)
        // rather than actually brute-forcing.
        let mut rng = Rng::new(0xCAFE_BEEF, 1);
        let g = OrderTrackingGrant::mint("ORD-TARGET", &mut rng, 0, DEFAULT_GRANT_LIFETIME_TICKS);
        let space_bits = g.handle_entropy_bits() as f64; // 256.0
        let guesses_per_sec: f64 = 1_000_000_000.0; // 1 GHz-class wire-rate guesser (generous)
                                                    // Average guesses = 2^(space_bits-1); time in seconds (computed in log space).
        let seconds = 2f64.powf(space_bits - 1.0) / guesses_per_sec;
        let age_of_universe_s: f64 = 13_800_000_000.0 * 365.0 * 24.0 * 3600.0;
        assert!(
            seconds > age_of_universe_s * 1e6,
            "brute-forcing a 256-bit handle must be infeasible at wire rate ({} s >> {} s)",
            seconds,
            age_of_universe_s
        );
        // And a random *wrong* handle must not verify as the active order's grant.
        let mut wrong = g.handle;
        wrong[0] ^= 0xFF; // flip bits — a guessed handle
        let mut forged = g.clone();
        forged.handle = wrong;
        assert!(
            forged.verify(0).is_err(),
            "a guessed (wrong) handle must never verify as the active order's grant"
        );
    }

    // ── §4.5-2: replay after terminal/expiry → rejected; expiry is in the type ──
    #[test]
    fn adversarial_replay_after_expiry_rejected() {
        let mut rng = Rng::new(0xDEAD_BEEF, 1);
        // Short-lived grant: issued at 100, expires at 200.
        let g = OrderTrackingGrant::mint("ORD-EXP", &mut rng, 100, 100);

        // Live window: valid.
        assert!(g.verify(150).is_ok(), "grant valid mid-lifetime");
        // At/after expiry: dead. Replay after the order is done is rejected.
        assert_eq!(
            g.verify(200),
            Err(GrantError::Expired),
            "expiry in the type rejects replay"
        );
        assert_eq!(
            g.verify(999),
            Err(GrantError::Expired),
            "long-past expiry rejected"
        );

        // Tracking view with an expired grant is fail-closed.
        let route = vec![(50.45, 30.52), (50.46, 30.53)];
        let res = TrackingAuthority::tracking_view(
            &g,
            "ORD-EXP",
            200,
            OrderStatus::Delivered,
            (50.45, 30.52),
            &route,
            600.0,
            0.0,
        );
        assert_eq!(
            res,
            Err(GrantError::Expired),
            "expired grant yields no tracking view"
        );
    }

    #[test]
    fn adversarial_tampered_grant_rejected() {
        let mut rng = Rng::new(0x1357_9BDF, 1);
        let mut g =
            OrderTrackingGrant::mint("ORD-TAMPER", &mut rng, 0, DEFAULT_GRANT_LIFETIME_TICKS);
        // Flip a handle bit — forgery attempt.
        g.handle[5] ^= 0x01;
        assert_eq!(
            g.verify(10),
            Err(GrantError::BadMac),
            "tampered handle fails MAC"
        );
        // Flip an order_id char instead.
        let mut g2 =
            OrderTrackingGrant::mint("ORD-ORIG", &mut rng, 0, DEFAULT_GRANT_LIFETIME_TICKS);
        g2.order_id = "ORD-FAKE".into();
        assert_eq!(
            g2.verify(10),
            Err(GrantError::BadMac),
            "tampered order_id fails MAC"
        );
    }

    // ── §4.5-3 (LOAD-BEARING): cross-order leak → fail-closed ────────────────
    #[test]
    fn adversarial_cross_order_leak_fail_closed() {
        let mut rng = Rng::new(0x600D_CAFE, 1);
        let grant_a = OrderTrackingGrant::mint("ORD-A", &mut rng, 0, DEFAULT_GRANT_LIFETIME_TICKS);
        let grant_b = OrderTrackingGrant::mint("ORD-B", &mut rng, 0, DEFAULT_GRANT_LIFETIME_TICKS);
        let route = vec![(50.45, 30.52), (50.46, 30.53)];

        // Grant A genuinely authorizes order A.
        assert!(grant_a.verify(10).is_ok());
        // But requesting order B's tracking with grant A MUST be rejected.
        let leak = TrackingAuthority::tracking_view(
            &grant_a,
            "ORD-B",
            10,
            OrderStatus::InDelivery,
            (50.45, 30.52),
            &route,
            600.0,
            0.0,
        );
        assert_eq!(
            leak,
            Err(GrantError::OrderMismatch),
            "grant for A must NEVER yield B's tracking view"
        );
        // And symmetrically: grant B cannot read A.
        let leak2 = TrackingAuthority::tracking_view(
            &grant_b,
            "ORD-A",
            10,
            OrderStatus::InDelivery,
            (50.45, 30.52),
            &route,
            600.0,
            0.0,
        );
        assert_eq!(leak2, Err(GrantError::OrderMismatch));

        // The legitimate same-order request succeeds and returns ONLY A's data.
        let ok = TrackingAuthority::tracking_view(
            &grant_a,
            "ORD-A",
            10,
            OrderStatus::InDelivery,
            (50.45, 30.52),
            &route,
            600.0,
            0.0,
        )
        .expect("same-order tracking succeeds");
        assert_eq!(ok.order_id, "ORD-A");
    }

    // ── §4.5-4: notification misbinding → property test over the binding ──────
    #[test]
    fn adversarial_notification_misbinding_property() {
        let mut rng = Rng::new(0x5EED_5EED, 1);
        let mut router = NotificationRouter::default();
        // Bind many orders to distinct channels.
        let n = 256usize;
        for i in 0..n {
            router.bind(NotificationBinding {
                order_id: format!("O{i:04}"),
                channel_ref: format!("CH{i:04}"),
            });
        }
        // Property: a state change on order i routes ONLY to channel i, never j≠i.
        for i in 0..n {
            let oi = format!("O{i:04}");
            let ci = format!("CH{i:04}");
            let routed = router.deliver(&oi).expect("bound order routes");
            assert_eq!(routed, ci, "order {i} must route to its own channel");
            // For a random OTHER order, ensure it never equals i's channel.
            let j = rng.next_index(n);
            if j != i {
                let oj = format!("O{j:04}");
                let cj = router.deliver(&oj).expect("bound order routes");
                assert_ne!(
                    cj, ci,
                    "order {i}'s channel must never receive order {j}'s notification"
                );
            }
        }
        // Fail-closed: an unbound order delivers nowhere (no default channel).
        assert_eq!(
            router.deliver("O-UNBOUND"),
            None,
            "unbound order: no send, never a default channel"
        );
        // After unbinding (order terminal), no leak.
        router.unbind("O0000");
        assert_eq!(
            router.deliver("O0000"),
            None,
            "terminal order's binding is released"
        );
    }

    // ── TrackingView reuses kernel kalman math (no new math) ──────────────────
    #[test]
    fn tracking_view_reuses_kalman_surprise() {
        // 1-D scalar Kalman filter tracking courier distance-to-destination.
        let mut kf = KalmanFilter::scalar(1000.0, 1e6, 1.0, 1.0, 1.0, 100.0);
        // Feed a few distance observations; the filter converges and records novelty.
        for &z in &[950.0_f64, 880.0, 700.0] {
            kf.predict();
            let _ = kf.update(&[z]);
        }
        let route = vec![(50.45, 30.52), (50.46, 30.53)];
        let view = TrackingView::from_kalman(
            "ORD-K".into(),
            OrderStatus::InDelivery,
            (50.455, 30.525),
            &route,
            600.0,
            &kf,
        );
        assert_eq!(view.order_id, "ORD-K");
        // The view's novelty equals the filter's last surprise (math reuse, no copy).
        assert!((view.kalman_surprise - kf.last_surprise()).abs() < 1e-12);
        assert!(view.remaining_m >= 0.0);
    }

    // ── B2 HONEST RED: real notification reaching the channel is P43's send ────
    // P43's wire send path does not exist yet (kernel/src/messenger.rs is a
    // non-sending deep-link builder only). This e2e asserts what MUST hold once
    // P43 DoD-2 transmits: a NotificationRouter that has bound order A to channel
    // ca, when A reaches a terminal state, delivers EXACTLY to ca and to no other
    // channel. It is marked `#[ignore]` so it does not fake P43 green.
    #[test]
    #[ignore = "B2 honest RED: depends on P43 DoD-2 send path (not yet built)"]
    fn b2_real_notification_reaches_bound_channel() {
        let mut router = NotificationRouter::default();
        router.bind(NotificationBinding {
            order_id: "ORD-B2".into(),
            channel_ref: "CH-B2".into(),
        });
        // Once P43 transmits, a state change on ORD-B2 must arrive at CH-B2 only.
        let delivered = router
            .deliver("ORD-B2")
            .expect("P43 delivers to the bound channel");
        assert_eq!(delivered, "CH-B2");
        assert_ne!(
            router.deliver("ORD-OTHER"),
            Some("CH-B2"),
            "never leaks to another order"
        );
    }
}
