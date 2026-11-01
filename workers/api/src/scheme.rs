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

/// Pure: the choice for an object that may not know its name. NO NAME IS NOT sha256 (W-MG-TRANS
/// H3): an object is "not reliably told" its name (`hubdo/timer.rs`), and leaving such a log on the
/// crate's default wrote sha256 on an unflipped venue. Only `*` covers a name nobody knows.
pub fn chosen(list: &str, venue: Option<&str>) -> bool {
    match venue {
        Some(v) => flipped(list, v),
        None => list.trim() == "*",
    }
}

/// `Some(flipped)` once installed; `None` in a process that never installed the var.
fn choice(venue: Option<&str>) -> Option<bool> {
    #[cfg(test)]
    if behaviour::on() {
        return Some(false);
    }
    LIST.get().map(|l| chosen(l, venue))
}

/// THE BEHAVIOUR CHECK (W-MG-TRANS, `tools/deploy/scheme-armed.sh --behaviour`). A file grep passed
/// over four doors that wrote sha256 at `""`. So the whole Worker suite is run with every armed log
/// on legacy, and the one door every image goes through (`hubdo/store.rs::put_chunks`) writes down
/// any image that holds a sha256 record its previous version did not. An unarmed door shows up
/// there, whichever route reached it, the day it is written.
#[cfg(test)]
pub mod behaviour {
    pub fn on() -> bool {
        std::env::var_os("SCHEME_BEHAVIOUR").is_some()
    }

    /// `old` and `new` are an image's two versions; a line per new sha256 record's image.
    pub fn check(id: &str, old: Option<&[u8]>, new: &[u8]) {
        if !on() {
            return;
        }
        let had = old.and_then(dowiz_hub::glue::walk::flagged).unwrap_or(0);
        let has = dowiz_hub::glue::walk::flagged(new).unwrap_or(0);
        if had == 0 && has > 0 {
            let t = std::thread::current();
            let line = format!("UNARMED image={id} records={has} test={}\n", t.name().unwrap_or("?"));
            let path = std::env::var_os("SCHEME_BEHAVIOUR").unwrap_or_default();
            let _ = std::fs::OpenOptions::new().create(true).append(true).open(path).and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
        }
    }
}

/// A log whose id scheme the Worker chooses: the three the hub crate writes.
pub trait Switch {
    fn set_sha256(&mut self, on: bool);
    fn set_entropy(&mut self, seed: [u8; 32]);
}
impl Switch for dowiz_hub::Hub {
    fn set_sha256(&mut self, on: bool) {
        dowiz_hub::Hub::set_sha256(self, on)
    }
    fn set_entropy(&mut self, seed: [u8; 32]) {
        dowiz_hub::Hub::set_entropy(self, seed)
    }
}
impl Switch for dowiz_hub::logimage::LogImage {
    fn set_sha256(&mut self, on: bool) {
        dowiz_hub::logimage::LogImage::set_sha256(self, on)
    }
    fn set_entropy(&mut self, seed: [u8; 32]) {
        dowiz_hub::logimage::LogImage::set_entropy(self, seed)
    }
}
impl Switch for dowiz_hub::stock::StockLog {
    fn set_sha256(&mut self, on: bool) {
        dowiz_hub::stock::StockLog::set_sha256(self, on)
    }
    fn set_entropy(&mut self, seed: [u8; 32]) {
        dowiz_hub::stock::StockLog::set_entropy(self, seed)
    }
}

/// ARM `log` FOR `venue`, ONE CALL (W-MG-TRANS H7): the nonce seed and the scheme from the installed
/// list. EVERY site that loads or creates a log it will append to calls this
/// (`tools/deploy/scheme-armed.sh` lists them), and the log carries both through every rebuild
/// (`LogImage::fresh`, `reload`). The scheme is a request, not a decision: a log already on sha256
/// ignores `false` (hub crate). A seed that cannot be drawn leaves a derived nonce, flagged.
pub fn arm<T: Switch>(log: &mut T, venue: &str) {
    arm_named(log, Some(venue));
}

/// `arm` for an object that may not know its name.
pub fn arm_named<T: Switch>(log: &mut T, venue: Option<&str>) {
    if let Some(on) = choice(venue) {
        armed(log, on);
    }
}

/// The one thing arming is: a seed and a scheme. Live, `choice` always answers, so every log gets both.
fn armed<T: Switch>(log: &mut T, on: bool) {
    if let Some(seed) = crate::entropy_seed() {
        log.set_entropy(seed);
    }
    log.set_sha256(on);
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

    /// H3: an object that does not know its name is not flipped, whatever the list names but `*`.
    #[test]
    fn no_name_fails_closed_to_legacy() {
        assert!(!chosen("", None));
        assert!(!chosen("qa-durres", None), "a list names venues, and this is none of them");
        assert!(chosen("*", None), "everybody means everybody");
        assert!(chosen("qa-durres", Some("qa-durres")), "twin: a name decides as `flipped` does");
    }

    /// H7: arming is one call: it seeds as well as it switches.
    #[test]
    fn arm_seeds_the_log() {
        let mut log = dowiz_hub::logimage::LogImage::create_sized(64 * 1024).unwrap();
        armed(&mut log, true);
        log.append("x", "s", "{}").unwrap();
        assert_eq!(log.chain_check().derived, 0, "armed: the nonce was drawn, not derived");
        let mut bare = dowiz_hub::logimage::LogImage::create_sized(64 * 1024).unwrap();
        bare.append("x", "s", "{}").unwrap();
        assert_eq!(bare.chain_check().derived, 1, "control: unarmed, it is derived");
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
            fn set_entropy(&mut self, _seed: [u8; 32]) {}
        }
        assert!(LIST.get().is_none(), "no test installs the list");
        let mut p = Probe(None);
        arm(&mut p, "qa-durres");
        let want = behaviour::on().then_some(false); // the behaviour run answers legacy for everyone
        assert_eq!(p.0, want, "not installed: the log keeps the hub crate's default");
    }
}
