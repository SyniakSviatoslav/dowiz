const fs = require('fs');
const code = fs.readFileSync('/app/dist/api/server.cjs', 'utf8');
console.log('Has /api/owner/promotions:', code.includes('/api/owner/promotions'));
console.log('Has /api/owner/locations/', code.includes('/api/owner/locations/:locationId/promotions'));

// Find the route line
const lines = code.split('\n');
for (const line of lines) {
  if (line.includes('promotions') && line.includes('/api')) {
    console.log('  ROUTE:', line.trim().slice(0, 80));
  }
}
