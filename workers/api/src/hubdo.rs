//! The hub's images, in a Durable Object — one object per venue.
//!
//! WHY THIS EXISTS. `hubstore`'s header has named this destination since the
//! first version: every request read the whole hub image out of D1, parsed it,
//! answered, and threw it away, so the cost of asking "what is in the queue"
//! grew with how long the venue had been open. Three things were wrong with
//! that and they are all the same thing — the image had no home that outlived a
//! request.
//!
//!   1. THE READ. A Durable Object is one instance that stays alive between
//!      requests, so the image is already in memory. D1 was a round trip and a
//!      megabyte of hex; this is a field access.
//!   2. THE WRITE. Two Workers reading the same image and writing back would
//!      lose one of the two appends. `hubstore` answers that with a generation
//!      guard and a bounded retry, which is correct but is a retry loop standing
//!      in for serialisation. A Durable Object IS the serialisation: one
//!      instance, one writer, no race to guard against. The guard is kept
//!      anyway — see `put_image` — because a guard that is never needed costs
//!      nothing and a guard that was removed the day it became needed costs an
//!      order.
//!   3. THE TENANT. `id_from_name(location_id)` gives each venue its own object
//!      with its own storage. That is P67's hub-per-tenant, arriving as a
//!      property of where the bytes live rather than as a column everything has
//!      to remember to filter on.
//!
//! NO SQL REACHES THIS MODULE. Storage here is a key/value map addressed by
//! exactly the key we ask for; nothing queries inside an image, which was always
//! true and is now also true of the thing storing it.
//!
//! CHUNKED AT 96 KiB because a Durable Object refuses a single value over 128,
//! the same shape of limit D1 had at a million. The meta record is written LAST,
//! for the same reason `hubstore` writes the head last: a crash between the two
//! leaves the previous generation readable rather than half of the new one.

use std::cell::RefCell;
use std::collections::HashMap;

use worker::*;

/// Under the Durable Object's 128 KiB per-value ceiling with room for the key
/// and the framing, and derived here rather than written as a number so the
/// next person can check it: 96 * 1024.
const CHUNK: usize = 96 * 1024;

/// The image the projections are about. Named here as well as in `hubstore`
/// because this object folds THAT image and no other: a settings image has no
/// orders in it, and a projection of one would be an empty list rather than an
/// error.
const LOG_IMAGE: &str = "log";

/// The room's commands (amend, pay, transfer, the till), in their own file.
mod room;

/// The forget command: redact a customer and append a declaration, in one turn.
pub mod forget;

/// The refund command: move an order to REFUNDING and settle the shelf.
pub mod refund;
mod aggregator;

/// The till link (ebills.al): import, crosswalk, floor, poller state.
mod ebills;

/// The kitchen's "seen" ack: one `Noted`, the broadcast (A13, §2.6).
mod kitchen_ack;
mod idem; // a placement's claim marked committed in its turn (W-FIX O2), `hubdo/idem.rs`

