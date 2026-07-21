//! P48-INTAKE Phase 1 — hub intake port: channel-agnostic inbound-message vocabulary.
//!
//! This is the **zero-I/O firewall** between external channel adapters and the
//! kernel's order-placement pipeline. Provider payload types (Telegram `Update`,
//! Meta `WebhookEntry`, etc.) die at the adapter boundary and never reach here.
//!
//! Design mirrors `engine/src/intent.rs` — many input shapes, one downstream
//! vocabulary — enforced by the same compile-firewall and grep-gate discipline:
//! no provider type name may appear outside the adapter crate.
//!
//! The canonical vocabulary comes from `BLUEPRINT-P48-owner-hub-surface.md` §2
//! and `BLUEPRINT-P48-INTAKE-2026-07-20.md` §3 — adopted verbatim, not invented.
//!
//! Guard: this is NOT `kernel/src/intake.rs` (unrelated constraint compiler —
//! naming collision only, fact 0.10 in the blueprint).

use std::fmt;

/// Channel identifier constant — the canonical string table for `Order.channel`
/// (§4.5 in the blueprint). Pure metadata the kernel never branches on; the intake
/// layer is the ONLY writer, from this table, so grep and analytics have one
/// spelling per channel.
pub mod channel_str {
    pub const TELEGRAM: &str = "telegram";
    pub const WEB: &str = "web";
    pub const WHATSAPP: &str = "whatsapp";
    pub const SIMPLEX: &str = "simplex";
    pub const INSTAGRAM: &str = "instagram";
    pub const FACEBOOK: &str = "facebook";
}

/// A single already-received inbound message, channel-agnostic.
/// Provider-specific fields (bot token, webhook headers, message type)
/// are consumed at the adapter edge and never reach this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundMessage {
    /// The venue/hub this message targets (the hub's own id).
    pub venue_id: String,
    /// Canonical channel string from [`channel_str`] — one spelling per channel.
    pub channel: String,
    /// Opaque sender address (Telegram chat_id, WhatsApp wa_id, etc.).
    /// Never merged across channels (omnichannel-inbox law — §2 in the blueprint).
    pub sender: String,
    /// Provider-assigned message id — the per-channel dedup key.
    pub provider_msg_id: String,
    /// The message text body (plain text from the customer).
    pub text: String,
    /// Unix timestamp in milliseconds (provider-supplied).
    pub unix_ms: u64,
}

/// The closed set of intent-classification outcomes.
/// Typed uncertainty, never a silent guess — `Ambiguous` carries the reason,
/// `NotAnOrder` means the message did not parse as an order request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentOutcome {
    /// A fully resolved order intent — prices ABSENT by design; the menu fold
    /// (`compute_order_total`) is the only price authority.
    Order(OrderIntent),
    /// Partially parseable — unknown item, no quantity, etc. The hub owner
    /// resolves this in their inbox pane. AI-assist only consumes `Ambiguous`
    /// outcomes under P41's `AiMode::Off`-works invariant (§8.7 in the blueprint).
    Ambiguous(Ambiguity),
    /// The message is not an order request at all (greeting, complaint, etc.).
    NotAnOrder,
}

/// A resolved order intent — prices ABSENT by design. A customer message cannot
/// name a price; the menu fold (`compute_order_total`, `kernel/src/domain.rs`)
/// is the only price authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIntent {
    pub venue_id: String,
    pub items: Vec<OrderLine>,
    /// Free-form delivery address text (never parsed into coordinates here).
    pub delivery_addr: Option<String>,
    /// Channel-specific reply reference (for the confirmation reply on the same channel).
    pub reply_to: Option<String>,
}

/// A single line in an order intent — the item name and quantity, no price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderLine {
    /// Menu item name as the customer typed it (normalized by the parser).
    pub item_name: String,
    /// Requested quantity (always > 0 if parsed).
    pub quantity: u32,
}

/// Why an `Ambiguous` classification was returned — the owner-facing hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ambiguity {
    /// No recognized item name in the message.
    UnknownItem,
    /// Item recognized but quantity is missing or zero.
    MissingQuantity,
    /// Multiple different items with no clear grouping.
    MultipleItemsAmbiguous,
    /// Other unclassifiable ambiguity (free-form reason for diagnostics).
    Other(String),
}

