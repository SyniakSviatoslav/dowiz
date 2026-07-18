//! P58 — a11y-mirror-everywhere: the ONE semantic source of truth (M1) + the
//! draft-parity invariant (M6).
//!
//! This module is the shared contract every surface blueprint (P69/P70/P71/P73,
//! P57's a11y half, P51's map) consumes. It is PURE Rust, zero-dependency, and
//! **presentation-free**: `mirror(&SemanticScene)` takes *only* the semantic
//! scene — no design token, no view/projection matrix, no render floor (those
//! live in the render engine's `FrameUniforms`, a different type). That absence
//! is load-bearing: §M6's draft-parity invariant is structural, not a runtime
//! check. Because the signature cannot accept a presentation input, the
//! "accessible in draft, inaccessible in preview" failure mode is
//! *un-representable*, not merely tested-against.
//!
//! The same `SemanticScene` also feeds the GPU frame (`field_frame::compose`),
//! so one state ⇒ one frame AND one accessible mirror — the keystone of X1.

use std::collections::HashMap;

/// Stable per-widget identity across frames. The reconcile key for both the
/// web DOM mirror and the native AccessKit tree.
pub type WidgetId = u32;

/// AccessKit-aligned role set (Wave-0 subset). Each variant maps 1:1 onto an
/// `accesskit::Role` (so the native adapter is a table lookup, not a translation
/// layer — see `a11y_native.rs`) and onto an ARIA role token for the web mirror.
///
/// The `match` in `a11y_native.rs::role_to_accesskit` is exhaustive with **no**
/// `_` arm: adding a `Role` variant without a mapping is a COMPILE ERROR on the
/// `a11y_native` build (§M4 adversarial), not a runtime skip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Role {
    Group,
    Heading,
    Label,
    Button,
    Link,
    Image,
    List,
    ListItem,
    /// order-state / progress — web: `aria-live="polite"` (§4.1 money-as-text).
    Status,
    /// errors / money-action confirmation — web: `aria-live="assertive"`.
    Alert,
    /// the synthetic ARIA-textbox surface (§M3); native: text field w/ selection.
    TextInput,
}

/// Semantic state. `value_text` carries formatted money via `money_guard`
/// (`present_money` → `String`), NEVER a tweened value (money implements no
/// `FieldValue`, so a tweened money announcement does not compile).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeState {
    pub disabled: bool,
    pub selected: bool,
    pub busy: bool,
    /// e.g. `"DELIVERED"`, `"$12.40"`, `"3 items"` — already formatted text.
    pub value_text: Option<String>,
}

/// Live text-editing state. Produced by P57's cosmic-text editor; the a11y WIRE
/// SHAPE is owned HERE (§4.3) so the web mirror and the native adapter agree on
/// one representation. Byte offsets are grapheme-aligned by P57's buffer.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EditState {
    /// current buffer contents (Latin+Cyrillic Wave-0).
    pub text: String,
    /// byte offset of the cursor.
    pub caret: usize,
    /// selection anchor; `sel_anchor == caret` ⇒ no selection.
    pub sel_anchor: usize,
    /// ALWAYS false Wave-0 (no IME, §0.2-2); reserved for v2. A `true` value
    /// reaching the Wave-0 path is a typed refusal (§M3 adversarial).
    pub composing: bool,
}

/// A semantic node — authored beside the render geometry, keyed by `WidgetId`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SemanticNode {
    pub id: WidgetId,
    pub role: Role,
    /// accessible name (label / order id / item name).
    pub name: String,
    /// x,y,w,h in SCREEN space — projection-FLATTENED (§M6-3).
    pub bounds: [f32; 4],
    pub focusable: bool,
    /// keyboard/tab order; `0` = not in the tab ring.
    pub tab_index: u32,
    pub state: NodeState,
    /// `Some` ⇔ `Role::TextInput`; drives the ARIA-textbox (§M3).
    pub edit: Option<EditState>,
    /// tree structure (List→ListItem, Group→…).
    pub children: Vec<WidgetId>,
}

