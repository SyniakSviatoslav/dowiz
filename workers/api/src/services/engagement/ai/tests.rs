//! The pure halves of the owner's AI: the plan, the budget, the closed list, the
//! lexicon, the answers read from fold answers, the templates and their guard.

use super::answer::{self, Fold};
use super::budget::{self, Meter};
use super::lexicon;
use super::provider::{self, Cfg, Key, Mode, Off, Route};
use super::query::{self, Kind, Query};
use super::words;
use super::explain;
use serde_json::{json, Value};

fn cfg(enabled: bool, mode: Mode, endpoint: &str, key: Option<&str>) -> Cfg {
    Cfg { enabled, mode, endpoint: endpoint.into(), model: "m".into(), key: key.and_then(Key::new) }
}

fn names(p: &provider::Plan) -> Vec<&'static str> {
    p.routes.iter().map(Route::name).collect()
}

// ── the plan ───────────────────────────────────────────────────────────────

#[test]
fn the_plan_tries_the_owners_key_then_workers_ai_then_says_why_off() {
    let own = cfg(true, Mode::Auto, "https://openrouter.ai/api/v1", Some("sk-or-1"));
    assert_eq!(names(&provider::plan(&own, true, true)), ["own", "workers-ai"]);
    // No key of the owner's: Workers AI alone, and the console is told why.
    let p = provider::plan(&cfg(true, Mode::Auto, "", None), true, true);
    assert_eq!((names(&p), p.skipped.clone()), (vec!["workers-ai"], vec![("own", Off::NeedsKey)]));
    // Nothing reachable: no route, and every reason.
    let p = provider::plan(&cfg(true, Mode::Auto, "", None), false, true);
    assert!(p.routes.is_empty());
    assert_eq!(p.skipped, vec![("own", Off::NeedsKey), ("workers-ai", Off::NoBinding)]);
    // A spent share refuses Workers AI and leaves the owner's key.
    let p = provider::plan(&own, true, false);
    assert_eq!((names(&p), p.skipped.clone()), (vec!["own"], vec![("workers-ai", Off::Budget)]));
}

/// W-TASTE2 (operator 2026-10-06): a venue that never touched the switch has AI ON; the owner's
/// stored "0" is OFF and sends nothing; with no endpoint and no binding the plan is empty and the
/// callers fall back (lexicon draft, deterministic answer) instead of failing.
#[test]
fn ai_is_on_by_default_an_owners_off_stays_off_and_no_route_is_not_an_error() {
    let mut s = dowiz_hub::settings::Settings::create().unwrap();
    let on = Cfg::of(&s);
    assert!(on.enabled, "unset = on");
    assert_eq!(names(&provider::plan(&on, true, true)), vec!["workers-ai"], "on by default: Workers AI through the binding");
    let bare = provider::plan(&on, false, false);
    assert!(bare.routes.is_empty(), "no binding, share spent: nothing to try");
    assert!(!bare.skipped.iter().any(|(_, o)| *o == Off::Disabled), "and it is NOT reported as switched off: {bare:?}");
    assert!(crate::integrations::ai_usable(&s, true) && !crate::integrations::ai_usable(&s, false));
    s.set("ai.enabled", "0");
    let off = Cfg::of(&s);
    assert!(!off.enabled, "the owner's off");
    assert_eq!(provider::plan(&off, true, true).skipped, vec![("all", Off::Disabled)]);
    assert!(!crate::integrations::ai_usable(&s, true), "off is never usable, binding or not");
}

#[test]
fn switched_off_tries_nothing_at_all() {
    let p = provider::plan(&cfg(false, Mode::Auto, "https://x.example/v1", Some("sk-1")), true, true);
    assert!(p.routes.is_empty(), "{p:?}");
    assert_eq!(p.skipped, vec![("all", Off::Disabled)]);
}

#[test]
fn a_chosen_route_is_the_only_one_and_http_is_refused() {
    let p = provider::plan(&cfg(true, Mode::Own, "https://x.example/v1", None), true, true);
    assert_eq!((names(&p), p.skipped.clone()), (vec!["own"], vec![("workers-ai", Off::NotChosen)]));
    let p = provider::plan(&cfg(true, Mode::Workers, "https://x.example/v1", Some("k-123")), true, true);
    assert_eq!((names(&p), p.skipped.clone()), (vec!["workers-ai"], vec![("own", Off::NotChosen)]));
    let p = provider::plan(&cfg(true, Mode::Own, "http://x.example/v1", None), true, true);
    assert_eq!(p.skipped[0], ("own", Off::NotHttps));
    assert_eq!(provider::validate("ai.provider", "openai"), Err("ai.provider is auto, own or workers, not \"openai\"".into()));
    assert_eq!(provider::validate("ai.provider", "workers"), Ok(()));
    assert_eq!(provider::validate("ai.model", "anything"), Ok(()));
}

