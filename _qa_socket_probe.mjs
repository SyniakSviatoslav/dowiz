// Does the live socket actually open, and does it carry an event?
import { chromium } from 'playwright';
import fs from 'node:fs';
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));
const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
const ctx = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block' });
const p = await ctx.newPage();
const seen = [];
p.on('websocket', ws => {
  seen.push(`open ${ws.url().replace(HOST, '')}`);
  ws.on('framereceived', f => seen.push(`in  ${String(f.payload).slice(0, 120)}`));
  ws.on('framesent', f => seen.push(`out ${String(f.payload).slice(0, 80)}`));
  ws.on('socketerror', e => seen.push(`ERR ${String(e).slice(0, 120)}`));
  ws.on('close', () => seen.push('close'));
});
p.on('console', m => { if (/WebSocket|live/i.test(m.text())) seen.push(`console ${m.text().slice(0, 140)}`); });
await p.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
await p.waitForSelector('#e', { timeout: 40000 });
await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
await p.click('#go');
await p.waitForTimeout(12000);
console.log(seen.length ? seen.join('\n') : 'no websocket activity at all');
await b.close();
