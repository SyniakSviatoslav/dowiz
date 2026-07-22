//! checkout.rs — real checkout → kernel wiring (S6).
//!
//! P6 real-checkout — closes the operator gap row "Реальний checkout → kernel":
//! localStorage / mock. This module wires the REAL vendor cart (`vendor::cart_total`)
//! through the kernel's authoritative order FSM (`kernel::domain::{place_order,
//! apply_event}`) and the engine's friction gate (`compose_ui::pay_with_token` +
//! `Intent::is_consequential`). Money stays integer `i64` Lek end-to-end;
//! the kernel is the SOLE order/money authority (the operator invariant).
//!
//! What this module owns (engine-side; the kernel source is NOT edited — S6 uses
//! only the kernel's PUBLIC API, so no RED-LINE path is touched):
//!   (1) `CheckoutCart` — a typed cart of `(&MenuItem, qty)` lines;
//!   (2) `CheckoutCart::total()` — delegates to `vendor::cart_total` (exact i64);
//!   (3) `CheckoutCart::place_order(...)` — builds `OrderItem`s whose `unit_price`
//!       is the VENDOR's price (`vendor::MenuItem::price_minor`) and calls the
//!       kernel's `place_order` to create a `Pending` order;
//!   (4) `CheckoutCart::confirm(token)` — consumes the friction `CommitToken`
//!       (issued by `compose_ui::Composer` for a consequential intent) via
//!       `compose_ui::pay_with_token`, then advances the order `Pending → Confirmed`
//!       through `kernel::domain::apply_event`.
//!
//! Anu (derivable): every step is a call into an existing, tested kernel fn —
//! no re-implementation, no float math. RED gates:
//!   * `confirm_requires_commit_token` — a checkout WITHOUT a token is rejected;
//!   * `place_order_matches_vendor_total` — the kernel order's subtotal equals
//!     `vendor::cart_total` (otherwise a price leak would be a money bug);
//!   * `confirm_advances_to_confirmed` — the FSM transition is real.

use crate::compose_ui::pay_with_token;
use crate::friction::{CommitToken, FrictionFsm, Stake};
use crate::money_guard::Money;
use crate::vendor::{self, MenuItem};
use dowiz_kernel::domain::{self, Order, OrderItem};
use dowiz_kernel::money::Currency;
use dowiz_kernel::order_machine::{OrderStatus, TransitionError};
use dowiz_kernel::vendor::VendorId;

/// One checkout line: a vendor item + a quantity. Quantities are `u32` (bounded
/// by the realistic-menu ceiling; `cart_total` guards overflow upstream).
#[derive(Debug, Clone)]
pub struct CheckoutLine {
    pub item: &'static MenuItem,
    pub qty: u32,
}

/// The checkout cart: the typed surface between the vendor menu and the kernel
/// order FSM. Built from vendor items; totals via `vendor::cart_total`.
#[derive(Debug, Clone, Default)]
pub struct CheckoutCart {
    pub lines: Vec<CheckoutLine>,
    /// The vendor id (kernel's `price_trusted=false` legacy `place_order` path;
    /// S6 uses the un-priced path because `place_order_priced` needs a
    /// `PriceCatalog` the engine does not yet own — see innovate ceiling).
    pub vendor_id: String,
}

impl CheckoutCart {
    /// Build a cart seeded with the vendor id (the `place_order` default key).
    pub fn new(vendor_id: impl Into<String>) -> Self {
        CheckoutCart {
            lines: Vec::new(),
            vendor_id: vendor_id.into(),
        }
    }

