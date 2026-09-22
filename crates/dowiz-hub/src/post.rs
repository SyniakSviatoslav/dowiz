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

pub const DEFAULT_POSTS_BYTES: usize = 512 * 1024;
const P_POST: &str = "post:";

/// What the post is about — a fact the hub computed, never a theme the model chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A dish that was unavailable and is on the menu again.
    BackOnTheMenu { product: String },
    /// The dish ordered most over the period looked at.
    MostOrdered { product: String, orders: i64 },
    /// A dish that appeared in the catalogue and has never been posted about.
    NewDish { product: String },
    /// The venue went from closed to open.
    Reopened,
}

impl Subject {
    /// A stable key, so the same fact is not drafted twice.
    ///
    /// This is what stops a venue posting "the sushi is back" every hour for a
    /// day because a loop noticed the same change on every pass.
    pub fn key(&self) -> String {
        match self {
            Subject::BackOnTheMenu { product } => format!("back:{product}"),
            Subject::MostOrdered { product, .. } => format!("top:{product}"),
            Subject::NewDish { product } => format!("new:{product}"),
            Subject::Reopened => "reopened".into(),
        }
    }

    /// The fact, in plain words, handed to the model as the thing to phrase.
    pub fn fact(&self) -> String {
        match self {
            Subject::BackOnTheMenu { product } => {
                format!("{product} is available again after being off the menu")
            }
            Subject::MostOrdered { product, orders } => {
                format!("{product} was ordered {orders} times this week, more than anything else")
            }
            Subject::NewDish { product } => format!("{product} has been added to the menu"),
            Subject::Reopened => "the restaurant is open again".into(),
        }
    }

    // `tag()` WAS HERE, returning "back"/"top"/"new"/"reopened", and nothing
    // ever called it. It was for a categorisation that never shipped, and
    // leaving it beside `fact()` invited exactly one mistake: `subject_tag` is
    // NAMED like the tag and HOLDS the fact, on purpose -- see the field.
}

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
        let mut store = Store::create_bytes(DEFAULT_POSTS_BYTES);
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

/// Work out what is worth saying, by comparing now against last time.
///
/// Returns subjects the owner has not already been shown. The ORDER is
/// deliberate: a dish returning is more interesting than a dish being popular,
/// which is more interesting than a dish merely existing.
pub fn derive_subjects(
    previous: &[(String, bool)],
    current: &[(String, String, bool)],
    top: Option<(String, i64)>,
    was_closed: bool,
    is_open: bool,
    seen: &dyn Fn(&str) -> bool,
) -> Vec<Subject> {
    let mut out = Vec::new();
    let name_of = |id: &str| {
        current
            .iter()
            .find(|(i, _, _)| i == id)
            .map(|(_, n, _)| n.clone())
            .unwrap_or_else(|| id.to_string())
    };

    if was_closed && is_open {
        out.push(Subject::Reopened);
    }

    // FIRST RUN IS SILENT. With no previous snapshot every dish looks new, and
    // a venue switching this on would be handed fifty drafts about a menu that
    // has not changed.
    if !previous.is_empty() {
        for (id, name, available) in current {
            match previous.iter().find(|(p, _)| p == id) {
                Some((_, was)) if !was && *available => {
                    out.push(Subject::BackOnTheMenu { product: name.clone() })
                }
                None if *available => out.push(Subject::NewDish { product: name.clone() }),
                _ => {}
            }
        }
    }

    if let Some((id, orders)) = top {
        // One order is not a trend. Below this it is noise, and a venue posting
        // "our most ordered dish this week" about a single order looks worse
        // than saying nothing.
        if orders >= 3 {
            out.push(Subject::MostOrdered { product: name_of(&id), orders });
        }
    }

    out.retain(|s| !seen(&s.key()));
    out
}

