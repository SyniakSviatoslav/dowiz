//! Pure, framework/DB-free mapping functions shared by the S1 route handlers — the ported
//! equivalents of the small inline helpers scattered through the Node route files. Kept
//! separate from `routes/*` (which do extraction + repo calls + response building) so the
//! actual *logic* is unit-testable without axum or a database, per the build brief.

use serde_json::Value as Json;

use crate::dto::{
    PublicMenu, PublicMenuCategory, PublicMenuCurrency, PublicMenuProduct, VenueStatus, Weekday,
    WeeklyHoursEntry,
};

/// Ports `resolveMediaUrl` (menu.ts:26-29): an absolute http(s) key passes through, else it
/// becomes a `/media/<key>` proxy path. `None` in, `None` out (menu.ts's `if (!key) return null`).
pub fn resolve_media_url(key: Option<&str>) -> Option<String> {
    let key = key?;
    if key.is_empty() {
        return None;
    }
    let lower = key.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some(key.to_string())
    } else {
        Some(format!("/media/{key}"))
    }
}

/// Ports `getImageUrl` (`image-url.ts`) verbatim: http(s)/data: keys pass through; an
/// `R2_PUBLIC_URL` env wins over the `/images/<key>` app-base fallback.
pub fn get_image_url(
    image_key: Option<&str>,
    r2_public_url: Option<&str>,
    app_base_url: &str,
) -> Option<String> {
    let image_key = image_key?;
    if image_key.is_empty() {
        return None;
    }
    if image_key.starts_with("http://")
        || image_key.starts_with("https://")
        || image_key.starts_with("data:")
    {
        return Some(image_key.to_string());
    }
    let clean_key = image_key.strip_prefix('/').unwrap_or(image_key);
    if let Some(r2) = r2_public_url.filter(|s| !s.is_empty()) {
        let joined = if r2.ends_with('/') {
            r2.to_string()
        } else {
            format!("{r2}/")
        };
        return Some(format!("{joined}{clean_key}"));
    }
    let base = app_base_url.strip_suffix('/').unwrap_or(app_base_url);
    Some(format!("{base}/images/{clean_key}"))
}

/// Ports `theme.ts:20`'s inline uuid-shape regex verbatim
/// (`/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i` — used to decide whether
/// `getThemeCss`'s `locationId` path param needs a slug->uuid lookup first). Deliberately a
/// hand-rolled shape check, NOT `Uuid::parse_str` — that parser is more lenient (accepts
/// non-hyphenated/braced/urn: forms the Node regex rejects), which would make a slug that
/// happens to parse loosely skip the slug lookup Node always runs for it.
pub fn is_uuid_format(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    let expected_lens = [8, 4, 4, 4, 12];
    groups.len() == 5
        && groups
            .iter()
            .zip(expected_lens)
            .all(|(g, len)| g.len() == len && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Ports the caller-controlled locale normalization guarding the menu cache key
/// (`menu.ts:238-241`): lowercase, strip anything outside `[a-z0-9_-]`, cap at 12 chars.
pub fn normalize_locale(raw: &str) -> String {
    raw.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_' || *c == '-')
        .take(12)
        .collect()
}

/// Ports `mediaServingAllowed` (`product-media-validation.ts:145-147`) verbatim: rich media is
/// served only when the deploy-time flag is on AND the location plan is exactly `'business'`.
pub fn media_serving_allowed(flag_enabled: bool, plan: Option<&str>) -> bool {
    flag_enabled && plan == Some("business")
}

/// Ports `adaptPreviewMenu` (`menu.ts:40-68`): reshapes a shadow tenant's `read_preview_menu`
/// payload into the public `PublicMenu` wire shape, `is_preview: true`, `menu_version: 0`,
/// `location_id: null` — a shadow has no menu_versions row and its id is never exposed.
pub fn adapt_preview_menu(preview: &Json) -> PublicMenu {
    let default_locale = preview
        .get("default_locale")
        .and_then(Json::as_str)
        .unwrap_or("sq")
        .to_string();
    let currency = preview
        .get("currency")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"code": "ALL", "minor_unit": 0}));
    let currency = PublicMenuCurrency {
        code: currency
            .get("code")
            .and_then(Json::as_str)
            .unwrap_or("ALL")
            .to_string(),
        minor_unit: currency
            .get("minor_unit")
            .and_then(Json::as_i64)
            .and_then(|n| i32::try_from(n).ok())
            .unwrap_or(0),
    };

    let categories = preview
        .get("categories")
        .and_then(Json::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|c| PublicMenuCategory {
            id: c
                .get("id")
                .and_then(Json::as_str)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(uuid::Uuid::nil),
            name: c
                .get("name")
                .and_then(Json::as_str)
                .unwrap_or("")
                .to_string(),
            sort_order: c
                .get("sort_order")
                .and_then(Json::as_i64)
                .and_then(|n| i32::try_from(n).ok())
                .unwrap_or(0),
            products: c
                .get("products")
                .and_then(Json::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|p| PublicMenuProduct {
                    id: p
                        .get("id")
                        .and_then(Json::as_str)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_else(uuid::Uuid::nil),
                    name: p
                        .get("name")
                        .and_then(Json::as_str)
                        .unwrap_or("")
                        .to_string(),
                    description: p
                        .get("description")
                        .and_then(Json::as_str)
                        .map(str::to_string),
                    price: p.get("price").and_then(Json::as_i64).unwrap_or(0),
                    available: !matches!(p.get("is_available"), Some(Json::Bool(false))),
                    image_key: None,
                    primary_media_id: None,
                    image_url: None,
                    attributes: p.get("attributes").cloned(),
                    prep_time_minutes: None,
                    modifier_groups: vec![],
                })
                .collect(),
        })
        .collect();

    PublicMenu {
        menu_version: 0,
        location_id: None,
        location_id_alias: None,
        location_name: preview
            .get("name")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string(),
        default_locale,
        supported_locales: preview
            .get("default_locale")
            .and_then(Json::as_str)
            .map(|l| vec![l.to_string()])
            .unwrap_or_else(|| vec!["sq".to_string()]),
        currency,
        is_preview: Some(true),
        categories,
    }
}

