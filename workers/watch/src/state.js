// The watcher's memory: the last result per target, the last tick, the last mail. One
// SQLite-backed Durable Object, written only when something changed, plus one row per tick
// (the tick time is the thing /healthz must be able to read after an eviction).
import { DurableObject } from 'cloudflare:workers';
import { transitions } from './fold.js';


export class WatchState extends DurableObject {
  /// Fold one tick in. Returns {transitions}. Writes only the targets whose status changed.
  async record(results, atMs) {
    const prev = (await this.ctx.storage.get('targets')) || {};
    const changes = transitions(prev, results);
    const next = {};
    for (const r of results) {
      const p = prev[r.name];
      const same = p && p.status === r.status;
      next[r.name] = {
        url: r.url,
        status: r.status,
        detail: r.detail,
        code: r.code,
        ms: r.ms,
        since: same ? p.since : atMs,
        checked: atMs,
      };
    }
    // `checked`, `detail` and `ms` move every tick but are only stored with a status change; the
    // tick row says when the last check was, and /status reports both.
    const changedSet = new Set(changes.map((c) => c.name));
    const removed = Object.keys(prev).some((k) => !next[k]);
    if (changedSet.size || removed || Object.keys(prev).length !== results.length) {
      await this.ctx.storage.put('targets', next);
    }
    await this.ctx.storage.put('tick', { atMs, results: results.length, latest: Object.fromEntries(results.map((r) => [r.name, { status: r.status, detail: r.detail, ms: r.ms }])) });
    return { transitions: changes };
  }

  async noteMail(entry) {
    await this.ctx.storage.put('mail', entry);
  }

  async read() {
    const [targets, tick, mail] = await Promise.all([
      this.ctx.storage.get('targets'),
      this.ctx.storage.get('tick'),
      this.ctx.storage.get('mail'),
    ]);
    return { targets: targets || {}, tick: tick || null, mail: mail || null };
  }
}
