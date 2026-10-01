//! WHAT TO SAY, AND WHAT MUST NOT GO OUT: the subjects worth a post, the
//! instructions the model writes under, and the last gate on what it wrote.

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
    let language = crate::lang::english_name(lang);
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
