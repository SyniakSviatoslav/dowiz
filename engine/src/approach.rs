//! Approach — кути огляду для "останньої милі" доставки.
//!
//! Клієнт може виставити кут огляду для адреси доставки — звідки він
//! очікує кур'єра. Система обчислює bearing, перевіряє пряму видимість
//! (line-of-sight) та генерує підказку для взаємознаходження.
//!
//! Використовує `kernel::geo::bearing_deg`, `angular_diff_deg`,
//! `in_field_of_view`, `los_clear`.

use std::sync::atomic::{AtomicU64, Ordering};
use dowiz_kernel::geo;

static APPROACH_TOTAL: AtomicU64 = AtomicU64::new(0);
static LOS_CHECK_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Кут огляду для підходу кур'єра.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApproachAngle {
    /// Звідки приходить кур'єр (compass degrees, 0=N).
    pub bearing_from_courier: f32,
    /// Куди дивитись клієнту (compass degrees, 0=N).
    pub bearing_to_address: f32,
    /// Відстань до адреси (метри).
    pub distance_m: f32,
    /// Поле огляду для підказки (градуси).
    pub fov_deg: f32,
    /// Чи є пряма видимість (немає будівель на шляху).
    pub los_clear: bool,
}

/// Підказка для UI про підхід кур'єра.
#[derive(Debug, Clone)]
pub struct CourierApproachHint {
    pub approach: ApproachAngle,
    /// Ротація стрілки на екрані (градуси, 0 = north-up).
    pub arrow_rotation: f32,
    /// Текстове позначення напрямку.
    pub label: &'static str,
}

/// Telemetry: загальна кількість обчислених підходів (від початку програми).
pub fn approach_count() -> u64 {
    APPROACH_TOTAL.load(Ordering::Relaxed)
}

/// Telemetry: загальна кількість LOS-перевірок (від початку програми).
pub fn los_check_count() -> u64 {
    LOS_CHECK_TOTAL.load(Ordering::Relaxed)
}

/// Визначити кут підходу кур'єра до адреси.
///
/// * `courier` — поточна позиція кур'єра (lat, lng).
/// * `address` — адреса доставки (lat, lng).
/// * `address_bearing` — кут огляду, який виставив клієнт (опціонально).
///    Якщо `None`, використовується стандартне FOV 120°.
/// * `footprints` — контури будівель для LOS-перевірки (опціонально).
pub fn calculate_approach(
    courier: (f64, f64),
    address: (f64, f64),
    address_bearing: Option<f32>,
    footprints: &[Vec<(f64, f64)>],
) -> ApproachAngle {
    APPROACH_TOTAL.fetch_add(1, Ordering::Relaxed);
    crate::telemetry_count!("approach", "calculate", 1);
    let bearing = geo::bearing_deg(courier.0, courier.1, address.0, address.1) as f32;
    let to_address = (bearing + 180.0) % 360.0;
    let dist = geo::haversine_meters(courier.0, courier.1, address.0, address.1) as f32;
    let los = if footprints.is_empty() {
        true
    } else {
        LOS_CHECK_TOTAL.fetch_add(1, Ordering::Relaxed);
        geo::los_clear(courier, address, footprints)
    };
    let fov = address_bearing.unwrap_or(120.0);
    ApproachAngle {
        bearing_from_courier: bearing,
        bearing_to_address: to_address,
        distance_m: dist,
        fov_deg: fov,
        los_clear: los,
    }
}

/// Згенерувати підказку для UI на основі кута підходу.
pub fn generate_hint(approach: &ApproachAngle) -> CourierApproachHint {
    let label = direction_label(approach.bearing_from_courier);
    CourierApproachHint {
        approach: *approach,
        arrow_rotation: approach.bearing_from_courier,
        label,
    }
}

/// Перевірити, чи клієнт бачить кур'єра (в межах FOV).
pub fn client_sees_courier(
    address_bearing: f32,
    courier_bearing_from_address: f32,
    fov_deg: f32,
) -> bool {
    geo::in_field_of_view(address_bearing as f64, courier_bearing_from_address as f64, fov_deg as f64)
}

