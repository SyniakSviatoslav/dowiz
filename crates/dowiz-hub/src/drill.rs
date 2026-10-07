//! THE RESTORE DRILL (W-PITR, R-BEBOPDB D.1 #10): is last night's off-site copy a venue?
//!
//! A backup nobody has restored is a hope. This takes the bytes of one nightly copy (the
//! `dowiz-hub-backup/1` bundle `cloud::push_place` uploads, after `seal-open` and `gunzip`)
//! and the census the night published beside it, and asks of every image what the venue's
//! Durable Object would ask when it loaded it -- the SAME checked loaders (`Hub::load`,
//! `Catalog::load` = `Kv::load_checked` with crc, `LogImage::load`, `StockLog::load`, ...).
//! Then the order log's chain is walked (`chain_check`) and its tip compared with the tip
//! the witness published. It writes nothing anywhere: a drill, not a restore.
//!
//! A REFUSAL IS A SENTENCE IN `refused`. The copy passes only when that list is empty; a
//! quarantined record is a refusal here (the live object would serve around it, a backup
//! that needs serving around is not a clean backup), and so is a witnessed tip this copy
//! does not hold.

use sha2::{Digest, Sha256};

use crate::catalog::{edits, Catalog};
use crate::logimage::LogImage;
use crate::{ChainCheck, Hub};

pub const FORMAT: &str = "dowiz-hub-backup/1";
/// The journal image `catalog.edits` (`workers/api/src/catalog_history.rs`).
pub const EDITS_IMAGE: &str = "catalog.edits";

/// What the witness said against what the copy holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Tip {
    /// No census was given, or it names no tip.
    #[default]
    NoWitness,
    /// The copy's log tip IS the published tip.
    Equal,
    /// The published tip is in the copy (log or an archive in it), with records after it.
    Held,
    /// The published tip is nowhere in this copy: history the witness saw is missing.
    Missing,
}

/// One image that loaded clean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub id: String,
    pub bytes: usize,
    /// Records, for the append images.
    pub records: Option<usize>,
}

#[derive(Debug, Default)]
pub struct Drill {
    pub venue: String,
    pub taken_at_ms: i64,
    pub loaded: Vec<Loaded>,
    /// Every reason this copy is NOT restorable. Empty = it passed.
    pub refused: Vec<String>,
    pub chain: Option<ChainCheck>,
    pub log_tip: Option<String>,
    pub witness_tip: Option<String>,
    pub tip: Tip,
    /// Things worth saying that do not fail the copy.
    pub notes: Vec<String>,
}

impl Drill {
    pub fn passed(&self) -> bool {
        self.refused.is_empty()
    }
}

