import type { InputHTMLAttributes } from 'react';
import { useI18n } from '../../lib/I18nProvider.js';

// Omit the native `size` (character-width number) — we repurpose `size` as the visual variant.
interface SearchInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'size'> {
  /** When provided AND the field has a value, renders a clear (×) button that calls this. */
  onClear?: () => void;
  /** Width / flex classes for the wrapper (e.g. 'flex-1 sm:w-48'). */
  containerClassName?: string;
  /** 'md' (default, 44px) for admin toolbars; 'sm' (36px) for compact rows like the storefront. */
  size?: 'sm' | 'md';
}

/**
 * Canonical search field — the leading-search-icon + input pattern that was hand-rolled per screen
 * (CRM, supplies, menu, dashboard…). Matches the <Input> spec (py-2.5 / min-h-11 / brand-border /
 * rounded-[brand-radius-sm] / brand-surface / focus-ring-2) with a token search glyph and an
 * optional clear button. Pass width/flex via containerClassName.
 */
export function SearchInput({ onClear, className = '', containerClassName = '', value, size = 'md', ...props }: SearchInputProps) {
  const { t } = useI18n();
  const hasValue = value != null && value !== '';
  // 16px on mobile (no iOS zoom-on-focus), compact on desktop (md = 14px, sm = 12px).
  const sizeCls = size === 'sm' ? 'py-1.5 min-h-9 text-base md:text-step-xs pl-9' : 'py-2.5 min-h-11 text-base md:text-sm pl-10';
  const iconCls = size === 'sm' ? 'left-2.5 text-step-xs' : 'left-3';
  return (
    <div className={`relative ${containerClassName}`}>
      <i className={`ti ti-search absolute ${iconCls} top-1/2 -translate-y-1/2 text-brand-text-muted pointer-events-none`} aria-hidden="true" />
      <input
        type="search"
        value={value}
        className={`w-full ${sizeCls} ${onClear ? 'pr-10' : 'pr-4'} bg-brand-surface border border-brand-border rounded-md text-brand-text placeholder-brand-text-muted font-body outline-none transition-[border-color,box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none hover:border-brand-text-muted/60 focus:ring-2 focus:ring-brand-primary [&::-webkit-search-cancel-button]:hidden ${className}`}
        {...props}
      />
      {onClear && hasValue && (
        <button
          type="button"
          onClick={onClear}
          aria-label={t('common.clear_search', 'Clear search')}
          className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center justify-center w-7 h-7 rounded-full text-brand-text-muted hover:text-brand-text hover:bg-brand-surface-raised transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-primary"
        >
          <i className="ti ti-x" aria-hidden="true" />
        </button>
      )}
    </div>
  );
}