/// The kitchen print rail: poll, job, ack (LAST-MILE §3.1).
mod print;
mod exceptions; // the exception alert (P1-5), `hubdo/exceptions.rs`
mod fiscal; // the fiscal document queued at placement (B6/B8), `hubdo/fiscal.rs`
mod routed; // the groups' messages written in the turn (W0a/W0b), `hubdo/routed.rs`
mod stock_turn; // a stock movement as one turn (W0a), `hubdo/stock_turn.rs`
mod menu; mod publish; // the catalogue projection's routes (R2) and its publish to R2 on every menu write (BN2): `hubdo/{menu,publish}.rs`
mod preps; // the ПФ reads, answered here (R3), `hubdo/preps.rs`
mod basket; // the basket's catalogue nodes and the room's recipes, answered here (BN1), `hubdo/basket.rs`
mod reads; // the owner's and the kitchen's folds over the images, answered here (BN1), `hubdo/reads.rs`
mod catalogue; // the catalogue's derived nodes and the venue's record, answered here (BN1), `hubdo/catalogue.rs`
mod catview; mod logmark; mod chunks; use chunks::changed_chunks; mod atomic; pub(crate) mod compact; mod journal; // the catalogue read in place, crc once per generation (W-ZC); the log's crc once per record (W-HUBCRC); the chunk diff; one atomic write per image and the v2 pin for a rollback (W-ATOMIC); the menu's edit journal in the same write (W-PITR2)
mod facts; // the folds over the log and the catalogue together, answered here (BN1), `hubdo/facts.rs`
mod bulk; // a supplies / recipes spreadsheet as one turn (BN1, BN4's shape), `hubdo/bulk.rs`
mod archives; // the archives' folds for rebuild's R5 crossing, `hubdo/archives.rs`
mod timer; mod edges; // edges: what every /fold/* route derives from what (R-GRAPH D4); timer: the venue's alarm: timed work without the minute cron (DAG Phase 2), `hubdo/timer.rs`
mod push_turn; mod chat; mod sms_turn; mod lost; // sms_turn: the customer's order-status text in the turn (W-SMS); push_turn: the phones' messages in the turn (W-PUSH); chat: the courier chat nudge to the two parties' sockets (W-URGENT), `hubdo/chat.rs`
/// Where the object lives: the platform, or (tests) memory (W-COV C2), `hubdo/host.rs`.
pub(crate) mod host;
/// W-SPLIT (R-GRAPH F10): this file is the object; the table is `hubdo/route.rs`, the images `store.rs`,
/// the sockets `{socket,platform}.rs`, the reads `answers.rs`, the commands `{place,orders,verbs}.rs`.
mod route; mod store; mod socket; mod platform; mod answers; mod verbs; mod orders; mod place; pub(crate) mod command; // command: ONE command, ONE object turn (BN4), `hubdo/command.rs`
pub use socket::{tag_courier, tag_order, TAG_CONSOLE, TAG_COURIER};
use crate::wire::Call;

/// The catalogue image, which holds the venue's own record as well as its
/// dishes. Named here because `/fold/venue` reads it and nothing else does.
const CATALOG_IMAGE: &str = "catalog";

/// How long a courier's fix is worth serving. `live_eta` reads the newest fix
/// within twenty minutes; anything older is not a position, it is a memory.
const POSITION_KEEP_MS: i64 = 20 * 60 * 1000;

/// What a stored image is, apart from its bytes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
struct Meta {
    generation: i64,
    chunks: usize,
    len: usize,
}

mod deltas; pub use deltas::{changes_since, Change}; use deltas::RECENT_KEEP; pub(crate) mod counters; // the catch-up window (`hubdo/deltas.rs`); AX0's counters (`hubdo/counters.rs`)

/// A courier's last known position, held in the object rather than in D1.
///
/// GPS was a D1 ROW PER FIX, kept for nobody: the map reads the newest fix per
/// courier within twenty minutes and nothing else ever reads the table. In
/// memory it is a field; when the object hibernates the fixes are lost, which
/// is exactly right for a value whose meaning expires in minutes.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct Fix {
    pub lat_e6: i64,
    pub lng_e6: i64,
    pub at_ms: i64,
}

/// One order as a projection carries it. MOVED to `dowiz_hub::room::view` (D7 phase 1),
/// so the room's deciders read the same type in the Worker and in wasm.
pub use dowiz_hub::room::view::OrderView;

