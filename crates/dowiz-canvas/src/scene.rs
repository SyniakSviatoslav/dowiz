//! THE SCENE TREE: everything a person can tap, as data.
//!
//! A DOM page answers "where is the Accept button" with `querySelector('[data-tour=...]')`; a
//! canvas has no elements, so the scene answers it instead. Every frame rebuilds the scene from
//! state (nodes are cheap: an id, a rectangle, an action), and the same list then serves
//!   * the hit-test (`hit`): the LAST node pushed at a point wins, as the topmost element does;
//!   * the learn tour (`find_tour`): a lesson step's anchor (`kitchen.bump`, `hud.lang`, ...) is
//!     a node's `tour`, so the tour engine can ring a rectangle it was given by Rust;
//!   * the tap-size check (`too_small`): every actionable node at least 44 x 44 CSS px.
//! Containers (a list area, a card) are nodes too, with `Act::None`; they nest by `parent`.

use crate::geom::Rect;

/// What a tap on a node asks for. The board turns it into an `Intent` for the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// A container or a label: never the answer to a tap.
    None,
    /// Switch the board's view tab (index into the board's tab list).
    Tab(u8),
    /// Filter tickets by station (0 = all).
    Station(u8),
    /// The ticket's one big button: move it to its next status.
    Bump(u16),
    /// The ticket's head: "the kitchen has seen it".
    Seen(u16),
    /// Open the reject/cancel sheet for a ticket.
    Stop(u16),
    /// The sheet's scrim and its Close button.
    SheetClose,
    /// The sheet's Reject/Cancel button.
    SheetSend,
    /// Swallows a tap (a sheet's panel, so a tap on it does not reach the scrim below).
    Block,
    /// An open table: hand over to the full room page.
    Table(u16),
    /// Focus a text field.
    Field(u8),
    Submit,
    ClaimToggle,
    Lang,
    Refresh,
    SignOut,
}

impl Act {
    pub const fn actionable(&self) -> bool {
        !matches!(self, Act::None)
    }
    /// Does the tap-size rule apply? (A panel that only swallows taps is not a control.)
    pub const fn control(&self) -> bool {
        !matches!(self, Act::None | Act::Block)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    /// Stable within a frame: the push order.
    pub id: u16,
    /// The parent container's id, or `u16::MAX` for the root.
    pub parent: u16,
    /// The learn tour's anchor word, or "" when no lesson points here.
    pub tour: &'static str,
    pub rect: Rect,
    pub act: Act,
}

pub const NODES: usize = 512;
pub const ROOT: u16 = u16::MAX;

pub struct Scene {
    nodes: [Node; NODES],
    len: usize,
    /// Nodes that did not fit (counted, never silent).
    pub dropped: u32,
}

const BLANK: Node = Node { id: 0, parent: ROOT, tour: "", rect: Rect::ZERO, act: Act::None };

impl Scene {
    pub const fn new() -> Scene {
        Scene { nodes: [BLANK; NODES], len: 0, dropped: 0 }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.dropped = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes[..self.len]
    }

    /// Add a node; its id, or `ROOT` when the pool is full.
    pub fn push(&mut self, parent: u16, tour: &'static str, rect: Rect, act: Act) -> u16 {
        if self.len == NODES {
            self.dropped += 1;
            return ROOT;
        }
        let id = self.len as u16;
        self.nodes[self.len] = Node { id, parent, tour, rect, act };
        self.len += 1;
        id
    }

    /// Is the point inside every ancestor of `n`? A card scrolled under the header is clipped
    /// by its list, and a tap there belongs to the header, not to the hidden card.
    fn visible_at(&self, n: &Node, x: i32, y: i32) -> bool {
        let mut p = n.parent;
        let mut hops = 0;
        while p != ROOT && (p as usize) < self.len && hops < 32 {
            let a = &self.nodes[p as usize];
            if !a.rect.contains(x, y) {
                return false;
            }
            p = a.parent;
            hops += 1;
        }
        true
    }

    /// The topmost ACTIONABLE node at a point.
    pub fn hit(&self, x: i32, y: i32) -> Option<&Node> {
        self.nodes().iter().rev().find(|n| n.act.actionable() && n.rect.contains(x, y) && self.visible_at(n, x, y))
    }

    /// The first node carrying a tour anchor.
    pub fn find_tour(&self, tour: &str) -> Option<&Node> {
        self.nodes().iter().find(|n| !n.tour.is_empty() && n.tour == tour)
    }

    /// Actionable nodes smaller than `min` CSS px on either side (WCAG 2.5.5 / tap-size gate).
    pub fn too_small(&self, min: i32) -> usize {
        self.nodes().iter().filter(|n| n.act.control() && (n.rect.w < min || n.rect.h < min)).count()
    }
}
