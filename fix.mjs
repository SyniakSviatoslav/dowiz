import fs from 'fs';
for (const file of ['apps/api/src/client/status/app.ts', 'apps/api/src/client/checkout/app.ts']) {
  let content = fs.readFileSync(file, 'utf8');
  content = content.replace(/\\`/g, '`');
  content = content.replace(/\\\${/g, '${');
  fs.writeFileSync(file, content);
}
