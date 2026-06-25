import type { SelectHTMLAttributes, ReactNode } from 'react';

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  helper?: string;
  error?: string;
  children: ReactNode;
}

/**
 * Canonical select — matches <Input>'s spec exactly (px-4 py-2.5 / min-h-11 / brand-border /
 * radius-sm / brand-surface / focus-ring-2) so form controls are visually coherent everywhere.
 * Native <select> with appearance-none + a token chevron. Pass <option>s as children.
 */
export function Select({ label, helper, error, className = '', id, children, ...props }: SelectProps) {
  const selectId = id || props.name;
  const describedBy = error ? `${selectId}-error` : helper ? `${selectId}-helper` : undefined;
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label htmlFor={selectId} className="text-sm font-medium text-brand-text">
          {label}
        </label>
      )}
      <div className="relative">
        <select
          id={selectId}
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy}
          className={`w-full appearance-none px-4 pr-10 py-2.5 min-h-11 bg-brand-surface border rounded-[var(--brand-radius-sm)] text-brand-text font-body transition-[border-color,box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:border-brand-text-muted/60 focus:outline-none focus:ring-2 ${
            error ? 'border-semantic-danger focus:ring-semantic-danger' : 'border-brand-border focus:ring-brand-primary'
          } ${className}`}
          {...props}
        >
          {children}
        </select>
        <i className="ti ti-chevron-down absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-brand-text-muted" aria-hidden="true" />
      </div>
      {error && <p id={`${selectId}-error`} className="text-xs text-semantic-danger">{error}</p>}
      {helper && !error && <p id={`${selectId}-helper`} className="text-xs text-brand-text-muted">{helper}</p>}
    </div>
  );
}
