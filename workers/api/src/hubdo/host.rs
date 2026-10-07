//! WHERE THE OBJECT LIVES: the platform calls `HubImages` makes, behind one enum (W-COV C2).
//!
//! `Live` is the Durable Object's `State` and `Env`, and every arm of it is the call the object
//! made before this file existed, moved here unchanged. `Mem` (tests only) is a key/value map,
//! an alarm slot, a list of tagged sockets that record what they are sent, and a name — enough
//! for the REAL object code (images, chunking, the generation guard, place, the room, refunds,
//! the till link, the print rail, the menu memo, the alarm) to run under `cargo test`.
//!
//! AN ENUM, NOT A GENERIC: `impl HubImages` is spread over seventeen files, and a type parameter
//! would have to be carried through every one of them for no behaviour. The method names mirror
//! the ones the object already called on `State`/`Storage`, so call sites read the same.

use worker::wasm_bindgen::{JsCast, JsValue};
use worker::*;

pub(super) enum Host {
    Live { state: State, env: Env },
    #[cfg(test)]
    Mem(std::rc::Rc<mem::MemHost>),
}

/// A socket the object can send to.
pub(super) enum Sock {
    Live(WebSocket),
    #[cfg(test)]
    Mem(std::rc::Rc<std::cell::RefCell<Vec<String>>>),
}

impl Sock {
    pub fn send_with_str(&self, msg: &str) -> Result<()> {
        match self {
            Sock::Live(ws) => ws.send_with_str(msg),
            #[cfg(test)]
            Sock::Mem(out) => {
                out.borrow_mut().push(msg.to_string());
                Ok(())
            }
        }
    }
}

/// The object's storage, as the object uses it.
pub(super) enum Store {
    Live(Storage),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::MemHost>),
}

/// A stored chunk comes back as whatever the platform decided to hand us —
/// `Uint8Array` or the `ArrayBuffer` behind one. Accept both rather than assume,
/// because assuming is a corrupt image reported a long way from here.
fn chunk_bytes(v: &JsValue) -> Option<Vec<u8>> {
    if let Some(a) = v.dyn_ref::<js_sys::Uint8Array>() {
        return Some(a.to_vec());
    }
    if let Some(b) = v.dyn_ref::<js_sys::ArrayBuffer>() {
        return Some(js_sys::Uint8Array::new(b).to_vec());
    }
    None
}

/// THE PLATFORM'S CEILING ON ONE `put(entries)`: "Supports up to 128 key-value pairs at a time."
/// https://developers.cloudflare.com/durable-objects/api/storage-api/ (read 2026-10-06, W-ATOMIC).
pub(super) const MAX_KEYS: usize = 128;

/// One `put(entries)`: chunk keys with their bytes, and the meta when this call carries it.
/// The SAME page: puts issued "without performing any `await` in the meantime ... will
/// automatically be combined and submitted atomically", and on a machine failure "either all
/// of the writes will have been stored to disk or none" -- so one call lands whole or not at all.
pub(super) struct Batch<'a, T> {
    pub chunks: Vec<(String, &'a [u8])>,
    pub value: Option<(String, &'a T)>,
}

impl<T> Batch<'_, T> {
    pub fn keys(&self) -> usize {
        self.chunks.len() + usize::from(self.value.is_some())
    }
}

