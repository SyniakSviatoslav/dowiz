//! The fan-out re-authz chokepoint (ADR-0013 + `#4`, proposal §5 Q2 🔴). Admission gates a NEW
//! subscribe; a principal admitted and later revoked keeps streaming until disconnect unless every
//! frame is re-authorized at fan-out time. `TtlGate` is the ONE shared cache/ceiling mechanism
//! (Q-WS-RELAY-GUARD: "collapse to ONE generic `RelayGuard<Policy>`") behind both concrete guards
//! below — they differ only in what they check and whether an `Unavailable` streak eventually
//! evicts (courier: yes, ~60s wall; owner: never, `websocket.ts`'s `createOwnerRelayGuard` has no
//! ceiling because owners have no GPS-rate stream to bound).
//!
//! ## Scope cut (flagged, not silently dropped)
//! The Node guards (`courier-relay-guard.ts`/`createOwnerRelayGuard`) additionally do
//! inflight-dedup (collapse a burst of concurrent frames for the same key into one DB read) and an
//! LRU bound on the allow cache. This port keeps the load-bearing correctness property — an
//! absolute TTL, no-refresh-on-read, and (courier) an in-memory-only ceiling that fires even under
//! total DB starvation — but each stale frame independently awaits its own re-check rather than
//! withholding-and-kicking-a-background-refresh. At the proposal's own back-of-envelope (§2: ~20
//! concurrent GPS streams at 1Hz), redundant concurrent reads inside one cold TTL window are
//! negligible on the 6543 pool; the two Node-only optimizations are out of scope for this build
//! and are not required by any REV-S6 DoD test.
//!
//! ## REV-S6-2 — courier session-liveness (🔴, the load-bearing fix this module adds)
//! `courier-room-authz.ts`'s binding read touches ONLY `courier_assignments`, never
//! `courier_sessions` — confirmed against the migrations (`courier_assignments`'s CHECK constraint
//! and every write site touch only lifecycle status; no logout/session-revoke path resets it,
//! `courier/me.ts:162`, `courier/auth.ts:418-443,500` all `UPDATE courier_sessions ...`, never
//! `courier_assignments`). **Answer: NO — deactivation does NOT reset the `courier_assignments`
//! binding.** So [`CourierRelayGuard::check`] combines the ADR-0013 binding verdict with the S2
//! REV-1 session-liveness check (reusing `AuthRepo::courier_session_bind` +
//! `auth::service::courier_session_valid` VERBATIM — no new session logic, no `auth/` edit) so a
//! logged-out-but-still-bound courier is evicted from the live GPS tail within the same TTL
//! cadence, not merely at next reconnect.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use tokio::time::{Duration, Instant};
use uuid::Uuid;

use super::repo::{Verdict, WsAuthzRepo};
use crate::auth::repo::AuthRepo;
use crate::auth::service::{self, CourierBind, CourierSessionRow};

/// What the fan-out dispatcher does with one frame for one member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayOutcome {
    /// A fresh (or newly-confirmed) `Allow` — the caller sends the frame.
    Relayed,
    /// Stale/`Unavailable` cache — the caller sends NOTHING this frame (never relay-then-check).
    Withheld,
    /// A real `Deny` (or an `Unavailable` streak past the ceiling) — the caller evicts the member
    /// from the room and sends the sanctioned revocation notice (`ControlFrame::Error`), same as
    /// `websocket.ts`'s `evict` callbacks. NEVER a socket close.
    Evict,
}

struct KeyState {
    allow_until: Option<Instant>,
    unavail_since: Option<Instant>,
}

/// The shared TTL + optional-ceiling cache engine. `ceiling = None` ⇒ an `Unavailable` streak is
/// withheld forever (owner policy); `ceiling = Some(wall)` ⇒ a streak older than `wall` evicts,
/// measured purely from in-memory state (holds even under total DB starvation, Breaker NEW-A).
struct TtlGate<K: Eq + Hash + Clone> {
    ttl: Duration,
    ceiling: Option<Duration>,
    state: StdMutex<HashMap<K, KeyState>>,
}

