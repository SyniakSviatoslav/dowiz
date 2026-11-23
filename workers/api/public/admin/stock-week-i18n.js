// The week's purchase list's words, merged into the console's dictionary at
// import (the shape `stock-health-i18n.js` set). The table itself is
// `stock-week-words.js`, which node tests.
import { T } from '/admin/i18n.js';
import { merge } from '/admin/stock-health-words.js';
import { WORDS } from '/admin/stock-week-words.js';

merge(T, WORDS);
export { WORDS };
