//! A NUMBER AS PEOPLE TYPE IT, TO E.164 -- or nothing. PURE.
//!
//! Albanians type `069 123 4567`, Ukrainians `067 123 45 67`; a gateway wants
//! `+355691234567` / `+380671234567`. A national spelling is completed from
//! the ORDER'S CURRENCY (ALL -> +355, UAH -> +380), the one fact the order
//! carries about where it was placed. Anything else that is not already
//! international is refused: texting a guess is texting a stranger.

/// The country code a national number is completed with, by the order's currency.
pub fn country_of(currency: &str) -> Option<&'static str> {
    match currency {
        "ALL" => Some("355"),
        "UAH" => Some("380"),
        _ => None,
    }
}

/// `+` and 8..=15 digits (ITU-T E.164), or `None`.
pub fn e164(raw: &str, currency: &str) -> Option<String> {
    let t = raw.trim();
    let plus = t.starts_with('+');
    if t.chars().any(|c| !(c.is_ascii_digit() || " -().+/".contains(c))) {
        return None;
    }
    let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    let full = if plus {
        digits
    } else if let Some(rest) = digits.strip_prefix("00") {
        rest.to_string()
    } else if digits.starts_with("355") && digits.len() >= 11 || digits.starts_with("380") && digits.len() == 12 {
        digits
    } else if let Some(rest) = digits.strip_prefix('0') {
        format!("{}{rest}", country_of(currency)?)
    } else {
        return None;
    };
    (8..=15).contains(&full.len()).then(|| format!("+{full}")).filter(|_| !full.starts_with('0'))
}

/// Every E.164 number an owner's typing can mean, for a STOP: one when it is
/// international, else its completion in each country we serve. Filing a
/// withdrawal under a number nobody ordered with is harmless (a pseudonym
/// with no grant); missing the one they did order with is not.
pub fn candidates(raw: &str) -> Vec<String> {
    if let Some(one) = e164(raw, "") {
        return vec![one];
    }
    let mut out: Vec<String> = ["ALL", "UAH"].iter().filter_map(|c| e164(raw, c)).collect();
    out.dedup();
    out
}

#[cfg(test)]
mod tests;