fn over_limit<T>(batches: &[Batch<'_, T>]) -> Result<()> {
    match batches.iter().find(|b| b.keys() > MAX_KEYS) {
        Some(b) => Err(Error::RustError(format!("one storage put of {} keys; the platform takes {MAX_KEYS}", b.keys()))),
        None => Ok(()),
    }
}

impl Store {
    pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self {
            Store::Live(s) => s.get(key).await,
            #[cfg(test)]
            Store::Mem(m) => m.get(key),
        }
    }
    /// Every key in ONE call; `None` where a key holds no bytes.
    pub async fn get_chunks(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        match self {
            Store::Live(s) => {
                let got = s.get_multiple(keys.to_vec()).await?;
                Ok(keys.iter().map(|k| chunk_bytes(&got.get(&JsValue::from_str(k)))).collect())
            }
            #[cfg(test)]
            Store::Mem(m) => m.get_chunks(keys),
        }
    }
    /// ONE `put(entries)`, so it lands whole or not at all; refused unsent above `MAX_KEYS`.
    pub async fn put_batch<T: serde::Serialize>(&self, batch: &Batch<'_, T>) -> Result<()> {
        self.put_together(std::slice::from_ref(batch)).await
    }
    /// Several `put(entries)` ISSUED IN ONE TURN, no await between them, which the platform
    /// combines into one atomic write (the page above). Every call's size is checked BEFORE any
    /// is issued: a refusal half-way would be exactly the partial write this exists to prevent.
    pub async fn put_together<T: serde::Serialize>(&self, batches: &[Batch<'_, T>]) -> Result<()> {
        over_limit(batches)?;
        match self {
            Store::Live(s) => {
                let mut objs = Vec::with_capacity(batches.len());
                for b in batches {
                    let obj = js_sys::Object::new();
                    for (k, bytes) in &b.chunks {
                        js_sys::Reflect::set(&obj, &JsValue::from_str(k), &js_sys::Uint8Array::from(*bytes))?;
                    }
                    if let Some((k, v)) = &b.value {
                        // A plain object, as `put(key, value)` stores one: `get::<Meta>` reads either.
                        let json = serde_json::to_string(v).map_err(|e| Error::RustError(e.to_string()))?;
                        js_sys::Reflect::set(&obj, &JsValue::from_str(k), &js_sys::JSON::parse(&json)?)?;
                    }
                    objs.push(obj);
                }
                // `join_all` polls every future once, in order, before it yields: each call reaches
                // `storage.put` in that first poll, so no await separates them.
                let done = futures_util::future::join_all(objs.into_iter().map(|o| s.put_multiple_raw(o))).await;
                done.into_iter().collect::<Result<Vec<()>>>().map(|_| ())
            }
            #[cfg(test)]
            Store::Mem(m) => {
                let mut all = Vec::new();
                for b in batches {
                    for (k, bytes) in &b.chunks {
                        all.push((k.clone(), mem::Stored::Bytes(bytes.to_vec())));
                    }
                    if let Some((k, v)) = &b.value {
                        let v = serde_json::to_value(v).map_err(|e| Error::RustError(e.to_string()))?;
                        all.push((k.clone(), mem::Stored::Json(v)));
                    }
                }
                m.put_all(all)
            }
        }
    }
    pub async fn put<T: serde::Serialize>(&self, key: &str, value: T) -> Result<()> {
        match self {
            Store::Live(s) => s.put(key, value).await,
            #[cfg(test)]
            Store::Mem(m) => m.put(key, &value),
        }
    }
    pub async fn delete(&self, key: &str) -> Result<bool> {
        match self {
            Store::Live(s) => s.delete(key).await,
            #[cfg(test)]
            Store::Mem(m) => Ok(m.kv.borrow_mut().remove(key).is_some()),
        }
    }
    pub async fn get_alarm(&self) -> Result<Option<i64>> {
        match self {
            Store::Live(s) => s.get_alarm().await,
            #[cfg(test)]
            Store::Mem(m) => Ok(m.alarm.get()),
        }
    }
    pub async fn set_alarm_ms(&self, at: i64) -> Result<()> {
        match self {
            Store::Live(s) => s.set_alarm(ScheduledTime::new(js_sys::Date::new(&(at as f64).into()))).await,
            #[cfg(test)]
            Store::Mem(m) => {
                m.alarm.set(Some(at));
                Ok(())
            }
        }
    }
    pub async fn delete_alarm(&self) -> Result<()> {
        match self {
            Store::Live(s) => s.delete_alarm().await,
            #[cfg(test)]
            Store::Mem(m) => {
                m.alarm.set(None);
                Ok(())
            }
        }
    }
}

impl Host {
    pub fn storage(&self) -> Store {
        match self {
            Host::Live { state, .. } => Store::Live(state.storage()),
            #[cfg(test)]
            Host::Mem(m) => Store::Mem(m.clone()),
        }
    }
    pub fn get_websockets_with_tag(&self, tag: &str) -> Vec<Sock> {
        match self {
            Host::Live { state, .. } => state.get_websockets_with_tag(tag).into_iter().map(Sock::Live).collect(),
            #[cfg(test)]
            Host::Mem(m) => m.tagged(tag),
        }
    }
    /// Is `venue` this object's name? `id_from_name` says, and nothing else.
    pub fn is_me(&self, venue: &str) -> bool {
        match self {
            Host::Live { state, env } => {
                let Ok(ns) = env.durable_object("HUB") else { return false };
                !venue.is_empty() && ns.id_from_name(venue).is_ok_and(|id| id.to_string() == state.id().to_string())
            }
            #[cfg(test)]
            Host::Mem(m) => !venue.is_empty() && m.name.as_deref() == Some(venue),
        }
    }
    /// The name the platform addressed this object by, when it kept one.
    pub fn own_name(&self) -> Option<String> {
        match self {
            Host::Live { state, .. } => state.id().name(),
            #[cfg(test)]
            Host::Mem(_) => None,
        }
    }
    pub fn secret(&self, name: &str) -> Option<String> {
        match self {
            Host::Live { env, .. } => env.secret(name).ok().map(|v| v.to_string()),
            #[cfg(test)]
            Host::Mem(m) => m.secrets.get(name).cloned(),
        }
    }
    /// A test's fixed clock; the live object reads the platform's (`hubdo.rs`, clock-gated).
    pub fn fixed_clock(&self) -> Option<i64> {
        match self {
            Host::Live { .. } => None,
            #[cfg(test)]
            Host::Mem(m) => Some(m.clock.get()),
        }
    }
    pub fn env(&self) -> Option<&Env> {
        match self {
            Host::Live { env, .. } => Some(env),
            #[cfg(test)]
            Host::Mem(_) => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod mem;