impl<K: Eq + Hash + Clone> TtlGate<K> {
    fn new(ttl: Duration, ceiling: Option<Duration>) -> Self {
        TtlGate {
            ttl,
            ceiling,
            state: StdMutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<K, KeyState>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn fresh_allow(&self, key: &K, now: Instant) -> bool {
        matches!(
            self.lock().get(key),
            Some(KeyState { allow_until: Some(exp), .. }) if now < *exp
        )
    }

    fn record_allow(&self, key: &K, now: Instant) {
        self.lock().insert(
            key.clone(),
            KeyState {
                allow_until: Some(now + self.ttl),
                unavail_since: None,
            },
        );
    }

    fn clear(&self, key: &K) {
        self.lock().remove(key);
    }

    /// Records an `Unavailable` occurrence for `key`; returns `true` iff the ceiling just breached
    /// (the caller must evict). Always `false` for a no-ceiling (owner) gate.
    fn record_unavailable(&self, key: &K, now: Instant) -> bool {
        let Some(ceiling) = self.ceiling else {
            return false;
        };
        let mut state = self.lock();
        let entry = state.entry(key.clone()).or_insert(KeyState {
            allow_until: None,
            unavail_since: None,
        });
        entry.allow_until = None;
        let since = *entry.unavail_since.get_or_insert(now);
        now.checked_duration_since(since).unwrap_or(Duration::ZERO) >= ceiling
    }
}

// ─────────────────────────────────── courier policy ───────────────────────────────────

/// The live context a courier fan-out re-check needs — read fresh from the member's connection
/// state on EVERY frame (never from the cache), matching `courier-relay-guard.ts`'s
/// `check: (orderId, sub, activeLocationId) => ...` closure signature.
#[derive(Debug, Clone, Copy)]
pub struct CourierRelayCtx {
    pub order_id: Uuid,
    pub courier_sub: Uuid,
    pub active_location_id: Uuid,
    /// The token's session id (REV-1). `None` only for a dev-mock-minted token — treated as
    /// "no session to revoke", i.e. the binding verdict alone decides (parity with the S2
    /// `CourierSession` extractor's own `None => dev_login_allowed()` branch; a dev-mock courier
    /// has no real `courier_sessions` row to check against).
    pub jti: Option<Uuid>,
}

pub struct CourierRelayGuard {
    gate: TtlGate<(Uuid, Uuid)>, // (order_id, courier_sub)
    ws_repo: Arc<dyn WsAuthzRepo>,
    auth_repo: Arc<dyn AuthRepo>,
}

const RELAY_TTL: Duration = Duration::from_secs(10);
const COURIER_CEILING: Duration = Duration::from_secs(60);

impl CourierRelayGuard {
    pub fn new(ws_repo: Arc<dyn WsAuthzRepo>, auth_repo: Arc<dyn AuthRepo>) -> Self {
        CourierRelayGuard {
            gate: TtlGate::new(RELAY_TTL, Some(COURIER_CEILING)),
            ws_repo,
            auth_repo,
        }
    }

    /// Gate one frame for one courier member. `now` is threaded through (rather than read via
    /// `Instant::now()` internally) so tests can drive `tokio::time::pause`/`advance` deterministically.
    pub async fn relay(&self, ctx: CourierRelayCtx, now: Instant) -> RelayOutcome {
        let key = (ctx.order_id, ctx.courier_sub);
        if self.gate.fresh_allow(&key, now) {
            return RelayOutcome::Relayed;
        }
        match self.check(ctx).await {
            Verdict::Allow => {
                self.gate.record_allow(&key, now);
                RelayOutcome::Relayed
            }
            Verdict::Deny => {
                self.gate.clear(&key);
                RelayOutcome::Evict
            }
            Verdict::Unavailable => {
                if self.gate.record_unavailable(&key, now) {
                    self.gate.clear(&key);
                    RelayOutcome::Evict
                } else {
                    RelayOutcome::Withheld
                }
            }
        }
    }

    /// REV-S6-2: binding verdict AND (when the token carries a live session id) session-liveness —
    /// reusing the S2 repo/service functions verbatim, never re-deriving the policy.
    async fn check(&self, ctx: CourierRelayCtx) -> Verdict {
        let binding = self
            .ws_repo
            .courier_binding_verdict(ctx.courier_sub, ctx.active_location_id, ctx.order_id)
            .await;
        if binding != Verdict::Allow {
            return binding;
        }
        let Some(jti) = ctx.jti else {
            return Verdict::Allow; // dev-mock token — no session row to revoke, see struct doc.
        };
        match self
            .auth_repo
            .courier_session_bind(jti, ctx.active_location_id, ctx.courier_sub)
            .await
        {
            Ok(bind) => {
                let row = bind.map(|b| CourierSessionRow {
                    revoked_at: b.revoked,
                    expired: b.expired,
                    has_location: b.has_location,
                });
                match service::courier_session_valid(row.as_ref(), true) {
                    CourierBind::Valid => Verdict::Allow,
                    // A revoked/expired/location-removed session is a REAL negative — the courier
                    // is logged out and must not keep watching this order's live tail.
                    CourierBind::Rejected | CourierBind::NoJti => Verdict::Deny,
                }
            }
            Err(_err) => Verdict::Unavailable,
        }
    }

    #[cfg(test)]
    fn stats_len(&self) -> usize {
        self.gate.lock().len()
    }
}

// ─────────────────────────────────── owner policy ───────────────────────────────────

/// The live context an owner fan-out re-check needs. `room_key` is the `(kind, location_or_order)`
/// pair the TTL cache is keyed on — owners re-authorize per ROOM (a `location:` dashboard and an
/// `order:` room the same owner watches are independent memberships, `websocket.ts`'s
/// `ownerRoomVerdict` re-derives per room string).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OwnerRoomKind {
    Location(Uuid),
    Order(Uuid),
}

#[derive(Debug, Clone, Copy)]
pub struct OwnerRelayCtx {
    pub room: OwnerRoomKind,
    pub owner_user_id: Uuid,
}

pub struct OwnerRelayGuard {
    gate: TtlGate<(OwnerRoomKind, Uuid)>,
    ws_repo: Arc<dyn WsAuthzRepo>,
}

impl OwnerRelayGuard {
    pub fn new(ws_repo: Arc<dyn WsAuthzRepo>) -> Self {
        // `ceiling: None` — OR-9/proposal §5: owners have no GPS-rate stream to bound; a pool blip
        // must not bounce a live owner, so `Unavailable` withholds FOREVER rather than evicting.
        OwnerRelayGuard {
            gate: TtlGate::new(RELAY_TTL, None),
            ws_repo,
        }
    }

