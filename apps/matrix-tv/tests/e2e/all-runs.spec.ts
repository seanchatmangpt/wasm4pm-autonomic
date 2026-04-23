import { expect, test } from '@playwright/test';

/**
 * Smoke test covering every Sprawl-trilogy run, not just the 11
 * hand-crafted ones. Each run must resolve through `runBySlug` via
 * its URL, render the episode scene, and expose a verdict badge.
 *
 * These ids mirror `apps/matrix-tv/lib/runs.ts::SPRAWL_ROWS`.
 */

const ALL_RUN_IDS = [
  // Neuromancer
  'n1', 'n2', 'n3', 'n4', 'n5', 'n6', 'n7', 'n8', 'n9', 'n10',
  // Count Zero
  'cz1', 'cz2', 'cz3', 'cz4', 'cz5', 'cz6', 'cz7', 'cz8', 'cz9', 'cz10',
  // Mona Lisa Overdrive
  'mlo1', 'mlo2', 'mlo3', 'mlo4', 'mlo5', 'mlo6', 'mlo7', 'mlo8', 'mlo9', 'mlo10',
];

test.describe('All Sprawl runs — smoke', () => {
  for (const id of ALL_RUN_IDS) {
    test(`/episode/${id} renders with a verdict badge`, async ({ page }) => {
      await page.goto(`/episode/${id}`);
      await expect(page.getByTestId('episode-scene')).toBeVisible();
      const badge = page.getByTestId('verdict-badge');
      await expect(badge).toBeVisible();
      const verdict = await badge.getAttribute('data-verdict');
      expect(['lawful', 'unlawful']).toContain(verdict);
    });
  }

  test('home page links to every run', async ({ page }) => {
    await page.goto('/');
    for (const id of ALL_RUN_IDS) {
      await expect(page.getByTestId(`run-link-${id}`)).toBeVisible();
    }
  });
});
