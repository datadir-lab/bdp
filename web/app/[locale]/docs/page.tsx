import DocsIntroductionEn from './content/en/introduction.mdx';
import DocsIntroductionDe from './content/de/introduction.mdx';
import { locales } from '@/i18n/config';
import { loadLocalizedContent, createContentMap } from '@/lib/docs-loader';
import { TranslationFallbackBanner } from '@/components/docs/translation-fallback-banner';

// Force static generation of docs pages
export const dynamic = 'force-static';

// Generate static params for all locales
export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

// Define available translations for this page
const contentMap = createContentMap({
  en: DocsIntroductionEn,
  de: DocsIntroductionDe,
});

export default async function DocsIntroduction({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  // Load localized content with automatic fallback
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