impl fmt::Display for Ambiguity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ambiguity::UnknownItem => write!(f, "unknown item"),
            Ambiguity::MissingQuantity => write!(f, "missing quantity"),
            Ambiguity::MultipleItemsAmbiguous => write!(f, "multiple items, unclear grouping"),
            Ambiguity::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Pure, deterministic, menu-anchored intent parser. Total — never fails;
/// classifies every input into exactly one `IntentOutcome`.
///
/// The menu reference is passed in as a slice of known item names; the parser
/// matches against this reference set, never against a network-fetched catalog.
/// This keeps the trust boundary clean: the parser is a pure function of
/// `(text, menu_names)`, no I/O, no side effects.
pub trait IntentParser {
    /// Classify an inbound message text against a known menu. Returns the
    /// `IntentOutcome` that best describes the customer's intent.
    fn parse(&self, text: &str, menu_names: &[&str]) -> IntentOutcome;
}

/// A simple keyword-matching intent parser — the default implementation.
/// Matches item names from the menu and extracts a leading quantity.
///
/// This is intentionally minimal; AI-assist (P41) can provide a richer parser
/// that consumes `Ambiguous` outcomes, but the fallback must always work
/// deterministically without any model.
pub struct KeywordParser;

impl IntentParser for KeywordParser {
    fn parse(&self, text: &str, menu_names: &[&str]) -> IntentOutcome {
        let text_lower = text.trim().to_lowercase();
        if text_lower.is_empty() {
            return IntentOutcome::NotAnOrder;
        }

        // Try to extract a leading quantity (e.g. "2 sushi" → quantity=2, item="sushi").
        let (quantity, remainder) = extract_quantity(&text_lower);

        // Find which menu items appear in the remainder text.
        let mut matched: Vec<&str> = Vec::new();
        for name in menu_names {
            if remainder.contains(&name.to_lowercase()) {
                matched.push(name);
            }
        }

        match matched.len() {
            0 => {
                if quantity > 0 {
                    // Had a number but no recognized item → UnknownItem, not NotAnOrder.
                    IntentOutcome::Ambiguous(Ambiguity::UnknownItem)
                } else {
                    IntentOutcome::NotAnOrder
                }
            }
            1 => {
                if quantity == 0 {
                    IntentOutcome::Ambiguous(Ambiguity::MissingQuantity)
                } else {
                    IntentOutcome::Order(OrderIntent {
                        venue_id: String::new(), // filled by the intake service
                        items: vec![OrderLine {
                            item_name: matched[0].to_string(),
                            quantity,
                        }],
                        delivery_addr: None,
                        reply_to: None,
                    })
                }
            }
            _ => {
                // Multiple items matched — ambiguous unless we can disambiguate
                // (e.g. all the same item from repeated mentions).
                if quantity > 0 && matched.windows(2).all(|w| w[0] == w[1]) {
                    IntentOutcome::Order(OrderIntent {
                        venue_id: String::new(),
                        items: vec![OrderLine {
                            item_name: matched[0].to_string(),
                            quantity,
                        }],
                        delivery_addr: None,
                        reply_to: None,
                    })
                } else {
                    IntentOutcome::Ambiguous(Ambiguity::MultipleItemsAmbiguous)
                }
            }
        }
    }
}

