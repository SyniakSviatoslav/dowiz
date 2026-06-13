import { formatMoney, type CurrencyCode } from '@deliveryos/shared-types';
import { useCurrency } from '../../lib/CurrencyProvider.js';

interface PriceDisplayProps {
  amount: number;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
  currency?: CurrencyCode;
  rate?: number;
}

const sizes = {
  sm: 'text-sm',
  md: 'text-base',
  lg: 'text-xl font-bold',
};

export function PriceDisplay({ amount, className = '', size = 'md', currency: explicitCurrency, rate }: PriceDisplayProps) {
  const { currency: contextCurrency } = useCurrency();
  const displayCurrency = explicitCurrency || contextCurrency;
  const formatted = formatMoney(amount, displayCurrency, rate);

  return (
    <span className={`font-semibold text-brand-text ${sizes[size]} ${className}`}>
      {formatted}
    </span>
  );
}
