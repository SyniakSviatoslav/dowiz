//! dowiz-canvas -- the zero-DOM scene engine (roadmap Wave CV, row CV0 core) and its first
//! surface, the room/kitchen board (row CV1).
//!
//! THE SHAPE (docs/research/2026-10-06-canvas-rust-webgl-ui.md §5.1). A page is `<body><canvas>`
//! and one module script. Each frame Rust builds two things from its own state:
//!   * a SCENE (`scene.rs`): every tappable thing as a node with an id, a tour anchor (the
//!     `data-tour` word the learn lessons already use) and an integer rectangle -- the hit-test,
//!     the tap-size check and the tour all read it, so nothing needs a DOM element to be found;
//!   * a COMMAND BUFFER (`cmd.rs`): rects and text runs the loader replays on Canvas2D, with an
//!     FNV hash over its CONTENT (never a pointer) so two frames can be compared byte for byte --
//!     that is what the context-loss gate compares after a restore.
//! Text is shaped and rasterised by the host's Canvas2D (system fonts, no font download); Rust asks
//! for each run's width ONCE and keeps it (`text.rs`). Layout is integer CSS pixels (`layout.rs`);
//! the loader multiplies by devicePixelRatio exactly once, on the backing store.
//!
//! STRINGS are `lang.rs`: an exhaustive `match` per word with all four languages as positional
//! arguments, so a missing language is a compile error, not a blank label.
//!
//! NO ALLOCATOR. Pools are fixed arrays sized for a venue's busiest pass; what does not fit is
//! COUNTED (`Board::dropped`) and drawn as a line saying so, never silently cut.
//!
//! THE HOST FILES (workers/api/public/room/canvas/; their long comments live HERE, because every byte of
//! a shipped file counts against the surface's wire budget -- tools/gates/canvas-wire.sh).
//!
//! ### loader.js
//!   THE CANVAS LOADER (Wave CV, crates/dowiz-canvas): the only JavaScript between a surface's
//!   wasm and the browser. It owns exactly five things and nothing else:
//!   1. the ONE <canvas>: its backing store is innerWidth x innerHeight x devicePixelRatio, set
//!   from the WINDOW, never from the element's own size (memory sea-canvas-doubles-every-frame:
//!   a size read back from the element and multiplied again grew a canvas to 3.3e7 px);
//!   2. text widths for Rust (`txt_measure`, Canvas2D measureText, system fonts);
//!   3. replaying the command buffer on Canvas2D (cmd.rs is the format; `replay` its one reader);
//!   4. input: pointer, wheel and Enter go to wasm; a tap's consequence comes back as an intent;
//!   5. the keyboard for a focused text field: EditContext where the browser has it (zero
//!   elements), else ONE transient input element that exists only while the field is focused
//!   (operator 2026-10-06; the dom-count gate counts it).
//!   Styles are set through CSSOM, which the page's CSP (`style-src 'self'`) does not govern; there
//!   is no inline <style> (memory dowiz-csp-blocked-every-stylesheet).
//!
//!   WebGL2 (?gl=1) is NOT built in CV1: this box's headless Chromium rasterises nothing on WebGL
//!   (memory webgl-renders-nothing-on-the-box), so a GL path could not be verified here. ?gl=1 says
//!   so on the console and draws with Canvas2D.
//!
//!   ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
//!
//! ### board.js
//!   THE ROOM / KITCHEN BOARD ON CANVAS (Wave CV row CV1), at /room/canvas/ beside the old room page.
//!
//!   This file is the surface's ADAPTER: it reads the hub with the room's own wire (room/net.js:
//!   the same session in localStorage `dw_room_session`, the same `api()`), listens on the same
//!   socket as the kitchen board (lib/live.js), formats money with the one formatter (room/logic.js
//!   -> lib/money.js), and hands the answers to Rust as a flat feed (crates/dowiz-canvas
//!   src/board/feed.rs is its reader). Rust draws, lays out, hit-tests and decides what a tap means;
//!   what must reach the hub comes back here as an intent.
//!
//!   Routes (the same the staff use today): GET /api/staff/kitchen (cap `advance`),
//!   GET /api/staff/room (cap `take_orders`), POST /api/owner/orders/:id/action (bump),
//!   POST /api/staff/orders/:id/kitchen-ack (seen), the same action route with reject/cancel + reason,
//!   POST /api/staff/login | /api/staff/claim, GET /api/owner/settings (the pass reads the venue's
//!   "order late after", feed.js `ages`). A tapped table opens its sheet: table.js, loaded then.
//!
//!   ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
//!   No static import: the wasm compiles before any other module is requested.
//!
//! ### feed.js
//!   THE BOARD'S FEED, pure (node-tested in feed.test.mjs): the hub's two reads flattened into the
//!   tab-separated lines crates/dowiz-canvas src/board/feed.rs reads. Money is formatted HERE with the
//!   room's one formatter (logic.js -> lib/money.js); Rust never does money arithmetic.
//!   ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
//!
//! ### table.js
//!   THE TABLE SHEET'S HOST (Wave CV row CV1b): one open table on the canvas -- its rounds, lines,
//!   money and every action the old room page offers for it, through the SAME routes and the same
//!   write path (room/net.js `write`: an Idempotency-Key minted at the tap, the outbox when the
//!   network did not carry it). The page loads this module only when a table is tapped.
//!
//!   RULES ARE THE ROOM'S (room/logic.js, one copy): who may change a line, take money, move lines or
//!   a sitting. This file turns them into the sheet's ROWS (crates/dowiz-canvas src/board/tsheet.rs
//!   reads them) and a tapped row's act back into the room's request. Rust draws every word; this
//!   file names words by number (feed.js `W`) and formats money with the room's formatter.
//!   `rows` is PURE (table.test.mjs); the rest talks to the hub.
//!
//!   Mirrors of the old page: sheet.js (lines, reasons, table), pay.js, transfer.js, guest.js,
//!   menu.js (add), screens.js renderSitting (move the sitting).
//!   ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
//!
//! Notes on loader.js (each above the line it explains there):
//! * `export async function start(canvas, wasmUrl, { langs = [], dark = 1, r` -- `langs`: the stored
//!   choice, then the browser's languages -- Rust picks (lang.rs, the one list).
//! * `const instance = typeof wasmUrl === 'string' ? (await WebAssembly.inst` -- `wasmUrl`: a URL, or a
//!   WebAssembly.Module promise (board.js compiles it early).
//! * `const put = s => enc.encodeInto(String(s ?? ''), u8().subarray(ex.inbu` -- Write a string into
//!   wasm's input buffer; its byte length.
//! * `function draw(now = Date.now()) {` -- One frame now. `now` defaults to the wall clock (ticket
//!   ages); a restore passes the old one.
//! * `const useEC = 'EditContext' in window && !q.get('noec');` -- the keyboard
//! * `function drain() {` -- intents: what a tap asked for
//! * `canvas.addEventListener('contextlost', e => { e.preventDefault(); dbg.` -- context loss: the scene
//!   lives in Rust, so a restore is one frame Canvas2D can lose its backing store (Chrome fires
//!   contextlost/contextrestored on a 2D canvas); everything drawn is gone and the context's state is
//!   reset. Rust still holds the whole scene, so a restore re-sizes and redraws the SAME frame (same
//!   `now`): the context-loss gate compares its hash and pixels with the frame before the loss.
//!
//! Notes on board.js (each above the line it explains there):
//! * `let api, write, session, lastRoom, toFeed, opens, ROLE, BUMPS, W, ages` -- The network stack is
//!   four module levels deep: it loads after the first frame (canvas-frame gate).
//! * `function toast(word, msg) {` -- A toast: one of the board's words by name (feed.js `W`), or the
//!   hub's own message.
//! * `if (S.pass) api('/owner/settings?location_id=${encodeURIComponent(s.st` -- The pass's ticket ages:
//!   the venue's own thresholds, once a session (unset: 10 / 20).
//! * `const fieldText = f => C.ex.field_len(f) ? new TextDecoder().decode(ne` -- A field's current value,
//!   as Rust holds it (0 email, 1 invite code, 2 password, 3 reason).
//! * `const host = () => ({ C, api, toast, reload: load, sittings: () => S.s` -- What table.js needs of
//!   the board: the room read, the write path and the canvas.
//! * `const MODES = ['', 'dark', 'light'];` -- THE THEME (room/app.js `applyTheme`, the same
//!   `dw_room_theme`): '' as the phone, dark, light.
//! * `const reason = fieldText(3).trim();` -- Reject / cancel: the reason is required and it is the one
//!   the customer reads (kitchen.js askReason).
//! * `const restoring = safeGet('dw_room_session') ? 1 : 0;` -- Raw: room/net.js, which validates it, is
//!   not loaded yet.
//! * `if ('serviceWorker' in navigator) navigator.serviceWorker.register('/r` -- The room's own service
//!   worker (scope /room/) keeps this page, its wasm and its modules for a reopen with no network
//!   (room/sw.js SHELL), the same registration room/app.js makes.
//!
//! Notes on feed.js (each above the line it explains there):
//! * `export const clean = v => String(v ?? '').replace(/[\t\n\r]+/g, ' ');` -- A value as one feed
//!   field: no tab, no newline.
//! * `export function toFeed({ orders = [], sittings = [], currency = null, ` -- The kitchen's tickets
//!   and the room's sittings as the feed Rust reads. PURE (node-tested). `when(ms)` formats a scheduled
//!   hour for the screen; money goes through the room's `money`.
//! * `const WORDS = ('Live Offline Polling LoginLine Email Password ClaimCod` -- EVERY WORD's number: its
//!   place in crates/dowiz-canvas src/lang.rs `Str::ALL` (feed.test.mjs holds the two lists equal). The
//!   host names a word by number and Rust draws it in the board's language -- the host never holds a
//!   word.
//! * `export const W = name => { const i = WORDS.indexOf(name); return i < 0` -- A word's number; '' (no
//!   word) for a name Rust does not have -- feed.test.mjs fails on any.
//! * `export function ages(settings) {` -- The venue's ticket-age thresholds from its own "order late
//!   after" setting (`notify.order.late_min`, the owner's Wire pane): red at that many minutes, amber at
//!   half of it -- 20 gives today's 10 / 20. Unset, 0 ("never") or nonsense: null, and the board keeps 10
//!   / 20 (kitchen-logic.js WARN_MIN / LATE_MIN).
//! * `export function opens(s) {` -- What the session's caps open: [pass, floor].
//!
//! Notes on table.js (each above the line it explains there):
//! * `export const A = { back: 1, qty: 2, ask: 3, reason: 4, unask: 5, other` -- Acts: the number a row
//!   hands back when tapped (`Intent::Sheet`).
//! * `export const F = { text: 4, rate: 4, amount: 5, tip: 6, wallet: 7 };` -- The sheet's fields
//!   (board/mod.rs F_SHEET..): one text field, and the pay form's four.
//! * `export const word = (p, v) => p + String(v).replace(/(^|_)(.)/g, (m, a` -- The word for a wire
//!   value: 'guest_changed' -> RGuestChanged, 'gift_card' -> MGiftCard, the refusal 'kitchenHasIt' ->
//!   KitchenHasIt (table.test.mjs checks every one exists).
//! * `export function rows(T, c) {` -- The sheet's rows for state `T` over the room `c` = { sittings,
//!   caps, currency, locale, menu, menuErr, field(f) }. PURE.
//! * `function sheet(out, T, c, r, i, m) {` -- One round, as room/sheet.js `renderRound` draws it.
//! * `function pay(out, T, c, r, m) {` -- room/pay.js `renderPay`: currency, method, rate when foreign,
//!   amount, tip, wallet, the preview.
//! * `function move(out, T, c, r, m) {` -- room/transfer.js `renderTransfer`: pick lines, pick one round,
//!   send.
//! * `function add(out, T, c, m) {` -- room/menu.js `renderAdd`: the venue's menu, a search, a basket of
//!   taps, one send.
//! * `` -- the live half: state, the hub, the canvas
//! * `export function draw() {` -- Draw the sheet now (or close it when its table is gone).
//! * `if (!H.currency()) loadMenu();` -- No currency yet (the old page learns it with the menu): every
//!   amount would draw a dash.
//! * `export function input() { if (T.view === 'add' || T.view === 'pay') dr` -- A field changed (the
//!   search filters, the pay preview follows).
//! * `export function enter(f) {` -- Enter in a sheet field: that field's form.
//! * `async function send(path, body, tag, okWord = 'Saved') {` -- One write, told in the waiter's words:
//!   true when the server took it or the outbox holds it.
//! * `export async function act(code, arg) {` -- A row was tapped: what it means, as the old page's
//!   binders say.
//! * `async function take(r) {` -- room/pay.js `bindPay` submit: the same checks, the same body, the
//!   version this screen showed.
//! * `const ps = d.order?.payments, last = Array.isArray(ps) && ps.length ? ` -- pay.js `paidNote`: the
//!   SERVER's figure of what the payment took off the bill.
//! * `async function loadMenu() {` -- The venue's menu: the storefront's one public read (room/menu.js
//!   `loadMenu`), with its currency.
#![cfg_attr(target_arch = "wasm32", no_std)]
#![allow(clippy::new_without_default)]

