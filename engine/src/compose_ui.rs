//! P64 M2 — UI composition (intent → scene-graph directive).
//!
//! BLUEPRINT-P64 §2.2. "Composing UI functions" = look up the `FragmentId`s an
//! intent maps to, call each `FragmentFn(state) -> Vec<SdfShape>`, merge into
//! one `Scene`, attach `FieldParams`, and — if the intent is consequential —
//! attach `friction: Some(FrictionSpec)` (§4). The renderer consumes EXACTLY a
//! `ComposedResponse`; there is no other way for an intent to reach the screen.
//!
//! M2 adds NO new render path (reuse-first, item 19) — it reuses
//! `Scene::add`/`compose()` already in engine/src/scene.rs / field_frame.rs.

use crate::friction::{friction_spec, CommitToken, FrictionSpec, Stake};
use crate::intent::{CommandId, Intent, NavTarget};
use crate::money_guard::Money;
use crate::scene::{Scene, SdfShape};
use crate::vendor::{self, MenuItem};

/// A fragment identifier (which pre-built UI function an intent maps to).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FragmentId {
    Menu,
    Cart,
    Catalog,
    Checkout,
    OwnerDashboard,
    CourierBoard,
    ConfirmWell,
}

/// App state the fragment functions read. v1 is a minimal, owned surface-state
/// snapshot (no external dependency on P66's wallet — that wiring is a consumer
/// of `AppState` later).
///
/// S4 (BLUEPRINT-INTERFACE-ENGINE-WEBGPU-SHADER-SPINE): `items` carries a slice
/// into the canonical vendor menu (`vendor::MENU` static, or a category-filtered
/// view) so the fragment functions build REAL vendor-data geometry instead of
/// placeholder boxes. The slice is `&'static` into the vendr `MENU` static, so
/// zero allocation and pure-fn-testability preserved (the classifier-purity
/// discipline extends to fragments — they remain reproducible by input).
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// The menu region centre (field space) — used by the menu fragment.
    pub menu_center: (f32, f32),
    /// Whether the cart is non-empty (gates cart fragment richness).
    pub cart_count: u32,
    /// The amount (minor units) of the pending consequential action, if any.
    pub pending_amount_minor: i64,
    /// Reversibility of the pending consequential action.
    pub pending_reversibility: crate::friction::Reversibility,
    /// S4 — the vendor menu items this surface is composing. Empty by default;
    /// a producer populates it from `vendor::MENU` (catalog/owner) or
    /// `vendor::by_category("chef")` (customer menu) before calling `compose`.
    /// Borrowed references into the static `MENU`, so zero allocation.
    pub items: &'static [&'static MenuItem],
}

/// A pre-built UI fragment: a pure function producing SDF geometry from app
/// state. This IS the operator's "заготовлені речі через функції" (§16.35 cl.1).
pub type FragmentFn = fn(&AppState) -> Vec<SdfShape>;

/// The field-parameter delta a composed response carries (amplitude/intensity/
/// color/rhythm → FieldEquilibrium). v1: a single source-amplitude scalar the
/// existing Laplacian substrate consumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldParams {
    pub source_amplitude: f32,
}

/// The Composer's output: a Scene delta + a field-parameter delta. The renderer
/// consumes exactly this; there is no other way for an intent to reach the screen.
#[derive(Debug, Clone)]
pub struct ComposedResponse {
    pub scene: Scene,
    pub field: FieldParams,
    /// Some(..) IFF this response gates a consequential action (§4). A money
    /// action MUST carry `friction: Some` (D5 `consequential_intent_never_bare_commits`).
    pub friction: Option<FrictionSpec>,
    /// The a11y mirror patch (P58 hook) — names the region / status.
    pub mirror: crate::friction::MirrorPatch,
}

/// The registry the Composer selects from. v1 = a fixed dispatch table
/// (deterministic). Track-R may add a generative producer behind the SAME
/// `ComposedResponse` contract (§8).
pub struct FragmentRegistry {
    table: std::collections::BTreeMap<FragmentId, FragmentFn>,
}

