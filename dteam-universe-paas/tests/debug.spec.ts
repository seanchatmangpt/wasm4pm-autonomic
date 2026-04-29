import { test } from '@playwright/test';

test('Debug HTML', async ({ page }) => {
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(5000);
  const html = await page.evaluate(() => document.body.innerHTML);
  expect(html).toBeTruthy();
});