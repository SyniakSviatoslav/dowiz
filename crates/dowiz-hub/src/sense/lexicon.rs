//! A DRAFT from the words the venue already wrote: the dish's name, description and ingredients.
//!
//! Deterministic, offline, four languages (sq/en/uk/ru). It is the "Suggest" button's first
//! answer and the starter defaults of the common sushi menu; the owner reviews it and saves it
//! with the ordinary save -- nothing here is ever written by itself. Every value it gives says
//! which word gave it (`why`), so the owner can see a guess for what it is.
//!
//! THE STARTER DEFAULTS are typical, publicly described profiles of the standard dishes
//! (a Philadelphia roll is salmon + cream cheese: salty/umami, creamy, marine), not this venue's
//! recipe. They are marked `starter` and editable; a venue's own words win where both speak.

use super::{Dim, Sense};

/// (needles, [(key, value)]). A needle is matched as a lower-case substring.
type Rule = (&'static [&'static str], &'static [(&'static str, i64)]);

/// The common sushi menu, by the dish's NAME (the starter defaults).
const STARTER: &[Rule] = &[
    (&["philadelphia", "филадельф", "філадельф"], &[("t:salty", 2), ("t:umami", 3), ("t:sweet", 1), ("x:creamy", 3), ("x:soft", 2), ("a:marine", 2), ("a:buttery", 1)]),
    (&["california", "калифорн", "каліфорн"], &[("t:sweet", 2), ("t:umami", 2), ("t:salty", 2), ("x:creamy", 2), ("x:soft", 2), ("a:marine", 2)]),
    (&["tempura", "темпур"], &[("x:crispy", 3), ("x:crunchy", 2), ("a:toasty", 2), ("t:salty", 2)]),
    (&["dragon", "дракон"], &[("t:sweet", 2), ("t:umami", 3), ("x:creamy", 2), ("x:tender", 2), ("a:smoky", 1)]),
    (&["unagi", "eel", "ngjala", "угорь", "вугор"], &[("t:sweet", 3), ("t:umami", 3), ("x:tender", 3), ("a:smoky", 2), ("a:toasty", 1)]),
    (&["sashimi", "сашими", "сашимі"], &[("t:umami", 3), ("t:salty", 1), ("x:silky", 3), ("x:tender", 2), ("a:marine", 3)]),
    (&["nigiri", "нигири", "нігірі"], &[("t:umami", 3), ("t:sour", 1), ("x:soft", 2), ("x:silky", 2), ("a:marine", 2)]),
    (&["maki", "маки", "макі", "roll", "ролл", "рол"], &[("t:umami", 2), ("t:salty", 2), ("t:sour", 1), ("x:soft", 2), ("a:marine", 1)]),
    (&["miso", "мисо", "місо"], &[("t:salty", 3), ("t:umami", 4), ("x:silky", 2), ("a:fermented", 3), ("a:marine", 1)]),
    (&["ramen", "рамен"], &[("t:salty", 3), ("t:umami", 4), ("x:chewy", 3), ("x:tender", 2), ("a:toasty", 1)]),
    (&["gyoza", "гедза", "гьоза", "гёдза"], &[("t:salty", 2), ("t:umami", 3), ("x:crispy", 2), ("x:juicy", 2), ("x:chewy", 1), ("a:toasty", 2)]),
    (&["edamame", "эдамаме", "едамаме"], &[("t:salty", 2), ("x:chewy", 1), ("x:tender", 2), ("a:earthy", 1)]),
    (&["teriyaki", "терияки", "теріякі"], &[("t:sweet", 3), ("t:salty", 3), ("t:umami", 3), ("x:tender", 2), ("a:toasty", 2)]),
    (&["wakame", "chuka", "seaweed salad", "вакаме", "чука"], &[("t:sour", 2), ("t:salty", 2), ("t:umami", 2), ("x:crunchy", 2), ("a:marine", 3)]),
    (&["mochi", "моти", "мочі"], &[("t:sweet", 4), ("x:chewy", 3), ("x:soft", 2)]),
    (&["poke", "поке"], &[("t:salty", 2), ("t:umami", 3), ("t:sour", 1), ("x:tender", 2), ("x:juicy", 2), ("a:marine", 2)]),
];

