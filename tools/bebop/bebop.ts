#!/usr/bin/env node
// Bebop — your own coding agent CLI for dowiz (a la Claude Code / Hermes).
// Owns the tool loop, model routing, and hooks; bakes the dowiz Operating System in as NATIVE
// behavior. Brand: Warm Cosmo-Noir, main signal color = Cowboy Bebop ship teal #46B0A4.
//
// Subcommands:
//   boot            run the guard self-test (Verified-by-Math — refuse to start if gates can't go RED)
//   run [task]      run the agentic loop (default: deterministic stub, no live model)
//   recall <q>      query the living-knowledge §0·GP retriever
//   route <class>   show the token-router decision (doer/reason/redline)
//   help            this text

import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { selfTest } from './src/guard.ts';
import { runLoop } from './src/loop.ts';
import { route, enforceRouting, type TaskClass } from './src/router.ts';
import { recall, rememberLocal } from './src/knowledge.ts';
import { livingMemory } from './src/memory.ts';
import { ContentStore } from './src/store.ts';
import { banner, makePaint } from './src/theme.ts';
import { BOOT } from './src/voice.ts';
import { init, loadProfile, statusLine, writeProfile } from './src/init.ts';
import { probeAll, selectBackend } from './src/routing.ts';
import { BEBOP_PRESET } from './src/profile.ts';
import { runBackend, type Backend } from './src/backend.ts';
import { runCopilot } from './src/copilot.ts';
import { Governor } from './src/governor.ts';
import { startSyncServer } from './src/sync-server.ts';
import { selfMaintain, selfEvolve, recordSession, selfLoop } from './src/consciousness.ts';
import { createOrUnlock, lock, unlock, loadBlob } from './src/vault.ts';

const HERE = path.dirname(fileURLToPath(import.meta.url));

