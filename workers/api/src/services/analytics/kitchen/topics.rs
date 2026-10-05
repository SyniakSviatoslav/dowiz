//! PURE. What guests' notes are ABOUT, per dish per week (P16b, research
//! 2026-10-03): "the rice was cold", "too salty", "small portion", "very
//! tasty", in sq/en/uk/ru, counted from `feedback.text` (`orders/feedback.rs`).
//!
//! A TOPIC IS ABOUT A DISH, NEVER ABOUT A PERSON. The fold is keyed by
//! (week, dish, topic) and nothing else: no order id, no guest, no courier, no
//! member of staff ever reaches the key or the answer, so "late" counted here
//! is a kitchen's week, not somebody's record (DECISIONS.md OD-8,
//! `tools/gates/no-scoring.sh`). No score either: a count of notes that say a
//! thing, which a kitchen can act on, and never a number about a guest's mood.
//!
//! THE EXAMPLE PHRASE IS REDACTED, not trusted: a word holding a digit (a
//! phone, a flat, an order number) or an `@`, a capitalised word past the
//! sentence's first (a name), and the word after "courier"/"my name is" are
//! replaced with `…`. Deterministic, no model: the same note always gives
//! the same topics and the same phrase.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::sales::{Dish, Window};
use dowiz_hub::stock::meta::{day_number, day_of_number};

/// `(topic, words)`. A word ending in `*` matches as a prefix (a stem: Ukrainian
/// and Russian change endings); any other matches whole. Albanian is folded
/// (ë -> e, ç -> c), as a phone keyboard often types it.
const TOPICS: &[(&str, &[&str])] = &[
    ("cold", &["cold", "lukewarm", "холодн*", "остиг*", "охолол*", "прохолодн*", "остыл*", "остыв*", "ftoht*", "ftohur*"]),
    ("late", &["late", "delay*", "slow", "запізн*", "затрим*", "довго", "опозд*", "задерж*", "долго", "vone", "vonu*", "vones*"]),
    ("salty", &["salty", "oversalted", "солон*", "пересол*", "солён*", "krip*"]),
    ("small_portion", &["small", "tiny", "skimpy", "маленьк*", "мало", "замал*", "vogel", "pak"]),
    ("tasty", &["tasty", "delicious", "yummy", "смачн*", "смакота", "вкусн*", "shijshem*", "shijshme"]),
    ("not_tasty", &["bland", "tasteless", "awful", "disgusting", "несмачн*", "прісн*", "жахлив*", "невкусн*", "пресн*", "ужасн*", "keq"]),
    ("fresh", &["fresh", "свіж*", "свеж*", "fresk*"]),
    ("stale", &["stale", "несвіж*", "несвеж*", "bajat*"]),
    ("spicy", &["spicy", "гостр*", "пекуч*", "остр*", "djeges*", "pikant*"]),
    ("packaging", &["packaging", "packed", "spilled", "spilt", "leaked", "leaking", "упаков*", "розлил*", "протік*", "протекл*", "разлил*", "ambalazh*", "paketim*", "derdh*"]),
];
/// A negated hit becomes this topic, or is dropped ("not cold" says nothing).
const NEGATED: &[(&str, &str)] = &[("tasty", "not_tasty"), ("fresh", "stale")];
const NOT: &[&str] = &["not", "no", "never", "wasnt", "isnt", "werent", "не", "ні", "нет", "nuk", "jo", "s", "pa"];
/// The word after one of these is somebody's name.
const NAME_CUES: &[&str] = &[
    "courier", "driver", "rider", "waiter", "name", "курєр*", "курьер*", "офіціант*", "официант*", "звати", "зовут", "імя", "имя", "korrier*", "kamarier*", "quhet", "emri",
];
/// At most this many example phrases per (week, dish, topic), each at most `PHRASE_WORDS` words.
pub const EXAMPLES: usize = 2;
pub const PHRASE_WORDS: usize = 12;

fn flat(w: &str) -> String {
    w.chars().filter(|c| !matches!(c, '\'' | '\u{2019}' | '\u{02bc}')).map(|c| match c { 'ë' => 'e', 'ç' => 'c', c => c }).collect()
}

fn hit(word: &str, pattern: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(stem) => word.starts_with(stem),
        None => word == pattern,
    }
}

/// The note's words: lowercased and folded, punctuation out.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}' | '\u{02bc}')))
        .filter(|w| !w.is_empty())
        .map(|w| flat(&w.to_lowercase()))
        .collect()
}