/// Текстова мітка напрямку для UI.
pub fn direction_label(bearing_deg: f32) -> &'static str {
    let b = ((bearing_deg + 22.5) % 360.0 / 45.0) as u32;
    match b {
        0 => "N",
        1 => "NE",
        2 => "E",
        3 => "SE",
        4 => "S",
        5 => "SW",
        6 => "W",
        7 => "NW",
        _ => "N",
    }
}

/// Кут на екрані для стрілки (north-up).
pub fn arrow_rotation(bearing_deg: f32, view_rotation: f32) -> f32 {
    geo::arrow_screen_rotation_deg(bearing_deg as f64, view_rotation as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approach_bearing_matches_kernel() {
        let courier = (50.45, 30.52);
        let address = (50.455, 30.53);
        let a = calculate_approach(courier, address, None, &[]);
        let expected = geo::bearing_deg(courier.0, courier.1, address.0, address.1) as f32;
        assert!(
            (a.bearing_from_courier - expected).abs() < 1e-4,
            "bearing must match kernel: got {} expected {}",
            a.bearing_from_courier, expected
        );
    }

    #[test]
    fn address_bearing_overrides_fov() {
        let courier = (50.45, 30.52);
        let address = (50.455, 30.53);
        let a = calculate_approach(courier, address, Some(90.0), &[]);
        assert!((a.fov_deg - 90.0).abs() < 1e-4, "FOV must be overridden to 90, got {}", a.fov_deg);
    }

    #[test]
    fn los_clear_when_no_footprints() {
        let courier = (50.45, 30.52);
        let address = (50.455, 30.53);
        let a = calculate_approach(courier, address, None, &[]);
        assert!(a.los_clear, "without footprints LOS must be clear");
    }

    #[test]
    fn los_blocked_by_building() {
        let courier = (1.0, -1.0);
        let address = (1.0, 3.0);
        let rect = vec![(0.0, 0.0), (0.0, 2.0), (2.0, 2.0), (2.0, 0.0)];
        let a = calculate_approach(courier, address, None, &[rect]);
        assert!(!a.los_clear, "LOS must be blocked by building");
    }

    #[test]
    fn distance_is_positive() {
        let courier = (50.45, 30.52);
        let address = (50.455, 30.53);
        let a = calculate_approach(courier, address, None, &[]);
        assert!(a.distance_m > 0.0, "must have positive distance");
    }

    #[test]
    fn bearing_to_address_is_opposite() {
        let courier = (50.45, 30.52);
        let address = (50.455, 30.53);
        let a = calculate_approach(courier, address, None, &[]);
        let diff = (a.bearing_from_courier - a.bearing_to_address + 180.0) % 360.0;
        assert!(
            diff.abs() < 1.0,
            "to_address must be opposite: diff={}", diff
        );
    }

    #[test]
    fn generate_hint_labels() {
        for deg in &[0.0_f32, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0] {
            let approach = ApproachAngle {
                bearing_from_courier: *deg,
                bearing_to_address: (*deg + 180.0) % 360.0,
                distance_m: 100.0,
                fov_deg: 120.0,
                los_clear: true,
            };
            let hint = generate_hint(&approach);
            assert!(!hint.label.is_empty());
            assert!((hint.arrow_rotation - deg).abs() < 1e-4);
        }
    }

    #[test]
    fn direction_labels_cover_all() {
        let labels: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
        for (i, expected) in labels.iter().enumerate() {
            let deg = i as f32 * 45.0;
            assert_eq!(direction_label(deg), *expected, "at {} deg", deg);
        }
    }

    #[test]
    fn client_sees_courier_in_fov() {
        assert!(client_sees_courier(0.0, 10.0, 120.0));
        assert!(!client_sees_courier(0.0, 200.0, 120.0));
    }

    #[test]
    fn arrow_rotation_with_view() {
        let rot = arrow_rotation(45.0, 10.0);
        assert!((rot - 35.0).abs() < 1e-4);
    }

    #[test]
    fn approach_near_zero_distance() {
        let a = calculate_approach((50.45, 30.52), (50.45, 30.52), None, &[]);
        assert!(a.distance_m < 1.0, "same point distance near zero");
    }
}
