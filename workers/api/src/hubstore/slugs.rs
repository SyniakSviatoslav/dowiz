//! A4 -- WHICH VENUE A PUBLIC SLUG NAMES, WITHOUT ASKING THE PLATFORM OBJECT EVERY TIME (W-CHEAP).
//!
//! THE COST, MEASURED. Every public route (`menu`, `eta`, `context`, `reach`, the bookings, the
//! wallet, placement) resolved its slug through `Place::of_slug`, which loads the WHOLE registry
//! image out of the `__platform` object: one billed Durable Object request per public request,
//! on top of the venue's own. Cloudflare's per-object counts for 2026-10-09 show it plainly: the
//! platform object (dc1b6cc6) took 1,734 requests on a day the three venues took ~870 each, all
//! from the 5-minute monitor's `menu` and `eta` probes (2 slug lookups x 3 venues x 288 ticks).
//!
//! WHY CACHING IS SAFE HERE, AND ONLY FOR THIS SHAPE. A venue is created with `id = slug`
//! (`platform::create_hub`; `bootstrap` defaults the slug to the id), slugs are unique keys in the
//! registry, and no route deletes a venue. So an IDENTITY answer -- "slug s is the venue s" -- can
//! never become WRONG about which object a request reaches: the only later change is the slug
//! being dropped (a bootstrap re-seed under another slug), after which the registry would say
//! "no such venue" and this says "venue s" -- the same object that slug always reached, and a
//! venue created later under that slug would get the id s, i.e. that SAME object again. A
//! non-identity answer (a venue re-seeded with a slug other than its id) has no such guarantee,
//! so it is NEVER remembered, and neither is "unknown" (a venue created a second later must be
//! found). Both go to the registry every time, as before.
//!
//! TWO LAYERS. The isolate's memory (free, lives as long as the isolate), then the colo's edge
//! cache for a day (free; survives the isolate, which the 5-minute monitor needs). Neither is
//! ever written with anything but an identity answer.

use crate::edge::Env;

/// Remembered slugs per isolate before the memory is dropped and refilled (a bound, not a tuning).
pub const MEMO_MAX: usize = 4096;
/// How long the edge cache keeps an identity answer.
pub const EDGE_TTL_S: u32 = 86_400;

/// Is the registry's answer for `slug` one this module may remember? Only "slug s is venue s".
pub fn rememberable(slug: &str, id: &str) -> bool {
    !id.is_empty() && id == norm(slug)
}

/// The slug as the registry's index keys it (`identity_store::loc_by_slug` folds case).
pub fn norm(slug: &str) -> String {
    slug.trim().to_ascii_lowercase()
}

/// The edge-cache key: a URL no route serves, on the platform's own host.
fn edge_key(env: &Env, slug: &str) -> String {
    let host = env.var("PLATFORM_HOST").unwrap_or_else(|_| "dowiz.org".to_string());
    format!("https://{host}/__registry/slug/{}", crate::mcp::enc(&norm(slug)))
}

/// The isolate's memory. In tests it is tied to ONE in-memory world, so one test's venues can
/// never answer for another's (`Scope`).
struct Memo {
    #[cfg(test)]
    world: Option<std::rc::Weak<crate::edge::mem::World>>,
    map: std::collections::HashMap<String, String>,
}

thread_local! {
    static MEMO: std::cell::RefCell<Memo> = std::cell::RefCell::new(Memo {
        #[cfg(test)]
        world: None,
        map: std::collections::HashMap::new(),
    });
}

/// Run `f` over this env's memory, resetting it when the env is another world (tests only).
fn with_memo<T>(env: &Env, f: impl FnOnce(&mut std::collections::HashMap<String, String>) -> T) -> T {
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        #[cfg(test)]
        {
            let now = mem_world(env);
            let same = match (&m.world, &now) {
                (Some(w), Some(n)) => w.upgrade().is_some_and(|w| std::rc::Rc::ptr_eq(&w, n)),
                (None, None) => true,
                _ => false,
            };
            if !same {
                m.map.clear();
                m.world = now.as_ref().map(std::rc::Rc::downgrade);
            }
        }
        #[cfg(not(test))]
        let _ = env;
        f(&mut m.map)
    })
}

#[cfg(test)]
fn mem_world(env: &Env) -> Option<std::rc::Rc<crate::edge::mem::World>> {
    match env {
        Env::Mem(w) => Some(w.clone()),
        Env::Own(o) => mem_world(&o.base),
        Env::Live(_) => None,
    }
}

/// The venue `slug` names, if this isolate or this colo already knows it. Never asks an object.
pub async fn known(env: &Env, slug: &str) -> Option<String> {
    let key = norm(slug);
    if let Some(id) = with_memo(env, |m| m.get(&key).cloned()) {
        return Some(id);
    }
    // The edge cache answers only on the platform (`edge::cache_get` is `None` in memory).
    let mut hit = crate::edge::cache_get(env, &edge_key(env, slug)).await.ok().flatten()?;
    let id = hit.text().await.ok()?;
    if !rememberable(slug, &id) {
        return None; // never trust a cached answer this module would not have written
    }
    with_memo(env, |m| remember_in(m, &key, &id));
    Some(id)
}

/// The registry said `slug` is `id`: remember it when that is an identity answer.
pub async fn learned(env: &Env, slug: &str, id: &str) {
    if !rememberable(slug, id) {
        return;
    }
    let key = norm(slug);
    with_memo(env, |m| remember_in(m, &key, id));
    if let Ok(mut r) = crate::wire::Reply::ok(id.to_string()) {
        let _ = r.headers_mut().set("cache-control", &format!("public, max-age={EDGE_TTL_S}"));
        let _ = crate::edge::cache_put(env, &edge_key(env, slug), &r).await;
    }
}

fn remember_in(m: &mut std::collections::HashMap<String, String>, key: &str, id: &str) {
    if m.len() >= MEMO_MAX {
        m.clear();
    }
    m.insert(key.to_string(), id.to_string());
}

#[cfg(test)]
mod tests;
