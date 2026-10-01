//! What the venue says in public, and what it is allowed to say.
//!
//! TWO RULES, and everything here follows from them.
//!
//! 1. A POST IS ABOUT SOMETHING TRUE. The subject is derived from the hub's own
//!    data -- a dish that came back on the menu, the one people ordered most
//!    this week, a dish that was just added -- and the model is given that fact
//!    and asked to phrase it. It is never asked what to promote. A model free to
//!    invent a subject eventually announces a discount the venue is not running
//!    or a dish it does not have, on the venue's own account, in public.
//!
//! 2. NOTHING PUBLISHES WITHOUT A PERSON. Every post is a DRAFT until the owner
//!    approves it. This is the same shape as the voice surface: the machine
//!    proposes, a human disposes. A restaurant's public voice is not a thing to
//!    automate blindly, and "the AI wrote it" is not a defence anyone accepts
//!    for what appears under their name.
//!
//! Drafts are small and few, so they live in the same eager KV layout as the
//! catalogue: a venue posts a handful of times a week, not a thousand times a
//! day.

use crate::minijson::{esc, int_field, str_field};
use crate::HubError;
use bebop_store::kv::Kv;
use bebop_store::Store;

pub const DEFAULT_POSTS_BYTES: usize = crate::CEILING_BYTES;
const P_POST: &str = "post:";