/// The semantic scene — the SAME state the renderer consumes for widgets. NO
/// theme token, NO view/projection matrix, NO render floor is a field here.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SemanticScene {
    pub nodes: Vec<SemanticNode>,
    pub root: WidgetId,
}

/// The accessible tree — the ONE artifact both platform paths consume.
/// Flattened in tab/DOM order; `mirror()` is the sole producer.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct A11yTree {
    pub nodes: Vec<A11yNode>,
}

/// A flattened accessible node (role/name/state/bounds/edit + tree position).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct A11yNode {
    pub id: WidgetId,
    pub role: Role,
    pub name: String,
    pub state: NodeState,
    pub bounds: [f32; 4],
    pub focusable: bool,
    pub tab_index: u32,
    pub edit: Option<EditState>,
    pub depth: u16,
    pub parent: Option<WidgetId>,
}

/// A typed refusal from `mirror()`. Each variant is a tested-unreachable bad
/// state of a `SemanticScene` (§M1 adversarial / §5.4 Self-Termination leg).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirrorError {
    /// A node's `children` references an id that is not present in the scene.
    MissingChild(WidgetId),
    /// A `children` cycle (A→B→A) — traversal must be bounded, never loop.
    Cycle(WidgetId),
    /// Duplicate `WidgetId` — the reconcile key must be unique or diffing is
    /// undefined.
    DupId(WidgetId),
    /// A `Role::TextInput` node with `edit: None`. A text field MUST carry
    /// `EditState` (§M3).
    TextInputWithoutEdit(WidgetId),
}

/// Reconcile event emitted when comparing two successive `A11yTree`s keyed by
/// `WidgetId` (§M1 — model the mirror as a diff event stream, matching the
/// kernel's `decide`/fold law; tests assert on the event sequence).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum A11yEvent {
    NodeAdded(WidgetId),
    NodeRemoved(WidgetId),
    NodeUpdated(WidgetId),
    CaretMoved(WidgetId, /* caret */ usize),
    SelectionChanged(WidgetId, /* anchor */ usize, /* focus */ usize),
}

impl SemanticScene {
    /// Builder helper: push a node, returning its id (auto-assigned).
    pub fn add_node(&mut self, mut node: SemanticNode) -> WidgetId {
        node.id = self
            .nodes
            .last()
            .map(|n| n.id.saturating_add(1))
            .unwrap_or(0);
        let id = node.id;
        self.nodes.push(node);
        id
    }
}

/// THE keystone (§M1/§M6). Pure, deterministic, presentation-free.
///
/// Flattens the child tree into tab order, copies role/name/state/bounds/edit,
/// and computes `depth`/`parent`. One `Vec` sized to `scene.nodes.len()` —
/// allocation-bounded, no hidden axis. Cycles/dup-ids/missing-children/
/// text-input-without-edit are REFUSED (typed `MirrorError`), never looped or
/// silently accepted.
pub fn mirror(scene: &SemanticScene) -> Result<A11yTree, MirrorError> {
    // 1) uniqueness check FIRST (the reconcile key must be unique; dup-id must be
    //    detected before any derived scan, since a dup-id makes every other scan
    //    ambiguous).
    let mut seen = std::collections::HashSet::new();
    for n in &scene.nodes {
        if !seen.insert(n.id) {
            return Err(MirrorError::DupId(n.id));
        }
    }
    // 2) role/child-shape checks.
    for n in &scene.nodes {
        if n.role == Role::TextInput && n.edit.is_none() {
            return Err(MirrorError::TextInputWithoutEdit(n.id));
        }
        for &c in &n.children {
            if !scene.nodes.iter().any(|m| m.id == c) {
                return Err(MirrorError::MissingChild(c));
            }
        }
    }

    let by_id: HashMap<WidgetId, &SemanticNode> = scene.nodes.iter().map(|n| (n.id, n)).collect();

    // 2) bounded tree walk from the root; a visited-set rejects cycles (§M1).
    let mut nodes: Vec<A11yNode> = Vec::with_capacity(scene.nodes.len());
    let mut visited: std::collections::HashSet<WidgetId> = std::collections::HashSet::new();
    walk(scene.root, None, 0, &by_id, &mut visited, &mut nodes)?;

    // 3) detached nodes (not reachable from root, but valid) are still emitted
    //    so no widget is silently dropped; appended in input order, depth 0.
    for n in &scene.nodes {
        if visited.insert(n.id) {
            nodes.push(flatten(n, 0, None));
        }
    }

    Ok(A11yTree { nodes })
}