#[test]
fn a_key_never_prints_and_is_scrubbed_from_what_comes_back() {
    let k = Key::new("  sk-or-v1-SECRET  ").unwrap();
    assert_eq!(k.expose(), "sk-or-v1-SECRET");
    let shown = format!("{k:?} {:?}", cfg(true, Mode::Auto, "https://x/v1", Some("sk-or-v1-SECRET")));
    assert!(!shown.contains("SECRET"), "{shown}");
    assert_eq!(k.scrub("401 bad key sk-or-v1-SECRET!"), "401 bad key \u{2022}\u{2022}\u{2022}\u{2022}!");
    assert!(Key::new("   ").is_none());
}

#[test]
fn a_reasoning_models_thinking_is_dropped() {
    assert_eq!(provider::without_thinking("<think>x=1</think>\n{\"query\":\"orders\"}"), "{\"query\":\"orders\"}");
    assert_eq!(provider::without_thinking("plain"), "plain");
    assert_eq!(provider::without_thinking("{\"query\":\"orders\"}<think>cut off mid-"), "{\"query\":\"orders\"}");
    assert_eq!(provider::without_thinking("<think>never closed"), "");
}

// ── the budget ─────────────────────────────────────────────────────────────

#[test]
fn neurons_round_up_and_the_worst_case_admits_or_refuses() {
    assert_eq!(budget::neurons(0, 0), 0);
    assert_eq!(budget::neurons(1, 0), 1, "a fraction of a neuron is spent");
    // 1M in + 1M out = the two published rates.
    assert_eq!(budget::neurons(1_000_000, 1_000_000), budget::IN_PER_M + budget::OUT_PER_M);
    assert_eq!(budget::worst_case("abcdef", 100), budget::neurons(2, 100));
    let m = Meter { day: 5, used: 290, cap: 300 };
    assert!(m.admits(10));
    assert!(!m.admits(11), "one neuron over the share is refused");
    assert_eq!(m.left(), 10);
}

#[test]
fn the_share_renews_each_utc_day_and_a_clock_turned_back_does_not_refill_it() {
    let day = budget::utc_day(1_700_000_000_000);
    assert_eq!(budget::spent_today(Some(&format!("{day}:120")), day), 120);
    assert_eq!(budget::spent_today(Some(&format!("{}:120", day - 1)), day), 0, "yesterday is not today");
    assert_eq!(budget::spent_today(Some(&format!("{}:120", day + 1)), day), 120, "a future day keeps its count");
    assert_eq!(budget::spent_today(Some("garbage"), day), 0);
    assert_eq!(budget::record(Some(&format!("{day}:120")), day, 30), format!("{day}:150"));
    assert_eq!(budget::record(Some(&format!("{}:999", day - 1)), day, 30), format!("{day}:30"));
    assert_eq!(budget::cap_of(None), budget::DEFAULT_VENUE_DAILY);
    assert_eq!(budget::cap_of(Some("1200")), 1200);
    assert_eq!(budget::cap_of(Some("20000")), budget::DEFAULT_VENUE_DAILY, "never more than the account has");
    assert_eq!(budget::cap_of(Some("0")), budget::DEFAULT_VENUE_DAILY);
}

// ── the closed list ────────────────────────────────────────────────────────

fn menu() -> Vec<query::Dish> {
    vec![("d1".into(), "Futomaki".into()), ("d2".into(), "Miso soup".into())]
}

