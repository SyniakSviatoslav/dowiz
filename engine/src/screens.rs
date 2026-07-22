//! screens.rs — role-screen layout (vendor-data-driven SDF scenes).
//!
//! P-screen — pixel-verification spine. Each role's "screen" is a [`Scene`] of
//! SDF primitives laid out from the REAL vendor menu (see `vendor.rs`), not a
//! placeholder box. The composer's `compose_ui.rs` consumes these layouts to
//! produce `ComposedResponse`s; this module is the dedicated, deterministic
//! layout authority a backend pixel-verification harness rasterizes headlessly.
//!
//! Why this exists (the operator directive): the interface is generated + checked
//! on the BACKEND so Playwright (a browser-driven frontend probe) is not needed.
//! `Scene::render_frame` is already bit-deterministic (scene.rs gate 1), so a
//! backend raster + a golden signature IS the falsifiable "does the layout still
//! look right" check — rendered once, verified forever, no GPU/browser in the loop.
//!
//! Layout convention (shared with the WGSL UI shader spine):
//! - World units = field-space "tiles"; one menu card = a 2.0×1.4 rounded box.
//! - The menu grid is a column-major flow: card k sits at
//!   `(col_x, row_y)` where `col_x = -W/2 + 2.0 + 2.4*col`, `row_y = H/2 - 1.2 - 1.7*row_in_col`.
//! - Owner/courier screens reuse the same field space; role changes which
//!   fragments compose, not the world units (one intent interface, role-tinted render).
//!
//! Innovate: ceiling — v1 lays out a fixed grid sized to the first N items; a real
//! flow-layout (variable card heights for set/cocktail categories) is a P-flow
//! upgrade. Trigger: when the composer's generative producer (P64 §8 Track-R)
//! lands, replace `customer_menu_screen` with a `FragmentFn` that flows from
//! `AppState.item_count` instead of a static slice.

use crate::scene::{Scene, SdfShape};
use crate::vendor;

/// Field-space "card" half-extents (a menu product card).
const CARD_HW: f64 = 1.0;
const CARD_HH: f64 = 0.7;
/// Horizontal/vertical pitch between cards in the grid.
const PITCH_X: f64 = 2.4;
const PITCH_Y: f64 = 1.7;

/// The customer "menu" screen — a grid of the vendor's Chef's Picks, laid out as
/// SDF rounded boxes (one per item) so the raster encodes the real menu shape.
/// A cart badge (circle) appears at top-right when `cart_count > 0`.
pub fn customer_menu_screen(cart_count: u32) -> Scene {
    let picks = vendor::by_category("chef");
    let cols = 4usize;
    let rows = (picks.len() + cols - 1) / cols;
    let _ = rows; // rows is implied by per-col placement below
    let mut scene = Scene::new().with_scale(0.25); // 1 tile = 16 px

    for (k, item) in picks.iter().enumerate() {
        let col = k % cols;
        let row = k / cols;
        let cx = -(cols as f64) * PITCH_X * 0.5 + PITCH_X * 0.5 + (col as f64) * PITCH_X;
        // row 0 at the top, increasing row = downward (negative y).
        let cy = 1.0 - (row as f64) * PITCH_Y;
        scene.add(SdfShape::RoundedBox {
            bx: cx,
            by: cy,
            hx: CARD_HW,
            hy: CARD_HH,
            r: 0.18,
        });
        // A tiny line "tab" under each card (the price strip) — uses the vendor
        // item's price presence so the raster differs by content (not just count).
        if !item.drink_ask {
            scene.add(SdfShape::LineSegment {
                ax: cx - CARD_HW * 0.6,
                ay: cy - CARD_HH - 0.05,
                bx: cx + CARD_HW * 0.6,
                by: cy - CARD_HH - 0.05,
            });
        }
    }

    // Cart badge (top-right circle) when cart is non-empty. Kept inside the
    // visible field window (the canvas samples x ∈ [-W/2·s, +W/2·s)), so a cart
    // state change shows up as a discrete jump in inside_count + FNV — the
    // pixel-verify signature distinguishes an empty cart from a non-empty one.
    if cart_count > 0 {
        scene.add(SdfShape::Circle {
            cx: 3.2,
            cy: 1.6,
            r: 0.35,
        });
    }
    scene
}

