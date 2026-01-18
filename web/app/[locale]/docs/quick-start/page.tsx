// Force static generation
export const dynamic = 'force-static';

export default function QuickStartPage() {
  return (
    <>
      <h1>Quick Start</h1>

      <p>
        This guide will help you get started with BDP in just a few minutes.
        We'll walk through creating your first project and adding data sources.
      </p>

      <h2>Prerequisites</h2>

      <p>
        Before you begin, make sure you have:
      </p>

      <ul>
        <li>BDP CLI installed (see <a href="/docs/installation">Installation Guide</a>)</li>
        <li>Basic familiarity with command-line tools</li>
      </ul>

      <h2>Step 1: Initialize a New Project</h2>

      <p>
        Create a new BDP project by running:
      </p>

      <pre><code>bdp init --name my-bioinformatics-project</code></pre>

      <p>
        This will create:
      </p>

      <ul>
        <li><code>bdp.yml</code> - Project configuration file</li>
        <li><code>.bdp/</code> - Local data directory</li>
        <li><code>.gitignore</code> - Git ignore rules for BDP files</li>
      </ul>

      <h2>Step 2: Add a Data Source</h2>

      <p>
        Let's add a UniProt protein sequence as an example:
      </p>

      <pre><code>bdp source add uniprot:P01308-fasta@1.0</code></pre>

      <p>
        This command:
      </p>

      <ul>
        <li>Adds the UniProt entry P01308 (human insulin) to your project</li>
        <li>Requests the FASTA format</li>
        <li>Specifies version 1.0</li>
      </ul>

      <h2>Step 3: View Your Sources</h2>

      <p>
        List all configured sources:
      </p>

      <pre><code>bdp source list</code></pre>

      <p>
        You should see output similar to:
      </p>

      <pre><code>{`Sources in project:
- uniprot:P01308-fasta@1.0`}</code></pre>

      <h2>Step 4: Pull Data</h2>

      <p>
        Download the data from configured sources:
      </p>

      <pre><code>bdp pull</code></pre>

      <p>
        This will:
      </p>

      <ul>
        <li>Connect to the BDP backend</li>
        <li>Download the specified data</li>
        <li>Store it in your local <code>.bdp/</code> directory</li>
      </ul>

      <h2>Step 5: Check Status</h2>

      <p>
        Verify your project status:
      </p>

      <pre><code>bdp status</code></pre>

      <p>
        This shows:
      </p>

      <ul>
        <li>Downloaded sources</li>
        <li>Available updates</li>
        <li>Local storage usage</li>
      </ul>

      <h2>Working with Tools</h2>

      <p>
        BDP also supports tools and packages. Here's how to add a tool:
      </p>

      <pre><code>bdp tool add blast@2.14.0</code></pre>

      <p>
        List all tools:
      </p>

      <pre><code>bdp tool list</code></pre>

      <h2>Project Configuration</h2>

      <p>
        Your <code>bdp.yml</code> file contains your project configuration:
      </p>

      <pre><code>{`name: my-bioinformatics-project
version: "1.0"

sources:
  - id: uniprot:P01308-fasta@1.0
    format: fasta

tools:
  - id: blast@2.14.0`}</code></pre>

      <p>
        You can edit this file directly or use the CLI commands to manage it.
      </p>

      <h2>Audit Trail</h2>

      <p>
        View your project's audit trail:
      </p>

      <pre><code>bdp audit</code></pre>

      <p>
        This shows all changes made to your project, which is crucial for reproducibility.
      </p>

      <h2>Next Steps</h2>

      <p>
        Now that you have a basic project set up, explore more features:
      </p>

      <ul>
        <li>
          <a href="/docs/concepts/sources">Learn about Data Sources</a> - Understand different source types
        </li>
        <li>
          <a href="/docs/concepts/tools">Learn about Tools</a> - Manage bioinformatics tools
        </li>
        <li>
          <a href="/docs/cli/commands">CLI Command Reference</a> - Explore all available commands
        </li>
        <li>
          <a href="/docs/concepts/architecture">Architecture</a> - Understand how BDP works
        </li>
      </ul>

      <h2>Common Patterns</h2>

      <h3>Adding Multiple Sources</h3>

      <pre><code>{`bdp source add uniprot:P01308-fasta@1.0
bdp source add pdb:1A2B@1.0
bdp source add ncbi-taxonomy:9606@1.0`}</code></pre>

      <h3>Removing a Source</h3>

      <pre><code>bdp source remove uniprot:P01308-fasta@1.0</code></pre>

      <h3>Updating All Sources</h3>

      <pre><code>bdp pull --update</code></pre>

      <h2>Getting Help</h2>

      <p>
        For any command, use the <code>--help</code> flag:
      </p>

      <pre><code>bdp --help
bdp source --help
bdp source add --help</code></pre>

      <p>
        Need more assistance? Check out our <a href="https://github.com/bdp-dev/bdp/discussions" target="_blank" rel="noopener noreferrer">community discussions</a>.
      </p>
    </>
  );
}
