// The words of a write's notice (`/lib/outcome-words.js`), merged into the
// console's T on import. ASCII QUOTES ONLY as string delimiters.
import { T } from '/admin/i18n.js';
import { WORDS } from '/lib/outcome-words.js';

for (const [l, w] of Object.entries(WORDS)) if (T[l]) Object.assign(T[l], w);
