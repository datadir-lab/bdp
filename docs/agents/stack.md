# Technology Stack

Complete breakdown of all technologies used in BDP and rationale for each choice.

## Backend: Rust

### Why Rust?
- **Memory Safety**: Eliminates entire classes of bugs (null pointers, data races)
- **Performance**: Comparable to C/C++, crucial for dependency resolution
- **CLI Excellence**: Rust produces fast, single-binary executables
- **Ecosystem**: Excellent web frameworks (axum), HTTP clients (reqwest), async runtime (tokio)
- **Type Safety**: Strong type system catches bugs at compile time
- **Community**: Growing bioinformatics presence (rust-bio, needletail)

### Web Framework: axum 0.7

**Dependencies:**
```toml
[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }
```

**Why axum?**
- Built on tokio (most mature async runtime)
- Type-safe extractors and responses
- Excellent ergonomics
- Composable middleware (Tower)
- Active development, modern API
- Used by production projects (shuttle.rs, etc.)

**Resources:**
- [axum Documentation](https://docs.rs/axum/)
- [axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)

### Database: PostgreSQL + SQLx

**Dependencies:**
```toml
sqlx = { version = "0.7", features = [
    "runtime-tokio",
    "postgres",
    "migrate",
    "uuid",
    "chrono",
    "json"
] }
```

**Why PostgreSQL?**
- ACID guarantees for package consistency
- JSON support (JSONB) for flexible metadata
- Full-text search (tsvector/tsquery)
- Mature, battle-tested
- Excellent Rust support (sqlx, diesel)

**Why SQLx over Diesel?**
- Compile-time checked queries
- Async-first (diesel is still catching up)
- Less boilerplate for simple queries
- Flexible: raw SQL when needed, macros for safety

**Resources:**
- [SQLx Documentation](https://docs.rs/sqlx/)
- [PostgreSQL JSON Functions](https://www.postgresql.org/docs/current/functions-json.html)

### CLI Framework: clap 4.x

**Dependencies:**
```toml
clap = { version = "4.5", features = ["derive", "cargo", "env"] }
clap_complete = "4.5"  # Shell completions
```

**Why clap?**
- Derive API for ergonomic CLI definitions
- Automatic help generation
- Shell completion support
- Subcommand support
- Environment variable integration

**Resources:**
- [clap Documentation](https://docs.rs/clap/)
- [clap Derive Tutorial](https://docs.rs/clap/latest/clap/_derive/index.html)

### Additional Rust Crates

#### HTTP Client
```toml
reqwest = { version = "0.11", features = ["json", "stream"] }
```
For CLI to communicate with registry API.

#### Serialization
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"  # For bdp.toml parsing
```

#### Security
```toml
argon2 = "0.5"          # Password hashing
jsonwebtoken = "9"      # JWT tokens
sha2 = "0.10"          # SHA-256 checksums
```

#### Terminal UI (CLI)
```toml
indicatif = "0.17"      # Progress bars
console = "0.15"        # Terminal colors/formatting
dialoguer = "0.11"      # Interactive prompts
```

#### Background Jobs
```toml
apalis = { version = "0.5", features = ["postgres"] }
```
For async job processing (indexing, security scans).

#### Logging
```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Resources:**
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Async Book](https://rust-lang.github.io/async-book/)

## Frontend: Next.js 16 / Nextra

### Framework: Next.js 16

**Package.json:**
```json
{
  "dependencies": {
    "next": "^16.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  }
}
```

**Why Next.js 16?**
- Latest stable version with enhanced performance
- React Server Components (RSC) by default
- App Router (mature and optimized)
- Built-in TypeScript support
- Server-side rendering (SEO)
- API routes (if needed for BFF pattern)
- Incremental Static Regeneration (ISR)
- Image optimization
- Improved caching and revalidation
- Stable Turbopack for faster builds

**Key Features for BDP:**
- Server Components: Reduce client bundle size
- Streaming: Better UX for package search results
- Metadata API: SEO for package pages
- Route handlers: Proxy to Rust API if needed
- Enhanced turbopack performance

**Resources:**
- [Next.js 16 Documentation](https://nextjs.org/docs)
- [Next.js 16 Release Notes](https://nextjs.org/blog/next-16)
- [React Server Components](https://nextjs.org/docs/app/building-your-application/rendering/server-components)
- [App Router Guide](https://nextjs.org/docs/app)

### Documentation: Nextra

**Dependencies:**
```json
{
  "dependencies": {
    "nextra": "^3.0.0",
    "nextra-theme-docs": "^3.0.0"
  }
}
```

**Why Nextra?**
- Built on Next.js
- MDX support (Markdown + React components)
- Built-in search (FlexSearch)
- Syntax highlighting (Shiki)
- Perfect for package documentation
- Mobile-responsive

**Resources:**
- [Nextra Documentation](https://nextra.site/)
- [Nextra Theme Docs](https://nextra.site/docs/docs-theme/start)

### UI Framework: Tailwind CSS + Radix UI

**Dependencies:**
```json
{
  "devDependencies": {
    "tailwindcss": "^3.4",
    "autoprefixer": "^10",
    "postcss": "^8"
  },
  "dependencies": {
    "@radix-ui/react-dialog": "^1.0",
    "@radix-ui/react-dropdown-menu": "^2.0",
    "@radix-ui/react-select": "^2.0",
    "@radix-ui/react-tabs": "^1.0"
  }
}
```

**Why Tailwind?**
- Utility-first CSS
- Minimal CSS bundle (purges unused)
- Consistent design system
- Fast prototyping

**Why Radix UI?**
- Unstyled, accessible components
- ARIA-compliant
- Keyboard navigation
- Style with Tailwind

**Resources:**
- [Tailwind CSS](https://tailwindcss.com/docs)
- [Radix UI](https://www.radix-ui.com/primitives)

### Data Fetching: SWR

**Dependencies:**
```json
{
  "dependencies": {
    "swr": "^2.2"
  }
}
```

**Why SWR?**
- Stale-while-revalidate strategy
- Built-in caching
- Optimistic updates
- Works great with RSC + client components

**Alternative:** TanStack Query (more features, larger bundle)

**Resources:**
- [SWR Documentation](https://swr.vercel.app/)

### TypeScript

**tsconfig.json:**
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "jsx": "preserve",
    "module": "esnext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
```

**Resources:**
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
- [Next.js TypeScript](https://nextjs.org/docs/app/building-your-application/configuring/typescript)

## Database: PostgreSQL

**Version:** PostgreSQL 16+

**Why PostgreSQL 16?**
- Latest stable version
- Improved performance (parallel queries)
- Better JSON/JSONB performance
- Logical replication improvements

**Essential Extensions:**
```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";  -- UUID generation
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- Fuzzy search
```

**Resources:**
- [PostgreSQL 16 Documentation](https://www.postgresql.org/docs/16/)
- [PostgreSQL JSON Functions](https://www.postgresql.org/docs/16/functions-json.html)

## Object Storage

### Options

**1. MinIO (Recommended for self-hosted)**
```toml
# Rust client
minio = "0.1"
# OR generic S3
aws-sdk-s3 = "1.0"
```

**Why MinIO?**
- S3-compatible API
- Self-hostable
- High performance
- Open source

**2. AWS S3 / Cloudflare R2**
For production with CDN.

**Resources:**
- [MinIO Documentation](https://min.io/docs/)
- [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/)

## Development Tools

### Database Migrations

**SQLx CLI:**
```bash
cargo install sqlx-cli --no-default-features --features postgres
```

**Usage:**
```bash
sqlx migrate add create_packages_table
sqlx migrate run
```

### Code Quality

**Formatter:**
```bash
cargo fmt
```

**Linter:**
```bash
cargo clippy -- -D warnings
```

**Next.js:**
```json
{
  "scripts": {
    "lint": "next lint",
    "format": "prettier --write ."
  }
}
```

## Deployment

### Reverse Proxy: Caddy

**Why Caddy?**
- Automatic HTTPS (Let's Encrypt)
- Simple configuration
- HTTP/2, HTTP/3 support
- Reverse proxy built-in

**Caddyfile:**
```
registry.bdp.dev {
    reverse_proxy localhost:3000
}

api.bdp.dev {
    reverse_proxy localhost:8000
}
```

**Resources:**
- [Caddy Documentation](https://caddyserver.com/docs/)

### Process Management: systemd

**Why systemd?**
- Standard on Linux
- Automatic restarts
- Log management (journalctl)
- Dependency management

## Summary

| Component | Technology | Why |
|-----------|-----------|-----|
| Backend API | Rust + axum | Performance, safety, modern async |
| CLI | Rust + clap | Single binary, cross-platform |
| Database | PostgreSQL 16 | ACID, JSONB, full-text search |
| ORM | SQLx | Compile-time checked, async |
| Frontend | Next.js 15 | RSC, SEO, modern React |
| Docs | Nextra | MDX, built-in search |
| UI | Tailwind + Radix | Fast styling, accessibility |
| Storage | MinIO/S3 | Scalable object storage |
| Proxy | Caddy | Automatic HTTPS |
| Process | systemd | Reliability, logs |

---

**Next**: See [Rust Backend](./rust-backend.md) for implementation patterns.
