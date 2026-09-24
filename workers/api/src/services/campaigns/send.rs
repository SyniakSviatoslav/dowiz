//! A CAMPAIGN BECOMES OUTBOX ENTRIES — one per person, each minted only
//! against a `Consented` (§5 G1), and re-checked against the consent fold at
//! the moment the drain would deliver it (§5 G4).
//!
//! THE OUTBOX IS REUSED UNCHANGED: its backoff, its six tries, its
//! `Verdict::Abandon`, its health pane. A campaign entry is an `outbox::Entry`
//! of kind `campaign` whose id is `camp:<campaign>:<key>` -- so a retried
//! send is ONE entry while it waits (`outbox.rs`'s own idempotency), and the
//! `sent` rows in the campaign log make it one message after it has gone.
//!
//! THE MESSAGE IS RENDERED AT ENQUEUE TIME, as the approved TEMPLATE's
//! request object (`template.rs`): a campaign is business-initiated, and
//! WhatsApp carries nothing else outside the 24-hour window. Free text with a
//! STOP line appended was what this queued before, and Meta would have refused
//! every one of them. The way out is now the template's own words (the
//! consent sentence promised "reply STOP"), and `stop.rs` honours the reply.
//!
//! ONE PRESS, TWO IMAGES, AND A RETRY THAT CANNOT DUPLICATE. The outbox write
//! puts each entry AND a mark (`MARK`, same id) in ONE table write; the mark
//! outlives the entry's delivery. The campaign log's `sent` rows follow. If the
//! second write fails, the next press plans the same people again (their `sent`
//! rows are missing) and `admit` finds their marks: nothing is queued twice,
//! even for somebody the drain already delivered to in between -- which is
//! the case a put-if-absent on the entry alone let through. Once the rows are
//! filed, `prune` drops the marks, so they do not squat the outbox budget.
//!
//! BOUNDED: a send queues at most what fits in HALF the outbox's bytes, with
//! what is already waiting counted. Order notices share the same image, and a
//! campaign must never be the reason a kitchen is not told about an order.
//! What does not fit is `left`; pressing send again continues where it
//! stopped, because the people already queued are skipped.

use std::collections::BTreeSet;

use dowiz_hub::consent::{self, Consented, PURPOSE_MARKETING};
use dowiz_hub::logimage::Entry as LogEntry;

use dowiz_hub::logimage::LogImage;
use dowiz_hub::table::Table;

use super::audience::Recipient;
use super::campaign::{sent_row, sent_to, Def, CHANNEL, KIND_SENT};
use super::template::{self, Template};
use crate::outbox::{Entry, KIND as OUTBOX_ENTRY, OUTBOX_BYTES};

/// The outbox kind the drain routes to `rail::deliver`.
pub const OUTBOX_KIND: &str = "campaign";
const ID_PREFIX: &str = "camp:";
/// The share of the outbox one campaign may fill.
pub const BUDGET_BYTES: usize = OUTBOX_BYTES / 2;

/// The way out, in the person's language. The same promise the consent
/// wording made (`consent::WORDINGS`: "...by replying STOP").
pub fn stop_line(lang: &str) -> &'static str {
    match lang {
        "en" => "Reply STOP to stop these messages.",
        "uk" => "Відповідайте STOP, щоб більше не отримувати ці повідомлення.",
        _ => "Përgjigjuni me STOP për të mos marrë më këto mesazhe.",
    }
}

pub fn entry_id(campaign: &str, key: &str) -> String {
    format!("{ID_PREFIX}{campaign}:{key}")
}

/// `camp:<campaign>:<key>` → (campaign, key).
pub fn parse_id(id: &str) -> Option<(&str, &str)> {
    let rest = id.strip_prefix(ID_PREFIX)?;
    let (c, k) = rest.split_once(':')?;
    (!c.is_empty() && !k.is_empty()).then_some((c, k))
}

/// G1. THE ONLY WAY A CAMPAIGN MESSAGE IS MADE, and it needs the witness.
/// The entry is addressed under the key that CONSENTED (`who.key()`), which
/// is the key the drain re-asks the fold about.
/// `text` is the template's request object, as JSON.
pub fn entry(def: &Def, tpl: &Template, who: &Consented, to: &str, now_ms: i64) -> Entry {
    let text = template::object(tpl).to_string();
    Entry::new(entry_id(&def.id, who.key()), OUTBOX_KIND, to.to_string(), text, now_ms)
}

