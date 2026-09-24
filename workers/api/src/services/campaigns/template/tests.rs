//! The template a campaign is sent as: the refusals with their twins, the
//! exact request shape Meta documents, and the refusal at preview/send.

use super::*;
use crate::services::campaigns::campaign::{define, DefIn};
use serde_json::json;

fn t(name: &str, lang: &str, params: &[&str]) -> Template {
    Template { name: name.into(), lang: lang.into(), params: params.iter().map(|p| p.to_string()).collect() }
}

fn def(template: Option<serde_json::Value>) -> Def {
    let mut b = json!({ "name": "A", "text": "Soup is back", "segment": { "kind": "everyone_consented" } });
    if let Some(tp) = template {
        b["template"] = tp;
    }
    define(serde_json::from_value::<DefIn>(b).unwrap(), &[], "o", 1_000).unwrap()
}

#[test]
fn a_well_formed_template_passes_and_each_refusal_names_itself() {
    assert!(check(&t("autumn_soup_2", "sq", &["SOUP10"])).is_ok());
    assert!(check(&t("autumn", "en_US", &[])).is_ok());
    assert!(check(&t("Autumn Soup", "sq", &[])).unwrap_err().contains("template name"));
    assert!(check(&t("", "sq", &[])).is_err());
    assert!(check(&t(&"a".repeat(NAME_MAX + 1), "sq", &[])).is_err());
    assert!(check(&t("a", "english", &[])).unwrap_err().contains("language"));
    assert!(check(&t("a", "en_us", &[])).is_err());
    assert!(check(&t("a", "sq", &["two\nlines"])).unwrap_err().contains("{{1}}"));
    assert!(check(&t("a", "sq", &["ok", "tab\there"])).unwrap_err().contains("{{2}}"));
    assert!(check(&t("a", "sq", &["five     spaces"])).is_err());
    assert!(check(&t("a", "sq", &[" "])).is_err());
    let many: Vec<&str> = vec!["x"; PARAMS_MAX + 1];
    assert!(check(&t("a", "sq", &many)).is_err());
    assert!(check(&t("a", "sq", &vec!["x"; PARAMS_MAX])).is_ok(), "the twin: exactly the limit");
}

/// THE SHAPE META DOCUMENTS (templates/overview, read 2026-09-24), with the
/// envelope `channels::template_body` puts around it.
#[test]
fn the_request_is_the_cloud_api_template_shape() {
    let body = crate::channels::template_body("+16505551234", &object(&t("order_confirmation", "en_US", &["Jessica", "SKBUP2-4CPIG9"])));
    assert_eq!(body, json!({
        "messaging_product": "whatsapp", "recipient_type": "individual", "to": "+16505551234",
        "type": "template",
        "template": { "name": "order_confirmation", "language": { "code": "en_US" },
            "components": [{ "type": "body", "parameters": [
                { "type": "text", "text": "Jessica" }, { "type": "text", "text": "SKBUP2-4CPIG9" } ] }] }
    }));
    // A body with no variables sends no components at all.
    assert_eq!(object(&t("hello", "sq", &[])), json!({ "name": "hello", "language": { "code": "sq" } }));
}

#[test]
fn a_whatsapp_campaign_without_a_template_is_refused_with_the_fix() {
    let bare = def(None);
    let why = required(&bare).unwrap_err();
    assert!(why.contains("WhatsApp Manager") && why.contains("24 hours"), "{why}");
    let named = def(Some(json!({ "name": "autumn_soup", "lang": "sq", "params": ["SOUP10"] })));
    assert_eq!(required(&named).unwrap().name, "autumn_soup");
}

#[test]
fn the_template_is_checked_when_the_campaign_is_defined_and_old_defs_still_read() {
    let b = json!({ "name": "A", "text": "x", "segment": { "kind": "everyone_consented" },
        "template": { "name": "Bad Name", "lang": "sq" } });
    assert!(define(serde_json::from_value::<DefIn>(b).unwrap(), &[], "o", 1).is_err());
    let unknown = json!({ "name": "A", "text": "x", "segment": { "kind": "everyone_consented" },
        "template": { "name": "a", "lang": "sq", "header": "x" } });
    assert!(serde_json::from_value::<DefIn>(unknown).is_err(), "a closed shape");
    // A `def` filed before templates existed parses, as a draft.
    let old = r#"{"id":"c1","name":"n","text":"t","segment":{"kind":"everyone_consented"},"channel":"whatsapp","atMs":1,"by":"o"}"#;
    let d: Def = serde_json::from_str(old).unwrap();
    assert!(d.template.is_none() && required(&d).is_err());
}
