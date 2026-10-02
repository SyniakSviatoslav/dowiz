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
    pub async fn put_bytes(&self, key: &str, bytes: &[u8]) -> Result<()> {
        match self {
            Store::Live(s) => s.put_raw(key, js_sys::Uint8Array::from(bytes)).await,
            #[cfg(test)]
            Store::Mem(m) => m.put_bytes(key, bytes),
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
    /// POST the venue's runner (`cron~<venue>`) and answer its status.
    pub async fn call_runner(&self, venue: &str, now_ms: i64) -> Result<u16> {
        match self {
            Host::Live { env, .. } => {
                let ns = env.durable_object("HUB")?;
                let stub = ns.id_from_name(&crate::cron::runner_name(venue))?.get_stub()?;
                let req = Request::new_with_init(
                    &crate::cron::runner_path(venue, now_ms),
                    RequestInit::new().with_method(Method::Post),
                )?;
                Ok(stub.fetch_with_request(req).await?.status_code())
            }
            #[cfg(test)]
            Host::Mem(m) => {
                m.runner_calls.borrow_mut().push((venue.to_string(), now_ms));
                Ok(m.runner_status.get())
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod mem;
