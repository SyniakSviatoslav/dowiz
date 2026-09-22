//! WHO MAY BE SENT WHAT, AND THE PROOF THAT THEY SAID SO.
//!
//! Albania's Law 124/2024 (in force 31 January 2025, aligned with GDPR)
//! requires prior, explicit, informed consent before direct marketing by
//! electronic means, withdrawable at any time, and requires the controller to
//! be able to DEMONSTRATE it (Art. 7(1)) and to make withdrawal as easy as
//! giving (Art. 7(3)). Recital 32: a pre-ticked box is not consent. Meta's
//! WhatsApp policy adds a second, separate rule: an opt-in before any
//! business-initiated message, per business, naming it.
//!
//! SO THE RECORD IS THE ICO'S FOUR QUESTIONS -- who, when, how, and what you
//! told people -- and not a boolean. A `marketing_ok` column can answer none
//! of them, and the one a dispute asks is always the fourth.
//!
//! APPEND-ONLY, FOLDED TO CURRENT STATE, for the same reason the census is:
//! "evidence is not updated in place" (`witness/mod.rs`). A withdrawal is a
//! new record, never an edit, so the proof that the venue STOPPED survives --
//! and it must, because it is the only defence against the complaint.
//!
//! PURE. No clock and no I/O: the instant is the request's (`ctx.data.now_ms`,
//! the rule `tools/gates/clock.sh` counts), and the records arrive as
//! `logimage::Entry` from the venue's `consent` image.
//!
//! ── G1 ──
//! [`Consented`] is the WITNESS: its fields are private and it has no public
//! constructor, so the only way to hold one is [`state`] returning `Some`. A
//! send that takes a `&Consented` therefore cannot be written for a person who
//! did not say yes -- not "should not", cannot. The grep half, for the send
//! sites that take a bare string, is `tools/gates/consent.sh`.

pub mod wordings;
pub use wordings::{wording_id, wording_json, wording_of, LANGS, WORDINGS};

use crate::logimage::Entry;
use crate::minijson::{esc, int_field, str_field};

/// One consent act. The `w` records are the sentences those acts name.
pub const KIND_ACT: &str = "c";
pub const KIND_WORDING: &str = "w";

/// CONSENT IS PER PURPOSE. "Marketing" is one; a loyalty count would be
/// another, and an unknown purpose is refused rather than treated as marketing
/// -- a permission with a name nobody defined is a permission for anything.
pub const PURPOSE_MARKETING: &str = "marketing";
pub const PURPOSES: [&str; 1] = [PURPOSE_MARKETING];

/// AND PER CHANNEL, because Meta's opt-in is: a WhatsApp consent is not a
/// Telegram one, and this venue's customers reach it on three.
pub const CHANNEL_WHATSAPP: &str = "whatsapp";
pub const CHANNEL_TELEGRAM: &str = "telegram";
pub const CHANNEL_INSTAGRAM: &str = "instagram";
pub const CHANNELS: [&str; 3] = [CHANNEL_WHATSAPP, CHANNEL_TELEGRAM, CHANNEL_INSTAGRAM];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Given,
    Withdrawn,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Given => "given",
            State::Withdrawn => "withdrawn",
        }
    }
    pub fn of(s: &str) -> Option<State> {
        match s {
            "given" => Some(State::Given),
            "withdrawn" => Some(State::Withdrawn),
            _ => None,
        }
    }
}

/// HOW the venue came to believe it. Recital 32's "clear affirmative action",
/// made into the three acts this product can actually observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// The unticked box under the phone field on the checkout.
    CheckoutBox,
    /// The customer wrote START or STOP in a thread they opened.
    WhatsappKeyword,
    /// Paper or verbal, typed in by the owner. Needs `evidence`.
    OwnerEntered,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::CheckoutBox => "checkout_box",
            Method::WhatsappKeyword => "whatsapp_keyword",
            Method::OwnerEntered => "owner_entered",
        }
    }
    pub fn of(s: &str) -> Option<Method> {
        match s {
            "checkout_box" => Some(Method::CheckoutBox),
            "whatsapp_keyword" => Some(Method::WhatsappKeyword),
            "owner_entered" => Some(Method::OwnerEntered),
            _ => None,
        }
    }
}

/// One act, as it is written and as it is read back.
///
/// NO CONTACT DETAILS. `key` is the venue's pseudonymous handle
/// (`customer_key`, an HMAC over the digits); the number itself lives in the
/// order log, and a record of who may be messaged that also held the number
/// would double the exposure -- the call `Revealed` already made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Act {
    pub key: String,
    pub purpose: String,
    pub channel: String,
    pub state: State,
    pub at_ms: i64,
    pub method: Method,
    /// What the owner saw. Required for `OwnerEntered` grants.
    pub evidence: String,
    /// The hash of the sentence they read. Required for a grant.
    pub wording_id: String,
    /// The order id, thread key or owner the act arrived on.
    pub via: String,
}

