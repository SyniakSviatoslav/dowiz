import { formatALL } from '../../utils/index.js';

interface PriceDisplayProps {
  amount: number;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

const sizes = {
  sm: 'text-sm',
  md: 'text-base',
  lg: 'text-xl font-bold',
};

export function PriceDisplay({ amount, className = '', size = 'md' }: PriceDisplayProps) {
  return (
    <span className={`font-semibold text-brand-text ${sizes[size]} ${className}`}>
      {formatALL(amount)}
    </span>
  );
}