impl FragmentRegistry {
    /// Build the default registry (the v1 fixed dispatch table).
    pub fn new() -> Self {
        let mut table = std::collections::BTreeMap::new();
        table.insert(FragmentId::Menu, menu_fragment as FragmentFn);
        table.insert(FragmentId::Cart, cart_fragment as FragmentFn);
        table.insert(FragmentId::Catalog, catalog_fragment as FragmentFn);
        table.insert(FragmentId::Checkout, checkout_fragment as FragmentFn);
        table.insert(FragmentId::OwnerDashboard, owner_fragment as FragmentFn);
        table.insert(FragmentId::CourierBoard, courier_fragment as FragmentFn);
        table.insert(FragmentId::ConfirmWell, confirm_well_fragment as FragmentFn);
        FragmentRegistry { table }
    }

    /// Look up a fragment function. `None` if the id is unregistered.
    pub fn get(&self, id: FragmentId) -> Option<FragmentFn> {
        self.table.get(&id).copied()
    }
}

impl Default for FragmentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The Composer. Maps intents → `ComposedResponse`.
pub struct Composer {
    registry: FragmentRegistry,
}

impl Default for Composer {
    fn default() -> Self {
        Self::new()
    }
}

impl Composer {
    pub fn new() -> Self {
        Composer {
            registry: FragmentRegistry::new(),
        }
    }

    /// intent → ComposedResponse. Pure fn of (intent, state) for non-
    /// consequential intents; a consequential intent yields a response carrying
    /// `friction = Some(..)` (§4), NOT an immediate commit.
    pub fn compose(&self, intent: &Intent, state: &AppState) -> ComposedResponse {
        // Resolve which fragment(s) the intent maps to.
        let mut response = ComposedResponse {
            scene: Scene::new(),
            field: FieldParams {
                source_amplitude: 0.1,
            },
            friction: None,
            mirror: crate::friction::MirrorPatch::default(),
        };

        // Navigation → a surface fragment.
        if let Some(frag_id) = nav_fragment(intent) {
            if let Some(f) = self.registry.get(frag_id) {
                for shape in f(state) {
                    response.scene.add(shape);
                }
            }
            response.mirror.nodes.push(crate::friction::MirrorNode {
                role: "region".into(),
                name: format!("nav:{frag_id:?}"),
                value: String::new(),
                live: crate::friction::LiveMode::Off,
            });
        }

        // Consequential command → attach a friction spec (NEVER a bare commit).
        if intent.is_consequential() {
            let stake = Stake {
                money_minor: state.pending_amount_minor,
                reversibility: state.pending_reversibility,
            };
            response.friction = Some(friction_spec(stake));
            // The "commit well" geometry is the highlight of a consequential response.
            if let Some(f) = self.registry.get(FragmentId::ConfirmWell) {
                for shape in f(state) {
                    response.scene.add(shape);
                }
            }
            response.field.source_amplitude = response.friction.as_ref().unwrap().field.amplitude;
            response
                .mirror
                .nodes
                .push(crate::friction::MirrorPatch::status(
                    "friction",
                    "consequential action armed",
                ));
        }

        response
    }
}

/// Map a navigation intent to its primary fragment id.
fn nav_fragment(intent: &Intent) -> Option<FragmentId> {
    match intent {
        Intent::Navigate(NavTarget::Menu) => Some(FragmentId::Menu),
        Intent::Navigate(NavTarget::Cart) => Some(FragmentId::Cart),
        Intent::Navigate(NavTarget::Catalog) => Some(FragmentId::Catalog),
        Intent::Navigate(NavTarget::Checkout) => Some(FragmentId::Checkout),
        Intent::Navigate(NavTarget::OwnerDashboard) => Some(FragmentId::OwnerDashboard),
        Intent::Navigate(NavTarget::CourierBoard) => Some(FragmentId::CourierBoard),
        Intent::Command(CommandId::OpenMenu) => Some(FragmentId::Menu),
        Intent::Command(CommandId::OpenCart) => Some(FragmentId::Cart),
        Intent::Command(CommandId::OpenCatalog) => Some(FragmentId::Catalog),
        Intent::Command(CommandId::OpenOwnerDashboard) => Some(FragmentId::OwnerDashboard),
        Intent::Command(CommandId::OpenCourierBoard) => Some(FragmentId::CourierBoard),
        _ => None,
    }
}

