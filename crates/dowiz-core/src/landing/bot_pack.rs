//! LANDING bot-pack (P73 M4) — the marketing/product-schema sibling to P69's catalog pack.
//!
//! Pure `no_std` string generation: `robots.txt`, `sitemap.xml`, Open Graph tags,
//! `web_manifest.json`, `llms.txt`, and schema.org JSON-LD for dowiz.org. Ranked per
//! R1 §7: JSON-LD FIRST (load-bearing AEO substrate), `llms.txt` a forward-looking
//! secondary extra. Generated from FIXED dowiz.org facts, NOT from any catalog (there
//! is none — §16.21 anti-scope §2.2).
//!
//! Anti-scope teeth (§5.1 / §6 not-done clause): `build_landing_bot_pack` takes ONLY
//! `canonical_url` / `hub_source_url` — there is NO parameter through which a vendor
//! could enter the sitemap or JSON-LD. "dowiz.org listed a vendor" is a type-level-
//! unreachable state, falsified by `sitemap_has_no_vendor_slugs` + the
//! `landing_no_vendor_catalog` grep-gate. The schema `@type` is `WebSite` +
//! `Organization` + `SoftwareApplication` for the OPEN hub software — NEVER
//! `Restaurant`/`Menu`/`Offer` (there is no menu on dowiz.org).
//!
//! The JSON-LD is emitted through the hand-rolled `crate::json::Value` (not serde_json),
//! so this module compiles in the no_std core without the `json-api` feature.

use alloc::string::String;
use alloc::vec::Vec;

use crate::json::Value;

/// Build an insertion-ordered JSON object from `(&str, Value)` pairs.
fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (String::from(k), v))
            .collect(),
    )
}

/// The fixed, const set of dowiz.org's OWN pages. fed to `sitemap_xml` — zero
/// `/s/:slug` / vendor entries by construction (the structural anti-scope).
pub const LANDING_OWN_PAGES: &[&str] = &[
    "https://dowiz.org/",
    "https://dowiz.org/install",
    "https://dowiz.org/github",
];

/// `robots.txt` for dowiz.org. `Sitemap:` points at the landing's own sitemap only.
pub fn robots_txt(sitemap_url: &str) -> String {
    let mut s = String::from("User-agent: *\nAllow: /\nSitemap: ");
    s.push_str(sitemap_url);
    s.push('\n');
    s
}

/// `sitemap.xml` from a fixed set of URLs. The landing feeds it ONLY `LANDING_OWN_PAGES`
/// (see `build_landing_bot_pack`) — never a vendor-slug set.
pub fn sitemap_xml(urls: &[String]) -> String {
    let mut body = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for u in urls {
        body.push_str("  <url><loc>");
        body.push_str(u);
        body.push_str("</loc></url>\n");
    }
    body.push_str("</urlset>\n");
    body
}

/// Open Graph tags for the product page (link-unfurl facts for the demo/landing).
pub fn open_graph_tags(canonical_url: &str, hub_source_url: &str) -> String {
    let mut s = String::from(
        "<meta property=\"og:type\" content=\"website\" />\n<meta property=\"og:title\" content=\"dowiz — sovereign delivery infra for venue owners\" />\n<meta property=\"og:url\" content=\"",
    );
    s.push_str(canonical_url);
    s.push_str("\" />\n<meta property=\"og:description\" content=\"Self-hostable, open-source delivery infrastructure. Claim your own hub.\" />\n<meta property=\"og:see_also\" content=\"");
    s.push_str(hub_source_url);
    s.push_str("\" />\n");
    s
}

/// `manifest.json` — installability facts for the product page (the "install the app" CTA).
pub fn web_manifest(canonical_url: &str, hub_source_url: &str) -> String {
    let mut s = String::from("{\n  \"name\": \"dowiz\",\n  \"short_name\": \"dowiz\",\n  \"start_url\": \"");
    s.push_str(canonical_url);
    s.push_str("/\",\n  \"display\": \"standalone\",\n  \"description\": \"Sovereign, open-source delivery infrastructure for venue owners.\",\n  \"related_applications\": [{\"platform\": \"web\", \"url\": \"");
    s.push_str(hub_source_url);
    s.push_str("\"}]\n}\n");
    s
}

