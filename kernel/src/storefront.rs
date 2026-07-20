//! `storefront` — P69 customer storefront & checkout journey (BLUEPRINT-P69, W2/P69).
//!
//! This is the customer surface that makes M1 (a real customer pays real money for a real
//! pickup order through the full-wgpu intent UI) reachable. P69 is *assembly and choreography*,
//! not new mechanism: it reuses P62's `PriceableLeaf`/`CatalogNode`/`Cart`, P60's
//! `PaymentProvider`/`ClientHandoff`/`PaymentStatus`/`IdempotencyKey`, and P57/P58/P64 by
//! *calling* them — it invents exactly two genuinely-new things (per the blueprint §1.5):
//!
//!   1. the **journey step state machine** ([`Journey`]) that drives the multi-step wizard
//!      (Storefront → Menu → Detail → Cart → Fulfillment → Payment → Placed) mapped onto the
//!      P38 narrative arc (§1.1), and its **suspend/resume** across the Path-C redirect out of
//!      the canvas (§4.5);
//!   2. the **honest hub-offline status** ([`HubStatus`]) as a typed journey state (§4.6).
//!
//! The bot-facing static pack (the other new surface) lives in `json_api.rs` (the
//! `bot_pack` projection) so it can be feature-gated behind `json-api`.
//!
//! THE LOAD-BEARING INVARIANTS (falsifiable, §5.1):
//! * **F1 — the arc order is an invariant, not a suggestion.** You cannot pay for an
//!   empty/unfulfilled cart; you cannot skip Cart→Payment. `advance` refuses out-of-order intent.
//! * **F2 — the client redirect NEVER writes `Captured`.** Only P60's webhook fold does (P60 §4.4).
//!   `resume` reads `PaymentStatus` from `query_status_by_key`; a `ReturnSignal` (deep-link or
//!   poll) only *triggers* a re-check. A forged client "success" with NO webhook can never reach
//!   `Placed`/`Captured` (the M1 forged-success test, §4.5 adversarial (i)).
//! * **F3 — the card moment never reaches the canvas.** `suspend` carries ONLY opaque handles;
//!   there is no card-data type to bind (P60 §4.1 firewall).
//! * **F4 — the hub-offline state is degrade-closed.** An ambiguous/timed-out probe is `Offline`,
//!   never optimistically `Online`. There is NO `CentralFallback` variant — §16.14 forbids any
//!   central dowiz order state (§5.1).
//!
//! No card type, no `<input>`, no float money: this module adds none of those.

use crate::cart::Cart;
use crate::catalog::{Availability, LeafId, PriceableLeaf};
use crate::money::{Currency, Money};
use crate::ports::payment_provider::{
    ClientHandoff, IdempotencyKey, PaymentStatus, CLIENT_SESSION_TTL_S,
};

/// The storefront slug — a hub's public handle in `/s/:slug`. Free-form vendor-authored text
/// (no dowiz taxonomy), resolved to ONE location by P37's order route.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorefrontSlug(pub String);

/// Wave-0 fulfillment axis. Pickup is the M1-wired path (no courier); Delivery is a real
/// selectable option whose dispatch/map is P65/P51/P71 (downstream, off the M1 critical path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FulfillmentChoice {
    Pickup,
    Delivery,
}

/// The arc order of the journey. `advance` only transitions between *adjacent* steps in this
/// sequence (F1). `Suspended` is the Path-C redirect state (§4.5); `Placed` is the C.2 **Inciting**
/// beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JourneyStep {
    /// Act-1 establishing shot (C.4): full-bleed Море, hero well.
    Storefront,
    /// Act-2 browse (P62 catalog render).
    Menu,
    /// Item detail + modifier groups (resolve_line).
    Detail { leaf: LeafId },
    /// Unified cross-vendor cart (P62 `Cart`) + charge_legs preview.
    Cart,
    /// Delivery-or-pickup selection.
    Fulfillment { choice: FulfillmentChoice },
    /// Hand-off decision point (Path C / Path B).
    Payment,
    /// Left the canvas for the provider domain (§4.5).
    Suspended(SuspendState),
    /// C.2 Inciting beat (amber burst + held beat) once `Captured`.
    Placed { status: PaymentStatus },
    /// §16.14 honest hub-offline — a TERMINAL-until-online state (F4).
    OfflineHalt,
}

