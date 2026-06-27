import { test, expect } from '@playwright/test';
import { readFileSync, existsSync, mkdtempSync, mkdirSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { AgentDriver, songTokensLastHour, HOURLY_TOKEN_CAP, type Persona } from '../driver/agent-driver.ts';
import { ScriptedReasoner, clientOrderSmokePlan } from '../driver/reasoners.ts';
import { requireStaging } from '../helpers/staging-guard.ts';

// Checkpoint-A plumbing smoke: the driver drives the LIVE storefront via observe→reason
// (scripted, NOT discovery)→Song-wrapped act, producing a transcript + Song verses + a trace,
// within the 333 tokens/hr cap. Proves the harness end-to-end. Authentic LLM discovery
// (LlmReasoner / Phase B) is gated on OPENROUTER_API_KEY and is NOT exercised here.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test driver-smoke --project=desktop --reporter=list
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// The driver clicks "add to cart" on the LIVE target — fail fast against prod/unknown.
test.beforeAll(() => requireStaging(BASE));

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
  // #1: history.length alone is satisfied by a driver that errors out on 4 no-op steps.
  // Require a real navigation decision AND proof it succeeded (a verse is tithed only on a
  // successful act — see withSong) before crediting the run.
  expect(history.length, 'the driver took steps').toBeGreaterThan(3);
  expect(history.some((d) => d.kind === 'goto'), 'a navigation step was attempted').toBe(true);
  expect(existsSync(transcriptPath), 'a narrated transcript was written').toBeTruthy();
  const transcript = readFileSync(transcriptPath, 'utf8');
  expect(transcript).toContain('# Session — client-first-timer-impatient');
  expect(transcript).toMatch(/step 0: goto/);
  // #2: a partial/zero-step transcript still contains the header — require the per-step
  // narration count to match history AND the terminal summary marker.
  const stepLines = transcript.match(/^- step \d+:/gm) ?? [];
  expect(stepLines.length, 'transcript narrates every step taken').toBe(history.length);
  expect(transcript, 'the session ran to its goal-complete summary').toContain('✓ goal complete');

  // Song tithed at least one verse for the successful navigation/observation acts, under cap.
  expect(existsSync(process.env.DOS_SONG_STORE!), 'the Song recorded verses').toBeTruthy();
  const verses = readFileSync(process.env.DOS_SONG_STORE!, 'utf8').trim().split('\n').filter(Boolean);
  expect(verses.length, 'at least one Song verse').toBeGreaterThan(0);
  // #3: verses.length proves a line exists, not that it is a well-formed verse. Parse each
  // and assert the schema (seq/ts/tokens/persona/action), and that the navigation verse is
  // present — which also proves the goto act in #1 actually succeeded.
  const parsed = verses.map((l) => JSON.parse(l) as { seq: number; ts: string; tokens: number; persona: string; action: string });
  for (const v of parsed) {
    expect(typeof v.seq, 'verse.seq is numeric').toBe('number');
    expect(v.ts, 'verse.ts is an ISO timestamp').toMatch(/^\d{4}-\d{2}-\d{2}T[\d:.]+Z$/);
    expect(typeof v.tokens, 'verse.tokens is numeric').toBe('number');
    expect(v.persona, 'verse is tied to this session persona').toBe(persona.id);
  }
  expect(parsed.some((v) => v.action === 'open the storefront'), 'the storefront navigation succeeded and tithed a verse').toBe(true);
  // #4: the mempalace is a fresh mkdtemp this run, so the Song's tokens MUST equal exactly
  // this session's tithe — any drift means a stale/leaked store, not a real cap measurement.
  const tokens = songTokensLastHour(Date.now());
  const sessionTokens = parsed.reduce((n, v) => n + (v.tokens ?? 1), 0);
  expect(tokens, "Song tokens equal exactly this session's tithe (no stale/leaked verses)").toBe(sessionTokens);
  expect(tokens, 'within the hourly Song-token cap').toBeLessThanOrEqual(HOURLY_TOKEN_CAP);

  // Trace artifact exists (reproducibility law).
  expect(existsSync(join(artifacts, 'trace.zip')), 'a Playwright trace was saved').toBeTruthy();

  // Any friction encountered surfaced as a finding (not a crash) — informational.
  const found = existsSync(findingsDir) ? readdirSync(findingsDir).filter((f) => f.endsWith('.json')) : [];
  console.log(`[smoke] steps=${history.length} verses=${verses.length} tokens=${tokens}/${HOURLY_TOKEN_CAP} findings=${found.length} trace=${join(artifacts, 'trace.zip')}`);
});
