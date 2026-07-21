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

/// Menu fragment: a rounded box at the menu centre.
fn menu_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    vec![SdfShape::RoundedBox {
        bx: cx as f64,
        by: cy as f64,
        hx: 4.0,
        hy: 3.0,
        r: 0.5,
    }]
}

fn cart_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    let _ = state.cart_count;
    vec![SdfShape::Box {
        bx: (cx + 6.0) as f64,
        by: cy as f64,
        hx: 2.0,
        hy: 2.0,
    }]
}

fn catalog_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    vec![SdfShape::Box {
        bx: (cx - 6.0) as f64,
        by: cy as f64,
        hx: 3.0,
        hy: 2.0,
    }]
}

fn checkout_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    vec![SdfShape::RoundedBox {
        bx: cx as f64,
        by: (cy + 6.0) as f64,
        hx: 3.0,
        hy: 2.0,
        r: 0.3,
    }]
}

fn owner_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    vec![SdfShape::Box {
        bx: (cx + 9.0) as f64,
        by: cy as f64,
        hx: 2.5,
        hy: 2.5,
    }]
}

fn courier_fragment(state: &AppState) -> Vec<SdfShape> {
    let (cx, cy) = state.menu_center;
    vec![SdfShape::Box {
        bx: (cx - 9.0) as f64,
        by: cy as f64,
        hx: 2.5,
        hy: 2.5,
    }]
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
        }
    }

    // D4 — Intent::Navigate(Menu) → ComposedResponse.scene contains the menu
    //      fragment's SdfShapes; mirror names the menu region.
    #[test]
    fn intent_composes_registered_fragment() {
        let composer = Composer::new();
        let resp = composer.compose(&Intent::Navigate(NavTarget::Menu), &state());
        // The menu fragment is a single RoundedBox; assert the shape count + a known centre.
        assert_eq!(resp.scene.shape_count(), 1, "menu fragment = 1 rounded box");
        assert_eq!(
            resp.scene.shapes()[0],
            SdfShape::RoundedBox {
                bx: 0.0,
                by: 0.0,
                hx: 4.0,
                hy: 3.0,
                r: 0.5
            }
        );
        // Mirror names the menu region (P58 hook).
        assert!(resp
            .mirror
            .nodes
            .iter()
            .any(|n| n.role == "region" && n.name == "nav:Menu"));
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
