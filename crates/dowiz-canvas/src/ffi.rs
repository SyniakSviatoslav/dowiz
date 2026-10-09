//! THE HOST ABI (wasm32 only): one import, a handful of exports, no wasm-bindgen.
//!
//! The one import is `env.txt_measure(ptr, len, px, weight) -> f32` (Canvas2D measureText). Text
//! the host sends in (the feed, a field's value, a toast) is written into `inbuf()` first. The
//! frame is `frame(now_ms)`; the loader then replays `cmd_ptr()[..cmd_len()]` (see cmd.rs).
//! A tap's consequence is read back with `intent()` until it answers kind 0.
#![allow(static_mut_refs)]

use core::mem::MaybeUninit;

use crate::board::{Board, Intent, Role, Sync};
use crate::cmd::Cmd;
use crate::lang::Lang;
use crate::scene::Scene;
use crate::text::{Measure, Widths};
use crate::ui::{Ui, DARK, LIGHT};

#[link(wasm_import_module = "env")]
extern "C" {
    fn txt_measure(ptr: *const u8, len: usize, px: f32, weight: u32) -> f32;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

struct Host;
impl Measure for Host {
    fn measure(&mut self, s: &str, px: i32, weight: i32) -> i32 {
        // Ceil: a run is never laid out narrower than it draws.
        let w = unsafe { txt_measure(s.as_ptr(), s.len(), px as f32, weight as u32) };
        let i = w as i32;
        if (i as f32) < w { i + 1 } else { i }
    }
}

const IN: usize = 65_536;
static mut INBUF: [u8; IN] = [0; IN];
// BOARD and SCENE hold non-zero bytes (string pointers, enum tags), so a `static` initialised
// with them would be stored IN THE MODULE: measured 278 KB raw before this. Left uninitialised
// they are zero pages the runtime allocates, and the first export to run builds them once.
static mut BOARD_M: MaybeUninit<Board> = MaybeUninit::uninit();
static mut SCENE_M: MaybeUninit<Scene> = MaybeUninit::uninit();
static mut READY: bool = false;
static mut CMD: Cmd = Cmd::new();
static mut WIDTHS: Widths = Widths::new();
static mut INTENT: [i32; 8] = [0; 8];
static mut STATS: [u32; 12] = [0; 12];

unsafe fn ready() {
    if !READY {
        BOARD_M.write(Board::new());
        SCENE_M.write(Scene::new());
        READY = true;
    }
}
unsafe fn bd() -> &'static mut Board {
    ready();
    BOARD_M.assume_init_mut()
}
unsafe fn sc() -> &'static mut Scene {
    ready();
    SCENE_M.assume_init_mut()
}

unsafe fn input(len: u32) -> &'static [u8] {
    crate::head(&INBUF, len as usize)
}

#[no_mangle]
pub unsafe extern "C" fn inbuf() -> *mut u8 {
    INBUF.as_mut_ptr()
}
#[no_mangle]
pub extern "C" fn inbuf_cap() -> u32 {
    IN as u32
}

