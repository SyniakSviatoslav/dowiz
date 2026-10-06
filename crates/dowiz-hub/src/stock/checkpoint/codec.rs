//! A checkpoint's text: a JSON header line, then space-separated tokens --
//! strings as `<bytes>:<text>` so no name can forge a token, `-` for an
//! absent value -- in the one order the fold keeps its lists.

use crate::minijson::int_field;
use crate::stock::cost::Pool;
use crate::stock::journal::Journal;
use crate::stock::lots::Lot;
use crate::stock::StockLevel;

pub(in crate::stock) const HEAD: &str = r#"{"k":"checkpoint","v":1"#;

struct W(String);

impl W {
    fn n(&mut self, v: impl std::fmt::Display) {
        self.0.push(' ');
        self.0.push_str(&v.to_string());
    }
    fn s(&mut self, v: &str) {
        self.n(format!("{}:{v}", v.len()));
    }
    fn on(&mut self, v: Option<impl std::fmt::Display>) {
        match v {
            Some(x) => self.n(x),
            None => self.n("-"),
        }
    }
    fn os(&mut self, v: &Option<String>) {
        match v {
            Some(x) => self.s(x),
            None => self.n("-"),
        }
    }
}

/// The state, as the tokens a checkpoint stores. Deterministic: every list in
/// the order the fold keeps it.
pub(in crate::stock) fn body(j: &Journal) -> String {
    let mut w = W(String::new());
    let led = &j.ledger;
    w.n("L");
    w.n(led.levels.len());
    for (i, l) in &led.levels {
        w.s(i);
        w.n(l.on_hand);
        w.n(l.reserved);
    }
    for (tag, list) in [("O", &led.open), ("S", &led.served)] {
        w.n(tag);
        w.n(list.len());
        for ((o, i), q) in list {
            w.s(o);
            w.s(i);
            w.n(q);
        }
    }
    w.n("C");
    w.n(led.counted.len());
    for i in &led.counted {
        w.s(i);
    }
    w.n("B");
    w.n(j.book.pools.len());
    for (i, p) in &j.book.pools {
        w.s(i);
        w.n(p.qty);
        w.n(p.value);
        w.on(p.last_avg);
    }
    w.n("T");
    w.n(j.lots.lots.len());
    for l in &j.lots.lots {
        w.s(&l.item);
        w.s(&l.code);
        w.on(l.expiry);
        w.os(&l.supplier);
        w.os(&l.doc);
        w.on(l.at);
        w.on(l.unit_cost);
        w.on(l.per);
        w.n(l.received);
        w.n(l.left);
        w.n(l.seq);
    }
    w.n("J");
    w.n(j.seen);
    w.on(j.max_at);
    w.n(j.undated.plain);
    w.n(j.undated.by_order.len());
    for (o, c) in &j.undated.by_order {
        w.s(o);
        w.n(c);
    }
    // THE CARRY (SPEC-SEMI-FINISHED §c), written ONLY when there is one: a
    // checkpoint on a log with no fractional draw is byte-identical to the
    // one written before this section existed, so `verify_checkpoints` still
    // holds every checkpoint already on a live venue.
    if !j.carry.rem.is_empty() || !j.carry.open.is_empty() {
        w.n("X");
        w.n(j.carry.rem.len());
        for (i, r) in &j.carry.rem {
            w.s(i);
            w.n(r);
        }
        w.n(j.carry.open.len());
        for ((o, i), (uq, q)) in &j.carry.open {
            w.s(o);
            w.s(i);
            w.n(uq);
            w.n(q);
        }
    }
    // THE STORAGES (P12), written ONLY once a record named one: an old
    // venue's checkpoint stays byte-identical (`oldimage_tests.rs`).
    if j.stores.used {
        w.n("P");
        w.n(j.stores.levels.len());
        for ((i, s), q) in &j.stores.levels {
            w.s(i);
            w.s(s);
            w.n(q);
        }
        w.n(j.stores.last.len());
        for (i, s) in &j.stores.last {
            w.s(i);
            w.s(s);
        }
    }
    // THE BINDINGS AND THE BOUND RESERVATIONS (W-STORE2), each written ONLY
    // when non-empty: every checkpoint of a venue that never bound a station
    // keeps its bytes.
    if !j.stores.bound.is_empty() {
        w.n("K");
        w.n(j.stores.bound.len());
        for (st, s) in &j.stores.bound {
            w.s(st);
            w.s(s);
        }
    }
    if !j.stores.held.is_empty() {
        w.n("B");
        w.n(j.stores.held.len());
        for ((o, i), shares) in &j.stores.held {
            w.s(o);
            w.s(i);
            w.n(shares.len());
            for (s, u) in shares {
                w.s(s);
                w.n(u);
            }
        }
    }
    w.0
}

