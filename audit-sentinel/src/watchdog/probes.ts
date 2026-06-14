import { type Env } from '../config.js';

export interface ProbeResult {
  name: string;
  spec_ref: string;
  passed: boolean;
  detail: string;
  evidence: string;
  severity: 'BLOCKER' | 'MAJOR' | 'MINOR' | 'INFO';
  timestamp: string;
}

export async function probeTls(env: Env): Promise<ProbeResult> {
  const domains = [new URL(env.BASE_URL).hostname];
  if (env.SITE_URL) domains.push(new URL(env.SITE_URL).hostname);
  if (env.MENU_URL) domains.push(new URL(env.MENU_URL).hostname);

  const results: string[] = [];
  let allPassed = true;

  for (const domain of domains) {
    try {
      const resp = await fetch(`https://${domain}/`, { method: 'HEAD', signal: AbortSignal.timeout(5000) });
      results.push(`${domain}: TLS OK (status ${resp.status})`);
    } catch (e: any) {
      allPassed = false;
      results.push(`${domain}: TLS FAILED - ${e.message || 'unknown error'}`);
    }
  }

  return {
    name: 'TLS check',
    spec_ref: 'E1',
    passed: allPassed,
    detail: allPassed ? 'All domains have valid TLS' : 'One or more domains TLS failed',
    evidence: results.join('; '),
    severity: allPassed ? 'INFO' : 'BLOCKER',
    timestamp: new Date().toISOString(),
  };
}

export async function probeHeaders(env: Env): Promise<ProbeResult[]> {
  const url = env.BASE_URL.replace(/\/$/, '');
  const results: ProbeResult[] = [];
  const timestamp = new Date().toISOString();

  try {
    const httpResp = await fetch(url.replace('https://', 'http://'), { method: 'HEAD', redirect: 'manual', signal: AbortSignal.timeout(5000) });
    const isRedirect = httpResp.status === 301 || httpResp.status === 308;
    results.push({
      name: 'HTTP→HTTPS redirect',
      spec_ref: 'E2a',
      passed: isRedirect,
      detail: isRedirect ? 'OK' : `Expected redirect, got ${httpResp.status}`,
      evidence: `Status: ${httpResp.status}`,
      severity: isRedirect ? 'INFO' : 'BLOCKER',
      timestamp,
    });
  } catch (e: any) {
    results.push({
      name: 'HTTP→HTTPS redirect',
      spec_ref: 'E2a',
      passed: false,
      detail: `Failed: ${e.message}`,
      evidence: e.message || 'error',
      severity: 'BLOCKER',
      timestamp,
    });
  }

  try {
    const resp = await fetch(url, { method: 'HEAD', signal: AbortSignal.timeout(5000) });
    const hsts = resp.headers.get('strict-transport-security');
    const xcto = resp.headers.get('x-content-type-options');
    const xfo = resp.headers.get('x-frame-options');
    const csp = resp.headers.get('content-security-policy');
    const setCookie = resp.headers.get('set-cookie');

    results.push({
      name: 'HSTS header',
      spec_ref: 'E2b',
      passed: !!hsts,
      detail: hsts ? `HSTS present: ${hsts.substring(0, 60)}` : 'HSTS missing',
      evidence: hsts || 'absent',
      severity: hsts ? 'INFO' : 'BLOCKER',
      timestamp,
    });

    const hasSecurity = csp || xcto || xfo;
    results.push({
      name: 'Security headers',
      spec_ref: 'E2c',
      passed: !!hasSecurity,
      detail: hasSecurity
        ? `CSP:${!!csp}, X-CTO:${!!xcto}, X-FO:${!!xfo}`
        : 'All security headers missing on SPA root',
      evidence: `CSP:${csp || 'none'}, X-CTO:${xcto || 'none'}, X-FO:${xfo || 'none'}`,
      severity: hasSecurity ? 'INFO' : 'BLOCKER',
      timestamp,
    });

    results.push({
      name: 'No cookies',
      spec_ref: 'E2e',
      passed: !setCookie,
      detail: setCookie ? `COOKIES SET: ${setCookie}` : 'Zero cookies (OK)',
      evidence: setCookie || 'none',
      severity: setCookie ? 'BLOCKER' : 'INFO',
      timestamp,
    });

    const cc = resp.headers.get('cache-control') || '';
    results.push({
      name: 'Asset caching (root)',
      spec_ref: 'E4c',
      passed: !cc.includes('max-age=0') || url.includes('/s/'),
      detail: cc ? `Cache-Control: ${cc}` : 'No cache header',
      evidence: cc || 'absent',
      severity: cc.includes('max-age=0') ? 'MAJOR' : 'INFO',
      timestamp,
    });
  } catch (e: any) {
    results.push({
      name: 'HTTPS headers probe',
      spec_ref: 'E2',
      passed: false,
      detail: `Failed to fetch: ${e.message}`,
      evidence: e.message || 'error',
      severity: 'BLOCKER',
      timestamp,
    });
  }

  return results;
}

