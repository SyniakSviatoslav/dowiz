import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import type { AdminOrder } from '@deliveryos/ui';
import { mergeDelta } from '../dashboard-utils.js';

const sampleOrder: AdminOrder = {
  id: 'abc-123',
  status: 'PENDING',
  total: 1500,
  createdAt: '2026-06-13T10:00:00Z',
  items: [{ name: 'Burger', quantity: 2 }],
  customerName: 'Sara',
  customerPhone: '+355691234567',
  shortId: '#ABC1',
  itemCount: 2,
  itemsSummary: '2×Burger',
  courierName: null,
};

function makePayload(overrides: Record<string, any> = {}) {
  return {
    orderId: 'abc-123',
    status: 'CONFIRMED',
    total: 1500,
    itemCount: 2,
    itemsSummary: '2×Burger',
    shortId: '#ABC1',
    courierName: null,
    ...overrides,
  };
}

describe('mergeDelta — order.created (isNew=true)', () => {
  it('prepends new order to empty list', () => {
    const result = mergeDelta([], makePayload({ orderId: 'new-1', status: 'PENDING', shortId: '#NEW1' }), true);
    assert.equal(result.length, 1);
    assert.equal(result[0]!.id, 'new-1');
    assert.equal(result[0]!.status, 'PENDING');
    assert.equal(result[0]!.shortId, '#NEW1');
  });

  it('deduplicates by id — does not add if exists', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ orderId: 'abc-123' }), true);
    assert.equal(result.length, 1);
    assert.equal(result, prev);
  });

  it('prepends new order ahead of existing', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ orderId: 'new-2', status: 'PENDING' }), true);
    assert.equal(result.length, 2);
    assert.equal(result[0]!.id, 'new-2');
    assert.equal(result[1]!.id, 'abc-123');
  });

  it('zero network requests — no fetch called', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ orderId: 'new-3' }), true);
    assert.equal(result.length, 2);
  });
});

describe('mergeDelta — order.status (isNew=false)', () => {
  it('updates status field by id', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ status: 'CONFIRMED' }), false);
    assert.equal(result.length, 1);
    assert.equal(result[0]!.status, 'CONFIRMED');
    assert.notEqual(result, prev);
  });

  it('updates courierName when present', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ status: 'IN_DELIVERY', courierName: 'Ardit' }), false);
    assert.equal(result[0]!.courierName, 'Ardit');
  });

  it('identity preservation — identical delta returns prev', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({
      status: 'PENDING',
      total: 1500,
      courierName: null,
      itemCount: 2,
      itemsSummary: '2×Burger',
      shortId: '#ABC1',
    }), false);
    assert.equal(result, prev);
  });

  it('identity preservation — skip when no field changed', () => {
    const prev = [sampleOrder];
    const payload = { orderId: 'abc-123', status: 'PENDING', courierName: null };
    const result = mergeDelta(prev, payload, false);
    assert.equal(result, prev);
  });

  it('does not modify orders not in list', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ orderId: 'nonexistent' }), false);
    assert.equal(result.length, 1);
    assert.equal(result, prev);
  });

  it('partial update — only changes specified fields', () => {
    const prev = [sampleOrder];
    const payload = { orderId: 'abc-123', status: 'CONFIRMED' };
    const result = mergeDelta(prev, payload, false);
    assert.equal(result[0]!.status, 'CONFIRMED');
    assert.equal(result[0]!.total, 1500);
    assert.equal(result[0]!.courierName, null);
  });
});

describe('mergeDelta — reconcile after reconnect', () => {
  it('full GET sync after status update', () => {
    const prev = [sampleOrder];
    const result = mergeDelta(prev, makePayload({ status: 'DELIVERED', courierName: 'Ardit' }), false);
    assert.equal(result[0]!.status, 'DELIVERED');
    assert.equal(result[0]!.courierName, 'Ardit');
  });

  it('drift correction — accepts valid forward transition', () => {
    const step1 = mergeDelta([sampleOrder], makePayload({ status: 'CONFIRMED' }), false);
    assert.equal(step1[0]!.status, 'CONFIRMED');
    const step2 = mergeDelta(step1, makePayload({ status: 'PREPARING' }), false);
    assert.equal(step2[0]!.status, 'PREPARING');
  });
});
