//! PLACING AN ORDER, AS ONE COMMAND THE OBJECT EXECUTES.
//!
//! THE DEFECT THIS EXISTS TO CLOSE. Placement was a saga run by the Worker,
//! one network hop away from the images it was changing:
//!
//!   1. `with_stock` — read the stock image, append the reservations, write it.
//!   2. `with_hub` — read the WHOLE log image, count the promo's uses, redeem
//!      it, append `Placed`, write it.
//!   3. if (2) failed, `with_stock` again to release what (1) held — and if
//!      THAT failed, `console_error!` and nothing else.
//!
//! Three things were wrong with it and each cost something real:
//!
//! * **The compensation was the weakest link in the money path.** A stranded
//!   reservation makes a kitchen believe it is out of something it has, and the
//!   only thing that noticed was a console line in a sampled trace.
//! * **The promo total was computed inside the CAS closure and the caller never
//!   learned it.** `envelope["total"]` got the discount; the outer `total`
//!   binding did not, and the outer one was what went to Stripe. A card
//!   customer with a working code saw 2700, had 2700 stored against the order
//!   and was charged 3000 — and the webhook recorded 3000 as `amount_received`,
//!   so nothing downstream disagreed with anything.
//! * **Every write was a read-modify-write over the network**, retried five
//!   times against a generation guard that exists because the decision is made
//!   on the wrong side of the hop.
//!
//! WHAT REPLACES IT, AND WHY THE COMPENSATION IS GONE RATHER THAN IMPROVED.
//! The object holds both images. Inside ONE turn it can do every fallible thing
//! IN MEMORY — reserve, count, redeem, append — and only then write. There is
//! no state to undo, because nothing was written until everything had already
//! succeeded. A compensation that never has to run is better than one that runs
//! correctly; this file is how you get the first kind.
//!
//! AND THE CORE IS PURE, which is the other half of the point. `decide` takes a
//! `Hub` and a `StockLog` — both `dowiz_hub` types, bytes in and bytes out, no
//! clock and no I/O — so the whole placement rule is exercised by ordinary
//! `cargo test` with no Durable Object anywhere near it. Until now not one line
//! of it had a native test, because reaching it meant standing up an object.

use super::Refused;
use serde::{Deserialize, Serialize};

/// Everything the object needs to place an order, and nothing it can look up
/// itself.
///
/// PRICING STAYS IN THE WORKER AND ONLY THE TAIL MOVES. `subtotal`, `fee` and
/// `tip` are decided by `services::ordering::pricing`, which is pure and has
/// its own tests; recomputing them here would be a second pricer, and there
/// were already two of those once (`a18886d4`). What the object decides is the
/// part that must not be decided anywhere else: whether the ingredients are
/// there, and whether this code still has a use left.
///
/// `bom_lines` RATHER THAN THE RESERVATIONS THEMSELVES, because `StockEvent`
/// has no serde derive and should not grow one to cross a boundary inside the
/// same process: `stock::reservations_for` is pure, so the object calls it.
///
/// `promo` IS THE CATALOGUE'S RAW RECORD, parsed here by `Promo::parse`. The
/// Worker has already refused an unknown code with a 400 — that is a message
/// for the customer about what they typed, and it needs the catalogue, which
/// the Worker is holding anyway.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceIn {
    pub order_id: String,
    /// The order as the Worker built it, JSON. The object patches the discount
    /// into it and writes THAT, which is what makes the stored envelope and the
    /// charge the same number by construction.
    pub envelope: String,
    pub seq: u64,
    pub bom_lines: Vec<(String, i64)>,
    pub promo: Option<String>,
    pub promo_code: Option<String>,
    pub subtotal: i64,
    pub fee: i64,
    pub tip: i64,
    pub now_ms: i64,
    /// The kitchen's message, ALREADY RENDERED.
    ///
    /// RENDERED HERE AND ENQUEUED THERE. `notify::order_text` needs the basket
    /// lines and the currency, which the Worker is holding; WHO it goes to is
    /// in the settings image, which the object is holding. So the text crosses
    /// and the recipients do not, and neither side reads an image it did not
    /// already have.
    ///
    /// RENDERED NOW rather than at send time, deliberately: the drain runs
    /// minutes later and must not re-read a catalogue that has changed, or a
    /// kitchen is told about a dish at the wrong price.
    pub notify_text: Option<String>,
}

