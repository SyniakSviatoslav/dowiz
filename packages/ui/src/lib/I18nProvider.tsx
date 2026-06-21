import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { type Locale, getLocale, setLocale as setModuleLocale, getLocales, subscribeToLocale, translate } from './i18n.js';

interface LocaleInfo { code: Locale; name: string; displayCode: string; }

interface I18nContextValue {
  locale: Locale;
  locales: LocaleInfo[];
  t: (key: string, fallback?: string, options?: Record<string, any>) => string;
  setLocale: (locale: Locale) => void;
}

const I18nContext = createContext<I18nContextValue>({
  locale: 'sq',
  locales: [],
  t: (k, f, o) => f || k,
  setLocale: () => {},
});

export function I18nProvider({ children }: { children: ReactNode }) {
  // React state — this is THE reactive source
  const [locale, setLocaleState] = useState<Locale>(() => getLocale());
  const locales = getLocales();

  // Subscribe to external locale changes (from other tabs etc)
  useEffect(() => {
    const unsub = subscribeToLocale(() => {
      setLocaleState(getLocale());
    });
    return unsub;
  }, []);

  // Create a fresh t() on every locale change so React detects dependency change
  const t = useCallback(
    (key: string, fallback?: string, options?: Record<string, any>) => translate(locale, key, fallback, options),
    [locale],
  );

  const changeLocale = useCallback((newLocale: Locale) => {
    setModuleLocale(newLocale);
    setLocaleState(newLocale);
  }, []);

  return (
    <I18nContext.Provider value={{ locale, locales, t, setLocale: changeLocale }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n(): I18nContextValue {
  return useContext(I18nContext);
}

const DISPLAY_MAP: Record<string, string> = { sq: 'SQ', en: 'EN', uk: 'UA' };

export function LanguageSwitcher({ variant = 'compact', allowed }: { variant?: 'compact' | 'full'; allowed?: string[] }) {
  const { locale, locales, setLocale: changeLocale } = useI18n();
  const [open, setOpen] = useState(false);
  // Storefronts pass the tenant's supported_locales so we never offer a language
  // the menu can't render (it would silently fall back to the default locale).
  const shown = allowed && allowed.length ? locales.filter(l => allowed.includes(l.code)) : locales;
  const currentLoc = locales.find(l => l.code === locale);
  if (shown.length <= 1) return null;
  const currentDisplay = DISPLAY_MAP[locale] || locale.toUpperCase();

  if (variant === 'full') {
    return (
      <div className="inline-flex items-center rounded-lg overflow-hidden" style={{ background: 'var(--brand-surface-raised)', border: '1px solid var(--brand-border)' }}>
        {shown.map((l) => (
          <button
            key={l.code}
            onClick={() => changeLocale(l.code)}
            className={`px-3 min-h-11 inline-flex items-center text-xs font-medium transition-all duration-200 ${
              locale === l.code ? 'text-[var(--brand-bg)] font-semibold' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'
            }`}
            style={locale === l.code ? { background: 'var(--brand-primary)' } : {}}
          >
            {l.displayCode}
          </button>
        ))}
      </div>
    );
  }

  return (
    <div className="relative">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-1 px-2.5 min-h-11 text-xs font-medium rounded-lg transition-colors hover:bg-[var(--brand-surface-raised)]"
        style={{ color: 'var(--brand-text-muted)', border: '1px solid var(--brand-border)' }}
        aria-label={`Switch language. Current: ${currentLoc?.name || locale}`}
      >
        <i className="ti ti-language" style={{ fontSize: '0.85rem' }} />
        <span className="hidden sm:inline">{currentDisplay}</span>
      </button>
      {open && (
        <>
          <div
            className="fixed inset-0 z-40"
            role="button"
            tabIndex={0}
            aria-label="Close menu"
            onClick={() => setOpen(false)}
            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setOpen(false); } }}
          />
          <div className="absolute right-0 top-full mt-1 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[130px] scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
            {shown.map((l) => (
              <button key={l.code} onClick={() => { changeLocale(l.code); setOpen(false); }}
                className={`flex items-center gap-2 w-full px-3 py-2 text-xs transition-colors hover:bg-[var(--brand-surface-raised)] ${locale === l.code ? 'font-semibold' : ''}`}
                style={{ color: locale === l.code ? 'var(--brand-primary)' : 'var(--brand-text)' }}
              >
                <span className="text-[10px] font-mono font-bold w-7 text-center rounded-sm px-0.5 py-px" style={{
                  background: locale === l.code ? 'var(--brand-primary-light)' : 'var(--brand-surface-raised)',
                  color: locale === l.code ? 'var(--brand-primary)' : 'var(--brand-text-muted)',
                }}>{l.displayCode}</span>
                <span className="flex-1">{l.name}</span>
                {locale === l.code && <i className="ti ti-check ml-1" style={{ color: 'var(--brand-primary)', fontSize: '0.7rem' }} />}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
