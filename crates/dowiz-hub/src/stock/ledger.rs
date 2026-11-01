//! THE LEDGER: the fold of the stock log into levels, reservations and what was
//! served -- `decide` refuses an event, `apply` records one. A projection,
//! rebuilt by fold and never edited in place.

use super::*;

impl StockLedger {
    pub fn level(&self, item: &str) -> StockLevel {
        self.level_at.get(item).and_then(|i| self.levels.get(*i)).map(|(_, l)| *l).unwrap_or_default()
    }

    pub fn available(&self, item: &str) -> Qty {
        self.level(item).available()
    }

    /// Whether this item's shelf has ever been measured (received or counted).
    /// An uncounted item is reserved and consumed like any other -- its
    /// `on_hand` goes below zero -- but never refuses an order: its level is
    /// "what was used since nobody looked", not what is there.
    pub fn is_counted(&self, item: &str) -> bool {
        self.counted_set.contains(item)
    }

    /// Every item this ledger knows about, sorted, so two folds of one log
    /// produce byte-identical output (I4).
    pub fn items(&self) -> Vec<(String, StockLevel)> {
        let mut out = self.levels.clone();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// A new item, last in first-seen order: where it stands.
    pub(super) fn place(&mut self, item: String, level: StockLevel) -> usize {
        let at = self.levels.len();
        self.level_at.insert(item.clone(), at);
        self.levels.push((item, level));
        at
    }

    fn level_mut(&mut self, item: &str) -> &mut StockLevel {
        let at = match self.level_at.get(item) {
            Some(i) => *i,
            None => self.place(item.to_string(), StockLevel::default()),
        };
        &mut self.levels[at].1
    }

    fn served_qty(&self, order_id: &str, item: &str) -> Qty {
        self.served.get(&(order_id.to_string(), item.to_string())).copied().unwrap_or(0)
    }

    /// What an order has served and not had reversed, `(item, qty)`, sorted:
    /// exactly the `Unserved` a void of it emits.
    pub fn served_of(&self, order_id: &str) -> Vec<(String, Qty)> {
        // The map is ordered by (order, item): one order's lines are one
        // contiguous, item-sorted range.
        self.served
            .range((order_id.to_string(), String::new())..)
            .take_while(|((o, _), _)| o == order_id)
            .map(|((_, i), q)| (i.clone(), *q))
            .collect()
    }

    fn held(&self, order_id: &str, item: &str) -> Qty {
        self.open.get(&(order_id.to_string(), item.to_string())).copied().unwrap_or(0)
    }

    pub(super) fn mark_counted(&mut self, item: &str) {
        if self.counted_set.insert(item.to_string()) {
            self.counted.push(item.to_string());
        }
    }

    fn take_held(&mut self, order_id: &str, item: &str, qty: Qty) {
        let key = (order_id.to_string(), item.to_string());
        if let Some(q) = self.open.get_mut(&key) {
            *q = q.saturating_sub(qty);
            if *q <= 0 {
                self.open.remove(&key);
            }
        }
    }

    pub(super) fn apply(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.decide(ev)?;
        self.record(ev)
    }

    /// `apply` without the door: what happened, as it happened. Only the arithmetic can refuse.
    fn record(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        match ev {
            StockEvent::Received { item, qty } => {
                // THE FIRST DELIVERY ARMS THE ITEM AT WHAT CAME IN, as it
                // always did: an uncounted item's negative is use nobody
                // measured against, not a debt this box of salmon pays.
                let was_counted = self.is_counted(item);
                self.mark_counted(item);
                let l = self.level_mut(item);
                if !was_counted {
                    l.on_hand = l.on_hand.max(0);
                }
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Reserved { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.reserved = l.reserved.checked_add(*qty).ok_or(StockError::Overflow)?;
                let q = self.open.entry((order_id.clone(), item.clone())).or_insert(0);
                *q = q.saturating_add(*qty);
            }
            StockEvent::Consumed { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
                l.reserved = l.reserved.checked_sub(*qty).ok_or(StockError::Overflow)?;
                self.take_held(order_id, item, *qty);
            }
            StockEvent::Released { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.reserved = l.reserved.checked_sub(*qty).ok_or(StockError::Overflow)?;
                self.take_held(order_id, item, *qty);
            }
            StockEvent::Wasted { item, qty, .. } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Stocktake { item, observed, .. } => {
                // The basis resets here (I2): everything before this count
                // stops contributing to on_hand. Reservations survive, because
                // they are promises to customers, not a property of the shelf.
                self.mark_counted(item);
                let l = self.level_mut(item);
                l.on_hand = *observed;
            }
            StockEvent::Served { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
                let q = self.served.entry((order_id.clone(), item.clone())).or_insert(0);
                *q = q.saturating_add(*qty);
            }
            StockEvent::Unserved { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
                let key = (order_id.clone(), item.clone());
                if let Some(q) = self.served.get_mut(&key) {
                    *q = q.saturating_sub(*qty);
                    if *q <= 0 {
                        self.served.remove(&key);
                    }
                }
            }
            StockEvent::Returned { item, qty, resell, .. } => {
                if *resell {
                    let l = self.level_mut(item);
                    l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
                }
            }
            StockEvent::Produced { item, qty, out, into, .. } => {
                if let Some(to) = moved_into(item, into).map(str::to_string) {
                    let l = self.level_mut(item);
                    l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
                    // What came off the board was weighed: a measurement.
                    self.mark_counted(&to);
                    let l = self.level_mut(&to);
                    l.on_hand = l.on_hand.checked_add(*out).ok_or(StockError::Overflow)?;
                }
            }
            StockEvent::Removed { item, .. } => self.forget(item),
            StockEvent::Cooked { item, qty, .. } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Made { item, qty, .. } => {
                // The batch was weighed: a measurement, like a delivery's.
                let was_counted = self.is_counted(item);
                self.mark_counted(item);
                let l = self.level_mut(item);
                if !was_counted {
                    l.on_hand = l.on_hand.max(0);
                }
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
        }
        Ok(())
    }

    /// Pure, checked, deterministic. The only way to obtain a ledger.
    pub fn fold(events: &[StockEvent]) -> Result<StockLedger, StockError> {
        let mut led = StockLedger::default();
        for ev in events {
            led.apply(ev)?;
        }
        Ok(led)
    }

    /// THE FOLD OF A HISTORY, door open (W-MG-MERGE2). `fold` runs `decide` over every event, which
    /// is right for a log one door wrote and wrong for a merged one: two offline devices each pass
    /// their own door, and the union holds what no door would have let through. A named conflict is
    /// such a history, and gluing detects and never resolves, so refusing to read it locked the file
    /// for good (`atk5_stuck`). This is not a write path: nothing here says a movement was allowed.
    pub fn history(events: &[StockEvent]) -> Result<StockLedger, StockError> {
        let mut led = StockLedger::default();
        for ev in events {
            led.record(ev)?;
        }
        Ok(led)
    }

    /// The largest magnitude any level, hold or served line holds: what a sum must start from.
    pub fn peak(&self) -> i128 {
        let wide = |q: Qty| (q as i128).abs();
        let levels = self.levels.iter().map(|(_, l)| wide(l.on_hand).max(wide(l.reserved)));
        let lines = self.open.values().chain(self.served.values()).map(|q| wide(*q));
        levels.chain(lines).max().unwrap_or(0)
    }

    /// I3, checked over a whole log: every reservation is eventually matched.
    ///
    /// Returns the ones that are not. A stranded reservation is stock a venue
    /// believes it owes to an order that ended -- the slow leak that makes a
    /// kitchen think it is out of something it has.
    pub fn stranded(&self) -> Vec<(String, String, Qty)> {
        let mut out: Vec<(String, String, Qty)> = self
            .open
            .iter()
            .map(|((o, i), q)| (o.clone(), i.clone(), *q))
            .collect();
        out.sort();
        out
    }
}

mod decide;
