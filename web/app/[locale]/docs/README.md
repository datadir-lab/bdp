# Documentation System

This directory contains the BDP documentation pages with full localization and mobile responsiveness support.

## Features

### Mobile Responsiveness
- **Hamburger Menu**: On mobile devices (< 768px), the left navigation sidebar is hidden and replaced with a hamburger menu button
- **Slide-out Navigation**: Clicking the hamburger opens a smooth slide-out drawer with the full navigation
- **Auto-close**: The drawer automatically closes when a link is clicked
- **Sticky Header**: The mobile navigation header sticks to the top for easy access

### Localization
- **Automatic Fallback**: If a translation is missing, the system automatically falls back to English
- **Warning Banner**: When fallback occurs, a clear warning banner is displayed with a link to contribute translations
- **Type-safe**: The system is fully type-safe with TypeScript
- **Switch-based**: Uses a clean switch statement for locale handling (easy to extend)

## Project Structure

```
docs/
├── components/
│   ├── DocsSidebar.tsx          # Navigation sidebar (used in both desktop and mobile)
│   └── MobileDocsNav.tsx         # Mobile hamburger menu wrapper
├── content/
│   ├── en/                       # English content
│   │   └── introduction.mdx
│   ├── de/                       # German content
│   │   └── introduction.mdx
│   └── [locale]/                 # Add more locales as needed
├── layout.tsx                    # Main docs layout with responsive structure
├── page.tsx                      # Introduction/home page
├── installation/
│   └── page.tsx
├── quick-start/
│   └── page.tsx
└── README.md                     # This file
```

## Adding a New Localized Documentation Page

### Step 1: Create MDX Content Files

Create your content in the `content/[locale]/` directory:

```
content/
├── en/
│   └── my-new-page.mdx
└── de/
    └── my-new-page.mdx (optional - will fallback to English if missing)
```

### Step 2: Create the Page Component

Create `my-new-page/page.tsx`:

```tsx
import DocsPageEn from '../content/en/my-new-page.mdx';
import DocsPageDe from '../content/de/my-new-page.mdx';
import { locales } from '@/i18n/config';
import { loadLocalizedContent, createContentMap } from '@/lib/docs-loader';
import { TranslationFallbackBanner } from '@/components/docs/translation-fallback-banner';

export const dynamic = 'force-static';

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

// Define available translations
const contentMap = createContentMap({
  en: DocsPageEn,
  de: DocsPageDe, // Optional - omit if not available yet
});

export default async function MyNewPage({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  const { Content, loadedLocale, isFallback } = loadLocalizedContent(
    locale,
    contentMap
  );

  return (
    <>
      {isFallback && (
        <TranslationFallbackBanner
          requestedLocale={locale}
          fallbackLocale={loadedLocale}
        />
      )}
      <Content />
    </>
  );
}
```

### Step 3: Add to Navigation

Update `components/DocsSidebar.tsx` to include your new page:

```tsx
const docsNavigation: NavItem[] = [
  {
    title: 'Your Section',
    href: '/docs/your-section',
    items: [
      { title: 'My New Page', href: '/docs/my-new-page' },
    ],
  },
];
```

## Adding a New Locale

To add support for a new language (e.g., Spanish):

### Step 1: Update i18n Config

Edit `web/i18n/config.ts`:

```tsx
export const locales = ['en', 'de', 'es'] as const; // Add 'es'
```

### Step 2: Update the Docs Loader

Edit `web/lib/docs-loader.tsx` and add a case to the switch statement:

```tsx
switch (locale as Locale) {
  case 'en':
    Content = contentMap.en;
    break;
  case 'de':
    Content = contentMap.de;
    break;
  case 'es':  // Add this
    Content = contentMap.es;
    break;
  default:
    Content = undefined;
}
```

### Step 3: Create Content Directory

Create `content/es/` directory and add translations:

```
content/
├── en/
├── de/
└── es/          # New locale
    └── introduction.mdx
```

### Step 4: Import in Pages

Update your page components to import the new translations:

```tsx
import DocsPageEs from '../content/es/my-page.mdx';

const contentMap = createContentMap({
  en: DocsPageEn,
  de: DocsPageDe,
  es: DocsPageEs,  // Add this
});
```

## Responsive Breakpoints

The documentation layout uses these breakpoints:

- **Mobile**: < 768px (md)
  - Hamburger menu visible
  - Full-width content
  - No sidebars visible

- **Tablet**: 768px - 1279px (md to xl)
  - Left sidebar visible
  - Content with margins
  - Right sidebar hidden

- **Desktop**: ≥ 1280px (xl)
  - All sidebars visible
  - Full three-column layout

## Components Reference

### `<DocsSidebar />`
Navigation sidebar component.

Props:
- `onLinkClick?: () => void` - Optional callback when a link is clicked (used by mobile menu to close)

### `<MobileDocsNav />`
Mobile hamburger menu wrapper. Automatically hidden on desktop.

### `<TranslationFallbackBanner />`
Warning banner shown when content falls back to English.

Props:
- `requestedLocale: string` - The locale the user requested
- `fallbackLocale: string` - The locale actually being shown

### `loadLocalizedContent()`
Utility function to load localized MDX content with automatic fallback.

Returns:
- `Content: ComponentType` - The MDX component to render
- `loadedLocale: Locale` - Which locale was loaded
- `isFallback: boolean` - Whether fallback was used

## Styling

The documentation uses:
- **Tailwind CSS** for utility styling
- **Prose plugin** for beautiful typography in MDX content
- **Radix UI** for accessible components (Sheet, Dialog, etc.)
- **Dark mode** support built-in

## Best Practices

1. **Always provide English content** - It's the fallback language
2. **Use the type-safe helpers** - `createContentMap()` ensures English exists
3. **Keep MDX files focused** - One topic per file
4. **Test on mobile** - Always verify the hamburger menu works
5. **Keep navigation depth shallow** - Max 2 levels for usability
6. **Add aria labels** - For accessibility (already included in components)

## Troubleshooting

### "No content available" error
- Ensure you have at least English content defined in `createContentMap()`
- Check that the MDX file exists and exports a default component

### Mobile menu not closing
- Ensure `onLinkClick` is passed to `<DocsSidebar />` in mobile context
- Verify the Sheet's `onOpenChange` is properly wired

### Translation not showing
- Check that the locale is added to `i18n/config.ts`
- Verify the switch case exists in `docs-loader.tsx`
- Ensure the MDX file is imported in the page component
