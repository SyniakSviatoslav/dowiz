//! S5 channel attribution (Q-CHANNEL-META) — ports `apps/api/src/lib/channel.ts` VERBATIM.
//!
//! The acquisition channel (how the customer reached `/s/:slug` — QR sticker, NFC tag, Google
//! Business Profile, a social link, …) travels as the `x-channel` REQUEST HEADER (never a
//! `CreateOrderInput` body field — the schema is `.strict()`, so the header sidesteps it, exactly
//! like `x-otp-verified`) and is folded WRITE-ONLY into `orders.metadata.channel`. It is NEVER read
//! by pricing, the state machine, dispatch, or any authz/RLS decision — the owner dashboard's
//! metadata passthrough is the one expected reader.
//!
//! CARRY: there is NO `sales_channel` table (grep-verified) — the DB is frozen; promoting this
//! attribution to a typed entity is a post-rebuild schema-evolution council (Q5a DEFER).

/// The 13-value allowlist (`channel.ts:19-22`). Order preserved for parity/readability.
pub const CHANNEL_ALLOWLIST: [&str; 13] = [
    "web-direct",
    "qr",
    "nfc",
    "gbp",
    "apple-maps",
    "instagram",
    "facebook",
    "whatsapp",
    "telegram-tma",
    "kiosk",
    "widget",
    "agent",
    "other",
];

/// Missing/empty/direct-organic visit (`channel.ts:25`).
pub const DEFAULT_CHANNEL: &str = "web-direct";
/// Anything malformed/unknown/over-length (`channel.ts` fall-through).
pub const OTHER_CHANNEL: &str = "other";

const MAX_HEADER_LEN: usize = 32;

/// Ports `normalizeChannel` (`channel.ts:38-45`) — NEVER throws (a malformed header must never block
/// order creation):
///   - missing / empty                    → `web-direct`
///   - over `MAX_HEADER_LEN` chars         → `other`
///   - whitespace-only (after trim)        → `web-direct`
///   - in the allowlist (trimmed, lowercased) → the matched value
///   - otherwise                           → `other`
///
/// Returns a `&'static str` from the allowlist (or the two sentinels above) so the write-only fold
/// stores a known token, never arbitrary attacker-supplied header bytes.
pub fn normalize_channel(raw: Option<&str>) -> &'static str {
    let Some(raw) = raw else {
        return DEFAULT_CHANNEL;
    };
    if raw.is_empty() {
        return DEFAULT_CHANNEL;
    }
    // Length check is on the RAW header (channel.ts:40, before trim) — an over-long header is `other`.
    if raw.len() > MAX_HEADER_LEN {
        return OTHER_CHANNEL;
    }
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return DEFAULT_CHANNEL;
    }
    CHANNEL_ALLOWLIST
        .into_iter()
        .find(|&c| c == trimmed)
        .unwrap_or(OTHER_CHANNEL)
}

/// Ports `channelFromHeader` (`channel.ts:49-51`): a header sent more than once arrives as multiple
/// values — take the FIRST occurrence, then normalize (`normalizeChannel` handles the rest).
pub fn channel_from_header<'a>(mut values: impl Iterator<Item = &'a str>) -> &'static str {
    normalize_channel(values.next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_is_web_direct() {
        assert_eq!(normalize_channel(None), "web-direct");
        assert_eq!(normalize_channel(Some("")), "web-direct");
    }

    #[test]
    fn allowlisted_case_insensitive_trimmed() {
        assert_eq!(normalize_channel(Some("qr")), "qr");
        assert_eq!(normalize_channel(Some("  QR  ")), "qr");
        assert_eq!(normalize_channel(Some("Telegram-TMA")), "telegram-tma");
        assert_eq!(normalize_channel(Some("apple-maps")), "apple-maps");
    }

    #[test]
    fn whitespace_only_is_web_direct() {
        assert_eq!(normalize_channel(Some("   ")), "web-direct");
    }

    #[test]
    fn unknown_is_other() {
        assert_eq!(normalize_channel(Some("myspace")), "other");
    }

    #[test]
    fn over_length_is_other() {
        let long = "a".repeat(33);
        assert_eq!(normalize_channel(Some(&long)), "other");
    }

    #[test]
    fn exactly_max_len_unknown_is_still_other_not_length_reject() {
        // 32 chars: passes the length gate, then fails the allowlist → other (not a different path).
        let at_max = "z".repeat(32);
        assert_eq!(normalize_channel(Some(&at_max)), "other");
    }

    #[test]
    fn header_takes_first_occurrence() {
        let values = ["qr", "nfc"];
        assert_eq!(channel_from_header(values.into_iter()), "qr");
    }

    #[test]
    fn header_absent_is_web_direct() {
        let empty: [&str; 0] = [];
        assert_eq!(channel_from_header(empty.into_iter()), "web-direct");
    }

    /// The returned token is ALWAYS one of the allowlist sentinels — never arbitrary header bytes
    /// (the write-only fold stores a known enum value).
    #[test]
    fn output_is_always_an_allowlist_token() {
        for input in ["qr", "MYSPACE", "", "   ", &"x".repeat(99), "nfc"] {
            let out = normalize_channel(Some(input));
            assert!(
                CHANNEL_ALLOWLIST.contains(&out),
                "output {out} not in allowlist for input {input:?}"
            );
        }
        assert!(CHANNEL_ALLOWLIST.contains(&normalize_channel(None)));
    }
}