/// The topics a note raises, each once, with the index of the word that raised it.
pub fn topics_of(text: &str) -> Vec<(&'static str, usize)> {
    let ws = words(text);
    let mut out: Vec<(&'static str, usize)> = Vec::new();
    for (i, w) in ws.iter().enumerate() {
        let Some((topic, _)) = TOPICS.iter().find(|(_, ps)| ps.iter().any(|p| hit(w, p))) else { continue };
        let negated = ws[i.saturating_sub(3)..i].iter().any(|p| NOT.contains(&p.as_str()));
        let topic = match (negated, NEGATED.iter().find(|(t, _)| t == topic)) {
            (false, _) => *topic,
            (true, Some((_, flipped))) => *flipped,
            (true, None) => continue,
        };
        if !out.iter().any(|(t, _)| *t == topic) {
            out.push((topic, i));
        }
    }
    out
}

/// The sentence around word `at`, with digits, addresses and names taken out.
pub fn example(text: &str, at: usize) -> String {
    let raw: Vec<&str> = text.split_whitespace().collect();
    // Word indices of `words()` and `split_whitespace` differ on punctuation;
    // the phrase is cut around the same position, which is near enough.
    let at = at.min(raw.len().saturating_sub(1));
    let from = at.saturating_sub(PHRASE_WORDS / 2);
    let to = (from + PHRASE_WORDS).min(raw.len());
    let mut out: Vec<String> = Vec::new();
    let mut after_cue = false;
    for (k, w) in raw[from..to].iter().enumerate() {
        let core: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
        // The first word of a sentence may be capitalised without being a name.
        let first = (from + k).checked_sub(1).map_or(true, |j| raw[j].ends_with(['.', '!', '?']));
        let lower = flat(&core.to_lowercase());
        let hidden = after_cue
            || w.chars().any(|c| c.is_ascii_digit() || c == '@' || c == '+')
            || (!first && core.chars().next().is_some_and(char::is_uppercase));
        after_cue = NAME_CUES.iter().any(|p| hit(&lower, p));
        let shown = if hidden { "…".to_string() } else { w.to_lowercase() };
        if !(hidden && out.last().is_some_and(|l| l == "…")) {
            out.push(shown);
        }
    }
    out.join(" ")
}

/// The Monday (`yyyymmdd`) of the venue's week holding `day` (`yyyymmdd`).
pub fn week_of(day: i64) -> i64 {
    let n = day_number(day);
    day_of_number(n - (n + 3).rem_euclid(7))
}

/// Does this note name the dish? Every word of four letters or more in the
/// dish's name appears in the note (its stem: all but the last letter).
fn names(dish: &Dish, ws: &[String]) -> bool {
    let long: Vec<String> = words(&dish.name).into_iter().filter(|w| w.chars().count() >= 4).collect();
    !long.is_empty()
        && long.iter().all(|n| {
            let stem: String = n.chars().take(n.chars().count() - 1).collect();
            ws.iter().any(|w| w.starts_with(&stem))
        })
}

/// THE FOLD: every delivered order's note inside the window, its topics
/// counted once per (week, dish, topic) -- for the dishes the note names, or
/// every dish of the order when it names none. Sorted by week, dish, topic.
pub fn fold(orders: &[Value], dishes: &std::collections::HashMap<String, Dish>, w: &Window) -> Value {
    let mut cells: BTreeMap<(i64, String, &'static str), (u32, Vec<String>)> = BTreeMap::new();
    for o in orders {
        let Some(fb) = o.get("feedback") else { continue };
        let (Some(text), Some(at)) = (fb.get("text").and_then(Value::as_str), fb.get("at").and_then(Value::as_i64)) else { continue };
        let Some(day) = w.bucket(at).map(|i| w.days[i]) else { continue };
        let found = topics_of(text);
        if found.is_empty() {
            continue;
        }
        let mut ids: Vec<&str> = o.get("items").and_then(Value::as_array).into_iter().flatten()
            .filter_map(|it| it.get("product_id").and_then(Value::as_str)).filter(|p| dishes.contains_key(*p)).collect();
        ids.sort_unstable();
        ids.dedup();
        let ws = words(text);
        let named: Vec<&str> = ids.iter().copied().filter(|id| names(&dishes[*id], &ws)).collect();
        let about = if named.is_empty() { ids } else { named };
        for id in about {
            for (topic, at) in &found {
                let cell = cells.entry((week_of(day), id.to_string(), *topic)).or_default();
                cell.0 += 1;
                if cell.1.len() < EXAMPLES {
                    cell.1.push(example(text, *at));
                }
            }
        }
    }
    Value::Array(
        cells.into_iter()
            .map(|((week, dish, topic), (count, examples))| {
                let name = dishes.get(&dish).map(|d| d.name.clone()).unwrap_or_else(|| dish.clone());
                json!({ "week": week, "dish": dish, "name": name, "topic": topic, "count": count, "examples": examples })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests;
