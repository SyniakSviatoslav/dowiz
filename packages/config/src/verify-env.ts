import { loadEnv } from './index.js';

try {
  loadEnv();
  console.log('OK');
  process.exit(0);
} catch (error: unknown) {
  console.error((error as Error).message);
  process.exit(1);
}