/// What the object answers with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOut {
    /// The envelope AS WRITTEN, discount and all. The only total with authority
    /// — see the module header for what happened when the caller trusted its
    /// own copy instead.
    pub stored: String,
    pub generation: i64,
    pub events: usize,
}

/// THE WHOLE PLACEMENT RULE, over two images already in memory.
///
/// Both arguments are mutated IN PLACE and the caller writes them only if this
/// returns `Ok`. That ordering is the entire reason the compensation is gone:
/// on any refusal the caller drops both and nothing was ever persisted.
///
/// ORDER MATTERS AND IT IS NOT ARBITRARY. The stock reservation runs FIRST
/// because it is the one that fails in ordinary trade — a venue runs out of
/// salmon several times a week and never runs out of promo codes. Doing the
/// cheap common refusal before the log is touched means the usual failure costs
/// one ledger fold instead of a log append that then has to be thought about.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    listed: &[crate::hubdo::OrderView],
    venue_tax: &Result<Option<crate::services::ordering::tax_cfg::VenueTax>, String>,
    input: &PlaceIn,
) -> Result<String, Refused> {
    // ── INGREDIENTS, ALL OR NOTHING ──
    //
    // A third line that is short must not leave the first two held by an order
    // that was never placed. `append_all` is that atomicity: it refuses the
    // whole batch, and because nothing is written until the end of this
    // function, a refusal here leaves the image exactly as it was found.
    let reservations = dowiz_hub::stock::reservations_for(&input.order_id, &input.bom_lines);
    if !reservations.is_empty() {
        stock.append_all(&reservations).map_err(|e| Refused::Stock(format!("{e}")))?;
    }

    // ── THE DISCOUNT, IN THE SAME BREATH AS THE APPEND THAT MAKES IT REAL ──
    //
    // Counting the uses and then appending in a separate step would let two
    // customers spend the last use of one code at once — rare at one
    // restaurant, and exactly the kind of rare that only ever surfaces as an
    // unexplained loss. Here "count" and "spend" are two statements inside one
    // object turn, which is as close together as they can be.
    let mut envelope: serde_json::Value = serde_json::from_str(&input.envelope)
        .map_err(|e| Refused::Append(format!("envelope is not json: {e}")))?;
    if let Some(raw) = &input.promo {
        let Some(p) = dowiz_hub::promo::Promo::parse(raw) else {
            return Err(Refused::Promo(dowiz_hub::promo::Refusal::Unknown.as_str().to_string()));
        };
        let code = input.promo_code.as_deref().unwrap_or(&p.code);
        let used = crate::services::ordering::promo_fields::promo_uses_in(listed, code);
        let cut = p
            .redeem(input.subtotal, input.now_ms, used)
            .map_err(|r| Refused::Promo(r.as_str().to_string()))?;
        envelope["discount"] = serde_json::json!(cut);
        envelope["promo"] = serde_json::json!({ "code": code, "discount": cut });
        envelope["total"] = serde_json::json!(input.subtotal - cut + input.fee + input.tip);
    }

    // ── THE TAX, ONCE, AFTER THE CUT (G2) ──
    //
    // Here and nowhere else, because this is where the discount is known: the
    // taxable base is what the customer pays for. A venue with no rate places
    // untaxed (the transition rule); a venue WITH one never logs an order it
    // could not tax — `Untaxed` names why.
    match venue_tax {
        Ok(None) => {}
        Ok(Some(v)) => {
            let cut = envelope.get("discount").and_then(serde_json::Value::as_i64).unwrap_or(0);
            crate::services::ordering::tax_block::stamp(&mut envelope, v, input.fee, input.tip, cut)
                .map_err(Refused::Untaxed)?;
        }
        Err(e) => return Err(Refused::Untaxed(e.clone())),
    }

    // G4: an order whose source is not in the set is never logged.
    crate::services::ordering::channel::of(&envelope).map_err(|u| Refused::Append(u.to_string()))?;
    let stored = serde_json::to_string(&envelope).unwrap_or_else(|_| input.envelope.clone());
    hub.append(dowiz_hub::EventKind::Placed, &input.order_id, &stored, input.seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok(stored)
}
