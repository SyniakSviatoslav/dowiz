//! THE WIRE SHAPES ebills.al answers with, as measured on 2026-09-22
//! (`docs/design/BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md` §1.5, §1.6).
//!
//! Fields the mapping does not read are not declared: an unknown key is
//! ignored by serde, a missing DECLARED key is a parse error. That is the
//! asymmetry wanted at a border with no OpenAPI document (§1.3: it exists and
//! is `403` to this account) -- the platform may add fields freely, and the
//! day it renames one this crate refuses loudly instead of mapping `null`.
//!
//! WHAT IS DELIBERATELY ABSENT. `client.name / nipt / address`,
//! `extraUser.operatorCode`, the floor's `server` (a person's name) and
//! `activeUser`, `business.certificate` and `keystorePass` (§1.7): none has a
//! field here, so none can be kept by accident (§6.8, §6.9).

use serde::Deserialize;

/// A sale as `GET /api/sales` lists it and `GET /api/sales/{id}` wraps it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Sale {
    pub(crate) id: i64,
    pub(crate) inv_ord_num: i64,
    pub(crate) uuid: String,
    pub(crate) fic: Option<String>,
    pub(crate) timestamp: String,
    pub(crate) status: String,
    /// The platform's own spelling of "fiscal status"; renamed here so the
    /// typo lives in exactly one place.
    #[serde(rename = "fiscalSatus")]
    pub(crate) fiscal_status: String,
    pub(crate) draft: i64,
    pub(crate) summary_invoice: bool,
    pub(crate) payment_method: String,
    pub(crate) total_value: f64,
    pub(crate) currency_rate: f64,
    pub(crate) currency: Option<Currency>,
    pub(crate) sale_unit: Option<SaleUnit>,
    pub(crate) sale_unit_order: Option<SaleUnitOrder>,
    /// `null` on the LIST, the lines on the DETAIL (§1.5). `Option`, so a
    /// list row mapped by mistake fails as `NoLines`, not as a parse error.
    #[serde(default)]
    pub(crate) sale_records: Option<Vec<SaleRecord>>,
    #[serde(default)]
    pub(crate) log_cis: Option<Vec<LogCis>>,
    pub(crate) point_of_sale: Option<Ref>,
    pub(crate) extra_user: Option<Ref>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Currency {
    pub(crate) currency_code: String,
}

/// The table (`type: TABLE | ROOM | OTHER`); `identifier` is the number
/// painted on it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaleUnit {
    pub(crate) identifier: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
}

/// Present on a COURSE, `null` on a bill and on a counter sale (§1.6).
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SaleUnitOrder {
    pub(crate) id: i64,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaleRecord {
    pub(crate) item_name: String,
    pub(crate) amount: f64,
    pub(crate) price: f64,
    pub(crate) total_value: f64,
    pub(crate) vat: Option<String>,
    pub(crate) discount: Option<f64>,
    pub(crate) item_in_sale: Option<ItemRef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemRef {
    pub(crate) item_code: Option<String>,
}

/// One row of the fiscalisation log. `iic` is the issuer's hash, `fic` the
/// tax authority's receipt; `status` is `SUCCESS` or the sale never happened
/// fiscally.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LogCis {
    pub(crate) iic: Option<String>,
    pub(crate) fic: Option<String>,
    pub(crate) status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Ref {
    pub(crate) id: i64,
}

/// `GET /api/sales` -- `{"sales":[...],"total":32850.0}`; the row count is
/// in the `X-Total-Count` header, which is the caller's to read.
#[derive(Debug, Deserialize)]
pub(crate) struct SaleList {
    pub(crate) sales: Vec<Sale>,
    pub(crate) total: f64,
}

/// `GET /api/sales/{id}` -- a wrapper whose every top-level field but `sale`
/// is null (§1.5). Only `sale` is read.
#[derive(Debug, Deserialize)]
pub(crate) struct Detail {
    pub(crate) sale: Sale,
}

/// One table on the live floor, `GET /api/sale-units-tables?pointOfSaleId=`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TableState {
    pub(crate) id: i64,
    pub(crate) identifier: String,
    pub(crate) status: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    /// The unpaid running total while `OCCUPIED`; `null` otherwise.
    pub(crate) order_total: Option<f64>,
    pub(crate) point_of_sale_id: i64,
}

impl TableState {
    /// Someone is sitting there with an unpaid order. `RESERVED` is not
    /// occupied, and `ACTIVE` is the platform's word for free (§1.6).
    pub(super) fn occupied(&self) -> bool {
        self.status == "OCCUPIED"
    }
}
