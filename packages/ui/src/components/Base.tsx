import React from 'react';
import type { InputHTMLAttributes, ButtonHTMLAttributes } from 'react';
import { formatALL } from '../utils/index.js';
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
    let baseStyles = 'inline-flex items-center justify-center font-medium transition-all duration-200 focus-visible:outline-2 focus-visible:outline-[var(--brand-primary)] disabled:opacity-50 disabled:pointer-events-none rounded-[var(--brand-radius-btn)] active:scale-[0.98]';
    
    const variants = {
      // primary-strong (primary darkened ~15%) so light --brand-bg text clears
      // WCAG AA even when the tenant primary is a mid-tone (e.g. rose #e11d48,
      // where white text on raw primary = 4.3). Documented AA-button pattern.
      primary: 'bg-[var(--brand-primary-strong)] text-[var(--brand-bg)] hover:bg-[var(--brand-primary)]',
      secondary: 'bg-[var(--brand-surface-raised)] text-[var(--brand-text)] hover:bg-[var(--brand-border)]',
      outline: 'border border-[var(--brand-border)] text-[var(--brand-text)] hover:bg-[var(--brand-surface-raised)]',
      ghost: 'text-[var(--brand-text)] hover:bg-[var(--brand-surface-raised)]',
      danger: 'bg-[var(--color-danger-strong)] text-white hover:opacity-90',
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
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
        ) : null}
        {children}
      </button>
    );
  }
);
Button.displayName = 'Button';

// --- Input ---
export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: boolean;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className = '', error, ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={`flex h-10 w-full rounded-[var(--brand-radius-sm)] border bg-[var(--brand-surface)] px-3 py-2 text-sm text-[var(--brand-text)] placeholder-[var(--brand-text-muted)] focus:outline-none focus:ring-2 focus:ring-[var(--brand-primary)] disabled:cursor-not-allowed disabled:opacity-50
          ${error ? 'border-[var(--color-danger)] focus:ring-[var(--color-danger)]' : 'border-[var(--brand-border)]'}
          ${className}`}
        {...props}
      />
    );
  }
);
Input.displayName = 'Input';

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
      {label && <label className="block text-sm font-medium text-[var(--brand-text)]">{label}</label>}
      {children}
      {error && <p className="text-sm" style={{ color: 'var(--color-danger)' }}>{error}</p>}
      {!error && helperText && <p className="text-sm text-[var(--brand-text-muted)]">{helperText}</p>}
    </div>
  );
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
