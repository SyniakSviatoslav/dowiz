//! The in-memory host (tests only): storage, alarm, sockets, a name, a clock — and the two
//! faults the object's comments say it survives (a write that fails part-way, a read that
//! errors), so those paths are exercised rather than described.

use super::Sock;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use worker::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Stored {
    Json(serde_json::Value),
    Bytes(Vec<u8>),
}

type Outbox = Rc<RefCell<Vec<String>>>;

#[derive(Default)]
pub struct MemHost {
    pub kv: RefCell<BTreeMap<String, Stored>>,
    pub alarm: Cell<Option<i64>>,
    /// The venue this object was addressed by (`id_from_name`).
    pub name: Option<String>,
    pub clock: Cell<i64>,
    pub secrets: HashMap<String, String>,
    pub sockets: RefCell<Vec<(Vec<String>, Outbox)>>,
    /// Writes allowed before every further write fails (`None`: never fail).
    pub puts_left: Cell<Option<usize>>,
    /// Every read errors while set.
    pub reads_fail: Cell<bool>,
    /// Every storage write, in order (key only).
    pub writes: RefCell<Vec<String>>,
}

impl MemHost {
    pub fn named(venue: &str, clock: i64) -> Rc<Self> {
        Rc::new(MemHost {
            name: Some(venue.to_string()),
            clock: Cell::new(clock),
            ..Default::default()
        })
    }

    /// A socket with these tags; the returned list fills with what it is sent.
    pub fn socket(&self, tags: &[&str]) -> Outbox {
        let out: Outbox = Rc::default();
        self.sockets.borrow_mut().push((tags.iter().map(|t| t.to_string()).collect(), out.clone()));
        out
    }

    pub(super) fn tagged(&self, tag: &str) -> Vec<Sock> {
        self.sockets
            .borrow()
            .iter()
            .filter(|(tags, _)| tags.iter().any(|t| t == tag))
            .map(|(_, out)| Sock::Mem(out.clone()))
            .collect()
    }

    fn read_ok(&self) -> Result<()> {
        if self.reads_fail.get() {
            return Err(Error::RustError("mem: storage read failed (injected)".into()));
        }
        Ok(())
    }

    fn write_ok(&self, key: &str) -> Result<()> {
        if let Some(n) = self.puts_left.get() {
            if n == 0 {
                return Err(Error::RustError(format!("mem: write of {key} failed (injected)")));
            }
            self.puts_left.set(Some(n - 1));
        }
        self.writes.borrow_mut().push(key.to_string());
        Ok(())
    }

    pub(super) fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        self.read_ok()?;
        match self.kv.borrow().get(key) {
            None => Ok(None),
            Some(Stored::Json(v)) => serde_json::from_value(v.clone())
                .map(Some)
                .map_err(|e| Error::RustError(format!("mem: {key} does not deserialise: {e}"))),
            Some(Stored::Bytes(_)) => Err(Error::RustError(format!("mem: {key} holds bytes, not a value"))),
        }
    }

    pub(super) fn get_chunks(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        self.read_ok()?;
        let kv = self.kv.borrow();
        Ok(keys
            .iter()
            .map(|k| match kv.get(k) {
                Some(Stored::Bytes(b)) => Some(b.clone()),
                _ => None,
            })
            .collect())
    }

    pub(super) fn put_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        self.write_ok(key)?;
        self.kv.borrow_mut().insert(key.to_string(), Stored::Bytes(bytes.to_vec()));
        Ok(())
    }

    pub(super) fn put<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.write_ok(key)?;
        let v = serde_json::to_value(value).map_err(|e| Error::RustError(e.to_string()))?;
        self.kv.borrow_mut().insert(key.to_string(), Stored::Json(v));
        Ok(())
    }

    /// The keys currently stored under a prefix, for assertions.
    pub fn keys(&self, prefix: &str) -> Vec<String> {
        self.kv.borrow().keys().filter(|k| k.starts_with(prefix)).cloned().collect()
    }
}

/// The object over a `MemHost`, driven through its real `route` (tests only).
pub struct Harness {
    pub host: Rc<MemHost>,
    pub obj: crate::hubdo::HubImages,
}

/// The clock every harness starts at: 2023-11-14T22:13:20Z.
pub const T0: i64 = 1_700_000_000_000;

impl Harness {
    pub fn new() -> Self {
        Self::over(MemHost::named("v1", T0))
    }
    pub fn over(host: Rc<MemHost>) -> Self {
        Harness { obj: crate::hubdo::HubImages::in_memory(host.clone()), host }
    }
    /// A NEW object over the SAME storage: what an eviction or a redeploy leaves.
    pub fn cold(&self) -> Self {
        Self::over(self.host.clone())
    }
    pub fn try_call(&self, c: crate::wire::Call) -> Result<crate::wire::Reply> {
        crate::edge::mem::block_on(self.obj.route(c))
    }
    pub fn call(&self, c: crate::wire::Call) -> crate::wire::Reply {
        self.try_call(c).expect("the route answered Err")
    }
    pub fn get(&self, path: &str) -> crate::wire::Reply {
        self.call(crate::wire::Call::new(&format!("https://hub{path}"), worker::Method::Get).unwrap())
    }
    pub fn post(&self, path: &str, body: &serde_json::Value) -> crate::wire::Reply {
        self.call(crate::wire::Call::new(&format!("https://hub{path}"), worker::Method::Post).unwrap().with_json(body))
    }
    pub fn put(&self, id: &str, generation: i64, bytes: &[u8]) -> crate::wire::Reply {
        self.call(
            crate::wire::Call::new(&format!("https://hub/img/{id}"), worker::Method::Put)
                .unwrap()
                .with_header("x-generation", &generation.to_string())
                .with_body(bytes.to_vec()),
        )
    }
    /// The x-generation header of a reply, as a number.
    pub fn gen_of(r: &crate::wire::Reply) -> i64 {
        r.headers().get("x-generation").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(-1)
    }
}
