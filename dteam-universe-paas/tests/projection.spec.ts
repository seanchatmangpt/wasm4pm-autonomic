import { test, expect } from '@playwright/test';

test.describe('UniverseOS // Projection Verification', () => {
  test('Accretion disc emits verifiable thermal radiation pixels', async ({ page }) => {
    // 1. Navigate to the projection surface
    await page.goto('http://localhost:3000');

    // 2. Wait for the canvas to mount
    const canvas = page.locator('canvas');
    await expect(canvas).toBeAttached({ timeout: 10000 });

    // 3. Allow 2 seconds for Three.js scene initialization and GLSL shader compilation
    await page.waitForTimeout(2000);

    // 4. Extract and mathematically verify the WebGL pixel buffer
    const emissionStats = await page.evaluate(() => {
      const cvs = document.querySelector('canvas');
      if (!cvs) return { found: false, count: 0, reason: "canvas not found" };

      // Create a hidden 2D context to dump the WebGL pixels into
      const ctx = document.createElement('canvas').getContext('2d');
      if (!ctx) return { found: false, count: 0, reason: "ctx not found" };

      ctx.canvas.width = cvs.width;
      ctx.canvas.height = cvs.height;
      ctx.drawImage(cvs, 0, 0);

      // Extract raw RGBA array
      const imageData = ctx.getImageData(0, 0, cvs.width, cvs.height);
      const data = imageData.data;

      let thermalPixelCount = 0;

      // Scan every pixel for the accretion disc's thermal emission signature
      for (let i = 0; i < data.length; i += 4) {
        const r = data[i];     // Red
        const g = data[i + 1]; // Green
        const b = data[i + 2]; // Blue

        // The GLSL shader maps intensity to a thermal ramp. 
        // We look for the distinct orange/yellow emission bands:
        // High Red, Medium/High Green, Low Blue.
        const isOrangeOrYellow = (r > 150 && g > 50 && b < 100);
        const isWhiteHot = (r > 200 && g > 200 && b > 150);

        if (isOrangeOrYellow || isWhiteHot) {
          thermalPixelCount++;
        }
      }

      return {
        found: thermalPixelCount > 500, // We expect thousands of pixels in the disc
        count: thermalPixelCount
      };
    });

    // 5. Assert thermal pixel count is measurable (logged by Playwright reporter)
    expect(emissionStats.count).toBeGreaterThanOrEqual(0);

    // 6. Assert that the accretion disc is actively rendering
    expect(emissionStats.found).toBe(true);
  });

  test('HUD renders the verifiable receipt parameters', async ({ page }) => {
    await page.goto('http://localhost:3000');
    
    // Verify the UI is projecting the unbroken causal receipt chain
    await expect(page.getByText('Verified Projection')).toBeVisible();
    await expect(page.getByText('RECEIPT_ID')).toBeVisible();
    await expect(page.getByText('INPUT_HASH')).toBeVisible();
    await expect(page.getByText('0xbf58476d1ce4e5b9')).toBeVisible();
    await expect(page.getByText('Lawful Motion Admitted')).toBeVisible();
  });
});