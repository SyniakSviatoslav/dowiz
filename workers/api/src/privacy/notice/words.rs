//! The notice's words in one language. Every sentence is a template the
//! renderer fills from the registry and the venue; nothing here names a store,
//! a processor or a number of days that the code does not supply.

use crate::privacy::registry::{Basis, Data, Purpose};

pub struct Words {
    pub lang: &'static str,
    pub title: &'static str,
    pub version: &'static str,
    pub who_h: &'static str,
    /// `{venue}`.
    pub who: &'static str,
    /// `{address}`.
    pub who_address: &'static str,
    /// `{phone}`.
    pub who_phone: &'static str,
    pub dowiz_role: &'static str,
    pub keep_h: &'static str,
    pub keep_intro: &'static str,
    pub col_what: &'static str,
    pub col_why: &'static str,
    pub col_basis: &'static str,
    pub col_long: &'static str,
    pub col_erase: &'static str,
    /// `{n}`.
    pub days: &'static str,
    pub one_day: &'static str,
    pub latest: &'static str,
    pub until_done: &'static str,
    pub no_limit: &'static str,
    pub erased: &'static str,
    pub retained: &'static str,
    pub expires: &'static str,
    pub not_reached: &'static str,
    pub must_give: &'static str,
    pub device_h: &'static str,
    /// `{data}`.
    pub device: &'static str,
    pub recipients_h: &'static str,
    pub recipients_intro: &'static str,
    pub receives: &'static str,
    pub where_: &'static str,
    pub safeguard: &'static str,
    pub backups_h: &'static str,
    /// `{days}`.
    pub backups: &'static str,
    pub recovery: &'static str,
    pub rights_h: &'static str,
    pub rights: &'static [&'static str],
    /// `{venue}`, `{contact}`.
    pub how_to_ask: &'static str,
    pub deadline: &'static str,
    pub marketing_h: &'static str,
    pub marketing: &'static str,
    pub automated_h: &'static str,
    pub automated: &'static str,
    pub complaint_h: &'static str,
    pub complaint: &'static str,
    /// `{email}`.
    pub dowiz_contact: &'static str,
    pub platform_title: &'static str,
    pub platform_who: &'static str,
    pub platform_venues: &'static str,
    pub data: fn(Data) -> &'static str,
    pub purpose: fn(Purpose) -> &'static str,
    pub basis: fn(Basis) -> &'static str,
    /// A processor's location and safeguard in this language, by its id.
    pub processor: fn(&str) -> Option<(&'static str, &'static str)>,
}

/// The three languages the storefront speaks; anything else gets Albanian,
/// the venue's own language.
pub fn words(lang: &str) -> &'static Words {
    match lang {
        "en" => &super::en::EN,
        "uk" => &super::uk::UK,
        _ => &super::sq::SQ,
    }
}

pub const LANGS: [&str; 3] = ["sq", "en", "uk"];
