export { ProductCard } from './ProductCard.js';
export { StateChip } from './StateChip.js';
export type { StateChipState, StateChipProps } from './StateChip.js';
export { OTPModal } from './OTPModal.js';
export { OrderProgress } from './OrderProgress.js';

// The shared cart line-item shape (used by apps/web CartProvider/cartReconcile).
// Lived in CartDrawer.tsx until that dead component was removed.
export interface CartItem {
  id: string;
  productId: string;
  name: string;
  quantity: number;
  price: number;
  options?: Record<string, string[]>;
}
