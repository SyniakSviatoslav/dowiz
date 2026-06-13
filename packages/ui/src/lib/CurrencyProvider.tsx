import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import type { CurrencyCode } from '@deliveryos/shared-types';
import { getCurrency, setCurrency as setModuleCurrency, getCurrencies, subscribeToCurrency } from './currency.js';

interface CurrencyInfo { code: CurrencyCode; name: string; symbol: string; }

interface CurrencyContextValue {
  currency: CurrencyCode;
  currencies: CurrencyInfo[];
  setCurrency: (code: CurrencyCode) => void;
}

const CurrencyContext = createContext<CurrencyContextValue>({
  currency: 'ALL',
  currencies: [],
  setCurrency: () => {},
});

export function CurrencyProvider({ children }: { children: ReactNode }) {
  const [currency, setCurrencyState] = useState<CurrencyCode>(() => getCurrency());
  const currencies = getCurrencies();

  useEffect(() => {
    const unsub = subscribeToCurrency(() => {
      setCurrencyState(getCurrency());
    });
    return unsub;
  }, []);

  const changeCurrency = useCallback((newCurrency: CurrencyCode) => {
    setModuleCurrency(newCurrency);
    setCurrencyState(newCurrency);
  }, []);

  return (
    <CurrencyContext.Provider value={{ currency, currencies, setCurrency: changeCurrency }}>
      {children}
    </CurrencyContext.Provider>
  );
}

export function useCurrency(): CurrencyContextValue {
  return useContext(CurrencyContext);
}
