//! The `__platform` object (`platform_store.rs`): accounts, sessions, couriers,
//! the waiting list, the witness and the erasure register. For these dowiz is
//! the CONTROLLER (its own accounts and prospects) — except courier and staff
//! records, which it keeps for the venue that hired them.

use super::Basis;
use super::Data::*;
use super::Home::Platform;
use super::Purpose as P;
use super::Subject::*;
use super::{Eraser, Exporter, Retention, Store};

const NO_EXPORT: Exporter = Exporter::Missing("P7");

pub const STORES: &[Store] = &[
    // Owner / staff accounts, memberships, the platform operator, staff invites.
    Store { image: "identity", kinds: &["user", "member", "admin", "sinvite"], home: Platform,
        holds: &[Email, Name, Password], subjects: &[Owner, Staff],
        purpose: P::Account, basis: Basis::Contract,
        retention: Retention::NoLimitYet("kept while the account exists"),
        erase: Eraser::Missing("P7"),
        export: NO_EXPORT },
    // Which host is which venue; the DPA version each venue accepted.
    Store { image: "registry", kinds: &["loc"], home: Platform,
        holds: &[Phone], subjects: &[Owner],
        purpose: P::Account, basis: Basis::Contract,
        retention: Retention::NoLimitYet("kept while the venue is on the platform"),
        erase: Eraser::Missing("P7"),
        export: NO_EXPORT },
    // Refresh tokens, courier and staff sessions, API-key hashes.
    Store { image: "sessions", kinds: &["refresh", "csession", "apikey", "ssession"], home: Platform,
        holds: &[Session, Device], subjects: &[Owner, Staff, Courier],
        purpose: P::Security, basis: Basis::Contract,
        retention: Retention::UntilDone("each session and key expires; revoked ones are swept"),
        erase: Eraser::Expires("session lifetime (auth.rs *_TTL_MS)"),
        export: Exporter::NotPersonal },
    // Couriers: name, phone (a hash index), password hash; rosters; invites.
    Store { image: "couriers", kinds: &["courier", "roster", "invite"], home: Platform,
        holds: &[Name, Phone, Password], subjects: &[Courier],
        purpose: P::Account, basis: Basis::Contract,
        retention: Retention::NoLimitYet("kept while the courier account exists"),
        erase: Eraser::Missing("P7"),
        export: NO_EXPORT },
    // The landing page's waiting list: email, venue name, language, source.
    Store { image: "waitlist", kinds: &["wl"], home: Platform,
        holds: &[Email, Name], subjects: &[Prospect],
        purpose: P::Prospect, basis: Basis::Consent,
        retention: Retention::NoLimitYet("no limit yet; P6 proposes 24 months"),
        erase: Eraser::Missing("P7"),
        export: NO_EXPORT },
    // The erasure register (P3): pseudonymous keys and order ids, no phone or name.
    Store { image: "erasures", kinds: &["forgot"], home: Platform,
        holds: &[Consent], subjects: &[Customer],
        purpose: P::Erasure, basis: Basis::LegalObligation,
        retention: Retention::NoLimitYet("kept so a restored backup is forgotten again (Law 124 Art. 15(2))"),
        erase: Eraser::Retain("it is the record that makes an erasure survive a restore; it holds a pseudonym and order ids, never a phone or a name"),
        export: Exporter::NotPersonal },
    // Platform-level failures (`errlog.rs`): error lines, pruned like the venue's.
    Store { image: "errors", kinds: &["error"], home: Platform,
        holds: &[StaffId], subjects: &[Staff, Customer],
        purpose: P::Security, basis: Basis::LegitimateInterest,
        retention: Retention::Ms(crate::errlog::KEEP_MS, "errlog.rs: KEEP_MS, pruned nightly"),
        erase: Eraser::Expires("seven days (errlog::KEEP_MS)"),
        export: Exporter::NotPersonal },
    // The witness: (records, tip) per image. No person.
    Store { image: "witness", kinds: &["census"], home: Platform,
        holds: &[], subjects: &[],
        purpose: P::Operations, basis: Basis::NotPersonal,
        retention: Retention::Latest,
        erase: Eraser::NotPersonal("record counts and hashes per image; no person"),
        export: Exporter::NotPersonal },
];
