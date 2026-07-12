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
  primary: `bg-brand-primary text-brand-bg font-semibold shadow-[var(--elevation-1)] hover:bg-brand-primary-hover hover:shadow-elevation-2 active:scale-[0.98] disabled:opacity-50 disabled:shadow-none disabled:cursor-not-allowed`,
  secondary: `bg-brand-surface text-brand-text border border-brand-border hover:bg-brand-surface-raised hover:border-brand-primary/40 active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed`,
  outline: `outline`,
  ghost: `bg-transparent text-brand-text hover:bg-brand-surface active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed`,
  danger: `bg-semantic-danger text-white shadow-[var(--elevation-1)] hover:opacity-90 hover:shadow-elevation-2 active:scale-[0.98] disabled:opacity-50 disabled:shadow-none disabled:cursor-not-allowed`,
};

const sizes: Record<ButtonSize, string> = {
  sm: 'px-3 py-1.5 text-sm min-h-11',
  md: 'px-4 py-2 text-sm min-h-11',
  lg: 'px-6 py-3 text-base min-h-11',
  xl: `px-8 py-4 text-base min-h-[56px]`,
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
      className={`inline-flex items-center justify-center gap-2 font-body font-semibold rounded-full transition-[transform,background-color,border-color,box-shadow,opacity] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-primary ${variants[variant]} ${sizes[size]} ${className}`}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? (
        <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
          <path className="opacity-75" fill="currentColor" d={`d`} />
        </svg>
      ) : icon ? (
        <span className="icon">{icon}</span>
      ) : null}
      {children}
    </button>
  );
}
