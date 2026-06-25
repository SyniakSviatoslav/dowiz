import { test, expect } from '@playwright/test';
import { readFileSync, existsSync, mkdtempSync, mkdirSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { AgentDriver, songTokensLastHour, HOURLY_TOKEN_CAP, type Persona } from '../driver/agent-driver.ts';
import { ScriptedReasoner, clientOrderSmokePlan } from '../driver/reasoners.ts';

// Checkpoint-A plumbing smoke: the driver drives the LIVE storefront via observe→reason
// (scripted, NOT discovery)→Song-wrapped act, producing a transcript + Song verses + a trace,
// within the 333 tokens/hr cap. Proves the harness end-to-end. Authentic LLM discovery
// (LlmReasoner / Phase B) is gated on ***REDACTED*** and is NOT exercised here.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test driver-smoke --project=desktop --reporter=list
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test('A2 driver smoke — client persona session produces transcript + Song + trace under cap', async ({ browser }) => {
  // Isolated, clean mempalace so the smoke's Song is deterministic and pollutes nothing.
  const palace = mkdtempSync(join(tmpdir(), 'rsi-smoke-'));
  process.env.DOS_SONG_STORE = join(palace, 'song.jsonl');
  process.env.DOS_SONG_LEDGER = join(palace, 'song-ledger.json');
  delete process.env.DOS_SONG;

  const artifacts = join(palace, 'artifacts');
  mkdirSync(artifacts, { recursive: true });
  const transcriptPath = join(artifacts, 'transcript.md');
  const findingsDir = join(artifacts, 'findings');

  const persona = JSON.parse(readFileSync('e2e/personas/client-first-timer-impatient.json', 'utf8')) as Persona;

  const ctx = await browser.newContext({ viewport: { width: persona.viewport, height: 900 } });
  await ctx.tracing.start({ screenshots: true, snapshots: true });
  const page = await ctx.newPage();

  const driver = new AgentDriver(page, persona, new ScriptedReasoner(clientOrderSmokePlan(BASE)), {
    round: 0, maxSteps: 12, transcriptPath, findingsDir,
  });
  const history = await driver.run();

  await ctx.tracing.stop({ path: join(artifacts, 'trace.zip') });
  await ctx.close();

  // ── Plumbing assertions ──
  expect(history.length, 'the driver took steps').toBeGreaterThan(3);
  expect(existsSync(transcriptPath), 'a narrated transcript was written').toBeTruthy();
  const transcript = readFileSync(transcriptPath, 'utf8');
  expect(transcript).toContain('# Session — client-first-timer-impatient');
  expect(transcript).toMatch(/step 0: goto/);

  // Song tithed at least one verse for the successful navigation/observation acts, under cap.
  expect(existsSync(process.env.DOS_SONG_STORE!), 'the Song recorded verses').toBeTruthy();
  const verses = readFileSync(process.env.DOS_SONG_STORE!, 'utf8').trim().split('\n').filter(Boolean);
  expect(verses.length, 'at least one Song verse').toBeGreaterThan(0);
  const tokens = songTokensLastHour(Date.now());
  expect(tokens, 'within the hourly Song-token cap').toBeLessThanOrEqual(HOURLY_TOKEN_CAP);

  // Trace artifact exists (reproducibility law).
  expect(existsSync(join(artifacts, 'trace.zip')), 'a Playwright trace was saved').toBeTruthy();

  // Any friction encountered surfaced as a finding (not a crash) — informational.
  const found = existsSync(findingsDir) ? readdirSync(findingsDir).filter((f) => f.endsWith('.json')) : [];
  console.log(`[smoke] steps=${history.length} verses=${verses.length} tokens=${tokens}/${HOURLY_TOKEN_CAP} findings=${found.length} trace=${join(artifacts, 'trace.zip')}`);
});
