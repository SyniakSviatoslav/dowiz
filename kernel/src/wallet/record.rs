//! M1 — on-device wallet store + checkout autofill (BLUEPRINT-P66 §3 / §4.1).
//!
//! The wallet holds ONLY a payment-method *reference* (an opaque provider id,
//! e.g. Stripe `pm_…`) — NEVER a PAN/CVV/expiry. The no-card-data guarantee is
//! structural (no such field exists) PLUS [`crate::wallet::no_card_data_in_wallet`].
//!
//! The wallet is a versioned JSON blob persisted through the [`WalletStore`] port.
//! Last-write-wins via a strictly-monotone `rev` (single-writer, NO CRDT — R4 §3.1).
//! The pure crate serializes with a hand-rolled serde-free codec so the DEFAULT
//! kernel build stays serde-free (same discipline as `money`/`event_log`).

/// The on-device wallet schema version. Forward-compat gate on load.
pub const WALLET_SCHEMA_VERSION: u16 = 1;

/// Opaque provider-scoped payment-method reference (e.g. Stripe `pm_…`). NEVER a PAN/CVV/expiry.
/// This is the ONLY payment datum the wallet stores (§4.1). Defined here and re-exported by
/// `draft` so `record` + `draft` + `transfer` all share ONE type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentMethodRef(pub String);

/// A saved delivery address. Free-form lines (the catalog/address parse is not P66's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub label: String,
    pub lines: Vec<String>,
    pub note: Option<String>,
}

/// The minimum contact set the customer consents to hand a hub at checkout (§16.23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contact {
    pub email: Option<String>,
    pub phone_e164: Option<String>,
}

/// THE on-device wallet. Versioned JSON, LAST-WRITE-WINS (R4 §3.1 — NOT a CRDT).
/// `rev` is a strictly-monotone local counter; on the (rare, non-required) two-tab race
/// the higher `rev` wins — single-writer makes this strictly correct, no merge needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletRecord {
    pub schema_version: u16, // = WALLET_SCHEMA_VERSION; forward-compat gate on load
    pub rev: u64,            // monotone; ++ on every committed edit (LWW ordering key)
    pub updated_at_ms: u64,  // wall clock, advisory only (rev is the authority)
    pub wallet_id: [u8; 32], // stable per-device client id (feeds the idem-key derivation)
    pub name: Option<String>,
    pub addresses: Vec<Address>,
    pub contact: Option<Contact>,
    pub method_ref: Option<PaymentMethodRef>, // reference only — never card data
}

impl WalletRecord {
    /// A fresh wallet for a new device/client id.
    pub fn new(wallet_id: [u8; 32]) -> Self {
        WalletRecord {
            schema_version: WALLET_SCHEMA_VERSION,
            rev: 0,
            updated_at_ms: 0,
            wallet_id,
            name: None,
            addresses: Vec::new(),
            contact: None,
            method_ref: None,
        }
    }

    /// Apply a committed edit: bump `rev` (LWW ordering key — strictly monotone) + advisory clock.
    pub fn bump_rev(&mut self, now_ms: u64) {
        self.rev = self.rev.saturating_add(1);
        self.updated_at_ms = now_ms;
    }
}

/// Typed store errors — a storage fault reaches the logic ONLY as a value, never a panic (bulkhead §5.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletStoreError {
    Io(String),
    Corrupt,
    VersionTooNew(u16),
    QuotaExceeded,
}

/// The on-device store port. The pure crate knows nothing about tauri/idb (bulkhead §5.3).
pub trait WalletStore {
    /// Load the persisted record (None if never saved).
    fn load(&self) -> Result<Option<WalletRecord>, WalletStoreError>;
    /// Persist the record. A Tauri impl calls `store.save()` HERE (explicit, per edit — R4 §3.2).
    fn save(&mut self, rec: &WalletRecord) -> Result<(), WalletStoreError>;
    /// User self-delete (§16.58).
    fn clear(&mut self) -> Result<(), WalletStoreError>;
}

