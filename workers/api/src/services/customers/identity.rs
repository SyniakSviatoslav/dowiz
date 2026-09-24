//! ONE PHONE, SPELLED TWO WAYS (§3.4 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22).
//!
//! PURE. `canonical_digits` is the one normaliser the rule-written aliases come
//! from: `+355 69 123 4567`, `069 123 4567`, `00355691234567` and
//! `355691234567` are one Albanian phone, and all four answer `355691234567`.
//!
//! IT DOES NOT CHANGE ANY KEY. `customer_key` stays the HMAC of the digits as
//! typed, so a card, a consent act, a wallet or a reveal filed under a key
//! before this existed still names the same person. What this feeds is the
//! ALIAS: the national spelling's key is written as an alias of the E.164
//! spelling's key (`alias::rule_link`), and the fold groups by the alias.
//!
//! WHAT IS REFUSED: a number that says it is from ANOTHER country (`+383…`,
//! `00383…`) is not this venue's to rewrite, and answers `None`; so does
//! anything that is not a phone. `None` means "no alias", never "no customer".

/// The dialling code of every venue on the platform today (Durrës, Albania).
/// THE VENUE HAS NO COUNTRY FIELD: the day one opens elsewhere, this becomes
/// the venue's, passed in by the placement.
pub const VENUE_DIAL: &str = "355";

/// Characters a typed phone may carry besides digits.
const PUNCT: [char; 6] = [' ', '+', '-', '(', ')', '.'];

/// The E.164 digits (no `+`) of `phone` for a venue whose dialling code is
/// `country` (`"355"`, `"+355"`; `"AL"` is accepted as the same). `None` for
/// garbage, for another country's number, and for a country not supported.
pub fn canonical_digits(phone: &str, country: &str) -> Option<String> {
    let cc = match country.trim_start_matches('+') {
        "355" | "AL" => "355",
        _ => return None,
    };
    // The placement reads the phone trimmed in one place and as sent in
    // another; both must answer the same.
    let phone = phone.trim();
    if phone.chars().any(|c| !c.is_ascii_digit() && !PUNCT.contains(&c)) {
        return None;
    }
    let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
    let international = phone.trim_start().starts_with('+') || digits.starts_with("00");
    let digits = digits.strip_prefix("00").unwrap_or(&digits);
    // THE NATIONALLY SIGNIFICANT NUMBER: after the country code, or after the
    // national trunk `0`, or a bare mobile typed without either.
    let nsn = if let Some(rest) = digits.strip_prefix(cc) {
        rest
    } else if international {
        // `+383…` / `00383…`: a Kosovan (or any other) number is not ours.
        return None;
    } else if let Some(rest) = digits.strip_prefix('0') {
        rest
    } else if digits.len() == 9 && digits.starts_with('6') {
        digits
    } else {
        return None;
    };
    albanian_nsn(nsn).then(|| format!("{cc}{nsn}"))
}

/// Albania's national numbers: mobiles are `6x` + 7 digits (9 in all);
/// fixed lines are 8 digits (Tirana `4…`, the regions `2…`-`8…`). The first
/// digit is never 0 or 1.
fn albanian_nsn(n: &str) -> bool {
    let first = n.as_bytes().first().copied().unwrap_or(b'0');
    let len_ok = if first == b'6' { n.len() == 9 } else { n.len() == 8 };
    len_ok && (b'2'..=b'9').contains(&first)
}

/// THE ONE KEY FOR A PERSON, from any spelling of their phone (audit D38).
///
/// `customer_key` of the E.164 digits when the number is this venue's
/// country's, of the phone as typed otherwise (a visitor's `+39…` is still a
/// phone). Bookings, the wallet and the rule's alias target all derive from
/// THIS function, so `069 123 4567` and `+355 69 123 4567` name one person in
/// every module -- the booking module had its own copy of these two lines,
/// and the wallet used the key of the spelling as typed.
pub fn person_key(secret: &[u8], phone: &str) -> String {
    let canonical = canonical_digits(phone, VENUE_DIAL);
    super::handlers::customer_key(secret, canonical.as_deref().unwrap_or(phone))
}

/// The rule's alias for a placement, as `(from, to)` customer keys: the key of
/// the spelling as typed, and the key of its E.164 spelling. `None` when the
/// phone is already in the canonical spelling (the two keys are equal) or
/// cannot be normalised -- in both cases there is nothing to link.
pub fn rule_pair(secret: &[u8], phone: &str, country: &str) -> Option<(String, String)> {
    use super::handlers::customer_key;
    let canonical = canonical_digits(phone, country)?;
    let (from, to) = (customer_key(secret, phone), customer_key(secret, &canonical));
    (from != to).then_some((from, to))
}

/// What the placement hands `at_placement::remember_spelling`: the key this
/// order's spelling is linked TO, under the venue's dialling code. Its `from`
/// is the placement's own `customer_key(secret, phone)`, by construction.
pub fn alias_at_placement(secret: &[u8], phone: &str) -> Option<String> {
    rule_pair(secret, phone, VENUE_DIAL).map(|(_, to)| to)
}

#[cfg(test)]
mod tests;
