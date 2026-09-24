//! "STOP" IN A THREAD IS A WITHDRAWAL (§3.2 of BLUEPRINT-CRM-CONSENT-LOYALTY-
//! 2026-09-22): "the word STOP in a WhatsApp thread (`channels.rs`'s inbound
//! path, one match) ... a STOP that reaches the inbox and is not folded within
//! the minute is a defect, not a delay." Every consent sentence the checkout
//! shows ends "...by replying STOP", in all three languages.
//!
//! THE ONE SITE is `channels::webhook`, which calls `heard` for each fresh
//! inbound message. `heard` does NOTHING unless the text is a stop word, so an
//! ordinary message costs no read. A stop files one `withdrawn` act per key
//! the sender is known under, through `consent_log::file` -- the path the
//! owner's withdrawal route uses -- and the drain's re-check (`send::gate`,
//! G4) then drops any campaign entry still waiting for them.
//!
//! WHO THE SENDER IS. WhatsApp's `from` is the E.164 number without `+`. The
//! consent was filed under the `customer_key` of the phone AS TYPED at the
//! checkout, which may be the national spelling (`069…`), so the stop covers
//! every spelling of the number and every key linked to them (`Aliases`).
//! Instagram and Telegram senders are platform ids, not phones: their stop is
//! filed for that id on that channel, which is all the venue can resolve.
//!
//! WHOLE MESSAGE ONLY. "don't stop the soup" is not a withdrawal; "Stop.",
//! " stop ", "СТОП!" are. Over-matching a sentence would withdraw people who
//! never asked; the keyword is what the sentence told them to send.
//!
//! PURE except `heard`.

use dowiz_hub::consent::{Act, Method, State, CHANNEL_INSTAGRAM, CHANNEL_TELEGRAM, CHANNEL_WHATSAPP, PURPOSE_MARKETING};

use crate::services::customers::identity::{canonical_digits, VENUE_DIAL};

/// Upper-cased. `STOP` is the word every consent sentence names; `NDALO` is
/// Albanian, `СТОП` Ukrainian, `СТОР` is "STOP" typed on a Cyrillic layout
/// (С Т О Р look like S T O P), `STOP PROMOTIONS` is Meta's own opt-out
/// button on a marketing template, which arrives as a `button` message.
pub const WORDS: [&str; 6] = ["STOP", "NDALO", "СТОП", "СТОР", "STOP PROMOTIONS", "UNSUBSCRIBE"];

/// Whether a whole inbound message is a stop word.
pub fn is_stop(text: &str) -> bool {
    let t = text.trim_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation() || c == '¡' || c == '¿');
    let words: Vec<String> = t.split_whitespace().map(str::to_uppercase).collect();
    let t = words.join(" ");
    !t.is_empty() && WORDS.contains(&t.as_str())
}

/// The consent channel a thread's channel files under; `None` for any other.
pub fn channel_of(thread: &str) -> Option<&'static str> {
    match thread {
        "whatsapp" => Some(CHANNEL_WHATSAPP),
        "instagram" => Some(CHANNEL_INSTAGRAM),
        "telegram" => Some(CHANNEL_TELEGRAM),
        _ => None,
    }
}

/// Every way the sender's number may have been typed at a checkout. A
/// WhatsApp `from` in the venue's country is also `0` + the national number
/// and, for a mobile, the national number bare -- the spellings
/// `canonical_digits` folds into it. Any other sender is its own id.
pub fn spellings(thread: &str, peer: &str) -> Vec<String> {
    let mut out = vec![peer.to_string()];
    if thread == "whatsapp" {
        if let Some(nsn) = canonical_digits(peer, VENUE_DIAL).as_deref().and_then(|c| c.strip_prefix(VENUE_DIAL)) {
            out.push(format!("0{nsn}"));
            if nsn.starts_with('6') {
                out.push(nsn.to_string());
            }
        }
    }
    out
}

/// PURE. The withdrawals one inbound message files: none unless it is a stop
/// word on a known channel. `key_of` is `customer_key` under the venue's
/// secret; `circle` is `Aliases::circle` (the key, its canonical row and every
/// key linked into it). `via` is the message's own id, for the audit.
pub fn acts_for(
    thread: &str,
    peer: &str,
    text: &str,
    via: &str,
    at_ms: i64,
    key_of: impl Fn(&str) -> String,
    circle: impl Fn(&str) -> Vec<String>,
) -> Vec<Act> {
    let Some(channel) = channel_of(thread).filter(|_| is_stop(text) && !peer.trim().is_empty()) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = spellings(thread, peer).iter().flat_map(|s| circle(&key_of(s))).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .map(|key| Act {
            key,
            purpose: PURPOSE_MARKETING.into(),
            channel: channel.into(),
            state: State::Withdrawn,
            at_ms,
            // The method is the keyword, whichever thread carried it.
            method: Method::WhatsappKeyword,
            evidence: text.trim().chars().take(40).collect(),
            wording_id: String::new(),
            via: via.to_string(),
        })
        .filter(|a| dowiz_hub::consent::check(a).is_ok())
        .collect()
}

/// THE HOOK `channels::webhook` calls for each fresh inbound message.
/// A failure is loud in `consent_log::file`; the webhook still answers 200.
pub async fn heard(place: &crate::hubstore::Place, env: &worker::Env, thread: &str, peer: &str, text: &str, via: &str, at_ms: i64) {
    if !is_stop(text) {
        return;
    }
    use crate::services::customers::alias::Aliases;
    let aliases = match crate::hubstore::load_table(place, crate::hubstore::IMAGE_PEOPLE, crate::hubstore::PEOPLE_BYTES).await {
        Ok(p) => Aliases::of(&p.table),
        Err(e) => {
            // WITHOUT THE ALIASES the stop still covers every spelling of the
            // number; a linked second number is what is missed, and said.
            crate::loud!(&place.ns, Some(&place.venue), "consent.stop", "aliases unreadable, stop filed without them: {e}");
            Aliases::default()
        }
    };
    let secret = crate::services::customers::handlers::signing_secret(env);
    let key_of = |p: &str| crate::services::customers::handlers::customer_key(&secret, p);
    for act in acts_for(thread, peer, text, via, at_ms, key_of, |k| aliases.circle(k, None)) {
        let _ = crate::services::customers::consent_log::file(place, &act).await;
    }
}

#[cfg(test)]
mod tests;
