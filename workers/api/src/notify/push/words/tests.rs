//! Every word in every language; nothing but the order number.
use super::*;

#[test]
fn every_language_has_every_word_and_none_is_the_english_by_accident() {
    for st in ["CONFIRMED", "PREPARING", "READY", "IN_DELIVERY", "DELIVERED", "PICKED_UP", "REJECTED", "CANCELLED", "SCHEDULED", "REFUNDING", "COMPENSATED_REFUND"] {
        let en = status("en", st).unwrap();
        for l in ["sq", "uk", "ru"] {
            assert_ne!(status(l, st).unwrap(), en, "{l} {st}");
        }
    }
    for l in ["sq", "uk", "ru"] {
        assert_ne!(new_order(l), new_order("en"));
        assert_ne!(assigned(l), assigned("en"));
        assert_ne!(ready_to_collect(l), ready_to_collect("en"));
    }
}

#[test]
fn pending_and_an_unknown_status_say_nothing_and_a_real_one_says_something() {
    assert_eq!(status("en", "PENDING"), None);
    assert_eq!(status("en", "WHATEVER"), None);
    assert_eq!(status("uk", "READY"), Some("Готове"));
}

#[test]
fn the_title_is_the_short_order_number_and_nothing_else() {
    assert_eq!(title("sq", "1a2b3c4d-5e6f-7a8b"), "Porosia #1a2b3c4d");
    assert_eq!(title("xx", "abc"), "Order #abc");
}

#[test]
fn speaks_is_the_set_of_statuses_with_words() {
    assert!(speaks("READY") && speaks("DELIVERED"));
    assert!(!speaks("PENDING") && !speaks("nonsense"));
}
