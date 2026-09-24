//! PURE. Where an order came from, and what that source means for trust.
//!
//! `channel` is the order's SOURCE; `fulfilment.kind` is how the food reaches
//! the guest. The two are orthogonal (Toast's "order source" vs "dining
//! option", BLUEPRINT-TAX-PRICE-CHANNEL §2.8): a WhatsApp order can be a
//! pickup, an ebills import can be a table.
//!
//! A CLOSED SET, DECIDED BY THE HANDLER FROM THE PRINCIPAL, NEVER FROM THE
//! BODY. The storefront passed the word `"storefront"` to the kernel by hand
//! and the ebills mapper wrote `"ebills"` by hand; the kernel stores any
//! string, so a third route typing the kernel's own test word `"web"` would
//! have filed orders under a source no reader has a branch for.
//! `tools/gates/channel-closed.sh` (G4) keeps these constants the only place
//! the words are spelled.

use serde_json::Value;

/// The venue's own web ordering: a guest, no staff token.
pub const STOREFRONT: &str = "storefront";
/// A round typed in by a signed member of staff or the owner (`placer.rs`).
pub const CONSOLE: &str = "console";
/// The WhatsApp messaging webhook, once it places orders (none today).
pub const WHATSAPP: &str = "whatsapp";
/// The Instagram messaging webhook, once it places orders (none today).
pub const INSTAGRAM: &str = "instagram";
/// A sale imported from the venue's fiscal platform (`ebills::to_order`).
pub const EBILLS: &str = "ebills";

/// Marketplaces (OPERATIONAL-BLIND-SPOTS §2.9, P2-3). Entered by staff from the
/// platform's own tablet (`command::aggregator`) until a partner API exists;
/// no adapter speaks to any of them yet.
pub const WOLT: &str = "wolt";
pub const GLOVO: &str = "glovo";
pub const BABOON: &str = "baboon";

/// Every source an order can have.
pub const ALL: [&str; 8] = [STOREFRONT, CONSOLE, WHATSAPP, INSTAGRAM, EBILLS, WOLT, GLOVO, BABOON];

pub fn known(c: &str) -> bool {
    ALL.contains(&c)
}

/// A channel word this build does not know, or a `channel` that is not a
/// string at all. REFUSED, never read as `storefront`: an unknown source
/// filed as first-party would be priced, fiscalised and counted as ours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown(pub String);

impl std::fmt::Display for Unknown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown channel: {}", self.0)
    }
}

/// The four axes on which a source changes what the platform must do
/// (§2.8's table), as facts rather than prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// The venue's own source, not a marketplace's.
    pub first_party: bool,
    /// dowiz's pricer set the line prices. `false` is what the order records
    /// as `price_trusted: false`: somebody else priced it and dowiz keeps what
    /// the customer paid (P2-3).
    pub priced_by_us: bool,
    /// dowiz is the one that must fiscalise it. `false` for a sale that was
    /// fiscalised before it reached us — pushing it again would double it.
    pub fiscalised_by_us: bool,
    /// The venue holds the customer's contact, not a platform that masks it.
    pub owns_customer: bool,
    /// Commission owed to the source, parts per million (22 % = 220_000).
    /// NEVER netted out of an order's `total`: the total is what the customer
    /// was charged (conservation law 3), and a commission is a cost the
    /// venue settles with the platform.
    pub commission_ppm: u32,
    /// The venue's own couriers carry it when it leaves the building. `false`:
    /// the platform's courier collects it at the counter, so it is entered as a
    /// PICKUP and never reaches this venue's courier pool.
    pub delivered_by_us: bool,
}

const OURS: Profile = Profile {
    first_party: true,
    priced_by_us: true,
    fiscalised_by_us: true,
    owns_customer: true,
    commission_ppm: 0,
    delivered_by_us: true,
};

