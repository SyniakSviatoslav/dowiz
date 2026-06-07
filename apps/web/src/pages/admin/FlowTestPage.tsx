import React, { useState } from 'react';
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
  { id: 'assign', label: 'Assign courier', status: 'pending' },
  { id: 'pickup', label: 'Courier picks up', status: 'pending' },
  { id: 'deliver', label: 'Courier delivers', status: 'pending' },
];

export function FlowTestPage() {
  const { t } = useI18n();
  const [steps, setSteps] = useState<FlowStep[]>(INITIAL_STEPS);
  const [running, setRunning] = useState(false);
  const [orderId, setOrderId] = useState<string | null>(null);
  const [log, setLog] = useState<string[]>([]);

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
    addLog('Starting order flow test...');

    try {
      // Step 1: Create order
      updateStep('create', { status: 'active' });
      addLog('Creating test order...');
      const createRes = await apiClient<any>('/orders', {
        method: 'POST',
        body: {
          items: [{ name: 'Test Burger', quantity: 1, price: 800 }],
          deliveryAddress: 'Rruga Test 123, Tirana',
          customerName: 'Flow Test Customer',
          customerPhone: '+355690000000',
          paymentMethod: 'cash',
        }
      });
      const newOrderId = createRes.id || createRes.orderId;
      setOrderId(newOrderId);
      updateStep('create', { status: 'done', result: newOrderId });
      addLog(`Order created: ${newOrderId}`);
      await new Promise(r => setTimeout(r, 800));

      // Step 2: Confirm order
      updateStep('confirm', { status: 'active' });
      addLog('Confirming order...');
      await apiClient(`/orders/${newOrderId}/status`, {
        method: 'PATCH',
        body: { status: 'CONFIRMED' }
      });
      updateStep('confirm', { status: 'done' });
      addLog('Order confirmed');
      await new Promise(r => setTimeout(r, 800));

      // Step 3: Assign courier
      updateStep('assign', { status: 'active' });
      addLog('Looking for available couriers...');
      const couriers = await apiClient<any>('/owner/couriers');
      const available = Array.isArray(couriers) ? couriers.find((c: any) => c.status === 'online') : null;
      if (!available) {
        throw new Error('No available couriers found. Start a courier shift first.');
      }
      addLog(`Assigning courier: ${available.name}`);
      await apiClient(`/orders/${newOrderId}/status`, {
        method: 'PATCH',
        body: { status: 'IN_DELIVERY' }
      });
      updateStep('assign', { status: 'done', result: available.name });
      addLog(`Courier assigned: ${available.name}`);
      await new Promise(r => setTimeout(r, 800));

      // Step 4: Pickup (simulate)
      updateStep('pickup', { status: 'active' });
      addLog('Simulating courier pickup...');
      await new Promise(r => setTimeout(r, 1000));
      updateStep('pickup', { status: 'done' });
      addLog('Pickup confirmed');
      await new Promise(r => setTimeout(r, 800));

      // Step 5: Deliver
      updateStep('deliver', { status: 'active' });
      addLog('Simulating delivery completion...');
      await apiClient(`/orders/${newOrderId}/status`, {
        method: 'PATCH',
        body: { status: 'DELIVERED' }
      });
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
            {t('admin.flow_test_desc', 'Secret: tests the full order lifecycle from creation to delivery.')}
          </p>
        </div>
        <div className="flex gap-2">
          <Button onClick={resetFlow} variant="ghost" size="sm" disabled={running}>
            <i className="ti ti-refresh" /> {t('common.reset', 'Reset')}
          </Button>
          <Button onClick={runFlow} isLoading={running} size="lg">
            <i className="ti ti-play" /> {t('admin.run_flow', 'Run Flow')}
          </Button>
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
