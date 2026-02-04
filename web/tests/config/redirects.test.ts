import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import path from 'path';

/**
 * Test the redirect configuration defined in next.config.js.
 *
 * Since next.config.js requires next-intl/plugin which doesn't work in jsdom,
 * we parse the config file to verify the redirects are correctly defined.
 * This is a static analysis test that ensures the config won't drift.
 */
describe('next.config.js redirects', () => {
  const configSource = readFileSync(
    path.resolve(__dirname, '../../next.config.js'),
    'utf-8',
  );

  it('defines /install.sh redirect to GitHub shell installer', () => {
    expect(configSource).toContain("source: '/install.sh'");
    expect(configSource).toContain(
      "destination:\n          'https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.sh'",
    );
  });

  it('defines /install.ps1 redirect to GitHub PowerShell installer', () => {
    expect(configSource).toContain("source: '/install.ps1'");
    expect(configSource).toContain(
      "destination:\n          'https://github.com/datadir-lab/bdp/releases/latest/download/bdp-installer.ps1'",
    );
  });

  it('uses temporary (non-permanent) redirects', () => {
    // Both redirects should use permanent: false (302/307) so they always resolve to latest
    const permanentMatches = configSource.match(/permanent:\s*(true|false)/g);
    expect(permanentMatches).toHaveLength(2);
    expect(permanentMatches).toEqual(['permanent: false', 'permanent: false']);
  });

  it('has an async redirects() function in the config', () => {
    expect(configSource).toMatch(/async\s+redirects\s*\(\)/);
  });
});
