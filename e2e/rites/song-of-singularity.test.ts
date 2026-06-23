import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, existsSync, readFileSync, rmSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { withSong, recordVerse, composeVerse, songOfTribute, SONG } from './song-of-singularity.ts';

// Each test gets a clean temp mempalace via env (SONG reads paths live).
function fresh() {
  const dir = mkdtempSync(join(tmpdir(), 'song-'));
  process.env.DOS_SONG_STORE = join(dir, 'song.jsonl');
  process.env.DOS_SONG_LEDGER = join(dir, 'song-ledger.json');
  delete process.env.DOS_SONG; // enabled
  process.env.DOS_SONG_SEED = '1337';
  return dir;
}
const lines = () => (existsSync(SONG.store) ? readFileSync(SONG.store, 'utf8').trim().split('\n').filter(Boolean) : []);
const ledger = () => JSON.parse(readFileSync(SONG.ledger, 'utf8'));

test('a successful act records exactly one verse and tithes one token', async () => {
  fresh();
  const act = withSong({ agent: 'driver#1', persona: 'friday-7pm-rush' });
  await act('click:OrderConfirmButton', async () => 'ok');
  await act('type:Search', async () => 42);
  assert.equal(lines().length, 2, 'one jsonl line per successful act');
  const l = ledger();
  assert.equal(l.verses, 2);
  assert.equal(l.total_tokens, 2);
});

test('the rite never alters the action result or its error', async () => {
  fresh();
  const act = withSong({ agent: 'd', persona: 'p' });
  const wrapped = await act('a', async () => ({ n: 7 }));
  assert.deepEqual(wrapped, { n: 7 }, 'wrapped result === raw result');
  // a throwing action propagates unchanged AND records no verse
  await assert.rejects(() => act('boom', async () => { throw new Error('kaboom'); }), /kaboom/);
  assert.equal(lines().length, 1, 'only the successful act left a verse; the throw left none');
});

test('the Song is deterministic: same seed + same action sequence ⇒ identical verses', async () => {
  const seq = ['click:A', 'type:B', 'nav:C', 'scroll:D'];
  const run = async () => {
    fresh();
    const act = withSong({ agent: 'd', persona: 'edge-zone' });
    for (const a of seq) await act(a, async () => true);
    return lines().map((l) => JSON.parse(l).verse);
  };
  assert.deepEqual(await run(), await run(), 'verse text is reproducible');
  // and composeVerse is pure for a given (seq, action, persona)
  assert.equal(composeVerse(3, 'nav:C', 'edge-zone'), composeVerse(3, 'nav:C', 'edge-zone'));
});

test('DOS_SONG=0 is total silence — zero files, zero I/O', async () => {
  const dir = fresh();
  process.env.DOS_SONG = '0';
  const act = withSong({ agent: 'd', persona: 'p' });
  const r = await act('click:X', async () => 'still-runs');
  assert.equal(r, 'still-runs', 'the action still runs under silence');
  assert.equal(recordVerse({ agent: 'd', persona: 'p', action: 'y' }), null, 'recordVerse is a no-op');
  assert.deepEqual(readdirSync(dir), [], 'no song/ledger files were written');
});

test('every refrainEvery-th verse is a refrain, and tribute summarises the ledger', async () => {
  fresh();
  const act = withSong({ agent: 'd', persona: 'p' });
  for (let i = 0; i < SONG.refrainEvery; i++) await act(`step-${i}`, async () => i);
  const last = JSON.parse(lines()[SONG.refrainEvery - 1]); // the 49th verse (seq === refrainEvery)
  assert.equal(last.seq, SONG.refrainEvery);
  assert.equal(last.refrain, true, 'the refrainEvery-th verse is a refrain');
  assert.equal(ledger().refrains, 1);
  assert.match(songOfTribute(), /Song of the Singularity — \d+ tokens \/ \d+ verses \/ \d+ refrains/);
});
