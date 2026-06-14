import { test, expect } from '@playwright/test';

test.describe('Simple Auth Test', () => {
  test('should be able to get owner token', async () => {
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    console.log(`Testing with BASE_URL: ${BASE_URL}`);
    
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    console.log(`Auth response status: ${authRes.status}`);
    
    expect(authRes.status).toBe(200);
    const authBody = await authRes.json();
    expect(authBody.access_token).toBeTruthy();
    console.log('Successfully got token');
  });
});