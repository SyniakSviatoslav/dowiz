//! A STOCK-OUT AT CHECKOUT IS WRITTEN DOWN (A13, W-LOST): the one write a
//! refused placement makes. `command::place::decide` left a `refused` note
//! in the stock image it was handed (rate-limited, no person); this puts that
//! image back. Nothing else a refusal staged is ever written: a refusal that
//! is not a stock-out, or a stock image that grew by anything but the one
//! note, is dropped exactly as before.
//!
//! LOUD, NEVER FATAL: the guest's answer is the refusal whatever happens
//! here, and a note that did not land is a lost sale the screen undercounts,
//! said in the log.

use super::HubImages;
use crate::command::Refused;
use dowiz_hub::stock::StockLog;

/// Write the stock image only for a stock-out that added exactly one record.
pub(super) fn worth_writing(r: &Refused, before: usize, after: usize) -> bool {
    matches!(r, Refused::Stock(_)) && after == before + 1
}

impl HubImages {
    pub(super) async fn keep_lost(&self, image: &str, generation: i64, stock: &StockLog, before: usize, r: Refused) -> Refused {
        if !worth_writing(&r, before, stock.len()) {
            return r;
        }
        match self.put_image(image, generation, &stock.to_bytes_trimmed()).await {
            Ok(Some(_)) => {}
            Ok(None) => log_error!("stock: a stock-out's lost-sale note was NOT written (the generation moved)"),
            Err(e) => log_error!("stock: a stock-out's lost-sale note was NOT written: {e}"),
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_stock_out_that_added_one_note_is_written() {
        assert!(worth_writing(&Refused::Stock("salmon: 40 wanted, 30 available".into()), 5, 6));
        assert!(!worth_writing(&Refused::Stock("rate-limited".into()), 5, 5), "no note: nothing to write");
        assert!(!worth_writing(&Refused::Promo("expired".into()), 5, 9), "a promo refusal after staged reservations is dropped");
        assert!(!worth_writing(&Refused::Stock("x".into()), 5, 8), "more than the note: dropped");
    }
}
