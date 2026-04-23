import { expect, test } from '@playwright/test';

test.describe('Sprawl MUD replay', () => {
  test('/sprawl loads the replay and renders all events', async ({ page }) => {
    await page.goto('/sprawl');
    const scene = page.getByTestId('sprawl-page');
    await expect(scene).toBeVisible();
    // The Rust sprawl walk emits 18 events for the Case→Loa arc.
    await expect(scene).toHaveAttribute('data-event-count', '18');
    await expect(page.getByTestId('quest-hud')).toBeVisible();
    await expect(page.getByTestId('scrub')).toBeVisible();
  });

  test('scrub moves the current index without panicking', async ({ page }) => {
    await page.goto('/sprawl');
    const scene = page.getByTestId('sprawl-page');
    await expect(scene).toHaveAttribute('data-event-count', '18');
    const scrub = page.getByTestId('scrub');
    await scrub.fill('10');
    await expect(scene).toHaveAttribute('data-current-index', '10');
    await scrub.fill('0');
    await expect(scene).toHaveAttribute('data-current-index', '0');
  });

  test('home page links to /sprawl', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByTestId('sprawl-link')).toBeVisible();
    await page.getByTestId('sprawl-link').click();
    await expect(page).toHaveURL(/\/sprawl$/);
    await expect(page.getByTestId('sprawl-page')).toBeVisible();
  });
});
