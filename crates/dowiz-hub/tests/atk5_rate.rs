//! W-MG-ATK5: HOW OFTEN AN HONEST FILE READS `recheck=DIFFERS`. No sampling and no seed: every pair of
//! five named movements on one shelf of 2, under three named protocols. Law: the count is 0
//! (W-MG-MERGE2). The rate is of this grid, not of a kitchen.
mod atk5_common;

use atk5_common::*;
use dowiz_hub::doc_seal::merge::Recheck;
use dowiz_hub::stock::StockEvent;

const N: usize = 5;
fn op(i: usize, tag: &str) -> StockEvent {
    match i {
        0 => received("salmon", 1),
        1 => served("salmon", 1, tag),
        2 => wasted("salmon", 1),
        3 => counted("salmon", 1, tag),
        _ => reserved("salmon", 1, tag),
    }
}
const NAMES: [&str; N] = ["received", "served", "wasted", "counted", "reserved"];

fn try_merge(ours: &[u8], theirs: &[u8], key: &dowiz_hub::doc_seal::DeviceKey, seed: u8) -> Option<Vec<u8>> {
    let mut sh = opened(ours, seed);
    sh.merge(theirs, key, 500).ok()?;
    Some(trimmed(&sh))
}
fn differs(f: &[u8]) -> bool {
    audit(f).0.contains(&Recheck::Differs)
}
#[derive(Default)]
struct Tally {
    cases: usize,
    refused: usize,
    differs: usize,
    which: Vec<String>,
}
impl Tally {
    fn add(&mut self, name: String, f: Option<bool>) {
        self.cases += 1;
        match f {
            None => self.refused += 1,
            Some(true) => {
                self.differs += 1;
                self.which.push(name);
            }
            Some(false) => {}
        }
    }
    fn say(&self, p: &str) {
        println!("ATK5 rate {p}: cases={} refused={} differs={} {:?}", self.cases, self.refused, self.differs, self.which);
    }
}
fn pair(i: usize, j: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let s = genesis(&[received("salmon", 2)]);
    let fa = offline(&s, 2, &a(), &[op(i, "a")]);
    let fb = offline(&s, 3, &b(), &[op(j, "b")]);
    (s, fa, fb)
}

/// P1: two devices, one merge, B takes it in. The file a demo makes.
#[test]
fn p1_one_merge() {
    let mut t = Tally::default();
    for i in 0..N {
        for j in 0..N {
            let (_, fa, fb) = pair(i, j);
            let got = try_merge(&fa, &fb, &a(), 4).and_then(|m| try_merge(&fb, &m, &b(), 5)).map(|f| differs(&f));
            t.add(format!("{}/{}", NAMES[i], NAMES[j]), got);
        }
    }
    t.say("P1 one merge");
    assert_eq!(t.differs, 0);
}

/// P2: both merge in both orders, cross-adopt, sell one more each, merge, converge.
#[test]
fn p2_two_rounds() {
    let mut t = Tally::default();
    for i in 0..N {
        for j in 0..N {
            let (_, fa, fb) = pair(i, j);
            let got = (|| {
                let (x, y) = (try_merge(&fa, &fb, &a(), 4)?, try_merge(&fb, &fa, &b(), 5)?);
                let (x, y) = (try_merge(&x, &y, &a(), 6)?, try_merge(&y, &x, &b(), 7)?);
                let (xa, yb) = (offline(&x, 8, &a(), &[served("salmon", 1, "a2")]), offline(&y, 9, &b(), &[served("salmon", 1, "b2")]));
                let (mx, my) = (try_merge(&xa, &yb, &a(), 10)?, try_merge(&yb, &xa, &b(), 11)?);
                let (ex, ey) = (try_merge(&mx, &my, &a(), 12)?, try_merge(&my, &mx, &b(), 13)?);
                Some(differs(&ex) || differs(&ey))
            })();
            t.add(format!("{}/{}", NAMES[i], NAMES[j]), got);
        }
    }
    t.say("P2 two rounds");
    assert_eq!(t.differs, 0);
}

/// P3: one merge, then a third device that never wrote before it does one thing.
#[test]
fn p3_a_later_device() {
    let mut t = Tally::default();
    for i in 0..N {
        for j in 0..N {
            let (_, fa, fb) = pair(i, j);
            let Some(m) = try_merge(&fa, &fb, &a(), 4) else {
                (0..N).for_each(|_| t.add(String::new(), None));
                continue;
            };
            for k in 0..N {
                let later = offline(&m, 5, &c(), &[op(k, "c")]);
                t.add(format!("{}/{}+{}", NAMES[i], NAMES[j], NAMES[k]), Some(differs(&later)));
            }
        }
    }
    t.say("P3 later device");
    assert_eq!(t.differs, 0);
}