#[test]
fn the_picker_refuses_anything_outside_the_closed_list() {
    let m = menu();
    let bad = [
        (json!({"query": "popularity_contest"}), "not on the list"),
        (json!({"query": "revenue", "sql": "drop"}), "not a field"),
        (json!({"query": "revenue", "days": 14}), "is not 1, 7 or 30"),
        (json!({"query": "food_cost", "weekday": "mon"}), "has no weekday"),
        (json!({"query": "dish_sold", "dish": "Pizza"}), "not on the menu"),
        (json!({"query": "dish_sold"}), "needs a dish"),
        (json!({"query": "revenue", "dish": "Futomaki"}), "names no dish"),
        (json!({"query": "orders", "weekday": "funday"}), "not a weekday"),
        (json!({"query": "orders", "weekday": "mon", "days": 1}), "needs a window"),
        (json!(["revenue"]), "not an object"),
        (json!({}), "no query"),
    ];
    for (v, why) in bad {
        let e = query::validate(&v, &m).expect_err(&v.to_string());
        assert!(e.contains(why), "{v}: {e}");
    }
    // Its positive twins.
    assert_eq!(query::validate(&json!({"query": "revenue"}), &m), Ok(Query { kind: Kind::Revenue, days: 7, weekday: None, dish: None }));
    assert_eq!(
        query::validate(&json!({"query": "worst_dish", "weekday": "mon"}), &m),
        Ok(Query { kind: Kind::WorstDish, days: 30, weekday: Some(0), dish: None })
    );
    assert_eq!(
        query::validate(&json!({"query": "dish_sold", "dish": "futomaki", "days": 1, "weekday": null}), &m),
        Ok(Query { kind: Kind::DishSold, days: 1, weekday: None, dish: Some("d1".into()) })
    );
    assert_eq!(query::object_in("```json\n{\"query\": \"orders\"}\n```"), Some(json!({"query": "orders"})));
    assert_eq!(query::object_in("no object"), None);
    for (name, kind) in query::KINDS {
        assert_eq!(Kind::of(name), Some(kind));
        assert_eq!(kind.name(), name);
    }
}

// ── the lexicon ────────────────────────────────────────────────────────────

fn read(q: &str) -> Option<(Kind, i64, Option<usize>, Option<String>)> {
    lexicon::read(q, &menu()).map(|q| (q.kind, q.days, q.weekday, q.dish))
}

#[test]
fn the_lexicon_reads_the_four_languages() {
    assert_eq!(read("що продавалось найгірше в понеділок?"), Some((Kind::WorstDish, 30, Some(0), None)));
    assert_eq!(read("что продавалось хуже всего в среду?"), Some((Kind::WorstDish, 30, Some(2), None)));
    assert_eq!(read("Cila pjatë u shit më pak të hënën?"), Some((Kind::WorstDish, 30, Some(0), None)));
    assert_eq!(read("What sold best this week?"), Some((Kind::BestDish, 7, None, None)));
    assert_eq!(read("Sa të ardhura sot?"), Some((Kind::Revenue, 1, None, None)));
    assert_eq!(read("скільки замовлень за тиждень"), Some((Kind::Orders, 7, None, None)));
    assert_eq!(read("food cost last month"), Some((Kind::FoodCost, 30, None, None)));
    assert_eq!(read("яка страва має найгіршу маржу?"), Some((Kind::WorstMargin, 7, None, None)));
    assert_eq!(read("скільки продали Futomaki за місяць"), Some((Kind::DishSold, 30, None, Some("d1".into()))));
    assert_eq!(read("самый загруженный час за месяц"), Some((Kind::BusiestHour, 30, None, None)));
    assert_eq!(read("which hour is the quietest?"), Some((Kind::QuietestHour, 7, None, None)));
    assert_eq!(read("what was the worst day this month"), Some((Kind::WorstDay, 30, None, None)));
    assert_eq!(read("what is running low?"), Some((Kind::LowStock, 7, None, None)));
    assert_eq!(read("середнє замовлення"), Some((Kind::AverageOrder, 7, None, None)), "average is not Wednesday");
    assert_eq!(read("tell me a joke"), None);
}

#[test]
fn a_question_about_a_person_is_never_read() {
    for q in ["who is the best courier?", "який найкращий кур'єр", "лучший официант", "cili është korrieri më i mirë"] {
        assert_eq!(read(q), None, "{q}");
        assert!(lexicon::about_person(q), "{q}");
    }
    assert!(!lexicon::about_person("what sold best"));
}

#[test]
fn a_phone_or_an_email_never_reaches_the_picker() {
    assert_eq!(lexicon::withheld("orders from +355 69 123 4567 or +355691234567 a@b.al on 2023"), "orders from [withheld] or [withheld] [withheld] on 2023");
}

