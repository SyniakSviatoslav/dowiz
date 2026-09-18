// localStorage throws in private mode / blocked-site-data. Never let that break
// the page: an unreadable cart is an empty cart, not a crash. Separate from
// state.js so i18n.js can read the language without importing the whole state.
export function safeGet(k){ try { return localStorage.getItem(k); } catch { return null; } }
export function safeSet(k, v){ try { localStorage.setItem(k, v); } catch {} }
export function safeDel(k){ try { localStorage.removeItem(k); } catch {} }
