//! THE TILL LINK'S COMMANDS, in the venue's object (`/fold/ebills/<what>`).
//! The Worker fetches from ebills.al and maps; this object holds the images
//! and decides: the import's check-and-append, the crosswalk, the floor,
//! the poller's own state. Each command is one turn: read in memory, decide,
//! write -- the log first, the shelf second, the `ebills` image last.

use super::{HubImages, CATALOG_IMAGE};
use crate::command::Refused;
use crate::ebills::cmd::{ConfigIn, FloorIn, ImportIn, ImportOut, MapIn, ReportIn, TickIn};
use crate::ebills::glue;
use crate::ebills::import::{self, Lookups, Waiting};
use crate::ebills::state::{self, Mapping, State, CEILING, FLOOR_IMAGE, IMAGE, K_MAP, K_STATE, ONE};
use crate::ebills::status;
use dowiz_hub::table::Table;
use serde_json::{json, Value};
use worker::*;

fn bad(e: impl std::fmt::Display) -> Error {
    Error::RustError(format!("{e}"))
}

fn reply<T: serde::Serialize>(r: std::result::Result<T, Refused>) -> Result<Response> {
    match r {
        Ok(v) => Response::from_json(&v),
        Err(r) => Response::error(r.message().to_string(), r.status()),
    }
}

impl HubImages {
    /// `POST /fold/ebills/<what>`. The Worker authenticated the owner for the
    /// owner's three (`status`, `config`, `map`); the cron is the caller of the rest.
    pub(super) async fn ebills(&self, what: &str, mut req: Request) -> Result<Response> {
        match what {
            "tick" => Response::from_json(&self.ebills_tick(req.json().await?).await?),
            "import" => reply(self.ebills_import(req.json().await?).await?),
            "floor" => reply(self.ebills_floor(req.json().await?).await?),
            "report" => reply(self.ebills_report(req.json().await?).await?),
            "status" => Response::from_json(&self.ebills_status().await?),
            "config" => reply(self.ebills_config(req.json().await?).await?),
            "map" => reply(self.ebills_map(req.json().await?).await?),
            _ => Response::error("no such ebills command", 404),
        }
    }

    /// The `ebills` table and the generation it was read at; a fresh one if
    /// the venue has never configured the link.
    async fn ebills_table(&self) -> Result<(i64, Option<Table>)> {
        match self.image(IMAGE).await? {
            Some((meta, bytes)) => Ok((meta.generation, Some(Table::load(&bytes, CEILING).map_err(|_| bad("ebills image is unreadable"))?))),
            None => Ok((0, None)),
        }
    }

    async fn ebills_save(&self, generation: i64, t: &mut Table) -> Result<std::result::Result<i64, Refused>> {
        let bytes = t.to_bytes().map_err(|e| bad(format!("ebills image will not serialise: {e:?}")))?;
        Ok(self.put_image(IMAGE, generation, &bytes).await?.ok_or_else(|| Refused::Append("the ebills generation moved".into())))
    }

    async fn catalog(&self) -> Result<Option<dowiz_hub::catalog::Catalog>> {
        Ok(match self.image(CATALOG_IMAGE).await? {
            Some((_, b)) => Some(dowiz_hub::catalog::Catalog::load(&b).map_err(|_| bad("catalogue image is unreadable"))?),
            None => None,
        })
    }

    /// What this firing should do (`glue::plan_for`). A venue that never
    /// configured the link answers `enabled: false` from one storage read.
    async fn ebills_tick(&self, input: TickIn) -> Result<state::Plan> {
        let (_, Some(t)) = self.ebills_table().await? else { return Ok(state::Plan::default()) };
        let loc: Value = self.catalog().await?.and_then(|c| c.location()).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
        glue::plan_for(&t, &loc, input.now_ms).map_err(|r| bad(r.message()))
    }

