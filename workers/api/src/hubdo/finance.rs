//! `/fold/finance?venue=&now=&month=` (W-FINANCE): the owner's money sheet, read here where
//! the images are in memory; every number is decided in `services/finance/profit.rs`.

use super::{query, venue_and_now, HubImages};
use crate::services::finance::{self, Expense, FINANCE_BYTES, IMAGE, KIND};
use crate::services::operations::payroll::rules;
use dowiz_hub::table::Table;
use worker::*;
use crate::wire::{Call as Request, Reply as Response};

impl HubImages {
    pub(super) async fn fold_finance(&self, req: &Request) -> Result<Response> {
        let (venue, now) = match venue_and_now(req)? {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
        let month = query(req, "month")?;
        let cat = self.catalogue().await?;
        let (_, listed) = self.orders_view().await?;
        let (_, stock) = self.stock_log().await?;
        let (_, bytes) = self.cube_image().await?;
        let parsed = super::cube::parsed(&bytes);
        let cold = super::cube::rows_of(&parsed);
        let expenses = match self.image(IMAGE).await? {
            Some((_, b)) => expense_rows(&Table::load(&b, FINANCE_BYTES).map_err(|_| Error::RustError("the finance image is unreadable".into()))?)?,
            None => Vec::new(),
        };
        let paid = match self.image(rules::IMAGE).await? {
            Some((_, b)) => {
                let table = Table::load(&b, rules::BYTES).map_err(|_| Error::RustError("the payroll image is unreadable".into()))?;
                rules::parse_all(table.all(rules::K_ENTRY)).map_err(Error::RustError)?
            }
            None => Vec::new(),
        };
        // The same figure as the payroll tab's month total; one too big to add up fails the sheet.
        let payroll = |m: &str| rules::month_total(&paid, m).ok_or_else(|| format!("the payroll of {m} is too big to add up"));
        match finance::profit::answer(listed, &cat, &stock, expenses, &venue, now, month.as_deref(), &cold, &payroll) {
            Ok(v) => Response::from_json(&v),
            Err((status, why)) => Response::error(why, status),
        }
    }
}

/// Every stored entry; a record that does not parse fails the read by name, never dropped.
fn expense_rows(t: &Table) -> Result<Vec<(String, Expense)>> {
    t.all(KIND)
        .into_iter()
        .map(|(id, j)| serde_json::from_str::<Expense>(&j).map(|e| (id.clone(), e)).map_err(|e| Error::RustError(format!("finance entry {id}: {e}"))))
        .collect()
}
