//! AX3 EARLY CUTOFF FOR THE MENU (R-GRAPH D5, `hubdo/edges.rs` row `menu`).
//!
//! The memo is keyed by its INPUT generations (`Gens`), so every write to the
//! catalogue, the translations or the settings refolds it -- and, before this,
//! every dependent moved with it: the publish rewrote the R2 root and the
//! `published` record for a write that changed no byte a storefront can see
//! (a notification setting, a poller's state: the menu reads settings only
//! through `features::all`).
//!
//! THE OUTPUT GETS ITS OWN GENERATION. `out_bytes` is everything the memo
//! serves from -- the venue record, its storefront base (where settings land),
//! the categories, every product as stored, the translation rows, the blocks
//! and the publishable key -- framed so two different memos cannot frame to
//! the same bytes. The per-locale renderings are left out: they are a pure
//! function of these and a clock. A successor whose out bytes equal its
//! predecessor's keeps the predecessor's generation and its dependents do not
//! run; anything else is generation + 1.
//!
//! K64 IS AN INDEX, NEVER A PROOF (`dowiz_hub::block::schema::k64` is crc32 and
//! the length): different keys settle it, EQUAL keys are confirmed byte for
//! byte before anything is cut off.

use super::Memo;

/// One framed field: its length, then its bytes.
fn put(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

impl Memo {
    /// Everything the memo serves from, framed (see the module).
    pub fn out_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put(&mut out, self.record.as_deref().unwrap_or("\u{0}none").as_bytes());
        match &self.venue {
            Ok(Some(v)) => put(&mut out, v.base.to_string().as_bytes()),
            Ok(None) => put(&mut out, b"\x00no venue"),
            Err(e) => put(&mut out, format!("\x00err {e}").as_bytes()),
        }
        for (id, name, sort) in &self.cats {
            put(&mut out, id.as_bytes());
            put(&mut out, name.as_bytes());
            put(&mut out, &sort.to_le_bytes());
        }
        put(&mut out, b"\x00products");
        let mut ids: Vec<&String> = self.stored.keys().collect();
        ids.sort();
        for id in ids {
            put(&mut out, id.as_bytes());
            put(&mut out, self.stored[id].as_bytes());
        }
        match &self.i18n {
            Ok(rows) => {
                put(&mut out, b"\x00i18n");
                for (k, v) in rows {
                    put(&mut out, k.as_bytes());
                    put(&mut out, v.as_bytes());
                }
            }
            Err(e) => put(&mut out, format!("\x00i18n err {e}").as_bytes()),
        }
        match &self.blocks {
            Ok(b) => {
                for bytes in [&b.menu_prices, &b.bom, &b.names] {
                    put(&mut out, bytes);
                }
                put(&mut out, b.taste.as_deref().unwrap_or(b"\x00no taste"));
                put(&mut out, b.skipped.join("\n").as_bytes());
            }
            Err(e) => put(&mut out, format!("\x00blocks err {e}").as_bytes()),
        }
        put(&mut out, self.stripe_key.as_deref().unwrap_or("\u{0}none").as_bytes());
        out
    }

    /// The output's K64, computed once per memo.
    pub fn out_key(&self) -> u64 {
        if let Some(k) = self.key.get() {
            return k;
        }
        let k = dowiz_hub::block::schema::k64(&self.out_bytes());
        self.key.set(Some(k));
        k
    }

    /// The output's generation: moves only when the out bytes do.
    pub fn out_generation(&self) -> i64 {
        self.out_gen
    }

    /// Did this memo's output reach its sink (or was there no sink to reach)?
    pub fn published(&self) -> bool {
        self.published
    }

    /// Say whether the publish of this memo's output landed.
    pub fn mark_published(&mut self, ok: bool) {
        self.published = ok;
    }

    /// THIS MEMO REPLACES `prev` (the fold before a write to one of its
    /// inputs). Its output generation is `prev`'s when the out bytes are the
    /// same -- keys first, then bytes -- and `prev`'s + 1 otherwise. Answers
    /// whether the dependents may be CUT OFF: the same output, and `prev`'s
    /// output had reached its sink (a failed publish is retried by the next
    /// write, cut off or not).
    pub fn succeed(&mut self, prev: &Memo) -> bool {
        let same = self.out_key() == prev.out_key() && self.out_bytes() == prev.out_bytes();
        self.out_gen = if same { prev.out_gen } else { prev.out_gen.saturating_add(1) };
        self.published = same && prev.published;
        self.published
    }
}

#[cfg(test)]
mod tests;
