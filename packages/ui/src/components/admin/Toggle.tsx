import React from 'react';

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  'aria-label'?: string;
}
export function Toggle({ checked, onChange, label, 'aria-label': ariaLabel }: ToggleProps) {
  return (
    <label className="flex items-center gap-3 cursor-pointer">
      <div
        role="switch"
        aria-checked={checked}
        aria-label={ariaLabel}
        tabIndex={0}
        onClick={() => onChange(!checked)}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onChange(!checked); } }}
        className={`relative w-12 h-6 rounded-full transition-colors ${checked ? 'bg-[var(--brand-primary)]' : 'bg-[var(--brand-surface-raised)]'}`}
      >
        <div className={`absolute top-1 left-1 w-4 h-4 rounded-full bg-[var(--color-on-primary)] transition-transform ${checked ? 'translate-x-6' : 'translate-x-0'}`} />
      </div>
      {label && <span className="font-medium text-[var(--brand-text)]">{label}</span>}
    </label>
  );
}
