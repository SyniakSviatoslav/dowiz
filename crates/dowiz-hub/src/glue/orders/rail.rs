//! SHEAF S2b, THE TABLET'S SIDE: the rail it last read and the taps it queued (`super` header).
//! The write door is the order machine; nothing here touches the network or the order log.

use dowiz_core::order_machine::{assert_transition, OrderStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Kind, Op, NEXT, TAIL_MAX};

/// One ticket as the tablet's rail holds it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub seen: bool,
}

/// THE TABLET'S STALK FOR ORDERS: the rail it last read, and the taps not yet sent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rail {
    pub tickets: Vec<Ticket>,
    pub queue: Vec<String>,
    /// The next op number (op ids are `<at>.<n>`).
    #[serde(default)]
    pub n: u64,
}

/// Why the tablet refused a tap, before queueing anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Local {
    /// The order is not on this tablet's rail.
    NotOnRail,
    /// Not one of the four bumps a pass may take offline.
    NotABump(String),
    /// The order machine's refusal: its code and its words.
    Illegal { code: &'static str, words: String },
    /// `TAIL_MAX` taps wait already: sync first.
    Full,
}

impl Local {
    pub fn words(&self) -> String {
        match self {
            Local::NotOnRail => "this order is not on the tablet's rail".into(),
            Local::NotABump(s) => format!("{s}: not a step the pass takes offline"),
            Local::Illegal { code, words } => format!("{code}: {words}"),
            Local::Full => format!("{TAIL_MAX} taps wait for a sync"),
        }
    }
}

impl Rail {
    /// The rail from `GET /api/staff/kitchen`'s `orders` (tickets: `id`, `status`, `kitchen.seen`),
    /// or from a rail's own `tickets` (`seen: true`).
    pub fn tickets_of(orders: &Value) -> Vec<Ticket> {
        orders
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| {
                        let id = t.get("id")?.as_str()?.to_string();
                        let status = t.get("status")?.as_str()?.to_string();
                        Some(Ticket { id, status, seen: t.pointer("/kitchen/seen/at").is_some() || t.get("seen") == Some(&Value::Bool(true)) })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The op on this rail: `Ok(true)` it moved a ticket, `Ok(false)` nothing to do (seen already).
    fn apply(&mut self, op: &Op) -> Result<bool, Local> {
        let t = self.tickets.iter_mut().find(|t| t.id == op.order).ok_or(Local::NotOnRail)?;
        match &op.kind {
            Kind::Ack if t.seen => Ok(false),
            Kind::Ack => {
                t.seen = true;
                Ok(true)
            }
            Kind::Advance { next, .. } => {
                let bad = |e: dowiz_core::order_machine::TransitionError| Local::Illegal { code: e.code(), words: e.message() };
                let from = OrderStatus::from_str(&t.status).ok_or(Local::NotOnRail)?;
                let to = OrderStatus::from_str(next).ok_or(Local::NotABump(next.clone()))?;
                assert_transition(from, to).map_err(bad)?;
                t.status = next.clone();
                Ok(true)
            }
        }
    }

    /// RECORD ONE TAP: `next` is a bump's status, `None` the ack. Decided against the rail by the
    /// order machine; on `Ok(Some(line))` the ticket moved and the line is queued.
    pub fn record(&mut self, order: &str, next: Option<&str>, at: i64) -> Result<Option<String>, Local> {
        if self.queue.len() >= TAIL_MAX {
            return Err(Local::Full);
        }
        let kind = match next {
            None => Kind::Ack,
            Some(n) if NEXT.contains(&n) => {
                let from = self.tickets.iter().find(|t| t.id == order).ok_or(Local::NotOnRail)?.status.clone();
                Kind::Advance { from, next: n.into() }
            }
            Some(n) => return Err(Local::NotABump(n.into())),
        };
        let op = Op { oid: format!("{}.{}", at.max(0), self.n), order: order.into(), kind, at: at.max(0) };
        if !self.apply(&op)? {
            return Ok(None);
        }
        self.n += 1;
        let line = op.line();
        self.queue.push(line.clone());
        Ok(Some(line))
    }

    /// AFTER A SYNC (or any fresh read): the first `sent` taps are the hub's now; the rail is what
    /// the hub shows, with the taps still queued laid over it. A queued tap that no longer applies
    /// stays queued -- the hub decides it, and a refusal comes back as a decision, not a loss.
    pub fn refresh(&mut self, tickets: Vec<Ticket>, sent: usize) {
        self.queue.drain(..sent.min(self.queue.len()));
        self.tickets = tickets;
        for (i, l) in self.queue.clone().iter().enumerate() {
            if let Ok(op) = Op::parse(l, i) {
                let _ = self.apply(&op);
            }
        }
    }
}
