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
      <span className="text-sm font-medium text-[var(--brand-text-muted)]">{label}</span>
      <div className="flex items-center gap-2">
        <div className="w-10 h-10 rounded-[var(--brand-radius-sm)] border border-[var(--brand-border)] overflow-hidden shrink-0">
          <input
            type="color"
            value={pickerValue}
            onChange={e => onChange(e.target.value)}
            className="w-full h-full p-0 border-none cursor-pointer scale-150"
          />
        </div>
        <input
          type="text"
          value={resolveColor(value)}
          onChange={e => onChange(e.target.value)}
          className="flex-1 bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius-sm)] px-3 py-2 text-[var(--brand-text)]"
        />
      </div>
    </div>
  );
}
