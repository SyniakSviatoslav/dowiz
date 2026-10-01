//! Voice commands, classified deterministically.
//!
//! THE RULES COME FROM P64, and they are not negotiable here:
//!
//!   * The classifier is DETERMINISTIC. No model decides what a consequential
//!     command meant. A model that mishears "reject" as "ready" costs a
//!     customer their dinner and the venue the argument afterwards.
//!   * A CONSEQUENTIAL command is never executed from one utterance. Voice
//!     PROPOSES; a person confirms. `needs_confirmation` is how this module
//!     says so, and the route refuses to act without it.
//!   * Ambiguity is REJECTED, never guessed. Two orders that could both be "the
//!     ready one" produce a question, not a coin toss.
//!   * Low confidence is rejected before the words are even read.
//!   * Voice works with the AI switched off. Nothing in this file consults it;
//!     anything the grammar does not recognise becomes `Ask`, which the caller
//!     may route to an assistant IF one is configured, and may drop otherwise.
//!
//! WHY A GRAMMAR AND NOT A MODEL. The command vocabulary is small, fixed and
//! spoken under bad conditions -- a kitchen, a road, a phone at arm's length.
//! A grammar answers in microseconds, offline, identically every time, and can
//! say "I did not understand" honestly. A model would be slower, needs a
//! network or 2 GB of RAM, and is confidently wrong in exactly the cases that
//! matter.
//!
//! EVERY LANGUAGE OF `crate::lang::LANGS`, because the venue is Albanian, the
//! operator is Ukrainian, the tooling is English and many guests and staff
//! speak Russian. A courier must not have to switch language to be understood.

/// Who is speaking. The same words mean different things to different people:
/// "готово" from a kitchen means the food is ready, from a courier it means
/// delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speaker {
    Owner,
    Courier,
}

