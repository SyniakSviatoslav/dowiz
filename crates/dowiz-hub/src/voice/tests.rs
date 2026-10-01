use super::*;
use Speaker::{Courier, Owner};

fn owner(t: &str) -> Command {
    classify(t, 0.9, true, Owner)
}
fn courier(t: &str) -> Command {
    classify(t, 0.9, true, Courier)
}

/// The ticket's own number as the board prints it -- hex, "#A57B" -- names
/// that ticket (QA walk Q3: it was read as "57" and refused). The positive
/// twin: plain digits, and digits said apart, still name it as before.
#[test]
fn the_number_the_board_shows_names_the_ticket() {
    assert_eq!(owner("move order A57B to ready"), Command::Order { verb: "ready", target: Target::Digits("a57b".into()) });
    assert_eq!(owner("confirm #a57b"), Command::Order { verb: "confirm", target: Target::Digits("a57b".into()) });
    assert_eq!(owner("готове 4821"), Command::Order { verb: "ready", target: Target::Digits("4821".into()) });
    assert_eq!(owner("ready 4 8 2 1"), Command::Order { verb: "ready", target: Target::Digits("4821".into()) });
    // A word of hex letters with no digit is a word, not a number.
    assert_eq!(owner("confirm the cafe order"), Command::Order { verb: "confirm", target: Target::Unsaid });
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