/// Where a post goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// A Telegram channel the venue's own bot administers. The ONLY channel
    /// implemented, and deliberately so: the transport already exists, the API
    /// is free, and a venue can set one up in two minutes without an app review.
    Telegram,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Telegram => "telegram",
        }
    }
    pub fn from_str(s: &str) -> Option<Channel> {
        match s {
            "telegram" => Some(Channel::Telegram),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Written, waiting for a person.
    Draft,
    /// The owner said no. Kept rather than deleted, so the same fact is not
    /// drafted again next hour.
    Rejected,
    Published,
    /// Approved, attempted, and the channel refused. Carries why.
    Failed,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Draft => "draft",
            State::Rejected => "rejected",
            State::Published => "published",
            State::Failed => "failed",
        }
    }
    pub fn from_str(s: &str) -> Option<State> {
        match s {
            "draft" => Some(State::Draft),
            "rejected" => Some(State::Rejected),
            "published" => Some(State::Published),
            "failed" => Some(State::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    pub id: String,
    /// The subject key, so a fact is drafted once.
    pub subject_key: String,
    /// THE FACT, IN PLAIN WORDS, despite the name. Both surfaces set this from
    /// `Subject::fact()` and both consoles render it as the line UNDER the
    /// draft ("Sake Futomaki has been added to the menu") -- it is what tells
    /// an owner what a draft is about before they read it. A short tag there
    /// would say nothing. The name is the leftover; the content is deliberate.
    pub subject_tag: String,
    pub text: String,
    pub channel: Channel,
    pub state: State,
    pub created_ms: i64,
    pub decided_ms: i64,
    /// Why publishing failed, when it did.
    pub error: String,
}

pub struct Posts {
    store: Store,
    kv: Kv,
}

impl Posts {
    /// What this image has spent. See [`crate::Usage`].
    ///
    /// The post book is rewritten COMPACTED on every save, so the capacity in the
    /// image is whatever the doubling loop last picked and is not the limit.
    /// What refuses a write is `compacted_bytes_fit` running out of doublings
    /// at `DEFAULT_POSTS_BYTES`, so that is the ceiling measured against.
    pub fn usage(&self) -> crate::Usage {
        crate::usage_of(&self.store, crate::ceiling_cells(DEFAULT_POSTS_BYTES))
    }

    pub fn create() -> Result<Self, HubError> {
        let mut store = Store::create_bytes(DEFAULT_POSTS_BYTES)?;
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Posts { store, kv })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Posts { store, kv })
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
        Ok(self.kv.compacted_bytes_fit(DEFAULT_POSTS_BYTES)?)
    }

    fn encode(p: &Post) -> String {
        format!(
            r#"{{"id":"{}","subject_key":"{}","subject_tag":"{}","text":"{}","channel":"{}","state":"{}","created_ms":{},"decided_ms":{},"error":"{}"}}"#,
            esc(&p.id),
            esc(&p.subject_key),
            esc(&p.subject_tag),
            esc(&p.text),
            p.channel.as_str(),
            p.state.as_str(),
            p.created_ms,
            p.decided_ms,
            esc(&p.error)
        )
    }

    fn decode(rec: &str) -> Option<Post> {
        Some(Post {
            id: str_field(rec, "id")?,
            subject_key: str_field(rec, "subject_key").unwrap_or_default(),
            subject_tag: str_field(rec, "subject_tag").unwrap_or_default(),
            text: str_field(rec, "text").unwrap_or_default(),
            channel: Channel::from_str(&str_field(rec, "channel")?)?,
            state: State::from_str(&str_field(rec, "state")?)?,
            created_ms: int_field(rec, "created_ms").unwrap_or(0),
            decided_ms: int_field(rec, "decided_ms").unwrap_or(0),
            error: str_field(rec, "error").unwrap_or_default(),
        })
    }

    pub fn put(&mut self, p: &Post) {
        self.kv.put(&format!("{P_POST}{}", p.id), Self::encode(p).as_bytes());
    }

    pub fn get(&self, id: &str) -> Option<Post> {
        self.kv
            .get(&format!("{P_POST}{id}"))
            .and_then(|v| Self::decode(&String::from_utf8_lossy(&v)))
    }

    pub fn all(&self) -> Vec<Post> {
        let mut out: Vec<Post> = self
            .kv
            .entries
            .iter()
            .filter(|(k, _)| k.starts_with(P_POST))
            .filter_map(|(_, v)| Self::decode(&String::from_utf8_lossy(v)))
            .collect();
        // Newest first: the owner is looking at what was just drafted.
        out.sort_by_key(|p| -p.created_ms);
        out
    }

    /// Has this fact already been put to the owner?
    ///
    /// ANY state counts, including rejected. A fact the owner turned down must
    /// not come back an hour later -- that is how an assistant becomes something
    /// people switch off.
    pub fn already_seen(&self, subject_key: &str) -> bool {
        self.all().iter().any(|p| p.subject_key == subject_key)
    }

    pub fn drafts(&self) -> Vec<Post> {
        self.all().into_iter().filter(|p| p.state == State::Draft).collect()
    }

    /// What the catalogue looked like last time subjects were derived.
    ///
    /// A SMALL PIECE OF HISTORY, and the only one. "Back on the menu" and "new
    /// dish" are not visible in a single snapshot of the catalogue -- they are
    /// differences between two. Without this the assistant could only ever
    /// notice things that are countable right now, and a dish returning after a
    /// week off is exactly the kind of thing a venue wants to say.
    ///
    /// Stored as `id:available` pairs, one per line, which is enough to answer
    /// both questions and nothing more.
    pub fn catalogue_snapshot(&self) -> Vec<(String, bool)> {
        self.kv
            .get("state:catalogue")
            .map(|v| {
                String::from_utf8_lossy(&v)
                    .lines()
                    .filter_map(|l| {
                        let (id, av) = l.rsplit_once(':')?;
                        Some((id.to_string(), av == "1"))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_catalogue_snapshot(&mut self, items: &[(String, bool)]) {
        let body: String = items
            .iter()
            .map(|(id, av)| format!("{id}:{}", if *av { 1 } else { 0 }))
            .collect::<Vec<_>>()
            .join("\n");
        self.kv.put("state:catalogue", body.as_bytes());
    }

    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }
}

mod compose;
pub use compose::{derive_subjects, prompt_for, unusable, Subject, SYSTEM_POST};

#[cfg(test)]
mod tests;
