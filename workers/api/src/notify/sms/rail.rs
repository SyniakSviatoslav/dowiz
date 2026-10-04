//! THE SEND: one outbox entry of kind `sms`, inside the venue's drain
//! (`outbox/rails.rs`), never on a customer's request.
//!
//! IN THIS ORDER, for every entry:
//!   1. the venue's configuration (`config::of`), read once a drain: switched
//!      OFF = the entry is dropped (the owner turned texts off); switched on
//!      but INCOMPLETE = it waits, said once, no try spent (as a missing
//!      Telegram token waits);
//!   2. AGE: a text queued more than `STALE_MS` ago is dropped -- "confirmed"
//!      two hours late is noise;
//!   3. CONSENT, RE-ASKED: the venue's consent fold for this customer's key,
//!      `order_status` on `sms`. A STOP filed after the order wins: dropped,
//!      never sent. Only a `Consented` reaches `gateway::sms_text`;
//!   4. THE DAILY BUDGET (`notify.sms.daily`, default 60): over it, dropped and
//!      said -- a consumer SIM that sends bulk is a SIM the carrier blocks;
//!   5. the send, classified (`gateway::classify`).
//!
//! THE HEALTH RECORD (`HEALTH_KIND`/`HEALTH_ID` in the outbox image): today's
//! count, the last success and the last failure with its cause. Written in the
//! drain's ONE image write; read by `GET /api/owner/sms` and the health pane.
//! The day is the UTC day of the drain's clock (`now_ms / 86 400 000`).

use serde::{Deserialize, Serialize};

use super::config::{self, Cfg};
use super::gateway::{self, Outcome};
use super::plan::Payload;
use crate::outbox::{Entry, Verdict};

/// The outbox record kind and id of the venue's SMS health.
pub const HEALTH_KIND: &str = "sms_h";
pub const HEALTH_ID: &str = "venue";
/// Older than this, a status text is not sent.
pub const STALE_MS: i64 = 2 * 3_600_000;
const DAY_MS: i64 = 86_400_000;

/// The last failure, as the console shows it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Failure {
    pub at_ms: i64,
    pub code: u16,
    /// A console key (`gateway::why`).
    pub why: String,
}

/// The venue's SMS health.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Health {
    pub day: i64,
    pub sent: u32,
    #[serde(default)]
    pub over_budget: u32,
    #[serde(default)]
    pub stopped: u32,
    #[serde(default)]
    pub last_ok_ms: i64,
    #[serde(default)]
    pub last_err: Option<Failure>,
}

impl Health {
    /// Read from the outbox image's record, rolled to today.
    pub fn of(rec: Option<&str>, now_ms: i64) -> Health {
        let mut h: Health = rec.and_then(|r| serde_json::from_str(r).ok()).unwrap_or_default();
        let today = now_ms.div_euclid(DAY_MS);
        if h.day != today {
            h = Health { day: today, last_ok_ms: h.last_ok_ms, last_err: h.last_err, ..Health::default() };
        }
        h
    }
}

/// What one entry comes to, before the send. PURE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    Wait,
    Drop(Option<String>),
    Send(Payload),
}

/// Steps 1, 2 and 4 (and the payload's shape); consent is step 3, in `send`.
pub fn gate(cfg: &Result<Cfg, &'static str>, e: &Entry, h: &Health, now_ms: i64) -> Gate {
    let cfg = match cfg {
        Ok(c) => c,
        Err("sms_off") => return Gate::Drop(None),
        Err(_) => return Gate::Wait,
    };
    if now_ms - e.queued_at_ms > STALE_MS {
        return Gate::Drop(Some(format!("{}: not texted, queued more than two hours ago", e.id)));
    }
    let Ok(p) = serde_json::from_str::<Payload>(&e.text) else {
        return Gate::Drop(Some(format!("{}: unreadable sms entry", e.id)));
    };
    if h.sent >= cfg.daily {
        return Gate::Drop(Some(format!("{}: not texted, the venue's daily SMS limit ({}) is reached", e.id, cfg.daily)));
    }
    Gate::Send(p)
}

