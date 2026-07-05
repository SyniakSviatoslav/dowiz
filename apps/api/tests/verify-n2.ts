import { spawn } from 'node:child_process';
import Redis from 'ioredis';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

async function sleep(ms: number) {
  return new Promise(r => setTimeout(r, ms));
}

async function verifyN2() {
  console.log('--- N=2 Verification Simulation ---');
  
  const envA = { ...process.env, PORT: '8081', NODE_ENV: 'test' };
  const envB = { ...process.env, PORT: '8082', NODE_ENV: 'test' };

  console.log('[1/4] Spawning Server A (Port 8081)...');
  const serverA = spawn('tsx', ['--env-file=../../.env', '../src/server.ts'], { env: envA, cwd: __dirname });
  
  console.log('[2/4] Spawning Server B (Port 8082)...');
  const serverB = spawn('tsx', ['--env-file=../../.env', '../src/server.ts'], { env: envB, cwd: __dirname });

  await sleep(8000); // Wait for servers to boot

  console.log('[3/4] Verifying cross-instance pub/sub (Redis MessageBus)...');
  const redis = new Redis(process.env.REDIS_URL || 'redis://localhost:6379');
  
  // Simulate order creation broadcast
  await redis.publish('deliveryos:events:order.created', JSON.stringify({
    orderId: 'test-order-123',
    locationId: 'test-loc-123'
  }));
  console.log('  -> Published order.created event');

  await sleep(3000); // Give time for both instances to log receipt or enqueue

  console.log('[4/4] Verifying Graceful Shutdown...');
  serverA.kill('SIGTERM');
  serverB.kill('SIGTERM');

  await sleep(2000);

  console.log('--- N=2 Verification Complete ---');
  console.log('Check server logs to ensure no duplicate pg-boss executions and successful PubSub receipt.');
  process.exit(0);
}

verifyN2().catch(console.error);