/// What an order is referred to as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// "the last one", "останнє" -- the most recent.
    Newest,
    /// "the first", "перше" -- the one waiting longest.
    Oldest,
    /// Digits the speaker read out; matched against the TAIL of an order id.
    Digits(String),
    /// Nothing said. Resolvable only when exactly one order is a candidate.
    Unsaid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// An owner action, by its kernel verb: confirm, preparing, ready, reject, cancel.
    Order { verb: &'static str, target: Target },
    /// A courier moving their own run.
    Pickup { target: Target },
    Deliver { target: Target },
    /// Shift on or off.
    Shift { open: bool },
    /// "what is waiting", "скільки замовлень" -- read-only, safe to run at once.
    Status,
    /// Not a command. The caller may pass it to an assistant if one is
    /// configured; nothing here requires that.
    Ask(String),
    /// Heard, not understood. Carries what to say back.
    Unclear(&'static str),
}

impl Command {
    /// Does acting on this change anything a customer would notice?
    ///
    /// Everything that moves an order or a shift does. `Status` and `Ask` do
    /// not, and running those immediately is what makes voice feel like an
    /// answer rather than a form.
    pub fn needs_confirmation(&self) -> bool {
        matches!(
            self,
            Command::Order { .. } | Command::Pickup { .. } | Command::Deliver { .. } | Command::Shift { .. }
        )
    }

    /// A short line to read back before acting, in the speaker's own language.
    /// Confirming something the person cannot restate is not confirmation.
    pub fn readback(&self, lang: &str) -> String {
        // One word per language, in `crate::lang::LANGS` order (sq, en, uk,
        // ru); the array's length is the language count, so a language added
        // there does not compile until it is said here. Unknown is English.
        type Say = [&'static str; crate::lang::LANGS.len()];
        let i = crate::lang::LANGS.iter().position(|l| lang.starts_with(l)).unwrap_or(1);
        let verb_word = |v: &str| -> String {
            let w: Say = match v {
                "confirm" => ["konfirmo", "confirm", "підтвердити", "подтвердить"],
                "preparing" => ["po gatuhet", "start preparing", "готуємо", "готовим"],
                "ready" => ["gati", "mark ready", "готове", "готово"],
                "reject" => ["refuzo", "reject", "відхилити", "отклонить"],
                "cancel" => ["anulo", "cancel", "скасувати", "отменить"],
                other => return other.to_string(),
            };
            w[i].to_string()
        };
        let which = |t: &Target| -> String {
            let w: Say = match t {
                Target::Newest => ["të fundit", "the newest", "останнє", "последний"],
                Target::Oldest => ["më të vjetrin", "the oldest", "найстаріше", "самый старый"],
                Target::Unsaid => ["aktualin", "the current one", "поточне", "текущий"],
                Target::Digits(d) => return format!("#{}", d.to_uppercase()),
            };
            w[i].to_string()
        };
        let said = |w: Say| w[i].to_string();
        match self {
            Command::Order { verb, target } => format!("{} {}", verb_word(verb), which(target)),
            Command::Pickup { target } => format!("{} {}", said(["mora", "picked up", "забрав", "забрал"]), which(target)),
            Command::Deliver { target } => format!("{} {}", said(["dorëzova", "delivered", "доставив", "доставил"]), which(target)),
            Command::Shift { open: true } => said(["nis turnin", "start the shift", "почати зміну", "начать смену"]),
            Command::Shift { open: false } => said(["mbyll turnin", "end the shift", "завершити зміну", "завершить смену"]),
            Command::Status => said(["statusi", "status", "статус", "статус"]),
            Command::Ask(q) => q.clone(),
            Command::Unclear(why) => (*why).to_string(),
        }
    }
}

/// Below this, the words are not read at all.
///
/// A phone in a kitchen mis-hears constantly. Acting on a transcript the
/// recogniser itself doubts is how "cancel order" becomes "confirm order".
pub const MIN_CONFIDENCE: f64 = 0.55;

fn norm(s: &str) -> String {
    // Lowercase, collapse whitespace, drop punctuation. Apostrophes go too:
    // "кур'єр" and "курєр" must be the same word, and a recogniser picks
    // whichever it feels like.
    let mut out = String::with_capacity(s.len());
    let mut space = false;
    for c in s.to_lowercase().chars() {
        if c.is_alphanumeric() {
            if space && !out.is_empty() {
                out.push(' ');
            }
            space = false;
            out.push(c);
        } else {
            space = true;
        }
    }
    out
}

fn has(hay: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| {
        // Word-boundary containment: "готово" must not match inside another
        // word, and a bare substring test would fire on half the vocabulary.
        hay.split(' ').any(|w| w == *n) || hay.contains(&format!(" {n} ")) || hay.starts_with(&format!("{n} ")) || hay.ends_with(&format!(" {n}"))
    })
}

fn target_of(t: &str) -> Target {
    // THE NUMBER THE BOARD SHOWS. The kitchen board calls a ticket by the last
    // four characters of its id (`kitchen-logic.js` shortId), and an id is
    // hex: "#A57B". Only the digits were read, so "A57B ready" became "57",
    // too short, and the hub asked "which one?" about a ticket the cook had
    // just named (QA walk Q3, 2026-09-26). A word of hex with a digit in it is
    // that reference, as typed or read off the ticket.
    if let Some(tag) = t.split(' ').find(|w| {
        (3..=12).contains(&w.len())
            && w.chars().all(|c| c.is_ascii_hexdigit())
            && w.chars().any(|c| c.is_ascii_digit())
            && w.chars().any(|c| c.is_ascii_alphabetic())
    }) {
        return Target::Digits(tag.to_string());
    }
    // Digits win: a number said aloud is the most specific reference there is.
    let digits: String = t.chars().filter(char::is_ascii_digit).collect();
    if digits.len() >= 3 {
        return Target::Digits(digits);
    }
    if has(t, &["останнє", "останній", "остання", "последний", "последнее", "последняя", "last", "latest", "fundit", "fundit"]) {
        return Target::Newest;
    }
    if has(t, &["перше", "перший", "найстаріше", "первый", "первое", "первая", "старый", "first", "oldest", "parin", "pari"]) {
        return Target::Oldest;
    }
    Target::Unsaid
}

/// Classify one utterance.
///
/// `confidence` is the recogniser's own, 0..1. `is_final` distinguishes a
/// settled transcript from an interim guess -- acting on an interim one means
/// acting on half a sentence.
pub fn classify(transcript: &str, confidence: f64, is_final: bool, who: Speaker) -> Command {
    if !is_final {
        return Command::Unclear("interim");
    }
    // NaN is not confidence (W-AUDIT S4, 2026-09-27): `NaN < 0.55` is false,
    // so a recogniser that answered NaN had "cancel 4821" carried out.
    if confidence.is_nan() || confidence < MIN_CONFIDENCE {
        return Command::Unclear("not sure I heard that");
    }
    let t = norm(transcript);
    if t.is_empty() {
        return Command::Unclear("nothing said");
    }

    // Shift, first: it takes no target and cannot be confused with an order.
    let open = has(&t, &["почати", "почни", "відкрити", "начать", "начни", "открыть", "открой", "start", "open", "nis", "hap"]);
    let close = has(&t, &["завершити", "заверши", "закрити", "завершить", "закрыть", "закрой", "закончить", "end", "finish", "close", "mbyll", "perfundo"]);
    if has(&t, &["зміну", "зміна", "смену", "смена", "shift", "turn", "turnin"]) && (open || close) {
        // Both words present is not a command, it is a sentence about shifts.
        if open && close {
            return Command::Unclear("start or end?");
        }
        return Command::Shift { open };
    }

    let target = target_of(&t);

    // Consequential verbs. Checked BEFORE status, so "reject" is never read as
    // a question about rejections.
    let reject = has(&t, &["відхилити", "відхили", "відмова", "отклонить", "отклони", "отказ", "reject", "refuse", "refuzo"]);
    let cancel = has(&t, &["скасувати", "скасуй", "отменить", "отмени", "cancel", "anulo"]);
    let confirm = has(&t, &["підтвердити", "підтверди", "прийняти", "прийми", "подтвердить", "подтверди", "принять", "прими", "confirm", "accept", "konfirmo", "prano"]);
    let preparing = has(&t, &["готуємо", "готую", "готувати", "готовим", "готовлю", "готовить", "preparing", "cooking", "gatuaj"]);
    let ready = has(&t, &["готове", "готово", "готовий", "готов", "готова", "ready", "gati"]);
    let picked = has(&t, &["забрав", "забрала", "взяв", "забрал", "взял", "взяла", "picked", "pickup", "mora"]);
    let delivered = has(&t, &["доставив", "доставила", "віддав", "доставил", "отдал", "отдала", "delivered", "dorezova", "dorezoi"]);

    // TWO CONSEQUENTIAL VERBS IN ONE UTTERANCE is the dangerous case -- "cancel,
    // no, confirm" -- and it is refused rather than resolved by precedence.
    let n = [reject, cancel, confirm, preparing, ready, picked, delivered]
        .iter()
        .filter(|x| **x)
        .count();
    if n > 1 {
        return Command::Unclear("heard more than one command");
    }

    match who {
        Speaker::Courier => {
            if picked {
                return Command::Pickup { target };
            }
            if delivered || ready {
                // For a courier "ready/готово" means the run is done. This is
                // exactly the word whose meaning depends on who says it.
                return Command::Deliver { target };
            }
            // A courier cannot confirm or reject an order; those are the
            // kitchen's. Saying so is better than silently doing nothing.
            if confirm || reject || cancel || preparing {
                return Command::Unclear("that is the kitchen's to decide");
            }
        }
        Speaker::Owner => {
            if reject {
                return Command::Order { verb: "reject", target };
            }
            if cancel {
                return Command::Order { verb: "cancel", target };
            }
            if confirm {
                return Command::Order { verb: "confirm", target };
            }
            if preparing {
                return Command::Order { verb: "preparing", target };
            }
            if ready {
                return Command::Order { verb: "ready", target };
            }
            if picked || delivered {
                return Command::Unclear("that is the courier's to say");
            }
        }
    }

    if has(&t, &["статус", "скільки", "що", "стан", "сколько", "что", "состояние", "status", "how many", "what", "sa", "cfare"]) {
        return Command::Status;
    }

    // Anything else is a question, not a command. The caller decides whether
    // there is an assistant to answer it.
    Command::Ask(transcript.trim().to_string())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ru_tests;

/// W-AUDIT S4 (2026-09-27): NaN is not confidence.
#[cfg(test)]
mod nan_tests;
