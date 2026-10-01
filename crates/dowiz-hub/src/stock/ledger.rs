//! THE LEDGER: the fold of the stock log into levels, reservations and what was
//! served -- `decide` refuses an event, `apply` records one. A projection,
//! rebuilt by fold and never edited in place.

use super::*;

impl StockLedger {
    pub fn level(&self, item: &str) -> StockLevel {
        self.levels
            .iter()
            .find(|(i, _)| i == item)
            .map(|(_, l)| *l)
            .unwrap_or_default()
    }

    pub fn available(&self, item: &str) -> Qty {
        self.level(item).available()
    }

    /// Whether this item's shelf has ever been measured (received or counted).
    /// An uncounted item is reserved and consumed like any other -- its
    /// `on_hand` goes below zero -- but never refuses an order: its level is
    /// "what was used since nobody looked", not what is there.
    pub fn is_counted(&self, item: &str) -> bool {
        self.counted.iter().any(|i| i == item)
    }

    /// Every item this ledger knows about, sorted, so two folds of one log
    /// produce byte-identical output (I4).
    pub fn items(&self) -> Vec<(String, StockLevel)> {
        let mut out = self.levels.clone();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn level_mut(&mut self, item: &str) -> &mut StockLevel {
        if let Some(pos) = self.levels.iter().position(|(i, _)| i == item) {
            return &mut self.levels[pos].1;
        }
        self.levels.push((item.to_string(), StockLevel::default()));
        let last = self.levels.len() - 1;
        &mut self.levels[last].1
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

    fn mark_counted(&mut self, item: &str) {
        if !self.is_counted(item) {
            self.counted.push(item.to_string());
        }
    }

    fn take_held(&mut self, order_id: &str, item: &str, qty: Qty) {
        let key = (order_id.to_string(), item.to_string());
        if let Some(q) = self.open.get_mut(&key) {
            *q -= qty;
            if *q <= 0 {
                self.open.remove(&key);
            }
        }
    }

    pub(super) fn apply(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.decide(ev)?;
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
                *self.open.entry((order_id.clone(), item.clone())).or_insert(0) += qty;
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
                    *q -= qty;
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
