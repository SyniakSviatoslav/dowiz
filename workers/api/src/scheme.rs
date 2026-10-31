//! THE PER-VENUE ID-SCHEME SWITCH (W-MG-MIG, 2026-10-31): which venues' logs write sha256
//! records (dowiz-hub M0) and which still write the legacy FNV records prod ac5ff0c4 reads.
//!
//! WHY A SWITCH. The old build does not refuse a sha256 record: it QUARANTINES it (the
//! nonce's first byte reads as an unknown event kind), writes FNV records after it, and its
//! `grow` strips the flag -- measured 2026-10-31 with the real binary, quoted in
//! `docs/design/MIGRATION-sha256-ids.md` §1. So the first build that reads both schemes must
//! WRITE the old one until a venue is deliberately flipped: expand, then contract.
//!
//! THE SWITCH IS `SHA256_VENUES` in `wrangler.toml`: `""` nobody (the expand release), a
//! comma-separated list of venue ids (`"qa-durres"` first), `"*"` everybody. The operator
//! flips it by editing the file and deploying; each flip is a Worker version, so a
//! `wrangler rollback` restores the previous list -- and a venue already flipped STAYS sha256,
//! because the hub crate makes a flip one-way in the bytes (`Entropy::writes_sha256`).
//! `tools/deploy/rollback.sh` refuses a target older than the first build that reads sha256
//! once any venue has flipped (`tools/deploy/deploy.sh` records the floor).
//!
//! THE VALUE IS INSTALLED ONCE PER ISOLATE (`install`, from the Worker's `main` and the
//! Durable Object's `new`) and read by `arm` wherever a log is loaded or created: the
//! platform-level logs (`errlog`) count as the venue `PLATFORM`. A test host that never
//! installed it leaves the hub crate's default (sha256), which is what every crate test was
//! written against; in production both entry points install before any log is touched.

use std::sync::OnceLock;

/// The `wrangler.toml` var.
pub const VAR: &str = "SHA256_VENUES";
/// The venue name the platform's own logs flip under.
pub const PLATFORM: &str = "platform";

static LIST: OnceLock<String> = OnceLock::new();

/// Remember the var's value for this isolate. A missing var is `""`: nobody flipped.
pub fn install(value: Option<String>) {
    let _ = LIST.set(value.unwrap_or_default());
}

/// Pure: does `list` flip `venue`? `""` nobody, `"*"` everybody, else exact names separated
/// by commas (spaces around a name are ignored; a name never contains one).
pub fn flipped(list: &str, venue: &str) -> bool {
    let list = list.trim();
    if list.is_empty() {
        return false;
    }
    list == "*" || list.split(',').any(|v| v.trim() == venue)
}

/// `Some(flipped)` once installed; `None` in a process that never installed the var.
fn choice(venue: &str) -> Option<bool> {
    LIST.get().map(|l| flipped(l, venue))
}

/// A log whose id scheme the Worker chooses: the three the hub crate writes.
pub trait Switch {
    fn set_sha256(&mut self, on: bool);
}
impl Switch for dowiz_hub::Hub {
    fn set_sha256(&mut self, on: bool) {
        dowiz_hub::Hub::set_sha256(self, on)
    }
}
impl Switch for dowiz_hub::logimage::LogImage {
    fn set_sha256(&mut self, on: bool) {
        dowiz_hub::logimage::LogImage::set_sha256(self, on)
    }
}
impl Switch for dowiz_hub::stock::StockLog {
    fn set_sha256(&mut self, on: bool) {
        dowiz_hub::stock::StockLog::set_sha256(self, on)
    }
}

/// Set `log`'s scheme for `venue` from the installed list. EVERY site that loads or creates
/// a log it will append to calls this (`tools/deploy/scheme-armed.sh` lists them). The call
/// is a request, not a decision: a log already on sha256 ignores `false` (hub crate).
pub fn arm<T: Switch>(log: &mut T, venue: &str) {
    if let Some(on) = choice(venue) {
        log.set_sha256(on);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nobody_one_some_everybody() {
        assert!(!flipped("", "qa-durres"), "the expand release: nobody");
        assert!(!flipped("   ", "qa-durres"));
        assert!(flipped("qa-durres", "qa-durres"), "qa-durres first");
        assert!(!flipped("qa-durres", "dubin-durres"), "and only qa-durres");
        assert!(!flipped("qa-durres", "qa"), "exact names, not prefixes");
        assert!(flipped("qa-durres, dubin-durres", "dubin-durres"), "a list, spaces ignored");
        assert!(flipped("*", "anyone"), "everybody");
        assert!(!flipped("qa-durres", PLATFORM), "the platform's logs flip only when named or under *");
        assert!(flipped("*", PLATFORM));
    }

    /// A PROCESS THAT NEVER INSTALLED THE VAR LEAVES EVERY LOG ON THE CRATE'S DEFAULT: `arm` is
    /// a no-op. This is what every other test in this binary relies on (a Mem host has no env),
    /// so NO test here may call `install` -- the list is set once per process. The installed
    /// path is `choice` over `flipped`, tested above; live, both entry points install.
    #[test]
    fn arm_is_a_no_op_until_the_var_is_installed() {
        struct Probe(Option<bool>);
        impl Switch for Probe {
            fn set_sha256(&mut self, on: bool) {
                self.0 = Some(on);
            }
        }
        assert!(LIST.get().is_none(), "no test installs the list");
        let mut p = Probe(None);
        arm(&mut p, "qa-durres");
        assert_eq!(p.0, None, "not installed: the log keeps the hub crate's default");
    }
}
