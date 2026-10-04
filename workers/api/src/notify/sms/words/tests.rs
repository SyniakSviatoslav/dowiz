use super::*;

const ORDER: &str = "1a2b3c4d-5e6f-7a8b-9c0d-e1f2a3b4c5d6";

#[test]
fn the_text_carries_the_order_number_venue_state_and_stop_and_nothing_else() {
    let t = text("sq", "Sushi Durrës", ORDER, "CONFIRMED").unwrap();
    assert_eq!(t, "Sushi Durres: Porosia #1a2b3c4d - U konfirmua. STOP: thuajini lokalit");
    assert!(!t.contains("5e6f"), "only the first eight characters of the id");
    for l in ["sq", "en", "uk", "ru"] {
        for st in ["CONFIRMED", "READY", "IN_DELIVERY", "REJECTED", "CANCELLED"] {
            // The longest venue name a text carries (30 characters).
            let t = text(l, "Restorant Peshku i Detit Durrës Plazh", ORDER, st).unwrap();
            // sq/en: ONE GSM-7 segment. uk/ru are UCS-2: at most two.
            let max = if l == "uk" || l == "ru" { 2 } else { 1 };
            assert!(segments(&t) <= max, "{l}/{st} costs {} SMS: {t}", segments(&t));
            if max == 1 {
                assert!(t.is_ascii(), "{l}/{st} left the GSM alphabet: {t}");
            }
        }
    }
}

#[test]
fn only_the_statuses_worth_a_text_are_texted() {
    assert!(texts("CONFIRMED", "delivery"));
    assert!(texts("IN_DELIVERY", "delivery"));
    assert!(texts("CANCELLED", "pickup"));
    assert!(texts("READY", "pickup"));
    assert!(!texts("READY", "delivery"), "a delivery hears 'on the way' next");
    assert!(!texts("PENDING", "delivery"), "the placement is on the customer's screen");
    assert!(!texts("PREPARING", "delivery"));
    assert!(!texts("DELIVERED", "delivery"));
    assert_eq!(text("en", "V", ORDER, "PENDING"), None);
}

#[test]
fn the_venue_name_is_one_short_line() {
    assert_eq!(venue_short("  Sushi\n  Durrës  "), "Sushi Durrës");
    assert_eq!(venue_short(&"x".repeat(50)).chars().count(), 30);
}

#[test]
fn segments_are_counted_as_a_phone_counts_them() {
    assert_eq!(segments(&"a".repeat(160)), 1);
    assert_eq!(segments(&"a".repeat(161)), 2);
    assert_eq!(segments(&"я".repeat(70)), 1);
    assert_eq!(segments(&"я".repeat(71)), 2);
    assert_eq!(gsm("Durrës Çlirimi ç"), "Durres Çlirimi c");
}
