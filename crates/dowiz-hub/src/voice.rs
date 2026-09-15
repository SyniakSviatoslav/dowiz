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
//! THREE LANGUAGES, because the venue is Albanian, the operator is Ukrainian
//! and the tooling is English. A courier must not have to switch language to
//! be understood.

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
        let uk = lang.starts_with("uk");
        let sq = lang.starts_with("sq");
        fn verb_word(v: &str, uk: bool, sq: bool) -> &str {
            match (v, uk, sq) {
                ("confirm", true, _) => "підтвердити",
                ("confirm", _, true) => "konfirmo",
                ("confirm", ..) => "confirm",
                ("preparing", true, _) => "готуємо",
                ("preparing", _, true) => "po gatuhet",
                ("preparing", ..) => "start preparing",
                ("ready", true, _) => "готове",
                ("ready", _, true) => "gati",
                ("ready", ..) => "mark ready",
                ("reject", true, _) => "відхилити",
                ("reject", _, true) => "refuzo",
                ("reject", ..) => "reject",
                ("cancel", true, _) => "скасувати",
                ("cancel", _, true) => "anulo",
                ("cancel", ..) => "cancel",
                (other, ..) => other,
            }
        }
        let which = |t: &Target| match (t, uk, sq) {
            (Target::Newest, true, _) => "останнє".to_string(),
            (Target::Newest, _, true) => "të fundit".to_string(),
            (Target::Newest, ..) => "the newest".to_string(),
            (Target::Oldest, true, _) => "найстаріше".to_string(),
            (Target::Oldest, _, true) => "më të vjetrin".to_string(),
            (Target::Oldest, ..) => "the oldest".to_string(),
            (Target::Digits(d), ..) => format!("#{d}"),
            (Target::Unsaid, true, _) => "поточне".to_string(),
            (Target::Unsaid, _, true) => "aktualin".to_string(),
            (Target::Unsaid, ..) => "the current one".to_string(),
        };
        match self {
            Command::Order { verb, target } => format!("{} {}", verb_word(verb, uk, sq), which(target)),
            Command::Pickup { target } => match (uk, sq) {
                (true, _) => format!("забрав {}", which(target)),
                (_, true) => format!("mora {}", which(target)),
                _ => format!("picked up {}", which(target)),
            },
            Command::Deliver { target } => match (uk, sq) {
                (true, _) => format!("доставив {}", which(target)),
                (_, true) => format!("dorëzova {}", which(target)),
                _ => format!("delivered {}", which(target)),
            },
            Command::Shift { open: true } => match (uk, sq) {
                (true, _) => "почати зміну".into(),
                (_, true) => "nis turnin".into(),
                _ => "start the shift".into(),
            },
            Command::Shift { open: false } => match (uk, sq) {
                (true, _) => "завершити зміну".into(),
                (_, true) => "mbyll turnin".into(),
                _ => "end the shift".into(),
            },
            Command::Status => match (uk, sq) {
                (true, _) => "статус".into(),
                (_, true) => "statusi".into(),
                _ => "status".into(),
            },
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
    // Digits win: a number said aloud is the most specific reference there is.
    let digits: String = t.chars().filter(char::is_ascii_digit).collect();
    if digits.len() >= 3 {
        return Target::Digits(digits);
    }
    if has(t, &["останнє", "останній", "остання", "last", "latest", "fundit", "fundit"]) {
        return Target::Newest;
    }
    if has(t, &["перше", "перший", "найстаріше", "first", "oldest", "parin", "pari"]) {
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
    if confidence < MIN_CONFIDENCE {
        return Command::Unclear("not sure I heard that");
    }
    let t = norm(transcript);
    if t.is_empty() {
        return Command::Unclear("nothing said");
    }

    // Shift, first: it takes no target and cannot be confused with an order.
    let open = has(&t, &["почати", "почни", "відкрити", "start", "open", "nis", "hap"]);
    let close = has(&t, &["завершити", "заверши", "закрити", "end", "finish", "close", "mbyll", "perfundo"]);
    if has(&t, &["зміну", "зміна", "shift", "turn", "turnin"]) && (open || close) {
        // Both words present is not a command, it is a sentence about shifts.
        if open && close {
            return Command::Unclear("start or end?");
        }
        return Command::Shift { open };
    }

    let target = target_of(&t);

    // Consequential verbs. Checked BEFORE status, so "reject" is never read as
    // a question about rejections.
    let reject = has(&t, &["відхилити", "відхили", "відмова", "reject", "refuse", "refuzo"]);
    let cancel = has(&t, &["скасувати", "скасуй", "cancel", "anulo"]);
    let confirm = has(&t, &["підтвердити", "підтверди", "прийняти", "прийми", "confirm", "accept", "konfirmo", "prano"]);
    let preparing = has(&t, &["готуємо", "готую", "готувати", "preparing", "cooking", "gatuaj"]);
    let ready = has(&t, &["готове", "готово", "готовий", "ready", "gati"]);
    let picked = has(&t, &["забрав", "забрала", "взяв", "picked", "pickup", "mora"]);
    let delivered = has(&t, &["доставив", "доставила", "віддав", "delivered", "dorezova", "dorezoi"]);

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

    if has(&t, &["статус", "скільки", "що", "стан", "status", "how many", "what", "sa", "cfare"]) {
        return Command::Status;
    }

    // Anything else is a question, not a command. The caller decides whether
    // there is an assistant to answer it.
    Command::Ask(transcript.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use Speaker::{Courier, Owner};

    fn owner(t: &str) -> Command {
        classify(t, 0.9, true, Owner)
    }
    fn courier(t: &str) -> Command {
        classify(t, 0.9, true, Courier)
    }

    /// The gate before the words: an unsure recogniser is not obeyed.
    #[test]
    fn low_confidence_is_never_a_command() {
        assert_eq!(classify("скасувати останнє", 0.5, true, Owner), Command::Unclear("not sure I heard that"));
        assert_eq!(classify("скасувати останнє", 0.0, true, Owner), Command::Unclear("not sure I heard that"));
        // And an interim transcript is half a sentence.
        assert_eq!(classify("скасу", 0.99, false, Owner), Command::Unclear("interim"));
    }

    #[test]
    fn the_owners_verbs_map_to_the_kernels() {
        assert_eq!(owner("підтверди останнє"), Command::Order { verb: "confirm", target: Target::Newest });
        assert_eq!(owner("confirm the last one"), Command::Order { verb: "confirm", target: Target::Newest });
        assert_eq!(owner("konfirmo të fundit"), Command::Order { verb: "confirm", target: Target::Newest });
        assert_eq!(owner("готове"), Command::Order { verb: "ready", target: Target::Unsaid });
        assert_eq!(owner("готуємо перше"), Command::Order { verb: "preparing", target: Target::Oldest });
        assert_eq!(owner("відхили 4821"), Command::Order { verb: "reject", target: Target::Digits("4821".into()) });
    }

    /// The word whose meaning depends entirely on who says it.
    #[test]
    fn ready_means_different_things_to_the_kitchen_and_the_courier() {
        assert_eq!(owner("готово"), Command::Order { verb: "ready", target: Target::Unsaid });
        assert_eq!(courier("готово"), Command::Deliver { target: Target::Unsaid });
    }

    /// Roles cannot reach into each other's actions, and are TOLD so rather
    /// than silently ignored.
    #[test]
    fn a_courier_cannot_confirm_and_an_owner_cannot_deliver() {
        assert_eq!(courier("підтверди останнє"), Command::Unclear("that is the kitchen's to decide"));
        assert_eq!(courier("відхили 1234"), Command::Unclear("that is the kitchen's to decide"));
        assert_eq!(owner("забрав"), Command::Unclear("that is the courier's to say"));
    }

    /// The dangerous utterance: a person correcting themselves mid-sentence.
    /// Precedence would pick one. Refusing asks again, which costs three
    /// seconds instead of an order.
    #[test]
    fn two_commands_in_one_breath_are_refused() {
        assert_eq!(owner("скасуй ні підтверди"), Command::Unclear("heard more than one command"));
        assert_eq!(owner("cancel no confirm"), Command::Unclear("heard more than one command"));
        assert_eq!(courier("забрав доставив"), Command::Unclear("heard more than one command"));
    }

    #[test]
    fn shifts_open_and_close() {
        assert_eq!(courier("почати зміну"), Command::Shift { open: true });
        assert_eq!(courier("start shift"), Command::Shift { open: true });
        assert_eq!(courier("завершити зміну"), Command::Shift { open: false });
        assert_eq!(courier("mbyll turnin"), Command::Shift { open: false });
        // A sentence ABOUT shifts is not a command to change one.
        assert_eq!(courier("почати чи завершити зміну"), Command::Unclear("start or end?"));
    }

    #[test]
    fn digits_beat_every_other_reference() {
        assert_eq!(owner("підтверди останнє 9931"), Command::Order { verb: "confirm", target: Target::Digits("9931".into()) });
        // Fewer than three digits is not an order number; it is probably a
        // quantity, and guessing would act on the wrong order.
        assert_eq!(owner("підтверди 12"), Command::Order { verb: "confirm", target: Target::Unsaid });
    }

    #[test]
    fn a_question_is_not_a_command() {
        assert!(matches!(owner("скільки ще чекає"), Command::Status));
        assert!(matches!(owner("how many are waiting"), Command::Status));
        match owner("яка виручка за вівторок минулого тижня") {
            Command::Ask(q) => assert!(q.contains("виручка")),
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    /// Everything that changes something must be confirmed; nothing that only
    /// reads should be.
    #[test]
    fn consequence_decides_what_needs_confirming() {
        for c in [
            owner("підтверди останнє"),
            owner("відхили 1234"),
            courier("забрав"),
            courier("доставив"),
            courier("почати зміну"),
        ] {
            assert!(c.needs_confirmation(), "{c:?} changes something and must be confirmed");
        }
        for c in [owner("скільки чекає"), owner("яка виручка")] {
            assert!(!c.needs_confirmation(), "{c:?} only reads and must run at once");
        }
    }

    /// A read-back the speaker cannot understand is not a confirmation.
    #[test]
    fn readback_speaks_the_speakers_language() {
        let c = owner("підтверди останнє");
        assert_eq!(c.readback("uk"), "підтвердити останнє");
        assert_eq!(c.readback("en"), "confirm the newest");
        assert_eq!(c.readback("sq"), "konfirmo të fundit");
        assert_eq!(owner("відхили 4821").readback("uk"), "відхилити #4821");
        assert_eq!(courier("почати зміну").readback("uk"), "почати зміну");
    }

    /// Apostrophes and punctuation must not change the meaning: a recogniser
    /// spells "кур'єр" whichever way it likes.
    #[test]
    fn punctuation_and_apostrophes_do_not_matter() {
        assert_eq!(owner("Підтверди, останнє!"), Command::Order { verb: "confirm", target: Target::Newest });
        assert_eq!(owner("  ГОТОВЕ  "), Command::Order { verb: "ready", target: Target::Unsaid });
    }

    #[test]
    fn silence_is_not_a_command() {
        assert_eq!(owner(""), Command::Unclear("nothing said"));
        assert_eq!(owner("   ...  "), Command::Unclear("nothing said"));
    }

    /// The property that keeps this safe: NOTHING in this module can produce a
    /// consequential command that does not ask first.
    #[test]
    fn no_utterance_produces_an_unconfirmed_consequential_command() {
        let corpus = [
            "підтверди", "готове", "відхили останнє", "скасуй 1234", "забрав", "доставив",
            "почати зміну", "завершити зміну", "confirm", "ready", "reject 9999",
            "скільки чекає", "нічого", "алло", "що там з замовленням",
        ];
        for phrase in corpus {
            for who in [Owner, Courier] {
                let c = classify(phrase, 0.95, true, who);
                if let Command::Order { .. } | Command::Pickup { .. } | Command::Deliver { .. } | Command::Shift { .. } = c {
                    assert!(c.needs_confirmation(), "{phrase:?} as {who:?} would act unconfirmed");
                }
            }
        }
    }
}
