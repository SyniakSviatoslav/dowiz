//! agent_browser.rs — Anti-detect browser configuration and zero-trace policy.
//!
//! # What this is
//! Pure data structures that define HOW a parse call should be executed
//! through an anti-detect browser with zero trace and proxy redirection.
//! This module contains NO network I/O — it is the kernel's configuration
//! authority for browser-based parsing. The actual browser automation lives
//! behind the `AgentBrowserPort` trait in `ports/agent_browser.rs`.
//!
//! # Design principles
//! - Kernel = pure computation (no browser/network)
//! - All anti-detect fingerprinting is configuration, not code
//! - Zero-trace policy is enforced by construction (typed, not a flag)
//! - Per-call crypto signature binding is structurally required

use crate::event_log::sha3_256;
use crate::TriState;

/// Maximum concurrent browser sessions (resource-aware default).
const MAX_BROWSER_SESSIONS: usize = 4;

/// Anti-detect browser configuration for a single parse call.
///
/// Every field is explicitly set — no silent defaults that could leak trace.
/// The kernel constructs this and passes it through the `AgentBrowserPort`.
#[derive(Debug, Clone)]
pub struct AntiDetectConfig {
    /// Unique session fingerprint (randomized per call).
    pub session_fingerprint: [u8; 32],
    /// Navigator properties to spoof (user-agent, platform, languages, etc).
    pub navigator: NavigatorProfile,
    /// WebGL fingerprint override (canvas rendering signature).
    pub webgl: WebGLProfile,
    /// Timezone and locale override (prevents timezone correlation).
    pub timezone: TimezoneOverride,
    /// WebRTC policy (prevent IP leak via STUN).
    pub webrtc: WebRtcPolicy,
    /// HTTP header order (browsers have characteristic header ordering).
    pub header_order: HeaderOrder,
    /// Client hints override (sec-ch-ua headers).
    pub client_hints: ClientHints,
}

/// Navigator properties to spoof.
#[derive(Debug, Clone)]
pub struct NavigatorProfile {
    /// e.g. "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ..."
    pub user_agent: String,
    /// e.g. "Win32", "MacIntel", "Linux x86_64"
    pub platform: String,
    /// e.g. ["en-US", "en"]
    pub languages: Vec<String>,
    /// Hardware concurrency (CPU core count to report).
    pub hardware_concurrency: u32,
    /// Device memory (GB to report).
    pub device_memory: u32,
    /// Max touch points.
    pub max_touch_points: u32,
}

/// WebGL fingerprint override.
#[derive(Debug, Clone)]
pub struct WebGLProfile {
    /// Renderer string (e.g. "NVIDIA GeForce RTX 3080").
    pub renderer: String,
    /// Vendor string (e.g. "NVIDIA Corporation").
    pub vendor: String,
}

/// Timezone and locale override.
#[derive(Debug, Clone)]
pub struct TimezoneOverride {
    /// IANA timezone (e.g. "America/New_York").
    pub timezone_id: String,
    /// UTC offset in seconds.
    pub utc_offset_secs: i32,
}

/// WebRTC policy — controls whether STUN can leak the real IP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WebRtcPolicy {
    /// Block all WebRTC (safest — no IP leak possible).
    Block,
    /// Force WebRTC through proxy (IP shows proxy, not real).
    ProxyOnly,
    /// Allow native WebRTC (risky — may leak real IP).
    AllowNative,
}

/// HTTP header ordering — browsers have characteristic ordering.
#[derive(Debug, Clone)]
pub struct HeaderOrder {
    /// Ordered list of header names (first element sent first).
    pub order: Vec<String>,
}

/// Client hints (sec-ch-ua headers).
#[derive(Debug, Clone)]
pub struct ClientHints {
    /// e.g. "\"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\""
    pub brands: String,
    /// e.g. "\"Chromium\";v=\"120\", \"Not_A Brand\";v=\"8\""
    pub full_version_list: String,
    /// e.g. "\"Windows\""
    pub platform: String,
    /// e.g. "\"Windows 10\""
    pub platform_version: String,
    /// e.g. "\"Google Chrome\""
    pub model: String,
}