#[test]
fn the_language_of_a_question() {
    assert_eq!(lexicon::lang_of("що продавалось найгірше?"), "uk");
    assert_eq!(lexicon::lang_of("скільки замовлень"), "uk");
    assert_eq!(lexicon::lang_of("сколько заказов"), "ru");
    assert_eq!(lexicon::lang_of("Sa porosi sot?"), "sq");
    assert_eq!(lexicon::lang_of("Çfarë u shit?"), "sq");
    assert_eq!(lexicon::lang_of("How many orders?"), "en");
}

// ── the answers ────────────────────────────────────────────────────────────

/// Seven days from Monday 2023-11-06 to Sunday 2023-11-12 (kitchen) and the
/// analytics answer over the same days.
fn kitchen() -> Value {
    json!({
        "currency": "ALL",
        "days": ["2023-11-06", "2023-11-07", "2023-11-08", "2023-11-09", "2023-11-10", "2023-11-11", "2023-11-12"],
        "dishes": [
            {"id": "d1", "name": "Futomaki", "sold": 9, "revenue": 8100, "byDay": [0, 2, 2, 2, 1, 1, 1], "marginPortion": 500},
            {"id": "d2", "name": "Miso soup", "sold": 5, "revenue": 2500, "byDay": [4, 0, 0, 0, 0, 1, 0], "marginPortion": 300},
            {"id": "d3", "name": "Gyoza", "sold": 3, "revenue": 1800, "byDay": [1, 1, 1, 0, 0, 0, 0], "marginPortion": null}
        ],
        "totals": {"revenue": 12400, "cogs": 3100, "foodCostPm": 250, "wasteValue": 700},
        "waste": [{"reason": "spoiled", "rows": 1, "value": 500}, {"reason": "dropped", "rows": 1, "value": 200}],
        "ingredients": [{"id": "salmon", "name": "Salmon", "reorder": 2000}, {"id": "rice", "name": "Rice", "reorder": null}],
    })
}

fn analytics() -> Value {
    // Local midnights of Europe/Tirane (UTC+1 in November) as UTC instants.
    let at = |k: i64| 1_699_225_200_000 + k * 86_400_000;
    json!({
        "currency": "ALL", "orders": 10, "revenue": 13000, "rejected": 1, "averageOrder": 1444,
        "byChannel": {"storefront": 7, "telegram": 3},
        "byDay": (0..7).map(|k| json!({"at": at(k), "orders": k + 1, "revenue": 1000 * (k + 1)})).collect::<Vec<_>>(),
        "byHour": (0..24).map(|h| if h == 20 { 5 } else if h == 12 { 1 } else if h == 13 { 4 } else { 0 }).collect::<Vec<_>>(),
    })
}

/// Every number equals the sum of the cells it names: nothing is computed
/// that the fold did not give.
fn traced(fold: &Value, a: &answer::Answer) {
    for n in &a.numbers {
        if n.pointers.is_empty() {
            continue;
        }
        let sum: i64 = n.pointers.iter().map(|p| fold.pointer(p).and_then(Value::as_i64).unwrap_or_else(|| panic!("{p} in {fold}"))).sum();
        assert_eq!(sum, n.value, "{} {:?}", a.text, n.pointers);
    }
}

#[test]
fn the_worst_dish_on_mondays_is_read_from_the_monday_cells() {
    let k = kitchen();
    let q = Query { kind: Kind::WorstDish, days: 7, weekday: Some(0), dish: None };
    let a = answer::answer(&q, &k, "uk");
    // Monday: Futomaki 0, Miso 4, Gyoza 1.
    assert!(a.text.contains("Futomaki") && a.text.contains(" 0 "), "{}", a.text);
    assert_eq!(a.numbers[0].pointers, vec!["/dishes/0/byDay/0"]);
    assert_eq!(a.numbers[0].days, vec!["2023-11-06"]);
    assert_eq!(answer::weekday_of("2023-11-06"), Some(0), "2023-11-06 was a Monday");
    traced(&k, &a);
    let best = answer::answer(&Query { kind: Kind::BestDish, days: 7, weekday: None, dish: None }, &k, "en");
    assert_eq!(best.text, "In the last 7 days: the best seller was Futomaki, 9 portions, 8100 ALL.");
    traced(&k, &best);
}

