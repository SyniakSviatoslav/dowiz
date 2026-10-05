//! A PERSONALISED OFFER'S WORDS, composed by the server (W-SENSE row 7). PURE.
//!
//! A campaign whose segment is `taste` offers ONE dish to guests whose taste profile falls in the
//! owner's segment. Three rules make it lawful, and each is enforced here or in `campaign::define`:
//!   * LABELLED: every such message begins with "Personalised offer" in its language (EU Omnibus
//!     Directive 2019/2161: the guest is told when an offer was chosen for them);
//!   * THE PUBLIC FIGURE: the only number in it is the dish's price as the menu shows it to
//!     everyone, read from the catalogue at the moment the campaign is filed -- the owner types no
//!     text and no number for a taste offer, and a taste offer carries no code (a code is a
//!     reduction, and a reduction chosen by taste is a personalised price);
//!   * NEVER WORSE, NEVER A REFUSAL: nothing here, or anywhere a taste segment is read, sets a
//!     price, a fee or whether a guest is served. A loyalty reward on top of the public figure is
//!     allowed by the operator's ruling but is not offered by this build.

/// The languages a taste offer may be written in: the product's own set.
pub use dowiz_hub::lang::LANGS;

/// The label every taste offer starts with.
pub fn label(lang: &str) -> &'static str {
    match lang {
        "sq" => "Ofertë e personalizuar",
        "uk" => "Персоналізована пропозиція",
        "ru" => "Персонализированное предложение",
        "en" => "Personalised offer",
        _ => "Personalised offer",
    }
}

fn tail(lang: &str) -> &'static str {
    match lang {
        "sq" => "çmimi i menysë, i njëjti për të gjithë. E zgjodhëm nga shijet që porositni më shpesh.",
        "uk" => "ціна з меню, однакова для всіх. Обрано за смаками, які ви замовляєте найчастіше.",
        "ru" => "цена из меню, одинаковая для всех. Выбрано по вкусам, которые вы заказываете чаще всего.",
        "en" => "the menu price, the same for everyone. Chosen from the tastes you order most.",
        _ => "the menu price, the same for everyone. Chosen from the tastes you order most.",
    }
}

/// An integer amount in minor units, written in the currency's own decimals ("900 ALL", "12.50 EUR").
pub fn figure(minor: i64, code: &str) -> String {
    let units = dowiz_core::money::Currency::from_code(code).map_or(0, |c| c.minor_units());
    if units == 0 {
        return format!("{minor} {code}");
    }
    let div = 10_i64.pow(units);
    let sign = if minor < 0 { "-" } else { "" };
    format!("{sign}{}.{:0w$} {code}", minor.abs() / div, minor.abs() % div, w = units as usize)
}

/// The whole message: label, dish, its public figure, and why.
pub fn compose(lang: &str, dish: &str, price_minor: i64, currency: &str) -> String {
    format!("{}: {} -- {}, {}", label(lang), dish.trim(), figure(price_minor, currency), tail(lang))
}

#[cfg(test)]
#[path = "offer/tests.rs"]
mod tests;
