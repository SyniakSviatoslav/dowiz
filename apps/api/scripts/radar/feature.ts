import { BASE_URL, RADAR_TIMEOUT } from './config.js';
import { type FlowEntry, type FlowStatus, type ProbeResult, type RadarReport, type Severity, formatReport } from './types.js';
import { loginLocal, loginMockOwner, apiGet } from './harness/auth.js';
import { getLocationInfo, getMenu, placeOrder, findFirstProduct } from './harness/order.js';
import { observeHealth, observeAudit, probeEndpoints } from './harness/observe.js';

export interface FeatureRadarOptions {
  baseUrl?: string;
  timeout?: number;
  flows: FlowEntry[];
}

async function runProbe(flow: FlowEntry): Promise<ProbeResult> {
  const start = Date.now();
  const result: ProbeResult = {
    flowId: flow.id,
    status: 'UNKNOWN',
    durationMs: 0,
    actual: {},
    divergence: [],
  };

  try {
    switch (flow.id) {
      // ─── Auth flows ───
      case 'auth-local-login': {
        const { status, body } = await apiGet('/api/auth/local/login');
        // Actually POST; get uses the get helper incorrectly for demonstration
        // Real probe does the actual login
        result.actual.method = 'POST /api/auth/local/login';
        result.actual.status = 'not implemented yet';
        result.status = 'BLOCKED';
        result.blockedReason = 'Probe needs to handle POST body differently';
        break;
      }

      // ─── Order flows ───
      case 'order-create': {
        // 1. Get location info
        const info = await getLocationInfo('demo');
        if (!info?.id) {
          result.status = 'BLOCKED';
          result.blockedReason = 'Demo location info not found';
          break;
        }
        result.actual.locationId = info.id;

        // 2. Get menu to find a product
        const menu = await getMenu(info.id);
        const product = await findFirstProduct(menu);
        if (!product) {
          result.status = 'BLOCKED';
          result.blockedReason = 'No available product in menu';
          break;
        }
        result.actual.product = product;

        // 3. Place order
        const order = await placeOrder(info.id, product.id);
        result.actual.order = order;

        // 4. Verify order shape
        const issues: ProbeResult['divergence'] = [];
        if (!order.id) issues.push({
          expected: 'order.id to be a UUID string',
          actual: `Got ${typeof order.id}: ${JSON.stringify(order.id)}`,
          evidence: 'order response body',
          severity: '🔴',
          rootCauseHypothesis: 'API did not return id in order response',
        });
        if (order.status !== 'PENDING') issues.push({
          expected: 'order.status === "PENDING"',
          actual: `Got "${order.status}"`,
          evidence: 'order response body',
          severity: '🔴',
          rootCauseHypothesis: 'order-status.ts default is not PENDING',
        });
        if (order.total <= 0) issues.push({
          expected: 'order.total > 0 (subtotal + delivery fee)',
          actual: `Got ${order.total}`,
          evidence: 'order response body',
          severity: '🟠',
          rootCauseHypothesis: 'Price calculation produced zero or negative total',
        });

        result.divergence = issues.length > 0 ? issues : undefined;
        result.status = issues.length > 0 ? 'DIVERGENCE' : 'OK';
        break;
      }

      // ─── Health flows ───
      case 'health-check': {
        const health = await observeHealth();
        result.actual.health = health;
        const issues: ProbeResult['divergence'] = [];

        if (health?.checks?.postgres?.status !== 'ok') issues.push({
          expected: 'postgres check = ok',
          actual: `Got "${health?.checks?.postgres?.status}"`,
          evidence: '/health response',
          severity: '🔴',
          rootCauseHypothesis: 'Database connection issue',
        });
        if (health?.checks?.telegram?.status !== 'ok') issues.push({
          expected: 'telegram check = ok',
          actual: `Got "${health?.checks?.telegram?.status}"`,
          evidence: '/health response',
          severity: '🟠',
          rootCauseHypothesis: 'Telegram bot API issue or token expired',
        });

        result.divergence = issues.length > 0 ? issues : undefined;
        result.status = issues.length > 0 ? 'DIVERGENCE' : 'OK';
        break;
      }

      default:
        result.status = 'BLOCKED';
        result.blockedReason = `No probe defined for flow "${flow.id}"`;
    }
  } catch (err: any) {
    result.status = 'DIVERGENCE';
    result.divergence = [{
      expected: `${flow.trigger} should succeed`,
      actual: `Threw: ${err.message}`,
      evidence: err.stack || err.message,
      severity: '🔴',
      rootCauseHypothesis: 'Probe execution error — check env/staging availability',
    }];
  }

  result.durationMs = Date.now() - start;
  return result;
}

export async function runFeatureRadar(options: FeatureRadarOptions): Promise<RadarReport> {
  if (options.baseUrl) process.env.RADAR_BASE_URL = options.baseUrl;

  // Auto-login with mock owner for all probes
  try {
    await loginMockOwner();
  } catch {
    // Fall back to local login
    await loginLocal('test@dowiz.com', 'test123456');
  }

  const results: ProbeResult[] = [];
  for (const flow of options.flows) {
    console.log(`[Radar] Probing ${flow.id}...`);
    const result = await runProbe(flow);
    results.push(result);
    const icon = result.status === 'OK' ? '✅' : result.status === 'DIVERGENCE' ? '🔴' : '⚪';
    console.log(`  ${icon} ${result.status} (${result.durationMs}ms)`);
  }

  const report: RadarReport = {
    timestamp: new Date().toISOString(),
    target: options.baseUrl || BASE_URL,
    total: results.length,
    ok: results.filter(r => r.status === 'OK').length,
    divergences: results.filter(r => r.status === 'DIVERGENCE').length,
    blocked: results.filter(r => r.status === 'BLOCKED').length,
    results,
  };

  return report;
}

// CLI entry point
const isMain = process.argv[1]?.replace(/\\/g, '/').endsWith('radar/feature.ts');
if (isMain) {
  const flowId = process.argv[2] || 'health-check';
  const flow: FlowEntry = {
    id: flowId,
    description: flowId,
    trigger: 'CLI',
    expectedEffects: [],
  };

  runFeatureRadar({ flows: [flow] }).then(report => {
    console.log(formatReport(report));
    process.exit(report.divergences > 0 ? 1 : 0);
  }).catch(err => {
    console.error('Radar failed:', err);
    process.exit(1);
  });
}
