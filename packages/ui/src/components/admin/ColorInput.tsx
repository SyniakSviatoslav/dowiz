import React from 'react';

interface ColorInputProps {
  value: string;
  onChange: (value: string) => void;
  label: string;
}
export function ColorInput({ value, onChange, label }: ColorInputProps) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-sm font-medium text-[var(--brand-text-muted)]">{label}</span>
      <div className="flex items-center gap-2">
        <div className="w-10 h-10 rounded-[var(--brand-radius-sm)] border border-[var(--brand-border)] overflow-hidden shrink-0">
          <input
            type="color"
            value={value}
            onChange={e => onChange(e.target.value)}
            className="w-full h-full p-0 border-none cursor-pointer scale-150"
          />
        </div>
        <input
          type="text"
          value={value}
          onChange={e => onChange(e.target.value)}
          className="flex-1 bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius-sm)] px-3 py-2 text-[var(--brand-text)]"
        />
      </div>
    </div>
  );
}