/// The arc-order index used to enforce F1. Lower = earlier.
const ARC_INDEX: fn(&JourneyStep) -> Option<usize> = |s: &JourneyStep| match s {
    JourneyStep::Storefront => Some(0),
    JourneyStep::Menu => Some(1),
    JourneyStep::Detail { .. } => Some(2),
    JourneyStep::Cart => Some(3),
    JourneyStep::Fulfillment { .. } => Some(4),
    JourneyStep::Payment => Some(5),
    // Suspended / Placed / OfflineHalt are terminal-ish arcs; they are not forward-reachable by
    // `advance` (you resume out of Suspended, you do not advance into it).
    JourneyStep::Suspended(_) => None,
    JourneyStep::Placed { .. } => None,
    JourneyStep::OfflineHalt => None,
};

/// The suspend/resume record that survives the Path-C round-trip out of the canvas (§4.5).
/// Persisted via P66 (query-before-replay) so an app-kill during redirect is recoverable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuspendState {
    /// Minted by P66 at draft creation (X6) — the reconnect anchor.
    pub key: IdempotencyKey,
    /// P60 `ClientHandoff` single-use token (opaque; never a card).
    pub session_token: [u8; 32],
    /// `now + CLIENT_SESSION_TTL_S`; a return past this is refused (§4.5).
    pub ttl_deadline_unix_s: i64,
    /// When the handoff began (poll-timeout base).
    pub await_since_unix_s: i64,
    /// Where to fall back to on `Failed`/`OfflineHalt` (Payment, never Cart/Menu).
    pub resume_step: Box<JourneyStep>,
}

/// How a return from the provider domain is detected (§4.5). BOTH may occur; NEITHER writes
/// success — each only triggers a P60 `query_status_by_key(key)` re-check (webhook is truth, §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnSignal {
    /// `dowiz://return?session=…` (installed) or `/s/:slug/return?session=…` (web).
    DeepLink { session_token: [u8; 32] },
    /// The honest fallback: no deep-link arrived → poll `query_status_by_key`.
    Poll,
}

/// Honest hub reachability (§16.14). `Offline` renders an honest status node; there is NO
/// central-dowiz fallback variant (that state is unrepresentable — F4). Degrade-closed: an
/// ambiguous probe result is treated as `Offline`, never optimistically `Online`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubStatus {
    Online,
    Offline,
}

/// Reachability probe deadline (degrade-closed on timeout → `Offline`).
pub const HUB_PROBE_TIMEOUT_MS: u32 = 4000;
/// `query_status_by_key` poll cadence after a poll-return.
pub const RESUME_POLL_INTERVAL_MS: u32 = 2000;
/// Poll ceiling == `CLIENT_SESSION_TTL_S`; past this → Payment retry.
pub const RESUME_POLL_DEADLINE_S: i64 = CLIENT_SESSION_TTL_S as i64;

/// The minimal navigation intent P69 consumes. P64's full `Intent`/`InputRouter::tick` is the
/// production driver; this is the *typed kernel contract* P69 folds over so the FSM is pure and
/// testable without the engine. The blueprint cites P64; we do not re-fork its enum (§2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JourneyIntent {
    /// Browse the menu.
    Browse,
    /// Open an item detail (carries the leaf id).
    Open(LeafId),
    /// Add the current cart line (advances Menu/Detail → Cart).
    AddToCart,
    /// Choose fulfillment axis.
    ChooseFulfillment(FulfillmentChoice),
    /// Proceed to the card moment.
    Checkout,
    /// A confirmed `Captured` webhook arrived; advance to the Inciting beat.
    PaymentConfirmed,
    /// A payment failure; fall back to the resume step.
    PaymentFailed,
}