fn walk(
    id: WidgetId,
    parent: Option<WidgetId>,
    depth: u16,
    by_id: &HashMap<WidgetId, &SemanticNode>,
    visited: &mut std::collections::HashSet<WidgetId>,
    out: &mut Vec<A11yNode>,
) -> Result<(), MirrorError> {
    let node = match by_id.get(&id) {
        Some(n) => n,
        None => return Ok(()), // root may be a sentinel absent from `nodes`
    };
    if !visited.insert(id) {
        return Err(MirrorError::Cycle(id));
    }
    out.push(flatten(node, depth, parent));
    for &c in &node.children {
        walk(c, Some(id), depth + 1, by_id, visited, out)?;
    }
    Ok(())
}

fn flatten(n: &SemanticNode, depth: u16, parent: Option<WidgetId>) -> A11yNode {
    A11yNode {
        id: n.id,
        role: n.role,
        name: n.name.clone(),
        state: n.state.clone(),
        bounds: n.bounds,
        focusable: n.focusable,
        tab_index: n.tab_index,
        edit: n.edit.clone(),
        depth,
        parent,
    }
}

/// Compute the keyed diff between two `A11yTree`s. The `WidgetId` is the join
/// key; the produced event stream lets both platform paths reconcile with an
/// O(changed) patch (§5.2), and tests assert on the sequence, not only the end
/// tree. `before` is the prior mirror, `after` the fresh one.
pub fn diff(before: &A11yTree, after: &A11yTree) -> Vec<A11yEvent> {
    let before_ids: std::collections::HashSet<WidgetId> =
        before.nodes.iter().map(|n| n.id).collect();
    let after_by_id: HashMap<WidgetId, &A11yNode> = after.nodes.iter().map(|n| (n.id, n)).collect();
    let before_by_id: HashMap<WidgetId, &A11yNode> =
        before.nodes.iter().map(|n| (n.id, n)).collect();

    let mut events = Vec::new();

    // Added (in `after`, not in `before`) and Updated.
    for n in &after.nodes {
        if !before_ids.contains(&n.id) {
            events.push(A11yEvent::NodeAdded(n.id));
            continue;
        }
        let prev = before_by_id[&n.id];
        if prev != n {
            events.push(A11yEvent::NodeUpdated(n.id));
        }
        // caret / selection changes are their own, distinct events (§M1).
        match (&prev.edit, &n.edit) {
            (Some(p), Some(c)) => {
                if p.caret != c.caret {
                    events.push(A11yEvent::CaretMoved(n.id, c.caret));
                }
                if p.sel_anchor != c.sel_anchor || p.caret != c.caret {
                    events.push(A11yEvent::SelectionChanged(n.id, c.sel_anchor, c.caret));
                }
            }
            _ => {}
        }
    }

    // Removed (in `before`, not in `after`).
    for n in &before.nodes {
        if !after_by_id.contains_key(&n.id) {
            events.push(A11yEvent::NodeRemoved(n.id));
        }
    }

    events
}

