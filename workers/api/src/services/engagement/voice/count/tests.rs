//! P14: the count grammar's table, 21 utterances in each of the four
//! languages, and every question it asks instead of guessing.

use super::*;

fn c(item: &str, qty: i64, unit: Option<&'static str>) -> Option<Heard> {
    Some(Heard::Count { item: item.into(), qty, unit })
}
const G: Option<&str> = Some("g");
const ML: Option<&str> = Some("ml");
const PCS: Option<&str> = Some("unit");

fn table(rows: &[(&str, &str, i64, Option<&'static str>)]) {
    for (said_, item, qty, unit) in rows {
        assert_eq!(said(said_), c(item, *qty, *unit), "{said_:?}");
    }
}

#[test]
fn english_counts() {
    table(&[
        ("count salmon two kilo three hundred", "salmon", 2300, G),
        ("count salmon 2,3 kg", "salmon", 2300, G),
        ("count salmon 2.3 kg", "salmon", 2300, G),
        ("count salmon, 2,3 kg", "salmon", 2300, G),
        ("count rice 12 kg", "rice", 12000, G),
        ("count rice 1 kg 200 g", "rice", 1200, G),
        ("count soy sauce 1,5 l", "soy sauce", 1500, ML),
        ("count soy sauce one and a half litres", "soy sauce", 1500, ML),
        ("count nori 30 pcs", "nori", 30, PCS),
        ("count nori thirty pieces", "nori", 30, PCS),
        ("count tuna half a kilo", "tuna", 500, G),
        ("count tuna 750 g", "tuna", 750, G),
        ("count ginger twenty five grams", "ginger", 25, G),
        ("counted wasabi 0,25 kg", "wasabi", 250, G),
        ("count mayo 2 litres", "mayo", 2000, ML),
        ("count avocado zero", "avocado", 0, None),
        ("count rice one thousand two hundred grams", "rice", 1200, G),
        ("count salmon 2 kg 50 g", "salmon", 2050, G),
        ("count sesame kilo two hundred", "sesame", 1200, G),
        ("count cream 300 ml", "cream", 300, ML),
        ("count 14 pcs lemons", "lemons", 14, PCS),
    ]);
}

#[test]
fn ukrainian_counts() {
    table(&[
        ("рахую лосось два кіло триста", "лосось", 2300, G),
        ("рахую лосось 2,3 кг", "лосось", 2300, G),
        ("порахував рис 12 кг", "рис", 12000, G),
        ("порахуй рис 1 кг 200 г", "рис", 1200, G),
        ("рахую соєвий соус півтора літра", "соєвий соус", 1500, ML),
        ("рахую соєвий соус 1,5 л", "соєвий соус", 1500, ML),
        ("рахую норі тридцять штук", "норі", 30, PCS),
        ("рахую норі 30 шт", "норі", 30, PCS),
        ("рахую тунець пів кіло", "тунець", 500, G),
        ("рахую тунець 750 г", "тунець", 750, G),
        ("рахую імбир двадцять п'ять грамів", "імбир", 25, G),
        ("перерахунок васабі 0,25 кг", "васабі", 250, G),
        ("рахую майонез два літри", "майонез", 2000, ML),
        ("рахую авокадо нуль", "авокадо", 0, None),
        ("рахую рис тисяча двісті грамів", "рис", 1200, G),
        ("рахую лосось 2 кг 50 г", "лосось", 2050, G),
        ("рахую кунжут кіло двісті", "кунжут", 1200, G),
        ("рахую вершки 300 мл", "вершки", 300, ML),
        ("рахую лимони 14 штук", "лимони", 14, PCS),
        ("рахую лосось три кілограми", "лосось", 3000, G),
        ("рахую рис пять кілограмів сімсот", "рис", 5700, G),
    ]);
}

#[test]
fn russian_counts() {
    table(&[
        ("считаю лосось два кило триста", "лосось", 2300, G),
        ("считаю лосось 2,3 кг", "лосось", 2300, G),
        ("посчитал рис 12 кг", "рис", 12000, G),
        ("посчитай рис 1 кг 200 г", "рис", 1200, G),
        ("считаю соевый соус полтора литра", "соевый соус", 1500, ML),
        ("считаю соевый соус 1,5 л", "соевый соус", 1500, ML),
        ("считаю нори тридцать штук", "нори", 30, PCS),
        ("считаю нори 30 шт", "нори", 30, PCS),
        ("считаю тунец пол кило", "тунец", 500, G),
        ("считаю тунец 750 г", "тунец", 750, G),
        ("считаю имбирь двадцать пять граммов", "имбирь", 25, G),
        ("пересчёт васаби 0,25 кг", "васаби", 250, G),
        ("считаю майонез два литра", "майонез", 2000, ML),
        ("считаю авокадо ноль", "авокадо", 0, None),
        ("считаю рис тысяча двести граммов", "рис", 1200, G),
        ("считаю лосось 2 кг 50 г", "лосось", 2050, G),
        ("считаю кунжут кило двести", "кунжут", 1200, G),
        ("считаю сливки 300 мл", "сливки", 300, ML),
        ("считаю лимоны 14 штук", "лимоны", 14, PCS),
        ("считаю лосось три килограмма", "лосось", 3000, G),
        ("инвентаризация рис пять килограммов семьсот", "рис", 5700, G),
    ]);
}

#[test]
fn albanian_counts() {
    table(&[
        ("numëro losos dy kile e treqind", "losos", 2300, G),
        ("numero losos dy kile e treqind", "losos", 2300, G),
        ("numëro losos 2,3 kg", "losos", 2300, G),
        ("numërova oriz 12 kg", "oriz", 12000, G),
        ("numëro oriz 1 kg 200 g", "oriz", 1200, G),
        ("numëro salcë soje një e gjysmë litra", "salcë soje", 1500, ML),
        ("numëro salcë soje 1,5 l", "salcë soje", 1500, ML),
        ("numëro nori tridhjetë copë", "nori", 30, PCS),
        ("numëro nori 30 copë", "nori", 30, PCS),
        ("numëro ton gjysmë kile", "ton", 500, G),
        ("numëro ton 750 g", "ton", 750, G),
        ("numëro xhenxhefil njëzet e pesë gramë", "xhenxhefil", 25, G),
        ("numërim wasabi 0,25 kg", "wasabi", 250, G),
        ("numëro majonezë dy litra", "majonezë", 2000, ML),
        ("numëro avokado zero", "avokado", 0, None),
        ("numëro oriz një mijë e dyqind gramë", "oriz", 1200, G),
        ("numëro losos 2 kg 50 g", "losos", 2050, G),
        ("numëro susam kile e dyqind", "susam", 1200, G),
        ("numëro krem 300 ml", "krem", 300, ML),
        ("numëro limonë 14 copë", "limonë", 14, PCS),
        ("numëro losos tre kilogramë", "losos", 3000, G),
    ]);
}

#[test]
fn a_count_it_cannot_read_is_a_question_never_a_guess() {
    let q = |s: &str| said(s);
    let ask = |k: &'static str| Some(Heard::Unclear(k));
    // No unit: 2 kg and 2 g are both somebody's shelf.
    assert_eq!(q("count salmon 2"), ask("count_unit"));
    assert_eq!(q("рахую лосось два"), ask("count_unit"));
    assert_eq!(q("считаю лосось 2"), ask("count_unit"));
    assert_eq!(q("numëro losos dy"), ask("count_unit"));
    assert_eq!(q("count 3 salmon"), ask("count_unit"));
    // "2,300" is 2.3 in Albanian and 2300 in English.
    assert_eq!(q("count salmon 2,300 kg"), ask("decimal_unclear"));
    assert_eq!(q("count salmon 1.250 kg"), ask("decimal_unclear"));
    // Numbers that do not compose.
    assert_eq!(q("count salmon two three kilo"), ask("two_numbers"));
    assert_eq!(q("count salmon 2 3 kg"), ask("two_numbers"));
    assert_eq!(q("count salmon twelve five kilo"), ask("two_numbers"));
    assert_eq!(q("count salmon 2 kg 3 l"), ask("two_numbers"));
    assert_eq!(q("count salmon 2 g 300"), ask("two_numbers"));
    assert_eq!(q("count salmon 2 kg 1500"), ask("two_numbers"), "more grams than a kilo is a second number");
    // What, and how much.
    assert_eq!(q("count 2 kg"), ask("which_supply"));
    assert_eq!(q("count salmon"), ask("how_much"));
    assert_eq!(q("count salmon g"), ask("how_much"));
    assert_eq!(q("count salmon 5000 kg"), ask("how_much"), "five tonnes is a typo");
    assert_eq!(q("count salmon 2,5 g"), ask("how_much"), "half a gram is not a base unit");
    // Not a count at all: the other grammars read it.
    assert_eq!(q("received 4 kg salmon"), None);
    assert_eq!(q("table 5 paid cash"), None);
}

mod proposal {
    use super::super::super::decide::Out;
    use super::super::super::dish::Dish;
    use super::super::super::kitchen::{decide, stock_said, Said, Supply};
    use super::super::super::scope;
    use dowiz_hub::caps::{Cap, Caps, Preset};
    use serde_json::json;

    fn sup(id: &str, names: &[&str], unit: &str) -> Supply {
        Supply { dish: Dish { id: id.into(), names: names.iter().map(|n| n.to_string()).collect(), available: true }, unit: unit.into() }
    }
    fn shelf() -> Vec<Supply> {
        vec![
            sup("salmon", &["Salmon", "Лосось", "Losos"], "g"),
            sup("soy", &["Soy sauce", "Соєвий соус", "Salcë soje"], "ml"),
            sup("nori", &["Nori", "Норі"], "unit"),
            sup("rice-sushi", &["Sushi rice", "Рис для суші"], "g"),
            sup("rice-jasmine", &["Jasmine rice", "Рис жасмин"], "g"),
        ]
    }
    fn hear(s: &str, lang: &str, caps: &Caps) -> Out {
        decide(&stock_said(s).expect("a kitchen utterance"), caps, lang, &shelf())
    }

    #[test]
    fn a_count_is_a_proposal_in_the_speakers_language() {
        let k = Preset::Kitchen.caps();
        for (said, lang, readback) in [
            ("count salmon 2,3 kg", "en", "counted: 2300 g Salmon"),
            ("рахую лосось два кіло триста", "uk", "пораховано: 2300 g Salmon"),
            ("считаю лосось два кило триста", "ru", "посчитано: 2300 g Salmon"),
            ("numëro losos dy kile e treqind", "sq", "numëruar: 2300 g Salmon"),
        ] {
            let Out::Propose { verb, arg, readback: r, extra } = hear(said, lang, &k) else { panic!("{said}") };
            assert_eq!((verb, arg.as_str(), r.as_str()), ("count", "salmon|2300", readback), "{said}");
            assert_eq!(extra, json!({ "itemId": "salmon", "observed": 2300 }));
        }
        assert!(matches!(hear("numëro salcë soje 1,5 l", "sq", &k), Out::Propose { arg, .. } if arg == "soy|1500"));
        assert!(matches!(hear("count nori zero", "en", &k), Out::Propose { arg, .. } if arg == "nori|0"));
        assert_eq!(scope::decode("count", "salmon|2300"), Some(json!({ "itemId": "salmon", "observed": 2300 })));
        assert!(scope::is_ours("count"));
    }

    /// Two supplies that fit is a question naming both; a unit the supply is
    /// not counted in, a supply nobody stocks, and a role without the shelf are refused.
    #[test]
    fn ambiguity_becomes_a_question_never_a_guess() {
        let k = Preset::Kitchen.caps();
        let Out::Refuse(q) = hear("count rice 2 kg", "en", &k) else { panic!("rice is two supplies") };
        assert!(q.starts_with("Several ingredients fit:") && q.contains("Sushi rice") && q.contains("Jasmine rice"), "{q}");
        let Out::Refuse(q) = hear("рахую рис 2 кг", "uk", &k) else { panic!() };
        assert!(q.starts_with("Підходить кілька інгредієнтів:"), "{q}");
        assert!(matches!(hear("count sushi rice 2 kg", "en", &k), Out::Propose { arg, .. } if arg == "rice-sushi|2000"), "more words, one supply");
        assert!(matches!(hear("count salmon 2", "en", &k), Out::Refuse(s) if s.starts_with("In what unit?")));
        assert!(matches!(hear("numëro losos dy", "sq", &k), Out::Refuse(s) if s.starts_with("Në çfarë njësie?")));
        assert!(matches!(hear("count salmon 2,300 kg", "en", &k), Out::Refuse(s) if s.starts_with("Say the number again")));
        assert!(matches!(hear("count nori 2 kg", "en", &k), Out::Refuse(s) if s.contains("another unit")));
        assert!(matches!(hear("count tuna 2 kg", "en", &k), Out::Refuse(s) if s.contains("«tuna»")));
        assert!(matches!(hear("count salmon 2 kg", "en", &Preset::Waiter.caps()), Out::Refuse(s) if s == "Your role does not do that"));
        assert!(matches!(hear("count salmon 2 kg", "en", &Caps::of(&[Cap::OpenTill])), Out::Refuse(_)), "the till does not count");
        assert!(matches!(hear("count salmon 2 kg", "en", &Caps::of(&[Cap::Stock])), Out::Propose { .. }));
        assert_eq!(stock_said("count received salmon 2 kg"), Some(Said::Unclear("more_than_one")));
    }
}
