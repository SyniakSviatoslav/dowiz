import { loadEnv } from '../packages/config/src/index.js';
import { signAuthToken } from '../packages/platform/src/index.js';
import crypto from 'crypto';
import WebSocket from 'ws';

const env = loadEnv();

async function runTests() {
  console.log('--- WebSocket Auth Test ---');
  
  // Create a valid token
  const token = await signAuthToken({ role: 'owner', userId: crypto.randomUUID() }, '15m');
  
  // Try connecting
  const ws = new WebSocket(`ws://localhost:${env.PORT || 3000}`);
  
  ws.on('open', () => {
    console.log('Connected, sending auth...');
    ws.send(JSON.stringify({ type: 'auth', token }));
  });

  ws.on('message', (data) => {
    console.log('Received:', data.toString());
    if (data.toString().includes('auth_success')) {
      console.log('✅ WS Handshake succeeded');
      ws.close();
    }
  });

  ws.on('close', (code, reason) => {
    console.log(`Connection closed: ${code} ${reason}`);
    if (code !== 1000) {
      process.exit(1);
    }
  });

  ws.on('error', (err) => {
    console.error('WS Error:', err);
    process.exit(1);
  });
}

runTests();