/// Autofill seam (P57 §3). P66 projects a [`WalletRecord`] into opaque `TextField`-like
/// widgets at any hub's checkout: `name`/`address`/`contact`/`method_ref` become `String`
/// values the consumer pushes via `set_value` (P57 owns the real `TextField`). P66 stores
/// nothing across sessions — it only READS the wallet and WRITES the prefill.
///
/// # `TextField` port contract (consumed, never redefined — P57)
/// * `set_value(&str)` — P66 calls this to prefill (restore). Scope-gated on P57's side.
/// * `value() -> &str` — P69 (or the consumer) calls this at the submit boundary to read back.
pub trait TextField {
    fn set_value(&mut self, s: &str);
}

/// The consumer-submit read side: a value-bearing `TextField` the consumer can read back via
/// `value()` (P57 `TextField::value`). P66 never mutates it; it only observes the submitted snapshot.
pub trait TextFieldRead {
    fn value(&self) -> &str;
}

/// The projection P66 feeds the checkout wizard: one `Widget` per autofill slot.
#[derive(Default)]
pub struct AutofillTargets {
    pub name: Option<Box<dyn TextField>>,
    pub address: Option<Box<dyn TextField>>,
    pub contact: Option<Box<dyn TextField>>,
    pub method: Option<Box<dyn TextField>>,
}