/// The instructions the model writes under.
///
/// Tight on purpose. This text goes out under a restaurant's name, so the model
/// is given one fact, a length, and an explicit ban on the two things a
/// marketing-trained model reaches for by default: invented offers and invented
/// urgency.
pub const SYSTEM_POST: &str = "\
You write one short social post for a single restaurant. \
You are given ONE FACT that the restaurant's own system computed. Write about \
that fact and nothing else. \
NEVER invent a discount, a price, a deadline, an ingredient, an award, or a \
quantity that is not in the fact. NEVER write 'limited time', 'hurry', or \
'don't miss out'. \
Two sentences at most. Warm, plain, the way a small restaurant actually speaks. \
Write in the language you are asked for. Output only the post text: no quotes, \
no hashtags unless they are the restaurant's own name, no emoji beyond one.";

/// Build the prompt for one subject.
pub fn prompt_for(subject: &Subject, venue: &str, lang: &str) -> String {
    let language = match lang {
        l if l.starts_with("uk") => "Ukrainian",
        l if l.starts_with("sq") => "Albanian",
        _ => "English",
    };
    format!(
        "RESTAURANT: {venue}\nLANGUAGE: {language}\nFACT: {}\n\nWrite the post.",
        subject.fact()
    )
}

/// What a model must not be allowed to publish, whatever it wrote.
///
/// THE LAST GATE. The system prompt asks for these to be absent; this checks. A
/// model that invents "50% off" because its training says posts look like that
/// would otherwise put a discount the venue is not running in front of its
/// customers, and the venue would have to honour it or argue with the person
/// holding the screenshot.
///
/// Returns the reason it is unusable, or `None` if it is fine.
pub fn unusable(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if t.is_empty() {
        return Some("the model returned nothing");
    }
    // Long enough to be a post, short enough to be one.
    if t.chars().count() > 600 {
        return Some("far longer than a post");
    }
    let low = t.to_lowercase();
    // A number followed by a percent sign is a discount. There is no fact in
    // this module that contains one, so its presence means it was invented.
    if low.contains('%') {
        return Some("mentions a percentage, which no fact here contains");
    }
    for word in [
        "discount", "sale", "% off", "free ", "coupon", "promo",
        "знижк", "акці", "безкоштовн", "купон",
        "zbritje", "falas", "kupon",
    ] {
        if low.contains(word) {
            return Some("mentions an offer the restaurant is not running");
        }
    }
    for word in ["hurry", "limited time", "last chance", "don't miss",
                 "поспіш", "встигн", "останній шанс",
                 "nxito", "mos e humb"] {
        if low.contains(word) {
            return Some("manufactured urgency");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(id: &str, key: &str, state: State, at: i64) -> Post {
        Post {
            id: id.into(),
            subject_key: key.into(),
            subject_tag: "new".into(),
            text: "Sake Futomaki is on the menu again.".into(),
            channel: Channel::Telegram,
            state,
            created_ms: at,
            decided_ms: 0,
            error: String::new(),
        }
    }

    #[test]
    fn posts_survive_the_byte_image() {
        let mut p = Posts::create().expect("create");
        p.put(&post("p1", "back:item-01", State::Draft, 100));
        p.put(&post("p2", "new:item-02", State::Published, 200));
        let bytes = p.to_bytes().expect("bytes");

        let p = Posts::load(&bytes).expect("load");
        assert_eq!(p.all().len(), 2);
        // Newest first.
        assert_eq!(p.all()[0].id, "p2");
        assert_eq!(p.drafts().len(), 1);
        assert_eq!(p.get("p1").unwrap().state, State::Draft);
        assert_eq!(p.get("nope"), None);
    }

    /// The property that stops a venue posting the same thing every hour.
    #[test]
    fn a_fact_already_put_to_the_owner_is_not_drafted_again() {
        let mut p = Posts::create().expect("create");
        p.put(&post("p1", "back:item-01", State::Draft, 100));
        assert!(p.already_seen("back:item-01"));
        assert!(!p.already_seen("back:item-02"));

        // REJECTED counts too. A fact the owner turned down coming back an hour
        // later is how an assistant becomes something people switch off.
        p.put(&post("p2", "new:item-09", State::Rejected, 200));
        assert!(p.already_seen("new:item-09"));
        // And so does published.
        p.put(&post("p3", "top:item-03", State::Published, 300));
        assert!(p.already_seen("top:item-03"));
    }

    #[test]
    fn a_subject_key_is_stable_and_distinct() {
        let a = Subject::BackOnTheMenu { product: "item-01".into() };
        let b = Subject::BackOnTheMenu { product: "item-01".into() };
        assert_eq!(a.key(), b.key());
        assert_ne!(a.key(), Subject::NewDish { product: "item-01".into() }.key());
        // The count does NOT enter the key: "most ordered" is one fact about a
        // dish, and a different number next week is not a new thing to say.
        assert_eq!(
            Subject::MostOrdered { product: "x".into(), orders: 4 }.key(),
            Subject::MostOrdered { product: "x".into(), orders: 9 }.key()
        );
    }

    #[test]
    fn a_fact_reads_as_a_sentence() {
        assert!(Subject::BackOnTheMenu { product: "Sake Futomaki".into() }
            .fact()
            .contains("available again"));
        assert!(Subject::MostOrdered { product: "Ebi Maki".into(), orders: 12 }
            .fact()
            .contains("12 times"));
        assert_eq!(Subject::Reopened.fact(), "the restaurant is open again");
    }

    /// The last gate. Each of these is something a marketing-trained model
    /// reaches for by default and that the restaurant would then have to honour.
    #[test]
    fn invented_offers_are_refused() {
        for bad in [
            "Sake Futomaki is back! 20% off today only.",
            "Our sushi is back. Limited time discount!",
            "Суші повернулись — знижка 10%!",
            "Ebi Maki is back, and the first one is free today",
            "Zbritje 30% sot!",
            "Hurry, the sushi is back",
            "Поспішайте, суші знову в меню",
        ] {
            assert!(unusable(bad).is_some(), "let through: {bad}");
        }
    }

    #[test]
    fn an_honest_post_passes() {
        for good in [
            "Sake Futomaki is on the menu again. We missed it too.",
            "Суші сет знову в меню — сьогодні готуємо.",
            "Ebi Maki was the one you ordered most this week. Thank you.",
            "Byrek me spinaq është sërish në meny.",
            "We are open again. 🙂",
        ] {
            assert_eq!(unusable(good), None, "wrongly refused: {good}");
        }
    }

    #[test]
    fn nothing_and_a_wall_of_text_are_both_refused() {
        assert!(unusable("").is_some());
        assert!(unusable("   \n  ").is_some());
        assert!(unusable(&"word ".repeat(200)).is_some());
    }

    /// The prompt must carry the fact and the language, and nothing that invites
    /// the model to choose a subject.
    #[test]
    fn the_prompt_carries_one_fact_and_a_language() {
        let p = prompt_for(
            &Subject::BackOnTheMenu { product: "Sake Futomaki".into() },
            "Dubin & Sushi",
            "uk",
        );
        assert!(p.contains("Dubin & Sushi"));
        assert!(p.contains("Ukrainian"));
        assert!(p.contains("Sake Futomaki"));
        assert!(p.contains("FACT:"));
        assert!(prompt_for(&Subject::Reopened, "V", "sq").contains("Albanian"));
        assert!(prompt_for(&Subject::Reopened, "V", "en").contains("English"));
    }

    /// The system prompt has to forbid the two failure modes by name.
    #[test]
    fn the_instructions_ban_what_the_gate_checks() {
        for banned in ["discount", "limited time", "NEVER invent"] {
            assert!(SYSTEM_POST.contains(banned), "missing {banned}");
        }
    }

    #[test]
    fn the_catalogue_snapshot_round_trips() {
        let mut p = Posts::create().expect("create");
        assert!(p.catalogue_snapshot().is_empty(), "nothing remembered yet");
        p.set_catalogue_snapshot(&[("a".into(), true), ("b".into(), false)]);
        let bytes = p.to_bytes().expect("bytes");
        let back = Posts::load(&bytes).expect("load").catalogue_snapshot();
        assert_eq!(back, vec![("a".to_string(), true), ("b".to_string(), false)]);
    }

    /// The first run must say NOTHING. Otherwise a venue switching this on is
    /// handed a draft about every dish on a menu that has not changed.
    #[test]
    fn the_first_run_is_silent() {
        let current = vec![
            ("a".to_string(), "Sake".to_string(), true),
            ("b".to_string(), "Ebi".to_string(), true),
        ];
        let got = derive_subjects(&[], &current, None, false, true, &|_| false);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn a_dish_returning_and_a_dish_arriving_are_both_noticed() {
        let previous = vec![("a".to_string(), false), ("b".to_string(), true)];
        let current = vec![
            ("a".to_string(), "Sake".to_string(), true),   // came back
            ("b".to_string(), "Ebi".to_string(), true),    // unchanged
            ("c".to_string(), "Uni".to_string(), true),    // brand new
        ];
        let got = derive_subjects(&previous, &current, None, false, true, &|_| false);
        assert!(got.contains(&Subject::BackOnTheMenu { product: "Sake".into() }), "{got:?}");
        assert!(got.contains(&Subject::NewDish { product: "Uni".into() }), "{got:?}");
        assert_eq!(got.len(), 2, "an unchanged dish is not news: {got:?}");
    }

    /// A dish going OFF the menu is not something to announce.
    #[test]
    fn a_dish_disappearing_is_not_a_post() {
        let previous = vec![("a".to_string(), true)];
        let current = vec![("a".to_string(), "Sake".to_string(), false)];
        assert!(derive_subjects(&previous, &current, None, false, true, &|_| false).is_empty());
    }

    #[test]
    fn one_order_is_not_a_trend() {
        let prev = vec![("a".to_string(), true)];
        let cur = vec![("a".to_string(), "Sake".to_string(), true)];
        let few = derive_subjects(&prev, &cur, Some(("a".into(), 2)), false, true, &|_| false);
        assert!(few.is_empty(), "two orders is noise: {few:?}");
        let many = derive_subjects(&prev, &cur, Some(("a".into(), 7)), false, true, &|_| false);
        assert_eq!(many, vec![Subject::MostOrdered { product: "Sake".into(), orders: 7 }]);
    }

    #[test]
    fn reopening_is_noticed_only_on_the_transition() {
        let p = vec![("a".to_string(), true)];
        let c = vec![("a".to_string(), "Sake".to_string(), true)];
        assert!(derive_subjects(&p, &c, None, true, true, &|_| false).contains(&Subject::Reopened));
        assert!(!derive_subjects(&p, &c, None, false, true, &|_| false).contains(&Subject::Reopened));
        assert!(!derive_subjects(&p, &c, None, true, false, &|_| false).contains(&Subject::Reopened));
    }

    /// Anything already put to the owner is filtered out, whatever they said.
    #[test]
    fn subjects_already_seen_are_dropped() {
        let previous = vec![("a".to_string(), false)];
        let current = vec![("a".to_string(), "Sake".to_string(), true)];
        let seen = |k: &str| k == "back:Sake";
        assert!(derive_subjects(&previous, &current, None, false, true, &seen).is_empty());
    }

    #[test]
    fn a_hostile_draft_cannot_forge_a_record() {
        let mut p = Posts::create().expect("create");
        let mut evil = post("p1", "k", State::Draft, 1);
        evil.text = r#"x","state":"published"#.into();
        p.put(&evil);
        assert_eq!(p.get("p1").unwrap().state, State::Draft, "state must not move");
    }
}
