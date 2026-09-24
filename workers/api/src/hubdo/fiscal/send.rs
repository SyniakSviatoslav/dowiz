//! THE eBills SENDER IN THE VENUE'S OBJECT (card L70): `plan` claims, `answer`
//! settles, `status` shows, `receipt` prints. The rules are `fiscal::ebills_*`,
//! pure and tested natively; this file reads the images, runs them, and writes.
//!
//! THE INTENT IS WRITTEN BEFORE THE SEND: `plan` puts each claimed document's
//! `Sending` row into the `fiscal` image and only then answers the batch --
//! and if that write loses its generation race, it answers NOT READY, so no
//! Worker ever holds a batch whose claim was not recorded.
//!
//! THE LOG FIRST, THE QUEUE SECOND in `answer`: a lost write after the log
//! leaves the entry and its `Sending` intent, the next firing reconciles,
//! finds the sale, and `note` writes nothing twice.

use super::super::{HubImages, CATALOG_IMAGE};
use crate::command::Refused;
use crate::ebills::state::{self as eb, Mapping, Seen};
use crate::fiscal::ebills_arm::{arming, cancels_owed, gated_plan, CancelEntry, LinkState, CANCEL};
use crate::fiscal::ebills_cmd::{view, AnswerIn, PlanIn, PlanOut, SenderState, K_STATE, ONE};
use crate::fiscal::ebills_fire::Batch;
use crate::fiscal::ebills_sender::{note, settle, EbillsSender, Intent, IntentWrite, Outcome, INTENT};
use crate::fiscal::queue::{drain, KIND};
use crate::fiscal::wire::{self, Config};
use crate::outbox::{Entry, Verdict};
use dowiz_hub::settings::Settings;
use dowiz_hub::table::Table;
use serde_json::{json, Value};
use worker::*;

fn bad(e: impl std::fmt::Display) -> Error {
    Error::RustError(format!("{e}"))
}

fn rows<T: serde::de::DeserializeOwned>(t: &Table, kind: &str) -> Result<Vec<T>> {
    t.all(kind).into_iter().map(|(id, j)| serde_json::from_str(&j).map_err(|e| bad(format!("fiscal {kind}/{id} is unreadable: {e}")))).collect()
}

fn put<T: serde::Serialize>(t: &mut Table, kind: &str, id: &str, v: &T) -> Result<()> {
    t.put(kind, id, &serde_json::to_string(v).map_err(bad)?, &[], &[]).map_err(|e| bad(format!("fiscal {kind}: {e:?}")))
}