#[test]
fn every_kitchen_kind_answers_from_its_cells() {
    let k = kitchen();
    for kind in [Kind::BestDish, Kind::WorstDish, Kind::FoodCost, Kind::Waste, Kind::LowStock, Kind::WorstMargin] {
        let a = answer::answer(&Query { kind, days: 7, weekday: None, dish: None }, &k, "sq");
        assert_eq!(answer::fold_of(kind), Fold::Kitchen);
        assert!(!a.numbers.is_empty(), "{kind:?}: {}", a.text);
        traced(&k, &a);
    }
    let fc = answer::answer(&Query { kind: Kind::FoodCost, days: 7, weekday: None, dish: None }, &k, "en");
    assert_eq!(fc.text, "In the last 7 days: food cost 25.0%: 3100 ALL of goods against 12400 ALL of sales.");
    let m = answer::answer(&Query { kind: Kind::WorstMargin, days: 7, weekday: None, dish: None }, &k, "en");
    assert!(m.text.contains("Miso soup") && m.text.contains("300 ALL"), "{}", m.text);
    let sold = answer::answer(&Query { kind: Kind::DishSold, days: 7, weekday: None, dish: Some("d2".into()) }, &k, "ru");
    assert!(sold.text.contains("Miso soup") && sold.text.contains("5"), "{}", sold.text);
}

#[test]
fn every_analytics_kind_answers_from_its_cells() {
    let a = analytics();
    for kind in [Kind::Revenue, Kind::Orders, Kind::AverageOrder, Kind::Rejected, Kind::BusiestHour, Kind::QuietestHour, Kind::BestDay, Kind::WorstDay, Kind::Channels] {
        for days in [1, 7] {
            let ans = answer::answer(&Query { kind, days, weekday: None, dish: None }, &a, "en");
            assert_eq!(answer::fold_of(kind), Fold::Analytics);
            assert!(!ans.numbers.is_empty(), "{kind:?}: {}", ans.text);
            traced(&a, &ans);
        }
    }
    let r = answer::answer(&Query { kind: Kind::Revenue, days: 7, weekday: None, dish: None }, &a, "en");
    assert_eq!((r.numbers[0].value, r.numbers[0].pointers.clone()), (13000, vec!["/revenue".to_string()]), "the fold's own total");
    let today = answer::answer(&Query { kind: Kind::Revenue, days: 1, weekday: None, dish: None }, &a, "en");
    assert_eq!(today.numbers[0].value, 7000, "the last day's cell");
    let h = answer::answer(&Query { kind: Kind::QuietestHour, days: 7, weekday: None, dish: None }, &a, "en");
    assert!(h.text.contains("12:00"), "the quietest hour WITH orders: {}", h.text);
    // Wednesdays: the third day (2023-11-08) only.
    let wd = answer::answer(&Query { kind: Kind::Revenue, days: 7, weekday: Some(2), dish: None }, &a, "en");
    assert_eq!((wd.numbers[0].value, wd.numbers[0].days.clone()), (3000, vec!["2023-11-08".to_string()]));
    assert_eq!(answer::source_of(Fold::Analytics, 1), "/api/owner/analytics?days=7&v=2");
}

#[test]
fn an_empty_window_says_nothing_sold_and_invents_no_number() {
    let a = answer::answer(&Query { kind: Kind::BestDish, days: 7, weekday: None, dish: None }, &json!({"days": [], "dishes": []}), "en");
    assert!(a.numbers.is_empty());
    assert!(a.text.contains("nothing was sold"), "{}", a.text);
}

// ── the templates and their guard ──────────────────────────────────────────

#[test]
fn a_rewording_is_shown_only_when_it_keeps_every_number_and_adds_none() {
    let t = "In the last 7 days: revenue 13000 ALL from 10 orders.";
    assert!(words::keeps_numbers(t, "In the last 7 days you took 13000 ALL across 10 orders."));
    assert!(!words::keeps_numbers(t, "Over the past week you took 13000 ALL across 10 orders."), "the window is a number too");
    assert!(!words::keeps_numbers(t, "You took about 13,000 ALL from 10 orders."), "a separator splits a number");
    assert!(!words::keeps_numbers(t, "You took 13000 ALL."), "a number dropped");
    assert!(!words::keeps_numbers(t, "You took 13000 ALL from 10 orders, up 12%."), "a number added");
    assert!(!words::keeps_numbers(t, ""));
    assert_eq!(words::numbers("09:00 and 2023-11-06"), vec!["9", "0", "2023", "11", "6"]);
    assert_eq!(words::signed_pct(-45), "-4.5%");
    assert_eq!(words::signed_pct(123), "+12.3%");
    for l in dowiz_hub::lang::LANGS {
        assert!(!words::fill(&words::REVENUE, l, &["x".into(), "1".into(), "2".into()]).contains('{'), "{l}");
    }
}

