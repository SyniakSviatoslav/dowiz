import type { ButtonHTMLAttributes, ReactNode } from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger';
type ButtonSize = 'sm' | 'md' | 'lg' | 'xl';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  loading?: boolean;
  icon?: ReactNode;
  children?: ReactNode;
}

const variants: Record<ButtonVariant, string> = {
  primary: 'bg-brand-primary text-[var(--brand-bg)] font-semibold hover:bg-brand-primary-hover active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed',
  secondary: 'bg-brand-surface text-brand-text border border-brand-border hover:bg-brand-surface-raised active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed',
  outline: 'bg-transparent text-brand-primary border border-brand-primary hover:bg-brand-primary-light active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed',
  ghost: 'bg-transparent text-brand-text hover:bg-brand-surface active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed',
  danger: 'bg-semantic-danger text-white hover:opacity-90 active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed',
};

const sizes: Record<ButtonSize, string> = {
  sm: 'px-3 py-1.5 text-sm min-h-11',
  md: 'px-4 py-2 text-sm',
  lg: 'px-6 py-3 text-base',
  xl: 'px-8 py-4 text-base min-h-[56px]',
};

export function Button({
  variant = 'primary',
  size = 'md',
  loading = false,
  icon,
  children,
  className = '',
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      className={`inline-flex items-center justify-center gap-2 font-body font-semibold rounded-full transition-all duration-200 focus-visible:outline-2 focus-visible:outline-brand-primary ${variants[variant]} ${sizes[size]} ${className}`}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? (
        <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
        </svg>
      ) : icon ? (
        <span className="icon">{icon}</span>
      ) : null}
      {children}
    </button>
  );
}
