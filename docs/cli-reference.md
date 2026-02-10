   Compiling ring v0.17.14
   Compiling rustls v0.23.36
   Compiling rustls-webpki v0.103.9
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.7
   Compiling reqwest v0.12.28
   Compiling bdp-cli v0.1.38 (D:\dev\datadir\bdp\crates\bdp-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 21s
     Running `C:\tmp_target\debug\bdp.exe --markdown-help`
# Command-Line Help for `bdp`

This document contains the help content for the `bdp` command-line program.

**Command Overview:**

* [`bdp`↴](#bdp)
* [`bdp init`↴](#bdp-init)
* [`bdp source`↴](#bdp-source)
* [`bdp source add`↴](#bdp-source-add)
* [`bdp source remove`↴](#bdp-source-remove)
* [`bdp source list`↴](#bdp-source-list)
* [`bdp pull`↴](#bdp-pull)
* [`bdp status`↴](#bdp-status)
* [`bdp audit`↴](#bdp-audit)
* [`bdp audit list`↴](#bdp-audit-list)
* [`bdp audit verify`↴](#bdp-audit-verify)
* [`bdp audit export`↴](#bdp-audit-export)
* [`bdp clean`↴](#bdp-clean)
* [`bdp config`↴](#bdp-config)
* [`bdp config get`↴](#bdp-config-get)
* [`bdp config set`↴](#bdp-config-set)
* [`bdp config show`↴](#bdp-config-show)
* [`bdp uninstall`↴](#bdp-uninstall)
* [`bdp search`↴](#bdp-search)
* [`bdp cache`↴](#bdp-cache)
* [`bdp cache set`↴](#bdp-cache-set)
* [`bdp cache show`↴](#bdp-cache-show)
* [`bdp cache reset`↴](#bdp-cache-reset)
* [`bdp generate`↴](#bdp-generate)
* [`bdp generate python`↴](#bdp-generate-python)
* [`bdp generate snakemake`↴](#bdp-generate-snakemake)
* [`bdp generate nextflow`↴](#bdp-generate-nextflow)
* [`bdp query`↴](#bdp-query)

## `bdp`

BDP - Biological Dataset Package Manager

**Usage:** `bdp [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `init` — Initialize a new BDP project
* `source` — Manage data sources
* `pull` — Download and cache sources from manifest
* `status` — Show status of cached sources
* `audit` — Audit trail management
* `clean` — Clean cache
* `config` — Manage configuration
* `uninstall` — Uninstall BDP from your system
* `search` — Search for data sources and tools in the registry
* `cache` — Manage local data cache directory
* `generate` — Generate workflow integration files (Python, Snakemake, Nextflow)
* `query` — Advanced SQL-like querying of data sources and metadata

###### **Options:**

* `-v`, `--verbose` — Verbose output
* `--server-url <SERVER_URL>` — Server URL

  Default value: `http://localhost:8000`



## `bdp init`

Initialize a new BDP project

**Usage:** `bdp init [OPTIONS] [PATH]`

###### **Arguments:**

* `<PATH>` — Project directory (defaults to current directory)

  Default value: `.`

###### **Options:**

* `-n`, `--name <NAME>` — Project name (defaults to directory name)
* `-V`, `--version <VERSION>` — Project version

  Default value: `0.1.0`
* `-d`, `--description <DESCRIPTION>` — Project description
* `-f`, `--force` — Force overwrite if bdp.yml exists



## `bdp source`

Manage data sources

**Usage:** `bdp source <COMMAND>`

###### **Subcommands:**

* `add` — Add a source to the manifest
* `remove` — Remove a source from the manifest
* `list` — List sources in the manifest



## `bdp source add`

Add a source to the manifest

**Usage:** `bdp source add <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — Source specification (e.g., "uniprot:P01308-fasta@1.0")



## `bdp source remove`

Remove a source from the manifest

**Usage:** `bdp source remove <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — Source specification



## `bdp source list`

List sources in the manifest

**Usage:** `bdp source list`



## `bdp pull`

Download and cache sources from manifest

**Usage:** `bdp pull [OPTIONS]`

###### **Options:**

* `-f`, `--force` — Force re-download even if cached



## `bdp status`

Show status of cached sources

**Usage:** `bdp status`



## `bdp audit`

Audit trail management

**Usage:** `bdp audit <COMMAND>`

###### **Subcommands:**

* `list` — List audit events
* `verify` — Verify audit trail integrity
* `export` — Export audit trail to regulatory format



## `bdp audit list`

List audit events

**Usage:** `bdp audit list [OPTIONS]`

###### **Options:**

* `-l`, `--limit <LIMIT>` — Limit number of events to show

  Default value: `20`
* `-s`, `--source <SOURCE>` — Show events from specific source



## `bdp audit verify`

Verify audit trail integrity

**Usage:** `bdp audit verify`



## `bdp audit export`

Export audit trail to regulatory format

**Usage:** `bdp audit export [OPTIONS]`

###### **Options:**

* `-f`, `--format <FORMAT>` — Export format (fda, nih, ema, das, json)

  Default value: `fda`
* `-o`, `--output <OUTPUT>` — Output file path (optional, defaults to audit-{format}.{ext})
* `--from <FROM>` — Filter events from date (ISO 8601)
* `--to <TO>` — Filter events to date (ISO 8601)
* `-n`, `--project-name <PROJECT_NAME>` — Project name for report
* `-v`, `--project-version <PROJECT_VERSION>` — Project version for report



## `bdp clean`

Clean cache

**Usage:** `bdp clean [OPTIONS]`

###### **Options:**

* `-a`, `--all` — Clean all cached files
* `--search-cache` — Clean only search cache



## `bdp config`

Manage configuration

**Usage:** `bdp config <COMMAND>`

###### **Subcommands:**

* `get` — Get configuration value
* `set` — Set configuration value
* `show` — Show all configuration



## `bdp config get`

Get configuration value

**Usage:** `bdp config get <KEY>`

###### **Arguments:**

* `<KEY>` — Configuration key



## `bdp config set`

Set configuration value

**Usage:** `bdp config set <KEY> <VALUE>`

###### **Arguments:**

* `<KEY>` — Configuration key
* `<VALUE>` — Configuration value



## `bdp config show`

Show all configuration

**Usage:** `bdp config show`



## `bdp uninstall`

Uninstall BDP from your system

**Usage:** `bdp uninstall [OPTIONS]`

###### **Options:**

* `-y`, `--yes` — Skip confirmation prompt
* `--purge` — Also remove cache and configuration files



## `bdp search`

Search for data sources and tools in the registry

**Usage:** `bdp search [OPTIONS] <QUERY>...`

###### **Arguments:**

* `<QUERY>` — Search query (multiple words will be joined)

###### **Options:**

* `-o`, `--org <ORG>` — Filter by organization (e.g., uniprot, ncbi)
* `-t`, `--type <ENTRY_TYPE>` — Filter by entry type (can be repeated)
* `-s`, `--source-type <SOURCE_TYPE>` — Filter by source type (can be repeated)
* `-f`, `--format <FORMAT>` — Output format

  Default value: `interactive`
* `--no-interactive` — Force non-interactive mode
* `-l`, `--limit <LIMIT>` — Number of results per page (1-100)

  Default value: `10`
* `-p`, `--page <PAGE>` — Page number (for non-interactive pagination)

  Default value: `1`



## `bdp cache`

Manage local data cache directory

**Usage:** `bdp cache <COMMAND>`

###### **Subcommands:**

* `set` — Set cache directory path
* `show` — Show current cache directory
* `reset` — Reset cache path to default (.bdp/data)



## `bdp cache set`

Set cache directory path

**Usage:** `bdp cache set <PATH>`

###### **Arguments:**

* `<PATH>` — Path to cache directory (relative to project root, or absolute)



## `bdp cache show`

Show current cache directory

**Usage:** `bdp cache show`



## `bdp cache reset`

Reset cache path to default (.bdp/data)

**Usage:** `bdp cache reset`



## `bdp generate`

Generate workflow integration files (Python, Snakemake, Nextflow)

**Usage:** `bdp generate <COMMAND>`

###### **Subcommands:**

* `python` — Generate Python data paths module (bdp_data.py)
* `snakemake` — Generate Snakemake config file (config/bdp_data.yaml)
* `nextflow` — Generate Nextflow config file (conf/bdp_data.config)



## `bdp generate python`

Generate Python data paths module (bdp_data.py)

**Usage:** `bdp generate python`



## `bdp generate snakemake`

Generate Snakemake config file (config/bdp_data.yaml)

**Usage:** `bdp generate snakemake`



## `bdp generate nextflow`

Generate Nextflow config file (conf/bdp_data.config)

**Usage:** `bdp generate nextflow`



## `bdp query`

Advanced SQL-like querying of data sources and metadata

**Usage:** `bdp query [OPTIONS] [ENTITY]`

###### **Arguments:**

* `<ENTITY>` — Entity to query (protein, gene, genome, tools, orgs, etc.) or use --sql for raw SQL

###### **Options:**

* `--select <SELECT>` — Select specific fields (comma-separated)
* `-w`, `--where <WHERE_CLAUSE>` — Filter results (can be repeated, AND combined) Simple: --where organism=human Complex: --where "organism='human' AND downloads>1000"
* `--order-by <ORDER_BY>` — Sort results by field[:asc|desc]
* `-l`, `--limit <LIMIT>` — Limit number of results (default: 1000)

  Default value: `1000`
* `--offset <OFFSET>` — Skip first N results
* `--group-by <GROUP_BY>` — Group results by field
* `--aggregate <AGGREGATE>` — Aggregation expression (COUNT(*), SUM(field), etc.)
* `--having <HAVING>` — Filter grouped results
* `--join <JOIN>` — Join with another entity/table
* `--on <ON>` — Join condition
* `--sql <SQL>` — Execute raw SQL query directly
* `-f`, `--format <FORMAT>` — Output format
* `-o`, `--output <OUTPUT>` — Write output to file instead of stdout
* `--no-header` — Omit header row (for CSV/TSV)
* `--explain` — Show query execution plan
* `--dry-run` — Show generated SQL without executing



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