/// The owner "dashboard" screen — a header band + N stat tiles (N = number of
/// vendor categories) + a sidebar. Encodes the vendor's MENU/CATEGORIES counts so
/// the raster changes when the menu changes (a regression here = a content drift).
pub fn owner_dashboard_screen() -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    // Header band.
    scene.add(SdfShape::RoundedBox {
        bx: 0.0,
        by: 2.6,
        hx: 6.0,
        hy: 0.4,
        r: 0.1,
    });
    // Sidebar.
    scene.add(SdfShape::RoundedBox {
        bx: -5.4,
        by: 0.0,
        hx: 0.7,
        hy: 2.2,
        r: 0.1,
    });
    // One stat tile per category (15 tiles laid out 5×3).
    let cols = 5usize;
    for (k, _cat) in vendor::CATEGORIES.iter().enumerate() {
        let col = k % cols;
        let row = k / cols;
        let cx = -2.8 + (col as f64) * 1.4;
        let cy = 1.0 - (row as f64) * 1.4;
        scene.add(SdfShape::RoundedBox {
            bx: cx,
            by: cy,
            hx: 0.6,
            hy: 0.5,
            r: 0.08,
        });
    }
    scene
}

/// The courier "board" (tasks) screen — a status pill + a vertical task list (the
/// active deliveries). Role-stable (same world units as the customer screen) so
/// the courier render is a tint of the same field.
pub fn courier_board_screen(active_tasks: u32) -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    // Status pill (top-centre).
    scene.add(SdfShape::RoundedBox {
        bx: 0.0,
        by: 2.4,
        hx: 1.5,
        hy: 0.3,
        r: 0.28,
    });
    let n = active_tasks.min(5);
    for k in 0..n {
        let cy = 1.4 - (k as f64) * 0.9;
        scene.add(SdfShape::RoundedBox {
            bx: 0.0,
            by: cy,
            hx: 4.5,
            hy: 0.35,
            r: 0.12,
        });
    }
    scene
}

// ── S7 per-role sub-screen variants ─────────────────────────────────────────
//
// Each of the 30 `.polish/final/` screens gets a layout function here whose
// signature geometry DIFFERS from the dashboard/board so the pixel-verify harness
// can distinguish them by FNV digest (not just by role tint). Owner sub-screens
// share a sidebar (the zero-chrome "one recomposed screen" discipline — they are
// the dashboard recomposed by intent, NOT different routes), but the main panel
// shape varies per screen. Courier sub-screens share the status pill.

/// Owner "orders" screen — a full-width order list (one row per order in the
/// queue). Distinct from the dashboard's category-tile grid: rows are wide &
/// flat (hx 5.0, hy 0.35), stacked vertically.
pub fn owner_orders_screen(order_count: u32) -> Scene {
    let mut scene = owner_shell();
    let n = order_count.min(6);
    for k in 0..n {
        let cy = 1.6 - (k as f64) * 0.6;
        scene.add(SdfShape::RoundedBox {
            bx: 0.6,
            by: cy,
            hx: 5.0,
            hy: 0.35,
            r: 0.12,
        });
    }
    scene
}

/// Owner "menu management" screen — the vendor menu as a dense card grid (one
/// card per item, 6 cols). Distinct from the customer menu (4 cols) so the count
/// differs and the raster separates them.
pub fn owner_menu_screen() -> Scene {
    let mut scene = owner_shell();
    let cols = 6usize;
    for (k, _item) in vendor::MENU.iter().enumerate() {
        let col = k % cols;
        let row = k / cols;
        let cx = -3.0 + (col as f64) * 1.1;
        let cy = 1.8 - (row as f64) * 0.5;
        scene.add(SdfShape::RoundedBox {
            bx: cx,
            by: cy,
            hx: 0.45,
            hy: 0.18,
            r: 0.05,
        });
    }
    scene
}

