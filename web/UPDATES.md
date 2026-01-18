# Latest Updates - 2026-01-16

## Logo & Font Updates

### 1. **JetBrains Mono Font** ✅
- Imported from Google Fonts (inspired by opencode.ai)
- Applied to logo and monospace text
- CSS variable: `--font-mono`
- Fallback chain: JetBrains Mono → Courier New → monospace

### 2. **Reusable Logo Component** ✅
- **File**: `components/shared/logo.tsx`
- **Props**:
  - `size`: 'sm' | 'md' | 'lg'
  - `className`: Optional custom classes
  - `linkable`: Boolean (default: true)
- **Font**: Uses `font-mono` (JetBrains Mono)
- **Text**: Simply "bdp" (no box, no tagline)
- **Usage**:
  - Header: `<Logo size="md" />`
  - Footer: `<Logo size="lg" linkable={false} />`

### 3. **Simplified Header** ✅
- Logo shows just "bdp" in top-left
- Removed tagline and colored box
- Clean, minimal terminal aesthetic

### 4. **Updated Footer** ✅
- Uses Logo component (size="lg")
- Logo is not linkable in footer
- Updated description to bioinformatics focus
- Uses translations for dynamic content

## UI Component Updates

### 5. **Locale Switcher Fixes** ✅
- **Fixed**: Dropdown now closes properly after selection
- **Controlled state**: Added `open` and `onOpenChange` handlers
- **Variant**: Changed from `ghost` to `outline`
- **Behavior**: Closes immediately on selection
- **Text**: Shows language name (not flag) on desktop

### 6. **Theme Toggle Styling** ✅
- **Variant**: Changed from `ghost` to `outline`
- **Size**: Consistent with locale switcher
- **Width**: Fixed width (w-9) for icon button
- Both switchers now have matching outlined button style

## Font Configuration

### Tailwind Config
```js
fontFamily: {
  sans: ['var(--font-inter)', 'system-ui', 'sans-serif'],
  mono: ['var(--font-mono)', 'JetBrains Mono', 'Courier New', 'monospace'],
}
```

### Layout Setup
```tsx
const inter = Inter({ subsets: ['latin'], variable: '--font-inter' });
const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
  display: 'swap',
});
```

## Component Structure

```
components/
├── shared/
│   ├── logo.tsx              # NEW: Reusable logo component
│   ├── locale-switcher.tsx   # UPDATED: Fixed dropdown, outlined style
│   └── theme-toggle.tsx      # UPDATED: Outlined style
├── layout/
│   ├── header.tsx            # UPDATED: Uses Logo component
│   └── footer.tsx            # UPDATED: Uses Logo component
```

## Visual Changes

### Before
- Logo: "bdp" in colored box + "BDP" text + tagline
- Switchers: Ghost variant buttons
- Locale: Showed flag + name
- Footer: Old "B" logo

### After
- Logo: Just "bdp" in JetBrains Mono font
- Switchers: Outlined buttons (matching style)
- Locale: Shows name (English/Deutsch) + flag in dropdown
- Footer: Reuses same logo component (larger)

## Benefits

1. **Consistency**: Logo is reused in header and footer
2. **Terminal Feel**: JetBrains Mono gives developer/CLI aesthetic
3. **Clean Design**: Simplified logo is more professional
4. **Accessibility**: Outlined buttons are clearer UI elements
5. **UX**: Dropdown closes properly after selection
6. **Matching Style**: Both theme and locale buttons have same appearance

## Testing

✅ Logo appears in header (md size)
✅ Logo appears in footer (lg size)
✅ Logo uses JetBrains Mono font
✅ Locale dropdown closes after selection
✅ Both switchers are outlined buttons
✅ Footer uses translations
✅ All fonts load correctly

---

**Last Updated**: 2026-01-16
**Components Modified**: 6 files
**New Components**: 1 (Logo)
