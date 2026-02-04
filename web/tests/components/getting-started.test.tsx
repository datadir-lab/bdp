import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, beforeEach } from 'vitest';
import { GettingStarted } from '@/components/shared/getting-started';
import { siteConfig } from '@/lib/site-config';

function setUserAgent(ua: string) {
  Object.defineProperty(navigator, 'userAgent', {
    value: ua,
    configurable: true,
  });
}

const UA_MACOS =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36';
const UA_WINDOWS =
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36';
const UA_LINUX =
  'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36';

describe('GettingStarted', () => {
  describe('OS auto-detection', () => {
    it('defaults to unix tab on macOS user agent', () => {
      setUserAgent(UA_MACOS);
      render(<GettingStarted />);

      // Unix prompts visible
      const prompts = screen.getAllByText('$');
      expect(prompts).toHaveLength(4);
    });

    it('defaults to unix tab on Linux user agent', () => {
      setUserAgent(UA_LINUX);
      render(<GettingStarted />);

      const prompts = screen.getAllByText('$');
      expect(prompts).toHaveLength(4);
    });

    it('auto-selects windows tab on Windows user agent', async () => {
      setUserAgent(UA_WINDOWS);
      render(<GettingStarted />);

      // useEffect fires and switches to windows
      await waitFor(() => {
        expect(screen.getAllByText('PS>')).toHaveLength(4);
      });
    });

    it('shows PowerShell label when Windows is detected', async () => {
      setUserAgent(UA_WINDOWS);
      render(<GettingStarted />);

      await waitFor(() => {
        expect(screen.getByText('PowerShell')).toBeInTheDocument();
      });
    });

    it('shows macOS window controls when unix is active', () => {
      setUserAgent(UA_MACOS);
      render(<GettingStarted />);

      // Traffic light dots should not show "PowerShell"
      expect(screen.queryByText('PowerShell')).not.toBeInTheDocument();
    });
  });

  describe('tab switching', () => {
    beforeEach(() => {
      setUserAgent(UA_MACOS);
    });

    it('switches to windows tab when clicked', async () => {
      render(<GettingStarted />);
      const user = userEvent.setup();

      await user.click(screen.getByRole('button', { name: 'Windows' }));

      expect(screen.getAllByText('PS>')).toHaveLength(4);
      expect(screen.getByText('PowerShell')).toBeInTheDocument();
    });

    it('switches back to unix tab from windows', async () => {
      render(<GettingStarted />);
      const user = userEvent.setup();

      // Go to windows
      await user.click(screen.getByRole('button', { name: 'Windows' }));
      expect(screen.getAllByText('PS>')).toHaveLength(4);

      // Back to unix
      await user.click(screen.getByRole('button', { name: 'Unix / macOS' }));
      expect(screen.getAllByText('$')).toHaveLength(4);
      expect(screen.queryByText('PowerShell')).not.toBeInTheDocument();
    });

    it('highlights the active tab', async () => {
      render(<GettingStarted />);
      const user = userEvent.setup();

      const unixTab = screen.getByRole('button', { name: 'Unix / macOS' });
      const windowsTab = screen.getByRole('button', { name: 'Windows' });

      // Unix is initially active
      expect(unixTab.className).toContain('bg-background');
      expect(windowsTab.className).not.toContain('bg-background');

      // Switch to windows
      await user.click(windowsTab);
      expect(windowsTab.className).toContain('bg-background');
      expect(unixTab.className).not.toContain('bg-background');
    });
  });

  describe('command content', () => {
    beforeEach(() => {
      setUserAgent(UA_MACOS);
    });

    it('shows unix install command on unix tab', () => {
      render(<GettingStarted />);

      expect(screen.getByText(siteConfig.install.unix)).toBeInTheDocument();
    });

    it('shows windows install command on windows tab', async () => {
      render(<GettingStarted />);
      const user = userEvent.setup();

      await user.click(screen.getByRole('button', { name: 'Windows' }));

      expect(screen.getByText(siteConfig.install.windows)).toBeInTheDocument();
    });

    it('shows all four commands on unix tab', () => {
      render(<GettingStarted />);

      expect(screen.getByText(siteConfig.install.unix)).toBeInTheDocument();
      expect(screen.getByText('bdp init')).toBeInTheDocument();
      expect(screen.getByText(/^bdp source add /)).toBeInTheDocument();
      expect(screen.getByText('bdp pull')).toBeInTheDocument();
    });

    it('shows all four commands on windows tab', async () => {
      render(<GettingStarted />);
      const user = userEvent.setup();

      await user.click(screen.getByRole('button', { name: 'Windows' }));

      expect(screen.getByText(siteConfig.install.windows)).toBeInTheDocument();
      expect(screen.getByText('bdp init')).toBeInTheDocument();
      expect(screen.getByText(/^bdp source add /)).toBeInTheDocument();
      expect(screen.getByText('bdp pull')).toBeInTheDocument();
    });

    it('shows install comments on each line', () => {
      render(<GettingStarted />);

      expect(screen.getByText('# Install BDP')).toBeInTheDocument();
      expect(screen.getByText('# Initialize project')).toBeInTheDocument();
      expect(screen.getByText('# Add data source')).toBeInTheDocument();
      expect(screen.getByText('# Download and cache')).toBeInTheDocument();
    });
  });

  describe('copy to clipboard', () => {
    beforeEach(() => {
      setUserAgent(UA_MACOS);
    });

    it('shows copy buttons on each terminal line', () => {
      render(<GettingStarted />);

      const copyButtons = screen.getAllByRole('button', { name: 'Copy to clipboard' });
      expect(copyButtons).toHaveLength(4);
    });
  });
});
