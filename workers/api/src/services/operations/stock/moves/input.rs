//! THE STOCK DOOR'S BODY (`POST /api/owner/stock/:kind`), strict: every key
//! a movement, a card, a transfer or a freezing record may carry, and no
//! other -- `deny_unknown_fields` turns a forged `by` into a 400.

use serde::Deserialize;
use serde_json::Value;

/// One counted line of a session.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct CountLineIn {
    pub item: String,
    pub observed: i64,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StockMoveIn {
    #[serde(default)]
    pub item: String,
    #[serde(default)]
    pub qty: Option<i64>,
    /// For a stocktake: what was actually counted.
    #[serde(default)]
    pub observed: Option<i64>,
    /// For waste: one of `WasteReason::allowed_words()`. Required.
    #[serde(default)]
    pub reason: Option<String>,
    // NO `by`. THE SIGNER IS WHO AUTHENTICATED, never a field the caller
    // fills in: a body that could name its signer is a write-off anybody can
    // put on somebody else. `deny_unknown_fields` turns a `by` into a 400.
    /// A delivery's price: minor units per `per` base units ...
    #[serde(default)]
    pub unit_cost: Option<i64>,
    #[serde(default)]
    pub per: Option<i64>,
    /// ... or the invoice line's total for the whole `qty`.
    #[serde(default)]
    pub total: Option<i64>,
    #[serde(default)]
    pub supplier: Option<String>,
    /// The invoice / delivery note number.
    #[serde(default)]
    pub doc: Option<String>,
    #[serde(default)]
    pub lot: Option<String>,
    /// `yyyy-mm-dd`, the label's date.
    #[serde(default)]
    pub expiry: Option<String>,
    /// A session's lines.
    #[serde(default)]
    pub lines: Option<Vec<CountLineIn>>,
    #[serde(default)]
    pub session: Option<String>,
    /// Prep: what came off the board, and after which stage.
    #[serde(default)]
    pub out: Option<i64>,
    #[serde(default)]
    pub stage: Option<String>,
    /// Prep into ANOTHER stocked supply (whole fish -> fillet).
    #[serde(default)]
    pub into: Option<String>,
    /// `as-is`: the dishes to link to their own piece (`as_is`).
    #[serde(default)]
    pub products: Option<Vec<String>>,
    /// `removed`: the supplies deleted from the nomenclature (`removed`).
    #[serde(default)]
    pub items: Option<Vec<String>>,
    /// `supplier` / `ordered`: a supplier's card or an order sent (`suppliers`, W-STOCK P5).
    /// Declared so the strict parse admits it; `suppliers::run` reads it off the raw body.
    #[allow(dead_code)]
    #[serde(default)]
    pub card: Option<Value>,
    /// P12: the storage a delivery goes into, a write-off or prep happens
    /// in, or a count was taken in (`dowiz_hub::stock::storages`).
    #[serde(default)]
    pub store: Option<String>,
    /// P12 `moved`: the transfer's two storages. Declared so the strict parse
    /// admits them; `storages::run` reads them off the raw body.
    #[allow(dead_code)]
    #[serde(default)]
    pub from: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub to: Option<String>,
    /// P13: a delivery the supplier already treated (frozen at source): its paper.
    #[serde(default)]
    pub treated: Option<String>,
    /// P13 `frozen`: how long and how cold a lot was frozen in-house.
    #[allow(dead_code)]
    #[serde(default)]
    pub hours: Option<i64>,
    #[allow(dead_code)]
    #[serde(default)]
    pub temp_c: Option<i64>,
    /// W-STORE2 `frozen`: when the freezing started, the venue's local
    /// `yyyy-mm-ddThh:mm`; `storages::run` reads it off the raw body.
    #[allow(dead_code)]
    #[serde(default)]
    pub started: Option<String>,
    /// W-STORE2 `frozen`: when the freezing ended, the same local text.
    #[allow(dead_code)]
    #[serde(default)]
    pub ended: Option<String>,
    /// W-STORE2 `bound`: the kitchen station bound to `store`.
    #[allow(dead_code)]
    #[serde(default)]
    pub station: Option<String>,
}

