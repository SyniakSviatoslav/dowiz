use super::*;

#[test]
fn settings_survive_the_byte_image() {
    let mut s = Settings::create().expect("create");
    s.set("ai.model", "llama3.2");
    s.set("ai.token", "sk-secret-value");
    let bytes = s.to_bytes().expect("bytes");

    let s = Settings::load(&bytes).expect("load");
    assert_eq!(s.get("ai.model").as_deref(), Some("llama3.2"));
    assert_eq!(s.get("ai.token").as_deref(), Some("sk-secret-value"));
    assert_eq!(s.get("nothing.here"), None);
}

/// The one that matters: a token must never come back out for display.
#[test]
fn a_secret_is_never_rendered() {
    let mut s = Settings::create().expect("create");
    s.set("ai.token", "sk-secret-value");
    s.set("ai.model", "llama3.2");

    let json = s.as_json();
    assert!(!json.contains("sk-secret-value"), "the token leaked: {json}");
    assert!(json.contains("llama3.2"), "non-secrets must still show: {json}");
    assert!(json.contains("set"), "the owner must see that it IS configured: {json}");

    // And the code that CALLS a provider still gets the real thing.
    assert_eq!(s.get("ai.token").as_deref(), Some("sk-secret-value"));
}

/// Secrecy follows the key's shape, so an integration added later cannot be
/// forgotten into a list that was never updated.
#[test]
fn secrecy_is_decided_by_shape_not_by_a_list() {
    for k in [
        "ai.token",
        "social.telegram.token",
        "anything.at.all.key",
        "x.secret",
        "y.password",
        "z.apikey",
    ] {
        assert!(is_secret(k), "{k} must be secret");
    }
    for k in ["ai.model", "ai.endpoint", "ai.enabled", "tokens.count", "keyboard"] {
        assert!(!is_secret(k), "{k} must NOT be treated as a secret");
    }
}

#[test]
fn known_keys_fall_back_to_their_declared_defaults() {
    let mut s = Settings::create().expect("create");
    // EMPTY, not a loopback URL. The declared default was changed when it
    // turned out a Worker can never reach `http://127.0.0.1:11434/v1` --
    // see the comment on the key -- and this assertion kept the old value,
    // so the suite has been red at HEAD ever since. A test that pins a
    // default the code deliberately abandoned is not a regression guard,
    // it is a second opinion nobody asked for.
    assert_eq!(s.known("ai.endpoint"), "", "not configured is the honest default");
    assert_eq!(s.known("ai.model"), "llama3.2");
    assert!(!s.flag("ai.enabled"), "the assistant must be OFF until switched on");

    s.set("ai.model", "qwen2.5");
    assert_eq!(s.known("ai.model"), "qwen2.5");
    s.set("ai.enabled", "1");
    assert!(s.flag("ai.enabled"));
    s.set("ai.enabled", "0");
    assert!(!s.flag("ai.enabled"));
}

#[test]
fn clearing_a_setting_restores_its_default() {
    let mut s = Settings::create().expect("create");
    s.set("ai.model", "qwen2.5");
    s.clear("ai.model");
    assert_eq!(s.get("ai.model"), None);
    assert_eq!(s.known("ai.model"), "llama3.2", "back to the declared default");
    // And the cleared key is not rendered as an empty row.
    assert!(!s.as_json().contains("ai.model"));
}

/// A value with a quote in it must not be able to add a settings field.
#[test]
fn a_hostile_value_cannot_forge_a_setting() {
    let mut s = Settings::create().expect("create");
    s.set("ai.model", r#"x","ai.enabled":"1"#);
    let json = s.as_json();
    assert!(json.contains(r#"\"ai.enabled\""#), "must be escaped, not structural: {json}");
    assert!(!s.flag("ai.enabled"), "the flag must not have been set by a model name");
}

/// Item 2 of BLUEPRINT-TAX-PRICE-CHANNEL: the venue's tax is four DECLARED
/// keys. The key space is closed, so a key that is not here is refused by
/// `POST /api/owner/settings` — which is exactly what `"tax_rate": "0.20"`
/// gets, and what `tax.default_ppm` must not.
#[test]
fn the_tax_keys_are_declared_and_unconfigured_by_default() {
    for k in ["tax.default_ppm", "tax.prices_include", "tax.delivery_fee_ppm", "tax.schedule"] {
        assert!(KNOWN.iter().any(|d| d.key == k), "{k} must be a declared setting");
        assert!(!is_secret(k), "{k} is not a secret");
    }
    assert!(!KNOWN.iter().any(|d| d.key == "tax_rate"), "the float spelling is not a key");
    let s = Settings::create().expect("create");
    // NOT CONFIGURED is the honest default: no rate means no `tax` block, and
    // the fiscal seam refuses such an order by name (`NoTax`), never a zero.
    assert_eq!(s.known("tax.default_ppm"), "");
    assert_eq!(s.known("tax.schedule"), "");
    // Albania and the EU print tax-INCLUSIVE menu prices.
    assert!(s.flag("tax.prices_include"));
}