    pub async fn relay(&self, ctx: OwnerRelayCtx, now: Instant) -> RelayOutcome {
        let key = (ctx.room, ctx.owner_user_id);
        if self.gate.fresh_allow(&key, now) {
            return RelayOutcome::Relayed;
        }
        let verdict = match ctx.room {
            OwnerRoomKind::Location(loc) => {
                self.ws_repo
                    .owner_location_verdict(ctx.owner_user_id, loc)
                    .await
            }
            OwnerRoomKind::Order(order) => {
                self.ws_repo
                    .owner_order_verdict(ctx.owner_user_id, order)
                    .await
            }
        };
        match verdict {
            Verdict::Allow => {
                self.gate.record_allow(&key, now);
                RelayOutcome::Relayed
            }
            Verdict::Deny => {
                self.gate.clear(&key);
                RelayOutcome::Evict
            }
            Verdict::Unavailable => {
                self.gate.record_unavailable(&key, now); // always false (no ceiling) — withhold only.
                RelayOutcome::Withheld
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::repo::CourierSessionBindRow;
    use crate::auth::repo::fake::FakeAuthRepo;
    use crate::ws::repo::fake::FakeWsAuthzRepo;

    fn now() -> Instant {
        Instant::now()
    }

    // ── courier guard: TTL + ceiling core (mirrors courier-relay-guard.test.ts) ──

    #[tokio::test(start_paused = true)]
    async fn cold_frame_awaits_the_check_then_a_later_frame_relays_from_cache() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        let guard = CourierRelayGuard::new(ws_repo.clone(), Arc::new(FakeAuthRepo::default()));
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: Uuid::new_v4(),
            active_location_id: Uuid::new_v4(),
            jti: None,
        };
        assert_eq!(
            guard.relay(ctx, now()).await,
            RelayOutcome::Relayed,
            "ALLOW verdict relays immediately"
        );
        assert_eq!(
            guard.relay(ctx, now()).await,
            RelayOutcome::Relayed,
            "second frame served from cache"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn absolute_ttl_no_refresh_on_access_then_re_checks_after_expiry() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        let guard = CourierRelayGuard::new(ws_repo, Arc::new(FakeAuthRepo::default()));
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: Uuid::new_v4(),
            active_location_id: Uuid::new_v4(),
            jti: None,
        };
        let t0 = now();
        assert_eq!(guard.relay(ctx, t0).await, RelayOutcome::Relayed);
        // Within TTL — still relayed from cache, does not extend the expiry.
        let t_within = t0 + Duration::from_millis(9_999);
        assert_eq!(guard.relay(ctx, t_within).await, RelayOutcome::Relayed);
        // Past the absolute TTL (measured from t0, NOT refreshed by the t_within access) — re-checks.
        let t_after = t0 + Duration::from_secs(10);
        assert_eq!(
            guard.relay(ctx, t_after).await,
            RelayOutcome::Relayed,
            "re-check still ALLOW"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn deny_evicts_and_clears_the_cache() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.courier_binding.lock().unwrap() = Verdict::Deny;
        let guard = CourierRelayGuard::new(ws_repo, Arc::new(FakeAuthRepo::default()));
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: Uuid::new_v4(),
            active_location_id: Uuid::new_v4(),
            jti: None,
        };
        assert_eq!(guard.relay(ctx, now()).await, RelayOutcome::Evict);
        assert_eq!(
            guard.stats_len(),
            0,
            "a denied key must not linger in the cache"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn unavailable_withholds_but_does_not_evict_before_the_ceiling() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.courier_binding.lock().unwrap() = Verdict::Unavailable;
        let guard = CourierRelayGuard::new(ws_repo, Arc::new(FakeAuthRepo::default()));
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: Uuid::new_v4(),
            active_location_id: Uuid::new_v4(),
            jti: None,
        };
        let t0 = now();
        assert_eq!(
            guard.relay(ctx, t0).await,
            RelayOutcome::Withheld,
            "a single DB blip must not evict"
        );
        assert_eq!(
            guard.relay(ctx, t0 + Duration::from_secs(30)).await,
            RelayOutcome::Withheld
        );
    }

    #[tokio::test(start_paused = true)]
    async fn unavailable_streak_past_the_60s_wall_evicts_from_in_memory_state_alone() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.courier_binding.lock().unwrap() = Verdict::Unavailable;
        let guard = CourierRelayGuard::new(ws_repo, Arc::new(FakeAuthRepo::default()));
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: Uuid::new_v4(),
            active_location_id: Uuid::new_v4(),
            jti: None,
        };
        let t0 = now();
        assert_eq!(guard.relay(ctx, t0).await, RelayOutcome::Withheld);
        assert_eq!(
            guard.relay(ctx, t0 + Duration::from_secs(60)).await,
            RelayOutcome::Evict,
            "the ceiling fires purely from the wall-clock delta, no successful DB read needed"
        );
    }