#[durable_object]
pub struct HubImages {
    /// The platform's `State` and `Env` (or, in tests, memory): `hubdo/host.rs`.
    state: host::Host,
    /// The images this object has in hand. THE WHOLE POINT: a Durable Object
    /// outlives a request, so this survives between them and the storage below
    /// is touched only when the object is cold or something is written.
    mem: RefCell<HashMap<String, (Meta, Vec<u8>)>>,
    /// The catch-up window: recent changes, oldest first. See `changes_since`.
    recent: RefCell<Vec<Change>>,
    /// Where the couriers are, as they last said. Not persisted on purpose:
    /// see `Fix`.
    positions: RefCell<HashMap<String, Fix>>,
    /// The log image, FOLDED, at the generation it was folded from, and
    /// KEPT CURRENT by the writes (R1, `fold::projection`): an append or a
    /// command steps it over the events it added, a whole-image write drops
    /// it. Keyed by generation so it cannot go stale: a generation that does
    /// not match is simply refolded.
    folded: RefCell<Option<crate::fold::projection::Orders>>,
    /// The catalogue, translations and settings FOLDED into the storefront's
    /// menu, per generation of the three (R2, `fold::menu`, `hubdo/menu.rs`).
    menu: RefCell<Option<crate::fold::menu::Memo>>,
    /// An `alarm()` run is under way: the writes it causes do not re-arm,
    /// its end does (`hubdo/timer.rs`).
    in_alarm: std::cell::Cell<bool>,
    /// The catalogue image in `mem` passed the full crc check at this generation (W-ZC, `hubdo/catview.rs`).
    cat_checked: std::cell::Cell<Option<(i64, dowiz_hub::catalog::view::Checked)>>,
    log_marks: RefCell<HashMap<String, (i64, dowiz_hub::LogMark)>>, // what each append log's last scan proved, per image at its generation (W-HUBCRC, `hubdo/logmark.rs`)
    /// Who signed the catalogue write under way, for its journal record (W-PITR2, `hubdo/journal.rs`).
    edit: RefCell<journal::Desk>, // + the journal kept in hand between writes (W-LOOPB, `hubdo/journal/desk.rs`)
    /// AX0: what this wake served, read by `/api/owner/health` (`hubdo/counters.rs`).
    counters: counters::Counters,
}

#[cfg(test)]
mod tests;

impl HubImages {
    /// The object's clock: the platform's, or a test's fixed one. The only
    /// place this file reads it for a write (`clock.sh` exempts `hubdo.rs`).
    fn now_ms(&self) -> i64 {
        self.state.fixed_clock().unwrap_or_else(|| Date::now().as_millis() as i64)
    }

    /// The object over memory, for tests (`hubdo/host/mem.rs`).
    #[cfg(test)]
    pub(crate) fn in_memory(host: std::rc::Rc<host::mem::MemHost>) -> Self {
        Self::with_host(host::Host::Mem(host))
    }

    fn with_host(state: host::Host) -> Self {
        Self {
            state,
            mem: RefCell::new(HashMap::new()),
            recent: RefCell::new(Vec::new()),
            positions: RefCell::new(HashMap::new()),
            folded: RefCell::new(None),
            menu: RefCell::new(None),
            in_alarm: std::cell::Cell::new(false),
            cat_checked: std::cell::Cell::new(None), log_marks: RefCell::new(HashMap::new()),
            edit: RefCell::new(journal::Desk::default()),
            counters: counters::Counters::woke(),
        }
    }

    fn meta_key(id: &str) -> String {
        format!("m:{id}")
    }
    fn chunk_key(id: &str, n: usize) -> String {
        format!("c:{id}:{n}")
    }

    /// `put_image`, saying how a LOG write landed (`fold::projection::Written`).
    ///
    /// THE OBJECT'S CLOCK IS READ HERE, where `clock.sh` allows it: the write
    /// itself is `put_chunks` and the log's tail `log_moved` (`hubdo/store.rs`).
    async fn put_image_as(
        &self,
        id: &str,
        expected: i64,
        bytes: &[u8],
        how: crate::fold::projection::Written,
    ) -> Result<Option<i64>> {
        let Some(next) = self.put_chunks(id, expected, bytes, &how, || self.now_ms()).await? else {
            return Ok(None);
        };
        // TIMED WORK ARMS THE ALARM in the write that makes it due (DAG Phase 2).
        if crate::cron::timer::TIMED.contains(&id) || id == crate::hubstore::IMAGE_SETTINGS { // settings: a rail set up (`outbox/park.rs`)
            self.timer_after_write(id, self.now_ms()).await;
        }
        self.log_moved(id, expected, next, how);
        Ok(Some(next))
    }