/// One weekday's `{open, close}` window (or closed) from `hours_json[day]`.
struct DayWindow {
    is_open: bool,
    open: Option<String>,
    close: Option<String>,
}

fn day_window(hours_json: Option<&Json>, day: &str) -> Option<DayWindow> {
    let day_data = hours_json?.get(day)?;
    if !day_data.is_object() {
        return None;
    }
    if day_data.get("isOpen") == Some(&Json::Bool(false)) {
        return Some(DayWindow {
            is_open: false,
            open: None,
            close: None,
        });
    }
    let open = day_data
        .get("open")
        .and_then(Json::as_str)
        .map(str::to_string);
    let close = day_data
        .get("close")
        .and_then(Json::as_str)
        .map(str::to_string);
    Some(DayWindow {
        is_open: true,
        open,
        close,
    })
}

fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    Some((h.parse().ok()?, m.parse().ok()?))
}

/// Ports the inline `isOpen`/`closesAt` computation (`menu.ts:333-359`): `now` is passed in
/// (never read from the system clock inside this function) so the day/time-window logic is
/// fully deterministic and unit-testable.
pub fn compute_open_and_closes_at(
    hours_json: Option<&Json>,
    delivery_paused: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> (bool, Option<String>) {
    let mut is_open = !delivery_paused;
    let mut closes_at = None;

    // JS `Date#getDay()`: 0=Sunday..6=Saturday, matching `menu.ts:339`'s `days` array.
    let days = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
    ];
    let day_name = days[now.format("%w").to_string().parse::<usize>().unwrap_or(0)];

    if let Some(window) = day_window(hours_json, day_name) {
        if !window.is_open {
            is_open = false;
        } else if let (Some(open), Some(close)) = (&window.open, &window.close) {
            closes_at = Some(close.clone());
            if is_open {
                if let (Some((oh, om)), Some((ch, cm))) = (parse_hhmm(open), parse_hhmm(close)) {
                    let now_mins = now.format("%H").to_string().parse::<u32>().unwrap_or(0) * 60
                        + now.format("%M").to_string().parse::<u32>().unwrap_or(0);
                    let open_mins = oh * 60 + om;
                    let close_mins = ch * 60 + cm;
                    is_open = now_mins >= open_mins && now_mins < close_mins;
                }
            }
        }
    }
    (is_open, closes_at)
}