/// Owner "promotions" screen — a small set of promo cards (3) + a sidebar list.
/// Distinct geometry: 3 large rounded boxes on the right.
pub fn owner_promotions_screen() -> Scene {
    let mut scene = owner_shell();
    for k in 0..3 {
        let cx = 1.5 + (k as f64) * 2.6;
        scene.add(SdfShape::RoundedBox {
            bx: cx,
            by: 0.5,
            hx: 1.1,
            hy: 1.4,
            r: 0.2,
        });
    }
    scene
}

/// Owner "CRM" screen — a customer table layout: header + 5 rows of 2 columns.
pub fn owner_crm_screen() -> Scene {
    let mut scene = owner_shell();
    // Header row.
    scene.add(SdfShape::RoundedBox {
        bx: 0.6,
        by: 2.2,
        hx: 5.0,
        hy: 0.25,
        r: 0.08,
    });
    for k in 0..5 {
        let cy = 1.5 - (k as f64) * 0.5;
        scene.add(SdfShape::LineSegment {
            ax: -4.0,
            ay: cy,
            bx: 5.4,
            by: cy,
        });
    }
    scene
}

/// Owner "supplies" screen — an inventory grid (10 small tiles in 5×2).
pub fn owner_supplies_screen() -> Scene {
    let mut scene = owner_shell();
    for k in 0..10 {
        let col = k % 5;
        let row = k / 5;
        let cx = -2.0 + (col as f64) * 1.0;
        let cy = 1.0 - (row as f64) * 1.2;
        scene.add(SdfShape::Box {
            bx: cx,
            by: cy,
            hx: 0.35,
            hy: 0.4,
        });
    }
    scene
}

/// Owner "couriers" screen — a list of courier status pills (4 rows).
pub fn owner_couriers_screen() -> Scene {
    let mut scene = owner_shell();
    for k in 0..4 {
        let cy = 1.6 - (k as f64) * 0.8;
        scene.add(SdfShape::RoundedBox {
            bx: 0.6,
            by: cy,
            hx: 5.0,
            hy: 0.3,
            r: 0.25,
        });
    }
    scene
}

/// Owner "analytics" screen — a chart placeholder: axes (2 line segments) + 5
/// bars (Boxes of varying height). Distinct line+box combination.
pub fn owner_analytics_screen() -> Scene {
    let mut scene = owner_shell();
    // X + Y axes.
    scene.add(SdfShape::LineSegment {
        ax: -3.0,
        ay: -1.5,
        bx: 5.0,
        by: -1.5,
    });
    scene.add(SdfShape::LineSegment {
        ax: -3.0,
        ay: -1.5,
        bx: -3.0,
        by: 2.2,
    });
    // 5 bars of varying height.
    let heights = [0.8, 1.4, 1.0, 1.8, 1.2];
    for (k, h) in heights.iter().enumerate() {
        let cx = -2.0 + (k as f64) * 1.4;
        scene.add(SdfShape::Box {
            bx: cx,
            by: -1.5 + h * 0.5,
            hx: 0.4,
            hy: *h,
        });
    }
    scene
}

/// Owner "settings" screen — a 4-row settings list (each row = line + small box).
pub fn owner_settings_screen() -> Scene {
    let mut scene = owner_shell();
    for k in 0..4 {
        let cy = 1.4 - (k as f64) * 0.7;
        scene.add(SdfShape::LineSegment {
            ax: -3.5,
            ay: cy,
            bx: 4.5,
            by: cy,
        });
        scene.add(SdfShape::RoundedBox {
            bx: 4.2,
            by: cy,
            hx: 0.3,
            hy: 0.18,
            r: 0.1,
        });
    }
    scene
}

