import React, { type InputHTMLAttributes, type ButtonHTMLAttributes } from 'react';
import { t } from '../lib/i18n.js';

// --- Button ---
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
  loading?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className = '', variant = 'primary', size = 'md', isLoading, loading, children, ...props }, ref) => {
    const isBusy = isLoading || loading;
    const baseStyles = `inline-flex items-center justify-center font-medium transition-all duration-200 focus-visible:outline-2 focus-visible:outline-[var(--brand-primary)] disabled:opacity-50 disabled:pointer-events-none rounded-full active:scale-[0.98]`;

    const variants = {
      primary: `bg-[var(--brand-primary)] text-[var(--brand-bg)] hover:bg-brand-primary-hover`,
      secondary: `bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-brand-border`,
      outline: `border border-[var(--brand-border)] text-[var(--brand-text)] hover:bg-brand-surface-raised`,
      ghost: `text-[var(--brand-text)] hover:bg-brand-surface-raised`,
      danger: `bg-semantic-danger text-white hover:opacity-90`,
    };

    const sizes = {
      sm: 'h-8 px-3 text-xs',
      md: 'h-10 px-4 py-2 text-sm',
      lg: 'h-12 px-8 text-base',
    };

    return (
      <button
        ref={ref}
        className={`${baseStyles} ${variants[variant]} ${sizes[size]} ${className}`}
        disabled={isBusy || props.disabled}
        {...props}
      >
        {isBusy ? (
          <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
            <path className="opacity-75" fill="currentColor" d={`d`} />
          </svg>
        ) : null}
        {children}
      </button>
    );
  }
);
Button.displayName = 'Button'; // eslint-disable-line local/no-hardcoded-string -- component identifier, not UI copy

// --- Input ---
export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: boolean;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className = '', error, ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={`flex h-10 w-full rounded-md border bg-[var(--brand-surface)] px-3 py-2 text-sm text-[var(--brand-text)] placeholder-[var(--brand-text-muted)] focus:outline-none focus:ring-2 focus:ring-brand-primary disabled:cursor-not-allowed disabled:opacity-50
          ${error ? 'border-[var(--color-danger)] focus:ring-semantic-danger' : 'border-brand-border'}
          ${className}`}
        {...props}
      />
    );
  }
);
Input.displayName = 'Input'; // eslint-disable-line local/no-hardcoded-string -- component identifier, not UI copy

// --- FormField ---
export function FormField({
  label,
  error,
  helperText,
  children
}: {
  label?: string;
  error?: string;
  helperText?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1 mb-4">
      {label && <label className="block text-sm font-medium text-brand-text">{label}</label>}
      {children}
      {error && <p className="text-sm" style={{ color: 'var(--color-danger)' }}>{error}</p>}
      {!error && helperText && <p className="text-sm text-brand-text-muted">{helperText}</p>}
    </div>
  );
}

// --- BrandLogo ---
export function BrandLogo({ name, logoUrl }: { name: string; logoUrl?: string | null }) {
  if (logoUrl) {
    return <img src={logoUrl} alt={name} className="h-8 object-contain" />;
  }
  return <span className="font-bold text-xl text-brand-primary" style={{ fontFamily: 'var(--brand-font-heading)' }}>{name}</span>;
}

// --- PriceDisplay ---
export { PriceDisplay } from './atoms/PriceDisplay.js';

// --- StatusBadge ---
export function StatusBadge({ status, pulse }: { status: string; pulse?: boolean }) {
  const key = status.toUpperCase().replace(/-/g, '_');
  const orderKey = `order.${key.toLowerCase()}`;
  const label = t(orderKey, status.replace(/_/g, ' '));
  const colorVar = `--status-${key.toLowerCase()}`;

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold text-white ${pulse ? 'animate-pulse' : ''}`}
      style={{ backgroundColor: `var(${colorVar})` }}
    >
      {pulse && <span className="w-1.5 h-1.5 rounded-full bg-white" />}
      {label}
    </span>
  );
}