/// Ports the `status` computation (`menu.ts:361-367`): `busy` iff open AND
/// `kitchen_busy_until` is a future timestamp.
pub fn compute_venue_status(
    is_open: bool,
    kitchen_busy_until: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> VenueStatus {
    let is_busy = is_open && kitchen_busy_until.is_some_and(|t| t > now);
    if !is_open {
        VenueStatus::Closed
    } else if is_busy {
        VenueStatus::Busy
    } else {
        VenueStatus::Open
    }
}

/// Ports the `weeklyHours` normalization (`menu.ts:372-381`): Mon..Sun, `None` when there's no
/// `hours_json` OR every day is closed.
pub fn compute_weekly_hours(hours_json: Option<&Json>) -> Option<Vec<WeeklyHoursEntry>> {
    let hours_json = hours_json?;
    if !hours_json.is_object() {
        return None;
    }
    let order = [
        ("monday", Weekday::Monday),
        ("tuesday", Weekday::Tuesday),
        ("wednesday", Weekday::Wednesday),
        ("thursday", Weekday::Thursday),
        ("friday", Weekday::Friday),
        ("saturday", Weekday::Saturday),
        ("sunday", Weekday::Sunday),
    ];
    let entries: Vec<WeeklyHoursEntry> = order
        .into_iter()
        .map(|(key, day)| match day_window(Some(hours_json), key) {
            Some(w) if w.is_open => WeeklyHoursEntry {
                day,
                is_open: true,
                open: w.open,
                close: w.close,
            },
            _ => WeeklyHoursEntry {
                day,
                is_open: false,
                open: None,
                close: None,
            },
        })
        .collect();
    entries.iter().any(|e| e.is_open).then_some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn resolve_media_url_passes_through_absolute_and_proxies_relative() {
        assert_eq!(
            resolve_media_url(Some("https://cdn.example.com/x.webp")),
            Some("https://cdn.example.com/x.webp".to_string())
        );
        assert_eq!(
            resolve_media_url(Some("abc123.webp")),
            Some("/media/abc123.webp".to_string())
        );
        assert_eq!(resolve_media_url(None), None);
        assert_eq!(resolve_media_url(Some("")), None);
    }

    #[test]
    fn get_image_url_prefers_r2_over_app_base() {
        assert_eq!(
            get_image_url(
                Some("abc.webp"),
                Some("https://cdn.r2.dev"),
                "https://dowiz.fly.dev"
            ),
            Some("https://cdn.r2.dev/abc.webp".to_string())
        );
        assert_eq!(
            get_image_url(Some("abc.webp"), None, "https://dowiz.fly.dev"),
            Some("https://dowiz.fly.dev/images/abc.webp".to_string())
        );
        assert_eq!(
            get_image_url(
                Some("https://ext.example/x.png"),
                None,
                "https://dowiz.fly.dev"
            ),
            Some("https://ext.example/x.png".to_string())
        );
        assert_eq!(get_image_url(None, None, "https://dowiz.fly.dev"), None);
    }

    #[test]
    fn is_uuid_format_matches_canonical_hyphenated_shape_only() {
        assert!(is_uuid_format("6f0f1234-5678-90ab-cdef-1234567890ab"));
        assert!(
            is_uuid_format("6F0F1234-5678-90AB-CDEF-1234567890AB"),
            "case-insensitive"
        );
        assert!(
            !is_uuid_format("eljos-pizza"),
            "a slug must not be mistaken for a uuid"
        );
        assert!(
            !is_uuid_format("6f0f123456789 0abcdef1234567890ab"),
            "no hyphens rejected"
        );
        assert!(!is_uuid_format(""));
    }

    #[test]
    fn normalize_locale_strips_and_bounds() {
        assert_eq!(normalize_locale("SQ"), "sq");
        assert_eq!(normalize_locale("en-US!!"), "en-us");
        assert_eq!(normalize_locale(&"x".repeat(20)), "x".repeat(12));
        assert_eq!(normalize_locale(""), "");
    }

    #[test]
    fn media_serving_allowed_requires_flag_and_business_plan() {
        assert!(media_serving_allowed(true, Some("business")));
        assert!(!media_serving_allowed(false, Some("business")));
        assert!(!media_serving_allowed(true, Some("starter")));
        assert!(!media_serving_allowed(true, None));
    }

    #[test]
    fn adapt_preview_menu_sets_shadow_invariants() {
        let preview = serde_json::json!({
            "name": "Shadow Cafe",
            "default_locale": "sq",
            "currency": {"code": "ALL", "minor_unit": 0},
            "categories": [{
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "Mains",
                "sort_order": 0,
                "products": [{
                    "id": "00000000-0000-0000-0000-000000000002",
                    "name": "Byrek",
                    "price": 300,
                    "is_available": true,
                }],
            }],
        });
        let menu = adapt_preview_menu(&preview);
        assert_eq!(menu.is_preview, Some(true));
        assert_eq!(menu.menu_version, 0);
        assert!(menu.location_id.is_none());
        assert_eq!(menu.categories[0].products[0].name, "Byrek");
        assert!(menu.categories[0].products[0].available);
    }

    #[test]
    fn adapt_preview_menu_maps_is_available_false_to_unavailable() {
        let preview = serde_json::json!({
            "name": "Shadow Cafe",
            "categories": [{
                "id": "00000000-0000-0000-0000-000000000001",
                "name": "Mains",
                "products": [{"id": "00000000-0000-0000-0000-000000000002", "name": "X", "price": 1, "is_available": false}],
            }],
        });
        let menu = adapt_preview_menu(&preview);
        assert!(!menu.categories[0].products[0].available);
    }

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn compute_open_and_closes_at_within_window_is_open() {
        // 2026-07-06 is a Monday.
        let hours = serde_json::json!({"monday": {"open": "09:00", "close": "22:00"}});
        let now = dt(2026, 7, 6, 12, 0);
        let (is_open, closes_at) = compute_open_and_closes_at(Some(&hours), false, now);
        assert!(is_open);
        assert_eq!(closes_at.as_deref(), Some("22:00"));
    }

    #[test]
    fn compute_open_and_closes_at_outside_window_is_closed() {
        let hours = serde_json::json!({"monday": {"open": "09:00", "close": "22:00"}});
        let now = dt(2026, 7, 6, 23, 30);
        let (is_open, _) = compute_open_and_closes_at(Some(&hours), false, now);
        assert!(!is_open);
    }

    #[test]
    fn compute_open_and_closes_at_delivery_paused_forces_closed() {
        let hours = serde_json::json!({"monday": {"open": "00:00", "close": "23:59"}});
        let now = dt(2026, 7, 6, 12, 0);
        let (is_open, _) = compute_open_and_closes_at(Some(&hours), true, now);
        assert!(!is_open);
    }

    #[test]
    fn compute_open_and_closes_at_explicit_day_closed() {
        let hours = serde_json::json!({"monday": {"isOpen": false}});
        let now = dt(2026, 7, 6, 12, 0);
        let (is_open, closes_at) = compute_open_and_closes_at(Some(&hours), false, now);
        assert!(!is_open);
        assert!(closes_at.is_none());
    }

    #[test]
    fn compute_venue_status_busy_only_when_open_and_future_kitchen_busy_until() {
        let now = dt(2026, 7, 6, 12, 0);
        assert_eq!(
            compute_venue_status(true, Some(dt(2026, 7, 6, 12, 30)), now),
            VenueStatus::Busy
        );
        assert_eq!(
            compute_venue_status(true, Some(dt(2026, 7, 6, 11, 0)), now),
            VenueStatus::Open
        );
        assert_eq!(compute_venue_status(true, None, now), VenueStatus::Open);
        assert_eq!(
            compute_venue_status(false, Some(dt(2026, 7, 6, 12, 30)), now),
            VenueStatus::Closed
        );
    }

    #[test]
    fn compute_weekly_hours_none_when_all_closed() {
        let hours = serde_json::json!({
            "monday": {"isOpen": false}, "tuesday": {"isOpen": false}, "wednesday": {"isOpen": false},
            "thursday": {"isOpen": false}, "friday": {"isOpen": false}, "saturday": {"isOpen": false},
            "sunday": {"isOpen": false},
        });
        assert!(compute_weekly_hours(Some(&hours)).is_none());
        assert!(compute_weekly_hours(None).is_none());
    }

    #[test]
    fn compute_weekly_hours_some_when_any_day_open() {
        let hours = serde_json::json!({"monday": {"open": "09:00", "close": "17:00"}});
        let weekly = compute_weekly_hours(Some(&hours)).unwrap();
        assert_eq!(weekly.len(), 7);
        assert!(weekly[0].is_open); // Monday first per order
        assert_eq!(weekly[0].open.as_deref(), Some("09:00"));
        assert!(!weekly[1].is_open); // Tuesday defaults closed (absent from hours_json)
    }
}
