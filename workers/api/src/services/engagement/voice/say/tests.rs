use super::*;

/// Every refusal has a different line in every language, none empty.
#[test]
fn every_key_is_said_in_every_language() {
    for k in keys() {
        let en = line(k, "en");
        assert_ne!(en, k, "{k} has no line");
        for l in dowiz_hub::lang::LANGS {
            let said = line(k, l);
            assert!(!said.is_empty(), "{l}.{k}");
            if l != "en" {
                assert_ne!(said, en, "{l}.{k} is not translated");
            }
        }
        assert_ne!(line(k, "ru"), line(k, "uk"), "ru.{k} is the Ukrainian line");
    }
    // A key nobody wrote reads as itself -- visible, not silently English.
    assert_eq!(line("no_such_key", "uk"), "no_such_key");
    // An unknown language is English.
    assert_eq!(line("which_table", "de"), "Which table?");
    assert_eq!(line("which_table", "uk-UA"), "Який стіл?");
    assert_eq!(line("which_table", "ru-RU"), "Какой стол?");
}

#[test]
fn readbacks_speak_the_speakers_language() {
    assert_eq!(add("en", 2, "Margherita", "5"), "add 2 × Margherita to table 5");
    assert_eq!(add("uk", 2, "Маргарита", "5"), "додати 2 × Маргарита на стіл 5");
    assert_eq!(add("sq", 2, "Margarita", "5"), "shto 2 × Margarita në tavolinën 5");
    let items = vec![("Cola".to_string(), 2), ("Tiramisu".to_string(), 1)];
    assert_eq!(send("en", "5", &items), "send to table 5: 2 × Cola, 1 × Tiramisu");
    assert!(send("uk", "5", &items).starts_with("відправити на стіл 5"));
    assert!(send("sq", "5", &items).starts_with("dërgo në tavolinën 5"));
    assert_eq!(add("ru", 2, "Маргарита", "5"), "добавить 2 × Маргарита на стол 5");
    assert!(send("ru", "5", &items).starts_with("отправить на стол 5"));
    assert_eq!(paid("ru", "5", true), "стол 5: оплата наличными");
    assert_eq!(dish_sale("ru", "Cola", false), "снять с продажи: Cola");
    assert_eq!(venue("ru", "busy"), "заведение: занято");
    assert_eq!(receive("ru", 2000, "g", "Лосось"), "приход на склад: 2000 g Лосось");
    assert_eq!(waste("ru", 300, "g", "Лосось", "spoiled"), "списать: 300 g Лосось (испортилось)");
    for l in dowiz_hub::lang::LANGS {
        assert_ne!(paid(l, "5", true), paid(l, "5", false));
        assert_ne!(dish_sale(l, "Cola", true), dish_sale(l, "Cola", false));
        assert_ne!(venue(l, "open"), venue(l, "closed"));
        assert_ne!(venue(l, "busy"), venue(l, "closed"));
    }
    assert_eq!(paid("en", "5", true), "table 5: paid in cash");
    assert_eq!(paid("uk", "5", false), "стіл 5: оплата карткою");
    assert_eq!(dish_sale("sq", "Cola", false), "hiq nga shitja: Cola");
    assert_eq!(venue("uk", "busy"), "заклад: зайнято");
    assert_eq!(venue("sq", "open"), "lokali: i hapur");
    assert_eq!(venue("en", "closed"), "the venue: closed");
}