impl Act {
    pub fn parse(json: &str) -> Option<Act> {
        Some(Act {
            key: str_field(json, "key")?,
            purpose: str_field(json, "purpose")?,
            channel: str_field(json, "channel")?,
            state: State::of(&str_field(json, "state")?)?,
            at_ms: int_field(json, "atMs")?,
            method: Method::of(&str_field(json, "method")?)?,
            evidence: str_field(json, "evidence").unwrap_or_default(),
            wording_id: str_field(json, "wordingId").unwrap_or_default(),
            via: str_field(json, "via").unwrap_or_default(),
        })
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"key\":\"{}\",\"purpose\":\"{}\",\"channel\":\"{}\",\"state\":\"{}\",\
             \"atMs\":{},\"method\":\"{}\",\"evidence\":\"{}\",\"wordingId\":\"{}\",\"via\":\"{}\"}}",
            esc(&self.key),
            esc(&self.purpose),
            esc(&self.channel),
            self.state.as_str(),
            self.at_ms,
            self.method.as_str(),
            esc(&self.evidence),
            esc(&self.wording_id),
            esc(&self.via),
        )
    }
}

/// THE PROOF THAT A PERSON SAID YES, and the only way to hold one is [`state`].
///
/// The fields are private and there is no constructor. A function that takes
/// one cannot be called for somebody who never consented, and no amount of
/// hurry at 23:00 on a Friday can produce one -- which is the difference
/// between a rule and a type. See the module header, G1.
pub struct Consented {
    key: String,
    purpose: String,
    channel: String,
    at_ms: i64,
    wording_id: String,
}

impl Consented {
    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn purpose(&self) -> &str {
        &self.purpose
    }
    pub fn channel(&self) -> &str {
        &self.channel
    }
    /// When they said yes. A send has to be able to say how old the permission
    /// is; nothing here expires it, because the law does not.
    pub fn at_ms(&self) -> i64 {
        self.at_ms
    }
    /// Which sentence they read.
    pub fn wording_id(&self) -> &str {
        &self.wording_id
    }
}

/// The subject an act is filed under, so one person's consent is one prefix
/// read. The same spelling the reveal audit uses.
pub fn subject_of(key: &str) -> String {
    format!("cust:{key}")
}

/// Why an act would be refused. ONE FUNCTION, applied by the writer AND by the
/// fold: the log is append-only, so a record written by an older build or by a
/// hand cannot be taken back, and a reader that trusted what a writer would
/// have refused is a reader that can be handed consent by anyone who can write
/// the image.
pub fn check(act: &Act) -> Result<(), String> {
    if act.key.trim().is_empty() || act.key.contains(char::is_whitespace) {
        return Err("a consent act needs the customer's handle".into());
    }
    if !PURPOSES.contains(&act.purpose.as_str()) {
        return Err(format!("unknown purpose {:?}", act.purpose));
    }
    if !CHANNELS.contains(&act.channel.as_str()) {
        return Err(format!("unknown channel {:?}", act.channel));
    }
    // THE INSTANT IS THE ONE COMPARISON THIS LOG EXISTS TO MAKE -- which came
    // last, the grant or the withdrawal. A record with no clock cannot be
    // ordered against anything.
    if act.at_ms <= 0 {
        return Err("a consent act needs the instant it happened".into());
    }
    if act.state == State::Withdrawn {
        // ART. 7(3): AS EASY AS GIVING. A withdrawal that had to reproduce the
        // sentence, or name evidence, would be harder than the box that
        // started it -- so nothing below applies to one.
        return Ok(());
    }
    if act.wording_id.trim().is_empty() {
        return Err("a grant must name the wording the person read".into());
    }
    if act.method == Method::OwnerEntered && act.evidence.trim().is_empty() {
        return Err("owner-entered consent needs evidence: what did you see?".into());
    }
    Ok(())
}

/// THE FOLD. The newest act for this person, purpose and channel decides.
///
/// NEWEST BY THE ACT'S OWN CLOCK, not by position: `entries()` is newest-first
/// by where a record landed, and two writes racing through a retry would
/// otherwise be settled by whichever won the guard. On a tie the WITHDRAWAL
/// wins -- the only safe direction for a rule whose failure sends a stranger a
/// message they refused.
pub fn state(entries: &[Entry], key: &str, purpose: &str, channel: &str) -> Option<Consented> {
    let mut best: Option<Act> = None;
    for e in entries.iter().filter(|e| e.kind == KIND_ACT) {
        let Some(act) = Act::parse(&e.json) else { continue };
        if act.key != key || act.purpose != purpose || act.channel != channel {
            continue;
        }
        if check(&act).is_err() {
            continue;
        }
        let takes = match &best {
            None => true,
            Some(b) => {
                act.at_ms > b.at_ms
                    || (act.at_ms == b.at_ms && act.state == State::Withdrawn)
            }
        };
        if takes {
            best = Some(act);
        }
    }
    let act = best?;
    if act.state != State::Given {
        return None;
    }
    Some(Consented {
        key: act.key,
        purpose: act.purpose,
        channel: act.channel,
        at_ms: act.at_ms,
        wording_id: act.wording_id,
    })
}

#[cfg(test)]
mod tests;
