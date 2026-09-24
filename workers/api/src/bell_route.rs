//! PRINT ROUTING BY STATION (BLUEPRINT-POS-THE-ROOM §2.7, §7 item 7). PURE.
//!
//! The Telegram chat IS the printer at this venue, and a bar that reads the
//! kitchen's ticket pours from a list of sushi. A product carries `station`
//! (`kitchen | bar`, default kitchen); it travels on the line the way the
//! dish's name does, and `hubdo::enqueue_bell` asks this module who gets what.
//!
//! A VENUE THAT HAS SET NO BAR CHAT GETS ONE BELL EXACTLY AS TODAY: the same
//! id (`{order}/telegram`), the same chat, the same text. The kitchen's id is
//! today's id on purpose — an order placed across the deploy that is retried
//! by the idempotency layer still writes over its own entry instead of adding
//! a second message. Only a venue that set `notify.telegram.chat.bar` and
//! ordered a bar line gets a split, and then each station's ticket carries
//! only its own lines.
//!
//! AN `Amended` THAT ADDS LINES rings the stations those lines belong to, under
//! ids that name the amendment's seq, so it can never overwrite the placement's
//! bell (a pending entry replaced by id is a lost message).
//!
//! NO I/O: settings, lines and text arrive as values.

use serde_json::Value;

/// Where a line is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Station {
    Kitchen,
    Bar,
}

impl Station {
    pub fn as_str(self) -> &'static str {
        match self {
            Station::Kitchen => "kitchen",
            Station::Bar => "bar",
        }
    }

    /// LENIENT, for a line already stored: missing or unknown is the kitchen,
    /// because a ticket that goes nowhere is the one outcome worse than a
    /// ticket at the wrong station.
    pub fn of_line(line: &Value) -> Station {
        match line.get("station").and_then(Value::as_str) {
            Some("bar") => Station::Bar,
            _ => Station::Kitchen,
        }
    }

    /// STRICT, for what an owner types: a closed set, refused rather than
    /// guessed, so "Bar " or "grill" never silently becomes the kitchen.
    pub fn from_wire(s: &str) -> Result<Station, String> {
        match s {
            "kitchen" => Ok(Station::Kitchen),
            "bar" => Ok(Station::Bar),
            other => Err(format!("station {other:?}: it is kitchen or bar")),
        }
    }
}

/// A DISH EDIT and the station. PURE. The console sends `station` only when
/// the owner moved it, so `None` leaves the dish where it was: a price edit
/// never sends the bar's drinks back to the kitchen's chat.
pub fn edit_station(product: &mut Value, st: Option<Station>) {
    if let Some(st) = st {
        product["station"] = Value::from(st.as_str());
    }
}

/// THE STATION TRAVELS ON THE LINE, only when it is not the kitchen: absent IS
/// the kitchen, and every stored byte is paid for.
pub fn stamp_line(line: &mut Value, st: Station) {
    if st == Station::Bar {
        line["station"] = Value::from(st.as_str());
    }
}

/// The lines of an order, grouped by station, each group in input order.
/// Kitchen first; a station with no lines has no group.
pub fn split_by_station(lines: &[Value]) -> Vec<(Station, Vec<Value>)> {
    let mut out: Vec<(Station, Vec<Value>)> = Vec::new();
    for s in [Station::Kitchen, Station::Bar] {
        let mine: Vec<Value> = lines.iter().filter(|l| Station::of_line(l) == s).cloned().collect();
        if !mine.is_empty() {
            out.push((s, mine));
        }
    }
    out
}

/// The outbox id. The kitchen's is today's `{order}/{kind}`; the bar's adds
/// `/bar`. Distinct per station, stable per order.
pub fn entry_id(order_id: &str, kind: &str, station: Station) -> String {
    match station {
        Station::Kitchen => format!("{order_id}/{kind}"),
        Station::Bar => format!("{order_id}/{kind}/bar"),
    }
}