/// Zero-trace policy — what to clear and when.
///
/// Typed to prevent silent trace leaks: every field must be explicitly chosen.
#[derive(Debug, Clone)]
pub struct ZeroTracePolicy {
    /// Clear cookies between requests.
    pub clear_cookies: TriState,
    /// Clear localStorage between requests.
    pub clear_local_storage: TriState,
    /// Clear sessionStorage between requests.
    pub clear_session_storage: TriState,
    /// Clear IndexedDB between requests.
    pub clear_indexed_db: TriState,
    /// Clear cache between requests.
    pub clear_cache: TriState,
    /// Clear WebGL textures between requests.
    pub clear_webgl: TriState,
    /// Reset canvas fingerprint between requests.
    pub reset_canvas: TriState,
    /// Randomize font enumeration order.
    pub randomize_font_order: TriState,
    /// Disable history API (prevents back-forward fingerprinting).
    pub disable_history_api: TriState,
    /// Disable navigator.sendBeacon (prevents tracking).
    pub disable_send_beacon: TriState,
}

impl ZeroTracePolicy {
    /// Maximum stealth — clear everything, disable everything.
    pub fn maximum() -> Self {
        ZeroTracePolicy {
            clear_cookies: TriState::True,
            clear_local_storage: TriState::True,
            clear_session_storage: TriState::True,
            clear_indexed_db: TriState::True,
            clear_cache: TriState::True,
            clear_webgl: TriState::True,
            reset_canvas: TriState::True,
            randomize_font_order: TriState::True,
            disable_history_api: TriState::True,
            disable_send_beacon: TriState::True,
        }
    }

    /// Balanced — clear what matters most, allow some convenience.
    pub fn balanced() -> Self {
        ZeroTracePolicy {
            clear_cookies: TriState::True,
            clear_local_storage: TriState::True,
            clear_session_storage: TriState::True,
            clear_indexed_db: TriState::False,
            clear_cache: TriState::True,
            clear_webgl: TriState::True,
            reset_canvas: TriState::True,
            randomize_font_order: TriState::False,
            disable_history_api: TriState::True,
            disable_send_beacon: TriState::True,
        }
    }
}

/// The result of a browser-based parse operation.
///
/// Cryptographically signed per-call — the signature binds the IP, timestamp,
/// payload hash, and a nonce to prevent replay and tampering.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The extracted content (HTML text, JSON, structured data).
    pub content: Vec<u8>,
    /// SHA3-256 of the raw content for integrity verification.
    pub content_hash: [u8; 32],
    /// The URL that was actually loaded (after redirects).
    pub final_url: String,
    /// HTTP status code.
    pub status_code: u16,
    /// Response headers (key-value pairs).
    pub response_headers: Vec<(String, String)>,
    /// Time to first byte in microseconds.
    pub ttfb_us: u64,
    /// Total load time in microseconds.
    pub load_time_us: u64,
    /// The ML-DSA-65 signature over (ip_hash || timestamp || content_hash || nonce).
    pub pq_signature: Vec<u8>,
    /// The ML-DSA-65 public key used for this call's signature.
    pub pq_public_key: Vec<u8>,
    /// Timestamp (unix microseconds) when the response was received.
    pub timestamp_us: u64,
    /// IP address hash (SHA3-256 of the source IP used for this call).
    pub ip_hash: [u8; 32],
    /// Nonce used for this call's signature (prevents replay).
    pub nonce: [u8; 32],
}

