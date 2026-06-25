import type { InputHTMLAttributes } from 'react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helper?: string;
  error?: string;
}

export function Input({ label, helper, error, className = '', id, ...props }: InputProps) {
  const inputId = id || props.name;
  const describedBy = error ? `${inputId}-error` : helper ? `${inputId}-helper` : undefined;
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label htmlFor={inputId} className="text-sm font-medium text-brand-text">
          {label}
        </label>
      )}
      <input
        id={inputId}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy}
        className={`w-full px-4 py-2.5 min-h-11 bg-brand-surface border rounded-[var(--brand-radius-sm)] text-base md:text-sm text-brand-text placeholder-brand-text-muted font-body transition-[border-color,box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:border-brand-text-muted/60 focus:outline-none focus:ring-2 ${
          error ? 'border-semantic-danger focus:ring-semantic-danger' : 'border-brand-border focus:ring-brand-primary'
        } ${className}`}
        {...props}
      />
      {error && <p id={`${inputId}-error`} className="text-xs text-semantic-danger">{error}</p>}
      {helper && !error && <p id={`${inputId}-helper`} className="text-xs text-brand-text-muted">{helper}</p>}
    </div>
  );
}
