import { CreateOrderInput } from '@deliveryos/shared-types';
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import { loadEnv } from '@deliveryos/config';

// Ensure config is loaded
// loadEnv(); // uncomment when actually starting the server

export function planOrder(input: CreateOrderInput) {
  console.log(`Planning order for ${input.customer.phone}`);
}