/// What one press of "send" does.
#[derive(Debug, Default)]
pub struct Plan {
    /// (canonical key, outbox entry, `sent` row), in recipient order.
    pub queue: Vec<(String, Entry, String)>,
    /// Recipients skipped because this campaign already reached them.
    pub already: usize,
    /// Recipients that did not fit the budget this time.
    pub left: usize,
}

fn bytes_of(e: &Entry) -> usize {
    serde_json::to_string(e).map(|s| s.len()).unwrap_or(usize::MAX / 4)
}

/// PURE. `already` = `campaign::sent_to`; `waiting` = the outbox as it is.
/// `Err` = the campaign cannot be sent at all (no approved template), and
/// nothing is planned.
pub fn plan(def: &Def, recipients: &[Recipient], already: &BTreeSet<String>, waiting: &[Entry], now_ms: i64) -> Result<Plan, String> {
    let tpl = template::required(def)?;
    let mut used: usize = waiting.iter().map(bytes_of).sum();
    let mut p = Plan::default();
    for r in recipients {
        if already.contains(&r.key) {
            p.already += 1;
            continue;
        }
        let e = entry(def, tpl, &r.witness, &r.to, now_ms);
        let n = bytes_of(&e);
        if used + n > BUDGET_BYTES {
            p.left += 1;
            continue;
        }
        used += n;
        let row = sent_row(&def.id, &r.key, r.witness.key(), &e.id, r.witness.at_ms(), r.witness.wording_id(), now_ms);
        p.queue.push((r.key.clone(), e, row));
    }
    Ok(p)
}

/// The outbox record kind that says "this campaign entry was queued once".
pub const MARK: &str = "cm";

/// WRITE 1, inside the outbox's own write. Each planned entry is put with its
/// mark unless it is waiting or marked already. Returns (newly queued, keys
/// that are now marked -- new or from an earlier press).
pub fn admit(t: &mut Table, queue: &[(String, Entry, String)]) -> Result<(usize, Vec<String>), String> {
    let (mut new, mut marked) = (0usize, Vec::new());
    for (k, e, _) in queue {
        if !t.has(OUTBOX_ENTRY, &e.id) && !t.has(MARK, &e.id) {
            let rec = serde_json::to_string(e).map_err(|x| x.to_string())?;
            t.put(OUTBOX_ENTRY, &e.id, &rec, &[], &[]).map_err(|x| format!("outbox: {x:?}"))?;
            new += 1;
        }
        t.put(MARK, &e.id, "1", &[], &[]).map_err(|x| format!("outbox: {x:?}"))?;
        marked.push(k.clone());
    }
    Ok((new, marked))
}

/// WRITE 2, the history: one `sent` row per marked key the log lacks.
pub fn file_history(log: &mut LogImage, campaign: &str, queue: &[(String, Entry, String)], marked: &[String]) -> Result<usize, String> {
    let have = sent_to(&log.entries(), campaign);
    let mut n = 0usize;
    for (k, _, row) in queue.iter().filter(|(k, _, _)| marked.contains(k) && !have.contains(k)) {
        log.append(KIND_SENT, k, row).map_err(|e| format!("{e:?}"))?;
        n += 1;
    }
    Ok(n)
}

/// WRITE 3, best effort: drop this campaign's marks whose `sent` row is
/// filed (`filed` = `campaign::sent_entries`, the rows' entry ids). A mark
/// left behind because this write failed is dropped by the next press.
pub fn prune(t: &mut Table, campaign: &str, filed: &BTreeSet<String>) -> usize {
    let ids: Vec<String> = t
        .all(MARK)
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| parse_id(id).map(|(c, _)| c) == Some(campaign) && filed.contains(id))
        .collect();
    ids.iter().filter(|id| t.remove(MARK, id)).count()
}

/// G4, AT THE DRAIN: the witness for a waiting campaign entry, asked of the
/// consent fold AS IT IS NOW. `None` -- the person withdrew after the entry
/// was queued, or the id is not a campaign's -- and the entry must not go.
pub fn gate(e: &Entry, acts: &[LogEntry]) -> Option<Consented> {
    if e.kind != OUTBOX_KIND {
        return None;
    }
    let (_, key) = parse_id(&e.id)?;
    consent::state(acts, key, PURPOSE_MARKETING, CHANNEL)
}

/// Whether any entry the drain is about to try is a campaign's -- the consent
/// image is read only then.
pub fn any_campaign<'a>(mut due: impl Iterator<Item = &'a Entry>) -> bool {
    due.any(|e| e.kind == OUTBOX_KIND)
}

#[cfg(test)]
mod tests;
