//! One venue's Durable Object (`HubImages`): every image, and what it keeps.
//! Facts cite the file that makes them true; `super` explains the columns.

use super::Basis;
use super::Data::*;
use super::Home::Venue;
use super::Purpose as P;
use super::Subject::*;
use super::{Eraser, Exporter, Retention, Store};

const NO_EXPORT: Exporter = Exporter::Missing("P5");

pub const STORES: &[Store] = &[
    // The order log: every order's contact and address, in the order JSON.
    Store { image: "log", kinds: &[], home: Venue,
        holds: &[Name, Phone, Address, Coordinates, OrderContent, Note, Payment], subjects: &[Customer],
        purpose: P::Order, basis: Basis::Contract,
        retention: Retention::NoLimitYet("the log and its archives are never pruned; P6 adds contact redaction by age"),
        erase: Eraser::Redact("hubdo/forget.rs: Hub::redact over the log and every archive, declared as Forgotten"),
        export: NO_EXPORT },
    // The daily sales cube (W-HIST P2b): a cache of the archives, verified against them.
    Store { image: "cube", kinds: &[], home: Venue, holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::NoLimitYet("derived from the archives and rebuildable (rebuild: true); never pruned"),
        erase: Eraser::NotPersonal("per day: counts, money and dish ids; no contact, no customer key"),
        export: Exporter::NotPersonal },
    // The customer card (note, tags, allergens, usual table, birthday MM-DD).
    Store { image: "people", kinds: &["cust", "alias"], home: Venue,
        holds: &[CustomerCard, Name], subjects: &[Customer],
        purpose: P::Crm, basis: Basis::LegitimateInterest,
        retention: Retention::NoLimitYet("kept while the venue keeps the card; P6"),
        erase: Eraser::Remove("services/customers/forget.rs: forget_people"),
        export: NO_EXPORT },
    // W-MR0 MR8 (D0 amendment 2026-10-04, operator ruling): the guest's taste profile, beside the card,
    // automatic under LEGITIMATE INTEREST (balancing test: docs/privacy/DPIA-personalisation.md), stopped
    // and deleted by one objection (Art. 21); never read to set a sum (tools/gates/no-scoring.sh).
    // W-SENSE (2026-10-04): the same record also holds the guest's taste/texture/aroma weights, the
    // same per CONTEXT AT THE VENUE (band, weekday or weekend, weather -- never the guest's place),
    // weekday x band order counts and 12 monthly snapshots; the objection, the export and the
    // erasure cover them with the record. No new store: the venue's weather is an edge-cache entry
    // keyed by the venue's own coordinates and holds no personal data.
    Store { image: "taste", kinds: &["taste"], home: Venue,
        holds: &[Taste, OrderContent], subjects: &[Customer],
        purpose: P::Personalisation, basis: Basis::LegitimateInterest,
        retention: Retention::Ms(crate::services::customers::taste::KEEP_MS, "12 months after the last order (GUESS 2026-10-04): an older profile reads as absent and is replaced on the next write (taste.rs::expired); no nightly sweep yet"),
        erase: Eraser::Remove("services/customers/hubdo/forget.rs step 1b: taste::forget; taste_routes.rs: the objection (guest_withdraw, or taste_off at checkout) deletes it"),
        export: Exporter::Route("GET /api/order/:id/taste (the guest's own link)") },
    // Proof of consent: pseudonymous key, channel, the exact wording, the time.
    Store { image: "consent", kinds: &[], home: Venue,
        holds: &[Consent], subjects: &[Customer],
        purpose: P::ConsentProof, basis: Basis::LegalObligation,
        retention: Retention::NoLimitYet("Law 124 Art. 8(1) makes the controller able to prove consent; the withdrawal is kept as that proof"),
        erase: Eraser::Redact("dowiz_hub::consent::forget: redacted, the withdrawal kept as proof"),
        export: NO_EXPORT },
    Store { image: "bookings", kinds: &["rsv", "ev"], home: Venue,
        holds: &[Name, Phone, Booking], subjects: &[Customer],
        purpose: P::Booking, basis: Basis::Contract,
        retention: Retention::NoLimitYet("never pruned; P6 adds 12 months after the slot"),
        erase: Eraser::Redact("booking::forget::redact, called by hubdo/forget.rs (G8)"),
        export: NO_EXPORT },
    // WhatsApp / Instagram conversations with the venue.
    // "s" = a WhatsApp delivery state (sent/delivered/read/failed) of an owner reply (W-INT2 #9).
    Store { image: "inbox", kinds: &["m", "r", "s"], home: Venue,
        holds: &[Phone, Messages], subjects: &[Customer],
        purpose: P::CustomerCare, basis: Basis::LegitimateInterest,
        retention: Retention::NoLimitYet("bounded by count by the prune in channels.rs, not by age; P6"),
        erase: Eraser::Missing("P2"),
        export: NO_EXPORT },
    // Guest <-> staff messages about a table or an order ("m"), and the
    // customer <-> courier chat of one order ("cc", W-URGENT 2026-10-02):
    // text and a server time, under the order id; never a phone number.
    Store { image: "threads", kinds: &["m", "cc"], home: Venue,
        holds: &[Messages], subjects: &[Customer, Staff, Courier],
        purpose: P::TableChat, basis: Basis::Contract,
        retention: Retention::Ms(crate::services::orders::chat::store::KEEP_MS, "courier chat: 30 days after the message, pruned nightly (chat::store::prune_at); the venue's own thread is not pruned yet (P6)"),
        erase: Eraser::Remove("hubdo/forget.rs: the threads image is rebuilt without the forgotten orders' messages (chat::store::without_orders)"),
        export: NO_EXPORT },
    // Rendered kitchen tickets waiting to be sent (Telegram, WhatsApp).
    // W-TG: "route" (an event waiting to be fanned out to the groups),
    // "digest" (a group's recurring summary), "dg" (a line waiting for that
    // summary, rendered at the group's personal-data level), "h" (a target's
    // last success and Telegram's last refusal: a chat id and an error, no person).
    // W-FIX O4: "drain" (the one drain lease: an expiry time, no person).
    // W-PUSH: a "push" entry carries one device's endpoint and keys until it is sent.
    // W-SMS: an "sms" entry carries the customer's number and the rendered text
    // until it is sent; "sms_o" the ticked order's stamp (number, consent key),
    // removed when the order ends and in any case after two days
    // (notify/sms/checkout.rs STAMP_TTL_MS); "sms_h" the venue's SMS counts and
    // last failure, no person.
    // W-REFUND: a "stripe_refund" entry carries an order id, the venue, the
    // PaymentIntent id and an amount until Stripe answers; no name, no card.
    Store { image: "outbox", kinds: &["o", "print", "route", "digest", "dg", "h", "drain", "sms", "sms_o", "sms_h", "stripe_refund"], home: Venue,
        holds: &[Name, Phone, Address, OrderContent, Messages, Device], subjects: &[Customer, Courier, Staff, Owner],
        purpose: P::Kitchen, basis: Basis::Contract,
        retention: Retention::UntilDone("removed once delivered (outbox/rails.rs); given up after six tries"),
        erase: Eraser::Remove("services/customers/forget/queued.rs: drop_queued (G8)"),
        export: NO_EXPORT },
    // Exactly-once: the full response of a replayable call, an order included.
    Store { image: "idem", kinds: &["k"], home: Venue,
        holds: &[Name, Phone, Address, OrderContent], subjects: &[Customer],
        purpose: P::ExactlyOnce, basis: Basis::Contract,
        retention: Retention::Ms(crate::idempotency::KEEP_MS, "idempotency/mod.rs: KEEP_MS, swept nightly"),
        erase: Eraser::Expires("one day (idempotency::KEEP_MS)"),
        export: Exporter::NotPersonal },
    // Campaign send marks per pseudonymous customer key.
    Store { image: "campaign", kinds: &[], home: Venue,
        holds: &[Consent], subjects: &[Customer],
        purpose: P::Marketing, basis: Basis::Consent,
        retention: Retention::UntilDone("pruned when the campaign is filed (services/campaigns/send.rs)"),
        erase: Eraser::Missing("P2"),
        export: NO_EXPORT },
    // Stored value: wallet postings under a pseudonymous key.
    Store { image: "ledger", kinds: &["tx"], home: Venue,
        holds: &[Wallet, Payment], subjects: &[Customer],
        purpose: P::StoredValue, basis: Basis::Contract,
        retention: Retention::NoLimitYet("money records are kept for the accounts"),
        erase: Eraser::Retain("a posting is money the venue owes or was paid; Law 124 Art. 15(3)(b) keeps what accounting law requires. The key is a pseudonym and names no one once the person's orders are redacted"),
        export: NO_EXPORT },
    // Couriers: assignments, shifts, the last position fix.
    Store { image: "ops", kinds: &["asg", "shift", "pos"], home: Venue,
        holds: &[CourierPosition, StaffId], subjects: &[Courier],
        purpose: P::Dispatch, basis: Basis::Contract,
        retention: Retention::Latest,
        erase: Eraser::Missing("P7"),
        export: NO_EXPORT },
    // The venue's own error and audit log (who revealed which pseudonym).
    Store { image: "audit", kinds: &["error"], home: Venue,
        holds: &[StaffId, Session], subjects: &[Staff, Customer],
        purpose: P::Security, basis: Basis::LegitimateInterest,
        retention: Retention::Ms(crate::errlog::KEEP_MS, "errlog.rs: KEEP_MS, pruned nightly"),
        erase: Eraser::Expires("seven days (errlog::KEEP_MS)"),
        export: NO_EXPORT },
    // Cash drawer events carry the staff member's id.
    Store { image: "till", kinds: &[], home: Venue,
        holds: &[StaffId, Payment], subjects: &[Staff],
        purpose: P::Accounting, basis: Basis::LegalObligation,
        retention: Retention::NoLimitYet("cash records are kept for the accounts"),
        erase: Eraser::Retain("cash-control records a law requires the venue to keep; they carry a staff id, not a customer"),
        export: NO_EXPORT },
    // Fiscal documents: no buyer fields; operator codes.
    Store { image: "fiscal", kinds: &["fiscal"], home: Venue,
        holds: &[StaffId], subjects: &[Staff],
        purpose: P::Accounting, basis: Basis::LegalObligation,
        retention: Retention::NoLimitYet("fiscal documents are kept for the tax authority"),
        erase: Eraser::Retain("fiscal documents are a legal obligation (Law 124 Art. 15(3)(b)); they name no customer"),
        export: NO_EXPORT },
    // The ebills.al link: config (the till user's credential), import state, crosswalk.
    Store { image: "ebills", kinds: &["config", "state", "map", "seen"], home: Venue,
        holds: &[StaffId, Password], subjects: &[Staff],
        purpose: P::Operations, basis: Basis::Contract,
        retention: Retention::NoLimitYet("until the owner disconnects the link"),
        erase: Eraser::Remove("ebills/routes.rs: the owner's disconnect clears the credential"),
        export: NO_EXPORT },
    Store { image: "floor", kinds: &[], home: Venue,
        holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::NoLimitYet("the till's floor as last read"),
        erase: Eraser::NotPersonal("tables and their state, read from the till; no person"),
        export: Exporter::NotPersonal },
    // Venue configuration; notification tokens; the AI endpoint.
    // The venue's configuration. W-TG: `notify.tg.groups` keeps, per linked
    // Telegram group, its title and the Telegram user id of the staff member
    // who sent the link code (who connected that chat); unlinking the group
    // removes both. No customer.
    Store { image: "settings", kinds: &["fiscal.sender", "notify.tg.groups"], home: Venue,
        holds: &[StaffId], subjects: &[Staff],
        purpose: P::Operations, basis: Basis::Contract,
        retention: Retention::NoLimitYet("configuration; a linked group's linker id lives as long as the link"),
        erase: Eraser::Remove("notify/hook/owner.rs: unlink removes the group and its linker id"),
        export: NO_EXPORT },
    Store { image: "catalog", kinds: &[], home: Venue,
        holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::NoLimitYet("the menu"),
        erase: Eraser::NotPersonal("the menu and the venue record"),
        export: Exporter::NotPersonal },
    Store { image: "posts", kinds: &[], home: Venue,
        holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::NoLimitYet("drafts"),
        erase: Eraser::NotPersonal("marketing drafts about dishes"),
        export: Exporter::NotPersonal },
    // Ingredient movements (received, wasted, counted, cooked, ...) carry the
    // id of the staff member who signed them (`by`), like the cash drawer.
    // Since W-STOCK P5 the same log holds supplier cards as NOTE records
    // (stock/notes.rs): the supplier's contact name, phone and Telegram handle.
    Store { image: "stock", kinds: &["cooked"], home: Venue,
        holds: &[StaffId, Name, Phone], subjects: &[Staff, Supplier],
        purpose: P::Operations, basis: Basis::LegitimateInterest,
        retention: Retention::NoLimitYet("the ingredient ledger; a supplier card stays until the owner edits it"),
        erase: Eraser::Retain("the venue's stock records: a staff id of who moved an ingredient, and the supplier contacts the owner typed; no customer"),
        export: NO_EXPORT },
    Store { image: "i18n", kinds: &["t"], home: Venue,
        holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::NoLimitYet("translations"),
        erase: Eraser::NotPersonal("dish translations"),
        export: Exporter::NotPersonal },
    // W-PUSH: a device's Web Push subscription -- the push service's endpoint
    // URL and the browser's RFC 8291 keys (p256dh, auth) -- and the language to
    // write in, under the order (pc), the courier (pk) or the staff member (ps).
    Store { image: "push", kinds: &["pc", "pk", "ps"], home: Venue,
        holds: &[Device], subjects: &[Customer, Courier, Staff, Owner],
        purpose: P::Order, basis: Basis::Consent,
        retention: Retention::UntilDone("a customer's: removed when the order ends, and in any case after two days (notify/push/subs.rs CUSTOMER_KEEP_MS); anyone's: on 'turn off', or when the push service answers 404/410; at most five devices a person"),
        erase: Eraser::Remove("POST /api/push/unsubscribe (the person's own 'turn off'); hubdo/push_turn.rs at the order's end; notify/push/rail.rs on a 404/410"),
        export: NO_EXPORT },
    Store { image: "rails", kinds: &["rail"], home: Venue,
        holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::Latest,
        erase: Eraser::NotPersonal("delivery-rail health counters"),
        export: Exporter::NotPersonal },
];
