//! Resend email adapter — Q-EMAIL-DIRECT (`docs/design/rebuild-jobs-s8-council/proposal.md`
//! §4.3). Carries the separation VERBATIM: this is a direct ops-alert path, deliberately OUTSIDE
//! the tenant dispatcher (no `location_id`, no prefs/quiet-hours/audit) — only
//! `access-request.notify` uses it. `RESEND_API_KEY` is a 🔴 secret (config-only, held as
//! `config::Secret`, never logged), same posture as VAPID.
//!
//! ## Why this module is `#[allow(dead_code)]`
//! `access-request.notify` (the ONE caller, §4.3) is not part of this pass's build (it is neither
//! a REV-S8-1..6 named test nor one of the money/PII/VAPID 🔴 items this pass prioritized) — this
//! adapter is real and tested, awaiting that call site the same way `crate::jobs::bridge` awaits
//! S5's order-creation call site.
#![allow(
    dead_code,
    reason = "access-request.notify (the one caller, §4.3) is not wired this pass"
)]

use std::time::Duration;

use crate::config::Secret;
use crate::jobs::channels::SendOutcome;

const SEND_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_API_BASE: &str = "https://api.resend.com";
const DEFAULT_FROM: &str = "dowiz <onboarding@resend.dev>";

pub struct EmailAdapter {
    client: reqwest::Client,
    api_key: Secret,
    api_base: String,
    default_from: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ResendPayload<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<&'a str>,
}

impl EmailAdapter {
    /// Soft-disables (returns `None`) when `RESEND_API_KEY` is absent — mirrors every other
    /// optional-channel gate in this surface (`NotificationsConfig`'s doc); the caller (the
    /// `access-request.notify` job handler) must treat `None` as "email is dark," not error.
    pub fn new(api_key: Option<Secret>) -> Option<Result<Self, reqwest::Error>> {
        api_key.map(|key| {
            let client = reqwest::Client::builder().timeout(SEND_TIMEOUT).build()?;
            Ok(EmailAdapter {
                client,
                api_key: key,
                api_base: DEFAULT_API_BASE.to_string(),
                default_from: DEFAULT_FROM.to_string(),
            })
        })
    }

    pub async fn send_ops(
        &self,
        to: &str,
        subject: &str,
        text: &str,
    ) -> Result<SendOutcome, reqwest::Error> {
        let payload = ResendPayload {
            from: &self.default_from,
            to: vec![to],
            subject,
            text,
            html: None,
        };

        let response = match self
            .client
            .post(format!("{}/emails", self.api_base))
            .bearer_auth(self.api_key.expose())
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => return Ok(SendOutcome::TimedOut),
            Err(e) => {
                return Ok(SendOutcome::NetworkError {
                    message: e.to_string(),
                });
            }
        };

        let status = response.status();
        if status.is_success() {
            return Ok(SendOutcome::Delivered);
        }
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5));
            return Ok(SendOutcome::RateLimited { retry_after });
        }
        if matches!(status.as_u16(), 401 | 403) {
            return Ok(SendOutcome::PermanentlyRejected {
                reason: status.to_string(),
            });
        }
        Ok(SendOutcome::NetworkError {
            message: format!("unexpected status {status}"),
        })
    }
}

impl std::fmt::Debug for EmailAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailAdapter")
            .field("api_key", &self.api_key)
            .field("api_base", &self.api_base)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_adapter_is_dark_when_no_api_key() {
        assert!(EmailAdapter::new(None).is_none());
    }

    #[test]
    fn email_adapter_builds_when_api_key_present() {
        let built = EmailAdapter::new(Some(Secret::new("re_test_key")));
        assert!(built.is_some());
        assert!(built.unwrap().is_ok());
    }

    #[test]
    fn email_adapter_debug_never_prints_the_api_key() {
        let adapter = EmailAdapter::new(Some(Secret::new("re_super_secret")))
            .unwrap()
            .unwrap();
        let rendered = format!("{adapter:?}");
        assert!(!rendered.contains("re_super_secret"));
    }

    #[test]
    fn resend_payload_omits_html_when_absent() {
        let payload = ResendPayload {
            from: "a@b.com",
            to: vec!["c@d.com"],
            subject: "s",
            text: "t",
            html: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(!json.as_object().unwrap().contains_key("html"));
    }

    // ── live-network proof (requires a real RESEND_API_KEY; not run in this sandbox) ──

    #[tokio::test]
    #[ignore = "requires a real RESEND_API_KEY — run manually"]
    async fn send_ops_delivers_a_real_email() {
        let adapter = EmailAdapter::new(Some(Secret::new(
            std::env::var("RESEND_API_KEY").expect("set RESEND_API_KEY"),
        )))
        .unwrap()
        .unwrap();
        let outcome = adapter
            .send_ops("test@example.com", "S8 adapter smoke test", "hello")
            .await
            .expect("send must not error at the transport level");
        assert_eq!(outcome, SendOutcome::Delivered);
    }
}
