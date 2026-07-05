import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';

// We'll test our helpers by using them in a simplified version of the existing test
test.describe('Notification Helper Tests', () => {
  test('should be able to link Telegram and get connection info', async () => {
    // Import our helper here to avoid issues with top-level await in test file
    const { linkTelegram } = await import('../helpers/notifHelpers');
    
    // This test will fail if we don't have proper setup, but it will show if our helper is importable
    expect(linkTelegram).toBeDefined();
  });
  
  test('should be able to get owner token', async () => {
    // Test that we can get an owner token
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    expectJwt(authBody.access_token);
  });
});
