import type { InputHTMLAttributes } from 'react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helper?: string;
  error?: string;
}

export function Input({ label, helper, error, className = '', id, ...props }: InputProps) {
  const inputId = id || props.name;
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label htmlFor={inputId} className="text-sm font-medium text-brand-text">
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`w-full px-4 py-2.5 bg-brand-surface border rounded-md text-brand-text placeholder-brand-text-muted font-body transition-colors focus:outline-none focus:ring-2 focus:ring-brand-primary ${
          error ? 'border-semantic-danger' : 'border-brand-border'
        } ${className}`}
        {...props}
      />
      {error && <p className="text-xs text-semantic-danger">{error}</p>}
      {helper && !error && <p className="text-xs text-brand-text-muted">{helper}</p>}
    </div>
  );
}