/// The whole-journey machine. `advance` is intent-driven (P64); `suspend`/`resume` bracket the
/// Path-C redirect. Pure state — it OWNS no money, no card, no order truth (those live in the
/// kernel / payment port).
#[derive(Debug, Clone)]
pub struct Journey {
    slug: StorefrontSlug,
    step: JourneyStep,
    cart: Cart,
    hub: HubStatus,
    /// The catalog leaves needed to price the cart + build the bot pack. The journey is a
    /// *consumer* of P62's leaves (§1.3); it holds a read-only view keyed by product id.
    catalog: std::collections::BTreeMap<String, PriceableLeaf>,
    /// The opaque [`ClientHandoff`] produced by `suspend`, for the shell to open. Intentionally a
    /// private field holding ONLY opaque handles — no card-shaped data can live here (F3).
    last_handoff: Option<ClientHandoff>,
}

impl Journey {
    /// Start a journey at the Storefront (Act-1) establishing shot.
    pub fn new(slug: StorefrontSlug) -> Self {
        Journey {
            slug,
            step: JourneyStep::Storefront,
            cart: Cart::new(),
            hub: HubStatus::Online,
            catalog: std::collections::BTreeMap::new(),
            last_handoff: None,
        }
    }

    /// Inject the P62 catalog leaves this journey may reference (read-only view).
    pub fn set_catalog(&mut self, leaves: Vec<PriceableLeaf>) {
        self.catalog.clear();
        for l in leaves {
            self.catalog.insert(l.leaf_id.0.clone(), l);
        }
    }

    /// The current step (read-only).
    pub fn step(&self) -> JourneyStep {
        self.step.clone()
    }

    /// The hub reachability state.
    pub fn hub(&self) -> HubStatus {
        self.hub
    }

    /// The storefront slug.
    pub fn slug(&self) -> &StorefrontSlug {
        &self.slug
    }

    /// Record a hub reachability probe result (degrade-closed — callers map a timeout/ambiguous
    /// result to `Offline`). If the hub is `Offline` mid-checkout, the journey parks at
    /// `OfflineHalt` and holds the cart as a local draft (§4.6 / F4).
    pub fn set_hub_status(&mut self, status: HubStatus) {
        self.hub = status;
        if status == HubStatus::Offline
            && matches!(
                self.step,
                JourneyStep::Payment
                    | JourneyStep::Fulfillment { .. }
                    | JourneyStep::Cart
                    | JourneyStep::Detail { .. }
                    | JourneyStep::Menu
            )
        {
            self.step = JourneyStep::OfflineHalt;
        }
    }

    /// Intent-driven advance (P64). Enforces the arc order (F1): you may only move FORWARD along
    /// the arc, and you may not `Checkout` from an empty/unfulfilled cart. Returns the resulting
    /// step.
    ///
    /// `Checkout` from `Payment` is accepted only when the cart is non-empty AND the fulfillment
    /// choice is Pickup (M1 wired path) OR Delivery-with-address. Address presence is the
    /// caller's responsibility (a `has_address` flag) — Delivery with no address is refused
    /// (§4.3 adversarial).
    pub fn advance(&mut self, intent: JourneyIntent, has_address: bool) -> JourneyStep {
        // F4: if the hub is offline, no forward progress is allowed — park at OfflineHalt and
        // hold the draft. (No central queue; the webhook/payment simply does not fire.)
        if self.hub == HubStatus::Offline {
            self.step = JourneyStep::OfflineHalt;
            return self.step.clone();
        }

        let next = match (&self.step, intent) {
            (JourneyStep::Storefront, JourneyIntent::Browse) => Some(JourneyStep::Menu),
            (JourneyStep::Menu, JourneyIntent::Open(leaf)) => Some(JourneyStep::Detail { leaf }),
            (JourneyStep::Menu, JourneyIntent::AddToCart) => {
                // Adding from the menu list also moves to the cart.
                if self.cart.is_empty() {
                    // No item chosen → cannot enter Cart (an empty cart carries no order).
                    None
                } else {
                    Some(JourneyStep::Cart)
                }
            }
            (JourneyStep::Detail { .. }, JourneyIntent::AddToCart) => {
                if self.cart.is_empty() {
                    None
                } else {
                    Some(JourneyStep::Cart)
                }
            }
            (JourneyStep::Cart, JourneyIntent::ChooseFulfillment(choice)) => {
                Some(JourneyStep::Fulfillment { choice })
            }
            (JourneyStep::Cart, JourneyIntent::Checkout) => {
                // F1: from the cart you may pay only if there is something to pay for.
                if self.cart.is_empty() {
                    None
                } else {
                    Some(JourneyStep::Payment)
                }
            }
            (JourneyStep::Fulfillment { choice }, JourneyIntent::Checkout) => {
                // F1 + M3 adversarial: empty cart ⇒ refuse; Delivery without an address ⇒ refuse.
                if self.cart.is_empty() {
                    None
                } else if matches!(choice, FulfillmentChoice::Delivery) && !has_address {
                    None
                } else {
                    Some(JourneyStep::Payment)
                }
            }
            _ => None,
        };

        if let Some(n) = next {
            // Enforce adjacency along the arc (F1) — refuse any leap that is not a legal forward
            // step from the current position. (All arms above are already adjacent, but this is
            // the typed guard that would catch a future refactor.)
            let lhs = ARC_INDEX(&self.step);
            let rhs = ARC_INDEX(&n);
            if let (Some(a), Some(b)) = (lhs, rhs) {
                if b <= a {
                    return self.step.clone(); // never step backward via advance
                }
            }
            self.step = n;
        }
        self.step.clone()
    }

