//! PLACE AN ORDER, AND RING THE BELL FOR IT (split out of `hubdo.rs`, W-SPLIT):
//! the reservation, the redemption and the append in one turn, then the
//! kitchen's messages queued from the settings image this object holds.

use super::{HubImages, LOG_IMAGE};
use worker::*;

impl HubImages {
    /// PLACE AN ORDER: the reservation, the redemption and the append, in ONE
    /// object turn.
    ///
    /// WHAT THIS REPLACED, and it is worth stating where the code is rather
    /// than only in `command::place`'s header: the Worker used to run this as a
    /// saga over the network -- write the stock image, write the log image, and
    /// if the second failed, write the stock image again to release what the
    /// first had held. If THAT failed it printed a line and gave up, so a
    /// stranded reservation made a kitchen believe it was out of something it
    /// had, and nothing but a sampled trace knew.
    ///
    /// THE COMPENSATION IS GONE BECAUSE THERE IS NOTHING TO COMPENSATE.
    /// `command::place::decide` does every fallible thing against images held in
    /// memory; this function writes only after it has returned `Ok`. A refusal
    /// drops both copies and the venue's stored state was never touched.
    ///
    /// THE LOG IS WRITTEN FIRST, AND THE ORDER IS DELIBERATE. Both writes
    /// happen in one turn, so the runtime will normally commit them together --
    /// but "normally" is not a guarantee this code is entitled to make, so the
    /// order is chosen for what happens if the second one is lost:
    ///   * log then stock (this) leaves an order whose ingredients are not
    ///     held. The kitchen can see the order, the conservation audit can see
    ///     the disagreement, and a human can fix it.
    ///   * stock then log leaves ingredients held for an order that does not
    ///     exist. Nothing shows it to anyone. That is the failure this whole
    ///     change exists to remove, so it is not the one to re-introduce at a
    ///     smaller size.
    pub(super) async fn place(
        &self,
        input: crate::command::place::PlaceIn,
        claim: Option<crate::idempotency::commit::Claim>,
    ) -> Result<std::result::Result<crate::command::place::PlaceOut, crate::command::Refused>>
    {
        // The projection is what the promo count is taken from -- the same one
        // every reader uses, memoised against this generation, so counting a
        // code's uses does not fold the log a second time.
        let (log_generation, listed) = self.orders_view().await?;
        let mut hub = match self.image(LOG_IMAGE).await? {
            Some((meta, bytes)) => self.load_log(meta.generation, &bytes).map_err(|e| crate::image_err::unreadable("hub", e))?,
            // BORN SMALL, as `append` does it: a hub created at 4 MiB made the
            // third order of the day exceed the isolate's limit.
            None => dowiz_hub::Hub::create_sized(64 * 1024)
                .map_err(|_| Error::RustError("cannot create hub image".into()))?,
        };
        self.arm_scheme(&mut hub);
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
        self.arm_scheme(&mut stock);
        let reserved_before = stock.len();
        stock.set_clock(input.now_ms);

        // THE VENUE'S TAX, from the settings image THIS object holds, at the
        // request's one clock. G2: a venue with a rate never logs an untaxed order.
        let venue_tax = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => match dowiz_hub::settings::Settings::load(&b) {
                Ok(s) => crate::services::ordering::tax_cfg::resolve(|k| s.known(k), input.now_ms),
                Err(_) => Err("tax: the settings image is unreadable".into()),
            },
            None => Ok(None),
        };
        // `stock.low` (W0a): the shelf before, only when a group hears it.
        let low_before = self.low_watch(&stock).await;
        let stored = match crate::command::place::decide(&mut hub, &mut stock, &listed, &venue_tax, &input) {
            Ok(v) => v,
            // NOTHING ELSE IS WRITTEN: only a stock-out's `refused` note (A13, `hubdo/lost.rs`).
            Err(r) => return Ok(Err(self.keep_lost(stock_image, stock_generation, &stock, reserved_before, r).await)),
        };

        let events = hub.len();
        let Some(next) = self.put_log(log_generation, &hub).await?
        else {
            // Cannot happen inside a serialised object -- the generation was
            // read above and nothing else runs between -- but the caller's
            // contract already says what a moved generation means.
            return Ok(Err(crate::command::Refused::Append(
                "the log generation moved during a placement".into(),
            )));
        };
        // THE ORDER EXISTS: its claim is marked committed now (W-FIX O2).
        let out = crate::command::place::PlaceOut { stored: stored.clone(), generation: next, events };
        self.commit_claim(claim.as_ref(), &serde_json::to_string(&out).unwrap_or_default()).await;

        // A venue that models no ingredients reserved none, and writing the
        // stock image to say so would cost a request per order for no change.
        if stock.len() != reserved_before {
            if self
                .put_derived(stock_image, stock_generation, &stock.to_bytes_trimmed())
                .await?
                .is_none()
            {
                // LOUD, AND THE ORDER IS NAMED. The order exists and its
                // ingredients are not held; that is the recoverable direction
                // (see the header) but it is not a silent one.
                log_error!(
                    "stock: order {} was placed and its reservations were NOT written",
                    input.order_id
                );
            }
        }