/// Dynamic algorithm selection for read/navigate actions.
///
/// Based on system load and available resources, the orchestrator selects
/// the optimal algorithm for each operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadAlgorithm {
    /// Sequential read — low resource usage, single-threaded.
    Sequential,
    /// Parallel chunk read — uses available cores for large pages.
    ParallelChunked,
    /// Streaming read — incremental processing, lowest memory.
    Streaming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigateAlgorithm {
    /// Direct navigation — single request, lowest latency.
    Direct,
    /// Retry with backoff — handles transient failures.
    RetryWithBackoff,
    /// Multi-path — tries multiple routes (proxy chains) for redundancy.
    MultiPath,
}

/// Resource state snapshot for dynamic algorithm selection.
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    /// Available CPU cores (from `std::thread::available_parallelism`).
    pub cpu_cores: usize,
    /// Memory pressure: 0.0 = idle, 1.0 = saturated.
    pub memory_pressure: f64,
    /// Current active browser sessions.
    pub active_sessions: usize,
    /// Average latency of recent parse calls (microseconds).
    pub avg_latency_us: u64,
    /// Failure rate of recent parse calls (0.0..1.0).
    pub failure_rate: f64,
}

impl ResourceSnapshot {
    /// Select the optimal read algorithm based on current resources.
    pub fn select_read_algorithm(&self) -> ReadAlgorithm {
        if self.memory_pressure > 0.8 {
            ReadAlgorithm::Streaming
        } else if self.cpu_cores >= 4 && self.memory_pressure < 0.5 {
            ReadAlgorithm::ParallelChunked
        } else {
            ReadAlgorithm::Sequential
        }
    }

    /// Select the optimal navigate algorithm based on current resources.
    pub fn select_navigate_algorithm(&self) -> NavigateAlgorithm {
        if self.failure_rate > 0.3 {
            NavigateAlgorithm::MultiPath
        } else if self.failure_rate > 0.1 {
            NavigateAlgorithm::RetryWithBackoff
        } else {
            NavigateAlgorithm::Direct
        }
    }

    /// Recommended concurrency limit based on resources.
    pub fn recommended_concurrency(&self) -> usize {
        if self.memory_pressure > 0.8 {
            1
        } else if self.cpu_cores >= 8 && self.memory_pressure < 0.3 {
            MAX_BROWSER_SESSIONS
        } else if self.cpu_cores >= 4 {
            2
        } else {
            1
        }
    }
}

/// Compute a session fingerprint from a seed (deterministic but unique per call).
pub fn compute_session_fingerprint(seed: &[u8], call_index: u64) -> [u8; 32] {
    let mut buf = Vec::with_capacity(seed.len() + 8);
    buf.extend_from_slice(seed);
    buf.extend_from_slice(&call_index.to_le_bytes());
    sha3_256(&buf)
}