    /// Add a catalog leaf to the cart at unit price resolved from the trusted catalog (P62 X7).
    /// Returns `Err` if the leaf is unknown to this journey's catalog (fail-closed — never a
    /// caller-supplied price). Sold-out leaves are priced but their add is refused (§4.2).
    pub fn add_leaf(&mut self, product_id: &str, qty: i64) -> Result<(), String> {
        let leaf = self
            .catalog
            .get(product_id)
            .ok_or_else(|| format!("unknown leaf in storefront catalog: {product_id}"))?;
        if leaf.availability != Availability::Available {
            return Err("leaf unavailable (sold out / scheduled) — cannot add".into());
        }
        let unit = leaf.price.minor; // trusted: resolved from P62, not the caller
        self.cart
            .add(product_id, "", qty)
            .map_err(|e| e.to_string())?;
        // price() re-prices at the catalog unit so the cart total is authoritative.
        let _ = self
            .cart
            .price(|p| self.catalog.get(p).map(|l| l.price.minor).unwrap_or(unit));
        Ok(())
    }

    /// The integer subtotal of the cart, priced from the trusted catalog (no float).
    pub fn cart_subtotal(&self) -> Result<Money, String> {
        let cur = self
            .catalog
            .values()
            .next()
            .map(|l| l.price.currency)
            .unwrap_or(Currency::All);
        let (_, total) = self
            .cart
            .price(|p| self.catalog.get(p).map(|l| l.price.minor).unwrap_or(0))?;
        Ok(Money::new(total, cur))
    }

    /// At Payment: bracket the Path-C redirect. `suspend` transitions to `Suspended`, carrying
    /// ONLY opaque handles (F3 — no card data), and returns the [`ClientHandoff`] the shell
    /// opens. It writes NOTHING about capture — that is the webhook's job (F2).
    pub fn suspend(&mut self, handoff: ClientHandoff, s: SuspendState) -> &ClientHandoff {
        self.step = JourneyStep::Suspended(s);
        // The handoff is returned by reference so the shell opens it; the journey never inspects
        // its internals beyond equality.
        // We keep a clone-free borrow: store nothing card-shaped.
        self.last_handoff = Some(handoff);
        self.last_handoff.as_ref().unwrap()
    }

