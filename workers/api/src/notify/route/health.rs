//! A TARGET'S HEALTH: when it last took a message, and Telegram's last
//! refusal. Written by the drain into the outbox image it already writes
//! (record kind `HEALTH_KIND`, one per target), read by the console. PURE.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Health {
    #[serde(default)]
    pub ok_ms: i64,
    #[serde(default)]
    pub err_ms: i64,
    #[serde(default)]
    pub err: String,
}

impl Health {
    pub fn read(rec: Option<&str>) -> Health {
        rec.and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default()
    }

    /// After one attempt: a success keeps the last error (it is history the
    /// owner may want), a failure keeps the last success.
    pub fn after(mut self, ok: bool, err: &str, now_ms: i64) -> Health {
        if ok {
            self.ok_ms = now_ms;
        } else {
            self.err_ms = now_ms;
            self.err = err.chars().take(200).collect();
        }
        self
    }

    /// Failing now: the last error is newer than the last success.
    pub fn failing(&self) -> bool {
        self.err_ms > self.ok_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_success_after_a_failure_is_healthy_and_the_error_is_kept() {
        let h = Health::read(None).after(false, "Forbidden: bot was kicked", 10);
        assert!(h.failing());
        let h = h.after(true, "", 20);
        assert!(!h.failing());
        assert_eq!((h.ok_ms, h.err_ms, h.err.as_str()), (20, 10, "Forbidden: bot was kicked"));
        let back = Health::read(Some(&serde_json::to_string(&h).unwrap()));
        assert_eq!(back, h);
        assert_eq!(Health::read(Some("garbage")), Health::default());
    }
}
