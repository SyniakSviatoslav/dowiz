//! CRYPTO-SHREDDING FOR NEW LOGS (DG10; research 2026-09-28-bebop-dag §11.1).
//!
//! WHY. `forget.rs` redacts a person IN PLACE: the record keeps its `id` and
//! `prev` and stops matching its own content. That is right for history that
//! is already in clear, and wrong for a store whose blocks are named by the
//! hash of their bytes and copied by design (archives, nightly copies, a Box
//! network): a redacted block no longer hashes to its name, and any peer that
//! still holds the old block still holds the person. So for NEW logs the
//! personal fields are written SEALED, under a key that belongs to the person
//! and lives OUTSIDE the log, in a small mutable `KeyTable`. Forgetting drops
//! the key and appends one `Forgotten` declaration: no block is touched, every
//! block keeps its `K256`, and every copy of every block becomes unreadable at
//! the same instant. History already in clear keeps declared redaction.
//!
//! THE PRIMITIVE IS NOT HERE. AES-256-GCM is `dowiz_core::pq::aes_gcm`,
//! reused unchanged: hand-rolled once, KAT-gated against GCM spec TC15 in
//! `dowiz-core` and again through this module's own seal path
//! (`tests::kat_aes256gcm_vector_from_core`). Nothing in this file touches a
//! round, a counter or a tag.
//!
//! PURE, like the rest of the crate: no clock, no randomness. The caller
//! supplies each person's key (32 bytes from `getRandomValues`) and a FRESH
//! random 12-byte nonce for every seal. A nonce is never derived here, because
//! a derived nonce repeats the moment the state it was derived from is lost.
//!
//! THE SUBJECT IS A CUSTOMER KEY (16 lowercase hex, the Worker's HMAC of the
//! phone), never the phone: `KeyTable::ensure` refuses anything else, so the
//! table cannot hold a personal byte (`tests::key_table_holds_no_personal_bytes`).
//!
//! THE TABLE NEVER SHRINKS. Forgetting overwrites the person's 32 key bytes
//! with zeros IN PLACE and keeps the entry: the image keeps its length, so a
//! writer that stores only the chunks that changed rewrites exactly the one
//! that held the key and cannot leave a stale tail chunk behind (memory:
//! write-only-what-changed). An all-zero key is the forgotten mark, which is
//! why `ensure` refuses one as a key.
//!
//! FORMAT (little-endian): `DWK1` | count u32 | count x (subject 16 ascii |
//! key 32) sorted by subject, unique | crc32 (zlib) of everything before it.
//! ENVELOPE (text, so it sits in a JSON string): `shr1:<subject>:<hex(nonce ||
//! ciphertext || tag)>`.

use std::collections::BTreeMap;

use dowiz_core::pq::aes_gcm::Aes256Gcm;

use crate::{EventKind, Hub};

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const SUBJECT_LEN: usize = 16;
const ENTRY: usize = SUBJECT_LEN + KEY_LEN;
const MAGIC: [u8; 4] = *b"DWK1";
/// Every sealed field starts with this.
pub const PREFIX: &str = "shr1:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShredError {
    /// Not a customer key (16 lowercase hex).
    Subject,
    /// An all-zero key: that is the forgotten mark, not a key.
    WeakKey,
    /// Sealing for a subject with no live key: `ensure` first.
    NoKey,
    /// The key is gone. The field is unreadable everywhere, by design.
    Shredded,
    /// The key is present and the tag does not verify.
    Tampered,
    /// Not an envelope.
    Malformed,
    /// A key table image that fails its own checks, by name.
    Corrupt(&'static str),
    /// The declaration could not be appended.
    Log(String),
}

/// What [`Hub::shred_forget`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shredded {
    /// The key was live and is now gone (false on a retry).
    pub dropped: bool,
    /// Sealed fields of the subject in the logs it was shown.
    pub sealed: usize,
    /// This run appended the `Forgotten` declaration.
    pub declared: bool,
}

