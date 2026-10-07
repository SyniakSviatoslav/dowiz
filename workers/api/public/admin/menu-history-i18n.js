// The menu history's words (W-PITR), merged into the console's dictionary at
// import (the shape `lost-sales-i18n.js` set). The table itself is
// `menu-history-words.js`, which node tests.
import { T } from '/admin/i18n.js';
import { WORDS, merge } from '/admin/menu-history-words.js';

merge(T, WORDS);
export { WORDS };