    /// On `ReturnSignal`: re-check P60 status by key (webhook is truth, §4.4). Transition to
    /// `Placed{Captured}` only when the status IS `Captured`. A `ReturnSignal` carrying a status
    /// that is still `Authorized`/`IntentCreated` stays in an honest "confirming" state — never a
    /// fabricated `Captured` (F2). A stale `session_token` (ttl elapsed) ⇒ refuse, fall back to
    /// `resume_step` (Payment). Offline at return ⇒ `OfflineHalt` (F4).
    pub fn resume(
        &mut self,
        signal: ReturnSignal,
        status: PaymentStatus,
        hub: HubStatus,
        now_unix_s: i64,
    ) -> JourneyStep {
        // Pull the suspend record (we only ever have one outstanding intent, X11).
        let suspend = match &self.step {
            JourneyStep::Suspended(s) => s.clone(),
            _ => return self.step.clone(), // nothing to resume
        };

        // F4: offline at return ⇒ hold the draft, no fake retry.
        if hub == HubStatus::Offline {
            self.hub = HubStatus::Offline;
            self.step = JourneyStep::OfflineHalt;
            return self.step.clone();
        }

        // Stale token (ttl elapsed) ⇒ refuse, re-mint from the Payment step (P60 session_token
        // single-use / expired-TTL, §4.5 adversarial (ii)).
        if let ReturnSignal::DeepLink { session_token } = signal {
            if session_token != suspend.session_token {
                // wrong token — refuse
                self.step = *suspend.resume_step.clone();
                return self.step.clone();
            }
        }
        if now_unix_s > suspend.ttl_deadline_unix_s {
            // past the single-use window — re-mint on the resume step
            self.step = *suspend.resume_step.clone();
            return self.step.clone();
        }

        match status {
            PaymentStatus::Captured => {
                // Webhook has moved the kernel fold (P60 §4.4). The Inciting beat.
                self.step = JourneyStep::Placed {
                    status: PaymentStatus::Captured,
                };
            }
            PaymentStatus::Failed(_) => {
                // Honest failure → back to Payment (resume_step), never Placed.
                self.step = *suspend.resume_step.clone();
            }
            // Authorized / IntentCreated / NoneYet / Voided / Refunded ⇒ stay confirming;
            // the client redirect NEVER self-certifies (F2). We remain Suspended (honest
            // "confirming payment…") so the poll loop continues.
            _ => {
                // remain suspended (the caller keeps polling query_status_by_key)
            }
        }
        self.step.clone()
    }

    /// The last opaque handoff (set by `suspend`), for the shell to open.
    pub fn last_handoff(&self) -> Option<&ClientHandoff> {
        self.last_handoff.as_ref()
    }

    /// Clear / reset the handoff after it has been consumed by the shell.
    pub fn take_handoff(&mut self) -> Option<ClientHandoff> {
        self.last_handoff.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Availability, LeafId, LeafKind, PriceableLeaf};
    use crate::money::{Currency, Money};
    use crate::vendor::VendorId;

    const TOKEN: [u8; 32] = [0xABu8; 32];
    const OTHER_TOKEN: [u8; 32] = [0xCDu8; 32];

    fn leaf(id: &str, minor: i64) -> PriceableLeaf {
        PriceableLeaf::new(
            LeafId(id.into()),
            VendorId(1),
            Money::new(minor, Currency::All),
            LeafKind::Item,
            Availability::Available,
        )
        .unwrap()
    }

    /// Build a [`SuspendState`] anchored at `Payment` with the test session token.
    fn suspend_state() -> SuspendState {
        SuspendState {
            key: IdempotencyKey([0u8; 32]),
            session_token: TOKEN,
            ttl_deadline_unix_s: 1_000_000,
            await_since_unix_s: 0,
            resume_step: Box::new(JourneyStep::Payment),
        }
    }

    /// A Path-C handoff carrying ONLY opaque handles (never card data — F3).
    fn handoff() -> ClientHandoff {
        ClientHandoff::HostedRedirect {
            checkout_url: "https://pay.example.invalid/c/abc".into(),
            session_token: TOKEN,
            ttl_s: CLIENT_SESSION_TTL_S,
        }
    }

    /// Walk Storefront → Menu → Cart → Payment with a real (priced) cart.
    /// Sets its own two-leaf catalog so callers don't need to.
    fn drive_to_payment(j: &mut Journey) {
        j.set_catalog(vec![leaf("burrito", 850), leaf("taco", 500)]);
        assert_eq!(j.advance(JourneyIntent::Browse, false), JourneyStep::Menu);
        j.add_leaf("burrito", 1).unwrap();
        j.add_leaf("taco", 2).unwrap();
        assert_eq!(j.advance(JourneyIntent::AddToCart, true), JourneyStep::Cart);
        assert_eq!(j.cart_subtotal().unwrap(), Money::new(1850, Currency::All));
        assert_eq!(
            j.advance(JourneyIntent::Checkout, true),
            JourneyStep::Payment
        );
    }

