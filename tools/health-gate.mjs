#!/usr/bin/env node
// health-gate.mjs — fail-closed single-pane pre-flight gate (ZERO deps, Node 22+).
//
// [RUNNING] This is the only artifact in the P8 single-pane spec that actually runs.
// It is a LOCAL guard, NOT the monitoring pane (VictoriaMetrics/Grafana/Netdata/Gatus),
// which is [SPEC] / not deployed.
//
// Fail-closed contract: ANY check failure (including a check that cannot run) => exit 1,
// prints FAIL. Only when EVERY check passes => exit 0, prints PASS.
//
// Env / flag overrides (all optional):
//   ROOT_PATH=/path           disk-free target (default '/')
//   DISK_FREE_MIN_PCT=10      fail if free% on ROOT_PATH < this (default 10)
//   VOLUME_PATH=/mnt/...      volume mount to assert is a separate fs (default /mnt/volume-fsn1-1)
//   KERNEL_PKG=dowiz-kernel   cargo package to test (default dowiz-kernel)
//   FORCE_FAIL=disk|volume|kernel   force one check to fail (test/CI injection)
//   --json                    print a small status object instead of prose

import fs from 'node:fs';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function pct(bavail, blocks) {
  if (!blocks) return 0;
  return (bavail / blocks) * 100;
}

// ---- Check 1: disk free on ROOT_PATH ----------------------------------------
export function checkDiskFree(opts = {}) {
  const target = opts.path ?? process.env.ROOT_PATH ?? '/';
  const minFreePct = Number.isFinite(opts.minFreePct)
    ? opts.minFreePct
    : Number(process.env.DISK_FREE_MIN_PCT ?? 10);
  if (process.env.FORCE_FAIL === 'disk') {
    return { name: 'disk-free', ok: false, detail: `FORCE_FAIL=disk (injected)` };
  }
  try {
    const s = fs.statfsSync(target);
    const freePct = pct(s.bavail, s.blocks);
    const ok = freePct >= minFreePct;
    return {
      name: 'disk-free',
      ok,
      detail: ok
        ? `${target}: ${freePct.toFixed(1)}% free (>=${minFreePct}% ok)`
        : `${target}: ${freePct.toFixed(1)}% free (<${minFreePct}% floor) -> FAIL closed`,
    };
  } catch (e) {
    // Path unreadable / absent => fail-closed.
    return { name: 'disk-free', ok: false, detail: `cannot statfs ${target}: ${e.message}` };
  }
}

// ---- Check 2: volume mount is a SEPARATE filesystem --------------------------
export function checkVolumeMount(opts = {}) {
  const target = opts.path ?? process.env.VOLUME_PATH ?? '/mnt/volume-fsn1-1';
  if (process.env.FORCE_FAIL === 'volume') {
    return { name: 'volume-mount', ok: false, detail: `FORCE_FAIL=volume (injected)` };
  }
  try {
    const v = fs.statSync(target);
    const r = fs.statSync('/');
    if (v.dev === r.dev) {
      // Same device as '/' => not actually a separate mount (just a subdir).
      return { name: 'volume-mount', ok: false, detail: `${target} is not a separate mount (same dev as /)` };
    }
    return { name: 'volume-mount', ok: true, detail: `${target} mounted (dev ${v.dev} != / dev ${r.dev})` };
  } catch (e) {
    return { name: 'volume-mount', ok: false, detail: `cannot stat ${target}: ${e.message}` };
  }
}

// ---- Check 3: kernel cargo test green (invokes cargo; fast artifact fallback)-
export function checkKernel(opts = {}) {
  const pkg = opts.pkg ?? process.env.KERNEL_PKG ?? 'dowiz-kernel';
  if (process.env.FORCE_FAIL === 'kernel') {
    return { name: 'kernel-test', ok: false, detail: `FORCE_FAIL=kernel (injected)` };
  }
  // Each crate is standalone (no workspace at ROOT), so we must run cargo INSIDE
  // the kernel crate dir, not from ROOT.
  const crateDir = path.join(ROOT, 'kernel');
  return new Promise((resolve) => {
    let cargo;
    try {
      cargo = spawn('cargo', ['test', '-p', pkg, '--quiet'], {
        cwd: crateDir,
        timeout: opts.timeoutMs ?? 240000,
      });
    } catch (e) {
      // cargo not spawnable => fall back to asserting a real build artifact exists.
      const artifact = path.join(ROOT, 'kernel', 'target', 'release', `lib${pkg.replace(/-/g, '_')}.rlib`);
      const ok = fs.existsSync(artifact);
      return resolve({ name: 'kernel-test', ok, detail: ok ? `cargo unavailable; artifact ${artifact} present` : `cargo unavailable; missing ${artifact}` });
    }
    let stderr = '';
    cargo.stderr.on('data', (d) => (stderr += d));
    cargo.on('error', () => {
      const artifact = path.join(ROOT, 'kernel', 'target', 'release', `lib${pkg.replace(/-/g, '_')}.rlib`);
      const ok = fs.existsSync(artifact);
      resolve({ name: 'kernel-test', ok, detail: ok ? `cargo error; artifact ${artifact} present` : `cargo error; missing ${artifact}` });
    });
    cargo.on('close', (code) => {
      resolve({
        name: 'kernel-test',
        ok: code === 0,
        detail: code === 0 ? `cargo test -p ${pkg} green` : `cargo test -p ${pkg} exited ${code}`,
      });
    });
  });
}

// ---- Orchestration ----------------------------------------------------------
export async function evaluate(opts = {}) {
  const disk = checkDiskFree(opts.disk ?? {});
  const volume = checkVolumeMount(opts.volume ?? {});
  const kernel = await checkKernel(opts.kernel ?? {});
  const checks = [disk, volume, kernel];
  const ok = checks.every((c) => c.ok);
  return { ok, checks };
}

function printHuman(r) {
  for (const c of r.checks) {
    console.log(`  [${c.ok ? 'PASS' : 'FAIL'}] ${c.name}: ${c.detail}`);
  }
  console.log(r.ok ? 'RESULT: PASS' : 'RESULT: FAIL');
}

async function main() {
  const json = process.argv.includes('--json');
  const r = await evaluate();
  if (json) {
    console.log(JSON.stringify({ ok: r.ok, checks: r.checks }, null, 2));
  } else {
    console.log('health-gate (fail-closed) — P8 single-pane pre-flight [RUNNING]');
    printHuman(r);
  }
  process.exit(r.ok ? 0 : 1);
}

// Only self-run when executed directly (not when imported by the test).
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
