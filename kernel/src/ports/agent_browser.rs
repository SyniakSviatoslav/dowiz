//! ports/agent_browser.rs — AgentBrowserPort trait (the kernel↔browser seam).
//!
//! # What this is
//! The port trait that bridges the kernel's parse configuration (anti-detect,
//! zero-trace, proxy, crypto signing) with an external browser automation
//! implementation. The kernel defines WHAT to do; the adapter DOES it.
//!
//! # Design principles
//! - Kernel is pure computation: no browser/network code here
//! - The trait is synchronous (the kernel never blocks on I/O)
//! - All anti-detect/zero-trace config is passed through, not inferred
//! - Per-call PQ signing is the adapter's responsibility (kernel provides
//!   the signing primitives via `crypto_signer`)
//! - Failed connections degrade to `Err(BrowserError)` — never panic

use crate::agent_browser::{AntiDetectConfig, ParseResult, ZeroTracePolicy};

/// Errors from browser operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserError {
    /// Connection to the browser automation backend failed.
    ConnectionFailed(String),
    /// The page failed to load (timeout, DNS, etc).
    PageLoadFailed(String),
    /// The anti-detect fingerprint was rejected by the target.
    FingerprintRejected(String),
    /// All proxies in the pool are unhealthy.
    NoHealthyProxy,
    /// The browser session crashed or timed out.
    SessionFailed(String),
    /// Content extraction failed (page structure unexpected).
    ExtractionFailed(String),
    /// The PQ signature on the response could not be verified.
    SignatureVerificationFailed(String),
}

/// The kernel's port for browser-based operations.
///
/// Implementations live OUTSIDE the kernel (in `agent-facade`, `llm-adapters`,
/// or a dedicated browser-automation crate). The kernel provides the
/// configuration and signing primitives; the adapter executes the actual
/// browser operations.
pub trait AgentBrowserPort {
    /// Fetch a URL through the anti-detect browser.
    ///
    /// The adapter must:
    /// 1. Apply the anti-detect fingerprint (navigator, WebGL, etc)
    /// 2. Clear browser state per the zero-trace policy
    /// 3. Route through the proxy specified in the config
    /// 4. Wait for the page to load
    /// 5. Extract the text content
    /// 6. Sign the response using ML-DSA-65 (via `crypto_signer`)
    /// 7. Return the signed, traced result
    fn fetch(
        &self,
        url: &str,
        anti_detect: &AntiDetectConfig,
        zero_trace: &ZeroTracePolicy,
    ) -> Result<ParseResult, BrowserError>;

    /// Navigate to a URL and return the page state (without extracting content).
    ///
    /// Useful for multi-step flows (click through pagination, fill forms).
    fn navigate(
        &self,
        url: &str,
        anti_detect: &AntiDetectConfig,
        zero_trace: &ZeroTracePolicy,
    ) -> Result<PageState, BrowserError>;

    /// Read the current page content (after navigation).
    ///
    /// Called after `navigate` to extract content from the loaded page.
    fn read(&self) -> Result<PageContent, BrowserError>;

    /// Check if the browser backend is available and responsive.
    fn health_check(&self) -> Result<(), BrowserError>;
}

/// The state of a browser page after navigation.
#[derive(Debug, Clone)]
pub struct PageState {
    /// The final URL after redirects.
    pub final_url: String,
    /// HTTP status code.
    pub status_code: u16,
    /// Page title.
    pub title: String,
    /// Whether the page has finished loading.
    pub loaded: bool,
    /// Time to navigation complete (microseconds).
    pub nav_time_us: u64,
}

/// Extracted page content.
#[derive(Debug, Clone)]
pub struct PageContent {
    /// The extracted text content.
    pub text: String,
    /// SHA3-256 of the raw HTML for signing.
    pub raw_hash: [u8; 32],
    /// Any structured data found (JSON-LD, microdata).
    pub structured: Vec<StructuredData>,
    /// Links found on the page.
    pub links: Vec<String>,
}

/// Structured data extracted from a page.
#[derive(Debug, Clone)]
pub struct StructuredData {
    /// The format (e.g. "json-ld", "microdata", "opengraph").
    pub format: String,
    /// The extracted data as raw bytes.
    pub data: Vec<u8>,
}

/// A no-op implementation for testing (always returns errors).
///
/// This is the kernel-default: no real browser exists inside the kernel.
/// Tests that need a real browser must inject a concrete implementation.
pub struct NoOpBrowser;

impl AgentBrowserPort for NoOpBrowser {
    fn fetch(
        &self,
        _url: &str,
        _anti_detect: &AntiDetectConfig,
        _zero_trace: &ZeroTracePolicy,
    ) -> Result<ParseResult, BrowserError> {
        Err(BrowserError::ConnectionFailed(
            "no browser backend configured".to_string(),
        ))
    }

    fn navigate(
        &self,
        _url: &str,
        _anti_detect: &AntiDetectConfig,
        _zero_trace: &ZeroTracePolicy,
    ) -> Result<PageState, BrowserError> {
        Err(BrowserError::ConnectionFailed(
            "no browser backend configured".to_string(),
        ))
    }

    fn read(&self) -> Result<PageContent, BrowserError> {
        Err(BrowserError::SessionFailed(
            "no active browser session".to_string(),
        ))
    }

    fn health_check(&self) -> Result<(), BrowserError> {
        Err(BrowserError::ConnectionFailed(
            "no browser backend configured".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_browser::compute_session_fingerprint;

    #[test]
    fn no_op_browser_returns_connection_error() {
        let browser = NoOpBrowser;
        let config = AntiDetectConfig {
            session_fingerprint: compute_session_fingerprint(&[0u8; 32], 0),
            navigator: crate::agent_browser::NavigatorProfile {
                user_agent: "test".to_string(),
                platform: "TestOS".to_string(),
                languages: vec!["en".to_string()],
                hardware_concurrency: 4,
                device_memory: 8,
                max_touch_points: 0,
            },
            webgl: crate::agent_browser::WebGLProfile {
                renderer: "Test".to_string(),
                vendor: "Test".to_string(),
            },
            timezone: crate::agent_browser::TimezoneOverride {
                timezone_id: "UTC".to_string(),
                utc_offset_secs: 0,
            },
            webrtc: crate::agent_browser::WebRtcPolicy::Block,
            header_order: crate::agent_browser::HeaderOrder {
                order: vec!["Host".to_string()],
            },
            client_hints: crate::agent_browser::ClientHints {
                brands: "Test".to_string(),
                full_version_list: "Test".to_string(),
                platform: "Test".to_string(),
                platform_version: "1.0".to_string(),
                model: "Test".to_string(),
            },
        };
        let trace = crate::agent_browser::ZeroTracePolicy::maximum();

        let result = browser.fetch("https://example.com", &config, &trace);
        assert!(matches!(result, Err(BrowserError::ConnectionFailed(_))));
    }

    #[test]
    fn no_op_health_check_fails() {
        let browser = NoOpBrowser;
        assert!(browser.health_check().is_err());
    }

    #[test]
    fn page_state_fields() {
        let state = PageState {
            final_url: "https://example.com/page".to_string(),
            status_code: 200,
            title: "Test Page".to_string(),
            loaded: true,
            nav_time_us: 150,
        };
        assert_eq!(state.status_code, 200);
        assert!(state.loaded);
    }
}
