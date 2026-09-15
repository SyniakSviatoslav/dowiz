//! Can this venue take orders yet?
//!
//! THREE THINGS, AND THE REASON THERE ARE THREE. An order that arrives is
//! useless unless all of: there is something to sell, somebody hears the order
//! land, and there is a way to get it to the customer. Miss any one and the
//! failure is invisible from the owner's side and total from the customer's --
//! they place an order into a kitchen that never finds out.
//!
//! The venue that got this wrong is not hypothetical: the commonest way a
//! delivery service fails on its first evening is notifications. The menu is
//! obvious, the delivery fee is obvious, and nobody notices that nothing is
//! bound to the bot until an order sits unanswered for forty minutes.
//!
//! So the check is a GATE, not a checklist: a venue cannot be opened until all
//! three pass, and the refusal names which one is missing.
//!
//! No clock, no store, no I/O. The caller gathers the facts; this decides.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Requirement {
    /// At least one dish that is on sale and priced.
    Menu,
    /// Somebody hears an order land: a bound Telegram chat, or a phone on the
    /// venue that a customer can fall back to.
    Notifications,
    /// A way to get the food to the customer: delivery, or pickup.
    Fulfilment,
}

impl Requirement {
    pub fn key(self) -> &'static str {
        match self {
            Requirement::Menu => "menu",
            Requirement::Notifications => "notifications",
            Requirement::Fulfilment => "fulfilment",
        }
    }

    /// What is missing, and what to do about it. One sentence each, because a
    /// gate that says "not configured" sends the owner hunting.
    pub fn as_str(self) -> &'static str {
        match self {
            Requirement::Menu => {
                "no dish is on sale yet -- add at least one with a price and switch it on"
            }
            Requirement::Notifications => {
                "nothing would hear an order arrive -- connect Telegram, or add the venue's phone"
            }
            Requirement::Fulfilment => {
                "there is no way to get an order to the customer -- set a delivery fee and area, \
                 or turn on pickup"
            }
        }
    }
}

/// The facts the gate reads. Deliberately a struct of plain booleans and counts:
/// everything that had to be looked up has been looked up, so the decision is
/// pure and testable at every combination.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts {
    pub sellable_dishes: usize,
    pub telegram_chats: usize,
    pub has_venue_phone: bool,
    pub delivery_configured: bool,
    pub pickup_enabled: bool,
}

/// Everything that is not yet true, in the order an owner would fix it.
pub fn missing(f: &Facts) -> Vec<Requirement> {
    let mut out = Vec::new();
    if f.sellable_dishes == 0 {
        out.push(Requirement::Menu);
    }
    if f.telegram_chats == 0 && !f.has_venue_phone {
        out.push(Requirement::Notifications);
    }
    if !f.delivery_configured && !f.pickup_enabled {
        out.push(Requirement::Fulfilment);
    }
    out
}

pub fn can_open(f: &Facts) -> bool {
    missing(f).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> Facts {
        Facts {
            sellable_dishes: 12,
            telegram_chats: 1,
            has_venue_phone: false,
            delivery_configured: true,
            pickup_enabled: false,
        }
    }

    #[test]
    fn a_ready_venue_opens() {
        assert!(can_open(&ready()));
        assert_eq!(missing(&ready()), vec![]);
    }

    #[test]
    fn each_leg_stands_alone() {
        assert_eq!(missing(&Facts { sellable_dishes: 0, ..ready() }), vec![Requirement::Menu]);
        assert_eq!(
            missing(&Facts { telegram_chats: 0, ..ready() }),
            vec![Requirement::Notifications]
        );
        assert_eq!(
            missing(&Facts { delivery_configured: false, ..ready() }),
            vec![Requirement::Fulfilment]
        );
    }

    /// Either half satisfies its leg. A venue that answers the phone is a venue
    /// that hears its orders, and a venue that only does pickup is a venue.
    #[test]
    fn either_half_of_a_leg_is_enough() {
        assert!(can_open(&Facts { telegram_chats: 0, has_venue_phone: true, ..ready() }));
        assert!(can_open(&Facts {
            delivery_configured: false,
            pickup_enabled: true,
            ..ready()
        }));
    }

    /// A brand new hub is missing all three, and says so all at once rather
    /// than one at a time across three attempts.
    #[test]
    fn a_new_hub_names_all_three_at_once() {
        assert_eq!(
            missing(&Facts::default()),
            vec![Requirement::Menu, Requirement::Notifications, Requirement::Fulfilment]
        );
        assert!(!can_open(&Facts::default()));
    }

    /// The commonest real failure: everything looks done and nothing is
    /// listening. It must not be the leg that is easiest to miss.
    #[test]
    fn a_venue_nobody_would_hear_cannot_open() {
        let f = Facts {
            sellable_dishes: 52,
            telegram_chats: 0,
            has_venue_phone: false,
            delivery_configured: true,
            pickup_enabled: true,
        };
        assert_eq!(missing(&f), vec![Requirement::Notifications]);
        assert!(missing(&f)[0].as_str().contains("hear"));
    }

    #[test]
    fn every_requirement_explains_itself() {
        for r in [Requirement::Menu, Requirement::Notifications, Requirement::Fulfilment] {
            assert!(!r.key().is_empty());
            assert!(r.as_str().contains("--"), "{} gives no instruction", r.key());
        }
    }
}
