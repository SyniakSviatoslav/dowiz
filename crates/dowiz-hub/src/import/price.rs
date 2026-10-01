//! READING A PRICE OUT OF WHATEVER THE OWNER TYPED -- the dangerous part of an
//! import, kept apart so it can be read, and tested, on its own. Ambiguity is
//! refused with a reason, never guessed.

/// Read a price out of whatever the owner typed.
///
/// Returns `Err` with a human reason rather than a guess. Accepted: digits,
/// with spaces, thin spaces or commas as thousands separators, and an optional
/// trailing or leading currency word ("900", "1 200", "1,200", "900 lek",
/// "ALL 900", "900L").
///
/// REFUSED: anything with a decimal point or comma-as-decimal. The whole system
/// prints these integers WHOLE — the storefront, the admin pane and the courier
/// app all use `maximumFractionDigits: 0`, because the lek's minor unit is the
/// lek. So "9.50" could mean 9, 10, 950 or 9.5, and every one of those is a
/// different price. Refusing costs the owner one correction; guessing costs
/// them the difference on every order until someone notices.
pub fn parse_price(raw: &str) -> Result<i64, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("no price".into());
    }
    let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return Err(format!("no digits in {t:?}"));
    }
    // ONE NUMBER, NOT EVERY DIGIT IN THE CELL (W-AUDIT S6, 2026-09-27).
    // "450/650" (small/large) imported as 450650, "900-1200" as 9001200,
    // "2 x 450" as 2450 and "-900" as 900: the digits were joined whatever
    // stood between them. Two runs of digits are one number only when what
    // separates them is a thousands group -- a separator and exactly three
    // digits, as in "1 200" and "1,200". Anything else is two prices, a range
    // or a sign, and the owner is asked which one they meant.

    // A separator with one or two digits after it and nothing further is a
    // DECIMAL, not a thousands group: "1,50" and "9.50" are prices with
    // fractions, "1,500" and "1.500" are thousands. Distinguishing them by
    // guesswork is exactly what this refuses to do.
    if let Some(pos) = t.rfind(['.', ',']) {
        let after: String = t[pos + 1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        let tail_is_only_digits = t[pos + 1..]
            .chars()
            .skip(after.len())
            .all(|c| !c.is_ascii_digit());
        if (after.len() == 1 || after.len() == 2) && tail_is_only_digits {
            return Err(format!(
                "{t:?} looks like a fractional price; this menu's currency has no subunit, \
                 so write it as a whole number"
            ));
        }
    }

    // AFTER the fraction rule above, so "9.50" keeps its own refusal.
    if t.starts_with('-') {
        return Err(format!("{t:?} is negative; a price is not"));
    }
    if !one_number(t) {
        return Err(format!("{t:?} holds more than one number; write one price per cell"));
    }
    digits.parse::<i64>().map_err(|_| format!("price {t:?} is too large"))
}

/// Do the digit runs of `t` spell ONE number? Consecutive runs may only be
/// joined by a thousands group: one of `. , ' space nbsp` and then exactly
/// three digits. A leading `0` before a separator is a fraction, not a group.
fn one_number(t: &str) -> bool {
    let mut runs: Vec<String> = Vec::new();
    let mut seps: Vec<String> = Vec::new();
    let (mut cur, mut sep, mut in_run) = (String::new(), String::new(), false);
    for c in t.chars() {
        if c.is_ascii_digit() {
            if !in_run {
                if !runs.is_empty() {
                    seps.push(std::mem::take(&mut sep));
                }
                sep.clear();
                in_run = true;
            }
            cur.push(c);
        } else if in_run {
            runs.push(std::mem::take(&mut cur));
            in_run = false;
            sep.push(c);
        } else {
            sep.push(c);
        }
    }
    if in_run {
        runs.push(cur);
    }
    if runs.len() > 1 && runs[0] == "0" {
        return false;
    }
    seps.iter().enumerate().all(|(i, s)| {
        matches!(s.as_str(), "." | "," | "'" | " " | "\u{a0}" | "\u{202f}" | "\u{2009}") && runs[i + 1].len() == 3
    })
}