// ── the explain cards ──────────────────────────────────────────────────────

#[test]
fn the_explain_cards_read_the_screen_and_only_what_it_has() {
    let a = analytics();
    let cards = explain::analytics(&a, "en", 7);
    assert_eq!(cards.iter().map(|c| c.kind).collect::<Vec<_>>(), ["trend", "hours"]);
    // No `compare`: the second half (days 5..7 = 5000+6000+7000) against the first (1000+2000+3000).
    assert_eq!(cards[0].text, "The second half of the period took 18000 ALL, the first 6000 ALL (+200.0%).");
    assert_eq!(cards[1].text, "Busiest hour 20:00 (5 orders); quietest hour with orders 12:00 (1).");
    let mut v2 = a.clone();
    v2["compare"] = json!({"prev": {"revenue": 10000, "deltaPm": {"revenue": 300}}});
    assert_eq!(explain::analytics(&v2, "en", 7)[0].text, "Revenue 13000 ALL against 10000 ALL in the period before (+30.0%).");
    let k = kitchen();
    let kc = explain::kitchen(&k, "uk", 7);
    assert_eq!(kc.iter().map(|c| c.kind).collect::<Vec<_>>(), ["food_cost", "waste", "low_stock"], "no matrix without `menu`");
    let mut k2 = k.clone();
    k2["menu"] = json!({"dishes": [
        {"name": "Futomaki", "quadrant": "star"}, {"name": "Miso soup", "quadrant": "plowhorse", "raiseBy": 120}, {"name": "Gyoza", "quadrant": "dog"}
    ]});
    let kc = explain::kitchen(&k2, "en", 7);
    let kinds: Vec<&str> = kc.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, ["food_cost", "waste", "low_stock", "menu", "star", "plowhorse", "dog"]);
    assert_eq!(kc[3].text, "Menu: 1 stars, 1 plowhorses, 0 puzzles, 1 dogs.");
    assert!(kc[5].text.contains("120 ALL"), "{}", kc[5].text);
}

/// The console's starter questions (`public/admin/ai-i18n.js` `ai_q_*`) are
/// SENT as written, so every one of them must be read by the lexicon.
#[test]
fn every_starter_question_of_the_console_is_read() {
    let want = [Kind::Revenue, Kind::BestDish, Kind::WorstDish, Kind::BusiestHour, Kind::FoodCost, Kind::LowStock];
    let starters: [[&str; 6]; 4] = [
        ["Sa të ardhura këtë javë?", "Cila pjatë u shit më shumë këtë javë?", "Cila pjatë u shit më pak të hënën?",
         "Cila është ora më e ngarkuar këtë muaj?", "Sa është kostoja e ushqimit këtë javë?", "Çfarë po mbaron?"],
        ["What was the revenue this week?", "What sold best this week?", "What sold worst on Monday?",
         "What is the busiest hour this month?", "What is the food cost this week?", "What is running low?"],
        ["Яка виручка за тиждень?", "Що продавалось найкраще за тиждень?", "Що продавалось найгірше в понеділок?",
         "Яка найзавантаженіша година за місяць?", "Який фудкост за тиждень?", "Що закінчується?"],
        ["Какая выручка за неделю?", "Что продавалось лучше всего за неделю?", "Что продавалось хуже всего в понедельник?",
         "Какой самый загруженный час за месяц?", "Какой фудкост за неделю?", "Что заканчивается?"],
    ];
    for (lang, row) in dowiz_hub::lang::LANGS.iter().zip(starters) {
        for (q, kind) in row.iter().zip(want) {
            assert_eq!(read(q).map(|r| r.0), Some(kind), "{lang}: {q}");
            assert_eq!(lexicon::lang_of(q), *lang, "{q}");
        }
    }
}

#[test]
fn both_answer_shapes_are_read_and_usage_is_charged_as_neurons() {
    use super::call::{neurons_of, text_of};
    assert_eq!(text_of(&json!({"response": " <think>a</think> OK "})), "OK");
    assert_eq!(text_of(&json!({"choices": [{"message": {"content": "OK"}}]})), "OK");
    assert_eq!(text_of(&json!({"result": "x"})), "");
    assert_eq!(neurons_of(&json!({"usage": {"prompt_tokens": 1000, "completion_tokens": 100}})), Some(budget::neurons(1000, 100)));
    assert_eq!(neurons_of(&json!({"usage": {}})), None, "no usage: the caller charges the worst case");
}
