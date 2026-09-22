//! What an owner is shown instead of a customer's name and number.
//!
//! THE MASK IS THE WHOLE PROTECTION. There is no customer registry behind the
//! owner's customer list — it is a fold over the orders, computed per request
//! and stored nowhere — so what the venue gains over reading the orders itself
//! is the CONVENIENCE of a ready-made list, sorted by value, one click from
//! export. These two functions are the only thing standing between that list
//! and every number the venue has ever served.
//!
//! IT LIVES HERE BECAUSE THERE WERE TWO OF IT: `workers/api` and
//! `tools/native-spa-server` each had a copy, character for character, and
//! each had the same defect below and no test. A rule with two implementations
//! is a rule that will be fixed once.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Enough to recognise a number you already know; not enough to dial one you
/// do not.
///
/// A MASK MUST HIDE AT LEAST AS MUCH AS IT SHOWS, and this one did not. The
/// rule was "first three, five dots, last two" applied to anything with four
/// digits or more, so:
///
/// * `1234` became `+123•••••34` — all four digits, one of them twice;
/// * `12345` became `+123•••••45` — all five;
/// * `123456` became `+123•••••56` — five of six.
///
/// The dots made it look masked. Anything short — an internal extension, a
/// short code, a number somebody typed half of — was printed in full on a
/// screen whose entire purpose is to not print it. So the length now decides
/// how much may be shown.
///
/// THE `+` IS NO LONGER INVENTED either: `0691234567` was rendered
/// `+069•••••67`, which states a country code that is not there, and an owner
/// reading that back to a courier dials a number that does not exist.
pub fn phone(p: &str) -> String {
    let d: Vec<char> = p.chars().filter(char::is_ascii_digit).collect();
    let plus = if p.trim_start().starts_with('+') { "+" } else { "" };
    let last2 = |d: &[char]| d[d.len() - 2..].iter().collect::<String>();
    match d.len() {
        // Nothing can be shown here that does not show most of it. The length
        // survives, which is what tells an owner the field is not empty.
        0..=6 => {
            let n = if d.is_empty() { 1 } else { d.len() };
            "•".repeat(n)
        }
        // Shows 2, hides at least 5.
        7..=9 => format!("{plus}•••••{}", last2(&d)),
        // Shows 5, hides at least 5.
        _ => format!("{plus}{}•••••{}", d[..3].iter().collect::<String>(), last2(&d)),
    }
}

/// A name as initials: "A. H." recognises somebody you know and identifies
/// nobody you do not.
///
/// An empty or punctuation-only name is an em dash rather than an empty field —
/// a blank cell reads as a bug and the owner goes looking for the name.
pub fn name(n: &str) -> String {
    let parts: Vec<String> = n
        .split_whitespace()
        .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
        .map(|c| format!("{}.", c.to_uppercase()))
        .collect();
    if parts.is_empty() {
        "—".to_string()
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A MASK MUST HIDE AT LEAST AS MUCH AS IT SHOWS. The old rule returned a
    /// four-digit number with all four of its digits — one of them twice —
    /// behind a row of dots that made it look protected.
    #[test]
    fn a_short_number_is_not_partly_masked_it_is_entirely_masked() {
        for short in ["1234", "12345", "123456", "12-34-56"] {
            let m = phone(short);
            let digits = short.chars().filter(char::is_ascii_digit).count();
            assert!(!m.chars().any(|c| c.is_ascii_digit()), "{short} -> {m}");
            assert_eq!(m.chars().count(), digits, "the length is all that is left");
        }
    }

    /// SEVEN TO NINE DIGITS KEEPS THE LAST TWO AND NOTHING ELSE: showing the
    /// first three as well would leave fewer digits hidden than shown.
    #[test]
    fn a_mid_length_number_keeps_only_its_last_two() {
        assert_eq!(phone("1234567"), "•••••67");
        assert_eq!(phone("123456789"), "•••••89");
    }

    /// A REAL NUMBER IS STILL RECOGNISABLE: three at the front, two at the
    /// back, at least five hidden between them.
    #[test]
    fn a_full_number_shows_five_and_hides_at_least_five() {
        assert_eq!(phone("+355 69 123 4567"), "+355•••••67");
        let hidden = "+355 69 123 4567".chars().filter(char::is_ascii_digit).count() - 5;
        assert!(hidden >= 5, "hidden {hidden}");
    }

    /// An owner reading `+069•••••67` back to a courier dials a number that
    /// does not exist.
    #[test]
    fn a_national_number_does_not_grow_a_country_code() {
        assert_eq!(phone("0691234567"), "069•••••67");
        assert_eq!(phone("+355691234567"), "+355•••••67");
    }

    #[test]
    fn a_nameless_order_is_a_dash_and_punctuation_is_not_an_initial() {
        assert_eq!(name(""), "—");
        assert_eq!(name("   "), "—");
        assert_eq!(name("..."), "—");
        assert_eq!(name("Arben Hoxha"), "A. H.");
        assert_eq!(name("(arben)"), "A.", "the bracket is not the initial");
        assert_eq!(phone(""), "•");
    }
}
