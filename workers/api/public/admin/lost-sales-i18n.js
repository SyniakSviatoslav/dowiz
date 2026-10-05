// The W-LOST screens' words (lost sales, option recipes), merged into the
// console's dictionary at import (the shape `prep-list-i18n.js` set). The
// table itself is `lost-sales-words.js`, which node tests.
import { T } from '/admin/i18n.js';
import { WORDS, merge } from '/admin/lost-sales-words.js';

merge(T, WORDS);
export { WORDS };
