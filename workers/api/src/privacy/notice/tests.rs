//! The notice says what the registry and the venue's switches say, in each
//! language, and nothing it cannot back.

use super::*;
use crate::privacy::registry::{Data, Purpose, Basis};

fn venue() -> Venue {
    Venue { name: "Sushi Durrës".into(), address: Some("Rruga Tregtare 1, Durrës".into()), phone: Some("+355 69 000 0000".into()) }
}

fn named(html: &str, id: &str) -> bool {
    html.contains(&format!("data-processor=\"{id}\""))
}

/// P8's CHECK, native: each language names every processor ON and none OFF.
/// Switching Telegram off makes it disappear.
#[test]
fn a_processor_is_named_exactly_when_it_is_switched_on() {
    let all = On { telegram: true, meta: true, cloud: true, stripe: true, ai: true };
    let none = On::default();
    for lang in words::LANGS {
        let on = render_venue(&venue(), &all, lang);
        for id in ["cloudflare", "telegram", "meta", "s3", "stripe", "ai", "osm", "openfreemap"] {
            assert!(named(&on, id), "{lang}: {id} is ON and not named");
        }
        let off = render_venue(&venue(), &none, lang);
        for id in ["telegram", "meta", "s3", "stripe", "ai"] {
            assert!(!named(&off, id), "{lang}: {id} is OFF and still named");
        }
        assert!(named(&off, "cloudflare"), "{lang}: Cloudflare hosts every venue");
        let no_tg = render_venue(&venue(), &On { telegram: false, ..all }, lang);
        assert!(!named(&no_tg, "telegram") && named(&no_tg, "meta"), "{lang}: only Telegram went");
    }
}

/// Never shown to customers: nothing of theirs goes there (import-only link,
/// fiscal sending switched off in this build).
#[test]
fn ebills_and_the_tax_authority_are_not_customer_recipients_today() {
    let all = On { telegram: true, meta: true, cloud: true, stripe: true, ai: true };
    let ids: Vec<&str> = recipients(&all).iter().map(|p| p.id).collect();
    assert!(!ids.contains(&"ebills"));
    assert_eq!(ids.contains(&"tax"), crate::fiscal::SEND_ENABLED);
}

/// The numbers are the code's: 30 days (Law 124 Art. 12(4)), the backup window
/// from `cloud::KEEP_WEEKLY_MS`, the Commissioner, the 16-year line, the version.
#[test]
fn the_notice_carries_the_deadline_the_backup_window_the_commissioner_and_its_version() {
    let days = backup_days();
    assert_eq!(days, crate::cloud::KEEP_WEEKLY_MS / (24 * 60 * 60 * 1000) + 1);
    assert!(days <= 30, "a backup copy must not outlive Law 124's 30-day erasure window");
    let on = On { cloud: true, ..On::default() };
    for lang in words::LANGS {
        let h = render_venue(&venue(), &on, lang);
        assert!(h.contains("30"), "{lang}: the 30-day answer");
        assert!(h.contains(&format!(" {days} ")), "{lang}: the backup window {days}");
        assert!(h.contains("idp.al") && h.contains("86"), "{lang}: the Commissioner and Art. 86");
        assert!(h.contains("16") && h.contains("8(6)"), "{lang}: the 16-year line");
        assert!(h.contains(VERSION), "{lang}: the version");
        assert!(h.contains("Sushi Durrës") && h.contains("+355 69 000 0000"), "{lang}: the controller");
        assert!(h.contains(DOWIZ_PRIVACY_EMAIL), "{lang}: dowiz's contact");
        assert!(!h.to_lowercase().contains("lorem"), "{lang}: no placeholder");
        assert!(!h.to_lowercase().contains("quantum"), "{lang}: no post-quantum claim");
        assert!(h.starts_with(&format!("<!doctype html><html lang=\"{lang}\"")));
    }
    // Without a bucket there is no nightly copy to talk about.
    let h = render_venue(&venue(), &On::default(), "en");
    assert!(!h.contains(&format!("at most {days} days")));
}

/// Every customer store in the registry is a row, so a new one appears here
/// without this file changing.
#[test]
fn every_customer_store_is_a_row() {
    let h = render_venue(&venue(), &On::default(), "en");
    let n = registry::stores().filter(|s| s.personal() && s.about(Subject::Customer)).count();
    assert!(n >= 10, "the registry lists {n} customer stores");
    assert_eq!(h.matches("<tr data-store=").count(), n);
    assert!(h.contains("data-store=\"bookings\"") && h.contains("data-store=\"log\""));
}

/// A venue name is escaped, never markup.
#[test]
fn the_venue_name_is_escaped() {
    let v = Venue { name: "<script>x</script>".into(), address: None, phone: None };
    let h = render_venue(&v, &On::default(), "sq");
    assert!(!h.contains("<script>") && h.contains("&lt;script&gt;"));
}

/// Every word exists in every language: no English processor text leaks into
/// the Albanian or Ukrainian page.
#[test]
fn every_language_has_every_processor_and_every_word() {
    for lang in words::LANGS {
        let w = words::words(lang);
        assert_eq!(w.lang, lang);
        for p in PROCESSORS {
            assert!((w.processor)(p.id).is_some(), "{lang}: no text for processor {}", p.id);
        }
        for d in [Data::Name, Data::Phone, Data::Photo, Data::Ip, Data::Wallet] {
            assert!(!(w.data)(d).is_empty());
        }
        assert!(!(w.purpose)(Purpose::Erasure).is_empty() && !(w.basis)(Basis::Consent).is_empty());
        assert_eq!(w.rights.len(), 7);
    }
    assert_eq!(words::words("de").lang, "sq", "an unknown language falls back to Albanian");
}

/// dowiz's own notice lists its controller stores (accounts, waiting list) and
/// no venue's customer store.
#[test]
fn the_platform_notice_is_about_accounts_and_the_waitlist() {
    let h = render_platform("en");
    assert!(h.contains("waiting list") && h.contains("email address"));
    assert!(!h.contains("to prepare and deliver your order"));
}
