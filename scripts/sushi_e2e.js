const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 }, // iPhone 14
    recordVideo: { dir: '/tmp/sushi_screenshots/', size: { width: 390, height: 844 } }
  });
  const page = await context.newPage();
  
  const URL = process.env.SUSHI_URL || 'http://localhost:8080';
  await page.goto(URL, { waitUntil: 'networkidle' });
  
  // Screenshot 1: Customer menu
  await page.screenshot({ path: '/tmp/sushi_screenshots/01_customer_menu.png', fullPage: true });
  
  // Screenshot 2: Add to cart
  const addButtons = await page.$$('button:has-text("Add")');
  if (addButtons.length > 0) await addButtons[0].click();
  await page.screenshot({ path: '/tmp/sushi_screenshots/02_cart.png', fullPage: true });
  
  // Screenshot 3: Switch to Courier
  const courierBtn = await page.$('[data-role="courier"]');
  if (courierBtn) await courierBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: '/tmp/sushi_screenshots/03_courier.png', fullPage: true });
  
  // Screenshot 4: Switch to Restaurant
  const restBtn = await page.$('[data-role="restaurant"]');
  if (restBtn) await restBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: '/tmp/sushi_screenshots/04_restaurant.png', fullPage: true });
  
  // Screenshot 5: Back to Customer, open assistant
  const custBtn = await page.$('[data-role="customer"]');
  if (custBtn) await custBtn.click();
  const asstBtn = await page.$('[data-assistant="toggle"]');
  if (asstBtn) await asstBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: '/tmp/sushi_screenshots/05_assistant.png', fullPage: true });
  
  // Screenshot 6: Switch to Arabic
  const arBtn = await page.$('[data-lang="ar"]');
  if (arBtn) await arBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: '/tmp/sushi_screenshots/06_arabic_rtl.png', fullPage: true });
  
  // Screenshot 7: Switch to Albanian
  const sqBtn = await page.$('[data-lang="sq"]');
  if (sqBtn) await sqBtn.click();
  await page.waitForTimeout(500);
  await page.screenshot({ path: '/tmp/sushi_screenshots/07_albanian.png', fullPage: true });
  
  // Close video
  await context.close();
  await browser.close();
  
  console.log(`Screenshots saved to /tmp/sushi_screenshots/`);
  const { execSync } = require('child_process');
  console.log(`Files: ${execSync('ls /tmp/sushi_screenshots/*.png | wc -l').toString().trim()} screenshots`);
})();
