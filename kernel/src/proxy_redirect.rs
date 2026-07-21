//! proxy_redirect.rs — Proxy pool management, rotation strategies, and health tracking.
//!
//! # What this is
//! Pure data structures for managing a pool of proxy endpoints used by the
//! anti-detect browser for parse operations. Each parse call is routed through
//! a different proxy (or chain) to prevent IP correlation. This module contains
//! NO network I/O — it is the kernel's routing authority. The actual proxy
//! connection lives in the external `AgentBrowserPort` implementation.
//!
//! # Design principles
//! - Kernel = pure computation (no network sockets)
//! - Proxy selection is deterministic given (pool_state, call_index)
//! - Health tracking prevents routing through dead proxies
//! - Rotation strategies are typed (no silent behavioral changes)

use crate::event_log::sha3_256;
use crate::TriState;

/// A single proxy endpoint.
#[derive(Debug, Clone)]
pub struct ProxyEndpoint {
    /// Hostname or IP.
    pub host: String,
    /// Port number.
    pub port: u16,
    /// Proxy protocol.
    pub protocol: ProxyProtocol,
    /// Authentication (if required).
    pub auth: Option<ProxyAuth>,
    /// Geographic region code (e.g. "US-East", "EU-West").
    pub region: String,
    /// When this proxy was last successfully used (unix microseconds).
    pub last_success_us: u64,
    /// When this proxy last failed (unix microseconds).
    pub last_failure_us: u64,
    /// Number of consecutive failures since last success.
    pub consecutive_failures: u32,
    /// Total requests made through this proxy.
    pub total_requests: u64,
    /// Total successful requests.
    pub total_successes: u64,
    /// Average response latency (microseconds).
    pub avg_latency_us: u64,
}

impl ProxyEndpoint {
    /// Success rate of this proxy (0.0..1.0).
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_successes as f64 / self.total_requests as f64
        }
    }

    /// Whether this proxy is considered healthy.
    pub fn is_healthy(&self) -> TriState {
        TriState::from_bool(self.consecutive_failures < 3 && self.success_rate() > 0.5)
    }

    /// Record a successful request.
    pub fn record_success(&mut self, latency_us: u64) {
        self.consecutive_failures = 0;
        self.total_requests += 1;
        self.total_successes += 1;
        self.last_success_us = now_us();
        // Exponential moving average for latency.
        if self.avg_latency_us == 0 {
            self.avg_latency_us = latency_us;
        } else {
            self.avg_latency_us = (self.avg_latency_us * 7 + latency_us) / 8;
        }
    }

    /// Record a failed request.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.total_requests += 1;
        self.last_failure_us = now_us();
    }
}

/// Proxy protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProxyProtocol {
    Http,
    Https,
    Socks5,
}

/// Proxy authentication.
#[derive(Debug, Clone)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

/// Rotation strategy for proxy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RotationStrategy {
    /// Round-robin through the pool (simple, predictable).
    RoundRobin,
    /// Weighted random — healthy proxies get higher weight.
    WeightedRandom,
    /// Geographic routing — select proxy matching target's region.
    GeoRouting,
    /// Least-latency — prefer the proxy with lowest avg latency.
    LeastLatency,
    /// Rotating chains — route through 2+ proxies in sequence (highest anonymity).
    Chain(usize), // chain length
}

/// The proxy pool — manages all available endpoints.
#[derive(Debug)]
pub struct ProxyPool {
    /// All proxy endpoints.
    endpoints: Vec<ProxyEndpoint>,
    /// Current rotation index (for round-robin).
    round_robin_index: usize,
    /// The active rotation strategy.
    strategy: RotationStrategy,
    /// Maximum chain length (for Chain strategy).
    max_chain_length: usize,
}

impl ProxyPool {
    /// Create an empty pool.
    pub fn new(strategy: RotationStrategy) -> Self {
        ProxyPool {
            endpoints: Vec::new(),
            round_robin_index: 0,
            strategy,
            max_chain_length: 4,
        }
    }

    /// Add a proxy endpoint to the pool.
    pub fn add(&mut self, endpoint: ProxyEndpoint) {
        self.endpoints.push(endpoint);
    }

    /// Select the next proxy based on the current strategy.
    ///
    /// Returns `None` if no healthy proxies are available (caller should degrade).
    pub fn select(&mut self, call_index: u64) -> Option<&ProxyEndpoint> {
        if self.endpoints.is_empty() {
            return None;
        }

        match self.strategy {
            RotationStrategy::RoundRobin => self.select_round_robin(),
            RotationStrategy::WeightedRandom => self.select_weighted_random(call_index),
            RotationStrategy::GeoRouting => self.select_geo_routing(call_index),
            RotationStrategy::LeastLatency => self.select_least_latency(),
            RotationStrategy::Chain(len) => {
                self.max_chain_length = len;
                self.select_round_robin() // base selection; chain is assembled by caller
            }
        }
    }