    /// THE IMPORT (§6.1): check-and-append against the log in memory, the
    /// shelf drawn for what was served, the poller's state moved -- one turn.
    /// Written log first, shelf second, the `ebills` image last.
    async fn ebills_import(&self, input: ImportIn) -> Result<std::result::Result<ImportOut, Refused>> {
        let (tgen, Some(mut t)) = self.ebills_table().await? else {
            return Ok(Err(Refused::Conflict("the till link is not configured".into())));
        };
        let st: State = state::get(&t, K_STATE, ONE).map_err(bad)?.unwrap_or_default();
        let (log_gen, listed) = self.orders_view().await?;
        let (_, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log().await?;
        let before = stock.len();
        let cat = self.catalog().await?;
        let out = {
            let map = |code: &str| state::get::<Mapping>(&t, K_MAP, code).ok().flatten().map(|m| m.product_id);
            let product = |pid: &str| cat.as_ref().and_then(|c| c.product(pid));
            let look = Lookups { map: &map, product: &product };
            let waiting = Waiting { bills: st.pending.clone(), leads: st.leads.clone() };
            match import::decide(&mut hub, &mut stock, &listed, &look, waiting, &input.sales, input.now_ms) {
                Ok(o) => o,
                // NOTHING HAS BEEN WRITTEN: a storage refusal drops every copy.
                Err(r) => return Ok(Err(r)),
            }
        };
        let mut generation = log_gen;
        if !out.written.is_empty() {
            let moved = (stock.len() != before).then_some((stock_gen, &stock));
            generation = match self.write_both("an ebills import", log_gen, &hub, moved).await? {
                Ok(n) => n,
                Err(r) => return Ok(Err(r)),
            };
            for (kind, id, body) in &out.written {
                self.broadcast(*kind as u8, id, body, generation);
            }
        }
        let short = match stock.len() != before {
            true => Some(dowiz_hub::stock::short(&stock.ledger().map_err(|e| bad(format!("stock: {e}")))?)),
            false => None,
        };
        let st = match glue::record_import(&mut t, &input, &out, short) {
            Ok(s) => s,
            Err(r) => return Ok(Err(r)),
        };
        if let Err(r) = self.ebills_save(tgen, &mut t).await? {
            return Ok(Err(r));
        }
        let refused = input.refused.into_iter().chain(out.refused).collect();
        Ok(Ok(ImportOut { placed: out.placed, noted: out.noted, paid: out.paid, unchanged: out.unchanged, pending: out.pending.len(), refused, short: st.short, generation }))
    }

    /// The live floor, into its own image -- only when a table changed.
    async fn ebills_floor(&self, input: FloorIn) -> Result<std::result::Result<Value, Refused>> {
        let (fgen, old) = match self.image(FLOOR_IMAGE).await? {
            Some((m, b)) => (m.generation, Some(b)),
            None => (0, None),
        };
        let bytes = glue::floor_bytes(old.as_deref(), &input.tables, input.now_ms);
        if let Some(b) = &bytes {
            if self.put_image(FLOOR_IMAGE, fgen, b).await?.is_none() {
                return Ok(Err(Refused::Append("the floor generation moved".into())));
            }
        }
        if let (Some(s), (tgen, Some(mut t))) = (input.session, self.ebills_table().await?) {
            if let Err(r) = glue::keep_session(&mut t, s) {
                return Ok(Err(r));
            }
            if let Err(r) = self.ebills_save(tgen, &mut t).await? {
                return Ok(Err(r));
            }
        }
        Ok(Ok(json!({ "changed": bytes.is_some() })))
    }

    /// A failed firing: named, counted, and backed off from (`glue::apply_report`).
    async fn ebills_report(&self, input: ReportIn) -> Result<std::result::Result<Value, Refused>> {
        let (tgen, Some(mut t)) = self.ebills_table().await? else { return Ok(Ok(json!({}))) };
        let v = match glue::apply_report(&mut t, input) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        Ok(self.ebills_save(tgen, &mut t).await?.map(|_| v))
    }

    /// The owner's view (`ebills::status::view`), and the health lines' source.
    pub(super) async fn ebills_status(&self) -> Result<Value> {
        let (_, t) = self.ebills_table().await?;
        let t = match t {
            Some(t) => t,
            None => Table::create(CEILING).map_err(|_| bad("cannot create ebills image"))?,
        };
        let products = self.catalog().await?.map(|c| status::products(&c.products())).unwrap_or_default();
        let floor = match self.image(FLOOR_IMAGE).await? {
            Some((_, b)) => serde_json::from_slice(&b).unwrap_or(Value::Null),
            None => Value::Null,
        };
        status::view(&t, &products, floor).map_err(bad)
    }

    /// The owner's configuration (`glue::apply_config`); the password is
    /// written here and answered nowhere.
    async fn ebills_config(&self, input: ConfigIn) -> Result<std::result::Result<Value, Refused>> {
        let (tgen, mut t) = self.ebills_table_or_new().await?;
        let v = match glue::apply_config(&mut t, input) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        Ok(self.ebills_save(tgen, &mut t).await?.map(|_| v))
    }

    /// SET BY THE OWNER, NEVER BY A GUESS (`glue::apply_map`).
    async fn ebills_map(&self, input: MapIn) -> Result<std::result::Result<Value, Refused>> {
        let (tgen, mut t) = self.ebills_table_or_new().await?;
        let cat = self.catalog().await?;
        let has = |pid: &str| cat.as_ref().and_then(|c| c.product(pid)).is_some();
        let v = match glue::apply_map(&mut t, input, &has) {
            Ok(v) => v,
            Err(r) => return Ok(Err(r)),
        };
        Ok(self.ebills_save(tgen, &mut t).await?.map(|_| v))
    }

    async fn ebills_table_or_new(&self) -> Result<(i64, Table)> {
        match self.ebills_table().await? {
            (g, Some(t)) => Ok((g, t)),
            (g, None) => Ok((g, Table::create(CEILING).map_err(|_| bad("cannot create ebills image"))?)),
        }
    }
}