/// `llms.txt` — the forward-looking extra (R1 §7): a routing file for agents, NOT a
/// crawlability bet. Curates the GitHub source + install page. May be empty + still valid.
pub fn llms_txt(canonical_url: &str, hub_source_url: &str) -> String {
    let mut s = String::from(
        "# dowiz\n\n> Sovereign, open-source delivery infrastructure for venue owners.\n\n- Landing: ",
    );
    s.push_str(canonical_url);
    s.push_str("/\n- Install: ");
    s.push_str(canonical_url);
    s.push_str("/install\n- Source (AGPLv3): ");
    s.push_str(hub_source_url);
    s.push('\n');
    s
}

/// schema.org JSON-LD for dowiz.org — the load-bearing AEO substrate (R1 §7). The
/// `@type` set is `WebSite` + `Organization` + `SoftwareApplication` (the OPEN hub
/// software). NEVER `Restaurant`/`Menu`/`Offer` (§2.2). Emitted through the hand-rolled
/// `crate::json::Value` (compact, valid RFC 8259) so the schema-type gate
/// (`landing_jsonld_is_software_not_restaurant`) can parse it.
pub fn landing_jsonld(canonical_url: &str, hub_source_url: &str) -> String {
    let graph = Value::Array(vec![
        obj(vec![
            ("@type", Value::from("WebSite")),
            ("@id", Value::from(canonical_url)),
            ("name", Value::from("dowiz")),
            ("url", Value::from(canonical_url)),
            (
                "about",
                Value::from(
                    "Sovereign, open-source delivery infrastructure for venue owners.",
                ),
            ),
        ]),
        obj(vec![
            ("@type", Value::from("Organization")),
            ("name", Value::from("dowiz")),
            ("url", Value::from(canonical_url)),
            ("sameAs", Value::Array(vec![Value::from(hub_source_url)])),
        ]),
        obj(vec![
            ("@type", Value::from("SoftwareApplication")),
            ("name", Value::from("dowiz hub")),
            ("applicationCategory", Value::from("BusinessApplication")),
            ("operatingSystem", Value::from("self-hosted")),
            (
                "offers",
                obj(vec![
                    ("@type", Value::from("Offer")),
                    ("price", Value::from("0")),
                    ("priceCurrency", Value::from("USD")),
                    (
                        "description",
                        Value::from("Open-source (AGPLv3) hub software — free to self-host."),
                    ),
                ]),
            ),
            ("codeRepository", Value::from(hub_source_url)),
            ("url", Value::from(hub_source_url)),
        ]),
    ]);
    let doc = obj(vec![
        ("@context", Value::from("https://schema.org")),
        ("@graph", graph),
    ]);
    doc.to_string()
}

/// The landing pack — a SEPARATE, thinner artifact than P69's `BotPack` (NO catalog
/// fields). `sitemap_xml` is fed dowiz.org's OWN pages ONLY (a fixed const set), never
/// a vendor-slug set. The signature takes `canonical_url`/`hub_source_url` ONLY — there
/// is no parameter through which a vendor could enter (the anti-scope is by absence).
#[derive(Clone, PartialEq, Debug)]
pub struct LandingBotPack {
    pub robots_txt: String,
    pub sitemap_xml: String,
    pub landing_jsonld: String,
    pub open_graph: String,
    pub web_manifest: String,
    pub llms_txt: String,
}

