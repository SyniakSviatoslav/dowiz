import { fileURLToPath } from 'url';
import { runInterconnectedRadar } from './internconnected.js';
import { loginMockOwner } from './harness/auth.js';

const source = process.argv[2] || 'apps/api/src/lib/orderStatusService.ts';
console.log(`Interconnected radar for: ${source}`);

const scriptPath = fileURLToPath(new URL('.', import.meta.url));
const rootDir = scriptPath.replace(/apps[/\\]api[/\\]scripts[/\\]radar[/\\].*$/i, '');
const sourceFile = rootDir + source.replace(/^[\\/]/, '');
console.log(`Full path: ${sourceFile}`);

loginMockOwner()
  .then(() => runInterconnectedRadar(sourceFile))
  .catch(err => { console.error('Radar failed:', err); process.exit(1); });
