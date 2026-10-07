//! THE FEED: how the host hands the hub's answers to the board.
//!
//! The host already holds the JSON (`JSON.parse` is native and free); parsing it again in Rust
//! would cost wasm bytes for nothing. So the adapter (`room/canvas/board.js` `toFeed`) flattens
//! the two reads into tab-separated lines, one record each, and this module is their one reader:
//!
//! ```text
//! T <id> <STATUS> <start_ms> <seen 0|1> <t|p|d> <table> <note> <when>     a ticket
//! L <qty> <station 1|2|3> <name> <note>                                   a line of the last T
//! R <sitting> <table> <rounds> <due text> <guest 0|1> <STATUS,STATUS,...> an open table
//! ```
//! A feed is a whole snapshot: it replaces every ticket and table. What does not fit a pool is
//! counted in `dropped` and drawn as "+N more" -- never silently cut. Values never contain a tab
//! or a newline (the adapter turns both into spaces).

use super::model::*;

pub const ARENA: usize = 49_152;
pub const MAX_TICKETS: usize = 96;
pub const MAX_LINES: usize = 768;
pub const MAX_TABLES: usize = 64;

pub struct Data {
    pub arena: [u8; ARENA],
    pub used: usize,
    pub tickets: [Ticket; MAX_TICKETS],
    pub nt: usize,
    pub lines: [Line; MAX_LINES],
    pub nl: usize,
    pub tables: [Table; MAX_TABLES],
    pub ntab: usize,
    /// Records the pools could not hold (tickets, lines, tables, or arena bytes).
    pub dropped: u32,
    /// Lines the reader did not understand.
    pub bad: u32,
    /// The last ticket was dropped, so its lines are too (never pinned on the ticket before it).
    skip: bool,
}

fn num(b: &[u8]) -> Option<i64> {
    let (neg, d) = match b.first() {
        Some(b'-') => (true, &b[1..]),
        _ => (false, b),
    };
    if d.is_empty() || d.len() > 18 || !d.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let v = d.iter().fold(0i64, |a, &c| a * 10 + (c - b'0') as i64);
    Some(if neg { -v } else { v })
}

impl Data {
    pub const fn new() -> Data {
        Data {
            arena: [0; ARENA], used: 0, tickets: [BLANK_TICKET; MAX_TICKETS], nt: 0,
            lines: [BLANK_LINE; MAX_LINES], nl: 0, tables: [BLANK_TABLE; MAX_TABLES], ntab: 0,
            dropped: 0, bad: 0, skip: false,
        }
    }

    pub fn str(&self, s: Span) -> &str {
        let (a, b) = (s.off as usize, (s.off + s.len) as usize);
        if b > self.used {
            return "";
        }
        core::str::from_utf8(&self.arena[a..b]).unwrap_or("")
    }

    fn put(&mut self, b: &[u8]) -> Span {
        if core::str::from_utf8(b).is_err() {
            self.bad += 1;
            return Span::default();
        }
        if self.used + b.len() > ARENA {
            self.dropped += 1;
            return Span::default();
        }
        let s = Span { off: self.used as u32, len: b.len() as u32 };
        self.arena[self.used..self.used + b.len()].copy_from_slice(b);
        self.used += b.len();
        s
    }

    /// Replace everything with the snapshot in `feed`. Returns the number of tickets read.
    pub fn apply(&mut self, feed: &[u8]) -> usize {
        self.used = 0;
        self.nt = 0;
        self.nl = 0;
        self.ntab = 0;
        self.dropped = 0;
        self.bad = 0;
        self.skip = false;
        for rec in feed.split(|&c| c == b'\n') {
            if rec.is_empty() {
                continue;
            }
            let mut f: [&[u8]; 9] = [&[]; 9];
            let mut n = 0;
            for part in rec.split(|&c| c == b'\t') {
                if n < f.len() {
                    f[n] = part;
                }
                n += 1;
            }
            match f[0] {
                b"T" if n == 9 => self.ticket(&f),
                b"L" if n == 5 => self.line(&f),
                b"R" if n == 7 => self.table(&f),
                _ => self.bad += 1,
            }
        }
        self.nt
    }

    fn ticket(&mut self, f: &[&[u8]; 9]) {
        let Some(start) = num(f[3]) else { self.bad += 1; return };
        if self.nt == MAX_TICKETS {
            self.dropped += 1;
            self.skip = true;
            return;
        }
        self.skip = false;
        let kind = match f[5] {
            b"t" => Where::Table,
            b"d" => Where::Delivery,
            _ => Where::Pickup,
        };
        let t = Ticket {
            id: self.put(f[1]), status: Status::parse(f[2]), start_ms: start, seen: f[4] == b"1", kind,
            table: self.put(f[6]), note: self.put(f[7]), when: self.put(f[8]), line0: self.nl as u16, nlines: 0,
        };
        self.tickets[self.nt] = t;
        self.nt += 1;
    }

    fn line(&mut self, f: &[&[u8]; 9]) {
        let (Some(qty), Some(st)) = (num(f[1]), num(f[2])) else { self.bad += 1; return };
        if self.nt == 0 {
            self.bad += 1;
            return;
        }
        if self.skip || self.nl == MAX_LINES {
            self.dropped += 1;
            return;
        }
        let l = Line { qty: qty.clamp(0, 9999) as i32, station: st.clamp(1, 3) as u8, name: self.put(f[3]), note: self.put(f[4]) };
        self.lines[self.nl] = l;
        self.nl += 1;
        self.tickets[self.nt - 1].nlines += 1;
    }

    fn table(&mut self, f: &[&[u8]; 9]) {
        let Some(rounds) = num(f[3]) else { self.bad += 1; return };
        if self.ntab == MAX_TABLES {
            self.dropped += 1;
            return;
        }
        let mut t = Table {
            sitting: self.put(f[1]), table: self.put(f[2]), rounds: rounds.clamp(0, 999) as u16, due: self.put(f[4]),
            guest: f[5] == b"1", st: [Status::Other; ROUND_ST], nst: 0,
        };
        for s in f[6].split(|&c| c == b',').filter(|s| !s.is_empty()) {
            if (t.nst as usize) < ROUND_ST {
                t.st[t.nst as usize] = Status::parse(s);
                t.nst += 1;
            }
        }
        self.tables[self.ntab] = t;
        self.ntab += 1;
    }
}
