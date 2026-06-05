import { AuthToken } from '@deliveryos/shared-types';

export function getRoleFromToken(token: AuthToken) {
  return token.role;
}
