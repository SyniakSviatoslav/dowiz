//! The engine core: geometry, layout, text fitting, the command buffer's hash, the scene.
use crate::cmd::Cmd;
use crate::geom::Rect;
use crate::lang::{Lang, Str};
use crate::layout;
use crate::scene::{Act, Scene, ROOT};
use crate::text::{fit, Measure, Widths};

/// A fixed-advance font: each character is `px * 11 / 20` wide (an average Latin advance).
pub struct Mono {
    pub calls: u32,
}
impl Measure for Mono {
    fn measure(&mut self, s: &str, px: i32, _w: i32) -> i32 {
        self.calls += 1;
        (s.chars().count() as i32 * px * 11 + 19) / 20
    }
}

#[test]
fn columns_sum_exactly_for_every_width() {
    let mut out = [Rect::ZERO; 4];
    for w in 1..2000 {
        for n in 1..=4 {
            layout::columns(Rect::new(7, 0, w, 10), n, 6, &mut out);
            let used: i32 = out[..n].iter().map(|r| r.w).sum::<i32>() + 6 * (n as i32 - 1);
            if w >= 6 * (n as i32 - 1) {
                assert_eq!(used, w, "w={w} n={n}");
                assert_eq!(out[n - 1].right(), 7 + w, "last cell ends at the edge");
            }
        }
    }
}

#[test]
fn flow_wraps_and_stack_sums() {
    let mut out = [Rect::ZERO; 4];
    let h = layout::flow(Rect::new(0, 0, 100, 0), &[60, 30, 50, 10], 20, 4, &mut out);
    assert_eq!((out[0].y, out[1].y, out[2].y, out[3].y), (0, 0, 24, 24));
    assert_eq!(h, 44);
    let h = layout::stack(Rect::new(0, 10, 50, 0), &[5, 6, 7], 2, &mut out);
    assert_eq!((h, out[2].y), (5 + 6 + 7 + 4, 10 + 5 + 2 + 6 + 2));
}

#[test]
fn rect_contains_is_half_open() {
    let r = Rect::new(10, 10, 5, 5);
    assert!(r.contains(10, 10) && r.contains(14, 14));
    assert!(!r.contains(15, 10) && !r.contains(10, 15));
    assert_eq!(r.inset(3), Rect::new(13, 13, 0, 0));
}

#[test]
fn widths_are_measured_once() {
    let mut w = Widths::new();
    let mut m = Mono { calls: 0 };
    let a = w.get(&mut m, "Gati", 16, 700);
    let b = w.get(&mut m, "Gati", 16, 700);
    assert_eq!((a, b, m.calls, w.misses), (36, 36, 1, 1));
    w.get(&mut m, "Gati", 17, 700);
    assert_eq!(m.calls, 2, "another size is another width");
}

#[test]
fn fit_cuts_on_a_character_boundary_with_room_for_the_ellipsis() {
    let mut w = Widths::new();
    let mut m = Mono { calls: 0 };
    let (s, cut) = fit(&mut w, &mut m, "Готується", 20, 400, 1000);
    assert_eq!((s, cut), ("Готується", false));
    let (s, cut) = fit(&mut w, &mut m, "Готується", 20, 400, 60);
    assert!(cut);
    assert!(w.get(&mut m, s, 20, 400) + w.get(&mut m, "…", 20, 400) <= 60);
    assert_eq!(s, "Готу", "11 px a char at 20 px: four chars + the ellipsis = 55 <= 60, five = 66");
}

#[test]
fn hash_is_over_content_not_pointers() {
    let mut a = Box::new(Cmd::new());
    let mut b = Box::new(Cmd::new());
    let one = String::from("Prano");
    let two = String::from("Prano");
    a.text(&one, 1, 2, 16, 700, 0xff);
    b.text(&two, 1, 2, 16, 700, 0xff);
    assert_ne!(one.as_ptr(), two.as_ptr());
    assert_eq!(a.hash(), b.hash());
    b.clear();
    b.text("Prano", 1, 3, 16, 700, 0xff);
    assert_ne!(a.hash(), b.hash(), "a moved run is a different frame");
}

#[test]
fn a_full_buffer_counts_what_it_lost() {
    let mut c = Box::new(Cmd::new());
    for _ in 0..crate::cmd::CAP {
        c.rect(0, 0, 1, 1, 0, 0, 0);
    }
    assert!(c.overflow > 0);
    assert!(c.len <= crate::cmd::CAP);
}

#[test]
fn hit_takes_the_topmost_actionable_node_inside_its_parents() {
    let mut s = Scene::new();
    let list = s.push(ROOT, "", Rect::new(0, 100, 100, 100), Act::None);
    s.push(list, "kitchen.bump", Rect::new(0, 50, 100, 80), Act::Bump(3));
    s.push(ROOT, "hud.lang", Rect::new(0, 0, 100, 100), Act::Lang);
    assert_eq!(s.hit(10, 60).map(|n| n.act), Some(Act::Lang), "the card under the header is clipped by its list");
    assert_eq!(s.hit(10, 110).map(|n| n.act), Some(Act::Bump(3)));
    assert_eq!(s.hit(10, 250), None);
    assert_eq!(s.find_tour("kitchen.bump").map(|n| n.rect.h), Some(80));
    assert_eq!(s.too_small(44), 0);
    s.push(ROOT, "", Rect::new(0, 0, 43, 60), Act::Refresh);
    assert_eq!(s.too_small(44), 1);
}

#[test]
fn every_word_exists_in_all_four_languages() {
    for k in Str::ALL {
        for l in Lang::ALL {
            assert!(!l.s(k).trim().is_empty(), "{k:?} is blank in {}", l.code());
        }
    }
    let codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
    assert_eq!(codes, ["sq", "en", "uk", "ru"], "lib/langs.js LANGS order");
    let mut l = Lang::Sq;
    for _ in 0..4 {
        l = l.next();
    }
    assert_eq!(l, Lang::Sq, "nextLang cycles through all four");
    assert_eq!(Lang::from_code(b"uk"), Lang::Uk);
    assert_eq!(Lang::from_code(b"xx"), Lang::Sq, "Albanian is the default, as the room's");
}

#[test]
fn itoa_writes_digits() {
    let mut b = [0u8; 20];
    assert_eq!(crate::itoa(0, &mut b), "0");
    assert_eq!(crate::itoa(1500, &mut b), "1500");
    assert_eq!(crate::itoa(u64::MAX, &mut b), "18446744073709551615");
}