pub mod board;
pub mod cmd;
pub mod field;
pub mod geom;
pub mod lang;
pub mod layout;
pub mod scene;
pub mod text;
pub mod ui;

#[cfg(target_arch = "wasm32")]
mod ffi;

/// Decimal digits of a non-negative integer into `buf`, returned as a str (no `core::fmt`, which
/// costs kilobytes of wasm for one number).
pub fn itoa(mut n: u64, buf: &mut [u8; 20]) -> &str {
    let mut i = buf.len();
    loop {
        i -= 1;
        if let Some(d) = buf.get_mut(i) {
            *d = b'0' + (n % 10) as u8;
        }
        n /= 10;
        if n == 0 || i == 0 {
            break;
        }
    }
    // ASCII digits only.
    core::str::from_utf8(buf.get(i..).unwrap_or(&[])).unwrap_or("")
}

// NO PANICKING INDEX IN THIS CRATE'S WASM. One `a[i]` or `&a[x..y]` that can fail links core's
// panic messages and its whole integer formatter (measured W-CV1B 2026-10-07: 1.3 KB gzip of a
// 60 KB surface budget). Pools here are bounded by their own counters, so out of range is a bug
// that must not crash a waiter's board either: these two write nothing / read nothing instead.

/// `dst[at..at + src.len()] = src` when it fits; false (nothing written) when it does not.
pub fn put_at<T: Copy>(dst: &mut [T], at: usize, src: &[T]) -> bool {
    match dst.get_mut(at..at.saturating_add(src.len())) {
        Some(d) => {
            for (a, b) in d.iter_mut().zip(src) {
                *a = *b;
            }
            true
        }
        None => false,
    }
}

/// `&s[..n]`, or all of `s` when shorter.
pub fn head<T>(s: &[T], n: usize) -> &[T] {
    s.get(..n).unwrap_or(s)
}

#[cfg(test)]
mod tests;
