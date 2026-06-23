import { useState } from 'react';
import { useCurrency } from '../../lib/CurrencyProvider.js';

export function CurrencySwitcher() {
  const { currency, currencies, setCurrency } = useCurrency();
  const [open, setOpen] = useState(false);
  const current = currencies.find(c => c.code === currency);

  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1 px-2.5 py-1.5 min-h-11 text-xs font-medium rounded-lg transition-colors hover:bg-[var(--brand-surface-raised)]"
        style={{ color: 'var(--brand-text)', border: '1px solid var(--brand-border)', background: 'var(--brand-surface)' }}
        aria-label={`Switch currency. Current: ${current?.code || currency}`}
      >
        <i className="ti ti-coin" style={{ color: 'var(--brand-text-muted)', fontSize: '0.85rem' }} aria-hidden="true" />
        <span className="font-mono font-bold">{current?.symbol || currency}</span>
        <span className="hidden sm:inline text-[10px]">{currency}</span>
      </button>
      {open && (
        <>
          <div
            className="fixed inset-0 z-40"
            role="button"
            tabIndex={0}
            aria-label="Close menu"
            onClick={() => setOpen(false)}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setOpen(false); } }}
          />
          <div className="absolute right-0 top-full mt-1 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[110px] scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
            {currencies.map((c) => (
              <button key={c.code} onClick={() => { setCurrency(c.code); setOpen(false); }}
                className={`flex items-center gap-2 w-full px-3 py-2 text-xs transition-colors hover:bg-[var(--brand-surface-raised)] ${currency === c.code ? 'font-semibold' : ''}`}
                style={{ color: currency === c.code ? 'var(--brand-primary)' : 'var(--brand-text)' }}
              >
                <span className="text-[10px] font-mono font-bold w-6 text-center">{c.symbol}</span>
                <span className="flex-1">{c.name}</span>
                {currency === c.code && <i className="ti ti-check ml-1" style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
