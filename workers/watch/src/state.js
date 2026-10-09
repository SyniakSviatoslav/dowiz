// The watcher's memory: the last result per target, the last tick, the last mail. One
// SQLite-backed Durable Object, written only when something changed, plus one row per tick
// (the tick time is the thing /healthz must be able to read after an eviction).
import { DurableObject } from 'cloudflare:workers';
import { transitions } from './fold.js';
import { newRun, step, canStart } from './evals.js';


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

  // THE NIGHTLY EVALS (evals.js): one run = one alarm per phase, so each phase has its own
  // invocation's subrequest budget. `evals_run` is the run in flight; `evals` the last finished one.
  async startEvals(atMs, cause) {
    const running = await this.ctx.storage.get('evals_run');
    if (!canStart(running, atMs)) return { started: false, running: { startedAtMs: running.startedAtMs, phase: running.phase } };
    await this.ctx.storage.put('evals_run', newRun(atMs, cause));
    await this.ctx.storage.setAlarm(Date.now());
    return { started: true };
  }

  async alarm() {
    const run = await this.ctx.storage.get('evals_run');
    if (!run) return;
    // A phase that throws is retried by the runtime (alarms retry with backoff); the collectors
    // themselves turn their own failures into collector_ok=0 rows, so a throw here is the runtime's.
    const r = await step(run, this.env, fetch, await this.ctx.storage.get('evals'));
    if (r.run) {
      await this.ctx.storage.put('evals_run', r.run);
      await this.ctx.storage.setAlarm(Date.now() + 1000);
      return;
    }
    await this.ctx.storage.put('evals', r.done);
    await this.ctx.storage.delete('evals_run');
  }

  async readEvals() {
    const [latest, running] = await Promise.all([this.ctx.storage.get('evals'), this.ctx.storage.get('evals_run')]);
    return { latest: latest || null, running: running || null };
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
