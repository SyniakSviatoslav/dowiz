//! THE PERSONAL-DATA REGISTRY — every place this platform keeps something
//! about a person, as a table in code (P1, `BLUEPRINT-GDPR-AND-MCP-2026-09-24`
//! §4.1).
//!
//! A TABLE, NOT A DOCUMENT, because a document is right on the day it is
//! written and a table is checked on every commit: `tools/gates/personal-data.sh`
//! fails when an `IMAGE_*`/`K_*` constant, an `https://` host in this crate, a
//! host in the storefront's CSP or a `dw_*`/`dowiz.*` browser key appears
//! without a row here. The privacy notice (`notice.rs`), the processor register
//! (`docs/privacy/PROCESSORS.md`) and the DPA's annex are read from the same
//! rows, so a new store cannot be kept quietly: it either has a row, and the
//! notice says so, or the gate is red.
//!
//! EVERY ROW SAYS WHAT IS TRUE AT HEAD, not what is planned. A store no erasure
//! reaches yet says `Eraser::Missing` and names the roadmap row that will build
//! it; a store kept on purpose says `Retain` and must carry its reason
//! (`tests.rs` refuses an empty one).
//!
//! The rows live in three files beside this one so each stays readable:
//! `registry/venue.rs` (one venue's Durable Object), `registry/platform.rs` (the
//! `__platform` object) and `registry/outside.rs` (hosts, processors, browser
//! keys).

mod outside;
mod platform;
mod venue;

pub use outside::{BROWSER, PROCESSORS};
/// Read as text by `tools/gates/personal-data.sh`, and by the tests.
#[cfg(test)]
pub use outside::HOSTS;

/// What kind of thing is kept. The notice prints one plain word per class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Data {
    Name,
    Phone,
    Address,
    Coordinates,
    OrderContent,
    Note,
    Messages,
    Booking,
    Payment,
    Wallet,
    Consent,
    CustomerCard,
    Email,
    Password,
    Session,
    CourierPosition,
    StaffId,
    Device,
    Ip,
    Photo,
}

/// Whose data it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    Customer,
    Courier,
    Staff,
    Owner,
    Prospect,
}

/// Why it is kept. One purpose per row; the notice groups rows by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Purpose {
    Order,
    Accounting,
    Booking,
    CustomerCare,
    TableChat,
    Kitchen,
    Crm,
    ConsentProof,
    Marketing,
    StoredValue,
    Dispatch,
    Security,
    ExactlyOnce,
    Account,
    Prospect,
    Erasure,
    Operations,
}

/// The lawful basis, Law 124/2024 Art. 7(1) (GDPR Art. 6(1)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// Art. 7(1)(b): needed to do what the person asked for.
    Contract,
    /// Art. 7(1)(c): a law makes the controller keep it.
    LegalObligation,
    /// Art. 7(1)(dh): the controller's legitimate interest, weighed.
    LegitimateInterest,
    /// Art. 7(1)(a) with Art. 8: a separate, unticked, withdrawable box.
    Consent,
    /// Holds nothing about a person.
    NotPersonal,
}

/// How long it is kept, as the code keeps it today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retention {
    /// A constant in the tree bounds it: milliseconds, and where it lives.
    Ms(i64, &'static str),
    /// Replaced in place; only the latest value exists.
    Latest,
    /// Gone once its job is done (delivered, expired, used).
    UntilDone(&'static str),
    /// No time limit is enforced yet. The string says what bounds it today and
    /// which roadmap row will add one.
    NoLimitYet(&'static str),
}

/// What reaches it when a person is erased.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eraser {
    /// Redacted in place by the named function; the record stays, the person goes.
    Redact(&'static str),
    /// Removed by the named function.
    Remove(&'static str),
    /// Kept on purpose, and why (a law, the accounts). Never empty.
    Retain(&'static str),
    /// Ages out on its own; the string is the bound.
    Expires(&'static str),
    /// Nothing reaches it yet. The string names the roadmap row that will.
    Missing(&'static str),
    /// Holds nothing about a person, and why.
    NotPersonal(&'static str),
}

/// What an export of one person reads from it (P5 walks this).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exporter {
    /// No export reads it yet; the roadmap row that will.
    Missing(&'static str),
    /// Nothing to export.
    NotPersonal,
}

/// Which object the image lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    /// One per venue (`HubImages`).
    Venue,
    /// The one `__platform` object.
    Platform,
}

/// One store: an image, the record kinds inside it, and everything the law asks.
#[derive(Clone, Copy, Debug)]
pub struct Store {
    pub image: &'static str,
    /// Read as text by the gate (and by the tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub kinds: &'static [&'static str],
    pub home: Home,
    pub holds: &'static [Data],
    pub subjects: &'static [Subject],
    pub purpose: Purpose,
    pub basis: Basis,
    pub retention: Retention,
    pub erase: Eraser,
    /// P5's export walks this; until it lands only the tests read it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub export: Exporter,
}

impl Store {
    pub fn personal(&self) -> bool {
        self.basis != Basis::NotPersonal && !self.holds.is_empty()
    }
    pub fn about(&self, s: Subject) -> bool {
        self.subjects.contains(&s)
    }
}

/// Every store, venue ones first.
pub fn stores() -> impl Iterator<Item = &'static Store> {
    venue::STORES.iter().chain(platform::STORES.iter())
}

/// The row for an image name, if there is one.
#[cfg(test)]
pub fn store(image: &str) -> Option<&'static Store> {
    stores().find(|s| s.image == image)
}

/// A party outside this Worker that receives something.
#[derive(Clone, Copy, Debug)]
pub struct Processor {
    pub id: &'static str,
    pub name: &'static str,
    pub receives: &'static [Data],
    /// P10's records of processing group by it; the tests read it today.
    #[cfg_attr(not(test), allow(dead_code))]
    pub role: Role,
    /// When it receives anything for a venue.
    pub switch: Switch,
    /// Where, as far as its own terms say.
    pub location: &'static str,
    /// What makes the transfer lawful (Law 124 Art. 40-41).
    pub safeguard: &'static str,
    /// Its own data-processing terms.
    pub terms: &'static str,
}

/// Its relation to the venue (the controller for customer data).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// dowiz's sub-processor (DPA §5).
    SubProcessor,
    /// A processor the venue chose and contracts itself.
    VenueProcessor,
    /// Receives under its own terms as a controller (payments).
    IndependentController,
    /// A public authority receiving under a legal obligation.
    Authority,
    /// Contacted by the diner's browser directly; the page gives it the
    /// device's address and what is asked of it.
    BrowserDirect,
}

/// Which of a venue's settings turns a processor on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    Always,
    Telegram,
    Meta,
    Cloud,
    Stripe,
    Ai,
    FiscalSend,
    EbillsLink,
    Map,
}

/// A host named in this crate or in the storefront's CSP, and what it is.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct Host {
    pub host: &'static str,
    pub recipient: Recipient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum Recipient {
    /// Receives data: the processor's `id`.
    Processor(&'static str),
    /// Receives nothing about a person, and why.
    NotARecipient(&'static str),
}

/// A browser storage key (or key prefix) the front ends write.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct DeviceKey {
    pub key: &'static str,
    /// `true` when `key` is a prefix (`dw_cart_<slug>`).
    pub prefix: bool,
    pub holds: &'static [Data],
    pub surface: &'static str,
}

/// The processor with this id.
#[cfg(test)]
pub fn processor(id: &str) -> Option<&'static Processor> {
    PROCESSORS.iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests;
