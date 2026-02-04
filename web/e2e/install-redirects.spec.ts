import { test, expect } from '@playwright/test';

test.describe('Install script redirects', () => {
  test('GET /install.sh returns 302 to GitHub shell installer', async ({
    request,
  }) => {
    const response = await request.get('/install.sh', {
      maxRedirects: 0,
    });

    // Next.js uses 307 for temporary redirects (permanent: false)
    expect(response.status()).toBe(307);
    expect(response.headers()['location']).toBe(
      'https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh',
    );
  });

  test('GET /install.ps1 returns 302 to GitHub PowerShell installer', async ({
    request,
  }) => {
    const response = await request.get('/install.ps1', {
      maxRedirects: 0,
    });

    // Next.js uses 307 for temporary redirects (permanent: false)
    expect(response.status()).toBe(307);
    expect(response.headers()['location']).toBe(
      'https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1',
    );
  });
});
