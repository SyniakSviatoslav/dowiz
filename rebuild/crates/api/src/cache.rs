//! Generic in-process TTL + stale-while-revalidate (SWR) + stale-on-error cache.
//!
//! Ports the *shape* of the Node public-menu/location-info cache
//! (`apps/api/src/routes/public/menu.ts:76-111`, `:104-111`, `:317-330` — "the storefront-blink
//! fix"): a customer burst against a hot read collapses into ONE upstream call per key per fresh
//! window; an expired-but-recent entry is served INSTANTLY while a single deduped refresh runs in
//! the background; a refresh that errors falls back to a recent-enough stale entry rather than
//! surfacing a 5xx to the customer.
//!
//! Generic over the cached value `V` so both S1 follow-up call sites (menu jsonb, location-info
//! row) share one implementation — `crates/api/src/repo.rs`'s `CachedRepo` wraps `PublicRepo`
//! with two instances of this (see that module for the wiring + the cache-BOUNDARY parity note).
//!
//! Node-parity note (SWR window): Node's OWN location-info cache has no SWR window — it BLOCKS on
//! a fresh refresh once `freshUntil` passes (`menu.ts:301-310`, no `staleUntil` field on
//! `InfoCacheEntry` at all), only ever serving stale data on a refresh ERROR. This build gives
//! location-info caching a genuine SWR window too, matching this follow-up's brief ("fresh TTL,
//! stale-while-revalidate window, stale-on-error fallback") uniformly for BOTH call sites rather
//! than porting two subtly different cache shapes. Strictly more available than Node (never
//! serves data staler than Node would within the same TTL, and removes the exact blocking-refresh
//! burst risk the menu cache was built to fix) — flagged here, not silently diverged, for whoever
//! finalizes a live deploy.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// What `get_or_refresh` actually served, so a caller can set a `X-*-Cache: stale-on-error`
/// response header the way Node does (`menu.ts:255`, `:324`) without this module knowing
/// anything about HTTP. `Hit` covers BOTH a fresh serve and a stale-but-usable SWR serve — Node
/// sets no distinguishing header for either of those, only for the error-fallback path.
#[derive(Debug, Clone, PartialEq)]
pub enum CacheOutcome<V> {
    /// Fresh cache hit, a stale-but-usable SWR hit (background refresh kicked off), or a
    /// cold/expired refresh that just succeeded.
    Hit(V),
    /// Served from a cache entry beyond every normal window ONLY because the refresh attempt
    /// that would have replaced it failed (`menu.ts:248-256`, `:320-325`).
    StaleOnError(V),
    /// The refresh succeeded but reported "nothing here" (`Ok(None)`) — never cached (Node never
    /// caches a null `read_public_menu`/`read_preview_menu` result either, `menu.ts:118,127`; a
    /// call site that DOES want a "not found" result cached, like location-info, wraps its own
    /// `Option` an extra layer deep — see `repo.rs`'s `CachedRepo::location_info`).
    Miss,
}

struct Entry<V> {
    payload: V,
    born_at: Instant,
}

/// One `(key -> V)` cache with three time windows anchored to an entry's `born_at` (the moment
/// its underlying refresh last SUCCEEDED — matches Node's `bornAt`):
///   `[0, fresh_ttl)`          fresh — served with zero refresh call.
///   `[fresh_ttl, stale_ttl)`  stale-but-usable — served immediately, a refresh is kicked off in
///                             the background (deduped per key, matches Node's
///                             `menuInflight`/`infoInflight` Maps).
///   `[0, stale_on_error_ttl)` usable as a fallback ONLY when a refresh attempt errors — an
///                             independent, normally-WIDER window (Node: 1h vs. 300s) checked
///                             regardless of whether the entry is still within `stale_ttl`.
pub struct TtlSwrCache<V> {
    entries: Mutex<HashMap<String, Entry<V>>>,
    refreshing: Mutex<HashSet<String>>,
    fresh_ttl: Duration,
    stale_ttl: Duration,
    stale_on_error_ttl: Duration,
    max_entries: usize,
}

