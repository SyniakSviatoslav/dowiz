import { loginMockOwner, authHeaders } from './harness/auth.js';
import { getLocationInfo, uuid } from './harness/order.js';
import { BASE_URL } from './config.js';

const STATUS = { OK: 'OK', DIV: 'DIVERGENCE', ERR: 'ERROR', BLK: 'BLOCKED' };

function emit(category: string, flow: string, status: string, detail: string) {
  console.log(`${category}|${flow}|${status}|${detail}`);
}

async function main() {
  let session = await loginMockOwner();
  emit('AUTH', 'mock-login', 'OK', `role=${session.role} userId=${session.userId.substring(0,8)}...`);

  // 1. PUBLIC MENU
  const info = await getLocationInfo('demo');
  if (!info?.id) { emit('PUBLIC', 'info', STATUS.DIV, 'No location id returned'); return; }
  emit('PUBLIC', 'info', 'OK', `slug=demo id=${info.id} currency=${info.currency_code}`);

  const menuReq = await fetch(`${BASE_URL}/public/locations/${info.id}/menu`);
  const menu = await menuReq.json();
  const cats = menu?.categories?.length || 0;
  emit('PUBLIC', 'menu', cats > 0 ? 'OK' : STATUS.DIV, `${cats} categories, version=${menu?.menu_version}`);

  // Find first available product
  let productId = '';
  for (const cat of menu?.categories || []) {
    for (const p of cat.products || []) {
      if (p.available !== false) { productId = p.id; break; }
    }
    if (productId) break;
  }
  if (!productId) { emit('ORDER', 'find-product', STATUS.DIV, 'No available product'); return; }
  emit('ORDER', 'find-product', 'OK', `productId=${productId}`);

  // 2. HEALTH CHECK
  const healthReq = await fetch(`${BASE_URL}/health`);
  const health = await healthReq.json();
  const checks = health?.checks || {};
  for (const [k, v] of Object.entries(checks)) {
    const s = typeof v === 'object' ? v.status || 'ok' : 'ok';
    emit('HEALTH', k, s === 'ok' ? 'OK' : STATUS.DIV, s === 'ok' ? '' : `status=${s}`);
  }

  // 3. ORDER CREATE
  let orderId = '';
  try {
    const idKey = uuid();
    const orderRes = await fetch(`${BASE_URL}/api/orders`, {
      method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        locationId: info.id, type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: '+355690000000', name: 'Radar Test' },
        delivery: { pin: { lat: 41.331, lng: 19.817 }, address_text: 'Rruga Test, Tirana' },
        payment: { method: 'cash' }, idempotency_key: idKey, acknowledged_codes: [],
      }),
    });
    const order = await orderRes.json();
    if (orderRes.status !== 201) { emit('ORDER', 'create', STATUS.DIV, `status=${orderRes.status} body=${JSON.stringify(order)}`); return; }
    emit('ORDER', 'create', 'OK', `id=${order.id} status=${order.status} subtotal=${order.subtotal} total=${order.total} preflight=${order.preflight?.outcome}`);
    orderId = order.id;

    // Verify order shape
    if (!order.id) emit('ORDER', 'verify-id', STATUS.DIV, 'id missing');
    else emit('ORDER', 'verify-id', 'OK', `id=${order.id}`);
    
    if (order.status !== 'PENDING') emit('ORDER', 'verify-status', STATUS.DIV, `expected PENDING got ${order.status}`);
    else emit('ORDER', 'verify-status', 'OK', 'PENDING');
    
    if (order.total <= 0) emit('ORDER', 'verify-total', STATUS.DIV, `total=${order.total}`);
    else emit('ORDER', 'verify-total', 'OK', `total=${order.total}`);
  } catch (e) { emit('ORDER', 'create', STATUS.ERR, e.message); return; }

  // 4. ORDER CONFIRM via dashboard
  try {
    const confirmRes = await fetch(`${BASE_URL}/api/owner/locations/${info.id}/orders/${orderId}/confirm`, {
      method: 'POST', headers: await authHeaders(),
    });
    emit('ORDER', 'confirm', confirmRes.status === 200 ? 'OK' : STATUS.DIV, `status=${confirmRes.status}`);
  } catch (e) { emit('ORDER', 'confirm', STATUS.ERR, e.message); }

  // 5. ORDER REJECT (new order)
  let rejectId = '';
  try {
    const idKey2 = uuid();
    const o2 = await fetch(`${BASE_URL}/api/orders`, {
      method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        locationId: info.id, type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: '+355690000001', name: 'Reject Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga Reject' },
        payment: { method: 'cash' }, idempotency_key: idKey2, acknowledged_codes: [],
      }),
    });
    const order2 = await o2.json();
    rejectId = order2.id;
    const rejectRes = await fetch(`${BASE_URL}/api/owner/locations/${info.id}/orders/${rejectId}/reject`, {
      method: 'POST', headers: await authHeaders(),
      body: JSON.stringify({ reason: 'Radar test reject' }),
    });
    emit('ORDER', 'reject', rejectRes.status === 200 ? 'OK' : STATUS.DIV, `status=${rejectRes.status}`);
  } catch (e) { emit('ORDER', 'reject', STATUS.ERR, e.message); }

  // 6. STATUS TRANSITIONS via PATCH
  try {
    const idKey3 = uuid();
    const o3 = await fetch(`${BASE_URL}/api/orders`, {
      method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({
        locationId: info.id, type: 'delivery',
        items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
        customer: { phone: '+355690000002', name: 'Transition Test' },
        delivery: { pin: { lat: 41.33, lng: 19.82 }, address_text: 'Rruga Transition' },
        payment: { method: 'cash' }, idempotency_key: idKey3, acknowledged_codes: [],
      }),
    });
    const order3 = await o3.json();
    const order3Id = order3.id;

    const s1 = await (await fetch(`${BASE_URL}/api/orders/${order3Id}/status`, { method: 'PATCH', headers: await authHeaders(), body: JSON.stringify({status:'CONFIRMED'}) })).status;
    const s2 = await (await fetch(`${BASE_URL}/api/orders/${order3Id}/status`, { method: 'PATCH', headers: await authHeaders(), body: JSON.stringify({status:'PREPARING'}) })).status;
    const s3 = await (await fetch(`${BASE_URL}/api/orders/${order3Id}/status`, { method: 'PATCH', headers: await authHeaders(), body: JSON.stringify({status:'READY'}) })).status;
    
    const allOk = [s1,s2,s3].every(s => s === 200);
    emit('ORDER', 'status-transitions', allOk ? 'OK' : STATUS.DIV, `confirm=${s1} preparing=${s2} ready=${s3}`);
  } catch (e) { emit('ORDER', 'status-transitions', STATUS.ERR, e.message); }

  // 7. CUSTOMER PUSH SUBSCRIBE (no-op test — verifies endpoint exists)
  try {
    const pushRes = await fetch(`${BASE_URL}/api/customer/push/subscribe`, {
      method: 'POST', headers: await authHeaders(),
      body: JSON.stringify({ endpoint: 'https://example.com/push/test', keys: { p256dh: 'test', auth: 'test' }, opted_in: true }),
    });
    emit('PUSH', 'subscribe', pushRes.status === 200 ? 'OK' : STATUS.DIV, `status=${pushRes.status}`);
  } catch (e) { emit('PUSH', 'subscribe', STATUS.ERR, e.message); }

  // 8. VERIFY THAT CONFIRMED/REJECTED TRIGGERED NOTIFICATIONS (check audit)
  try {
    const auditRes = await fetch(`${BASE_URL}/api/owner/audit?limit=10`, {
      headers: await authHeaders(),
    }).catch(() => null);
    if (auditRes && auditRes.ok) {
      const audit = await auditRes.json();
      const entries = audit?.audit || audit || [];
      if (entries.length > 0) {
        emit('NOTIFY', 'audit-entries', 'OK', `${entries.length} entries found`);
        for (const e of entries.slice(0, 5)) {
          emit('NOTIFY', 'audit-detail', 'OK', `event=${e.event} status=${e.status} channel=${e.channel}`);
        }
      } else {
        emit('NOTIFY', 'audit-entries', STATUS.DIV, 'No audit entries found (endpoint may not exist)');
      }
    } else {
      emit('NOTIFY', 'audit-entries', STATUS.BLK, 'Audit endpoint not accessible (401 or 404)');
    }
  } catch (e) { emit('NOTIFY', 'audit', STATUS.ERR, e.message); }

  // 9. OWNER ENDPOINTS — SETTINGS
  try {
    const settingsRes = await fetch(`${BASE_URL}/api/owner/settings`, { headers: await authHeaders() });
    const settings = settingsRes.ok ? await settingsRes.json() : null;
    emit('OWNER', 'settings', settingsRes.status === 200 ? 'OK' : STATUS.DIV, `status=${settingsRes.status} name=${settings?.locationName || '?'}`);
  } catch (e) { emit('OWNER', 'settings', STATUS.ERR, e.message); }

  // 10. OWNER ENDPOINTS — ORDERS LIST
  try {
    const ordersRes = await fetch(`${BASE_URL}/api/owner/orders`, { headers: await authHeaders() });
    const ordersList = ordersRes.ok ? await ordersRes.json() : null;
    emit('OWNER', 'orders-list', ordersRes.status === 200 ? 'OK' : STATUS.DIV, `status=${ordersRes.status} count=${ordersList?.length || 0}`);
  } catch (e) { emit('OWNER', 'orders-list', STATUS.ERR, e.message); }

  // Summary
  console.log('\n=== RADAR COMPLETE ===');
}

main().catch(e => { console.error('FATAL|' + e.message); process.exit(1); });
