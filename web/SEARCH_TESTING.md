# Search Testing Guide

## Testing Pagefind Search

After running `just web`, the search functionality should be available in the header.

### Expected Behavior

1. **Search Button**: A search button appears in the header with text "Search documentation..." and `⌘K` shortcut
2. **Keyboard Shortcut**: Press `Cmd+K` (Mac) or `Ctrl+K` (Windows/Linux) to open search
3. **Search Modal**: Opens a modal with:
   - Search input field
   - Real-time search results as you type
   - Keyboard navigation (↑↓ arrows, Enter to select, Esc to close)

### Verifying Pagefind Index

1. Check that the index exists:
   ```bash
   ls web/public/_pagefind/
   ```

   Should show files like:
   - `pagefind.js`
   - `pagefind.de_*.pf_meta`
   - `pagefind.en_*.pf_meta`
   - `wasm.de.pagefind`
   - `wasm.en.pagefind`

2. Check browser console:
   - Open DevTools (F12)
   - Should see: "Pagefind script loaded" and "Pagefind initialized successfully"
   - If you see errors, check the Network tab for 404s on `/_pagefind/pagefind.js`

### Testing Search Results

1. Open search (⌘K or click search button)
2. Type a search term (e.g., "installation", "quick start")
3. Should see results appear with:
   - Page title
   - Excerpt/snippet
   - Highlighted matches

### Troubleshooting

**Search button not working:**
- Check browser console for Pagefind loading errors
- Verify files exist in `web/public/_pagefind/`
- Make sure you ran `yarn pagefind` after building

**No search results:**
- Verify that docs pages have `data-pagefind-body` attribute (should be on `<article>` in docs layout)
- Check that static HTML files were generated in `.next/server/app/en/docs/*.html`
- Re-run Pagefind indexing: `cd web && yarn pagefind`

**404 errors on pagefind.js:**
- Check that `public/_pagefind/` directory exists
- Verify Next.js is serving static files from `public/` directory
- In standalone mode, ensure public files are accessible

## Quick Test Commands

```bash
# Build and index
just build-web
cd web && yarn pagefind

# Start production server
cd web && yarn start

# Open http://localhost:3000 and test search with ⌘K
```