        // ── THE BELL IS OWED THE MOMENT THE ORDER EXISTS ──
        //
        // Written HERE, in the turn that wrote the order, rather than awaited
        // by the Worker afterwards. `storefront::place` used to `await
        // notify::order_placed` once, with no memory of it: Telegram refuses,
        // or the isolate is cut off at the end of the response, and the kitchen
        // is never told about an order that exists and is paid for.
        //
        // THE TEXT CROSSES AND THE RECIPIENTS DO NOT. Rendering needs the
        // basket lines, which the Worker is holding; WHO it goes to is in the
        // settings image, which this object is holding. So neither side reads
        // an image it did not already have.
        //
        // A FAILURE TO ENQUEUE DOES NOT FAIL THE ORDER. The order is in the log
        // and the customer has been charged; refusing now would lose a placed
        // order to protect a message about it. It is loud instead.
        if let Some(text) = input.notify_text.as_deref() {
            // The stored lines carry each dish's `station` (`bell_route`).
            let lines = crate::bell_route::lines_of(&stored);
            if let Err(e) = self.enqueue_bell(&input.order_id, text, &lines, None, input.now_ms).await {
                log_error!("outbox: order {} was placed and the bell was NOT queued: {e}", input.order_id);
            }
        }

        // THE FISCAL DOCUMENT IS OWED THE MOMENT THE ORDER TAKES MONEY (B6/B8,
        // `hubdo/fiscal.rs`), at a venue with `fiscal.since_ms` set; loud, never fatal.
        match self.enqueue_fiscal(&stored, input.now_ms).await {
            Ok(crate::fiscal::wire::AtPlacement::Refused(r)) => {
                log_error!("fiscal: order {} owes a document and none was built: {r:?}", input.order_id)
            }
            Ok(_) => {}
            Err(e) => log_error!("fiscal: order {} was placed and its document was NOT queued: {e}", input.order_id),
        }

        // THE GROUPS (W0a/W0b): a crossing of a low threshold, and orders now late.
        if let Some(before) = low_before {
            self.tell_low(&before, &stock, input.now_ms).await;
        }
        let mut orders: Vec<(String, String)> = listed.iter().map(|o| (o.order_id.clone(), o.order_json.clone())).collect();
        orders.push((input.order_id.clone(), stored.clone()));
        self.tell_orders(None, &orders, input.now_ms).await;

        // AFTER THE WRITE LANDED, never before.
        self.broadcast(dowiz_hub::EventKind::Placed as u8, &input.order_id, &stored, next);
        Ok(Ok(out))
    }

    /// Queue the kitchen's message for every channel this venue has configured.
    ///
    /// THE RECIPIENTS COME FROM THE SETTINGS IMAGE THIS OBJECT ALREADY HOLDS,
    /// which is the only reason this costs nothing: the Worker would have had
    /// to fetch it.
    ///
    /// ONE ENTRY PER CHANNEL, keyed by order AND channel, so a command replayed
    /// by the idempotency layer writes over its own entry rather than adding a
    /// second message to a kitchen. Idempotency at the queue as well as at the
    /// route.
    ///
    /// A VENUE THAT HAS CONFIGURED NOTHING QUEUES NOTHING, and that is not a
    /// failure: it is most venues, and writing an entry with nowhere to send it
    /// would fill an image with messages that can only ever be abandoned.
    ///
    /// ONE TELEGRAM TICKET PER STATION (`bell_route`, §2.7) when the venue set
    /// `notify.telegram.chat.bar` and a bar line is on the bell; otherwise the
    /// one entry it always was, same id, same text. `lines` are the lines the
    /// bell is about; `amend_seq` is `Some` for an amendment's added lines,
    /// which ring Telegram only — WhatsApp and the print rail announce orders.
    pub(super) async fn enqueue_bell(
        &self,
        order_id: &str,
        text: &str,
        lines: &[serde_json::Value],
        amend_seq: Option<u64>,
        now_ms: i64,
    ) -> Result<()> {
        let settings = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, bytes)) => dowiz_hub::settings::Settings::load(&bytes)
                .map_err(|e| crate::image_err::unreadable("settings", e))?,
            None => return Ok(()),
        };
        let mut queued: Vec<crate::outbox::Entry> = crate::notify::route::bell(&settings, order_id, text, lines, amend_seq, now_ms);
        // ASKED FOR, NOT ASSUMED, exactly as the inline version had it: Meta
        // bills every one of these and Telegram carries the same text for
        // nothing, so a venue that has not set `notify.whatsapp.status = on` is
        // not billed for announcements it never asked for.
        if amend_seq.is_none() && settings.flag("notify.whatsapp.status") {
            if let Some(wa) = crate::channels::whatsapp_cfg(&settings) {
                if !wa.to.is_empty() {
                    queued.push(crate::outbox::Entry::new(
                        format!("{order_id}/whatsapp"),
                        "whatsapp",
                        wa.to.clone(),
                        text.to_string(),
                        now_ms,
                    ));
                }
            }
        }
        // THE KITCHEN'S TICKET, when the venue has a printer (`print_rail.rs`).
        if amend_seq.is_none() {
            queued.extend(crate::print_rail::entry_for(&settings.known(crate::print_rail::SETTING), order_id, text, now_ms));
        }
        if queued.is_empty() {
            return Ok(());
        }

        let image = crate::outbox::IMAGE_OUTBOX;
        let (generation, mut table) = match self.image(image).await? {
            Some((meta, bytes)) => (
                meta.generation,
                dowiz_hub::table::Table::load(&bytes, crate::outbox::OUTBOX_BYTES)
                    .map_err(|e| crate::image_err::unreadable("outbox", e))?,
            ),
            None => (
                0,
                dowiz_hub::table::Table::create(crate::outbox::OUTBOX_BYTES)
                    .map_err(|_| Error::RustError("cannot create outbox image".into()))?,
            ),
        };
        for e in &queued {
            let rec = serde_json::to_string(e).unwrap_or_default();
            table
                .put(crate::outbox::KIND, &e.id, &rec, &[], &[])
                .map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
        }
        let bytes = table
            .to_bytes()
            .map_err(|e| Error::RustError(format!("outbox will not serialise: {e:?}")))?;
        if self.put_image(image, generation, &bytes).await?.is_none() {
            return Err(Error::RustError("the outbox generation moved".into()));
        }
        Ok(())
    }
}
