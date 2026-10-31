//! THE LOG'S OWN COMMANDS (split out of `hubdo.rs`, W-SPLIT): an append the
//! Worker decided, an advance that settles the shelf, an assignment that
//! writes the lock -- each read, decided and written in this object's turn.

use super::{HubImages, LOG_IMAGE};
use crate::wire::{Call, Reply as Response};
use worker::*;

/// What an append asks for. The kernel has already decided; this is the record.
#[derive(serde::Deserialize)]
struct AppendIn {
    kind: u8,
    order_id: String,
    payload: String,
    clock: u64,
    /// The courier tap's claim and answer (W-O2), absent on every other append.
    #[serde(default)]
    idem: Option<crate::idempotency::commit::Marked>,
}

impl HubImages {
    /// Append one event to the log and persist it, under the same generation
    /// guard a write of the whole image carries.
    ///
    /// THE DECISION IS STILL THE WORKER'S. The kernel decided this transition
    /// was legal and computed the payload; this is the record of it. What
    /// moves here is the WRITE, which no longer means shipping the whole image
    /// back -- an append sends a few hundred bytes instead of the log.
    async fn append(&self, expected: i64, ev: AppendIn) -> Result<Option<(i64, usize)>> {
        let (generation, bytes) = match self.image(LOG_IMAGE).await? {
            Some((meta, bytes)) => (meta.generation, Some(bytes)),
            None => (0, None),
        };
        if expected != generation {
            return Ok(None);
        }
        let mut hub = match bytes {
            Some(b) => self.load_log(generation, &b).map_err(|e| crate::image_err::unreadable("hub", e))?,
            // BORN SMALL, as `hubstore::load` does it: a hub created at 4 MiB
            // made the third order of the day exceed the isolate's limit.
            None => dowiz_hub::Hub::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create hub image".into()))?,
        };
        let kind = dowiz_hub::EventKind::from_u8(ev.kind)
            .ok_or_else(|| Error::RustError(format!("unknown event kind {}", ev.kind)))?;
        // THE VERSION MOVES FORWARD (W-AUDIT F9): the clock the Worker sends
        // is the seq only when it is ahead of the order's newest event. Three
        // writers in one coarse millisecond used to leave the seq where it
        // was, and a tablet quoting that seq passed its `base_seq` check.
        let seq = dowiz_hub::room::amend::next_seq(
            dowiz_hub::room::view::latest_seq(&hub, &ev.order_id),
            i64::try_from(ev.clock).unwrap_or(i64::MAX),
        );
        hub.append(kind, &ev.order_id, &ev.payload, seq, [0u8; 32])
            .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
        let len = hub.len();
        // THE ONE EVENT THIS WRITE ADDS is the memo's whole dirty set (R1).
        let added = dowiz_hub::Event { kind, order_id: ev.order_id.clone(), order_json: ev.payload.clone(), seq };
        let written = crate::fold::projection::Written::Appended(added);
        match self.put_image_as(LOG_IMAGE, generation, &hub.to_bytes_trimmed(), written).await? {
            Some(next) => {
                // AFTER THE WRITE LANDED, never before: a subscriber told about
                // an event that was not persisted would be told the truth about
                // a log that does not say it.
                self.broadcast(ev.kind, &ev.order_id, &ev.payload, next);
                self.tell_appended(ev.kind, &ev.order_id, &ev.payload).await; // W-PUSH, W-SMS, at the object's clock (`hubdo.rs`)
                Ok(Some((next, len)))
            }
            // Cannot happen inside a serialised object -- the generation was
            // read two lines ago -- but the caller's contract already says
            // what to do about it, so say it rather than assume.
            None => Ok(None),
        }
    }

    /// `/fold/append`: the Worker's decision recorded, under the generation it read.
    pub(super) async fn append_route(&self, req: Call) -> Result<Response> {
        let expected: i64 = req
            .headers()
            .get("x-generation")
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(-1);
        if expected < 0 {
            return Response::error("x-generation is required on a write", 400);
        }
        let mut req = req;
        let mut ev: AppendIn = crate::body::parse(&mut req).await?;
        let mark = ev.idem.take();
        match self.append(expected, ev).await? {
            Some((generation, len)) => {
                // THE EVENT LANDED: its claim is marked now (W-O2).
                if let Some(m) = &mark {
                    self.commit_claim(Some(&m.claim), &m.output).await;
                }
                let mut res = Response::from_json(
                    &serde_json::json!({ "generation": generation, "events": len }),
                )?;
                res.headers_mut().set("x-generation", &generation.to_string())?;
                Ok(res)
            }
            None => Response::error("generation moved", 409),
        }
    }

