//! The venue's `consent` image, as the Worker reads and writes it (§3.2).
//!
//! The record shape, the fold and the `Consented` witness are
//! `dowiz_hub::consent`; what lands in the image is `consent::log::write`.
//! This file turns a checkout box and an owner's act into an `Act`, and files
//! it.

use serde::Deserialize;
use dowiz_hub::consent::{
    log::lang_of_wording, Act, Method, State, CHANNEL_WHATSAPP, PURPOSE_MARKETING,
};

use crate::hubstore::Place;

/// The image. Backed up by OVERWRITE, never by date (§2.2 rule 4).
pub const IMAGE_CONSENT: &str = "consent";

/// The checkout's box, as the order body carries it. Sent ONLY when ticked;
/// a CLOSED shape, so a flag smuggled in under another name is a 400.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ConsentIn {
    pub marketing_whatsapp: bool,
    /// The id of the sentence the diner was shown (`consent::wording_id`).
    pub wording: String,
}

/// PURE. The act a placement files, if any. `key` is the customer's
/// `customer_key`, `None` when no number was given.
///
/// `Ok(None)` for no box and an unticked one -- Recital 32, placing the order
/// is not consent. A tick the hub cannot prove is REFUSED by name: without a
/// number there is nobody to consent, and an unknown sentence is no proof of
/// what they read. `via` is filled in once the order has an id.
pub fn at_placement(c: Option<&ConsentIn>, key: Option<&str>, now_ms: i64) -> Result<Option<Act>, String> {
    let Some(c) = c.filter(|c| c.marketing_whatsapp) else { return Ok(None) };
    let Some(key) = key else {
        return Err("the offers box needs a phone number to send them to".into());
    };
    if lang_of_wording(&c.wording).is_none() {
        return Err("the offers sentence on this page is out of date; reload and tick it again".into());
    }
    let act = Act {
        key: key.to_string(),
        purpose: PURPOSE_MARKETING.into(),
        channel: CHANNEL_WHATSAPP.into(),
        state: State::Given,
        at_ms: now_ms,
        method: Method::CheckoutBox,
        evidence: String::new(),
        wording_id: c.wording.clone(),
        via: String::new(),
    };
    dowiz_hub::consent::check(&act)?;
    Ok(Some(act))
}

/// An owner's act on the customer's behalf: a STOP heard at the counter, or
/// a paper form. A CLOSED shape.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct OwnerActIn {
    /// `given` or `withdrawn`.
    pub state: String,
    /// What the owner saw. Required for a grant (§4: "never without evidence").
    #[serde(default)]
    pub evidence: String,
    /// The language of the sentence the person was shown. Required for a grant.
    #[serde(default)]
    pub lang: String,
}

/// PURE. The act an owner files. `by` is the owner's id, the `via` a dispute
/// needs. A withdrawal needs nothing but the state (Art. 7(3)); a grant needs
/// evidence and the sentence shown, and `consent::check` has the last word.
pub fn owner_act(key: &str, body: &OwnerActIn, by: &str, now_ms: i64) -> Result<Act, String> {
    let state = State::of(&body.state).ok_or_else(|| format!("unknown state {:?}", body.state))?;
    let wording_id = match state {
        State::Withdrawn => String::new(),
        State::Given => dowiz_hub::consent::wording_id(&body.lang),
    };
    let act = Act {
        key: key.to_string(),
        purpose: PURPOSE_MARKETING.into(),
        channel: CHANNEL_WHATSAPP.into(),
        state,
        at_ms: now_ms,
        method: Method::OwnerEntered,
        evidence: body.evidence.trim().to_string(),
        wording_id,
        via: by.to_string(),
    };
    dowiz_hub::consent::check(&act)?;
    Ok(act)
}

/// File one act. A refusal or a failed write is RECORDED in the venue's error
/// log with its reason and answered as `Err`, never dropped.
pub async fn file(place: &Place, act: &Act) -> std::result::Result<(), String> {
    let a = act.clone();
    let wrote = crate::hubstore::with_log(place, IMAGE_CONSENT, move |log| {
        Ok(dowiz_hub::consent::log::write(log, &a))
    })
    .await;
    let why = match wrote {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(why)) => why,
        Err(e) => format!("consent image: {e}"),
    };
    crate::loud!(&place.ns, Some(&place.venue), "consent.file", "cust:{} not filed: {why}", act.key);
    Err(why)
}
