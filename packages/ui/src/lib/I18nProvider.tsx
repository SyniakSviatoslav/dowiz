import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { type Locale, getLocale, setLocale as setModuleLocale, getLocales, subscribeToLocale, translate } from './i18n.js';

interface LocaleInfo { code: Locale; name: string; displayCode: string; }

interface I18nContextValue {
  locale: Locale;
  locales: LocaleInfo[];
  t: (key: string, fallback?: string) => string;
  setLocale: (locale: Locale) => void;
}

const I18nContext = createContext<I18nContextValue>({
  locale: 'sq',
  locales: [],
  t: (k, f) => f || k,
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
    (key: string, fallback?: string) => translate(locale, key, fallback),
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

export function LanguageSwitcher({ variant = 'compact' }: { variant?: 'compact' | 'full' }) {
  const { locale, locales, setLocale: changeLocale } = useI18n();
  const [open, setOpen] = useState(false);
  const currentLoc = locales.find(l => l.code === locale);
  const currentDisplay = DISPLAY_MAP[locale] || locale.toUpperCase();

  if (variant === 'full') {
    return (
      <div className="inline-flex items-center rounded-lg overflow-hidden" style={{ background: 'var(--brand-surface-raised)', border: '1px solid var(--brand-border)' }}>
        {locales.map((l) => (
          <button
            key={l.code}
            onClick={() => changeLocale(l.code)}
            className={`px-2.5 py-1.5 text-xs font-medium transition-all duration-200 ${
              locale === l.code ? 'text-white' : 'text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'
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
        className="flex items-center gap-1 px-2.5 py-1.5 text-xs font-medium rounded-lg transition-colors hover:bg-[var(--brand-surface-raised)]"
        style={{ color: 'var(--brand-text-muted)', border: '1px solid var(--brand-border)' }}
        aria-label={`Switch language. Current: ${currentLoc?.name || locale}`}
      >
        <i className="ti ti-language" style={{ fontSize: '0.85rem' }} />
        <span className="hidden sm:inline">{currentDisplay}</span>
      </button>
      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full mt-1 z-50 rounded-lg shadow-elevation-3 py-1 min-w-[130px] scale-in" style={{ background: 'var(--brand-surface)', border: '1px solid var(--brand-border)' }}>
            {locales.map((l) => (
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
