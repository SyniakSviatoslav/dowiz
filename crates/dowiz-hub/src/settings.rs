//! What this venue has configured.
//!
//! One flat key/value store for everything an owner can set that is not the
//! menu, the roster or an order: which AI they use, where it lives, which
//! channels they post to, and the tokens for each.
//!
//! SECRETS ARE NOT ENCRYPTED AT REST, and pretending otherwise would be worse
//! than saying so. No AEAD crate is inside native-spa-server's zero-dep
//! allowlist, and the only key this process could use to encrypt would have to
//! live on the same disk, in the same directory, readable by the same user —
//! which is ceremony, not security. What protects them is what protects the
//! token signing key sitting beside them: file mode 0600 on hardware the venue
//! owns. Encryption that means something needs a key that lives somewhere else,
//! which is what P68's sovereign backup is for; until that lands, this is the
//! honest arrangement rather than a comforting one.
//!
//! WHAT THAT BUYS, and it is not nothing: a hub is ONE venue's, on their own
//! VPS. There is no shared secret store to breach, and a compromise reaches one
//! restaurant's tokens rather than every restaurant's.

use crate::minijson::esc;
use crate::HubError;
use bebop_store::kv::Kv;
use bebop_store::Store;

pub const DEFAULT_SETTINGS_BYTES: usize = crate::CEILING_BYTES;

const PREFIX: &str = "set:";

pub struct Settings {
    store: Store,
    kv: Kv,
}

/// Is this key a secret?
///
/// Decided by the key's SHAPE rather than by a list, so a setting added later
/// cannot be forgotten: anything whose last segment is `token`, `key`,
/// `secret` or `password` is redacted everywhere it is read back for display.
/// A list would have to be updated in lockstep with every new integration, and
/// the failure mode of forgetting is a token in an HTTP response.
pub fn is_secret(key: &str) -> bool {
    matches!(
        key.rsplit('.').next().unwrap_or(""),
        "token" | "key" | "secret" | "password" | "apikey"
    )
}

impl Settings {
    /// What this image has spent. See [`crate::Usage`].
    ///
    /// The settings map is rewritten COMPACTED on every save, so the capacity in the
    /// image is whatever the doubling loop last picked and is not the limit.
    /// What refuses a write is `compacted_bytes_fit` running out of doublings
    /// at `DEFAULT_SETTINGS_BYTES`, so that is the ceiling measured against.
    pub fn usage(&self) -> crate::Usage {
        crate::usage_of(&self.store, crate::ceiling_cells(DEFAULT_SETTINGS_BYTES))
    }

    pub fn create() -> Result<Self, HubError> {
        let mut store = Store::create_bytes(DEFAULT_SETTINGS_BYTES);
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Settings { store, kv })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Settings { store, kv })
    }

    /// THE IMAGE IS REWRITTEN WHOLE, not appended to.
    ///
    /// The store is append-only: every commit allocates a new generation and
    /// the old one is never reclaimed. For a KV that rewrites the same small
    /// map over and over, the arena is spent by the NUMBER OF WRITES rather
    /// than by the data -- measured at 313 empty commits before a fresh roster
    /// refused, while five hundred sessions in ONE commit fitted easily. A hub
    /// would therefore stop accepting logins after a few hundred of them.
    ///
    /// `compacted_bytes` commits the entries into a fresh image, so the file is
    /// as large as its content rather than as large as its history. See
    /// `Kv::compacted_bytes` for what that gives up (nothing anything here
    /// reads).
    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        Ok(self.kv.compacted_bytes_fit(DEFAULT_SETTINGS_BYTES)?)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.kv.put(&format!("{PREFIX}{key}"), value.as_bytes());
    }

    /// Clear a setting. A tombstone, like everywhere else in this crate: the KV
    /// layout is rewritten whole and has no delete, and an empty value is the
    /// same thing as absent for every reader here.
    pub fn clear(&mut self, key: &str) {
        self.kv.put(&format!("{PREFIX}{key}"), b"");
    }

    /// The real value, secrets included. Used by the code that CALLS a provider;
    /// never by anything that renders.
    pub fn get(&self, key: &str) -> Option<String> {
        self.kv
            .get(&format!("{PREFIX}{key}"))
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .filter(|v| !v.is_empty())
    }

    pub fn get_or<'a>(&self, key: &str, default: &'a str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }

    /// Everything, with secrets replaced by a marker.
    ///
    /// Returns whether each secret IS SET rather than what it is, because an
    /// owner needs to know they have a token configured and must never be shown
    /// it again — a value rendered into a page is a value in a screenshot, a
    /// cache and a support ticket.
    pub fn redacted(&self) -> Vec<(String, String)> {
        self.kv
            .entries
            .iter()
            .filter(|(k, _)| k.starts_with(PREFIX))
            .map(|(k, v)| (k[PREFIX.len()..].to_string(), v.clone()))
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| {
                let shown = if is_secret(&k) {
                    "\u{2022}\u{2022}\u{2022}\u{2022} set".to_string()
                } else {
                    String::from_utf8_lossy(&v).into_owned()
                };
                (k, shown)
            })
            .collect()
    }

    pub fn as_json(&self) -> String {
        let body: Vec<String> = self
            .redacted()
            .into_iter()
            .map(|(k, v)| format!(r#""{}":"{}""#, esc(&k), esc(&v)))
            .collect();
        format!("{{{}}}", body.join(","))
    }

    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }
}

mod known;
pub use known::{Known, KNOWN};

impl Settings {
    /// A known key's value, falling back to its declared default.
    pub fn known(&self, key: &str) -> String {
        let default = KNOWN.iter().find(|k| k.key == key).map(|k| k.default).unwrap_or("");
        self.get_or(key, default)
    }

    pub fn flag(&self, key: &str) -> bool {
        matches!(self.known(key).trim(), "1" | "true" | "yes" | "on")
    }
}

#[cfg(test)]
mod tests;