/// Stable, presentation-independent hash of an `A11yTree` (FNV-1a over its
/// serialized form). Used by the web mirror's interim "reconcile only on tree
/// change" trigger (§M2, pre-FE-14-settle) — the same "only on change" semantics
/// without depending on the unlanded `engine/src/settle.rs` gate.
pub fn tree_hash(tree: &A11yTree) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for n in &tree.nodes {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&(n.id as u64).to_le_bytes());
        h = fnv_mix(h, &buf);
        h = fnv_mix(h, n.name.as_bytes());
        h = fnv_mix(h, &[n.role as u8]);
        h = fnv_mix(h, &[n.tab_index as u8]);
        // state flags + value_text — a status text change MUST change the hash
        // (the web mirror reconciles on hash change, §M2).
        let flags =
            (n.state.disabled as u8) << 2 | (n.state.selected as u8) << 1 | (n.state.busy as u8);
        h = fnv_mix(h, &[flags]);
        if let Some(v) = &n.state.value_text {
            h = fnv_mix(h, v.as_bytes());
        }
        if let Some(e) = &n.edit {
            h = fnv_mix(h, e.text.as_bytes());
            h = fnv_mix(h, &(e.caret as u64).to_le_bytes());
            h = fnv_mix(h, &(e.sel_anchor as u64).to_le_bytes());
        }
    }
    h
}

fn fnv_mix(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// The synthetic ARIA-textbox convention (§M3) — the OWNED web contract. This
/// mapping is OFFLINE and shared by the wasm mirror export (the only place the
/// tree crosses the boundary under the TS/NODE BAN) and by the future
/// `web/src/lib/a11y_mirror.mjs` when it lands. P57 reconciles its editor to
/// these tokens (§4.3).
pub const MIRROR_ROOT_STYLE: &str =
    "position:fixed;clip-path:inset(50%);width:1px;height:1px;overflow:hidden;";
/// per-screen cap (§4.2 axis; a surface BP may TIGHTEN, never loosen without a note).
pub const MIRROR_NODE_BUDGET_DEFAULT: usize = 256;
/// change-frames only (fits P38 §6's 0.5 ms mirror slice).
pub const RECONCILE_BUDGET_MS: f64 = 0.5;

/// Machine-checkable STATE (AT announcement is SR-dependent; STATE is not) — the
/// harness asserts on THESE, so caret/selection is falsifiable cross-browser.
pub const ATTR_CARET: &str = "data-caret"; // = EditState.caret (byte)
pub const ATTR_SEL_ANCHOR: &str = "data-sel-anchor"; // = EditState.sel_anchor (byte)
pub const ATTR_TEXT: &str = "data-text"; // = EditState.text (round-trip pin)
/// Announcers (synthesize what a native editable element would emit).
pub const CARET_ANNOUNCER_LIVE: &str = "polite"; // emits grapheme/word crossed on caret move
pub const SELECT_ANNOUNCER_LIVE: &str = "assertive"; // emits "selected N: <text>" on selection change

/// Map a `Role` to its canonical ARIA role token (the web mirror's `role=…`).
/// Offline, no DOM — pure string mapping shared with the wasm export (§2).
pub fn role_to_aria_token(role: Role) -> &'static str {
    match role {
        Role::Group => "group",
        Role::Heading => "heading",
        Role::Label => "label",
        Role::Button => "button",
        Role::Link => "link",
        Role::Image => "img",
        Role::List => "list",
        Role::ListItem => "listitem",
        Role::Status => "status",
        Role::Alert => "alert",
        Role::TextInput => "textbox",
    }
}

#[cfg(test)]
mod offline_contract_tests {
    use super::*;

    // M3 — the ARIA-textbox token is the canonical synthetic-textbox convention
    //      P57 consumes; Status/Alert map to live regions.
    #[test]
    fn aria_tokens_are_canonical() {
        assert_eq!(role_to_aria_token(Role::TextInput), "textbox");
        assert_eq!(role_to_aria_token(Role::Status), "status");
        assert_eq!(role_to_aria_token(Role::Alert), "alert");
        assert_eq!(role_to_aria_token(Role::Button), "button");
    }