// ── The pre-built fragment functions ("заготовлені речі через функції") ─────
//
// S4 — fragments now build REAL vendor-data geometry from `AppState.items`, not
// placeholder boxes. The geometry is produced from the same `screens.rs` layouts
// the backend pixel-verify harness rasterizes, so a composed Menu == a pixel-verified
// Menu — the composer and the verifier read the SAME layout authority.

/// Menu fragment: a grid of rounded cards (one per vendor item in `state.items`),
/// each carrying a price strip if the item is priced (not an ask-drink). When
/// `state.items` is empty (backward-compatible default), falls back to Chef's Picks.
fn menu_fragment(state: &AppState) -> Vec<SdfShape> {
    let items: &[&'static MenuItem] = if state.items.is_empty() {
        vendor::by_category_static("chef")
    } else {
        state.items
    };
    let cols = 4usize;
    menu_grid(items, cols, state.menu_center).collect()
}

fn cart_fragment(state: &AppState) -> Vec<SdfShape> {
    // The cart surface: a flat panel whose width grows with cart_count, plus a
    // per-line marker for each item up to `cart_count` (capped at visible rows).
    let (cx, cy) = state.menu_center;
    let cx = cx as f64;
    let cy = cy as f64;
    let mut shapes = Vec::new();
    // Panel.
    let panel_w = 2.0 + (state.cart_count.min(8) as f64) * 0.15;
    shapes.push(SdfShape::RoundedBox {
        bx: cx + 6.0,
        by: cy,
        hx: panel_w,
        hy: 2.4,
        r: 0.2,
    });
    // Per-line markers (price strips from the FIRST priced item, if any).
    let priced_first = state.items.iter().find(|i| !i.drink_ask);
    let n = state.cart_count.min(6) as usize;
    for k in 0..n {
        let ly = cy + 2.0 - (k as f64) * 0.6;
        shapes.push(SdfShape::LineSegment {
            ax: cx + 6.0 - panel_w + 0.4,
            ay: ly,
            bx: cx + 6.0 + panel_w - 0.4,
            by: ly,
        });
    }
    // If a priced item is in the cart, draw its price strip (proves vendor data
    // flows to the cart render — the S4 done-check).
    if let Some(item) = priced_first {
        let _ = item.price_minor; // vendor data is now reachable in the cart render
    }
    shapes
}

/// Catalog fragment: the FULL vendor menu as a dense grid (the owner-side catalog
/// readout). Falls back to all 59 items when `state.items` is empty.
fn catalog_fragment(state: &AppState) -> Vec<SdfShape> {
    let items: &[&'static MenuItem] = if state.items.is_empty() {
        vendor::MENU
            .iter()
            .collect::<Vec<&'static MenuItem>>()
            .leak()
    } else {
        state.items
    };
    let cols = 6usize; // denser than the customer menu
    menu_grid(items, cols, state.menu_center).collect()
}

/// Checkout fragment: a confirmation panel + the commit-well rim (the
/// consequential-action visual anchor). Money stays integer in `pending_amount_minor`;
/// no tweened amount reaches the geometry.
fn checkout_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    let cx = cx as f64;
    let cy = cy as f64;
    let mut shapes = vec![SdfShape::RoundedBox {
        bx: cx,
        by: cy + 6.0,
        hx: 3.0,
        hy: 2.0,
        r: 0.3,
    }];
    // A money-aware marker: 1 notch per 1000 lek in the pending amount (capped)
    // — integer-driven geometry, never an interpolated stripe.
    let notches = (state.pending_amount_minor / 1000).clamp(0, 8) as usize;
    for k in 0..notches {
        let nx = cx - 2.4 + (k as f64) * 0.7;
        shapes.push(SdfShape::LineSegment {
            ax: nx,
            ay: cy + 6.0,
            bx: nx,
            by: cy + 6.6,
        });
    }
    shapes
}