/// The venue's own words, in any of the four languages: description and ingredients.
const WORDS: &[Rule] = &[
    // taste
    (&["spicy", "chili", "chilli", "sriracha", "jalape", "djeg", "pikant", "остр", "гостр", "чили", "чилі", "kimchi"], &[("t:spicy", 3)]),
    (&["wasabi", "васаби", "васабі"], &[("t:spicy", 2), ("a:herbal", 1)]),
    (&["sweet", "honey", "mjal", "ëmbël", "embel", "сладк", "солодк", "мед", "sugar", "sheqer", "caramel", "karamel"], &[("t:sweet", 3)]),
    (&["salty", "kripur", "солен", "солон", "soy", "soje", "соев", "соєв"], &[("t:salty", 3)]),
    (&["sour", "vinegar", "uthull", "thartë", "кисл", "уксус", "оцет", "pickled", "turshi", "маринов", "мариноване"], &[("t:sour", 3)]),
    (&["bitter", "hidhur", "горьк", "гірк", "matcha", "матча", "grapefruit", "грейпфрут"], &[("t:bitter", 2)]),
    (&["umami", "parmesan", "parmixhan", "пармезан", "mushroom", "kërpudh", "kerpudh", "гриб", "shiitake", "шиитаке", "шиітаке", "dashi", "даши", "даші"], &[("t:umami", 3)]),
    (&["lemon", "lime", "limon", "лимон", "лайм", "yuzu", "юдзу", "ponzu", "понзу", "orange", "portokall", "апельсин"], &[("a:citrus", 3), ("t:sour", 2)]),
    // texture
    (&["crispy", "crisp", "krokant", "хрустящ", "хрустк", "fried", "skuqur", "жарен", "смажен", "panko", "панко"], &[("x:crispy", 3)]),
    (&["crunchy", "kërcit", "кранч", "cucumber", "kastravec", "огур", "огір", "sesame", "susam", "кунжут"], &[("x:crunchy", 2)]),
    (&["cream cheese", "philadelphia cheese", "krem djath", "сливочн", "вершков", "creamy", "kremoz", "кремов", "avocado", "avokado", "авокадо", "mayo", "majonez", "майонез"], &[("x:creamy", 3)]),
    (&["tender", "butë", "нежн", "ніжн"], &[("x:tender", 3)]),
    (&["chewy", "noodle", "petë", "лапш", "локшин", "udon", "удон"], &[("x:chewy", 2)]),
    (&["juicy", "lëngsh", "сочн", "соков"], &[("x:juicy", 3)]),
    (&["silky", "mëndafsh", "шелков", "шовков", "tofu", "тофу"], &[("x:silky", 2)]),
    (&["flaky", "fletë", "слоён", "слоен", "листков"], &[("x:flaky", 3)]),
    (&["soft", "мягк", "мʼяк", "м'як"], &[("x:soft", 2)]),
    // aroma
    (&["smoked", "smoky", "tymosur", "тымос", "копчен", "копчён", "torched", "aburi", "абури", "grill", "zgarë", "гриль"], &[("a:smoky", 3)]),
    (&["herb", "barishte", "трав", "зелен", "basil", "borzilok", "базилик", "базилік", "mint", "nenexhik", "мят", "мʼят", "coriander", "cilantro", "kinz", "кинз", "кінз", "shiso", "шисо"], &[("a:herbal", 2)]),
    (&["flower", "floral", "lule", "цвет", "квіт", "jasmine", "jasemin", "жасмин", "sakura", "сакур"], &[("a:floral", 2)]),
    (&["nuts", "nutty", "walnut", "hazelnut", "arra", "lajthi", "орех", "горіх", "peanut", "kikirik", "арахис", "арахіс", "almond", "bajame", "миндал", "мигдал"], &[("a:nutty", 3)]),
    (&["toast", "roasted", "pjekur", "жарен", "печен", "bread", "bukë", "хлеб", "хліб", "furikake", "фурикаке"], &[("a:toasty", 2)]),
    (&["salmon", "salmoni", "лосос", "сёмг", "семг", "tuna", "tonn", "тунец", "тунц", "shrimp", "karkalec", "креветк", "crab", "gaforre", "краб", "nori", "нори", "норі", "caviar", "havjar", "икр", "ікр", "tobiko", "тобико", "тобіко", "fish", "peshk", "рыб", "риб"], &[("a:marine", 2), ("t:umami", 2)]),
    (&["ferment", "fermentuar", "фермент", "kimchi", "кимчи", "кімчі", "natto", "натто"], &[("a:fermented", 3)]),
    (&["mango", "манго", "strawberr", "luleshtrydh", "клубник", "полуниц", "fruit", "frut", "фрукт", "passion", "маракуй", "pineapple", "ananas", "ананас"], &[("a:fruity", 3), ("t:sweet", 2)]),
    (&["truffle", "tartuf", "трюфел", "beet", "panxhar", "свекл", "буряк"], &[("a:earthy", 3)]),
    (&["butter", "gjalp", "сливочное масло", "вершкове масло", "brioche", "бриошь", "бріош"], &[("a:buttery", 3)]),
    (&["cinnamon", "kanell", "кориц", "ginger", "xhenxhefil", "имбир", "імбир", "curry", "kerri", "карри", "каррі", "clove", "karafil", "гвоздик", "anise", "анис", "аніс"], &[("a:spice-warm", 2)]),
];

/// The draft and, per key, the word that gave it. Taste and tags keep their highest value.
pub fn suggest(name: &str, description: &str, ingredients: &[String]) -> (Sense, Vec<(String, String)>) {
    let name = name.to_lowercase();
    let words = format!("{} {}", description, ingredients.join(" ")).to_lowercase();
    let mut out = Sense::default();
    let mut why: Vec<(String, String)> = Vec::new();
    for (rules, hay, from) in [(STARTER, &name, "starter"), (WORDS, &name, "name"), (WORDS, &words, "words")] {
        for (needles, gives) in rules {
            let Some(hit) = needles.iter().find(|n| hay.contains(*n)) else { continue };
            for (key, val) in gives.iter() {
                let Some((p, id)) = key.split_once(':') else { continue };
                let Some(d) = Dim::ALL.iter().copied().find(|d| d.prefix() == p) else { continue };
                let slot = out.map_mut(d).entry(id.to_string()).or_insert(0);
                if *val > *slot {
                    *slot = (*val).clamp(d.min(), d.max());
                    why.retain(|(k, _)| k != key);
                    why.push((key.to_string(), format!("{from}: {hit}")));
                }
            }
        }
    }
    (out, why)
}