async function main() {
  const [, , cmd, ...args] = process.argv;
  const paint = makePaint();

  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    console.log(banner(paint));
    console.log(paint.dim('  boot | init [--preset bebop|--json {...}] | run [doer|reason|redline] | dispatch "<task>"'));
    console.log(paint.dim('  status | recall <query> | route <class> | sync [--port N] | help'));
    console.log(paint.dim(`  ${BOOT.link}`));
    return;
  }

  if (cmd === 'boot') {
    const t = selfTest();
    for (const l of t.log) console.log(paint.dim('  · ' + l));
    if (t.ok) {
      console.log(paint.teal('  ✓ Bebop guard OS certified: gates deny on red, pass on green.'));
    } else {
      console.log(paint.blood('  ✖ Guard self-test FAILED. The machine refuses to lie — fix before ship.'));
      process.exit(1);
    }
    return;
  }

  if (cmd === 'init') {
    const preset = args.includes('--preset') ? args[args.indexOf('--preset') + 1] : undefined;
    const jsonIdx = args.indexOf('--json');
    const json = jsonIdx >= 0 ? args[jsonIdx + 1] : undefined;
    const force = args.includes('--force');
    const profile = await init({ preset, json, force });
    const p = writeProfile(profile);
    console.log(paint.teal(`  ✓ Profile written → ${p}`));
    console.log(paint.dim(`  origin=${profile.origin} class=${profile.classKind} narration=${profile.narration} patrons=${profile.patrons} looks=${profile.looks}`));
    console.log(paint.dim(`  backend rotation: ${statusLine(profile)}`));
    console.log(paint.bold(paint.bone(`  ${BEBOP_PRESET === profile ? 'Bebop native preset engaged. Hybrid is a feature, not a bug.' : 'Custom profile engaged.'}`)));
    return;
  }

  if (cmd === 'status') {
    const profile = loadProfile() ?? BEBOP_PRESET;
    console.log(banner(paint));
    console.log(paint.dim(`  rotation: ${statusLine(profile)}  (* = not installed / no key)`));
    for (const r of probeAll(profile)) {
      const mark = r.available ? paint.teal('ready') : paint.amber('idle');
      console.log(paint.dim(`  · ${r.backend.padEnd(9)} ${mark}`));
    }
    return;
  }

  if (cmd === 'sync') {
    const portIdx = args.indexOf('--port');
    const port = portIdx >= 0 ? Number(args[portIdx + 1]) : Number(process.env.BEBOP_SYNC_PORT ?? 8787);
    console.log(paint.teal(`  ◈ Starting Bebop sync node (Better Auth, self-hosted) on :${port}`));
    console.log(paint.dim('    No Supabase. No Fly. Your keys, your machine. Ctrl-C to stop.'));
    const srv = await startSyncServer({ port });
    console.log(paint.teal(`  ✓ Sync node live → ${srv.url}  (signup: ${srv.url}/sign-up)`));
    // Keep the process alive until interrupted.
    await new Promise<void>((resolve) => {
      process.on('SIGINT', () => resolve());
      process.on('SIGTERM', () => resolve());
    });
    await srv.close();
    console.log(paint.dim('  sync node stopped.'));
    return;
  }

  if (cmd === 'route') {
    const cls = (args[0] as TaskClass) ?? 'doer';
    const d = route(cls);
    const g = enforceRouting(cls, d.model);
    console.log(paint.teal(`  ${cls} → ${paint.bold(d.model)}`));
    console.log(paint.dim('  ' + d.rationale));
    if (!g.ok) console.log(paint.blood('  ' + g.note));
    return;
  }

  if (cmd === 'recall') {
    const q = args.join(' ');
    const r = recall(q);
    console.log(paint.dim(`  §0·GP recall — ${r.note}`));
    for (const h of r.hits) console.log(paint.teal(`  ◈ ${h.id}: ${h.text.slice(0, 100)}`));
    return;
  }

  if (cmd === 'remember') {
    // bebop remember <concept> :: <payload>  — write into the ONE living memory (this session included)
    const raw = args.join(' ');
    const sep = raw.indexOf('::');
    if (sep < 0) {
      console.log(paint.blood('  usage: bebop remember <concept> :: <payload>'));
      process.exit(2);
    }
    const concept = raw.slice(0, sep).trim();
    const payload = raw.slice(sep + 2).trim();
    const id = rememberLocal(concept, payload, args.includes('--link') ? [args[args.indexOf('--link') + 1]] : undefined);
    console.log(paint.teal(`  ✓ remembered "${concept}" → ${id.slice(0, 12)} (living memory size=${livingMemory().size})`));
    return;
  }

  if (cmd === 'memory') {
    // bebop memory — show the ONE living memory state (this Hermes session is a node)
    const mem = livingMemory();
    const sub = args[0];
    if (sub === 'tick') {
      // advance the forgetting clock: decay + eviction (human-like memory)
      const n = Math.max(1, Number(args[1] ?? 1));
      const before = mem.size;
      for (let i = 0; i < n; i++) mem.tick();
      console.log(paint.dim(`  ticked ${n}×: size ${before} → ${mem.size} (forgot ${before - mem.size})`));
      console.log(paint.dim(`  layers: working=${mem.layerSize('working')} short=${mem.layerSize('short')} long=${mem.layerSize('long')}`));
      return;
    }
    if (sub === 'layers') {
      console.log(paint.dim(`  layers: working=${mem.layerSize('working')} short=${mem.layerSize('short')} long=${mem.layerSize('long')} (total=${mem.size})`));
      return;
    }
    console.log(paint.dim(`  living memory size=${mem.size}`));
    console.log(paint.dim(`  layers: working=${mem.layerSize('working')} short=${mem.layerSize('short')} long=${mem.layerSize('long')}`));
    console.log(paint.dim(`  nearest to "copilot": ${JSON.stringify(mem.nearest('copilot', 3))}`));
    console.log(paint.dim(`  recall "copilot": ${JSON.stringify(mem.recall('copilot', 2))}`));
    return;
  }

  if (cmd === 'store') {
    // bebop store <dir> [append <cause> <data> | put <index> <text> | verify]
    const dir = args[0] ?? path.resolve(HERE, '.bebop', 'store');
    const op = args[1];
    const store = new ContentStore(dir);
    if (op === 'append') {
      const cause = args[2] ?? 'cause-x';
      const data = args.slice(3).join(' ') || 'tick';
      const ev = store.appendEvent(cause, data);
      console.log(paint.teal(`  ✓ event #${ev.seq} chained (hash ${ev.hash.slice(0, 12)})`));
    } else if (op === 'put') {
      const idx = Number(args[2] ?? 0);
      const text = args.slice(3).join(' ') || 'piece';
      const p = store.putPiece(idx, new TextEncoder().encode(text));
      console.log(paint.teal(`  ✓ piece #${idx} address ${p.hash.slice(0, 12)}`));
    } else {
      console.log(paint.dim(`  store dir=${dir} events=${store.eventCount} chainOk=${store.verifyChain()}`));
    }
    return;
  }

  if (cmd === 'dispatch') {
    const task = args.join(' ');
    // Governor: PID authority from copilot verdict (reject = mistake ⇒ freedom shrinks; approve = air).
    const gov = new Governor({ kp: 1.4, ki: 0.22, kd: 1.5, iMin: -1, iMax: 1, uMin: 0, uMax: 1, targetQuality: 0.9, deadIC: 0.02, icirVolatile: 0.3, plantM: 1, plantB: 0.6, samplePeriod: 0, anomalyK: 3, maxStep: 1 });
    const profile = loadProfile();
    // Native copilot mode is DEFAULT: the doer (below) produces, a DISTINCT checker (above) verifies
    // in real time. Pass --no-copilot to opt out.
    const copilotOff = args.includes('--no-copilot');
    let authority = 1;
    const res = runCopilot({
      task,
      profile: loadProfile() ?? undefined,
      enabled: !copilotOff,
      runNative: (t) => ({ ok: true, backend: 'native', summary: `native handled: ${t.slice(0, 40)}`, exitCode: 0 }),
    });
    // feed the verdict as proven quality telemetry. The Governor is a SERVO (error = target − actual),
    // so to get "approve ⇒ more freedom / reject ⇒ less" we feed the QUALITY DEFICIT (1 − quality):
    // approve (quality 1) ⇒ deficit 0 ⇒ error +0.9 ⇒ authority rises; reject ⇒ deficit 1 ⇒ authority falls.
    const quality = res.ok ? 1 : 0;
    const st = gov.step({ t: Date.now(), predictedQuality: quality, actualQuality: 1 - quality, cost: 1e-18, volume: 100 });
    authority = st.authority;
    console.log(paint.dim(`  [doer=${res.doer} checker=${res.checker}] ${res.doerOutput}`));
    console.log(paint.dim(`  copilot verdict: ${res.verdict}${res.ok ? '' : ' — QUARANTINED'} | governor authority=${authority.toFixed(3)} (factor=${st.factorStatus}, resonance=${st.resonanceRisky ? 'RISKY' : 'ok'})`));
    if (!res.ok) process.exit(1);
    return;
  }

  if (cmd === 'node') {
    // Bebop node identity — encrypted-at-rest vault; a node keeps its PQ identity across restarts.
    const vaultPath = args.includes('--path') ? args[args.indexOf('--path') + 1] : path.resolve(HERE, '.bebop', 'node.vault.json');
    const pass = args.includes('--pass') ? args[args.indexOf('--pass') + 1] : 'bebop';
    const id = createOrUnlock(vaultPath, pass);
    console.log(paint.dim(`  node id=${id.id.slice(0, 24)}… (encrypted vault ${vaultPath})`));
    console.log(paint.dim(`  pqPublic=${Buffer.from(id.pqPublic).toString('hex').slice(0, 24)}… edPublic=${Buffer.from(id.edPublic).toString('hex').slice(0, 16)}…`));
    return;
  }

  if (cmd === 'govern') {
    // L5 telemetry governor applied LIVE to any agent/model/process (operator directive).
    // Feed a stream of quality samples; the servo computes math-proven authority (PID), factor
    // health (ICIR), resonance risk BEFORE any gain change, and anomaly signals (>3σ).
    // Usage:  bebop govern "0.9,0.6,0.2,0.9,0.95,0.1,..."   (comma/space separated 0..1)
    //         echo "0.9 0.6 0.2" | bebop govern              (stdin)
    const cfg = { kp: 1.4, ki: 0.22, kd: 1.5, iMin: -1, iMax: 1, uMin: 0, uMax: 1, targetQuality: 0.9, deadIC: 0.02, icirVolatile: 0.3, plantM: 1, plantB: 0.6, samplePeriod: 0, anomalyK: 3, maxStep: 1 };
    const gov = new Governor(cfg);
    let raw = args.join(' ').trim();
    if (!raw && !process.stdin.isTTY) {
      // read from stdin (sync, small inputs only)
      try { raw = require('node:fs').readFileSync(0, 'utf8'); } catch { raw = ''; }
    }
    const samples = raw.split(/[\s,]+/).map(Number).filter((n) => !Number.isNaN(n));
    if (samples.length === 0) {
      console.log(paint.dim('  usage: bebop govern "0.9,0.6,0.2,..."   (quality stream 0..1)'));
      console.log(paint.dim('  or:    echo "0.9 0.6 0.2" | bebop govern'));
      return;
    }
    console.log(paint.teal('  t  quality  authority  factor      resonance  anomaly'));
    let anomalies = 0;
    samples.forEach((q, t) => {
      // predicted = previous actual (a simple, honest predictor; ICIR measures its skill)
      const predicted = t > 0 ? samples[t - 1] : q;
      const st = gov.step({ t, predictedQuality: predicted, actualQuality: q, cost: 1e-18, volume: 100 });
      if (st.anomaly) anomalies++;
      const flag = st.anomaly ? paint.blood('ANOMALY') : 'ok';
      console.log(paint.dim(`  ${String(t).padStart(2)} ${q.toFixed(2)}     ${st.authority.toFixed(3)}     ${st.factorStatus.padEnd(9)}  ${st.resonanceRisky ? 'RISKY' : 'ok    '}    ${flag}`));
    });
    console.log(paint.dim(`  → ${samples.length} samples, ${anomalies} anomaly signal(s); final authority=${gov.authority.toFixed(3)}`));
    return;
  }

  if (cmd === 'self') {
    // Bebop soul: self-maintenance / self-evolution / session-as-node (fail-closed, recursive).
    const sub = args[0];
    if (sub === 'maintain' || !sub) {
      const h = selfMaintain();
      console.log(paint.dim(`  self-maintain ok=${h.ok} pass=${h.pass} fail=${h.fail}`));
    } else if (sub === 'evolve') {
      const idea = args.slice(1).join(' ');
      const r = selfEvolve(idea);
      console.log(paint.dim(`  self-evolve accepted=${r.accepted} reason=${r.reason}${r.id ? ' id=' + r.id.slice(0, 12) : ''}`));
    } else if (sub === 'session') {
      const id = recordSession({ id: args[1] ?? 'hermes-now', summary: args.slice(2).join(' ') || 'active hermes session node' });
      console.log(paint.dim(`  session recorded as living-memory node ${id.slice(0, 12)}`));
    } else if (sub === 'loop') {
      const r = selfLoop(args.slice(1).length ? args.slice(1) : ['tighten the copilot checker invariant']);
      console.log(paint.dim(`  self-loop health ok=${r.health.ok} evolutions=${JSON.stringify(r.evolutions)}`));
    } else {
      console.log(paint.blood('  usage: bebop self [maintain|evolve "<idea>"|session <id> <summary>|loop "<idea>"...]'));
    }
    return;
  }

  if (cmd === 'run') {
    const cls = (args[0] as TaskClass) ?? 'doer';
    const profile = loadProfile() ?? undefined;
    const res = await runLoop({ cwd: path.resolve(HERE, '..', '..'), taskClass: cls, profile });
    for (const line of res.transcript) console.log(line);
    console.log(paint.dim(`  steps=${res.steps} mutations=${res.mutations} denied=${res.denied} ok=${res.ok} envelopes=${res.log.length}`));
    if (!res.ok) process.exit(1);
    return;
  }

  console.log(paint.blood(`  unknown command: ${cmd}`));
  process.exit(2);
}

main().catch((e) => {
  const paint = makePaint();
  console.log(paint.blood('  fatal: ' + (e?.message ?? e)));
  process.exit(1);
});
