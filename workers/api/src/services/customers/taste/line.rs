//! PURE. One ordered dish as the taste profile reads it, from the dish's catalogue JSON.

use super::Line;

/// The dish's tags and category from its catalogue JSON.
pub fn line_of(product_json: &str, qty: i64) -> Line {
    let v: serde_json::Value = serde_json::from_str(product_json).unwrap_or_default();
    Line {
        tags: v.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        category: v.get("categoryId").and_then(|c| c.as_str()).map(str::to_string),
        qty,
        sense: dowiz_hub::sense::of_product(&v).map(|s| dowiz_hub::sense::vector(&s)).unwrap_or_default(),
        id: v.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
    }
}
