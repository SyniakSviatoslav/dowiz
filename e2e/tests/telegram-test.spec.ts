import { test, expect } from '@playwright/test';

test.describe('Telegram Bot Tests', () => {
  test('should get bot info', async () => {
    const BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN || '8996764379:AAHkuc5mgYQdkWG5rLZEjHc8a8k5MQsHDIk';
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getMe`;
    const resp = await fetch(url);
    expect(resp.ok).toBeTruthy();
    const data = await resp.json();
    expect(data.ok).toBeTruthy();
    expect(data.result.username).toBeTruthy();
  });
  
  test('should get webhook info', async () => {
    const BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN || '8996764379:AAHkuc5mgYQdkWG5rLZEjHc8a8k5MQsHDIk';
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getWebhookInfo`;
    const resp = await fetch(url);
    expect(resp.ok).toBeTruthy();
    const data = await resp.json();
    expect(data.ok).toBeTruthy();
    console.log('Webhook info:', data.result);
  });
  
  test('should get updates', async () => {
    const BOT_TOKEN = process.env.TELEGRAM_BOT_TOKEN || '8996764379:AAHkuc5mgYQdkWG5rLZEjHc8a8k5MQsHDIk';
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getUpdates?offset=0&limit=1&timeout=0`;
    const resp = await fetch(url);
    // Don't fail on 409, just log it
    console.log(`getUpdates status: ${resp.status}`);
    if (resp.ok) {
      const data = await resp.json();
      console.log('Updates data:', data);
    } else {
      const errorData = await resp.json();
      console.log('Error data:', errorData);
    }
  });
});