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
static mut STATS: [u32; 10] = [0; 10];

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
    &INBUF[..(len as usize).min(IN)]
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
/// A toast in the board's own words: 1 saved, 2 error, 3 offline (the host never holds words).
#[no_mangle]
pub unsafe extern "C" fn toast_word(k: u32) {
    let w = match k {
        1 => crate::lang::Str::Saved,
        3 => crate::lang::Str::Offline,
        _ => crate::lang::Str::Error,
    };
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
/// 10 stop (a: 0 reject 1 cancel; ptr/len: the order id; the reason is field 3).
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
            let k = board.fields[(f as usize).min(3)].kind.host_code();
            INTENT = [6, f as i32, k, 0, r.x, r.y, r.w, r.h];
        }
        Some(Intent::Blur) => INTENT[0] = 7,
        Some(Intent::Submit { claim }) => INTENT[..2].copy_from_slice(&[8, claim as i32]),
        Some(Intent::Table { sitting }) => {
            let (p, n) = span(sitting);
            INTENT[..4].copy_from_slice(&[9, 0, p, n]);
        }
        Some(Intent::Stop { cancel }) => {
            let id = board.ask_id();
            INTENT[..4].copy_from_slice(&[10, cancel as i32, id.as_ptr() as usize as i32, id.len() as i32]);
        }
    }
    INTENT.as_ptr()
}

/// The rectangle of a tour anchor in the last frame: [found, x, y, w, h].
#[no_mangle]
pub unsafe extern "C" fn tour_rect(len: u32) -> *const i32 {
    let want = core::str::from_utf8(input(len)).unwrap_or("");
    INTENT = [0; 8];
    if let Some(n) = sc().find_tour(want) {
        INTENT[..5].copy_from_slice(&[1, n.rect.x, n.rect.y, n.rect.w, n.rect.h]);
    }
    INTENT.as_ptr()
}
/// Where the bump button of the ticket whose id is `inbuf()[..len]` is in the last frame:
/// [found, x, y, w, h]. The live probe taps exactly its own TEST order with it.
#[no_mangle]
pub unsafe extern "C" fn bump_rect(len: u32) -> *const i32 {
    let want = input(len);
    INTENT = [0; 8];
    for n in sc().nodes() {
        if let crate::scene::Act::Bump(i) = n.act {
            let t = bd().data.tickets[(i as usize).min(crate::board::feed::MAX_TICKETS - 1)];
            if bd().data.str(t.id).as_bytes() == want {
                INTENT[..5].copy_from_slice(&[1, n.rect.x, n.rect.y, n.rect.w, n.rect.h]);
                break;
            }
        }
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
        INBUF[n..n + t.len()].copy_from_slice(t);
        INBUF[n + t.len()] = b'\n';
        n += t.len() + 1;
    }
    n as u32
}

/// [nodes, cmd words, width misses, cmd overflow, scene dropped, data dropped, data bad,
///  nodes under 44 px, tickets, tables]
#[no_mangle]
pub unsafe extern "C" fn stats() -> *const u32 {
    STATS = [
        sc().len() as u32, CMD.len as u32, WIDTHS.misses, CMD.overflow, sc().dropped, bd().data.dropped,
        bd().data.bad, sc().too_small(crate::ui::TAP) as u32, bd().data.nt as u32, bd().data.ntab as u32,
    ];
    STATS.as_ptr()
}
