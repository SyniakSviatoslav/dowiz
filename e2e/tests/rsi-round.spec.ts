import { test, expect } from '@playwright/test';
import { readFileSync, existsSync, mkdirSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { AgentDriver, songTokensLastHour, HOURLY_TOKEN_CAP, CapReached, type Persona } from '../driver/agent-driver.ts';
import { LlmReasoner } from '../driver/reasoners.ts';

// Phase B — ONE authentic discovery round. Each persona is driven by the LlmReasoner (real
// reasoning, free-tier model chain) against the LIVE service; friction → findings. Shares one
// Song ledger so the 333 tokens/hr cap is cumulative across the round (CapReached stops it).
// Requires OPENROUTER_API_KEY. Round/persona set via env (a subset here — NOT a saturation round).
//   OPENROUTER_API_KEY=… VITE_BASE_URL=https://dowiz-staging.fly.dev \
//   DOS_PERSONAS=client-first-timer-impatient,client-price-skeptic DOS_ROUND=1 \
//   pnpm exec playwright test rsi-round --project=desktop --reporter=list --timeout=600000
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const ROUND = Number(process.env.DOS_ROUND ?? 1);
const PERSONAS = (process.env.DOS_PERSONAS || 'client-first-timer-impatient,client-price-skeptic').split(',').map((s) => s.trim());
const MAX_STEPS = Number(process.env.DOS_MAX_STEPS ?? 8);

test('RSI round — LLM persona discovery against the live service', async ({ browser }) => {
  test.setTimeout(600_000);
  // Persistent shared Song ledger → the hourly cap is cumulative across personas this round.
  process.env.DOS_SONG_STORE = 'e2e/mempalace/song.jsonl';
  process.env.DOS_SONG_LEDGER = 'e2e/mempalace/song-ledger.json';
  delete process.env.DOS_SONG;

  const reasoner = new LlmReasoner(); // throws if no key — Phase B cannot fake discovery
  const roundDir = `e2e/findings/round-${ROUND}`;
  const summary: Array<{ persona: string; steps: number; findings: number; capped: boolean }> = [];

  for (const id of PERSONAS) {
    const path = `e2e/personas/${id}.json`;
    if (!existsSync(path)) { console.log(`[round] skip unknown persona ${id}`); continue; }
    const persona = JSON.parse(readFileSync(path, 'utf8')) as Persona;
    const out = join(roundDir, id);
    mkdirSync(out, { recursive: true });
    const findingsDir = join(out, 'findings');

    const ctx = await browser.newContext({ viewport: { width: persona.viewport, height: 900 }, locale: persona.locale });
    await ctx.tracing.start({ screenshots: true, snapshots: true });
    const page = await ctx.newPage();
    // Each session starts at the persona's natural entry surface.
    const entry = persona.role === 'client' ? `${BASE}/s/demo` : persona.role === 'courier' ? `${BASE}/courier` : `${BASE}/admin`;
    await page.goto(entry, { waitUntil: 'domcontentloaded' });

    const driver = new AgentDriver(page, persona, reasoner, {
      round: ROUND, maxSteps: MAX_STEPS, transcriptPath: join(out, 'transcript.md'), findingsDir,
    });
    let capped = false; let steps = 0; let aborted = '';
    try { steps = (await driver.run()).length; }
    catch (e) {
      // CapReached stops the whole round; any other session error is isolated to this persona
      // (logged, not fatal) so one flaky session can't sink the round.
      if (e instanceof CapReached) capped = true;
      else aborted = (e as Error)?.message?.split('\n')[0] ?? String(e);
    }
    if (aborted) console.log(`[round ${ROUND}] ${id}: session aborted — ${aborted}`);

    await ctx.tracing.stop({ path: join(out, 'trace.zip') });
    await ctx.close();
    const findings = existsSync(findingsDir) ? readdirSync(findingsDir).filter((f) => f.endsWith('.json')).length : 0;
    summary.push({ persona: id, steps, findings, capped });
    console.log(`[round ${ROUND}] ${id}: steps=${steps} findings=${findings} capped=${capped} tokens=${songTokensLastHour(Date.now())}/${HOURLY_TOKEN_CAP}`);
    if (capped) break;
  }

  console.log(`[round ${ROUND}] summary:`, JSON.stringify(summary));
  // The round ran authentic sessions and produced artifacts (findings are the open-ended output).
  expect(summary.length, 'at least one persona session ran').toBeGreaterThan(0);
  expect(summary.some((s) => s.steps > 0 || s.capped), 'sessions made progress or hit the cap').toBeTruthy();
});