    /// The phones' and the customer's texts for an appended event (W-PUSH,
    /// W-SMS), each at the object's clock as `append` read it.
    async fn tell_appended(&self, kind: u8, order_id: &str, payload: &str) {
        self.tell_push_appended(kind, order_id, payload, self.now_ms()).await;
        self.tell_sms_appended(kind, order_id, payload, self.now_ms()).await;
    }

    /// The couriers' fixes still worth serving (`POSITION_KEEP_MS`), at the
    /// object's clock: `/fold/positions` (`hubdo/answers.rs`).
    fn fresh_positions(&self) -> HashMap<String, Fix> {
        let now = self.now_ms();
        self.positions
            .borrow()
            .iter()
            .filter(|(_, f)| now - f.at_ms < POSITION_KEEP_MS)
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// A courier's fix, stamped with the object's clock (`hubdo/socket.rs`).
    fn fix_now(&self, lat_e6: i64, lng_e6: i64) -> Fix {
        Fix { lat_e6, lng_e6, at_ms: self.now_ms() }
    }
}

impl DurableObject for HubImages {
    fn new(state: State, env: worker::Env) -> Self {
        Self::with_host(host::Host::Live { state, env })
    }

    /// The object's whole surface. NOT REACHABLE FROM THE INTERNET: a Durable
    /// Object is addressable only through a stub held by a Worker that has the
    /// binding, so these paths need no authentication of their own — the
    /// handlers that call them have already done it.
    ///
    /// TWO KINDS OF ROUTE, and the difference is phase 2. `/img/...` hands over
    /// BYTES: the catalogue, the settings, a backup, anything whose reader is
    /// not this object. `/fold/...` hands over ANSWERS: the queue, one order,
    /// an append. The bytes route stays because an image still has to be
    /// exportable and importable; the fold route exists so that asking what is
    /// in the queue stops costing the whole history of the venue.
    async fn fetch(&self, req: worker::Request) -> Result<worker::Response> {
        // The cron and the socket upgrade need the platform (`hubdo/platform.rs`).
        if let Some(answer) = self.platform_route(&req).await? {
            return Ok(answer);
        }
        self.route(Call::from_worker(req).await?).await?.into_response()
    }

    /// What a client may say over its socket (`hubdo/socket.rs`).
    async fn websocket_message(&self, ws: WebSocket, message: WebSocketIncomingMessage) -> Result<()> {
        self.socket_frame(&ws, message);
        Ok(())
    }

    /// A socket closing is the ordinary end of a socket. The runtime forgets
    /// it; there is nothing here to clean up, and the default handler would
    /// have panicked.
    async fn websocket_close(&self, _ws: WebSocket, _code: usize, _reason: String, _was_clean: bool) -> Result<()> {
        Ok(())
    }

    /// Same, for a socket that ended badly. Logged, because a storm of these
    /// is worth seeing, and not turned into an error that would take the
    /// object down with it.
    async fn websocket_error(&self, _ws: WebSocket, error: Error) -> Result<()> {
        log_line!("hub socket error: {error}");
        Ok(())
    }

    /// THE VENUE'S TIMED WORK, due now (`hubdo/timer.rs`): the outbox drain,
    /// the till link's poll and the fiscal firing, in place of the minute cron.
    /// The object's own clock read, as `append` stamps an event with one.
    async fn alarm(&self) -> Result<worker::Response> {
        self.timer_alarm(Date::now().as_millis() as i64).await?.into_response()
    }
}
