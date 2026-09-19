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

pub const DEFAULT_SETTINGS_BYTES: usize = 256 * 1024;

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

/// A key the hub actually consults, with what it means and what it defaults to.
///
/// Declared as data so the settings pane can be BUILT from it rather than
/// listing the same keys again in JavaScript — two lists of settings drift, and
/// the one that drifts is the one the owner reads.
pub struct Known {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub default: &'static str,
}

pub const KNOWN: &[Known] = &[
    Known {
        key: "ai.endpoint",
        label: "AI endpoint",
        hint: "An OpenAI-compatible /v1 base URL. Leave the default for a local Ollama; \
               plain http is allowed ONLY for an address on this machine.",
        default: "http://127.0.0.1:11434/v1",
    },
    Known {
        key: "ai.model",
        label: "AI model",
        hint: "The model name as that endpoint spells it.",
        default: "llama3.2",
    },
    Known {
        key: "ai.token",
        label: "AI token",
        hint: "Only needed for a hosted provider. A local model needs none.",
        default: "",
    },
    Known {
        key: "social.telegram.channel",
        label: "Telegram channel",
        hint: "Your public channel, e.g. @dubinsushi. Add your bot as an \
               administrator of it first, or posting will be refused.",
        default: "",
    },
    Known {
        key: "social.enabled",
        label: "Draft social posts",
        hint: "Off by default. When on, dowiz drafts posts about real changes to \
               your menu. Nothing is ever published until you approve it.",
        default: "0",
    },
    Known {
        key: "ai.enabled",
        label: "AI assistant",
        hint: "Off by default. Nothing is sent anywhere until this is on.",
        default: "0",
    },
    // ── owner notifications ──
    //
    // The bot is the VENUE'S, not the platform's: an owner makes one with
    // @BotFather in a minute and pastes its token here, so a hub needs no
    // operator to hand out Telegram. The token's key ends in `token`, so
    // `is_secret` redacts it everywhere it is read back.
    Known {
        key: "notify.telegram.token",
        label: "Telegram bot token",
        hint: "From @BotFather. The bot messages you about every new order; it \
               is also what posts to your channel when no platform bot exists.",
        default: "",
    },
    Known {
        key: "notify.telegram.chat",
        label: "Telegram chat",
        hint: "The chat the bot writes to: your own user id, or a group's id. \
               Send the bot any message first, then use the test button.",
        default: "",
    },
    // ── WhatsApp, through Meta's Cloud API ──
    //
    // The venue's own WhatsApp Business number. A permanent System User token
    // and the number's phone-number id come from the Meta developer console;
    // `to` is where new orders are announced. The same credentials answer
    // customers who write to the number (see `channel_messages`).
    Known {
        key: "notify.whatsapp.token",
        label: "WhatsApp access token",
        hint: "A permanent token from Meta Business (System User, whatsapp_business_messaging).",
        default: "",
    },
    Known {
        key: "notify.whatsapp.phone_id",
        label: "WhatsApp phone number id",
        hint: "The numeric id of your business number in the WhatsApp Manager, not the number itself.",
        default: "",
    },
    Known {
        key: "notify.whatsapp.to",
        label: "WhatsApp number to notify",
        hint: "International format without +, e.g. 355691234567. Message the business number \
               from it once a day, or Meta only allows approved templates.",
        default: "",
    },
    Known {
        key: "notify.whatsapp.verify",
        label: "Webhook verify token",
        hint: "Any phrase you choose; paste the same one into the webhook form in Meta's console.",
        default: "",
    },
    Known {
        key: "notify.meta.secret",
        label: "Meta app secret",
        hint: "Optional. When set, every webhook delivery must carry Meta's signature over it.",
        default: "",
    },
    // ── Instagram, through the Graph API ──
    Known {
        key: "social.instagram.token",
        label: "Instagram access token",
        hint: "A long-lived token with instagram_content_publish and instagram_manage_messages.",
        default: "",
    },
    Known {
        key: "social.instagram.user_id",
        label: "Instagram account id",
        hint: "The numeric id of the professional account, from the Graph API or the Meta console.",
        default: "",
    },
    // ── cloud storage: any S3-compatible bucket ──
    //
    // R2, AWS S3, Backblaze B2, Wasabi, MinIO: keys alone connect them, which
    // is why this and not a Drive OAuth dance is the venue's off-site copy.
    Known {
        key: "cloud.s3.endpoint",
        label: "Storage endpoint",
        hint: "https://<account>.r2.cloudflarestorage.com, https://s3.eu-central-1.amazonaws.com, …",
        default: "",
    },
    Known {
        key: "cloud.s3.region",
        label: "Storage region",
        hint: "auto for R2; the region name for AWS and others.",
        default: "auto",
    },
    Known {
        key: "cloud.s3.bucket",
        label: "Bucket",
        hint: "The bucket the backups land in. It must already exist.",
        default: "",
    },
    Known {
        key: "cloud.s3.key",
        label: "Access key id",
        hint: "The key pair of a user allowed to write to that bucket.",
        default: "",
    },
    Known {
        key: "cloud.s3.secret",
        label: "Secret access key",
        hint: "Stored, never shown again.",
        default: "",
    },
    Known {
        key: "cloud.s3.prefix",
        label: "Object prefix",
        hint: "A folder inside the bucket; the venue's id is appended.",
        default: "dowiz",
    },
];

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
mod tests {
    use super::*;

