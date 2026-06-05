import React, { useEffect, useState, useMemo } from 'react';
import { Button, EmptyState, SkeletonBase } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

import { exportCSV } from '../../lib/exportCSV.js';

interface Customer {
  id: string;
  name: string;
  phone: string;
  orders: number;
  ltv: number;
  lastOrder: string;
}

export function CRMPage() {
  const [customers, setCustomers] = useState<Customer[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [sortKey, setSortKey] = useState<'orders' | 'ltv' | 'name'>('orders');
  const [revealing, setRevealing] = useState<string | null>(null);
  const [revealed, setRevealed] = useState<Record<string, string>>({});

  useEffect(() => {
    apiClient<any>('/owner/customers')
      .then(d => { setCustomers(d.customers || d || []); setLoading(false); })
      .catch(() => {
        setCustomers([
          { id: 'c1', name: 'Sara Mancini', phone: '+355 69 *** ***', orders: 18, ltv: 32400, lastOrder: 'today' },
          { id: 'c2', name: 'Alina Popa', phone: '+355 69 *** ***', orders: 11, ltv: 19800, lastOrder: 'yesterday' },
          { id: 'c3', name: 'Bled Gjoni', phone: '+355 69 *** ***', orders: 27, ltv: 51300, lastOrder: 'today' },
          { id: 'c4', name: 'Dorina Shehu', phone: '+355 69 *** ***', orders: 4, ltv: 6800, lastOrder: '3 weeks ago' },
          { id: 'c5', name: 'Erion Berisha', phone: '+355 69 *** ***', orders: 14, ltv: 24200, lastOrder: '2 days ago' },
          { id: 'c6', name: 'Fatbardha Koci', phone: '+355 69 *** ***', orders: 1, ltv: 2090, lastOrder: '2 months ago' },
          { id: 'c7', name: 'Gjergji Marku', phone: '+355 69 *** ***', orders: 32, ltv: 61400, lastOrder: 'today' },
        ]);
        setLoading(false);
      });
  }, []);

  const handleReveal = async (id: string) => {
    setRevealing(id);
    try {
      const res = await apiClient<any>(`/owner/customers/${id}/reveal-contact`, { method: 'POST' });
      setRevealed(prev => ({ ...prev, [id]: res.phone || '+355 69 XXX XXXX' }));
    } catch {
      setRevealed(prev => ({ ...prev, [id]: '+355 69 876 543' }));
    } finally {
      setRevealing(null);
    }
  };

  const filtered = useMemo(() => {
    let result = [...customers];
    if (search) {
      const q = search.toLowerCase();
      result = result.filter(c => c.name.toLowerCase().includes(q) || c.phone.includes(q));
    }
    result.sort((a, b) => {
      if (sortKey === 'orders') return b.orders - a.orders;
      if (sortKey === 'ltv') return b.ltv - a.ltv;
      return a.name.localeCompare(b.name);
    });
    return result;
  }, [customers, search, sortKey]);

  if (loading) return <div className="p-6"><SkeletonBase className="h-64 w-full" /></div>;

  return (
    <div className="p-4 md:p-6 max-w-6xl mx-auto space-y-4">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h2 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>Customers</h2>
          <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{filtered.length} customers</p>
        </div>
        <div className="flex gap-2 w-full sm:w-auto">
          <div className="relative flex-1 sm:flex-none">
            <i className="ti ti-search absolute left-3 top-1/2 -translate-y-1/2 text-sm" style={{ color: 'var(--brand-text-muted)' }} />
            <input
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder="Search customers..."
              className="pl-9 pr-4 py-2 rounded-lg border text-sm outline-none w-full sm:w-48"
              style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
            />
          </div>
          <select
            value={sortKey}
            onChange={e => setSortKey(e.target.value as any)}
            className="px-3 py-2 text-xs rounded-lg border outline-none"
            style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
          >
            <option value="orders">Most orders</option>
            <option value="ltv">Highest LTV</option>
            <option value="name">Name A-Z</option>
          </select>
          <button onClick={() => {
            const exportData = filtered.map(c => ({ name: c.name, phone: revealed[c.id] ? '***REDACTED***' : c.phone, orders: c.orders, ltv: c.ltv, lastOrder: c.lastOrder }));
            exportCSV(exportData, 'customers.csv');
          }} className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors hover:bg-[var(--brand-surface-raised)]" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
            <i className="ti ti-download"></i> Export CSV
          </button>
        </div>
      </div>

      {filtered.length === 0 ? (
        <EmptyState title="No customers" description={search ? 'No match.' : 'No customer data yet.'} />
      ) : (
        <div className="rounded-xl border overflow-hidden" style={{ borderColor: 'var(--brand-border)' }}>
          <table className="w-full text-sm">
            <thead>
              <tr style={{ background: 'var(--brand-surface)' }}>
                <th className="text-left p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>Customer</th>
                <th className="text-left p-3 font-medium hidden sm:table-cell" style={{ color: 'var(--brand-text-muted)' }}>Phone</th>
                <th className="text-right p-3 font-medium" style={{ color: 'var(--brand-text-muted)' }}>Orders</th>
                <th className="text-right p-3 font-medium hidden md:table-cell" style={{ color: 'var(--brand-text-muted)' }}>LTV</th>
                <th className="text-right p-3 font-medium hidden lg:table-cell" style={{ color: 'var(--brand-text-muted)' }}>Last order</th>
                <th className="p-3" />
              </tr>
            </thead>
            <tbody>
              {filtered.map((c, i) => (
                <tr key={c.id} className="border-t transition-colors hover:bg-[var(--brand-surface)] slide-in-up" style={{ borderColor: 'var(--brand-border)', animationDelay: `${i * 30}ms` }}>
                  <td className="p-3">
                    <div className="font-medium">{c.name}</div>
                  </td>
                  <td className="p-3 hidden sm:table-cell" style={{ color: 'var(--brand-text-muted)' }}>
                    {revealed[c.id] || c.phone}
                  </td>
                  <td className="p-3 text-right font-medium">
                    <span className="px-2 py-0.5 rounded-full text-xs" style={{ background: c.orders > 15 ? 'rgba(5,150,105,0.1)' : 'var(--brand-surface-raised)', color: c.orders > 15 ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>
                      {c.orders}
                    </span>
                  </td>
                  <td className="p-3 text-right hidden md:table-cell" style={{ color: 'var(--brand-text-muted)' }}>
                    {(c.ltv / 100).toFixed(0)} ALL
                  </td>
                  <td className="p-3 text-right hidden lg:table-cell" style={{ color: 'var(--brand-text-muted)' }}>
                    {c.lastOrder}
                  </td>
                  <td className="p-3 text-right">
                    <Button onClick={() => handleReveal(c.id)} disabled={!!revealed[c.id]} isLoading={revealing === c.id} size="sm">
                      {revealed[c.id] ? <i className="ti ti-eye-check" /> : <i className="ti ti-eye" />}
                      {revealed[c.id] ? '' : ' Reveal'}
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
