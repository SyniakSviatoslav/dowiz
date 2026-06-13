import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import type { CurrencyCode } from '@deliveryos/shared-types';
import { getCurrency, setCurrency as setModuleCurrency, getCurrencies, subscribeToCurrency } from './currency.js';

interface CurrencyInfo { code: CurrencyCode; name: string; symbol: string; }

interface CurrencyContextValue {
  currency: CurrencyCode;
  currencies: CurrencyInfo[];
  setCurrency: (code: CurrencyCode) => void;
  eurRate: number | null;
}

const CurrencyContext = createContext<CurrencyContextValue>({
  currency: 'ALL',
  currencies: [],
  setCurrency: () => {},
  eurRate: null,
});

export function CurrencyProvider({ children }: { children: ReactNode }) {
  const [currency, setCurrencyState] = useState<CurrencyCode>(() => getCurrency());
  const [eurRate, setEurRate] = useState<number | null>(null);
  const currencies = getCurrencies();

  useEffect(() => {
    const unsub = subscribeToCurrency(() => {
      setCurrencyState(getCurrency());
    });
    return unsub;
  }, []);

  // Fetch EUR rate on mount
  useEffect(() => {
    fetch('/v1/rates')
      .then(r => r.ok ? r.json() : null)
      .then(data => {
        if (data && typeof data.rate === 'number') setEurRate(data.rate);
      })
      .catch(() => { /* rate unavailable — display ALL only */ });
  }, []);

  const changeCurrency = useCallback((newCurrency: CurrencyCode) => {
    setModuleCurrency(newCurrency);
    setCurrencyState(newCurrency);
  }, []);

  return (
    <CurrencyContext.Provider value={{ currency, currencies, setCurrency: changeCurrency, eurRate }}>
      {children}
    </CurrencyContext.Provider>
  );
}

export function useCurrency(): CurrencyContextValue {
  return useContext(CurrencyContext);
}