impl<V> TtlSwrCache<V>
where
    V: Clone + Send + Sync + 'static,
{
    pub fn new(
        fresh_ttl: Duration,
        stale_ttl: Duration,
        stale_on_error_ttl: Duration,
        max_entries: usize,
    ) -> Self {
        TtlSwrCache {
            entries: Mutex::new(HashMap::new()),
            refreshing: Mutex::new(HashSet::new()),
            fresh_ttl,
            stale_ttl,
            stale_on_error_ttl,
            max_entries,
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// `refresh` is the upstream call (a DB query, in production) — invoked at most once per
    /// cold/expired request (blocking), or once per key in the background while stale-serving.
    /// `Ok(Some(v))` caches `v`; `Ok(None)` is a genuine miss and is NEVER cached (matches Node);
    /// `Err(e)` triggers the stale-on-error fallback if a recent-enough entry exists, else
    /// propagates `e` untouched (matches "no usable cache -> surface the failure",
    /// `menu.ts:327-328`'s typed 503).
    pub async fn get_or_refresh<F, Fut, E>(
        self: &Arc<Self>,
        key: &str,
        refresh: F,
    ) -> Result<CacheOutcome<V>, E>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Option<V>, E>> + Send + 'static,
        E: Send + 'static,
    {
        let now = Instant::now();
        let snapshot = self.snapshot(key);

        if let Some((payload, born_at)) = &snapshot {
            if now.duration_since(*born_at) < self.fresh_ttl {
                return Ok(CacheOutcome::Hit(payload.clone()));
            }
            if now.duration_since(*born_at) < self.stale_ttl {
                self.spawn_background_refresh(key, refresh);
                return Ok(CacheOutcome::Hit(payload.clone()));
            }
        }

        // Cold, or past the stale-serve window: block on a single refresh.
        match refresh().await {
            Ok(Some(v)) => {
                self.insert(key, v.clone());
                Ok(CacheOutcome::Hit(v))
            }
            Ok(None) => Ok(CacheOutcome::Miss),
            Err(err) => {
                if let Some((payload, born_at)) = snapshot {
                    if now.duration_since(born_at) < self.stale_on_error_ttl {
                        return Ok(CacheOutcome::StaleOnError(payload));
                    }
                }
                Err(err)
            }
        }
    }

    fn snapshot(&self, key: &str) -> Option<(V, Instant)> {
        self.lock_entries()
            .get(key)
            .map(|e| (e.payload.clone(), e.born_at))
    }

    fn insert(&self, key: &str, payload: V) {
        let mut entries = self.lock_entries();
        if !entries.contains_key(key) && entries.len() >= self.max_entries {
            // FIFO eviction on insert past the cap (`menu.ts:198-200`, `:292-294` — bounds
            // memory against a caller-controlled key space, e.g. `?locale=<counter>`).
            if let Some(oldest) = entries.keys().next().cloned() {
                entries.remove(&oldest);
            }
        }
        entries.insert(
            key.to_string(),
            Entry {
                payload,
                born_at: Instant::now(),
            },
        );
    }

    fn spawn_background_refresh<F, Fut, E>(self: &Arc<Self>, key: &str, refresh: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Option<V>, E>> + Send + 'static,
        E: Send + 'static,
    {
        {
            let mut refreshing = self
                .refreshing
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if !refreshing.insert(key.to_string()) {
                return; // a background refresh for this key is already in flight (Node's dedup)
            }
        }
        let this = Arc::clone(self);
        let key = key.to_string();
        tokio::spawn(async move {
            // Mirrors Node's `.catch(() => null)` on the background refresh: a failure here just
            // leaves the existing (now further-aged) entry in place for the next request to deal
            // with, rather than propagating anywhere — there is no caller left to hear about it.
            if let Ok(Some(v)) = refresh().await {
                this.insert(&key, v);
            }
            this.refreshing
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&key);
        });
    }

    fn lock_entries(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry<V>>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_refresh(
        counter: Arc<AtomicUsize>,
        value: i32,
    ) -> impl FnOnce() -> std::future::Ready<Result<Option<i32>, String>> {
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(Some(value)))
        }
    }

    #[tokio::test]
    async fn fresh_hit_never_calls_refresh() {
        let cache = Arc::new(TtlSwrCache::new(
            Duration::from_secs(30),
            Duration::from_secs(300),
            Duration::from_secs(3600),
            500,
        ));
        let calls = Arc::new(AtomicUsize::new(0));

        let first = cache
            .get_or_refresh("k", counting_refresh(calls.clone(), 1))
            .await
            .unwrap();
        assert_eq!(first, CacheOutcome::Hit(1));

        // Still within fresh_ttl: must NOT call refresh again.
        let second = cache
            .get_or_refresh("k", counting_refresh(calls.clone(), 999))
            .await
            .unwrap();
        assert_eq!(
            second,
            CacheOutcome::Hit(1),
            "must serve the cached value, not re-refresh"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "refresh must run exactly once"
        );
    }

    #[tokio::test]
    async fn stale_within_swr_window_serves_stale_and_refreshes_in_background() {
        let cache = Arc::new(TtlSwrCache::new(
            Duration::from_millis(10),
            Duration::from_millis(500),
            Duration::from_secs(3600),
            500,
        ));
        let calls = Arc::new(AtomicUsize::new(0));

        cache
            .get_or_refresh("k", counting_refresh(calls.clone(), 1))
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        tokio::time::sleep(Duration::from_millis(30)).await; // past fresh_ttl, within stale_ttl

        let outcome = cache
            .get_or_refresh("k", counting_refresh(calls.clone(), 2))
            .await
            .unwrap();
        assert_eq!(
            outcome,
            CacheOutcome::Hit(1),
            "must serve the OLD value immediately, not block on the refresh"
        );

        // Let the deduped background refresh run and land.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "the background refresh must have run exactly once"
        );
        let refreshed = cache
            .get_or_refresh("k", counting_refresh(calls.clone(), 999))
            .await
            .unwrap();
        assert_eq!(
            refreshed,
            CacheOutcome::Hit(2),
            "subsequent reads must see the value the background refresh installed"
        );
    }

    #[tokio::test]
    async fn refresh_error_past_swr_window_serves_stale_on_error_not_the_error() {
        let cache = Arc::new(TtlSwrCache::new(
            Duration::from_millis(5),
            Duration::from_millis(10),
            Duration::from_secs(3600),
            500,
        ));

        cache
            .get_or_refresh("k", || std::future::ready(Ok::<_, String>(Some(1))))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await; // past stale_ttl -> cold/blocking path

        let outcome: Result<CacheOutcome<i32>, String> = cache
            .get_or_refresh("k", || {
                std::future::ready(Err("db unavailable".to_string()))
            })
            .await;

        assert_eq!(
            outcome,
            Ok(CacheOutcome::StaleOnError(1)),
            "a refresh failure with a recent-enough entry must serve stale, never bubble the error"
        );
    }

    #[tokio::test]
    async fn refresh_error_with_no_entry_at_all_propagates() {
        let cache: Arc<TtlSwrCache<i32>> = Arc::new(TtlSwrCache::new(
            Duration::from_secs(30),
            Duration::from_secs(300),
            Duration::from_secs(3600),
            500,
        ));

        let outcome = cache
            .get_or_refresh("unknown-key", || {
                std::future::ready(Err::<Option<i32>, _>("db unavailable".to_string()))
            })
            .await;

        assert_eq!(
            outcome,
            Err("db unavailable".to_string()),
            "with nothing cached at all there is no stale fallback — the error must surface \
             (this is what lets `get_public_location_info` still return its typed 503)"
        );
    }

    #[tokio::test]
    async fn refresh_ok_none_is_a_miss_and_is_never_cached() {
        let cache: Arc<TtlSwrCache<i32>> = Arc::new(TtlSwrCache::new(
            Duration::from_secs(30),
            Duration::from_secs(300),
            Duration::from_secs(3600),
            500,
        ));

        let outcome = cache
            .get_or_refresh("k", || std::future::ready(Ok::<Option<i32>, String>(None)))
            .await
            .unwrap();
        assert_eq!(outcome, CacheOutcome::Miss);
        assert_eq!(cache.len(), 0, "a miss must never occupy a cache slot");
    }

    #[tokio::test]
    async fn max_entries_evicts_fifo_on_overflow() {
        let cache: Arc<TtlSwrCache<i32>> = Arc::new(TtlSwrCache::new(
            Duration::from_secs(30),
            Duration::from_secs(300),
            Duration::from_secs(3600),
            2,
        ));

        cache
            .get_or_refresh("a", || std::future::ready(Ok::<_, String>(Some(1))))
            .await
            .unwrap();
        cache
            .get_or_refresh("b", || std::future::ready(Ok::<_, String>(Some(2))))
            .await
            .unwrap();
        assert_eq!(cache.len(), 2);

        cache
            .get_or_refresh("c", || std::future::ready(Ok::<_, String>(Some(3))))
            .await
            .unwrap();
        assert_eq!(cache.len(), 2, "must stay bounded at max_entries");
    }

    #[tokio::test]
    async fn distinct_keys_are_independent() {
        let cache = Arc::new(TtlSwrCache::new(
            Duration::from_secs(30),
            Duration::from_secs(300),
            Duration::from_secs(3600),
            500,
        ));
        let calls = Arc::new(AtomicUsize::new(0));

        let a = cache
            .get_or_refresh("a", counting_refresh(calls.clone(), 10))
            .await
            .unwrap();
        let b = cache
            .get_or_refresh("b", counting_refresh(calls.clone(), 20))
            .await
            .unwrap();

        assert_eq!(a, CacheOutcome::Hit(10));
        assert_eq!(b, CacheOutcome::Hit(20));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "each key must trigger its own refresh"
        );
    }
}
