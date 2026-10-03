//! WHATSAPP DELIVERY STATES (W-INT2 #9): Meta's `value.statuses[]` -- sent, delivered, read,
//! failed -- recorded against the message they answer, and shown where that message is.
//!
//! Before this, every status Meta posted to `/api/webhooks/meta` was signed, checked and thrown
//! away: the webhook read `value.messages[]` only, so an owner who replied on WhatsApp could not
//! tell a message the customer read from one Meta never delivered.
//!
//! WHAT IS RECORDED. A status is kept only when its `wamid` is a message this venue SENT and
//! stored (an owner's reply in the inbox, `direction = out`); anything else -- a campaign's or a
//! bell's message, which is not in the inbox, or a status for nobody -- is counted and dropped,
//! so the inbox log grows with the conversation, not with Meta's receipts. And only a state
//! that moves FORWARD is appended (sent < delivered < read; failed is final): Meta repeats and
//! reorders these, and the log must not grow with the repeats.

use dowiz_hub::logimage::{Entry, LogImage};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The inbox log's kind for a delivery state; the subject is the conversation's (`conv`).
pub(super) const K_STATUS: &str = "s";

/// One status from one delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Status {
    pub peer: String,
    pub wamid: String,
    pub state: &'static str,
    pub at_ms: i64,
}

/// The four states Meta sends, in the order they can follow each other; `None` = not one.
fn state_of(s: &str) -> Option<(&'static str, u8)> {
    Some(match s {
        "sent" => ("sent", 1),
        "delivered" => ("delivered", 2),
        "read" => ("read", 3),
        "failed" => ("failed", 4),
        _ => return None,
    })
}

fn rank(s: &str) -> u8 {
    state_of(s).map_or(0, |(_, r)| r)
}

/// Every WhatsApp status in one delivery (`entry[].changes[].value.statuses[]`).
pub(super) fn statuses_of(body: &Value, now_ms: i64) -> Vec<Status> {
    let mut out = Vec::new();
    for entry in body["entry"].as_array().into_iter().flatten() {
        for change in entry["changes"].as_array().into_iter().flatten() {
            let v = &change["value"];
            if v["messaging_product"].as_str() != Some("whatsapp") {
                continue;
            }
            for s in v["statuses"].as_array().into_iter().flatten() {
                let (Some(wamid), Some(peer), Some((state, _))) =
                    (s["id"].as_str(), s["recipient_id"].as_str(), s["status"].as_str().and_then(state_of))
                else {
                    continue;
                };
                let at_ms = s["timestamp"].as_str().and_then(|t| t.parse::<i64>().ok()).map(|t| t * 1000).unwrap_or(now_ms);
                out.push(Status { peer: peer.to_string(), wamid: wamid.to_string(), state, at_ms });
            }
        }
    }
    out
}

/// The furthest state of each message, from a conversation's status records.
pub(super) fn latest(records: &[Entry]) -> HashMap<String, String> {
    let mut best: HashMap<String, String> = HashMap::new();
    for e in records {
        let Ok(v) = serde_json::from_str::<Value>(&e.json) else { continue };
        let (Some(w), Some(s)) = (v["wamid"].as_str(), v["state"].as_str()) else { continue };
        if rank(s) > best.get(w).map_or(0, |b| rank(b)) {
            best.insert(w.to_string(), s.to_string());
        }
    }
    best
}

/// Append each status that answers a message this venue sent and moves it forward. Returns
/// how many were recorded. Runs inside one `with_log` turn.
pub(super) fn record(log: &mut LogImage, list: &[Status], conv: impl Fn(&str) -> String, k_msg: &str) -> Result<usize, String> {
    let mut n = 0;
    for s in list {
        let subject = conv(&s.peer);
        let ours = log.about(k_msg, Some(&subject), usize::MAX).iter().any(|e| {
            serde_json::from_str::<Value>(&e.json).ok().is_some_and(|v| {
                v["direction"].as_str() == Some("out") && v["externalId"].as_str() == Some(s.wamid.as_str())
            })
        });
        if !ours {
            continue;
        }
        let now = latest(&log.about(K_STATUS, Some(&subject), usize::MAX));
        if rank(s.state) <= now.get(&s.wamid).map_or(0, |b| rank(b)) {
            continue;
        }
        let rec = json!({ "wamid": s.wamid, "state": s.state, "atMs": s.at_ms }).to_string();
        log.append(K_STATUS, &subject, &rec).map_err(|e| format!("inbox status: {e:?}"))?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests;