struct R<'a>(&'a str);

impl<'a> R<'a> {
    fn tok(&mut self) -> Option<&'a str> {
        let t = self.0.strip_prefix(' ')?;
        let end = t.find(' ').unwrap_or(t.len());
        self.0 = &t[end..];
        Some(&t[..end])
    }
    fn n<T: std::str::FromStr>(&mut self) -> Option<T> {
        self.tok()?.parse().ok()
    }
    fn on<T: std::str::FromStr>(&mut self) -> Option<Option<T>> {
        match self.tok()? {
            "-" => Some(None),
            t => t.parse().ok().map(Some),
        }
    }
    fn s(&mut self) -> Option<String> {
        let t = self.0.strip_prefix(' ')?;
        let (len, rest) = t.split_once(':')?;
        let len: usize = len.parse().ok()?;
        let v = rest.get(..len)?;
        self.0 = &rest[len..];
        Some(v.to_string())
    }
    fn os(&mut self) -> Option<Option<String>> {
        if self.0.starts_with(" -") && self.0[2..].chars().next().is_none_or(|c| c == ' ') {
            self.0 = &self.0[2..];
            return Some(None);
        }
        self.s().map(Some)
    }
    fn tag(&mut self, t: &str) -> Option<usize> {
        (self.tok()? == t).then_some(())?;
        self.n()
    }
}

/// A checkpoint record's state and its clock; `None` for any other record and
/// for one that does not parse to the last token.
pub(in crate::stock) fn parse(payload: &[u8]) -> Option<(Journal, Option<i64>)> {
    let text = std::str::from_utf8(payload).ok()?;
    let (head, rest) = text.strip_prefix(HEAD).map(|_| text.split_once('\n'))??;
    let at = int_field(head, "at");
    let mut r = R(rest);
    let mut j = Journal::default();
    let led = &mut j.ledger;
    for _ in 0..r.tag("L")? {
        let item = r.s()?;
        led.levels.push((item, StockLevel { on_hand: r.n()?, reserved: r.n()? }));
    }
    for tag in ["O", "S"] {
        // A map since W-AUDIT M1: the fold keeps these keyed, not scanned.
        let mut list = std::collections::BTreeMap::new();
        for _ in 0..r.tag(tag)? {
            list.insert((r.s()?, r.s()?), r.n()?);
        }
        if tag == "O" { led.open = list } else { led.served = list }
    }
    for _ in 0..r.tag("C")? {
        led.counted.push(r.s()?);
    }
    for _ in 0..r.tag("B")? {
        let item = r.s()?;
        j.book.pools.push((item, Pool { qty: r.n()?, value: r.n()?, last_avg: r.on()? }));
    }
    for _ in 0..r.tag("T")? {
        j.lots.lots.push(Lot {
            item: r.s()?,
            code: r.s()?,
            expiry: r.on()?,
            supplier: r.os()?,
            doc: r.os()?,
            at: r.on()?,
            unit_cost: r.on()?,
            per: r.on()?,
            received: r.n()?,
            left: r.n()?,
            seq: r.n()?,
        });
    }
    (r.tok()? == "J").then_some(())?;
    j.seen = r.n()?;
    j.max_at = r.on()?;
    j.undated.plain = r.n()?;
    for _ in 0..r.n::<usize>()? {
        j.undated.by_order.push((r.s()?, r.n()?));
    }
    if r.0.starts_with(" X") {
        for _ in 0..r.tag("X")? {
            let item = r.s()?;
            j.carry.rem.insert(item, r.n()?);
        }
        for _ in 0..r.n::<usize>()? {
            let (o, i) = (r.s()?, r.s()?);
            j.carry.open.insert((o, i), (r.n()?, r.n()?));
        }
    }
    if r.0.starts_with(" P") {
        j.stores.used = true;
        for _ in 0..r.tag("P")? {
            let (i, s) = (r.s()?, r.s()?);
            j.stores.levels.insert((i, s), r.n()?);
        }
        for _ in 0..r.n::<usize>()? {
            let i = r.s()?;
            j.stores.last.insert(i, r.s()?);
        }
    }
    if r.0.starts_with(" K") {
        for _ in 0..r.tag("K")? {
            let st = r.s()?;
            j.stores.bound.insert(st, r.s()?);
        }
    }
    if r.0.starts_with(" B") {
        for _ in 0..r.tag("B")? {
            let key = (r.s()?, r.s()?);
            let mut shares = Vec::new();
            for _ in 0..r.n::<usize>()? {
                shares.push((r.s()?, r.n()?));
            }
            j.stores.held.insert(key, shares);
        }
    }
    if !r.0.is_empty() {
        return None;
    }
    j.before = j.undated.clone();
    Some((j, at))
}