export async function probeHealth(env: Env): Promise<ProbeResult> {
  const url = `${env.BASE_URL.replace(/\/$/, '')}/health`;
  const timestamp = new Date().toISOString();

  try {
    const resp = await fetch(url, { signal: AbortSignal.timeout(10000) });
    const body = await resp.json();

    const pgOk = body.checks?.postgres?.status === 'ok';
    const workersOk = body.checks?.workers?.status === 'ok';
    const isDegraded = body.status === 'degraded';
    const isUnhealthy = body.status === 'unhealthy';

    const checks: string[] = [];
    if (!pgOk) checks.push('Postgres DOWN');
    if (!workersOk) checks.push('Workers DOWN');
    for (const [k, v] of Object.entries<any>(body.checks || {})) {
      if (v?.status === 'degraded') checks.push(`${k}: degraded`);
    }

    return {
      name: '/health check',
      spec_ref: 'E3',
      passed: resp.status === 200 && !isUnhealthy,
      detail: isDegraded ? `Degraded: ${checks.join(', ')}` : isUnhealthy ? 'UNHEALTHY' : 'Healthy',
      evidence: JSON.stringify({ status: body.status, degraded: checks }),
      severity: isUnhealthy ? 'BLOCKER' : isDegraded ? 'MAJOR' : 'INFO',
      timestamp,
    };
  } catch (e: any) {
    return {
      name: '/health check',
      spec_ref: 'E3',
      passed: false,
      detail: `Health endpoint failed: ${e.message}`,
      evidence: e.message || 'error',
      severity: 'BLOCKER',
      timestamp,
    };
  }
}

export async function probeCache(env: Env): Promise<ProbeResult> {
  if (!env.MENU_URL) {
    return { name: 'Cache probe', spec_ref: 'E4', passed: true, detail: 'Skipped (no MENU_URL)', evidence: 'N/A', severity: 'INFO', timestamp: new Date().toISOString() };
  }

  const timestamp = new Date().toISOString();
  try {
    const resp1 = await fetch(env.MENU_URL, { signal: AbortSignal.timeout(10000) });
    const cc = resp1.headers.get('cache-control') || '';
    const menuVer = resp1.headers.get('x-menu-version') || 'none';

    return {
      name: 'Cache headers (menu)',
      spec_ref: 'E4b',
      passed: cc.includes('max-age') && menuVer !== 'none',
      detail: `Cache: ${cc}, Menu version: ${menuVer}`,
      evidence: `Cache-Control: ${cc}, x-menu-version: ${menuVer}`,
      severity: 'INFO',
      timestamp,
    };
  } catch (e: any) {
    return {
      name: 'Cache probe',
      spec_ref: 'E4',
      passed: false,
      detail: `Menu URL failed: ${e.message}`,
      evidence: e.message || 'error',
      severity: 'MAJOR',
      timestamp,
    };
  }
}

export async function probeRateLimit(env: Env): Promise<ProbeResult> {
  if (!env.MENU_URL) {
    return { name: 'Rate limit', spec_ref: 'E5', passed: true, detail: 'Skipped (no MENU_URL)', evidence: 'N/A', severity: 'INFO', timestamp: new Date().toISOString() };
  }

  const timestamp = new Date().toISOString();
  try {
    const resp = await fetch(env.MENU_URL, { signal: AbortSignal.timeout(10000) });
    const limit = resp.headers.get('x-ratelimit-limit');
    const remaining = resp.headers.get('x-ratelimit-remaining');

    return {
      name: 'Rate limit headers',
      spec_ref: 'E5',
      passed: !!limit,
      detail: limit ? `Limit: ${limit}, Remaining: ${remaining}` : 'No rate-limit headers',
      evidence: `x-ratelimit-limit: ${limit || 'none'}, x-ratelimit-remaining: ${remaining || 'none'}`,
      severity: limit ? 'INFO' : 'MAJOR',
      timestamp,
    };
  } catch (e: any) {
    return {
      name: 'Rate limit probe',
      spec_ref: 'E5',
      passed: false,
      detail: `Failed: ${e.message}`,
      evidence: e.message || 'error',
      severity: 'MAJOR',
      timestamp,
    };
  }
}
