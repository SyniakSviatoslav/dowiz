//! THE ONE RECURSION through a tree of cards -- the importer's flattening
//! (`import/recipes/flatten.rs`) and the catalogue's semi-finished products
//! (`super`) both walk with this, so there is one cycle check, one depth cap
//! and one arithmetic (`Rat`, exact, checked).
//!
//! A NODE is anything `card(id)` answers a [`Card`] for: its lines and the
//! batch those lines make. A line naming an id with no card is a LEAF. Every
//! leaf's quantity is summed over every path that reaches it, exactly, and
//! handed back once -- the caller rounds once, or refuses.

use crate::import::recipes::num::Rat;

/// One line of a node: an item and its gross quantity, exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub item: String,
    pub qty: Rat,
}

/// What a node is made of, and how much of it that makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub lines: Vec<Edge>,
    /// The batch's net output, in the node's own base unit. Positive.
    pub batch: Rat,
}

/// One leaf, summed over every path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    pub item: String,
    pub qty: Rat,
    /// How many lines reached it (1: one direct line, whose own net/yield
    /// still describe it -- the importer keeps those only then).
    pub uses: u32,
    /// Reached by a line of the ROOT itself (not through a card).
    pub direct: bool,
}

/// Why a walk stopped. Every variant names where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// A card names itself, directly or through others: the path, root first.
    Cycle(Vec<String>),
    /// More card levels than [`Walk::depth_max`] allows: the path.
    TooDeep(Vec<String>),
    /// A card's batch is not positive.
    NoBatch(String),
    /// An i128 would have wrapped.
    Overflow,
    /// `card()` refused an id with its own words (the importer's messages).
    Card(String),
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stop::Cycle(p) => write!(f, "semi-finished products name each other: {}", p.join(" → ")),
            Stop::TooDeep(p) => write!(f, "more than {} levels of semi-finished products: {}", p.len() - 1, p.join(" → ")),
            Stop::NoBatch(id) => write!(f, "{id:?} has no batch size"),
            Stop::Overflow => write!(f, "the fractions grew past what can be computed exactly"),
            Stop::Card(why) => f.write_str(why),
        }
    }
}

/// What a walk answers: the leaves, and every card it went through with how
/// many lines it had (the importer's "expanded into N line(s)" report).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Walked {
    pub leaves: Vec<Leaf>,
    pub through: Vec<(String, usize)>,
    /// The longest chain of cards under the root, in cards.
    pub depth: usize,
}

/// The walk's bounds.
pub struct Walk {
    /// Cards allowed on one path under the root.
    pub depth_max: usize,
}

impl Walk {
    /// Expand `root` (its lines, each times `factor`) down to leaves.
    /// `card(id)` answers `Ok(None)` for a leaf, `Ok(Some)` for a node, `Err`
    /// to refuse the id in its own words.
    pub fn run(
        &self,
        root: &str,
        lines: &[Edge],
        card: &mut dyn FnMut(&str) -> Result<Option<Card>, String>,
    ) -> Result<Walked, Stop> {
        let mut out = Walked::default();
        let mut stack = vec![root.to_string()];
        self.expand(lines, Rat::new(1, 1), card, &mut stack, &mut out, true)?;
        Ok(out)
    }

    fn expand(
        &self,
        lines: &[Edge],
        factor: Rat,
        card: &mut dyn FnMut(&str) -> Result<Option<Card>, String>,
        stack: &mut Vec<String>,
        out: &mut Walked,
        direct: bool,
    ) -> Result<(), Stop> {
        for line in lines {
            let q = line.qty.checked_mul(factor).ok_or(Stop::Overflow)?;
            if stack.contains(&line.item) {
                let mut path = stack.clone();
                path.push(line.item.clone());
                return Err(Stop::Cycle(path));
            }
            match card(&line.item).map_err(Stop::Card)? {
                None => add_leaf(out, &line.item, q, direct)?,
                Some(c) => {
                    if c.batch.num <= 0 || c.batch.den <= 0 {
                        return Err(Stop::NoBatch(line.item.clone()));
                    }
                    stack.push(line.item.clone());
                    if stack.len() - 1 > self.depth_max {
                        return Err(Stop::TooDeep(stack.clone()));
                    }
                    out.depth = out.depth.max(stack.len() - 1);
                    let f = q.checked_div(c.batch).ok_or(Stop::Overflow)?;
                    self.expand(&c.lines, f, card, stack, out, false)?;
                    stack.pop();
                    out.through.push((line.item.clone(), c.lines.len()));
                }
            }
        }
        Ok(())
    }
}

fn add_leaf(out: &mut Walked, item: &str, q: Rat, direct: bool) -> Result<(), Stop> {
    match out.leaves.iter_mut().find(|l| l.item == item) {
        Some(l) => {
            l.qty = l.qty.checked_add(q).ok_or(Stop::Overflow)?;
            l.uses += 1;
            l.direct |= direct;
        }
        None => out.leaves.push(Leaf { item: item.to_string(), qty: q, uses: 1, direct }),
    }
    Ok(())
}
