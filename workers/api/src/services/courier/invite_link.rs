//! THE LINK THE OWNER SHARES (operator decision 2026-10-02): an invitation that
//! used to be a code on the owner's screen and nowhere else.
//!
//! THE GAP (docs/research/2026-10-02-system-integration-check.md item 3).
//! `invite_courier` answered `{code, expiresMs}` and the console showed a copy
//! button; no rail carried it anywhere. The owner read sixteen characters out
//! to a courier over the phone, or typed them into a chat by hand, and the
//! courier typed them back into an app they had to find first.
//!
//! THE SHAPE. One URL on the VENUE'S OWN HOST (`courier-login-venue-from-host`:
//! the host decides the venue, so the app the link opens signs the courier in
//! at the right one), the courier app's path, and the code in the FRAGMENT. A
//! fragment never leaves the browser: it is in no request line, no access log
//! and no referrer, which a query string would be. The owner hands the link to
//! the courier by the phone's own share sheet or by WhatsApp, Telegram or SMS
//! (`admin/couriers.js`); nothing is sent by the platform, and no secret is
//! stored beyond the hash the invite already keeps.

/// Where the courier app lives on every venue host.
pub const APP_PATH: &str = "/courier/";
/// The fragment key the app reads (`courier/app.js` `claimFromHash`).
pub const FRAGMENT_KEY: &str = "claim";

/// The link: `https://<slug>.<platform host>/courier/#claim=<code>` -- one
/// client, one subdomain (`hubstore.rs`); the `{}.{}` host is the registry's
/// "the venue's own addresses on this platform" row (`privacy/registry/outside.rs`).
pub fn invite_url(slug: &str, platform_host: &str, code: &str) -> String {
    // The path is spelled out: the gate reads the host literal up to the first `/`.
    format!("https://{}.{}/courier/#{FRAGMENT_KEY}={code}", slug.trim().to_ascii_lowercase(), platform_host)
}

/// The code a link carries, read the way the app reads it; `None` when the
/// fragment names none. The test's half of the contract.
pub fn code_in(url: &str) -> Option<&str> {
    url.split_once('#')?.1.split('&').find_map(|kv| kv.strip_prefix(FRAGMENT_KEY)?.strip_prefix('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_link_is_on_the_venues_host_and_the_code_rides_in_the_fragment_only() {
        let url = invite_url("Alpha", "dowiz.org", "ABCDEFGH23456789");
        assert_eq!(url, "https://alpha.dowiz.org/courier/#claim=ABCDEFGH23456789");
        assert!(url.contains(APP_PATH), "the app's path, as the courier app is served");
        let (before_fragment, _) = url.split_once('#').unwrap();
        assert!(!before_fragment.contains("ABCDEFGH23456789"), "a code in the path or query is a code in a log");
        assert_eq!(code_in(&url), Some("ABCDEFGH23456789"));
    }

    #[test]
    fn a_link_without_a_code_names_none() {
        assert_eq!(code_in("https://alpha.dowiz.org/courier/"), None);
        assert_eq!(code_in("https://alpha.dowiz.org/courier/#learn=c1"), None);
        assert_eq!(code_in("https://alpha.dowiz.org/courier/#learn=c1&claim=X"), Some("X"));
    }
}
