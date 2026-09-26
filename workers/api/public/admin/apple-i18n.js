// The Apple pass's words, merged into the console's dictionary at import
// (lane W-APPLE, 2026-09-26). `admin/app.js` imports this once, next to
// `ux-i18n.js`; `parts.js` reads EXAMPLES and FOOTERS from apple-words.js
// directly, so a field asks for its example without going through here.
import { T } from '/admin/i18n.js';
import { WORDS, merge } from '/admin/apple-words.js';

merge(T, WORDS);
export { WORDS };
