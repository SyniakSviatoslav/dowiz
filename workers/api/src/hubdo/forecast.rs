//! THE KITCHEN'S FORECAST, ANSWERED HERE (W-PREP, BN1's shape): the orders,
//! the catalogue, the stock log, the cube and the bookings are all in this
//! object, so the prep list is folded here and only the answer crosses.
//!
//!   GET /fold/prep?venue=&now=[&day=yyyy-mm-dd]   `services::analytics::forecast`
//!
//! And P7: the day's `stock.expiring` (told on the first movement of the
//! venue's day, `stock_turn.rs`) is rewritten with what the forecast will
//! not use before each lot's date (`tell::with_surplus`). Once a day; a
//! failure never fails the movement and is never silent: the event carries
//! `forecast: <why>`.

use super::super::HubImages;
use super::{cube, query, venue_and_now};
use crate::services::analytics::forecast::{self, expand};
use crate::services::operations::stock::{tell, turn::StockTurnIn};
use crate::notify::route::produce::Supply;
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

impl HubImages {
    /// See the module.
    pub(in crate::hubdo) async fn fold_prep(&self, req: &Request) -> Result<Response> {
        let (venue, now) = match venue_and_now(req)? {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
        let day = query(req, "day")?;
        let (_, listed) = self.orders_view().await?;
        let cat = self.catalogue().await?;
        let (_, stock) = self.stock_log().await?;
        let (_, bytes) = self.cube_image().await?;
        let parsed = cube::parsed(&bytes);
        let cold = cube::rows_of(&parsed);
        let booked = self.image(crate::booking::IMAGE_BOOKINGS).await?.map(|(_, b)| b);
        let why = std::cell::RefCell::new(None::<String>);
        let covers = |from: i64, to: i64| match crate::booking::covers::of_image(booked.as_deref(), from, to) {
            Ok(c) => {
                if c.unreadable > 0 {
                    *why.borrow_mut() = Some(format!("{} bookings could not be read", c.unreadable));
                }
                c.guests
            }
            Err(e) => {
                *why.borrow_mut() = Some(e);
                0
            }
        };
        match forecast::answer_with(listed, &cat, &stock, &venue, now, day.as_deref(), &covers, &cold) {
            Ok(mut v) => {
                v["bookings"]["error"] = serde_json::json!(why.into_inner());
                Response::from_json(&v)
            }
            Err((status, why)) => Response::error(why, status),
        }
    }

    /// P7: see the module. `told` is the movement's events; only a
    /// `stock.expiring` among them is touched.
    pub(in crate::hubdo) async fn expiring_surplus(&self, told: &mut [(&'static str, serde_json::Value)], log: &dowiz_hub::stock::StockLog, input: &StockTurnIn, cat: &dowiz_hub::catalog::Catalog) {
        if !told.iter().any(|(e, _)| *e == tell::EXPIRING) {
            return;
        }
        let lots_of = log.journal();
        let supply = |id: &str| input.supplies.get(id).map(|s| Supply { name: s.name.clone(), unit: s.unit.clone(), low_at: s.low_at });
        let table = match self.use_table(input, cat).await {
            Ok(t) => t,
            Err(e) => (None, Some(format!("the forecast could not be read: {e}"))),
        };
        let Ok(journal) = lots_of else {
            tell::with_surplus(told, &[], input.today, &supply, None, Some("the lots could not be read".into()));
            return;
        };
        let lots = journal.lots.open();
        let today = dowiz_hub::stock::meta::day_number(input.today);
        match &table.0 {
            Some(t) => {
                let until = |item: &str, expiry: i64| expand::use_until(t, today, item, expiry);
                tell::with_surplus(told, &lots, input.today, &supply, Some(&until as &dyn Fn(&str, i64) -> Option<i64>), table.1);
            }
            None => tell::with_surplus(told, &lots, input.today, &supply, None, table.1),
        }
    }

    /// The forecast's raw use per item per day ahead, and why it is absent
    /// or partial (learning, or an unreadable cube).
    /// `cat`: the movement's own catalogue, decoded once by `stock_move` (W-LOOPB).
    async fn use_table(&self, input: &StockTurnIn, cat: &dowiz_hub::catalog::Catalog) -> Result<(Option<std::collections::BTreeMap<String, Vec<i64>>>, Option<String>)> {
        let venue = forecast::venue_of(cat);
        let loc = venue.as_ref().and_then(|v| v.get("id")).and_then(serde_json::Value::as_str).unwrap_or("").to_string();
        let zone = crate::hubstore::zone_of(venue.as_ref());
        let today = dowiz_hub::stock::meta::day_number(input.today);
        let (_, listed) = self.orders_view().await?;
        let (_, bytes) = self.cube_image().await?;
        let parsed = cube::parsed(&bytes);
        let cold = cube::rows_of(&parsed);
        let (h, unread, _) = forecast::history_with(listed, &loc, zone, input.now_ms, today, &cold);
        let warn = crate::services::operations::stock::view::EXPIRY_WARN_DAYS;
        let table = expand::raw_by_day(&h, today, cat, warn);
        let why = unread.or_else(|| table.is_none().then(|| "learning".to_string()));
        Ok((table, why))
    }
}