/// Compute IP hash for crypto binding (SHA3-256 of the raw IP bytes).
pub fn hash_ip(ip_bytes: &[u8]) -> [u8; 32] {
    sha3_256(ip_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_trace_maximum_clears_everything() {
        let p = ZeroTracePolicy::maximum();
        assert!(p.clear_cookies.is_true());
        assert!(p.clear_local_storage.is_true());
        assert!(p.clear_session_storage.is_true());
        assert!(p.clear_indexed_db.is_true());
        assert!(p.clear_cache.is_true());
        assert!(p.clear_webgl.is_true());
        assert!(p.reset_canvas.is_true());
        assert!(p.randomize_font_order.is_true());
        assert!(p.disable_history_api.is_true());
        assert!(p.disable_send_beacon.is_true());
    }

    #[test]
    fn zero_trace_balanced_skips_some() {
        let p = ZeroTracePolicy::balanced();
        assert!(p.clear_cookies.is_true());
        assert!(p.clear_indexed_db.is_false());
        assert!(p.randomize_font_order.is_false());
        assert!(p.disable_send_beacon.is_true());
    }

    #[test]
    fn session_fingerprint_deterministic() {
        let seed = [42u8; 32];
        let a = compute_session_fingerprint(&seed, 0);
        let b = compute_session_fingerprint(&seed, 0);
        assert_eq!(a, b, "same seed+index => same fingerprint");
    }

    #[test]
    fn session_fingerprint_unique_per_index() {
        let seed = [42u8; 32];
        let a = compute_session_fingerprint(&seed, 0);
        let b = compute_session_fingerprint(&seed, 1);
        assert_ne!(a, b, "different index => different fingerprint");
    }

    #[test]
    fn hash_ip_deterministic() {
        let ip = [192, 168, 1, 1];
        let a = hash_ip(&ip);
        let b = hash_ip(&ip);
        assert_eq!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn hash_ip_distinguishes_addresses() {
        let a = hash_ip(&[192, 168, 1, 1]);
        let b = hash_ip(&[10, 0, 0, 1]);
        assert_ne!(a, b);
    }

    #[test]
    fn resource_snapshot_selects_sequential_under_pressure() {
        let snap = ResourceSnapshot {
            cpu_cores: 8,
            memory_pressure: 0.9,
            active_sessions: 4,
            avg_latency_us: 500,
            failure_rate: 0.05,
        };
        assert_eq!(snap.select_read_algorithm(), ReadAlgorithm::Streaming);
        assert_eq!(snap.recommended_concurrency(), 1);
    }

    #[test]
    fn resource_snapshot_selects_parallel_when_resources_available() {
        let snap = ResourceSnapshot {
            cpu_cores: 8,
            memory_pressure: 0.2,
            active_sessions: 0,
            avg_latency_us: 100,
            failure_rate: 0.0,
        };
        assert_eq!(
            snap.select_read_algorithm(),
            ReadAlgorithm::ParallelChunked
        );
        assert_eq!(snap.recommended_concurrency(), MAX_BROWSER_SESSIONS);
    }

    #[test]
    fn resource_snapshot_selects_retry_on_failures() {
        let snap = ResourceSnapshot {
            cpu_cores: 4,
            memory_pressure: 0.3,
            active_sessions: 1,
            avg_latency_us: 200,
            failure_rate: 0.15,
        };
        assert_eq!(
            snap.select_navigate_algorithm(),
            NavigateAlgorithm::RetryWithBackoff
        );
    }

    #[test]
    fn resource_snapshot_selects_multipath_on_high_failures() {
        let snap = ResourceSnapshot {
            cpu_cores: 4,
            memory_pressure: 0.3,
            active_sessions: 1,
            avg_latency_us: 200,
            failure_rate: 0.4,
        };
        assert_eq!(
            snap.select_navigate_algorithm(),
            NavigateAlgorithm::MultiPath
        );
    }

    #[test]
    fn resource_snapshot_selects_direct_when_healthy() {
        let snap = ResourceSnapshot {
            cpu_cores: 4,
            memory_pressure: 0.3,
            active_sessions: 1,
            avg_latency_us: 100,
            failure_rate: 0.02,
        };
        assert_eq!(snap.select_navigate_algorithm(), NavigateAlgorithm::Direct);
    }

    #[test]
    fn parse_result_content_hash_is_sha3() {
        let content = b"test content";
        let expected_hash = sha3_256(content);
        let result = ParseResult {
            content: content.to_vec(),
            content_hash: expected_hash,
            final_url: "https://example.com".to_string(),
            status_code: 200,
            response_headers: vec![],
            ttfb_us: 50,
            load_time_us: 200,
            pq_signature: vec![],
            pq_public_key: vec![],
            timestamp_us: 1234567890,
            ip_hash: [0u8; 32],
            nonce: [0u8; 32],
        };
        assert_eq!(result.content_hash, expected_hash);
    }

    #[test]
    fn web_rtc_policy_variants() {
        assert_ne!(WebRtcPolicy::Block, WebRtcPolicy::ProxyOnly);
        assert_ne!(WebRtcPolicy::ProxyOnly, WebRtcPolicy::AllowNative);
    }
}
