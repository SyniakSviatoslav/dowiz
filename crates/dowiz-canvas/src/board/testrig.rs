//! A board driven the way the host drives it (tests only): feed -> draw -> tap -> intent.
use super::{Board, Role};
use crate::cmd::Cmd;
use crate::scene::Scene;
use crate::tests::Mono;
use crate::text::Widths;
use crate::ui::{Ui, DARK, LIGHT};

pub const NOW: i64 = 1_800_000_000_000;

pub fn feed_text() -> String {
    let t = |id: &str, st: &str, ago_min: i64, seen: u8, kind: &str, table: &str| {
        format!("T\t{id}\t{st}\t{}\t{seen}\t{kind}\t{table}\t\t\n", NOW - ago_min * 60_000)
    };
    let mut f = String::new();
    f += &t("ord_aaaa1111", "PENDING", 2, 0, "t", "4");
    f += "L\t2\t1\tSalmon nigiri\t\nL\t1\t2\tMiso soup\tno onion\n";
    f += &t("ord_bbbb2222", "PREPARING", 12, 1, "p", "");
    f += "L\t1\t3\tLemonade\t\n";
    f += &t("ord_cccc3333", "READY", 25, 1, "d", "");
    f += "L\t3\t2\tRamen\t\n";
    f += &t("ord_dddd4444", "READY", 1, 1, "t", "7");
    f += "L\t1\t2\tGyoza\t\n";
    f += "R\tsit_1\t4\t2\t1 500 L\t1\tPENDING,READY\n";
    f += "R\tsit_2\t9\t1\t800 L\t0\tPREPARING\n";
    f
}

pub struct Rig {
    pub b: Box<Board>,
    pub cmd: Box<Cmd>,
    pub scene: Box<Scene>,
    widths: Box<Widths>,
    m: Mono,
}

impl Rig {
    pub fn new(w: i32, h: i32) -> Rig {
        let mut b = Box::new(Board::new());
        b.w = w;
        b.h = h;
        b.now_ms = NOW;
        Rig { b, cmd: Box::new(Cmd::new()), scene: Box::new(Scene::new()), widths: Box::new(Widths::new()), m: Mono { calls: 0 } }
    }
    pub fn signed(mut self, pass: bool, floor: bool) -> Rig {
        self.b.session(true, pass, floor, Role::CounterManager);
        self.b.feed(feed_text().as_bytes());
        self
    }
    pub fn frame(&mut self) -> u32 {
        let pal = if self.b.dark { DARK } else { LIGHT };
        let mut ui = Ui { cmd: &mut self.cmd, scene: &mut self.scene, widths: &mut self.widths, host: &mut self.m, pal, lang: self.b.lang };
        self.b.draw(&mut ui);
        self.cmd.hash()
    }
    /// `frame` the way ffi.rs `frame` builds it: through the dirty flag (dirty.rs).
    pub fn frame_gated(&mut self, d: &mut super::dirty::Dirty) -> Option<u32> {
        let pal = if self.b.dark { DARK } else { LIGHT };
        let mut ui = Ui { cmd: &mut self.cmd, scene: &mut self.scene, widths: &mut self.widths, host: &mut self.m, pal, lang: self.b.lang };
        super::dirty::frame(d, &mut self.b, &mut ui)
    }
    pub fn tap_tour(&mut self, tour: &str) {
        let r = self.scene.find_tour(tour).unwrap_or_else(|| panic!("no node {tour}")).rect;
        self.tap(r.x + r.w / 2, r.y + r.h / 2);
    }
    pub fn tap(&mut self, x: i32, y: i32) {
        self.b.pointer(&self.scene, 0, x, y);
        self.b.pointer(&self.scene, 2, x, y);
        self.frame();
    }
}