/// One drain's SMS state.
#[derive(Default)]
pub struct Rail {
    cfg: Option<Result<Cfg, &'static str>>,
    /// The consent log's entries; `None` inside = unreadable (entries wait).
    acts: Option<Option<Vec<dowiz_hub::logimage::Entry>>>,
    health: Option<Health>,
    pub said: Vec<String>,
}

impl Rail {
    /// Attempt one entry. `Some(ok)` = attempted (the drain applies the
    /// backoff); `None` = waited, or dropped (its verdict pushed here).
    pub async fn send(
        &mut self,
        place: &crate::hubstore::Place,
        settings: &dowiz_hub::settings::Settings,
        image: &dowiz_hub::table::Table,
        e: &Entry,
        now_ms: i64,
        verdicts: &mut Vec<(String, Verdict)>,
    ) -> Option<bool> {
        if self.cfg.is_none() {
            let c = config::of(settings);
            if let Err(w) = c.filter_missing() {
                self.said.push(format!("sms: switched on but not set up ({w}); texts wait"));
            }
            self.cfg = Some(c);
        }
        let h = self.health.get_or_insert_with(|| Health::of(image.get(HEALTH_KIND, HEALTH_ID).as_deref(), now_ms));
        let cfg = self.cfg.as_ref().expect("set above");
        let p = match gate(cfg, e, h, now_ms) {
            Gate::Wait => return None,
            Gate::Drop(why) => {
                if why.as_deref().is_some_and(|w| w.contains("daily SMS limit")) {
                    h.over_budget += 1;
                }
                self.said.extend(why);
                verdicts.push((e.id.clone(), Verdict::Abandon { after: 0 }));
                return None;
            }
            Gate::Send(p) => p,
        };
        if self.acts.is_none() {
            let image = crate::services::customers::consent_log::IMAGE_CONSENT;
            self.acts = Some(match crate::hubstore::load_log(place, image).await {
                Ok(l) => Some(l.log.entries()),
                Err(err) => {
                    self.said.push(format!("sms: the consent image is unreadable, texts wait: {err}"));
                    None
                }
            });
        }
        let Some(Some(acts)) = &self.acts else { return None };
        use dowiz_hub::consent::{state, CHANNEL_SMS, PURPOSE_ORDER_STATUS};
        let Some(who): Option<dowiz_hub::consent::Consented> = state(acts, &p.key, PURPOSE_ORDER_STATUS, CHANNEL_SMS) else {
            // WITHDRAWN SINCE IT WAS QUEUED (or never filed): never sent.
            let h = self.health.as_mut().expect("set above");
            h.stopped += 1;
            verdicts.push((e.id.clone(), Verdict::Abandon { after: 0 }));
            return None;
        };
        let Ok(c) = cfg else { return None };
        let (outcome, code) = gateway::sms_text(c, &who, &e.to, &p.body, &e.id).await;
        let h = self.health.as_mut().expect("set above");
        match outcome {
            Outcome::Sent => {
                h.sent += 1;
                h.last_ok_ms = now_ms;
                Some(true)
            }
            Outcome::Refused => {
                h.last_err = Some(Failure { at_ms: now_ms, code, why: gateway::why(code).into() });
                self.said.push(format!("{}: the SMS provider refused this text for good ({code})", e.id));
                verdicts.push((e.id.clone(), Verdict::Abandon { after: 0 }));
                None
            }
            Outcome::Failed => {
                h.last_err = Some(Failure { at_ms: now_ms, code, why: gateway::why(code).into() });
                Some(false)
            }
        }
    }

    /// After the loop: what to say, and the health record for the drain's write.
    pub fn finish(&mut self, records: &mut Vec<(&'static str, String, Option<String>)>, said: &mut Vec<String>) {
        said.append(&mut self.said);
        if let Some(h) = &self.health {
            records.push((HEALTH_KIND, HEALTH_ID.to_string(), Some(serde_json::to_string(h).unwrap_or_default())));
        }
    }
}

/// `Err` for a configuration that is on but incomplete; `Ok` otherwise.
trait Missing {
    fn filter_missing(&self) -> Result<(), &'static str>;
}

impl Missing for Result<Cfg, &'static str> {
    fn filter_missing(&self) -> Result<(), &'static str> {
        match self {
            Err(w) if *w != "sms_off" => Err(w),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
