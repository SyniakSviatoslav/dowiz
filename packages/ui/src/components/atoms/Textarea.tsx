import type { TextareaHTMLAttributes } from 'react';

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  helper?: string;
  error?: string;
}

/**
 * Canonical multi-line input — mirrors <Input>'s spec (px-4 py-2.5 / brand-border / radius-sm /
 * brand-surface / focus-ring-2) so multi-line fields match single-line ones everywhere.
 */
export function Textarea({ label, helper, error, className = '', id, rows = 3, ...props }: TextareaProps) {
  const taId = id || props.name;
  const describedBy = error ? `${taId}-error` : helper ? `${taId}-helper` : undefined;
  return (
    <div className="flex flex-col gap-1">
      {label && (
        <label htmlFor={taId} className="text-sm font-medium text-brand-text">
          {label}
        </label>
      )}
      <textarea
        id={taId}
        rows={rows}
        aria-invalid={error ? true : undefined}
        aria-describedby={describedBy}
        className={`w-full px-4 py-2.5 bg-brand-surface border rounded-[var(--brand-radius-sm)] text-base md:text-sm text-brand-text placeholder-brand-text-muted font-body resize-y transition-[border-color,box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:border-brand-text-muted/60 focus:outline-none focus:ring-2 ${
          error ? 'border-semantic-danger focus:ring-semantic-danger' : 'border-brand-border focus:ring-brand-primary'
        } ${className}`}
        {...props}
      />
      {error && <p id={`${taId}-error`} className="text-xs text-semantic-danger">{error}</p>}
      {helper && !error && <p id={`${taId}-helper`} className="text-xs text-brand-text-muted">{helper}</p>}
    </div>
  );
}
