//! THE OBJECT'S ROUTE TABLE (split out of `hubdo.rs`, W-SPLIT): every `/fold/*`
//! route but the two that need the platform itself (`hubdo/platform.rs`), one
//! arm each, and the `/img/*` bytes. The arms only dispatch; what each route
//! does lives with its family. `edges/tests.rs` reads this table out of the
//! source and holds it to `hubdo/edges.rs`, row for route.

use super::HubImages;
use crate::wire::{Call, Reply};
use worker::*;

impl HubImages {
    /// Every route but the two that need the platform itself (the runner's
    /// cron and the socket upgrade, which stay in `fetch`), over a `Call` so
    /// the whole surface runs under `cargo test` (W-COV C2, `crate::wire`).
    pub(crate) async fn route(&self, req: Call) -> Result<Reply> {
        use Reply as Response;
        let path = req.path();
        let mut seg = path.split('/').filter(|s| !s.is_empty());
        let head = seg.next().unwrap_or("");
        if head == "fold" {
            let what = seg.next().unwrap_or("");
            if req.method() == Method::Get { self.counters.read(); }
            return match (req.method(), what) {
                (Method::Get, "counters") => self.counters_route(&req), // AX0 (`hubdo/counters.rs`)
                // The nightly's safety net: re-arm a lost alarm (`hubdo/timer.rs`).
                (Method::Post, "timer") => self.timer_route(req).await,
                (Method::Post, "compact") => self.compact_route(&req).await, // KV images back to v2 before a rollback (W-ATOMIC, `hubdo/compact.rs`)
                // Where the couriers are, as they last said over their sockets (`hubdo/answers.rs`).
                (Method::Get, "positions") => self.answer_positions(),
                // THE VENUE'S OWN RECORD, and nothing else in the catalogue (`hubdo/catalogue.rs`).
                (Method::Get, "venue") => self.fold_venue().await,
                // THE STOREFRONT'S MENU AND THE PRODUCTS AN ESTIMATE NEEDS, folded here once per catalogue
                // generation and answered as bytes (R2, `hubdo/menu.rs`), instead of the catalogue image.
                (Method::Get, "menu") => self.fold_menu(&req).await,
                (_, "publish") => self.publish_route(&req).await, // what R2 holds, or publish now (BN2, `hubdo/publish.rs`)
                (Method::Get, "products") => self.fold_products(&req).await,
                (Method::Get, "preps") => self.fold_preps(&req).await,
                // THE CATALOGUE READS OF BN1, answered from the images here (`hubdo/reads.rs`).
                (Method::Get, "basket" | "analytics" | "kitchen" | "stock" | "exceptions" | "week_top" | "prep" | "haccp") => self.fold_read(what, &req).await, // haccp: unreachable until D4's edge test named it
                // THE CATALOGUE'S DERIVED NODES (`hubdo/catalogue.rs`) and the folds over the log and the catalogue together (`hubdo/facts.rs`).
                (Method::Get, "catalogue") => self.fold_catalogue(&req).await,
                (Method::Get, "reveals" | "assist" | "graph" | "kitchen_facts" | "waste") => self.fold_facts(what, &req).await,
                // A SUPPLIES / RECIPES SPREADSHEET, judged and applied in one turn (`hubdo/bulk.rs`).
                (Method::Post, "bulk") => self.bulk_route(req).await,
                // THE GENERATION, WHAT CHANGED SINCE, THE QUEUE, ONE ORDER (`hubdo/answers.rs`).
                (Method::Get, "generation") => self.answer_generation().await,
                (Method::Get, "changes") => self.answer_changes(&req).await,
                (Method::Get, "orders") => self.answer_orders().await,
                (Method::Get, "order") => self.answer_order(&req).await,
                // ── THE COMMAND SURFACE ──
                //
                // `append` below is the Worker's decision being recorded;
                // `place` is the object making one. That is the difference
                // `BLUEPRINT-ARCHITECTURE-EVOLUTION` P1 is about, and it is why
                // that route takes no `x-generation`: there is no read-modify-
                // write over a hop to guard, because the read and the write are
                // two statements inside the same turn. Each command's body is
                // parsed and its refusal answered in `hubdo/verbs.rs`.
                // THE ROOM (`hubdo/room.rs`): `/fold/room/<command>`.
                // THE PRINT RAIL (`hubdo/print.rs`): `/fold/print/<what>`.
                (Method::Post, "print") => self.print(seg.next().unwrap_or(""), req).await,
                (Method::Post, "room") => {
                    let cmd = seg.next().unwrap_or("").to_string();
                    self.room(&cmd, req).await
                }
                // THE CHAT NUDGE (`hubdo/chat.rs`): the two parties' sockets, never the console or the kitchen.
                (Method::Post, "chat") => self.chat_nudge(&req),
                // FORGET A CUSTOMER: `/fold/forget`
                (Method::Post, "forget") => self.forget_route(req).await,
                // THE TILL LINK (`hubdo/ebills.rs`): `/fold/ebills/<command>`.
                (Method::Post, "ebills") => self.ebills(seg.next().unwrap_or(""), req).await,
                // REFUND AN ORDER: `/fold/refund`
                (Method::Post, "refund") => self.refund_route(req).await,
                // AN AGGREGATOR ORDER ENTERED BY HAND: `/fold/aggregator`
                (Method::Post, "aggregator") => self.aggregator_route(req).await,
                // THE FOOD A REFUSED DELIVERY BROUGHT BACK: `/fold/returned`
                (Method::Post, "returned") => self.returned_route(req).await,
                // A STOCK MOVEMENT (W0a): `/fold/stock_move`
                (Method::Post, "stock_move") => self.stock_move_route(req).await,
                // THE OWNER'S INGREDIENTS RESET: `/fold/stock_reset`
                (Method::Post, "stock_reset") => self.stock_reset().await,
                // SHEAF S2: A KITCHEN TABLET'S SYNC, and the decisions it leaves (`hubdo/glue_turn.rs`).
                (Method::Post, "glue") => { let mut req = req; self.glue_route(crate::body::parse(&mut req).await?).await }
                (Method::Post, "desk") => { let mut req = req; self.desk_route(crate::body::parse(&mut req).await?).await }
                (Method::Post, "glue_orders") => { let mut req = req; self.glue_orders_route(crate::body::parse(&mut req).await?).await } // S2b: a tablet's offline order taps
                // THE KITCHEN SAW THE TICKET: `/fold/kitchen_ack`
                (Method::Post, "kitchen_ack") => self.kitchen_ack_route(req).await,
                (Method::Post, "place") => self.place_route(req).await,
                // A WHOLE REQUEST RUN IN THIS TURN (BN4, `hubdo/command.rs`): one object request per placement.
                (Method::Post, "command") => self.command_route(req).await,
                // THE REBUILD, read-only, for the conservation audit's law 8 (`hubdo/answers.rs`).
                (Method::Get, "rebuild") => Response::from_json(&self.rebuild().await?),
                (Method::Post, "advance") => self.advance_route(req).await,
                (Method::Post, "assign") => self.assign_route(req).await,
                (Method::Post, "append") => self.append_route(req).await, // `hubdo/orders.rs`
                _ => Response::error("no such projection", 404),
            };
        }
        // THE BYTES OF ONE IMAGE, by its id (`hubdo/answers.rs`).
        self.image_route(&path, req).await
    }
}