/// Size in whole CSS px, the colour scheme, and the language (0 sq, 1 en, 2 uk, 3 ru).
#[no_mangle]
pub unsafe extern "C" fn init(w: i32, h: i32, dark: u32, lang: u32) {
    bd().w = w;
    bd().h = h;
    bd().dark = dark != 0;
    bd().lang = Lang::ALL[(lang & 3) as usize];
}
/// The language to start in: `inbuf()[..len]` is the stored choice, then the browser's
/// languages, one per line (lib/langs.js `pickLang`: a stored code exactly, else the first
/// browser tag whose first two letters name one; else sq). Its index (0 sq, 1 en, 2 uk, 3 ru).
#[no_mangle]
pub unsafe extern "C" fn lang_pick(len: u32) -> u32 {
    Lang::pick(input(len)).index()
}
/// The two ASCII letters of language `i`'s code.
#[no_mangle]
pub extern "C" fn lang_code(i: u32) -> *const u8 {
    Lang::ALL[(i & 3) as usize].code().as_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn resize(w: i32, h: i32) {
    bd().w = w;
    bd().h = h;
}
/// The text metrics changed (a devicePixelRatio or font change): measure again.
#[no_mangle]
pub unsafe extern "C" fn forget_widths() {
    WIDTHS.reset();
}
#[no_mangle]
pub unsafe extern "C" fn set_dark(d: u32) {
    bd().dark = d != 0;
}
/// role: 0 waiter, 1 counter-manager, 2 kitchen, 3 owner. `signed` 2: a stored session is being
/// restored (Board::restore) -- the first frame says "Loading", not the login form.
#[no_mangle]
pub unsafe extern "C" fn session(signed: u32, pass: u32, floor: u32, role: u32) {
    if signed == 2 {
        bd().restore();
        return;
    }
    let r = match role {
        1 => Role::CounterManager,
        2 => Role::Kitchen,
        3 => Role::Owner,
        _ => Role::Waiter,
    };
    bd().session(signed != 0, pass != 0, floor != 0, r);
}
/// 0 offline, 1 live (socket), 2 polling.
#[no_mangle]
pub unsafe extern "C" fn set_sync(s: u32) {
    bd().sync = match s {
        1 => Sync::Live,
        2 => Sync::Polling,
        _ => Sync::Offline,
    };
}
/// The snapshot in `inbuf()[..len]` (feed.rs). Returns the tickets read.
#[no_mangle]
pub unsafe extern "C" fn feed(len: u32) -> u32 {
    bd().feed(input(len)) as u32
}
/// The open table's sheet rows in `inbuf()[..len]` (board/tsheet.rs); 0 closes it. Returns the
/// rows read.
#[no_mangle]
pub unsafe extern "C" fn sheet(len: u32) -> u32 {
    bd().sheet(input(len)) as u32
}
/// The stored theme (0 as the phone, 1 dark, 2 light) and whether it draws dark now.
#[no_mangle]
pub unsafe extern "C" fn set_theme(mode: u32, dark: u32) {
    bd().theme = (mode as u8).min(2);
    bd().dark = dark != 0;
}
/// The stored sound choice (1 on, 0 off; the host's default is on).
#[no_mangle]
pub unsafe extern "C" fn set_sound(on: u32) {
    bd().sound = on != 0;
}
/// The venue's ticket-age thresholds, minutes (amber, red); nonsense keeps 10 / 20.
#[no_mangle]
pub unsafe extern "C" fn set_ages(warn: u32, late: u32) {
    bd().set_ages(warn as i64, late as i64);
}
#[no_mangle]
pub unsafe extern "C" fn field_set(f: u32, len: u32) -> u32 {
    bd().fields.get_mut(f as usize).is_some_and(|x| x.set(input(len))) as u32
}
#[no_mangle]
pub unsafe extern "C" fn field_ptr(f: u32) -> *const u8 {
    bd().fields.get(f as usize).map_or(core::ptr::null(), |x| x.value().as_ptr())
}
#[no_mangle]
pub unsafe extern "C" fn field_len(f: u32) -> u32 {
    bd().fields.get(f as usize).map_or(0, |x| x.value().len() as u32)
}
/// 255 = no field focused.
#[no_mangle]
pub unsafe extern "C" fn focused() -> u32 {
    bd().focus.map_or(255, |f| f as u32)
}
#[no_mangle]
pub unsafe extern "C" fn blur() {
    bd().focus = None;
}
#[no_mangle]
pub unsafe extern "C" fn toast(len: u32) {
    bd().set_toast(input(len));
}
/// A toast in the board's own words: `k` is the word's number in `Str::ALL` (the host never
/// holds words; room/canvas/feed.js `W` names them).
#[no_mangle]
pub unsafe extern "C" fn toast_word(k: u32) {
    let w = crate::lang::Str::at(k as usize).unwrap_or(crate::lang::Str::Error);
    let s = bd().lang.s(w);
    bd().set_toast(s.as_bytes());
}
/// Draw the frame. Returns the command words.
#[no_mangle]
pub unsafe extern "C" fn frame(now_ms: f64) -> u32 {
    bd().now_ms = now_ms as i64;
    let mut host = Host;
    let mut ui = Ui {
        cmd: &mut CMD, scene: sc(), widths: &mut WIDTHS, host: &mut host,
        pal: if bd().dark { DARK } else { LIGHT }, lang: bd().lang,
    };
    bd().draw(&mut ui);
    CMD.len as u32
}
#[no_mangle]
pub unsafe extern "C" fn cmd_ptr() -> *const i32 {
    CMD.words.as_ptr()
}
#[no_mangle]
pub unsafe extern "C" fn frame_hash() -> u32 {
    CMD.hash()
}
/// kind: 0 down, 1 move, 2 up, 3 cancel. 1 = redraw.
#[no_mangle]
pub unsafe extern "C" fn pointer(kind: u32, x: i32, y: i32) -> u32 {
    bd().pointer(sc(), kind, x, y) as u32
}
#[no_mangle]
pub unsafe extern "C" fn wheel(dy: i32) {
    bd().wheel(dy);
}
#[no_mangle]
pub unsafe extern "C" fn key_enter() {
    bd().key_enter();
}

/// The next intent as 8 words: [kind, a, ptr, len, x, y, w, h]; kind 0 = none.
/// 1 bump (a: 0 confirm 1 preparing 2 ready 3 collected; ptr/len: the order id), 2 seen (id),
/// 3 lang (ptr/len: code), 4 refresh, 5 sign out, 6 focus (a: field; x..h its rect;
/// ptr: field kind), 7 blur, 8 submit (a: 1 claim), 9 table (ptr/len: sitting id),
/// 10 stop (a: 0 reject 1 cancel; ptr/len: the order id; the reason is field 3),
/// 11 sheet act (a: the row's act code; ptr/len: its argument), 12 Enter in sheet field a,
/// 13 theme, 14 sound.
#[no_mangle]
pub unsafe extern "C" fn intent() -> *const i32 {
    INTENT = [0; 8];
    let board = bd();
    let next = board.take_intent();
    let d = &board.data;
    let span = |s: crate::board::model::Span| -> (i32, i32) {
        let t = d.str(s);
        (t.as_ptr() as usize as i32, t.len() as i32)
    };
    let st = |s: &'static str| (s.as_ptr() as usize as i32, s.len() as i32);
    match next {
        None => {}
        Some(Intent::Bump { order, bump }) => {
            let (p, n) = span(order);
            INTENT[..4].copy_from_slice(&[1, bump as i32, p, n]);
        }
        Some(Intent::Seen { order }) => {
            let (p, n) = span(order);
            INTENT[..4].copy_from_slice(&[2, 0, p, n]);
        }
        Some(Intent::Lang(l)) => {
            let (p, n) = st(l.code());
            INTENT[..4].copy_from_slice(&[3, l.index() as i32, p, n]);
        }
        Some(Intent::Refresh) => INTENT[0] = 4,
        Some(Intent::SignOut) => INTENT[0] = 5,
        Some(Intent::Focus(f)) => {
            let r = board.field_rect(sc(), f).unwrap_or_default();
            let k = board.fields[(f as usize).min(crate::board::FIELDS - 1)].kind.host_code();
            INTENT = [6, f as i32, k, 0, r.x, r.y, r.w, r.h];
        }
        Some(Intent::Blur) => INTENT[0] = 7,
        Some(Intent::Submit { claim }) => INTENT[..2].copy_from_slice(&[8, claim as i32]),
        Some(Intent::Table { sitting }) => {
            let (p, n) = span(sitting);
            INTENT[..4].copy_from_slice(&[9, 0, p, n]);
        }
        Some(Intent::Sheet { act, arg }) => {
            let t = board.ts.str(arg);
            INTENT[..4].copy_from_slice(&[11, act as i32, t.as_ptr() as usize as i32, t.len() as i32]);
        }
        Some(Intent::SheetEnter(f)) => INTENT[..2].copy_from_slice(&[12, f as i32]),
        Some(Intent::Theme) => INTENT[0] = 13,
        Some(Intent::Sound) => INTENT[0] = 14,
        Some(Intent::Stop { cancel }) => {
            let id = board.ask_id();
            INTENT[..4].copy_from_slice(&[10, cancel as i32, id.as_ptr() as usize as i32, id.len() as i32]);
        }
    }
    INTENT.as_ptr()
}

/// Where a thing is in the last frame, as [found, x, y, w, h]; `inbuf()[..len]` names it.
/// kind 0: the first node with that learn anchor (`tour`); 1: the bump button of the ticket with
/// that order id; 2: the card of the open table with that sitting id. The live probe taps exactly
/// its own TEST order / TEST table with these, and the gates find controls by anchor.
#[no_mangle]
pub unsafe extern "C" fn rect_of(kind: u32, len: u32) -> *const i32 {
    let want = input(len);
    INTENT = [0; 8];
    let d = &bd().data;
    let hit = sc().nodes().iter().find(|n| match (kind, n.act) {
        (0, _) => !n.tour.is_empty() && n.tour.as_bytes() == want,
        (1, crate::scene::Act::Bump(i)) => d.tickets.get(i as usize).is_some_and(|t| d.str(t.id).as_bytes() == want),
        (2, crate::scene::Act::Table(i)) => d.tables.get(i as usize).is_some_and(|t| d.str(t.sitting).as_bytes() == want),
        _ => false,
    });
    if let Some(n) = hit {
        INTENT[..5].copy_from_slice(&[1, n.rect.x, n.rect.y, n.rect.w, n.rect.h]);
    }
    INTENT.as_ptr()
}
/// Every tour anchor in the last frame, as "a\nb\n..." written into `inbuf()`; returns the length.
#[no_mangle]
pub unsafe extern "C" fn tour_list() -> u32 {
    let mut n = 0usize;
    for node in sc().nodes() {
        let t = node.tour.as_bytes();
        if t.is_empty() || n + t.len() + 1 > IN {
            continue;
        }
        crate::put_at(&mut INBUF, n, t);
        crate::put_at(&mut INBUF, n + t.len(), b"\n");
        n += t.len() + 1;
    }
    n as u32
}

/// [nodes, cmd words, width misses, cmd overflow, scene dropped, data dropped, data bad,
///  nodes under 44 px, tickets, tables, sheet rows (0 = closed), sheet dropped + bad]
#[no_mangle]
pub unsafe extern "C" fn stats() -> *const u32 {
    STATS = [
        sc().len() as u32, CMD.len as u32, WIDTHS.misses, CMD.overflow, sc().dropped, bd().data.dropped,
        bd().data.bad, sc().too_small(crate::ui::TAP) as u32, bd().data.nt as u32, bd().data.ntab as u32,
        if bd().ts.open { bd().ts.n as u32 } else { 0 }, bd().ts.dropped + bd().ts.bad,
    ];
    STATS.as_ptr()
}
