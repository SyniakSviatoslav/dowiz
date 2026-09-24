//! The agreement: a hub is refused without it, the version lands on the
//! `loc` record, and the served text is the tree's, in three languages.

use super::*;

/// P9's CHECK: no acceptance, or an old one, is refused; the current is not.
#[test]
fn a_hub_without_the_current_dpa_is_refused() {
    assert!(check(None).is_err());
    assert!(check(Some("")).is_err());
    assert!(check(Some("dpa v0 2026-01-01")).unwrap_err().contains(VERSION));
    assert!(check(Some(VERSION)).is_ok());
    assert!(check(Some(&format!("  {VERSION} "))).is_ok());
}

#[test]
fn the_acceptance_is_stamped_on_the_loc_record_and_read_back() {
    let mut rec = json!({ "id": "sushi-durres", "slug": "sushi-durres", "name": "Sushi" });
    assert_eq!(state(Some(&rec))["current"], json!(false));
    assert_eq!(state(Some(&rec))["accepted"], Value::Null);
    stamp(&mut rec, 1_790_000_000_000, "platform:ops");
    assert_eq!(rec["dpa_version"], json!(VERSION));
    assert_eq!(rec["slug"], json!("sushi-durres"), "the rest of the record is kept");
    let s = state(Some(&rec));
    assert_eq!(s["current"], json!(true));
    assert_eq!(s["accepted"]["by"], json!("platform:ops"));
    assert_eq!(s["accepted"]["atMs"], json!(1_790_000_000_000_i64));
    // An older acceptance is shown, and is not current.
    rec["dpa_version"] = json!("dpa v0 2026-01-01");
    assert_eq!(state(Some(&rec))["current"], json!(false));
    assert_eq!(state(None)["current"], json!(false));
}

/// Each text names its version, both parties' roles, the 30-day and 72-hour
/// clocks, and never claims post-quantum encryption.
#[test]
fn each_text_is_versioned_and_says_what_the_law_needs() {
    for lang in ["sq", "en", "uk"] {
        let t = text(lang);
        assert!(t.contains(VERSION), "{lang}: carries its version");
        assert!(t.contains("26"), "{lang}: cites Law 124/2024 Art. 26");
        assert!(t.contains("30") && t.contains("72"), "{lang}: the clocks");
        assert!(t.contains("privacy@dowiz.org"), "{lang}: the contact");
        let low = t.to_lowercase();
        assert!(!low.contains("quantum") && !low.contains("invest"), "{lang}: no PQ or funding claims");
        for p in crate::privacy::registry::PROCESSORS {
            if p.role == crate::privacy::registry::Role::SubProcessor {
                assert!(t.contains("Cloudflare"), "{lang}: names the sub-processor {}", p.id);
            }
        }
    }
    assert_ne!(text("sq"), text("en"));
    assert_ne!(text("uk"), text("en"));
    assert_eq!(text("xx"), text("sq"));
}

#[test]
fn the_markdown_is_escaped_before_it_is_markup() {
    let h = to_html("# T\n\nA <b>x</b> & y\n\n- one\n- two\n\nend");
    assert_eq!(h, "<h1>T</h1><p>A &lt;b&gt;x&lt;/b&gt; &amp; y</p><ul><li>one</li><li>two</li></ul><p>end</p>");
}
