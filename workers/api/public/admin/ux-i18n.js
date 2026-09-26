// The redesign's words, merged into the console's dictionary at import (the
// shape `kitchen-i18n.js` set): `t()`, `data-t` and `retranslate()` read them
// like any other key. The table itself is `ux-words.js`, which node tests.
import { T } from '/admin/i18n.js';
import { WORDS, merge } from '/admin/ux-words.js';

merge(T, WORDS);
export { WORDS };