/// Build the landing bot-pack from fixed dowiz.org facts.
pub fn build_landing_bot_pack(canonical_url: &str, hub_source_url: &str) -> LandingBotPack {
    let sitemap_url = format!("{canonical_url}/sitemap.xml");
    let own_pages: Vec<String> = LANDING_OWN_PAGES
        .iter()
        .map(|s| String::from(*s))
        .collect();
    LandingBotPack {
        robots_txt: robots_txt(&sitemap_url),
        sitemap_xml: sitemap_xml(&own_pages),
        landing_jsonld: landing_jsonld(canonical_url, hub_source_url),
        open_graph: open_graph_tags(canonical_url, hub_source_url),
        web_manifest: web_manifest(canonical_url, hub_source_url),
        llms_txt: llms_txt(canonical_url, hub_source_url),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::landing::HUB_SOURCE_URL;
    use crate::json::parse;

    /// The emitted JSON-LD `@type` set is `{WebSite, Organization, SoftwareApplication}`
    /// and contains NO `Restaurant`/`Menu`/`Offer` node (the schema-type gate — §4.4 / §6
    /// not-done clause). Adversarial: a hand-edited `Restaurant` node would make this RED.
    #[test]
    fn landing_jsonld_is_software_not_restaurant() {
        let jld = landing_jsonld("https://dowiz.org", HUB_SOURCE_URL);
        let v = parse(&jld).expect("valid JSON-LD");
        let types: Vec<String> = v
            .get("@graph")
            .and_then(|g| g.as_array())
            .expect("graph array")
            .iter()
            .map(|n| {
                n.get("@type")
                    .and_then(|t| t.as_str())
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert!(types.contains(&"WebSite".to_string()));
        assert!(types.contains(&"Organization".to_string()));
        assert!(types.contains(&"SoftwareApplication".to_string()));
        // The load-bearing anti-scope: NO catalog/restaurant schema on dowiz.org.
        assert!(
            !types.iter().any(|t| {
                matches!(
                    t.as_str(),
                    "Restaurant" | "Menu" | "MenuItem" | "FoodEstablishment"
                )
            }),
            "dowiz.org JSON-LD must NOT be a Restaurant/Menu schema: {types:?}"
        );
        // No restaurant/menu node anywhere in the document (defense in depth on the
        // serialized text — catches a hand-edit that a typed test might miss).
        assert!(
            !jld.contains("Restaurant") && !jld.contains("\"Menu\""),
            "serialized JSON-LD must contain no Restaurant/Menu token"
        );
    }

    /// The emitted `sitemap.xml` matches only dowiz.org's own fixed pages and contains
    /// ZERO `/s/` vendor entries (the falsifiable anti-scope test — §6 not-done clause).
    #[test]
    fn sitemap_has_no_vendor_slugs() {
        let pack = build_landing_bot_pack("https://dowiz.org", HUB_SOURCE_URL);
        assert!(
            !pack.sitemap_xml.contains("/s/"),
            "landing sitemap must contain no /s/ vendor entries: {}",
            pack.sitemap_xml
        );
        // Every <loc> is one of dowiz.org's own fixed pages.
        for m in pack.sitemap_xml.split("<loc>").skip(1) {
            let url = m.split("</loc>").next().unwrap();
            assert!(
                url.starts_with("https://dowiz.org/"),
                "unexpected sitemap entry (vendor leak?): {url}"
            );
        }
    }

    /// The pack is valid with JSON-LD present even if `llms.txt` is empty (JSON-LD is the
    /// load-bearing one; `llms.txt` is a secondary extra — R1 §7).
    #[test]
    fn llms_txt_is_secondary() {
        let pack = build_landing_bot_pack("https://dowiz.org", HUB_SOURCE_URL);
        assert!(
            !pack.landing_jsonld.is_empty(),
            "JSON-LD must be present (load-bearing)"
        );
        // llms.txt MAY legitimately be empty; the pack is still complete.
        let empty_llms = LandingBotPack {
            llms_txt: String::new(),
            ..pack
        };
        assert!(empty_llms.llms_txt.is_empty());
        assert!(!empty_llms.landing_jsonld.is_empty());
    }
}
