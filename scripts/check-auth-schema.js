import { readFileSync } from 'fs';
const code = readFileSync('/app/dist/api/server.cjs', 'utf8');
const idx = code.indexOf('z.literal(\'owner\')');
if (idx >= 0) {
  const snippet = code.slice(idx, idx + 200);
  console.log('Owner schema snippet:', snippet);
  console.log('');
  console.log('Has activeLocationId:', snippet.includes('activeLocationId'));
} else {
  console.log('Could not find owner schema in bundle');
  // Try alternate format
  const idx2 = code.indexOf('z.literal("owner")');
  if (idx2 >= 0) {
    console.log('Found with double quotes:', code.slice(idx2, idx2 + 200));
  }
}
