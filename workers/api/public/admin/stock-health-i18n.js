// The stock check's words, merged into the console's dictionary at import (the
// shape `ux-i18n.js` set). The table itself is `stock-health-words.js`, which node tests.
import { T } from '/admin/i18n.js';
import { WORDS, merge } from '/admin/stock-health-words.js';

merge(T, WORDS);
export { WORDS };
