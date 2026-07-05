import React from 'react';

interface ColorInputProps {
  value: string;
  onChange: (value: string) => void;
  label: string;
}

function resolveColor(value: string): string {
  if (!value || !value.startsWith('var(')) return value;
  if (typeof window === 'undefined') return '#000000';
  const varName = value.match(/var\(([^,)]+)/)?.[1]?.trim();
  if (!varName) return '#000000';
  const resolved = getComputedStyle(document.documentElement).getPropertyValue(varName).trim();
  return /^#[0-9a-fA-F]{6}$/i.test(resolved) ? resolved : '#000000';
}

export function ColorInput({ value, onChange, label }: ColorInputProps) {
  const pickerValue = resolveColor(value);

  return (
    <div className="flex flex-col gap-1">
      <span className="text-sm font-medium text-[var(--brand-text)]">{label}</span>
      <div className="flex items-center gap-2">
        <div className="w-10 h-10 rounded-[var(--brand-radius-sm)] border border-[var(--brand-border)] overflow-hidden shrink-0 transition-shadow duration-150 ease-[var(--ease-soft)] focus-within:ring-2 focus-within:ring-[var(--brand-primary)] focus-within:ring-offset-1 focus-within:ring-offset-[var(--brand-surface)]">
          <input
            type="color"
            value={pickerValue}
            onChange={e => onChange(e.target.value)}
            aria-label={label}
            className="w-full h-full p-0 border-none cursor-pointer scale-150 focus:outline-none"
          />
        </div>
        <input
          type="text"
          value={resolveColor(value)}
          onChange={e => onChange(e.target.value)}
          aria-label={label}
          className="flex-1 min-w-0 bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius-sm)] px-3 py-2 text-[var(--brand-text)] transition-[border-color,box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:border-brand-text-muted/60 focus:outline-none focus:ring-2 focus:ring-[var(--brand-primary)]"
        />
      </div>
    </div>
  );
}
