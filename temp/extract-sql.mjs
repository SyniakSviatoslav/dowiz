import { readFileSync, writeFileSync } from 'fs';
const src = readFileSync('packages/db/migrations/1790000000018_fix-public-menu-slug-lookup.ts', 'utf8');
// Extract SQL from pgm.sql(`...`)
const start = src.indexOf("pgm.sql(`") + 9;
const end = src.indexOf("`);", start);
const sql = src.substring(start, end).trim();
writeFileSync('temp/fix-function.sql', sql);
console.log('Extracted ' + sql.length + ' chars');
