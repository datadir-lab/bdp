# Pagefind Search Integration - CI/CD Documentation

This document explains how the Pagefind search indexing is integrated into the BDP web application and CI/CD pipeline.

## Overview

The BDP documentation (`/docs`) includes a full-text search feature powered by [Pagefind](https://pagefind.app/), a static site search library. The search index is automatically generated during the build process.

## How It Works

### Local Development

1. **Build the application:**
   ```bash
   cd web
   yarn build
   ```

2. **Automatic index generation:**
   - Next.js builds the application to `.next/`
   - The `postbuild` npm hook automatically runs after the build completes
   - Pagefind scans `.next/server/app` for content marked with `data-pagefind-body`
   - Search index is generated and output to `public/_pagefind/`

3. **Configuration in `package.json`:**
   ```json
   {
     "scripts": {
       "build": "next build",
       "postbuild": "npx pagefind --site .next/server/app --output-path public/_pagefind"
     }
   }
   ```

### CI/CD Pipeline

#### GitHub Actions Workflow (`.github/workflows/ci.yml`)

The frontend build verification job includes Pagefind indexing:

```yaml
- name: Install dependencies
  run: cd web && yarn install --frozen-lockfile

- name: Build frontend (includes Pagefind indexing)
  run: just build-web

- name: Verify Pagefind index was created
  run: |
    if [ -d "web/public/_pagefind" ]; then
      echo "✓ Pagefind search index created successfully"
      ls -lh web/public/_pagefind/
    else
      echo "✗ Pagefind search index not found"
      exit 1
    fi
```

**Key Points:**
- Uses **yarn** (not npm) with `--frozen-lockfile` for reproducible builds
- Caches yarn dependencies using `web/yarn.lock`
- Verifies that the Pagefind index was successfully created
- Fails the build if the search index is missing

#### Deploy Workflow (`.github/workflows/deploy.yml`)

The deployment workflow builds and uploads both the Next.js app and the search index:

```yaml
- name: Build frontend (includes Pagefind indexing)
  run: just build-web
  env:
    NEXT_PUBLIC_API_URL: ${{ secrets.NEXT_PUBLIC_API_URL }}

- name: Upload frontend build
  uses: actions/upload-artifact@v4
  with:
    name: frontend-build
    path: |
      web/.next
      web/public/_pagefind
```

**Artifact Contents:**
- `web/.next` - Next.js build output (standalone server)
- `web/public/_pagefind` - Pagefind search index

## Architecture

### Build Order

1. **Next.js Build** → Generates server-rendered HTML in `.next/server/app`
2. **Pagefind Indexing** → Scans the built HTML and creates search index
3. **Deployment** → Both `.next/` and `public/_pagefind/` are deployed

### Content Marking

Documentation pages are marked for indexing in the layout:

```tsx
// web/app/(docs)/docs/layout.tsx
<article data-pagefind-body>
  {children}
</article>
```

The `data-pagefind-body` attribute tells Pagefind to index this content.

### Search Component

The search UI is implemented in `DocsSearch.tsx`:
- Loads Pagefind library dynamically from `/_pagefind/pagefind.js`
- Provides keyboard shortcuts (Cmd/Ctrl+K)
- Debounced search with 200ms delay
- Keyboard navigation (arrow keys, Enter, Escape)

## Configuration Details

### Pagefind CLI Options

```bash
npx pagefind \
  --site .next/server/app \           # Source directory to index
  --output-path public/_pagefind      # Output directory for index
```

### MDX Plugins for Better Indexing

The following plugins enhance the search experience:

```javascript
// next.config.js
rehypePlugins: [
  require('rehype-slug'),              // Add IDs to headings
  require('rehype-autolink-headings'), // Make headings linkable
]
```

These plugins ensure that:
- Headings have stable IDs for deep linking
- Search results can link directly to specific sections

## Deployment Considerations

### Output Mode

The app uses `output: 'standalone'` for server-side deployment:

```javascript
// next.config.js
const nextConfig = {
  output: 'standalone',
  // ...
}
```

**Why this works:**
- Next.js generates static HTML during build (SSG/SSR)
- Pagefind indexes this pre-rendered HTML
- The search index works client-side, no server needed
- Compatible with both static and server deployments

### Static Assets

The Pagefind index is served as static files:
- `/_pagefind/pagefind.js` - Search library
- `/_pagefind/pagefind-ui.css` - UI styles (if using default UI)
- `/_pagefind/*.pf_index` - Search index files
- `/_pagefind/*.pf_meta` - Metadata files

### Caching Strategy

**Not cached:**
- The search index is regenerated on every build
- Ensures the index is always up-to-date with content changes

**Cached:**
- Node modules (via yarn cache)
- Rust dependencies (via cargo cache)

## Comparison with Soul Player

The BDP implementation is based on the [Soul Player marketing site](https://github.com/soulaudio/soul-player/applications/marketing):

| Aspect | Soul Player | BDP |
|--------|------------|-----|
| **Output Mode** | `export` (static) | `standalone` (server) |
| **Index Source** | `.next/server/app` | `.next/server/app` ✓ |
| **Index Output** | `out/_pagefind` | `public/_pagefind` |
| **Package Manager** | yarn | yarn ✓ |
| **CI/CD** | GitHub Actions | GitHub Actions ✓ |
| **Build Hook** | `postbuild` | `postbuild` ✓ |
| **Deployment** | GitHub Pages | Docker + artifacts |

**Key Differences:**
- Soul Player uses static export (`output: 'export'`) for GitHub Pages
- BDP uses standalone output for Docker deployment with API routes
- Both use the same Pagefind indexing approach

## Testing Locally

### Build and Test

```bash
cd web
yarn build
yarn start
```

Navigate to `http://localhost:3000/docs` and test the search:
1. Press `Cmd/Ctrl+K` to open search
2. Type a query (e.g., "installation")
3. Navigate results with arrow keys
4. Press Enter to open a result

### Verify Index Files

```bash
ls -lh web/public/_pagefind/
```

You should see:
- `pagefind.js` - Main search library
- `*.pf_index` - Index files
- `*.pf_meta` - Metadata files
- `*.pf_fragment` - Content fragments

## Troubleshooting

### Search Not Working

**Symptoms:** Search modal opens but shows "No results"

**Checks:**
1. Verify index was created: `ls web/public/_pagefind/`
2. Check browser console for 404 errors on `/_pagefind/pagefind.js`
3. Ensure `data-pagefind-body` is present in docs layout
4. Rebuild: `cd web && yarn build`

### Index Not Generated in CI/CD

**Symptoms:** Build succeeds but no search index

**Checks:**
1. Check workflow logs for "Verify Pagefind index was created"
2. Ensure `postbuild` script runs: look for "npx pagefind" in logs
3. Verify dependencies are installed: `pagefind` in `devDependencies`

### Search Results Missing Content

**Symptoms:** Search works but results are incomplete

**Checks:**
1. Ensure content is wrapped with `data-pagefind-body`
2. Check that pages are being built (not 404s)
3. Rebuild to regenerate index

## Performance

### Index Size

The search index size depends on content volume:
- **Typical size:** 50-200 KB for small documentation sites
- **Large sites:** 500 KB - 2 MB
- **Per-page overhead:** ~1-5 KB per documentation page

### Search Speed

- **First search:** ~100-300ms (loads index)
- **Subsequent searches:** ~20-50ms (cached index)
- **Debounced:** 200ms delay prevents excessive queries

### Optimization

To reduce index size:
1. Mark only essential content with `data-pagefind-body`
2. Exclude navigation, headers, footers from indexing
3. Use Pagefind filters for large sites

## Future Enhancements

Potential improvements:
- [ ] Add search filters (by section, category)
- [ ] Implement search result highlighting
- [ ] Add search analytics
- [ ] Support for multilingual search (i18n)
- [ ] Custom ranking and relevance tuning

## References

- [Pagefind Documentation](https://pagefind.app/)
- [Next.js Build Output](https://nextjs.org/docs/app/api-reference/next-config-js/output)
- [Soul Player Implementation](https://github.com/soulaudio/soul-player)
- [GitHub Actions Workflow Syntax](https://docs.github.com/en/actions/reference/workflow-syntax-for-github-actions)
