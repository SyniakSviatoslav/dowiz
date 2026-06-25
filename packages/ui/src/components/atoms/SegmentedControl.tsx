import type { ReactNode } from 'react';

export interface SegmentOption<T extends string = string> {
  value: T;
  label: ReactNode;
  /** Optional Tabler icon class, e.g. 'ti ti-flame'. */
  icon?: string;
}

interface SegmentedControlProps<T extends string = string> {
  options: ReadonlyArray<SegmentOption<T>>;
  value: T;
  onChange: (value: T) => void;
  /** Accessible name for the group (radiogroup semantics via aria-pressed buttons). */
  'aria-label'?: string;
  size?: 'sm' | 'md';
  className?: string;
}

/**
 * Canonical filter / sort / segment selector — one coherent treatment for the toggle-chip pattern
 * that was hand-rolled per screen (supplies, menu filters, etc.). Active = brand-primary fill;
 * inactive = surface-raised muted. Horizontally scrollable, snap-aligned, 44px-friendly tap targets.
 */
export function SegmentedControl<T extends string = string>({
  options, value, onChange, size = 'md', className = '', ...aria
}: SegmentedControlProps<T>) {
  const pad = size === 'sm' ? 'px-3 py-1.5 text-step-2xs' : 'px-3.5 py-2 text-step-xs';
  return (
    <div role="group" aria-label={aria['aria-label']} className={`flex gap-1.5 overflow-x-auto no-scrollbar snap-x ${className}`}>
      {options.map(opt => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            type="button"
            onClick={() => onChange(opt.value)}
            aria-pressed={active}
            className={`flex items-center gap-1 ${pad} font-medium rounded-full snap-start shrink-0 whitespace-nowrap outline-none transition-[background-color,color,box-shadow,transform] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--brand-bg)] ${
              active
                ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] shadow-[var(--elev-1)]'
                : 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] [@media(hover:hover)]:hover:text-[var(--brand-text)]'
            }`}
          >
            {opt.icon && <i className={opt.icon} aria-hidden="true" style={{ fontSize: '0.8rem' }} />}
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