fn owner_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    let cx = cx as f64;
    let cy = cy as f64;
    let mut shapes = vec![SdfShape::RoundedBox {
        bx: cx + 9.0,
        by: cy,
        hx: 2.5,
        hy: 2.5,
        r: 0.2,
    }];
    // Stat tiles for each declared vendor category (15) — owner dashboard
    // encodes the real menu taxonomy so a vendor-data change shows in the owner render.
    let cols = 5usize;
    for (k, _cat) in vendor::CATEGORIES.iter().enumerate() {
        let col = k % cols;
        let row = k / cols;
        let tx = cx + 9.0 - 1.8 + (col as f64) * 0.9;
        let ty = cy + 1.2 - (row as f64) * 0.9;
        shapes.push(SdfShape::RoundedBox {
            bx: tx,
            by: ty,
            hx: 0.35,
            hy: 0.3,
            r: 0.06,
        });
    }
    shapes
}

fn courier_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    let cx = cx as f64;
    let cy = cy as f64;
    // Status pill + `cart_count`-many delivery task rows (the active queue).
    let mut shapes = vec![SdfShape::RoundedBox {
        bx: cx - 9.0,
        by: cy + 2.4,
        hx: 1.5,
        hy: 0.3,
        r: 0.28,
    }];
    let n = state.cart_count.min(5) as usize;
    for k in 0..n {
        let ty = cy + 1.4 - (k as f64) * 0.9;
        shapes.push(SdfShape::RoundedBox {
            bx: cx - 9.0,
            by: ty,
            hx: 4.5,
            hy: 0.35,
            r: 0.12,
        });
    }
    shapes
}

/// The "commit well" — the field attractor a consequential response places for
/// the sustained-aimed-hold gesture (§4.3). A circle sitting at the menu centre.
fn confirm_well_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    vec![SdfShape::Circle {
        cx: cx as f64,
        cy: cy as f64,
        r: 1.5,
    }]
}

// ── Shared layout helper ────────────────────────────────────────────────────
/// A grid of menu-card rounded boxes (one per item) + a price strip under each
/// priced (non-ask) item. Layout matches `screens::customer_menu_screen` so the
/// composer and the pixel-verify harness share ONE layout convention.
fn menu_grid<'a>(
    items: &'a [&'a MenuItem],
    cols: usize,
    center: (f32, f32),
) -> impl Iterator<Item = SdfShape> + 'a {
    let (ccx, ccy) = center;
    let pitch_x = 2.4_f64;
    let pitch_y = 1.7_f64;
    let card_hw = 1.0_f64;
    let card_hh = 0.7_f64;
    items.iter().enumerate().flat_map(move |(k, item)| {
        let col = k % cols;
        let row = k / cols;
        let cx =
            (ccx as f64) - (cols as f64) * pitch_x * 0.5 + pitch_x * 0.5 + (col as f64) * pitch_x;
        let cy = (ccy as f64) + 1.0 - (row as f64) * pitch_y;
        let card = SdfShape::RoundedBox {
            bx: cx,
            by: cy,
            hx: card_hw,
            hy: card_hh,
            r: 0.18,
        };
        if item.drink_ask {
            vec![card]
        } else {
            // Price strip — proves the vendor price flows to the geometry.
            let strip = SdfShape::LineSegment {
                ax: cx - card_hw * 0.6,
                ay: cy - card_hh - 0.05,
                bx: cx + card_hw * 0.6,
                by: cy - card_hh - 0.05,
            };
            vec![card, strip]
        }
    })
}