impl HubImages {
    async fn settings_image(&self) -> Result<Option<Settings>> {
        match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => Ok(Some(Settings::load(&b).map_err(|_| bad("settings image is unreadable"))?)),
            None => Ok(None),
        }
    }

    async fn table_of(&self, id: &str, ceiling: usize) -> Result<(i64, Option<Table>)> {
        match self.image(id).await? {
            Some((meta, b)) => Ok((meta.generation, Some(Table::load(&b, ceiling).map_err(|_| bad(format!("{id} image is unreadable")))?))),
            None => Ok((0, None)),
        }
    }

    async fn save_table(&self, id: &str, generation: i64, t: &mut Table) -> Result<bool> {
        let bytes = t.to_bytes().map_err(|e| bad(format!("{id} will not serialise: {e:?}")))?;
        Ok(self.put_image(id, generation, &bytes).await?.is_some())
    }

    /// `fiscal_plan`: the gate, then the claim, then the batch.
    pub(in crate::hubdo) async fn fiscal_plan(&self, input: PlanIn) -> Result<PlanOut> {
        let s = self.settings_image().await?;
        let since = s.as_ref().map_or(Config::Off, |s| wire::config(&s.known(wire::SETTING)));
        let a = s.as_ref().map(|s| arming(&|k| s.get(k))).unwrap_or_default();
        // AN UNARMED VENUE COSTS ONE SETTINGS READ A MINUTE, and nothing else.
        if !a.armed || !matches!(since, Config::From(_)) {
            let healthy = LinkState { usable: true, last_ok_ms: 1, ..LinkState::default() };
            return Ok(PlanOut { why: crate::fiscal::ebills_arm::not_ready(&since, &a, &healthy), ..PlanOut::default() });
        }
        let (_, et) = self.table_of(eb::IMAGE, eb::CEILING).await?;
        let et = et.unwrap_or(Table::create(eb::CEILING).map_err(|_| bad("cannot create ebills image"))?);
        let cfg: eb::Config = eb::get(&et, eb::K_CONFIG, eb::ONE).map_err(bad)?.unwrap_or_default();
        let st: eb::State = eb::get(&et, eb::K_STATE, eb::ONE).map_err(bad)?.unwrap_or_default();
        let link = LinkState { usable: cfg.usable(), halted: st.halted, failures: st.failures, last_ok_ms: st.last_ok_ms };
        let (fgen, ft) = self.table_of(wire::IMAGE, wire::CEILING).await?;
        let Some(mut ft) = ft else { return Ok(PlanOut { why: vec!["no fiscal document has been queued".into()], ..PlanOut::default() }) };
        let entries: Vec<Entry> = rows(&ft, KIND)?;
        let intents: Vec<Intent> = rows(&ft, INTENT)?;
        let (items, claims) = match gated_plan(&since, &a, &link, &entries, &intents, input.now_ms) {
            Ok(p) => p,
            Err(why) => return Ok(PlanOut { why, ..PlanOut::default() }),
        };
        let (_, orders) = self.orders_view().await?;
        let owed = cancels_owed(&orders, &|id| ft.has(CANCEL, id), input.now_ms);
        for c in &owed {
            put(&mut ft, CANCEL, &c.order_id, c)?;
        }
        for c in &claims {
            put(&mut ft, INTENT, &c.uuid, c)?;
        }
        if !(owed.is_empty() && claims.is_empty()) && !self.save_table(wire::IMAGE, fgen, &mut ft).await? {
            return Ok(PlanOut { why: vec!["the fiscal image moved while claiming: next firing".into()], ..PlanOut::default() });
        }
        let loc: Value = match self.image(CATALOG_IMAGE).await? {
            Some((_, b)) => dowiz_hub::catalog::Catalog::load(&b).ok().and_then(|c| c.location()).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({})),
            None => json!({}),
        };
        let p = crate::ebills::glue::plan_for(&et, &loc, input.now_ms).map_err(|r| bad(r.message()))?;
        let mut crosswalk = Vec::new();
        for (code, j) in et.all(eb::K_MAP) {
            let m: Mapping = serde_json::from_str(&j).map_err(|e| bad(format!("ebills map/{code}: {e}")))?;
            crosswalk.push((code, m.product_id));
        }
        let batch = Batch {
            pos_id: cfg.pos_id, today: p.today, yesterday: p.yesterday, sale_unit: a.sale_unit.clone(),
            fee_item: Some(a.fee_item.clone()).filter(|f| !f.is_empty()), crosswalk, items,
        };
        Ok(PlanOut { ready: true, why: Vec::new(), user: cfg.user, secret: cfg.secret, session: st.session, batch })
    }

    /// `fiscal_answer`: `Noted{fiscal}` on the log, then the queue and the intents.
    pub(in crate::hubdo) async fn fiscal_answer(&self, input: AnswerIn) -> Result<std::result::Result<Value, Refused>> {
        let (fgen, Some(mut ft)) = self.table_of(wire::IMAGE, wire::CEILING).await? else {
            return Ok(Err(Refused::Conflict("no fiscal image".into())));
        };
        let entries: Vec<Entry> = rows(&ft, KIND)?;
        let intents: Vec<Intent> = rows(&ft, INTENT)?;
        let drained = drain(&entries, &EbillsSender(input.outcomes.clone()), input.now_ms);
        // THE LOG FIRST.
        let (log_gen, listed) = self.orders_view().await?;
        let (_, mut hub) = self.log_hub().await?;
        let mut written = Vec::new();
        for (uuid, o) in &input.outcomes {
            let Outcome::Sent { sale_id, codes } = o else { continue };
            let Some(order_id) = entries.iter().find(|e| &e.id == uuid).map(|e| e.to.clone()) else { continue };
            let current = listed.iter().find(|v| v.order_id == order_id);
            match note(&mut hub, current, *sale_id, codes, input.now_ms) {
                Ok(Some((body, _))) => written.push((order_id, body)),
                Ok(None) => {}
                Err(r) => console_error!("fiscal: order {order_id} registered as ebills sale {sale_id} and NOT noted: {}", r.message()),
            }
        }
        if !written.is_empty() {
            let next = match self.write_both("a fiscal registration", log_gen, &hub, None).await? {
                Ok(n) => n,
                Err(r) => return Ok(Err(r)),
            };
            for (id, body) in &written {
                self.broadcast(dowiz_hub::EventKind::Noted as u8, id, body, next);
            }
        }
        // THE QUEUE SECOND: the drain's verdicts, the intents, the record.
        for (id, v) in &drained.verdicts {
            match v {
                Verdict::Sent | Verdict::Abandon { .. } => {
                    ft.remove(KIND, id);
                }
                Verdict::Retry { tries, next_at_ms } => {
                    if let Some(mut e) = entries.iter().find(|e| &e.id == id).cloned() {
                        (e.tries, e.next_at_ms) = (*tries, *next_at_ms);
                        put(&mut ft, KIND, id, &e)?;
                    }
                }
            }
        }
        for w in settle(&intents, &input.outcomes, input.now_ms) {
            match w {
                IntentWrite::Put(i) => put(&mut ft, INTENT, &i.uuid.clone(), &i)?,
                IntentWrite::Remove(u) => {
                    ft.remove(INTENT, &u);
                }
            }
        }
        let mut st: SenderState = ft.get(K_STATE, ONE).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
        st.after(&input.outcomes, input.now_ms);
        put(&mut ft, K_STATE, ONE, &st)?;
        if !self.save_table(wire::IMAGE, fgen, &mut ft).await? {
            return Ok(Err(Refused::Append("the fiscal image moved: the next firing reconciles".into())));
        }
        if let (Some(s), (egen, Some(mut et))) = (input.session, self.table_of(eb::IMAGE, eb::CEILING).await?) {
            if crate::ebills::glue::keep_session(&mut et, s).is_ok() && !self.save_table(eb::IMAGE, egen, &mut et).await? {
                console_error!("fiscal: the rotated ebills session was not kept (generation moved)");
            }
        }
        let exceptions: Vec<Value> = drained.exceptions.iter().map(|r| json!({ "order_id": r.order_id, "kind": r.kind, "reason": r.reason })).collect();
        Ok(Ok(json!({ "sent": drained.sent.len(), "noted": written.len(), "held": drained.held, "exceptions": exceptions })))
    }

    /// `fiscal_status`: the owner's pane (`ebills_cmd::view`).
    pub(in crate::hubdo) async fn fiscal_status(&self, now_ms: i64) -> Result<Value> {
        let s = self.settings_image().await?;
        let since = s.as_ref().map_or(Config::Off, |s| wire::config(&s.known(wire::SETTING)));
        let a = s.as_ref().map(|s| arming(&|k| s.get(k))).unwrap_or_default();
        let (_, et) = self.table_of(eb::IMAGE, eb::CEILING).await?;
        let et = et.unwrap_or(Table::create(eb::CEILING).map_err(|_| bad("cannot create ebills image"))?);
        let cfg: eb::Config = eb::get(&et, eb::K_CONFIG, eb::ONE).map_err(bad)?.unwrap_or_default();
        let st: eb::State = eb::get(&et, eb::K_STATE, eb::ONE).map_err(bad)?.unwrap_or_default();
        let link = LinkState { usable: cfg.usable(), halted: st.halted, failures: st.failures, last_ok_ms: st.last_ok_ms };
        let why = crate::fiscal::ebills_arm::not_ready(&since, &a, &link);
        let (_, ft) = self.table_of(wire::IMAGE, wire::CEILING).await?;
        let ft = ft.unwrap_or(Table::create(wire::CEILING).map_err(|_| bad("cannot create fiscal image"))?);
        let sst: SenderState = ft.get(K_STATE, ONE).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
        let cancels: Vec<CancelEntry> = rows(&ft, CANCEL)?;
        let floor: Vec<String> = match self.image(eb::FLOOR_IMAGE).await? {
            Some((_, b)) => serde_json::from_slice::<Value>(&b).ok().and_then(|v| v["tables"].as_array().map(|t| t.iter().filter_map(|r| r["table"].as_str().map(str::to_string)).collect())).unwrap_or_default(),
            None => Vec::new(),
        };
        let items: Vec<(String, String)> = et.all(eb::K_SEEN).into_iter().filter_map(|(c, j)| serde_json::from_str::<Seen>(&j).ok().map(|s| (c, s.name))).collect();
        let since_ms = match since {
            Config::From(ms) => Some(ms),
            _ => None,
        };
        Ok(view(&why, &a, since_ms, &rows(&ft, KIND)?, &rows(&ft, INTENT)?, &cancels, &sst, floor, items, now_ms))
    }

    /// `fiscal_receipt`: the art. 29 text for one order, with its codes once registered.
    pub(in crate::hubdo) async fn fiscal_receipt(&self, order_id: &str) -> Result<std::result::Result<Value, Refused>> {
        let (_, listed) = self.orders_view().await?;
        let Some(v) = listed.iter().find(|o| o.order_id == order_id) else { return Ok(Err(Refused::NotFound)) };
        let order: Value = serde_json::from_str(&v.order_json).map_err(bad)?;
        let (_, ft) = self.table_of(wire::IMAGE, wire::CEILING).await?;
        let queued = ft.as_ref().and_then(|t| t.all(KIND).into_iter().find_map(|(_, j)| serde_json::from_str::<Entry>(&j).ok().filter(|e| e.to == order_id)));
        let currency = match self.image(CATALOG_IMAGE).await? {
            Some((_, b)) => crate::services::venue::currency_of(&dowiz_hub::catalog::Catalog::load(&b).map_err(|_| bad("catalogue image is unreadable"))?),
            None => "ALL".into(),
        };
        let issued = order.pointer("/fiscal/at").or_else(|| order.get("created_at_ms")).and_then(Value::as_i64).unwrap_or(0);
        let doc = match queued.and_then(|e| serde_json::from_str(&e.text).ok()) {
            Some(d) => d,
            None => match crate::fiscal::document::document(&order, &currency, issued) {
                Ok(d) => d,
                Err(r) => return Ok(Err(Refused::Conflict(format!("this order has no fiscal document: {r:?}")))),
            },
        };
        let codes = order.get("fiscal").and_then(|f| serde_json::from_value::<crate::fiscal::sender::Codes>(f.clone()).ok());
        Ok(Ok(json!({ "order_id": order_id, "text": crate::fiscal::receipt::receipt_text(&doc, codes.as_ref()), "registered": codes.is_some() })))
    }
}
