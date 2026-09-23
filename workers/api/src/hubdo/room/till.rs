//! THE TILL'S COMMANDS, in the venue's object: read the till log and the
//! order log, decide (`command::till::decide`, pure), append one record,
//! write ONE image. The order log is only read here — a till command never
//! writes it, and `pay` never writes the till: two images, each with one
//! writer per command (`tools/gates/one-image.sh`).

use super::super::{HubImages, CATALOG_IMAGE};
use crate::command::till::{self, CashIn, Cmd, Period, Report, TillOut, IMAGE_TILL};
use crate::command::Refused;
use dowiz_hub::logimage::LogImage;
use serde::Deserialize;
use serde_json::Value;
use worker::*;

/// `/fold/room/till_report` — which venue's orders to fold the cash from.
#[derive(Deserialize)]
pub(super) struct ReportIn {
    location_id: String,
}

type Folded = std::result::Result<(i64, LogImage, Vec<Period>), Refused>;

impl HubImages {
    /// The till log, its generation, and its periods. A log that does not
    /// fold is a refusal naming the record, never an empty drawer.
    pub(super) async fn till_state(&self) -> Result<Folded> {
        let (gen, log) = match self.image(IMAGE_TILL).await? {
            Some((meta, bytes)) => (
                meta.generation,
                LogImage::load(&bytes).map_err(|_| Error::RustError("till image is unreadable".into()))?,
            ),
            None => (
                0,
                LogImage::create_sized(dowiz_hub::logimage::DEFAULT_LOG_BYTES)
                    .map_err(|_| Error::RustError("cannot create till image".into()))?,
            ),
        };
        Ok(match till::periods(&log.entries()) {
            Ok(p) => Ok((gen, log, p)),
            Err(e) => Err(Refused::Append(format!("the till log does not fold: {e}"))),
        })
    }

    /// The venue's currency, from its own record. No record yet is lek, as
    /// `services::venue::currency_of` says; an UNREADABLE record is an error.
    pub(super) async fn venue_currency(&self) -> Result<String> {
        match self.image(CATALOG_IMAGE).await? {
            None => Ok("ALL".into()),
            Some((_, bytes)) => {
                let cat = dowiz_hub::catalog::Catalog::load(&bytes)
                    .map_err(|_| Error::RustError("catalog image is unreadable".into()))?;
                Ok(crate::services::venue::currency_of(&cat))
            }
        }
    }

    /// Every cash payment on this venue's orders, from the memoised fold.
    async fn cash(&self, location_id: &str) -> Result<Vec<CashIn>> {
        let (_, listed) = self.orders_view().await?;
        let orders: Vec<Value> = listed
            .iter()
            .filter_map(|o| {
                let mut v: Value = serde_json::from_str(&o.order_json).ok()?;
                if v.get("id").is_none() {
                    v["id"] = Value::String(o.order_id.clone());
                }
                Some(v)
            })
            .collect();
        Ok(till::cash_payments(&orders, location_id, &self.venue_currency().await?))
    }

    /// ONE TILL COMMAND: decide, append, write the till image, answer with the
    /// period as it now is. The Worker decides what of it a person may see.
    pub(super) async fn till(&self, cmd: Cmd) -> Result<std::result::Result<TillOut, Refused>> {
        let loc = match &cmd {
            Cmd::Open(i) => i.location_id.clone(),
            Cmd::PayIn(i) | Cmd::PayOut(i) => i.location_id.clone(),
            Cmd::Count(i) => i.location_id.clone(),
            Cmd::Close(i) => i.location_id.clone(),
        };
        let (gen, mut log, periods) = match self.till_state().await? {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        let cash = self.cash(&loc).await?;
        let (kind, subject, rec) = match till::decide(&periods, &cash, &cmd) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        if let Err(e) = log.append(kind, &subject, &rec.to_string()) {
            return Ok(Err(Refused::Append(format!("the till would not take {kind}: {e:?}"))));
        }
        let Some(next) = self.put_image(IMAGE_TILL, gen, &log.to_bytes()).await? else {
            return Ok(Err(Refused::Append(format!("the till generation moved during {kind}"))));
        };
        let now = match till::periods(&log.entries()) {
            Ok(p) => p,
            Err(e) => return Ok(Err(Refused::Append(format!("the till log does not fold after {kind}: {e}")))),
        };
        let Some(last) = now.last() else {
            return Ok(Err(Refused::Append(format!("{kind} was written and no period folds from it"))));
        };
        let period = match till::report(std::slice::from_ref(last), &cash) {
            Ok(r) => r.periods.into_iter().next(),
            Err(r) => return Ok(Err(r)),
        };
        Ok(match period {
            Some(period) => Ok(TillOut { kind: kind.to_string(), till_id: subject, period, generation: next }),
            None => Err(Refused::Append(format!("{kind}: the period did not report"))),
        })
    }

    /// The `health.till` block: every period with its parts re-derived, and
    /// the cash that fell outside all of them.
    pub(super) async fn till_report(&self, q: ReportIn) -> Result<std::result::Result<Report, Refused>> {
        let periods = match self.till_state().await? {
            Ok((_, _, p)) => p,
            Err(r) => return Ok(Err(r)),
        };
        let cash = self.cash(&q.location_id).await?;
        Ok(till::report(&periods, &cash))
    }
}
