const fs = require('fs');
let content = fs.readFileSync('apps/api/src/routes/orders.ts', 'utf8');
content = content.replace(/\\\`/g, '`').replace(/\\\$/g, '$');
fs.writeFileSync('apps/api/src/routes/orders.ts', content);