/// Strict base64 (standard alphabet, padded): anything else is not a backup's image.
pub fn b64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let s = s.as_bytes();
    if s.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for (i, q) in s.chunks(4).enumerate() {
        let last = i + 1 == s.len() / 4;
        let pad = q.iter().rev().take_while(|&&c| c == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for &c in &q[..4 - pad] {
            n = n << 6 | val(c)?;
        }
        n <<= 6 * pad as u32;
        out.extend_from_slice(&n.to_be_bytes()[1..4 - pad]);
    }
    Some(out)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Run the drill. `witness` is the `<stamp>.witness.json` census, when there is one.
pub fn run(bundle: &[u8], witness: Option<&[u8]>) -> Drill {
    let mut d = Drill::default();
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(bundle) else {
        d.refused.push("not JSON: open a .sealed copy with seal-open and gunzip a .gz one first".into());
        return d;
    };
    if v.get("format").and_then(|x| x.as_str()) != Some(FORMAT) {
        d.refused.push(format!("not a {FORMAT} bundle"));
        return d;
    }
    d.venue = v.get("venue").and_then(|x| x.as_str()).unwrap_or("").to_string();
    d.taken_at_ms = v.get("taken_at_ms").and_then(|x| x.as_i64()).unwrap_or(0);
    let mut named: Vec<(String, &serde_json::Value)> = vec![];
    for part in ["images", "archives"] {
        if let Some(m) = v.get(part).and_then(|x| x.as_object()) {
            named.extend(m.iter().map(|(k, e)| (k.clone(), e)));
        }
    }
    if !named.iter().any(|(id, _)| id == "log") {
        d.refused.push("the copy has no order log".into());
    }
    let (mut hub, mut archives, mut catalog, mut journal) = (None, vec![], None, None);
    for (id, e) in &named {
        let Some(bytes) = e.get("image").and_then(|x| x.as_str()).and_then(b64) else {
            d.refused.push(format!("{id}: its bytes are not base64"));
            continue;
        };
        if e.get("sha256").and_then(|x| x.as_str()) != Some(hex(&Sha256::digest(&bytes)).as_str()) {
            d.refused.push(format!("{id}: does not match its sha256; the copy is damaged"));
            continue;
        }
        if e.get("bytes").and_then(|x| x.as_u64()) != Some(bytes.len() as u64) {
            d.refused.push(format!("{id}: is {} bytes, the manifest says {:?}", bytes.len(), e.get("bytes")));
            continue;
        }
        let records = match load(id, &bytes) {
            Ok(Image::Log(h)) => {
                let n = h.len();
                if id == "log" { hub = Some(h) } else { archives.push(h) }
                Some(n)
            }
            Ok(Image::Catalog(c)) => {
                catalog = Some(c);
                None
            }
            Ok(Image::Journal(j)) => {
                let n = j.len();
                journal = Some(j);
                Some(n)
            }
            Ok(Image::Other(n)) => n,
            Err(why) => {
                d.refused.push(format!("{id}: {why}"));
                continue;
            }
        };
        d.loaded.push(Loaded { id: id.clone(), bytes: bytes.len(), records });
    }
    if let Some(h) = &hub {
        let c = h.chain_check();
        if !c.intact() {
            d.refused.push(format!("log: chain broken at {} of {} records", c.broken, c.records));
        }
        d.chain = Some(c);
        d.log_tip = h.tip();
    }
    for (i, a) in archives.iter().enumerate() {
        if !a.chain_check().intact() {
            d.refused.push(format!("archive {}: chain broken", i + 1));
        }
    }
    witnessed(&mut d, witness, hub.as_ref(), &archives);
    if let (Some(j), Some(c)) = (&journal, &catalog) {
        match edits::replay(j, edits::Cut::All) {
            Err(e) => d.refused.push(format!("{EDITS_IMAGE}: does not replay: {e:?}")),
            Ok(s) => {
                // FOLD-EQUAL: the journal's state rebuilt into a catalogue has this catalogue's root.
                let same = edits::rebuild(&s).is_ok_and(|r| r.root() == c.root());
                let behind = edits::diff(&s, &edits::state_of(c)).len();
                d.notes.push(if same {
                    "the menu edit journal replays to exactly this catalogue".into()
                } else {
                    format!("the menu edit journal is {behind} key(s) behind this catalogue (recorded as unseen at the next edit)")
                });
            }
        }
    }
    d
}

enum Image {
    Log(Hub),
    Catalog(Catalog),
    Journal(LogImage),
    Other(Option<usize>),
}

/// The loader the object uses for this image id, and its quarantine count as a refusal.
fn load(id: &str, bytes: &[u8]) -> Result<Image, String> {
    let e = |x: crate::HubError| format!("refused by its loader: {x:?}");
    let archive = id.strip_prefix("log@").is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()));
    match id {
        _ if id == "log" || archive => {
            let h = Hub::load(bytes).map_err(e)?;
            match h.quarantined().len() {
                0 => Ok(Image::Log(h)),
                n => Err(format!("{n} record(s) quarantined (crc or framing)")),
            }
        }
        "catalog" => Catalog::load(bytes).map(Image::Catalog).map_err(e),
        "settings" => crate::settings::Settings::load(bytes).map(|_| Image::Other(None)).map_err(e),
        "posts" => crate::post::Posts::load(bytes).map(|_| Image::Other(None)).map_err(e),
        "stock" => {
            let s = crate::stock::StockLog::load(bytes).map_err(e)?;
            match s.quarantined() {
                0 => Ok(Image::Other(Some(s.len()))),
                n => Err(format!("{n} record(s) quarantined (crc or framing)")),
            }
        }
        EDITS_IMAGE => {
            let j = LogImage::load(bytes).map_err(e)?;
            match j.quarantined().len() {
                0 => Ok(Image::Journal(j)),
                n => Err(format!("{n} record(s) quarantined (crc or framing)")),
            }
        }
        _ => Err("an image this build does not know; a restore would refuse it".into()),
    }
}

/// Compare the published tip with the copy.
fn witnessed(d: &mut Drill, witness: Option<&[u8]>, hub: Option<&Hub>, archives: &[Hub]) {
    let Some(w) = witness else { return };
    let Ok(c) = serde_json::from_slice::<serde_json::Value>(w) else {
        d.refused.push("the witness is not JSON".into());
        return;
    };
    if let Some(venue) = c.get("venue").and_then(|x| x.as_str()) {
        if !d.venue.is_empty() && venue != d.venue {
            d.refused.push(format!("the witness is {venue}'s, the copy is {}'s", d.venue));
        }
    }
    d.witness_tip = c.get("tip").and_then(|x| x.as_str()).map(str::to_string);
    let Some(t) = d.witness_tip.clone() else { return };
    d.tip = if d.log_tip.as_deref() == Some(t.as_str()) {
        Tip::Equal
    } else if hub.is_some_and(|h| h.holds(&t)) || archives.iter().any(|a| a.holds(&t)) {
        Tip::Held
    } else {
        d.refused.push(format!("the published tip {}… is in neither the log nor an archive of this copy", &t[..t.len().min(16)]));
        Tip::Missing
    };
}

#[cfg(test)]
mod tests;