    /// Select next in round-robin order.
    fn select_round_robin(&mut self) -> Option<&ProxyEndpoint> {
        let len = self.endpoints.len();
        if len == 0 {
            return None;
        }
        // Skip unhealthy proxies.
        for _ in 0..len {
            let idx = self.round_robin_index % len;
            self.round_robin_index += 1;
            if self.endpoints[idx].is_healthy().is_true() {
                return Some(&self.endpoints[idx]);
            }
        }
        None
    }

    /// Select by weighted random (healthier proxies = higher weight).
    fn select_weighted_random(&self, call_index: u64) -> Option<&ProxyEndpoint> {
        let healthy: Vec<(usize, f64)> = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_, ep)| ep.is_healthy().is_true())
            .map(|(i, ep)| {
                let weight = ep.success_rate() * 100.0 + 1.0;
                (i, weight)
            })
            .collect();

        if healthy.is_empty() {
            return None;
        }

        let total_weight: f64 = healthy.iter().map(|(_, w)| w).sum();
        // Deterministic "random" from call_index (not OS RNG).
        let pick = (call_index as f64 * 7919.0) % total_weight; // prime multiplier
        let mut accum = 0.0;
        for (idx, weight) in &healthy {
            accum += weight;
            if pick < accum {
                return Some(&self.endpoints[*idx]);
            }
        }
        // Fallback to last.
        healthy.last().map(|(idx, _)| &self.endpoints[*idx])
    }

    /// Select by geographic routing (simple: pick the region from call_index).
    fn select_geo_routing(&self, call_index: u64) -> Option<&ProxyEndpoint> {
        // Use call_index to select a region, then pick the best proxy in that region.
        let regions: Vec<&str> = self
            .endpoints
            .iter()
            .filter(|ep| ep.is_healthy().is_true())
            .map(|ep| ep.region.as_str())
            .collect();
        if regions.is_empty() {
            return None;
        }
        let region_idx = (call_index as usize) % regions.len();
        let target_region = regions[region_idx];

        // Pick the best proxy in that region.
        self.endpoints
            .iter()
            .filter(|ep| ep.is_healthy().is_true() && ep.region == target_region)
            .max_by(|a, b| {
                a.success_rate()
                    .partial_cmp(&b.success_rate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Select by least latency.
    fn select_least_latency(&self) -> Option<&ProxyEndpoint> {
        self.endpoints
            .iter()
            .filter(|ep| ep.is_healthy().is_true())
            .min_by_key(|ep| ep.avg_latency_us)
    }

    /// Record a success for the proxy at the given index.
    pub fn record_success(&mut self, index: usize, latency_us: u64) {
        if let Some(ep) = self.endpoints.get_mut(index) {
            ep.record_success(latency_us);
        }
    }

    /// Record a failure for the proxy at the given index.
    pub fn record_failure(&mut self, index: usize) {
        if let Some(ep) = self.endpoints.get_mut(index) {
            ep.record_failure();
        }
    }

    /// Number of healthy proxies.
    pub fn healthy_count(&self) -> usize {
        self.endpoints.iter().filter(|ep| ep.is_healthy().is_true()).count()
    }

    /// Total proxies.
    pub fn total_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Current rotation strategy.
    pub fn strategy(&self) -> RotationStrategy {
        self.strategy
    }

    /// Get a proxy by index.
    pub fn get(&self, index: usize) -> Option<&ProxyEndpoint> {
        self.endpoints.get(index)
    }
}

/// Compute a deterministic proxy selection seed from call context.
pub fn proxy_selection_seed(url_hash: &[u8; 32], call_index: u64) -> u64 {
    let mut buf = Vec::with_capacity(40);
    buf.extend_from_slice(url_hash);
    buf.extend_from_slice(&call_index.to_le_bytes());
    let h = sha3_256(&buf);
    u64::from_le_bytes(h[..8].try_into().unwrap())
}

/// Monotonic timestamp in microseconds (platform-specific).
fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proxy(host: &str, healthy: bool) -> ProxyEndpoint {
        ProxyEndpoint {
            host: host.to_string(),
            port: 8080,
            protocol: ProxyProtocol::Socks5,
            auth: None,
            region: "US".to_string(),
            last_success_us: 0,
            last_failure_us: 0,
            consecutive_failures: if healthy { 0 } else { 5 },
            total_requests: 100,
            total_successes: if healthy { 95 } else { 10 },
            avg_latency_us: 100,
        }
    }

    #[test]
    fn empty_pool_returns_none() {
        let mut pool = ProxyPool::new(RotationStrategy::RoundRobin);
        assert!(pool.select(0).is_none());
    }

    #[test]
    fn round_robin_skips_unhealthy() {
        let mut pool = ProxyPool::new(RotationStrategy::RoundRobin);
        pool.add(make_proxy("dead.proxy", false));
        pool.add(make_proxy("alive.proxy", true));

        let selected = pool.select(0);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().host, "alive.proxy");
    }

    #[test]
    fn round_robin_rotates() {
        let mut pool = ProxyPool::new(RotationStrategy::RoundRobin);
        pool.add(make_proxy("proxy-a", true));
        pool.add(make_proxy("proxy-b", true));
        pool.add(make_proxy("proxy-c", true));

        let a = pool.select(0).unwrap().host.clone();
        let b = pool.select(1).unwrap().host.clone();
        let c = pool.select(2).unwrap().host.clone();
        // After 3, we wrap around.
        let a2 = pool.select(3).unwrap().host.clone();
        assert_eq!(a, a2);
        assert_ne!(a, b);
        assert_ne!(b, c);
    }

    #[test]
    fn weighted_random_selects_healthy() {
        let mut pool = ProxyPool::new(RotationStrategy::WeightedRandom);
        pool.add(make_proxy("dead", false));
        pool.add(make_proxy("alive", true));

        let selected = pool.select(0);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().host, "alive");
    }

    #[test]
    fn least_latency_selects_fastest() {
        let mut pool = ProxyPool::new(RotationStrategy::LeastLatency);
        let mut fast = make_proxy("fast", true);
        fast.avg_latency_us = 50;
        let mut slow = make_proxy("slow", true);
        slow.avg_latency_us = 500;
        pool.add(slow);
        pool.add(fast);

        let selected = pool.select(0).unwrap();
        assert_eq!(selected.host, "fast");
    }

    #[test]
    fn record_success_updates_metrics() {
        let mut ep = make_proxy("test", true);
        ep.total_requests = 10;
        ep.total_successes = 5;
        ep.avg_latency_us = 200;

        ep.record_success(100);
        assert_eq!(ep.consecutive_failures, 0);
        assert_eq!(ep.total_requests, 11);
        assert_eq!(ep.total_successes, 6);
        // EMA: (200*7 + 100) / 8 = 187.5
        assert_eq!(ep.avg_latency_us, 187);
    }

    #[test]
    fn record_failure_increments_consecutive() {
        let mut ep = make_proxy("test", true);
        ep.consecutive_failures = 0;
        ep.record_failure();
        assert_eq!(ep.consecutive_failures, 1);
        ep.record_failure();
        assert_eq!(ep.consecutive_failures, 2);
        assert!(ep.is_healthy().is_true()); // still healthy at 2 failures
        ep.record_failure();
        assert!(ep.is_healthy().is_false()); // unhealthy at 3
    }

    #[test]
    fn success_rate_calculation() {
        let mut ep = make_proxy("test", true);
        ep.total_requests = 100;
        ep.total_successes = 95;
        assert!((ep.success_rate() - 0.95).abs() < f64::EPSILON);

        let mut ep2 = make_proxy("test2", true);
        ep2.total_requests = 0;
        assert_eq!(ep2.success_rate(), 0.0);
    }

    #[test]
    fn healthy_count_filters() {
        let mut pool = ProxyPool::new(RotationStrategy::RoundRobin);
        pool.add(make_proxy("alive1", true));
        pool.add(make_proxy("dead1", false));
        pool.add(make_proxy("alive2", true));

        assert_eq!(pool.total_count(), 3);
        assert_eq!(pool.healthy_count(), 2);
    }

    #[test]
    fn proxy_selection_seed_deterministic() {
        let url_hash = sha3_256(b"https://example.com");
        let a = proxy_selection_seed(&url_hash, 0);
        let b = proxy_selection_seed(&url_hash, 0);
        assert_eq!(a, b);
    }

    #[test]
    fn proxy_selection_seed_varies_by_index() {
        let url_hash = sha3_256(b"https://example.com");
        let a = proxy_selection_seed(&url_hash, 0);
        let b = proxy_selection_seed(&url_hash, 1);
        assert_ne!(a, b);
    }
}
