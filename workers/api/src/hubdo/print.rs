//! THE PRINT RAIL, in the venue's object (`/fold/print/<what>`). The rules
//! are `print_rail.rs`, pure; this file only reads the images, runs them, and
//! writes what they decided.
//!
//! AN ACK WRITES TWO IMAGES IN ONE TURN, LOG FIRST, for the reason
//! `write_both` gives: a lost second write leaves an order that says
//! "printed" with its entry still queued -- the next DELETE is answered from
//! the order and never stacks a second note (`decide_ack`) -- never an entry
//! gone with nothing saying it printed.

use super::HubImages;
use crate::outbox::{Entry, IMAGE_OUTBOX, KIND as OUTBOX_KIND, OUTBOX_BYTES};
use crate::print_rail::{self as pr, AckIn, Change, JobIn, PollIn};
use dowiz_hub::table::Table;
use serde_json::{json, Value};
use worker::*;

impl HubImages {
    /// `POST /fold/print/<what>` — the Worker has resolved the venue from the
    /// printer's key; what arrives here is that venue's object.
    pub(super) async fn print(&self, what: &str, mut req: Request) -> Result<Response> {
        match what {
            "poll" => {
                let input: PollIn = req.json().await?;
                self.print_poll(input.now_ms).await
            }
            "job" => {
                let input: JobIn = req.json().await?;
                let (_, table) = self.outbox_table().await?;
                let found = pr::id_of(&input.token)
                    .and_then(|id| table.get(OUTBOX_KIND, &id))
                    .and_then(|j| serde_json::from_str::<Entry>(&j).ok());
                match found {
                    Some(e) => Response::from_json(&json!({ "text": e.text, "printer": e.to })),
                    None => Response::error("no such job", 404),
                }
            }
            "ack" => {
                let input: AckIn = req.json().await?;
                self.print_ack(input).await
            }
            _ => Response::error("no such print command", 404),
        }
    }

    /// The venue's outbox, and the generation it was read at.
    async fn outbox_table(&self) -> Result<(i64, Table)> {
        Ok(match self.image(IMAGE_OUTBOX).await? {
            Some((meta, bytes)) => (
                meta.generation,
                Table::load(&bytes, OUTBOX_BYTES).map_err(|_| Error::RustError("outbox image is unreadable".into()))?,
            ),
            None => (0, Table::create(OUTBOX_BYTES).map_err(|_| Error::RustError("cannot create outbox image".into()))?),
        })
    }

    async fn save_outbox(&self, generation: i64, mut table: Table) -> Result<()> {
        let bytes = table.to_bytes().map_err(|e| Error::RustError(format!("outbox will not serialise: {e:?}")))?;
        if self.put_image(IMAGE_OUTBOX, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the outbox generation moved".into()));
        }
        Ok(())
    }

    /// Hand the oldest free job to the printer, marking it `printing`.
    async fn print_poll(&self, now_ms: i64) -> Result<Response> {
        let (generation, mut table) = self.outbox_table().await?;
        let entries: Vec<Entry> =
            table.all(OUTBOX_KIND).into_iter().filter_map(|(_, j)| serde_json::from_str(&j).ok()).collect();
        let Some(job) = pr::decide_poll(&entries, now_ms) else {
            return Response::from_json(&json!({ "jobReady": false }));
        };
        let rec = serde_json::to_string(&job).map_err(|e| Error::RustError(format!("print: {e}")))?;
        table.put(OUTBOX_KIND, &job.id, &rec, &[], &[]).map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
        self.save_outbox(generation, table).await?;
        Response::from_json(&json!({
            "jobReady": true, "mediaTypes": [pr::MEDIA], "jobToken": pr::token_of(&job.id),
        }))
    }

    /// The printer's DELETE: printed, one more failure, or given up on.
    async fn print_ack(&self, input: AckIn) -> Result<Response> {
        let (generation, mut table) = self.outbox_table().await?;
        let id = pr::id_of(&input.token).unwrap_or_default();
        let entry: Option<Entry> = table.get(OUTBOX_KIND, &id).and_then(|j| serde_json::from_str(&j).ok());
        let (log_gen, listed) = self.orders_view().await?;
        let current = listed.into_iter().find(|o| o.order_id == pr::order_of(&id));
        let order: Option<(Value, u64)> =
            current.as_ref().and_then(|c| serde_json::from_str(&c.order_json).ok().map(|v| (v, c.seq)));
        let Some(plan) = pr::decide_ack(entry.as_ref(), order.as_ref().map(|(v, s)| (v, *s)), &input) else {
            return Response::error("no such job", 404);
        };
        if let Some((order_id, body, seq)) = &plan.note {
            let (_, mut hub) = self.log_hub().await?;
            hub.append(dowiz_hub::EventKind::Noted, order_id, body, *seq, [0u8; 32])
                .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
            let next = match self.write_both("a print ack", log_gen, &hub, None).await? {
                Ok(n) => n,
                Err(r) => return Response::error(r.message().to_string(), r.status()),
            };
            self.broadcast(dowiz_hub::EventKind::Noted as u8, order_id, body, next);
        }
        match &plan.change {
            Change::Keep => {}
            Change::Remove => {
                table.remove(OUTBOX_KIND, &id);
                self.save_outbox(generation, table).await?;
            }
            Change::Put(e) => {
                let rec = serde_json::to_string(e).map_err(|e| Error::RustError(format!("print: {e}")))?;
                table.put(OUTBOX_KIND, &e.id, &rec, &[], &[]).map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
                self.save_outbox(generation, table).await?;
            }
        }
        Response::from_json(&plan.out)
    }
}
