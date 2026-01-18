// Force static generation
export const dynamic = 'force-static';

export default function InstallationPage() {
  return (
    <>
      <h1>Installation</h1>

      <p>
        This guide will help you install BDP on your system. BDP currently supports
        Windows, macOS, and Linux.
      </p>

      <h2>System Requirements</h2>

      <table>
        <thead>
          <tr>
            <th>Requirement</th>
            <th>Minimum</th>
            <th>Recommended</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Operating System</td>
            <td>Windows 10, macOS 10.15, Ubuntu 20.04</td>
            <td>Windows 11, macOS 13+, Ubuntu 22.04+</td>
          </tr>
          <tr>
            <td>Memory</td>
            <td>4 GB RAM</td>
            <td>8 GB RAM</td>
          </tr>
          <tr>
            <td>Disk Space</td>
            <td>1 GB</td>
            <td>5 GB</td>
          </tr>
        </tbody>
      </table>

      <h2>Installation Methods</h2>

      <h3>Using Pre-built Binaries (Recommended)</h3>

      <p>
        Download the latest release for your platform from the{' '}
        <a href="https://github.com/bdp-dev/bdp/releases" target="_blank" rel="noopener noreferrer">
          GitHub Releases
        </a>{' '}
        page.
      </p>

      <h4>Windows</h4>

      <ol>
        <li>Download <code>bdp-windows-x64.zip</code></li>
        <li>Extract the archive to a location of your choice (e.g., <code>C:\Program Files\BDP</code>)</li>
        <li>Add the BDP directory to your PATH environment variable</li>
        <li>Verify installation by running:
          <pre><code>bdp --version</code></pre>
        </li>
      </ol>

      <h4>macOS</h4>

      <ol>
        <li>Download <code>bdp-macos-universal.tar.gz</code></li>
        <li>Extract and move to <code>/usr/local/bin</code>:
          <pre><code>{`tar -xzf bdp-macos-universal.tar.gz
sudo mv bdp /usr/local/bin/
sudo chmod +x /usr/local/bin/bdp`}</code></pre>
        </li>
        <li>Verify installation:
          <pre><code>bdp --version</code></pre>
        </li>
      </ol>

      <h4>Linux</h4>

      <ol>
        <li>Download <code>bdp-linux-x64.tar.gz</code></li>
        <li>Extract and install:
          <pre><code>{`tar -xzf bdp-linux-x64.tar.gz
sudo mv bdp /usr/local/bin/
sudo chmod +x /usr/local/bin/bdp`}</code></pre>
        </li>
        <li>Verify installation:
          <pre><code>bdp --version</code></pre>
        </li>
      </ol>

      <h3>Building from Source</h3>

      <p>
        If you prefer to build from source or need the latest development version:
      </p>

      <h4>Prerequisites</h4>

      <ul>
        <li>
          <a href="https://www.rust-lang.org/tools/install" target="_blank" rel="noopener noreferrer">
            Rust
          </a>{' '}
          (1.70 or later)
        </li>
        <li>Git</li>
        <li>PostgreSQL 15+ (for running the backend)</li>
      </ul>

      <h4>Build Steps</h4>

      <ol>
        <li>Clone the repository:
          <pre><code>git clone https://github.com/bdp-dev/bdp.git
cd bdp</code></pre>
        </li>
        <li>Build the CLI:
          <pre><code>cargo build --release --bin bdp</code></pre>
        </li>
        <li>The binary will be available at:
          <pre><code>target/release/bdp</code></pre>
        </li>
        <li>Optionally, install globally:
          <pre><code>cargo install --path crates/cli</code></pre>
        </li>
      </ol>

      <h2>Configuration</h2>

      <p>
        After installation, you can configure BDP by setting environment variables or
        using the configuration file.
      </p>

      <h3>Environment Variables</h3>

      <ul>
        <li>
          <code>BDP_API_URL</code> - Backend API URL (default: <code>http://localhost:8000</code>)
        </li>
        <li>
          <code>BDP_DATA_DIR</code> - Data directory (default: <code>~/.bdp</code>)
        </li>
      </ul>

      <h3>Configuration File</h3>

      <p>
        BDP uses a <code>bdp.yml</code> file in your project root for project-specific configuration.
        This file is created automatically when you run <code>bdp init</code>.
      </p>

      <h2>Verify Installation</h2>

      <p>
        Run the following command to verify BDP is installed correctly:
      </p>

      <pre><code>bdp --version</code></pre>

      <p>
        You should see output similar to:
      </p>

      <pre><code>bdp 0.1.0</code></pre>

      <h2>Next Steps</h2>

      <ul>
        <li>
          <a href="/docs/quick-start">Quick Start Guide</a> - Get started with BDP
        </li>
        <li>
          <a href="/docs/cli/commands">CLI Commands</a> - Learn about all available commands
        </li>
        <li>
          <a href="/docs/concepts/architecture">Architecture</a> - Understand how BDP works
        </li>
      </ul>

      <h2>Troubleshooting</h2>

      <h3>Command not found</h3>

      <p>
        If you get a "command not found" error, make sure the BDP binary is in your PATH.
      </p>

      <h3>Permission denied</h3>

      <p>
        On Unix-like systems, ensure the binary has execute permissions:
      </p>

      <pre><code>chmod +x /path/to/bdp</code></pre>

      <h3>Connection errors</h3>

      <p>
        If you encounter connection errors, verify that the backend API is running and
        the <code>BDP_API_URL</code> environment variable is set correctly.
      </p>
    </>
  );
}