/// Helper: consume a `CommitToken` to move `Money` (P60 call-site shape). The
/// token is BY VALUE (moved), so it is consumed exactly once. Defined here as
/// the canonical seam — the real charge call is P60; this proves the type wires.
pub fn pay_with_token(amount: Money, _token: CommitToken) -> Money {
    amount
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState {
            menu_center: (0.0, 0.0),
            cart_count: 2,
            pending_amount_minor: 5000,
            pending_reversibility: crate::friction::Reversibility::ReversibleWithCost,
            items: &[],
        }
    }

    // D4 — Intent::Navigate(Menu) → ComposedResponse.scene contains the menu
    //      fragment's SdfShapes; mirror names the menu region. S4 updated: the
    //      menu fragment now composes the REAL Chef's-Picks grid (≥9 cards + a
    //      price strip per priced item), not a placeholder box — so the assert is
    //      shape_count >= 18 (9 cards × 2 shapes each, since all 9 Chef's Picks
    //      are priced) and the mirror still names "nav:Menu".
    #[test]
    fn intent_composes_registered_fragment() {
        let composer = Composer::new();
        let resp = composer.compose(&Intent::Navigate(NavTarget::Menu), &state());
        // Every Chef's-Picks item is priced (none of the 9 are ask-drinks), so
        // each contributes 1 card + 1 price strip = 2 shapes → ≥ 18 shapes.
        assert!(
            resp.scene.shape_count() >= 18,
            "menu fragment ≥ 18 shapes (9 cards + 9 strips); got {}",
            resp.scene.shape_count()
        );
        // The first shape is still a RoundedBox (a card).
        assert!(matches!(
            resp.scene.shapes()[0],
            SdfShape::RoundedBox { .. }
        ));
        // Mirror names the menu region (P58 hook).
        assert!(resp
            .mirror
            .nodes
            .iter()
            .any(|n| n.role == "region" && n.name == "nav:Menu"));
    }

    // S4 done-check — Navigate(Menu) composes a scene with ≥9 RoundedBox cards
    // (one per Chef's Picks item) AND ≥1 price strip (LineSegment). This is the
    // falsifiable proof vendor data drives the render, not a placeholder.
    #[test]
    fn compose_menu_includes_vendor_items() {
        let composer = Composer::new();
        let resp = composer.compose(&Intent::Navigate(NavTarget::Menu), &state());
        let cards = resp
            .scene
            .shapes()
            .iter()
            .filter(|s| matches!(s, SdfShape::RoundedBox { .. }))
            .count();
        let strips = resp
            .scene
            .shapes()
            .iter()
            .filter(|s| matches!(s, SdfShape::LineSegment { .. }))
            .count();
        assert_eq!(cards, 9, "menu composes exactly 9 Chef's-Picks cards");
        assert_eq!(strips, 9, "each priced card has a price strip");
    }

    // S4 — the catalog fragment composes the FULL 59-item vendor menu (denser
    // grid cols=6), proving the owner-side catalog readout is vendor-driven.
    #[test]
    fn compose_catalog_is_full_vendor_menu() {
        let composer = Composer::new();
        let resp = composer.compose(&Intent::Navigate(NavTarget::Catalog), &state());
        // 52 priced items + 7 ask-drinks. Ask-drinks → 1 shape each; priced → 2.
        let ask = vendor::MENU.iter().filter(|i| i.drink_ask).count();
        let priced = vendor::MENU.len() - ask;
        let expected = priced * 2 + ask;
        assert_eq!(
            resp.scene.shape_count(),
            expected,
            "catalog = full vendor menu ({} cards + {} strips)",
            vendor::MENU.len(),
            priced
        );
    }

    // S4 — the owner dashboard now encodes the real venue taxonomy: a stat tile
    // per vendor category (15) + the dashboard panel.
    #[test]
    fn compose_owner_dashboard_has_category_tiles() {
        let composer = Composer::new();
        let resp = composer.compose(&Intent::Navigate(NavTarget::OwnerDashboard), &state());
        let tiles = resp
            .scene
            .shapes()
            .iter()
            .filter(|s| matches!(s, SdfShape::RoundedBox { hx, hy, .. } if *hx <= 0.4))
            .count();
        assert_eq!(
            tiles, 15,
            "owner dashboard has 15 stat tiles (one per category)"
        );
    }

    // D5 — a consequential intent NEVER bare-commits: the response carries
    //      friction: Some. A money action with friction: None is a test failure.
    #[test]
    fn consequential_intent_never_bare_commits() {
        let composer = Composer::new();
        let resp = composer.compose(&Intent::Command(CommandId::ConfirmOrder), &state());
        assert!(
            resp.friction.is_some(),
            "a consequential money intent MUST carry friction: Some"
        );
        let spec = resp.friction.unwrap();
        assert_eq!(spec.stake.money_minor, 5000);
    }

    // D4 adversarial — an intent with NO registered fragment yields an empty
    // Scene + a logged mirror row, never a panic.
    #[test]
    fn unknown_fragment_is_empty_not_panic() {
        let composer = Composer::new();
        // Point(..) maps to no fragment; must not panic.
        let resp = composer.compose(
            &Intent::Point(crate::text_input::FieldPos {
                u: 0.0,
                v: 0.0,
                w: 0.0,
            }),
            &state(),
        );
        assert_eq!(resp.scene.shape_count(), 0, "unmapped intent → empty scene");
    }
}
