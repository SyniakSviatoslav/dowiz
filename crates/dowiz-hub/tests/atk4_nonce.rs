//! W-MG-ATK4, ATTACK 5: THE NONCE. M4's promise: once a record is redacted its kept id cannot
//! confirm a guessed payload. That rests on the writer having a seed. The attacker here holds
//! only the redacted image and a list of guesses: no key, no seed.

use bebop_store::evlog::doc::id_message;
use bebop_store::evlog::{EvLog, Record};
use bebop_store::nodekey::sha256;
use bebop_store::Store;
use dowiz_hub::consent::{self, Act, Method, State, KIND_ACT};
use dowiz_hub::logimage::LogImage;

const KEY: &str = "a1b2c3d4e5f60718";
const PHONE: &str = "+355691111111";

fn act(evidence: &str) -> Act {
    Act {
        key: KEY.into(),
        purpose: consent::PURPOSE_MARKETING.into(),
        channel: consent::CHANNEL_WHATSAPP.into(),
        state: State::Given,
        at_ms: 10,
        method: Method::OwnerEntered,
        evidence: evidence.into(),
        wording_id: consent::wording_id("en"),
        via: "owner_7".into(),
    }
}

/// The attacker's own recomputation of a seedless writer's id, from the public format.
fn id_if(r: &Record, guess: &str) -> [u8; 32] {
    let (k, s, j) = (KIND_ACT.as_bytes(), consent::subject_of(KEY), act(guess).to_json());
    let mut body = vec![k.len() as u8];
    body.extend_from_slice(k);
    body.push(s.len() as u8);
    body.extend_from_slice(s.as_bytes());
    body.extend_from_slice(j.as_bytes());
    let mut m = b"dowiz/nonce/derived".to_vec();
    m.extend_from_slice(&r.prev);
    m.extend_from_slice(&body);
    let mut payload = dowiz_hub::crypto::b64url_encode(&sha256(&m)[..12]).into_bytes();
    payload.extend_from_slice(&body);
    sha256(&id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &payload))
}

/// Who confirms what, from the image alone, over ten thousand phones.
fn confirmed(c: &LogImage) -> Vec<String> {
    let target = act_record(c);
    (0..10_000u32)
        .map(|n| format!("paper form, Arben, +35569111{n:04}"))
        .filter(|g| id_if(&target, g) == target.id)
        .collect()
}

/// The person's oldest act (a wording record sits below it).
fn act_record(c: &LogImage) -> Record {
    let s = consent::subject_of(KEY);
    EvLog::walk(&Store::from_bytes(&c.to_bytes())).into_iter().rev().find(|r| r.payload.windows(s.len()).any(|w| w == s.as_bytes())).unwrap()
}

fn forget_of(c: &mut LogImage) {
    consent::log::write(c, &act(&format!("paper form, Arben, {PHONE}"))).unwrap();
    consent::forget::forget(c, KEY, 99).unwrap();
    assert!(!String::from_utf8_lossy(&c.to_bytes()).contains(PHONE), "control: the phone left the bytes");
}

/// H7. The attack itself, on a log nobody armed: the id confirms the phone. This is what `derived`
/// announces, and it is why the Worker has no way to reach a log unseeded (`scheme::arm` is one
/// call, `platform_store::load_log_at` requires the venue). The instrument's own control too.
#[test]
fn an_unseeded_log_confirms_and_says_so() {
    let mut c = LogImage::create_sized(64 * 1024).unwrap();
    forget_of(&mut c);
    assert_eq!(confirmed(&c).len(), 1, "the attack works on a seedless writer");
    assert!(c.chain_check().derived > 0, "and the log says so");
}

/// H7, the reload half: the seed survives an erasure's reload, so what it appends is drawn too.
#[test]
fn a_seed_survives_the_erasure_that_reloads_the_log() {
    let mut c = LogImage::create_sized(64 * 1024).unwrap();
    c.set_entropy([3; 32]);
    forget_of(&mut c);
    assert_eq!(c.chain_check().derived, 0, "every record after the reload is drawn");
}

/// Twin: a seed. The same attack finds nothing: the rule is the seed, not the format.
#[test]
fn twin_a_seeded_consent_log_confirms_nothing() {
    let mut c = LogImage::create_sized(64 * 1024).unwrap();
    c.set_entropy([3; 32]);
    forget_of(&mut c);
    assert_eq!(confirmed(&c), Vec::<String>::new());
}
