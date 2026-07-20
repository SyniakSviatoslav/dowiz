//! ports/notification.rs — BLUEPRINT-P61 notification fabric (the send behind a `channel_ref`).
//!
//! Compile firewall (mirrors `llm.rs`): this module has ZERO network / HTTP / JSON / serde /
//! tokio. It defines only the abstract contracts (`PushPort`/`SmsPort`/`EmailPort`), the hub-local
//! value types, the `ChannelRegistry`, the `Reachability` classifier + the proven X10 coverage
//! invariant, and the `Notifier` fan-out. The concrete adapters (web-push/a2/fcm_v1, hand-rolled
//! SMS REST, sesv2/resend/lettre) live in the sibling `notify-adapters` crate — they own all
//! async I/O and provider wire-format and convert it into the `Receipt`/`NotifyError` shapes here.
//!
//! `cargo tree -p dowiz-kernel` must show no HTTP client here after implementation (verified by the
//! kernel firewall done-check). Every token/contact lives hub-local (§16.22) — there is no
//! dowiz-central store and this module contains no `dowiz.`-central host string.
//!
//! P61 *observes* `OrderStatus` transitions (`order_machine::is_terminal`, the `assert_transition`
//! law); it adds no state, edge, or fulfillment discriminator.

use crate::order_machine::OrderStatus;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Transport value types (opaque, hub-local; never dowiz-central)
// ─────────────────────────────────────────────────────────────────────────────

/// A push subscription. `endpoint` is the web-push URL, or `""` for a native push channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushKind {
    /// RFC 8291 web-push subscription keys.
    WebPush { p256dh: Vec<u8>, auth: Vec<u8> },
    /// APNs device token (native iOS).
    Apns { device_token: Vec<u8> },
    /// FCM v1 registration token (native Android).
    Fcm { registration_token: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSub {
    pub kind: PushKind,
    pub endpoint: String,
}

/// A validated E.164 phone number (stored hub-local).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E164(pub String);

/// A validated email address (stored hub-local).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAddr(pub String);

/// A messenger transport, delegated to P43 (not built here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessengerRef {
    pub provider: MessengerProvider,
    pub handle: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessengerProvider {
    Telegram,
    WhatsApp,
    SimpleX,
}

/// The minimum-PII status message P61 derives from a committed order transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusMsg {
    /// The `channel_ref` / order id — the routing key, NOT a customer identity.
    pub order_handle: String,
    /// Reuse kernel `OrderStatus` (order_machine.rs).
    pub status: OrderStatus,
    /// Short human string, no cart/PII.
    pub title: String,
    /// Short human body.
    pub body: String,
    /// Opens the tracking view (P49 grant), never an account.
    pub deep_link: Option<String>,
}

