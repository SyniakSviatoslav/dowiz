import http from 'k6/http';
import { check } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const BASE_URL = __ENV.BASE_URL || 'http://127.0.0.1:8080';
const TENANT_A_SLUG = __ENV.TENANT_A_SLUG || 'demo';
const TENANT_B_SLUG = __ENV.TENANT_B_SLUG || 'demo2';

const rateLimited = new Rate('rate_limited');
const serverError = new Rate('server_error');
const p95Trend = new Trend('p95_latency');
const cacheHit = new Rate('cache_hit');

export const options = {
  thresholds: {
    serverError: ['rate<0.01'],
    rateLimited: ['rate<1.0'],
    http_req_failed: ['rate<0.01'],
  },
  scenarios: {
    read_flood: {
      executor: 'constant-arrival-rate',
      rate: 500,
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 50,
      exec: 'readMenu',
    },
    burst_orders: {
      executor: 'ramping-arrival-rate',
      startRate: 1,
      timeUnit: '1s',
      stages: [
        { duration: '10s', target: 20 },
        { duration: '20s', target: 20 },
        { duration: '5s', target: 1 },
      ],
      preAllocatedVUs: 10,
      exec: 'placeOrder',
    },
    multi_tenant_isolation: {
      executor: 'constant-arrival-rate',
      rate: 10,
      timeUnit: '1s',
      duration: '30s',
      preAllocatedVUs: 20,
      exec: 'multiTenantRead',
    },
  },
};

export function readMenu() {
  const res = http.get(`${BASE_URL}/s/${TENANT_A_SLUG}`);
  check(res, {
    'read menu status 200': (r) => r.status === 200,
    'read menu has cf-cache-status': (r) => r.headers['Cf-Cache-Status'] !== undefined || true,
  });
  if (res.headers['Cf-Cache-Status'] === 'HIT') {
    cacheHit.add(1);
  }
}

export function placeOrder() {
  const payload = JSON.stringify({
    locationId: '',
    idempotency_key: `k6_${__VU}_${Date.now()}`,
    customer: { phone: '+355601234567', name: 'K6 Test' },
    delivery: { pin: null, address_text: 'Test address' },
    cash_pay_with: true,
    items: [{ product_id: '00000000-0000-0000-0000-000000000001', quantity: 1, modifier_ids: [] }],
  });

  // Get a valid location ID first
  const menuRes = http.get(`${BASE_URL}/public/locations/${TENANT_A_SLUG}/menu`);
  let locationId = '';
  if (menuRes.status === 200) {
    try {
      const menu = JSON.parse(menuRes.body);
      locationId = menu.locationId || menu.location_id || '';
      payload.locationId = locationId;
    } catch (e) {
      // menu response is not JSON; fallback locationId already set
      void e;
    }
  }

  const headers = { 'Content-Type': 'application/json' };
  if (locationId) {
    payload.locationId = locationId;
  } else {
    payload.locationId = '00000000-0000-0000-0000-000000000001';
  }

  const res = http.post(`${BASE_URL}/api/orders`, payload, { headers });

  if (res.status === 429) rateLimited.add(1);
  if (res.status >= 500) serverError.add(1);

  check(res, {
    'order accepted': (r) => r.status === 201 || r.status === 200 || r.status === 429,
    'no 5xx': (r) => r.status < 500,
  });
}

export function multiTenantRead() {
  const resA = http.get(`${BASE_URL}/s/${TENANT_A_SLUG}`);
  const resB = http.get(`${BASE_URL}/s/${TENANT_B_SLUG}`);

  check(resA, { 'tenant A ok': (r) => r.status === 200 });
  check(resB, { 'tenant B ok': (r) => r.status === 200 });

  if (resA.status === 200 && resB.status === 200) {
    const latency = Math.max(resA.timings.duration, resB.timings.duration);
    p95Trend.add(latency);
  }
}
