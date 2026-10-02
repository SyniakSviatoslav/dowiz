//! THE ROUTE SEAM (W-COV C2): the platform bindings a handler touches, behind enums whose `Live`
//! arm is the `worker` call it always was and whose `Mem` arm (tests only) is a world of
//! in-process Durable Objects -- the REAL `HubImages`, over memory, reached through its `route`.
//!
//! A handler file imports these under the worker names (`Request`, `Response`, `RouteContext`,
//! `Env`, `Stub`, ...) after `use worker::*;`; an explicit import shadows the glob, so its body
//! does not change. The router adapts at one place, [`run`]. Bindings with no native meaning
//! (D1, KV, R2, e-mail, an outbound `fetch`) are reached through [`Env::live`], which refuses
//! loudly in memory rather than pretending.

use crate::wire::{Call, Reply};
use worker::{Error, Result};

#[cfg(test)]
pub(crate) mod mem;
/// A platform in memory for route tests (`Site`).
#[cfg(test)]
pub(crate) mod site;

/// `worker::Env`.
#[derive(Clone)]
pub enum Env {
    Live(worker::Env),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::World>),
}

#[cfg_attr(not(test), allow(dead_code))]
fn not_here(what: &str) -> Error {
    Error::RustError(format!("{what}: no such binding outside the platform"))
}

impl Env {
    pub fn secret(&self, binding: &str) -> Result<String> {
        match self {
            Env::Live(e) => e.secret(binding).map(|s| s.to_string()),
            #[cfg(test)]
            Env::Mem(w) => w.secrets.get(binding).cloned().ok_or_else(|| not_here(binding)),
        }
    }
    pub fn var(&self, binding: &str) -> Result<String> {
        match self {
            Env::Live(e) => e.var(binding).map(|s| s.to_string()),
            #[cfg(test)]
            Env::Mem(w) => w.vars.get(binding).cloned().ok_or_else(|| not_here(binding)),
        }
    }
    pub fn durable_object(&self, binding: &str) -> Result<ObjectNamespace> {
        match self {
            Env::Live(e) => e.durable_object(binding).map(ObjectNamespace::Live),
            #[cfg(test)]
            Env::Mem(w) => Ok(ObjectNamespace::Mem(w.clone())),
        }
    }
    /// The platform's own `Env`, for the bindings that have no native meaning.
    pub fn live(&self) -> Result<&worker::Env> {
        match self {
            Env::Live(e) => Ok(e),
            #[cfg(test)]
            Env::Mem(_) => Err(not_here("the platform Env")),
        }
    }
    pub fn kv(&self, binding: &str) -> Result<Kv> {
        match self {
            Env::Live(e) => e.kv(binding).map(Kv::Live).map_err(|e| Error::RustError(e.to_string())),
            #[cfg(test)]
            Env::Mem(w) => Ok(Kv::Mem(w.clone())),
        }
    }
    pub fn bucket(&self, binding: &str) -> Result<worker::Bucket> {
        self.live()?.bucket(binding)
    }
}

/// `worker::ObjectNamespace`.
#[derive(Clone)]
pub enum ObjectNamespace {
    Live(worker::ObjectNamespace),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::World>),
}

/// `worker::ObjectId`, owning its namespace (the platform's borrows it).
pub struct ObjectId {
    ns: ObjectNamespace,
    name: String,
}

impl ObjectNamespace {
    pub fn id_from_name(&self, name: &str) -> Result<ObjectId> {
        Ok(ObjectId { ns: self.clone(), name: name.to_string() })
    }
}

impl std::fmt::Display for ObjectId {
    /// The platform's hex id when live; the name in memory (one object per name either way).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.ns {
            ObjectNamespace::Live(ns) => match ns.id_from_name(&self.name) {
                Ok(id) => write!(f, "{id}"),
                Err(_) => write!(f, "{}", self.name),
            },
            #[cfg(test)]
            ObjectNamespace::Mem(_) => write!(f, "{}", self.name),
        }
    }
}

impl ObjectId {
    pub fn get_stub(&self) -> Result<Stub> {
        match &self.ns {
            ObjectNamespace::Live(ns) => ns.id_from_name(&self.name)?.get_stub().map(Stub::Live),
            #[cfg(test)]
            ObjectNamespace::Mem(w) => Ok(Stub::Mem(w.object(&self.name))),
        }
    }
}

/// `worker::Stub`: a call to one venue's object.
pub enum Stub {
    Live(worker::Stub),
    #[cfg(test)]
    Mem(std::rc::Rc<crate::hubdo::HubImages>),
}

impl Stub {
    pub async fn fetch_with_str(&self, url: &str) -> Result<Reply> {
        self.fetch_with_request(Call::new(url, worker::Method::Get)?).await
    }
    pub async fn fetch_with_request(&self, req: Call) -> Result<Reply> {
        match self {
            Stub::Live(s) => Reply::from_worker(s.fetch_with_request(req.into_worker()?).await?).await,
            #[cfg(test)]
            Stub::Mem(o) => o.route(req).await,
        }
    }
}

