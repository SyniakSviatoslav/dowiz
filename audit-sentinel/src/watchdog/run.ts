import { loadEnv } from '../config.js';
import { probeTls, probeHeaders, probeHealth, probeCache, probeRateLimit } from './probes.js';
import type { ProbeResult } from './probes.js';
import { loadBaseline, saveBaseline, diffFindings } from '../store/baseline.js';
import type { Finding } from '../store/findings.js';
import { makeFindingId } from '../store/findings.js';

function probeToFindings(results: ProbeResult[]): Finding[] {
  return results
    .filter((r) => !r.passed)
    .map((r) => ({
      id: makeFindingId('EDGE', r.name, r.spec_ref),
      layer: 'EDGE' as const,
      severity: (r.severity === 'INFO' ? 'MINOR' : r.severity) as 'BLOCKER' | 'MAJOR' | 'MINOR',
      target: r.name,
      expected: 'Pass',
      actual: r.detail,
      evidence: r.evidence,
      status: 'NEW' as const,
      first_seen: r.timestamp,
      last_seen: r.timestamp,
      spec_ref: r.spec_ref,
    }));
}

async function main() {
  console.log('=== Audit Sentinel · Tier-1 Watchdog ===');
  console.log(`Timestamp: ${new Date().toISOString()}`);

  const env = loadEnv();

  if (env.AUDIT_AGENT_ENABLED !== 'true') {
    console.log('AUDIT_AGENT_ENABLED is not true — exiting');
    process.exit(0);
  }

  console.log(`ENV: ${env.ENV} | BASE_URL: ${env.BASE_URL} | Tenant: ${env.TEST_TENANT}`);

  const allResults: ProbeResult[] = [];

  // E1: TLS
  const tls = await probeTls(env);
  allResults.push(tls);

  // E2: Headers + redirects + cookies
  const headers = await probeHeaders(env);
  allResults.push(...headers);

  // E3: Health endpoint
  const health = await probeHealth(env);
  allResults.push(health);

  // E4: Cache
  const cache = await probeCache(env);
  allResults.push(cache);

  // E5: Rate limit
  const rateLimit = await probeRateLimit(env);
  allResults.push(rateLimit);

  // Summary
  const passed = allResults.filter((r) => r.passed).length;
  const failed = allResults.filter((r) => !r.passed).length;
  const blockers = allResults.filter((r) => !r.passed && r.severity === 'BLOCKER').length;

  console.log(`\nResults: ${passed} passed, ${failed} failed (${blockers} blockers)`);
  for (const r of allResults) {
    console.log(`  [${r.passed ? '✓' : '✗'}] ${r.name}: ${r.detail}`);
  }

  // Diff against baseline
  const baseline = loadBaseline();
  const findings = probeToFindings(allResults);
  const { new_findings, regressed } = diffFindings(findings, baseline);

  // Update baseline
  const greenChecks = allResults.filter((r) => r.passed).map((r) => r.name);
  const redChecks = allResults.filter((r) => !r.passed).map((r) => r.name);

  const knownFindings: Record<string, Finding> = {};
  for (const f of [...(baseline?.known_findings ? Object.values(baseline.known_findings) : []), ...findings]) {
    knownFindings[f.id] = f;
  }

  saveBaseline({
    updated_at: new Date().toISOString(),
    env: env.ENV,
    base_url: env.BASE_URL,
    health_ok: allResults.find((r) => r.name === '/health check')?.passed || false,
    known_findings: knownFindings,
    green_checks: greenChecks,
    red_checks: redChecks,
  });

  // Escalate only new + regressed
  const toEscalate = [...new_findings, ...regressed];

  if (toEscalate.length > 0) {
    console.log(`\nEscalating ${toEscalate.length} findings:`);
    for (const f of toEscalate) {
      console.log(`  [${f.status}] [${f.severity}] ${f.target}: ${f.actual}`);
    }

    // Trigger Tier-2 via anomaly signal
    if (blockers > 0 || new_findings.length > 0) {
      console.log('\nAnomaly detected — would trigger Tier-2 auditor');
      console.log('Set GITHUB_TOKEN + GITHUB_REPO to auto-trigger via repository_dispatch');
    }

    // If prod + BLOCKER, exit non-zero for CI alerting
    if (env.ENV === 'prod' && blockers > 0) {
      console.error(`\n${blockers} BLOCKER(s) detected on prod — failing CI for visibility`);
      process.exitCode = 1;
    }
  } else {
    console.log('\nNo new findings — system stable');
  }

  console.log('\nWatchdog complete.');
}

main().catch((err) => {
  console.error('Watchdog fatal error:', err);
  process.exitCode = 2;
});