/// Project the wallet into the target `TextField`s via `set_value` (P57 prefill seam).
/// A field is skipped when the wallet slot is `None` (never fabricates data).
pub fn autofill_into(rec: &WalletRecord, targets: &mut AutofillTargets) {
    if let Some(name) = &rec.name {
        if let Some(t) = targets.name.as_mut() {
            t.set_value(name);
        }
    }
    if let Some(addr) = rec
        .addresses
        .first()
        .map(|a| a.lines.join("\n"))
        .filter(|s| !s.is_empty())
    {
        if let Some(t) = targets.address.as_mut() {
            t.set_value(&addr);
        }
    }
    if let Some(contact) = &rec.contact {
        let s = contact
            .email
            .clone()
            .or_else(|| contact.phone_e164.clone())
            .unwrap_or_default();
        if let Some(t) = targets.contact.as_mut() {
            t.set_value(&s);
        }
    }
    if let Some(m) = &rec.method_ref {
        if let Some(t) = targets.method.as_mut() {
            t.set_value(&m.0);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hand-rolled serde-free JSON codec (keeps the default kernel build serde-free).
// Numbers/strings only — no f64, no card data. Unknown keys ignored (forward-tolerant).
// ─────────────────────────────────────────────────────────────────────────────

/// Serialize a [`WalletRecord`] to a stable JSON string (LWW `rev`-ordered fields).
pub fn serialize(rec: &WalletRecord) -> String {
    let mut s = String::new();
    s.push('{');
    s.push_str("\"schema_version\":");
    s.push_str(&rec.schema_version.to_string());
    s.push_str(",\"rev\":");
    s.push_str(&rec.rev.to_string());
    s.push_str(",\"updated_at_ms\":");
    s.push_str(&rec.updated_at_ms.to_string());
    s.push_str(",\"wallet_id\":\"");
    write_hex(&rec.wallet_id, &mut s);
    s.push('"');
    s.push_str(",\"name\":");
    push_opt_str(&rec.name, &mut s);
    s.push_str(",\"addresses\":[");
    for (i, a) in rec.addresses.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str("{\"label\":");
        push_str(&a.label, &mut s);
        s.push_str(",\"lines\":[");
        for (j, l) in a.lines.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            push_str(l, &mut s);
        }
        s.push_str("],\"note\":");
        push_opt_str(&a.note, &mut s);
        s.push('}');
    }
    s.push(']');
    s.push_str(",\"contact\":");
    match &rec.contact {
        None => s.push_str("null"),
        Some(c) => {
            s.push_str("{\"email\":");
            push_opt_str(&c.email, &mut s);
            s.push_str(",\"phone_e164\":");
            push_opt_str(&c.phone_e164, &mut s);
            s.push('}');
        }
    }
    s.push_str(",\"method_ref\":");
    push_opt_str(&rec.method_ref.as_ref().map(|m| m.0.clone()), &mut s);
    s.push('}');
    s
}

/// Parse a [`WalletRecord`] from JSON produced by [`serialize`] (or a compatible LWW blob).
pub fn deserialize(src: &str) -> Result<WalletRecord, WalletStoreError> {
    let mut p = Parser::new(src);
    let obj = match p.parse_value() {
        Ok(Value::Object(o)) => o,
        _ => return Err(WalletStoreError::Corrupt),
    };
    let schema = obj
        .get_u16("schema_version")
        .ok_or(WalletStoreError::Corrupt)?;
    if schema > WALLET_SCHEMA_VERSION {
        return Err(WalletStoreError::VersionTooNew(schema));
    }
    let rev = obj.get_u64("rev").ok_or(WalletStoreError::Corrupt)?;
    let updated_at_ms = obj.get_u64("updated_at_ms").unwrap_or(0);
    let wallet_id = match obj.get_str("wallet_id") {
        Some(h) => parse_hex32(h).ok_or(WalletStoreError::Corrupt)?,
        None => return Err(WalletStoreError::Corrupt),
    };
    let name = obj.get_str("name").map(|s| s.to_string());
    let addresses = match obj.get_val("addresses") {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| match v {
                Value::Object(o) => {
                    let label = o.get_str("label").unwrap_or("").to_string();
                    let lines = match o.get_val("lines") {
                        Some(Value::Array(la)) => la
                            .iter()
                            .filter_map(|l| match l {
                                Value::String(s) => Some(s.clone()),
                                _ => None,
                            })
                            .collect(),
                        _ => Vec::new(),
                    };
                    let note = o.get_str("note").map(|s| s.to_string());
                    Some(Address { label, lines, note })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let contact = match obj.get_val("contact") {
        Some(Value::Object(o)) => Some(Contact {
            email: o.get_str("email").map(|s| s.to_string()),
            phone_e164: o.get_str("phone_e164").map(|s| s.to_string()),
        }),
        _ => None,
    };
    let method_ref = obj
        .get_str("method_ref")
        .map(|s| PaymentMethodRef(s.to_string()));
    Ok(WalletRecord {
        schema_version: schema,
        rev,
        updated_at_ms,
        wallet_id,
        name,
        addresses,
        contact,
        method_ref,
    })
}

// ── minimal JSON value model (numbers/strings/bools/arrays/objects only) ──

#[derive(Debug, Clone)]
enum Value {
    Null,
    #[allow(dead_code)]
    Bool(bool),
    Num(i64),
    String(String),
    Array(Vec<Value>),
    Object(std::collections::HashMap<String, Value>),
}

impl Value {
    #[allow(dead_code)]
    fn get(&self, k: &str) -> Option<&Value> {
        match self {
            Value::Object(o) => o.get(k),
            _ => None,
        }
    }
    #[allow(dead_code)]
    fn get_str(&self, k: &str) -> Option<&str> {
        match self.get(k) {
            Some(Value::String(s)) => Some(s),
            _ => None,
        }
    }
    #[allow(dead_code)]
    fn get_u16(&self, k: &str) -> Option<u16> {
        match self.get(k) {
            Some(Value::Num(n)) => u16::try_from(*n).ok(),
            _ => None,
        }
    }
    #[allow(dead_code)]
    fn get_u64(&self, k: &str) -> Option<u64> {
        match self.get(k) {
            Some(Value::Num(n)) => u64::try_from(*n).ok(),
            _ => None,
        }
    }
}

/// Helpers over the `HashMap` produced by `Value::Object`, so `deserialize` can read fields
/// directly off an object map (the `get_*` methods live on `Value`; this trait mirrors them for
/// the already-destructured `HashMap`).
trait Obj {
    fn get_u16(&self, k: &str) -> Option<u16>;
    fn get_u64(&self, k: &str) -> Option<u64>;
    fn get_str(&self, k: &str) -> Option<&str>;
    fn get_val(&self, k: &str) -> Option<&Value>;
}
impl Obj for std::collections::HashMap<String, Value> {
    fn get_u16(&self, k: &str) -> Option<u16> {
        match self.get(k) {
            Some(Value::Num(n)) => u16::try_from(*n).ok(),
            _ => None,
        }
    }
    fn get_u64(&self, k: &str) -> Option<u64> {
        match self.get(k) {
            Some(Value::Num(n)) => u64::try_from(*n).ok(),
            _ => None,
        }
    }
    fn get_str(&self, k: &str) -> Option<&str> {
        match self.get(k) {
            Some(Value::String(s)) => Some(s),
            _ => None,
        }
    }
    fn get_val(&self, k: &str) -> Option<&Value> {
        self.get(k)
    }
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser {
            b: s.as_bytes(),
            i: 0,
        }
    }
    fn skip_ws(&mut self) {
        while self.i < self.b.len() {
            match self.b[self.i] {
                b' ' | b'\t' | b'\n' | b'\r' => self.i += 1,
                _ => break,
            }
        }
    }
    fn parse_value(&mut self) -> Result<Value, ()> {
        self.skip_ws();
        if self.i >= self.b.len() {
            return Err(());
        }
        match self.b[self.i] {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => Ok(Value::String(self.parse_string()?)),
            b't' | b'f' => self.parse_bool(),
            b'n' => self.parse_null(),
            _ => self.parse_num(),
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), ()> {
        self.skip_ws();
        if self.i < self.b.len() && self.b[self.i] == c {
            self.i += 1;
            Ok(())
        } else {
            Err(())
        }
    }
    fn parse_object(&mut self) -> Result<Value, ()> {
        self.expect(b'{')?;
        let mut o = std::collections::HashMap::new();
        self.skip_ws();
        if self.i < self.b.len() && self.b[self.i] == b'}' {
            self.i += 1;
            return Ok(Value::Object(o));
        }
        loop {
            self.skip_ws();
            if self.b[self.i] != b'"' {
                return Err(());
            }
            let key = self.parse_string()?;
            self.expect(b':')?;
            let v = self.parse_value()?;
            o.insert(key, v);
            self.skip_ws();
            match self.b[self.i] {
                b',' => {
                    self.i += 1;
                    continue;
                }
                b'}' => {
                    self.i += 1;
                    break;
                }
                _ => return Err(()),
            }
        }
        Ok(Value::Object(o))
    }
    fn parse_array(&mut self) -> Result<Value, ()> {
        self.expect(b'[')?;
        let mut a = Vec::new();
        self.skip_ws();
        if self.i < self.b.len() && self.b[self.i] == b']' {
            self.i += 1;
            return Ok(Value::Array(a));
        }
        loop {
            let v = self.parse_value()?;
            a.push(v);
            self.skip_ws();
            match self.b[self.i] {
                b',' => {
                    self.i += 1;
                    continue;
                }
                b']' => {
                    self.i += 1;
                    break;
                }
                _ => return Err(()),
            }
        }
        Ok(Value::Array(a))
    }
    fn parse_string(&mut self) -> Result<String, ()> {
        self.expect(b'"')?;
        let mut out = String::new();
        while self.i < self.b.len() {
            let c = self.b[self.i];
            if c == b'"' {
                self.i += 1;
                return Ok(out);
            }
            if c == b'\\' {
                self.i += 1;
                if self.i >= self.b.len() {
                    return Err(());
                }
                let e = self.b[self.i];
                match e {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    b'0' => out.push('\0'),
                    _ => return Err(()),
                }
                self.i += 1;
            } else {
                // Collect a UTF-8 char boundary (no surrogate/encoding validation needed for wallet fields).
                let start = self.i;
                while self.i < self.b.len() && self.b[self.i] != b'"' && self.b[self.i] != b'\\' {
                    self.i += 1;
                }
                out.push_str(std::str::from_utf8(&self.b[start..self.i]).map_err(|_| ())?);
            }
        }
        Err(())
    }
    fn parse_bool(&mut self) -> Result<Value, ()> {
        if self.b[self.i..].starts_with(b"true") {
            self.i += 4;
            Ok(Value::Bool(true))
        } else if self.b[self.i..].starts_with(b"false") {
            self.i += 5;
            Ok(Value::Bool(false))
        } else {
            Err(())
        }
    }
    fn parse_null(&mut self) -> Result<Value, ()> {
        if self.b[self.i..].starts_with(b"null") {
            self.i += 4;
            Ok(Value::Null)
        } else {
            Err(())
        }
    }
    fn parse_num(&mut self) -> Result<Value, ()> {
        let start = self.i;
        while self.i < self.b.len() && matches!(self.b[self.i], b'0'..=b'9' | b'-') {
            self.i += 1;
        }
        if self.i == start {
            return Err(());
        }
        let s = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| ())?;
        s.parse::<i64>().map(Value::Num).map_err(|_| ())
    }
}

fn push_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
}

fn push_opt_str(o: &Option<String>, out: &mut String) {
    match o {
        Some(s) => push_str(s, out),
        None => out.push_str("null"),
    }
}

fn write_hex(bytes: &[u8; 32], out: &mut String) {
    const H: &[u8; 16] = b"0123456789abcdef";
    for &b in bytes {
        out.push(H[(b >> 4) as usize] as char);
        out.push(H[(b & 0xf) as usize] as char);
    }
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = hex_val(b[i * 2])?;
        let lo = hex_val(b[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_wallet() -> WalletRecord {
        WalletRecord {
            schema_version: WALLET_SCHEMA_VERSION,
            rev: 3,
            updated_at_ms: 1_700_000_000_000,
            wallet_id: [0xabu8; 32],
            name: Some("Ada Lovelace".into()),
            addresses: vec![Address {
                label: "Home".into(),
                lines: vec!["1 Analytical Eng Way".into(), "London".into()],
                note: Some("leave at door".into()),
            }],
            contact: Some(Contact {
                email: Some("ada@example.com".into()),
                phone_e164: None,
            }),
            method_ref: Some(PaymentMethodRef("pm_1A2b3C".into())),
        }
    }

    #[test]
    fn wallet_round_trips_through_store() {
        let rec = sample_wallet();
        let json = serialize(&rec);
        let back = deserialize(&json).expect("round-trip parse");
        assert_eq!(rec, back, "wallet record must round-trip byte-identical");
    }

    #[test]
    fn method_ref_is_opaque_not_card() {
        let rec = sample_wallet();
        // The wallet holds a payment-method reference, never a PAN. Assert the field exists
        // and is opaque; there is NO card field to populate (the firewall enforces absence).
        assert!(rec.method_ref.as_ref().unwrap().0.starts_with("pm_"));
        // Serialized form must NOT carry any card-data token (structural guarantee).
        let json = serialize(&rec);
        for tok in ["pan", "cvv", "card_number", "exp_month"] {
            assert!(
                !json.to_lowercase().contains(tok),
                "wallet json leaked '{tok}'"
            );
        }
    }

    #[test]
    fn version_too_new_is_typed_error() {
        let mut rec = sample_wallet();
        rec.schema_version = WALLET_SCHEMA_VERSION + 1;
        let json = serialize(&rec);
        match deserialize(&json) {
            Err(WalletStoreError::VersionTooNew(v)) => assert_eq!(v, WALLET_SCHEMA_VERSION + 1),
            other => panic!("expected VersionTooNew, got {other:?}"),
        }
    }

    #[test]
    fn corrupt_blob_is_typed_error_not_panic() {
        assert_eq!(
            deserialize("{not valid json"),
            Err(WalletStoreError::Corrupt)
        );
        assert_eq!(deserialize(""), Err(WalletStoreError::Corrupt));
        assert_eq!(deserialize("42"), Err(WalletStoreError::Corrupt));
    }

    #[test]
    fn rev_is_monotone_authority() {
        let mut rec = WalletRecord::new([1u8; 32]);
        assert_eq!(rec.rev, 0);
        rec.bump_rev(100);
        assert_eq!(rec.rev, 1);
        rec.bump_rev(200);
        assert_eq!(rec.rev, 2);
    }

    // A mock TextField that records the last value set (and the last read).
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A mock TextField that records the last value set into a shared probe the test can read.
    struct MockField {
        probe: Rc<RefCell<String>>,
    }
    impl TextField for MockField {
        fn set_value(&mut self, s: &str) {
            *self.probe.borrow_mut() = s.to_string();
        }
    }

    #[test]
    fn autofill_sets_textfield_values() {
        let rec = sample_wallet();
        let name_probe = Rc::new(RefCell::new(String::new()));
        let addr_probe = Rc::new(RefCell::new(String::new()));
        let contact_probe = Rc::new(RefCell::new(String::new()));
        let method_probe = Rc::new(RefCell::new(String::new()));
        let mut targets = AutofillTargets {
            name: Some(Box::new(MockField {
                probe: name_probe.clone(),
            })),
            address: Some(Box::new(MockField {
                probe: addr_probe.clone(),
            })),
            contact: Some(Box::new(MockField {
                probe: contact_probe.clone(),
            })),
            method: Some(Box::new(MockField {
                probe: method_probe.clone(),
            })),
        };
        autofill_into(&rec, &mut targets);
        assert_eq!(*name_probe.borrow(), "Ada Lovelace");
        assert_eq!(*addr_probe.borrow(), "1 Analytical Eng Way\nLondon");
        assert_eq!(*contact_probe.borrow(), "ada@example.com");
        assert_eq!(*method_probe.borrow(), "pm_1A2b3C");
    }

    #[test]
    fn autofill_skips_absent_slots() {
        let rec = WalletRecord::new([2u8; 32]); // all None
        let name_probe = Rc::new(RefCell::new("untouched".to_string()));
        let mut targets = AutofillTargets {
            name: Some(Box::new(MockField {
                probe: name_probe.clone(),
            })),
            address: None,
            contact: None,
            method: None,
        };
        autofill_into(&rec, &mut targets);
        assert_eq!(
            *name_probe.borrow(),
            "untouched",
            "absent slot must not fabricate"
        );
    }

    // ── injected: empty / zero-rev / negative / dup-id / overflow / invalid-sig / empty-rec ──

    #[test]
    fn empty_wallet_all_slots_none() {
        let rec = WalletRecord::new([0u8; 32]);
        assert_eq!(rec.rev, 0);
        assert!(rec.name.is_none());
        assert!(rec.addresses.is_empty());
        assert!(rec.contact.is_none());
        assert!(rec.method_ref.is_none());
    }

    #[test]
    fn zero_rev_operations() {
        let mut rec = WalletRecord::new([255u8; 32]);
        assert_eq!(rec.rev, 0);
        rec.bump_rev(0);
        assert_eq!(rec.rev, 1); // saturating_add bumps even from 0
    }

    #[test]
    fn rev_overflow_saturating() {
        let mut rec = WalletRecord {
            schema_version: WALLET_SCHEMA_VERSION,
            rev: u64::MAX,
            updated_at_ms: 0,
            wallet_id: [0x42u8; 32],
            name: None,
            addresses: vec![],
            contact: None,
            method_ref: None,
        };
        rec.bump_rev(1);
        assert_eq!(rec.rev, u64::MAX); // saturating => stays at MAX
    }

    #[test]
    fn empty_contact_with_none_fields() {
        let rec = sample_wallet();
        assert!(rec.contact.is_some());
        // Contact with both None should still autofill to empty string
        let c = Contact { email: None, phone_e164: None };
        let mut rec2 = WalletRecord::new([9u8; 32]);
        rec2.contact = Some(c);
        let name_probe = Rc::new(RefCell::new(String::new()));
        let contact_probe = Rc::new(RefCell::new(String::new()));
        let mut targets = AutofillTargets {
            name: Some(Box::new(MockField { probe: name_probe.clone() })),
            address: None,
            contact: Some(Box::new(MockField { probe: contact_probe.clone() })),
            method: None,
        };
        autofill_into(&rec2, &mut targets);
        assert_eq!(*contact_probe.borrow(), ""); // both None => empty default
    }

    #[test]
    fn round_trip_with_all_nulls() {
        let rec = WalletRecord::new([0xdeu8; 32]);
        let json = serialize(&rec);
        let back = deserialize(&json).expect("round-trip parse");
        assert_eq!(rec, back);
    }

    #[test]
    fn round_trip_multiple_addresses() {
        let mut rec = WalletRecord::new([0xabu8; 32]);
        rec.addresses = vec![
            Address { label: "Home".into(), lines: vec!["1 Main St".into()], note: None },
            Address { label: "Work".into(), lines: vec!["2 Office Rd".into(), "Floor 3".into()], note: Some("ring buzzer".into()) },
        ];
        rec.contact = Some(Contact { email: Some("a@b.com".into()), phone_e164: Some("+12345678901".into()) });
        let json = serialize(&rec);
        let back = deserialize(&json).expect("round-trip parse");
        assert_eq!(rec, back);
    }

    #[test]
    fn empty_autofill_targets_no_fields_present() {
        let rec = sample_wallet();
        let mut targets = AutofillTargets::default();
        autofill_into(&rec, &mut targets);
        // All None => all skipped, no panic
    }

    #[test]
    fn rev_monotone_across_many_bumps() {
        let mut rec = WalletRecord::new([7u8; 32]);
        for i in 0..500 {
            rec.bump_rev(i);
            assert_eq!(rec.rev, i + 1);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Property tests — Flow 1: Payment / Value Transfer invariants
    // ═══════════════════════════════════════════════════════════════════════════

    /// prop-5a: Zero invariant — new wallet has rev=0, all slots None/empty.
    #[test]
    fn prop_zero_invariant_new_wallet_all_fields_empty() {
        let id = [0xCAu8; 32];
        let w = WalletRecord::new(id);
        assert_eq!(w.rev, 0, "new wallet rev must be 0");
        assert_eq!(w.schema_version, WALLET_SCHEMA_VERSION);
        assert_eq!(w.wallet_id, id);
        assert!(w.name.is_none(), "name must be None");
        assert!(w.addresses.is_empty(), "addresses must be empty");
        assert!(w.contact.is_none(), "contact must be None");
        assert!(w.method_ref.is_none(), "method_ref must be None");
    }

    /// prop-8a: Deterministic serialization — serialize→deserialize→serialize == original.
    #[test]
    fn prop_deterministic_wallet_serialization_idempotent() {
        let rec = sample_wallet();
        let json1 = serialize(&rec);
        let json2 = serialize(&rec);
        assert_eq!(json1, json2, "serialize must be deterministic");
        let back1 = deserialize(&json1).expect("deserialize");
        let back2 = deserialize(&json1).expect("deserialize again");
        assert_eq!(back1, back2, "deserialize must be deterministic");
        assert_eq!(serialize(&back1), json1, "serialize(deserialize(json)) == json");
    }
}
