import type { CSSProperties } from 'react';
import { formatMoney, type CurrencyCode } from '@deliveryos/shared-types';
import { useCurrency } from '../../lib/CurrencyProvider.js';

interface PriceDisplayProps {
  amount: number;
  className?: string;
  style?: CSSProperties;
  size?: 'sm' | 'md' | 'lg';
  currency?: CurrencyCode;
  rate?: number;
}

const sizes = {
  sm: 'text-sm',
  md: 'text-base',
  lg: 'text-xl font-bold',
};

export function PriceDisplay({ amount, className = '', style, size = 'md', currency: explicitCurrency, rate: explicitRate }: PriceDisplayProps) {
  const { currency: contextCurrency, eurRate } = useCurrency();
  const displayCurrency = explicitCurrency || contextCurrency;
  const rate = explicitRate ?? eurRate ?? undefined;
  const formatted = formatMoney(amount, displayCurrency, rate);

  return (
    <span className={`font-semibold text-brand-text whitespace-nowrap tabular-nums ${sizes[size]} ${className}`} style={style}>
      {formatted}
    </span>
  );
}
