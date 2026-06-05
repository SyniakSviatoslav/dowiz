import type { ReactNode } from 'react';

const EU_ALLERGENS = [
  'gluten', 'shellfish', 'eggs', 'fish', 'peanuts', 'soy',
  'milk', 'nuts', 'celery', 'mustard', 'sesame', 'sulphites', 'lupin', 'molluscs',
];

interface AllergenEditorProps {
  status: 'unset' | 'none' | 'listed';
  declaredAllergens: string[];
  bomAllergens?: string[];
  onStatusChange: (status: 'unset' | 'none' | 'listed') => void;
  onAllergensChange: (allergens: string[]) => void;
}

export function AllergenEditor({ status, declaredAllergens, bomAllergens, onStatusChange, onAllergensChange }: AllergenEditorProps) {
  const toggleAllergen = (a: string) => {
    if (declaredAllergens.includes(a)) {
      onAllergensChange(declaredAllergens.filter(x => x !== a));
    } else {
      onAllergensChange([...declaredAllergens, a]);
    }
  };

  return (
    <div className="space-y-3">
      <label className="text-xs font-medium block" style={{ color: 'var(--brand-text-muted)' }}>Allergen Attestation *</label>

      {/* Tri-state toggle */}
      <div className="flex rounded-lg p-0.5" style={{ background: 'var(--brand-surface-raised)' }}>
        {(['unset', 'none', 'listed'] as const).map(s => {
          const active = status === s;
          const labels: Record<string, string> = { unset: 'Not yet', none: 'None', listed: 'Has allergens' };
          const colors: Record<string, string> = { unset: 'var(--brand-text-muted)', none: 'var(--color-success)', listed: 'var(--color-warning)' };
          return (
            <button
              key={s}
              type="button"
              onClick={() => onStatusChange(s)}
              className={`flex-1 py-1.5 text-[10px] font-medium rounded-md transition-all ${
                active ? 'text-white shadow-sm' : ''
              }`}
              style={{
                background: active ? colors[s] : 'transparent',
                color: active ? '#fff' : 'var(--brand-text-muted)',
              }}
            >
              {labels[s]}
            </button>
          );
        })}
      </div>

      {/* Status message */}
      {status === 'unset' && (
        <div className="flex items-center gap-2 p-2 rounded-lg text-[11px]" style={{ background: 'rgba(217,119,6,0.08)', color: 'var(--color-warning)' }}>
          <i className="ti ti-alert-triangle" />
          <span>Product cannot be published until allergens are declared.</span>
        </div>
      )}
      {status === 'none' && (
        <div className="flex items-center gap-2 p-2 rounded-lg text-[11px]" style={{ background: 'rgba(5,150,105,0.08)', color: 'var(--color-success)' }}>
          <i className="ti ti-shield-check" />
          <span>Confirmed: this product contains no allergens.</span>
        </div>
      )}

      {/* Allergen chip selector (shown when 'listed') */}
      {status === 'listed' && (
        <div>
          <div className="flex flex-wrap gap-1">
            {EU_ALLERGENS.map(a => {
              const active = declaredAllergens.includes(a);
              return (
                <button
                  key={a}
                  type="button"
                  onClick={() => toggleAllergen(a)}
                  className={`px-2 py-1 rounded-full text-[10px] font-medium transition-all ${
                    active ? 'text-white' : 'hover:bg-[var(--brand-surface-raised)]'
                  }`}
                  style={{
                    background: active ? 'var(--color-warning)' : 'var(--brand-border)',
                    color: active ? '#fff' : 'var(--brand-text-muted)',
                  }}
                >
                  {a}
                </button>
              );
            })}
          </div>
        </div>
      )}

      {/* BOM cross-check warning */}
      {bomAllergens && bomAllergens.length > 0 && status !== 'unset' && (
        <div className="flex flex-wrap items-start gap-1">
          {bomAllergens.filter(a => !declaredAllergens.includes(a)).length > 0 && (
            <div className="flex items-center gap-1.5 p-2 rounded-lg text-[10px] w-full" style={{ background: 'rgba(217,119,6,0.06)', color: 'var(--color-warning)', border: '1px dashed rgba(217,119,6,0.3)' }}>
              <i className="ti ti-info-circle shrink-0" />
              <span>
                Recipe contains undeclared allergens:{' '}
                {bomAllergens.filter(a => !declaredAllergens.includes(a)).map(a => (
                  <span key={a} className="font-semibold mx-0.5">{a}</span>
                ))}
                . Please review — this is advisory only and does not block publishing.
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function ReadinessIndicator({
  checks,
}: {
  checks: Array<{ label: string; pass: boolean; action?: string; onAction?: () => void }>;
}) {
  const passed = checks.filter(c => c.pass).length;
  const total = checks.length;
  const isReady = passed === total;

  return (
    <div className="p-3 rounded-lg border" style={{
      background: isReady ? 'rgba(5,150,105,0.06)' : 'rgba(217,119,6,0.04)',
      borderColor: isReady ? 'var(--color-success)' : 'var(--color-warning)',
    }}>
      <div className="flex items-center gap-2 mb-2">
        <i className={`${isReady ? 'ti ti-circle-check' : 'ti ti-alert-triangle'}`} style={{ color: isReady ? 'var(--color-success)' : 'var(--color-warning)', fontSize: '0.9rem' }} />
        <span className="text-xs font-semibold" style={{ color: 'var(--brand-text)' }}>
          {isReady ? 'Ready to publish' : `${total - passed} item${total - passed > 1 ? 's' : ''} remaining`}
        </span>
        <span className="ml-auto text-[10px] font-mono px-1.5 py-0.5 rounded" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
          {passed}/{total}
        </span>
      </div>
      <div className="space-y-1">
        {checks.map((c, i) => (
          <div key={i} className="flex items-center gap-2 text-[11px]" style={{ color: c.pass ? 'var(--color-success)' : 'var(--brand-text-muted)' }}>
            <i className={`${c.pass ? 'ti ti-check' : 'ti ti-circle-dashed'}`} style={{ fontSize: '0.7rem' }} />
            <span className="flex-1">{c.label}</span>
            {!c.pass && c.action && (
              <button type="button" onClick={c.onAction} className="text-[9px] px-1.5 py-0.5 rounded font-medium hover:underline" style={{ color: 'var(--brand-primary)' }}>
                {c.action}
              </button>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
