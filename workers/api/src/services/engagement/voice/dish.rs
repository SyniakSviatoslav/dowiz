//! PURE. A dish named aloud, found on the menu -- or refused.
//!
//! EXACTLY ONE OR NOTHING. A spoken name that fits two dishes is the case where
//! a guess rings the wrong plate, so it is refused with the candidates read
//! back; a name that fits none is refused with what was heard. A dish is known
//! by every name it has (the venue's own and each translation), because a
//! waiter speaking Ukrainian at an Albanian venue says the Ukrainian name.
//!
//! INFLECTION, deterministically: Ukrainian and Albanian change a noun's ending
//! with its role in the sentence ("маргарита" -> "маргариту", "margarita" ->
//! "margaritën"). Two words match when one's STEM (all but the last letter,
//! for words of five letters or more) begins the other's and the shorter stem
//! is at least four letters. No edit distance, no model: the same words always
//! find the same dish.

use super::words::{norm, words};

/// One dish as the matcher sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    pub id: String,
    /// Every name, in any language, as written on the menu.
    pub names: Vec<String>,
    pub available: bool,
}

/// Why a name found no single dish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Miss {
    None,
    /// The candidates' first names, at most `SHOWN`.
    Many(Vec<String>),
}

/// How many candidates an ambiguity reads back.
pub const SHOWN: usize = 3;
/// Below this a stem is too short to stand for a word.
const MIN_STEM: usize = 4;
/// Words this long lose their last letter to the stem.
const STEMMED_FROM: usize = 5;

fn stem(w: &str) -> &str {
    let n = w.chars().count();
    if n < STEMMED_FROM {
        return w;
    }
    let cut = w.char_indices().nth(n - 1).map(|(i, _)| i).unwrap_or(w.len());
    &w[..cut]
}

/// Do two words name the same thing?
pub fn same_word(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (sa, sb) = (stem(a), stem(b));
    let (short, long) = if sa.chars().count() <= sb.chars().count() { (sa, sb) } else { (sb, sa) };
    short.chars().count() >= MIN_STEM && long.starts_with(short)
}

/// Does every spoken word appear in this name?
fn fits(said: &[&str], name: &str) -> bool {
    let n = norm(name);
    let nw = words(&n);
    !said.is_empty() && said.iter().all(|s| nw.iter().any(|w| same_word(s, w)))
}

/// Find the one dish a phrase names. An EXACT name wins over word matches, so
/// "cola" finds Cola even when "Cola Zero" is on the menu too.
pub fn find<'a>(menu: &'a [Dish], phrase: &str) -> Result<&'a Dish, Miss> {
    let p = norm(phrase);
    let exact: Vec<&Dish> = menu.iter().filter(|d| d.names.iter().any(|n| norm(n) == p)).collect();
    let hits = if exact.is_empty() {
        let said = words(&p);
        menu.iter().filter(|d| d.names.iter().any(|n| fits(&said, n))).collect()
    } else {
        exact
    };
    match hits.as_slice() {
        [one] => Ok(one),
        [] => Err(Miss::None),
        many => Err(Miss::Many(
            many.iter().take(SHOWN).map(|d| d.names.first().cloned().unwrap_or_default()).collect(),
        )),
    }
}

#[cfg(test)]
mod tests;
