//! THE KEYS AN ORDER KEEPS ACROSS A STATUS TRANSITION.
//!
//! Moved out of `hubstore.rs` (A13 follow-up) because the list grew and that
//! file is the file-size ratchet's worst; `hubstore` re-exports `carry_over`,
//! so every caller (`command/advance.rs`, `courier.rs`) is unchanged.

/// Every field the HUB owns, carried across a kernel transition.
///
/// THE KERNEL RETURNS ITS OWN ORDER and knows nothing about delivery, contact,
/// discounts, tips or timestamps, so anything not on this list is ERASED by the
/// next status change.
///
/// It lives here because there were THREE copies of it -- one in the owner's
/// action, one in the courier's, one implied by the storefront -- and they had
/// drifted. The courier's copy had eight fields and was missing
/// `created_at_ms`, which made every delivery invisible to the earnings fold
/// (its "today" filter compares against a timestamp that had become zero), and
/// missing `tip`, which quietly deleted the courier's own money on pickup.
///
/// A rule with three copies is three rules. This is the one.
pub const HUB_OWNED: &[&str] = &[
    "location_id",
    "contact",
    "fulfilment",
    "payment",
    "payment_status",
    // THE PAYMENT'S OWN RECORD, which used to be erased by the first status
    // change. Every transition stores `delta(old, merged)` where `merged` is
    // the kernel's order plus exactly these keys, and `fold::delta` records a
    // key missing from `merged` as a DELETION. `payment_status` was carried
    // and its evidence was not: the webhook writes the intent id, the amount
    // it actually received and the event fingerprint, and one click on
    // "confirm" deleted all three. A crypto order likewise forgot which of the
    // venue's wallets it was to be paid into. stripe.rs calls that difference
    // "the thing someone will need later", and it was gone before anyone
    // could need it.
    "payment_intent",
    "amount_received",
    "stripe_event",
    "crypto",
    // THE LINES THEMSELVES, because the kernel re-emits them with four fields
    // and `delta` deletes whatever it leaves out. A dish's NAME is written onto
    // the line at placement so a receipt stays true after the dish is renamed
    // or deleted; without this it would survive exactly as long as the order
    // sat at PENDING and vanish on the first confirm. The kernel does not
    // change `items` on a status transition — every transition it accepts is
    // about status — so carrying the stored lines forward is the same data,
    // with the part the hub owns still on it.
    "items",
    "delivery_fee",
    "courier_id",
    "created_at_ms",
    "rejection_reason",
    "cash_collected",
    "courier_note",
    "scheduled_for_ms",
    "tip",
    "discount",
    "promo",
    "feedback",
    "assigned_at_ms",
    "accepted_at_ms",
    "total",
    "at", // when each status was entered: the live estimate measures from these
    "tax", // stamped at placement (TAX blueprint §1.8); a confirm must not erase it
    // ── THE ROOM, added 2026-09-23 after `hubstore/carry/tests.rs` walked an
    // order through the real transition path and found TEN written keys
    // erased by the first confirm. Every one below was deleted on live data.
    //
    // WHICH SITTING AND WHO RANG IT (storefront.rs, staffed placement). Without
    // `sitting_id` a confirmed round leaves its sitting: the bill no longer
    // counts it and the table reads as settled with a dish still cooking.
    "placed_by",
    "sitting_id",
    // THE MONEY TAKEN ON A ROUND (command/pay.rs). `sitting::paid_of` and the
    // till's `cash_payments` (law 10) both sum this array; erased, a paid round
    // reopens and the drawer's expected cash drops while the notes stay in it.
    // `payment_status: "paid"` above survived, so the two disagreed.
    "payments",
    // WHAT WAS CHANGED AND WHY (command/amend.rs, command/transfer.rs): the
    // signer on every amendment and every comp. The room's audit trail; the
    // money itself is on `items`/`discount`/`total`, which were already carried.
    "amended",
    "adjustments",
    // THE REFUND'S RECORD (command/refund.rs): reason, signer, what was owed.
    // REFUNDING advances to REFUNDED through the kernel like any other edge.
    "refund",
    // THE KITCHEN'S "SEEN" (command/kitchen_ack.rs, A13 §2.6).
    "kitchen",
    // ── IMPORTS (ebills/map.rs) ──
    // The order's own currency: `pay` and `refund` read it before the venue's,
    // so erasing it re-prices a foreign-currency order in the venue's.
    "currency",
    // Which fiscal sale this is (uuid, fic, iic): the de-duplication key and
    // the evidence the tax authority asks for.
    "external",
    // Who priced it. The kernel reads orders as untrusted and never emits it.
    "price_trusted",
    // Who typed in an aggregator order off the platform's tablet
    // (services/orders/aggregator.rs, §2.9): the signer of a Placed no guest made.
    "entered_by",
    // NOT CARRIED, on purpose: `eta` is computed at READ time (`live_eta.rs`)
    // and a carried one would be a stale estimate written into the log; `id`,
    // `customer_id`, `status`, `subtotal` (recomputed from `items`),
    // `created_at_ms`, `channel` and `cash_pay_with` are round-tripped by the
    // kernel itself (`json_api::order_to_value`).
];

/// Copy `HUB_OWNED` from the order as it was onto the order the kernel returned.
pub fn carry_over(old: &serde_json::Value, updated: &mut serde_json::Value) {
    for k in HUB_OWNED {
        if let Some(v) = old.get(*k) {
            updated[*k] = v.clone();
        }
    }
}

#[cfg(test)]
mod tests;