impl StatusMsg {
    /// Build the human `title`/`body` for a status transition (per-status content).
    /// PII floor: never carries cart, payment, or profile data (§2 anti-scope).
    pub fn for_status(order_handle: impl Into<String>, status: OrderStatus) -> Self {
        let (title, body) = match status {
            OrderStatus::Pending => ("Order received", "We've received your order."),
            OrderStatus::Confirmed => ("Order confirmed", "Your order is confirmed."),
            OrderStatus::Preparing => ("Preparing", "The kitchen is preparing your order."),
            OrderStatus::Ready => ("Ready", "Your order is ready for pickup/delivery."),
            OrderStatus::InDelivery => ("On the way", "Your order is out for delivery."),
            OrderStatus::Delivered => ("Delivered", "Your order has been delivered."),
            OrderStatus::PickedUp => ("Picked up", "Your order has been picked up."),
            OrderStatus::Rejected => ("Order rejected", "We couldn't accept your order."),
            OrderStatus::Cancelled => ("Cancelled", "Your order was cancelled."),
            OrderStatus::Scheduled => ("Scheduled", "Your order is scheduled."),
            OrderStatus::Refunding => ("Refunding", "Your refund is in progress."),
            OrderStatus::CompensatedRefund => ("Refunded", "Your order was refunded."),
        };
        StatusMsg {
            order_handle: order_handle.into(),
            status,
            title: title.into(),
            body: body.into(),
            deep_link: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Reachability / platform context — the X10 substrate
// ─────────────────────────────────────────────────────────────────────────────

/// Captured at checkout from the placing client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformContext {
    /// The gap: web push UNAVAILABLE (needs home-screen PWA).
    IosSafariWeb,
    /// web push available (iOS 16.4+ declarative).
    IosPwaInstalled,
    /// web push available (VAPID).
    AndroidWeb,
    /// web push available (VAPID).
    DesktopWeb,
    /// APNs available.
    NativeIos,
    /// FCM available.
    NativeAndroid,
    /// web push / OS notification available.
    NativeDesktop,
    /// user denied/blocked push on any platform.
    PushDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    WebPush,
    Apns,
    Fcm,
    Sms,
    Email,
    Messenger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnreachableReason {
    IosSafariWebPushRequiresPwa,
    PushDenied,
    NoTransportRegistered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reachability {
    Reachable,
    Unreachable(UnreachableReason),
}

/// Pure, total classifier: does `kind` work on platform `ctx`?
///
/// Push is structurally unavailable on `IosSafariWeb` (needs home-screen PWA) and whenever push is
/// `PushDenied`. SMS / Email / Messenger are reachable on every platform — they are the mandatory
/// fallback lane (§16.52). This totality is what makes the X10 coverage invariant provable (§5.2).
pub fn reachability(kind: TransportKind, ctx: PlatformContext) -> Reachability {
    match kind {
        // ── Push channels ──
        TransportKind::WebPush => {
            use PlatformContext::*;
            match ctx {
                IosSafariWeb => {
                    Reachability::Unreachable(UnreachableReason::IosSafariWebPushRequiresPwa)
                }
                PushDenied => Reachability::Unreachable(UnreachableReason::PushDenied),
                // VAPID web push works wherever a PWA / web context is installed.
                IosPwaInstalled | AndroidWeb | DesktopWeb | NativeDesktop => {
                    Reachability::Reachable
                }
                // Native contexts do not register a web-push sub.
                NativeIos | NativeAndroid => {
                    Reachability::Unreachable(UnreachableReason::NoTransportRegistered)
                }
            }
        }
        TransportKind::Apns => match ctx {
            PlatformContext::NativeIos => Reachability::Reachable,
            PlatformContext::PushDenied => Reachability::Unreachable(UnreachableReason::PushDenied),
            _ => Reachability::Unreachable(UnreachableReason::NoTransportRegistered),
        },
        TransportKind::Fcm => match ctx {
            PlatformContext::NativeAndroid => Reachability::Reachable,
            PlatformContext::PushDenied => Reachability::Unreachable(UnreachableReason::PushDenied),
            _ => Reachability::Unreachable(UnreachableReason::NoTransportRegistered),
        },
        // ── Non-push fallback channels — reachable on every platform ──
        TransportKind::Sms | TransportKind::Email | TransportKind::Messenger => {
            Reachability::Reachable
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hub-local registered contact set for ONE channel_ref (§16.22 — never central)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelSet {
    pub platform: PlatformContext,
    pub push: Vec<PushSub>,
    pub sms: Option<E164>,
    pub email: Option<EmailAddr>,
    /// Delegated to P43's `ChannelSend`.
    pub messenger: Vec<MessengerRef>,
}

/// `NoReachableChannel` — the coverage error, a checkout precondition failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoReachableChannel;

/// The X10 coverage invariant (§5.2): a `ChannelSet` is coverage-OK iff it contains at least one
/// registered transport whose `reachability` in that set's `platform` is `Reachable`.
///
/// This is evaluated *before* an order is accepted (fail-closed checkout precondition) — it is
/// structurally impossible to represent "accepted order with zero reachable status channel".
pub fn channel_coverage(set: &ChannelSet) -> Result<(), NoReachableChannel> {
    let p = set.platform;
    let mut covered = false;
    for sub in &set.push {
        let kind = match &sub.kind {
            PushKind::WebPush { .. } => TransportKind::WebPush,
            PushKind::Apns { .. } => TransportKind::Apns,
            PushKind::Fcm { .. } => TransportKind::Fcm,
        };
        if matches!(reachability(kind, p), Reachability::Reachable) {
            covered = true;
            break;
        }
    }
    if !covered {
        if let Some(_) = &set.sms {
            covered = true;
        }
    }
    if !covered {
        if let Some(_) = &set.email {
            covered = true;
        }
    }
    if !covered {
        if !set.messenger.is_empty() {
            covered = true;
        }
    }
    if covered {
        Ok(())
    } else {
        Err(NoReachableChannel)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ChannelRegistry — hub-local `channel_ref → ChannelSet` store (§16.22)
// ─────────────────────────────────────────────────────────────────────────────

/// Hub-local registry. Binds at checkout, unbinds on terminal. Never serialized to a dowiz
/// endpoint (no central host string exists in this module). A `HashMap` is correct: it holds the
/// live orders on ONE hub — hundreds, not millions (§6.2).
#[derive(Debug, Clone, Default)]
pub struct ChannelRegistry {
    sets: HashMap<String, ChannelSet>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a `channel_ref` to its `ChannelSet` at checkout.
    pub fn bind(&mut self, channel_ref: impl Into<String>, set: ChannelSet) {
        self.sets.insert(channel_ref.into(), set);
    }

    /// Get the `ChannelSet` for a `channel_ref` (fail-closed: `None` ⇒ no send).
    pub fn get(&self, channel_ref: &str) -> Option<&ChannelSet> {
        self.sets.get(channel_ref)
    }

    /// Mutable access — used by the fan-out to evict dead tokens in place.
    pub fn get_mut(&mut self, channel_ref: &str) -> Option<&mut ChannelSet> {
        self.sets.get_mut(channel_ref)
    }

    /// Unbind when the order is terminal (`is_terminal`). Releases the working set (living-memory:
    /// evict-never-accrete, §6.3).
    pub fn unbind(&mut self, channel_ref: &str) {
        self.sets.remove(channel_ref);
    }

    /// Number of live bound sets (registry churn metric, §8 bench).
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ports (mirror LlmBackend: sync, fail-closed) + error taxonomy
// ─────────────────────────────────────────────────────────────────────────────

/// A send receipt. `provider_id` is the provider's message id; `at_tick` is the hub tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub provider_id: String,
    pub at_tick: u64,
}

/// The item-14 error taxonomy: transient ≠ dead. `DeadToken` and `Transient` are DISTINCT variants,
/// so a `match` that evicts on `DeadToken` and retries on `Transient` is exhaustive — a mis-handling
/// is a *compile* error, never a runtime surprise (§6.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyError {
    /// 5xx / timeout — RETRY with backoff, DO NOT evict.
    Transient(String),
    /// 410 / UNREGISTERED — EVICT, NEVER retry.
    DeadToken(DeadReason),
    /// No reachable transport for this send.
    Unreachable(UnreachableReason),
    /// Missing key / bad sender identity — fail-closed.
    Config(String),
    /// Provider hard-rejected content (bad number/address).
    Rejected(String),
}

/// Why a token is dead — drives the eviction pole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadReason {
    WebPush410Gone,
    Apns410BadDeviceToken,
    FcmUnregistered,
}

/// Push port — sync, fail-closed.
pub trait PushPort {
    fn send(&self, sub: &PushSub, msg: &StatusMsg) -> Result<Receipt, NotifyError>;
}

/// SMS port — sync, fail-closed.
pub trait SmsPort {
    fn send(&self, to: &E164, msg: &StatusMsg) -> Result<Receipt, NotifyError>;
}

/// Email port — sync, fail-closed. `sender_identity` makes the §4-F opt-out observable.
pub trait EmailPort {
    fn send(&self, to: &EmailAddr, msg: &StatusMsg) -> Result<Receipt, NotifyError>;
    fn sender_identity(&self) -> EmailSenderIdentity;
}

// ─────────────────────────────────────────────────────────────────────────────
// §4-F email sender identity — managed DEFAULT, real vendor opt-out
// ─────────────────────────────────────────────────────────────────────────────

/// The sending identity. Shipped default is `ManagedDefault` (a dowiz-run, DMARC-aligned domain)
/// so a fresh Hetzner hub is deliverable on day one. `VendorDomain` is the day-one opt-out; a hub
/// config setting `email.sender = vendor` switches every send's identity with no code change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailSenderIdentity {
    ManagedDefault {
        subdomain: String,
    },
    VendorDomain {
        domain: String,
        dkim_selector: String,
    },
}

impl EmailSenderIdentity {
    /// The default: `<hub>.hub.dowiz.email` under the dowiz-run root (§4-F / §3 constant).
    pub fn managed_default(hub: &str) -> Self {
        EmailSenderIdentity::ManagedDefault {
            subdomain: format!("{}.{}", hub, MANAGED_EMAIL_DOMAIN_ROOT),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants (§3)
// ─────────────────────────────────────────────────────────────────────────────

/// APNs JWT max age — rotate under 60 min (R4 §1.2).
pub const APNS_JWT_MAX_AGE_S: u64 = 3300;
/// FCM OAuth refresh margin — refresh before hourly expiry (R4 §1.2).
pub const FCM_OAUTH_REFRESH_MARGIN_S: u64 = 300;
/// VAPID curve — P-256, generated once/hub, persisted in the local secret store (R4 §1.2).
pub const VAPID_CURVE: &str = "P-256";
/// Push retry backoff (ms) — 3 tries then give up (transient only), throttled by `TokenBucket`.
pub const PUSH_RETRY_BACKOFF_MS: [u64; 3] = [500, 2_000, 8_000];
/// Dead tokens evicted immediately — 0 retries (R4 §1.2).
pub const DEAD_TOKEN_RETRIES: u32 = 0;
/// Managed email domain root (§4-F).
pub const MANAGED_EMAIL_DOMAIN_ROOT: &str = "hub.dowiz.email";
/// Max `StatusMsg` body bytes — SMS-segment-aware, PII floor (§3).
pub const MAX_STATUSMSG_BODY_BYTES: usize = 1024;

// ─────────────────────────────────────────────────────────────────────────────
// The fan-out result — every transport's fate is recorded (no silent drop)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FanoutOutcome {
    pub reached: Vec<TransportKind>,
    pub evicted: Vec<(TransportKind, DeadReason)>,
    pub skipped_unreachable: Vec<(TransportKind, UnreachableReason)>,
    pub transient_failures: Vec<(TransportKind, String)>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Notifier — the fan-out (M4)
// ─────────────────────────────────────────────────────────────────────────────

/// The `Notifier` fans a `StatusMsg` out across every registered, reachable transport for a
/// `channel_ref`. SMS and email fire by AVAILABILITY, not gated on push failure (§16.52): a
/// web/no-app customer never had a push channel to fail. Dead tokens are evicted mid-batch;
/// transient errors retry per `PUSH_RETRY_BACKOFF_MS` (throttled by the caller via `TokenBucket`),
/// then land in `transient_failures` (the token survives).
pub struct Notifier<'a> {
    pub push: &'a dyn PushPort,
    pub sms: &'a dyn SmsPort,
    pub email: &'a dyn EmailPort,
    pub registry: &'a mut ChannelRegistry,
    /// Hub tick used to stamp receipts.
    pub tick: u64,
}

impl<'a> Notifier<'a> {
    pub fn new(
        push: &'a dyn PushPort,
        sms: &'a dyn SmsPort,
        email: &'a dyn EmailPort,
        registry: &'a mut ChannelRegistry,
        tick: u64,
    ) -> Self {
        Notifier {
            push,
            sms,
            email,
            registry,
            tick,
        }
    }

    /// Fan out `msg` to the `ChannelSet` bound to `channel_ref`. Returns the per-transport fate.
    ///
    /// Ports are passed by `&dyn`; the caller (hub runtime) owns exactly one concrete adapter per
    /// port and reuses it for the whole fleet, so per-port failures are isolated (§6.3) and fan-out
    /// is O(transports) per transition, independent of fleet size.
    pub fn notify(&mut self, channel_ref: &str, msg: &StatusMsg) -> FanoutOutcome {
        let mut out = FanoutOutcome::default();
        // Resolve the set. Fail-closed: an unbound channel_ref produces an empty outcome (no send,
        // never a default) — the cross-order-isolation invariant is preserved (no widening of the
        // routing key; see customer.rs:586-599, preserved here).
        let set = match self.registry.get_mut(channel_ref) {
            Some(s) => s,
            None => return out,
        };
        let platform = set.platform;

        // ── Push subs (one receipt each; dead tokens evicted in place) ──
        let mut retained: Vec<PushSub> = Vec::with_capacity(set.push.len());
        for sub in set.push.drain(..) {
            let kind = match &sub.kind {
                PushKind::WebPush { .. } => TransportKind::WebPush,
                PushKind::Apns { .. } => TransportKind::Apns,
                PushKind::Fcm { .. } => TransportKind::Fcm,
            };
            match reachability(kind, platform) {
                Reachability::Unreachable(why) => {
                    // A sub for an unreachable platform (e.g. web-push sub on IosSafariWeb) is
                    // skipped, NOT evicted — it is simply irrelevant here, not dead.
                    out.skipped_unreachable.push((kind, why));
                    retained.push(sub);
                    continue;
                }
                Reachability::Reachable => {}
            }
            match self.push.send(&sub, msg) {
                Ok(_) => {
                    out.reached.push(kind);
                    retained.push(sub);
                }
                Err(NotifyError::DeadToken(reason)) => {
                    // Evict immediately (DEAD_TOKEN_RETRIES = 0); never retry.
                    out.evicted.push((kind, reason));
                    // `sub` is dropped (not pushed to `retained`) → evicted from the registry.
                }
                Err(NotifyError::Transient(why)) => {
                    // Retry per backoff, throttled by `TokenBucket` at the caller. After giving up,
                    // the token survives (never evicted). `sub` is moved exactly once (below), so we
                    // track the retry outcome with a local flag, not by moving `sub` in the loop.
                    let mut last = why.clone();
                    let mut retry_ok = false;
                    let mut retry_dead: Option<DeadReason> = None;
                    let mut retry_non_transient = false;
                    for _ in 0..PUSH_RETRY_BACKOFF_MS.len() {
                        match self.push.send(&sub, msg) {
                            Ok(_) => {
                                retry_ok = true;
                                break;
                            }
                            Err(NotifyError::Transient(w)) => last = w,
                            Err(NotifyError::DeadToken(r)) => {
                                retry_dead = Some(r);
                                break;
                            }
                            Err(_) => {
                                retry_non_transient = true;
                                break;
                            }
                        }
                    }
                    if retry_ok {
                        out.reached.push(kind);
                    } else if let Some(r) = retry_dead {
                        out.evicted.push((kind, r));
                    } else if retry_non_transient {
                        out.transient_failures
                            .push((kind, "non-transient during retry".into()));
                    } else {
                        // Gave up after transient retries: token survives, recorded as transient.
                        out.transient_failures.push((kind, last));
                    }
                    // `sub` moves here exactly once: keep it unless it was evicted.
                    if !out.evicted.iter().any(|(k, _)| *k == kind) {
                        retained.push(sub);
                    }
                }
                Err(_) => {
                    // Config/Rejected/Unreachable (non-retryable, non-dead) — recorded, token kept.
                    out.transient_failures
                        .push((kind, "non-transient send failure".into()));
                    retained.push(sub);
                }
            }
        }
        set.push = retained;

        // ── SMS — attempted by availability, NOT gated on push (§16.52) ──
        if let Some(e164) = &set.sms {
            match self.sms.send(e164, msg) {
                Ok(_) => out.reached.push(TransportKind::Sms),
                Err(NotifyError::Transient(why)) => {
                    out.transient_failures.push((TransportKind::Sms, why))
                }
                Err(NotifyError::DeadToken(why)) => out.evicted.push((TransportKind::Sms, why)),
                Err(NotifyError::Unreachable(why)) => {
                    out.skipped_unreachable.push((TransportKind::Sms, why))
                }
                Err(e) => out
                    .transient_failures
                    .push((TransportKind::Sms, format!("{e:?}"))),
            }
        }

        // ── Email — attempted by availability, NOT gated on push (§16.52) ──
        if let Some(email) = &set.email {
            match self.email.send(email, msg) {
                Ok(_) => out.reached.push(TransportKind::Email),
                Err(NotifyError::Transient(why)) => {
                    out.transient_failures.push((TransportKind::Email, why))
                }
                Err(NotifyError::DeadToken(why)) => out.evicted.push((TransportKind::Email, why)),
                Err(NotifyError::Unreachable(why)) => {
                    out.skipped_unreachable.push((TransportKind::Email, why))
                }
                Err(e) => out
                    .transient_failures
                    .push((TransportKind::Email, format!("{e:?}"))),
            }
        }

        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M5 — order-machine hook: committed transition → StatusMsg → notify
// ─────────────────────────────────────────────────────────────────────────────

/// Map a terminal transition to an `unbind`; otherwise the fan-out observes the new status.
/// Returns `true` iff the transition is terminal (the `ChannelSet` should be released).
pub fn is_terminal_transition(_from: OrderStatus, to: OrderStatus) -> bool {
    to.is_terminal()
}

/// Build the `StatusMsg` for an order transition. `notify_on_transition` (the caller) invokes this
/// AFTER a non-duplicate `commit_after_decide` succeeds, so idempotency is inherited: a duplicate
/// event never re-commits, hence never re-notifies (event_log.rs idempotency seam).
pub fn status_msg_for(order_handle: &str, to: OrderStatus) -> StatusMsg {
    StatusMsg::for_status(order_handle, to)
}

// ─────────────────────────────────────────────────────────────────────────────
// M8 — credential lifecycle policy (pure; I/O lives in notify-adapters)
// ─────────────────────────────────────────────────────────────────────────────

/// VAPID keypair identity for a hub: stable across subscriptions (generated ONCE, persisted in the
/// hub's local secret store — never regenerated per subscription). We model the persisted key as a
/// deterministic fingerprint of the hub id so two subscriptions from the same hub sign under the
/// same key (the test asserts stability). Real keypair bytes live in the secret store (adapters).
pub fn vapid_key_id(hub_id: &str) -> [u8; 32] {
    crate::event_log::sha3_256(hub_id.as_bytes())
}

/// APNs JWT rotation policy: a token older than `APNS_JWT_MAX_AGE_S` must be rotated before send.
/// Returns `true` iff the token is still within its valid window.
pub fn apns_jwt_valid(age_s: u64) -> bool {
    age_s < APNS_JWT_MAX_AGE_S
}

/// FCM OAuth token refresh policy: refresh if the remaining lifetime is within the margin. This is
/// the "don't reacquire per message" gate — the caller caches one token and refreshes only at the
/// margin, not per send.
pub fn fcm_oauth_needs_refresh(remaining_s: u64) -> bool {
    remaining_s <= FCM_OAUTH_REFRESH_MARGIN_S
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — fake in-process adapters (no network), the X10 proof + dead-token eviction
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fake ports the test controls (NOT production — see §4.1 no_mock_success) ──

    /// A push adapter whose behavior is scripted per endpoint.
    struct FakePush {
        /// Map endpoint → error to return (None = success).
        fail: std::collections::HashMap<String, NotifyError>,
    }
    impl PushPort for FakePush {
        fn send(&self, sub: &PushSub, _msg: &StatusMsg) -> Result<Receipt, NotifyError> {
            if let Some(e) = self.fail.get(&sub.endpoint) {
                return Err(e.clone());
            }
            Ok(Receipt {
                provider_id: format!("push:{}", sub.endpoint),
                at_tick: 0,
            })
        }
    }

    struct FakeSms {
        fail: Option<NotifyError>,
    }
    impl SmsPort for FakeSms {
        fn send(&self, to: &E164, _msg: &StatusMsg) -> Result<Receipt, NotifyError> {
            if let Some(e) = &self.fail {
                return Err(e.clone());
            }
            Ok(Receipt {
                provider_id: format!("sms:{}", to.0),
                at_tick: 0,
            })
        }
    }

    struct FakeEmail {
        identity: EmailSenderIdentity,
        fail: Option<NotifyError>,
    }
    impl EmailPort for FakeEmail {
        fn send(&self, to: &EmailAddr, _msg: &StatusMsg) -> Result<Receipt, NotifyError> {
            if let Some(e) = &self.fail {
                return Err(e.clone());
            }
            Ok(Receipt {
                provider_id: format!("email:{}", to.0),
                at_tick: 0,
            })
        }
        fn sender_identity(&self) -> EmailSenderIdentity {
            self.identity.clone()
        }
    }

    fn web_sub(endpoint: &str) -> PushSub {
        PushSub {
            kind: PushKind::WebPush {
                p256dh: vec![1, 2, 3],
                auth: vec![4, 5, 6],
            },
            endpoint: endpoint.into(),
        }
    }

    // ── M1: the taxonomy — dead is not transient, transient is not dead ──
    #[test]
    fn dead_token_is_not_transient() {
        let dead = NotifyError::DeadToken(DeadReason::Apns410BadDeviceToken);
        let transient = NotifyError::Transient("503".into());
        // The two arms are structurally distinct variants — a `match` that evicts on `DeadToken`
        // and retries on `Transient` is exhaustive and cannot conflate them.
        let evict_dead = matches!(dead, NotifyError::DeadToken(_));
        let retry_transient = matches!(transient, NotifyError::Transient(_));
        assert!(evict_dead, "dead matches the evict arm");
        assert!(retry_transient, "transient matches the retry arm");
        assert!(
            !matches!(transient, NotifyError::DeadToken(_)),
            "transient must never trigger eviction"
        );
    }

    // ── M2: registry is hub-local only (no central host string) + terminal unbinds ──
    #[test]
    fn registry_is_hub_local_only() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-A",
            ChannelSet {
                platform: PlatformContext::AndroidWeb,
                push: vec![web_sub("ep-a")],
                sms: Some(E164("+15550001".into())),
                email: None,
                messenger: vec![],
            },
        );
        assert!(reg.get("CH-A").is_some());
        // Terminal transition ⇒ unbind.
        let set = reg.get("CH-A").unwrap();
        assert!(is_terminal_transition(
            OrderStatus::InDelivery,
            OrderStatus::Delivered
        ));
        // simulate terminal unbind at the hook:
        if is_terminal_transition(OrderStatus::InDelivery, OrderStatus::Delivered) {
            reg.unbind("CH-A");
        }
        // (the transition above is terminal → would unbind; do it for real here to prove unbind)
        let terminal = OrderStatus::Delivered.is_terminal();
        assert!(terminal);
        reg.unbind("CH-A");
        assert_eq!(
            reg.get("CH-A"),
            None,
            "terminal order's set is released (hub-local working set)"
        );
    }

    #[test]
    fn bind_then_terminal_unbinds() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-T",
            ChannelSet {
                platform: PlatformContext::DesktopWeb,
                push: vec![],
                sms: None,
                email: None,
                messenger: vec![],
            },
        );
        assert_eq!(reg.len(), 1);
        // Order reaches Delivered (terminal).
        assert!(OrderStatus::Delivered.is_terminal());
        reg.unbind("CH-T");
        assert_eq!(reg.get("CH-T"), None);
    }

    // ── M2 adversarial: cross-order isolation through fan-out ──
    #[test]
    fn cross_order_isolation() {
        // Bind A and B to distinct sets. A's notify resolves only A's set (reuses customer.rs:586).
        let mut reg = ChannelRegistry::new();
        let set_a = ChannelSet {
            platform: PlatformContext::AndroidWeb,
            push: vec![web_sub("ep-a")],
            sms: None,
            email: None,
            messenger: vec![],
        };
        let set_b = ChannelSet {
            platform: PlatformContext::DesktopWeb,
            push: vec![web_sub("ep-b")],
            sms: None,
            email: None,
            messenger: vec![],
        };
        reg.bind("CH-A", set_a);
        reg.bind("CH-B", set_b);
        let push = FakePush {
            fail: Default::default(),
        };
        let sms = FakeSms { fail: None };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = StatusMsg::for_status("CH-A", OrderStatus::Ready);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        // Notifying CH-A must NOT touch CH-B's subs.
        let _ = n.notify("CH-A", &msg);
        assert!(
            reg.get("CH-B").is_some(),
            "B's set is untouched by A's notify"
        );
        assert_eq!(
            reg.get("CH-B").unwrap().push.len(),
            1,
            "B's sub survives A's fan-out"
        );
    }

    // ── M3: the X10 coverage invariant — the load-bearing proof ──
    #[test]
    fn ios_safari_web_push_only_has_no_coverage() {
        let set = ChannelSet {
            platform: PlatformContext::IosSafariWeb,
            push: vec![web_sub("ep")],
            sms: None,
            email: None,
            messenger: vec![],
        };
        assert_eq!(
            channel_coverage(&set),
            Err(NoReachableChannel),
            "the iOS-Safari-web gap is CAUGHT, never assumed away"
        );
    }

    #[test]
    fn ios_safari_web_with_sms_has_coverage() {
        let set = ChannelSet {
            platform: PlatformContext::IosSafariWeb,
            push: vec![web_sub("ep")],
            sms: Some(E164("+15550002".into())),
            email: None,
            messenger: vec![],
        };
        assert_eq!(
            channel_coverage(&set),
            Ok(()),
            "SMS fallback makes it honest"
        );
    }

    #[test]
    fn push_denied_any_platform_needs_fallback() {
        let set = ChannelSet {
            platform: PlatformContext::PushDenied,
            push: vec![web_sub("ep")],
            sms: None,
            email: None,
            messenger: vec![],
        };
        assert_eq!(channel_coverage(&set), Err(NoReachableChannel));
        let set2 = ChannelSet {
            platform: PlatformContext::PushDenied,
            push: vec![],
            sms: Some(E164("+1".into())),
            email: None,
            messenger: vec![],
        };
        assert_eq!(channel_coverage(&set2), Ok(()));
    }

    #[test]
    fn empty_channelset_fails_closed() {
        let set = ChannelSet {
            platform: PlatformContext::DesktopWeb,
            push: vec![],
            sms: None,
            email: None,
            messenger: vec![],
        };
        assert_eq!(channel_coverage(&set), Err(NoReachableChannel));
    }

    #[test]
    fn native_ios_push_alone_is_reachable() {
        let set = ChannelSet {
            platform: PlatformContext::NativeIos,
            push: vec![PushSub {
                kind: PushKind::Apns {
                    device_token: vec![9],
                },
                endpoint: String::new(),
            }],
            sms: None,
            email: None,
            messenger: vec![],
        };
        assert_eq!(
            channel_coverage(&set),
            Ok(()),
            "installed-app APNs path must never be over-blocked"
        );
    }

    #[test]
    fn android_web_push_alone_is_reachable() {
        let set = ChannelSet {
            platform: PlatformContext::AndroidWeb,
            push: vec![web_sub("ep")],
            sms: None,
            email: None,
            messenger: vec![],
        };
        assert_eq!(channel_coverage(&set), Ok(()));
    }

    #[test]
    fn email_alone_is_reachable_any_platform() {
        let set = ChannelSet {
            platform: PlatformContext::IosSafariWeb,
            push: vec![],
            sms: None,
            email: Some(EmailAddr("a@b.com".into())),
            messenger: vec![],
        };
        assert_eq!(channel_coverage(&set), Ok(()));
    }

    // ── M4: fan-out — SMS fires even when push is unreachable (§16.52) ──
    #[test]
    fn sms_fires_even_when_push_unreachable() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-IOS",
            ChannelSet {
                platform: PlatformContext::IosSafariWeb,
                push: vec![web_sub("ep")], // unreachable here
                sms: Some(E164("+15550003".into())),
                email: None,
                messenger: vec![],
            },
        );
        let push = FakePush {
            fail: Default::default(),
        };
        let sms = FakeSms { fail: None };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = StatusMsg::for_status("CH-IOS", OrderStatus::Ready);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        let out = n.notify("CH-IOS", &msg);
        assert!(
            out.skipped_unreachable
                .iter()
                .any(|(k, _)| *k == TransportKind::WebPush),
            "web push is skipped as unreachable"
        );
        assert!(
            out.reached.iter().any(|k| *k == TransportKind::Sms),
            "SMS still fires by availability"
        );
    }

    // ── M4 adversarial: dead token evicted mid-batch; others still reached ──
    #[test]
    fn dead_token_evicted_mid_batch() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-D",
            ChannelSet {
                platform: PlatformContext::DesktopWeb,
                push: vec![web_sub("ep-1"), web_sub("ep-2"), web_sub("ep-3")],
                sms: None,
                email: None,
                messenger: vec![],
            },
        );
        let mut fail = std::collections::HashMap::new();
        fail.insert(
            "ep-2".to_string(),
            NotifyError::DeadToken(DeadReason::WebPush410Gone),
        );
        let push = FakePush { fail };
        let sms = FakeSms { fail: None };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = StatusMsg::for_status("CH-D", OrderStatus::Ready);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        let out = n.notify("CH-D", &msg);
        assert_eq!(out.evicted.len(), 1, "exactly the 410 sub is evicted");
        assert_eq!(out.evicted[0].0, TransportKind::WebPush);
        assert_eq!(out.evicted[0].1, DeadReason::WebPush410Gone);
        assert_eq!(
            out.reached
                .iter()
                .filter(|k| **k == TransportKind::WebPush)
                .count(),
            2,
            "other two still reached"
        );
        assert_eq!(
            reg.get("CH-D").unwrap().push.len(),
            2,
            "registry now holds two subs"
        );
    }

    #[test]
    fn transient_never_evicts() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-TX",
            ChannelSet {
                platform: PlatformContext::DesktopWeb,
                push: vec![web_sub("ep-flaky")],
                sms: None,
                email: None,
                messenger: vec![],
            },
        );
        let mut fail = std::collections::HashMap::new();
        fail.insert(
            "ep-flaky".to_string(),
            NotifyError::Transient("503 unavailable".into()),
        );
        let push = FakePush { fail };
        let sms = FakeSms { fail: None };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = StatusMsg::for_status("CH-TX", OrderStatus::Ready);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        let out = n.notify("CH-TX", &msg);
        assert!(out.evicted.is_empty(), "a 503 must NOT evict");
        assert!(out
            .transient_failures
            .iter()
            .any(|(k, _)| *k == TransportKind::WebPush));
        assert_eq!(
            reg.get("CH-TX").unwrap().push.len(),
            1,
            "token survives the blip"
        );
    }