    // ── REV-S6-2: session-liveness eviction (the named DoD test) ──

    #[tokio::test(start_paused = true)]
    async fn revoked_courier_session_evicts_even_though_the_binding_is_still_active() {
        // The courier still holds a live `courier_assignments` row (binding=Allow) — a lazy port
        // would stop here and relay. REV-S6-2's whole point: the SESSION was revoked (logged out /
        // password-changed / reuse-detected), and `courier_assignments` never reflects that. The
        // combined check must still evict.
        let ws_repo = Arc::new(FakeWsAuthzRepo::default()); // binding defaults to Allow
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let sub = Uuid::new_v4();
        let auth_repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            sub,
            CourierSessionBindRow {
                revoked: true,
                expired: false,
                has_location: true,
            },
        ));
        let guard = CourierRelayGuard::new(ws_repo, auth_repo);
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: sub,
            active_location_id: loc,
            jti: Some(jti),
        };
        assert_eq!(
            guard.relay(ctx, now()).await,
            RelayOutcome::Evict,
            "a revoked session must evict the live GPS tail even with an active binding (REV-S6-2)"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn live_session_plus_live_binding_relays() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        let jti = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let sub = Uuid::new_v4();
        let auth_repo = Arc::new(FakeAuthRepo::default().with_courier_bind(
            jti,
            loc,
            sub,
            CourierSessionBindRow {
                revoked: false,
                expired: false,
                has_location: true,
            },
        ));
        let guard = CourierRelayGuard::new(ws_repo, auth_repo);
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: sub,
            active_location_id: loc,
            jti: Some(jti),
        };
        assert_eq!(guard.relay(ctx, now()).await, RelayOutcome::Relayed);
    }

    #[tokio::test(start_paused = true)]
    async fn no_jti_dev_mock_token_skips_the_session_check_and_defers_to_the_binding() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        let guard = CourierRelayGuard::new(ws_repo, Arc::new(FakeAuthRepo::default()));
        let ctx = CourierRelayCtx {
            order_id: Uuid::new_v4(),
            courier_sub: Uuid::new_v4(),
            active_location_id: Uuid::new_v4(),
            jti: None,
        };
        assert_eq!(guard.relay(ctx, now()).await, RelayOutcome::Relayed);
    }

    // ── owner guard: no ceiling ──

    #[tokio::test(start_paused = true)]
    async fn owner_unavailable_withholds_forever_never_evicts() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.owner_location.lock().unwrap() = Verdict::Unavailable;
        let guard = OwnerRelayGuard::new(ws_repo);
        let ctx = OwnerRelayCtx {
            room: OwnerRoomKind::Location(Uuid::new_v4()),
            owner_user_id: Uuid::new_v4(),
        };
        let t0 = now();
        for hours in 0..48 {
            let t = t0 + Duration::from_secs(hours * 3_600);
            assert_eq!(
                guard.relay(ctx, t).await,
                RelayOutcome::Withheld,
                "no ceiling for owners — never evict on a blip"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn owner_deny_evicts_membership_revoked() {
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.owner_order.lock().unwrap() = Verdict::Deny;
        let guard = OwnerRelayGuard::new(ws_repo);
        let ctx = OwnerRelayCtx {
            room: OwnerRoomKind::Order(Uuid::new_v4()),
            owner_user_id: Uuid::new_v4(),
        };
        assert_eq!(guard.relay(ctx, now()).await, RelayOutcome::Evict);
    }

    #[tokio::test(start_paused = true)]
    async fn owner_room_kinds_cache_independently_for_the_same_owner() {
        // A multi-location owner watching both a location dashboard and an order room must not
        // share one cache entry — each `OwnerRoomKind` is re-derived independently.
        let ws_repo = Arc::new(FakeWsAuthzRepo::default());
        *ws_repo.owner_location.lock().unwrap() = Verdict::Allow;
        *ws_repo.owner_order.lock().unwrap() = Verdict::Deny;
        let guard = OwnerRelayGuard::new(ws_repo);
        let owner = Uuid::new_v4();
        let t = now();
        assert_eq!(
            guard
                .relay(
                    OwnerRelayCtx {
                        room: OwnerRoomKind::Location(Uuid::new_v4()),
                        owner_user_id: owner
                    },
                    t
                )
                .await,
            RelayOutcome::Relayed
        );
        assert_eq!(
            guard
                .relay(
                    OwnerRelayCtx {
                        room: OwnerRoomKind::Order(Uuid::new_v4()),
                        owner_user_id: owner
                    },
                    t
                )
                .await,
            RelayOutcome::Evict
        );
    }
}
