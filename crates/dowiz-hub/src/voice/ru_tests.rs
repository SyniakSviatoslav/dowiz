//! Russian, the fourth language (lane W-RU, 2026-09-27): the owner's and the
//! courier's words are heard, and read back, in Russian -- and Russian adds no
//! way to act unconfirmed.
use super::*;
use Speaker::{Courier, Owner};

fn owner(t: &str) -> Command {
    classify(t, 0.9, true, Owner)
}
fn courier(t: &str) -> Command {
    classify(t, 0.9, true, Courier)
}

#[test]
fn the_owners_russian_verbs_map_to_the_kernels() {
    assert_eq!(owner("подтверди последний"), Command::Order { verb: "confirm", target: Target::Newest });
    assert_eq!(owner("прими первый"), Command::Order { verb: "confirm", target: Target::Oldest });
    assert_eq!(owner("отклони 4821"), Command::Order { verb: "reject", target: Target::Digits("4821".into()) });
    assert_eq!(owner("отмени последний"), Command::Order { verb: "cancel", target: Target::Newest });
    assert_eq!(owner("готовим 4821"), Command::Order { verb: "preparing", target: Target::Digits("4821".into()) });
    assert_eq!(owner("готов"), Command::Order { verb: "ready", target: Target::Unsaid });
    assert_eq!(owner("сколько ждёт"), Command::Status);
    assert_eq!(owner("забрал"), Command::Unclear("that is the courier's to say"));
}

#[test]
fn the_couriers_russian_words() {
    assert_eq!(courier("взял 4821"), Command::Pickup { target: Target::Digits("4821".into()) });
    assert_eq!(courier("доставил последний"), Command::Deliver { target: Target::Newest });
    assert_eq!(courier("начать смену"), Command::Shift { open: true });
    assert_eq!(courier("закрыть смену"), Command::Shift { open: false });
    assert_eq!(courier("подтверди"), Command::Unclear("that is the kitchen's to decide"));
    // "Where next" is a question, not a command: the assistant answers it.
    assert_eq!(courier("где следующий"), Command::Ask("где следующий".into()));
}

#[test]
fn two_russian_verbs_in_one_breath_are_refused() {
    assert_eq!(owner("отмени нет подтверди"), Command::Unclear("heard more than one command"));
    assert_eq!(courier("начать и закрыть смену"), Command::Unclear("start or end?"));
}

#[test]
fn readback_speaks_russian() {
    assert_eq!(owner("подтверди последний").readback("ru"), "подтвердить последний");
    assert_eq!(owner("отклони 4821").readback("ru-RU"), "отклонить #4821");
    assert_eq!(owner("готово").readback("ru"), "готово текущий");
    assert_eq!(owner("прими первый").readback("ru"), "подтвердить самый старый");
    assert_eq!(courier("взял 4821").readback("ru"), "забрал #4821");
    assert_eq!(courier("доставил").readback("ru"), "доставил текущий");
    assert_eq!(courier("начать смену").readback("ru"), "начать смену");
    assert_eq!(courier("закрыть смену").readback("ru"), "завершить смену");
    assert_eq!(owner("сколько ждёт").readback("ru"), "статус");
    // The other languages are untouched.
    assert_eq!(owner("подтверди последний").readback("uk"), "підтвердити останнє");
    assert_eq!(owner("подтверди последний").readback("en"), "confirm the newest");
}

#[test]
fn no_russian_utterance_acts_unconfirmed() {
    for phrase in ["подтверди", "готово", "отклони последний", "отмени 1234", "взял", "доставил", "начать смену", "закрыть смену", "сколько ждёт", "алло"] {
        for who in [Owner, Courier] {
            let c = classify(phrase, 0.95, true, who);
            if let Command::Order { .. } | Command::Pickup { .. } | Command::Deliver { .. } | Command::Shift { .. } = c {
                assert!(c.needs_confirmation(), "{phrase:?} as {who:?} would act unconfirmed");
            }
        }
    }
}
