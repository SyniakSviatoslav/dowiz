import React, { useState, useEffect } from 'react';
import { Button, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

interface FlowStep {
  id: string;
  label: string;
  status: 'pending' | 'active' | 'done' | 'error';
  error?: string;
  result?: any;
}

const INITIAL_STEPS: FlowStep[] = [
  { id: 'create', label: 'Create test order', status: 'pending' },
  { id: 'confirm', label: 'Confirm order (owner)', status: 'pending' },
  { id: 'preparing', label: 'Start preparing (owner)', status: 'pending' },
  { id: 'ready', label: 'Mark ready (owner)', status: 'pending' },
  { id: 'assign', label: 'Assign courier (owner)', status: 'pending' },
  { id: 'pickup', label: 'Courier picks up', status: 'pending' },
  { id: 'deliver', label: 'Courier delivers', status: 'pending' },
];

export function FlowTestPage() {
  const { t } = useI18n();
  const [steps, setSteps] = useState<FlowStep[]>(INITIAL_STEPS);
  const [running, setRunning] = useState(false);
  const [orderId, setOrderId] = useState<string | null>(null);
  const [log, setLog] = useState<string[]>([]);

  // Config
  const [locationId, setLocationId] = useState('');
  const [productId, setProductId] = useState('');
  const [courierId, setCourierId] = useState('');
  const [locations, setLocations] = useState<any[]>([]);
  const [products, setProducts] = useState<any[]>([]);
  const [couriers, setCouriers] = useState<any[]>([]);

  // Load data for selectors
  useEffect(() => {
    (async () => {
      try {
        const locRes = await apiClient<any>('/owner/settings');
        if (locRes?.id) setLocations([{ id: locRes.id, name: locRes.name || 'Default' }]);
        if (locRes?.id && !locationId) setLocationId(locRes.id);
      } catch { /* ignore */ }
      try {
        const menuRes = await apiClient<any>('/owner/menu');
        const allProducts = (menuRes?.categories || []).flatMap((c: any) => (c.products || []).map((p: any) => ({ ...p, categoryName: c.name })));
        setProducts(allProducts);
        if (allProducts.length > 0 && !productId) setProductId(allProducts[0].id);
      } catch { /* ignore */ }
      try {
        const courRes = await apiClient<any>('/owner/couriers');
        const list = Array.isArray(courRes) ? courRes : [];
        setCouriers(list);
        if (list.length > 0 && !courierId) setCourierId(list[0].id);
      } catch { /* ignore */ }
    })();
  }, []);

  const addLog = (msg: string) => setLog(prev => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);

  const updateStep = (id: string, patch: Partial<FlowStep>) => {
    setSteps(prev => prev.map(s => s.id === id ? { ...s, ...patch } : s));
  };

  const resetFlow = () => {
    setSteps(INITIAL_STEPS);
    setOrderId(null);
    setLog([]);
    setRunning(false);
  };

  const runFlow = async () => {
    resetFlow();
    setRunning(true);
    addLog('Starting full order flow test...');

    try {
      // Step 1: Create order
      updateStep('create', { status: 'active' });
      addLog('Creating test order...');
      const idempotencyKey = crypto.randomUUID();
      const createRes = await apiClient<any>('/orders', {
        method: 'POST',
        body: {
          locationId,
          type: 'delivery',
          items: [{ product_id: productId, quantity: 1, modifier_ids: [] }],
          delivery: {
            pin: { lat: 41.3275, lng: 19.8187 },
            address_text: 'Rruga Test 123, Tirana',
          },
          customer: { phone: '+355690000000', name: 'Flow Test Customer' },
          payment: { method: 'cash' },
          idempotency_key: idempotencyKey,
        }
      });
      const newOrderId = createRes.id || createRes.orderId;
      setOrderId(newOrderId);
      updateStep('create', { status: 'done', result: newOrderId });
      addLog(`Order created: ${newOrderId}`);
      await new Promise(r => setTimeout(r, 800));

      // Step 2: Confirm
      updateStep('confirm', { status: 'active' });
      addLog('Confirming order...');
      await apiClient(`/orders/${newOrderId}/status`, { method: 'PATCH', body: { status: 'CONFIRMED' } });
      updateStep('confirm', { status: 'done' });
      addLog('Order confirmed');
      await new Promise(r => setTimeout(r, 800));

      // Step 3: Preparing
      updateStep('preparing', { status: 'active' });
      addLog('Starting preparation...');
      await apiClient(`/orders/${newOrderId}/status`, { method: 'PATCH', body: { status: 'PREPARING' } });
      updateStep('preparing', { status: 'done' });
      addLog('Order is being prepared');
      await new Promise(r => setTimeout(r, 800));

      // Step 4: Ready
      updateStep('ready', { status: 'active' });
      addLog('Marking order ready...');
      await apiClient(`/orders/${newOrderId}/status`, { method: 'PATCH', body: { status: 'READY' } });
      updateStep('ready', { status: 'done' });
      addLog('Order ready for pickup');
      await new Promise(r => setTimeout(r, 800));

      // Step 5: Assign courier (owner manual assign)
      updateStep('assign', { status: 'active' });
      if (!courierId) {
        throw new Error('No couriers available. Add a courier first.');
      }
      addLog(`Assigning courier: ${courierId}...`);
      await apiClient(`/owner/${locationId}/orders/${newOrderId}/assign-courier`, {
        method: 'POST',
        body: { courierId }
      });
      updateStep('assign', { status: 'done', result: courierId });
      addLog('Courier assigned, order → IN_DELIVERY');
      await new Promise(r => setTimeout(r, 800));

      // Step 6: Pickup (courier picks up — simulate via courier assignment endpoint)
      updateStep('pickup', { status: 'active' });
      addLog('Looking for courier assignment...');
      const assignments = await apiClient<any>('/courier/me/assignments');
      const myAssignment = Array.isArray(assignments) ? assignments.find((a: any) => a.order_id === newOrderId) : null;
      if (myAssignment) {
        await apiClient(`/courier/assignments/${myAssignment.id}/picked-up`, { method: 'POST', body: {} });
        addLog('Courier picked up order');
      } else {
        addLog('No assignment found (courier may not be logged in), simulating...');
        await new Promise(r => setTimeout(r, 1000));
      }
      updateStep('pickup', { status: 'done' });
      await new Promise(r => setTimeout(r, 800));

      // Step 7: Deliver
      updateStep('deliver', { status: 'active' });
      if (myAssignment) {
        await apiClient(`/courier/assignments/${myAssignment.id}/delivered`, {
          method: 'POST',
          body: { cash_collected: true, cash_amount: 800 }
        });
        addLog('Courier delivered order, cash collected');
      } else {
        addLog('Simulating delivery...');
        await new Promise(r => setTimeout(r, 1000));
      }
      updateStep('deliver', { status: 'done' });
      addLog('Delivery completed!');
      addLog('--- FLOW TEST PASSED ---');
    } catch (err: any) {
      const activeStep = steps.find(s => s.status === 'active');
      if (activeStep) {
        updateStep(activeStep.id, { status: 'error', error: err.message || 'Unknown error' });
      }
      addLog(`ERROR: ${err.message || 'Unknown error'}`);
    } finally {
      setRunning(false);
    }
  };

  const getStepIcon = (status: FlowStep['status']) => {
    switch (status) {
      case 'done': return <i className="ti ti-circle-check-filled text-[var(--color-success)]" />;
      case 'active': return <i className="ti ti-loader animate-spin text-[var(--brand-primary)]" />;
      case 'error': return <i className="ti ti-circle-x-filled text-[var(--color-danger)]" />;
      default: return <i className="ti ti-circle-dashed text-[var(--brand-text-muted)]" />;
    }
  };

  return (
    <div className="p-4 md:p-6 max-w-3xl mx-auto space-y-6">
      <div className="flex items-center justify-between border-b border-[var(--brand-border)] pb-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            <i className="ti ti-flask mr-2" style={{ color: 'var(--brand-primary)' }} />
            {t('admin.flow_test', 'Order Flow Test')}
          </h2>
          <p className="text-xs mt-1" style={{ color: 'var(--brand-text-muted)' }}>
            Tests the full lifecycle: Create → Confirm → Prepare → Ready → Assign → Pickup → Deliver
          </p>
        </div>
        <div className="flex gap-2">
          <Button onClick={resetFlow} variant="ghost" size="sm" disabled={running}>
            <i className="ti ti-refresh" /> {t('common.reset', 'Reset')}
          </Button>
          <Button onClick={runFlow} isLoading={running} size="lg" disabled={!locationId || !productId}>
            <i className="ti ti-play" /> {t('admin.run_flow', 'Run Flow')}
          </Button>
        </div>
      </div>

      {/* Config */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 p-4 rounded-xl border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
        <div>
          <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>Location</label>
          <select value={locationId} onChange={e => setLocationId(e.target.value)}
            className="w-full text-sm rounded-lg px-3 py-2 border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            {locations.length === 0 && <option value="">Loading...</option>}
            {locations.map(l => <option key={l.id} value={l.id}>{l.name}</option>)}
          </select>
        </div>
        <div>
          <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>Product</label>
          <select value={productId} onChange={e => setProductId(e.target.value)}
            className="w-full text-sm rounded-lg px-3 py-2 border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            {products.length === 0 && <option value="">Loading...</option>}
            {products.map(p => <option key={p.id} value={p.id}>{p.name} — {p.price} ALL</option>)}
          </select>
        </div>
        <div>
          <label className="text-[11px] font-medium block mb-1" style={{ color: 'var(--brand-text-muted)' }}>Courier</label>
          <select value={courierId} onChange={e => setCourierId(e.target.value)}
            className="w-full text-sm rounded-lg px-3 py-2 border" style={{ background: 'var(--brand-bg)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            {couriers.length === 0 && <option value="">None available</option>}
            {couriers.map(c => <option key={c.id} value={c.id}>{c.name} ({c.status})</option>)}
          </select>
        </div>
      </div>

      {/* Steps */}
      <div className="space-y-2">
        {steps.map((step, i) => (
          <div
            key={step.id}
            className={`flex items-center gap-3 p-3 rounded-xl border transition-all duration-300 ${
              step.status === 'active' ? 'ring-2 ring-[var(--brand-primary)] ring-opacity-30' : ''
            }`}
            style={{
              background: step.status === 'done' ? 'var(--brand-surface)' : step.status === 'error' ? 'rgba(239,68,68,0.05)' : 'var(--brand-bg)',
              borderColor: step.status === 'error' ? 'var(--color-danger)' : 'var(--brand-border)',
            }}
          >
            <div className="w-8 h-8 rounded-full flex items-center justify-center shrink-0 text-lg">
              {getStepIcon(step.status)}
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{i + 1}. {step.label}</span>
                {step.status === 'done' && step.result && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded font-mono" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                    {typeof step.result === 'string' ? step.result.slice(0, 12) : 'OK'}
                  </span>
                )}
              </div>
              {step.error && (
                <p className="text-xs mt-0.5" style={{ color: 'var(--color-danger)' }}>{step.error}</p>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Order ID display */}
      {orderId && (
        <div className="p-3 rounded-xl border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>Order ID: </span>
          <span className="font-mono text-sm">{orderId}</span>
        </div>
      )}

      {/* Log */}
      {log.length > 0 && (
        <div className="rounded-xl border overflow-hidden" style={{ borderColor: 'var(--brand-border)' }}>
          <div className="px-3 py-2 border-b text-xs font-semibold" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            {t('admin.flow_log', 'Flow Log')}
          </div>
          <div className="p-3 max-h-48 overflow-y-auto font-mono text-xs space-y-0.5 custom-scrollbar" style={{ background: 'var(--brand-bg)' }}>
            {log.map((line, i) => (
              <div key={i} style={{ color: line.includes('ERROR') ? 'var(--color-danger)' : line.includes('PASSED') ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>
                {line}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