/// The per-person key table. Mutable, small, never content-addressed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct KeyTable {
    keys: BTreeMap<[u8; SUBJECT_LEN], [u8; KEY_LEN]>,
}

fn subject_bytes(s: &str) -> Result<[u8; SUBJECT_LEN], ShredError> {
    let ok = s.len() == SUBJECT_LEN && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    ok.then(|| s.as_bytes().try_into().ok()).flatten().ok_or(ShredError::Subject)
}

impl KeyTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(b: &[u8]) -> Result<Self, ShredError> {
        if b.len() < 12 {
            return Err(ShredError::Corrupt("short"));
        }
        let (body, crc) = b.split_at(b.len() - 4);
        if crate::block::crc32(body).to_le_bytes() != crc {
            return Err(ShredError::Corrupt("crc"));
        }
        if body[..4] != MAGIC {
            return Err(ShredError::Corrupt("magic"));
        }
        let n = u32::from_le_bytes([body[4], body[5], body[6], body[7]]) as usize;
        if body.len() != 8 + n.saturating_mul(ENTRY) {
            return Err(ShredError::Corrupt("length"));
        }
        let mut t = KeyTable::new();
        let mut last: Option<[u8; SUBJECT_LEN]> = None;
        for e in body[8..].chunks_exact(ENTRY) {
            let s = std::str::from_utf8(&e[..SUBJECT_LEN]).map_err(|_| ShredError::Corrupt("subject"))?;
            let s = subject_bytes(s).map_err(|_| ShredError::Corrupt("subject"))?;
            if last.is_some_and(|l| l >= s) {
                return Err(ShredError::Corrupt("order"));
            }
            last = Some(s);
            t.keys.insert(s, e[SUBJECT_LEN..].try_into().map_err(|_| ShredError::Corrupt("key"))?);
        }
        Ok(t)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + self.keys.len() * ENTRY);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&(self.keys.len() as u32).to_le_bytes());
        for (s, k) in &self.keys {
            out.extend_from_slice(s);
            out.extend_from_slice(k);
        }
        let crc = crate::block::crc32(&out);
        out.extend_from_slice(&crc.to_le_bytes());
        out
    }

    /// Give `subject` a key unless it has a live one; `fresh` is used only
    /// when it does not (a forgotten person who comes back gets a new key).
    pub fn ensure(&mut self, subject: &str, fresh: [u8; KEY_LEN]) -> Result<(), ShredError> {
        let s = subject_bytes(subject)?;
        if fresh == [0; KEY_LEN] {
            return Err(ShredError::WeakKey);
        }
        let slot = self.keys.entry(s).or_insert([0; KEY_LEN]);
        if *slot == [0; KEY_LEN] {
            *slot = fresh;
        }
        Ok(())
    }

    /// The live key of `subject`, if it has one.
    pub fn key(&self, subject: &str) -> Option<&[u8; KEY_LEN]> {
        let s = subject_bytes(subject).ok()?;
        self.keys.get(&s).filter(|k| **k != [0; KEY_LEN])
    }

    /// Overwrite the key with the forgotten mark. True when it was live.
    pub fn drop_key(&mut self, subject: &str) -> bool {
        let Ok(s) = subject_bytes(subject) else { return false };
        match self.keys.get_mut(&s) {
            Some(k) if *k != [0; KEY_LEN] => {
                *k = [0; KEY_LEN];
                true
            }
            _ => false,
        }
    }

    /// How many people hold a live key.
    pub fn len(&self) -> usize {
        self.keys.values().filter(|k| **k != [0; KEY_LEN]).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Seal `plaintext` for `subject` under its key with the caller's fresh nonce.
pub fn seal_bytes(t: &KeyTable, subject: &str, plaintext: &[u8], nonce: [u8; NONCE_LEN]) -> Result<String, ShredError> {
    subject_bytes(subject)?;
    let key = t.key(subject).ok_or(ShredError::NoKey)?;
    let mut body = nonce.to_vec();
    body.extend_from_slice(&Aes256Gcm::new(key).encrypt(&nonce, plaintext));
    Ok(format!("{PREFIX}{subject}:{}", crate::crypto::hex(&body)))
}

/// The subject an envelope is sealed for, when `env` is one.
pub fn subject_of(env: &str) -> Option<&str> {
    let rest = env.strip_prefix(PREFIX)?;
    let (s, _) = rest.split_once(':')?;
    subject_bytes(s).ok().map(|_| s)
}

/// Open an envelope: the plaintext, `Shredded` when the key is gone,
/// `Tampered` when it is present and the tag fails.
pub fn open_bytes(t: &KeyTable, env: &str) -> Result<Vec<u8>, ShredError> {
    let subject = subject_of(env).ok_or(ShredError::Malformed)?;
    let hex = &env[PREFIX.len() + SUBJECT_LEN + 1..];
    let body = crate::crypto::unhex(hex).ok_or(ShredError::Malformed)?;
    if body.len() < NONCE_LEN + TAG_LEN {
        return Err(ShredError::Malformed);
    }
    let key = t.key(subject).ok_or(ShredError::Shredded)?;
    let nonce: [u8; NONCE_LEN] = body[..NONCE_LEN].try_into().map_err(|_| ShredError::Malformed)?;
    Aes256Gcm::new(key).decrypt(&nonce, &body[NONCE_LEN..]).map_err(|_| ShredError::Tampered)
}

/// [`seal_bytes`] for a text field (a phone, a name, an address line).
pub fn seal_field(t: &KeyTable, subject: &str, plaintext: &str, nonce: [u8; NONCE_LEN]) -> Result<String, ShredError> {
    seal_bytes(t, subject, plaintext.as_bytes(), nonce)
}

/// [`open_bytes`] for a text field.
pub fn open_field(t: &KeyTable, env: &str) -> Result<String, ShredError> {
    String::from_utf8(open_bytes(t, env)?).map_err(|_| ShredError::Malformed)
}

impl Hub {
    /// FORGET BY SHREDDING: drop `subject`'s key from `keys`, and declare it
    /// in THIS (hot) log. `archives` are only counted, never written: that is
    /// the point. Returns what was done.
    ///
    /// THE DECLARATION redacts nothing in place, so it names `"records":0`
    /// (law 9, `chain.redacted == declared`, is untouched) and `"shredded"`:
    /// the sealed fields of the person in the logs shown.
    ///
    /// WRITE ORDER for the caller: the key table FIRST (the deletion is the
    /// act), then this log. A failure between them leaves the person forgotten
    /// and undeclared, and the retry converges: the key is already gone
    /// (`dropped: false`), no shred declaration exists yet, so it is appended
    /// then and never twice.
    pub fn shred_forget(
        &mut self,
        keys: &mut KeyTable,
        archives: &[&Hub],
        subject: &str,
        by: &str,
        now_ms: i64,
        seq: u64,
    ) -> Result<Shredded, ShredError> {
        subject_bytes(subject)?;
        let mark = format!("{PREFIX}{subject}:");
        let count = |h: &Hub| h.events().iter().map(|e| e.order_json.matches(mark.as_str()).count()).sum::<usize>();
        let sealed = count(self) + archives.iter().map(|a| count(a)).sum::<usize>();
        let dropped = keys.drop_key(subject);
        let who = format!("cust:{subject}");
        let already = self
            .events()
            .iter()
            .any(|e| e.kind == EventKind::Forgotten && e.order_id == who && e.order_json.contains("\"shredded\":"));
        let declared = !already && (dropped || sealed > 0);
        if declared {
            let by = serde_json::to_string(by).map_err(|e| ShredError::Log(e.to_string()))?;
            // `records` FIRST: `Hub::declared` reads the first `"records":`.
            let json = format!(r#"{{"records":0,"shredded":{sealed},"by":{by},"at":{now_ms},"reason":"shred"}}"#);
            self.append(EventKind::Forgotten, &who, &json, seq, [0u8; 32]).map_err(|e| ShredError::Log(format!("{e:?}")))?;
        }
        Ok(Shredded { dropped, sealed, declared })
    }
}

#[cfg(test)]
mod tests;
