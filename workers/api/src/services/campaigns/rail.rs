//! THE DRAIN'S SIDE: the three hooks `outbox::rails::drain` calls for a
//! campaign entry. The rules are `send.rs`'s and pure; this is the I/O.
//!
//!   `acts_if_due`  reads the consent image ONCE per drain, and only when a
//!                  campaign entry is due. `None` = the read failed, and the
//!                  drain LEAVES campaign entries where they are: an unreadable
//!                  consent log is not a withdrawal and not a permission.
//!   `deliver`      the send. It takes `&Consented`: G1 holds at the door.
//!   `record_gone`  files `gone` rows for entries removed undelivered, so the
//!                  report can say "withdrawn" and "abandoned" by name.

use dowiz_hub::consent::Consented;
use dowiz_hub::logimage::Entry as LogEntry;
use worker::*;

use super::campaign::{gone_row, IMAGE_CAMPAIGN, KIND_GONE};
use super::send::{any_campaign, parse_id};
use crate::hubstore::Place;
use crate::outbox::{Entry, Verdict};

pub async fn acts_if_due(place: &Place, due: &[&Entry]) -> Option<Vec<LogEntry>> {
    if !any_campaign(due.iter().copied()) {
        return Some(Vec::new());
    }
    let image = crate::services::customers::consent_log::IMAGE_CONSENT;
    match crate::hubstore::load_log(place, image).await {
        Ok(l) => Some(l.log.entries()),
        Err(e) => {
            console_error!("campaign drain {}: consent unreadable, campaign entries wait: {e}", place.venue);
            None
        }
    }
}

/// One campaign message. The witness is the one `send::gate` just produced
/// from the fold as it stands; the address and the TEMPLATE were fixed at
/// enqueue (`send::entry`). An entry whose text is not a template object --
/// one queued as free text before templates -- is not sent as text: WhatsApp
/// would refuse it outside the window, and it fails its way to `abandoned`.
pub async fn deliver(wa: &crate::channels::WhatsApp, who: &Consented, e: &Entry) -> bool {
    if parse_id(&e.id).map(|(_, k)| k) != Some(who.key()) {
        return false;
    }
    let Ok(tpl) = serde_json::from_str::<serde_json::Value>(&e.text) else { return false };
    if !tpl.get("name").is_some_and(serde_json::Value::is_string) {
        return false;
    }
    crate::channels::whatsapp_template(wa, &e.to, &tpl).await.is_ok()
}

/// `withdrawn` are the ids the gate refused this drain.
pub async fn record_gone(place: &Place, verdicts: &[(String, Verdict)], withdrawn: &[String], now_ms: i64) {
    let rows: Vec<(String, String)> = verdicts
        .iter()
        .filter_map(|(id, v)| {
            let (campaign, _) = parse_id(id)?;
            let (why, after) = match v {
                _ if withdrawn.contains(id) => ("withdrawn", 0),
                Verdict::Abandon { after } if *after > 0 => ("abandoned", *after),
                _ => return None,
            };
            Some((campaign.to_string(), gone_row(campaign, id, why, after, now_ms)))
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    let wrote = crate::hubstore::with_log(place, IMAGE_CAMPAIGN, move |log| {
        for (c, r) in &rows {
            log.append(KIND_GONE, c, r).map_err(|e| Error::RustError(format!("{e:?}")))?;
        }
        Ok(())
    })
    .await;
    if let Err(e) = wrote {
        crate::loud!(&place.ns, Some(&place.venue), "campaign.gone", "not filed: {e}");
    }
}