    #[test]
    fn settings_survive_the_byte_image() {
        let mut s = Settings::create().expect("create");
        s.set("ai.model", "llama3.2");
        s.set("ai.token", "sk-secret-value");
        let bytes = s.to_bytes().expect("bytes");

        let s = Settings::load(&bytes).expect("load");
        assert_eq!(s.get("ai.model").as_deref(), Some("llama3.2"));
        assert_eq!(s.get("ai.token").as_deref(), Some("sk-secret-value"));
        assert_eq!(s.get("nothing.here"), None);
    }

    /// The one that matters: a token must never come back out for display.
    #[test]
    fn a_secret_is_never_rendered() {
        let mut s = Settings::create().expect("create");
        s.set("ai.token", "sk-secret-value");
        s.set("ai.model", "llama3.2");

        let json = s.as_json();
        assert!(!json.contains("sk-secret-value"), "the token leaked: {json}");
        assert!(json.contains("llama3.2"), "non-secrets must still show: {json}");
        assert!(json.contains("set"), "the owner must see that it IS configured: {json}");

        // And the code that CALLS a provider still gets the real thing.
        assert_eq!(s.get("ai.token").as_deref(), Some("sk-secret-value"));
    }

    /// Secrecy follows the key's shape, so an integration added later cannot be
    /// forgotten into a list that was never updated.
    #[test]
    fn secrecy_is_decided_by_shape_not_by_a_list() {
        for k in [
            "ai.token",
            "social.telegram.token",
            "anything.at.all.key",
            "x.secret",
            "y.password",
            "z.apikey",
        ] {
            assert!(is_secret(k), "{k} must be secret");
        }
        for k in ["ai.model", "ai.endpoint", "ai.enabled", "tokens.count", "keyboard"] {
            assert!(!is_secret(k), "{k} must NOT be treated as a secret");
        }
    }

    #[test]
    fn known_keys_fall_back_to_their_declared_defaults() {
        let mut s = Settings::create().expect("create");
        assert_eq!(s.known("ai.endpoint"), "http://127.0.0.1:11434/v1");
        assert_eq!(s.known("ai.model"), "llama3.2");
        assert!(!s.flag("ai.enabled"), "the assistant must be OFF until switched on");

        s.set("ai.model", "qwen2.5");
        assert_eq!(s.known("ai.model"), "qwen2.5");
        s.set("ai.enabled", "1");
        assert!(s.flag("ai.enabled"));
        s.set("ai.enabled", "0");
        assert!(!s.flag("ai.enabled"));
    }

    #[test]
    fn clearing_a_setting_restores_its_default() {
        let mut s = Settings::create().expect("create");
        s.set("ai.model", "qwen2.5");
        s.clear("ai.model");
        assert_eq!(s.get("ai.model"), None);
        assert_eq!(s.known("ai.model"), "llama3.2", "back to the declared default");
        // And the cleared key is not rendered as an empty row.
        assert!(!s.as_json().contains("ai.model"));
    }

    /// A value with a quote in it must not be able to add a settings field.
    #[test]
    fn a_hostile_value_cannot_forge_a_setting() {
        let mut s = Settings::create().expect("create");
        s.set("ai.model", r#"x","ai.enabled":"1"#);
        let json = s.as_json();
        assert!(json.contains(r#"\"ai.enabled\""#), "must be escaped, not structural: {json}");
        assert!(!s.flag("ai.enabled"), "the flag must not have been set by a model name");
    }
}