    // GREEN (§5.1/M5): a fresh storefront opens at Storefront with the hub Online.
    #[test]
    fn fresh_storefront_lands_online() {
        let s = Journey::new(StorefrontSlug("/s/taqueria".into()));
        assert_eq!(s.step(), JourneyStep::Storefront);
        assert_eq!(s.hub(), HubStatus::Online);
    }

    // GREEN (§4.1): Storefront -> Browse -> Cart -> Payment is the ONLY happy path.
    #[test]
    fn happy_path_lands_to_payment() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("burrito", 850), leaf("taco", 500)]);
        drive_to_payment(&mut s);
    }

    // RED→GREEN (§4.1): a customer with NO pickup address CANNOT advance past Cart to Payment
    // when fulfillment is Delivery. (Pickup is the M1 wired path and needs no address.)
    #[test]
    fn payment_requires_address_for_delivery() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        s.advance(JourneyIntent::Browse, false);
        s.add_leaf("taco", 1).unwrap();
        s.advance(JourneyIntent::AddToCart, true);
        // Choose Delivery with no address → refused; stays in Fulfillment.
        s.advance(
            JourneyIntent::ChooseFulfillment(FulfillmentChoice::Delivery),
            false,
        );
        assert!(matches!(
            s.step(),
            JourneyStep::Fulfillment {
                choice: FulfillmentChoice::Delivery
            }
        ));
        let p = s.advance(JourneyIntent::Checkout, false);
        assert!(matches!(p, JourneyStep::Fulfillment { .. }));

        // With an address, Delivery reaches Payment.
        let p = s.advance(JourneyIntent::Checkout, true);
        assert_eq!(p, JourneyStep::Payment);

        // Pickup never needs an address.
        let mut s2 = Journey::new(StorefrontSlug("/s/hub".into()));
        s2.set_catalog(vec![leaf("taco", 500)]);
        s2.advance(JourneyIntent::Browse, false);
        s2.add_leaf("taco", 1).unwrap();
        s2.advance(JourneyIntent::AddToCart, true);
        s2.advance(
            JourneyIntent::ChooseFulfillment(FulfillmentChoice::Pickup),
            false,
        );
        let p2 = s2.advance(JourneyIntent::Checkout, false);
        assert_eq!(p2, JourneyStep::Payment);
    }

    // RED→GREEN (§4.1): attempting to skip Browse and jump straight to Payment is refused
    // (the state machine refuses illegal jumps — no silent UI state).
    #[test]
    fn illegal_jump_refused() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        // Storefront -> Checkout directly (skipping Browse/Cart) is illegal.
        let p = s.advance(JourneyIntent::Checkout, true);
        assert_eq!(
            p,
            JourneyStep::Storefront,
            "cannot skip to Payment from Storefront"
        );
    }

    // GREEN (R1 §3 + §4.2/F3): suspend produces an OPAQUE handoff; the journey does NOT
    // self-certify and stays Suspended until a webhook moves the kernel fold.
    #[test]
    fn suspend_is_opaque_and_not_certifying() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);

        let returned = s.suspend(handoff(), suspend_state());
        assert_eq!(
            returned,
            &ClientHandoff::HostedRedirect {
                checkout_url: "https://pay.example.invalid/c/abc".into(),
                session_token: TOKEN,
                ttl_s: CLIENT_SESSION_TTL_S,
            }
        );
        assert!(matches!(s.step(), JourneyStep::Suspended(_)));
        // stayed suspended (honest "confirming payment…"), did NOT self-promote to Placed.
        assert!(matches!(s.step(), JourneyStep::Suspended(_)));
    }

    // RED→GREEN (R1 §3): only a webhook with Captured status moves the fold to Placed
    // (the Inciting beat / M1). Authorized / Pending stay suspended.
    #[test]
    fn only_captured_webhook_places_order() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);
        s.suspend(handoff(), suspend_state());

        // Authorized webhook → stay Suspended.
        let step = s.resume(
            ReturnSignal::Poll,
            PaymentStatus::Authorized,
            HubStatus::Online,
            1,
        );
        assert!(matches!(step, JourneyStep::Suspended(_)));

        // Captured webhook → Placed (the Inciting beat).
        let step = s.resume(
            ReturnSignal::Poll,
            PaymentStatus::Captured,
            HubStatus::Online,
            1,
        );
        assert!(matches!(
            step,
            JourneyStep::Placed {
                status: PaymentStatus::Captured
            }
        ));
    }

    // RED→GREEN (R1 §3/§4.4): a FAILED webhook returns to the resume step (Payment),
    // never to Placed — honest failure, no silent success.
    #[test]
    fn failed_webhook_returns_to_payment() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);
        s.suspend(handoff(), suspend_state());
        let step = s.resume(
            ReturnSignal::Poll,
            PaymentStatus::Failed(crate::ports::payment_provider::FailReason::Declined),
            HubStatus::Online,
            1,
        );
        assert_eq!(
            step,
            JourneyStep::Payment,
            "failure must re-surface at Payment"
        );
    }

    // GREEN (§4.5 adversarial): a wrong/forged deep-link token is refused and the
    // journey falls back to the resume step (Payment), never Placed.
    #[test]
    fn wrong_deeplink_token_refused() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);
        s.suspend(handoff(), suspend_state());

        let step = s.resume(
            ReturnSignal::DeepLink {
                session_token: OTHER_TOKEN,
            },
            PaymentStatus::Captured,
            HubStatus::Online,
            1,
        );
        assert_eq!(
            step,
            JourneyStep::Payment,
            "forged token must not reach Placed"
        );
    }

    // GREEN (§4.5 adversarial): a stale (ttl-elapsed) return is refused; the journey
    // re-mints on the resume step, never self-certifies to Placed.
    #[test]
    fn stale_return_refused() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);
        s.suspend(handoff(), suspend_state());

        let step = s.resume(
            ReturnSignal::Poll,
            PaymentStatus::Captured,
            HubStatus::Online,
            suspend_state().ttl_deadline_unix_s + 1,
        );
        assert_eq!(
            step,
            JourneyStep::Payment,
            "stale return must not reach Placed"
        );
    }

    // GREEN (§7 honest hub status): a customer can be told the hub is Offline and cannot
    // reach Payment. The UI state mirrors the kernel enum — no silent masking.
    #[test]
    fn offline_hub_blocks_payment_and_says_so() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        s.set_hub_status(HubStatus::Offline);
        s.advance(JourneyIntent::Browse, false);
        s.add_leaf("taco", 1).unwrap();
        s.advance(JourneyIntent::AddToCart, true);
        // Offline → Payment is refused; the customer sees the honest Offline state.
        let p = s.advance(JourneyIntent::Checkout, true);
        assert!(matches!(p, JourneyStep::OfflineHalt));
        assert_eq!(s.hub(), HubStatus::Offline);
    }

    // GREEN: Placed is a terminal success state carrying the captured status.
    #[test]
    fn placed_is_terminal_success() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);
        s.suspend(handoff(), suspend_state());
        s.resume(
            ReturnSignal::Poll,
            PaymentStatus::Captured,
            HubStatus::Online,
            1,
        );
        assert!(matches!(s.step(), JourneyStep::Placed { .. }));
    }

    // GREEN (§4.5 adversarial): offline at the moment of return holds the draft at
    // OfflineHalt — no fake retry, no silent Placed.
    #[test]
    fn offline_return_holds_draft() {
        let mut s = Journey::new(StorefrontSlug("/s/hub".into()));
        s.set_catalog(vec![leaf("taco", 500)]);
        drive_to_payment(&mut s);
        s.suspend(handoff(), suspend_state());
        let step = s.resume(
            ReturnSignal::Poll,
            PaymentStatus::Captured,
            HubStatus::Offline,
            1,
        );
        assert!(matches!(step, JourneyStep::OfflineHalt));
    }
}