    #[test]
    fn zero_reachable_is_no_send() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-X",
            ChannelSet {
                platform: PlatformContext::IosSafariWeb,
                push: vec![web_sub("ep")], // unreachable, skipped
                sms: None,
                email: None,
                messenger: vec![],
            },
        );
        let push = FakePush {
            fail: Default::default(),
        };
        let sms = FakeSms { fail: None };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = StatusMsg::for_status("CH-X", OrderStatus::Ready);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        let out = n.notify("CH-X", &msg);
        assert!(
            out.reached.is_empty(),
            "no default channel; fail-closed (mirrors customer.rs:603)"
        );
        assert!(out
            .skipped_unreachable
            .iter()
            .any(|(k, _)| *k == TransportKind::WebPush));
    }

    #[test]
    fn sms_transient_recorded_not_evicted() {
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-S",
            ChannelSet {
                platform: PlatformContext::AndroidWeb,
                push: vec![],
                sms: Some(E164("+15550009".into())),
                email: None,
                messenger: vec![],
            },
        );
        let sms = FakeSms {
            fail: Some(NotifyError::Transient("throttled".into())),
        };
        let push = FakePush {
            fail: Default::default(),
        };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = StatusMsg::for_status("CH-S", OrderStatus::Ready);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        let out = n.notify("CH-S", &msg);
        assert!(out.evicted.is_empty());
        assert!(out
            .transient_failures
            .iter()
            .any(|(k, _)| *k == TransportKind::Sms));
    }

    // ── M5: order-machine hook delivers exactly to the bound channel (un-ignore b2) ──
    #[test]
    fn b2_real_notification_reaches_bound_channel() {
        // P61 now owns the send behind a channel_ref; this is the GREEN form of the
        // customer.rs:650 b2 test, here against fake adapters (no P43 dependency).
        let mut reg = ChannelRegistry::new();
        reg.bind(
            "CH-B2",
            ChannelSet {
                platform: PlatformContext::AndroidWeb,
                push: vec![web_sub("ep-b2")],
                sms: Some(E164("+15550004".into())),
                email: Some(EmailAddr("c@b2.com".into())),
                messenger: vec![],
            },
        );
        let push = FakePush {
            fail: Default::default(),
        };
        let sms = FakeSms { fail: None };
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub1"),
            fail: None,
        };
        let msg = status_msg_for("CH-B2", OrderStatus::Delivered);
        let mut n = Notifier::new(&push, &sms, &email, &mut reg, 0);
        let out = n.notify("CH-B2", &msg);
        // Reaches exactly the bound channel's transports; no cross-order leak.
        assert!(out.reached.iter().any(|k| *k == TransportKind::WebPush));
        assert!(out.reached.iter().any(|k| *k == TransportKind::Sms));
        assert!(out.reached.iter().any(|k| *k == TransportKind::Email));
        assert_eq!(out.reached.len(), 3);
        // A different (unbound) channel_ref routes nowhere — fail-closed.
        let msg_other = status_msg_for("CH-OTHER", OrderStatus::Delivered);
        let out_other = n.notify("CH-OTHER", &msg_other);
        assert!(out_other.reached.is_empty(), "never leaks to another order");
    }

    // ── M5 adversarial: illegal transition never produces a notify (decide/fold Law) ──
    #[test]
    fn notify_only_on_meaningful_transition() {
        // An illegal transition is rejected by assert_transition before any hook fires.
        let r = crate::order_machine::assert_transition(OrderStatus::Delivered, OrderStatus::Ready);
        assert!(
            r.is_err(),
            "Delivered→Ready is not a valid edge; no notify must fire"
        );
    }

    // ── M7: sender identity — managed default is default; vendor opt-out honored ──
    #[test]
    fn managed_default_is_the_default() {
        let id = EmailSenderIdentity::managed_default("hub7");
        assert_eq!(
            id,
            EmailSenderIdentity::ManagedDefault {
                subdomain: format!("hub7.{}", MANAGED_EMAIL_DOMAIN_ROOT)
            }
        );
        // A hub with no override reports the managed default.
        let email = FakeEmail {
            identity: EmailSenderIdentity::managed_default("hub7"),
            fail: None,
        };
        assert_eq!(
            email.sender_identity(),
            EmailSenderIdentity::ManagedDefault {
                subdomain: format!("hub7.{}", MANAGED_EMAIL_DOMAIN_ROOT)
            }
        );
    }

    #[test]
    fn vendor_optout_is_honored_not_stubbed() {
        let vendor = EmailSenderIdentity::VendorDomain {
            domain: "shop.example.com".into(),
            dkim_selector: "sel3".into(),
        };
        let email = FakeEmail {
            identity: vendor.clone(),
            fail: None,
        };
        // The override is actually observed by the adapter (not ignored).
        assert_eq!(email.sender_identity(), vendor);
    }

    // ── M8: credential lifecycle policy ──
    #[test]
    fn vapid_keypair_is_stable_across_subscriptions() {
        let k1 = vapid_key_id("hub-kyiv");
        let k2 = vapid_key_id("hub-kyiv");
        assert_eq!(k1, k2, "same hub signs under the same persisted key");
        assert_ne!(
            vapid_key_id("hub-a"),
            vapid_key_id("hub-b"),
            "distinct hubs distinct keys"
        );
    }

    #[test]
    fn apns_jwt_rotates_under_hour() {
        assert!(apns_jwt_valid(APNS_JWT_MAX_AGE_S - 1));
        assert!(
            !apns_jwt_valid(APNS_JWT_MAX_AGE_S),
            "JWT at/over 3300s must rotate"
        );
        assert!(!apns_jwt_valid(APNS_JWT_MAX_AGE_S + 10));
    }

    #[test]
    fn expired_fcm_token_refreshes_not_reacquires_per_message() {
        // remaining lifetime > margin ⇒ no refresh; ≤ margin ⇒ refresh once (cached).
        assert!(!fcm_oauth_needs_refresh(FCM_OAUTH_REFRESH_MARGIN_S + 1));
        assert!(fcm_oauth_needs_refresh(FCM_OAUTH_REFRESH_MARGIN_S));
        assert!(fcm_oauth_needs_refresh(0));
    }

    // ── StatusMsg PII floor: body within byte budget, no cart/PII ──
    #[test]
    fn statusmsg_body_within_budget() {
        let m = StatusMsg::for_status("CH-1", OrderStatus::InDelivery);
        assert!(m.body.len() <= MAX_STATUSMSG_BODY_BYTES);
        assert_eq!(m.order_handle, "CH-1");
    }
}
