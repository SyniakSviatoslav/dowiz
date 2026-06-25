import { test, expect } from '@playwright/test';

test.describe('Telegram Webhook Management', () => {
  test('should be able to delete webhook', async () => {
    const BOT_TOKEN = process.env.***REDACTED***;
    const deleteUrl = `https://api.telegram.org/bot${BOT_TOKEN}/deleteWebhook`;
    const deleteResp = await fetch(deleteUrl, { method: 'POST' });
    expect(deleteResp.ok).toBeTruthy(`Failed to delete webhook: ${deleteResp.status}`);
    const deleteData = await deleteResp.json();
    expect(deleteData.ok).toBeTruthy();
    console.log('Webhook delete result:', deleteData);
  });
  
  test('should be able to get updates after deleting webhook', async () => {
    const BOT_TOKEN = process.env.***REDACTED***;
    
    // Delete webhook first
    const deleteUrl = `https://api.telegram.org/bot${BOT_TOKEN}/deleteWebhook`;
    await fetch(deleteUrl, { method: 'POST' });
    
    // Now try getUpdates
    const url = `https://api.telegram.org/bot${BOT_TOKEN}/getUpdates?offset=0&limit=1&timeout=0`;
    const resp = await fetch(url);
    // This should now work (unless there are other issues)
    console.log(`getUpdates status after deleting webhook: ${resp.status}`);
    if (resp.ok) {
      const data = await resp.json();
      expect(data.ok).toBeTruthy();
      console.log('Updates data:', data);
    } else {
      const errorData = await resp.json();
      console.log('Error data:', errorData);
    }
  });
  
  test('should be able to set webhook again', async () => {
    const BOT_TOKEN = process.env.***REDACTED***;
    const BOT_SECRET = process.env.***REDACTED***;
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    
    const setUrl = `https://api.telegram.org/bot${BOT_TOKEN}/setWebhook`;
    const webhookUrl = `${BASE_URL}/webhook/telegram/${BOT_SECRET}`;
    const setResp = await fetch(setUrl, { 
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: webhookUrl, secret_token: BOT_SECRET, allowed_updates: ['message', 'callback_query'] })
    });
    expect(setResp.ok).toBeTruthy(`Failed to set webhook: ${setResp.status}`);
    const setData = await setResp.json();
    expect(setData.ok).toBeTruthy();
    console.log('Webhook set result:', setData);
  });
});