    // M2 — the mirror-root style is the SR-only clip technique (zero painted px).
    #[test]
    fn mirror_root_style_is_sr_only() {
        assert!(MIRROR_ROOT_STYLE.contains("clip-path:inset(50%)"));
        assert!(MIRROR_ROOT_STYLE.contains("width:1px"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> SemanticScene {
        SemanticScene {
            root: 0,
            nodes: vec![
                SemanticNode {
                    id: 0,
                    role: Role::Group,
                    name: "root".into(),
                    bounds: [0.0, 0.0, 100.0, 100.0],
                    focusable: false,
                    tab_index: 0,
                    state: NodeState::default(),
                    edit: None,
                    children: vec![1, 2],
                },
                SemanticNode {
                    id: 1,
                    role: Role::Button,
                    name: "Checkout".into(),
                    bounds: [0.0, 0.0, 50.0, 20.0],
                    focusable: true,
                    tab_index: 1,
                    state: NodeState::default(),
                    edit: None,
                    children: vec![],
                },
                SemanticNode {
                    id: 2,
                    role: Role::Status,
                    name: "Order #42".into(),
                    bounds: [0.0, 20.0, 50.0, 20.0],
                    focusable: false,
                    tab_index: 0,
                    state: NodeState {
                        value_text: Some("DELIVERED".into()),
                        ..Default::default()
                    },
                    edit: None,
                    children: vec![],
                },
            ],
        }
    }

    // M1 — `mirror` is pure: same `SemanticScene` twice ⇒ byte-identical tree.
    #[test]
    fn mirror_is_pure_and_deterministic() {
        let s = scene();
        let a = mirror(&s).unwrap();
        let b = mirror(&s).unwrap();
        assert_eq!(a, b, "mirror must be deterministic");
    }

    // M1 — mutating one node's name emits exactly one `NodeUpdated` and nothing
    //      else (the diff is a keyed event stream, not just an end-state).
    #[test]
    fn diff_emits_keyed_events() {
        let s1 = scene();
        let t1 = mirror(&s1).unwrap();
        let mut s2 = s1.clone();
        s2.nodes[1].name = "Pay now".into();
        let t2 = mirror(&s2).unwrap();
        let events = diff(&t1, &t2);
        assert_eq!(
            events,
            vec![A11yEvent::NodeUpdated(1)],
            "exactly one NodeUpdated for the changed node"
        );
    }

    // M1 adversarial — a `children` cycle is refused, never loops.
    #[test]
    fn cycle_is_refused() {
        let mut s = scene();
        // 0 -> 1 -> 2, and 2 -> 1 forms a cycle (1 <-> 2), not reachable as a DAG.
        s.nodes[2].children.push(1);
        assert_eq!(mirror(&s), Err(MirrorError::Cycle(1)));
    }

    // M1 adversarial — duplicate `WidgetId` is refused (reconcile key must be unique).
    #[test]
    fn dup_id_is_refused() {
        let mut s = scene();
        s.nodes[2].id = 1; // collide with node 1
        assert_eq!(mirror(&s), Err(MirrorError::DupId(1)));
    }

    // M1 adversarial — a `TextInput` with `edit: None` is refused.
    #[test]
    fn text_input_requires_edit() {
        let mut s = scene();
        s.nodes.push(SemanticNode {
            id: 3,
            role: Role::TextInput,
            name: "Name".into(),
            bounds: [0.0, 0.0, 40.0, 12.0],
            focusable: true,
            tab_index: 2,
            state: NodeState::default(),
            edit: None, // missing!
            children: vec![],
        });
        assert_eq!(mirror(&s), Err(MirrorError::TextInputWithoutEdit(3)));
    }

    // M1 — a `TextInput` WITH `edit` mirrors fine and carries the edit state.
    #[test]
    fn text_input_with_edit_mirrors() {
        let mut s = scene();
        s.nodes.push(SemanticNode {
            id: 3,
            role: Role::TextInput,
            name: "Name".into(),
            bounds: [0.0, 0.0, 40.0, 12.0],
            focusable: true,
            tab_index: 2,
            state: NodeState::default(),
            edit: Some(EditState {
                text: "hi".into(),
                caret: 2,
                sel_anchor: 0,
                composing: false,
            }),
            children: vec![],
        });
        let t = mirror(&s).unwrap();
        let n = t.nodes.iter().find(|n| n.id == 3).unwrap();
        assert_eq!(n.role, Role::TextInput);
        assert_eq!(n.edit.as_ref().unwrap().text, "hi");
    }

    // M1 — a caret move on a text field emits `CaretMoved` + `SelectionChanged`.
    #[test]
    fn caret_move_emits_events() {
        let mut s = scene();
        s.nodes.push(SemanticNode {
            id: 3,
            role: Role::TextInput,
            name: "Name".into(),
            bounds: [0.0, 0.0, 40.0, 12.0],
            focusable: true,
            tab_index: 2,
            state: NodeState::default(),
            edit: Some(EditState {
                text: "hi".into(),
                caret: 2,
                sel_anchor: 2,
                composing: false,
            }),
            children: vec![],
        });
        let t1 = mirror(&s).unwrap();
        s.nodes[3].edit.as_mut().unwrap().caret = 1;
        let t2 = mirror(&s).unwrap();
        let events = diff(&t1, &t2);
        assert!(events.contains(&A11yEvent::CaretMoved(3, 1)));
        assert!(events.contains(&A11yEvent::SelectionChanged(3, 2, 1)));
    }

    // M6 — the draft-parity invariant. `mirror` takes ONLY `&SemanticScene`;
    // brand tokens / view-projection / render floor are NOT inputs (they are
    // external values here, mirroring the real engine split). All trees must be
    // byte-identical. If a presentation arg were ever added to `mirror`, this
    // file STOPS COMPILING — that is the guard (§M6 / §2 not-done clause).
    #[test]
    fn a11y_tree_invariant_under_presentation_swap() {
        let s = scene();
        // "brand token set" — presented as a token the renderer would consume but
        // `mirror` cannot see (it is not a parameter). We vary it as a free value.
        let _brand_a = [0u8; 20];
        let _brand_b = [255u8; 20];
        // "view/projection matrix" — likewise external to `mirror`.
        let _m_ortho = [[1.0f32; 4]; 4];
        let _m_persp = [[0.5f32; 4]; 4];
        // "render floor" — WebGPU / WebGL2 / CPU.
        let _floor_a = "webgpu";
        let _floor_b = "cpu";

        let t_brand_a = mirror(&s).unwrap();
        let t_brand_b = mirror(&s).unwrap();
        let t_floor = mirror(&s).unwrap();
        let _ = (_brand_a, _brand_b, _m_ortho, _m_persp, _floor_a, _floor_b);
        assert_eq!(t_brand_a, t_brand_b);
        assert_eq!(t_brand_a, t_floor, "render floor must not alter the tree");
    }

    // M2 interim trigger — `tree_hash` changes only when the tree changes, so
    // the web mirror reconciles on hash change (pre-FE-14-settle).
    #[test]
    fn tree_hash_changes_only_on_change() {
        let s = scene();
        let t = mirror(&s).unwrap();
        let h1 = tree_hash(&t);
        let h2 = tree_hash(&t);
        assert_eq!(h1, h2, "same tree ⇒ same hash");

        let mut s2 = s.clone();
        s2.nodes[2].state.value_text = Some("SHIPPED".into());
        let h3 = tree_hash(&mirror(&s2).unwrap());
        assert_ne!(h1, h3, "state change must change the hash");
    }

    // M1/M2 — `tree_hash` is presentation-independent: byte-identical trees hash
    //         identically (the "reconcile only on change" trigger is sound).
    #[test]
    fn tree_hash_is_stable_for_identical_trees() {
        let t = mirror(&scene()).unwrap();
        assert_eq!(tree_hash(&t), tree_hash(&t));
    }
}