/// Extract a leading quantity from text. Returns `(quantity, remainder)`.
/// Handles: "2 sushi", "two sushi", "10x white monster", "5 white monsters".
fn extract_quantity(text: &str) -> (u32, &str) {
    let trimmed = text.trim_start();

    // Try numeric: "2 sushi..." or "10x white monster..."
    let digit_end = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(trimmed.len());
    if digit_end > 0 {
        if let Ok(n) = trimmed[..digit_end].parse::<u32>() {
            let rest = trimmed[digit_end..].trim_start().trim_start_matches('x').trim_start();
            return (n, rest);
        }
    }

    // Try word form: "two sushi..."
    const WORD_NUMS: &[(&str, u32)] = &[
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
    ];
    for &(word, n) in WORD_NUMS {
        if trimmed.starts_with(word) {
            let after = trimmed[word.len()..].trim_start();
            return (n, after);
        }
    }

    (0, trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── KeywordParser unit tests ──────────────────────────────────────────

    const MENU: &[&str] = &["sushi", "white monster", "varenyky", "juice"];

    #[test]
    fn happy_path_single_item() {
        let out = KeywordParser.parse("2 sushi", MENU);
        match out {
            IntentOutcome::Order(o) => {
                assert_eq!(o.items.len(), 1);
                assert_eq!(o.items[0].item_name, "sushi");
                assert_eq!(o.items[0].quantity, 2);
            }
            _ => panic!("expected Order"),
        }
    }

    #[test]
    fn quantity_word_form() {
        let out = KeywordParser.parse("three white monsters", MENU);
        match out {
            IntentOutcome::Order(o) => {
                assert_eq!(o.items[0].item_name, "white monster");
                assert_eq!(o.items[0].quantity, 3);
            }
            _ => panic!("expected Order"),
        }
    }

    #[test]
    fn no_quantity_is_ambiguous() {
        let out = KeywordParser.parse("sushi", MENU);
        assert!(matches!(out, IntentOutcome::Ambiguous(Ambiguity::MissingQuantity)));
    }

    #[test]
    fn unknown_item_is_not_an_order() {
        let out = KeywordParser.parse("hello", MENU);
        assert_eq!(out, IntentOutcome::NotAnOrder);
    }

    #[test]
    fn number_but_unknown_item() {
        let out = KeywordParser.parse("5 kebab", MENU);
        assert!(matches!(out, IntentOutcome::Ambiguous(Ambiguity::UnknownItem)));
    }

    #[test]
    fn multiple_different_items_ambiguous() {
        let out = KeywordParser.parse("1 sushi 2 white monsters", MENU);
        assert!(matches!(out, IntentOutcome::Ambiguous(Ambiguity::MultipleItemsAmbiguous)));
    }

    #[test]
    fn case_insensitive() {
        let out = KeywordParser.parse("2 SUSHI", MENU);
        match out {
            IntentOutcome::Order(o) => {
                assert_eq!(o.items[0].item_name, "sushi");
                assert_eq!(o.items[0].quantity, 2);
            }
            _ => panic!("expected Order, got {out:?}"),
        }
    }

    #[test]
    fn empty_text_is_not_an_order() {
        assert_eq!(KeywordParser.parse("", MENU), IntentOutcome::NotAnOrder);
        assert_eq!(KeywordParser.parse("   ", MENU), IntentOutcome::NotAnOrder);
    }

    // ── extract_quantity edge cases ───────────────────────────────────────

    #[test]
    fn extract_numeric_quantity() {
        assert_eq!(extract_quantity("2 sushi"), (2, "sushi"));
        assert_eq!(extract_quantity("10x white monster"), (10, "white monster"));
        assert_eq!(extract_quantity("0 sushi"), (0, "sushi")); // zero parsed but no early return
    }

    #[test]
    fn extract_word_quantity() {
        assert_eq!(extract_quantity("two sushi"), (2, "sushi"));
        assert_eq!(extract_quantity("twelve fries"), (12, "fries"));
    }

    #[test]
    fn no_quantity_found() {
        assert_eq!(extract_quantity("sushi"), (0, "sushi"));
        assert_eq!(extract_quantity("order please"), (0, "order please"));
    }

    // ── channel_str constistency ──────────────────────────────────────────

    #[test]
    fn channel_constants_are_lowercase_ascii() {
        for ch in [
            channel_str::TELEGRAM,
            channel_str::WEB,
            channel_str::WHATSAPP,
            channel_str::SIMPLEX,
            channel_str::INSTAGRAM,
            channel_str::FACEBOOK,
        ] {
            assert!(
                ch.chars().all(|c| c.is_ascii_lowercase()),
                "channel constant must be lowercase ASCII: {ch:?}"
            );
        }
    }
}
