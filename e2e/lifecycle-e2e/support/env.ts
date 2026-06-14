function required(name: string, fallback?: string): string {
  const v = process.env[name] ?? fallback;
  if (!v) throw new Error(`[e2e] Missing env var ${name}. See README.md.`);
  return v;
}

export const env = {
  customerBaseURL: required('E2E_BASE_URL'),
  adminBaseURL: process.env.E2E_ADMIN_URL ?? required('E2E_BASE_URL'),
  courierBaseURL: process.env.E2E_COURIER_URL ?? required('E2E_BASE_URL'),

  restaurantSlug: required('E2E_RESTAURANT_SLUG'),

  owner: { email: required('E2E_OWNER_EMAIL'), password: required('E2E_OWNER_PASSWORD') },
  courier: { email: required('E2E_COURIER_EMAIL'), password: required('E2E_COURIER_PASSWORD') },

  testPhone: process.env.E2E_TEST_PHONE ?? '+355691234567',

  // Dowiz uses POST /api/dev/mock-auth → { access_token, userId, activeLocationId }
  // Token stored in localStorage key 'dos_access_token'
  devLoginPath: '/api/dev/mock-auth',
  authStorageKey: 'dos_access_token',
  useUiLogin: process.env.USE_UI_LOGIN === '1',

  // Tirana defaults
  restaurantGeo: { latitude: 41.3275, longitude: 19.8187 },
  customerGeo: { latitude: 41.3302, longitude: 19.8149 },

  authDir: '.auth',
} as const;

export type Role = 'owner' | 'courier';
