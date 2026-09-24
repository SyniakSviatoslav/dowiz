//! THE MESSAGE META WILL CARRY: an APPROVED TEMPLATE, never free text.
//!
//! WhatsApp delivers free text only inside the 24-hour window a customer's own
//! message opens. A campaign is business-initiated -- it goes to people who
//! last wrote days or never -- so the Cloud API refuses a `type: "text"` send
//! to them. "Template messages are the only type of message that can be sent
//! to WhatsApp users outside of a customer service window"
//! (https://developers.facebook.com/documentation/business-messaging/whatsapp/templates/overview,
//! read 2026-09-24). The owner registers the template in Meta's WhatsApp
//! Manager, Meta approves it, and the campaign NAMES it here: its name, its
//! language, and the values of its body's positional `{{1}}..{{n}}`.
//!
//! THE REQUEST SHAPE, from the same page:
//!
//! ```text
//! { "messaging_product": "whatsapp", "recipient_type": "individual", "to": "+1650...",
//!   "type": "template",
//!   "template": { "name": "order_confirmation", "language": { "code": "en_US" },
//!                 "components": [ { "type": "body", "parameters": [
//!                     { "type": "text", "text": "Jessica" }, ... ] } ] } }
//! ```
//!
//! `object` builds the `template` member; `channels::template_body` wraps it.
//! The object is RENDERED AT ENQUEUE TIME into the outbox entry's `text`, for
//! the outbox's own reason: the drain must not re-read a campaign that changed.
//!
//! A CAMPAIGN WITHOUT ONE IS REFUSED AT PREVIEW AND AT SEND (`required`), and
//! never queued: a queued free-text campaign would be six refused attempts per
//! person and then an `abandoned` count, which is a queue pretending to work.
//!
//! PURE.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::campaign::{Def, CHANNEL};

/// Meta: "limited to a maximum of 512 characters, consisting of lowercase
/// alphanumeric characters and underscores".
pub const NAME_MAX: usize = 512;
/// Body parameters a campaign may fill. A marketing body with more than a few
/// variables is a form letter; ten is generous.
pub const PARAMS_MAX: usize = 10;
/// One parameter's length. Meta's body is 1024 characters in all.
pub const PARAM_MAX: usize = 200;

/// The template the owner registered in Meta. A CLOSED shape.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// As registered: `autumn_soup`.
    pub name: String,
    /// The approved translation's code: `sq`, `en`, `en_US`, `uk`.
    pub lang: String,
    /// The body's `{{1}}..{{n}}`, in order. Empty for a body with none.
    #[serde(default)]
    pub params: Vec<String>,
}

fn lang_ok(l: &str) -> bool {
    let (base, region) = match l.split_once('_') {
        Some((b, r)) => (b, Some(r)),
        None => (l, None),
    };
    (2..=3).contains(&base.len())
        && base.bytes().all(|b| b.is_ascii_lowercase())
        && region.map_or(true, |r| r.len() == 2 && r.bytes().all(|b| b.is_ascii_uppercase()))
}

/// The refusal, by name, or `Ok`.
pub fn check(t: &Template) -> Result<(), String> {
    let n = &t.name;
    if n.is_empty() || n.len() > NAME_MAX || !n.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return Err(format!("{n:?} is not a WhatsApp template name: lowercase letters, digits and _ as registered in Meta"));
    }
    if !lang_ok(&t.lang) {
        return Err(format!("{:?} is not a template language code (sq, en, en_US, uk)", t.lang));
    }
    if t.params.len() > PARAMS_MAX {
        return Err(format!("a template takes at most {PARAMS_MAX} values"));
    }
    for (i, p) in t.params.iter().enumerate() {
        // META REFUSES a parameter holding a newline or a tab, or more than
        // four spaces in a row; it is refused here, where the owner can fix
        // it, not six drains later as an `abandoned` count.
        let bad = p.trim().is_empty() || p.chars().count() > PARAM_MAX || p.contains(['\n', '\t', '\r']) || p.contains("     ");
        if bad {
            return Err(format!("value {{{{{}}}}} must be 1 to {PARAM_MAX} characters on one line", i + 1));
        }
    }
    Ok(())
}

/// The `template` member of the Cloud API request.
pub fn object(t: &Template) -> Value {
    let mut o = json!({ "name": t.name, "language": { "code": t.lang } });
    if !t.params.is_empty() {
        let ps: Vec<Value> = t.params.iter().map(|p| json!({ "type": "text", "text": p })).collect();
        o["components"] = json!([{ "type": "body", "parameters": ps }]);
    }
    o
}

/// PREVIEW AND SEND ASK THIS FIRST. A WhatsApp campaign without a template
/// is refused with the reason and the fix; nothing is counted as queued.
pub fn required(def: &Def) -> Result<&Template, String> {
    match (&def.template, def.channel.as_str()) {
        (Some(t), _) => check(t).map(|()| t),
        (None, CHANNEL) => Err("WhatsApp delivers a campaign only as a template Meta has approved \
             (free text reaches only people who wrote to you in the last 24 hours). \
             Register the message in WhatsApp Manager, then name the template and its language here."
            .into()),
        (None, other) => Err(format!("channel {other:?} cannot send campaigns")),
    }
}

#[cfg(test)]
mod tests;
