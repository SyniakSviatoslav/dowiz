//! WHICH SALE IDS THIS FIRING READS, as a pure state machine the poller
//! drives (§4). The list shows bills and counter sales and HIDES the courses
//! of a table (§1.6), so every id between the watermark and the newest listed
//! one that the list did not show is fetched by id -- and a few beyond it,
//! because the courses of a table still open are newer than any listed bill.
//!
//! THE WATERMARK MOVES ONLY OVER WHAT WAS HANDLED, in order: an id fetched,
//! a listed bill taken from its row, or a `404` INSIDE the listed range (a
//! deleted draft: recorded, never retried). Beyond the newest listed id a
//! `404` is simply the end of the sales so far, and the watermark stops
//! before it -- that id may still be rung up.

/// Details fetched per firing, at most: with the login, the floor, the list
/// and the menu this keeps a firing near thirty subrequests.
pub(crate) const BUDGET: u32 = 20;
/// Ids tried beyond the newest listed one, at most.
pub(crate) const PROBE: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// `GET /api/sales/{id}`.
    Fetch(i64),
    /// A bill the list already showed: map it from its row, no request.
    Listed(i64),
    Done,
}

#[derive(Debug, Clone)]
pub(crate) struct Walker {
    next: i64,
    newest: i64,
    bills: Vec<i64>,
    budget: u32,
    /// Every id at or below this has been handled.
    pub(crate) handled: i64,
    /// The budget ran out inside the listed range: poll again next firing.
    pub(crate) backlog: bool,
    /// Ids below this are LEADS: read on a venue's first run, from before
    /// its first window (`state::LEAD_IDS`). 0 when there is none.
    pub(crate) lead_below: i64,
    ended: bool,
}

impl Walker {
    /// `listed` are the list's ids; `bills` the listed ids that are bills.
    /// A FIRST RUN (watermark 0) starts `lead` ids BEFORE the oldest listed
    /// one: the window's first bills were rung up after courses the window
    /// does not reach, and those courses are read as leads -- placed only if
    /// a bill claims them. `lead_below` carries the boundary across firings.
    pub(crate) fn new(watermark: i64, listed: &[i64], bills: &[i64], budget: u32, lead: i64, lead_below: i64) -> Walker {
        let newest = listed.iter().copied().max().unwrap_or(watermark).max(watermark);
        let (start, lead_below) = match (watermark, listed.iter().copied().min()) {
            (0, Some(oldest)) => ((oldest - 1 - lead).max(0), oldest),
            (0, None) => (0, 0),
            (w, _) => (w, lead_below),
        };
        let ended = watermark == 0 && listed.is_empty();
        Walker { next: start + 1, newest, bills: bills.to_vec(), budget, handled: start, backlog: false, lead_below, ended }
    }

    /// Is this id from before the first window?
    pub(crate) fn is_lead(&self, id: i64) -> bool {
        id < self.lead_below
    }

    pub(crate) fn step(&mut self) -> Step {
        if self.ended {
            return Step::Done;
        }
        let id = self.next;
        if id <= self.newest && self.bills.contains(&id) {
            return Step::Listed(id);
        }
        if id > self.newest + PROBE {
            return Step::Done;
        }
        if self.budget == 0 {
            self.backlog = id <= self.newest;
            return Step::Done;
        }
        self.budget -= 1;
        Step::Fetch(id)
    }

    /// The last step's id was handled (fetched, mapped or refused by name).
    pub(crate) fn done(&mut self) {
        self.handled = self.next;
        self.next += 1;
    }

    /// The last step's id answered `404`.
    pub(crate) fn missing(&mut self) {
        if self.next <= self.newest {
            self.done();
        } else {
            self.ended = true;
        }
    }
}

#[cfg(test)]
mod tests;
