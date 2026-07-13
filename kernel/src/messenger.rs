//! Messenger deep-link builders — RW-08 (pure string logic → kernel authority).
//!
//! Ports `messenger.ts` (deep-link TG/WA/Viber regex normalize) into Rust.
//! Pure string transform, no DOM, no network. Parity with the TS implementation
//! is the RED→GREEN gate.
//!
//! NOTE: this is contact/link *construction* only — it never sends. Red-line
//! (auth/money) untouched.

/// Strip everything but digits and a leading `+`. Normalizes `+380 (67) 123-45-67`
/// → `+380671234567`.
pub fn normalize_phone(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut digits = String::new();
    for c in raw.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else if c == '+' && out.is_empty() && digits.is_empty() {
            // leading + preserved
        } else if c == '+' {
            // ignore internal + (defensive)
        }
    }
    // If the raw had a leading +, keep it; else assume local.
    if raw.trim_start().starts_with('+') {
        out.push('+');
    }
    out.push_str(&digits);
    out
}

/// Telegram deep link: `https://t.me/<username>` (no @, no spaces).
pub fn telegram_link(username: &str) -> String {
    let u = username.trim_start_matches('@').replace(' ', "");
    format!("https://t.me/{}", u)
}

/// WhatsApp click-to-chat link with an optional pre-filled (URL-encoded) message.
pub fn whatsapp_link(phone: &str, message: &str) -> String {
    let p = normalize_phone(phone);
    if message.is_empty() {
        format!("https://wa.me/{}", p.trim_start_matches('+'))
    } else {
        // RFC 3986-ish encode: spaces→%20, and the common unsafe chars.
        let enc = encode_query(message);
        format!("https://wa.me/{}?text={}", p.trim_start_matches('+'), enc)
    }
}

/// Viber deep link to a phone (call action).
pub fn viber_link(phone: &str) -> String {
    let p = normalize_phone(phone);
    format!("viber://chat?number={}", p.trim_start_matches('+'))
}

/// Minimal query-encoder (no external crate): percent-encode spaces and the
/// characters that break URLs. Sufficient for pre-filled message text.
fn encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            ' ' => out.push_str("%20"),
            ':' => out.push_str("%3A"),
            '/' => out.push_str("%2F"),
            '?' => out.push_str("%3F"),
            '&' => out.push_str("%26"),
            '=' => out.push_str("%3D"),
            '#' => out.push_str("%23"),
            '\n' => out.push_str("%0A"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Phone normalization: keep leading +, digits only.
    #[test]
    fn normalize_phone_strips_formatting() {
        assert_eq!(normalize_phone("+380 (67) 123-45-67"), "+380671234567");
        assert_eq!(normalize_phone("0671234567"), "0671234567");
        assert_eq!(normalize_phone("  +1 555 0100 "), "+15550100");
    }

    // Telegram: strip @ and spaces.
    #[test]
    fn telegram_link_clean() {
        assert_eq!(telegram_link("@courier_x"), "https://t.me/courier_x");
        assert_eq!(telegram_link("courier x"), "https://t.me/courierx");
    }

    // WhatsApp: with and without message; phone has no + in the path.
    #[test]
    fn whatsapp_link_parity() {
        assert_eq!(
            whatsapp_link("+380671234567", ""),
            "https://wa.me/380671234567"
        );
        let with_msg = whatsapp_link("+380671234567", "hello world");
        assert_eq!(with_msg, "https://wa.me/380671234567?text=hello%20world");
    }

    // Viber: scheme + number without +.
    #[test]
    fn viber_link_parity() {
        assert_eq!(
            viber_link("+380671234567"),
            "viber://chat?number=380671234567"
        );
    }

    // Encoder handles unsafe chars.
    #[test]
    fn encode_query_handles_unsafe() {
        assert_eq!(encode_query("a?b&c=d/e"), "a%3Fb%26c%3Dd%2Fe");
        assert_eq!(encode_query("line1\nline2"), "line1%0Aline2");
    }
}
