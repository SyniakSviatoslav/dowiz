//! Taste is authored per dish, never derived: five axes, three levels, an
//! absent axis is "not declared" (the old contract, `attributes.taste`).

use serde_json::{json, Value};

use super::{TASTE_AXES, TASTE_MAX, TASTE_MIN};

/// A taste map is five known axes at levels 1…3; anything else is refused.
pub fn validate_taste(m: &serde_json::Map<String, Value>) -> Result<serde_json::Map<String, Value>, String> {
    let mut out = serde_json::Map::new();
    for (k, v) in m {
        if !TASTE_AXES.contains(&k.as_str()) {
            return Err(format!("unknown taste axis {k:?}"));
        }
        match v.as_i64() {
            Some(n) if (TASTE_MIN..=TASTE_MAX).contains(&n) => {
                out.insert(k.clone(), json!(n));
            }
            Some(0) | None if v.is_null() || v.as_i64() == Some(0) => {} // 0 or null = not declared
            _ => return Err(format!("taste {k} is 1 to 3")),
        }
    }
    Ok(out)
}