/// `worker::RouteContext`: the request's data, the bindings, and the path's parameters.
pub struct Ctx<D> {
    pub data: D,
    pub env: Env,
    params: Params<D>,
}

enum Params<D> {
    Live(worker::RouteContext<D>),
    #[cfg(test)]
    Mem(Vec<(String, String)>),
}

impl<D: Clone> Ctx<D> {
    pub fn from_worker(ctx: worker::RouteContext<D>) -> Self {
        Ctx { data: ctx.data.clone(), env: Env::Live(ctx.env.clone()), params: Params::Live(ctx) }
    }
}

impl<D> Ctx<D> {
    #[cfg(test)]
    pub fn mem(data: D, env: Env, params: &[(&str, &str)]) -> Self {
        let params = params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Ctx { data, env, params: Params::Mem(params) }
    }
    pub fn param(&self, key: &str) -> Option<&String> {
        match &self.params {
            Params::Live(c) => c.param(key),
            #[cfg(test)]
            Params::Mem(p) => p.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        }
    }
    pub fn var(&self, binding: &str) -> Result<String> {
        self.env.var(binding)
    }
    pub fn durable_object(&self, binding: &str) -> Result<ObjectNamespace> {
        self.env.durable_object(binding)
    }
    pub fn kv(&self, binding: &str) -> Result<Kv> {
        self.env.kv(binding)
    }
}

/// `worker::kv::KvStore`, over the four calls the media routes make (W-COV C2): the Live arm is
/// the platform's builder chain, unchanged; in a test the World holds the bytes.
pub enum Kv {
    Live(worker::kv::KvStore),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::World>),
}

impl Kv {
    pub async fn put_bytes(&self, key: &str, value: &[u8]) -> Result<()> {
        match self {
            Kv::Live(k) => Ok(k.put_bytes(key, value)?.execute().await?),
            #[cfg(test)]
            Kv::Mem(w) => {
                w.kv.borrow_mut().insert(key.to_string(), value.to_vec());
                Ok(())
            }
        }
    }
    pub async fn put_text(&self, key: &str, value: &str) -> Result<()> {
        match self {
            Kv::Live(k) => Ok(k.put(key, value)?.execute().await?),
            #[cfg(test)]
            Kv::Mem(w) => {
                w.kv.borrow_mut().insert(key.to_string(), value.as_bytes().to_vec());
                Ok(())
            }
        }
    }
    pub async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self {
            Kv::Live(k) => Ok(k.get(key).bytes().await?),
            #[cfg(test)]
            Kv::Mem(w) => Ok(w.kv.borrow().get(key).cloned()),
        }
    }
    pub async fn get_text(&self, key: &str) -> Result<Option<String>> {
        match self {
            Kv::Live(k) => Ok(k.get(key).text().await?),
            #[cfg(test)]
            Kv::Mem(w) => Ok(w.kv.borrow().get(key).map(|b| String::from_utf8_lossy(b).into_owned())),
        }
    }
}

/// AN OUTBOUND CALL (Telegram, Stripe, S3, an AI provider, ...): the platform's `fetch`, read
/// whole into a `Reply`. In a test, a thread's hook answers instead (`mem::outbound`); with no
/// hook the call is an `Err`, as an unreachable provider is.
pub async fn fetch(req: Call) -> Result<Reply> {
    #[cfg(test)]
    if let Some(answer) = mem::outbound(&req) {
        return answer;
    }
    Reply::from_worker(worker::Fetch::Request(req.into_worker()?).send().await?).await
}

/// `worker::Date`, for `Date::now().as_millis()`: the platform's clock on wasm32, the system's
/// natively (a native test that reaches a handler's own clock read must not panic on a JS import).
pub struct Date(u64);

impl Date {
    pub fn now() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Date(worker::Date::now().as_millis())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            Date(d.as_millis() as u64)
        }
    }
    pub fn as_millis(&self) -> u64 {
        self.0
    }
}

/// The platform's edge cache, as a `Reply`. In memory there is no cache: a miss, and a put
/// that stores nothing -- which is what a cold colo answers too.
pub async fn cache_get(env: &Env, key: &str) -> Result<Option<Reply>> {
    if env.live().is_err() {
        return Ok(None);
    }
    match worker::Cache::default().get(key, false).await? {
        Some(hit) => Ok(Some(Reply::from_worker(hit).await?)),
        None => Ok(None),
    }
}

pub async fn cache_put(env: &Env, key: &str, res: &Reply) -> Result<()> {
    if env.live().is_err() {
        return Ok(());
    }
    worker::Cache::default().put(key, res.clone().into_response()?).await
}

/// THE ROUTER'S ADAPTER: the platform's request and context in, the handler run over the
/// native ones, the platform's response out. Every converted route goes through here.
pub async fn run<D, F, Fut>(req: worker::Request, ctx: worker::RouteContext<D>, handler: F) -> Result<worker::Response>
where
    D: Clone,
    F: FnOnce(Call, Ctx<D>) -> Fut,
    Fut: std::future::Future<Output = Result<Reply>>,
{
    let call = Call::from_worker(req).await?;
    handler(call, Ctx::from_worker(ctx)).await?.into_response()
}
