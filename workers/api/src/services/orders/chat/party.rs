//! WHO MAY READ AND WHO MAY SPEAK in an order's courier chat (operator
//! decisions 2026-10-02): the customer and the ASSIGNED courier talk; the
//! venue's owner reads, for a dispute, and never writes; the kitchen -- any
//! member of staff -- never sees it, because it is personal data the pass has
//! no use for. PURE, so every refusal is proved without an `Env`; the one
//! binder `principal-binds.sh` looks for (`thread_party`).

use crate::auth::Principal;
use serde_json::Value;

/// Which side of the conversation a principal is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Customer,
    Courier,
    /// The owner: reads everything, says nothing.
    Reader,
}

impl Side {
    /// The word a stored message carries in `from`; a reader writes none.
    pub fn as_str(&self) -> Option<&'static str> {
        match self {
            Self::Customer => Some("CUSTOMER"),
            Self::Courier => Some("COURIER"),
            Self::Reader => None,
        }
    }
}

/// `p`'s side in the chat of `order` (the folded order, with `id` and
/// `courier_id`) at `venue`, or the refusal. `speaking` is a post.
///
/// A customer of another order gets 404, never a 403 that would confirm the
/// order exists; a courier who is not the one carrying it, and every member of
/// staff, get 403; an owner of another venue 404.
pub fn thread_party(p: &Principal, venue: &str, order: &Value, speaking: bool) -> Result<Side, (u16, &'static str)> {
    let id = order.get("id").and_then(Value::as_str);
    let assigned = order.get("courier_id").and_then(Value::as_str);
    match p {
        Principal::Customer { order_id, location_id, .. } if location_id == venue && Some(order_id.as_str()) == id => {
            Ok(Side::Customer)
        }
        Principal::Customer { .. } => Err((404, "not found")),
        Principal::Courier { courier_id, active_location_id, .. }
            if active_location_id == venue && Some(courier_id.as_str()) == assigned =>
        {
            Ok(Side::Courier)
        }
        Principal::Courier { .. } => Err((403, "this is not your delivery")),
        Principal::Owner { active_location_id, .. } if active_location_id.as_deref() == Some(venue) => {
            if speaking {
                Err((403, "the venue reads this conversation and does not speak in it"))
            } else {
                Ok(Side::Reader)
            }
        }
        Principal::Owner { .. } => Err((404, "not found")),
        Principal::Staff { .. } => Err((403, "staff do not see the courier's conversation with the customer")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Principal;
    use serde_json::json;

    fn order() -> Value {
        json!({"id": "o1", "courier_id": "c1", "status": "IN_DELIVERY"})
    }
    fn customer(order_id: &str, venue: &str) -> Principal {
        Principal::Customer { customer_id: "x".into(), order_id: order_id.into(), location_id: venue.into() }
    }
    fn courier(id: &str, venue: &str) -> Principal {
        Principal::Courier { courier_id: id.into(), active_location_id: venue.into(), session_id: "s".into() }
    }
    fn owner(venue: Option<&str>) -> Principal {
        Principal::Owner { user_id: "u".into(), active_location_id: venue.map(str::to_string) }
    }
    fn kitchen(venue: &str) -> Principal {
        Principal::Staff { person_id: "p".into(), active_location_id: venue.into(), session_id: "s".into(), caps: dowiz_hub::caps::Preset::Kitchen.caps() }
    }

    #[test]
    fn the_customer_and_the_assigned_courier_speak_and_read() {
        for speaking in [false, true] {
            assert_eq!(thread_party(&customer("o1", "v"), "v", &order(), speaking), Ok(Side::Customer));
            assert_eq!(thread_party(&courier("c1", "v"), "v", &order(), speaking), Ok(Side::Courier));
        }
    }

    #[test]
    fn the_owner_reads_and_does_not_speak_and_another_venues_owner_is_not_told_the_order_exists() {
        assert_eq!(thread_party(&owner(Some("v")), "v", &order(), false), Ok(Side::Reader));
        assert_eq!(thread_party(&owner(Some("v")), "v", &order(), true).map_err(|e| e.0), Err(403));
        assert_eq!(thread_party(&owner(Some("w")), "v", &order(), false).map_err(|e| e.0), Err(404));
        assert_eq!(thread_party(&owner(None), "v", &order(), false).map_err(|e| e.0), Err(404));
        assert_eq!(Side::Reader.as_str(), None, "a reader has no word to write under");
    }

    #[test]
    fn the_kitchen_another_courier_and_another_customer_are_refused() {
        for speaking in [false, true] {
            assert_eq!(thread_party(&kitchen("v"), "v", &order(), speaking).map_err(|e| e.0), Err(403));
            assert_eq!(thread_party(&courier("c2", "v"), "v", &order(), speaking).map_err(|e| e.0), Err(403));
            assert_eq!(thread_party(&courier("c1", "w"), "v", &order(), speaking).map_err(|e| e.0), Err(403), "the right courier at the wrong venue");
            assert_eq!(thread_party(&customer("o2", "v"), "v", &order(), speaking).map_err(|e| e.0), Err(404));
            assert_eq!(thread_party(&customer("o1", "w"), "v", &order(), speaking).map_err(|e| e.0), Err(404));
        }
        // No courier yet: nobody is the courier.
        let unassigned = json!({"id": "o1", "status": "CONFIRMED"});
        assert_eq!(thread_party(&courier("c1", "v"), "v", &unassigned, false).map_err(|e| e.0), Err(403));
    }
}
