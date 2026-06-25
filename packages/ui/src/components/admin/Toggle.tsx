interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
  'aria-label'?: string;
}
export function Toggle({ checked, onChange, label, disabled = false, 'aria-label': ariaLabel }: ToggleProps) {
  const toggle = () => { if (!disabled) onChange(!checked); };
  return (
    <label className={`flex items-center gap-3 ${disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}>
      {/* 44px hit area: track is centered inside a min-h/min-w-11 padded box */}
      <span className="relative inline-flex h-11 min-w-11 items-center justify-center">
        <span
          role="switch"
          aria-checked={checked}
          aria-label={ariaLabel}
          aria-disabled={disabled || undefined}
          tabIndex={disabled ? -1 : 0}
          onClick={toggle}
          onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } }}
          className={`relative block w-12 h-6 rounded-full outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)] ${checked ? 'bg-[var(--brand-primary)]' : 'bg-[var(--brand-surface-raised)]'}`}
          style={{ transition: 'background-color var(--motion-fast) var(--ease-soft)' }}
        >
          <span
            className={`absolute top-1 left-1 w-4 h-4 rounded-full bg-[var(--color-on-primary)] shadow-sm ${checked ? 'translate-x-6' : 'translate-x-0'}`}
            style={{ transition: 'transform var(--motion-fast) var(--ease-out)' }}
          />
        </span>
      </span>
      {label && <span className="font-medium text-[var(--brand-text)]">{label}</span>}
    </label>
  );
}