/// Owner "branding" screen — a large preview frame + 3 swatch circles (the brand
/// palette pickers). Distinct: uses Circle shapes (the raster's only circles here).
pub fn owner_branding_screen() -> Scene {
    let mut scene = owner_shell();
    // Preview frame.
    scene.add(SdfShape::RoundedBox {
        bx: -1.5,
        by: 0.0,
        hx: 1.8,
        hy: 2.0,
        r: 0.15,
    });
    // 3 palette swatches (circles — unique to this screen).
    for (k, cx) in [1.8, 2.8, 3.8].iter().enumerate() {
        let _ = k;
        scene.add(SdfShape::Circle {
            cx: *cx,
            cy: 0.5,
            r: 0.4,
        });
    }
    scene
}

/// Owner "activation" screen — a plan-card (1 large rounded box) + a status pill.
pub fn owner_activation_screen() -> Scene {
    let mut scene = owner_shell();
    scene.add(SdfShape::RoundedBox {
        bx: 0.5,
        by: 0.3,
        hx: 3.5,
        hy: 2.2,
        r: 0.2,
    });
    scene.add(SdfShape::RoundedBox {
        bx: -1.5,
        by: -2.0,
        hx: 1.5,
        hy: 0.3,
        r: 0.25,
    });
    scene
}

/// Courier "home" screen — the courier_board variant with an online-status pill
/// and no task rows (the landing state). Distinct from board by the pill position.
pub fn courier_home_screen() -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    scene.add(SdfShape::RoundedBox {
        bx: 0.0,
        by: 1.8,
        hx: 2.0,
        hy: 0.4,
        r: 0.3,
    });
    scene.add(SdfShape::Circle {
        cx: -2.8,
        cy: 1.8,
        r: 0.25,
    }); // online dot
    scene
}

/// Courier "shift" screen — a shift timer ring (a large circle outline approximated
/// by a circle) + a start/stop button box.
pub fn courier_shift_screen() -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    scene.add(SdfShape::Circle {
        cx: 0.0,
        cy: 0.5,
        r: 2.0,
    });
    scene.add(SdfShape::RoundedBox {
        bx: 0.0,
        by: -2.2,
        hx: 1.5,
        hy: 0.4,
        r: 0.2,
    });
    scene
}

/// Courier "earnings" screen — a total banner + 3 period bars (day/week/month).
pub fn courier_earnings_screen() -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    scene.add(SdfShape::RoundedBox {
        bx: 0.0,
        by: 2.4,
        hx: 5.0,
        hy: 0.35,
        r: 0.1,
    });
    let heights = [0.6, 1.2, 1.8];
    for (k, h) in heights.iter().enumerate() {
        let cx = -2.0 + (k as f64) * 2.0;
        scene.add(SdfShape::Box {
            bx: cx,
            by: -0.5 + h * 0.5,
            hx: 0.7,
            hy: *h,
        });
    }
    scene
}

/// Courier "history" screen — a vertical timeline: 6 dots (circles) connected by
/// a vertical line.
pub fn courier_history_screen() -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    scene.add(SdfShape::LineSegment {
        ax: 0.0,
        ay: 2.4,
        bx: 0.0,
        by: -2.4,
    });
    for k in 0..6 {
        let cy = 2.0 - (k as f64) * 0.8;
        scene.add(SdfShape::Circle {
            cx: 0.0,
            cy,
            r: 0.2,
        });
    }
    scene
}

/// Shared owner shell — header band + sidebar (present on every owner sub-screen;
/// the recomposed-screen discipline means sub-screens share the chrome, differing
/// only in the main panel).-owner_dashboard_screen builds its own version inline
/// (it does not use the shell) because the dashboard IS the base surface.
fn owner_shell() -> Scene {
    let mut scene = Scene::new().with_scale(0.25);
    scene.add(SdfShape::RoundedBox {
        bx: 0.0,
        by: 2.7,
        hx: 6.0,
        hy: 0.3,
        r: 0.08,
    });
    scene.add(SdfShape::RoundedBox {
        bx: -5.5,
        by: 0.0,
        hx: 0.5,
        hy: 2.3,
        r: 0.1,
    });
    scene
}