/// A MARKETPLACE, as far as it is known without a merchant agreement.
///
/// * `priced_by_us: false` — the platform priced it; dowiz keeps what the
///   customer paid (`price_trusted: false`), never recomputes it.
/// * `owns_customer: false` — the platform masks the contact.
/// * `delivered_by_us: false` — the platform's courier collects it.
/// * `fiscalised_by_us: true` — A HYPOTHESIS (TAX §2.8, "not determined"):
///   the venue, as the seller, issues the fiscal invoice unless the platform
///   acts as principal. dowiz fiscalises nothing today, so nothing reads it yet;
///   the merchant agreement decides it.
/// * `commission_ppm: 0` — NOT KNOWN, not "none": 22-35 % are vendor-blog
///   hypotheses and no contract has been read. Nothing computes with it.
const MARKETPLACE: Profile = Profile {
    first_party: false,
    priced_by_us: false,
    fiscalised_by_us: true,
    owns_customer: false,
    commission_ppm: 0,
    delivered_by_us: false,
};

/// The profile of a MEMBER; `None` for anything else. There is no default
/// profile: a source that is not in `ALL` has no trust model to borrow.
pub fn profile(c: &str) -> Option<Profile> {
    match c {
        STOREFRONT | CONSOLE | WHATSAPP | INSTAGRAM => Some(OURS),
        // The venue's OWN till, so first-party and the venue's guest, no
        // commission — but priced by the till and already fiscalised by it.
        EBILLS => Some(Profile { priced_by_us: false, fiscalised_by_us: false, ..OURS }),
        WOLT | GLOVO | BABOON => Some(MARKETPLACE),
        _ => None,
    }
}

/// Is this a marketplace's word — somebody else's customer, priced and
/// delivered by them?
pub fn marketplace(c: &str) -> bool {
    marketplace_word(c).is_some()
}

/// THE SET'S OWN WORD for a marketplace a member of staff picked, or `None`.
/// The aggregator form is the one route whose body names a source, so what it
/// stamps is never the body's string: it is this constant, and only a
/// marketplace's -- a body that says `console` or `storefront` gets `None`.
pub fn marketplace_word(c: &str) -> Option<&'static str> {
    [WOLT, GLOVO, BABOON].into_iter().find(|m| *m == c).filter(|m| profile(m).is_some_and(|p| !p.first_party))
}

/// Who is placing this order, as the handler has already VERIFIED it: a room
/// round signed by staff (`placer::Staffed`) is `console`; anybody else on the
/// public route is a guest on the storefront. The body is never asked.
pub fn for_placement(signed_by_staff: bool) -> &'static str {
    if signed_by_staff {
        CONSOLE
    } else {
        STOREFRONT
    }
}

/// Write the channel onto an envelope, OVERWRITING whatever it carried — a
/// `channel` that rode in on a request is discarded before it is believed
/// (the `unit_price` precedent, `pricing.rs`). Refuses a non-member, so no
/// code path can store a word the readers do not know.
pub fn stamp(envelope: &mut Value, channel: &str) -> Result<(), Unknown> {
    if !known(channel) {
        return Err(Unknown(channel.to_string()));
    }
    if let Some(o) = envelope.as_object_mut() {
        o.insert("channel".into(), Value::from(channel));
        return Ok(());
    }
    Err(Unknown(format!("envelope is not an object (channel {channel})")))
}

/// The channel an order envelope carries.
///
/// ABSENT (OR `null`) MEANS `storefront`, and that is a compatibility rule
/// with a date on it: until 2026-09-23 the only route that appended `Placed`
/// was `storefront::place`, which has passed `"storefront"` to the kernel
/// since before this module; the kernel writes an explicit `null` on an order
/// that never had one (`fold.rs`). So every order that can lack the field is
/// a storefront order.
///
/// AN UNKNOWN WORD IS NOT ABSENCE, and is refused (`Unknown`) rather than
/// read as `storefront` — this is where this module deliberately differs from
/// `fulfilment::of`. An unknown kind can be pushed down the strictest branch;
/// an unknown SOURCE has no safe branch, because `storefront` means "we
/// priced it, we fiscalise it, it is our customer", which is the one claim an
/// unrecognised source must not inherit.
pub fn of(envelope: &Value) -> Result<&'static str, Unknown> {
    match envelope.get("channel") {
        None | Some(Value::Null) => Ok(STOREFRONT),
        Some(Value::String(c)) => ALL.iter().copied().find(|m| m == c).ok_or_else(|| Unknown(c.clone())),
        Some(other) => Err(Unknown(other.to_string())),
    }
}

#[cfg(test)]
mod tests;
