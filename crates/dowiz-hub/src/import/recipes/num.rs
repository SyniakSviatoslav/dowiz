//! Numbers, units, money and dates out of a spreadsheet cell — exactly.
//!
//! EVERY VALUE HERE IS AN INTEGER OVER A POWER OF TEN until the moment it is
//! scaled into the hub's integer base unit, and at that moment it either
//! divides exactly or it is REFUSED. "0.0425 kg" is 42.5 g; the ledger counts
//! whole grams (`stock::Qty`), and rounding it to 42 or 43 is a relative error
//! no consumer downstream can see — the kind that under-reserves a little on
//! every order for ever.

/// A decimal as written: `mant / 10^scale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dec {
    pub mant: i128,
    pub scale: u32,
}

/// Read a non-negative decimal. `.` is always a decimal point; `,` is one too
/// (Excel in a decimal-comma locale), EXCEPT where it is ambiguous: a comma
/// followed by exactly three digits after a non-zero whole part ("1,500") is
/// a thousands group or a decimal, and choosing is a guess, so it is refused.
pub fn parse_decimal(raw: &str) -> Result<Dec, String> {
    let t: String = raw
        .trim()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{202f}' && *c != '\u{a0}')
        .collect();
    if t.is_empty() {
        return Err("empty".into());
    }
    if t.starts_with('-') {
        return Err(format!("{raw:?} is negative"));
    }
    let seps: Vec<(usize, char)> = t.char_indices().filter(|(_, c)| *c == '.' || *c == ',').collect();
    if seps.len() > 1 {
        return Err(format!("{raw:?} has more than one separator; write the number plainly"));
    }
    let (int, frac, comma) = match seps.first() {
        Some(&(i, c)) => (&t[..i], &t[i + 1..], c == ','),
        None => (&t[..], "", false),
    };
    let digits_only = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if !digits_only(int) || !digits_only(frac) || (int.is_empty() && frac.is_empty()) {
        return Err(format!("{raw:?} is not a number"));
    }
    if comma && frac.len() == 3 && !int.trim_start_matches('0').is_empty() {
        return Err(format!("{raw:?} could be a thousand times this or that; write it without the comma"));
    }
    let frac = frac.trim_end_matches('0');
    let all = format!("{int}{frac}");
    if all.len() > 30 {
        return Err(format!("{raw:?} is too long"));
    }
    let mant = if all.is_empty() { 0 } else { all.parse::<i128>().map_err(|_| format!("{raw:?} is not a number"))? };
    Ok(Dec { mant, scale: frac.len() as u32 })
}

/// The three base units a supply is counted in (`recipe::UNITS` in the Worker).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    G,
    Ml,
    Unit,
}

impl Base {
    pub fn as_str(self) -> &'static str {
        match self {
            Base::G => "g",
            Base::Ml => "ml",
            Base::Unit => "unit",
        }
    }
    /// How many base units a supply's per-basis numbers describe (per 100 g
    /// or ml, per ONE piece) — `recipe::basis_of`.
    pub fn basis(self) -> i128 {
        if self == Base::Unit { 1 } else { 100 }
    }
}

/// A unit word, as the base unit it is counted in and how many of that base
/// one of it is. `None` for any other word: the set is closed (`recipe.rs`),
/// and a "kg" read as "g" under-reserves by a thousand.
pub fn unit_of(raw: &str) -> Option<(Base, i128)> {
    let w = raw.trim().to_lowercase();
    Some(match w.trim_end_matches('.') {
        "g" | "gr" | "gram" | "grams" | "gramë" | "г" | "гр" => (Base::G, 1),
        "kg" | "kilogram" | "kilograms" | "кг" => (Base::G, 1000),
        "ml" | "мл" => (Base::Ml, 1),
        "l" | "lt" | "litre" | "liter" | "litër" | "л" => (Base::Ml, 1000),
        "p" | "pc" | "pcs" | "piece" | "pieces" | "unit" | "units" | "u" | "шт" | "copë" | "cope" => {
            (Base::Unit, 1)
        }
        _ => return None,
    })
}

/// Split "40 g" / "0.5kg" into the number and the unit word written with it.
pub fn split_unit(cell: &str) -> (&str, Option<&str>) {
    let t = cell.trim();
    let at = t.find(|c: char| c.is_alphabetic()).unwrap_or(t.len());
    let (n, u) = (t[..at].trim(), t[at..].trim());
    (n, if u.is_empty() { None } else { Some(u) })
}

