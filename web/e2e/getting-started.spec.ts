import { test, expect } from '@playwright/test';

test.describe('Getting Started terminal - OS detection', () => {
  test('selects Unix tab by default on macOS user agent', async ({ browser }) => {
    const context = await browser.newContext({
      userAgent:
        'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    const page = await context.newPage();
    await page.goto('/');

    // Unix prompts should be visible
    await expect(page.locator('span').filter({ hasText: /^\$$/  }).first()).toBeVisible();

    // PowerShell label should not be present
    await expect(page.getByText('PowerShell')).not.toBeVisible();

    // Unix tab should have active styling
    const unixTab = page.getByRole('button', { name: 'Unix / macOS' });
    await expect(unixTab).toHaveClass(/bg-background/);

    await context.close();
  });

  test('auto-selects Windows tab on Windows user agent', async ({ browser }) => {
    const context = await browser.newContext({
      userAgent:
        'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    const page = await context.newPage();
    await page.goto('/');

    // PS> prompts should appear after useEffect
    await expect(page.locator('span').filter({ hasText: /^PS>$/ }).first()).toBeVisible();

    // PowerShell label should be visible
    await expect(page.getByText('PowerShell')).toBeVisible();

    // Windows tab should have active styling
    const windowsTab = page.getByRole('button', { name: 'Windows' });
    await expect(windowsTab).toHaveClass(/bg-background/);

    await context.close();
  });

  test('selects Unix tab on Linux user agent', async ({ browser }) => {
    const context = await browser.newContext({
      userAgent:
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    const page = await context.newPage();
    await page.goto('/');

    await expect(page.locator('span').filter({ hasText: /^\$$/ }).first()).toBeVisible();
    await expect(page.getByText('PowerShell')).not.toBeVisible();

    await context.close();
  });
});

test.describe('Getting Started terminal - tab switching', () => {
  test('switches from Unix to Windows tab', async ({ page }) => {
    await page.goto('/');

    // Initially on Unix
    await expect(page.locator('span').filter({ hasText: /^\$$/ }).first()).toBeVisible();

    // Click Windows tab
    await page.getByRole('button', { name: 'Windows' }).click();

    // Should now show PS> prompts
    await expect(page.locator('span').filter({ hasText: /^PS>$/ }).first()).toBeVisible();
    await expect(page.getByText('PowerShell')).toBeVisible();
  });

  test('switches from Windows back to Unix tab', async ({ page }) => {
    await page.goto('/');

    // Switch to Windows
    await page.getByRole('button', { name: 'Windows' }).click();
    await expect(page.locator('span').filter({ hasText: /^PS>$/ }).first()).toBeVisible();

    // Switch back to Unix
    await page.getByRole('button', { name: 'Unix / macOS' }).click();
    await expect(page.locator('span').filter({ hasText: /^\$$/ }).first()).toBeVisible();
    await expect(page.getByText('PowerShell')).not.toBeVisible();
  });
});

test.describe('Getting Started terminal - content', () => {
  test('shows correct unix install command', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('code').filter({ hasText: 'curl -sSfL' })).toBeVisible();
    await expect(page.locator('code').filter({ hasText: 'bdp init' })).toBeVisible();
    await expect(page.locator('code').filter({ hasText: 'bdp source add' })).toBeVisible();
    await expect(page.locator('code').filter({ hasText: 'bdp pull' })).toBeVisible();
  });

  test('shows correct windows install command', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Windows' }).click();

    await expect(page.locator('code').filter({ hasText: 'irm' })).toBeVisible();
    await expect(page.locator('code').filter({ hasText: 'bdp init' })).toBeVisible();
    await expect(page.locator('code').filter({ hasText: 'bdp source add' })).toBeVisible();
    await expect(page.locator('code').filter({ hasText: 'bdp pull' })).toBeVisible();
  });

  test('shows copy buttons for each terminal line', async ({ page }) => {
    await page.goto('/');

    const copyButtons = page.getByRole('button', { name: 'Copy to clipboard' });
    await expect(copyButtons).toHaveCount(4);
  });
});