    /// ADVANCE AN ORDER AND SETTLE THE SHELF, in one turn.
    ///
    /// The Worker used to append through `/fold/append` and then settle the
    /// stock ledger in a SECOND round trip whose failure it could only log --
    /// deliberately, because failing the transition afterwards would have left
    /// the order and the ledger disagreeing the other way round. Both are now
    /// decided against copies in memory and written once both have succeeded,
    /// so neither direction is reachable from a failure in the other.
    pub(super) async fn advance(
        &self,
        input: crate::command::advance::AdvanceIn,
        claim: Option<crate::idempotency::commit::Claim>,
    ) -> Result<std::result::Result<crate::command::advance::AdvanceOut, crate::command::Refused>>
    {
        let (log_generation, listed) = self.orders_view().await?;
        let current = listed.iter().find(|o| o.order_id == input.order_id).map(|o| o.order_json.clone());
        let Some((log_meta, log_bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok(Err(crate::command::Refused::NotFound));
        };
        let mut hub = self.load_log(log_meta.generation, &log_bytes).map_err(|e| crate::image_err::unreadable("hub", e))?;
        let stock_image = crate::hubstore::IMAGE_STOCK;
        let (stock_generation, mut stock) = match self.image(stock_image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                self.load_stock(meta.generation, &bytes).map_err(|e| crate::image_err::unreadable("stock", e))?,
            ),
            None => (
                0,
                dowiz_hub::stock::StockLog::create_sized(64 * 1024)
                    .map_err(|_| Error::RustError("cannot create stock image".into()))?,
            ),
        };
        stock.set_clock(input.now_ms);
        if let Some(seed) = crate::entropy_seed() {
            stock.set_entropy(seed); // M0 (a fresh stock image has no seed yet)
        }
        self.arm_scheme(&mut stock);
        let settled_before = stock.len();

        let merged = match crate::command::advance::decide(
            &mut hub,
            &mut stock,
            current.as_deref(),
            &input,
        ) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };

        let Some(next) = self.put_log(log_generation, &hub).await?
        else {
            return Ok(Err(crate::command::Refused::Append(
                "the log generation moved during a transition".into(),
            )));
        };
        // THE ORDER MOVED: its claim is marked committed now (W-O2).
        let out = crate::command::advance::AdvanceOut { merged: merged.to_string(), generation: next };
        self.commit_claim(claim.as_ref(), &serde_json::to_string(&out).unwrap_or_default()).await;
        if stock.len() != settled_before
            && self
                .put_derived(stock_image, stock_generation, &stock.to_bytes_trimmed())
                .await?
                .is_none()
        {
            log_error!(
                "stock: order {} advanced to {} and the settlement was NOT written",
                input.order_id,
                input.next
            );
        }
        let body = crate::fold::delta(
            &serde_json::from_str(current.as_deref().unwrap_or("{}")).unwrap_or_default(),
            &merged,
        )
        .to_string();
        self.broadcast(dowiz_hub::EventKind::Advanced as u8, &input.order_id, &body, next);
        // THE GROUPS (W0b): this order's new status, and orders now late, as
        // this turn left them.
        let now_json = merged.to_string();
        let orders: Vec<(String, String)> = listed
            .iter()
            .map(|o| (o.order_id.clone(), if o.order_id == input.order_id { now_json.clone() } else { o.order_json.clone() }))
            .collect();
        self.tell_orders(Some((&input.order_id, &input.next)), &orders, input.now_ms).await;
        Ok(Ok(out))
    }

    /// HAND AN ORDER TO A COURIER: the assignment record and the event that
    /// makes it visible, in one turn.
    ///
    /// THE LOG IS WRITTEN FIRST, and for this command the reasoning is the
    /// mirror of the old bug. The assignment record is also the LOCK -- the
    /// refusal of a second courier is `ops.get("asg", id).is_some()` -- so it
    /// used to land BEFORE the event that shows it, and a lost append left an
    /// invisible lock that the console could neither see nor clear. Writing the
    /// log first means a lost ops write leaves an order that names its courier
    /// and no record behind it: visible, and re-assignable.
    pub(super) async fn assign(
        &self,
        input: crate::command::assign::AssignIn,
    ) -> Result<std::result::Result<crate::command::assign::AssignOut, crate::command::Refused>>
    {
        let (log_generation, listed) = self.orders_view().await?;
        let current = listed.iter().find(|o| o.order_id == input.order_id).map(|o| o.order_json.clone());
        let Some((log_meta, log_bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok(Err(crate::command::Refused::NotFound));
        };
        let mut hub = self.load_log(log_meta.generation, &log_bytes).map_err(|e| crate::image_err::unreadable("hub", e))?;
        let ops_image = crate::hubstore::IMAGE_OPS;
        let ceiling = crate::hubstore::OPS_BYTES;
        let (ops_generation, mut ops) = match self.image(ops_image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                // A CORRUPT IMAGE IS NOT AN EMPTY ONE: reading it as empty here
                // would hand out an order that already has a courier.
                dowiz_hub::table::Table::load(&bytes, ceiling)
                    .map_err(|e| crate::image_err::unreadable("ops", e))?,
            ),
            None => (
                0,
                dowiz_hub::table::Table::create(ceiling)
                    .map_err(|_| Error::RustError("cannot create ops image".into()))?,
            ),
        };

        if let Err(r) =
            crate::command::assign::decide(&mut hub, &mut ops, current.as_deref(), &input)
        {
            return Ok(Err(r));
        }

        let Some(next) = self.put_log(log_generation, &hub).await?
        else {
            return Ok(Err(crate::command::Refused::Append(
                "the log generation moved during an assignment".into(),
            )));
        };
        let ops_bytes = ops
            .to_bytes()
            .map_err(|e| Error::RustError(format!("ops image will not serialise: {e:?}")))?;
        if self.put_image(ops_image, ops_generation, &ops_bytes).await?.is_none() {
            log_error!(
                "ops: order {} names courier {} and the assignment was NOT written",
                input.order_id,
                input.courier_id
            );
        }
        let body = serde_json::json!({
            "_d": { "courier_id": input.courier_id, "assigned_at_ms": input.now_ms }
        })
        .to_string();
        self.broadcast(dowiz_hub::EventKind::Noted as u8, &input.order_id, &body, next);
        self.tell_push_assigned(&input.order_id, &input.courier_id, input.now_ms).await; // W-PUSH
        Ok(Ok(crate::command::assign::AssignOut { generation: next }))
    }
}