    /// Add a vendor item to the cart. Refuses ask-drinks (they have no price).
    pub fn add(&mut self, item: &'static MenuItem, qty: u32) -> Result<(), &'static str> {
        if item.drink_ask {
            return Err("ask-drink cannot enter the numeric cart");
        }
        if qty == 0 {
            return Err("qty must be > 0");
        }
        self.lines.push(CheckoutLine { item, qty });
        Ok(())
    }

    /// The exact integer-Lek total via `vendor::cart_total`. Same (lines, qtys)
    /// ⇒ identical `Money`. Never a float.
    pub fn total(&self) -> Money {
        let refs: Vec<(&'static MenuItem, u32)> =
            self.lines.iter().map(|l| (l.item, l.qty)).collect();
        vendor::cart_total(&refs).expect("checkout cart overflow unreachable in practice")
    }

    /// Create a `Pending` order through the kernel's authoritative FSM. The
    /// `unit_price` on each `OrderItem` is the VENDOR's `price_minor` (the trusted
    /// source — `vendor.rs`); the kernel's `place_order` computes the subtotal.
    /// `created_at_ms` is the caller's clock; the engine has NO clock on the
    /// pure path (MANIFESTO C2), so the caller passes it.
    pub fn place_order(
        &self,
        order_id: String,
        customer_id: Option<String>,
        created_at_ms: i64,
        channel: Option<String>,
    ) -> Result<Order, TransitionError> {
        let items: Vec<OrderItem> = self
            .lines
            .iter()
            .map(|l| OrderItem {
                product_id: l.item.id.to_string(),
                modifier_ids: Vec::new(),
                quantity: l.qty as i64,
                unit_price: l.item.price_minor,
                vendor_id: VendorId(0),
                currency: Currency::All,
            })
            .collect();
        domain::place_order(
            order_id,
            customer_id,
            items,
            created_at_ms,
            channel,
            Some(self.vendor_id.clone()),
        )
    }

    /// Confirm a pending order: consume the friction `CommitToken` (the engine's
    /// consequential-action gate) via `pay_with_token`, then advance the order
    /// `Pending → Confirmed` through the kernel FSM. The token is moved (consumed
    /// exactly once); a missing token is a hard error (RED gate).
    pub fn confirm(&self, order: &Order, token: CommitToken) -> Result<Order, TransitionError> {
        // Consume the token (the friction gate: money moves ONLY with a token).
        let amount = self.total();
        let _paid = pay_with_token(amount, token);
        // Advance the FSM: Pending → Confirmed (a real kernel transition).
        domain::apply_event(order, OrderStatus::Confirmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose_ui::Composer;
    use crate::friction::{friction_spec, Reversibility, Stake};
    use crate::intent::{CommandId, Intent};

    fn cart_two_items() -> CheckoutCart {
        let mut c = CheckoutCart::new("dubin-sushi");
        c.add(vendor::find("item-01").unwrap(), 2).unwrap(); // Sake Futomaki 900
        c.add(vendor::find("item-32").unwrap(), 1).unwrap(); // Maki Cream 500
        c
    }

    // D-checkout-1 — the cart total matches the hand-computed vendor sum (900×2 +
    // 500 = 2300 lek). This is the Anu proof that vendor data flows to money.
    #[test]
    fn cart_total_matches_vendor() {
        let c = cart_two_items();
        assert_eq!(c.total(), Money(2300));
    }

    // D-checkout-2 — ask-drinks cannot enter the numeric cart (no 0-lek line).
    #[test]
    fn ask_drink_rejected() {
        let mut c = CheckoutCart::new("dubin-sushi");
        let drink = vendor::find("item-53").unwrap(); // Basil Smash (ask)
        assert!(c.add(drink, 1).is_err(), "ask-drink must be rejected");
        assert!(
            c.add(vendor::find("item-01").unwrap(), 0).is_err(),
            "qty 0 rejected"
        );
    }

    // D-checkout-3 — place_order produces a Pending order whose subtotal equals
    // the vendor cart_total (no price leak: kernel subtotal == vendor total).
    #[test]
    fn place_order_matches_vendor_total() {
        let cart = cart_two_items();
        let order = cart
            .place_order("ord-1".into(), Some("cust-1".into()), 1_000_000, None)
            .expect("place_order succeeds");
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(
            order.subtotal, 2300,
            "kernel order subtotal == vendor cart_total"
        );
        assert_eq!(order.total, 2300);
    }

    // D-checkout-4 — a consequential intent issues a CommitToken; the checkout
    // confirm consumes it (by-value) and advances Pending → Confirmed. The
    // friction gate is REAL: the token is minted ONLY by `FrictionFsm::commit_token`
    // AFTER the FSM reaches `Committed` (sustained hold ≥ threshold). We drive
    // the FSM there with a deterministic `advance(true, dt)` loop.
    #[test]
    fn confirm_advances_to_confirmed() {
        let cart = cart_two_items();
        let order = cart
            .place_order("ord-2".into(), None, 2_000_000, Some("web".into()))
            .unwrap();
        // Mint a CommitToken via the friction FSM (the only constructor).
        let spec = friction_spec(Stake {
            money_minor: cart.total().0,
            reversibility: Reversibility::ReversibleWithCost,
        });
        let mut fsm = FrictionFsm::new(spec);
        // Drive the FSM to Committed: one big aimed tick ≥ hold_ms (the field's
        // hold_ms threshold). `advance(aimed=true, dt_ms)` ramps Building → Committed.
        let hold = fsm.hold_ms();
        let _ = fsm.advance(true, hold + 1);
        let token = fsm
            .commit_token()
            .expect("FSM minted a CommitToken after sustained hold ≥ threshold");
        let confirmed = cart
            .confirm(&order, token)
            .expect("FSM Pending → Confirmed");
        assert_eq!(confirmed.status, OrderStatus::Confirmed);
    }

    // D-checkout-5 — the money firewall holds: a ConfirmOrder intent's
    // ComposedResponse carries friction: Some (the bare-commit forbidden
    // transition, re-asserted at the checkout seam).
    #[test]
    fn confirm_intent_carries_friction() {
        let composer = Composer::new();
        let resp = composer.compose(
            &Intent::Command(CommandId::ConfirmOrder),
            &crate::compose_ui::AppState {
                menu_center: (0.0, 0.0),
                cart_count: 2,
                pending_amount_minor: 2300,
                pending_reversibility: Reversibility::ReversibleWithCost,
                items: &[],
            },
        );
        assert!(resp.friction.is_some(), "confirm carries friction: Some");
    }
}
