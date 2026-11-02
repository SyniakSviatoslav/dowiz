//! The gluing contract: states, the default refusal, named resolvers, and the span rule the
//! tree merge instance relies on.

use super::*;

fn k(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[test]
fn the_four_states_without_an_obstruction() {
    let none: [Vec<u8>; 0] = [];
    assert_eq!(glue(&none, &none, &mut Refuse).unwrap().state, State::Same);
    assert_eq!(glue(&[k("a")], &none, &mut Refuse).unwrap().state, State::FastForward(Side::Left));
    assert_eq!(glue(&none, &[k("a")], &mut Refuse).unwrap().state, State::FastForward(Side::Right));
    assert_eq!(glue(&[k("a"), k("b")], &[k("c")], &mut Refuse).unwrap().state, State::Commutes);
}

/// OCC's validation IS `agree`: a shared key is the obstruction, named by its pair, and the
/// default resolver refuses it. The positive twin: disjoint keys agree.
#[test]
fn a_shared_key_is_refused_by_name_and_disjoint_keys_agree() {
    let writes = [k("order:7"), k("stock:salmon")];
    let reads = [k("menu:1"), k("stock:salmon")];
    assert_eq!(glue(&writes, &reads, &mut Refuse).unwrap_err(), Obstruction { left: 1, right: 1 });
    assert!(!agree(&writes, &reads));
    assert!(agree(&writes, &[k("menu:1"), k("stock:tuna")]));
    assert!(agree::<Vec<u8>>(&[], &reads), "nothing written since the base: always agrees");
}

/// A named resolver answers every obstruction: the state is `Resolved`, the pieces and the kept
/// sides are listed by pair, and a resolver may refuse the SECOND obstruction on one piece.
#[test]
fn a_named_resolver_keeps_or_glues_and_may_refuse_later() {
    struct KeepRightOnce(bool);
    impl Resolver<u64> for KeepRightOnce {
        type Out = u64;
        fn resolve(&mut self, l: &u64, r: &u64) -> Resolve<u64> {
            if self.0 {
                return Resolve::Refuse;
            }
            self.0 = true;
            if *l == 3 { Resolve::Glued(l + r) } else { Resolve::Keep(Side::Right) }
        }
    }
    let g = glue(&[1u64, 3], &[3u64, 9], &mut KeepRightOnce(false)).unwrap();
    assert_eq!(g.state, State::Resolved);
    assert_eq!(g.glued, vec![(1, 0, 6)]);
    assert!(g.kept.is_empty());
    let g = glue(&[1u64, 3], &[1u64, 9], &mut KeepRightOnce(false)).unwrap();
    assert_eq!(g.kept, vec![(0, 0, Side::Right)]);
    assert_eq!(glue(&[1u64, 3], &[1u64, 3], &mut KeepRightOnce(false)).unwrap_err(), Obstruction { left: 1, right: 1 });
}

/// The tree instance's rule: spans disagree when they intersect, an insertion touches both of
/// its neighbours, and `single` is the shape `Descend` can take one level down.
#[test]
fn spans_disagree_when_they_intersect_and_an_insertion_touches_its_neighbours() {
    let s = |lo, hi| Span { lo, hi };
    assert!(!s(0, 1).disagrees(&s(1, 2)), "adjacent slots commute");
    assert!(s(0, 2).disagrees(&s(1, 2)));
    assert!(s(3, 3).disagrees(&s(2, 3)), "an insertion at 3 touches slot 2");
    assert!(s(3, 3).disagrees(&s(3, 4)), "and slot 3");
    assert!(!s(3, 3).disagrees(&s(1, 2)));
    assert!(s(0, 0).disagrees(&s(0, 1)), "an insertion at the front touches slot 0 only");
    assert!(!s(0, 0).disagrees(&s(1, 2)));
    assert!(s(4, 5).single() && !s(4, 6).single() && !s(4, 4).single());
    let o = glue(&[s(0, 1), s(5, 6)], &[s(5, 6)], &mut Refuse).unwrap_err();
    assert_eq!(o, Obstruction { left: 1, right: 0 }, "the one shared slot, by its pair");
    assert_eq!(glue(&[s(0, 1), s(5, 6)], &[s(2, 3)], &mut Refuse).unwrap().state, State::Commutes);
}
