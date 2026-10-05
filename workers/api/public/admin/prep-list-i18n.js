// The prep list's words, merged into the console's dictionary at import (the
// shape `stock-health-i18n.js` set). The table itself is `prep-list-words.js`, which node tests.
import { T } from '/admin/i18n.js';
import { WORDS, merge } from '/admin/prep-list-words.js';

merge(T, WORDS);
export { WORDS };
