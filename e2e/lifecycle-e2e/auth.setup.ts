import { test as setup, expect } from '@playwright/test';
import fs from 'node:fs';
import { env } from './support/env';
import { extractToken } from './support/helpers';

setup('authenticate owner & courier', async ({ browser, request }) => {
  fs.mkdirSync(env.authDir, { recursive: true });

  for (const role of ['owner', 'courier'] as const) {
    const context = await browser.newContext();
    const page = await context.newPage();

    const mockData = role === 'courier' ? { role: 'courier' } : {};
    const res = await request.post(`${env.adminBaseURL}${env.devLoginPath}`, { data: mockData });
    expect(res.ok(), `[e2e] mock-auth failed for ${role} (${res.status()})`).toBeTruthy();
    const body = await res.json();
    const token = extractToken(body);
    expect(token).toBeTruthy();
    expect(body.activeLocationId).toBeTruthy();

    if (role === 'owner') {
      fs.writeFileSync(`${env.authDir}/locationId`, body.activeLocationId, 'utf8');
    } else {
      fs.writeFileSync(`${env.authDir}/courierId`, body.userId, 'utf8');
    }

    await page.goto(env.adminBaseURL);
    await page.evaluate(
      ([k, v]) => localStorage.setItem(k, v),
      [env.authStorageKey, token] as const,
    );

    await context.storageState({ path: `${env.authDir}/${role}.json` });
    await context.close();
  }
});