/// An exact fraction, for scaling through a semi-finished product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rat {
    pub num: i128,
    pub den: i128,
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

impl Rat {
    pub fn new(num: i128, den: i128) -> Rat {
        let g = gcd(num, den).max(1);
        Rat { num: num / g, den: den / g }
    }
    pub fn of(d: Dec, per: i128) -> Rat {
        Rat::new(d.mant * per, 10i128.pow(d.scale))
    }
    pub fn mul(self, o: Rat) -> Rat {
        Rat::new(self.num * o.num, self.den * o.den)
    }
    pub fn div(self, o: Rat) -> Rat {
        Rat::new(self.num * o.den, self.den * o.num)
    }
    pub fn add(self, o: Rat) -> Rat {
        Rat::new(self.num * o.den + o.num * self.den, self.den * o.den)
    }
    /// The whole number this is, or why it is not one.
    pub fn whole(self, base: Base, max: i64) -> Result<i64, String> {
        if self.num % self.den != 0 {
            let tenths = self.num * 10 / self.den;
            return Err(format!(
                "comes to {}.{}… {}; a fraction of a {} is refused, not rounded",
                tenths / 10,
                tenths % 10,
                base.as_str(),
                base.as_str()
            ));
        }
        let q = self.num / self.den;
        if q <= 0 {
            return Err("is zero".into());
        }
        if q > max as i128 {
            return Err(format!("is {q} {}, over the {max} a line may hold", base.as_str()));
        }
        Ok(q as i64)
    }
}

/// How the file writes money. The owner SAYS which; it is never guessed,
/// because Poster's "kopecks" in an account set to lek are either ×1 or a
/// defect, and nothing in the number tells which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostScale {
    /// "1800" or "18.50" — whole units of the currency.
    Major,
    /// "1800" meaning 18.00 — hundredths (Poster's "kopecks").
    Hundredths,
}

/// Minor units per major unit — `lib/money.js`'s `DECIMALS`, and 2 for the
/// rest as there.
pub fn decimals_of(code: &str) -> u32 {
    if code.eq_ignore_ascii_case("ALL") { 0 } else { 2 }
}

/// A currency written beside a number, as its code.
pub fn currency_word(w: &str) -> Option<String> {
    let l = w.trim().trim_end_matches('.').to_lowercase();
    Some(match l.as_str() {
        "" => return None,
        "l" | "lek" | "lekë" | "leke" => "ALL".into(),
        "€" | "euro" | "eur" => "EUR".into(),
        "$" | "usd" => "USD".into(),
        "₴" | "грн" | "uah" => "UAH".into(),
        s if s.len() == 3 && s.chars().all(|c| c.is_ascii_alphabetic()) => s.to_uppercase(),
        _ => return None,
    })
}

/// A cost per ONE of `per_unit`, as the venue's minor units per basis
/// (`costPerBasis`: per 100 g/ml, per piece). Exact, or refused.
pub fn cost_per_basis(d: Dec, scale: CostScale, currency: &str, per_unit: (Base, i128)) -> Result<i64, String> {
    let (base, per) = per_unit;
    let extra = if scale == CostScale::Hundredths { 2 } else { 0 };
    let r = Rat::new(
        d.mant * 10i128.pow(decimals_of(currency)) * base.basis(),
        10i128.pow(d.scale + extra) * per,
    );
    if r.num % r.den != 0 {
        return Err(format!(
            "comes to a fraction of the smallest {currency} unit per {}{}; refused, not rounded",
            if base == Base::Unit { "" } else { "100 " },
            base.as_str()
        ));
    }
    i64::try_from(r.num / r.den).map_err(|_| "is too large".into())
}

/// A date as a day number (days since 1970-01-01), from `YYYY-MM-DD` or
/// `DD.MM.YYYY`, the two shapes the incumbents' exports write.
pub fn parse_day(raw: &str) -> Result<i64, String> {
    let t = raw.trim();
    let parts: Vec<&str> = if t.contains('-') { t.split('-').collect() } else { t.split('.').collect() };
    let n: Vec<i64> = parts.iter().filter_map(|p| p.trim().parse().ok()).collect();
    if n.len() != 3 || parts.len() != 3 {
        return Err(format!("{t:?} is not a date (write 2026-09-22)"));
    }
    let (y, m, d) = if t.contains('-') { (n[0], n[1], n[2]) } else { (n[2], n[1], n[0]) };
    if !(1970..=9999).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(format!("{t:?} is not a date"));
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok(era * 146_097 + doe - 719_468)
}