/// Who gets a ticket. `None` is the venue's default chat. With no bar chat
/// set, or no bar line on the order, this is ONE target: the kitchen, the
/// default chat — today's bell.
pub fn targets<'a>(groups: &[(Station, Vec<Value>)], bar_chat: Option<&'a str>) -> Vec<(Station, Option<&'a str>)> {
    let bar = bar_chat.map(str::trim).filter(|c| !c.is_empty());
    match bar {
        Some(chat) if groups.iter().any(|(s, _)| *s == Station::Bar) => groups
            .iter()
            .map(|(s, _)| match s {
                Station::Bar => (Station::Bar, Some(chat)),
                Station::Kitchen => (Station::Kitchen, None),
            })
            .collect(),
        _ => vec![(Station::Kitchen, None)],
    }
}

/// One Telegram message owed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub id: String,
    pub to: String,
    pub text: String,
}

/// The header of a rendered bell: everything above its first blank line
/// (`notify::order_text` puts one between the header and the lines).
pub fn header_of(text: &str) -> &str {
    text.split("\n\n").next().unwrap_or(text)
}

/// A station's ticket: the header, the station, and only its lines. No prices:
/// a station makes food, it does not settle the bill.
/// The station is named only when the order was split.
pub fn ticket_text(header: &str, station: Option<Station>, lines: &[Value]) -> String {
    let mut out = format!("{header}\n\n");
    if let Some(s) = station {
        out.push_str(&format!("[{}]\n", s.as_str()));
    }
    for l in lines {
        let q = l.get("quantity").and_then(Value::as_i64).unwrap_or(0);
        let name = l
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| l.get("product_id").and_then(Value::as_str))
            .unwrap_or("?");
        out.push_str(&format!("{q} × {name}\n"));
    }
    out
}

/// The Telegram tickets for one bell.
///
/// `lines` are the lines this bell is about: the whole order at placement,
/// the added lines of an amendment. `amend_seq` is `Some` for an amendment.
/// `chat` is `notify.telegram.chat`, `bar_chat` `notify.telegram.chat.bar`.
/// A target with no chat to send to is dropped, as today.
pub fn telegram_tickets(
    order_id: &str,
    text: &str,
    lines: &[Value],
    amend_seq: Option<u64>,
    chat: &str,
    bar_chat: &str,
) -> Vec<Ticket> {
    let chat = chat.trim();
    let groups = split_by_station(lines);
    let to = targets(&groups, Some(bar_chat));
    let split = to.iter().any(|(s, _)| *s == Station::Bar);
    let base = match amend_seq {
        Some(seq) => format!("{order_id}/amend/{seq}"),
        None => order_id.to_string(),
    };
    let mut out = Vec::new();
    for (station, dest) in to {
        let dest = dest.unwrap_or(chat);
        if dest.is_empty() {
            continue;
        }
        let mine: &[Value] = if split {
            groups.iter().find(|(s, _)| *s == station).map(|(_, l)| l.as_slice()).unwrap_or(&[])
        } else {
            lines
        };
        if amend_seq.is_some() && mine.is_empty() {
            continue;
        }
        // UNSPLIT AT PLACEMENT IS THE TEXT AS RENDERED, byte for byte.
        let body = if !split && amend_seq.is_none() {
            text.to_string()
        } else {
            ticket_text(header_of(text), split.then_some(station), mine)
        };
        out.push(Ticket { id: entry_id(&base, "telegram", station), to: dest.to_string(), text: body });
    }
    out
}

/// The lines of a stored order.
pub fn lines_of(order_json: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(order_json)
        .ok()
        .and_then(|o| o.get("items").and_then(Value::as_array).cloned())
        .unwrap_or_default()
}

/// The lines an amendment ADDED. A removal, a recount or a comp is not rung.
pub fn added_lines(ops: &[crate::command::amend::Op]) -> Vec<Value> {
    ops.iter()
        .filter_map(|op| match op {
            crate::command::amend::Op::Add { line } => Some(line.clone()),
            _ => None,
        })
        .collect()
}

/// The header of an amendment's ticket: which round, which table.
pub fn amend_header(order_id: &str, round: &Value) -> String {
    let id: String = order_id.chars().take(8).collect();
    match round.get("fulfilment").and_then(|f| f.get("table")).and_then(Value::as_str) {
        Some(t) if !t.trim().is_empty() => format!("+ #{id} — table {}", t.trim()),
        _ => format!("+ #{id}"),
    }
}

#[cfg(test)]
mod tests;
